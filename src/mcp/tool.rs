use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use rmcp::{
    model::{CallToolRequestParam, CallToolResult, Tool as McpTool, ToolAnnotations},
    service::ServerSink,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::McpError;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters(&self) -> Value;
    fn annotations(&self) -> ToolAnnotations;
    async fn call(&self, args: Value) -> Result<CallToolResult>;
}

pub struct McpToolAdapter {
    tool: McpTool,
    server: ServerSink,
}

impl McpToolAdapter {
    pub fn new(tool: McpTool, server: ServerSink) -> Self {
        Self { tool, server }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> String {
        self.tool.name.clone().to_string()
    }

    fn description(&self) -> String {
        self.tool
            .description
            .clone()
            .unwrap_or_default()
            .to_string()
    }

    fn parameters(&self) -> Value {
        serde_json::to_value(&self.tool.input_schema).unwrap_or(serde_json::json!({}))
    }

    fn annotations(&self) -> ToolAnnotations {
        self.tool.annotations.clone().unwrap_or_default()
    }

    async fn call(&self, args: Value) -> Result<CallToolResult> {
        let arguments = match args {
            Value::Object(map) => Some(map),
            _ => None,
        };

        let result = self
            .server
            .call_tool(CallToolRequestParam {
                name: self.tool.name.clone(),
                arguments,
            })
            .await?;

        Ok(result)
    }
}

#[derive(Default)]
pub struct ToolSet {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolSet {
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn add<T: Tool + 'static>(&mut self, tools: Vec<T>) {
        for tool in tools {
            self.tools.insert(tool.name(), Arc::new(tool));
        }
    }

    /// Find and call a tool
    pub async fn call(&self, name: &str, args: Value) -> Result<CallToolResult> {
        let result = self
            .tools
            .get(name)
            .context(format!("Tool {} not found", name))?
            .call(args)
            .await?;
        Ok(result)
    }
}

pub async fn get_mcp_tools(server: ServerSink) -> Result<Vec<McpToolAdapter>> {
    let tools = server.list_all_tools().await?;
    Ok(tools
        .into_iter()
        .map(|tool| McpToolAdapter::new(tool, server.clone()))
        .collect())
}

pub trait IntoCallToolResult {
    fn into_call_tool_result(self) -> Result<ToolResult, McpError>;
}

impl<T> IntoCallToolResult for Result<T, McpError>
where
    T: Serialize,
{
    fn into_call_tool_result(self) -> Result<ToolResult, McpError> {
        match self {
            Ok(response) => {
                let content = Content {
                    content_type: "application/json".to_string(),
                    body: serde_json::to_string(&response).unwrap_or_default(),
                };
                Ok(ToolResult {
                    success: true,
                    contents: vec![content],
                })
            }
            Err(error) => {
                let content = Content {
                    content_type: "application/json".to_string(),
                    body: serde_json::to_string(&error).unwrap_or_default(),
                };
                Ok(ToolResult {
                    success: false,
                    contents: vec![content],
                })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub content_type: String,
    pub body: String,
}

impl Content {
    pub fn text(content: impl ToString) -> Self {
        Self {
            content_type: "text/plain".to_string(),
            body: content.to_string(),
        }
    }
}
