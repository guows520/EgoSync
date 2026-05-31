use tauri::State;

use crate::db::memories;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::memory::Memory;

#[tauri::command]
pub async fn memory_list(
    pool: State<'_, DbPool>,
    role_id: Option<String>,
) -> Result<Vec<Memory>, AppError> {
    memories::list_memories(&pool, role_id.as_deref()).await
}

#[tauri::command]
pub async fn memory_list_all(pool: State<'_, DbPool>) -> Result<Vec<Memory>, AppError> {
    memories::list_all_memories(&pool).await
}
