use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::mission::Mission;

pub async fn get_mission(pool: &SqlitePool) -> Result<Option<Mission>, AppError> {
    sqlx::query_as::<_, Mission>("SELECT id, content, format, updated_at FROM mission WHERE id = 'singleton'")
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::DbError(format!("读取使命宣言失败: {}", e)))
}

pub async fn upsert_mission(
    pool: &SqlitePool,
    content: Option<&str>,
    format: &str,
) -> Result<Mission, AppError> {
    let now = crate::db::settings::chrono_now_pub();
    sqlx::query(
        "INSERT OR REPLACE INTO mission (id, content, format, updated_at) VALUES ('singleton', ?1, ?2, ?3)",
    )
    .bind(content)
    .bind(format)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("写入使命宣言失败: {}", e)))?;

    get_mission(pool)
        .await?
        .ok_or_else(|| AppError::DbError("使命宣言写入后读取失败".to_string()))
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
            "CREATE TABLE IF NOT EXISTS mission (
                id TEXT PRIMARY KEY NOT NULL DEFAULT 'singleton',
                content TEXT,
                format TEXT NOT NULL DEFAULT 'free' CHECK(format IN ('free', 'structured')),
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        pool
    }

    #[tokio::test]
    async fn test_get_mission_empty() {
        let pool = setup_test_db().await;
        let result = get_mission(&pool).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_upsert_and_get_roundtrip() {
        let pool = setup_test_db().await;
        let mission = upsert_mission(&pool, Some("我的使命宣言"), "free")
            .await
            .unwrap();
        assert_eq!(mission.id, "singleton");
        assert_eq!(mission.content, Some("我的使命宣言".to_string()));
        assert_eq!(mission.format, "free");

        let loaded = get_mission(&pool).await.unwrap().unwrap();
        assert_eq!(loaded.content, Some("我的使命宣言".to_string()));
        assert_eq!(loaded.format, "free");
    }

    #[tokio::test]
    async fn test_upsert_empty_content() {
        let pool = setup_test_db().await;
        let mission = upsert_mission(&pool, None, "free").await.unwrap();
        assert_eq!(mission.content, None);
        assert_eq!(mission.format, "free");
    }

    #[tokio::test]
    async fn test_upsert_overwrites_existing() {
        let pool = setup_test_db().await;
        upsert_mission(&pool, Some("旧宣言"), "free")
            .await
            .unwrap();
        let updated = upsert_mission(&pool, Some("新宣言"), "structured")
            .await
            .unwrap();
        assert_eq!(updated.content, Some("新宣言".to_string()));
        assert_eq!(updated.format, "structured");

        let loaded = get_mission(&pool).await.unwrap().unwrap();
        assert_eq!(loaded.content, Some("新宣言".to_string()));
        assert_eq!(loaded.format, "structured");
    }

    #[tokio::test]
    async fn test_upsert_structured_format() {
        let pool = setup_test_db().await;
        let json_content = r#"{"role":"丈夫","value":"诚信","goal":"事业影响力"}"#;
        let mission = upsert_mission(&pool, Some(json_content), "structured")
            .await
            .unwrap();
        assert_eq!(mission.format, "structured");
        assert_eq!(mission.content, Some(json_content.to_string()));
    }
}
