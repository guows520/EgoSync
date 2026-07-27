use std::collections::BTreeSet;

use sqlx::SqlitePool;

static LLM_CONFIG_WRITE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use crate::db::settings as db;
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::LlmProvider;
use crate::models::settings::{
    CreateLlmConfigInput, LlmConfig, NetworkLocation, UpdateLlmConfigInput,
};
use crate::services::agent_config::AgentConfigService;
use crate::services::secret_store;

/// 从 URL 中提取规范化的 host（不含 scheme/port/path）。
/// 用于 NO_PROXY 匹配和内外网冲突检测。
/// 返回 None 表示 URL 无法解析或缺少 host。
pub fn normalize_host(base_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(base_url.trim_end_matches('/')).ok()?;
    let host = url.host_str()?.to_string();
    if host.is_empty() {
        None
    } else {
        Some(host.to_lowercase())
    }
}

/// 检查同一 host 是否已存在不同 network_location 的配置。
/// 规则以 host 为边界，不以 scheme/port 拆分。
/// `exclude_id` 用于更新场景，排除自身。
async fn check_host_conflict(
    pool: &SqlitePool,
    base_url: &str,
    network_location: &NetworkLocation,
    exclude_id: Option<&str>,
) -> Result<(), AppError> {
    let host = match normalize_host(base_url) {
        Some(host) => host,
        None if *network_location == NetworkLocation::External => return Ok(()),
        None => {
            return Err(AppError::ValidationError(format!(
                "内网配置的 Base URL '{}' 无法解析 host，请检查 URL 格式",
                base_url
            )));
        }
    };

    let existing = db::list_llm_configs(pool).await?;
    let conflicting = existing.iter().find(|c| {
        c.id != exclude_id.unwrap_or("")
            && &c.network_location != network_location
            && normalize_host(&c.base_url)
                .map(|h| h == host)
                .unwrap_or(false)
    });

    if let Some(conflict) = conflicting {
        return Err(AppError::ValidationError(format!(
            "Host '{}' 已被配置 '{}' 使用为 {}，同一 host 不能同时配置为内外网不同位置",
            host,
            conflict.name,
            conflict.network_location
        )));
    }

    Ok(())
}

/// 合并进程原有 bypass、localhost 与所有 internal 模型 host，并稳定排序去重。
pub fn build_no_proxy_value(original: Option<&str>, configs: &[LlmConfig]) -> String {
    let mut hosts = BTreeSet::new();
    if let Some(value) = original {
        hosts.extend(value.split(',').map(str::trim).filter(|v| !v.is_empty()).map(str::to_string));
    }
    hosts.insert("localhost".to_string());
    hosts.insert("127.0.0.1".to_string());
    for config in configs {
        if config.network_location == NetworkLocation::Internal {
            if let Some(host) = normalize_host(&config.base_url) { hosts.insert(host); }
        }
    }
    hosts.into_iter().collect::<Vec<_>>().join(",")
}

pub fn process_no_proxy_value() -> Option<String> {
    let merged = [std::env::var("NO_PROXY").ok(), std::env::var("no_proxy").ok()]
        .into_iter().flatten().collect::<Vec<_>>().join(",");
    (!merged.is_empty()).then_some(merged)
}

pub async fn generate_no_proxy_value(pool: &SqlitePool, original: Option<&str>) -> Result<String, AppError> {
    Ok(build_no_proxy_value(original, &db::list_llm_configs(pool).await?))
}

pub async fn list_configs(pool: &SqlitePool) -> Result<Vec<LlmConfig>, AppError> {
    db::list_llm_configs(pool).await
}

pub async fn create_config(
    pool: &SqlitePool,
    input: CreateLlmConfigInput,
) -> Result<LlmConfig, AppError> {
    let _write_guard = LLM_CONFIG_WRITE_LOCK.lock().await;
    // internal 配置必须能解析出 host（用于 NO_PROXY）
    if input.network_location == NetworkLocation::Internal {
        normalize_host(&input.base_url).ok_or_else(|| {
            AppError::ValidationError(format!(
                "内网配置的 Base URL '{}' 无法解析 host，请检查 URL 格式",
                input.base_url
            ))
        })?;
    }

    check_host_conflict(pool, &input.base_url, &input.network_location, None).await?;

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
        network_location: input.network_location,
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
    let _write_guard = LLM_CONFIG_WRITE_LOCK.lock().await;
    let existing = db::get_llm_config(pool, &id).await?;

    let name = input.name.unwrap_or(existing.name);
    let provider = input.provider.unwrap_or(existing.provider);
    let base_url = input.base_url.unwrap_or(existing.base_url);
    let model = input.model.unwrap_or(existing.model);
    let network_location = input
        .network_location
        .unwrap_or(existing.network_location);
    let net_loc = network_location.clone();

    // internal 配置必须能解析出 host
    if net_loc == NetworkLocation::Internal {
        normalize_host(&base_url).ok_or_else(|| {
            AppError::ValidationError(format!(
                "内网配置的 Base URL '{}' 无法解析 host，请检查 URL 格式",
                base_url
            ))
        })?;
    }

    check_host_conflict(pool, &base_url, &net_loc, Some(&id)).await?;

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
        network_location.as_str(),
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

    let net_loc = config.network_location.clone();
    let no_proxy = net_loc == NetworkLocation::Internal;

    let provider: Box<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Box::new(AnthropicProvider::new(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
        "minimax" => Box::new(OpenAiProvider::new_with_reasoning_split(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
        _ => Box::new(OpenAiProvider::new(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
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
pub async fn sync_default_to_opencode(pool: &SqlitePool, agent_config: &AgentConfigService) -> Result<(), AppError> {
    (|| async {
        let config = db::get_default_llm_config(pool).await?;
        let api_key = secret_store::load_secret(&config.api_key_ref)?.ok_or_else(|| {
            AppError::KeyringError(format!("未找到配置 '{}' 的 API Key", config.name))
        })?;

        // Map EgoSync provider IDs to opencode-recognized provider IDs.
        // 所有 OpenAI 兼容的提供商都映射为 "openai"，opencode 通过 baseURL 区分。
        let opencode_provider = match config.provider.as_str() {
            "anthropic" => "anthropic",
            _ => "openai",
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
    .await
}

/// 根据已保存配置的 ID 获取模型列表。
pub async fn list_models(
    pool: &SqlitePool,
    id: String,
) -> Result<Vec<String>, AppError> {
    let config = db::get_llm_config(pool, &id).await?;
    let api_key = secret_store::load_secret(&config.api_key_ref)?.ok_or_else(|| {
        AppError::KeyringError(format!(
            "未找到配置 '{}' 的 API Key，请重新保存",
            config.name
        ))
    })?;

    let net_loc = config.network_location.clone();
    let no_proxy = net_loc == NetworkLocation::Internal;

    fetch_models_by_params(&config.provider, &config.base_url, &api_key, no_proxy).await
}

/// 直接根据 provider / base_url / api_key 获取模型列表，无需先保存配置。
/// `network_location` 决定是否绕过代理。
pub async fn list_models_by_params(
    provider: &str,
    base_url: &str,
    api_key: &str,
    network_location: &NetworkLocation,
) -> Result<Vec<String>, AppError> {
    if api_key.is_empty() {
        return Err(AppError::SidecarError("请先填写 API Key".to_string()));
    }
    let no_proxy = network_location == &NetworkLocation::Internal;
    fetch_models_by_params(provider, base_url, api_key, no_proxy).await
}

/// 内部复用：根据提供商类型、API 地址和密钥，调用 /models 端点获取可用模型列表。
/// 支持 OpenAI 兼容 API（含智谱/Deepseek/Kimi/百炼等）和 Anthropic。
async fn fetch_models_by_params(
    provider: &str,
    base_url: &str,
    api_key: &str,
    no_proxy: bool,
) -> Result<Vec<String>, AppError> {
    let base_url = base_url.trim_end_matches('/');

    // Anthropic 使用 /v1/models 端点
    let models_url = if provider == "anthropic" {
        if base_url.is_empty() {
            "https://api.anthropic.com/v1/models".to_string()
        } else {
            format!("{}/v1/models", base_url)
        }
    } else {
        // OpenAI 兼容 API：base_url + /models
        if base_url.is_empty() {
            "https://api.openai.com/v1/models".to_string()
        } else {
            format!("{}/models", base_url)
        }
    };

    tracing::info!(models_url = %models_url, provider = %provider, no_proxy, "获取模型列表");

    let mut client_builder = reqwest::Client::builder();
    if no_proxy {
        client_builder = client_builder.no_proxy();
    }
    let client = client_builder.build().map_err(|e| {
        AppError::SidecarError(format!("构建 HTTP 客户端失败: {}", e))
    })?;
    let mut req = client.get(&models_url);

    // Anthropic 使用 x-api-key 头，OpenAI 兼容使用 Bearer
    if provider == "anthropic" {
        req = req
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req.send().await.map_err(|e| {
        AppError::SidecarError(format!("请求模型列表失败: {}", e))
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::SidecarError(format!(
            "获取模型列表失败 ({}): {}",
            status, body
        )));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| {
        AppError::SidecarError(format!("解析模型列表响应失败: {}", e))
    })?;

    // OpenAI 兼容格式: { "data": [{ "id": "gpt-4o" }, ...] }
    // Anthropic 格式: { "data": [{ "id": "claude-..." }, ...] }
    let models: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_host_valid_url() {
        assert_eq!(
            normalize_host("https://api.openai.com/v1"),
            Some("api.openai.com".to_string())
        );
        assert_eq!(
            normalize_host("https://internal.corp.com:8080/v1"),
            Some("internal.corp.com".to_string())
        );
        assert_eq!(
            normalize_host("http://10.0.0.5:11434/v1"),
            Some("10.0.0.5".to_string())
        );
    }

    #[test]
    fn test_normalize_host_trailing_slash() {
        assert_eq!(
            normalize_host("https://api.openai.com/v1/"),
            Some("api.openai.com".to_string())
        );
    }

    #[test]
    fn test_normalize_host_invalid_url() {
        assert_eq!(normalize_host("not-a-url"), None);
        assert_eq!(normalize_host(""), None);
        assert_eq!(normalize_host("://missing-scheme"), None);
    }

    #[test]
    fn test_network_location_from_str() {
        assert_eq!(
            NetworkLocation::from_str("internal"),
            Ok(NetworkLocation::Internal)
        );
        assert_eq!(
            NetworkLocation::from_str("external"),
            Ok(NetworkLocation::External)
        );
        assert!(NetworkLocation::from_str("other").is_err());
    }

    #[test]
    fn test_network_location_as_str() {
        assert_eq!(NetworkLocation::Internal.as_str(), "internal");
        assert_eq!(NetworkLocation::External.as_str(), "external");
    }

    #[test]
    fn test_network_location_serde_lowercase() {
        let json = serde_json::to_string(&NetworkLocation::Internal).unwrap();
        assert_eq!(json, "\"internal\"");
        let json = serde_json::to_string(&NetworkLocation::External).unwrap();
        assert_eq!(json, "\"external\"");
    }


    async fn setup_conflict_test_db() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE llm_configs (
                id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, provider TEXT NOT NULL,
                base_url TEXT NOT NULL, model TEXT NOT NULL, api_key_ref TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0, network_location TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT ''
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_external_invalid_url_skips_host_conflict_check() {
        let pool = setup_conflict_test_db().await;
        assert!(check_host_conflict(&pool, "not-a-url", &NetworkLocation::External, None)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_same_host_different_network_location_conflicts() {
        let pool = setup_conflict_test_db().await;
        sqlx::query(
            "INSERT INTO llm_configs
             (id, name, provider, base_url, model, api_key_ref, network_location)
             VALUES ('existing', 'Internal', 'openai_compatible',
                     'https://shared.example.com/v1', 'model', 'key', 'internal')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = check_host_conflict(
            &pool,
            "https://shared.example.com:8443/other",
            &NetworkLocation::External,
            None,
        )
        .await;
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_generate_no_proxy_value_no_configs() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS llm_configs (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                base_url TEXT NOT NULL,
                model TEXT NOT NULL,
                api_key_ref TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                network_location TEXT NOT NULL DEFAULT 'external',
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let no_proxy = generate_no_proxy_value(&pool, None).await.unwrap();
        assert!(no_proxy.contains("localhost"));
        assert!(no_proxy.contains("127.0.0.1"));
    }

    #[test]
    fn test_build_no_proxy_preserves_original_and_sorts_stably() {
        let configs = vec![LlmConfig { id: "1".into(), name: "internal".into(), provider: "openai_compatible".into(), base_url: "https://Z.Internal.test/v1".into(), model: "m".into(), api_key_ref: "k".into(), is_default: false, network_location: NetworkLocation::Internal, created_at: "2026-01-01".into(), updated_at: "2026-01-01".into() }];
        assert_eq!(build_no_proxy_value(Some("legacy.test,localhost, legacy.test"), &configs), "127.0.0.1,legacy.test,localhost,z.internal.test");
    }

    #[tokio::test]
    async fn test_generate_no_proxy_value_surfaces_db_error() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        assert!(matches!(generate_no_proxy_value(&pool, None).await, Err(AppError::DbError(_))));
    }

    #[tokio::test]
    async fn test_generate_no_proxy_value_with_internal() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS llm_configs (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                base_url TEXT NOT NULL,
                model TEXT NOT NULL,
                api_key_ref TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                network_location TEXT NOT NULL DEFAULT 'external',
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, network_location)
             VALUES ('test-1', 'Internal', 'openai_compatible', 'https://internal.corp.com/v1', 'gpt-4o', 'key1', 'internal')",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, network_location)
             VALUES ('test-2', 'External', 'openai_compatible', 'https://api.openai.com/v1', 'gpt-4o', 'key2', 'external')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let no_proxy = generate_no_proxy_value(&pool, None).await.unwrap();
        assert!(no_proxy.contains("localhost"));
        assert!(no_proxy.contains("127.0.0.1"));
        assert!(no_proxy.contains("internal.corp.com"));
        assert!(!no_proxy.contains("api.openai.com"));
    }
}
