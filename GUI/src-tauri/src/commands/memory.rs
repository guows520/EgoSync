use tauri::State;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::memory::{Memory, MemorySourceMessage};
use crate::services::memory_query;

#[tauri::command]
pub async fn memory_list(
    pool: State<'_, DbPool>,
    role_id: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    memory_query::list_role_memories(
        &pool,
        role_id.as_deref(),
        category.as_deref(),
        limit,
        offset,
    )
    .await
}

#[tauri::command]
pub async fn memory_list_all(
    pool: State<'_, DbPool>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    memory_query::list_all_memories(&pool, category.as_deref(), limit, offset).await
}

#[tauri::command]
pub async fn memory_count(
    pool: State<'_, DbPool>,
    role_id: Option<String>,
    include_role_memories: Option<bool>,
    category: Option<String>,
) -> Result<usize, AppError> {
    memory_query::count_memories(
        &pool,
        role_id.as_deref(),
        include_role_memories.unwrap_or(false),
        category.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn memory_get_source_messages(
    pool: State<'_, DbPool>,
    conversations_pool: State<'_, ConversationsPool>,
    memory_id: String,
) -> Result<Vec<MemorySourceMessage>, AppError> {
    memory_query::get_source_messages(&pool, &conversations_pool, &memory_id).await
}
