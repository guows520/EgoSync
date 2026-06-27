use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::weekly_review::WeeklyReview;

pub async fn create_weekly_review(
    pool: &SqlitePool,
    week_start: &str,
    week_end: &str,
    summary: &str,
    energy_trends: &str,
    bigrock_status: &str,
    new_memories_count: i64,
) -> Result<WeeklyReview, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO weekly_reviews (id, week_start, week_end, summary, energy_trends, bigrock_status, new_memories_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&id)
    .bind(week_start)
    .bind(week_end)
    .bind(summary)
    .bind(energy_trends)
    .bind(bigrock_status)
    .bind(new_memories_count)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建周复盘失败: {}", e)))?;

    Ok(WeeklyReview {
        id,
        week_start: week_start.to_string(),
        week_end: week_end.to_string(),
        summary: summary.to_string(),
        energy_trends: energy_trends.to_string(),
        bigrock_status: bigrock_status.to_string(),
        new_memories_count,
        created_at: now,
    })
}

pub async fn get_weekly_review_by_week_start(
    pool: &SqlitePool,
    week_start: &str,
) -> Result<Option<WeeklyReview>, AppError> {
    sqlx::query_as::<_, WeeklyReview>(
        "SELECT id, week_start, week_end, summary, energy_trends, bigrock_status, new_memories_count, created_at
         FROM weekly_reviews WHERE week_start = ?1",
    )
    .bind(week_start)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("按 week_start 查询周复盘失败: {}", e)))
}

pub async fn get_latest_weekly_review(pool: &SqlitePool) -> Result<Option<WeeklyReview>, AppError> {
    sqlx::query_as::<_, WeeklyReview>(
        "SELECT id, week_start, week_end, summary, energy_trends, bigrock_status, new_memories_count, created_at
         FROM weekly_reviews ORDER BY week_start DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询最新周复盘失败: {}", e)))
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
            "CREATE TABLE weekly_reviews (
                id TEXT PRIMARY KEY NOT NULL,
                week_start TEXT NOT NULL UNIQUE,
                week_end TEXT NOT NULL,
                summary TEXT NOT NULL,
                energy_trends TEXT NOT NULL DEFAULT '{}',
                bigrock_status TEXT NOT NULL DEFAULT '{}',
                new_memories_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        pool
    }

    #[tokio::test]
    async fn create_weekly_review_inserts_row() {
        let pool = setup_test_db().await;
        let review = create_weekly_review(
            &pool,
            "2026-06-22",
            "2026-06-28",
            "本周完成了很多事",
            "{}",
            "{}",
            5,
        )
        .await
        .unwrap();
        assert_eq!(review.week_start, "2026-06-22");
        assert_eq!(review.week_end, "2026-06-28");
        assert_eq!(review.summary, "本周完成了很多事");
        assert_eq!(review.new_memories_count, 5);
        assert!(!review.id.is_empty());
    }

    #[tokio::test]
    async fn get_weekly_review_by_week_start_returns_existing() {
        let pool = setup_test_db().await;
        create_weekly_review(&pool, "2026-06-22", "2026-06-28", "复盘", "{}", "{}", 3)
            .await
            .unwrap();

        let result = get_weekly_review_by_week_start(&pool, "2026-06-22")
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().summary, "复盘");
    }

    #[tokio::test]
    async fn get_weekly_review_by_week_start_returns_none_when_not_found() {
        let pool = setup_test_db().await;
        let result = get_weekly_review_by_week_start(&pool, "2026-06-22")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_latest_weekly_review_returns_most_recent() {
        let pool = setup_test_db().await;
        create_weekly_review(&pool, "2026-06-15", "2026-06-21", "第一周", "{}", "{}", 2)
            .await
            .unwrap();
        create_weekly_review(&pool, "2026-06-22", "2026-06-28", "第二周", "{}", "{}", 4)
            .await
            .unwrap();

        let result = get_latest_weekly_review(&pool).await.unwrap();
        assert!(result.is_some());
        let latest = result.unwrap();
        assert_eq!(latest.week_start, "2026-06-22");
        assert_eq!(latest.summary, "第二周");
    }

    #[tokio::test]
    async fn get_latest_weekly_review_returns_none_when_empty() {
        let pool = setup_test_db().await;
        let result = get_latest_weekly_review(&pool).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn create_weekly_review_duplicate_week_start_fails() {
        let pool = setup_test_db().await;
        create_weekly_review(&pool, "2026-06-22", "2026-06-28", "第一次", "{}", "{}", 1)
            .await
            .unwrap();
        let result = create_weekly_review(&pool, "2026-06-22", "2026-06-28", "第二次", "{}", "{}", 2)
            .await;
        assert!(result.is_err());
    }
}
