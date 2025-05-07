use crate::config::session::compress_session;
use crate::config::GlobalConfig;
use crate::memory::chats::{chat_add_messages, chat_create, chat_set_summary, ChatMessage};
use anyhow::{Context, Result};
use log::debug;

/// Write Session chat messages to the memory server.
/// If the Session has no chat ID, a new Chat is created and the ID is written to the Session.
pub async fn sync_session(config: &GlobalConfig, name: Option<&str>) -> Result<()> {
    debug!("Syncing session to memory server");

    let summary = compress_session(config).await?;

    // Read config and get session data
    let (memory_client, existing_chat_id, session_name, messages) = {
        let config_read = config.read();
        let memory_client = match config_read.memory_client() {
            Some(memory_client) => memory_client.clone(),
            None => {
                drop(config_read);
                debug!("Memory server not configured, saving locally");
                config.write().save_session(name)?;
                return Ok(());
            }
        };
        let session_ref = config_read
            .session
            .as_ref()
            .context("No active session to upload")?;

        let session_name = session_ref.name().to_string();
        let existing_chat_id = session_ref.chat_id().map(|id| id.to_string());

        // Format messages
        let messages: Vec<ChatMessage> = session_ref
            .get_compressed_messages()
            .iter()
            .filter(|m| !m.is_sync)
            .map(|m| ChatMessage {
                role: m.role,
                content: m.content.to_text(),
                is_sync: true,
            })
            .collect();

        (memory_client, existing_chat_id, session_name, messages)
    };

    let chat_id = match existing_chat_id {
        Some(id) => id,
        None => {
            // Create a chat for this session and write its ID to the session
            let new_chat_id = chat_create(&memory_client, &session_name).await?.id;
            config
                .write()
                .session
                .as_mut()
                .context("Session disappeared unexpectedly while trying to set chat ID")?
                .set_chat_id(&new_chat_id);
            new_chat_id
        }
    };

    chat_add_messages(&memory_client, &chat_id, messages).await?;
    chat_set_summary(&memory_client, &chat_id, &summary).await?;

    // Set is_sync to true for all compressed messages
    config
        .write()
        .session
        .as_mut()
        .context("Session disappeared unexpectedly while trying to set chat ID")?
        .set_messages_synced();

    // Save to disk
    config.write().save_session(name)?;

    Ok(())
}
