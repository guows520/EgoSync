use tauri::State;

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, UpdateLlmConfigInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::llm_config as service;

#[tauri::command]
pub async fn llm_config_list(pool: State<'_, DbPool>) -> Result<Vec<LlmConfig>, AppError> {
    service::list_configs(&pool).await
}

#[tauri::command]
pub async fn llm_config_create(
    input: CreateLlmConfigInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<LlmConfig, AppError> {
    let config = service::create_config(&pool, input).await?;
    service::sync_default_to_opencode(&pool, &agent_config).await;
    Ok(config)
}

#[tauri::command]
pub async fn llm_config_update(
    id: String,
    input: UpdateLlmConfigInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<LlmConfig, AppError> {
    let config = service::update_config(&pool, id, input).await?;
    service::sync_default_to_opencode(&pool, &agent_config).await;
    Ok(config)
}

#[tauri::command]
pub async fn llm_config_delete(
    id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    service::delete_config(&pool, id).await?;
    service::sync_default_to_opencode(&pool, &agent_config).await;
    Ok(())
}

#[tauri::command]
pub async fn llm_config_set_default(
    id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    service::set_default(&pool, id).await?;
    service::sync_default_to_opencode(&pool, &agent_config).await;
    Ok(())
}

#[tauri::command]
pub async fn llm_config_test_connection(
    id: String,
    pool: State<'_, DbPool>,
) -> Result<(), AppError> {
    service::test_connection(&pool, id).await
}

#[tauri::command]
pub async fn llm_config_list_models(
    id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<String>, AppError> {
    service::list_models(&pool, id).await
}

#[tauri::command]
pub async fn llm_config_list_models_by_params(
    provider: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, AppError> {
    service::list_models_by_params(&provider, &base_url, &api_key).await
}
