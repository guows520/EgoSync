#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Mission {
    pub id: String,
    pub content: Option<String>,
    pub format: String,
    pub updated_at: String,
}
