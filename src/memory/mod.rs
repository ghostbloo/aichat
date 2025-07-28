pub mod chats;

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryConfig {
    pub base_url: String,
}

/// Client for making requests to the memory server.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MemoryClient {
    pub client: Client,
    pub config: MemoryConfig,
}
