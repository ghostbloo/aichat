use crate::config::GlobalConfig;
use anyhow::Result;
use log::debug;

/// Write Session chat messages to the memory server.
/// Stub implementation since memory functionality was removed in upstream.
pub async fn sync_session(config: &GlobalConfig, name: Option<&str>) -> Result<()> {
    debug!("Memory functionality removed, saving session locally");
    config.write().save_session(name)?;
    Ok(())
}
