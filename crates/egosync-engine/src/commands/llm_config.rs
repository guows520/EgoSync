//! llm_config 域命令体（Story 15.4 自壳 `commands/llm_config.rs` 平移，业务
//! 逻辑零改动；State 取值改 `&EngineCtx`，密钥经 SecretStore 接缝注入）。

use std::sync::Arc;

use crate::commands::ctx::EngineCtx;
use crate::services::secret_store::SecretStore;
use crate::services::sidecar::SidecarManager;
use tokio::sync::Mutex;

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, NetworkLocation, UpdateLlmConfigInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::llm_config as service;

async fn refresh_runtime(
    pool: &sqlx::SqlitePool,
    secrets: &dyn SecretStore,
    agent_config: &AgentConfigService,
    sidecar: &Arc<Mutex<SidecarManager>>,
) -> Result<(), AppError> {
    let mut failures = Vec::new();
    if let Err(e) = service::sync_default_to_opencode(pool, secrets, agent_config).await { failures.push(format!("同步 opencode 配置失败: {}", e)); }
    let original = service::process_no_proxy_value();
    match service::generate_no_proxy_value(pool, original.as_deref()).await {
        Ok(value) => { let mut manager = sidecar.lock().await; if manager.update_env("NO_PROXY", value) { if let Err(e) = manager.restart().await { failures.push(format!("重启 sidecar 失败: {}", e)); } } }
        Err(e) => failures.push(format!("生成 NO_PROXY 失败: {}", e)),
    }
    if failures.is_empty() { Ok(()) } else { Err(AppError::RuntimeRefreshError(failures.join("；"))) }
}

pub async fn llm_config_list(ctx: &EngineCtx) -> Result<Vec<LlmConfig>, AppError> {
    service::list_configs(&ctx.pool).await
}

pub async fn llm_config_create(
    ctx: &EngineCtx,
    input: CreateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    let (pool, secrets, agent_config, sidecar): (
        &DbPool,
        &Arc<dyn SecretStore>,
        &AgentConfigService,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.pool, &ctx.secrets, &ctx.agent_config, &ctx.sidecar);
    let config = service::create_config(pool, secrets.as_ref(), input).await?;
    refresh_runtime(pool, secrets.as_ref(), agent_config, sidecar).await?;
    Ok(config)
}

pub async fn llm_config_update(
    ctx: &EngineCtx,
    id: String,
    input: UpdateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    let (pool, secrets, agent_config, sidecar): (
        &DbPool,
        &Arc<dyn SecretStore>,
        &AgentConfigService,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.pool, &ctx.secrets, &ctx.agent_config, &ctx.sidecar);
    let config = service::update_config(pool, secrets.as_ref(), id, input).await?;
    refresh_runtime(pool, secrets.as_ref(), agent_config, sidecar).await?;
    Ok(config)
}

pub async fn llm_config_delete(ctx: &EngineCtx, id: String) -> Result<(), AppError> {
    let (pool, secrets, agent_config, sidecar): (
        &DbPool,
        &Arc<dyn SecretStore>,
        &AgentConfigService,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.pool, &ctx.secrets, &ctx.agent_config, &ctx.sidecar);
    service::delete_config(pool, secrets.as_ref(), id).await?;
    refresh_runtime(pool, secrets.as_ref(), agent_config, sidecar).await?;
    Ok(())
}

pub async fn llm_config_set_default(ctx: &EngineCtx, id: String) -> Result<(), AppError> {
    service::set_default(&ctx.pool, id).await?;
    service::sync_default_to_opencode(&ctx.pool, ctx.secrets.as_ref(), &ctx.agent_config)
        .await
        .map_err(|e| AppError::RuntimeRefreshError(e.to_string()))?;
    Ok(())
}

pub async fn llm_config_test_connection(ctx: &EngineCtx, id: String) -> Result<(), AppError> {
    service::test_connection(&ctx.pool, ctx.secrets.as_ref(), id).await
}

pub async fn llm_config_list_models(ctx: &EngineCtx, id: String) -> Result<Vec<String>, AppError> {
    service::list_models(&ctx.pool, ctx.secrets.as_ref(), id).await
}

pub async fn llm_config_list_models_by_params(
    provider: String,
    base_url: String,
    api_key: String,
    network_location: NetworkLocation,
) -> Result<Vec<String>, AppError> {
    service::list_models_by_params(&provider, &base_url, &api_key, &network_location).await
}
