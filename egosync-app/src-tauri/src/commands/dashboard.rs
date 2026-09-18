//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::dashboard`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::dashboard::{DashboardMetrics, DashboardMetricsQuery, DashboardStatus};

#[tauri::command]
pub async fn dashboard_get_status(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<DashboardStatus>, AppError> {
    egosync_engine::commands::dashboard::dashboard_get_status(&ctx).await
}

#[tauri::command]
pub async fn dashboard_get_metrics(
    ctx: State<'_, Arc<EngineCtx>>,
    query: DashboardMetricsQuery,
) -> Result<DashboardMetrics, AppError> {
    egosync_engine::commands::dashboard::dashboard_get_metrics(&ctx, query).await
}
