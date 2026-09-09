use std::collections::HashMap;
use std::sync::Arc;

use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::db::{conversations, memories, tasks};
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::{
    ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent, ToolCall, ToolDefinition,
};
use crate::models::chat::{RoleProposedPayload, StreamPayload, STREAM_PHASE_ANSWERING, STREAM_PHASE_DONE, STREAM_PHASE_PROCESS, STREAM_PHASE_THINKING, STREAM_PHASE_TOOL};
use crate::commands::chat::OpencodeSessionState;
use crate::models::role::CreateRoleInput;
use crate::models::task::{CreateTaskInput, TaskOwnerType};
use crate::models::task_decomposition::{
    CreateTaskDecompositionProposalInput, TaskDecompositionItem,
};
use crate::services::secret_store;

const OPENCODE_FALLBACK_NOTICE: &str = "Agent 引擎暂时不可用，当前为基础对话模式。\n\n";
const OPENCODE_TOOL_EXECUTION_TIMEOUT: Duration = Duration::from_secs(60);
const OPENCODE_SKILL_LOAD_TIMEOUT: Duration = Duration::from_secs(15);

/// Resolve the opencode project directory to a stable path outside the dev
/// project tree. Using `"."` previously caused opencode to write `.opencode/`
/// session files into the tauri/vite source directory, triggering cargo-watch
/// or vite HMR full-page reloads during active sessions.
fn resolve_opencode_project_dir(app_handle: &tauri::AppHandle) -> String {
    use tauri::Manager;
    if let Ok(dir) = app_handle.path().app_data_dir() {
        let workspace = dir.join("opencode-workspace");
        let _ = std::fs::create_dir_all(&workspace);
        workspace.to_string_lossy().to_string()
    } else {
        // Fallback: use user home to avoid polluting source tree
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".egosync-workspace")
            .to_string_lossy()
            .to_string()
    }
}

fn resolve_requested_working_directory(
    app_handle: &tauri::AppHandle,
    working_directory: Option<&str>,
) -> Result<String, AppError> {
    let Some(raw_dir) = working_directory.map(str::trim).filter(|dir| !dir.is_empty()) else {
        return Ok(resolve_opencode_project_dir(app_handle));
    };
    let path = std::path::PathBuf::from(raw_dir);
    if !path.is_dir() {
        return Err(AppError::ValidationError(format!(
            "工作目录不存在或不是文件夹: {}",
            raw_dir
        )));
    }
    path.canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| AppError::ValidationError(format!("工作目录不可访问: {}", e)))
}

fn opencode_agent_key(role_id: Option<&str>) -> String {
    match role_id {
        Some(id) => crate::services::agent_config::AgentConfigService::role_to_agent_key(id),
        None => "butler".to_string(), // butler agent has egosync* tools enabled
    }
}

fn opencode_session_cache_key(working_directory: &str, mcp_scope_key: &str) -> String {
    format!("{}\n{}", working_directory, mcp_scope_key)
}

fn remember_opencode_session_for_directory(
    sessions: &mut std::collections::HashMap<String, OpencodeSessionState>,
    conversation_id: &str,
    session_id: &str,
    working_directory: &str,
    mcp_scope_key: &str,
) -> String {
    let state = sessions.entry(conversation_id.to_string()).or_default();
    let cache_key = opencode_session_cache_key(working_directory, mcp_scope_key);
    let remembered = state
        .sessions_by_directory
        .entry(cache_key)
        .or_insert_with(|| session_id.to_string())
        .clone();
    state.active_session_id = remembered.clone();
    remembered
}

fn stream_payload_from_sse(
    conversation_id: &str,
    message_id: Option<&str>,
    event: crate::models::agent::SseEvent,
) -> Option<StreamPayload> {
    match event {
        crate::models::agent::SseEvent::Text { content } => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: content,
            done: false,
            thinking: false,
            message_id: message_id.map(str::to_string),
            phase: None,
            status_text: None,
            tool_name: None,
            process_event: None,
        }),
        crate::models::agent::SseEvent::Thinking { content } => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: content,
            done: false,
            thinking: true,
            message_id: message_id.map(str::to_string),
            phase: Some(STREAM_PHASE_THINKING.to_string()),
            status_text: Some("思考中...".to_string()),
            tool_name: None,
            process_event: None,
        }),
        crate::models::agent::SseEvent::Done => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: String::new(),
            done: true,
            thinking: false,
            message_id: message_id.map(str::to_string),
            phase: None,
            status_text: None,
            tool_name: None,
            process_event: None,
        }),
        crate::models::agent::SseEvent::Error { message } => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: format!("抱歉，Agent 引擎返回错误：{}", summarize_error(&message)),
            done: true,
            thinking: false,
            message_id: message_id.map(str::to_string),
            phase: None,
            status_text: None,
            tool_name: None,
            process_event: None,
        }),
        crate::models::agent::SseEvent::ToolCall { name, arguments } => {
            tracing::info!(
                "[opencode] tool_call received: name={} args_len={}",
                name,
                arguments.len()
            );
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BusTextDeltaKind {
    Text,
    Thinking,
    WaitForPartType,
}

fn is_thinking_bus_part_type(part_type: &str) -> bool {
    matches!(part_type, "reasoning" | "thinking")
}

fn bus_delta_part_type(properties: &serde_json::Value) -> Option<&str> {
    properties
        .get("partType")
        .or_else(|| properties.get("part_type"))
        .and_then(|value| value.as_str())
        .or_else(|| {
            properties
                .get("part")
                .and_then(|part| part.get("type"))
                .and_then(|value| value.as_str())
        })
}

fn classify_bus_text_delta(
    properties: &serde_json::Value,
    visible_text_parts: &std::collections::HashSet<String>,
    reasoning_parts: &std::collections::HashSet<String>,
) -> BusTextDeltaKind {
    let part_id = properties
        .get("partID")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if !part_id.is_empty() && reasoning_parts.contains(part_id) {
        return BusTextDeltaKind::Thinking;
    }
    if let Some(part_type) = bus_delta_part_type(properties) {
        if is_thinking_bus_part_type(part_type) {
            return BusTextDeltaKind::Thinking;
        }
        if part_type == "text" {
            return BusTextDeltaKind::Text;
        }
    }
    if !part_id.is_empty() && visible_text_parts.contains(part_id) {
        return BusTextDeltaKind::Text;
    }
    BusTextDeltaKind::WaitForPartType
}

fn emit_stream_token(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    message_id: Option<&str>,
    token: &str,
    thinking: bool,
) {
    let _ = app_handle.emit(
        "llm:stream",
        StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: token.to_string(),
            done: false,
            thinking,
            message_id: message_id.map(str::to_string),
            phase: Some((if thinking { STREAM_PHASE_THINKING } else { STREAM_PHASE_ANSWERING }).to_string()),
            status_text: if thinking {
                Some("思考中...".to_string())
            } else {
                None
            },
            tool_name: None,
            process_event: None,
        },
    );
}

fn display_tool_name(tool_name: &str) -> String {
    let trimmed = tool_name.trim();
    let Some((server, tool)) = trimmed.split_once('_') else {
        return trimmed.to_string();
    };
    if server.is_empty() || tool.is_empty() || is_builtin_underscore_tool(trimmed) {
        return trimmed.to_string();
    }
    if server.contains('-')
        || tool.contains('-')
        || server.chars().any(|ch| !ch.is_ascii_alphanumeric())
    {
        format!("{}:{}", server, tool)
    } else {
        trimmed.to_string()
    }
}

fn is_builtin_underscore_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "create_role" | "delegate_to_role" | "record_emergence_rejection"
        | "create_task" | "complete_task" | "delete_task"
    )
}

fn tool_status_text(tool_name: &str, status: &str) -> Option<String> {
    let display_name = display_tool_name(tool_name);
    let trimmed = display_name.trim();
    match status {
        "running" => Some(if trimmed.is_empty() {
            "正在使用工具...".to_string()
        } else {
            format!("正在使用 {}...", trimmed)
        }),
        "completed" => Some(if trimmed.is_empty() {
            "工具已完成，正在整理结果...".to_string()
        } else {
            format!("{} 已完成，正在整理结果...", trimmed)
        }),
        _ => None,
    }
}

fn emit_tool_status(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    tool_name: &str,
    status: &str,
) {
    let Some(status_text) = tool_status_text(tool_name, status) else {
        return;
    };
    let _ = app_handle.emit(
        "llm:stream",
        StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: String::new(),
            done: false,
            thinking: false,
            message_id: None,
            phase: Some(STREAM_PHASE_TOOL.to_string()),
            status_text: Some(status_text),
            tool_name: {
                let display_name = display_tool_name(tool_name);
                if display_name.trim().is_empty() {
                    None
                } else {
                    Some(display_name)
                }
            },
            process_event: None,
        },
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpSessionRetryDecision {
    DoNotRetry,
    RefreshRuntimeAndRetry,
}

fn mcp_session_retry_decision(
    candidate: &ProcessEventCandidate,
    already_retried: bool,
) -> McpSessionRetryDecision {
    if already_retried || !matches!(candidate.status.as_deref(), Some("failed" | "error")) {
        return McpSessionRetryDecision::DoNotRetry;
    }
    let raw = candidate.raw_json.to_string().to_ascii_lowercase();
    if raw.contains("invalid session id") || raw.contains("mcp-session-id") || raw.contains("session id") && raw.contains("invalid") {
        McpSessionRetryDecision::RefreshRuntimeAndRetry
    } else {
        McpSessionRetryDecision::DoNotRetry
    }
}

#[derive(Debug, Clone)]
struct ProcessEventCandidate {
    event_type: String,
    tool_name: Option<String>,
    status: Option<String>,
    summary: String,
    raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
struct McpToolDisplayMap {
    namespace_to_name: HashMap<String, String>,
}

impl McpToolDisplayMap {
    fn from_servers(servers: &[crate::models::mcp::McpServer]) -> Self {
        let mut namespace_to_name = HashMap::new();
        for server in servers.iter().filter(|server| server.enabled) {
            let display_name = server.name.trim();
            if display_name.is_empty() {
                continue;
            }
            for namespace in [
                server.id.trim().to_string(),
                display_name.to_string(),
                opencode_safe_tool_namespace(display_name),
            ] {
                if !namespace.is_empty() {
                    namespace_to_name.insert(namespace, display_name.to_string());
                }
            }
        }
        Self { namespace_to_name }
    }

    fn display_name_and_tool_for_raw_tool<'a>(&'a self, raw_tool: &'a str) -> Option<(&'a str, &'a str)> {
        let trimmed = raw_tool.trim();
        self.namespace_to_name
            .iter()
            .filter_map(|(namespace, display_name)| {
                let prefix = format!("{}_", namespace);
                let tool = trimmed.strip_prefix(&prefix)?;
                if tool.is_empty() {
                    return None;
                }
                Some((namespace.len(), display_name.as_str(), tool))
            })
            .max_by_key(|(namespace_len, _, _)| *namespace_len)
            .map(|(_, display_name, tool)| (display_name, tool))
    }

    fn display_name_for_namespace(&self, namespace: &str) -> Option<&str> {
        self.namespace_to_name.get(namespace.trim()).map(String::as_str)
    }
}

fn opencode_safe_tool_namespace(namespace: &str) -> String {
    namespace
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn display_tool_name_with_map(tool_name: &str, display_map: &McpToolDisplayMap) -> String {
    let trimmed = tool_name.trim();
    if let Some((display_name, tool)) = display_map.display_name_and_tool_for_raw_tool(trimmed) {
        return format!("{}:{}", display_name, tool);
    }
    let Some((server, tool)) = trimmed.split_once('_') else {
        return trimmed.to_string();
    };
    if server.is_empty() || tool.is_empty() || is_builtin_underscore_tool(trimmed) {
        return trimmed.to_string();
    }
    if let Some(display_name) = display_map.display_name_for_namespace(server) {
        return format!("{}:{}", display_name, tool);
    }
    display_tool_name(trimmed)
}

fn process_tool_summary_with_display_map(
    tool_name: &str,
    status: &str,
    display_map: &McpToolDisplayMap,
) -> String {
    if let Some(text) = tool_status_text_with_display_map(tool_name, status, display_map) {
        return text;
    }
    let display_name = display_tool_name_with_map(tool_name, display_map);
    let trimmed = display_name.trim();
    match status {
        "failed" | "error" => {
            if trimmed.is_empty() { "工具执行失败".to_string() } else { format!("{} 失败", trimmed) }
        }
        "cancelled" | "canceled" => {
            if trimmed.is_empty() { "工具已取消".to_string() } else { format!("{} 已取消", trimmed) }
        }
        _ => {
            if trimmed.is_empty() { "工具状态已更新".to_string() } else { format!("{} 状态已更新", trimmed) }
        }
    }
}

fn tool_status_text_with_display_map(
    tool_name: &str,
    status: &str,
    display_map: &McpToolDisplayMap,
) -> Option<String> {
    let display_name = display_tool_name_with_map(tool_name, display_map);
    let trimmed = display_name.trim();
    match status {
        "running" => Some(if trimmed.is_empty() {
            "正在使用工具...".to_string()
        } else {
            format!("正在使用 {}...", trimmed)
        }),
        "completed" => Some(if trimmed.is_empty() {
            "工具已完成，正在整理结果...".to_string()
        } else {
            format!("{} 已完成，正在整理结果...", trimmed)
        }),
        _ => None,
    }
}

fn build_tool_process_event_candidate_with_display_map(
    part_raw: &serde_json::Value,
    display_map: &McpToolDisplayMap,
) -> Option<ProcessEventCandidate> {
    build_tool_process_event_candidate_inner(part_raw, Some(display_map))
}

#[derive(Debug, Clone)]
struct ToolExecutionDeadline {
    part_id: String,
    tool_name: String,
    deadline: Instant,
    timeout: Duration,
    retryable_skill_load: bool,
}

fn update_tool_execution_deadline(
    active: &mut Option<ToolExecutionDeadline>,
    part_id: &str,
    tool_name: &str,
    status: &str,
    retryable_skill_load: bool,
    now: Instant,
) {
    match status {
        "running" => {
            if active.as_ref().map(|current| current.part_id.as_str()) != Some(part_id) {
                let timeout = if retryable_skill_load {
                    OPENCODE_SKILL_LOAD_TIMEOUT
                } else {
                    OPENCODE_TOOL_EXECUTION_TIMEOUT
                };
                *active = Some(ToolExecutionDeadline {
                    part_id: part_id.to_string(),
                    tool_name: tool_name.to_string(),
                    deadline: now + timeout,
                    timeout,
                    retryable_skill_load,
                });
            }
        }
        "completed" | "failed" | "error" | "cancelled" | "canceled" => {
            if active.as_ref().map(|current| current.part_id.as_str()) == Some(part_id) {
                *active = None;
            }
        }
        _ => {}
    }
}

fn should_retry_skill_load_timeout(retryable_skill_load: bool, already_retried: bool) -> bool {
    retryable_skill_load && !already_retried
}

fn tool_execution_timeout_candidate(tool_name: &str, timeout: Duration) -> ProcessEventCandidate {
    ProcessEventCandidate {
        event_type: "tool".to_string(),
        tool_name: Some(tool_name.to_string()),
        status: Some("failed".to_string()),
        summary: format!("{} 加载超时，会话已终止", tool_name),
        raw_json: serde_json::json!({
            "tool": tool_name,
            "status": "failed",
            "error": "tool execution timeout",
            "timeoutSeconds": timeout.as_secs(),
        }),
    }
}

fn tool_execution_timeout_message(tool_name: &str) -> String {
    if tool_name.eq_ignore_ascii_case("find-skills") {
        format!(
            "抱歉，{} 加载超时，Agent 会话已终止。请检查 Skill 安装路径是否重复，或稍后重试。",
            tool_name
        )
    } else {
        format!(
            "抱歉，{} 执行超时，Agent 会话已终止，请稍后重试。",
            tool_name
        )
    }
}

fn process_tool_summary(tool_name: &str, status: &str) -> String {
    if let Some(text) = tool_status_text(tool_name, status) {
        return text;
    }
    let display_name = display_tool_name(tool_name);
    let trimmed = display_name.trim();
    match status {
        "failed" | "error" => {
            if trimmed.is_empty() { "工具执行失败".to_string() } else { format!("{} 失败", trimmed) }
        }
        "cancelled" | "canceled" => {
            if trimmed.is_empty() { "工具已取消".to_string() } else { format!("{} 已取消", trimmed) }
        }
        _ => {
            if trimmed.is_empty() { "工具状态已更新".to_string() } else { format!("{} 状态已更新", trimmed) }
        }
    }
}

fn first_json_value<'a>(raw: &'a serde_json::Value, paths: &[&[&str]]) -> Option<&'a serde_json::Value> {
    for path in paths {
        let mut current = raw;
        let mut found = true;
        for key in *path {
            match current.get(*key) {
                Some(next) => current = next,
                None => {
                    found = false;
                    break;
                }
            }
        }
        if found && !current.is_null() {
            return Some(current);
        }
    }
    None
}

fn extract_json_string(raw: &serde_json::Value, paths: &[&[&str]]) -> Option<String> {
    first_json_value(raw, paths).and_then(|value| {
        value.as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn live_tool_display_name(tool_name: &str, part_raw: &serde_json::Value, fallback: &str) -> String {
    if tool_name.eq_ignore_ascii_case("skill") {
        return extract_json_string(
            part_raw,
            &[
                &["input", "name"],
                &["state", "input", "name"],
                &["toolInvocation", "args", "name"],
            ],
        )
        .unwrap_or_else(|| fallback.to_string());
    }
    fallback.to_string()
}

fn normalize_read_input(tool_name: Option<&str>, input: Option<serde_json::Value>) -> Option<serde_json::Value> {
    let Some(tool_name) = tool_name else { return input };
    if !tool_name.eq_ignore_ascii_case("read") {
        return input;
    }
    let Some(mut input) = input else { return None };
    let serde_json::Value::Object(map) = &mut input else { return Some(input) };
    if map.get("file_path").is_some() {
        return Some(input);
    }
    for key in ["filePath", "filepath", "file_name", "filename", "path", "file", "target"] {
        if let Some(value) = map.get(key).cloned() {
            map.insert("file_path".to_string(), value);
            break;
        }
    }
    Some(input)
}

fn build_tool_process_event_candidate(part_raw: &serde_json::Value) -> Option<ProcessEventCandidate> {
    build_tool_process_event_candidate_inner(part_raw, None)
}

fn build_tool_process_event_candidate_inner(
    part_raw: &serde_json::Value,
    display_map: Option<&McpToolDisplayMap>,
) -> Option<ProcessEventCandidate> {
    let tool_name = extract_json_string(part_raw, &[&["tool"], &["toolInvocation", "toolName"], &["toolInvocation", "name"]]);
    let status = extract_json_string(part_raw, &[&["state", "status"], &["status"]])?;
    if !matches!(status.as_str(), "running" | "completed" | "failed" | "error" | "cancelled" | "canceled") {
        return None;
    }

    let command = extract_json_string(part_raw, &[
        &["command"],
        &["state", "input", "command"],
        &["input", "command"],
        &["toolInvocation", "args", "command"],
    ]);
    let input = normalize_read_input(tool_name.as_deref(), first_json_value(part_raw, &[
        &["input"],
        &["state", "input"],
        &["toolInvocation", "args"],
    ]).cloned());
    let output = first_json_value(part_raw, &[
        &["output"],
        &["state", "output"],
        &["toolInvocation", "result", "output"],
        &["state", "result", "output"],
    ]).cloned();
    let error = first_json_value(part_raw, &[
        &["error"],
        &["state", "error"],
        &["stderr"],
        &["state", "stderr"],
        &["error", "message"],
    ]).cloned();

    let raw_tool_name = tool_name.as_deref().unwrap_or("");
    let summary = match display_map {
        Some(display_map) => process_tool_summary_with_display_map(raw_tool_name, &status, display_map),
        None => process_tool_summary(raw_tool_name, &status),
    };
    let display_tool_name = tool_name.as_deref().map(|tool_name| match display_map {
        Some(display_map) => display_tool_name_with_map(tool_name, display_map),
        None => display_tool_name(tool_name),
    });
    let mut normalized = serde_json::Map::new();
    if let Some(tool_name) = &display_tool_name {
        normalized.insert("tool".to_string(), serde_json::Value::String(tool_name.clone()));
    }
    normalized.insert("status".to_string(), serde_json::Value::String(status.clone()));
    if let Some(command) = command {
        normalized.insert("command".to_string(), serde_json::Value::String(command));
    }
    if let Some(input) = input {
        normalized.insert("input".to_string(), input);
    }
    if let Some(output) = output {
        normalized.insert("output".to_string(), output);
    }
    if let Some(error) = error {
        normalized.insert("error".to_string(), error);
    }
    normalized.insert("rawPart".to_string(), part_raw.clone());

    Some(ProcessEventCandidate {
        event_type: "tool".to_string(),
        tool_name: display_tool_name,
        status: Some(status.clone()),
        summary,
        raw_json: serde_json::Value::Object(normalized),
    })
}

fn build_narration_process_event_candidate(part_raw: &serde_json::Value) -> Option<ProcessEventCandidate> {
    let part_type = extract_json_string(part_raw, &[&["type"], &["partType"], &["part_type"]]);
    if !matches!(part_type.as_deref(), Some("text")) {
        return None;
    }

    let text = extract_json_string(part_raw, &[&["text"]])?;
    let mut normalized = serde_json::Map::new();
    normalized.insert("text".to_string(), serde_json::Value::String(text.clone()));
    normalized.insert("rawPart".to_string(), part_raw.clone());

    Some(ProcessEventCandidate {
        event_type: "narration".to_string(),
        tool_name: None,
        status: None,
        summary: text,
        raw_json: serde_json::Value::Object(normalized),
    })
}

fn remove_recorded_narration_prefixes(text: &mut String, narrations: &[String]) {
    for narration in narrations {
        let prefix = narration.trim();
        if prefix.is_empty() {
            continue;
        }
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            *text = rest.trim_start().to_string();
        }
    }
}

fn buffer_narration_delta(text: &mut String, has_tool_process_event: &mut bool, delta: &str) {
    if delta.is_empty() {
        return;
    }
    text.push_str(delta);
    if text.trim().is_empty() {
        text.clear();
    }
    let _ = has_tool_process_event;
}

fn should_flush_final_narration(_has_tool_process_event: bool, _text: &str) -> bool {
    false
}

async fn persist_process_event(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    message_id: &str,
    opencode_session_id: &str,
    event_type: &str,
    tool_name: Option<&str>,
    status: Option<&str>,
    summary: &str,
    raw_json: &serde_json::Value,
    working_directory: Option<&str>,
) {
    let raw_json = serde_json::to_string(raw_json).unwrap_or_else(|_| "{}".to_string());
    if let Err(err) = conversations::insert_message_process_event(
        conv_pool,
        conversation_id,
        message_id,
        opencode_session_id,
        event_type,
        tool_name,
        status,
        summary,
        &raw_json,
        working_directory,
    )
    .await
    {
        tracing::warn!(conversation_id, message_id, error = %err, "persist process event failed");
    }
}

fn process_event_payload(
    conversation_id: &str,
    message_id: &str,
    opencode_session_id: &str,
    candidate: &ProcessEventCandidate,
    working_directory: Option<&str>,
) -> crate::models::chat::MessageProcessEvent {
    crate::models::chat::MessageProcessEvent {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        message_id: message_id.to_string(),
        opencode_session_id: opencode_session_id.to_string(),
        event_type: candidate.event_type.clone(),
        tool_name: candidate.tool_name.clone(),
        status: candidate.status.clone(),
        summary: candidate.summary.clone(),
        raw_json: serde_json::to_string(&candidate.raw_json).unwrap_or_else(|_| "{}".to_string()),
        working_directory: working_directory.map(str::to_string),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn emit_process_event(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    process_event: crate::models::chat::MessageProcessEvent,
) {
    let _ = app_handle.emit(
        "llm:stream",
        StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: String::new(),
            done: false,
            thinking: false,
            message_id: None,
            phase: Some(STREAM_PHASE_PROCESS.to_string()),
            status_text: None,
            tool_name: process_event.tool_name.clone(),
            process_event: Some(process_event),
        },
    );
}

async fn persist_and_emit_process_event(
    app_handle: &tauri::AppHandle,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    message_id: &str,
    opencode_session_id: &str,
    candidate: &ProcessEventCandidate,
    working_directory: Option<&str>,
) {
    emit_process_event(
        app_handle,
        conversation_id,
        process_event_payload(conversation_id, message_id, opencode_session_id, candidate, working_directory),
    );
    persist_process_event(
        conv_pool,
        conversation_id,
        message_id,
        opencode_session_id,
        &candidate.event_type,
        candidate.tool_name.as_deref(),
        candidate.status.as_deref(),
        &candidate.summary,
        &candidate.raw_json,
        working_directory,
    )
    .await;
}

fn completed_thinking_elapsed_seconds(started_at: Option<Instant>) -> u64 {
    started_at
        .map(|started| started.elapsed().as_secs())
        .unwrap_or(0)
        .max(1)
}

async fn flush_thinking_process_event(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    message_id: &str,
    opencode_session_id: &str,
    text: &mut String,
    started_at: &mut Option<Instant>,
    working_directory: Option<&str>,
) {
    let summary = text.trim().to_string();
    if summary.is_empty() {
        text.clear();
        *started_at = None;
        return;
    }
    let elapsed_seconds = completed_thinking_elapsed_seconds(*started_at);
    let candidate = ProcessEventCandidate {
        event_type: "thinking".to_string(),
        tool_name: None,
        status: Some("completed".to_string()),
        summary: summary.clone(),
        raw_json: serde_json::json!({ "content": summary, "elapsedSeconds": elapsed_seconds }),
    };
    // The live UI already owns a synthetic running thinking block. Persist the
    // completed segment here without emitting a second live process event, which
    // would duplicate that block before the next tool/text event.
    persist_process_event(
        conv_pool,
        conversation_id,
        message_id,
        opencode_session_id,
        &candidate.event_type,
        candidate.tool_name.as_deref(),
        candidate.status.as_deref(),
        &candidate.summary,
        &candidate.raw_json,
        working_directory,
    )
    .await;
    text.clear();
    *started_at = None;
}

async fn flush_narration_process_event(
    app_handle: &tauri::AppHandle,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    message_id: &str,
    opencode_session_id: &str,
    text: &mut String,
    working_directory: Option<&str>,
) -> Option<String> {
    let summary = text.trim().to_string();
    if summary.is_empty() {
        return None;
    }
    let part_raw = serde_json::json!({
        "type": "text",
        "text": summary,
    });
    text.clear();
    if let Some(candidate) = build_narration_process_event_candidate(&part_raw) {
        persist_and_emit_process_event(
            app_handle,
            conv_pool,
            conversation_id,
            message_id,
            opencode_session_id,
            &candidate,
            working_directory,
        )
        .await;
        return Some(candidate.summary);
    }
    None
}

fn emit_stream_done(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    message_id: Option<&str>,
) {
    let _ = app_handle.emit(
        "llm:stream",
        StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: String::new(),
            done: true,
            thinking: false,
            message_id: message_id.map(str::to_string),
            phase: Some(STREAM_PHASE_DONE.to_string()),
            status_text: None,
            tool_name: None,
            process_event: None,
        },
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetaSkillConfigOwner {
    Butler,
    Role,
}

fn meta_skill_discovery_disabled_message(owner: MetaSkillConfigOwner) -> &'static str {
    match owner {
        MetaSkillConfigOwner::Butler => {
            "当前没有启用 Skill 发现能力，需要在管家的 Skill 配置中开启后，我才能帮你查找或推荐 Skill。"
        }
        MetaSkillConfigOwner::Role => {
            "当前角色没有启用 Skill 发现能力，需要在该角色的 Skill 配置中开启后，我才能帮你查找或推荐 Skill。"
        }
    }
}

fn meta_skill_creator_disabled_message(owner: MetaSkillConfigOwner) -> &'static str {
    match owner {
        MetaSkillConfigOwner::Butler => {
            "当前没有启用 Skill 创建能力，需要在管家的 Skill 配置中开启后，我才能帮你创建或扩展 Skill。"
        }
        MetaSkillConfigOwner::Role => {
            "当前角色没有启用 Skill 创建能力，需要在该角色的 Skill 配置中开启后，我才能帮你创建或扩展 Skill。"
        }
    }
}

fn looks_like_meta_skill_discovery_request(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    if !normalized.contains("skill") && !normalized.contains("技能") {
        return false;
    }

    const DISCOVERY_MARKERS: &[&str] = &[
        "搜",
        "搜索",
        "找",
        "查",
        "查找",
        "发现",
        "推荐",
        "有哪些",
        "有什么",
        "list",
        "search",
        "find",
        "discover",
        "recommend",
    ];
    DISCOVERY_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn looks_like_meta_skill_creator_request(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    if !normalized.contains("skill") && !normalized.contains("技能") {
        return false;
    }

    const CREATOR_MARKERS: &[&str] = &[
        "创建",
        "新建",
        "生成",
        "扩展",
        "开发",
        "写一个",
        "做一个",
        "create",
        "generate",
        "build",
        "extend",
    ];
    CREATOR_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn disabled_meta_skill_message(
    find_skills: bool,
    skill_creator: bool,
    user_message: &str,
    owner: MetaSkillConfigOwner,
) -> Option<&'static str> {
    if !find_skills && looks_like_meta_skill_discovery_request(user_message) {
        Some(meta_skill_discovery_disabled_message(owner))
    } else if !skill_creator && looks_like_meta_skill_creator_request(user_message) {
        Some(meta_skill_creator_disabled_message(owner))
    } else {
        None
    }
}

fn append_completed_tail(
    app_handle: Option<&tauri::AppHandle>,
    conversation_id: &str,
    message_id: Option<&str>,
    target: &mut String,
    completed_text: &str,
    thinking: bool,
) {
    if completed_text.trim().is_empty() {
        return;
    }

    if target.trim().is_empty() {
        target.push_str(completed_text);
        if let Some(app_handle) = app_handle {
            emit_stream_token(
                app_handle,
                conversation_id,
                message_id,
                completed_text,
                thinking,
            );
        }
        return;
    }

    if completed_text.len() > target.len() && completed_text.starts_with(target.as_str()) {
        let delta = &completed_text[target.len()..];
        target.push_str(delta);
        if let Some(app_handle) = app_handle {
            emit_stream_token(app_handle, conversation_id, message_id, delta, thinking);
        }
    }
}

fn append_completed_followup_tail(
    app_handle: Option<&tauri::AppHandle>,
    conversation_id: &str,
    message_id: Option<&str>,
    target: &mut String,
    first_bubble_text: &str,
    completed_text: &str,
) {
    let tail = completed_text
        .strip_prefix(first_bubble_text)
        .map(str::trim_start)
        .unwrap_or(completed_text);
    append_completed_tail(app_handle, conversation_id, message_id, target, tail, false);
}

fn apply_completed_message_fallback(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    message_id: Option<&str>,
    completed: Option<crate::models::agent::OpencodeCompletedMessage>,
    text_target: &mut String,
    thinking_target: &mut String,
) {
    apply_completed_followup_message_fallback(
        Some(app_handle),
        conversation_id,
        message_id,
        completed,
        None,
        text_target,
        thinking_target,
    );
}

fn apply_completed_followup_message_fallback(
    app_handle: Option<&tauri::AppHandle>,
    conversation_id: &str,
    message_id: Option<&str>,
    completed: Option<crate::models::agent::OpencodeCompletedMessage>,
    first_bubble_text: Option<&str>,
    text_target: &mut String,
    thinking_target: &mut String,
) {
    let Some(completed) = completed else {
        return;
    };

    append_completed_tail(
        app_handle,
        conversation_id,
        None,
        thinking_target,
        &completed.thinking,
        true,
    );

    if let Some(first_bubble_text) = first_bubble_text {
        append_completed_followup_tail(
            app_handle,
            conversation_id,
            message_id,
            text_target,
            first_bubble_text,
            &completed.text,
        );
    } else {
        append_completed_tail(
            app_handle,
            conversation_id,
            message_id,
            text_target,
            &completed.text,
            false,
        );
    }
}

fn ensure_non_empty_opencode_result(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    message_id: Option<&str>,
    text: &mut String,
    thinking: &str,
) {
    if text.trim().is_empty() && thinking.trim().is_empty() {
        let fallback = "抱歉，这次没有生成可显示的回复，请再试一次。";
        text.push_str(fallback);
        emit_stream_token(app_handle, conversation_id, message_id, fallback, false);
    }
}

const BUTLER_SYSTEM_PROMPT: &str = "\
你是数字分身管家，用户的私人助理和生活协调者。\
你的语调稳重、可靠、有温度，像一位值得信赖的英式管家。\
你帮助用户管理角色、任务和日程，但决定权永远在用户手中。\
用简洁自然的中文回复，不用 emoji。";

const TRANSPARENCY_UNCERTAINTY_RULES: &str = "\
[透明推理与不确定性规则]\n\
- 可以基于[已知记忆]回答普通问题，但用户没有明确要求来源、依据、原文或你怎么知道时，不要主动展示记忆标签或内部链接。\n\
- 只有用户明确询问为什么、依据是什么、你怎么知道的、来源或原文时，才使用本 prompt 中的记忆内部链接 `[[记忆#YYYY/MM/DD HH:mm]](egosync-memory://memory-id)` 标注来源；用户只会看到时间标签。\n\
- 溯源回答必须原样输出本 prompt 中已有的记忆内部链接，不要删掉链接地址，不要把来源改写成自然语言时间（例如“你在 2026/06/02 11:15 告诉我”）。\n\
- 溯源回答要输出简洁依据链：记忆、规则、历史模式 → 建议。\n\
- 只能引用本 prompt 中出现的记忆内部链接；不得编造记忆标签，不得使用列表序号或内部 ID 冒充来源。\n\
- 没有可引用记忆时，明确说明“我现在没有可溯源的记忆依据”。\n\
- 不确定时主动声明，不把推断写成确定事实。\n\
- 不暴露隐藏 chain-of-thought，只给依据链/证据链。";

const HISTORY_LIMIT: i64 = 20;

/// Story 2.3 AC-7: 跨角色全局视野。
/// 用户可能在角色 X 视图私聊后回管家，管家 system prompt 必须自带各 active 角色近况摘要，
/// 否则管家会"失忆"——它只看到自己 conversation 里的内容，看不到角色私聊。
/// 摘要为纯文本注入 prompt，对用户不可见。无 active 角色或全部无历史时返回空串。
const CROSS_ROLE_SUMMARY_PER_ROLE: i64 = 4;
const CROSS_ROLE_SUMMARY_PER_LINE_CHARS: usize = 40;
const CROSS_ROLE_SUMMARY_TOTAL_CHARS: usize = 2000;
const BUTLER_MEMORY_PER_ROLE: usize = 6;
const BUTLER_MEMORY_PER_LINE_CHARS: usize = 90;
const BUTLER_MEMORY_TOTAL_CHARS: usize = 3000;
const TASK_CONTEXT_VISIBLE_LIMIT: usize = 50;
const TASK_CONTEXT_TITLE_CHARS: usize = 80;

const MISSION_VALUE_BASIS_RULES: &str = "\
[使命与价值观依据]\n\
- 回答用户的身份、价值观或长期优先级问题时，用户明确设定的使命宣言是第一依据，行为资料只能作为佐证。\n\
- 如果行为资料与使命宣言不一致，应指出行为可能偏离使命或与使命存在冲突，不得用行为推断覆盖或改写用户明确设定的使命。";

fn format_structured_mission(content: &str) -> Result<Option<String>, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(content)?;
    let mut lines = Vec::new();

    if let Some(mission) = value.get("mission").and_then(serde_json::Value::as_str) {
        if !mission.trim().is_empty() {
            lines.push(format!("使命：{}", mission.trim()));
        }
    }
    let principle = value
        .get("principle")
        .or_else(|| value.get("value"))
        .and_then(serde_json::Value::as_str);
    if let Some(principle) = principle {
        if !principle.trim().is_empty() {
            lines.push(format!("原则：{}", principle.trim()));
        }
    }
    if let Some(roles) = value.get("roles").and_then(serde_json::Value::as_array) {
        let role_lines = roles
            .iter()
            .filter_map(|role| {
                let name = role
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .trim();
                let goal = role
                    .get("goal")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .trim();
                match (name.is_empty(), goal.is_empty()) {
                    (false, false) => Some(format!("- {}——{}", name, goal)),
                    (false, true) => Some(format!("- {}", name)),
                    (true, false) => Some(format!("- {}", goal)),
                    (true, true) => None,
                }
            })
            .collect::<Vec<_>>();
        if !role_lines.is_empty() {
            lines.push(format!("角色目标：\n{}", role_lines.join("\n")));
        }
    } else {
        let role_name = value
            .get("role")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        let role_goal = value
            .get("goal")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        if !role_name.is_empty() || !role_goal.is_empty() {
            let role_line = match (role_name.is_empty(), role_goal.is_empty()) {
                (false, false) => format!("- {}——{}", role_name, role_goal),
                (false, true) => format!("- {}", role_name),
                (true, false) => format!("- {}", role_goal),
                (true, true) => unreachable!(),
            };
            lines.push(format!("角色目标：\n{}", role_line));
        }
    }

    Ok((!lines.is_empty()).then(|| lines.join("\n")))
}

fn butler_values_without_mission_context() -> String {
    format!(
        "{}\n- 当前未设置使命宣言；只有这时才可依据行为资料推断价值观，并必须明确说明结论是基于当前行为的推断且存在不确定性。",
        MISSION_VALUE_BASIS_RULES
    )
}

async fn build_butler_mission_context(main_pool: &DbPool) -> Result<String, AppError> {
    let mission = crate::db::mission::get_mission(main_pool).await?;
    let Some(content) = mission
        .as_ref()
        .and_then(|mission| mission.content.as_deref())
        .map(str::trim)
        .filter(|content| !content.is_empty())
    else {
        return Ok(butler_values_without_mission_context());
    };

    let formatted = if mission
        .as_ref()
        .is_some_and(|mission| mission.format == "structured")
    {
        match format_structured_mission(content) {
            Ok(Some(formatted)) => formatted,
            Ok(None) => return Ok(butler_values_without_mission_context()),
            Err(error) => {
                tracing::warn!(%error, "使命宣言 structured 内容无法解析，保留原文注入管家上下文");
                content.to_string()
            }
        }
    } else {
        content.to_string()
    };

    Ok(format!(
        "{}\n[用户明确设定的使命宣言]\n{}",
        MISSION_VALUE_BASIS_RULES, formatted
    ))
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn task_status_label(task: &crate::models::task::Task) -> &'static str {
    if task.is_completed {
        "已完成"
    } else {
        "未完成"
    }
}

fn format_task_context_line(task: &crate::models::task::Task) -> String {
    let mut badges = vec![format!("[{}]", task_status_label(task)), format!("[{}]", task.quadrant)];
    if task.is_big_rock {
        badges.push("[大石头]".to_string());
    }
    if task.protection_status != "normal" {
        badges.push(format!("[{}]", task.protection_status));
    }
    let deadline = task
        .deadline
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!(" 截止:{}", value))
        .unwrap_or_default();
    format!(
        "- id={} {}{} {}",
        task.id,
        badges.join(""),
        deadline,
        truncate_chars(task.title.trim(), TASK_CONTEXT_TITLE_CHARS)
    )
}

fn select_task_context_items(tasks: &[crate::models::task::Task]) -> (Vec<&crate::models::task::Task>, usize) {
    let mut selected = tasks
        .iter()
        .filter(|task| !task.is_completed)
        .collect::<Vec<_>>();
    let remaining_slots = TASK_CONTEXT_VISIBLE_LIMIT.saturating_sub(selected.len());
    let completed = tasks
        .iter()
        .filter(|task| task.is_completed)
        .collect::<Vec<_>>();
    let omitted_completed_count = completed.len().saturating_sub(remaining_slots);
    selected.extend(completed.into_iter().take(remaining_slots));
    (selected, omitted_completed_count)
}

pub async fn build_role_task_summary(
    main_pool: &DbPool,
    role_id: &str,
) -> Result<String, AppError> {
    let role = crate::db::roles::get_role(main_pool, role_id).await?;
    let role_tasks = tasks::list_tasks_by_role(main_pool, role_id).await?;
    if role_tasks.is_empty() {
        return Ok(String::new());
    }

    let (selected, omitted_completed_count) = select_task_context_items(&role_tasks);
    let mut lines = selected
        .into_iter()
        .map(format_task_context_line)
        .collect::<Vec<_>>();
    if omitted_completed_count > 0 {
        lines.push(format!("- 另有 {} 条已完成任务未注入。", omitted_completed_count));
    }

    Ok(format!(
        "[当前角色任务]\n{}：\n{}\n规则：这是 EgoSync 内部任务列表；当用户询问任务、待办、安排或下一步时，优先依据本段回答，不要去工作目录寻找任务文件。未完成任务必须全部覆盖；已完成任务只在总量不超过 {} 条时补充。任务 ID 仅用于 complete_task / delete_task 工具参数，不得在自然语言回复中展示。",
        role.name,
        lines.join("\n"),
        TASK_CONTEXT_VISIBLE_LIMIT
    ))
}

pub async fn build_butler_task_summary(main_pool: &DbPool) -> Result<String, AppError> {
    let roles = crate::db::roles::list_all_roles(main_pool).await?;
    if roles.is_empty() {
        return Ok(String::new());
    }

    let mut blocks = Vec::new();
    for role in roles {
        let role_tasks = tasks::list_tasks_by_role(main_pool, &role.id).await?;
        if role_tasks.is_empty() {
            continue;
        }
        let (selected, omitted_completed_count) = select_task_context_items(&role_tasks);
        let mut lines = selected
            .into_iter()
            .map(format_task_context_line)
            .collect::<Vec<_>>();
        if omitted_completed_count > 0 {
            lines.push(format!("- 另有 {} 条已完成任务未注入。", omitted_completed_count));
        }
        blocks.push(format!("{}（{}）：\n{}", role.name, role.status, lines.join("\n")));
    }

    // 管家自己的通用任务
    let butler_tasks = tasks::list_butler_tasks(main_pool).await?;
    if !butler_tasks.is_empty() {
        let (selected, omitted_completed_count) = select_task_context_items(&butler_tasks);
        let mut lines = selected
            .into_iter()
            .map(format_task_context_line)
            .collect::<Vec<_>>();
        if omitted_completed_count > 0 {
            lines.push(format!("- 另有 {} 条已完成任务未注入。", omitted_completed_count));
        }
        blocks.push(format!("管家通用任务：\n{}", lines.join("\n")));
    }

    if blocks.is_empty() {
        return Ok(String::new());
    }

    Ok(format!(
        "[各角色任务]\n{}\n规则：这是 EgoSync 内部任务列表；当用户询问任一角色的任务、待办、安排或下一步时，优先依据本段回答，不要去工作目录寻找任务文件。每个角色未完成任务必须全部覆盖；已完成任务只在该角色总量不超过 {} 条时补充。任务 ID 仅用于 complete_task / delete_task 工具参数，不得在自然语言回复中展示。",
        blocks.join("\n"),
        TASK_CONTEXT_VISIBLE_LIMIT
    ))
}

fn format_memory_reference_label(memory: &crate::models::memory::Memory) -> String {
    chrono::DateTime::parse_from_rfc3339(&memory.created_at)
        .map(|created_at| {
            created_at
                .with_timezone(&chrono::Local)
                .format("%Y/%m/%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| memory.created_at.clone())
}

fn format_memory_reference_line(memory: &crate::models::memory::Memory) -> String {
    format!(
        "- [[记忆#{}]](egosync-memory://{}) [{}] {}",
        format_memory_reference_label(memory),
        memory.id,
        memory.category,
        truncate_chars(memory.content.trim(), BUTLER_MEMORY_PER_LINE_CHARS)
    )
}

pub async fn build_butler_memory_summary(main_pool: &DbPool) -> Result<String, AppError> {
    let all_memories = memories::list_all_memories(main_pool).await?;
    if all_memories.is_empty() {
        return Ok(String::new());
    }

    let roles = crate::db::roles::list_active_roles(main_pool).await?;
    let role_names = roles
        .into_iter()
        .map(|role| (role.id, role.name))
        .collect::<std::collections::HashMap<_, _>>();

    let mut global_lines = Vec::new();
    let mut role_lines = std::collections::BTreeMap::<String, Vec<String>>::new();
    for memory in all_memories {
        let line = format_memory_reference_line(&memory);
        if let Some(role_id) = memory.role_id {
            let role_name = role_names
                .get(&role_id)
                .cloned()
                .unwrap_or_else(|| role_id.clone());
            let lines = role_lines.entry(role_name).or_default();
            if lines.len() < BUTLER_MEMORY_PER_ROLE {
                lines.push(line);
            }
        } else if global_lines.len() < BUTLER_MEMORY_PER_ROLE {
            global_lines.push(line);
        }
    }

    let mut sections = Vec::new();
    if !global_lines.is_empty() {
        sections.push(format!("全局记忆：\n{}", global_lines.join("\n")));
    }
    for (role_name, lines) in role_lines {
        if !lines.is_empty() {
            sections.push(format!("{}：\n{}", role_name, lines.join("\n")));
        }
    }
    if sections.is_empty() {
        return Ok(String::new());
    }

    let mut summary = format!("[已知记忆]\n{}", sections.join("\n"));
    if summary.chars().count() > BUTLER_MEMORY_TOTAL_CHARS {
        summary = truncate_chars(&summary, BUTLER_MEMORY_TOTAL_CHARS) + "…";
    }
    Ok(summary)
}

pub async fn build_cross_role_summary(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
) -> Result<String, AppError> {
    let roles = crate::db::roles::list_active_roles(main_pool).await?;
    if roles.is_empty() {
        return Ok(String::new());
    }

    let mut blocks: Vec<String> = Vec::new();
    for role in &roles {
        // 找该角色最近一条 conversation；不存在直接跳过（不要 get_or_create，避免无意义建空会话）
        let convs =
            crate::db::conversations::list_conversations_by_role(conv_pool, &role.id).await?;
        let Some(conv) = convs.first() else { continue };
        let recent = crate::db::conversations::get_recent_messages(
            conv_pool,
            &conv.id,
            CROSS_ROLE_SUMMARY_PER_ROLE,
        )
        .await?;
        if recent.is_empty() {
            continue;
        }

        let mut lines: Vec<String> = Vec::with_capacity(recent.len() + 1);
        lines.push(format!("- {}：", role.name));
        for m in &recent {
            let speaker = match m.role.as_str() {
                "user" => "用户",
                "assistant" => "角色",
                _ => continue, // system 类内部消息（如 onboarding 锚点）不进摘要
            };
            let snippet = truncate_chars(&m.content, CROSS_ROLE_SUMMARY_PER_LINE_CHARS);
            lines.push(format!("  {}: {}", speaker, snippet));
        }
        blocks.push(lines.join("\n"));
    }

    if blocks.is_empty() {
        return Ok(String::new());
    }

    let mut joined = format!("[各角色近况]\n{}", blocks.join("\n"));
    if joined.chars().count() > CROSS_ROLE_SUMMARY_TOTAL_CHARS {
        joined = joined
            .chars()
            .take(CROSS_ROLE_SUMMARY_TOTAL_CHARS)
            .collect::<String>()
            + "…";
    }
    Ok(joined)
}

/// Build the full butler system prompt: baseline identity + role roster + cross-role summary
/// + delegation guidelines + emergence instructions. Used by both the direct LLM path and
/// the opencode path to ensure the butler persona is consistent regardless of backend.
pub async fn build_butler_system_prompt(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
) -> Result<String, AppError> {
    // Story 2.3: butler system prompt 拼接顺序：基线 + 可委派角色清单 + 各角色近况 + 行为指南。
    // 顺序固定，保证 LLM 先建立身份，再看到资源，最后被告诉怎么用资源。
    let mut system_prompt = String::from(BUTLER_SYSTEM_PROMPT);

    let mission_context = build_butler_mission_context(main_pool).await?;
    system_prompt.push_str("\n\n");
    system_prompt.push_str(&mission_context);

    let butler_skills = crate::services::butler_config::get_butler_skills(main_pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load butler Skill config for prompt: {}", e);
            crate::services::butler_config::default_butler_skills()
        });
    let skills_config = crate::services::butler_config::skills_config_json(&butler_skills);
    let meta_skill_prompt = crate::services::role_config::meta_skill_prompt(&skills_config);
    if !meta_skill_prompt.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&meta_skill_prompt);
    }
    if let Ok(registry) = crate::db::skills::list_skills(main_pool).await {
        let custom_skill_lines = butler_skills
            .enabled_skill_ids
            .iter()
            .filter_map(|id| {
                registry
                    .iter()
                    .find(|skill| skill.id == *id)
                    .map(|skill| format!("- {}：{}", skill.name, skill.description))
            })
            .collect::<Vec<_>>();
        if !custom_skill_lines.is_empty() {
            system_prompt.push_str("\n\n[自定义 Skill]\n");
            system_prompt.push_str(&custom_skill_lines.join("\n"));
            system_prompt.push_str("\n只能按以上 EgoSync 已导入 Skill 的说明行动，不要调用或列出外部环境中的其它 Skill。");
        }
    }

    let active_roles = crate::db::roles::list_active_roles(main_pool).await?;
    if !active_roles.is_empty() {
        system_prompt.push_str("\n\n[可委派角色清单]");
        for r in &active_roles {
            system_prompt.push_str(&format!(
                "\n- id={} | 名称={} | 目标={}",
                r.id,
                r.name,
                if r.goal.trim().is_empty() {
                    "（未设定）"
                } else {
                    r.goal.trim()
                }
            ));
        }
    }

    let butler_task_summary = build_butler_task_summary(main_pool).await?;
    if !butler_task_summary.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&butler_task_summary);
    }

    let memory_summary = build_butler_memory_summary(main_pool).await?;
    if !memory_summary.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&memory_summary);
    }

    system_prompt.push_str("\n\n");
    system_prompt.push_str(TRANSPARENCY_UNCERTAINTY_RULES);

    let cross_summary = build_cross_role_summary(conv_pool, main_pool).await?;
    if !cross_summary.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&cross_summary);
    }

    if !active_roles.is_empty() {
        // 行为指南只在有可委派角色时才有意义；零角色时不要诱导 LLM 调用工具。
        system_prompt.push_str(
            "\n\n[行为指南]\n\
            - 先判断用户意图类型：陈述事实/偏好、查询任务、操作任务（创建/完成/删除）、委派任务给角色处理。\n\
            - 如果用户只是陈述某个角色相关事实或偏好（例如孩子叫什么、喜欢什么、学习表现如何），直接用自然口吻确认，不要调用工具，不要解释系统会如何记录或同步。\n\
            - 当用户要求创建任务、标记任务完成、删除任务时，调用对应的 create_task / complete_task / delete_task 工具；用户陈述某项已有任务已完成、已提交或已做完，也视为完成操作。完成或删除已有任务时不得调用 delegate_to_role。task_id 和 role_id 必须是[各角色任务]或「可委派角色清单」中列出的 ID。\n\
            - 当用户询问任务进展、有哪些任务、需要关注什么时，直接基于[各角色任务]回答，不要调用 delegate_to_role。查询任务不等于委派任务。\n\
            - 如果用户交代的是某个角色相关任务、安排、日程、待办、规划或需要跟进的事项（例如家长会、约定、准备材料、制定练习计划），只要能匹配 active 角色，就调用 delegate_to_role。\n\
            - 不要向用户暴露内部机制：不要说“系统会同步”“同步到某角色”“角色已收到”“委派成功”“工具调用”等。\n\
            - 当用户只是询问已知事实（例如某个孩子喜欢什么、某个固定安排是什么）时，优先基于[已知记忆]回答，并可说明目前只知道这些。\n\
            - 当用户询问明确属性或偏好槽位（例如“喜欢吃什么”“喜欢什么运动”“某天安排是什么”）时，只回答与该问题槽位直接相关的记忆；不要因为同一角色或同一对象存在其它记忆，就补充阅读、学习、家庭日等无关事实。若没有直接相关记忆，只简短说明还不知道，不要列出其它领域记忆。\n\
            - 用户一句话同时涉及多个角色且需要角色处理时：可以在同一轮内调用多个 delegate_to_role（并行委派）。\n\
            - 意图模糊或没有合适角色时：不要调用 delegate_to_role，直接用文字帮助用户或简短追问需求细节。但如果已满足[角色涌现行为]中的触发条件（用户连续 3 轮围绕同一主题），则必须在回复中建议创建角色——这不是可选的。\n\
            - 收到角色回复（tool result）后：融合成自然回复，不要强调内部流转；只有用户明确问是谁处理时，才说明对应角色。\n\
            - 如果 delegate_to_role 返回失败（包含「委派失败」字样），不要在同一轮中再次调用 delegate_to_role 重试。直接用自然语言告诉用户该事项已记录，稍后会由对应角色处理。\n\
            - 用户追问\u{201c}为什么\u{201d}\u{201c}依据是什么\u{201d}\u{201c}你怎么知道的\u{201d}\u{201c}来源\u{201d}时，回复中必须原样包含[已知记忆]中对应的记忆内部链接 `[[记忆#YYYY/MM/DD HH:mm]](egosync-memory://memory-id)`，不要只写自然语言。例如：你之前告诉过我「在读《深度工作》」[[记忆#2026/07/10 20:14]](egosync-memory://bfc561e4-217d-4cdd-a746-02bc2c6cdf4c)"
        );
    }

    // Story 2.5: 角色涌现行为指令 — 始终注入，无论是否有 active 角色。
    // 零角色场景下最需要涌现建议（引导用户创建第一个角色）。
    let cooldowns = crate::db::app_settings::get_emergence_cooldowns(main_pool)
        .await
        .unwrap_or_default();
    let now = chrono::Utc::now();
    let active_cooldowns: Vec<String> = cooldowns
        .into_iter()
        .filter(|(_, ts)| {
            chrono::DateTime::parse_from_rfc3339(ts)
                .map(|dt| now.signed_duration_since(dt).num_days() < 7)
                .unwrap_or(false)
        })
        .map(|(domain, _)| domain)
        .collect();

    let mut emergence_prompt = String::from(
        "\n\n[角色涌现行为]\n\
        - 当你发现用户在最近几轮对话中反复提到某个尚未被任何 active 角色覆盖的领域时，可以用自然对话的方式建议创建一个新角色。\n\
        - 不要在第一轮就建议，至少观察到用户 2-3 次提及同一领域后再提议。\n\
        - 建议时用自然口吻，例如：「我注意到你最近经常聊到 X，要不要创建一个专门的角色来帮你？」\n\
        - 角色命名用日常生活中直白的身份词，不要用比喻或口号式表达。角色代表用户自己的身份，不是外部服务提供者：用户自己健身管理用「健康管理」而非「健身教练」，自己学英语用「学习者」而非「英语老师」；但如果健身教练是用户的本职工作，则「健身教练」就是正确命名。好的命名（覆盖不同生活领域）：家庭：丈夫、父亲、母亲、儿子、女儿；工作：产品经理、教师、程序员、设计师、健身教练；社区：邻居、志愿者；自我：阅读者、学习者、健康管理。不好的命名：掌舵人、领航者、生命建筑师、灵魂守护者。用户看到名字就能明白这个角色管什么。\n\
        - 用户同意后：先用一句话说明你会准备角色提议、用户可在弹窗里确认或调整，然后调用 create_role 工具发起角色提议；不要在工具调用后再追加确认话术。\n\
        - 用户拒绝后：调用 record_emergence_rejection 工具记录被拒领域，然后自然地继续对话。"
    );

    if !active_cooldowns.is_empty() {
        emergence_prompt.push_str(&format!(
            "\n- 最近被拒绝的领域（7天内不要再建议）：{}",
            active_cooldowns.join("、")
        ));
    }

    system_prompt.push_str(&emergence_prompt);

    Ok(system_prompt)
}

/// Build only the dynamic part of the butler prompt (role roster, tasks, memory,
/// cross-role summary, delegation guidelines, cooldowns). Used by the opencode
/// path as a user-message prefix — static parts (identity, transparency rules,
/// emergence instructions) are already in the opencode agent's `prompt` field.
pub async fn build_butler_dynamic_prompt(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
) -> Result<String, AppError> {
    let mut dynamic_prompt = String::new();

    let mission_context = build_butler_mission_context(main_pool).await?;
    dynamic_prompt.push_str("\n\n");
    dynamic_prompt.push_str(&mission_context);

    let active_roles = crate::db::roles::list_active_roles(main_pool).await?;
    if !active_roles.is_empty() {
        dynamic_prompt.push_str("\n\n[可委派角色清单]");
        for r in &active_roles {
            dynamic_prompt.push_str(&format!(
                "\n- id={} | 名称={} | 目标={}",
                r.id,
                r.name,
                if r.goal.trim().is_empty() {
                    "（未设定）"
                } else {
                    r.goal.trim()
                }
            ));
        }
    }

    let butler_task_summary = build_butler_task_summary(main_pool).await?;
    if !butler_task_summary.is_empty() {
        dynamic_prompt.push_str("\n\n");
        dynamic_prompt.push_str(&butler_task_summary);
    }

    let memory_summary = build_butler_memory_summary(main_pool).await?;
    if !memory_summary.is_empty() {
        dynamic_prompt.push_str("\n\n");
        dynamic_prompt.push_str(&memory_summary);
    }

    let cross_summary = build_cross_role_summary(conv_pool, main_pool).await?;
    if !cross_summary.is_empty() {
        dynamic_prompt.push_str("\n\n");
        dynamic_prompt.push_str(&cross_summary);
    }

    if !active_roles.is_empty() {
        dynamic_prompt.push_str(r#"

[行为指南]
            - 先判断用户意图类型：陈述事实/偏好、查询任务、操作任务（创建/完成/删除）、委派任务给角色处理。
            - 如果用户只是陈述某个角色相关事实或偏好（例如孩子叫什么、喜欢什么、学习表现如何），直接用自然口吻确认，不要调用工具，不要解释系统会如何记录或同步。
            - 当用户要求创建任务、标记任务完成、删除任务时，调用对应的 create_task / complete_task / delete_task 工具；用户陈述某项已有任务已完成、已提交或已做完，也视为完成操作。完成或删除已有任务时不得调用 delegate_to_role。task_id 和 role_id 必须是[各角色任务]或「可委派角色清单」中列出的 ID。
            - 当用户询问任务进展、有哪些任务、需要关注什么时，直接基于[各角色任务]回答，不要调用 delegate_to_role。查询任务不等于委派任务。
            - 如果用户交代的是某个角色相关任务、安排、日程、待办、规划或需要跟进的事项（例如家长会、约定、准备材料、制定练习计划），只要能匹配 active 角色，就调用 delegate_to_role。target_role_id 必须是「可委派角色清单」中列出的 id（UUID），不要传角色名称。
            - 不要向用户暴露内部机制：不要说"系统会同步""同步到某角色""角色已收到""委派成功""工具调用"等。
            - 当用户只是询问已知事实（例如某个孩子喜欢什么、某个固定安排是什么）时，优先基于[已知记忆]回答，并可说明目前只知道这些。
            - 当用户询问明确属性或偏好槽位（例如"喜欢吃什么""喜欢什么运动""某天安排是什么"）时，只回答与该问题槽位直接相关的记忆；不要因为同一角色或同一对象存在其它记忆，就补充阅读、学习、家庭日等无关事实。若没有直接相关记忆，只简短说明还不知道，不要列出其它领域记忆。
            - 用户一句话同时涉及多个角色且需要角色处理时：可以在同一轮内调用多个 delegate_to_role（并行委派）。
            - 意图模糊或没有合适角色时：不要调用 delegate_to_role，直接用文字帮助用户或简短追问需求细节。但如果已满足[角色涌现行为]中的触发条件（用户连续 3 轮围绕同一主题），则必须在回复中建议创建角色——这不是可选的。
            - 收到角色回复（tool result）后：融合成自然回复，不要强调内部流转；只有用户明确问是谁处理时，才说明对应角色。
            - 如果 delegate_to_role 返回失败（包含「委派失败」字样），不要在同一轮中再次调用 delegate_to_role 重试。直接用自然语言告诉用户该事项已记录，稍后会由对应角色处理。
            - 用户追问"为什么""依据是什么""你怎么知道的""来源"时，回复中必须原样包含[已知记忆]中对应的记忆内部链接 `[[记忆#YYYY/MM/DD HH:mm]](egosync-memory://memory-id)`，不要只写自然语言。例如：你之前告诉过我「在读《深度工作》」[[记忆#2026/07/10 20:14]](egosync-memory://bfc561e4-217d-4cdd-a746-02bc2c6cdf4c)"#);
    }

    // Cooldowns (dynamic) — only the cooldown list, not the full emergence instructions
    let cooldowns = crate::db::app_settings::get_emergence_cooldowns(main_pool)
        .await
        .unwrap_or_default();
    let now = chrono::Utc::now();
    let active_cooldowns: Vec<String> = cooldowns
        .into_iter()
        .filter(|(_, ts)| {
            chrono::DateTime::parse_from_rfc3339(ts)
                .map(|dt| now.signed_duration_since(dt).num_days() < 7)
                .unwrap_or(false)
        })
        .map(|(domain, _)| domain)
        .collect();

    if !active_cooldowns.is_empty() {
        dynamic_prompt.push_str(&format!(
            "\n\n[角色涌现-冷却列表]\n- 最近被拒绝的领域（7天内不要再建议）：{}",
            active_cooldowns.join("、")
        ));
    }

    Ok(dynamic_prompt)
}

pub async fn build_butler_messages(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    user_message: &str,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let mut result = Vec::new();

    let system_prompt = build_butler_system_prompt(conv_pool, main_pool).await?;

    // 诊断日志：打印实际发送给 LLM 的 system prompt（前3000字符）
    tracing::info!(
        "[build_butler_messages] system prompt 预览 (前3000字符):\n{}",
        system_prompt.chars().take(3000).collect::<String>()
    );

    result.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: system_prompt,
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    });

    let history =
        conversations::get_recent_messages(conv_pool, conversation_id, HISTORY_LIMIT).await?;
    for msg in history {
        result.push(ChatCompletionMessage {
            role: msg.role,
            content: msg.content,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Only append user_message if it's not already the last message in history
    // (it may have been inserted into DB before this function is called)
    if !user_message.is_empty() {
        let already_in_history = result
            .last()
            .map(|m| m.role == "user" && m.content == user_message)
            .unwrap_or(false);
        if !already_in_history {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(result)
}

const ROLE_BASE_PERSONA_PROMPT: &str = "\
你是用户的「{name}」分身——用户在这个身份维度下的 AI 助手。\
用户自己就是「{name}」，你帮助 TA 以这个身份思考、规划和执行相关目标与任务。\
你不是独立于用户的另一个人，你是用户作为「{name}」时的延伸。\
用简洁自然的中文回复，不用 emoji。";

pub async fn build_role_memory_summary(
    main_pool: &DbPool,
    role_id: &str,
) -> Result<String, AppError> {
    let role = crate::db::roles::get_role(main_pool, role_id).await?;
    let memories = memories::list_memories(main_pool, Some(role_id), None, None, None).await?;
    if memories.is_empty() {
        return Ok(String::new());
    }

    let lines = memories
        .into_iter()
        .take(BUTLER_MEMORY_PER_ROLE)
        .map(|memory| format_memory_reference_line(&memory))
        .collect::<Vec<_>>();
    let mut summary = format!("[当前角色记忆]\n{}：\n{}", role.name, lines.join("\n"));
    if summary.chars().count() > BUTLER_MEMORY_TOTAL_CHARS {
        summary = truncate_chars(&summary, BUTLER_MEMORY_TOTAL_CHARS) + "…";
    }
    Ok(summary)
}

async fn build_role_system_prompt(
    main_pool: &DbPool,
    role: &crate::models::role::Role,
) -> Result<String, AppError> {
    let mut sections = vec![ROLE_BASE_PERSONA_PROMPT.replace("{name}", &role.name)];

    let mut role_definition = format!("[role_definition]\n角色名称：{}", role.name);
    if !role.goal.trim().is_empty() {
        role_definition.push_str("\n核心目标：");
        role_definition.push_str(role.goal.trim());
    }

    let personality = role.personality_prompt.trim();
    if !personality.is_empty() {
        role_definition.push_str("\n个性描述：");
        role_definition.push_str(personality);
    } else {
        role_definition.push_str("\n默认语调：根据角色名称与核心目标选择自然语气；产品/工作类偏简洁专业，家庭/生活类偏温暖关怀，学习/成长类偏好奇探索。");
    }
    sections.push(role_definition);
    let meta_skill_prompt = crate::services::role_config::meta_skill_prompt(&role.skills_config);
    if !meta_skill_prompt.is_empty() {
        sections.push(meta_skill_prompt);
    }
    // P10: 降级直连 prompt 路径也声明已启用的自定义 Skill，与 AgentConfigService
    // 的 opencode 路径口径一致（禁用/已删除的不声明）。registry 加载失败时跳过，不阻断对话。
    let enabled_skill_ids =
        crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config);
    if !enabled_skill_ids.is_empty() {
        if let Ok(registry) = crate::db::skills::list_skills(main_pool).await {
            let custom_skill_lines: Vec<String> = enabled_skill_ids
                .iter()
                .filter_map(|id| {
                    registry
                        .iter()
                        .find(|skill| skill.id == *id)
                        .map(|skill| format!("- {}：{}", skill.name, skill.description))
                })
                .collect();
            if !custom_skill_lines.is_empty() {
                sections.push(format!(
                    "[自定义 Skill]\n{}\n只能按以上 EgoSync 已导入 Skill 的说明行动，不要调用或列出外部环境中的其它 Skill。",
                    custom_skill_lines.join("\n")
                ));
            }
        }
    }

    let mcp_server_lines = match crate::db::mcp_servers::role_enabled_mcp_lines(main_pool, &role.id).await {
        Ok(lines) => lines,
        Err(error) => {
            tracing::warn!(
                role_id = %role.id,
                error = ?error,
                "读取角色 MCP Server 绑定失败，按无可用 MCP Server 构建角色 prompt"
            );
            Vec::new()
        }
    };
    sections.push(if mcp_server_lines.is_empty() {
        "[MCP Server 使用边界]\n当前角色未绑定任何可用的外部 MCP Server。\n不要声明、推荐或调用外部 MCP 工具；如果用户请求依赖外部 MCP 的能力，应如实说明当前角色没有该能力。".to_string()
    } else {
        format!(
            "[MCP Server 使用边界]\n当前角色绑定的外部 MCP Server 如下：\n{}\n仅在确实需要时调用列表中的 MCP Server；不要声明或调用未列出的 MCP Server。",
            mcp_server_lines.join("\n")
        )
    });

    let memory_summary = build_role_memory_summary(main_pool, &role.id).await?;
    if !memory_summary.is_empty() {
        sections.push(memory_summary);
    }
    let task_summary = build_role_task_summary(main_pool, &role.id).await?;
    if !task_summary.is_empty() {
        sections.push(task_summary);
    }
    sections.push(TRANSPARENCY_UNCERTAINTY_RULES.to_string());

    sections.push(
        "[context_injection]\n以下历史消息是当前对话上下文；不要引入未提供的记忆。".to_string(),
    );
    Ok(sections.join("\n\n"))
}

/// 构建角色对话上下文。system prompt 完全独立于管家基线，
/// 让 LLM 知道当前在扮演谁。这是 Story 2.2 AC-2 / AC-6 的核心：
/// 角色之间个性化语调差异从此 prompt 注入开始（FR-6）。
pub async fn build_role_messages(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    role_id: &str,
    user_message: &str,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let role = crate::db::roles::get_role(main_pool, role_id).await?;

    let system_prompt = build_role_system_prompt(main_pool, &role).await?;

    let mut result = Vec::new();
    result.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: system_prompt,
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    });

    let history =
        conversations::get_recent_messages(conv_pool, conversation_id, HISTORY_LIMIT).await?;
    for msg in history {
        result.push(ChatCompletionMessage {
            role: msg.role,
            content: msg.content,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    if !user_message.is_empty() {
        let already_in_history = result
            .last()
            .map(|m| m.role == "user" && m.content == user_message)
            .unwrap_or(false);
        if !already_in_history {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(result)
}

const ONBOARDING_SYSTEM_PROMPT: &str = "\
你是 EgoSync 的分身管家，正在引导新用户完成首次设置。\n\n\
【你的唯一任务】\n\
通过自然对话，帮用户确定一个「角色」并调用 create_role 工具向用户【提议】这个角色。\n\
角色 = 用户生活/工作中的一个身份维度（如：产品经理、父亲、健身者）。\n\
每个角色有：名称、图标（line-icon 标识符）、品牌色（hex）、一句话目标。\n\n\
【关键认知】\n\
create_role 工具不会立即创建角色。它只是【向用户发起一个提议】，前端会弹出一个确认窗口，\n\
让用户编辑后点击「创建」按钮才真正创建。所以：\n\
- 你绝对不要说\"已创建\"、\"创建成功\"、\"已经为你建好了\"。\n\
- 你应该说\"我帮你拟了一个，看看怎么样？\"、\"已经为你准备好提议，需要可以在弹窗里调整。\"\n\n\
【对话流程】严格按以下阶段推进，不要跳步也不要卡步：\n\n\
第1步 - 问名字：\n\
  用这句固定的话开场：\"你好，我是你的分身管家，很高兴见到你。请问怎么称呼你呢？\"\n\
  不要自行发挥其他开场白，必须用这句话。\n\n\
第2步 - 了解方向并引导创建角色：\n\
  用户回答名字后，热情回应，然后说：\n\
  \"是否需要我给你创建一个角色分身，你可以用它来管理该角色对应的任务、记忆等？你可以直接说需要创建什么角色，或者你最近有什么任务，我可以给你提供角色建议。\"\n\
  然后等用户回答。\n\n\
第3步 - 提议角色（一次性完成）：\n\
  从用户的回答中提炼出一个身份维度，直接调用 create_role 工具发起提议。\n\
  不要先问\"我帮你创建一个 XXX 角色怎么样？\"再等用户回答 —— 直接调用工具，前端会弹窗让用户选择。\n\
  你的文字回复只需一句话，例如：\"听起来你在 XXX 方面投入很多，我准备了一个角色提议，看看是否合适？\"\n\n\
第4步 - 等待用户操作：\n\
  调用工具后，等用户在弹窗中点「创建」或「不需要」。\n\
  - 如果用户在聊天里说\"再换一个\"/\"我想要别的\" → 重新调用 create_role 提议新的角色。\n\
  - 如果用户在聊天里说\"不用了\" → 简短回应，等他下一步指示。\n\n\
第5步 - 完成（前端会通知）：\n\
  用户真正确认创建后，你会在历史里看到通知。这时用一句话祝贺。\n\n\
【行为红线】\n\
- 绝对不要在文字中说\"已创建\"、\"创建成功\"。必须用\"提议\"、\"准备\"、\"看看\"这类未完成时态。\n\
- 绝对不要在一条消息里既提议又自己代用户同意。提议归提议，确认归用户。\n\
- 绝对不要变成通用助理（不帮列清单、不帮做规划、不回答知识问题）。\n\
- 如果用户跑题，一句话拉回来：\"这个我之后可以帮你，现在我们先把你的第一个角色定下来。\"\n\
- 每次回复不超过 2 句话。\n\
- 语调温暖简洁，不用 emoji。\n\
- 不要问用户喜欢什么图标、颜色 —— 你自己从白名单选最合适的。\n\n\
【工具使用规则】\n\
你有一个 create_role 工具。识别到合适的身份维度后【直接】调用：\n\
1. name：从对话中提炼的直白身份词（中文 2–6 字）。\n\
   必须是日常生活中常用的普通词语，不要用比喻或口号式表达。\n\
   角色代表用户自己的身份，不是外部服务提供者：用户自己健身管理用「健康管理」而非「健身教练」，自己学英语用「学习者」而非「英语老师」；但如果健身教练是用户的本职工作，则「健身教练」就是正确命名。\n\
   好的命名（覆盖不同生活领域）：\n\
   家庭：丈夫、父亲、母亲、儿子、女儿；\n\
   工作：产品经理、教师、程序员、设计师、健身教练；\n\
   社区：邻居、志愿者；\n\
   自我：阅读者、学习者、健康管理。\n\
   不好的命名：掌舵人、领航者、生命建筑师、灵魂守护者。\n\
   原则：用户看到名字就明白这个角色管什么，不需要额外解释。\n\
2. icon：从下列标识符中选最贴合的：\n\
   briefcase（工作/职业）、code（编程/技术）、chart-bar（数据/分析）、palette（设计/创作）、\n\
   pen-tool（写作）、book-open（阅读/学习）、graduation-cap（教育）、dumbbell（健身/运动）、\n\
   heart-pulse（健康/医疗）、leaf（自然/环保）、home（家庭/居家）、users（团队/朋友）、\n\
   baby（育儿）、gamepad-2（游戏）、music（音乐）、camera（摄影）、plane（旅行）、\n\
   utensils（美食）、coffee（休闲）、target（目标/通用）、sparkles（灵感）、lightbulb（想法）、\n\
   compass（探索/规划）、wallet（财务）\n\
3. color：从下列品牌色中选最贴合的（hex 大写）：\n\
   #4F46E5 靛蓝（理性/专业）、#0EA5E9 天蓝（科技/沟通）、#10B981 翠绿（健康/成长）、\n\
   #F59E0B 琥珀（活力/创意）、#EF4444 玫红（热情/家人）、#8B5CF6 紫罗兰（艺术/灵感）、\n\
   #EC4899 粉（生活/情感）、#64748B 石板灰（稳重/中性）\n\
4. goal：从对话中总结一句目标（10–20 字）";

const ONBOARDING_HISTORY_LIMIT: i64 = 10;

pub async fn build_onboarding_messages(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    user_message: &str,
    _step: u8,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let mut result = Vec::new();

    result.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: ONBOARDING_SYSTEM_PROMPT.to_string(),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    });

    let history =
        conversations::get_recent_messages(conv_pool, conversation_id, ONBOARDING_HISTORY_LIMIT)
            .await?;
    for msg in history {
        // Skip system messages like [onboarding_start] to avoid confusing the model
        if msg.role == "system" {
            continue;
        }
        result.push(ChatCompletionMessage {
            role: msg.role,
            content: msg.content,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Only append user_message if it's not already the last message in history
    // (it may have been inserted into DB before this function is called)
    if !user_message.is_empty() && user_message != "__onboarding_start__" {
        let already_in_history = result
            .last()
            .map(|m| m.role == "user" && m.content == user_message)
            .unwrap_or(false);
        if !already_in_history {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    } else if user_message == "__onboarding_start__" {
        // onboarding 启动信号：如果历史为空（首次对话），需要追加一个
        // 占位 user message，否则消息列表只有 system prompt，LLM 会报错。
        let has_user_message = result.iter().any(|m| m.role == "user");
        if !has_user_message {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: "你好".to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(result)
}

/// 受支持的图标标识符白名单（黑白线框 Lucide React 风格）。
/// 注意：必须与前端 `GUI/src/lib/roleIcons.ts` 中的 `ROLE_ICONS` 完全同步。
const SUPPORTED_ICONS: &[&str] = &[
    "briefcase",      // 工作/职业
    "code",           // 编程/技术
    "chart-bar",      // 数据/分析
    "palette",        // 设计/创作
    "pen-tool",       // 写作/编辑
    "book-open",      // 阅读/学习
    "graduation-cap", // 教育/进修
    "dumbbell",       // 健身/运动
    "heart-pulse",    // 健康/医疗
    "leaf",           // 自然/环保
    "home",           // 家庭/居家
    "users",          // 团队/朋友
    "baby",           // 育儿/孩子
    "gamepad-2",      // 游戏/娱乐
    "music",          // 音乐
    "camera",         // 摄影
    "plane",          // 旅行
    "utensils",       // 美食/烹饪
    "coffee",         // 咖啡/休闲
    "target",         // 目标/通用
    "sparkles",       // 灵感/创意
    "lightbulb",      // 想法
    "compass",        // 探索/规划
    "wallet",         // 财务
];

/// 受支持的品牌色色板。必须与前端 `ROLE_COLORS` 同步。
const SUPPORTED_COLORS: &[&str] = &[
    "#4F46E5", // indigo
    "#0EA5E9", // sky
    "#10B981", // emerald
    "#F59E0B", // amber
    "#EF4444", // rose
    "#8B5CF6", // violet
    "#EC4899", // pink
    "#64748B", // slate
];

/// Story 2.3: 管家委派工具。
/// LLM 在管家视图判断用户意图明确指向某个 active 角色时调用；
/// 单轮可并行触发多个 tool_calls（一次回合内同时委派多个角色）。
/// follow-up（看到 tool_results 后的回合）必须 tools=None，禁止嵌套。
/// Story 2.5: 涌现拒绝记录工具。
/// 管家在用户拒绝创建角色建议后调用，写入冷却记录避免短期内重复建议同一领域。
fn record_emergence_rejection_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "record_emergence_rejection".to_string(),
        description: "当用户明确拒绝了你提出的创建新角色建议时调用此工具，记录被拒领域。同一领域短期内不会再次建议。".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "domain": {
                    "type": "string",
                    "description": "被用户拒绝的角色领域描述，简短中文，如「健身/运动」「摄影」「理财」"
                }
            },
            "required": ["domain"]
        }),
    }
}

fn delegate_to_role_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "delegate_to_role".to_string(),
        description: "把用户的具体任务委派给当前活跃的某个角色处理。仅在能明确判断任务属于某个 active 角色时调用；意图模糊时不要硬猜，改为追问。允许同一轮内多次调用以委派给多个角色。"
            .to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "target_role_id": {
                    "type": "string",
                    "description": "要委派给的角色 id（必须是 system prompt 「可委派角色清单」中列出的 id 之一）"
                },
                "task_summary": {
                    "type": "string",
                    "description": "交给该角色处理的任务摘要，简洁中文，建议 10-80 字"
                },
                "context": {
                    "type": "string",
                    "description": "可选：用户原话或必要背景，建议不超过 200 字"
                }
            },
            "required": ["target_role_id", "task_summary"]
        }),
    }
}

fn create_role_tool_definition() -> ToolDefinition {
    let icon_enum: Vec<serde_json::Value> = SUPPORTED_ICONS
        .iter()
        .map(|s| serde_json::Value::String((*s).to_string()))
        .collect();
    let color_enum: Vec<serde_json::Value> = SUPPORTED_COLORS
        .iter()
        .map(|s| serde_json::Value::String((*s).to_string()))
        .collect();

    ToolDefinition {
        name: "create_role".to_string(),
        description:
            "当用户最近在关注的事情，对应用户的一个明确的角色，或者用户明确阐述了自己的角色身份时，调用此工具，为用户创建一个虚拟角色。"
                .to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "角色名称，必须是日常生活中直白的身份词，2-6个中文字。角色代表用户自己的身份，不是外部服务提供者：用户自己健身管理用「健康管理」而非「健身教练」，自己学英语用「学习者」而非「英语老师」；但如果健身教练是用户的本职工作，则「健身教练」就是正确命名。好的命名（覆盖不同生活领域）：家庭：丈夫、父亲、母亲、儿子、女儿；工作：产品经理、教师、程序员、设计师、健身教练；社区：邻居、志愿者；自我：阅读者、学习者、健康管理。不要用比喻、口号或文学化表达。不好的命名：掌舵人、领航者、生命建筑师、灵魂守护者、破浪者。命名原则：用户看到名字就能立刻明白这个角色管什么，不需要解释。"
                },
                "icon": {
                    "type": "string",
                    "description": "最贴合该角色的图标标识符，从白名单中选择，例如：briefcase=工作、dumbbell=健身、code=编程、book-open=阅读、users=家人。",
                    "enum": icon_enum,
                },
                "color": {
                    "type": "string",
                    "description": "该角色的品牌色，从白名单中选择最贴合的十六进制色值。",
                    "enum": color_enum,
                },
                "goal": {
                    "type": "string",
                    "description": "该角色的一句话目标描述，来自对话上下文，约 10–20 字"
                }
            },
            "required": ["name", "icon", "color", "goal"]
        }),
    }
}

fn assistant_tool_call_message(content: String, reasoning: &str, tool_calls: Vec<ToolCall>) -> ChatCompletionMessage {
    ChatCompletionMessage { role: "assistant".to_string(), content, reasoning_content: (!reasoning.trim().is_empty()).then(|| reasoning.to_string()), tool_calls: Some(tool_calls), tool_call_id: None }
}

fn build_tool_followup_messages(
    mut messages: Vec<ChatCompletionMessage>,
    assistant_content: String,
    accumulated_reasoning: &str,
    tool_calls: &[ToolCall],
    tool_results: &[String],
) -> Vec<ChatCompletionMessage> {
    messages.push(assistant_tool_call_message(
        assistant_content,
        accumulated_reasoning,
        tool_calls.to_vec(),
    ));
    for (tool_call, result_text) in tool_calls.iter().zip(tool_results.iter()) {
        let content = if tool_call.name == "create_role"
            && result_text.starts_with("role_proposal_emitted:")
        {
            "角色提议已发送给用户，等待用户在弹窗中确认或修改。不要宣布创建成功。".to_string()
        } else {
            result_text.clone()
        };
        messages.push(ChatCompletionMessage {
            role: "tool".to_string(),
            content,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: Some(tool_call.id.clone()),
        });
    }
    messages
}

fn get_butler_chat_options() -> ChatOptions {
    ChatOptions { disable_thinking: false, tools: Some(vec![delegate_to_role_tool_definition(), create_role_tool_definition(), record_emergence_rejection_tool_definition()]), tool_choice: None }
}

fn get_onboarding_chat_options(step: u8) -> ChatOptions {
    // Only provide the create_role tool from step 3 onwards.
    // Steps 1-2 are for greeting and asking about interests — the model must NOT
    // be able to call create_role until it has gathered enough context.
    if step >= 3 {
        ChatOptions {
            disable_thinking: true,
            tools: Some(vec![create_role_tool_definition()]),
            // Force the model to call the tool — prevents it from endlessly
            // asking clarifying questions instead of proposing a role.
            tool_choice: Some("required".to_string()),
        }
    } else {
        ChatOptions {
            disable_thinking: false,
            tools: None,
            tool_choice: None,
        }
    }
}

pub async fn resolve_default_provider(
    main_pool: &DbPool,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    use crate::db::settings as db;
    use crate::models::settings::NetworkLocation;

    let config = db::get_default_llm_config(main_pool).await?;

    let api_key = secret_store::load_secret(&config.api_key_ref)?.ok_or_else(|| {
        AppError::KeyringError(format!(
            "未找到配置 '{}' 的 API Key，请在设置中重新保存",
            config.name
        ))
    })?;

    let net_loc = config.network_location.clone();
    let no_proxy = net_loc == NetworkLocation::Internal;

    let provider: Arc<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Arc::new(AnthropicProvider::new(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
        "minimax" => Arc::new(OpenAiProvider::new_with_reasoning_split(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
        _ => Arc::new(OpenAiProvider::new(config.base_url, api_key, config.model, no_proxy)?),
    };

    Ok(provider)
}

#[derive(Debug)]
enum OpencodeStreamAttemptError {
    Fatal(AppError),
    InvalidMcpSession,
    SkillLoadTimeout { tool_name: String },
}

impl From<AppError> for OpencodeStreamAttemptError {
    fn from(error: AppError) -> Self {
        Self::Fatal(error)
    }
}

async fn try_run_opencode_stream(
    app_handle: &tauri::AppHandle,
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    assistant_message_id: &str,
    user_message_id: &str,
    user_message: &str,
    cancel_token: &CancellationToken,
    role_id: Option<&str>,
    working_directory: Option<&str>,
    opencode_sessions: Arc<Mutex<std::collections::HashMap<String, OpencodeSessionState>>>,
    agent_bridge: crate::services::agent_bridge::AgentBridge,
    event_router: Arc<crate::services::event_router::EventRouter>,
    delegate_bridge: crate::services::delegate_bridge::DelegateBridge,
    already_retried_mcp_session: bool,
    already_retried_skill_load: bool,
    selected_skill: Option<&crate::models::skill::SelectableSkill>,
) -> Result<(), OpencodeStreamAttemptError> {
    let stream_started_at = Instant::now();
    let mut first_event_logged = false;
    let mut first_thinking_logged = false;
    let disabled_message = if let Some(rid) = role_id {
        crate::db::roles::get_role(main_pool, rid)
            .await
            .ok()
            .and_then(|role| {
                let skills = crate::services::role_config::skills_from_config(&role.skills_config);
                disabled_meta_skill_message(
                    skills.find_skills,
                    skills.skill_creator,
                    user_message,
                    MetaSkillConfigOwner::Role,
                )
            })
    } else {
        let butler_skills = crate::services::butler_config::get_butler_skills(main_pool)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to load butler Skill config before opencode send: {}",
                    e
                );
                crate::services::butler_config::default_butler_skills()
            });
        disabled_meta_skill_message(
            butler_skills.find_skills,
            butler_skills.skill_creator,
            user_message,
            MetaSkillConfigOwner::Butler,
        )
    };
    if let Some(message) = disabled_message {
        conversations::update_message_content(conv_pool, assistant_message_id, message)
            .await
            .ok();
        conversations::mark_message_complete(conv_pool, assistant_message_id)
            .await
            .ok();
        emit_stream_token(app_handle, conversation_id, None, message, false);
        emit_stream_done(app_handle, conversation_id, None);
        return Ok(());
    }

    let config_refresh_requested = app_handle
        .try_state::<crate::services::agent_config::AgentConfigService>()
        .map(|state| state.take_runtime_refresh())
        .unwrap_or(false);
    let delegate_refresh_requested = delegate_bridge.take_runtime_refresh().await;
    let runtime_refresh_requested = config_refresh_requested || delegate_refresh_requested;
    if runtime_refresh_requested {
        if let Err(error) =
            refresh_opencode_runtime_for_mcp_retry(app_handle, &opencode_sessions).await
        {
            if config_refresh_requested {
                if let Some(agent_config) =
                    app_handle.try_state::<crate::services::agent_config::AgentConfigService>()
                {
                    agent_config.request_runtime_refresh();
                }
            }
            if delegate_refresh_requested {
                delegate_bridge.request_runtime_refresh().await;
            }
            return Err(error.into());
        }
    }
    let project_dir = resolve_requested_working_directory(app_handle, working_directory)?;
    if runtime_refresh_requested {
        let agent = opencode_agent_key(role_id);
        if !agent_bridge.agent_exists(&agent, &project_dir).await? {
            if let Some(agent_config) =
                app_handle.try_state::<crate::services::agent_config::AgentConfigService>()
            {
                agent_config.request_runtime_refresh();
            }
            return Err(AppError::SidecarError(format!(
                "OpenCode runtime refresh completed but agent `{}` is unavailable",
                agent
            ))
            .into());
        }
    }
    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] working directory resolved");
    let mcp_scope_key = crate::services::mcp_server::mcp_scope_key_for_role(main_pool, role_id).await?;
    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] MCP scope resolved");
    let mcp_scope_lock = app_handle
        .try_state::<crate::commands::chat::OpencodeMcpScopeLock>()
        .map(|state| state.0.clone());
    let session_id = {
        let _mcp_scope_guard = match mcp_scope_lock.as_ref() {
            Some(lock) => Some(lock.lock().await),
            None => None,
        };
        if let Some(agent_config) = app_handle.try_state::<crate::services::agent_config::AgentConfigService>() {
            crate::services::mcp_server::sync_mcp_scope_for_role(main_pool, &agent_config, role_id)
                .await?;
        }
        tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] MCP scope synchronized");
        let session_cache_key = opencode_session_cache_key(&project_dir, &mcp_scope_key);
        let session_id = {
            let sessions = opencode_sessions.lock().await;
            sessions
                .get(conversation_id)
                .and_then(|state| state.sessions_by_directory.get(&session_cache_key))
                .cloned()
        };
        match session_id {
            Some(id) => {
                let mut sessions = opencode_sessions.lock().await;
                if let Some(state) = sessions.get_mut(conversation_id) {
                    state.active_session_id = id.clone();
                }
                id
            }
            None => {
                let agent = opencode_agent_key(role_id);
                let session = agent_bridge.create_session(&agent, &project_dir).await?;
                let mut sessions = opencode_sessions.lock().await;
                remember_opencode_session_for_directory(
                    &mut sessions,
                    conversation_id,
                    &session.id,
                    &project_dir,
                    &mcp_scope_key,
                )
            }
        }
    };
    // Build the message content — inject full system prompt prefix.
    // For butler: includes role roster + emergence instructions (same as direct-LLM path).
    // For roles: includes the role's personality/goal prompt.
    let content = if let Some(rid) = role_id {
        if let Ok(role) = crate::db::roles::get_role(main_pool, rid).await {
            let role_prompt = build_role_system_prompt(main_pool, &role).await?;
            format!("[系统指示]\n{}\n---\n{}", role_prompt, user_message)
        } else {
            user_message.to_string()
        }
    } else {
        // Butler — only dynamic context (roster, tasks, memory, cooldowns).
        // Static parts (identity, transparency rules, emergence instructions)
        // are already in the opencode agent's `prompt` field.
        let dynamic_prompt = build_butler_dynamic_prompt(conv_pool, main_pool)
            .await
            .unwrap_or_default();
        if dynamic_prompt.is_empty() {
            user_message.to_string()
        } else {
            format!("[动态上下文]\n{}\n---\n{}", dynamic_prompt, user_message)
        }
    };

    let bridge_session_registered = if let Some(role_id) = role_id {
        delegate_bridge.register_role_session(&session_id, role_id).await;
        true
    } else if !user_message_id.is_empty() {
        delegate_bridge
            .register_session(&session_id, user_message_id)
            .await;
        true
    } else {
        false
    };
    tracing::info!(
        session_id,
        ?role_id,
        conversation_id,
        bridge_session_registered,
        elapsed_ms = stream_started_at.elapsed().as_millis() as u64,
        "[stream-stage] session resolved"
    );

    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] prompt built");

    // Subscribe BEFORE triggering the prompt so we don't miss early events.
    let mut event_rx = event_router.subscribe(&session_id).await;
    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] event subscription completed");

    // 诊断日志：打印实际发送给 opencode 的完整 content
    tracing::info!(
        "[try_run_opencode_stream] 发送内容预览 (前2000字符): {}",
        content.chars().take(2000).collect::<String>()
    );

    // Trigger the prompt. POST /session/{id}/message is synchronous (returns
    // the completed message as JSON). Live tokens stream via the event bus;
    // the returned body is retained as a fallback when events miss final text.
    let agent_key = opencode_agent_key(role_id);

    // The command resolved this immutable snapshot before any runtime side effect.
    // Retries reuse it and therefore cannot observe a different registry/config state.
    if let Some(entry) = selected_skill {
        if !already_retried_mcp_session && !already_retried_skill_load {
            persist_and_emit_process_event(
                app_handle,
                conv_pool,
                conversation_id,
                assistant_message_id,
                &session_id,
                &ProcessEventCandidate {
                    event_type: "tool".to_string(),
                    tool_name: Some("skill".to_string()),
                    status: Some("completed".to_string()),
                    summary: format!("显式使用 Skill：{}", entry.name),
                    raw_json: serde_json::json!({
                        "input": { "name": entry.name },
                        "skillKey": entry.key,
                        "skillName": entry.name,
                        "scope": role_id.unwrap_or("butler"),
                        "source": "explicit"
                    }),
                },
                working_directory,
            )
            .await;
        }
        tracing::info!(skill_key = %entry.key, skill_name = %entry.name, scope = ?role_id,
            "explicit Skill snapshot accepted; using send_command");
    }

    let bridge = agent_bridge.clone();
    let send_session_id = session_id.clone();
    let command_skill_name = selected_skill.map(|entry| entry.name.clone());
    let command_arguments = user_message.to_string();
    let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();
    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] prompt POST started");
    tokio::spawn(async move {
        let result = if let Some(skill_name) = command_skill_name {
            bridge
                .send_command(&send_session_id, &agent_key, &skill_name, &command_arguments)
                .await
        } else {
            bridge.send_message(&send_session_id, &content, &agent_key).await
        };
        let _ = result_tx.send(result);
    });

    // Track the latest text per part so we can emit *deltas* to ChatStream
    // (which appends each token to a bucket). opencode's part.updated event
    // carries the full part text each time, so we diff locally.
    let mut part_text: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut accumulated_text = String::new();
    let mut accumulated_thinking = String::new();
    let mut pending_thinking = String::new();
    let mut thinking_started_at: Option<Instant> = None;
    let mut completed = false;
    let mut drain_deadline: Option<tokio::time::Instant> = None;
    let mut active_tool_timeout: Option<ToolExecutionDeadline> = None;
    // Fix 2: messageID → role mapping. opencode sends message.part.updated for
    // *both* user and assistant messages. We must learn each message's role from
    // "message.updated" events and only stream assistant-role parts to the UI.
    let mut message_roles: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // Track which bus part IDs have known user-visible text vs reasoning/thinking.
    let mut visible_text_parts: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut reasoning_parts: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut processed_tool_parts: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut processed_tool_statuses: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut has_tool_process_event = false;
    let mut pending_narration = String::new();
    let mut recorded_action_narrations: Vec<String> = Vec::new();
    let mut bubble_state = DelegationBubbleState::default();
    let mut final_message_id: Option<String> = None;
    let mut first_bubble_text_at_split: Option<String> = None;
    let mut final_text = String::new();
    let mut completed_response: Option<crate::models::agent::OpencodeCompletedMessage> = None;
    let mut send_result_observed = false;
    let mut user_visible_error_appended = false;
    let mut delegate_workers: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let delegate_lock = Arc::new(Mutex::new(()));

    // 构建 MCP 工具名展示映射：把 opencode 注册的 namespace（server id / 中文名 /
    // 安全化名）映射回用户可读的 server 名称，用于把工具名格式化为 "<server>:<tool>"。
    let mcp_tool_display_map = match crate::db::mcp_servers::list_mcp_servers(main_pool).await {
        Ok(servers) => McpToolDisplayMap::from_servers(&servers),
        Err(e) => {
            tracing::warn!("加载 MCP servers 用于工具名映射失败: {}", e);
            McpToolDisplayMap::default()
        }
    };

    loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                event_router.unsubscribe(&session_id).await;
                if bridge_session_registered {
                    delegate_bridge.unregister_session(&session_id).await;
                }
                if final_message_id.is_some() {
                    if !accumulated_thinking.is_empty() {
                        conversations::update_message_thinking(
                        conv_pool,
                        assistant_message_id,
                        &accumulated_thinking,
                    )
                    .await
                    .ok();
                    }
                    conversations::update_message_content(
                        conv_pool,
                        assistant_message_id,
                        &final_text,
                    )
                    .await
                    .ok();
                    conversations::mark_message_complete(conv_pool, assistant_message_id)
                    .await
                    .ok();
                    emit_stream_done(app_handle, conversation_id, Some(assistant_message_id));
                } else {
                    conversations::update_message_content(
                        conv_pool,
                        assistant_message_id,
                        &accumulated_text,
                    )
                    .await
                    .ok();
                    if !accumulated_thinking.is_empty() {
                        conversations::update_message_thinking(conv_pool, assistant_message_id, &accumulated_thinking).await.ok();
                    }
                    conversations::mark_message_complete(conv_pool, assistant_message_id).await.ok();
                    emit_stream_done(app_handle, conversation_id, None);
                }
                return Ok(());
            }
            maybe_event = event_rx.recv() => {
                let Some(event) = maybe_event else { break };
                if !first_event_logged {
                    first_event_logged = true;
                    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, event_type = %event.event_type, "[stream-stage] first event received");
                }
                match event.event_type.as_str() {
                    "message.part.delta" => {
                        // Incremental text delta — the primary streaming path.
                        let field = event.properties.get("field")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        if field != "text" { continue; }
                        let delta = event.properties.get("delta")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        if delta.is_empty() { continue; }
                        let msg_id = event.properties.get("messageID")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        // Skip user-role deltas
                        if !msg_id.is_empty() {
                            match message_roles.get(msg_id).map(|s| s.as_str()) {
                                Some("user") => continue,
                                _ => {} // unknown or assistant — proceed
                            }
                        }
                        let part_id = event.properties.get("partID")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        let delta_kind = classify_bus_text_delta(&event.properties, &visible_text_parts, &reasoning_parts);
                        if matches!(delta_kind, BusTextDeltaKind::Text) {
                            flush_thinking_process_event(
                                conv_pool,
                                conversation_id,
                                assistant_message_id,
                                &session_id,
                                &mut pending_thinking,
                                &mut thinking_started_at,
                                Some(&project_dir),
                            )
                            .await;
                        }
                        match delta_kind {
                            BusTextDeltaKind::Thinking => {
                                if thinking_started_at.is_none() {
                                    thinking_started_at = Some(Instant::now());
                                }
                                pending_thinking.push_str(delta);
                                accumulated_thinking.push_str(delta);
                                if !part_id.is_empty() {
                                    let entry = part_text.entry(part_id.to_string()).or_default();
                                    entry.push_str(delta);
                                }
                                if !first_thinking_logged {
                                    first_thinking_logged = true;
                                    tracing::info!(elapsed_ms = stream_started_at.elapsed().as_millis() as u64, "[stream-stage] first thinking emitted");
                                }
                                emit_stream_token(app_handle, conversation_id, None, delta, true);
                                continue;
                            }
                            BusTextDeltaKind::Text => {}
                            BusTextDeltaKind::WaitForPartType => continue,
                        }
                        let emit_message_id = match bubble_state.classify_message(msg_id) {
                            BubbleSlot::First => {
                                accumulated_text.push_str(delta);
                                None
                            }
                            BubbleSlot::Followup => {
                                final_text.push_str(delta);
                                final_message_id.clone()
                            }
                        };
                        if !part_id.is_empty() {
                            let entry = part_text.entry(part_id.to_string()).or_default();
                            entry.push_str(delta);
                        }
                        buffer_narration_delta(&mut pending_narration, &mut has_tool_process_event, delta);
                        emit_stream_token(app_handle, conversation_id, emit_message_id.as_deref(), delta, false);
                    }
                    "message.updated" => {
                        // Learn messageID → role so we can filter parts later.
                        // Also extract assistant text from completed messages (some
                        // models/providers don't emit incremental message.part.updated
                        // events — the full text arrives here directly).
                        if let Some(info) = event.properties.get("info") {
                            if let (Some(id), Some(role)) = (
                                info.get("id").and_then(|v| v.as_str()),
                                info.get("role").and_then(|v| v.as_str()),
                            ) {
                                message_roles.insert(id.to_string(), role.to_string());
                                if role == "assistant" {
                                    if let Some(parts) = info.get("parts").and_then(|v| v.as_array()) {
                                        for p in parts {
                                            let ptype = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                            let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                            if text.is_empty() {
                                                continue;
                                            }

                                            let part_id = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                            if ptype == "text" {
                                                visible_text_parts.insert(part_id.to_string());
                                            } else if is_thinking_bus_part_type(ptype) {
                                                reasoning_parts.insert(part_id.to_string());
                                            } else {
                                                continue;
                                            }

                                            let thinking = is_thinking_bus_part_type(ptype);
                                            if !thinking {
                                                flush_thinking_process_event(
                                                    conv_pool,
                                                    conversation_id,
                                                    assistant_message_id,
                                                    &session_id,
                                                    &mut pending_thinking,
                                                    &mut thinking_started_at,
                                                    Some(&project_dir),
                                                )
                                                .await;
                                            }
                                            let (emit_message_id, accum): (Option<String>, &mut String) = if thinking {
                                                (None, &mut accumulated_thinking)
                                            } else {
                                                match bubble_state.classify_message(id) {
                                                    BubbleSlot::First => (None, &mut accumulated_text),
                                                    BubbleSlot::Followup => (final_message_id.clone(), &mut final_text),
                                                }
                                            };
                                            let prev = part_text.entry(part_id.to_string()).or_default();
                                            if text.len() > prev.len() && text.starts_with(prev.as_str()) {
                                                let delta = text[prev.len()..].to_string();
                                                *prev = text.to_string();
                                                accum.push_str(&delta);
                                                if thinking {
                                                    if thinking_started_at.is_none() {
                                                        thinking_started_at = Some(Instant::now());
                                                    }
                                                    pending_thinking.push_str(&delta);
                                                } else {
                                                    buffer_narration_delta(&mut pending_narration, &mut has_tool_process_event, &delta);
                                                }
                                                emit_stream_token(app_handle, conversation_id, emit_message_id.as_deref(), &delta, thinking);
                                            } else if text != prev.as_str() {
                                                *prev = text.to_string();
                                                accum.clear();
                                                accum.push_str(text);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "message.part.updated" => {
                        let Some(part_raw) = event.properties.get("part") else { continue };
                        let Ok(part) = serde_json::from_value::<crate::models::agent::BusPart>(part_raw.clone()) else { continue };
                        tracing::debug!(part_type = %part.part_type, part_id = %part.id, msg_id = %part.message_id, text_len = part.text.len(), "stream part updated");
                        if is_thinking_bus_part_type(&part.part_type) {
                            reasoning_parts.insert(part.id.clone());
                        } else if part.part_type == "text" {
                            visible_text_parts.insert(part.id.clone());
                        }
                        // Skip user-role parts — they echo the user's own input.
                        if !part.message_id.is_empty() {
                            match message_roles.get(&part.message_id).map(|s| s.as_str()) {
                                Some("user") => continue,
                                _ => {} // unknown or assistant — proceed
                            }
                        }
                        // Only stream user-visible content. Tool parts are intercepted
                        // to trigger Tauri-side effects (role proposals, delegation, etc.)
                        if part.part_type == "text" || part.part_type == "tool" {
                            flush_thinking_process_event(
                                conv_pool,
                                conversation_id,
                                assistant_message_id,
                                &session_id,
                                &mut pending_thinking,
                                &mut thinking_started_at,
                                Some(&project_dir),
                            )
                            .await;
                        }
                        let (emit_message_id, accum, thinking): (Option<String>, &mut String, bool) = match part.part_type.as_str() {
                            "text" => match bubble_state.classify_message(&part.message_id) {
                                BubbleSlot::First => (None, &mut accumulated_text, false),
                                BubbleSlot::Followup => (final_message_id.clone(), &mut final_text, false),
                            },
                            "reasoning" | "thinking" => (None, &mut accumulated_thinking, true),
                            "tool" => {
                                // Intercept custom tool results from opencode.
                                // Tool part structure: { tool: "create_role", state: { status, output, input } }
                                // We extract the output when status=completed and parse our JSON action.
                                let tool_name = part_raw
                                    .get("tool")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| part_raw.get("toolInvocation").and_then(|v| v.get("toolName")).and_then(|v| v.as_str()))
                                    .unwrap_or("");
                                let tool_status = part_raw
                                    .get("state")
                                    .and_then(|s| s.get("status"))
                                    .and_then(|v| v.as_str())
                                    .or_else(|| part_raw.get("status").and_then(|v| v.as_str()))
                                    .unwrap_or("");
                                if let Some(candidate) = build_tool_process_event_candidate_with_display_map(part_raw, &mcp_tool_display_map) {
                                    if matches!(
                                        mcp_session_retry_decision(&candidate, already_retried_mcp_session),
                                        McpSessionRetryDecision::RefreshRuntimeAndRetry
                                    ) {
                                        event_router.unsubscribe(&session_id).await;
                                        if bridge_session_registered {
                                            delegate_bridge.unregister_session(&session_id).await;
                                        }
                                        return Err(OpencodeStreamAttemptError::InvalidMcpSession);
                                    }
                                    let status_key = format!("{}:{}", part.id, candidate.status.as_deref().unwrap_or(""));
                                    if processed_tool_statuses.insert(status_key) {
                                        if let Some(summary) = flush_narration_process_event(
                                            app_handle,
                                            conv_pool,
                                            conversation_id,
                                            assistant_message_id,
                                            &session_id,
                                            &mut pending_narration,
                                            Some(&project_dir),
                                        )
                                        .await
                                        {
                                            recorded_action_narrations.push(summary);
                                        }
                                        let fallback_display_name = candidate
                                            .tool_name
                                            .clone()
                                            .unwrap_or_else(|| tool_name.to_string());
                                        let display_tool_name = live_tool_display_name(
                                            tool_name,
                                            part_raw,
                                            &fallback_display_name,
                                        );
                                        update_tool_execution_deadline(
                                            &mut active_tool_timeout,
                                            &part.id,
                                            &display_tool_name,
                                            tool_status,
                                            tool_name.eq_ignore_ascii_case("skill"),
                                            Instant::now(),
                                        );
                                        if matches!(
                                            candidate.status.as_deref(),
                                            Some("running" | "completed")
                                        ) {
                                            emit_tool_status(
                                                app_handle,
                                                conversation_id,
                                                &display_tool_name,
                                                tool_status,
                                            );
                                        }
                                        persist_and_emit_process_event(
                                            app_handle,
                                            conv_pool,
                                            conversation_id,
                                            assistant_message_id,
                                            &session_id,
                                            &candidate,
                                            Some(&project_dir),
                                        )
                                        .await;
                                        has_tool_process_event = true;
                                        let role_tool_active = role_id.is_some()
                                            && matches!(candidate.status.as_deref(), Some("running" | "completed"));
                                        if role_tool_active
                                            && final_message_id.is_none()
                                            && !accumulated_text.trim().is_empty()
                                            && bubble_state.note_role_tool_active()
                                        {
                                            first_bubble_text_at_split = Some(accumulated_text.clone());
                                            final_message_id = Some(assistant_message_id.to_string());
                                            emit_stream_token(app_handle, conversation_id, Some(assistant_message_id), "", false);
                                        }
                                    }
                                }
                                let delegate_active = tool_name == "delegate_to_role"
                                    && (tool_status == "running" || tool_status == "completed");
                                if delegate_active
                                    && role_id.is_none()
                                    && bubble_state.note_delegate_active()
                                {
                                    if final_message_id.is_none() {
                                        first_bubble_text_at_split = Some(accumulated_text.clone());
                                        final_message_id = Some(assistant_message_id.to_string());
                                        emit_stream_token(app_handle, conversation_id, Some(assistant_message_id), "", false);
                                    }
                                }
                                if !claim_completed_tool_part(&mut processed_tool_parts, &part.id, part_raw) {
                                    continue;
                                }
                                if let Some(worker) = handle_tool_part(
                                    app_handle,
                                    conv_pool,
                                    main_pool,
                                    conversation_id,
                                    user_message_id,
                                    part_raw,
                                    delegate_lock.clone(),
                                )
                                .await
                                {
                                    delegate_workers.push(worker);
                                }
                                continue;
                            }
                            _ => continue,
                        };
                        let prev = part_text.entry(part.id.clone()).or_default();
                        if part.text.len() > prev.len() && part.text.starts_with(prev.as_str()) {
                            let delta = part.text[prev.len()..].to_string();
                            *prev = part.text.clone();
                            accum.push_str(&delta);
                            if thinking {
                                if thinking_started_at.is_none() {
                                    thinking_started_at = Some(Instant::now());
                                }
                                pending_thinking.push_str(&delta);
                            }
                            emit_stream_token(app_handle, conversation_id, emit_message_id.as_deref(), &delta, thinking);
                        } else if part.text != *prev {
                            // Non-monotonic update (replacement). Replay full text.
                            *prev = part.text.clone();
                            accum.clear();
                            accum.push_str(&part.text);
                        }
                    }
                    "session.idle" | "session.status" => {
                        // session.status carries { status: { type: "idle" } }
                        if event.event_type == "session.status" {
                            let is_idle = event.properties.get("status")
                                .and_then(|v| v.get("type"))
                                .and_then(|v| v.as_str())
                                == Some("idle");
                            if !is_idle {
                                continue;
                            }
                        }
                        // One prompt round complete — finalize and exit.
                        completed = true;
                        break;
                    }
                    "session.error" => {
                        let detail = event.properties.get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("opencode session error");
                        let friendly = format!("抱歉，Agent 引擎返回错误：{}", summarize_error(detail));
                        let error_message_id = final_message_id.as_deref();
                        if final_message_id.is_some() {
                            final_text.push_str(&friendly);
                        } else {
                            accumulated_text.push_str(&friendly);
                        }
                        emit_stream_token(
                            app_handle,
                            conversation_id,
                            error_message_id,
                            &friendly,
                            false,
                        );
                        user_visible_error_appended = true;
                        completed = true;
                        break;
                    }
                    _ => { /* ignore unrelated events */ }
                }
            }
            send_done = &mut result_rx, if !send_result_observed => {
                // The POST returned (success or failure). For success this just
                // means the LLM finished; the closing `session.idle` event may
                // or may not have arrived yet — keep draining briefly.
                send_result_observed = true;
                let should_drain = match send_done {
                    Ok(Err(e)) => {
                        event_router.unsubscribe(&session_id).await;
                        if bridge_session_registered {
                            delegate_bridge.unregister_session(&session_id).await;
                        }
                        if accumulated_text.is_empty() && final_text.is_empty() {
                            let mut sessions = opencode_sessions.lock().await;
                            sessions.remove(conversation_id);
                            return Err(OpencodeStreamAttemptError::Fatal(e));
                        }
                        let friendly = format!("\n\n抱歉，Agent 引擎返回错误：{}", summarize_error(&e.to_string()));
                        if final_message_id.is_some() {
                            final_text.push_str(&friendly);
                        } else {
                            accumulated_text.push_str(&friendly);
                        }
                        emit_stream_token(
                            app_handle,
                            conversation_id,
                            final_message_id.as_deref(),
                            &friendly,
                            false,
                        );
                        user_visible_error_appended = true;
                        false
                    }
                    Ok(Ok(response)) => {
                        completed_response = response;
                        true
                    }
                    Err(_) => false,
                };
                if should_drain {
                    drain_deadline = Some(tokio::time::Instant::now() + Duration::from_millis(800));
                    continue;
                }
                completed = true;
                break;
            }
            _ = async {
                if let Some(deadline) = active_tool_timeout
                    .as_ref()
                    .map(|active| active.deadline)
                {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    std::future::pending::<()>().await;
                }
            }, if active_tool_timeout.is_some() => {
                let timed_out = active_tool_timeout.take().expect("guarded by is_some");
                tracing::error!(
                    session_id,
                    tool_name = %timed_out.tool_name,
                    part_id = %timed_out.part_id,
                    timeout_seconds = timed_out.timeout.as_secs(),
                    retryable_skill_load = timed_out.retryable_skill_load,
                    already_retried_skill_load,
                    "opencode tool execution timed out"
                );
                let _ = tokio::time::timeout(
                    Duration::from_secs(2),
                    agent_bridge.abort_session(&session_id),
                )
                .await;
                event_router.unsubscribe(&session_id).await;
                if bridge_session_registered {
                    delegate_bridge.unregister_session(&session_id).await;
                }
                for worker in delegate_workers {
                    worker.abort();
                }
                if should_retry_skill_load_timeout(
                    timed_out.retryable_skill_load,
                    already_retried_skill_load,
                ) {
                    opencode_sessions.lock().await.remove(conversation_id);
                    return Err(OpencodeStreamAttemptError::SkillLoadTimeout {
                        tool_name: timed_out.tool_name,
                    });
                }
                let candidate = tool_execution_timeout_candidate(&timed_out.tool_name, timed_out.timeout);
                persist_and_emit_process_event(
                    app_handle,
                    conv_pool,
                    conversation_id,
                    assistant_message_id,
                    &session_id,
                    &candidate,
                    Some(&project_dir),
                )
                .await;
                let friendly = tool_execution_timeout_message(&timed_out.tool_name);
                if final_message_id.is_some() {
                    final_text.push_str(&friendly);
                    conversations::update_message_content(conv_pool, assistant_message_id, &final_text).await.ok();
                } else {
                    accumulated_text.push_str(&friendly);
                    conversations::update_message_content(conv_pool, assistant_message_id, &accumulated_text).await.ok();
                }
                if !accumulated_thinking.is_empty() {
                    conversations::update_message_thinking(conv_pool, assistant_message_id, &accumulated_thinking).await.ok();
                }
                conversations::mark_message_complete(conv_pool, assistant_message_id).await.ok();
                emit_stream_done(app_handle, conversation_id, final_message_id.as_deref());
                opencode_sessions.lock().await.remove(conversation_id);
                return Ok(());
            }
            _ = async {
                if let Some(deadline) = drain_deadline {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    std::future::pending::<()>().await;
                }
            }, if drain_deadline.is_some() => {
                completed = true;
                break;
            }
        }
    }

    flush_thinking_process_event(
        conv_pool,
        conversation_id,
        assistant_message_id,
        &session_id,
        &mut pending_thinking,
        &mut thinking_started_at,
        Some(&project_dir),
    )
    .await;
    event_router.unsubscribe(&session_id).await;
    if bridge_session_registered {
        delegate_bridge.unregister_session(&session_id).await;
    }

    for worker in delegate_workers {
        if let Err(err) = worker.await {
            tracing::warn!(error = %err, "delegate worker join failed");
        }
    }

    if !completed {
        // Stream ended without explicit completion (channel dropped, etc.) —
        // still finalize so the UI doesn't get stuck.
        completed = true;
    }

    if !send_result_observed {
        match tokio::time::timeout(Duration::from_secs(2), result_rx).await {
            Ok(Ok(Ok(response))) => {
                completed_response = response;
            }
            Ok(Ok(Err(e))) => {
                if accumulated_text.is_empty() && final_text.is_empty() {
                    let mut sessions = opencode_sessions.lock().await;
                    sessions.remove(conversation_id);
                    return Err(OpencodeStreamAttemptError::Fatal(e));
                }
                if !user_visible_error_appended {
                    let friendly = format!(
                        "\n\n抱歉，Agent 引擎返回错误：{}",
                        summarize_error(&e.to_string())
                    );
                    if final_message_id.is_some() {
                        final_text.push_str(&friendly);
                    } else {
                        accumulated_text.push_str(&friendly);
                    }
                    emit_stream_token(
                        app_handle,
                        conversation_id,
                        final_message_id.as_deref(),
                        &friendly,
                        false,
                    );
                }
            }
            Ok(Err(_)) | Err(_) => {}
        }
    }

    if final_message_id.is_some() {
        apply_completed_followup_message_fallback(
            Some(app_handle),
            conversation_id,
            final_message_id.as_deref(),
            completed_response,
            first_bubble_text_at_split.as_deref(),
            &mut final_text,
            &mut accumulated_thinking,
        );
        ensure_non_empty_opencode_result(
            app_handle,
            conversation_id,
            final_message_id.as_deref(),
            &mut final_text,
            &accumulated_thinking,
        );
    } else {
        apply_completed_message_fallback(
            app_handle,
            conversation_id,
            None,
            completed_response,
            &mut accumulated_text,
            &mut accumulated_thinking,
        );
        ensure_non_empty_opencode_result(
            app_handle,
            conversation_id,
            None,
            &mut accumulated_text,
            &accumulated_thinking,
        );
    }

    if final_message_id.is_some() {
        if should_flush_final_narration(has_tool_process_event, &pending_narration) {
            flush_narration_process_event(
                app_handle,
                conv_pool,
                conversation_id,
                assistant_message_id,
                &session_id,
                &mut pending_narration,
                Some(&project_dir),
            )
            .await;
        }
        remove_recorded_narration_prefixes(&mut final_text, &recorded_action_narrations);
        conversations::update_message_content(conv_pool, assistant_message_id, &final_text)
            .await
            .ok();
        if !accumulated_thinking.is_empty() {
            conversations::update_message_thinking(
                conv_pool,
                assistant_message_id,
                &accumulated_thinking,
            )
            .await
            .ok();
        }
        conversations::mark_message_complete(conv_pool, assistant_message_id)
            .await
            .ok();
        emit_stream_done(app_handle, conversation_id, Some(assistant_message_id));
    } else {
        if should_flush_final_narration(has_tool_process_event, &pending_narration) {
            flush_narration_process_event(
                app_handle,
                conv_pool,
                conversation_id,
                assistant_message_id,
                &session_id,
                &mut pending_narration,
                Some(&project_dir),
            )
            .await;
        }
        remove_recorded_narration_prefixes(&mut accumulated_text, &recorded_action_narrations);
        // 普通单段对话：单气泡收尾（改动前的原行为）。
        conversations::update_message_content(conv_pool, assistant_message_id, &accumulated_text)
            .await
            .ok();
        if !accumulated_thinking.is_empty() {
            conversations::update_message_thinking(
                conv_pool,
                assistant_message_id,
                &accumulated_thinking,
            )
            .await
            .ok();
        }
        conversations::mark_message_complete(conv_pool, assistant_message_id)
            .await
            .ok();
        emit_stream_done(app_handle, conversation_id, None);
    }
    let _ = completed;
    Ok(())
}

async fn refresh_opencode_runtime_for_mcp_retry(
    app_handle: &tauri::AppHandle,
    opencode_sessions: &Arc<Mutex<std::collections::HashMap<String, OpencodeSessionState>>>,
) -> Result<(), AppError> {
    let Some(sidecar) = app_handle
        .try_state::<Arc<Mutex<crate::services::sidecar::SidecarManager>>>()
        .map(|state| state.inner().clone())
    else {
        opencode_sessions.lock().await.clear();
        return Ok(());
    };
    let mut manager = sidecar.lock().await;
    manager.restart().await?;
    opencode_sessions.lock().await.clear();
    Ok(())
}

pub async fn run_stream(
    app_handle: tauri::AppHandle,
    conv_pool: ConversationsPool,
    main_pool: DbPool,
    conversation_id: String,
    assistant_message_id: String,
    user_message: String,
    cancel_token: CancellationToken,
    onboarding_step: Option<u8>,
    role_id: Option<String>,
    // Story 2.3: 触发本轮的 user message id —— 用于在 delegate 执行后把 routing_metadata
    // 写回触发这一轮的那条 user message。仅 butler 委派路径使用，其他路径忽略。
    user_message_id: String,
    opencode_sessions: Arc<Mutex<std::collections::HashMap<String, OpencodeSessionState>>>,
    agent_bridge: crate::services::agent_bridge::AgentBridge,
    event_router: Arc<crate::services::event_router::EventRouter>,
    delegate_bridge: crate::services::delegate_bridge::DelegateBridge,
    working_directory: Option<String>,
    // Story 10.1: 用户显式指定并已授权解析的 Skill，None 走普通消息路径（AC-6）
    selected_skill: Option<crate::models::skill::SelectableSkill>,
) -> Result<(), AppError> {
    // onboarding 直接走 LLM provider，跳过 opencode stream。
    // opencode stream 用的是管家 agent 配置（butler system prompt），
    // 不是 onboarding system prompt，会导致引导流程不正确。
    let skip_opencode = onboarding_step.is_some();

    let mut already_retried_mcp_session = false;
    let mut already_retried_skill_load = false;
    if !skip_opencode {
        loop {
            let result = try_run_opencode_stream(
                &app_handle,
                &conv_pool,
                &main_pool,
                &conversation_id,
                &assistant_message_id,
                &user_message_id,
                &user_message,
                &cancel_token,
                role_id.as_deref(),
                working_directory.as_deref(),
                opencode_sessions.clone(),
                agent_bridge.clone(),
                event_router.clone(),
                delegate_bridge.clone(),
                already_retried_mcp_session,
                already_retried_skill_load,
                selected_skill.as_ref(),
            )
            .await;

            match result {
                Ok(()) => return Ok(()),
                Err(OpencodeStreamAttemptError::InvalidMcpSession) if !already_retried_mcp_session => {
                    already_retried_mcp_session = true;
                    tracing::warn!(
                        conversation_id,
                        "MCP session invalid during opencode tool call; refreshing runtime and retrying once"
                    );
                    if let Err(e) = refresh_opencode_runtime_for_mcp_retry(&app_handle, &opencode_sessions).await {
                        tracing::warn!(
                            conversation_id,
                            error = %e,
                            "opencode runtime refresh after MCP session invalid failed"
                        );
                        break;
                    }
                    let _ = conversations::delete_message_process_events(&conv_pool, &assistant_message_id).await;
                    let _ = conversations::update_message_content(&conv_pool, &assistant_message_id, "").await;
                    let _ = conversations::update_message_thinking(&conv_pool, &assistant_message_id, "").await;
                    continue;
                }
                Err(OpencodeStreamAttemptError::InvalidMcpSession) => {
                    tracing::warn!(
                        conversation_id,
                        "MCP session invalid after retry; falling back to LlmProvider"
                    );
                    break;
                }
                Err(OpencodeStreamAttemptError::SkillLoadTimeout { tool_name })
                    if !already_retried_skill_load =>
                {
                    already_retried_skill_load = true;
                    tracing::warn!(
                        conversation_id,
                        tool_name = %tool_name,
                        timeout_seconds = OPENCODE_SKILL_LOAD_TIMEOUT.as_secs(),
                        "opencode skill load stalled; refreshing runtime and retrying once"
                    );
                    if let Err(e) = refresh_opencode_runtime_for_mcp_retry(&app_handle, &opencode_sessions).await {
                        tracing::warn!(conversation_id, tool_name = %tool_name, error = %e, "opencode runtime refresh after skill load timeout failed");
                        break;
                    }
                    let _ = conversations::delete_message_process_events(&conv_pool, &assistant_message_id).await;
                    let _ = conversations::update_message_content(&conv_pool, &assistant_message_id, "").await;
                    let _ = conversations::update_message_thinking(&conv_pool, &assistant_message_id, "").await;
                    continue;
                }
                Err(OpencodeStreamAttemptError::SkillLoadTimeout { tool_name }) => {
                    tracing::warn!(conversation_id, tool_name = %tool_name, "opencode skill load timed out after retry");
                    break;
                }
                Err(OpencodeStreamAttemptError::Fatal(e)) => {
                    if selected_skill.is_some() {
                        let friendly = format!(
                            "指定的 Skill 执行失败，未降级为普通对话：{}",
                            summarize_error(&e.to_string())
                        );
                        conversations::update_message_content(
                            &conv_pool,
                            &assistant_message_id,
                            &friendly,
                        )
                        .await?;
                        conversations::mark_message_complete(&conv_pool, &assistant_message_id).await?;
                        emit_stream_token(&app_handle, &conversation_id, Some(&assistant_message_id), &friendly, false);
                        emit_stream_done(&app_handle, &conversation_id, Some(&assistant_message_id));
                        return Ok(());
                    }
                    tracing::warn!(
                        "opencode stream unavailable, falling back to LlmProvider: {}",
                        e
                    );
                    break;
                }
            }
        }
    }

    let messages = match (onboarding_step, role_id.as_deref()) {
        (Some(step), _) => {
            build_onboarding_messages(&conv_pool, &conversation_id, &user_message, step).await?
        }
        (None, Some(rid)) => {
            build_role_messages(&conv_pool, &main_pool, &conversation_id, rid, &user_message)
                .await?
        }
        (None, None) => {
            build_butler_messages(&conv_pool, &main_pool, &conversation_id, &user_message).await?
        }
    };
    let provider = resolve_default_provider(&main_pool).await?;

    let chat_options = match (onboarding_step, role_id.as_deref()) {
        // onboarding：原有 create_role 工具（分阶段开启），与本 story 互斥
        (Some(step), _) => get_onboarding_chat_options(step),
        // 角色视图：不挂任何工具（Story 2.3 AC-4：角色私聊不再触发跨角色委派）
        (None, Some(_)) => ChatOptions::default(),
        // 管家视图：挂 delegate_to_role + create_role + record_emergence_rejection
        // tool_choice=None 让 LLM 自主决定（意图模糊时追问）
        (None, None) => get_butler_chat_options(),
    };

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);

    let provider_clone = provider.clone();
    let messages_clone = messages.clone();
    let options_clone = chat_options.clone();
    let stream_start = Instant::now();
    tokio::spawn(async move {
        if let Err(e) = provider_clone
            .chat_stream(messages_clone, tx, options_clone)
            .await
        {
            tracing::error!("LLM 流式调用失败: {}", e);
        }
    });

    // onboarding 主动跳过 opencode stream，不是 fallback，不显示降级提示。
    // 其他场景（opencode 失败后 fallback）才显示降级提示。
    let is_onboarding = onboarding_step.is_some();
    let prefix = if is_onboarding { "" } else { OPENCODE_FALLBACK_NOTICE };
    let mut accumulated = prefix.to_string();
    if !is_onboarding {
        emit_stream_token(
            &app_handle,
            &conversation_id,
            None,
            OPENCODE_FALLBACK_NOTICE,
            false,
        );
    }
    let mut accumulated_thinking = String::new();
    let mut pending_thinking = String::new();
    let mut thinking_started_at: Option<Instant> = None;
    let mut pending = String::new();
    let mut saw_thinking = false;
    let mut last_emit = Instant::now();
    let mut tool_calls_received: Vec<ToolCall> = Vec::new();
    const EMIT_INTERVAL: Duration = Duration::from_millis(33);

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                tracing::info!("流式回复被用户中断");
                None
            }
            event = rx.recv() => event,
            _ = tokio::time::sleep(EMIT_INTERVAL) => {
                if !pending.is_empty() {
                    let batch = std::mem::take(&mut pending);
                    emit_stream_token(&app_handle, &conversation_id, None, &batch, false);
                    last_emit = Instant::now();
                }
                continue;
            }
        };

        match event {
            Some(StreamEvent::Thinking(token)) => {
                if !saw_thinking {
                    saw_thinking = true;
                    tracing::info!(
                        "模型发送 reasoning，开始展示思考过程，耗时: {:?}",
                        stream_start.elapsed()
                    );
                }
                if thinking_started_at.is_none() {
                    thinking_started_at = Some(Instant::now());
                }
                pending_thinking.push_str(&token);
                accumulated_thinking.push_str(&token);
                emit_stream_token(&app_handle, &conversation_id, None, &token, true);
            }
            Some(StreamEvent::Token(token)) => {
                flush_thinking_process_event(
                    &conv_pool,
                    &conversation_id,
                    &assistant_message_id,
                    "fallback",
                    &mut pending_thinking,
                    &mut thinking_started_at,
                    None,
                )
                .await;
                if accumulated.is_empty() {
                    tracing::info!(
                        "首个 content token 到达，耗时: {:?}",
                        stream_start.elapsed()
                    );
                }
                accumulated.push_str(&token);
                pending.push_str(&token);

                if last_emit.elapsed() >= EMIT_INTERVAL {
                    let batch = std::mem::take(&mut pending);
                    emit_stream_token(&app_handle, &conversation_id, None, &batch, false);
                    last_emit = Instant::now();
                }
            }
            Some(StreamEvent::ToolCall(tc)) => {
                flush_thinking_process_event(
                    &conv_pool,
                    &conversation_id,
                    &assistant_message_id,
                    "fallback",
                    &mut pending_thinking,
                    &mut thinking_started_at,
                    None,
                )
                .await;
                tracing::info!(
                    "[run_stream] 收到 ToolCall: name={} id={} args={}",
                    tc.name,
                    tc.id,
                    tc.arguments
                );
                emit_tool_status(&app_handle, &conversation_id, &tc.name, "running");
                tool_calls_received.push(tc);
            }
            Some(StreamEvent::Done) => {
                flush_thinking_process_event(
                    &conv_pool,
                    &conversation_id,
                    &assistant_message_id,
                    "fallback",
                    &mut pending_thinking,
                    &mut thinking_started_at,
                    None,
                )
                .await;
                tracing::info!(
                    "[run_stream] done: conv={} assistant_msg={} accumulated_len={} saw_thinking={} tool_calls={}",
                    conversation_id,
                    assistant_message_id,
                    accumulated.len(),
                    saw_thinking,
                    tool_calls_received.len()
                );
                if !pending.is_empty() {
                    let batch = std::mem::take(&mut pending);
                    emit_stream_token(&app_handle, &conversation_id, None, &batch, false);
                }

                // Handle tool calls if any
                if !tool_calls_received.is_empty() {
                    // Keep the opening text out of the visible chat bubble; the final answer reuses the same assistant message.
                    let first_bubble_text = accumulated.clone();
                    tracing::info!(
                        "[run_stream] handling_tools: conv={} assistant_msg={} first_bubble_len={} saw_thinking={} tool_calls={}",
                        conversation_id,
                        assistant_message_id,
                        first_bubble_text.len(),
                        saw_thinking,
                        tool_calls_received.len()
                    );
                    let first_bubble_visible = !first_bubble_text.trim().is_empty();

                    let only_create_role_tool = tool_calls_received.len() == 1
                        && tool_calls_received
                            .first()
                            .map(|tc| tc.name == "create_role")
                            .unwrap_or(false);

                    let mut precomputed_tool_results: Option<Vec<String>> = None;
                    if only_create_role_tool {
                        let tool_results = execute_tool_calls(
                            &app_handle,
                            &main_pool,
                            &conv_pool,
                            &conversation_id,
                            &user_message_id,
                            &tool_calls_received,
                        )
                        .await;
                        for tc in &tool_calls_received {
                            emit_tool_status(&app_handle, &conversation_id, &tc.name, "completed");
                        }
                        let proposal_emitted = tool_results
                            .first()
                            .map(|result| result.starts_with("role_proposal_emitted:"))
                            .unwrap_or(false);

                        if proposal_emitted {
                            tracing::info!(
                                "[run_stream] create_role_followup_skipped: conv={} assistant_msg={}",
                                conversation_id,
                                assistant_message_id
                            );
                            // 保存 LLM 在工具调用前输出的文本到数据库，否则切到管家页面后
                            // 从数据库加载历史会看到空 assistant 消息（"最后一句话消失"）。
                            if first_bubble_visible {
                                conversations::update_message_content(
                                    &conv_pool,
                                    &assistant_message_id,
                                    &first_bubble_text,
                                )
                                .await
                                .ok();
                            }
                            conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                                .await
                                .ok();
                            emit_stream_done(&app_handle, &conversation_id, None);
                            break;
                        }

                        tracing::warn!(
                            "[run_stream] create_role_followup_retained: conv={} assistant_msg={} result={:?}",
                            conversation_id,
                            assistant_message_id,
                            tool_results.first()
                        );
                        precomputed_tool_results = Some(tool_results);
                    }

                    let final_message_id = assistant_message_id.clone();
                    emit_stream_token(&app_handle, &conversation_id, Some(&final_message_id), "", false);

                    let tool_results = match precomputed_tool_results {
                        Some(results) => results,
                        None => {
                            let results = execute_tool_calls(
                                &app_handle,
                                &main_pool,
                                &conv_pool,
                                &conversation_id,
                                &user_message_id,
                                &tool_calls_received,
                            )
                            .await;
                            for tc in &tool_calls_received {
                                emit_tool_status(
                                    &app_handle,
                                    &conversation_id,
                                    &tc.name,
                                    "completed",
                                );
                            }
                            results
                        }
                    };

                    // Build follow-up messages with tool results for continuation
                    let followup_messages = build_tool_followup_messages(
                        messages.clone(),
                        first_bubble_text,
                        &accumulated_thinking,
                        &tool_calls_received,
                        &tool_results,
                    );

                    // Stream the follow-up response (LLM will generate text after seeing tool results)
                    let (tx2, mut rx2) = mpsc::channel::<StreamEvent>(128);
                    let provider_clone2 = provider.clone();
                    let followup_clone = followup_messages;
                    tokio::spawn(async move {
                        // Story 2.3 AC-3: follow-up 必须 tools=None，禁止嵌套委派/嵌套 create_role
                        if let Err(e) = provider_clone2
                            .chat_stream(
                                followup_clone,
                                tx2,
                                ChatOptions {
                                    disable_thinking: true,
                                    tools: None,
                                    tool_choice: None,
                                },
                            )
                            .await
                        {
                            tracing::error!("工具结果后续流式调用失败: {}", e);
                        }
                    });

                    // Stream final response tokens into the single assistant bubble.
                    let mut final_accumulated = String::new();
                    while let Some(event2) = rx2.recv().await {
                        match event2 {
                            StreamEvent::Token(token) => {
                                final_accumulated.push_str(&token);
                                emit_stream_token(
                                    &app_handle,
                                    &conversation_id,
                                    Some(&final_message_id),
                                    &token,
                                    false,
                                );
                            }
                            StreamEvent::Done => break,
                            StreamEvent::Error(e) => {
                                tracing::error!("后续流式回复出错: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }

                    // 落库最终回复 + 通知 done（复用原 assistant id 让前端定型同一气泡）。
                    tracing::info!(
                        "[run_stream] final_done: conv={} assistant_msg={} final_len={}",
                        conversation_id,
                        final_message_id,
                        final_accumulated.len()
                    );
                    if final_accumulated.trim().is_empty() {
                        tracing::warn!(
                            "[run_stream] empty_final: conv={} assistant_msg={} tool_calls={}",
                            conversation_id,
                            final_message_id,
                            tool_calls_received.len()
                        );
                    }
                    conversations::update_message_content(
                        &conv_pool,
                        &final_message_id,
                        &final_accumulated,
                    )
                    .await
                    .ok();
                    conversations::mark_message_complete(&conv_pool, &final_message_id)
                        .await
                        .ok();
                    emit_stream_done(&app_handle, &conversation_id, Some(&final_message_id));
                    break;
                } else if onboarding_step.is_some() && looks_like_fake_role_creation(&accumulated) {
                    // ========== 方案 A 兜底：模型伪装了"角色创建成功"但实际没发 tool_calls ==========
                    tracing::warn!(
                        "[fallback] onboarding 期间检测到伪造的角色创建文本，启动二次提取。accumulated_preview={:?}",
                        accumulated.chars().take(120).collect::<String>()
                    );
                    // 先尝试直接从文本里抠 JSON（极少出现但便宜）；否则再发一次 LLM 调用做提取
                    let role_input = match parse_role_json(&accumulated) {
                        Some(r) => {
                            tracing::info!(
                                "[fallback] 直接从 assistant 文本解析到角色 JSON: name={}",
                                r.name
                            );
                            Some(r)
                        }
                        None => try_fallback_extract_role(&provider, &messages, &accumulated).await,
                    };
                    if let Some(input) = role_input {
                        // 兜底分支同样改为「仅提议」：不直接写库，发 role:proposed 让用户在弹窗里确认。
                        // 归一化 icon/color 到白名单（与 execute_create_role 保持一致）
                        let normalized_icon = input.icon.as_deref().and_then(|s| {
                            let t = s.trim();
                            if SUPPORTED_ICONS.iter().any(|w| *w == t) {
                                Some(t.to_string())
                            } else {
                                None
                            }
                        });
                        let normalized_color = input.color.as_deref().and_then(|s| {
                            let t = s.trim().to_ascii_uppercase();
                            if SUPPORTED_COLORS.iter().any(|w| w.eq_ignore_ascii_case(&t)) {
                                Some(t)
                            } else {
                                None
                            }
                        });
                        tracing::info!(
                            "[fallback] 兜底提议角色: name={} icon={:?} color={:?}",
                            input.name,
                            normalized_icon,
                            normalized_color,
                        );
                        let _ = app_handle.emit(
                            "role:proposed",
                            RoleProposedPayload {
                                conversation_id: conversation_id.clone(),
                                name: input.name.clone(),
                                icon: normalized_icon,
                                color: normalized_color,
                                goal: input.goal.clone(),
                            },
                        );
                    } else {
                        tracing::warn!("[fallback] 二次提取未得到有效角色，跳过提议");
                    }
                    // ========== 兜底结束 ==========
                }

                // Save final message
                conversations::update_message_content(
                    &conv_pool,
                    &assistant_message_id,
                    &accumulated,
                )
                .await
                .ok();
                if !accumulated_thinking.is_empty() {
                    conversations::update_message_thinking(
                        &conv_pool,
                        &assistant_message_id,
                        &accumulated_thinking,
                    )
                    .await
                    .ok();
                }
                conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                    .await
                    .ok();

                emit_stream_done(&app_handle, &conversation_id, None);
                break;
            }
            Some(StreamEvent::Error(err_msg)) => {
                flush_thinking_process_event(
                    &conv_pool,
                    &conversation_id,
                    &assistant_message_id,
                    "fallback",
                    &mut pending_thinking,
                    &mut thinking_started_at,
                    None,
                )
                .await;
                let friendly = format!(
                    "抱歉，我现在无法回应。原因：{}。请检查一下模型配置是否正确。",
                    summarize_error(&err_msg)
                );

                conversations::update_message_content(&conv_pool, &assistant_message_id, &friendly)
                    .await
                    .ok();
                conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                    .await
                    .ok();

                emit_stream_token(&app_handle, &conversation_id, None, &friendly, false);
                emit_stream_done(&app_handle, &conversation_id, None);
                break;
            }
            None => {
                flush_thinking_process_event(
                    &conv_pool,
                    &conversation_id,
                    &assistant_message_id,
                    "fallback",
                    &mut pending_thinking,
                    &mut thinking_started_at,
                    None,
                )
                .await;
                if !pending.is_empty() {
                    let batch = std::mem::take(&mut pending);
                    emit_stream_token(&app_handle, &conversation_id, None, &batch, false);
                }

                conversations::update_message_content(
                    &conv_pool,
                    &assistant_message_id,
                    &accumulated,
                )
                .await
                .ok();
                if !accumulated_thinking.is_empty() {
                    conversations::update_message_thinking(
                        &conv_pool,
                        &assistant_message_id,
                        &accumulated_thinking,
                    )
                    .await
                    .ok();
                }
                conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                    .await
                    .ok();

                emit_stream_done(&app_handle, &conversation_id, None);
                break;
            }
        }
    }

    Ok(())
}

/// 委派两气泡路由的纯状态机。
///
/// 实测时间线（运行时诊断确认）：管家委派会产生两条不同 messageID 的
/// assistant 消息——第一段"稍等…"（msg_A）与委派回复"产品经理…"（msg_B），
/// 中间 `delegate_to_role` 工具经历 pending→running→completed（约 40s 等待）。
///
/// 路由规则：
/// - 委派激活前，所有 assistant 文本归第一段开场（`First`）。
/// - `delegate_to_role` 进入活动态（running/completed，取最早一次）时切换；
///   调用方据此让前端保持单个弹跳点气泡，并把后续文本作为最终结果写回同一条 assistant 消息。
/// - 委派激活后出现的**新** messageID 归最终文本槽（`Followup`）；
///   已知属于第一段开场的 messageID 仍保持 `First`（其残余 delta 不串桶）。
///
/// 不依赖具体 DB id，只做"该归哪个气泡"的决策，便于单测；I/O 留在调用方。
#[derive(Default)]
struct DelegationBubbleState {
    delegated: bool,
    role_tool_active: bool,
    first_message_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BubbleSlot {
    First,
    Followup,
}

impl DelegationBubbleState {
    /// 判定某条 assistant messageID 的文本应落入哪个气泡。
    fn classify_message(&mut self, message_id: &str) -> BubbleSlot {
        if self.role_tool_active {
            return BubbleSlot::Followup;
        }
        if !self.delegated {
            // 委派尚未激活：记住第一条 messageID，全部归第一气泡。
            if self.first_message_id.is_none() {
                self.first_message_id = Some(message_id.to_string());
            }
            return BubbleSlot::First;
        }
        // 委派已激活：第一气泡的原 messageID 仍归 First，其它新 messageID 归 Followup。
        match &self.first_message_id {
            Some(first) if first == message_id => BubbleSlot::First,
            _ => BubbleSlot::Followup,
        }
    }

    /// 标记 `delegate_to_role` 进入活动态。首次调用返回 true（调用方据此切到最终文本槽），后续重复上报返回 false。
    fn note_delegate_active(&mut self) -> bool {
        if self.delegated {
            return false;
        }
        self.delegated = true;
        true
    }

    fn note_role_tool_active(&mut self) -> bool {
        if self.role_tool_active {
            return false;
        }
        self.role_tool_active = true;
        true
    }

    /// 委派是否已激活（调用方用于决定 delta 是否需要路由到 followup 气泡）。
    #[allow(dead_code)]
    fn is_delegated(&self) -> bool {
        self.delegated
    }
}

fn claim_completed_tool_part(
    processed_tool_parts: &mut std::collections::HashSet<String>,
    part_id: &str,
    part_raw: &serde_json::Value,
) -> bool {
    let status = part_raw
        .get("state")
        .and_then(|s| s.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let output = part_raw
        .get("state")
        .and_then(|s| s.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    status == "completed"
        && !part_id.is_empty()
        && !output.is_empty()
        && processed_tool_parts.insert(part_id.to_string())
}

fn spawn_delegate_worker<F>(lock: Arc<Mutex<()>>, work: F) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let _guard = lock.lock().await;
        work.await;
    })
}

fn non_json_tool_output_preview(output: &str) -> String {
    output.chars().take(100).collect()
}

/// Handle a tool bus part from opencode's custom tools.
/// opencode tool parts have this structure:
/// ```json
/// { "type": "tool", "tool": "create_role",
///   "state": { "status": "completed", "output": "{\"action\":...}", "input": {...} } }
/// ```
/// We only act on status=completed parts where output contains our action JSON.
async fn handle_tool_part<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    butler_user_message_id: &str,
    part_raw: &serde_json::Value,
    delegate_lock: Arc<Mutex<()>>,
) -> Option<tokio::task::JoinHandle<()>> {
    // Only process completed tool calls
    let status = part_raw
        .get("state")
        .and_then(|s| s.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "completed" {
        return None;
    }

    let tool_name = part_raw.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let output = part_raw
        .get("state")
        .and_then(|s| s.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if output.is_empty() {
        return None;
    }

    tracing::info!(
        "[opencode-tool] completed: tool={} output_len={}",
        tool_name,
        output.len()
    );

    // Parse the JSON result from our custom tool's execute()
    let Ok(result_json) = serde_json::from_str::<serde_json::Value>(output) else {
        tracing::debug!(
            "[opencode-tool] non-JSON output, ignoring: {:?}",
            non_json_tool_output_preview(output)
        );
        return None;
    };

    let action = result_json
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    tracing::info!("[opencode-tool] action={} conv={}", action, conversation_id);

    match action {
        "create_role" => {
            let name = result_json
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if name.is_empty() {
                return None;
            }
            let goal = result_json
                .get("goal")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let icon = result_json
                .get("icon")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let t = s.trim();
                    if SUPPORTED_ICONS.contains(&t) {
                        Some(t.to_string())
                    } else {
                        None
                    }
                });
            let color = result_json
                .get("color")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let t = s.trim().to_ascii_uppercase();
                    if SUPPORTED_COLORS.iter().any(|c| c.eq_ignore_ascii_case(&t)) {
                        Some(t)
                    } else {
                        None
                    }
                });

            tracing::info!(
                "[opencode-tool] create_role proposal: name={} icon={:?} color={:?}",
                name,
                icon,
                color
            );

            let _ = app_handle.emit(
                "role:proposed",
                RoleProposedPayload {
                    conversation_id: conversation_id.to_string(),
                    name: name.to_string(),
                    icon,
                    color,
                    goal,
                },
            );
        }
        "delegate_to_role" => {
            let role_id = result_json
                .get("target_role_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let task_summary = result_json
                .get("task_summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if role_id.is_empty() || task_summary.is_empty() {
                return None;
            }
            let conv_pool = conv_pool.clone();
            let main_pool = main_pool.clone();
            let butler_user_message_id = butler_user_message_id.to_string();
            let result_json = result_json.clone();
            let worker = spawn_delegate_worker(delegate_lock, async move {
                let _ = handle_delegate_tool_result(
                    &conv_pool,
                    &main_pool,
                    &butler_user_message_id,
                    &result_json,
                )
                .await;
            });
            tracing::info!(
                "[opencode-tool] delegate_to_role: role_id={} task={}",
                role_id,
                task_summary
            );

            let _ = app_handle.emit(
                "role:delegated",
                serde_json::json!({
                    "roleId": role_id,
                    "taskSummary": task_summary,
                    "conversationId": conversation_id,
                }),
            );
            return Some(worker);
        }
        "create_task" | "complete_task" | "delete_task" => {
            let status = result_json
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = result_json
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tracing::info!(
                "[opencode-tool] {}: status={} message={}",
                action,
                status,
                message
            );
            if status == "ok" {
                let _ = app_handle.emit(
                    "task:tool-action",
                    serde_json::json!({
                        "action": action,
                        "message": message,
                        "conversationId": conversation_id,
                    }),
                );
            }
        }
        "record_emergence_rejection" => {
            let domain = result_json
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if domain.is_empty() {
                return None;
            }
            let now = chrono::Utc::now().to_rfc3339();
            if let Err(e) =
                crate::db::app_settings::set_emergence_cooldown(main_pool, domain, &now).await
            {
                tracing::error!("[opencode-tool] emergence rejection write failed: {}", e);
            }
            let _ = crate::db::app_settings::clear_expired_cooldowns(main_pool, 7).await;
            tracing::info!(
                "[opencode-tool] emergence rejection recorded: domain={}",
                domain
            );
        }
        _ => {
            tracing::debug!("[opencode-tool] unknown action={}, ignoring", action);
        }
    }
    None
}

async fn handle_delegate_tool_result(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    butler_user_message_id: &str,
    result_json: &serde_json::Value,
) -> Option<DelegationRecord> {
    let role_id = result_json
        .get("target_role_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let task_summary = result_json
        .get("task_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if role_id.is_empty() || task_summary.is_empty() {
        return None;
    }

    let args = serde_json::json!({
        "target_role_id": role_id,
        "task_summary": task_summary,
        "context": result_json
            .get("context")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    })
    .to_string();
    let source_conversation_id = sqlx::query_scalar::<_, String>(
        "SELECT conversation_id FROM messages WHERE id = ?1",
    )
    .bind(butler_user_message_id)
    .fetch_optional(&**conv_pool)
    .await
    .ok()
    .flatten();
    let (_text, record) = execute_delegate_to_role_with_source(
        main_pool,
        conv_pool,
        &args,
        source_conversation_id.as_deref(),
    )
    .await;
    append_delegation_metadata(conv_pool, butler_user_message_id, &record).await;
    Some(record)
}

pub async fn append_delegation_metadata(
    conv_pool: &ConversationsPool,
    butler_user_message_id: &str,
    record: &DelegationRecord,
) {
    let mut delegations = existing_delegations(conv_pool, butler_user_message_id).await;
    delegations.push(serde_json::to_value(record).unwrap_or_else(|_| serde_json::json!({})));
    match serde_json::to_string(&serde_json::json!({ "delegations": delegations })) {
        Ok(json) => {
            if let Err(e) = crate::db::conversations::update_message_routing_metadata(
                conv_pool,
                butler_user_message_id,
                &json,
            )
            .await
            {
                tracing::error!("[opencode-tool] 写入 routing_metadata 失败: {}", e);
            }
        }
        Err(e) => tracing::error!("[opencode-tool] 序列化 routing_metadata 失败: {}", e),
    }
}

async fn existing_delegations(
    conv_pool: &ConversationsPool,
    butler_user_message_id: &str,
) -> Vec<serde_json::Value> {
    if butler_user_message_id.is_empty() {
        return Vec::new();
    }
    let row = sqlx::query_scalar::<_, Option<String>>(
        "SELECT routing_metadata FROM messages WHERE id = ? LIMIT 1",
    )
    .bind(butler_user_message_id)
    .fetch_optional(&**conv_pool)
    .await;
    let Ok(Some(Some(metadata))) = row else {
        return Vec::new();
    };
    serde_json::from_str::<serde_json::Value>(&metadata)
        .ok()
        .and_then(|v| v.get("delegations").and_then(|d| d.as_array()).cloned())
        .unwrap_or_default()
}

/// Execute tool calls and return result strings for each
async fn execute_tool_calls(
    app_handle: &tauri::AppHandle,
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    butler_user_message_id: &str,
    tool_calls: &[ToolCall],
) -> Vec<String> {
    let mut results = Vec::new();
    // Story 2.3 AC-6: 收集本轮所有 delegate 的审计记录，最后合并写入 routing_metadata
    let mut delegations: Vec<DelegationRecord> = Vec::new();

    for tc in tool_calls {
        tracing::info!(
            "[tool] executing: conv={} user_msg={} name={} id={} args_len={}",
            conversation_id,
            butler_user_message_id,
            tc.name,
            tc.id,
            tc.arguments.len()
        );
        let result = match tc.name.as_str() {
            "create_role" => {
                execute_create_role(app_handle, main_pool, conversation_id, &tc.arguments).await
            }
            "delegate_to_role" => {
                // Story 2.3 AC-2: 单轮内多个 delegate_to_role 串行执行（V1）
                let (text, record) = execute_delegate_to_role_with_source(
                    main_pool,
                    conv_pool,
                    &tc.arguments,
                    Some(conversation_id),
                )
                .await;
                delegations.push(record);
                text
            }
            "record_emergence_rejection" => {
                execute_record_emergence_rejection(main_pool, &tc.arguments).await
            }
            _ => {
                tracing::warn!("未知工具调用: {}", tc.name);
                format!("错误：未知工具 {}", tc.name)
            }
        };
        results.push(result);
    }

    // Story 2.3 AC-6: 把本轮的所有 delegation 合并写入触发 user message
    if !delegations.is_empty() && !butler_user_message_id.is_empty() {
        match serde_json::to_string(&serde_json::json!({ "delegations": delegations })) {
            Ok(json) => {
                if let Err(e) = crate::db::conversations::update_message_routing_metadata(
                    conv_pool,
                    butler_user_message_id,
                    &json,
                )
                .await
                {
                    tracing::error!("[delegate] 写入 routing_metadata 失败: {}", e);
                }
            }
            Err(e) => tracing::error!("[delegate] 序列化 routing_metadata 失败: {}", e),
        }
    }

    results
}

/// Story 2.3 AC-6: 单条委派的审计记录，多个合并进 routing_metadata.delegations[]。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationRecord {
    target_role_id: String,
    target_role_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_conversation_id: Option<String>,
    task_summary: String,
    /// "ok" / "role_not_found"
    status: String,
}

const CREATE_DELEGATED_TASK_TOOL: &str = "create_delegated_task";
const DELEGATED_TASK_PROMPT: &str = "[委派任务跟踪]\n明确行动、未来日程、承诺、待办或交付物必须调用 create_delegated_task；纯咨询、分析、建议不要调用。一句话有多个独立行动时分别调用。截止时间可省略。工具不接受角色 ID，任务固定归属当前角色。单项调用结果 status=created 表示已创建；多项调用结果 status=proposed 表示仅生成待用户确认的拆分提案，绝不能声称任务已创建。失败时如实说明，不得重试工具。";

fn delegated_task_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: CREATE_DELEGATED_TASK_TOOL.to_string(),
        description: "为当前委派角色创建一个需跟踪的任务；一项行动调用一次，纯咨询不要调用。".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "非空的具体行动" },
                "deadline": { "type": "string", "description": "可选，优先使用 ISO 8601" }
            },
            "required": ["title"],
            "additionalProperties": false
        }),
    }
}

fn normalize_delegated_deadline(raw: Option<&str>) -> (Option<String>, Option<String>, String) {
    let Some(value) = raw.map(str::trim).filter(|s| !s.is_empty()) else { return (None, None, "none".to_string()) };
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        let normalized = dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
        return (Some(normalized.clone()), None, normalized);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%z") {
        let normalized = dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
        return (Some(normalized.clone()), Some("截止时间已规范化为 ISO 8601。".to_string()), normalized);
    }
    for format in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M", "%Y/%m/%d %H:%M"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, format) {
            let normalized = dt.format("%Y-%m-%dT%H:%M:%S").to_string();
            return (Some(normalized.clone()), Some("截止时间已规范化为 ISO 8601。".to_string()), normalized);
        }
    }
    for format in ["%Y-%m-%d", "%Y/%m/%d"] {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, format) {
            let normalized = date.format("%Y-%m-%d").to_string();
            return (Some(normalized.clone()), (normalized != value).then(|| "截止日期已规范化。".to_string()), normalized);
        }
    }
    (None, Some(format!("未保存截止时间：无法识别「{}」。", value)), format!("invalid:{}", value.to_lowercase()))
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DelegatedTaskOutcome {
    call_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")] title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] duplicate_of: Option<String>,
}

fn delegated_task_process_candidate(call: Option<&ToolCall>, outcome: &DelegatedTaskOutcome, call_index: usize) -> ProcessEventCandidate {
    let title = outcome.title.as_deref().unwrap_or("未命名任务");
    let (status, summary) = match outcome.status.as_str() {
        "created" => ("completed", format!("已创建任务：{}", title)),
        "proposed" => ("completed", format!("已提出任务拆分，等待用户确认：{}", title)),
        "duplicate_skipped" if outcome.error.is_some() || outcome.task_id.is_none() => (
            "error",
            outcome.error.clone().unwrap_or_else(|| format!("重复任务未创建：{}", title)),
        ),
        "duplicate_skipped" => ("skipped", format!("已跳过重复任务：{}", title)),
        _ => ("error", outcome.error.clone().unwrap_or_else(|| format!("创建任务失败：{}", title))),
    };
    let parsed_input = call
        .and_then(|call| serde_json::from_str::<serde_json::Value>(&call.arguments).ok())
        .unwrap_or_else(|| serde_json::json!({ "title": outcome.title, "deadline": outcome.deadline }));
    let mut input = match parsed_input {
        serde_json::Value::Object(input) => serde_json::Value::Object(input),
        raw => serde_json::json!({ "raw": raw }),
    };
    input["name"] = format!("delegated-task-call:{}:{}", call_index, outcome.call_id).into();
    let raw_tool_name = call.map(|call| call.name.as_str()).unwrap_or(CREATE_DELEGATED_TASK_TOOL);

    ProcessEventCandidate {
        event_type: "tool".to_string(),
        tool_name: Some("create_task".to_string()),
        status: Some(status.to_string()),
        summary,
        raw_json: serde_json::json!({
            "description": format!("创建任务：{}", title),
            "input": input,
            "output": if outcome.status == "error" { serde_json::Value::Null } else { serde_json::to_value(outcome).unwrap_or(serde_json::Value::Null) },
            "error": outcome.error,
            "delegatedToolName": raw_tool_name,
            "outcome": outcome,
        }),
    }
}

async fn persist_delegated_task_process_events(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    message_id: &str,
    calls: &[ToolCall],
    outcomes: &[DelegatedTaskOutcome],
) {
    let session_id = format!("delegated-role:{}", message_id);
    for (index, outcome) in outcomes.iter().enumerate() {
        let candidate = delegated_task_process_candidate(calls.get(index), outcome, index);
        persist_process_event(
            conv_pool, conversation_id, message_id, &session_id,
            &candidate.event_type, candidate.tool_name.as_deref(), candidate.status.as_deref(),
            &candidate.summary, &candidate.raw_json, None,
        ).await;
    }
}

async fn execute_delegated_task_calls(main_pool: &DbPool, role_id: &str, calls: &[ToolCall]) -> Vec<DelegatedTaskOutcome> {
    #[derive(serde::Deserialize)]
    struct Args { title: String, #[serde(default)] deadline: Option<String> }
    let mut seen = HashMap::<(String, String), usize>::new();
    let mut results: Vec<DelegatedTaskOutcome> = Vec::with_capacity(calls.len());
    for call in calls {
        if call.name != CREATE_DELEGATED_TASK_TOOL {
            results.push(DelegatedTaskOutcome { call_id: call.id.clone(), status: "error".into(), title: None, deadline: None, task_id: None, error: Some("不允许执行该工具。".into()), warning: None, duplicate_of: None });
            continue;
        }
        let args: Args = match serde_json::from_str(&call.arguments) {
            Ok(args) => args,
            Err(e) => { results.push(DelegatedTaskOutcome { call_id: call.id.clone(), status: "error".into(), title: None, deadline: None, task_id: None, error: Some(format!("任务参数无法解析：{}", e)), warning: None, duplicate_of: None }); continue; }
        };
        let title = args.title.split_whitespace().collect::<Vec<_>>().join(" ");
        if title.is_empty() {
            results.push(DelegatedTaskOutcome { call_id: call.id.clone(), status: "error".into(), title: None, deadline: None, task_id: None, error: Some("任务标题不能为空，本次未创建。".into()), warning: None, duplicate_of: None });
            continue;
        }
        let (deadline, warning, deadline_key) = normalize_delegated_deadline(args.deadline.as_deref());
        let key = (title.to_lowercase(), deadline_key);
        if let Some(first_index) = seen.get(&key).copied() {
            let first = &results[first_index];
            results.push(DelegatedTaskOutcome { call_id: call.id.clone(), status: "duplicate_skipped".into(), title: Some(title), deadline, task_id: first.task_id.clone(), error: first.error.clone(), warning, duplicate_of: Some(first.call_id.clone()) });
            continue;
        }
        seen.insert(key, results.len());
        let input = CreateTaskInput { owner_type: Some(TaskOwnerType::Role), role_id: Some(role_id.to_string()), title: title.clone(), deadline: deadline.clone(), quadrant: None, is_big_rock: None };
        match tasks::create_task(main_pool, &input).await {
            Ok(task) => {
                let pool = main_pool.clone();
                let task_id = task.id.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::services::task_classifier::classify_and_persist(&pool, &task_id).await {
                        tracing::warn!("[delegate] 任务自动分类失败，保留默认 Q2: task_id={} error={}", task_id, e);
                    }
                });
                results.push(DelegatedTaskOutcome { call_id: call.id.clone(), status: "created".into(), title: Some(title), deadline, task_id: Some(task.id), error: None, warning, duplicate_of: None });
            }
            Err(e) => results.push(DelegatedTaskOutcome { call_id: call.id.clone(), status: "error".into(), title: Some(title), deadline, task_id: None, error: Some(format!("任务创建失败：{}", e)), warning, duplicate_of: None }),
        }
    }
    results
}

async fn propose_or_execute_delegated_task_calls(
    main_pool: &DbPool,
    role_id: &str,
    source_conversation_id: Option<&str>,
    task_summary: &str,
    calls: &[ToolCall],
) -> Vec<DelegatedTaskOutcome> {
    #[derive(serde::Deserialize)]
    struct Args {
        title: String,
        #[serde(default)]
        deadline: Option<String>,
    }

    if calls.len() < 2 {
        return execute_delegated_task_calls(main_pool, role_id, calls).await;
    }

    let mut seen = HashMap::<(String, String), String>::new();
    let mut normalized = Vec::<TaskDecompositionItem>::new();
    let mut outcomes = Vec::<DelegatedTaskOutcome>::with_capacity(calls.len());
    for call in calls {
        if call.name != CREATE_DELEGATED_TASK_TOOL {
            outcomes.push(DelegatedTaskOutcome {
                call_id: call.id.clone(),
                status: "error".into(),
                title: None,
                deadline: None,
                task_id: None,
                error: Some("不允许执行该工具。".into()),
                warning: None,
                duplicate_of: None,
            });
            continue;
        }
        let args: Args = match serde_json::from_str(&call.arguments) {
            Ok(args) => args,
            Err(e) => {
                outcomes.push(DelegatedTaskOutcome {
                    call_id: call.id.clone(),
                    status: "error".into(),
                    title: None,
                    deadline: None,
                    task_id: None,
                    error: Some(format!("任务参数无法解析：{}", e)),
                    warning: None,
                    duplicate_of: None,
                });
                continue;
            }
        };
        let title = args.title.split_whitespace().collect::<Vec<_>>().join(" ");
        if title.is_empty() {
            outcomes.push(DelegatedTaskOutcome {
                call_id: call.id.clone(),
                status: "error".into(),
                title: None,
                deadline: None,
                task_id: None,
                error: Some("任务标题不能为空，本次未创建。".into()),
                warning: None,
                duplicate_of: None,
            });
            continue;
        }
        let (deadline, warning, _deadline_key) =
            normalize_delegated_deadline(args.deadline.as_deref());
        let key = (
            title.to_lowercase(),
            deadline.clone().unwrap_or_else(|| "none".to_string()),
        );
        if let Some(first_call_id) = seen.get(&key) {
            outcomes.push(DelegatedTaskOutcome {
                call_id: call.id.clone(),
                status: "duplicate_skipped".into(),
                title: Some(title),
                deadline,
                task_id: None,
                error: None,
                warning,
                duplicate_of: Some(first_call_id.clone()),
            });
            continue;
        }
        seen.insert(key, call.id.clone());
        normalized.push(TaskDecompositionItem {
            title: title.clone(),
            deadline: deadline.clone(),
        });
        outcomes.push(DelegatedTaskOutcome {
            call_id: call.id.clone(),
            status: "proposed".into(),
            title: Some(title),
            deadline,
            task_id: None,
            error: None,
            warning,
            duplicate_of: None,
        });
    }

    if normalized.len() < 2 {
        return execute_delegated_task_calls(main_pool, role_id, calls).await;
    }

    let Some(source_conversation_id) = source_conversation_id else {
        for outcome in &mut outcomes {
            if outcome.status == "proposed" {
                outcome.status = "error".into();
                outcome.error = Some("缺少来源管家会话，拆分提案未保存。".into());
            }
        }
        return outcomes;
    };

    let input = CreateTaskDecompositionProposalInput {
        role_id: role_id.to_string(),
        source_conversation_id: source_conversation_id.to_string(),
        task_summary: task_summary.to_string(),
        items: normalized,
    };
    if let Err(e) = crate::db::task_decomposition::create_proposal(main_pool, &input).await {
        for outcome in &mut outcomes {
            if outcome.status == "proposed" {
                outcome.status = "error".into();
                outcome.error = Some(format!("拆分提案保存失败：{}", e));
            }
        }
    }
    outcomes
}

#[derive(Default)]
struct LocalStreamOutput {
    text: String,
    tool_calls: Vec<ToolCall>,
    errors: Vec<String>,
}

fn record_local_stream_event(output: &mut LocalStreamOutput, event: StreamEvent) {
    match event {
        StreamEvent::Token(token) => output.text.push_str(&token),
        StreamEvent::ToolCall(call) => output.tool_calls.push(call),
        StreamEvent::Error(error) => output.errors.push(error),
        StreamEvent::Thinking(_) | StreamEvent::Done => {}
    }
}

async fn collect_local_stream(
    provider: &Arc<dyn LlmProvider>,
    messages: Vec<ChatCompletionMessage>,
    options: ChatOptions,
) -> LocalStreamOutput {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let call = provider.chat_stream(messages, tx, options);
    tokio::pin!(call);
    let mut output = LocalStreamOutput::default();
    let mut provider_done = false;
    loop {
        if provider_done {
            match rx.recv().await {
                Some(event) => record_local_stream_event(&mut output, event),
                None => break,
            }
            continue;
        }
        tokio::select! {
            result = &mut call => {
                provider_done = true;
                if let Err(error) = result { output.errors.push(error.to_string()); }
            }
            event = rx.recv() => match event {
                Some(event) => record_local_stream_event(&mut output, event),
                None => break,
            }
        }
    }
    output
}

struct DelegatedRoleRun {
    reply: String,
    task_calls: Vec<ToolCall>,
    outcomes: Vec<DelegatedTaskOutcome>,
}

fn delegated_task_authority_summary(outcomes: &[DelegatedTaskOutcome]) -> String {
    if outcomes.is_empty() {
        "本次角色未调用任务创建工具。".to_string()
    } else {
        format!(
            "任务处理权威结果（逐项状态，必须据此向用户陈述）：{}",
            serde_json::to_string(outcomes).unwrap_or_else(|_| "[]".to_string())
        )
    }
}

async fn run_delegated_role_provider(
    provider: &Arc<dyn LlmProvider>,
    main_pool: &DbPool,
    role_id: &str,
    source_conversation_id: Option<&str>,
    task_summary: &str,
    mut messages: Vec<ChatCompletionMessage>,
) -> DelegatedRoleRun {
    let first = collect_local_stream(provider, messages.clone(), ChatOptions {
        disable_thinking: true,
        tools: Some(vec![delegated_task_tool_definition()]),
        tool_choice: None,
    }).await;
    if first.tool_calls.is_empty() {
        let reply = if first.text.trim().is_empty() { "角色未能生成建议，请稍后重试。".to_string() } else { first.text };
        return DelegatedRoleRun { reply, task_calls: Vec::new(), outcomes: Vec::new() };
    }

    let outcomes = propose_or_execute_delegated_task_calls(
        main_pool,
        role_id,
        source_conversation_id,
        task_summary,
        &first.tool_calls,
    )
    .await;
    messages.push(ChatCompletionMessage { role: "assistant".to_string(), content: first.text, reasoning_content: None, tool_calls: Some(first.tool_calls.clone()), tool_call_id: None });
    for (call, outcome) in first.tool_calls.iter().zip(outcomes.iter()) {
        messages.push(ChatCompletionMessage { role: "tool".to_string(), content: serde_json::to_string(outcome).unwrap_or_else(|_| "{\"status\":\"error\"}".to_string()), reasoning_content: None, tool_calls: None, tool_call_id: Some(call.id.clone()) });
    }
    let followup = collect_local_stream(provider, messages, ChatOptions { disable_thinking: true, tools: None, tool_choice: None }).await;
    let reply = if followup.text.trim().is_empty() {
        "角色未能生成后续建议；请以任务处理权威结果为准。".to_string()
    } else {
        followup.text
    };
    DelegatedRoleRun { reply, task_calls: first.tool_calls, outcomes }
}

/// Story 2.3 AC-1 / AC-4 / AC-8: 把任务委派给角色 LLM 处理并返回结果。
/// 实现要点：
/// - 校验 role 存在且 status=active；否则走 AC-8 兜底，不污染角色对话
/// - 角色 LLM 调用本地化 drain，**不** emit `llm:stream`（避免污染管家流，见 Dev Notes 设计决策）
/// - 委派交互写入角色对话历史（AC-4：角色"记得"被委派）
pub async fn execute_delegate_to_role(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    arguments: &str,
) -> (String, DelegationRecord) {
    execute_delegate_to_role_with_source(main_pool, conv_pool, arguments, None).await
}

pub(crate) async fn execute_delegate_to_role_with_source(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    arguments: &str,
    source_conversation_id: Option<&str>,
) -> (String, DelegationRecord) {
    #[derive(serde::Deserialize)]
    struct DelegateArgs {
        target_role_id: String,
        task_summary: String,
        #[serde(default)]
        context: Option<String>,
    }

    let args: DelegateArgs = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("[delegate] 解析参数失败: {} (原始: {})", e, arguments);
            return (
                format!(
                    "参数解析失败：{}。请检查 target_role_id 和 task_summary。",
                    e
                ),
                DelegationRecord {
                    target_role_id: String::new(),
                    target_role_name: "未知".to_string(),
                    target_conversation_id: None,
                    task_summary: String::new(),
                    status: "parse_error".to_string(),
                },
            );
        }
    };

    // AC-8: 校验角色有效性
    let role = match crate::db::roles::get_role(main_pool, &args.target_role_id).await {
        Ok(r) if r.status == "active" => r,
        Ok(r) => {
            tracing::warn!(
                "[delegate] 目标角色 {} 已归档（status={}），拒绝委派",
                args.target_role_id,
                r.status
            );
            return (
                "目标角色已归档，无法委派。请用文字直接告诉用户该角色不可用。".to_string(),
                DelegationRecord {
                    target_role_id: args.target_role_id,
                    target_role_name: r.name,
                    target_conversation_id: None,
                    task_summary: args.task_summary,
                    status: "role_not_found".to_string(),
                },
            );
        }
        Err(_) => {
            tracing::warn!("[delegate] 目标角色 id={} 不存在", args.target_role_id);
            return (
                "目标角色不可用，请用文字直接告诉用户。".to_string(),
                DelegationRecord {
                    target_role_id: args.target_role_id,
                    target_role_name: "未知".to_string(),
                    target_conversation_id: None,
                    task_summary: args.task_summary,
                    status: "role_not_found".to_string(),
                },
            );
        }
    };

    // AC-4: 拼接委派消息写入角色对话
    let context_text = args.context.as_deref().unwrap_or("").trim();
    let user_content = if context_text.is_empty() {
        format!("[管家委派] {}", args.task_summary)
    } else {
        format!(
            "[管家委派] {}\n\n上下文：{}",
            args.task_summary, context_text
        )
    };

    let role_conv =
        match crate::db::conversations::get_or_create_conversation_by_role(conv_pool, &role.id)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[delegate] 获取/创建角色会话失败: {}", e);
                return (
                    "角色处理失败（无法准备会话），请告诉用户稍后再试。".to_string(),
                    DelegationRecord {
                        target_role_id: role.id,
                        target_role_name: role.name,
                        target_conversation_id: None,
                        task_summary: args.task_summary,
                        status: "conv_init_error".to_string(),
                    },
                );
            }
        };

    if let Err(e) = crate::db::conversations::insert_message(
        conv_pool,
        &role_conv.id,
        "user",
        &user_content,
        true,
    )
    .await
    {
        tracing::error!("[delegate] 写入委派 user 消息失败: {}", e);
        return (
            "角色处理失败（无法记录委派），请告诉用户稍后再试。".to_string(),
            DelegationRecord {
                target_role_id: role.id,
                target_role_name: role.name,
                target_conversation_id: Some(role_conv.id.clone()),
                task_summary: args.task_summary,
                status: "user_msg_insert_error".to_string(),
            },
        );
    }

    let provider = match resolve_default_provider(main_pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("[delegate] 获取 provider 失败: {}", e);
            return (
                "角色处理失败（模型未就绪），请告诉用户稍后再试。".to_string(),
                DelegationRecord {
                    target_role_id: role.id,
                    target_role_name: role.name,
                    target_conversation_id: Some(role_conv.id.clone()),
                    task_summary: args.task_summary,
                    status: "provider_error".to_string(),
                },
            );
        }
    };

    let role_assistant_msg = match crate::db::conversations::insert_message(
        conv_pool,
        &role_conv.id,
        "assistant",
        "",
        false,
    )
    .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("[delegate] 创建角色 assistant 占位失败: {}", e);
            return (
                "角色处理失败（无法准备回复），请告诉用户稍后再试。".to_string(),
                DelegationRecord {
                    target_role_id: role.id,
                    target_role_name: role.name,
                    target_conversation_id: Some(role_conv.id.clone()),
                    task_summary: args.task_summary,
                    status: "assistant_msg_insert_error".to_string(),
                },
            );
        }
    };

    // 跑角色 LLM —— 用 build_role_messages 与管家分支同源 system prompt
    let mut role_messages =
        match build_role_messages(conv_pool, main_pool, &role_conv.id, &role.id, &user_content)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("[delegate] 构建角色消息失败: {}", e);
                return (
                    "角色处理失败（无法构造上下文），请告诉用户稍后再试。".to_string(),
                    DelegationRecord {
                        target_role_id: role.id,
                        target_role_name: role.name,
                        target_conversation_id: Some(role_conv.id.clone()),
                        task_summary: args.task_summary,
                        status: "build_messages_error".to_string(),
                    },
                );
            }
        };
    if let Some(system) = role_messages.first_mut() {
        system.content.push_str("\n\n");
        system.content.push_str(DELEGATED_TASK_PROMPT);
    }

    // 本地化 drain，不 emit `llm:stream`；后端结果独立于角色建议，作为权威状态返回管家。
    let run = run_delegated_role_provider(
        &provider,
        main_pool,
        &role.id,
        source_conversation_id,
        &args.task_summary,
        role_messages,
    )
    .await;
    let role_reply = run.reply;
    let task_outcome_summary = delegated_task_authority_summary(&run.outcomes);

    // 持久化角色 assistant 消息
    let content_updated = crate::db::conversations::update_message_content(
        conv_pool,
        &role_assistant_msg.id,
        &role_reply,
    )
    .await
    .map_err(|error| {
        tracing::warn!(conversation_id = %role_conv.id, message_id = %role_assistant_msg.id, %error, "update delegated role assistant content failed");
        error
    })
    .is_ok();
    if content_updated {
        persist_delegated_task_process_events(conv_pool, &role_conv.id, &role_assistant_msg.id, &run.task_calls, &run.outcomes).await;
        if let Err(error) = crate::db::conversations::mark_message_complete(conv_pool, &role_assistant_msg.id).await {
            tracing::warn!(conversation_id = %role_conv.id, message_id = %role_assistant_msg.id, %error, "mark delegated role assistant complete failed");
            if let Err(cleanup_error) = crate::db::conversations::delete_message_process_events(conv_pool, &role_assistant_msg.id).await {
                tracing::warn!(conversation_id = %role_conv.id, message_id = %role_assistant_msg.id, error = %cleanup_error, "cleanup delegated task process events failed");
            }
        }
    } else {
        tracing::warn!(conversation_id = %role_conv.id, message_id = %role_assistant_msg.id, "skip delegated task process events because role assistant message was not finalized");
    }

    // 返回给管家 LLM 的 tool result：明确这是来自哪个角色的回复，引导管家转述
    let tool_result = format!(
        "来自角色「{}」的建议：\n{}\n\n{}\n\n（建议内容仅供转述；任务是否创建及每一项状态必须严格以权威结果为准。）",
        role.name, role_reply, task_outcome_summary
    );
    (
        tool_result,
        DelegationRecord {
            target_role_id: role.id,
            target_role_name: role.name,
            target_conversation_id: Some(role_conv.id),
            task_summary: args.task_summary,
            status: "ok".to_string(),
        },
    )
}

async fn execute_create_role(
    app_handle: &tauri::AppHandle,
    _main_pool: &DbPool,
    conversation_id: &str,
    arguments: &str,
) -> String {
    #[derive(serde::Deserialize)]
    struct CreateRoleArgs {
        name: String,
        icon: Option<String>,
        color: Option<String>,
        goal: Option<String>,
    }

    let args: CreateRoleArgs = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("解析 create_role 参数失败: {} (原始: {})", e, arguments);
            return format!("参数解析失败: {}", e);
        }
    };

    // 校验/归一化 icon 与 color。落到白名单之外的值用 None 回退（前端会用默认值）。
    let normalized_icon = args.icon.as_deref().and_then(|s| {
        let t = s.trim();
        if SUPPORTED_ICONS.iter().any(|w| *w == t) {
            Some(t.to_string())
        } else {
            tracing::warn!("LLM 返回了不在白名单中的 icon: {:?}, 已回退", t);
            None
        }
    });
    let normalized_color = args.color.as_deref().and_then(|s| {
        let t = s.trim().to_ascii_uppercase();
        if SUPPORTED_COLORS.iter().any(|w| w.eq_ignore_ascii_case(&t)) {
            Some(t)
        } else {
            tracing::warn!("LLM 返回了不在白名单中的 color: {:?}, 已回退", s);
            None
        }
    });

    tracing::info!(
        "[propose] 收到角色提议: name={} icon={:?} color={:?} goal_len={}",
        args.name,
        normalized_icon,
        normalized_color,
        args.goal.as_deref().map(|s| s.len()).unwrap_or(0),
    );

    // 不写库，仅向前端发提议事件；前端弹窗由用户确认后再调 roleService.create()
    match app_handle.emit(
        "role:proposed",
        RoleProposedPayload {
            conversation_id: conversation_id.to_string(),
            name: args.name.clone(),
            icon: normalized_icon,
            color: normalized_color,
            goal: args.goal.clone(),
        },
    ) {
        Ok(()) => {
            tracing::info!(
                "[propose] role_proposed_emitted: conv={} name={}",
                conversation_id,
                args.name
            );
            format!("role_proposal_emitted:{}", args.name)
        }
        Err(e) => {
            tracing::error!(
                "[propose] role_proposed_emit_failed: conv={} name={} error={}",
                conversation_id,
                args.name,
                e
            );
            format!("角色提议发送失败：{}", e)
        }
    }
}

/// Story 2.5: 记录用户拒绝了涌现角色建议，写入冷却。
async fn execute_record_emergence_rejection(main_pool: &DbPool, arguments: &str) -> String {
    #[derive(serde::Deserialize)]
    struct RejectArgs {
        domain: String,
    }

    let args: RejectArgs = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(
                "[emergence] 解析 rejection 参数失败: {} (原始: {})",
                e,
                arguments
            );
            return format!("参数解析失败: {}", e);
        }
    };

    let domain = args.domain.trim();
    if domain.is_empty() {
        return "领域描述为空，未记录冷却。".to_string();
    }

    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::db::app_settings::set_emergence_cooldown(main_pool, domain, &now).await {
        tracing::error!("[emergence] 写入冷却记录失败: {}", e);
        return "已记录用户拒绝（存储失败但不影响对话）。".to_string();
    }

    // 顺带清理过期记录
    let _ = crate::db::app_settings::clear_expired_cooldowns(main_pool, 7).await;

    tracing::info!("[emergence] 记录涌现拒绝: domain={}", domain);
    format!(
        "已记录：用户拒绝了「{}」领域的角色建议，7天内不再提议此领域。请自然地继续对话。",
        domain
    )
}

// ========== 方案 A：后备文本检测 + 二次提取 ==========

/// 启发式检测 assistant 文本是否在「假装」执行了角色创建。
/// 当出现典型创建语义的关键词组合时返回 true。
fn looks_like_fake_role_creation(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let strong_signals = [
        "**创建角色**",
        "角色名称：",
        "角色名称:",
        "角色创建成功",
        "已为您创建",
        "已经为您创建",
        "已为你创建",
        "已经为你创建",
        "角色已创建",
        "创建中...",
        "创建中…",
    ];
    if strong_signals.iter().any(|s| text.contains(s)) {
        return true;
    }
    // 弱信号组合：同时出现「创建」/「建好」 + 「角色」 + 「成功」/「完成」
    let has_create = text.contains("创建") || text.contains("建好") || text.contains("建立");
    let has_role = text.contains("角色");
    let has_done = text.contains("成功")
        || text.contains("完成")
        || text.contains("好了")
        || text.contains("OK")
        || text.contains("ok");
    has_create && has_role && has_done
}

/// 兜底解析：从任意文本中尝试抽出 `{ "name":..., "icon":..., "color":..., "goal":... }` 形式的 JSON。
/// 处理 markdown 代码块包裹、前后文字噪音的情况。
fn parse_role_json(text: &str) -> Option<CreateRoleInput> {
    let cleaned = text
        .replace("```json", "")
        .replace("```JSON", "")
        .replace("```", "");
    // 找第一个 '{' 与之后最远的 '}'
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')?;
    if end <= start {
        return None;
    }
    let candidate = &cleaned[start..=end];

    #[derive(serde::Deserialize)]
    struct ExtractedRole {
        name: Option<String>,
        icon: Option<String>,
        color: Option<String>,
        goal: Option<String>,
    }
    let parsed: ExtractedRole = serde_json::from_str(candidate).ok()?;
    let name = parsed.name?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(CreateRoleInput {
        name,
        icon: parsed.icon.filter(|s| !s.trim().is_empty()),
        color: parsed.color.filter(|s| !s.trim().is_empty()),
        goal: parsed.goal.filter(|s| !s.trim().is_empty()),
    })
}

/// 二次 LLM 调用：把对话历史 + 最后那条「假装创建」的 assistant 文本投喂给模型，
/// 要求它只输出一行 JSON，再解析为 CreateRoleInput。
async fn try_fallback_extract_role(
    provider: &Arc<dyn LlmProvider>,
    onboarding_messages: &[ChatCompletionMessage],
    assistant_text: &str,
) -> Option<CreateRoleInput> {
    let mut messages: Vec<ChatCompletionMessage> = Vec::new();
    messages.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: "你是一个严格的 JSON 提取器。仅输出一行 JSON，不要任何解释、前后缀或代码块包裹。"
            .to_string(),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    });
    // 仅保留对话内容（去掉原 onboarding system 提示，避免冲突）
    for m in onboarding_messages.iter().filter(|m| m.role != "system") {
        messages.push(m.clone());
    }
    // 加入这次"假装创建"的 assistant 输出
    messages.push(ChatCompletionMessage {
        role: "assistant".to_string(),
        content: assistant_text.to_string(),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    });
    messages.push(ChatCompletionMessage {
        role: "user".to_string(),
        content: "请从以上对话中提取要创建的角色信息，只输出一行 JSON，键固定为 name/icon/color/goal。\
                  name 必填（中文角色名）；icon 从这些标识符里选一个最贴合的：briefcase/code/chart-bar/palette/pen-tool/book-open/graduation-cap/dumbbell/heart-pulse/leaf/home/users/baby/gamepad-2/music/camera/plane/utensils/coffee/target/sparkles/lightbulb/compass/wallet；color 从这些 hex 里选最贴合：#4F46E5/#0EA5E9/#10B981/#F59E0B/#EF4444/#8B5CF6/#EC4899/#64748B；goal 为一句话。\
                  如果无法确定角色名，输出 {}。".to_string(),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    });

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let provider_clone = provider.clone();
    tokio::spawn(async move {
        if let Err(e) = provider_clone
            .chat_stream(
                messages,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::error!("[fallback] 二次提取流式调用失败: {}", e);
        }
    });

    let mut full = String::new();
    while let Some(ev) = rx.recv().await {
        match ev {
            StreamEvent::Token(t) => full.push_str(&t),
            StreamEvent::Done => break,
            StreamEvent::Error(e) => {
                tracing::error!("[fallback] 二次提取出错: {}", e);
                return None;
            }
            _ => {}
        }
    }

    tracing::info!("[fallback] 二次提取原始输出: {:?}", full);
    let role = parse_role_json(&full);
    if role.is_none() {
        tracing::warn!("[fallback] 二次提取 JSON 解析失败，放弃创建");
    }
    role
}

fn summarize_error(err: &str) -> &str {
    if err.contains("401") || err.contains("无效") {
        "API Key 无效或已过期"
    } else if err.contains("429") || err.contains("额度") {
        "请求额度不足"
    } else if err.contains("超时") || err.contains("timeout") {
        "连接超时"
    } else if err.contains("连接") || err.contains("connect") {
        "无法连接到模型服务"
    } else {
        "模型服务暂时不可用"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn completed_thinking_elapsed_seconds_never_persists_zero() {
        // WHY: a completed thinking segment must not regress from the live 1-second
        // display to a persisted 0-second display when the segment is shorter than one second.
        assert_eq!(completed_thinking_elapsed_seconds(None), 1);
    }

    use super::*;
    use std::collections::VecDeque;

    struct ScriptedDelegateProvider {
        rounds: Mutex<VecDeque<Vec<StreamEvent>>>,
        messages: Mutex<Vec<Vec<ChatCompletionMessage>>>,
    }

    #[async_trait::async_trait]
    impl LlmProvider for ScriptedDelegateProvider {
        async fn test_connection(&self) -> Result<(), AppError> { Ok(()) }
        async fn chat_stream(&self, messages: Vec<ChatCompletionMessage>, tx: mpsc::Sender<StreamEvent>, _options: ChatOptions) -> Result<(), AppError> {
            self.messages.lock().await.push(messages);
            let events = self.rounds.lock().await.pop_front().unwrap_or_default();
            for event in events { let _ = tx.send(event).await; }
            Ok(())
        }
    }

    fn scripted_provider(rounds: Vec<Vec<StreamEvent>>) -> Arc<dyn LlmProvider> {
        Arc::new(ScriptedDelegateProvider { rounds: Mutex::new(rounds.into()), messages: Mutex::new(Vec::new()) })
    }

    fn task_call(id: &str, title: &str, deadline: Option<&str>) -> ToolCall {
        let mut args = serde_json::json!({"title": title});
        if let Some(value) = deadline { args["deadline"] = value.into(); }
        ToolCall { id: id.to_string(), name: CREATE_DELEGATED_TASK_TOOL.to_string(), arguments: args.to_string() }
    }

    #[test]
    fn live_skill_status_exposes_loaded_skill_name() {
        // WHY: UAT must distinguish an actual native Skill load from a generic
        // narration that merely claims a Skill was used.
        let part = serde_json::json!({
            "tool": "skill",
            "state": { "status": "running", "input": { "name": "uat-greeting" } }
        });
        assert_eq!(live_tool_display_name("skill", &part, "skill"), "uat-greeting");
        assert_eq!(live_tool_display_name("bash", &part, "bash"), "bash");
    }

    #[test]
    fn delegated_task_tool_enforces_intent_and_hides_role_id() {
        let tool = delegated_task_tool_definition();
        assert!(tool.parameters["properties"].get("roleId").is_none());
        assert!(tool.parameters["properties"].get("role_id").is_none());
        assert!(DELEGATED_TASK_PROMPT.contains("纯咨询、分析、建议不要调用"));
        assert!(DELEGATED_TASK_PROMPT.contains("多个独立行动"));
        assert!(DELEGATED_TASK_PROMPT.contains("status=created"));
    }

    #[test]
    fn delegated_deadline_is_optional_corrected_or_degraded() {
        assert_eq!(normalize_delegated_deadline(None), (None, None, "none".to_string()));
        let (corrected, warning, _) = normalize_delegated_deadline(Some("2026/07/16 15:00"));
        assert_eq!(corrected.as_deref(), Some("2026-07-16T15:00:00"));
        assert!(warning.unwrap().contains("规范化"));
        let (degraded, warning, invalid_key) = normalize_delegated_deadline(Some("下周找个时间"));
        assert!(degraded.is_none());
        assert!(warning.unwrap().contains("未保存截止时间"));
        assert!(invalid_key.contains("下周找个时间"));
    }

    #[tokio::test]
    async fn delegated_provider_multiple_calls_create_one_proposal_without_tasks() {
        let pool = setup_test_main_pool().await;
        let current = crate::db::roles::create_role(&pool, &CreateRoleInput { name: "产品经理".into(), icon: None, color: None, goal: None }).await.unwrap();
        let other = crate::db::roles::create_role(&pool, &CreateRoleInput { name: "学习者".into(), icon: None, color: None, goal: None }).await.unwrap();
        let calls = vec![
            ToolCall { id: "1".into(), name: CREATE_DELEGATED_TASK_TOOL.into(), arguments: serde_json::json!({"title":" 准备   产品评审 ","deadline":"明天下午","role_id":other.id}).to_string() },
            task_call("2", "准备 产品评审", Some("明天下午")),
            task_call("3", "提交会议纪要", Some("2026-07-17")),
            task_call("4", "准备 产品评审", Some("后天下午")),
            task_call("5", "   ", None),
        ];
        let results = propose_or_execute_delegated_task_calls(
            &pool,
            &current.id,
            Some("conv-butler"),
            "准备产品评审",
            &calls,
        )
        .await;
        assert_eq!(results[0].status, "proposed");
        assert!(results[0].warning.as_deref().unwrap().contains("未保存截止时间"));
        assert_eq!(results[1].status, "duplicate_skipped");
        assert_eq!(results[2].status, "proposed");
        assert_eq!(results[3].status, "duplicate_skipped", "无法保存的截止时间不应制造重复提案项");
        assert_eq!(results[4].status, "error");
        let current_tasks = tasks::list_tasks_by_role(&pool, &current.id).await.unwrap();
        assert!(current_tasks.is_empty(), "多个不同任务必须等待用户确认");
        assert!(tasks::list_tasks_by_role(&pool, &other.id).await.unwrap().is_empty(), "模型角色 ID 不得覆盖当前角色");
        let proposals = crate::db::task_decomposition::list_pending_by_conversation(
            &pool,
            "conv-butler",
        )
        .await
        .unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].proposal.items.len(), 2);
        assert_eq!(proposals[0].proposal.role_id, current.id);
    }

    #[tokio::test]
    async fn delegated_creation_failure_is_diagnostic_and_never_reports_created() {
        let pool = setup_test_main_pool().await;
        let results = execute_delegated_task_calls(&pool, "", &[task_call("1", "准备评审", None), task_call("2", "准备评审", None)]).await;
        assert_eq!(results[0].status, "error");
        assert!(results[0].error.as_deref().unwrap().contains("创建失败"));
        assert_eq!(results[1].status, "duplicate_skipped", "相同失败调用不得再次写库");
        assert_eq!(results[1].error, results[0].error, "重复响应保留首次失败结果");
        let summary = delegated_task_authority_summary(&results);
        assert!(summary.contains("\"status\":\"error\"") && summary.contains("duplicate_skipped"));
        assert!(!summary.contains("\"status\":\"created\""));
    }

    #[tokio::test]
    async fn delegated_provider_drains_tool_and_followup_events_after_done() {
        let pool = setup_test_main_pool().await;
        let role = crate::db::roles::create_role(&pool, &CreateRoleInput { name: "产品经理".into(), icon: None, color: None, goal: None }).await.unwrap();
        let provider = scripted_provider(vec![
            vec![StreamEvent::Done, StreamEvent::ToolCall(task_call("call-1", "准备产品评审", None))],
            vec![StreamEvent::Done, StreamEvent::Token("已整理会议建议。".into())],
        ]);
        let messages = vec![ChatCompletionMessage { role: "system".into(), content: DELEGATED_TASK_PROMPT.into(), reasoning_content: None, tool_calls: None, tool_call_id: None }];

        let run = run_delegated_role_provider(
            &provider,
            &pool,
            &role.id,
            Some("conv-butler"),
            "准备产品评审",
            messages,
        )
        .await;

        assert_eq!(run.outcomes.len(), 1, "provider 完成后排队的 ToolCall 仍必须执行");
        assert_eq!(run.outcomes[0].status, "created");
        assert_eq!(run.reply, "已整理会议建议。", "Done 后排队的收尾 Token 仍必须保留");
        assert_eq!(tasks::list_tasks_by_role(&pool, &role.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delegated_provider_pure_consultation_returns_advice_without_task() {
        let pool = setup_test_main_pool().await;
        let role = crate::db::roles::create_role(&pool, &CreateRoleInput { name: "产品经理".into(), icon: None, color: None, goal: None }).await.unwrap();
        let provider = scripted_provider(vec![vec![StreamEvent::Token("建议先讲目标与取舍。".into()), StreamEvent::Done]]);
        let messages = vec![ChatCompletionMessage { role: "system".into(), content: DELEGATED_TASK_PROMPT.into(), reasoning_content: None, tool_calls: None, tool_call_id: None }];

        let run = run_delegated_role_provider(
            &provider,
            &pool,
            &role.id,
            Some("conv-butler"),
            "咨询产品评审",
            messages,
        )
        .await;

        assert!(run.outcomes.is_empty(), "纯咨询不应生成任务结果");
        assert_eq!(run.reply, "建议先讲目标与取舍。");
        assert!(tasks::list_tasks_by_role(&pool, &role.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delegated_provider_empty_followup_keeps_authoritative_partial_results() {
        let pool = setup_test_main_pool().await;
        let role = crate::db::roles::create_role(&pool, &CreateRoleInput { name: "产品经理".into(), icon: None, color: None, goal: None }).await.unwrap();
        let provider = scripted_provider(vec![
            vec![StreamEvent::ToolCall(task_call("ok", "准备产品评审", None)), StreamEvent::ToolCall(task_call("bad", "   ", None))],
            vec![StreamEvent::Error("模型收尾失败".into())],
        ]);
        let messages = vec![ChatCompletionMessage { role: "system".into(), content: DELEGATED_TASK_PROMPT.into(), reasoning_content: None, tool_calls: None, tool_call_id: None }];

        let run = run_delegated_role_provider(
            &provider,
            &pool,
            &role.id,
            Some("conv-butler"),
            "准备产品评审",
            messages,
        )
        .await;
        let summary = delegated_task_authority_summary(&run.outcomes);

        assert!(run.reply.contains("权威结果"), "收尾失败必须有确定性 fallback");
        assert!(summary.contains("\"status\":\"created\"") && summary.contains("\"status\":\"error\""));
        assert_eq!(tasks::list_tasks_by_role(&pool, &role.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delegated_task_process_events_preserve_status_order_and_role_ownership() {
        let conv_pool = setup_test_conv_pool().await;
        let role_conv = crate::db::conversations::create_conversation(&conv_pool, Some("role-pm")).await.unwrap();
        let role_message = crate::db::conversations::insert_message(&conv_pool, &role_conv.id, "assistant", "", false).await.unwrap();
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool).await.unwrap();
        let butler_message = crate::db::conversations::insert_message(&conv_pool, &butler_conv.id, "assistant", "管家回复", true).await.unwrap();
        let calls = vec![
            task_call("ok-1", "准备产品方案", Some("2026-07-16T15:00:00+08:00")),
            task_call("ok-2", "安排产品会议", None),
            task_call("bad", "   ", None),
            task_call("dup", "准备产品方案", Some("2026-07-16T15:00:00+08:00")),
        ];
        let outcomes = vec![
            DelegatedTaskOutcome { call_id: "ok-1".into(), status: "created".into(), title: Some("准备产品方案".into()), deadline: Some("2026-07-16T15:00:00+08:00".into()), task_id: Some("task-1".into()), error: None, warning: None, duplicate_of: None },
            DelegatedTaskOutcome { call_id: "ok-2".into(), status: "created".into(), title: Some("安排产品会议".into()), deadline: None, task_id: Some("task-2".into()), error: None, warning: None, duplicate_of: None },
            DelegatedTaskOutcome { call_id: "bad".into(), status: "error".into(), title: None, deadline: None, task_id: None, error: Some("任务标题不能为空，本次未创建。".into()), warning: None, duplicate_of: None },
            DelegatedTaskOutcome { call_id: "dup".into(), status: "duplicate_skipped".into(), title: Some("准备产品方案".into()), deadline: Some("2026-07-16T15:00:00+08:00".into()), task_id: Some("task-1".into()), error: None, warning: None, duplicate_of: Some("ok-1".into()) },
        ];

        persist_delegated_task_process_events(&conv_pool, &role_conv.id, &role_message.id, &calls, &outcomes).await;

        let events = crate::db::conversations::list_message_process_events(&conv_pool, &role_message.id).await.unwrap();
        assert_eq!(events.len(), 4, "每个任务调用都必须保留独立终态事件");
        assert_eq!(events.iter().map(|event| event.status.as_deref()).collect::<Vec<_>>(), vec![Some("completed"), Some("completed"), Some("error"), Some("skipped")]);
        assert!(events.iter().all(|event| event.tool_name.as_deref() == Some("create_task")));
        assert!(events.iter().all(|event| event.conversation_id == role_conv.id && event.message_id == role_message.id));
        let first_raw: serde_json::Value = serde_json::from_str(&events[0].raw_json).unwrap();
        assert_eq!(first_raw["delegatedToolName"], CREATE_DELEGATED_TASK_TOOL);
        assert_eq!(first_raw["input"]["title"], "准备产品方案");
        assert_eq!(first_raw["input"]["name"], "delegated-task-call:0:ok-1");
        assert_eq!(first_raw["outcome"]["taskId"], "task-1");
        let error_raw: serde_json::Value = serde_json::from_str(&events[2].raw_json).unwrap();
        assert!(error_raw["error"].as_str().unwrap().contains("标题不能为空"));
        let duplicate_raw: serde_json::Value = serde_json::from_str(&events[3].raw_json).unwrap();
        assert_eq!(duplicate_raw["outcome"]["duplicateOf"], "ok-1");
        assert_eq!(duplicate_raw["input"]["name"], "delegated-task-call:3:dup", "相邻同标题调用必须具有独立 UI identity");
        assert!(crate::db::conversations::list_message_process_events(&conv_pool, &butler_message.id).await.unwrap().is_empty(), "角色过程不得写入管家消息");
    }

    #[test]
    fn delegated_duplicate_of_failed_creation_remains_an_error_event() {
        let outcome = DelegatedTaskOutcome {
            call_id: "dup-failed".into(), status: "duplicate_skipped".into(), title: Some("准备评审".into()),
            deadline: None, task_id: None, error: Some("任务创建失败：角色不存在".into()), warning: None,
            duplicate_of: Some("first-failed".into()),
        };

        let candidate = delegated_task_process_candidate(None, &outcome, 0);

        assert_eq!(candidate.status.as_deref(), Some("error"), "重复调用不得掩盖首次写库失败");
        assert!(candidate.summary.contains("角色不存在"));

        let non_object_call = ToolCall { id: String::new(), name: CREATE_DELEGATED_TASK_TOOL.into(), arguments: "[]".into() };
        let first = delegated_task_process_candidate(Some(&non_object_call), &outcome, 2);
        let second = delegated_task_process_candidate(Some(&non_object_call), &outcome, 3);
        assert_eq!(first.raw_json["input"]["raw"], serde_json::json!([]));
        assert_ne!(first.raw_json["input"]["name"], second.raw_json["input"]["name"], "非法参数和重复 call ID 仍须保持独立 UI identity");
    }

    #[tokio::test]
    async fn delegated_pure_consultation_persists_no_task_process_event() {
        let conv_pool = setup_test_conv_pool().await;
        let role_conv = crate::db::conversations::create_conversation(&conv_pool, Some("role-pm")).await.unwrap();
        let role_message = crate::db::conversations::insert_message(&conv_pool, &role_conv.id, "assistant", "建议先明确目标。", true).await.unwrap();

        persist_delegated_task_process_events(&conv_pool, &role_conv.id, &role_message.id, &[], &[]).await;

        assert!(crate::db::conversations::list_message_process_events(&conv_pool, &role_message.id).await.unwrap().is_empty(), "纯咨询不应制造任务过程");
    }

    #[test]
    fn test_delegation_bubble_state_routes_final_text_after_tool_starts() {
        let mut state = DelegationBubbleState::default();
        assert_eq!(state.classify_message("msg_A"), BubbleSlot::First);
        assert_eq!(state.classify_message("msg_A"), BubbleSlot::First);
        assert!(state.note_delegate_active());
        assert!(!state.note_delegate_active());
        assert_eq!(state.classify_message("msg_B"), BubbleSlot::Followup);
        assert_eq!(state.classify_message("msg_B"), BubbleSlot::Followup);
    }

    #[test]
    fn test_role_tool_bubble_state_routes_final_text_after_tool_starts() {
        let mut state = DelegationBubbleState::default();

        assert_eq!(state.classify_message("msg_A"), BubbleSlot::First);
        assert!(state.note_role_tool_active());
        assert!(!state.note_role_tool_active());
        assert_eq!(state.classify_message("msg_A"), BubbleSlot::Followup);
        assert_eq!(state.classify_message("msg_B"), BubbleSlot::Followup);
    }

    #[test]
    fn test_delegation_bubble_state_no_split_without_delegation() {
        // WHY: 普通单段对话（无委派）即使 opencode 偶发多条 assistant 消息，
        // 也不得拆分气泡——保持改动前的单气泡语义，避免回归。
        let mut state = DelegationBubbleState::default();
        assert_eq!(state.classify_message("msg_X"), BubbleSlot::First);
        assert_eq!(state.classify_message("msg_Y"), BubbleSlot::First);
    }

    #[test]
    fn test_opencode_agent_key_routes_butler_and_roles() {
        // WHY: butler sessions use the "butler" agent key so opencode enables
        // egosync* MCP tools; roles use a prefixed key.
        assert_eq!(opencode_agent_key(None), "butler");
        assert_eq!(opencode_agent_key(Some("abc-123")), "role-abc-123");
    }

    #[test]
    fn test_opencode_session_cache_key_changes_when_mcp_scope_changes() {
        let calendar_scope = opencode_session_cache_key("D:\\Work\\A", "role:pm|calendar");
        let mail_scope = opencode_session_cache_key("D:\\Work\\A", "role:pm|mail");

        assert_ne!(calendar_scope, mail_scope);
    }

    #[test]
    fn test_remember_opencode_session_reuses_existing_conversation_session() {
        // WHY: one EgoSync conversation must keep one opencode session so the
        // Agent Loop retains context across turns instead of starting fresh.
        let mut sessions = std::collections::HashMap::new();
        let first = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-a",
            "D:\\Work\\A",
            "role:pm|calendar",
        );
        let second = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-b",
            "D:\\Work\\A",
            "role:pm|calendar",
        );

        assert_eq!(first, "session-a");
        assert_eq!(second, "session-a");
        assert_eq!(
            sessions
                .get("conv-1")
                .and_then(|state| state.sessions_by_directory.get(&opencode_session_cache_key("D:\\Work\\A", "role:pm|calendar")))
                .map(String::as_str),
            Some("session-a")
        );
    }

    #[test]
    fn test_remember_opencode_session_creates_new_session_when_working_directory_changes() {
        let mut sessions = std::collections::HashMap::new();
        let first = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-a",
            "D:\\Work\\A",
            "role:pm|calendar",
        );
        let reused = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-b",
            "D:\\Work\\A",
            "role:pm|calendar",
        );
        let changed = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-c",
            "D:\\Work\\B",
            "role:pm|calendar",
        );

        assert_eq!(first, "session-a");
        assert_eq!(reused, "session-a");
        assert_eq!(changed, "session-c");
    }

    #[test]
    fn test_remember_opencode_session_reuses_previous_directory_after_switching_back() {
        let mut sessions = std::collections::HashMap::new();
        let first = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-a",
            "D:\\Work\\A",
            "role:pm|calendar",
        );
        let changed = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-b",
            "D:\\Work\\B",
            "role:pm|calendar",
        );
        let switched_back = remember_opencode_session_for_directory(
            &mut sessions,
            "conv-1",
            "session-c",
            "D:\\Work\\A",
            "role:pm|calendar",
        );

        assert_eq!(first, "session-a");
        assert_eq!(changed, "session-b");
        assert_eq!(switched_back, "session-a");
        assert_eq!(
            sessions.get("conv-1").map(|state| state.active_session_id.as_str()),
            Some("session-a")
        );
    }

    #[test]
    fn test_sse_text_maps_to_stream_payload_contract() {
        // WHY: ChatStream depends on this exact payload shape; changing it would
        // break streaming without any frontend compile error.
        let payload = stream_payload_from_sse(
            "conv-1",
            Some("msg-1"),
            crate::models::agent::SseEvent::Text {
                content: "hello".to_string(),
            },
        )
        .expect("text should map to payload");

        assert_eq!(payload.conversation_id, "conv-1");
        assert_eq!(payload.token, "hello");
        assert!(!payload.done);
        assert!(!payload.thinking);
        assert_eq!(payload.message_id.as_deref(), Some("msg-1"));
    }

    #[test]
    fn text_part_description_produces_narration_process_event() {
        let part_raw = serde_json::json!({
            "id": "part-text",
            "type": "text",
            "text": "先检查 markitdown 是否已安装。"
        });

        let candidate = build_narration_process_event_candidate(&part_raw)
            .expect("user-visible text part should produce a narration process event");

        assert_eq!(candidate.event_type, "narration");
        assert_eq!(candidate.tool_name, None);
        assert_eq!(candidate.status, None);
        assert_eq!(candidate.summary, "先检查 markitdown 是否已安装。");
        assert_eq!(candidate.raw_json.get("text").and_then(|v| v.as_str()), Some("先检查 markitdown 是否已安装。"));
        assert!(candidate.raw_json.get("rawPart").is_some());
    }

    #[test]
    fn narration_buffer_flushes_complete_description_before_tool() {
        let mut text = String::new();
        let mut has_tool = false;

        buffer_narration_delta(&mut text, &mut has_tool, "先检查 ");
        buffer_narration_delta(&mut text, &mut has_tool, "markitdown 是否已安装。");

        assert_eq!(text, "先检查 markitdown 是否已安装。");
        assert!(!has_tool);
        has_tool = true;
        assert!(has_tool);
    }

    #[test]
    fn final_narration_flush_is_suppressed_after_tool_process_event() {
        assert!(!should_flush_final_narration(false, "这是一个普通回答。"));
        assert!(!should_flush_final_narration(true, "   "));
        assert!(!should_flush_final_narration(true, "转换完成，查看输出内容。"));
    }

    #[test]
    fn running_tool_starts_one_fixed_deadline_until_terminal_status() {
        // WHY: repeated running updates must not postpone the timeout forever,
        // otherwise a stuck Skill can keep the conversation loading indefinitely.
        let now = Instant::now();
        let mut active = None;

        update_tool_execution_deadline(
            &mut active,
            "part-skill",
            "find-skills",
            "running",
            true,
            now,
        );
        let original_deadline = active
            .as_ref()
            .expect("running tool needs a deadline")
            .deadline;
        update_tool_execution_deadline(
            &mut active,
            "part-skill",
            "find-skills",
            "running",
            true,
            now + Duration::from_secs(10),
        );

        assert_eq!(original_deadline, now + OPENCODE_SKILL_LOAD_TIMEOUT);
        assert_eq!(active.as_ref().unwrap().deadline, original_deadline);

        update_tool_execution_deadline(
            &mut active,
            "part-skill",
            "find-skills",
            "completed",
            true,
            now + Duration::from_secs(20),
        );
        assert!(active.is_none(), "terminal status must cancel the timeout");
    }

    #[test]
    fn unrelated_tool_completion_does_not_clear_active_deadline() {
        // WHY: opencode may report multiple tool parts; only the matching part
        // may cancel the guard protecting a still-running Skill.
        let now = Instant::now();
        let mut active = None;
        update_tool_execution_deadline(&mut active, "part-skill", "find-skills", "running", true, now);
        update_tool_execution_deadline(&mut active, "part-other", "read", "failed", false, now);

        assert_eq!(
            active.as_ref().map(|item| item.part_id.as_str()),
            Some("part-skill")
        );
    }

    #[test]
    fn only_read_only_skill_load_retries_once() {
        // WHY: automatic replay is safe only for the native read-only Skill loader;
        // side-effecting tools and a second stalled attempt must fail explicitly.
        assert!(should_retry_skill_load_timeout(true, false));
        assert!(!should_retry_skill_load_timeout(true, true));
        assert!(!should_retry_skill_load_timeout(false, false));
    }

    #[test]
    fn tool_timeout_candidate_is_explicit_and_actionable() {
        // WHY: timeout must become a persisted failed process event instead of
        // silently leaving the UI spinner and assistant message incomplete.
        let candidate = tool_execution_timeout_candidate("find-skills", OPENCODE_SKILL_LOAD_TIMEOUT);

        assert_eq!(candidate.event_type, "tool");
        assert_eq!(candidate.tool_name.as_deref(), Some("find-skills"));
        assert_eq!(candidate.status.as_deref(), Some("failed"));
        assert_eq!(
            candidate.raw_json.get("error").and_then(|value| value.as_str()),
            Some("tool execution timeout")
        );
        assert_eq!(
            candidate
                .raw_json
                .get("timeoutSeconds")
                .and_then(|value| value.as_u64()),
            Some(OPENCODE_SKILL_LOAD_TIMEOUT.as_secs())
        );
        assert!(
            tool_execution_timeout_message("find-skills").contains("Skill 安装路径是否重复")
        );
    }

    #[test]
    fn tool_process_event_extracts_bash_failure_details() {
        let part_raw = serde_json::json!({
            "id": "part-shell",
            "type": "tool",
            "tool": "bash",
            "state": {
                "status": "failed",
                "input": { "command": "npm run test:frontend" },
                "output": "stdout text",
                "error": "exit code 1"
            }
        });

        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("failed shell tool part should produce a process event");

        assert_eq!(candidate.event_type, "tool");
        assert_eq!(candidate.tool_name.as_deref(), Some("bash"));
        assert_eq!(candidate.status.as_deref(), Some("failed"));
        assert!(candidate.summary.contains("bash"));
        assert!(candidate.summary.contains("失败"));
        assert_eq!(candidate.raw_json.get("tool").and_then(|v| v.as_str()), Some("bash"));
        assert_eq!(candidate.raw_json.get("status").and_then(|v| v.as_str()), Some("failed"));
        assert_eq!(candidate.raw_json.get("command").and_then(|v| v.as_str()), Some("npm run test:frontend"));
        assert_eq!(candidate.raw_json.get("output").and_then(|v| v.as_str()), Some("stdout text"));
        assert_eq!(candidate.raw_json.get("error").and_then(|v| v.as_str()), Some("exit code 1"));
        assert!(candidate.raw_json.get("input").is_some());
        assert!(candidate.raw_json.get("rawPart").is_some());
    }

    #[test]
    fn tool_process_event_extracts_completed_input_output() {
        let part_raw = serde_json::json!({
            "id": "part-read",
            "type": "tool",
            "tool": "read",
            "state": {
                "status": "completed",
                "input": { "file_path": "D:\\Workspace\\a.ts" },
                "output": "file content"
            }
        });

        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("completed read tool part should produce a process event");

        assert_eq!(candidate.event_type, "tool");
        assert_eq!(candidate.tool_name.as_deref(), Some("read"));
        assert_eq!(candidate.status.as_deref(), Some("completed"));
        assert_eq!(candidate.raw_json.get("tool").and_then(|v| v.as_str()), Some("read"));
        assert_eq!(candidate.raw_json.get("status").and_then(|v| v.as_str()), Some("completed"));
        assert_eq!(candidate.raw_json.get("output").and_then(|v| v.as_str()), Some("file content"));
        assert_eq!(
            candidate.raw_json
                .get("input")
                .and_then(|v| v.get("file_path"))
                .and_then(|v| v.as_str()),
            Some("D:\\Workspace\\a.ts")
        );
        assert!(candidate.raw_json.get("error").is_none());
    }

    #[test]
    fn tool_process_event_formats_mcp_tool_name_with_server_namespace() {
        let part_raw = serde_json::json!({
            "id": "part-mcp",
            "type": "tool",
            "tool": "12306-mcp_get-tickets",
            "state": {
                "status": "completed",
                "input": { "from": "深圳", "to": "汕头" },
                "output": "G638 深圳北 08:12 汕头 10:58"
            }
        });

        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("MCP tool part should produce a process event");

        assert_eq!(candidate.tool_name.as_deref(), Some("12306-mcp:get-tickets"));
        assert!(candidate.summary.contains("12306-mcp:get-tickets"));
        assert!(!candidate.summary.contains("12306-mcp_get-tickets"));
        assert_eq!(
            candidate.raw_json.get("tool").and_then(|v| v.as_str()),
            Some("12306-mcp:get-tickets")
        );
        assert_eq!(
            candidate
                .raw_json
                .get("rawPart")
                .and_then(|v| v.get("tool"))
                .and_then(|v| v.as_str()),
            Some("12306-mcp_get-tickets")
        );
    }

    #[test]
    fn tool_process_event_formats_mcp_tool_name_in_error_summary() {
        let part_raw = serde_json::json!({
            "id": "part-mcp-error",
            "type": "tool",
            "tool": "12306-mcp_get-tickets",
            "state": {
                "status": "error",
                "input": { "from": "深圳", "to": "汕头" },
                "error": "HTTP 500"
            }
        });

        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("failed MCP tool part should produce a process event");

        assert_eq!(candidate.tool_name.as_deref(), Some("12306-mcp:get-tickets"));
        assert!(candidate.summary.contains("12306-mcp:get-tickets"));
        assert!(!candidate.summary.contains("12306-mcp_get-tickets"));
        assert_eq!(
            candidate.raw_json.get("tool").and_then(|v| v.as_str()),
            Some("12306-mcp:get-tickets")
        );
    }

    #[test]
    fn tool_process_event_uses_mcp_display_map_for_weather_namespaces() {
        let display_map = McpToolDisplayMap::from_servers(&[crate::models::mcp::McpServer {
            id: "96691589-a73c-4cea-9db2-1130c875afcd".to_string(),
            name: "天气查询".to_string(),
            server_type: "sse".to_string(),
            command_or_url: "https://mcp.example/weather".to_string(),
            env_refs: "{}".to_string(),
            description: String::new(),
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        }]);

        for raw_tool in ["_____maps_weather", "96691589-a73c-4cea-9db2-1130c875afcd_maps_weather"] {
            let part_raw = serde_json::json!({
                "id": format!("part-{raw_tool}"),
                "type": "tool",
                "tool": raw_tool,
                "state": {
                    "status": "error",
                    "input": { "city": "深圳" },
                    "error": "Invalid session id"
                }
            });

            let candidate = build_tool_process_event_candidate_with_display_map(&part_raw, &display_map)
                .expect("weather MCP tool part should produce a process event");

            assert_eq!(candidate.tool_name.as_deref(), Some("天气查询:maps_weather"));
            assert!(candidate.summary.contains("天气查询:maps_weather"));
            assert!(!candidate.summary.contains(raw_tool));
            assert_eq!(
                candidate.raw_json.get("tool").and_then(|v| v.as_str()),
                Some("天气查询:maps_weather")
            );
            assert_eq!(
                candidate
                    .raw_json
                    .get("rawPart")
                    .and_then(|v| v.get("tool"))
                    .and_then(|v| v.as_str()),
                Some(raw_tool)
            );
        }
    }

    #[test]
    fn invalid_mcp_session_tool_error_requests_runtime_refresh_retry_once() {
        let part_raw = serde_json::json!({
            "id": "part-weather-error",
            "type": "tool",
            "tool": "_____maps_weather",
            "state": {
                "status": "error",
                "input": { "city": "深圳" },
                "error": "HTTP 404: Invalid OAuth error response. Raw body: {\"error\":{\"message\":\"Invalid session id.\"}}"
            }
        });
        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("invalid MCP session tool part should still parse as a process event candidate");

        assert_eq!(mcp_session_retry_decision(&candidate, false), McpSessionRetryDecision::RefreshRuntimeAndRetry);
        assert_eq!(mcp_session_retry_decision(&candidate, true), McpSessionRetryDecision::DoNotRetry);
    }

    #[test]
    fn non_session_tool_error_does_not_request_runtime_refresh_retry() {
        let part_raw = serde_json::json!({
            "id": "part-weather-error",
            "type": "tool",
            "tool": "_____maps_weather",
            "state": {
                "status": "error",
                "input": { "city": "深圳" },
                "error": "HTTP 500: upstream unavailable"
            }
        });
        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("generic MCP tool error should still parse as a process event candidate");

        assert_eq!(mcp_session_retry_decision(&candidate, false), McpSessionRetryDecision::DoNotRetry);
    }

    #[test]
    fn tool_process_event_keeps_builtin_underscore_tool_name() {
        let part_raw = serde_json::json!({
            "id": "part-role",
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "running",
                "input": { "roleId": "father" }
            }
        });

        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("builtin tool part should produce a process event");

        assert_eq!(candidate.tool_name.as_deref(), Some("delegate_to_role"));
        assert!(candidate.summary.contains("delegate_to_role"));
        assert_eq!(
            candidate.raw_json.get("tool").and_then(|v| v.as_str()),
            Some("delegate_to_role")
        );
    }

    #[test]
    fn tool_process_event_normalizes_read_file_path_alias() {
        let part_raw = serde_json::json!({
            "id": "part-read",
            "type": "tool",
            "tool": "read",
            "state": {
                "status": "running",
                "input": { "filePath": "D:\\Workspace\\alias.md" }
            }
        });

        let candidate = build_tool_process_event_candidate(&part_raw)
            .expect("read tool part should produce a process event");

        assert_eq!(
            candidate.raw_json
                .get("input")
                .and_then(|v| v.get("file_path"))
                .and_then(|v| v.as_str()),
            Some("D:\\Workspace\\alias.md")
        );
    }

    #[test]
    fn test_sse_done_maps_to_stream_completion() {
        // WHY: the frontend only unlocks the input after a done payload or
        // history refresh; opencode completion must preserve that contract.
        let payload = stream_payload_from_sse("conv-1", None, crate::models::agent::SseEvent::Done)
            .expect("done should map to payload");

        assert_eq!(payload.token, "");
        assert!(payload.done);
        assert!(!payload.thinking);
        assert!(payload.message_id.is_none());
    }

    #[test]
    fn test_sse_thinking_maps_to_user_visible_payload() {
        let payload = stream_payload_from_sse(
            "conv-1",
            Some("msg-1"),
            crate::models::agent::SseEvent::Thinking {
                content: "plan".to_string(),
            },
        )
        .expect("thinking should map to payload");

        assert_eq!(payload.token, "plan");
        assert!(!payload.done);
        assert!(payload.thinking);
        assert_eq!(payload.phase.as_deref(), Some("thinking"));
        assert_eq!(payload.status_text.as_deref(), Some("思考中..."));
    }

    #[test]
    fn test_bus_delta_waits_for_part_type_before_emitting_unknown_part() {
        let visible_text_parts = std::collections::HashSet::new();
        let reasoning_parts = std::collections::HashSet::new();
        let properties = serde_json::json!({
            "field": "text",
            "partID": "part-1",
            "delta": "pending text"
        });

        assert_eq!(
            classify_bus_text_delta(&properties, &visible_text_parts, &reasoning_parts),
            BusTextDeltaKind::WaitForPartType
        );
    }

    #[test]
    fn test_bus_delta_emits_thinking_when_reasoning_type_arrives_on_delta() {
        let visible_text_parts = std::collections::HashSet::new();
        let reasoning_parts = std::collections::HashSet::new();
        let reasoning_properties = serde_json::json!({
            "field": "text",
            "partID": "part-1",
            "partType": "reasoning",
            "delta": "visible reasoning"
        });
        let thinking_properties = serde_json::json!({
            "field": "text",
            "partID": "part-2",
            "part": { "type": "thinking" },
            "delta": "visible thinking"
        });

        assert_eq!(
            classify_bus_text_delta(&reasoning_properties, &visible_text_parts, &reasoning_parts),
            BusTextDeltaKind::Thinking
        );
        assert_eq!(
            classify_bus_text_delta(&thinking_properties, &visible_text_parts, &reasoning_parts),
            BusTextDeltaKind::Thinking
        );
    }

    #[test]
    fn test_bus_delta_emits_known_visible_text_part() {
        let visible_text_parts = std::collections::HashSet::from(["part-1".to_string()]);
        let reasoning_parts = std::collections::HashSet::new();
        let properties = serde_json::json!({
            "field": "text",
            "partID": "part-1",
            "delta": "visible text"
        });

        assert_eq!(
            classify_bus_text_delta(&properties, &visible_text_parts, &reasoning_parts),
            BusTextDeltaKind::Text
        );
    }

    #[test]
    fn test_sse_error_maps_to_done_payload() {
        // WHY: opencode stream errors must still close the streaming UI so the
        // user can recover and send the next message.
        let payload = stream_payload_from_sse(
            "conv-1",
            None,
            crate::models::agent::SseEvent::Error {
                message: "timeout".to_string(),
            },
        )
        .expect("error should map to payload");

        assert!(payload.done);
        assert!(!payload.thinking);
        assert!(payload.token.contains("Agent 引擎返回错误"));
    }

    #[test]
    fn non_json_tool_output_preview_truncates_on_char_boundary() {
        let output = "产品经理已经收到委派任务，并给出了真实回复。";
        let preview = non_json_tool_output_preview(output);

        assert_eq!(preview, output);
    }

    #[test]
    fn test_butler_system_prompt_not_empty() {
        assert!(!BUTLER_SYSTEM_PROMPT.is_empty());
        assert!(BUTLER_SYSTEM_PROMPT.contains("管家"));
    }

    #[test]
    fn test_onboarding_prompt_defined() {
        assert!(!ONBOARDING_SYSTEM_PROMPT.is_empty());
        assert!(ONBOARDING_SYSTEM_PROMPT.contains("引导"));
        assert!(ONBOARDING_SYSTEM_PROMPT.contains("create_role"));
        assert!(ONBOARDING_SYSTEM_PROMPT.contains("行为红线"));
    }

    #[test]
    fn test_onboarding_options_respect_pre_tool_and_forced_tool_boundaries() {
        // WHY: steps 1-2 gather context without tools; every supported step from 3 onward
        // must avoid DeepSeek's Thinking + required-tool conflict while still forcing create_role.
        for step in [1, 2] {
            let options = get_onboarding_chat_options(step);
            assert!(!options.disable_thinking, "step {step}");
            assert!(options.tools.is_none(), "step {step}");
            assert!(options.tool_choice.is_none(), "step {step}");
        }
        for step in [3, 4, 5] {
            let options = get_onboarding_chat_options(step);
            assert!(options.disable_thinking, "step {step}");
            assert_eq!(options.tool_choice.as_deref(), Some("required"), "step {step}");
            assert_eq!(options.tools.as_ref().map(Vec::len), Some(1), "step {step}");
        }
    }

    #[test]
    fn test_butler_tools_keep_thinking_without_forced_choice() {
        // WHY: only onboarding is downgraded; normal tools retain autonomous Thinking selection.
        let options = get_butler_chat_options();
        assert!(!options.disable_thinking);
        assert!(options.tools.is_some());
        assert!(options.tool_choice.is_none());
    }

    #[tokio::test]
    async fn test_thinking_chunks_reach_actual_tool_followup_messages() {
        // WHY: all Thinking chunks that precede a tool call must be preserved verbatim on
        // the exact assistant message sent in the provider's follow-up request.
        let provider = Arc::new(ScriptedDelegateProvider {
            rounds: Mutex::new(vec![
                vec![StreamEvent::Thinking("第一段 ".into()), StreamEvent::Thinking("第二段".into()), StreamEvent::ToolCall(ToolCall { id: "call-1".into(), name: "create_role".into(), arguments: "{}".into() }), StreamEvent::Done],
                vec![StreamEvent::Done],
            ].into()),
            messages: Mutex::new(Vec::new()),
        });
        let (tx, mut rx) = mpsc::channel(16);
        provider.chat_stream(Vec::new(), tx, ChatOptions::default()).await.unwrap();
        let mut reasoning = String::new();
        let mut tool_calls = Vec::new();
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Thinking(chunk) => reasoning.push_str(&chunk),
                StreamEvent::ToolCall(call) => tool_calls.push(call),
                StreamEvent::Done => break,
                _ => {}
            }
        }
        let followup = build_tool_followup_messages(Vec::new(), String::new(), &reasoning, &tool_calls, &["ok".into()]);
        let (tx2, _rx2) = mpsc::channel(16);
        provider.chat_stream(followup, tx2, ChatOptions { disable_thinking: true, tools: None, tool_choice: None }).await.unwrap();

        let recorded = provider.messages.lock().await;
        let assistant = &recorded[1][0];
        assert_eq!(assistant.reasoning_content.as_deref(), Some("第一段 第二段"));
        assert_eq!(assistant.tool_calls.as_ref().map(Vec::len), Some(1));
        assert_eq!(recorded[1][1].tool_call_id.as_deref(), Some("call-1"));

        let whitespace = assistant_tool_call_message(String::new(), "  \n\t", tool_calls);
        assert!(whitespace.reasoning_content.is_none());
    }

    #[test]
    fn test_summarize_error_categories() {
        assert_eq!(summarize_error("401 Unauthorized"), "API Key 无效或已过期");
        assert_eq!(
            summarize_error("429 Too Many Requests 额度不足"),
            "请求额度不足"
        );
        assert_eq!(summarize_error("连接超时"), "连接超时");
        assert_eq!(
            summarize_error("无法连接到 api.openai.com"),
            "无法连接到模型服务"
        );
        assert_eq!(summarize_error("unknown error"), "模型服务暂时不可用");
    }

    #[test]
    fn test_create_role_tool_definition() {
        let tool = create_role_tool_definition();
        assert_eq!(tool.name, "create_role");
        assert!(tool.description.contains("创建"));
        let params = tool.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["name"].is_object());
        assert!(params["properties"]["icon"].is_object());
        assert!(params["properties"]["color"].is_object());
        assert!(params["properties"]["goal"].is_object());
    }

    // ===== 方案 A：兜底逻辑相关测试 =====

    #[test]
    fn test_looks_like_fake_role_creation_strong_signals() {
        // 用户实际遇到的伪造文本
        let fake1 = "**创建角色**\n角色名称：产品经理\n创建中...\n角色创建成功";
        assert!(looks_like_fake_role_creation(fake1));

        assert!(looks_like_fake_role_creation(
            "好的，我已为您创建「健身教练」角色。"
        ));
        assert!(looks_like_fake_role_creation("已经为你创建好了"));
        assert!(looks_like_fake_role_creation("角色已创建，祝你顺利"));
        assert!(looks_like_fake_role_creation("正在创建中…"));
    }

    #[test]
    fn test_looks_like_fake_role_creation_weak_combo() {
        // 弱信号组合（创建 + 角色 + 成功/完成）
        assert!(looks_like_fake_role_creation(
            "好的，我帮你建立这个角色，已经完成。"
        ));
        assert!(looks_like_fake_role_creation("角色创建成功！"));
    }

    #[test]
    fn test_looks_like_fake_role_creation_negative_cases() {
        // 正常 onboarding 引导语不应触发
        assert!(!looks_like_fake_role_creation(""));
        assert!(!looks_like_fake_role_creation("你好，请问怎么称呼你？"));
        assert!(!looks_like_fake_role_creation(
            "你最近在忙什么？工作还是生活方面有什么特别关注的事情？"
        ));
        assert!(!looks_like_fake_role_creation(
            "听起来你在产品方面投入很多，我帮你创建一个「产品经理」角色来管理相关事务，怎么样？"
        ));
        // 提议但还没说"成功/完成"，不应触发
        assert!(!looks_like_fake_role_creation("我建议创建一个角色"));
    }

    #[test]
    fn test_parse_role_json_plain() {
        let s = r##"{"name":"产品经理","icon":"📋","color":"#4F46E5","goal":"打磨产品"}"##;
        let r = parse_role_json(s).expect("应能解析");
        assert_eq!(r.name, "产品经理");
        assert_eq!(r.icon.as_deref(), Some("📋"));
        assert_eq!(r.color.as_deref(), Some("#4F46E5"));
        assert_eq!(r.goal.as_deref(), Some("打磨产品"));
    }

    #[test]
    fn test_parse_role_json_with_markdown_fence() {
        let s = "```json\n{\"name\":\"健身教练\",\"icon\":\"💪\",\"color\":\"#10B981\",\"goal\":\"保持健康\"}\n```";
        let r = parse_role_json(s).expect("应能解析含 fence 的 JSON");
        assert_eq!(r.name, "健身教练");
        assert_eq!(r.icon.as_deref(), Some("💪"));
    }

    #[test]
    fn test_parse_role_json_with_surrounding_text() {
        let s = "好的，这是提取结果：\n{\"name\":\"父亲\",\"icon\":\"👨\",\"color\":\"#F59E0B\",\"goal\":\"陪伴家人\"}\n谢谢。";
        let r = parse_role_json(s).expect("应能从前后噪音里抠出 JSON");
        assert_eq!(r.name, "父亲");
    }

    #[test]
    fn test_parse_role_json_only_name() {
        let s = r##"{"name":"读者"}"##;
        let r = parse_role_json(s).expect("仅 name 也应成功");
        assert_eq!(r.name, "读者");
        assert!(r.icon.is_none());
        assert!(r.color.is_none());
        assert!(r.goal.is_none());
    }

    #[test]
    fn test_parse_role_json_empty_name_rejected() {
        let s = r##"{"name":"   "}"##;
        assert!(parse_role_json(s).is_none());
    }

    #[test]
    fn test_parse_role_json_empty_object_rejected() {
        // 二次提取协议里规定无法确定时输出 {}，应被拒绝
        assert!(parse_role_json("{}").is_none());
    }

    #[test]
    fn test_parse_role_json_garbage_rejected() {
        assert!(parse_role_json("not a json").is_none());
        assert!(parse_role_json("").is_none());
        assert!(parse_role_json("{").is_none());
    }

    #[test]
    fn test_parse_role_json_trims_blank_optional_fields() {
        let s = r##"{"name":"作家","icon":"","color":"  ","goal":"写作"}"##;
        let r = parse_role_json(s).expect("应能解析");
        assert_eq!(r.name, "作家");
        assert!(r.icon.is_none(), "空 icon 应被过滤为 None");
        assert!(r.color.is_none(), "空白 color 应被过滤为 None");
        assert_eq!(r.goal.as_deref(), Some("写作"));
    }

    // ===== AC-2 / AC-6: 角色 system prompt 注入 =====

    use crate::db::pool::ConversationsPool;
    use crate::models::role::CreateRoleInput;
    use crate::models::task::CreateTaskInput;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn setup_test_main_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test main db");

        sqlx::query(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT '🎯',
                color TEXT NOT NULL DEFAULT '#6366F1',
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
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create roles table");

        sqlx::raw_sql(include_str!("../../migrations/004_memories.sql"))
            .execute(&pool)
            .await
            .expect("failed to create memories table");
        sqlx::raw_sql(include_str!(
            "../../migrations/005_memory_role_scoped_dedupe.sql"
        ))
        .execute(&pool)
        .await
        .expect("failed to migrate memory dedupe index");
        sqlx::raw_sql(include_str!(
            "../../migrations/006_memory_single_owner_dedupe.sql"
        ))
        .execute(&pool)
        .await
        .expect("failed to migrate memory single-owner dedupe");
        sqlx::raw_sql(include_str!(
            "../../migrations/007_forgotten_memory_sources.sql"
        ))
        .execute(&pool)
        .await
        .expect("failed to create forgotten memory sources");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create app_settings table");
        sqlx::raw_sql(include_str!("../../migrations/008_skills_registry.sql"))
            .execute(&pool)
            .await
            .expect("failed to create skills table");
        sqlx::raw_sql(include_str!("../../migrations/009_skill_role_bindings.sql"))
            .execute(&pool)
            .await
            .expect("failed to create skill role bindings table");
        sqlx::raw_sql(include_str!("../../migrations/011_mcp_servers.sql"))
            .execute(&pool)
            .await
            .expect("failed to create MCP servers table");
        sqlx::raw_sql(include_str!("../../migrations/012_mcp_server_standard_types.sql"))
            .execute(&pool)
            .await
            .expect("failed to apply MCP server standard types migration");
        sqlx::raw_sql(include_str!("../../migrations/013_tasks.sql"))
            .execute(&pool)
            .await
            .expect("failed to create tasks table");
        // Story 3.3: 任务自动分类元数据，agent_engine 测试自建 schema 也需同步。
        sqlx::raw_sql(include_str!("../../migrations/014_task_classification_metadata.sql"))
            .execute(&pool)
            .await
            .expect("failed to apply task classification metadata migration");
        sqlx::raw_sql(include_str!("../../migrations/015_task_owner_scope.sql"))
            .execute(&pool)
            .await
            .expect("failed to apply task owner scope migration");
        sqlx::raw_sql(include_str!("../../migrations/020_mission.sql"))
            .execute(&pool)
            .await
            .expect("failed to create mission table");
        sqlx::raw_sql(include_str!(
            "../../migrations/027_task_decomposition_proposals.sql"
        ))
        .execute(&pool)
        .await
        .expect("failed to create task decomposition proposals");

        pool
    }

    async fn setup_test_conv_pool() -> ConversationsPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create test conv db");

        let schema = include_str!("../../migrations/002_conversations.sql");
        sqlx::raw_sql(schema)
            .execute(&pool)
            .await
            .expect("failed to apply conv schema");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN thinking_content TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("failed to add thinking_content");
        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("failed to add title");

        sqlx::raw_sql(
            "CREATE TABLE message_process_events (
                id TEXT PRIMARY KEY NOT NULL,
                conversation_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                opencode_session_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                tool_name TEXT,
                status TEXT,
                summary TEXT NOT NULL,
                raw_json TEXT NOT NULL,
                working_directory TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
                FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_message_process_events_message_id ON message_process_events(message_id);
            CREATE INDEX idx_message_process_events_conversation_id ON message_process_events(conversation_id);",
        )
        .execute(&pool)
        .await
        .expect("failed to add message_process_events");

        // Story 2.3: 与 run_conversations_migrations 同步，否则 list_messages SELECT 会爆
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
            .execute(&pool)
            .await
            .expect("failed to add routing_metadata");

        ConversationsPool(pool)
    }

    #[tokio::test]
    async fn test_role_messages_include_role_scoped_tasks() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("推进产品落地".to_string()),
            },
        )
        .await
        .unwrap();
        let other_role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "设计师".to_string(),
                icon: None,
                color: None,
                goal: Some("完善视觉体验".to_string()),
            },
        )
        .await
        .unwrap();
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let role_task = crate::db::tasks::create_task(
            &main_pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(role.id.clone()),
                title: "梳理需求范围".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(false),
            },
        )
        .await
        .unwrap();
        let other_role_task = crate::db::tasks::create_task(
            &main_pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(other_role.id.clone()),
                title: "准备视觉稿".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(false),
            },
        )
        .await
        .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "我有哪些任务")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("[当前角色任务]"));
        assert!(system.contains("梳理需求范围"));
        // WHY: natural-language completion/deletion must receive the authoritative
        // task UUID, while role-scoped context must not leak another role's UUID.
        assert!(system.lines().any(|line| {
            line.contains(&format!("id={}", role_task.id)) && line.contains("梳理需求范围")
        }));
        assert!(!system.contains("准备视觉稿"));
        assert!(!system.contains(&other_role_task.id));
        assert!(system.contains(
            "任务 ID 仅用于 complete_task / delete_task 工具参数，不得在自然语言回复中展示"
        ));
    }

    #[tokio::test]
    async fn test_butler_task_summary_keeps_unfinished_and_caps_completed_tasks() {
        let main_pool = setup_test_main_pool().await;

        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("推进产品落地".to_string()),
            },
        )
        .await
        .unwrap();
        let other_role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "运营".to_string(),
                icon: None,
                color: None,
                goal: Some("稳定项目节奏".to_string()),
            },
        )
        .await
        .unwrap();

        let first_unfinished = crate::db::tasks::create_task(
            &main_pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(role.id.clone()),
                title: "未完成任务 1".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(false),
            },
        )
        .await
        .unwrap();
        let second_unfinished = crate::db::tasks::create_task(
            &main_pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(role.id.clone()),
                title: "未完成任务 2".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(false),
            },
        )
        .await
        .unwrap();
        let mut completed_48 = None;
        let mut completed_49 = None;
        for index in 1..=50 {
            let task = crate::db::tasks::create_task(
                &main_pool,
                &CreateTaskInput {
                    owner_type: None,
                    role_id: Some(role.id.clone()),
                    title: format!("已完成任务 {:02}", index),
                    deadline: None,
                    quadrant: Some("Q3".to_string()),
                    is_big_rock: Some(false),
                },
            )
            .await
            .unwrap();
            if index == 48 {
                completed_48 = Some(task.id.clone());
            } else if index == 49 {
                completed_49 = Some(task.id.clone());
            }
            sqlx::query(
                "UPDATE tasks SET is_completed = 1, completed_at = '2026-06-16T00:00:00Z' WHERE id = ?1",
            )
            .bind(&task.id)
            .execute(&main_pool)
            .await
            .unwrap();
        }
        let other_role_task = crate::db::tasks::create_task(
            &main_pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(other_role.id.clone()),
                title: "运营待办 1".to_string(),
                deadline: None,
                quadrant: Some("Q1".to_string()),
                is_big_rock: Some(true),
            },
        )
        .await
        .unwrap();

        let summary = build_butler_task_summary(&main_pool).await.unwrap();

        assert!(summary.contains("[各角色任务]"));
        assert!(summary.contains("产品经理"));
        assert!(summary.contains("运营"));
        assert!(summary.contains("未完成任务 1"));
        assert!(summary.contains("未完成任务 2"));
        assert!(summary.contains("运营待办 1"));
        // WHY: every injected task must remain addressable by complete_task/delete_task.
        assert!(summary.lines().any(|line| {
            line.contains(&format!("id={}", first_unfinished.id)) && line.contains("未完成任务 1")
        }));
        assert!(summary.lines().any(|line| {
            line.contains(&format!("id={}", second_unfinished.id)) && line.contains("未完成任务 2")
        }));
        assert!(summary.lines().any(|line| {
            line.contains(&format!("id={}", other_role_task.id)) && line.contains("运营待办 1")
        }));
        assert!(summary.contains("已完成任务 48"));
        assert!(summary.contains(completed_48.as_deref().unwrap()));
        assert!(!summary.contains("已完成任务 49"));
        assert!(!summary.contains(completed_49.as_deref().unwrap()));
        assert!(!summary.contains("已完成任务 50"));
    }

    #[tokio::test]
    async fn test_role_task_summary_includes_all_unfinished_even_above_visible_limit() {
        let main_pool = setup_test_main_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("推进产品落地".to_string()),
            },
        )
        .await
        .unwrap();

        let mut unfinished_51 = None;
        for index in 1..=51 {
            let task = crate::db::tasks::create_task(
                &main_pool,
                &CreateTaskInput {
                    owner_type: None,
                    role_id: Some(role.id.clone()),
                    title: format!("未完成超限任务 {:02}", index),
                    deadline: None,
                    quadrant: Some("Q2".to_string()),
                    is_big_rock: Some(false),
                },
            )
            .await
            .unwrap();
            if index == 51 {
                unfinished_51 = Some(task.id);
            }
        }
        let completed = crate::db::tasks::create_task(
            &main_pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(role.id.clone()),
                title: "不应注入的已完成任务".to_string(),
                deadline: None,
                quadrant: Some("Q3".to_string()),
                is_big_rock: Some(false),
            },
        )
        .await
        .unwrap();
        sqlx::query("UPDATE tasks SET is_completed = 1 WHERE id = ?1")
            .bind(&completed.id)
            .execute(&main_pool)
            .await
            .unwrap();

        let summary = build_role_task_summary(&main_pool, &role.id).await.unwrap();

        assert!(summary.contains("未完成超限任务 01"));
        assert!(summary.contains("未完成超限任务 50"));
        assert!(summary.contains("未完成超限任务 51"));
        assert!(summary.lines().any(|line| {
            line.contains(unfinished_51.as_deref().unwrap()) && line.contains("未完成超限任务 51")
        }));
        assert!(!summary.contains("不应注入的已完成任务"));
        assert!(!summary.contains(&completed.id));
        assert!(summary.contains("另有 1 条已完成任务未注入"));
    }

    /// AC-6: role-aware system prompt 必须含 role.name 与 goal —
    /// 这是 FR-6 「角色个性化语调」的载体。如果 prompt 没有这些字段，
    /// LLM 会退化成通用管家口吻，用户切换角色就感觉不到差异。
    #[tokio::test]
    async fn test_build_role_messages_injects_name_goal_and_personality_layers() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("打磨产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let role = crate::db::roles::update_role(
            &main_pool,
            &role.id,
            &crate::models::role::UpdateRoleInput {
                name: None,
                icon: None,
                color: None,
                goal: None,
                personality_prompt: Some("简洁专业，先判断优先级再给建议".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "你好")
            .await
            .unwrap();

        let system = msgs.first().expect("应至少有 system prompt");
        assert_eq!(system.role, "system");
        assert!(
            system.content.contains("产品经理"),
            "system prompt 必须含角色名"
        );
        assert!(
            system.content.contains("打磨产品节奏"),
            "system prompt 必须含角色目标"
        );
        assert!(
            system.content.contains("简洁专业，先判断优先级再给建议"),
            "system prompt 必须含个性描述"
        );
        assert!(system.content.contains("[role_definition]"));
        assert!(system.content.contains("[context_injection]"));
        assert!(
            !system.content.contains("你是 EgoSync 的分身管家"),
            "角色 prompt 不应含管家身份"
        );
        assert!(
            system.content.contains("分身"),
            "角色 prompt 必须明确是用户的分身"
        );
    }

    #[tokio::test]
    async fn test_build_role_messages_declares_no_mcp_server_for_unbound_role() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "无 MCP 角色".to_string(),
                icon: None,
                color: None,
                goal: Some("只处理内部事项".to_string()),
            },
        )
        .await
        .unwrap();
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "查询天气")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("[MCP Server 使用边界]"));
        assert!(system.contains("当前角色未绑定任何可用的外部 MCP Server"));
        assert!(system.contains("不要声明、推荐或调用外部 MCP 工具"));
    }

    #[tokio::test]
    async fn test_build_role_messages_declares_only_role_bound_mcp_servers() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "天气角色".to_string(),
                icon: None,
                color: None,
                goal: Some("查询天气".to_string()),
            },
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO mcp_servers
             (id, name, server_type, command_or_url, env_refs, description, enabled, created_at, updated_at)
             VALUES ('weather', '天气服务', 'sse', 'https://example.com', '{}', '提供天气查询', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .execute(&main_pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO role_mcp_server_bindings (server_id, role_id, created_at)
             VALUES ('weather', ?1, '2026-01-01T00:00:00Z')",
        )
        .bind(&role.id)
        .execute(&main_pool)
        .await
        .unwrap();
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "查询天气")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("[MCP Server 使用边界]"));
        assert!(system.contains("天气服务"));
        assert!(system.contains("提供天气查询"));
        assert!(system.contains("不要声明或调用未列出的 MCP Server"));
    }

    #[tokio::test]
    async fn test_build_role_messages_injects_meta_skill_config() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("管理产品规划".to_string()),
            },
        )
        .await
        .unwrap();
        let role = crate::db::roles::update_role_skills(
            &main_pool,
            &role.id,
            &crate::models::role::UpdateRoleSkillsInput {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: None,
            },
        )
        .await
        .unwrap();
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "你好")
            .await
            .unwrap();

        let system = &msgs.first().unwrap().content;
        assert!(system.contains("[元 Skill 配置]"));
        assert!(system.contains("find-skills"));
        assert!(!system.contains("skill-creator"));
        assert!(!system.contains("未启用"));
        assert!(!system.contains("要开启吗"));
    }

    #[tokio::test]
    async fn test_build_butler_system_prompt_omits_disabled_meta_skills() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        crate::services::butler_config::set_butler_skills(
            &main_pool,
            &crate::models::role::UpdateRoleSkillsInput {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: None,
            },
        )
        .await
        .unwrap();

        let prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        assert!(prompt.contains("你是数字分身管家"));
        assert!(!prompt.contains("[元 Skill 配置]"));
        assert!(!prompt.contains("find-skills"));
        assert!(!prompt.contains("skill-creator"));
        assert!(!prompt.contains("未启用"));
        assert!(!prompt.contains("要开启吗"));
    }

    #[tokio::test]
    async fn test_build_butler_system_prompt_lists_only_enabled_meta_skills() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        crate::services::butler_config::set_butler_skills(
            &main_pool,
            &crate::models::role::UpdateRoleSkillsInput {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: None,
            },
        )
        .await
        .unwrap();

        let prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        assert!(prompt.contains("[元 Skill 配置]"));
        assert!(prompt.contains("find-skills"));
        assert!(!prompt.contains("skill-creator"));
        assert!(!prompt.contains("未启用"));
        assert!(!prompt.contains("要开启吗"));
    }

    #[tokio::test]
    async fn test_build_butler_system_prompt_injects_enabled_custom_skills() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let skill = crate::db::skills::create_skill(
            &main_pool,
            "daily-review",
            "日复盘助手",
            "skills/daily-review/SKILL.md",
            "hash-1",
        )
        .await
        .unwrap();
        crate::db::skill_bindings::replace_bindings(&main_pool, &skill.id, true, &[])
            .await
            .unwrap();
        crate::services::butler_config::set_butler_skills(
            &main_pool,
            &crate::models::role::UpdateRoleSkillsInput {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: Some(vec![skill.id.clone()]),
            },
        )
        .await
        .unwrap();

        let prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        assert!(prompt.contains("[自定义 Skill]"));
        assert!(prompt.contains("daily-review"));
        assert!(prompt.contains("日复盘助手"));
        assert!(prompt.contains("不要调用或列出外部环境中的其它 Skill"));
    }

    #[tokio::test]
    async fn test_build_butler_system_prompt_omits_disabled_custom_skills() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let skill = crate::db::skills::create_skill(
            &main_pool,
            "markitdown",
            "文件与文档转 Markdown",
            "skills/markitdown/SKILL.md",
            "hash-markitdown",
        )
        .await
        .unwrap();
        crate::db::skill_bindings::replace_bindings(&main_pool, &skill.id, true, &[])
            .await
            .unwrap();
        crate::services::butler_config::set_butler_skills(
            &main_pool,
            &crate::models::role::UpdateRoleSkillsInput {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: Some(Vec::new()),
            },
        )
        .await
        .unwrap();

        let prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        assert!(!prompt.contains("[自定义 Skill]"));
        assert!(!prompt.contains("markitdown"));
        assert!(!prompt.contains("文件与文档转 Markdown"));
    }

    /// AC-6: personality_prompt 为空时不应在 prompt 末尾留空行或 `personality:` 残骸 —
    /// 早期角色没有人格描述，prompt 必须仍然干净，否则 LLM 会被 trailing 空白困惑。
    #[tokio::test]
    async fn test_build_role_messages_skips_empty_personality() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "学习者".to_string(),
                icon: None,
                color: None,
                goal: Some("保持学习节奏".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "")
            .await
            .unwrap();

        let system = &msgs.first().unwrap().content;
        assert!(
            system.contains("默认语调"),
            "personality_prompt 为空时应提供轻量默认语调方向"
        );
        assert!(
            system.contains("[context_injection]"),
            "prompt 应保留三层结构中的上下文注入边界"
        );
    }

    /// AC-6: 若传入不存在的 role_id 必须立刻 fail-loud 返回 NotFound —
    /// 否则 LLM 会用空 prompt 静默裸聊，用户体验等同于"角色不存在但你看不见"，
    /// 与 PRD 透明可控原则相违。
    #[tokio::test]
    async fn test_build_role_messages_unknown_role_returns_not_found() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some("ghost"))
            .await
            .unwrap();

        let result = build_role_messages(&conv_pool, &main_pool, &conv.id, "ghost", "hi").await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ===== Story 2.3 AC-7: 跨角色全局摘要 =====

    /// AC-7 退化场景：无 active 角色时管家不应被拼一个空标题段误导。
    /// 返回空串是 caller 决定是否拼接的契约 —— 不能返回 `[各角色近况]\n`。
    #[tokio::test]
    async fn test_butler_memory_summary_includes_global_and_role_memories() {
        let main_pool = setup_test_main_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, status) VALUES ('role-1', '父亲', 'baby', '#EF4444', '成为小孩的榜样', 'active')",
        )
        .execute(&main_pool)
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            None,
            "global-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "preference".to_string(),
                content: "用户希望用中文沟通".to_string(),
                source_message_ids: vec!["global-msg".to_string()],
            }],
        )
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some("role-1"),
            "role-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "儿子喜欢书法课".to_string(),
                source_message_ids: vec!["role-msg".to_string()],
            }],
        )
        .await
        .unwrap();

        let summary = build_butler_memory_summary(&main_pool).await.unwrap();

        let global_memory = crate::db::memories::list_memories(&main_pool, None, None, None, None)
            .await
            .unwrap()
            .remove(0);
        let role_memory =
            crate::db::memories::list_memories(&main_pool, Some("role-1"), None, None, None)
                .await
                .unwrap()
                .remove(0);

        assert!(summary.contains("[已知记忆]"));
        assert!(summary.contains("全局记忆"));
        assert!(summary.contains(&format!(
            "[[记忆#{}]](egosync-memory://{})",
            format_memory_reference_label(&global_memory),
            global_memory.id
        )));
        assert!(!summary.contains(&format!(" {}", global_memory.id)));
        assert!(summary.contains("[preference] 用户希望用中文沟通"));
        assert!(summary.contains("父亲"));
        assert!(summary.contains(&format!(
            "[[记忆#{}]](egosync-memory://{})",
            format_memory_reference_label(&role_memory),
            role_memory.id
        )));
        assert!(!summary.contains(&format!(" {}", role_memory.id)));
        assert!(summary.contains("[fact] 儿子喜欢书法课"));
        assert!(!summary.contains("- [preference] 用户希望用中文沟通"));
    }

    #[tokio::test]
    async fn test_role_memory_summary_includes_only_current_role_visible_memories() {
        let main_pool = setup_test_main_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("打磨产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let father = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "父亲".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some(&pm.id),
            "pm-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "preference".to_string(),
                content: "产品建议需要先给依据".to_string(),
                source_message_ids: vec!["pm-msg".to_string()],
            }],
        )
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some(&father.id),
            "father-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "周五有家庭聚餐".to_string(),
                source_message_ids: vec!["father-msg".to_string()],
            }],
        )
        .await
        .unwrap();
        let pm_memory =
            crate::db::memories::list_memories(&main_pool, Some(&pm.id), None, None, None)
                .await
                .unwrap()
                .remove(0);

        let summary = build_role_memory_summary(&main_pool, &pm.id).await.unwrap();

        assert!(summary.contains("[当前角色记忆]"));
        assert!(summary.contains("产品经理"));
        assert!(summary.contains(&format!(
            "[[记忆#{}]](egosync-memory://{})",
            format_memory_reference_label(&pm_memory),
            pm_memory.id
        )));
        assert!(!summary.contains(&format!(" {}", pm_memory.id)));
        assert!(summary.contains("[preference] 产品建议需要先给依据"));
        assert!(!summary.contains("周五有家庭聚餐"));
    }

    #[tokio::test]
    async fn test_role_messages_include_memory_time_labels_and_transparency_rules() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("打磨产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some(&role.id),
            "pm-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "周五有产品评审".to_string(),
                source_message_ids: vec!["pm-msg".to_string()],
            }],
        )
        .await
        .unwrap();
        let memory =
            crate::db::memories::list_memories(&main_pool, Some(&role.id), None, None, None)
                .await
                .unwrap()
                .remove(0);
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs =
            build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "为什么这样排？")
                .await
                .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains(&format!(
            "[[记忆#{}]](egosync-memory://{})",
            format_memory_reference_label(&memory),
            memory.id
        )));
        assert!(!system.contains(&format!(" {}", memory.id)));
        assert!(system.contains("[透明推理与不确定性规则]"));
        assert!(!system.contains("必须使用本 prompt 中的 `[[记忆#YYYY/MM/DD HH:mm]](egosync-memory://memory-id)` 内部链接标注来源"));
        assert!(system.contains(
            "用户没有明确要求来源、依据、原文或你怎么知道时，不要主动展示记忆标签或内部链接"
        ));
        assert!(system.contains("只有用户明确询问为什么、依据是什么、你怎么知道的、来源或原文时，才使用本 prompt 中的记忆内部链接"));
        assert!(system.contains("溯源回答必须原样输出本 prompt 中已有的记忆内部链接"));
        assert!(system.contains("不要把来源改写成自然语言时间"));
        assert!(system.contains("不得编造记忆标签"));
        assert!(system.contains("不确定时主动声明"));
        assert!(system.contains("不暴露隐藏 chain-of-thought"));
    }

    #[tokio::test]
    async fn test_prompt_context_excludes_forgotten_memory_after_delete() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        crate::db::memories::insert_memories(
            &main_pool,
            None,
            "global-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "用户周五有家庭聚餐".to_string(),
                source_message_ids: vec!["global-msg".to_string()],
            }],
        )
        .await
        .unwrap();
        let memory = crate::db::memories::list_memories(&main_pool, None, None, None, None)
            .await
            .unwrap()
            .remove(0);
        crate::db::memories::delete_memory(&main_pool, &memory.id)
            .await
            .unwrap();
        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "为什么？")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(!system.contains(&memory.id));
        assert!(!system.contains("用户周五有家庭聚餐"));
    }

    #[tokio::test]
    async fn test_butler_prompt_tells_model_to_answer_from_memory_before_delegation() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, status) VALUES ('role-1', '父亲', 'baby', '#EF4444', '成为小孩的榜样', 'active')",
        )
        .execute(&main_pool)
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some("role-1"),
            "role-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "儿子喜欢书法课".to_string(),
                source_message_ids: vec!["role-msg".to_string()],
            }],
        )
        .await
        .unwrap();

        let prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();
        let dynamic_prompt = build_butler_dynamic_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        assert!(prompt.contains("儿子喜欢书法课"));
        assert!(prompt.contains("陈述某个角色相关事实或偏好"));
        assert!(prompt.contains("直接用自然口吻确认"));
        assert!(prompt.contains("不要向用户暴露内部机制"));
        assert!(
            prompt.contains("不要说“系统会同步”“同步到某角色”“角色已收到”“委派成功”“工具调用”等")
        );
        assert!(!prompt.contains("系统会把这类记忆同步到对应角色"));
        assert!(prompt.contains("角色相关任务、安排、日程、待办、规划或需要跟进"));
        assert!(prompt.contains("只要能匹配 active 角色，就调用 delegate_to_role"));
        assert!(prompt.contains("明确属性或偏好槽位"));
        assert!(prompt.contains("只回答与该问题槽位直接相关的记忆"));
        assert!(prompt.contains("不要因为同一角色或同一对象存在其它记忆"));
        assert!(prompt.contains("不要列出其它领域记忆"));
        assert!(!prompt.contains("当用户的需求清晰指向某个角色时"));
        assert!(prompt.contains("完成或删除已有任务时不得调用 delegate_to_role"));
        assert!(dynamic_prompt.contains("完成或删除已有任务时不得调用 delegate_to_role"));
        assert!(prompt.contains("已完成、已提交或已做完，也视为完成操作"));
        assert!(dynamic_prompt.contains("已完成、已提交或已做完，也视为完成操作"));
    }

    #[tokio::test]
    async fn test_butler_prompts_mark_behavior_only_values_as_uncertain_inference() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let system_prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();
        let dynamic_prompt = build_butler_dynamic_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        for prompt in [&system_prompt, &dynamic_prompt] {
            assert!(prompt.contains("当前未设置使命宣言"));
            assert!(prompt.contains("基于当前行为的推断且存在不确定性"));
            assert!(!prompt.contains("[用户明确设定的使命宣言]"));
        }

        crate::db::mission::upsert_mission(&main_pool, Some("{}"), "structured")
            .await
            .unwrap();
        let empty_structured_context = build_butler_mission_context(&main_pool).await.unwrap();
        assert!(empty_structured_context.contains("当前未设置使命宣言"));
        assert!(!empty_structured_context.contains("[用户明确设定的使命宣言]"));
    }

    #[tokio::test]
    async fn test_butler_prompts_prioritize_free_mission_over_behavior() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        crate::db::mission::upsert_mission(
            &main_pool,
            Some("通过持续创造，让更多普通人获得成长机会。"),
            "free",
        )
        .await
        .unwrap();

        let system_prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();
        let dynamic_prompt = build_butler_dynamic_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        for prompt in [&system_prompt, &dynamic_prompt] {
            assert!(prompt.contains("通过持续创造，让更多普通人获得成长机会。"));
            assert!(prompt.contains("用户明确设定的使命宣言是第一依据"));
            assert!(prompt.contains("行为资料只能作为佐证"));
            assert!(prompt.contains("行为可能偏离使命或与使命存在冲突"));
            assert!(prompt.contains("不得用行为推断覆盖或改写"));
        }
    }

    #[tokio::test]
    async fn test_butler_prompts_render_structured_mission_as_readable_sections() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let content = serde_json::json!({
            "mission": "帮助身边的人活得更从容",
            "principle": "诚实并持续学习",
            "roles": [
                { "name": "父亲", "goal": "陪伴孩子独立成长" },
                { "name": "创造者", "goal": "做长期有益的产品" }
            ]
        })
        .to_string();
        crate::db::mission::upsert_mission(&main_pool, Some(&content), "structured")
            .await
            .unwrap();

        let system_prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();
        let dynamic_prompt = build_butler_dynamic_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        for prompt in [&system_prompt, &dynamic_prompt] {
            assert!(prompt.contains("使命：帮助身边的人活得更从容"));
            assert!(prompt.contains("原则：诚实并持续学习"));
            assert!(prompt.contains("角色目标：\n- 父亲——陪伴孩子独立成长"));
            assert!(prompt.contains("- 创造者——做长期有益的产品"));
            assert!(!prompt.contains("\"mission\""));
        }
    }

    #[tokio::test]
    async fn test_butler_prompts_render_legacy_structured_mission_as_readable_sections() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let content = serde_json::json!({
            "role": "父亲",
            "value": "诚实并持续学习",
            "goal": "陪伴孩子独立成长"
        })
        .to_string();
        crate::db::mission::upsert_mission(&main_pool, Some(&content), "structured")
            .await
            .unwrap();

        let system_prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();
        let dynamic_prompt = build_butler_dynamic_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        for prompt in [&system_prompt, &dynamic_prompt] {
            assert!(prompt.contains("原则：诚实并持续学习"));
            assert!(prompt.contains("角色目标：\n- 父亲——陪伴孩子独立成长"));
            assert!(!prompt.contains("\"role\""));
        }
    }

    #[tokio::test]
    async fn test_structured_mission_parse_failure_preserves_original_content() {
        let main_pool = setup_test_main_pool().await;
        crate::db::mission::upsert_mission(
            &main_pool,
            Some("这不是合法 JSON，但仍是用户明确写下的使命"),
            "structured",
        )
        .await
        .unwrap();

        let context = build_butler_mission_context(&main_pool).await.unwrap();

        assert!(context.contains("这不是合法 JSON，但仍是用户明确写下的使命"));
        assert!(context.contains("用户明确设定的使命宣言是第一依据"));
    }

    #[tokio::test]
    async fn test_butler_prompt_builders_fail_when_mission_query_fails() {
        let conv_pool = setup_test_conv_pool().await;
        let system_main_pool = setup_test_main_pool().await;
        system_main_pool.close().await;
        let system_result = build_butler_system_prompt(&conv_pool, &system_main_pool).await;
        assert!(matches!(system_result, Err(AppError::DbError(_))));

        let dynamic_main_pool = setup_test_main_pool().await;
        dynamic_main_pool.close().await;
        let dynamic_result = build_butler_dynamic_prompt(&conv_pool, &dynamic_main_pool).await;
        assert!(matches!(dynamic_result, Err(AppError::DbError(_))));
    }

    #[tokio::test]
    async fn test_cross_role_summary_empty_when_no_roles() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.is_empty(), "无角色时不能输出任何文字");
    }

    /// AC-7 退化场景：有 active 角色但还没人跟它说过话 → 仍然返回空串。
    /// 不能让一个"什么都没发生"的角色出现在管家 prompt 里，否则管家会被迫复述虚无。
    #[tokio::test]
    async fn test_cross_role_summary_empty_when_role_has_no_messages() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "学习者".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.is_empty(), "角色无对话历史时不应进入摘要");
    }

    /// AC-7 核心：用户主动跟角色聊过后回到管家，管家必须能看到那段对话。
    /// 否则用户回管家说"我刚才跟产品经理聊了啥"管家就要装失忆。
    #[tokio::test]
    async fn test_cross_role_summary_includes_recent_messages_per_role() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("打磨产品".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&pm.id))
            .await
            .unwrap();
        crate::db::conversations::insert_message(
            &conv_pool,
            &conv.id,
            "user",
            "本周 OKR 怎么排",
            true,
        )
        .await
        .unwrap();
        crate::db::conversations::insert_message(
            &conv_pool,
            &conv.id,
            "assistant",
            "建议先看用户访谈",
            true,
        )
        .await
        .unwrap();

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.contains("[各角色近况]"));
        assert!(summary.contains("产品经理"), "摘要必须含角色名");
        assert!(summary.contains("本周 OKR 怎么排"), "user 消息必须可见");
        assert!(
            summary.contains("建议先看用户访谈"),
            "assistant 消息必须可见"
        );
    }

    /// AC-7 多角色：用户在 N 个角色私聊后回管家，摘要必须涵盖所有有历史的角色。
    /// 漏掉任何一个就是管家"偏听偏信"，破坏全局视野承诺。
    #[tokio::test]
    async fn test_cross_role_summary_covers_multiple_roles() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        for name in ["产品经理", "学习者"] {
            let r = crate::db::roles::create_role(
                &main_pool,
                &CreateRoleInput {
                    name: name.to_string(),
                    icon: None,
                    color: None,
                    goal: None,
                },
            )
            .await
            .unwrap();
            let c = crate::db::conversations::create_conversation(&conv_pool, Some(&r.id))
                .await
                .unwrap();
            crate::db::conversations::insert_message(
                &conv_pool,
                &c.id,
                "user",
                &format!("hi {}", name),
                true,
            )
            .await
            .unwrap();
        }

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.contains("产品经理"));
        assert!(summary.contains("学习者"));
    }

    // ===== Story 2.3 AC-1 / AC-5: delegate_to_role 工具与 butler prompt 增强 =====

    /// AC-1: 工具定义必须能让 LLM 在管家视图明确传达"我要把任务派给谁、派什么"。
    /// 缺 target_role_id 或 task_summary 会导致后端无法定位角色或写入审计 —— 缺字段不可妥协。
    #[test]
    fn test_delegate_to_role_tool_definition_has_required_fields() {
        let tool = delegate_to_role_tool_definition();
        assert_eq!(tool.name, "delegate_to_role");
        let params = tool.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["target_role_id"].is_object());
        assert!(params["properties"]["task_summary"].is_object());
        assert!(params["properties"]["context"].is_object());

        let required = params["required"].as_array().expect("required 应为数组");
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            required_strs.contains(&"target_role_id"),
            "target_role_id 必填"
        );
        assert!(required_strs.contains(&"task_summary"), "task_summary 必填");
        assert!(!required_strs.contains(&"context"), "context 仅可选");
    }

    /// AC-1: butler prompt 必须把可委派的角色 id+name+goal 全部展示给 LLM —
    /// 否则 LLM 只能瞎猜 id，导致 AC-8 的"目标无效"分支被大量触发。
    #[tokio::test]
    async fn test_build_butler_messages_includes_role_roster() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("打磨产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "帮我看下 OKR")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("[可委派角色清单]"), "需要可委派清单段落");
        assert!(
            system.contains(&pm.id),
            "角色 id 必须可见，否则 LLM 无法引用"
        );
        assert!(system.contains("产品经理"), "角色名必须可见");
        assert!(system.contains("打磨产品节奏"), "角色目标必须可见");
        assert!(system.contains("[行为指南]"), "行为指南段落必须出现");
        assert!(system.contains("delegate_to_role"), "行为指南要点名工具");
    }

    /// AC-5 退化场景：零 active 角色时管家不应被"行为指南"诱导调用工具 —
    /// 否则 LLM 会持续触发空 tool_call 导致 follow-up 死循环。
    #[tokio::test]
    async fn test_build_butler_messages_omits_guidance_when_no_active_roles() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "你好")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("分身管家"), "管家身份必须保留");
        assert!(!system.contains("[可委派角色清单]"));
        assert!(!system.contains("[行为指南]"));
        assert!(!system.contains("delegate_to_role"));
    }

    // ===== Story 2.3 AC-8: execute_delegate_to_role 兜底分支（不依赖 LLM provider）=====

    /// AC-8 核心：LLM 给出不存在的 target_role_id 时必须返回 role_not_found 记录，
    /// 不写入角色对话 —— 否则会污染一个不存在角色的对话历史，引发后续追溯混乱。
    #[tokio::test]
    async fn test_execute_delegate_role_not_found_returns_audit_only() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let args = serde_json::json!({
            "target_role_id": "ghost-role-id",
            "task_summary": "跟进 OKR"
        })
        .to_string();

        let (text, record) = execute_delegate_to_role(&main_pool, &conv_pool, &args).await;

        assert_eq!(record.status, "role_not_found");
        assert_eq!(record.target_role_id, "ghost-role-id");
        assert_eq!(record.task_summary, "跟进 OKR");
        assert!(
            text.contains("不可用"),
            "tool result 必须明确告知 LLM 该角色不可用"
        );

        // 关键：不应该创建任何 conversation/message —— 否则就污染了"不存在的角色"
        let convs =
            crate::db::conversations::list_conversations_by_role(&conv_pool, "ghost-role-id")
                .await
                .unwrap();
        assert!(convs.is_empty(), "无效角色不应留下任何对话痕迹");
    }

    /// AC-8 + AC-4 边界：归档角色不接受委派 —— 否则用户归档角色后还在被偷偷调用，
    /// 违反用户主动归档的明确意图（与 Story 2.1 归档语义一致）。
    #[tokio::test]
    async fn test_execute_delegate_archived_role_rejected() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "前同事".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        crate::db::roles::archive_role(&main_pool, &role.id)
            .await
            .unwrap();

        let args = serde_json::json!({
            "target_role_id": role.id.clone(),
            "task_summary": "敲门"
        })
        .to_string();

        let (_text, record) = execute_delegate_to_role(&main_pool, &conv_pool, &args).await;
        assert_eq!(
            record.status, "role_not_found",
            "已归档角色等同于『不可用』，必须走同一兜底分支"
        );
        // 同样不能给归档角色生成新对话
        let convs = crate::db::conversations::list_conversations_by_role(&conv_pool, &role.id)
            .await
            .unwrap();
        assert!(convs.is_empty());
    }

    /// AC-1 参数协议：parse 失败时返回 parse_error，不进入兜底业务路径 —
    /// 否则 LLM 可能用错误参数反复重试，污染审计日志。
    #[tokio::test]
    async fn test_execute_delegate_invalid_json_returns_parse_error() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let (text, record) = execute_delegate_to_role(&main_pool, &conv_pool, "not-json").await;
        assert_eq!(record.status, "parse_error");
        assert!(text.contains("参数解析失败"));
    }

    #[test]
    fn test_completed_tool_part_is_claimed_once_by_part_id() {
        let mut processed = std::collections::HashSet::new();
        let part_raw = serde_json::json!({
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "completed",
                "output": "{}"
            }
        });

        assert!(claim_completed_tool_part(
            &mut processed,
            "part-1",
            &part_raw
        ));
        assert!(!claim_completed_tool_part(
            &mut processed,
            "part-1",
            &part_raw
        ));
    }

    #[test]
    fn test_unfinished_tool_part_is_not_claimed() {
        let mut processed = std::collections::HashSet::new();
        let part_raw = serde_json::json!({
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "running",
                "output": "{}"
            }
        });

        assert!(!claim_completed_tool_part(
            &mut processed,
            "part-1",
            &part_raw
        ));
        assert!(processed.is_empty());
    }

    #[tokio::test]
    async fn test_delegate_worker_runs_in_background_and_serializes() {
        let lock = Arc::new(Mutex::new(()));
        let (first_started_tx, first_started_rx) = tokio::sync::oneshot::channel();
        let (release_first_tx, release_first_rx) = tokio::sync::oneshot::channel::<()>();
        let second_ran = Arc::new(Mutex::new(false));

        let first = spawn_delegate_worker(lock.clone(), async move {
            let _ = first_started_tx.send(());
            let _ = release_first_rx.await;
        });
        tokio::time::timeout(Duration::from_millis(100), first_started_rx)
            .await
            .expect("第一个后台委派任务应立即开始")
            .expect("第一个后台委派任务应发送 started 信号");

        let second_ran_for_task = second_ran.clone();
        let second = spawn_delegate_worker(lock, async move {
            *second_ran_for_task.lock().await = true;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!*second_ran.lock().await, "第二个委派任务必须等待同一锁");
        assert!(!first.is_finished(), "第一个委派任务不应阻塞调用方等待完成");

        release_first_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), first)
            .await
            .expect("第一个后台委派任务应完成")
            .expect("第一个后台委派任务不应 panic");
        tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("第二个后台委派任务应完成")
            .expect("第二个后台委派任务不应 panic");
        assert!(*second_ran.lock().await);
    }

    #[test]
    fn test_completed_response_appends_missing_tail() {
        let mut text = "这些技能的安装量都不是很高（低于1K），建议".to_string();
        append_completed_tail(
            None,
            "conversation-id",
            None,
            &mut text,
            "这些技能的安装量都不是很高（低于1K），建议先试用安装量最高的选项，再根据实际效果决定是否保留。",
            false,
        );

        assert!(text.ends_with("是否保留。"));
    }

    #[test]
    fn test_remove_recorded_narration_prefix_from_final_text() {
        let mut final_text = "好的，我来帮你把这个 PowerPoint 文件转换成 Markdown 格式。转换完成了。已保存到 output.md".to_string();
        let recorded_narrations = vec!["好的，我来帮你把这个 PowerPoint 文件转换成 Markdown 格式。".to_string()];

        remove_recorded_narration_prefixes(&mut final_text, &recorded_narrations);

        assert_eq!(final_text, "转换完成了。已保存到 output.md");
    }

    #[test]
    fn test_split_followup_completed_response_uses_tail_after_first_bubble() {
        let mut followup = String::new();
        append_completed_followup_tail(
            None,
            "conversation-id",
            Some("followup-id"),
            &mut followup,
            "我来帮你把 PPTX 文件转换成 Markdown 格式。",
            "我来帮你把 PPTX 文件转换成 Markdown 格式。转换完成。文件已保存。",
        );

        assert_eq!(followup, "转换完成。文件已保存。");
    }

    #[test]
    fn test_split_followup_completed_fallback_uses_tail_after_first_bubble() {
        let completed = crate::models::agent::OpencodeCompletedMessage {
            text: "我来帮你把 PPTX 文件转换成 Markdown 格式。转换完成。文件已保存。".to_string(),
            thinking: "内部思考".to_string(),
        };
        let mut followup = String::new();
        let mut thinking = String::new();

        apply_completed_followup_message_fallback(
            None,
            "conversation-id",
            Some("followup-id"),
            Some(completed),
            Some("我来帮你把 PPTX 文件转换成 Markdown 格式。"),
            &mut followup,
            &mut thinking,
        );

        assert_eq!(followup, "转换完成。文件已保存。");
        assert_eq!(thinking, "内部思考");
    }

    #[test]
    fn test_completed_response_does_not_replace_divergent_stream_text() {
        let mut text = "事件流文本".to_string();
        append_completed_tail(
            None,
            "conversation-id",
            None,
            &mut text,
            "另一个完整回复",
            false,
        );

        assert_eq!(text, "事件流文本");
    }

    #[test]
    fn test_meta_skill_discovery_request_detection() {
        assert!(looks_like_meta_skill_discovery_request(
            "搜索一下视频格式转换的skill"
        ));
        assert!(looks_like_meta_skill_discovery_request(
            "找一下音频转换技能"
        ));
        assert!(looks_like_meta_skill_discovery_request(
            "find skill for audio conversion"
        ));
        assert!(!looks_like_meta_skill_discovery_request(
            "帮我转换一个视频格式"
        ));
        assert!(looks_like_meta_skill_creator_request(
            "创建一个视频转换技能"
        ));
        assert!(looks_like_meta_skill_creator_request(
            "create a skill for product managers"
        ));
        assert!(!looks_like_meta_skill_creator_request("帮我写一个产品方案"));
    }

    #[test]
    fn test_role_meta_skill_disabled_message_blocks_discovery() {
        let message = disabled_meta_skill_message(
            false,
            true,
            "搜索一下视频格式转换的skill",
            MetaSkillConfigOwner::Role,
        );
        assert_eq!(
            message,
            Some("当前角色没有启用 Skill 发现能力，需要在该角色的 Skill 配置中开启后，我才能帮你查找或推荐 Skill。")
        );
    }

    #[test]
    fn test_butler_meta_skill_disabled_message_blocks_creator() {
        let message = disabled_meta_skill_message(
            true,
            false,
            "创建一个视频转换技能",
            MetaSkillConfigOwner::Butler,
        );
        assert_eq!(
            message,
            Some("当前没有启用 Skill 创建能力，需要在管家的 Skill 配置中开启后，我才能帮你创建或扩展 Skill。")
        );
    }

    #[test]
    fn test_meta_skill_disabled_message_allows_enabled_discovery() {
        let message = disabled_meta_skill_message(
            true,
            false,
            "搜索一下视频格式转换的skill",
            MetaSkillConfigOwner::Role,
        );
        assert!(message.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_completed_delegate_tool_part_persists_once() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();
        let butler_user_msg = crate::db::conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "明天有产品设计评审",
            true,
        )
        .await
        .unwrap();
        let output = serde_json::json!({
            "action": "delegate_to_role",
            "target_role_id": pm.id,
            "task_summary": "准备明天的产品设计评审",
        })
        .to_string();
        let part_raw = serde_json::json!({
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "completed",
                "output": output,
            }
        });
        let result_json: serde_json::Value =
            serde_json::from_str(part_raw["state"]["output"].as_str().unwrap()).unwrap();
        let mut processed = std::collections::HashSet::new();

        for _ in 0..2 {
            if claim_completed_tool_part(&mut processed, "part-1", &part_raw) {
                handle_delegate_tool_result(
                    &conv_pool,
                    &main_pool,
                    &butler_user_msg.id,
                    &result_json,
                )
                .await;
            }
        }

        let role_convs = crate::db::conversations::list_conversations_by_role(&conv_pool, &pm.id)
            .await
            .unwrap();
        let role_messages = crate::db::conversations::list_messages(&conv_pool, &role_convs[0].id)
            .await
            .unwrap();
        let delegated_user_messages = role_messages
            .iter()
            .filter(|m| m.role == "user" && m.content.contains("[管家委派]"))
            .count();
        assert_eq!(delegated_user_messages, 1);

        let messages = crate::db::conversations::list_messages(&conv_pool, &butler_conv.id)
            .await
            .unwrap();
        let metadata = messages
            .iter()
            .find(|m| m.id == butler_user_msg.id)
            .and_then(|m| m.routing_metadata.as_deref())
            .expect("routing_metadata 应存在");
        let parsed: serde_json::Value = serde_json::from_str(metadata).unwrap();
        assert_eq!(parsed["delegations"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_opencode_delegate_tool_part_persists_role_history_and_routing_metadata() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("把控产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();
        let butler_user_msg = crate::db::conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "明天有产品设计评审",
            true,
        )
        .await
        .unwrap();

        let tool_output = serde_json::json!({
            "action": "delegate_to_role",
            "target_role_id": pm.id,
            "task_summary": "准备明天的产品设计评审",
            "context": "明天有产品设计评审"
        })
        .to_string();

        handle_delegate_tool_result(
            &conv_pool,
            &main_pool,
            &butler_user_msg.id,
            &serde_json::from_str(&tool_output).unwrap(),
        )
        .await;

        let role_convs = crate::db::conversations::list_conversations_by_role(&conv_pool, &pm.id)
            .await
            .unwrap();
        assert_eq!(role_convs.len(), 1, "opencode 委派必须创建/复用角色对话");
        let role_messages = crate::db::conversations::list_messages(&conv_pool, &role_convs[0].id)
            .await
            .unwrap();
        assert!(
            role_messages.iter().any(|m| {
                m.role == "user" && m.content.contains("[管家委派] 准备明天的产品设计评审")
            }),
            "opencode 委派必须写入角色 user 消息"
        );

        let butler_messages = crate::db::conversations::list_messages(&conv_pool, &butler_conv.id)
            .await
            .unwrap();
        let routed_user_msg = butler_messages
            .iter()
            .find(|m| m.id == butler_user_msg.id)
            .expect("触发委派的 user message 应仍存在");
        let metadata = routed_user_msg
            .routing_metadata
            .as_deref()
            .expect("opencode 委派必须写 routing_metadata");
        assert!(metadata.contains("\"targetRoleName\":\"产品经理\""));
        assert!(metadata.contains("\"taskSummary\":\"准备明天的产品设计评审\""));
        assert!(metadata.contains("\"status\":"));
    }

    #[tokio::test]
    async fn test_opencode_delegate_tool_result_appends_routing_metadata() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();
        let butler_user_msg = crate::db::conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "安排产品评审和学习计划",
            true,
        )
        .await
        .unwrap();

        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        let learner = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "学习者".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();

        for (role_id, task_summary) in [(&pm.id, "准备产品评审"), (&learner.id, "安排学习计划")]
        {
            let result = serde_json::json!({
                "action": "delegate_to_role",
                "target_role_id": role_id,
                "task_summary": task_summary,
            });
            handle_delegate_tool_result(&conv_pool, &main_pool, &butler_user_msg.id, &result).await;
        }

        let messages = crate::db::conversations::list_messages(&conv_pool, &butler_conv.id)
            .await
            .unwrap();
        let metadata = messages
            .iter()
            .find(|m| m.id == butler_user_msg.id)
            .and_then(|m| m.routing_metadata.as_deref())
            .expect("routing_metadata 应存在");
        let parsed: serde_json::Value = serde_json::from_str(metadata).unwrap();
        let delegations = parsed["delegations"].as_array().unwrap();
        assert_eq!(delegations.len(), 2);
        assert_eq!(delegations[0]["targetRoleName"], "产品经理");
        assert_eq!(delegations[1]["targetRoleName"], "学习者");
    }

    // ===== Story 2.5: 涌现工具与 prompt 增强 =====

    #[test]
    fn test_record_emergence_rejection_tool_definition() {
        let tool = record_emergence_rejection_tool_definition();
        assert_eq!(tool.name, "record_emergence_rejection");
        assert!(tool.description.contains("拒绝"));
        let params = tool.parameters;
        let required = params["required"].as_array().expect("required 应为数组");
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(required_strs.contains(&"domain"), "domain 必填");
    }

    /// Story 2.5 AC-1: butler prompt 在有 active 角色时必须含涌现行为指令，
    /// 否则 LLM 不知道可以建议创建新角色。
    #[tokio::test]
    async fn test_build_butler_messages_includes_emergence_prompt() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        // 需要 app_settings 表用于冷却查询
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&main_pool)
        .await
        .expect("failed to create app_settings table");

        crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("打磨产品".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "最近想健身")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(
            system.contains("[角色涌现行为]"),
            "butler prompt 必须含涌现行为段落"
        );
        assert!(
            system.contains("create_role"),
            "涌现段落须提及 create_role 工具"
        );
        assert!(
            system.contains("record_emergence_rejection"),
            "涌现段落须提及 rejection 工具"
        );
    }

    /// Story 2.5 AC-3: 冷却列表注入 butler prompt
    #[tokio::test]
    async fn test_build_butler_messages_includes_cooldown_list() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&main_pool)
        .await
        .unwrap();

        // 写入一条冷却记录（1 天前，在 7 天内）
        let recent = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        crate::db::app_settings::set_emergence_cooldown(&main_pool, "健身/运动", &recent)
            .await
            .unwrap();

        crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "想健身")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(
            system.contains("健身/运动"),
            "冷却中的领域必须出现在 prompt 中"
        );
        assert!(system.contains("被拒绝的领域"), "冷却列表应有说明文案");
    }
}
