#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: String,
    pub role_id: Option<String>,
    pub category: String,
    pub content: String,
    pub source_conversation_id: String,
    pub source_message_ids: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedMemory {
    pub category: String,
    pub content: String,
    pub source_message_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySourceMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub is_source: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_source_message_serializes_only_public_trace_fields() {
        let message = MemorySourceMessage {
            id: "msg-1".to_string(),
            conversation_id: "conv-1".to_string(),
            role: "user".to_string(),
            content: "原始问题".to_string(),
            created_at: "2026-05-31T12:00:00Z".to_string(),
            is_source: true,
        };

        let value = serde_json::to_value(message).expect("serialize source message");

        assert_eq!(value["conversationId"], "conv-1");
        assert_eq!(value["createdAt"], "2026-05-31T12:00:00Z");
        assert_eq!(value["isSource"], true);
        assert!(value.get("thinkingContent").is_none());
        assert!(value.get("routingMetadata").is_none());
    }
}
