use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::settings::LlmConfig;

pub async fn list_llm_configs(pool: &SqlitePool) -> Result<Vec<LlmConfig>, AppError> {
    let configs = sqlx::query_as::<_, LlmConfig>(
        "SELECT id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at FROM llm_configs ORDER BY created_at ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 LLM 配置失败: {}", e)))?;

    Ok(configs)
}

pub async fn get_llm_config(pool: &SqlitePool, id: &str) -> Result<LlmConfig, AppError> {
    sqlx::query_as::<_, LlmConfig>(
        "SELECT id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at FROM llm_configs WHERE id = ?1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 LLM 配置失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("LLM 配置 {} 不存在", id)))
}

pub async fn insert_llm_config(pool: &SqlitePool, config: &LlmConfig) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    )
    .bind(&config.id)
    .bind(&config.name)
    .bind(&config.provider)
    .bind(&config.base_url)
    .bind(&config.model)
    .bind(&config.api_key_ref)
    .bind(config.is_default)
    .bind(&config.created_at)
    .bind(&config.updated_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("插入 LLM 配置失败: {}", e)))?;

    Ok(())
}

pub async fn update_llm_config(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    provider: &str,
    base_url: &str,
    model: &str,
    api_key_ref: &str,
) -> Result<(), AppError> {
    let now = chrono_now();
    let result = sqlx::query(
        "UPDATE llm_configs SET name = ?1, provider = ?2, base_url = ?3, model = ?4, api_key_ref = ?5, updated_at = ?6 WHERE id = ?7"
    )
    .bind(name)
    .bind(provider)
    .bind(base_url)
    .bind(model)
    .bind(api_key_ref)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新 LLM 配置失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("LLM 配置 {} 不存在", id)));
    }
    Ok(())
}

pub async fn delete_llm_config(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM llm_configs WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除 LLM 配置失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("LLM 配置 {} 不存在", id)));
    }
    Ok(())
}

pub async fn set_default_llm_config(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启事务失败: {}", e)))?;

    let now = chrono_now();
    sqlx::query("UPDATE llm_configs SET is_default = 0, updated_at = ?1")
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("重置默认配置失败: {}", e)))?;

    let result =
        sqlx::query("UPDATE llm_configs SET is_default = 1, updated_at = ?1 WHERE id = ?2")
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DbError(format!("设置默认配置失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("LLM 配置 {} 不存在", id)));
    }

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交事务失败: {}", e)))?;

    Ok(())
}

pub async fn get_default_llm_config(pool: &SqlitePool) -> Result<LlmConfig, AppError> {
    sqlx::query_as::<_, LlmConfig>(
        "SELECT id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at FROM llm_configs WHERE is_default = 1 LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询默认 LLM 配置失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound("未配置默认 LLM，请在设置中添加并设为默认".to_string()))
}

pub async fn count_llm_configs(pool: &SqlitePool) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM llm_configs")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("统计 LLM 配置失败: {}", e)))?;
    Ok(row.0)
}

pub(crate) fn chrono_now_pub() -> String {
    chrono_now()
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs = now;
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
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS llm_configs (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                base_url TEXT NOT NULL,
                model TEXT NOT NULL,
                api_key_ref TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        pool
    }

    fn test_config(id: &str, name: &str, is_default: bool) -> LlmConfig {
        LlmConfig {
            id: id.to_string(),
            name: name.to_string(),
            provider: "openai_compatible".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            api_key_ref: format!("llm_{}_api_key", id),
            is_default,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn test_insert_and_list() {
        let pool = setup_test_db().await;
        let config = test_config("test-1", "Test Config", false);

        insert_llm_config(&pool, &config).await.unwrap();
        let configs = list_llm_configs(&pool).await.unwrap();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "Test Config");
    }

    #[tokio::test]
    async fn test_get_config() {
        let pool = setup_test_db().await;
        let config = test_config("test-2", "Another Config", false);

        insert_llm_config(&pool, &config).await.unwrap();
        let loaded = get_llm_config(&pool, "test-2").await.unwrap();

        assert_eq!(loaded.id, "test-2");
        assert_eq!(loaded.model, "gpt-4o");
    }

    #[tokio::test]
    async fn test_get_nonexistent_returns_not_found() {
        let pool = setup_test_db().await;
        let result = get_llm_config(&pool, "nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_config() {
        let pool = setup_test_db().await;
        let config = test_config("test-3", "Original", false);
        insert_llm_config(&pool, &config).await.unwrap();

        update_llm_config(
            &pool,
            "test-3",
            "Updated",
            "anthropic",
            "https://api.anthropic.com",
            "claude-3",
            "llm_test-3_api_key",
        )
        .await
        .unwrap();

        let loaded = get_llm_config(&pool, "test-3").await.unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.provider, "anthropic");
    }

    #[tokio::test]
    async fn test_delete_config() {
        let pool = setup_test_db().await;
        let config = test_config("test-4", "ToDelete", false);
        insert_llm_config(&pool, &config).await.unwrap();

        delete_llm_config(&pool, "test-4").await.unwrap();
        let result = get_llm_config(&pool, "test-4").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_not_found() {
        let pool = setup_test_db().await;
        let result = delete_llm_config(&pool, "ghost").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_set_default() {
        let pool = setup_test_db().await;
        let c1 = test_config("d1", "First", true);
        let c2 = test_config("d2", "Second", false);
        insert_llm_config(&pool, &c1).await.unwrap();
        insert_llm_config(&pool, &c2).await.unwrap();

        set_default_llm_config(&pool, "d2").await.unwrap();

        let configs = list_llm_configs(&pool).await.unwrap();
        let first = configs.iter().find(|c| c.id == "d1").unwrap();
        let second = configs.iter().find(|c| c.id == "d2").unwrap();
        assert!(!first.is_default);
        assert!(second.is_default);
    }

    #[tokio::test]
    async fn test_count() {
        let pool = setup_test_db().await;
        assert_eq!(count_llm_configs(&pool).await.unwrap(), 0);

        insert_llm_config(&pool, &test_config("c1", "A", false))
            .await
            .unwrap();
        insert_llm_config(&pool, &test_config("c2", "B", false))
            .await
            .unwrap();
        assert_eq!(count_llm_configs(&pool).await.unwrap(), 2);
    }
}
