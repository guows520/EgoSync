use std::collections::BTreeSet;
use std::sync::Arc;

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
use crate::services::secret_store::SecretStore;

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

/// 密钥缺失统一错误（NFR-C7，人工裁决 A 2026-09-21）：SecretStore 对某
/// `api_key_ref` 无值（secrets.json 与 env 双通道皆缺）时的单一结构化
/// 错误。
///
/// 此前 6 处读取路径（list_models / test_connection / resolve_default_
/// provider / sync_default_to_opencode / mission 推断 / 任务分类）各自
/// 拼文案（措辞漂移且无重录指引）；收敛为本助手后所有触发面返回同一
/// 文案。variant 维持 `KeyringError`（15.1 defer 裁决不改名——收敛的是
/// 文案拼装，不是错误形状）。17.3 导入后的批量密钥可达性探测复用同一
/// 助手（deferred-work 已登记接线计划）。
///
/// 文案刻意包含重录路径：云端部署是密钥缺失的常态路径（桌面 keyring
/// 不随数据导出迁移），指路即修复指引。
pub fn missing_api_key_error(config_name: &str) -> AppError {
    AppError::KeyringError(format!(
        "未找到配置 '{}' 的 API Key（密钥缺失，常见于桌面→云端迁移后）。\
         请前往 设置 → 模型服务配置，编辑该配置并重新保存密钥",
        config_name
    ))
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
    secrets: &dyn SecretStore,
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

    secrets.save_secret(&api_key_ref, &input.api_key)?;

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
    secrets: &dyn SecretStore,
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
            secrets.save_secret(&existing.api_key_ref, api_key)?;
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

pub async fn delete_config(
    pool: &SqlitePool,
    secrets: &dyn SecretStore,
    id: String,
) -> Result<(), AppError> {
    let config = db::get_llm_config(pool, &id).await?;

    db::delete_llm_config(pool, &id).await?;

    let _ = secrets.delete_secret(&config.api_key_ref);

    tracing::info!(config_id = %id, "LLM 配置已删除");
    Ok(())
}

pub async fn set_default(pool: &SqlitePool, id: String) -> Result<(), AppError> {
    db::set_default_llm_config(pool, &id).await?;
    tracing::info!(config_id = %id, "已设为默认 LLM 配置");
    Ok(())
}

pub async fn test_connection(
    pool: &SqlitePool,
    secrets: &dyn SecretStore,
    id: String,
) -> Result<(), AppError> {
    let config = db::get_llm_config(pool, &id).await?;

    let api_key = secrets
        .load_secret(&config.api_key_ref)?
        .ok_or_else(|| missing_api_key_error(&config.name))?;

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

/// Story 15.2: 自桌面壳 agent_engine 提取的默认 provider 构造（provider 构造归位）。
///
/// 密钥经 SecretStore 接缝读取；返回 `Arc<dyn LlmProvider>` 供
/// briefing / review / suggestion / 委派执行等调用方共享。
pub async fn resolve_default_provider(
    main_pool: &SqlitePool,
    secret: &dyn SecretStore,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    let config = db::get_default_llm_config(main_pool).await?;

    let api_key = secret
        .load_secret(&config.api_key_ref)?
        .ok_or_else(|| missing_api_key_error(&config.name))?;

    let net_loc = config.network_location.clone();
    let no_proxy = net_loc == NetworkLocation::Internal;

    let provider: Arc<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Arc::new(AnthropicProvider::new(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
        "minimax" => Arc::new(OpenAiProvider::new_with_reasoning_split(
            config.base_url,
            api_key,
            config.model,
            no_proxy,
        )?),
        _ => Arc::new(OpenAiProvider::new(config.base_url, api_key, config.model, no_proxy)?),
    };

    Ok(provider)
}

/// Sync the current default LLM config (provider + model + API key) into
/// the same opencode.json that AgentConfigService manages. This ensures
/// a single config file contains agents, MCP, tools AND provider/model —
/// eliminating project-vs-global config conflicts.
/// Best-effort: logs on failure.
pub async fn sync_default_to_opencode(
    pool: &SqlitePool,
    secrets: &dyn SecretStore,
    agent_config: &AgentConfigService,
) -> Result<(), AppError> {
    (|| async {
        let config = db::get_default_llm_config(pool).await?;
        let api_key = secrets
            .load_secret(&config.api_key_ref)?
            .ok_or_else(|| missing_api_key_error(&config.name))?;

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
    secrets: &dyn SecretStore,
    id: String,
) -> Result<Vec<String>, AppError> {
    let config = db::get_llm_config(pool, &id).await?;
    let api_key = secrets
        .load_secret(&config.api_key_ref)?
        .ok_or_else(|| missing_api_key_error(&config.name))?;

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

    /// 测试用内存密钥库（与 data_export 测试同构）：验证 create/delete 真实走
    /// SecretStore 接缝——Story 15.1 把 keyring 自由函数改造为接缝注入，
    /// 落库形状（llm_{id}_api_key）与删除时序是桌面零回归的契约，
    /// 若接缝误接线（改回直连/漏传/键名漂移）下列断言会失败。
    #[derive(Default)]
    struct InMemorySecretStore(std::sync::Mutex<std::collections::HashMap<String, String>>);

    impl crate::services::secret_store::SecretStore for InMemorySecretStore {
        fn save_secret(&self, key: &str, value: &str) -> Result<(), AppError> {
            self.0
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }
        fn load_secret(&self, key: &str) -> Result<Option<String>, AppError> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        fn delete_secret(&self, key: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_and_delete_config_drive_secret_store_seam() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let pool = crate::db::pool::init_db(&dir.path().join("seam-test.db"))
            .await
            .expect("init db");
        let secrets = InMemorySecretStore::default();

        let config = create_config(
            &pool,
            &secrets,
            CreateLlmConfigInput {
                name: "接缝测试".into(),
                provider: "openai_compatible".into(),
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-4o".into(),
                api_key: "sk-seam-test".into(),
                network_location: NetworkLocation::External,
            },
        )
        .await
        .expect("create config");

        // 接缝落库形状：密钥必须以 llm_{id}_api_key 为键写入 SecretStore
        let key_ref = format!("llm_{}_api_key", config.id);
        assert_eq!(
            secrets.load_secret(&key_ref).expect("load secret"),
            Some("sk-seam-test".to_string())
        );
        assert_eq!(config.api_key_ref, key_ref);

        // 删除时序：delete_config 须同步清掉 SecretStore 中的条目
        delete_config(&pool, &secrets, config.id.clone())
            .await
            .expect("delete config");
        assert_eq!(secrets.load_secret(&key_ref).expect("load secret"), None);
    }

    /// NFR-C7（人工裁决 A，Story 17.1）：密钥缺失单一结构化错误契约。
    ///
    /// - 形状：`KeyringError`（15.1 defer 裁决维持 variant 名——收敛的是
    ///   文案拼装，不是错误形状；序列化仍为单键 map，HTTP/桌面两通道
    ///   形状不变）；
    /// - 文案：含配置名（用户能对上是哪条配置）+ 重录路径（设置 →
    ///   模型服务配置——云端部署是密钥缺失的常态路径，指路即修复指引）；
    /// - 6 处调用点共用本助手 ⇒ 文案一致性由构造保证（无第二拼装点）。
    #[test]
    fn missing_api_key_error_is_structured_and_points_to_reentry_path() {
        let err = missing_api_key_error("DeepSeek 主力");
        assert!(
            matches!(err, AppError::KeyringError(_)),
            "variant 必须维持 KeyringError（15.1 defer 裁决）"
        );
        // 序列化形状：单键 map（HTTP 200 body / 桌面 rejection 同构）
        let value = serde_json::to_value(&err).expect("序列化");
        let map = value.as_object().expect("AppError 序列化为 object");
        assert_eq!(map.len(), 1, "单键 map 形状: {map:?}");
        let msg = map.get("KeyringError").and_then(|v| v.as_str()).expect("KeyringError 键");
        assert!(msg.contains("DeepSeek 主力"), "文案须含配置名: {msg}");
        assert!(
            msg.contains("设置 → 模型服务配置"),
            "文案须含重录路径: {msg}"
        );
        assert!(
            msg.contains("API Key"),
            "文案须说明缺失的是 API Key: {msg}"
        );
    }

    /// NFR-C7 调用路径钉死（17.1 评审 #29）：构造函数测试只钉形状，
    /// 任一调用点回退旧拼装文案无门禁——本测试**经真实调用路径**触发
    ///（库内配置 + SecretStore 键被删 ⇒ 缺失态），断言错误文案与构造
    /// 契约一致（配置名 + 重录路径）。覆盖 test_connection / list_models /
    /// resolve_default_provider 三个读取端点（同一助手，一处证明）。
    #[tokio::test]
    async fn missing_key_call_paths_surface_structured_reentry_error() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let pool = crate::db::pool::init_db(&dir.path().join("missing-key.db"))
            .await
            .expect("init db");
        let secrets = InMemorySecretStore::default();

        let config = create_config(
            &pool,
            &secrets,
            CreateLlmConfigInput {
                name: "DeepSeek 主力".into(),
                provider: "openai_compatible".into(),
                base_url: "https://api.deepseek.com/v1".into(),
                model: "deepseek-chat".into(),
                api_key: "sk-tmp".into(),
                network_location: NetworkLocation::External,
            },
        )
        .await
        .expect("create config");
        // 首个配置自动 default——resolve_default_provider 同样命中它
        assert!(config.is_default);

        // 模拟密钥丢失（云端迁移/重装场景：库在、SecretStore 键不在）
        secrets
            .delete_secret(&config.api_key_ref)
            .expect("delete secret");

        let expect_reentry = |err: AppError| {
            let msg = match &err {
                AppError::KeyringError(m) => m.clone(),
                other => panic!("必须为 KeyringError，实得: {other:?}"),
            };
            assert!(msg.contains("DeepSeek 主力"), "含配置名: {msg}");
            assert!(msg.contains("设置 → 模型服务配置"), "含重录路径: {msg}");
        };

        expect_reentry(
            test_connection(&pool, &secrets, config.id.clone())
                .await
                .expect_err("缺密钥的 test_connection 必须报错"),
        );
        expect_reentry(
            list_models(&pool, &secrets, config.id.clone())
                .await
                .expect_err("缺密钥的 list_models 必须报错"),
        );
        // Arc<dyn LlmProvider> 无 Debug——expect_err 不可用，改手写分支
        let err = match resolve_default_provider(&pool, &secrets).await {
            Err(e) => e,
            Ok(_) => panic!("默认配置缺密钥必须报错"),
        };
        expect_reentry(err);
    }

    /// NFR-C7 源契约（17.1 评审 #29）：6 处调用点必须共用
    /// `missing_api_key_error`（llm_config ×4 + mission_inferrer /
    /// task_classifier 各 ×1），且重录文案拼装点全库唯一（helper 内），
    /// 旧文案内联构造回潮即红——后两文件的行为级测试需重 fixture
    ///（mission 上下文），源契约先行钉住接线（同款先例：sidecar 的
    /// --pure 源断言）。只扫描生产区段（`mod tests` 之前）。
    #[test]
    fn all_missing_key_call_sites_share_single_helper() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // (文件, 期望调用数, 期望拼装点数)——拼装点唯一在 helper 内
        let files = [
            ("src/services/llm_config.rs", 4, 1),
            ("src/services/mission_inferrer.rs", 1, 0),
            ("src/services/task_classifier.rs", 1, 0),
        ];
        let mut helper_uses = 0;
        for (rel, expected_calls, expected_assembly) in &files {
            let src = std::fs::read_to_string(base.join(rel))
                .unwrap_or_else(|e| panic!("read {rel}: {e}"));
            let prod = src.split("\nmod tests").next().unwrap_or(&src);
            // 调用点形态：ok_or_else(|| missing_api_key_error(&config.name))
            //（含全限定路径形态）；排除 helper 自身定义行
            let uses = prod
                .lines()
                .filter(|l| l.contains("missing_api_key_error("))
                .filter(|l| !l.contains("fn missing_api_key_error"))
                .count();
            assert_eq!(
                uses, *expected_calls,
                "{rel} 的 missing_api_key_error 调用数 {uses} != 期望 {expected_calls}"
            );
            helper_uses += uses;
            // 拼装点唯一性：重录文案在生产区段仅 helper 内一次（测试断言
            // 自身会引用该短语——只扫 prod）；任何第二拼装点即漂移
            let assembly = prod.matches("设置 → 模型服务配置").count();
            assert_eq!(
                assembly, *expected_assembly,
                "{rel} 的重录文案拼装点 {assembly} != 期望 {expected_assembly}"
            );
        }
        assert_eq!(helper_uses, 6, "调用点总数须为 6（评审 #29 基线）");
    }
}
