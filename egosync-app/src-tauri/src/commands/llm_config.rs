use std::sync::Arc;

use egosync_engine::services::secret_store::SecretStore;
use tauri::State;
use tokio::sync::Mutex;

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, NetworkLocation, UpdateLlmConfigInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::llm_config as service;
use crate::services::secret_store_keyring::KeyringSecretStore;
use crate::services::sidecar::SidecarManager;

async fn refresh_runtime(pool: &sqlx::SqlitePool, secrets: &dyn SecretStore, agent_config: &AgentConfigService, sidecar: &Arc<Mutex<SidecarManager>>) -> Result<(), AppError> {
    let mut failures = Vec::new();
    if let Err(e) = service::sync_default_to_opencode(pool, secrets, agent_config).await { failures.push(format!("同步 opencode 配置失败: {}", e)); }
    let original = service::process_no_proxy_value();
    match service::generate_no_proxy_value(pool, original.as_deref()).await {
        Ok(value) => { let mut manager = sidecar.lock().await; if manager.update_env("NO_PROXY", value) { if let Err(e) = manager.restart().await { failures.push(format!("重启 sidecar 失败: {}", e)); } } }
        Err(e) => failures.push(format!("生成 NO_PROXY 失败: {}", e)),
    }
    if failures.is_empty() { Ok(()) } else { Err(AppError::RuntimeRefreshError(failures.join("；"))) }
}

#[tauri::command]
pub async fn llm_config_list(pool: State<'_, DbPool>) -> Result<Vec<LlmConfig>, AppError> {
    service::list_configs(&pool).await
}

#[tauri::command]
pub async fn llm_config_create(
    input: CreateLlmConfigInput,
    pool: State<'_, DbPool>,
    secrets: State<'_, KeyringSecretStore>,
    agent_config: State<'_, AgentConfigService>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<LlmConfig, AppError> {
    let config = service::create_config(&pool, secrets.inner(), input).await?;
    refresh_runtime(&pool, secrets.inner(), &agent_config, &sidecar).await?;
    Ok(config)
}

#[tauri::command]
pub async fn llm_config_update(
    id: String,
    input: UpdateLlmConfigInput,
    pool: State<'_, DbPool>,
    secrets: State<'_, KeyringSecretStore>,
    agent_config: State<'_, AgentConfigService>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<LlmConfig, AppError> {
    let config = service::update_config(&pool, secrets.inner(), id, input).await?;
    refresh_runtime(&pool, secrets.inner(), &agent_config, &sidecar).await?;
    Ok(config)
}

#[tauri::command]
pub async fn llm_config_delete(
    id: String,
    pool: State<'_, DbPool>,
    secrets: State<'_, KeyringSecretStore>,
    agent_config: State<'_, AgentConfigService>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<(), AppError> {
    service::delete_config(&pool, secrets.inner(), id).await?;
    refresh_runtime(&pool, secrets.inner(), &agent_config, &sidecar).await?;
    Ok(())
}

#[tauri::command]
pub async fn llm_config_set_default(
    id: String,
    pool: State<'_, DbPool>,
    secrets: State<'_, KeyringSecretStore>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    service::set_default(&pool, id).await?;
    service::sync_default_to_opencode(&pool, secrets.inner(), &agent_config).await
        .map_err(|e| AppError::RuntimeRefreshError(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn llm_config_test_connection(
    id: String,
    pool: State<'_, DbPool>,
    secrets: State<'_, KeyringSecretStore>,
) -> Result<(), AppError> {
    service::test_connection(&pool, secrets.inner(), id).await
}

#[tauri::command]
pub async fn llm_config_list_models(
    id: String,
    pool: State<'_, DbPool>,
    secrets: State<'_, KeyringSecretStore>,
) -> Result<Vec<String>, AppError> {
    service::list_models(&pool, secrets.inner(), id).await
}

#[tauri::command]
pub async fn llm_config_list_models_by_params(
    provider: String,
    base_url: String,
    api_key: String,
    network_location: NetworkLocation,
) -> Result<Vec<String>, AppError> {
    service::list_models_by_params(&provider, &base_url, &api_key, &network_location).await
}
