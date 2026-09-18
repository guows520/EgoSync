//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::scheduler`），
//! 壳侧薄化为 wrapper。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use egosync_engine::commands::scheduler::SchedulerTimes;
use tauri::State;

use crate::error::AppError;

#[tauri::command]
pub async fn scheduler_get_times(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<SchedulerTimes, AppError> {
    egosync_engine::commands::scheduler::scheduler_get_times(&ctx).await
}

#[tauri::command]
pub async fn scheduler_set_times(
    ctx: State<'_, Arc<EngineCtx>>,
    moderate: Vec<String>,
    proactive: Vec<String>,
) -> Result<SchedulerTimes, AppError> {
    egosync_engine::commands::scheduler::scheduler_set_times(&ctx, moderate, proactive).await
}
