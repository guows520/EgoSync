use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::error::AppError;

const EMERGENCE_COOLDOWN_PREFIX: &str = "emergence_cooldown:";

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, AppError> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = ?1")
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::DbError(format!("读取设置失败: {}", e)))?;

    Ok(row.and_then(|r| r.0))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), AppError> {
    let now = crate::db::settings::chrono_now_pub();
    sqlx::query("INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)")
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("写入设置失败: {}", e)))?;

    Ok(())
}

/// 记录某领域的涌现建议被用户拒绝，写入冷却时间戳。
pub async fn set_emergence_cooldown(
    pool: &SqlitePool,
    domain: &str,
    rejected_at: &str,
) -> Result<(), AppError> {
    let key = format!("{}{}", EMERGENCE_COOLDOWN_PREFIX, domain);
    set_setting(pool, &key, rejected_at).await
}

/// 批量读取所有涌现冷却记录。返回 domain → rejected_at ISO 时间戳。
pub async fn get_emergence_cooldowns(
    pool: &SqlitePool,
) -> Result<HashMap<String, String>, AppError> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT key, value FROM app_settings WHERE key LIKE ?1",
    )
    .bind(format!("{}%", EMERGENCE_COOLDOWN_PREFIX))
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("读取涌现冷却失败: {}", e)))?;

    let prefix_len = EMERGENCE_COOLDOWN_PREFIX.len();
    let mut map = HashMap::new();
    for (key, value) in rows {
        if let Some(domain) = key.get(prefix_len..) {
            if let Some(ts) = value {
                map.insert(domain.to_string(), ts);
            }
        }
    }
    Ok(map)
}

/// 清理过期的涌现冷却记录（超过 days 天的）。
pub async fn clear_expired_cooldowns(
    pool: &SqlitePool,
    days: i64,
) -> Result<u64, AppError> {
    // 计算截止时间：当前时间 - days 天
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff.to_rfc3339();

    let result = sqlx::query(
        "DELETE FROM app_settings WHERE key LIKE ?1 AND value < ?2",
    )
    .bind(format!("{}%", EMERGENCE_COOLDOWN_PREFIX))
    .bind(&cutoff_str)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("清理过期冷却失败: {}", e)))?;

    Ok(result.rows_affected())
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
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        pool
    }

    #[tokio::test]
    async fn test_get_nonexistent_setting() {
        let pool = setup_test_db().await;
        let result = get_setting(&pool, "nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_set_and_get_setting() {
        let pool = setup_test_db().await;
        set_setting(&pool, "onboarding_completed", "true")
            .await
            .unwrap();
        let result = get_setting(&pool, "onboarding_completed").await.unwrap();
        assert_eq!(result, Some("true".to_string()));
    }

    #[tokio::test]
    async fn test_set_overwrites_existing() {
        let pool = setup_test_db().await;
        set_setting(&pool, "key1", "value1").await.unwrap();
        set_setting(&pool, "key1", "value2").await.unwrap();
        let result = get_setting(&pool, "key1").await.unwrap();
        assert_eq!(result, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_emergence_cooldown_set_and_get() {
        let pool = setup_test_db().await;
        set_emergence_cooldown(&pool, "健身/运动", "2026-05-20T10:00:00Z")
            .await
            .unwrap();
        set_emergence_cooldown(&pool, "摄影", "2026-05-21T10:00:00Z")
            .await
            .unwrap();
        let cooldowns = get_emergence_cooldowns(&pool).await.unwrap();
        assert_eq!(cooldowns.len(), 2);
        assert_eq!(
            cooldowns.get("健身/运动"),
            Some(&"2026-05-20T10:00:00Z".to_string())
        );
        assert_eq!(
            cooldowns.get("摄影"),
            Some(&"2026-05-21T10:00:00Z".to_string())
        );
    }

    #[tokio::test]
    async fn test_emergence_cooldown_empty_when_none() {
        let pool = setup_test_db().await;
        let cooldowns = get_emergence_cooldowns(&pool).await.unwrap();
        assert!(cooldowns.is_empty());
    }

    #[tokio::test]
    async fn test_clear_expired_cooldowns() {
        let pool = setup_test_db().await;
        // 写入一条过期记录（10 天前）和一条未过期记录（1 天前）
        let old = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        let recent = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        set_emergence_cooldown(&pool, "旧领域", &old).await.unwrap();
        set_emergence_cooldown(&pool, "新领域", &recent).await.unwrap();

        let deleted = clear_expired_cooldowns(&pool, 7).await.unwrap();
        assert_eq!(deleted, 1);

        let remaining = get_emergence_cooldowns(&pool).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(remaining.contains_key("新领域"));
        assert!(!remaining.contains_key("旧领域"));
    }
}
