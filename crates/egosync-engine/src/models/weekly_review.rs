#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReview {
    pub id: String,
    pub week_start: String,
    pub week_end: String,
    pub summary: String,
    pub energy_trends: String,
    pub bigrock_status: String,
    pub new_memories_count: i64,
    pub created_at: String,
}
