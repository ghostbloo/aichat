use crate::{
    config::{Agent, Config, GlobalConfig},
    utils::*,
};

use anyhow::{anyhow, bail, Context, Result};
use futures::future::join_all;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tokio::task::JoinHandle;

#[cfg(windows)]
const PATH_SEP: &str = ";";
#[cfg(not(windows))]
const PATH_SEP: &str = ":";

type ToolJoinResult = (usize, ToolCall, Result<Value>);

pub async fn eval_tool_calls(
    config: &GlobalConfig,
    mut calls: Vec<ToolCall>,
) -> Result<Vec<ToolResult>> {
    let output: Vec<ToolResult> = vec![];
    if calls.is_empty() {
        return Ok(output);
    }
    calls = ToolCall::dedup(calls);
    if calls.is_empty() {
        bail!("The request was aborted because an infinite loop of function calls was detected.")
    }

    // Dependencies
    let functions = &config.read().functions;
    let agent = &config.read().agent;

    let mut results_map: HashMap<usize, ToolResult> = HashMap::new(); // To store results and reorder later
    let mut concurrent_tasks: Vec<JoinHandle<ToolJoinResult>> = vec![];

    for (index, call) in calls.into_iter().enumerate() {
        let call_config = ToolCallConfig::extract(&call.name, &functions, &agent)?;

        if call_config.concurrent {
            let task: JoinHandle<ToolJoinResult> = tokio::spawn(async move {
                let result = call.eval(call_config).await;
                (index, call, result)
            });
            concurrent_tasks.push(task);
        } else {
            let result = call.eval(call_config).await;
            results_map.insert(index, ToolResult::new_from_eval_result(call, result));
        }
    }

    // Wait for all concurrent tasks to complete
    let concurrent_results = join_all(concurrent_tasks).await;

    // Process results from concurrent tasks
    for join_result in concurrent_results {
        match join_result {
            Ok((index, call, eval_result)) => {
                results_map.insert(index, ToolResult::new_from_eval_result(call, eval_result));
            }
            Err(e) => {
                bail!("A concurrent tool call task failed: {}", e);
            }
        }
    }

    // Reconstruct the output vector in the original order
    let mut final_output = Vec::with_capacity(results_map.len());
    for i in 0..results_map.len() {
        if let Some(result) = results_map.remove(&i) {
            final_output.push(result);
        } else {
            // This shouldn't happen if logic is correct
            bail!("Failed to reconstruct tool call results in order");
        }
    }

    let is_all_null = final_output
        .iter()
        .all(|tr| tr.output.is_null() || tr.output == json!("DONE"));
    if is_all_null {
        final_output = vec![];
    }
    Ok(final_output)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolResult {
    pub call: ToolCall,
    pub output: Value,
}

impl ToolResult {
    pub fn new(call: ToolCall, output: Value) -> Self {
        Self { call, output }
    }

    pub fn new_from_eval_result(call: ToolCall, eval_result: Result<Value>) -> Self {
        let output = match eval_result {
            Ok(result) => {
                if result.is_null() {
                    json!("DONE")
                } else {
                    result
                }
            }
            Err(e) => {
                json!({
                    "error": true,
                    "message": e.to_string()
                })
            }
        };
        Self::new(call, output)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Functions {
    declarations: Vec<FunctionDeclaration>,
}

impl Functions {
    pub fn init(declarations_path: &Path) -> Result<Self> {
        let declarations = load_declarations(declarations_path)?;

        Ok(Self { declarations })
    }

    pub fn find(&self, name: &str) -> Option<&FunctionDeclaration> {
        self.declarations.iter().find(|v| v.name == name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.declarations.iter().any(|v| v.name == name)
    }

    pub fn declarations(&self) -> &[FunctionDeclaration] {
        &self.declarations
    }

    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }
}

/// Loads function declarations from a file.
pub fn load_declarations(path: &Path) -> Result<Vec<FunctionDeclaration>> {
    let declarations: Vec<FunctionDeclaration> = if path.exists() {
        let ctx = || format!("Failed to load functions at {}", path.display());
        let content = fs::read_to_string(path).with_context(ctx)?;
        serde_json::from_str(&content).with_context(ctx)?
    } else {
        vec![]
    };
    Ok(declarations)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchema,
    #[serde(skip_serializing, default)]
    pub agent: bool,
    #[serde(default)]
    pub allow_concurrency: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
    #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<JsonSchema>>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_value: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl JsonSchema {
    pub fn is_empty_properties(&self) -> bool {
        match &self.properties {
            Some(v) => v.is_empty(),
            None => true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub id: Option<String>,
}

pub struct ToolCallConfig {
    pub name: String,
    pub cmd: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
    pub concurrent: bool,
}

impl ToolCallConfig {
    pub fn extract(
        function_name: &str,
        functions: &Functions,
        agent: &Option<Agent>,
    ) -> Result<Self> {
        if let Some(agent) = agent {
            if let Some(function) = agent.functions().find(&function_name) {
                if function.agent {
                    let config = Self::from_agent(function, agent);
                    if let Some(config) = config {
                        return Ok(config);
                    }
                }
            }
        }
        let function = functions
            .find(&function_name)
            .ok_or(anyhow!("Function not found: {function_name}"))?;
        Ok(Self::from_declaration(function))
    }

    pub fn from_declaration(function: &FunctionDeclaration) -> Self {
        Self {
            name: function.name.clone(),
            cmd: function.name.clone(),
            args: vec![],
            envs: Default::default(),
            concurrent: function.allow_concurrency,
        }
    }

    pub fn from_agent(function: &FunctionDeclaration, agent: &Agent) -> Option<Self> {
        if !function.agent {
            return None;
        }
        Some(Self {
            name: format!("{}-{}", agent.name(), &function.name),
            cmd: agent.name().to_string(),
            args: vec![function.name.clone()],
            envs: agent.variable_envs(),
            concurrent: function.allow_concurrency,
        })
    }
}

impl ToolCall {
    pub fn dedup(calls: Vec<Self>) -> Vec<Self> {
        let mut new_calls = vec![];
        let mut seen_ids = HashSet::new();

        for call in calls.into_iter().rev() {
            if let Some(id) = &call.id {
                if !seen_ids.contains(id) {
                    seen_ids.insert(id.clone());
                    new_calls.push(call);
                }
            } else {
                new_calls.push(call);
            }
        }

        new_calls.reverse();
        new_calls
    }

    pub fn new(name: String, arguments: Value, id: Option<String>) -> Self {
        Self {
            name,
            arguments,
            id,
        }
    }

    pub async fn eval(&self, config: ToolCallConfig) -> Result<Value> {
        let call_name = config.name;
        let cmd_name = config.cmd;
        let mut cmd_args = config.args;
        let envs = config.envs;

        let json_data = if self.arguments.is_object() {
            self.arguments.clone()
        } else if let Some(arguments) = self.arguments.as_str() {
            let arguments: Value = serde_json::from_str(arguments).map_err(|_| {
                anyhow!("The call '{call_name}' has invalid arguments: {arguments}")
            })?;
            arguments
        } else {
            bail!("The call '{call_name}' has invalid arguments: {}", self.arguments);
        };

        cmd_args.push(json_data.to_string());

        let output = match run_llm_function(cmd_name, cmd_args, envs)? {
            Some(contents) => serde_json::from_str(&contents)
                .ok()
                .unwrap_or_else(|| json!({"output": contents})),
            None => Value::Null,
        };

        Ok(output)
    }
}

/// Execute a local function.
pub fn run_llm_function(
    cmd_name: String,
    cmd_args: Vec<String>,
    mut envs: HashMap<String, String>,
) -> Result<Option<String>> {
    let prompt = format!("Call {cmd_name} {}", cmd_args.join(" "));

    let mut bin_dirs: Vec<PathBuf> = vec![];
    if cmd_args.len() > 1 {
        let dir = Config::agent_functions_dir(&cmd_name).join("bin");
        if dir.exists() {
            bin_dirs.push(dir);
        }
    }
    bin_dirs.push(Config::functions_bin_dir());
    let current_path = std::env::var("PATH").context("No PATH environment variable")?;
    let prepend_path = bin_dirs
        .iter()
        .map(|v| format!("{}{PATH_SEP}", v.display()))
        .collect::<Vec<_>>()
        .join("");
    envs.insert("PATH".into(), format!("{prepend_path}{current_path}"));

    let temp_file = temp_file("-eval-", "");
    envs.insert("LLM_OUTPUT".into(), temp_file.display().to_string());

    #[cfg(windows)]
    let cmd_name = polyfill_cmd_name(&cmd_name, &bin_dirs);
    if *IS_STDOUT_TERMINAL {
        println!("{}", dimmed_text(&prompt));
    }
    let (success, stdout, stderr) = run_command_with_output(&cmd_name, &cmd_args, Some(envs))
        .map_err(|err| anyhow!("Unable to run {cmd_name}, {err}"))?;
    if !success {
        println!("error: tool call failed: {:?}", stderr);
        bail!(json!({
            "error": true,
            "stdout": stdout,
            "stderr": stderr
        }));
    }
    let mut output = None;
    if temp_file.exists() {
        let contents =
            fs::read_to_string(temp_file).context("Failed to retrieve tool call output")?;
        if !contents.is_empty() {
            output = Some(contents);
        }
    };
    Ok(output)
}

#[cfg(windows)]
fn polyfill_cmd_name<T: AsRef<Path>>(cmd_name: &str, bin_dir: &[T]) -> String {
    let cmd_name = cmd_name.to_string();
    if let Ok(exts) = std::env::var("PATHEXT") {
        for name in exts.split(';').map(|ext| format!("{cmd_name}{ext}")) {
            for dir in bin_dir {
                let path = dir.as_ref().join(&name);
                if path.exists() {
                    return name.to_string();
                }
            }
        }
    }
    cmd_name
}
