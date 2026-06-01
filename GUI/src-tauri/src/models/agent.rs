use serde::{Deserialize, Serialize};

/// opencode session information returned when creating a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
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
    Thinking { content: String },
    ToolCall { name: String, arguments: String },
    Done,
    Error { message: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpencodeCompletedMessage {
    pub text: String,
    pub thinking: String,
}

impl OpencodeCompletedMessage {
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        let mut result = Self::default();
        extract_completed_message(value, &mut result);
        if result.text.is_empty() && result.thinking.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

fn extract_completed_message(value: &serde_json::Value, result: &mut OpencodeCompletedMessage) {
    if let Some(array) = value.as_array() {
        for item in array {
            extract_completed_message(item, result);
        }
        return;
    }

    let Some(object) = value.as_object() else {
        return;
    };

    for key in ["message", "info", "data"] {
        if let Some(nested) = object.get(key) {
            extract_completed_message(nested, result);
        }
    }

    let role = object.get("role").and_then(|v| v.as_str());
    if role == Some("user") {
        return;
    }

    let text_before_parts = result.text.len();
    let thinking_before_parts = result.thinking.len();
    if let Some(parts) = object.get("parts").and_then(|v| v.as_array()) {
        for part in parts {
            let part_type = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let text = part.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                continue;
            }
            match part_type {
                "text" => result.text.push_str(text),
                "reasoning" | "thinking" => result.thinking.push_str(text),
                _ => {}
            }
        }
        if result.text.len() != text_before_parts || result.thinking.len() != thinking_before_parts {
            return;
        }
    }

    if let Some(content) = object.get("content").and_then(|v| v.as_str()) {
        result.text.push_str(content);
    }
}

/// A single opencode message part (text, reasoning, tool-invocation, etc.).
/// We only consume the fields we route on; opencode may add more.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusPart {
    pub id: String,
    #[serde(rename = "type")]
    pub part_type: String,
    #[serde(default)]
    pub text: String,
    /// The message this part belongs to. Used to look up the message's role
    /// (user vs assistant) so we can filter out user-echo parts.
    #[serde(default, rename = "messageID")]
    pub message_id: String,
    /// For tool-invocation parts: the tool invocation details.
    #[serde(default)]
    pub tool_invocation: Option<BusToolInvocation>,
}

/// Tool invocation details within a message part.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusToolInvocation {
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub args: serde_json::Value,
    /// The tool result (output text from execute())
    #[serde(default)]
    pub result: Option<BusToolResult>,
    #[serde(default)]
    pub state: String,
}

/// Tool result within a tool invocation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusToolResult {
    #[serde(default)]
    pub output: String,
}

/// Raw bus event envelope from opencode `GET /event` SSE stream.
/// Body is `{ type: <event-name>, properties: <event-payload> }`.
#[derive(Debug, Clone, Deserialize)]
pub struct BusEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub properties: serde_json::Value,
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
    fn test_opencode_completed_message_extracts_parts_text_and_thinking() {
        let value = serde_json::json!({
            "id": "msg-1",
            "role": "assistant",
            "parts": [
                { "type": "reasoning", "text": "先分析" },
                { "type": "text", "text": "最终回答" }
            ]
        });

        let completed = OpencodeCompletedMessage::from_value(&value).expect("parse completed message");

        assert_eq!(completed.thinking, "先分析");
        assert_eq!(completed.text, "最终回答");
    }

    #[test]
    fn test_opencode_completed_message_ignores_user_message_parts() {
        let value = serde_json::json!({
            "message": {
                "id": "msg-user",
                "role": "user",
                "parts": [{ "type": "text", "text": "用户原文" }]
            }
        });

        assert!(OpencodeCompletedMessage::from_value(&value).is_none());
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
