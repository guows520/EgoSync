//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::review`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use egosync_engine::commands::review::BigRockPlanItem;
use tauri::State;

use crate::error::AppError;
use crate::models::task::Task;
use crate::models::weekly_review::WeeklyReview;
use crate::services::review_generator;

#[tauri::command]
pub async fn review_get_latest(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Option<WeeklyReview>, AppError> {
    egosync_engine::commands::review::review_get_latest(&ctx).await
}

#[tauri::command]
pub async fn review_get_by_week(
    ctx: State<'_, Arc<EngineCtx>>,
    week_start: String,
) -> Result<Option<WeeklyReview>, AppError> {
    egosync_engine::commands::review::review_get_by_week(&ctx, week_start).await
}

#[tauri::command]
pub async fn review_generate_now(ctx: State<'_, Arc<EngineCtx>>) -> Result<bool, AppError> {
    egosync_engine::commands::review::review_generate_now(&ctx).await
}

#[tauri::command]
pub async fn review_plan_bigrocks(
    ctx: State<'_, Arc<EngineCtx>>,
    items: Vec<BigRockPlanItem>,
) -> Result<Vec<Task>, AppError> {
    egosync_engine::commands::review::review_plan_bigrocks(&ctx, items).await
}

#[tauri::command]
pub async fn review_get_bigrock_suggestions(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<review_generator::RoleBigRockSuggestions>, AppError> {
    egosync_engine::commands::review::review_get_bigrock_suggestions(&ctx).await
}
