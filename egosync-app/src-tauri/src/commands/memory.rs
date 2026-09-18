//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::memory`），
//! 壳侧薄化为 wrapper（State<Arc<EngineCtx>> + 参数透传，桌面零回归）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::memory::{Memory, MemorySourceMessage};

#[tauri::command]
pub async fn memory_list(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    egosync_engine::commands::memory::memory_list(&ctx, role_id, category, limit, offset).await
}

#[tauri::command]
pub async fn memory_list_all(
    ctx: State<'_, Arc<EngineCtx>>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    egosync_engine::commands::memory::memory_list_all(&ctx, category, limit, offset).await
}

#[tauri::command]
pub async fn memory_count(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: Option<String>,
    include_role_memories: Option<bool>,
    category: Option<String>,
) -> Result<usize, AppError> {
    egosync_engine::commands::memory::memory_count(
        &ctx,
        role_id,
        include_role_memories,
        category,
    )
    .await
}

#[tauri::command]
pub async fn memory_get_source_messages(
    ctx: State<'_, Arc<EngineCtx>>,
    memory_id: String,
) -> Result<Vec<MemorySourceMessage>, AppError> {
    egosync_engine::commands::memory::memory_get_source_messages(&ctx, memory_id).await
}

#[tauri::command]
pub async fn memory_delete(
    ctx: State<'_, Arc<EngineCtx>>,
    memory_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::memory::memory_delete(&ctx, memory_id).await
}
