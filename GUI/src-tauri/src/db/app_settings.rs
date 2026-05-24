use sqlx::SqlitePool;

use crate::error::AppError;

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
}
