use tauri::State;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::dashboard::DashboardStatus;
use crate::services::dashboard_service;

#[tauri::command]
pub async fn dashboard_get_status(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Vec<DashboardStatus>, AppError> {
    dashboard_service::get_dashboard_status(&pool, &conv_pool).await
}
