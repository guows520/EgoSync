use tauri::State;

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, UpdateLlmConfigInput};
use crate::services::llm_config as service;

#[tauri::command]
pub async fn llm_config_list(pool: State<'_, DbPool>) -> Result<Vec<LlmConfig>, AppError> {
    service::list_configs(&pool).await
}

#[tauri::command]
pub async fn llm_config_create(
    input: CreateLlmConfigInput,
    pool: State<'_, DbPool>,
) -> Result<LlmConfig, AppError> {
    service::create_config(&pool, input).await
}

#[tauri::command]
pub async fn llm_config_update(
    id: String,
    input: UpdateLlmConfigInput,
    pool: State<'_, DbPool>,
) -> Result<LlmConfig, AppError> {
    service::update_config(&pool, id, input).await
}

#[tauri::command]
pub async fn llm_config_delete(id: String, pool: State<'_, DbPool>) -> Result<(), AppError> {
    service::delete_config(&pool, id).await
}

#[tauri::command]
pub async fn llm_config_set_default(id: String, pool: State<'_, DbPool>) -> Result<(), AppError> {
    service::set_default(&pool, id).await
}

#[tauri::command]
pub async fn llm_config_test_connection(
    id: String,
    pool: State<'_, DbPool>,
) -> Result<(), AppError> {
    service::test_connection(&pool, id).await
}
