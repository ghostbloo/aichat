use crate::config::GlobalConfig;
use crate::memory::chats::{chat_add_messages, chat_create, ChatMessage};
use crate::memory::MemoryClient;
use anyhow::{Context, Result};
use log::debug;

/// Write Session chat messages to the memory server.
/// If the Session has no chat ID, a new Chat is created and the ID is written to the Session.
pub async fn sync_session(config: &GlobalConfig) -> Result<()> {
    debug!("Syncing session to memory server");

    let (memory_conf, existing_chat_id, session_name, messages_to_upload) = {
        let config_guard = config.read();

        let memory_conf = match config_guard.memory_client().map(|mc| mc.config.clone()) {
            Some(memory_conf) => memory_conf,
            None => {
                debug!("No memory client configured for session upload");
                return Ok(());
            }
        };

        let session_ref = config_guard
            .session
            .as_ref()
            .context("No active session to upload")?;

        let session_name = session_ref.name().to_string();
        let existing_chat_id = session_ref.chat_id().map(|id| id.to_string());

        // Format messages
        let messages: Vec<ChatMessage> = session_ref
            .get_compressed_messages()
            .iter()
            .map(|m| ChatMessage {
                role: m.role,
                content: m.content.to_text(),
            })
            .collect();

        (memory_conf, existing_chat_id, session_name, messages)
    };

    let chat_id = match existing_chat_id {
        Some(id) => id,
        None => {
            let temp_memory_client = MemoryClient {
                client: reqwest::Client::new(),
                config: memory_conf.clone(),
            };

            let new_chat_id = chat_create(&temp_memory_client, &session_name).await?.id;
            {
                let mut config_guard = config.write();
                let session = config_guard
                    .session
                    .as_mut()
                    .context("Session disappeared unexpectedly while trying to set chat ID")?;
                debug!("Setting chat ID for session {}", session.name());
                session.set_chat_id(&new_chat_id);
            }
            new_chat_id
        }
    };

    let temp_memory_client = MemoryClient {
        client: reqwest::Client::new(),
        config: memory_conf,
    };
    let num_messages = messages_to_upload.len();
    chat_add_messages(&temp_memory_client, &chat_id, messages_to_upload).await?;
    debug!("Uploaded {} messages to chat {}", num_messages, chat_id);

    Ok(())
}
