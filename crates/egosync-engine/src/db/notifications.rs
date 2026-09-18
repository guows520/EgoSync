use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::notification::{CreateNotificationInput, Notification, NotificationWithRole};

/// 创建通知 — 纯 DB 插入，降级逻辑由 service 层处理。
pub async fn create_notification(
    pool: &SqlitePool,
    input: &CreateNotificationInput,
) -> Result<Notification, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO notifications (id, role_id, level, content, is_read, created_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5)",
    )
    .bind(&id)
    .bind(&input.role_id)
    .bind(&input.level)
    .bind(&input.content)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建通知失败: {}", e)))?;

    get_notification(pool, &id).await
}

/// 获取单条通知。
pub async fn get_notification(pool: &SqlitePool, id: &str) -> Result<Notification, AppError> {
    sqlx::query_as::<_, Notification>(
        "SELECT id, role_id, level, content, is_read, created_at
         FROM notifications WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询通知失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("通知 {} 不存在", id)))
}

/// 列出所有通知（带角色信息），按创建时间倒序。
pub async fn list_notifications(pool: &SqlitePool) -> Result<Vec<NotificationWithRole>, AppError> {
    sqlx::query_as::<_, NotificationWithRole>(
        "SELECT n.id, n.role_id, n.level, n.content, n.is_read, n.created_at,
                r.name AS role_name, r.icon AS role_icon, r.color AS role_color
         FROM notifications n
         INNER JOIN roles r ON n.role_id = r.id
         ORDER BY n.created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询通知列表失败: {}", e)))
}

/// 标记通知为已读。若通知不存在或已读则返回 NotFound。
pub async fn mark_read(pool: &SqlitePool, id: &str) -> Result<Notification, AppError> {
    let notification = get_notification(pool, id).await?;
    if notification.is_read {
        return Err(AppError::NotFound(format!("通知 {} 已读", id)));
    }

    sqlx::query("UPDATE notifications SET is_read = 1 WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("标记已读失败: {}", e)))?;

    get_notification(pool, id).await
}

/// 统计未读通知数量。
pub async fn count_unread(pool: &SqlitePool) -> Result<i64, AppError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE is_read = 0")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("统计未读通知失败: {}", e)))?;
    Ok(count)
}

/// 统计当天（本地日期）已创建的 knock 级通知数量。
///
/// `created_at` 以 UTC 存储，比较时统一转为本地日期，使每日上限按用户本地零点重置。
pub async fn count_knock_today(pool: &SqlitePool) -> Result<i64, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications
         WHERE level = 'knock' AND date(created_at, 'localtime') = date('now', 'localtime')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("统计当日敲门通知失败: {}", e)))?;
    Ok(count)
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
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT 'target',
                color TEXT NOT NULL DEFAULT '#4F46E5',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
                energy_updated_at TEXT,
                skills_config TEXT NOT NULL DEFAULT '{}',
                proactivity_level TEXT NOT NULL DEFAULT 'moderate',
                archived_at TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create roles table");

        sqlx::query("INSERT INTO roles (id, name, icon, color) VALUES ('role-a', '产品', '🎯', '#6366F1')")
            .execute(&pool)
            .await
            .expect("failed to insert role-a");
        sqlx::query("INSERT INTO roles (id, name, icon, color) VALUES ('role-b', '学习', '📚', '#10B981')")
            .execute(&pool)
            .await
            .expect("failed to insert role-b");

        sqlx::query(
            "CREATE TABLE notifications (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT NOT NULL,
                level TEXT NOT NULL CHECK (level IN ('whisper', 'tap', 'knock')),
                content TEXT NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create notifications table");

        pool
    }

    async fn insert_notification(
        pool: &SqlitePool,
        id: &str,
        role_id: &str,
        level: &str,
        content: &str,
        is_read: bool,
        created_at: &str,
    ) {
        sqlx::query(
            "INSERT INTO notifications (id, role_id, level, content, is_read, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(id)
        .bind(role_id)
        .bind(level)
        .bind(content)
        .bind(is_read)
        .bind(created_at)
        .execute(pool)
        .await
        .expect("failed to insert notification");
    }

    #[tokio::test]
    async fn create_notification_inserts_and_returns() {
        let pool = setup_test_db().await;
        let input = CreateNotificationInput {
            role_id: "role-a".to_string(),
            level: "knock".to_string(),
            content: "紧急提醒".to_string(),
        };
        let n = create_notification(&pool, &input).await.expect("create");
        assert_eq!(n.role_id, "role-a");
        assert_eq!(n.level, "knock");
        assert_eq!(n.content, "紧急提醒");
        assert!(!n.is_read);
    }

    #[tokio::test]
    async fn create_notification_invalid_level_fails() {
        let pool = setup_test_db().await;
        let input = CreateNotificationInput {
            role_id: "role-a".to_string(),
            level: "invalid".to_string(),
            content: "test".to_string(),
        };
        let result = create_notification(&pool, &input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_notifications_returns_with_role_info_desc() {
        let pool = setup_test_db().await;
        insert_notification(&pool, "n1", "role-a", "whisper", "耳语", false, "2026-06-20T10:00:00Z").await;
        insert_notification(&pool, "n2", "role-b", "knock", "敲门", false, "2026-06-21T10:00:00Z").await;

        let list = list_notifications(&pool).await.expect("list");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "n2");
        assert_eq!(list[0].role_name, "学习");
        assert_eq!(list[0].role_icon, "📚");
        assert_eq!(list[0].role_color, "#10B981");
        assert_eq!(list[1].id, "n1");
        assert_eq!(list[1].role_name, "产品");
    }

    #[tokio::test]
    async fn list_notifications_empty_when_none() {
        let pool = setup_test_db().await;
        let list = list_notifications(&pool).await.expect("list");
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn mark_read_updates_is_read() {
        let pool = setup_test_db().await;
        insert_notification(&pool, "n1", "role-a", "tap", "轻触", false, "2026-06-20T10:00:00Z").await;

        let updated = mark_read(&pool, "n1").await.expect("mark read");
        assert!(updated.is_read);
    }

    #[tokio::test]
    async fn mark_read_already_read_returns_not_found() {
        let pool = setup_test_db().await;
        insert_notification(&pool, "n1", "role-a", "tap", "轻触", true, "2026-06-20T10:00:00Z").await;

        let result = mark_read(&pool, "n1").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn mark_read_nonexistent_returns_not_found() {
        let pool = setup_test_db().await;
        let result = mark_read(&pool, "missing").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn count_unread_returns_correct_count() {
        let pool = setup_test_db().await;
        insert_notification(&pool, "n1", "role-a", "whisper", "a", false, "2026-06-20T10:00:00Z").await;
        insert_notification(&pool, "n2", "role-a", "tap", "b", false, "2026-06-20T11:00:00Z").await;
        insert_notification(&pool, "n3", "role-a", "knock", "c", true, "2026-06-20T12:00:00Z").await;

        let count = count_unread(&pool).await.expect("count unread");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn count_unread_zero_when_all_read() {
        let pool = setup_test_db().await;
        insert_notification(&pool, "n1", "role-a", "whisper", "a", true, "2026-06-20T10:00:00Z").await;

        let count = count_unread(&pool).await.expect("count unread");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn count_unread_zero_when_empty() {
        let pool = setup_test_db().await;
        let count = count_unread(&pool).await.expect("count unread");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn count_knock_today_counts_only_today_knocks() {
        let pool = setup_test_db().await;
        let today = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        insert_notification(&pool, "n1", "role-a", "knock", "a", false, &today).await;
        insert_notification(&pool, "n2", "role-a", "knock", "b", false, &today).await;
        insert_notification(&pool, "n3", "role-a", "knock", "c", false, &yesterday).await;
        insert_notification(&pool, "n4", "role-a", "tap", "d", false, &today).await;

        let count = count_knock_today(&pool).await.expect("count knock today");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn count_knock_today_zero_when_none() {
        let pool = setup_test_db().await;
        let count = count_knock_today(&pool).await.expect("count knock today");
        assert_eq!(count, 0);
    }
}
