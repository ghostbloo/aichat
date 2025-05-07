use crate::{config::{AgentConfig, AgentDefinition, Config, Session}, function::load_declarations, utils::list_file_names};
use crate::serve::Server;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::Response;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;

const PLAYGROUND_HTML: &[u8] = include_bytes!("../../assets/playground.html");
const ARENA_HTML: &[u8] = include_bytes!("../../assets/arena.html");

pub fn playground_page() -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let res = Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(PLAYGROUND_HTML)).boxed())?;
    Ok(res)
}

pub fn arena_page() -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let res = Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(ARENA_HTML)).boxed())?;
    Ok(res)
}

pub fn list_models(server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let data = json!({ "data": server.models });
    json_response(&data.to_string())
}

pub fn list_roles(server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let data = json!({ "data": server.roles });
    json_response(&data.to_string())
}

pub fn list_sessions() -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let session_dir = &Config::config_dir().join("sessions");
    let sessions = list_file_names(session_dir, ".yaml");
    let data = json!({ "data": sessions });
    json_response(&data.to_string())
}

pub fn get_session(session_id: &str, server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let session_path = Config::config_dir()
        .join("sessions")
        .join(session_id)
        .with_extension("yaml");
    let session = Session::load(&server.config, session_id, &session_path)?;
    let data = json!({ "data": session });
    json_response(&data.to_string())
}

pub fn list_agents(server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let data = json!({ "data": server.agents });
    json_response(&data.to_string())
}

pub fn get_agent(name: &str, server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    if !server.agents.contains(&name.to_string()) {
        return Err(anyhow!("Agent not found"));
    }
    let config = AgentConfig::load(&Config::agent_config_file(name))?;
    let definition = AgentDefinition::load(&Config::agent_definition_file(name))?;
    let data = json!({
        "data": {
            "config": config,
            "definition": definition,
        }
    });
    json_response(&data.to_string())
}

pub fn get_agent_functions(name: &str, server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    if !server.agents.contains(&name.to_string()) {
        return Err(anyhow!("Agent not found"));
    }
    let functions_path = Config::agent_functions_dir(name).join("functions.json");
    let functions = load_declarations(&functions_path)?;
    let data = json!({ "data": functions });
    json_response(&data.to_string())
}

pub fn get_agent_sessions(name: &str) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let sessions = list_file_names(Config::agent_sessions_dir(name), ".yaml");
    let data = json!({ "data": sessions });
    json_response(&data.to_string())
}

pub fn get_agent_session(
    agent_name: &str,
    session_id: &str,
    server: Arc<Server>,
) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let session = Session::load(
        &server.config,
        session_id,
        &Config::agent_sessions_dir(agent_name)
            .join(session_id)
            .with_extension("yaml")
    )?;
    let data = json!({ "data": session });
    json_response(&data.to_string())
}

pub fn list_rags(server: Arc<Server>) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let data = json!({ "data": server.rags });
    json_response(&data.to_string())
}

fn json_response(data: &str) -> Result<Response<BoxBody<Bytes, Infallible>>> {
    let res = Response::builder()
        .header("Content-Type", "application/json; charset=utf-8")
        .body(Full::new(Bytes::from(data.to_string())).boxed())?;
    Ok(res)
}
