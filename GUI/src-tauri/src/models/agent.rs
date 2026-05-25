use serde::{Deserialize, Serialize};

/// opencode session information returned when creating a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub directory: String,
    pub created_at: Option<String>,
}

/// A single message within an opencode session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    pub created_at: Option<String>,
}

/// Server-Sent Event variants from opencode streaming responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SseEvent {
    Text { content: String },
    ToolCall { name: String, arguments: String },
    Done,
    Error { message: String },
}

/// opencode configuration snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpencodeConfig {
    #[serde(default)]
    pub provider: serde_json::Value,
    #[serde(default)]
    pub agent: serde_json::Value,
}

/// Provider information from opencode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub models: Vec<String>,
}

/// Agent information from opencode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub description: String,
}

/// Sidecar status response for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarStatus {
    pub running: bool,
    pub port: u16,
    pub uptime_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_roundtrip() {
        let info = SessionInfo {
            id: "sess-123".into(),
            agent: "butler".into(),
            directory: "/tmp".into(),
            created_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: SessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "sess-123");
        assert_eq!(parsed.agent, "butler");
    }

    #[test]
    fn test_agent_message_deserialize() {
        let json = r#"{
            "id": "msg-1",
            "role": "assistant",
            "content": "Hello",
            "toolCalls": [],
            "createdAt": null
        }"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.id, "msg-1");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_sse_event_text_roundtrip() {
        let evt = SseEvent::Text {
            content: "token".into(),
        };
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        let parsed: SseEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            SseEvent::Text { content } => assert_eq!(content, "token"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_sse_event_done_roundtrip() {
        let evt = SseEvent::Done;
        let json = serde_json::to_string(&evt).unwrap();
        let parsed: SseEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, SseEvent::Done));
    }

    #[test]
    fn test_provider_info_deserialize() {
        let json = r#"{"id":"openai","name":"OpenAI","models":["gpt-4o"]}"#;
        let info: ProviderInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "openai");
        assert_eq!(info.models, vec!["gpt-4o"]);
    }

    #[test]
    fn test_agent_info_deserialize() {
        let json = r#"{"id":"butler","name":"管家","mode":"primary","description":"EgoSync管家"}"#;
        let info: AgentInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "butler");
        assert_eq!(info.mode, "primary");
    }

    #[test]
    fn test_sidecar_status_serialize() {
        let status = SidecarStatus {
            running: true,
            port: 4096,
            uptime_secs: Some(120),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"running\":true"));
        assert!(json.contains("\"port\":4096"));
        assert!(json.contains("\"uptimeSecs\":120"));
    }
}