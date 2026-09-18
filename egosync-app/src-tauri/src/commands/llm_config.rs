//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::llm_config`），
//! 壳侧薄化为 wrapper。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, NetworkLocation, UpdateLlmConfigInput};

#[tauri::command]
pub async fn llm_config_list(ctx: State<'_, Arc<EngineCtx>>) -> Result<Vec<LlmConfig>, AppError> {
    egosync_engine::commands::llm_config::llm_config_list(&ctx).await
}

#[tauri::command]
pub async fn llm_config_create(
    ctx: State<'_, Arc<EngineCtx>>,
    input: CreateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    egosync_engine::commands::llm_config::llm_config_create(&ctx, input).await
}

#[tauri::command]
pub async fn llm_config_update(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    input: UpdateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    egosync_engine::commands::llm_config::llm_config_update(&ctx, id, input).await
}

#[tauri::command]
pub async fn llm_config_delete(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::llm_config::llm_config_delete(&ctx, id).await
}

#[tauri::command]
pub async fn llm_config_set_default(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::llm_config::llm_config_set_default(&ctx, id).await
}

#[tauri::command]
pub async fn llm_config_test_connection(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::llm_config::llm_config_test_connection(&ctx, id).await
}

#[tauri::command]
pub async fn llm_config_list_models(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<Vec<String>, AppError> {
    egosync_engine::commands::llm_config::llm_config_list_models(&ctx, id).await
}

#[tauri::command]
pub async fn llm_config_list_models_by_params(
    provider: String,
    base_url: String,
    api_key: String,
    network_location: NetworkLocation,
) -> Result<Vec<String>, AppError> {
    egosync_engine::commands::llm_config::llm_config_list_models_by_params(
        provider,
        base_url,
        api_key,
        network_location,
    )
    .await
}
