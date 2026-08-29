use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, State};

use crate::db;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::notification::{
    CreateNotificationInput, Notification, NotificationNewPayload, NotificationWithRole,
};
use crate::services::notification_service;

/// 创建通知（含降级逻辑）并构造 `notification:new` 事件 payload。
///
/// 抽出为不依赖 `AppHandle` 的纯函数，便于单测；emit 由命令层负责。
async fn create_and_build_payload(
    pool: &SqlitePool,
    input: &CreateNotificationInput,
) -> Result<(Notification, NotificationNewPayload), AppError> {
    let requested_level = notification_service::parse_level(&input.level);

    let notification = notification_service::create_notification_for_role(
        pool,
        &input.role_id,
        requested_level,
        &input.content,
    )
    .await?;

    // 查询角色信息以构造事件 payload
    let role = db::roles::get_role(pool, &input.role_id).await?;

    let payload = NotificationNewPayload {
        id: notification.id.clone(),
        level: notification.level.clone(),
        content: notification.content.clone(),
        role_id: role.id.clone(),
        role_name: role.name.clone(),
        role_icon: role.icon.clone(),
        role_color: role.color.clone(),
        created_at: notification.created_at.clone(),
    };

    Ok((notification, payload))
}

/// 创建通知 — 调用 service 层（含降级逻辑），写入 DB 后 emit `notification:new` 事件。
#[tauri::command]
pub async fn notification_create(
    app: AppHandle,
    pool: State<'_, DbPool>,
    input: CreateNotificationInput,
) -> Result<Notification, AppError> {
    let (notification, payload) = create_and_build_payload(&pool, &input).await?;

    let _ = app
        .emit("notification:new", &payload)
        .map_err(|e| AppError::DbError(format!("发送通知事件失败: {}", e)));

    Ok(notification)
}

/// 列出所有通知（带角色信息），按创建时间倒序。
#[tauri::command]
pub async fn notification_list(
    pool: State<'_, DbPool>,
) -> Result<Vec<NotificationWithRole>, AppError> {
    db::notifications::list_notifications(&pool).await
}

/// 标记通知为已读。
#[tauri::command]
pub async fn notification_mark_read(
    app_handle: AppHandle,
    pool: State<'_, DbPool>,
    id: String,
) -> Result<Notification, AppError> {
    // app_handle 由 Tauri 自动注入（前端 invoke 不变）——Story 13.1
    // 评审决策①：已读须触发 STATE_DELTA，否则手机未读角标永远滞后。
    let notification = db::notifications::mark_read(&pool, &id).await?;
    if let Err(e) = app_handle.emit("notification:read", &notification) {
        tracing::warn!(event = "notification:read", error = %e, "notification 写事件发射失败");
    }
    Ok(notification)
}

/// 统计未读通知数量。
#[tauri::command]
pub async fn notification_count_unread(pool: State<'_, DbPool>) -> Result<i64, AppError> {
    db::notifications::count_unread(&pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db(proactivity_level: &str) -> SqlitePool {
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

        sqlx::query("INSERT INTO roles (id, name, icon, color, proactivity_level) VALUES ('role-a', '产品', '🎯', '#6366F1', ?1)")
            .bind(proactivity_level)
            .execute(&pool)
            .await
            .expect("failed to insert role-a");

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

    #[tokio::test]
    async fn create_and_build_payload_returns_notification_and_matching_payload() {
        let pool = setup_test_db("proactive").await;
        let input = CreateNotificationInput {
            role_id: "role-a".to_string(),
            level: "knock".to_string(),
            content: "紧急提醒".to_string(),
        };

        let (notification, payload) = create_and_build_payload(&pool, &input)
            .await
            .expect("create");

        assert_eq!(notification.level, "knock", "proactive 角色允许 knock");
        assert_eq!(payload.id, notification.id);
        assert_eq!(payload.level, notification.level);
        assert_eq!(payload.content, "紧急提醒");
        assert_eq!(payload.role_id, "role-a");
        assert_eq!(payload.role_name, "产品");
        assert_eq!(payload.role_icon, "🎯");
        assert_eq!(payload.role_color, "#6366F1");
        assert_eq!(payload.created_at, notification.created_at);
    }

    #[tokio::test]
    async fn create_and_build_payload_applies_proactivity_degradation() {
        let pool = setup_test_db("moderate").await;
        let input = CreateNotificationInput {
            role_id: "role-a".to_string(),
            level: "knock".to_string(),
            content: "测试".to_string(),
        };

        let (notification, payload) = create_and_build_payload(&pool, &input)
            .await
            .expect("create");

        assert_eq!(notification.level, "tap", "moderate 角色的 knock 应降级为 tap");
        assert_eq!(payload.level, "tap", "payload 级别应与降级后一致");
    }

    #[tokio::test]
    async fn create_and_build_payload_nonexistent_role_returns_error() {
        let pool = setup_test_db("proactive").await;
        let input = CreateNotificationInput {
            role_id: "missing".to_string(),
            level: "tap".to_string(),
            content: "测试".to_string(),
        };

        let result = create_and_build_payload(&pool, &input).await;
        assert!(result.is_err());
    }
}
