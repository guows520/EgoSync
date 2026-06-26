#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Briefing {
    pub id: String,
    pub content: String,
    pub date: String,
    pub created_at: String,
}
