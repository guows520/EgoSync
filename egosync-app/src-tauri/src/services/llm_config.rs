use sqlx::SqlitePool;

use crate::db::settings as db;
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::LlmProvider;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, UpdateLlmConfigInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::secret_store;

pub async fn list_configs(pool: &SqlitePool) -> Result<Vec<LlmConfig>, AppError> {
    db::list_llm_configs(pool).await
}

pub async fn create_config(
    pool: &SqlitePool,
    input: CreateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let api_key_ref = format!("llm_{}_api_key", id);

    secret_store::save_secret(&api_key_ref, &input.api_key)?;

    let is_first = db::count_llm_configs(pool).await? == 0;

    let now = current_timestamp();
    let config = LlmConfig {
        id: id.clone(),
        name: input.name,
        provider: input.provider,
        base_url: input.base_url,
        model: input.model,
        api_key_ref: api_key_ref.clone(),
        is_default: is_first,
        created_at: now.clone(),
        updated_at: now,
    };

    db::insert_llm_config(pool, &config).await?;

    tracing::info!(config_id = %id, "LLM 配置已创建");
    Ok(config)
}

pub async fn update_config(
    pool: &SqlitePool,
    id: String,
    input: UpdateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    let existing = db::get_llm_config(pool, &id).await?;

    let name = input.name.unwrap_or(existing.name);
    let provider = input.provider.unwrap_or(existing.provider);
    let base_url = input.base_url.unwrap_or(existing.base_url);
    let model = input.model.unwrap_or(existing.model);

    if let Some(api_key) = &input.api_key {
        if !api_key.is_empty() {
            secret_store::save_secret(&existing.api_key_ref, api_key)?;
        }
    }

    db::update_llm_config(
        pool,
        &id,
        &name,
        &provider,
        &base_url,
        &model,
        &existing.api_key_ref,
    )
    .await?;

    let updated = db::get_llm_config(pool, &id).await?;
    tracing::info!(config_id = %id, "LLM 配置已更新");
    Ok(updated)
}

pub async fn delete_config(pool: &SqlitePool, id: String) -> Result<(), AppError> {
    let config = db::get_llm_config(pool, &id).await?;

    db::delete_llm_config(pool, &id).await?;

    let _ = secret_store::delete_secret(&config.api_key_ref);

    tracing::info!(config_id = %id, "LLM 配置已删除");
    Ok(())
}

pub async fn set_default(pool: &SqlitePool, id: String) -> Result<(), AppError> {
    db::set_default_llm_config(pool, &id).await?;
    tracing::info!(config_id = %id, "已设为默认 LLM 配置");
    Ok(())
}

pub async fn test_connection(pool: &SqlitePool, id: String) -> Result<(), AppError> {
    let config = db::get_llm_config(pool, &id).await?;

    let api_key = secret_store::load_secret(&config.api_key_ref)?.ok_or_else(|| {
        AppError::KeyringError(format!(
            "未找到配置 '{}' 的 API Key，请重新保存",
            config.name
        ))
    })?;

    let provider: Box<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Box::new(AnthropicProvider::new(
            config.base_url,
            api_key,
            config.model,
        )?),
        "minimax" => Box::new(OpenAiProvider::new_with_reasoning_split(
            config.base_url,
            api_key,
            config.model,
        )?),
        _ => Box::new(OpenAiProvider::new(config.base_url, api_key, config.model)?),
    };

    provider.test_connection().await?;
    tracing::info!(config_id = %id, "LLM 连接测试成功");
    Ok(())
}

/// Sync the current default LLM config (provider + model + API key) into
/// the same opencode.json that AgentConfigService manages. This ensures
/// a single config file contains agents, MCP, tools AND provider/model —
/// eliminating project-vs-global config conflicts.
/// Best-effort: logs on failure.
pub async fn sync_default_to_opencode(pool: &SqlitePool, agent_config: &AgentConfigService) {
    let result: Result<(), AppError> = (|| async {
        let config = db::get_default_llm_config(pool).await?;
        let api_key = secret_store::load_secret(&config.api_key_ref)?.ok_or_else(|| {
            AppError::KeyringError(format!("未找到配置 '{}' 的 API Key", config.name))
        })?;

        // Map EgoSync provider IDs to opencode-recognized provider IDs.
        let opencode_provider = match config.provider.as_str() {
            "openai_compatible" | "openai" | "minimax" => "openai",
            other => other,
        };

        let mut options = serde_json::Map::new();
        options.insert("apiKey".to_string(), serde_json::json!(api_key));
        if !config.base_url.is_empty() {
            options.insert("baseURL".to_string(), serde_json::json!(config.base_url));
        }
        // Force Chat Completions API (not OpenAI Responses API) for
        // non-native OpenAI providers (e.g. 京东云, DeepSeek, etc.)
        if opencode_provider == "openai" && !config.base_url.is_empty() {
            options.insert("compatibility".to_string(), serde_json::json!("compatible"));
        }

        let model_str = format!("{}/{}", opencode_provider, config.model);

        // MiniMax needs reasoning_split=true to separate thinking content
        // from the main response. We inject it as a model-level option so
        // opencode's transform.ts routes it into providerOptions, which
        // @ai-sdk/openai-compatible merges into the request body.
        let model_entry = if config.provider == "minimax" {
            serde_json::json!({
                &config.model: {
                    "options": { "reasoning_split": true }
                }
            })
        } else {
            serde_json::json!({ &config.model: {} })
        };

        // Build provider entry with npm hint for compatible endpoints
        let mut provider_obj = serde_json::json!({
            "options": options,
            "models": model_entry
        });
        if opencode_provider == "openai" && !config.base_url.is_empty() {
            provider_obj.as_object_mut().unwrap().insert(
                "npm".to_string(),
                serde_json::json!("@ai-sdk/openai-compatible"),
            );
            // Ensure providerOptions namespace matches what transform.ts
            // expects for @ai-sdk/openai-compatible (issue #971 fix).
            if config.provider == "minimax" {
                provider_obj["options"]
                    .as_object_mut()
                    .unwrap()
                    .insert("name".to_string(), serde_json::json!("openai"));
            }
        }

        // Merge into the same opencode.json that AgentConfigService manages.
        let mut opencode_json = agent_config.load()?;

        let root = opencode_json
            .as_object_mut()
            .ok_or_else(|| AppError::SidecarError("opencode.json 不是 object".to_string()))?;
        root.insert(
            "$schema".to_string(),
            serde_json::json!("https://opencode.ai/config.json"),
        );
        root.insert("model".to_string(), serde_json::json!(&model_str));
        root.insert("small_model".to_string(), serde_json::json!(&model_str));

        // Merge provider — only update our provider entry, preserve others
        let providers = root
            .entry("provider")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(providers_obj) = providers.as_object_mut() {
            providers_obj.insert(opencode_provider.to_string(), provider_obj);
        }

        agent_config.save(&opencode_json)?;

        tracing::info!(
            provider = %opencode_provider,
            model = %config.model,
            "LLM provider 已同步到 opencode.json"
        );
        Ok(())
    })()
    .await;
    if let Err(e) = result {
        tracing::warn!("sync_default_to_opencode failed (degraded): {}", e);
    }
}

fn current_timestamp() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let rem = secs % 86400;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;
    let (year, month, day) = days_to_ymd(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
