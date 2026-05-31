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
