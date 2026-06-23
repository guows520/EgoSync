use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::q2_reminder::Q2Reminder;

/// 插入或更新提醒记录。首次调用 INSERT，后续调用 ON CONFLICT 递增 `reminded_count` 并刷新 `last_reminded_at`。
pub async fn upsert_reminder(pool: &SqlitePool, task_id: &str) -> Result<Q2Reminder, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO q2_reminders (id, task_id, reminded_count, last_reminded_at)
         VALUES (?1, ?2, 1, ?3)
         ON CONFLICT(task_id) DO UPDATE SET
             reminded_count = reminded_count + 1,
             last_reminded_at = ?3",
    )
    .bind(&id)
    .bind(task_id)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("写入 Q2 提醒记录失败: {}", e)))?;

    get_reminder_for_task(pool, task_id)
        .await?
        .ok_or_else(|| AppError::DbError("Q2 提醒记录写入后未找到".to_string()))
}

/// 查询某任务的提醒记录，不存在返回 None。
pub async fn get_reminder_for_task(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<Option<Q2Reminder>, AppError> {
    sqlx::query_as::<_, Q2Reminder>(
        "SELECT id, task_id, reminded_count, last_reminded_at, created_at
         FROM q2_reminders
         WHERE task_id = ?1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Q2 提醒记录失败: {}", e)))
}

/// 删除某任务的提醒记录（用户处理任务后调用），返回受影响行数。
pub async fn delete_reminder_for_task(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM q2_reminders WHERE task_id = ?1")
        .bind(task_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除 Q2 提醒记录失败: {}", e)))?;

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
            "CREATE TABLE q2_reminders (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                reminded_count INTEGER NOT NULL DEFAULT 1,
                last_reminded_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                UNIQUE(task_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create q2_reminders table");

        pool
    }

    #[tokio::test]
    async fn upsert_reminder_creates_new_record() {
        let pool = setup_test_db().await;

        let reminder = upsert_reminder(&pool, "task-1")
            .await
            .expect("upsert new");

        assert_eq!(reminder.task_id, "task-1");
        assert_eq!(reminder.reminded_count, 1);
        assert!(!reminder.last_reminded_at.is_empty());
    }

    #[tokio::test]
    async fn upsert_reminder_increments_count_on_conflict() {
        let pool = setup_test_db().await;

        let first = upsert_reminder(&pool, "task-1").await.expect("first upsert");
        assert_eq!(first.reminded_count, 1);

        let second = upsert_reminder(&pool, "task-1").await.expect("second upsert");
        assert_eq!(second.reminded_count, 2);
        assert_eq!(second.id, first.id, "同一 task_id 应保持同一 id");
    }

    #[tokio::test]
    async fn get_reminder_for_task_returns_none_when_absent() {
        let pool = setup_test_db().await;

        let result = get_reminder_for_task(&pool, "nonexistent")
            .await
            .expect("get absent");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_reminder_for_task_returns_some_when_present() {
        let pool = setup_test_db().await;

        upsert_reminder(&pool, "task-1").await.expect("upsert");

        let result = get_reminder_for_task(&pool, "task-1")
            .await
            .expect("get present");

        assert!(result.is_some());
        assert_eq!(result.unwrap().task_id, "task-1");
    }

    #[tokio::test]
    async fn delete_reminder_for_task_removes_record() {
        let pool = setup_test_db().await;

        upsert_reminder(&pool, "task-1").await.expect("upsert");

        let deleted = delete_reminder_for_task(&pool, "task-1")
            .await
            .expect("delete");

        assert_eq!(deleted, 1);

        let result = get_reminder_for_task(&pool, "task-1")
            .await
            .expect("get after delete");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_reminder_for_task_returns_zero_when_absent() {
        let pool = setup_test_db().await;

        let deleted = delete_reminder_for_task(&pool, "nonexistent")
            .await
            .expect("delete absent");

        assert_eq!(deleted, 0);
    }
}
