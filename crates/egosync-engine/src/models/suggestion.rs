#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub role_id: String,
    pub title: String,
    pub content: String,
    pub priority: String,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub converted_task_id: Option<String>,
    pub conversation_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSuggestionInput {
    pub role_id: String,
    pub title: String,
    pub content: String,
    pub priority: String,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionWithRole {
    pub id: String,
    pub role_id: String,
    pub title: String,
    pub content: String,
    pub priority: String,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub converted_task_id: Option<String>,
    pub conversation_id: Option<String>,
    pub created_at: String,
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
}
