mod chats;
mod savehandler;

use reqwest::Client;

/// Client for making requests to the memory server.
pub struct MemoryClient {
    pub client: Client,
    pub base_url: String,
}

impl Default for MemoryClient {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            client: Client::new(),
        }
    }
}
