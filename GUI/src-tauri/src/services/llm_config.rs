use sqlx::SqlitePool;

use crate::db::settings as db;
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::LlmProvider;
use crate::models::settings::{CreateLlmConfigInput, LlmConfig, UpdateLlmConfigInput};
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
        _ => Box::new(OpenAiProvider::new(config.base_url, api_key, config.model)?),
    };

    provider.test_connection().await?;
    tracing::info!(config_id = %id, "LLM 连接测试成功");
    Ok(())
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
