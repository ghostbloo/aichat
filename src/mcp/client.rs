use anyhow::{Context, Result};
use rmcp::{
    service::{DynService, RunningService, ServiceExt},
    transport::{child_process::TokioChildProcess, sse::SseTransport},
    RoleClient,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, process::Stdio};
use tokio::process::Command;

use super::tool::{get_mcp_tools, ToolSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, rename = "mcpServers")]
    pub servers: HashMap<String, McpServerConfig>,
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Create and start clients for the configured servers
    pub async fn create_clients(&self) -> Result<HashMap<String, McpServer>> {
        let mut servers = HashMap::new();
        for (name, config) in &self.servers {
            let server = config.connect().await?;
            servers.insert(name.clone(), server);
        }
        Ok(servers)
    }
}

type McpServer = RunningService<RoleClient, Box<dyn DynService<RoleClient> + 'static>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerConfig {
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
}

impl McpServerConfig {
    /// Connect to the transport
    pub async fn connect(&self) -> Result<McpServer> {
        let client = match self {
            McpServerConfig::Sse { url } => {
                ().into_dyn().serve(SseTransport::start(url).await?).await?
            }
            McpServerConfig::Stdio { command, args, env } => {
                ().into_dyn()
                    .serve(TokioChildProcess::new(
                        Command::new(command)
                            .args(args)
                            .envs(env)
                            .stderr(Stdio::inherit())
                            .stdout(Stdio::inherit()),
                    )?)
                    .await?
            }
        };
        Ok(client)
    }
}

pub struct McpAdapter {
    pub clients: HashMap<String, McpServer>,
    pub toolset: ToolSet,
}

impl McpAdapter {
    pub async fn init(configs: Config) -> Result<Self> {
        let mut clients = HashMap::new();
        let mut toolset = ToolSet::default();

        for (name, config) in configs.servers {
            let client = config.connect().await?;
            let service = client.service();
            let peer = DynService::get_peer(service)
                .context(format!("Could not get peer for server {}", name))?;
            let tools = get_mcp_tools(peer).await?;

            toolset.add(tools);

            clients.insert(name.clone(), client);
        }

        Ok(Self { clients, toolset })
    }
}
