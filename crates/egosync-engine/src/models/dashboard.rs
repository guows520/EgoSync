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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DashboardMetricsScope {
    All,
    Butler,
    #[serde(rename_all = "camelCase")]
    Role { role_id: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetricsQuery {
    pub scope: DashboardMetricsScope,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetrics {
    pub task_count: i64,
    pub memory_count: i64,
    pub conversation_count: i64,
    pub pending_task_count: i64,
    pub generated_at: String,
}
