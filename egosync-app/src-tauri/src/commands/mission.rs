//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::mission`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::mission::Mission;
use crate::services::mission_inferrer::{InferredValues, InferenceEligibility};

#[tauri::command]
pub async fn mission_get(ctx: State<'_, Arc<EngineCtx>>) -> Result<Option<Mission>, AppError> {
    egosync_engine::commands::mission::mission_get(&ctx).await
}

#[tauri::command]
pub async fn mission_update(
    ctx: State<'_, Arc<EngineCtx>>,
    content: Option<String>,
    format: String,
) -> Result<Mission, AppError> {
    egosync_engine::commands::mission::mission_update(&ctx, content, format).await
}

#[tauri::command]
pub async fn mission_infer(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Option<InferredValues>, AppError> {
    egosync_engine::commands::mission::mission_infer(&ctx).await
}

#[tauri::command]
pub async fn mission_infer_eligibility(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<InferenceEligibility, AppError> {
    egosync_engine::commands::mission::mission_infer_eligibility(&ctx).await
}
