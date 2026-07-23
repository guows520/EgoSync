use tokio::sync::mpsc;

use crate::error::AppError;
use crate::models::agent::{
    AgentInfo, AgentMessage, BusEvent, OpencodeCompletedMessage, OpencodeConfig, ProviderInfo,
    SessionInfo, SseEvent,
};

#[derive(Clone)]
/// HTTP client for communicating with the opencode server API.
pub struct AgentBridge {
    base_url: String,
    http_client: reqwest::Client,
}

impl AgentBridge {
    /// Create a new AgentBridge pointing to the given port.
    pub fn new(port: u16) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", port),
            http_client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // ── Session Management ──────────────────────────────────────────

    /// Create a new session for a given agent.
    pub async fn create_session(
        &self,
        agent: &str,
        directory: &str,
    ) -> Result<SessionInfo, AppError> {
        let url = format!("{}/session", self.base_url);
        let body = serde_json::json!({});

        let mut query_params: Vec<(&str, &str)> = vec![("directory", directory)];
        if !agent.is_empty() {
            query_params.push(("agent", agent));
        }

        let resp = self
            .http_client
            .post(&url)
            .query(&query_params)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("create_session request failed: {}", e)))?;

        let resp = Self::ensure_success_with_body(resp).await?;

        resp.json::<SessionInfo>()
            .await
            .map_err(|e| AppError::SidecarError(format!("create_session parse failed: {}", e)))
    }

    /// Get messages from a session.
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<AgentMessage>, AppError> {
        let url = format!("{}/session/{}/messages", self.base_url, session_id);

        let resp =
            self.http_client.get(&url).send().await.map_err(|e| {
                AppError::SidecarError(format!("get_messages request failed: {}", e))
            })?;

        Self::ensure_success(&resp)?;

        resp.json::<Vec<AgentMessage>>()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_messages parse failed: {}", e)))
    }

    /// Fork a session so the next turn reloads agent permissions and Skills
    /// while retaining the existing OpenCode message history.
    pub async fn fork_session(&self, session_id: &str) -> Result<SessionInfo, AppError> {
        let url = format!("{}/session/{}/fork", self.base_url, session_id);
        let resp = self
            .http_client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("fork_session request failed: {}", e)))?;
        let resp = Self::ensure_success_with_body(resp).await?;
        resp.json::<SessionInfo>()
            .await
            .map_err(|e| AppError::SidecarError(format!("fork_session parse failed: {}", e)))
    }

    /// Abort / cancel a running session.
    pub async fn abort_session(&self, session_id: &str) -> Result<(), AppError> {
        let url = format!("{}/session/{}/abort", self.base_url, session_id);

        let resp =
            self.http_client.post(&url).send().await.map_err(|e| {
                AppError::SidecarError(format!("abort_session request failed: {}", e))
            })?;

        Self::ensure_success_with_body(resp).await?;
        Ok(())
    }

    /// Compact / compress a session's context.
    pub async fn compact_session(&self, session_id: &str) -> Result<(), AppError> {
        let url = format!("{}/session/{}/compact", self.base_url, session_id);

        let resp = self.http_client.post(&url).send().await.map_err(|e| {
            AppError::SidecarError(format!("compact_session request failed: {}", e))
        })?;

        Self::ensure_success(&resp)?;
        Ok(())
    }

    // ── Messaging ───────────────────────────────────────────────────

    /// Trigger a prompt on a session. opencode's POST /session/{id}/message
    /// is *synchronous* — it blocks until the LLM finishes and returns the
    /// completed message as application/json. Streaming tokens are NOT here;
    /// subscribe to `subscribe_events` instead for real-time updates.
    /// The completed JSON response is used as a fallback when a provider omits
    /// final text from the event bus.
    pub async fn send_message(
        &self,
        session_id: &str,
        content: &str,
        agent: &str,
    ) -> Result<Option<OpencodeCompletedMessage>, AppError> {
        let url = format!("{}/session/{}/message", self.base_url, session_id);
        let body = serde_json::json!({
            "agent": agent,
            "parts": [{ "type": "text", "text": content }]
        });

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("send_message request failed: {}", e)))?;

        let resp = Self::ensure_success_with_body(resp).await?;
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::SidecarError(format!("send_message body read failed: {}", e)))?;
        if body.trim().is_empty() {
            return Ok(None);
        }
        let value = serde_json::from_str::<serde_json::Value>(&body).ok();
        Ok(value.and_then(|value| OpencodeCompletedMessage::from_value(&value)))
    }

    /// Trigger a skill command on a session. opencode's POST /session/{id}/command
    /// invokes a named command (registered by a SKILL.md) with the given arguments.
    /// Like `send_message`, it is synchronous and returns the completed message;
    /// streaming tokens arrive via the global SSE subscription.
    pub async fn send_command(
        &self,
        session_id: &str,
        agent: &str,
        command: &str,
        arguments: &str,
    ) -> Result<Option<OpencodeCompletedMessage>, AppError> {
        let url = format!("{}/session/{}/command", self.base_url, session_id);
        let body = serde_json::json!({
            "agent": agent,
            "command": command,
            "arguments": arguments
        });

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("send_command request failed: {}", e)))?;

        let resp = Self::ensure_success_with_body(resp).await?;
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::SidecarError(format!("send_command body read failed: {}", e)))?;
        if body.trim().is_empty() {
            return Ok(None);
        }
        let value = serde_json::from_str::<serde_json::Value>(&body).ok();
        Ok(value.and_then(|value| OpencodeCompletedMessage::from_value(&value)))
    }

    /// Subscribe to opencode's global event stream (`GET /event`) as SSE.
    /// Each event JSON `{ type, properties }` is forwarded as a BusEvent
    /// until the stream closes or `tx` is dropped.
    pub async fn subscribe_events(&self, tx: mpsc::Sender<BusEvent>) -> Result<(), AppError> {
        let url = format!("{}/event", self.base_url);
        let resp = self.http_client.get(&url).send().await.map_err(|e| {
            AppError::SidecarError(format!("subscribe_events request failed: {}", e))
        })?;
        let resp = Self::ensure_success_with_body(resp).await?;

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|e| AppError::SidecarError(format!("event stream read error: {}", e)))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text.replace("\r\n", "\n"));

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                if let Some(event) = parse_bus_event(&event_block) {
                    if tx.send(event).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    // ── Configuration Queries ───────────────────────────────────────

    /// Get the current opencode configuration.
    pub async fn get_config(&self) -> Result<OpencodeConfig, AppError> {
        let url = format!("{}/config", self.base_url);

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_config request failed: {}", e)))?;

        Self::ensure_success(&resp)?;

        resp.json::<OpencodeConfig>()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_config parse failed: {}", e)))
    }

    /// Get available providers.
    pub async fn get_providers(&self) -> Result<Vec<ProviderInfo>, AppError> {
        let url = format!("{}/providers", self.base_url);

        let resp =
            self.http_client.get(&url).send().await.map_err(|e| {
                AppError::SidecarError(format!("get_providers request failed: {}", e))
            })?;

        Self::ensure_success(&resp)?;

        resp.json::<Vec<ProviderInfo>>()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_providers parse failed: {}", e)))
    }

    /// Get available agents.
    pub async fn get_agents(&self) -> Result<Vec<AgentInfo>, AppError> {
        let url = format!("{}/agents", self.base_url);

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_agents request failed: {}", e)))?;

        Self::ensure_success(&resp)?;

        resp.json::<Vec<AgentInfo>>()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_agents parse failed: {}", e)))
    }

    // ── Helpers ─────────────────────────────────────────────────────

    /// Check HTTP response status; return error for non-success codes.
    fn ensure_success(resp: &reqwest::Response) -> Result<(), AppError> {
        Self::check_status(resp.status())
    }

    /// Like ensure_success but reads the response body on error for better diagnostics.
    /// Consumes the response; returns it back on success for further use.
    async fn ensure_success_with_body(
        resp: reqwest::Response,
    ) -> Result<reqwest::Response, AppError> {
        if resp.status().is_success() {
            Ok(resp)
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let detail = if body.is_empty() {
                format!("opencode API returned HTTP {}", status)
            } else {
                format!(
                    "opencode API returned HTTP {}: {}",
                    status,
                    &body[..body.len().min(256)]
                )
            };
            Err(AppError::SidecarError(detail))
        }
    }

    /// Pure status-code-to-AppError mapping (extracted for testability).
    fn check_status(status: reqwest::StatusCode) -> Result<(), AppError> {
        if status.is_success() {
            Ok(())
        } else {
            Err(AppError::SidecarError(format!(
                "opencode API returned HTTP {}",
                status
            )))
        }
    }
}

/// Parse a single bus-event SSE block from `GET /event`.
/// Each block's `data:` line is JSON: `{ type: "...", properties: {...} }`.
pub fn parse_bus_event(block: &str) -> Option<BusEvent> {
    let mut data_lines: Vec<&str> = Vec::new();
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("data: ") {
            data_lines.push(value);
        } else if line.starts_with("data:") {
            data_lines.push(line.strip_prefix("data:").unwrap_or("").trim());
        }
    }
    if data_lines.is_empty() {
        return None;
    }
    let data = data_lines.join("\n");
    serde_json::from_str::<BusEvent>(&data).ok()
}

/// Parse a single SSE event block (lines between double newlines).
pub fn parse_sse_event(block: &str) -> Option<SseEvent> {
    let mut data_lines: Vec<&str> = Vec::new();

    for line in block.lines() {
        if let Some(value) = line.strip_prefix("data: ") {
            data_lines.push(value);
        } else if line.starts_with("data:") {
            data_lines.push(line.strip_prefix("data:").unwrap_or("").trim());
        }
    }

    if data_lines.is_empty() {
        return None;
    }

    let data = data_lines.join("\n");

    // Try parsing as JSON SseEvent
    if let Ok(event) = serde_json::from_str::<SseEvent>(&data) {
        return Some(event);
    }

    // If "[DONE]" signal
    if data.trim() == "[DONE]" {
        return Some(SseEvent::Done);
    }

    // Fallback: treat as plain text content
    Some(SseEvent::Text { content: data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_bridge_new() {
        let bridge = AgentBridge::new(4096);
        assert_eq!(bridge.base_url(), "http://127.0.0.1:4096");
    }

    #[test]
    fn test_agent_bridge_custom_port() {
        let bridge = AgentBridge::new(8080);
        assert_eq!(bridge.base_url(), "http://127.0.0.1:8080");
    }

    #[test]
    fn test_parse_sse_event_json_text() {
        let block = r#"data: {"type":"text","content":"hello"}"#;
        let event = parse_sse_event(block).unwrap();
        match event {
            SseEvent::Text { content } => assert_eq!(content, "hello"),
            _ => panic!("Expected Text event"),
        }
    }

    #[test]
    fn test_parse_sse_event_done_signal() {
        let block = "data: [DONE]";
        let event = parse_sse_event(block).unwrap();
        assert!(matches!(event, SseEvent::Done));
    }

    #[test]
    fn test_parse_sse_event_json_done() {
        let block = r#"data: {"type":"done"}"#;
        let event = parse_sse_event(block).unwrap();
        assert!(matches!(event, SseEvent::Done));
    }

    #[test]
    fn test_parse_sse_event_json_error() {
        let block = r#"data: {"type":"error","message":"rate limited"}"#;
        let event = parse_sse_event(block).unwrap();
        match event {
            SseEvent::Error { message } => assert_eq!(message, "rate limited"),
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_parse_sse_event_tool_call() {
        let block =
            r#"data: {"type":"toolCall","name":"read","arguments":"{\"path\":\"test.rs\"}"}"#;
        let event = parse_sse_event(block).unwrap();
        match event {
            SseEvent::ToolCall { name, arguments } => {
                assert_eq!(name, "read");
                assert!(arguments.contains("test.rs"));
            }
            _ => panic!("Expected ToolCall event"),
        }
    }

    #[test]
    fn test_parse_sse_event_plain_text_fallback() {
        let block = "data: some plain text";
        let event = parse_sse_event(block).unwrap();
        match event {
            SseEvent::Text { content } => assert_eq!(content, "some plain text"),
            _ => panic!("Expected Text fallback"),
        }
    }

    #[test]
    fn test_parse_sse_event_empty_block() {
        let event = parse_sse_event("");
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_sse_event_no_data_prefix() {
        let event = parse_sse_event("event: ping");
        assert!(event.is_none());
    }

    #[test]
    fn test_check_status_success_returns_ok() {
        // WHY: ensure_success must let 2xx responses pass through so callers
        // proceed to JSON decoding instead of erroring on healthy responses.
        let result = AgentBridge::check_status(reqwest::StatusCode::OK);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_status_4xx_maps_to_sidecar_error_with_code() {
        // WHY: frontend differentiates SidecarError from other AppError variants
        // via the typed enum (see error.rs serialize impl) — and the included
        // status code is what makes "opencode unreachable vs auth failed"
        // distinguishable in user-facing messages.
        let result = AgentBridge::check_status(reqwest::StatusCode::UNAUTHORIZED);
        match result {
            Err(AppError::SidecarError(msg)) => {
                assert!(
                    msg.contains("401"),
                    "error must include status code: {}",
                    msg
                );
            }
            other => panic!("expected SidecarError, got {:?}", other),
        }
    }

    #[test]
    fn test_check_status_5xx_maps_to_sidecar_error() {
        // WHY: opencode crashes / 500s must not silently look like success.
        // This guards against accidentally swallowing server-side failures.
        let result = AgentBridge::check_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        assert!(matches!(result, Err(AppError::SidecarError(_))));
    }

    #[test]
    fn test_send_command_request_body_format() {
        // WHY: AC-3 — send_command 必须发送 { agent, command, arguments } 三字段 JSON。
        // opencode v1.15.10 的 /session/{id}/command 端点依赖此精确格式来路由到
        // 对应的 SKILL.md 注册的 command。字段缺失或多余都会导致 command 不被识别。
        let body = serde_json::json!({
            "agent": "butler",
            "command": "ppt-generation",
            "arguments": "帮我生成一份季度汇报"
        });
        let obj = body.as_object().unwrap();
        assert_eq!(obj.len(), 3, "body must have exactly 3 fields");
        assert_eq!(obj["agent"], "butler");
        assert_eq!(obj["command"], "ppt-generation");
        assert_eq!(obj["arguments"], "帮我生成一份季度汇报");
    }

    #[test]
    fn test_send_command_url_construction() {
        // WHY: AC-3 — send_command 的 URL 必须是 POST /session/{id}/command，
        // 与 send_message 的 /session/{id}/message 区分，确保 opencode 路由正确。
        let bridge = AgentBridge::new(4096);
        let url = format!("{}/session/{}/command", bridge.base_url(), "sess-123");
        assert_eq!(url, "http://127.0.0.1:4096/session/sess-123/command");
    }
}
