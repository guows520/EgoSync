//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::task`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::task::{CreateTaskInput, CrossRoleTask, Task, UpdateTaskInput};

#[tauri::command]
pub async fn task_create(
    ctx: State<'_, Arc<EngineCtx>>,
    input: CreateTaskInput,
) -> Result<Task, AppError> {
    egosync_engine::commands::task::task_create(&ctx, input).await
}

#[tauri::command]
pub async fn task_list_by_role(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
) -> Result<Vec<Task>, AppError> {
    egosync_engine::commands::task::task_list_by_role(&ctx, role_id).await
}

#[tauri::command]
pub async fn task_list_butler(ctx: State<'_, Arc<EngineCtx>>) -> Result<Vec<Task>, AppError> {
    egosync_engine::commands::task::task_list_butler(&ctx).await
}

#[tauri::command]
pub async fn task_list_all(
    ctx: State<'_, Arc<EngineCtx>>,
    quadrant: Option<String>,
    is_big_rock: Option<bool>,
) -> Result<Vec<CrossRoleTask>, AppError> {
    egosync_engine::commands::task::task_list_all(&ctx, quadrant, is_big_rock).await
}

#[tauri::command]
pub async fn task_update(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    input: UpdateTaskInput,
) -> Result<Task, AppError> {
    egosync_engine::commands::task::task_update(&ctx, id, input).await
}

#[tauri::command]
pub async fn task_delete(ctx: State<'_, Arc<EngineCtx>>, id: String) -> Result<(), AppError> {
    egosync_engine::commands::task::task_delete(&ctx, id).await
}

#[tauri::command]
pub async fn task_reorder(
    ctx: State<'_, Arc<EngineCtx>>,
    task_ids: Vec<String>,
) -> Result<(), AppError> {
    egosync_engine::commands::task::task_reorder(&ctx, task_ids).await
}

#[tauri::command]
pub async fn task_toggle_complete(
    ctx: State<'_, Arc<EngineCtx>>,
    task_id: String,
    is_completed: bool,
) -> Result<Task, AppError> {
    egosync_engine::commands::task::task_toggle_complete(&ctx, task_id, is_completed).await
}

#[tauri::command]
pub async fn task_check_protection_status(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<u64, AppError> {
    egosync_engine::commands::task::task_check_protection_status(&ctx).await
}

#[tauri::command]
pub async fn task_check_q2_reminders(ctx: State<'_, Arc<EngineCtx>>) -> Result<(), AppError> {
    egosync_engine::commands::task::task_check_q2_reminders(&ctx).await
}
