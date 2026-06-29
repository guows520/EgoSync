use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::briefing::Briefing;

pub async fn create_briefing(
    pool: &SqlitePool,
    content: &str,
    date: &str,
) -> Result<Briefing, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO briefings (id, content, date, created_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&id)
    .bind(content)
    .bind(date)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建简报失败: {}", e)))?;

    Ok(Briefing {
        id,
        content: content.to_string(),
        date: date.to_string(),
        created_at: now,
    })
}

pub async fn get_briefing_by_date(
    pool: &SqlitePool,
    date: &str,
) -> Result<Option<Briefing>, AppError> {
    sqlx::query_as::<_, Briefing>(
        "SELECT id, content, date, created_at FROM briefings WHERE date = ?1",
    )
    .bind(date)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("按日期查询简报失败: {}", e)))
}

pub async fn list_all_briefings(pool: &SqlitePool) -> Result<Vec<Briefing>, AppError> {
    sqlx::query_as::<_, Briefing>(
        "SELECT id, content, date, created_at FROM briefings ORDER BY date ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询全部简报失败: {}", e)))
}

pub async fn get_latest_briefing(pool: &SqlitePool) -> Result<Option<Briefing>, AppError> {
    sqlx::query_as::<_, Briefing>(
        "SELECT id, content, date, created_at FROM briefings ORDER BY date DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询最新简报失败: {}", e)))
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
            "CREATE TABLE briefings (
                id TEXT PRIMARY KEY NOT NULL,
                content TEXT NOT NULL,
                date TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        pool
    }

    #[tokio::test]
    async fn create_briefing_inserts_row() {
        let pool = setup_test_db().await;
        let briefing = create_briefing(&pool, "早上好！", "2026-06-25")
            .await
            .unwrap();
        assert_eq!(briefing.content, "早上好！");
        assert_eq!(briefing.date, "2026-06-25");
        assert!(!briefing.id.is_empty());
    }

    #[tokio::test]
    async fn get_briefing_by_date_returns_existing() {
        let pool = setup_test_db().await;
        create_briefing(&pool, "简报内容", "2026-06-25")
            .await
            .unwrap();

        let result = get_briefing_by_date(&pool, "2026-06-25")
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "简报内容");
    }

    #[tokio::test]
    async fn get_briefing_by_date_returns_none_when_not_found() {
        let pool = setup_test_db().await;
        let result = get_briefing_by_date(&pool, "2026-06-25")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_latest_briefing_returns_most_recent() {
        let pool = setup_test_db().await;
        create_briefing(&pool, "第一天的简报", "2026-06-24")
            .await
            .unwrap();
        create_briefing(&pool, "第二天的简报", "2026-06-25")
            .await
            .unwrap();

        let result = get_latest_briefing(&pool).await.unwrap();
        assert!(result.is_some());
        let latest = result.unwrap();
        assert_eq!(latest.date, "2026-06-25");
        assert_eq!(latest.content, "第二天的简报");
    }

    #[tokio::test]
    async fn get_latest_briefing_returns_none_when_empty() {
        let pool = setup_test_db().await;
        let result = get_latest_briefing(&pool).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn create_briefing_duplicate_date_fails() {
        let pool = setup_test_db().await;
        create_briefing(&pool, "第一次", "2026-06-25")
            .await
            .unwrap();
        let result = create_briefing(&pool, "第二次", "2026-06-25").await;
        assert!(result.is_err());
    }
}
