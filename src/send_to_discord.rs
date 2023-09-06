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

use anyhow::Result;
use tracing::{debug, error, info, Instrument};
use twilight_http::Client;
use twilight_model::{http::attachment::Attachment, id::Id};

use crate::get_update::Update;
use crate::{extensions::Extension, DiscordConfig};

#[tracing::instrument(skip(checked_extensions))]
pub async fn send_to_discord(
    config: DiscordConfig,
    checked_extensions: &Vec<(Extension, Result<Option<Update>>)>,
) {
    let mut updates_text = vec![];
    let mut errors_text = vec![];
    let mut attachments = vec![];
    let mut attachment_id = 0;

    for (extension, update) in checked_extensions {
        match update {
            Ok(ref update) => {
                if let Some(ref update) = update {
                    updates_text.push(format!(
                        "- {}: `{}` -> `{}`{}",
                        extension.display_name,
                        update.prev_version,
                        update.cur_version,
                        if update.diff.is_some() {
                            " (diff automatically generated)"
                        } else {
                            ""
                        }
                    ));
                    if let Some(diff) = &update.diff {
                        let filename = format!(
                            "{}-{}-{}.diff",
                            extension.name, update.prev_version, update.cur_version
                        );
                        attachments.push(Attachment {
                            description: None,
                            file: diff.as_bytes().to_vec(),
                            filename: filename.clone(),
                            id: attachment_id,
                        });
                        attachment_id += 1;

                        if let Err(error) =
                            tokio::fs::write(format!("./diff/{filename}"), diff).await
                        {
                            error!(%error, filename, "failed to write diff file");
                        }
                    }
                }
            }
            Err(error) => {
                errors_text.push(format!("- {}: {error}", extension.display_name).replace(
                    &std::env::current_dir().unwrap().display().to_string(),
                    "$PWD",
                ))
            }
        }
    }

    if updates_text.is_empty() && errors_text.is_empty() {
        info!("no updates or errors");
        return;
    }
    let newlines = if !updates_text.is_empty() && !errors_text.is_empty() {
        "\n\n"
    } else {
        ""
    };
    let updates_text = updates_text.join("\n");
    let errors_text = if !errors_text.is_empty() {
        format!(
            "The following errors occurred:\n\n{}",
            errors_text.join("\n")
        )
    } else {
        "".into()
    };
    let update_message = format!(
    "**__Extension Updates__**

{updates_text}{newlines}{errors_text}

> *🤖 Automated by <@1019305439000801311>. Please ping them for any questions or suggestions (don't expect them to respond quickly).*
> *Open source at <https://github.com/staticallyamazing/extension-version-watcher>.*
> *Version: {}*", env!("CARGO_PKG_VERSION"));

    let client = Client::new(config.token);
    let attachments = attachments.as_slice();
    for channel in config.channel_ids {
        async {
            debug!("sending update message");
            if let Err(error) = client
                .create_message(Id::new(channel))
                .attachments(attachments)
                .unwrap()
                .content(&update_message)
                .unwrap()
                .await
            {
                error!(?error, "failed to send update message");
            } else {
                info!("sent update message");
            }
        }
        .instrument(tracing::info_span!("channel", channel))
        .await
    }
}
