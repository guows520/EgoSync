#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub role_id: String,
    pub title: String,
    pub deadline: Option<String>,
    pub quadrant: String,
    pub is_big_rock: bool,
    pub is_completed: bool,
    pub completed_at: Option<String>,
    pub sort_order: i32,
    pub protection_status: String,
    pub confidence: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub role_id: String,
    pub title: String,
    pub deadline: Option<String>,
    pub quadrant: Option<String>,
    pub is_big_rock: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub deadline: Option<Option<String>>,
    pub quadrant: Option<String>,
    pub is_big_rock: Option<bool>,
}
