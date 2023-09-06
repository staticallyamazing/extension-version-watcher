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

use anyhow::{bail, Context, Result};
use reqwest::{get, Client};
use tracing::{debug, trace};
use xml::{reader::XmlEvent, EventReader};

use crate::extensions::Extension;

#[tracing::instrument(ret, err)]
pub async fn check_extension(extension: &Extension) -> Result<(String, String)> {
    debug!("checking");
    if let Some(url) = &extension.url {
        // Other

        let xml = get(url)
            .await
            .with_context(|| format!("couldn't fetch {url}"))?
            .bytes()
            .await
            .with_context(|| format!("couldn't convert response of {url} to bytes"))?
            .to_vec();
        let parser = EventReader::new(xml.as_slice());

        let mut found_app = false;
        let mut cur_version = None;
        let mut crx_url = None;

        for e in parser {
            match e.context("couldn't visit XML")? {
                XmlEvent::StartElement {
                    name,
                    attributes,
                    namespace: _,
                } => {
                    if found_app {
                        if name.local_name == "updatecheck" {
                            trace!("found updatecheck element");

                            if let Some(codebase) =
                                attributes.iter().find(|a| a.name.local_name == *"codebase")
                            {
                                trace!("found codebase attribute");
                                crx_url = Some(codebase.value.clone());
                            }

                            if let Some(version) =
                                attributes.iter().find(|a| a.name.local_name == *"version")
                            {
                                trace!("found version attribute");
                                cur_version = Some(version.value.clone());
                            }
                        }
                    } else if name.local_name == "app"
                        && attributes
                            .iter()
                            .any(|a| a.name.local_name == *"appid" && a.value == *extension.id)
                    {
                        found_app = true;
                        trace!("found app element");
                    }
                }
                _ => {}
            }
        }

        if let Some(cur_version) = cur_version {
            if let Some(crx_url) = crx_url {
                Ok((cur_version, crx_url))
            } else {
                bail!("no crx url");
            }
        } else {
            bail!("no version");
        }
    } else {
        // Webstore

        let url = format!(
            "https://chrome.google.com/webstore/ajax/detail?id={}&hl=en&pv=20210820",
            extension.id
        );
        let json = Client::new()
            .post(&url)
            .header("Content-Length", "0")
            .send()
            .await
            .with_context(|| format!("couldn't fetch {url}"))?
            .bytes()
            .await
            .with_context(|| format!("couldn't convert response for {url} to bytes"))?
            .to_vec();
        let json: serde_json::Value = serde_json::from_slice(&json[5..]).expect("not JSON?");
        let cur_version = &json[1][1][6];
        Ok((
            cur_version.as_str().unwrap().into(),
            format!(
                "https://clients2.google.com/service/update2/crx?response=redirect&acceptformat=crx2,crx3&prodversion=110.0&x=id%3D{}%26installsource%3Dondemand%26uc",
                extension.id
            ),
        ))
    }
}
