#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BigRockProtectionReminder {
    pub id: String,
    pub task_id: String,
    pub reminded_count: i64,
    pub last_reminded_at: String,
    pub created_at: String,
}
