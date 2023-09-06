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
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::future::join_all;
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

mod check_extension;
mod extensions;
mod get_update;
mod send_to_discord;

use crate::check_extension::check_extension;
use crate::extensions::{builtin_extensions, Extension};
use crate::get_update::{get_update, Update};
use crate::send_to_discord::send_to_discord;

const fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_true")]
    use_builtin_extensions: bool,
    force_generate_diffs: Option<bool>,
    extra_extensions: Option<Vec<Extension>>,
    discord: Option<DiscordConfig>,
}

#[derive(Deserialize)]
pub struct DiscordConfig {
    token: String,
    channel_ids: Vec<u64>,
}

impl Debug for DiscordConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordConfig")
            .field("channel_ids", &self.channel_ids)
            .finish()
    }
}

const CONFIG_EXAMPLE_PATH: &str = "./config.example.toml";
const CONFIG_PATH: &str = "./config.toml";
const DEFAULT_CONFIG: &str = r#"# config file for extension-version-watcher
# this file must be named config.toml and must be located in the current working directory when extension-version-watcher is run

# to customize logging, use the RUST_LOG environment variable. it should be a comma separated list of one of:
# - `foo=trace` (TARGET=LEVEL)
# - `foo[{bar,baz}]=info` (TARGET[{FIELD,+}]=LEVEL)
# - `trace` (bare LEVEL)
# - `foo` (bare TARGET)
# RUST_LOG defaults to `extension_version_watcher=trace`
# if you just want less verbose logging, set RUST_LOG to `extension_version_watcher=info`


# if you don't want the builtin extensions to be checked, uncomment the following line. defaults to true
#use_builtin_extensions = false

# if true, diffs will always be generated for all extensions.
# if false, diffs will never be generated for any extensions.
# if not specified, diffs will be generated according to extension specific settings.
# diff generation currently requires prettier (https://prettier.io) to be on PATH
# if you would like to use a custom prettier config, simply create .prettierrc.json in the current working directory. extension-version-watcher will see this and use it instead of the builtin config.
#force_generate_diffs = false

# extra extensions to add to the extension list
# template / format (you can also specify extra extensions as a normal array, but toml doesn't allow inline tables to have newlines so each extension is limited to 1 line):
#[[extra_extensions]]
#name = "" # the name of the extension. this should only contain alphanumeric characters, underscores and hyphens. it will be used for directory and file names, as well as keys for versions.toml
#          # after you set this value, do not change it. it will cause the extension's previously checked version to be reset.
#display_name = "" # the display name of the extension. this will show up in logs and in the update messages that are sent to discord
#id = "" # the chrome extension id
##url = "" # (optional) the chrome extension update URL to use when checking for updates.
#          # this should resolve to an XML file that has the chrome extension update format.
#          # if url is not specified, the chrome webstore is searched for an extension with the specified id
#generate_diff = true # if a diff should be automatically generated. can be overridden by force_generate_diffs


# if you comment the following line, update messages will not be sent to discord and you will not need to specify discord.token and discord.channel_ids
[discord]

# the bot token. must be a string
#token = ""

# discord channel IDs to send the update messages to. if you want to send update messages to discord, make sure to set this. must be an array of numbers that are >0
#channel_ids = []
"#;

const VERSIONS_PATH: &str = "./versions.toml";
const VERSIONS_HEADER: &str = r#"# versions file for extension-version-watcher
# this file must be named versions.toml and must be located in the current working directory when extension-version-watcher is run
# this file is used to track the previously downloaded extension versions. you should not modify this file
# if you want to reset the previously downloaded extension versions, simply delete this file

"#;

const PRETTIERRC_PATH: &str = "./.prettierrc.json";
const DEFAULT_PRETTIERRC: &str = include_str!("../.prettierrc.json");

#[tokio::main]
async fn main() {
    use tracing_subscriber::{
        filter::{LevelFilter, Targets},
        prelude::*,
    };
    let targets = match std::env::var("RUST_LOG").map(|e| e.parse::<Targets>()).ok() {
        Some(res) => match res {
            Ok(t) => t,
            Err(e) => {
                return eprintln!(
                    "an error occurred when parsing the RUST_LOG environment variable: {e}"
                )
            }
        },
        None => "extension_version_watcher=trace"
            .parse::<Targets>()
            .unwrap(),
    };
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_target(targets.would_enable("core", &tracing::Level::ERROR)) // only enable targets if targets other than extension_version_watcher are allowed
        .finish()
        .with(targets)
        .init();

    if let Err(error) = main_error_net().await {
        error!("{error:?}");
    }
}

async fn main_error_net() -> Result<()> {
    tokio::fs::create_dir_all("./crx")
        .await
        .context("couldn't create crx dir")?;
    tokio::fs::create_dir_all("./diff")
        .await
        .context("couldn't create diff dir")?;

    if tokio::fs::try_exists(CONFIG_EXAMPLE_PATH)
        .await
        .context("couldn't check if config.example.toml exists")?
    {
        tokio::fs::write(CONFIG_EXAMPLE_PATH, DEFAULT_CONFIG)
            .await
            .context("failed to write config.example.toml")?;
    }

    if !tokio::fs::try_exists(CONFIG_PATH)
        .await
        .context("couldn't check if config.toml exists")?
    {
        tokio::fs::write(CONFIG_PATH, DEFAULT_CONFIG)
            .await
            .context("failed to create config.toml")?;
    }

    let config = tokio::fs::read_to_string(CONFIG_PATH)
        .await
        .context("failed to read config.toml")?;
    let config: Config = toml::from_str(&config).context("failed to deserialize config.toml")?;

    let versions: HashMap<String, String> = match tokio::fs::read_to_string(VERSIONS_PATH).await {
        Ok(v) => toml::from_str(&v).context("failed to deserialize versions.toml")?,
        Err(error) => {
            warn!(%error, "failed to read versions.toml, versions will be empty");
            HashMap::new()
        }
    };
    debug!(?versions);
    let versions = Arc::new(Mutex::new(versions));

    let mut extensions = if config.use_builtin_extensions {
        builtin_extensions()
    } else {
        vec![]
    };
    if let Some(extra_extensions) = config.extra_extensions {
        extensions.reserve(extra_extensions.len());
        for extension in extra_extensions {
            info!(?extension, "adding extra extension");
            extensions.push(extension);
        }
    }

    let tmp_prettierrc = if !tokio::fs::try_exists(PRETTIERRC_PATH)
        .await
        .context("couldn't check if .prettierrc.json exists")?
    {
        tokio::fs::write(PRETTIERRC_PATH, DEFAULT_PRETTIERRC)
            .await
            .context("failed to create .prettierrc.json")?;
        true
    } else {
        info!(".prettierrc.json already exists. the builtin .prettierrc.json will not be used");
        false
    };

    let checked_extensions =
        check_extensions(config.force_generate_diffs, extensions, &versions).await;
    for (extension, update) in &checked_extensions {
        match update {
            Ok(ref update) => {
                if let Some(ref update) = update {
                    info!(
                        "{}: {} -> {}",
                        extension.display_name, update.prev_version, update.cur_version
                    );
                } else {
                    info!("{}: no update", extension.display_name);
                }
            }
            Err(error) => error!("{}: {error:?}", extension.display_name),
        }
    }

    if tmp_prettierrc {
        if let Err(error) = tokio::fs::remove_file(PRETTIERRC_PATH).await {
            warn!(%error, "failed to remove temporary .prettierrc.json");
        }
    }

    tokio::fs::write(
        VERSIONS_PATH,
        format!(
            "{VERSIONS_HEADER}{}",
            toml::to_string(&*versions.lock().await).unwrap()
        ),
    )
    .await
    .context("failed to write versions.toml")?;

    if let Some(discord_config) = config.discord {
        send_to_discord(discord_config, &checked_extensions).await;
    } else {
        info!("skipping sending update message to discord since there is no discord table in config.toml")
    }

    Ok(())
}

async fn check_extensions(
    force_generate_diffs: Option<bool>,
    extensions: Vec<Extension>,
    versions: &Arc<Mutex<HashMap<String, String>>>,
) -> Vec<(Extension, Result<Option<Update>>)> {
    info!("checking extensions");

    let mut tasks = vec![];
    for extension in extensions {
        let versions = Arc::clone(versions);
        tasks.push(tokio::task::spawn(async move {
            let update = match check_extension(&extension).await {
                Ok((cur_version, crx_url)) => {
                    get_update(
                        &force_generate_diffs,
                        &extension,
                        versions,
                        cur_version,
                        crx_url,
                    )
                    .await
                }
                Err(e) => Err(e),
            };
            (extension, update)
        }));
    }

    let output = join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.expect("join failed"))
        .collect();

    info!("done checking extensions");
    output
}
