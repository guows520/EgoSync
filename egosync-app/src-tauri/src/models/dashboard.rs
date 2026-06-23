#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatus {
    pub role_id: String,
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
    pub energy: i32,
    pub pending_tasks_count: i64,
    pub last_active_at: Option<String>,
    pub has_urgent: bool,
}
