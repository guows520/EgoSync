//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::suggestion`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
//!
//! `confirm_and_create_task` / `reject_with_reason` 经回引保持
//! `crate::commands::suggestion::` 路径语义不变（companion_dispatch 消费者零改动）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::suggestion::{Suggestion, SuggestionWithRole};

pub use egosync_engine::commands::suggestion::{confirm_and_create_task, reject_with_reason};

#[tauri::command]
pub async fn suggestion_list_pending(
    ctx: State<'_, Arc<EngineCtx>>,
    conversation_id: String,
) -> Result<Vec<SuggestionWithRole>, AppError> {
    egosync_engine::commands::suggestion::suggestion_list_pending(&ctx, conversation_id).await
}

#[tauri::command]
pub async fn suggestion_confirm(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<Suggestion, AppError> {
    egosync_engine::commands::suggestion::suggestion_confirm(&ctx, id).await
}

#[tauri::command]
pub async fn suggestion_reject(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    reason: String,
) -> Result<Suggestion, AppError> {
    egosync_engine::commands::suggestion::suggestion_reject(&ctx, id, reason).await
}
