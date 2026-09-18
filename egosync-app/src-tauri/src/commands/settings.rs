//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::settings`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use egosync_engine::commands::settings::ScheduleConfig;
use tauri::State;

use crate::error::AppError;

#[tauri::command]
pub async fn settings_get_schedule(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<ScheduleConfig, AppError> {
    egosync_engine::commands::settings::settings_get_schedule(&ctx).await
}

#[tauri::command]
pub async fn settings_update_schedule(
    ctx: State<'_, Arc<EngineCtx>>,
    briefing_time: Option<String>,
    review_day: Option<String>,
    review_time: Option<String>,
    bigrock_reminder_day: Option<String>,
    bigrock_reminder_time: Option<String>,
) -> Result<ScheduleConfig, AppError> {
    egosync_engine::commands::settings::settings_update_schedule(
        &ctx,
        briefing_time,
        review_day,
        review_time,
        bigrock_reminder_day,
        bigrock_reminder_time,
    )
    .await
}
