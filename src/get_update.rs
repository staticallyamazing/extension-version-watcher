/*
extension-version-watcher: rust program to check for updates in chrome extensions
Copyright (C) 2023  staticallyamazing

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use futures_util::future::join_all;
use reqwest::get;
use tokio::{process::Command, sync::Mutex};
use tracing::{debug, trace};
use walkdir::WalkDir;

use crate::{extensions::Extension, PRETTIERRC_PATH};

#[derive(Debug)]
pub struct Update {
    pub prev_version: String,
    pub cur_version: String,
    pub diff: Option<String>,
}

#[tracing::instrument(skip(versions), ret, err)]
pub async fn get_update(
    force_generate_diffs: &Option<bool>,
    extension: &Extension,
    versions: Arc<Mutex<HashMap<String, String>>>,
    cur_version: String,
    crx_url: String,
) -> Result<Option<Update>> {
    let mut versions = versions.lock().await;
    let mut prev_version = versions.remove(&extension.name).unwrap_or_default();
    versions.insert(extension.name.clone(), cur_version.clone());
    drop(versions);

    if prev_version.is_empty() {
        prev_version = "None".into();
    }

    if cur_version != prev_version {
        debug!(prev_version, "found update");

        let crx = get(&crx_url)
            .await
            .context("couldn't fetch")?
            .bytes()
            .await
            .context("couldn't convert response to bytes")?
            .to_vec();
        tokio::fs::write(format!("./crx/{}-{}.crx", extension.name, cur_version), crx)
            .await
            .context("couldn't write crx file")?;

        trace!("unzipping");
        tokio::fs::create_dir_all(format!("./crx/{}-{}", extension.name, cur_version))
            .await
            .context("couldn't create dir for extraction of crx file")?;
        Command::new("unzip")
            .arg("-u")
            .arg(format!("../{}-{cur_version}.crx", extension.name))
            .current_dir(format!("./crx/{}-{}", extension.name, cur_version))
            .spawn()?
            .wait()
            .await
            .context("couldn't extract crx file")?;
        trace!("deleting crx");
        tokio::fs::remove_file(format!("./crx/{}-{}.crx", extension.name, cur_version))
            .await
            .context("couldn't delete crx file")?;

        let mut generate_diff = extension.generate_diff;
        if let Some(ref force_generate_diffs) = force_generate_diffs {
            generate_diff = *force_generate_diffs;
        }
        if generate_diff {
            trace!("finding files to format");
            let dir_path = format!("./crx/{}-{cur_version}", extension.name);
            let mut files = tokio::task::spawn_blocking(move || {
                let mut children = vec![];
                for entry in WalkDir::new(dir_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    children.push((
                        entry.path().display().to_string(),
                        entry.metadata().unwrap().len(),
                    ));
                }
                children
            })
            .await
            .unwrap();
            files.sort_by(|(_, a), (_, b)| a.cmp(b));
            files.reverse();
            const CHUNKS: usize = 5;
            let file_chunks = {
                let mut chunks = vec![vec![]; CHUNKS];

                for unsorted_chunk in files.chunks(CHUNKS) {
                    for item in unsorted_chunk {
                        let chunk_with_least = {
                            let mut chunk_with_least = None;
                            for chunk in chunks.iter_mut() {
                                if chunk_with_least.is_none() {
                                    chunk_with_least = Some(chunk);
                                } else if chunk.iter().map(|c: &&(_, _)| c.1).sum::<u64>()
                                    < chunk_with_least.as_ref().unwrap().iter().map(|c| c.1).sum()
                                {
                                    chunk_with_least = Some(chunk);
                                }
                            }
                            chunk_with_least.unwrap()
                        };
                        chunk_with_least.push(item);
                    }
                }

                chunks
            };
            trace!(?file_chunks, "formatting");
            let children = file_chunks
                .into_iter()
                .map(|files| {
                    let mut command = Command::new("prettier");
                    command
                        .arg("--config")
                        .arg(PRETTIERRC_PATH)
                        .arg("--ignore-path=")
                        .arg("--write");
                    for (file, _) in files {
                        command.arg(file);
                    }
                    command.output()
                })
                .collect::<Vec<_>>();
            join_all(children)
                .await
                .into_iter()
                .filter_map(|c| c.ok())
                .for_each(|o| print!("{}", String::from_utf8_lossy(&o.stdout)));
        }

        if prev_version != "None" && generate_diff {
            trace!("getting diff");
            let diff = Command::new("diff")
                .arg("-U")
                .arg("10")
                .arg("-r")
                .arg(format!("./{}-{prev_version}", extension.name))
                .arg(format!("./{}-{cur_version}", extension.name))
                .current_dir("./crx")
                .output()
                .await
                .context("couldn't get diff")?;
            let stderr = String::from_utf8_lossy(&diff.stderr);
            let stderr = stderr.trim();
            if !stderr.is_empty() {
                eprintln!("{stderr}");
                bail!("diff did not exit successfully");
            } else {
                let diff = String::from_utf8_lossy(&diff.stdout);
                let diff = regex::Regex::new(r"\t\d\d\d\d-\d.*")
                    .unwrap()
                    .replace_all(&diff, "");
                Ok(Some(Update {
                    prev_version,
                    cur_version,
                    diff: Some(diff.to_string()),
                }))
            }
        } else {
            trace!("skipping diff");
            Ok(Some(Update {
                prev_version,
                cur_version,
                diff: None,
            }))
        }
    } else {
        debug!("no update found");
        Ok(None)
    }
}
