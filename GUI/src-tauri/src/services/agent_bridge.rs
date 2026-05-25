use tokio::sync::mpsc;

use crate::error::AppError;
use crate::models::agent::{
    AgentInfo, AgentMessage, OpencodeConfig, ProviderInfo, SessionInfo, SseEvent,
};

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
            http_client: reqwest::Client::new(),
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
        let body = serde_json::json!({
            "agent": agent,
            "directory": directory,
        });

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("create_session request failed: {}", e)))?;

        Self::ensure_success(&resp)?;

        resp.json::<SessionInfo>()
            .await
            .map_err(|e| AppError::SidecarError(format!("create_session parse failed: {}", e)))
    }

    /// Get messages from a session.
    pub async fn get_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, AppError> {
        let url = format!("{}/session/{}/messages", self.base_url, session_id);

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_messages request failed: {}", e)))?;

        Self::ensure_success(&resp)?;

        resp.json::<Vec<AgentMessage>>()
            .await
            .map_err(|e| AppError::SidecarError(format!("get_messages parse failed: {}", e)))
    }

    /// Abort / cancel a running session.
    pub async fn abort_session(&self, session_id: &str) -> Result<(), AppError> {
        let url = format!("{}/session/{}/abort", self.base_url, session_id);

        let resp = self
            .http_client
            .post(&url)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("abort_session request failed: {}", e)))?;

        Self::ensure_success(&resp)?;
        Ok(())
    }

    /// Compact / compress a session's context.
    pub async fn compact_session(&self, session_id: &str) -> Result<(), AppError> {
        let url = format!("{}/session/{}/compact", self.base_url, session_id);

        let resp = self
            .http_client
            .post(&url)
            .send()
            .await
            .map_err(|e| {
                AppError::SidecarError(format!("compact_session request failed: {}", e))
            })?;

        Self::ensure_success(&resp)?;
        Ok(())
    }

    // ── Messaging (SSE Stream) ──────────────────────────────────────

    /// Send a message and stream SSE events back through the provided channel.
    pub async fn send_message(
        &self,
        session_id: &str,
        content: &str,
        on_event: mpsc::Sender<SseEvent>,
    ) -> Result<(), AppError> {
        let url = format!("{}/session/{}/message", self.base_url, session_id);
        let body = serde_json::json!({ "content": content });

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::SidecarError(format!("send_message request failed: {}", e)))?;

        Self::ensure_success(&resp)?;

        // Read SSE stream
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                AppError::SidecarError(format!("SSE stream read error: {}", e))
            })?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete SSE lines
            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                if let Some(event) = parse_sse_event(&event_block) {
                    let is_done = matches!(event, SseEvent::Done);
                    if on_event.send(event).await.is_err() {
                        // Receiver dropped, stop streaming
                        return Ok(());
                    }
                    if is_done {
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

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
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
    Some(SseEvent::Text {
        content: data,
    })
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
        let block = r#"data: {"type":"toolCall","name":"read","arguments":"{\"path\":\"test.rs\"}"}"#;
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
                assert!(msg.contains("401"), "error must include status code: {}", msg);
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
}