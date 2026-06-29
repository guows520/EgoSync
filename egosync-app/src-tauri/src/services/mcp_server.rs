use std::{collections::HashMap, process::Stdio};

use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use tokio::process::Command;

use crate::db::mcp_servers as db;
use crate::error::AppError;
use crate::models::mcp::{CreateMcpServerInput, McpServer, UpdateMcpServerInput};
use crate::services::agent_config::AgentConfigService;

const MCP_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

const SECRET_MARKERS: &[&str] = &[
    "token=",
    "api_key=",
    "api-key=",
    "apikey=",
    "secret=",
    "password=",
    "passwd=",
    "auth=",
    "bearer ",
    "--token",
    "--api-key",
    "--apikey",
    "--secret",
    "--password",
    "--auth",
    "?token=",
    "&token=",
    "?api_key=",
    "&api_key=",
    "?api-key=",
    "&api-key=",
    "?apikey=",
    "&apikey=",
    "?secret=",
    "&secret=",
    "?password=",
    "&password=",
    "?auth=",
    "&auth=",
];

pub async fn list_servers(pool: &SqlitePool) -> Result<Vec<McpServer>, AppError> {
    db::list_mcp_servers(pool).await
}

pub async fn list_servers_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<Vec<McpServer>, AppError> {
    db::list_mcp_servers_for_role(pool, role_id).await
}

pub async fn list_available_servers_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<Vec<McpServer>, AppError> {
    db::list_available_mcp_servers_for_role(pool, role_id).await
}

pub async fn create_server(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    input: CreateMcpServerInput,
) -> Result<McpServer, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();
    let server = McpServer {
        id,
        name: normalized_name(&input.name)?,
        server_type: normalized_server_type(&input.server_type)?,
        command_or_url: normalized_command_or_url(&input.command_or_url)?,
        env_refs: normalize_env_refs(input.env_refs.as_deref())?,
        description: {
            let description = input.description.unwrap_or_default().trim().to_string();
            ensure_no_secret_like(&description, "MCP server 说明")?;
            description
        },
        enabled: input.enabled,
        created_at: now.clone(),
        updated_at: now,
    };
    db::insert_mcp_server(pool, &server).await?;
    sync_enabled_mcp_to_opencode(pool, agent_config).await;
    Ok(server)
}

pub async fn update_server(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    id: &str,
    input: UpdateMcpServerInput,
) -> Result<McpServer, AppError> {
    let mut server = db::get_mcp_server(pool, id).await?;
    if let Some(name) = input.name {
        server.name = normalized_name(&name)?;
    }
    if let Some(server_type) = input.server_type {
        server.server_type = normalized_server_type(&server_type)?;
    }
    if let Some(command_or_url) = input.command_or_url {
        server.command_or_url = normalized_command_or_url(&command_or_url)?;
    }
    if let Some(env_refs) = input.env_refs.as_deref() {
        server.env_refs = normalize_env_refs(Some(env_refs))?;
    }
    if let Some(description) = input.description {
        let description = description.trim().to_string();
        ensure_no_secret_like(&description, "MCP server 说明")?;
        server.description = description;
    }
    if let Some(enabled) = input.enabled {
        server.enabled = enabled;
    }
    db::update_mcp_server(pool, &server).await?;
    let updated = db::get_mcp_server(pool, id).await?;
    sync_enabled_mcp_to_opencode(pool, agent_config).await;
    sync_all_role_agents(pool, agent_config).await;
    Ok(updated)
}

pub async fn delete_server(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    id: &str,
) -> Result<(), AppError> {
    db::delete_mcp_server(pool, id).await?;
    sync_enabled_mcp_to_opencode(pool, agent_config).await;
    sync_all_role_agents(pool, agent_config).await;
    Ok(())
}

pub async fn test_server(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let server = db::get_mcp_server(pool, id).await?;
    match normalized_mcp_server_type(server.server_type.as_str()).as_str() {
        "streamable_http" => test_streamable_http_server(&server).await,
        "sse" => test_sse_server(&server).await,
        "stdio" => test_command_server(&server.command_or_url).await,
        _ => Err(AppError::ValidationError("MCP server 类型不支持".to_string())),
    }
}

async fn test_streamable_http_server(server: &McpServer) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .timeout(MCP_TEST_TIMEOUT)
        .build()
        .map_err(|e| AppError::ValidationError(format!("MCP client 初始化失败: {}", e)))?;

    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "EgoSync", "version": env!("CARGO_PKG_VERSION") }
        }
    });
    let response = send_mcp_json_rpc(&client, server, &initialize).await?;
    let session_id = extract_mcp_session_id(&response);
    let initialize_value = parse_mcp_json_response(response).await?;
    ensure_mcp_success_response(&initialize_value, "initialize")?;

    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    send_mcp_json_rpc_with_session(&client, server, &initialized, session_id.as_deref()).await?;

    let tools_list = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    let response = send_mcp_json_rpc_with_session(&client, server, &tools_list, session_id.as_deref()).await?;
    let tools_value = parse_mcp_json_response(response).await?;
    ensure_mcp_success_response(&tools_value, "tools/list")?;
    if !tools_value
        .get("result")
        .and_then(|value| value.get("tools"))
        .and_then(Value::as_array)
        .is_some()
    {
        return Err(AppError::ValidationError(
            "MCP server tools/list 未返回工具列表，请检查服务是否完整支持 MCP 协议".to_string(),
        ));
    }
    Ok(())
}

async fn test_sse_server(server: &McpServer) -> Result<(), AppError> {
    let response = reqwest::Client::new()
        .get(&server.command_or_url)
        .header("Accept", "text/event-stream")
        .timeout(MCP_TEST_TIMEOUT)
        .send()
        .await
        .map_err(|_| AppError::ValidationError("MCP server 无法连接，请检查 URL 或网络状态".to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::ValidationError(format!(
            "MCP server 返回 HTTP {}，请检查服务配置",
            response.status().as_u16()
        )));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !content_type.contains("text/event-stream") {
        return Err(AppError::ValidationError(
            "MCP SSE 连接未返回 text/event-stream，请检查 URL 是否为 MCP SSE endpoint".to_string(),
        ));
    }
    Ok(())
}

async fn send_mcp_json_rpc(
    client: &reqwest::Client,
    server: &McpServer,
    body: &Value,
) -> Result<reqwest::Response, AppError> {
    send_mcp_json_rpc_with_session(client, server, body, None).await
}

async fn send_mcp_json_rpc_with_session(
    client: &reqwest::Client,
    server: &McpServer,
    body: &Value,
    session_id: Option<&str>,
) -> Result<reqwest::Response, AppError> {
    let mut request = client
        .post(&server.command_or_url)
        .header(reqwest::header::ACCEPT, "application/json, text/event-stream")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(body);
    for (key, value) in env_refs_to_actual_http_headers(&server.env_refs)? {
        request = request.header(key, value);
    }
    if let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) {
        request = request.header("Mcp-Session-Id", session_id);
    }
    let response = request
        .send()
        .await
        .map_err(|_| AppError::ValidationError("MCP server 协议请求失败，请检查 URL、网络或认证配置".to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::ValidationError(format!(
            "MCP server 协议请求返回 HTTP {}，请检查服务配置",
            response.status().as_u16()
        )));
    }
    Ok(response)
}

fn extract_mcp_session_id(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get("Mcp-Session-Id")
        .or_else(|| response.headers().get("mcp-session-id"))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

async fn parse_mcp_json_response(response: reqwest::Response) -> Result<Value, AppError> {
    let text = response
        .text()
        .await
        .map_err(|_| AppError::ValidationError("读取 MCP server 响应失败".to_string()))?;
    let body = text.trim();
    if body.is_empty() {
        return Ok(json!({}));
    }
    if body.starts_with("data:") || body.contains("\ndata:") {
        for line in body.lines() {
            let Some(data) = line.trim().strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            return serde_json::from_str::<Value>(data).map_err(|_| {
                AppError::ValidationError("MCP server 返回了无法解析的 SSE JSON 响应".to_string())
            });
        }
        return Ok(json!({}));
    }
    serde_json::from_str::<Value>(body).map_err(|_| {
        AppError::ValidationError("MCP server 返回了非 JSON 响应，请检查 URL 是否为 MCP endpoint".to_string())
    })
}

fn ensure_mcp_success_response(value: &Value, method: &str) -> Result<(), AppError> {
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("未知 MCP 协议错误");
        return Err(AppError::ValidationError(format!(
            "MCP server {} 返回错误: {}",
            method,
            message
        )));
    }
    if value.get("result").is_none() && method != "notifications/initialized" {
        return Err(AppError::ValidationError(format!(
            "MCP server {} 未返回 result，请检查服务是否完整支持 MCP 协议",
            method
        )));
    }
    Ok(())
}

fn shell_command(command: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut shell = Command::new("cmd");
        shell.arg("/C").arg(command);
        shell
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut shell = Command::new("sh");
        shell.arg("-c").arg(command);
        shell
    }
}

async fn test_command_server(command: &str) -> Result<(), AppError> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("MCP command 不能为空".to_string()));
    }

    let mut child = shell_command(trimmed)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            AppError::ValidationError(format!(
                "MCP command 无法启动，请检查命令是否存在或依赖是否可用: {}",
                e
            ))
        })?;

    match tokio::time::timeout(std::time::Duration::from_secs(2), child.wait()).await {
        Ok(Ok(status)) if status.success() => Err(AppError::ValidationError(
            "MCP command 启动后立即退出，请检查它是否能长期运行".to_string(),
        )),
        Ok(Ok(status)) => Err(AppError::ValidationError(format!(
            "MCP command 启动失败（退出码 {}），请检查命令配置",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ))),
        Ok(Err(e)) => Err(AppError::ValidationError(format!(
            "等待 MCP command 进程状态失败: {}",
            e
        ))),
        Err(_) => {
            let _ = child.kill().await;
            Ok(())
        }
    }
}

pub async fn add_to_role(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    role_id: &str,
    server_id: &str,
) -> Result<(), AppError> {
    let server = db::get_mcp_server(pool, server_id).await?;
    if !server.enabled {
        return Err(AppError::ValidationError("该 MCP server 已全局停用，不能添加到角色".to_string()));
    }
    db::add_mcp_server_to_role(pool, role_id, server_id).await?;
    sync_role_agent(pool, agent_config, role_id).await;
    Ok(())
}

pub async fn remove_from_role(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    role_id: &str,
    server_id: &str,
) -> Result<(), AppError> {
    db::remove_mcp_server_from_role(pool, role_id, server_id).await?;
    sync_role_agent(pool, agent_config, role_id).await;
    Ok(())
}

pub async fn role_mcp_prompt_map(
    pool: &SqlitePool,
) -> Result<HashMap<String, Vec<String>>, AppError> {
    let roles = crate::db::roles::list_all_roles(pool).await?;
    let mut map = HashMap::new();
    for role in roles {
        let lines = db::role_enabled_mcp_lines(pool, &role.id).await?;
        if !lines.is_empty() {
            map.insert(role.id, lines);
        }
    }
    Ok(map)
}

pub async fn sync_role_agent_with_mcp(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    role: &crate::models::role::Role,
    registry: &[crate::models::skill::SkillRegistryEntry],
) -> Result<(), AppError> {
    let mcp_lines = db::role_enabled_mcp_lines(pool, &role.id).await?;
    agent_config.sync_role_updated_with_skills_and_mcp(role, registry, &mcp_lines)
}

pub async fn sync_mcp_scope_for_role(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
    role_id: Option<&str>,
) -> Result<(), AppError> {
    let servers = match role_id {
        Some(role_id) => db::list_mcp_servers_for_role(pool, role_id).await?,
        None => db::list_mcp_servers(pool).await?,
    };
    agent_config.sync_external_mcp_servers(&servers)
}

pub async fn mcp_scope_key_for_role(
    pool: &SqlitePool,
    role_id: Option<&str>,
) -> Result<String, AppError> {
    let mut servers = match role_id {
        Some(role_id) => db::list_mcp_servers_for_role(pool, role_id).await?,
        None => db::list_mcp_servers(pool).await?,
    }
    .into_iter()
    .filter(|server| server.enabled)
    .map(|server| {
        format!(
            "{}:{}:{}:{}:{}:{}",
            server.id,
            normalized_mcp_server_type(&server.server_type),
            server.command_or_url,
            server.env_refs,
            server.name,
            server.updated_at
        )
    })
    .collect::<Vec<_>>();
    servers.sort();
    let scope_owner = role_id
        .map(|id| format!("role:{}", id))
        .unwrap_or_else(|| "butler".to_string());
    Ok(format!("{}|{}", scope_owner, servers.join(",")))
}

pub async fn sync_enabled_mcp_to_opencode(
    pool: &SqlitePool,
    agent_config: &AgentConfigService,
) {
    match db::list_mcp_servers(pool).await {
        Ok(servers) => {
            if let Err(e) = agent_config.sync_external_mcp_servers(&servers) {
                tracing::warn!("sync MCP servers to opencode.json failed: {}", e);
            }
        }
        Err(e) => tracing::warn!("load MCP servers for opencode sync failed: {}", e),
    }
}

async fn sync_role_agent(pool: &SqlitePool, agent_config: &AgentConfigService, role_id: &str) {
    let result: Result<(), AppError> = (|| async {
        let role = crate::db::roles::get_role(pool, role_id).await?;
        let registry = crate::db::skills::list_skills(pool).await.unwrap_or_default();
        let mcp_lines = db::role_enabled_mcp_lines(pool, role_id).await?;
        agent_config.sync_role_updated_with_skills_and_mcp(&role, &registry, &mcp_lines)
    })()
    .await;
    if let Err(e) = result {
        tracing::warn!("sync role MCP prompt failed: {}", e);
    }
}

pub async fn sync_all_role_agents(pool: &SqlitePool, agent_config: &AgentConfigService) {
    let result: Result<(), AppError> = (|| async {
        let roles = crate::db::roles::list_all_roles(pool).await?;
        let registry = crate::db::skills::list_skills(pool).await.unwrap_or_default();
        let butler_skills = crate::services::butler_config::get_butler_skills(pool).await?;
        let mcp_prompts = role_mcp_prompt_map(pool).await?;
        agent_config.full_sync_with_skills_and_mcp(&roles, &butler_skills, &registry, &mcp_prompts)
    })()
    .await;
    if let Err(e) = result {
        tracing::warn!("full role MCP prompt sync failed: {}", e);
    }
}

fn ensure_no_secret_like(value: &str, field: &str) -> Result<(), AppError> {
    let lowered = value.to_ascii_lowercase();
    let has_url_credentials = value
        .split_once("://")
        .and_then(|(_, rest)| rest.split('/').next())
        .is_some_and(|authority| authority.contains('@') && authority.contains(':'));
    if has_url_credentials
        || lowered.starts_with("sk-")
        || SECRET_MARKERS.iter().any(|marker| lowered.contains(marker))
        || value
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '/' | '?' | '&' | '=' | ':' | ',' | ';' | '"' | '\'' | '(' | ')'))
            .any(looks_like_bare_secret)
    {
        return Err(AppError::ValidationError(format!("{} 不能包含密钥或令牌，请改用 env: 引用", field)));
    }
    Ok(())
}

fn looks_like_bare_secret(token: &str) -> bool {
    let trimmed = token.trim_matches(|ch: char| matches!(ch, '-' | '_' | '.'));
    trimmed.len() >= 24
        && trimmed.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && trimmed.chars().any(|ch| ch.is_ascii_lowercase())
        && trimmed.chars().any(|ch| ch.is_ascii_uppercase())
}

fn normalized_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("MCP server 名称不能为空".to_string()));
    }
    ensure_no_secret_like(trimmed, "MCP server 名称")?;
    Ok(trimmed.to_string())
}

fn normalized_server_type(server_type: &str) -> Result<String, AppError> {
    let normalized = normalized_mcp_server_type(server_type);
    if matches!(normalized.as_str(), "sse" | "streamable_http" | "stdio") {
        Ok(normalized)
    } else {
        Err(AppError::ValidationError("MCP server 类型必须是 SSE、Streamable HTTP 或 stdio".to_string()))
    }
}

fn normalized_mcp_server_type(server_type: &str) -> String {
    match server_type.trim() {
        "http_sse" => "sse".to_string(),
        "command" => "stdio".to_string(),
        other => other.to_string(),
    }
}

pub fn mcp_server_type_label(server_type: &str) -> &'static str {
    match normalized_mcp_server_type(server_type).as_str() {
        "streamable_http" => "Streamable HTTP",
        "stdio" => "stdio",
        _ => "SSE",
    }
}

fn normalized_command_or_url(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("MCP server 连接参数不能为空".to_string()));
    }
    ensure_no_secret_like(trimmed, "MCP server 连接参数")?;
    Ok(trimmed.to_string())
}

fn normalize_env_refs(raw: Option<&str>) -> Result<String, AppError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok("{}".to_string());
    };
    let parsed: Value = serde_json::from_str(raw)
        .map_err(|_| AppError::ValidationError("环境变量引用必须是 JSON 对象".to_string()))?;
    let Some(object) = parsed.as_object() else {
        return Err(AppError::ValidationError("环境变量引用必须是 JSON 对象".to_string()));
    };
    for value in object.values() {
        let Some(reference) = value.as_str() else {
            return Err(AppError::ValidationError("secret 只能保存 env: 引用".to_string()));
        };
        let Some(name) = reference.strip_prefix("env:") else {
            return Err(AppError::ValidationError("secret 只能保存 env: 引用".to_string()));
        };
        if name.trim().is_empty() {
            return Err(AppError::ValidationError("env: 引用名称不能为空".to_string()));
        }
    }
    serde_json::to_string(&Value::Object(object.clone()))
        .map_err(|e| AppError::ValidationError(format!("环境变量引用序列化失败: {}", e)))
}

fn env_refs_to_actual_http_headers(raw: &str) -> Result<Vec<(reqwest::header::HeaderName, String)>, AppError> {
    let parsed = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}));
    let Some(object) = parsed.as_object() else {
        return Ok(Vec::new());
    };
    let mut headers = Vec::new();
    for (key, value) in object {
        let Some(reference) = value.as_str() else {
            continue;
        };
        let Some(env_name) = reference.strip_prefix("env:").map(str::trim).filter(|name| !name.is_empty()) else {
            continue;
        };
        let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
            .map_err(|_| AppError::ValidationError("MCP header 名称无效，请检查环境变量引用配置".to_string()))?;
        let header_value = std::env::var(env_name).map_err(|_| {
            AppError::ValidationError(format!("MCP header 环境变量 {} 未设置", env_name))
        })?;
        headers.push((header_name, header_value));
    }
    Ok(headers)
}

pub fn mcp_server_config_for_opencode(server: &McpServer) -> Value {
    let env = env_refs_to_opencode_env(&server.env_refs);
    let mut object = Map::new();
    match normalized_mcp_server_type(server.server_type.as_str()).as_str() {
        "stdio" => {
            object.insert("type".to_string(), json!("local"));
            object.insert("command".to_string(), json!(split_command(&server.command_or_url)));
            if !env.is_empty() {
                object.insert("environment".to_string(), Value::Object(env));
            }
        }
        _ => {
            object.insert("type".to_string(), json!("remote"));
            object.insert("url".to_string(), json!(server.command_or_url));
            if !env.is_empty() {
                object.insert("headers".to_string(), Value::Object(env));
            }
        }
    }
    object.insert("managedByEgosync".to_string(), json!(true));
    Value::Object(object)
}

fn split_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in command.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn env_refs_to_opencode_env(raw: &str) -> Map<String, Value> {
    let parsed = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}));
    parsed
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .and_then(|reference| reference.strip_prefix("env:"))
                        .map(|name| (key.clone(), Value::String(format!("{{env:{}}}", name.trim()))))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn test_server_record(id: &str, name: &str) -> McpServer {
        McpServer {
            id: id.to_string(),
            name: name.to_string(),
            server_type: "sse".to_string(),
            command_or_url: format!("https://{}.example/mcp", id),
            env_refs: "{}".to_string(),
            description: String::new(),
            enabled: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT 'target',
                color TEXT NOT NULL DEFAULT '#4F46E5',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
                energy_updated_at TEXT,
                skills_config TEXT NOT NULL DEFAULT '{}',
                proactivity_level TEXT NOT NULL DEFAULT 'moderate',
                archived_at TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            );
            CREATE TABLE mcp_servers (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                server_type TEXT NOT NULL,
                command_or_url TEXT NOT NULL,
                env_refs TEXT NOT NULL DEFAULT '{}',
                description TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            );
            CREATE TABLE role_mcp_server_bindings (
                server_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                PRIMARY KEY (server_id, role_id)
            );
            INSERT INTO roles (id, name) VALUES ('role-1', '产品经理');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn sync_mcp_scope_for_role_exposes_only_role_bound_servers() {
        let pool = setup_test_db().await;
        db::insert_mcp_server(&pool, &test_server_record("calendar", "日历"))
            .await
            .unwrap();
        db::insert_mcp_server(&pool, &test_server_record("mail", "邮件"))
            .await
            .unwrap();
        db::add_mcp_server_to_role(&pool, "role-1", "calendar")
            .await
            .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(&path, serde_json::to_string_pretty(&json!({ "mcp": {} })).unwrap()).unwrap();
        let agent_config = AgentConfigService::new(path.clone());

        sync_mcp_scope_for_role(&pool, &agent_config, Some("role-1"))
            .await
            .unwrap();

        let scoped = agent_config.load().unwrap();
        assert!(scoped["mcp"].get("日历").is_some());
        assert!(scoped["mcp"].get("calendar").is_none());
        assert!(scoped["mcp"].get("mail").is_none());

        sync_mcp_scope_for_role(&pool, &agent_config, None)
            .await
            .unwrap();

        let global = agent_config.load().unwrap();
        assert!(global["mcp"].get("日历").is_some());
        assert!(global["mcp"].get("calendar").is_none());
        assert!(global["mcp"].get("邮件").is_some());
        assert!(global["mcp"].get("mail").is_none());
    }

    #[tokio::test]
    async fn mcp_scope_key_changes_when_role_bindings_change() {
        let pool = setup_test_db().await;
        db::insert_mcp_server(&pool, &test_server_record("calendar", "日历"))
            .await
            .unwrap();
        db::insert_mcp_server(&pool, &test_server_record("mail", "邮件"))
            .await
            .unwrap();
        db::add_mcp_server_to_role(&pool, "role-1", "calendar")
            .await
            .unwrap();

        let calendar_scope = mcp_scope_key_for_role(&pool, Some("role-1")).await.unwrap();
        db::add_mcp_server_to_role(&pool, "role-1", "mail")
            .await
            .unwrap();
        let calendar_mail_scope = mcp_scope_key_for_role(&pool, Some("role-1")).await.unwrap();

        assert_ne!(calendar_scope, calendar_mail_scope);
        assert!(calendar_scope.starts_with("role:role-1|"));
        assert!(calendar_scope.contains("calendar"));
        assert!(!calendar_scope.contains("mail"));
        assert!(calendar_mail_scope.contains("calendar"));
        assert!(calendar_mail_scope.contains("mail"));
    }

    #[tokio::test]
    async fn mcp_scope_key_changes_when_bound_server_config_changes() {
        let pool = setup_test_db().await;
        let mut calendar = test_server_record("calendar", "日历");
        db::insert_mcp_server(&pool, &calendar).await.unwrap();
        db::add_mcp_server_to_role(&pool, "role-1", "calendar").await.unwrap();

        let original_scope = mcp_scope_key_for_role(&pool, Some("role-1")).await.unwrap();
        calendar.command_or_url = "https://calendar.example/updated-mcp".to_string();
        db::update_mcp_server(&pool, &calendar).await.unwrap();
        let updated_scope = mcp_scope_key_for_role(&pool, Some("role-1")).await.unwrap();

        assert_ne!(original_scope, updated_scope);
        assert!(updated_scope.contains("calendar"));
    }

    #[tokio::test]
    async fn sync_role_agent_with_mcp_preserves_role_prompt_lines() {
        let pool = setup_test_db().await;
        db::insert_mcp_server(&pool, &test_server_record("calendar", "日历"))
            .await
            .unwrap();
        db::add_mcp_server_to_role(&pool, "role-1", "calendar")
            .await
            .unwrap();

        let role = crate::db::roles::get_role(&pool, "role-1").await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(&path, serde_json::to_string_pretty(&json!({ "agent": {} })).unwrap()).unwrap();
        let agent_config = AgentConfigService::new(path.clone());

        sync_role_agent_with_mcp(&pool, &agent_config, &role, &[])
            .await
            .unwrap();

        let config = agent_config.load().unwrap();
        let prompt = config["agent"]["role-role-1"]["prompt"].as_str().unwrap();
        assert!(prompt.contains("[外部 MCP 工具]"));
        assert!(prompt.contains("日历"));
    }

    async fn spawn_plain_http_200_server() -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body = "ok";
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        format!("http://{}/mcp", addr)
    }

    #[tokio::test]
    async fn test_server_rejects_plain_http_200_without_mcp_handshake() {
        let pool = setup_test_db().await;
        let mut server = test_server_record("plain-http", "普通 HTTP");
        server.server_type = "streamable_http".to_string();
        server.command_or_url = spawn_plain_http_200_server().await;
        db::insert_mcp_server(&pool, &server).await.unwrap();

        let result = test_server(&pool, "plain-http").await;

        assert!(matches!(result, Err(AppError::ValidationError(message)) if message.contains("MCP")));
    }

    #[tokio::test]
    async fn test_server_accepts_long_running_command_mcp() {
        let pool = setup_test_db().await;
        let mut server = test_server_record("long-running-command", "长运行命令");
        server.server_type = "stdio".to_string();
        #[cfg(target_os = "windows")]
        {
            server.command_or_url = "powershell -NoProfile -Command Start-Sleep -Seconds 5".to_string();
        }
        #[cfg(not(target_os = "windows"))]
        {
            server.command_or_url = "sh -c 'sleep 5'".to_string();
        }
        db::insert_mcp_server(&pool, &server).await.unwrap();

        let result = test_server(&pool, "long-running-command").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_rejects_missing_command_mcp() {
        let pool = setup_test_db().await;
        let mut server = test_server_record("missing-command", "缺失命令");
        server.server_type = "stdio".to_string();
        server.command_or_url = "egosync-missing-mcp-command-for-test".to_string();
        db::insert_mcp_server(&pool, &server).await.unwrap();

        let result = test_server(&pool, "missing-command").await;

        assert!(matches!(result, Err(AppError::ValidationError(message)) if message.contains("无法启动") || message.contains("启动失败")));
    }

    #[test]
    fn rejects_secret_like_free_text_fields() {
        assert!(matches!(normalized_command_or_url("https://example.com/sse?token=sk-plain"), Err(AppError::ValidationError(message)) if message.contains("连接参数")));
        assert!(matches!(normalized_name("sk-plain"), Err(AppError::ValidationError(message)) if message.contains("名称")));
        assert!(matches!(ensure_no_secret_like("token=sk-plain", "MCP server 说明"), Err(AppError::ValidationError(message)) if message.contains("说明")));
    }

    #[tokio::test]
    async fn update_server_rejects_secret_like_free_text_fields_without_persisting() {
        let pool = setup_test_db().await;
        db::insert_mcp_server(&pool, &test_server_record("calendar", "日历"))
            .await
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let agent_config = AgentConfigService::new(dir.path().join("opencode.json"));

        let result = update_server(
            &pool,
            &agent_config,
            "calendar",
            UpdateMcpServerInput {
                name: Some("sk-plain".to_string()),
                server_type: None,
                command_or_url: None,
                env_refs: None,
                description: None,
                enabled: None,
            },
        )
        .await;
        assert!(matches!(result, Err(AppError::ValidationError(message)) if message.contains("名称")));

        let result = update_server(
            &pool,
            &agent_config,
            "calendar",
            UpdateMcpServerInput {
                name: None,
                server_type: None,
                command_or_url: Some("https://example.com/sse?token=sk-plain".to_string()),
                env_refs: None,
                description: None,
                enabled: None,
            },
        )
        .await;
        assert!(matches!(result, Err(AppError::ValidationError(message)) if message.contains("连接参数")));

        let result = update_server(
            &pool,
            &agent_config,
            "calendar",
            UpdateMcpServerInput {
                name: None,
                server_type: None,
                command_or_url: None,
                env_refs: None,
                description: Some("token=sk-plain".to_string()),
                enabled: None,
            },
        )
        .await;
        assert!(matches!(result, Err(AppError::ValidationError(message)) if message.contains("说明")));

        let stored = db::get_mcp_server(&pool, "calendar").await.unwrap();
        assert_eq!(stored.name, "日历");
        assert_eq!(stored.command_or_url, "https://calendar.example/mcp");
        assert_eq!(stored.description, "");
    }

    #[test]
    fn rejects_plain_secret_values() {
        let result = normalize_env_refs(Some(r#"{"TOKEN":"sk-plain"}"#));
        assert!(matches!(result, Err(AppError::ValidationError(message)) if message.contains("secret 只能保存")));
    }

    #[test]
    fn rejects_keyring_and_empty_secret_refs() {
        assert!(matches!(normalize_env_refs(Some(r#"{"TOKEN":"keyring:calendar-token"}"#)), Err(AppError::ValidationError(message)) if message.contains("env:")));
        assert!(matches!(normalize_env_refs(Some(r#"{"TOKEN":"env:"}"#)), Err(AppError::ValidationError(message)) if message.contains("不能为空")));
        assert!(matches!(normalize_env_refs(Some(r#"{"TOKEN":"env:   "}"#)), Err(AppError::ValidationError(message)) if message.contains("不能为空")));
    }

    #[test]
    fn rejects_secret_like_url_and_command_tokens() {
        assert!(matches!(normalized_command_or_url("https://user:pass@example.com/sse"), Err(AppError::ValidationError(message)) if message.contains("连接参数")));
        assert!(matches!(normalized_command_or_url("https://example.com/mcp/abc1234567890abcdefABCDEF"), Err(AppError::ValidationError(message)) if message.contains("连接参数")));
        assert!(matches!(normalized_command_or_url("npx server --token abc1234567890abcdefABCDEF"), Err(AppError::ValidationError(message)) if message.contains("连接参数")));
    }

    #[test]
    fn maps_env_refs_without_plain_secret_values() {
        let server = McpServer {
            id: "mcp-1".to_string(),
            name: "日历".to_string(),
            server_type: "http_sse".to_string(),
            command_or_url: "https://calendar.example/sse".to_string(),
            env_refs: r#"{"TOKEN":"env:CALENDAR_TOKEN"}"#.to_string(),
            description: String::new(),
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let config = mcp_server_config_for_opencode(&server);
        assert_eq!(config["type"], "remote");
        assert_eq!(config["url"], "https://calendar.example/sse");
        assert_eq!(config["headers"]["TOKEN"], "{env:CALENDAR_TOKEN}");
        assert!(config.get("environment").is_none());
        assert!(config.get("env").is_none());
        assert!(!serde_json::to_string(&config).unwrap().contains("sk-"));
    }

    #[test]
    fn normalizes_standard_mcp_types_and_legacy_aliases() {
        assert_eq!(normalized_server_type("sse").unwrap(), "sse");
        assert_eq!(normalized_server_type("streamable_http").unwrap(), "streamable_http");
        assert_eq!(normalized_server_type("stdio").unwrap(), "stdio");
        assert_eq!(normalized_server_type("http_sse").unwrap(), "sse");
        assert_eq!(normalized_server_type("command").unwrap(), "stdio");
    }

    #[test]
    fn maps_standard_mcp_types_to_opencode_config() {
        let mut remote = test_server_record("weather", "天气");
        remote.server_type = "streamable_http".to_string();
        let remote_config = mcp_server_config_for_opencode(&remote);
        assert_eq!(remote_config["type"], "remote");
        assert_eq!(remote_config["url"], "https://weather.example/mcp");

        let mut local = test_server_record("filesystem", "文件系统");
        local.server_type = "stdio".to_string();
        local.command_or_url = "npx -y @modelcontextprotocol/server-filesystem".to_string();
        let local_config = mcp_server_config_for_opencode(&local);
        assert_eq!(local_config["type"], "local");
        assert_eq!(local_config["command"], json!(["npx", "-y", "@modelcontextprotocol/server-filesystem"]));
    }

    #[tokio::test]
    async fn role_prompt_uses_standard_mcp_type_label() {
        let pool = setup_test_db().await;
        let mut server = test_server_record("weather", "天气");
        server.server_type = "http_sse".to_string();
        server.description = "查询天气".to_string();
        db::insert_mcp_server(&pool, &server).await.unwrap();
        db::add_mcp_server_to_role(&pool, "role-1", "weather").await.unwrap();

        let lines = db::role_enabled_mcp_lines(&pool, "role-1").await.unwrap();

        assert_eq!(lines, vec!["- 天气（SSE）：查询天气"]);
    }

    #[test]
    fn command_mcp_uses_opencode_command_array_and_environment() {
        let server = McpServer {
            id: "mcp-local".to_string(),
            name: "文件系统".to_string(),
            server_type: "command".to_string(),
            command_or_url: "npx -y @modelcontextprotocol/server-filesystem".to_string(),
            env_refs: r#"{"ROOT":"env:FILES_ROOT"}"#.to_string(),
            description: String::new(),
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let config = mcp_server_config_for_opencode(&server);
        assert_eq!(config["type"], "local");
        assert_eq!(config["command"], json!(["npx", "-y", "@modelcontextprotocol/server-filesystem"]));
        assert_eq!(config["environment"]["ROOT"], "{env:FILES_ROOT}");
        assert!(config.get("env").is_none());
    }
}
