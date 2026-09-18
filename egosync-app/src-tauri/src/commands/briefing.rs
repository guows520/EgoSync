//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::briefing`），
//! 壳侧薄化为 wrapper。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::briefing::Briefing;

#[tauri::command]
pub async fn briefing_get_latest(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Option<Briefing>, AppError> {
    egosync_engine::commands::briefing::briefing_get_latest(&ctx).await
}

#[tauri::command]
pub async fn briefing_generate_now(ctx: State<'_, Arc<EngineCtx>>) -> Result<bool, AppError> {
    egosync_engine::commands::briefing::briefing_generate_now(&ctx).await
}
