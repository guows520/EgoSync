#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskOwnerType {
    Role,
    Butler,
}

impl TaskOwnerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskOwnerType::Role => "role",
            TaskOwnerType::Butler => "butler",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub owner_type: String,
    pub role_id: Option<String>,
    pub title: String,
    pub deadline: Option<String>,
    pub quadrant: String,
    pub is_big_rock: bool,
    pub is_completed: bool,
    pub completed_at: Option<String>,
    pub sort_order: i32,
    pub protection_status: String,
    pub confidence: Option<f64>,
    pub manual_override: bool,
    pub classification_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub owner_type: Option<TaskOwnerType>,
    pub role_id: Option<String>,
    pub title: String,
    pub deadline: Option<String>,
    /// 用户在 TaskModal 中显式选择的 quadrant。`None` 表示让系统自动判断。
    pub quadrant: Option<String>,
    pub is_big_rock: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CrossRoleTask {
    pub id: String,
    pub owner_type: String,
    pub role_id: Option<String>,
    pub title: String,
    pub deadline: Option<String>,
    pub quadrant: String,
    pub is_big_rock: bool,
    pub is_completed: bool,
    pub completed_at: Option<String>,
    pub sort_order: i32,
    pub protection_status: String,
    pub confidence: Option<f64>,
    pub manual_override: bool,
    pub classification_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub role_name: Option<String>,
    pub role_color: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub deadline: Option<Option<String>>,
    /// 若用户显式修改 quadrant，service 层会标记 manual_override = true。
    pub quadrant: Option<String>,
    pub is_big_rock: Option<bool>,
}
