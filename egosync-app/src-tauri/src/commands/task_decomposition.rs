use tauri::{AppHandle, Emitter, State};

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::task_decomposition::{
    TaskDecompositionProposal, TaskDecompositionProposalWithRole,
};
use crate::services::task_decomposition;

#[tauri::command]
pub async fn task_decomposition_list_pending(
    conversation_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<TaskDecompositionProposalWithRole>, AppError> {
    task_decomposition::list_pending(&pool, &conversation_id).await
}

#[tauri::command]
pub async fn task_decomposition_accept(
    id: String,
    pool: State<'_, DbPool>,
    app: AppHandle,
) -> Result<TaskDecompositionProposal, AppError> {
    let proposal = task_decomposition::accept(&pool, &id).await?;
    let _ = app.emit("task:tool-action", serde_json::json!({ "action": "create" }));
    Ok(proposal)
}

#[tauri::command]
pub async fn task_decomposition_keep_single(
    id: String,
    pool: State<'_, DbPool>,
    app: AppHandle,
) -> Result<TaskDecompositionProposal, AppError> {
    let proposal = task_decomposition::keep_single(&pool, &id).await?;
    let _ = app.emit("task:tool-action", serde_json::json!({ "action": "create" }));
    Ok(proposal)
}
