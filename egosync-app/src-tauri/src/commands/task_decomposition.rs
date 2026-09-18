//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::task_decomposition`），
//! 壳侧薄化为 wrapper；其 service 随迁引擎（services/mod.rs 回引）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::task_decomposition::{
    TaskDecompositionProposal, TaskDecompositionProposalWithRole,
};

#[tauri::command]
pub async fn task_decomposition_list_pending(
    ctx: State<'_, Arc<EngineCtx>>,
    conversation_id: String,
) -> Result<Vec<TaskDecompositionProposalWithRole>, AppError> {
    egosync_engine::commands::task_decomposition::task_decomposition_list_pending(
        &ctx,
        conversation_id,
    )
    .await
}

#[tauri::command]
pub async fn task_decomposition_accept(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<TaskDecompositionProposal, AppError> {
    egosync_engine::commands::task_decomposition::task_decomposition_accept(&ctx, id).await
}

#[tauri::command]
pub async fn task_decomposition_keep_single(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<TaskDecompositionProposal, AppError> {
    egosync_engine::commands::task_decomposition::task_decomposition_keep_single(&ctx, id).await
}
