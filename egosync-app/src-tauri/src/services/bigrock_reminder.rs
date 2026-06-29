//! Story 6.3: 大石头规划提醒服务 — 检测本周是否有大石头任务，若无则发送提醒
//!
//! 职责：
//! - 查询所有 `is_big_rock = true` 的未完成任务
//! - 若无未完成的大石头 → 创建"轻触"通知 + 写入管家对话消息 + emit `bigrock:reminder` 事件
//! - 若有未完成的大石头（无论来源）→ 不触发
//! - 错误只 `tracing::warn!`，不 panic，不阻塞调度器

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::services::notification_service;
use crate::services::suggestion_generator::NotificationLevel;

/// Tauri Event 名称
pub const BIGROCK_REMINDER_EVENT: &str = "bigrock:reminder";

/// 大石头提醒事件 payload，发给前端
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigrockReminderPayload {
    pub message: String,
    pub notification_id: String,
}

/// 提醒文案
const REMINDER_MESSAGE: &str = "还没规划本周大石头，要安排一下吗？";

/// 核心函数：检测本周是否有未完成的大石头任务，若无则发送提醒。
///
/// - `pool`: 主数据库连接池（tasks / notifications）
/// - `conv_pool`: 对话数据库连接池（管家对话消息）
/// - `app_handle`: 可选，有则 emit `bigrock:reminder` 事件给前端
///
/// 返回 `Ok(true)` 表示已触发提醒，`Ok(false)` 表示跳过（已有大石头）。
/// 错误只 `tracing::warn!`，不阻塞调度器。
pub async fn check_and_remind_if_needed(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<bool, AppError> {
    // 查询所有大石头任务（list_all_tasks 已过滤 deleted_at IS NULL）
    let big_rock_tasks = db::tasks::list_all_tasks(pool, None, Some(true)).await?;

    // AC6: 过滤掉已完成的任务，检查是否有未完成的大石头
    let has_unfinished_big_rocks = big_rock_tasks.iter().any(|t| !t.is_completed);

    if has_unfinished_big_rocks {
        tracing::info!(
            total_big_rocks = big_rock_tasks.len(),
            "本周已有未完成的大石头任务，跳过提醒"
        );
        return Ok(false);
    }

    // 无未完成的大石头 → 创建通知 + 写入管家对话 + emit 事件
    let notification_id = create_notification_if_possible(pool).await;
    write_butler_message(conv_pool).await;
    emit_reminder_event(app_handle, &notification_id);

    tracing::info!(
        notification_id = %notification_id,
        "大石头规划提醒已触发"
    );

    Ok(true)
}

/// 使用第一个活跃角色的 ID 创建"轻触"通知。
/// 若没有活跃角色则跳过通知创建，返回空字符串。
async fn create_notification_if_possible(pool: &SqlitePool) -> String {
    let roles = match db::roles::list_active_roles(pool).await {
        Ok(roles) => roles,
        Err(e) => {
            tracing::warn!(error = %e, "查询活跃角色失败，跳过通知创建");
            return String::new();
        }
    };

    let Some(first_role) = roles.first() else {
        tracing::info!("无活跃角色，跳过通知创建，只写入管家对话消息");
        return String::new();
    };

    match notification_service::create_notification_for_role(
        pool,
        &first_role.id,
        NotificationLevel::Tap,
        REMINDER_MESSAGE,
    )
    .await
    {
        Ok(notification) => {
            tracing::info!(
                role_id = %first_role.id,
                notification_id = %notification.id,
                level = %notification.level,
                "大石头提醒通知已创建"
            );
            notification.id
        }
        Err(e) => {
            tracing::warn!(
                role_id = %first_role.id,
                error = %e,
                "大石头提醒通知创建失败（继续写入对话消息）"
            );
            String::new()
        }
    }
}

/// 写入管家对话消息。
async fn write_butler_message(conv_pool: &ConversationsPool) {
    match db::conversations::get_or_create_butler_conversation(conv_pool).await {
        Ok(conv) => {
            match db::conversations::insert_message(
                conv_pool,
                &conv.id,
                "assistant",
                REMINDER_MESSAGE,
                true,
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(
                        conversation_id = %conv.id,
                        "大石头提醒管家对话消息已写入"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        conversation_id = %conv.id,
                        error = %e,
                        "大石头提醒对话消息写入失败"
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "获取/创建管家对话失败");
        }
    }
}

/// emit `bigrock:reminder` 事件给前端。
fn emit_reminder_event(app_handle: Option<&AppHandle>, notification_id: &str) {
    if let Some(handle) = app_handle {
        let payload = BigrockReminderPayload {
            message: REMINDER_MESSAGE.to_string(),
            notification_id: notification_id.to_string(),
        };
        if let Err(e) = handle.emit(BIGROCK_REMINDER_EVENT, &payload) {
            tracing::warn!(error = %e, "emit bigrock:reminder 事件失败");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigrock_reminder_event_name_is_correct() {
        assert_eq!(BIGROCK_REMINDER_EVENT, "bigrock:reminder");
    }

    #[test]
    fn reminder_message_is_set() {
        assert!(!REMINDER_MESSAGE.is_empty());
        assert!(REMINDER_MESSAGE.contains("大石头"));
    }

    #[test]
    fn payload_serializes_with_camel_case() {
        let payload = BigrockReminderPayload {
            message: "test".to_string(),
            notification_id: "abc-123".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("notificationId"));
        assert!(json.contains("message"));
    }

    // ---- 集成测试 ----

    async fn setup_test_db() -> (SqlitePool, ConversationsPool) {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT '🎯',
                color TEXT NOT NULL DEFAULT '#6366F1',
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

        sqlx::query(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY NOT NULL,
                owner_type TEXT NOT NULL DEFAULT 'role',
                role_id TEXT,
                title TEXT NOT NULL,
                deadline TEXT,
                quadrant TEXT NOT NULL DEFAULT 'Q2',
                is_big_rock INTEGER NOT NULL DEFAULT 0,
                is_completed INTEGER NOT NULL DEFAULT 0,
                completed_at TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                protection_status TEXT NOT NULL DEFAULT 'normal',
                confidence REAL,
                manual_override INTEGER NOT NULL DEFAULT 0,
                classification_reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create tasks table");

        sqlx::query(
            "CREATE TABLE notifications (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT NOT NULL,
                level TEXT NOT NULL CHECK (level IN ('whisper', 'tap', 'knock')),
                content TEXT NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create notifications table");

        sqlx::query(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create app_settings table");

        let conv_pool = ConversationsPool(
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .expect("failed to create conv test db"),
        );

        sqlx::query(
            "CREATE TABLE conversations (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT,
                title TEXT NOT NULL DEFAULT '',
                started_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&*conv_pool)
        .await
        .expect("failed to create conversations table");

        sqlx::query(
            "CREATE TABLE messages (
                id TEXT PRIMARY KEY NOT NULL,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                thinking_content TEXT NOT NULL DEFAULT '',
                is_complete INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
        )
        .execute(&*conv_pool)
        .await
        .expect("failed to create messages table");

        (pool, conv_pool)
    }

    #[tokio::test]
    async fn check_and_remind_returns_false_when_big_rocks_exist() {
        let (pool, conv_pool) = setup_test_db().await;

        // 插入一个活跃角色
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 插入一个未完成的大石头任务
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, is_completed, created_at, updated_at)
             VALUES ('task-1', 'role', 'role-1', '竞品分析', 'Q1', 1, 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = check_and_remind_if_needed(&pool, &conv_pool, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn check_and_remind_returns_true_when_no_big_rocks() {
        let (pool, conv_pool) = setup_test_db().await;

        // 插入一个活跃角色（通知创建需要）
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 无任何大石头任务
        let result = check_and_remind_if_needed(&pool, &conv_pool, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // 验证管家对话消息已写入
        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant' AND content LIKE '%大石头%'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 1);
    }

    #[tokio::test]
    async fn check_and_remind_skips_notification_when_no_active_roles() {
        let (pool, conv_pool) = setup_test_db().await;

        // 无活跃角色
        let result = check_and_remind_if_needed(&pool, &conv_pool, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // 验证没有通知被创建
        let notif_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(notif_count, 0);

        // 验证管家对话消息仍已写入
        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 1);
    }

    #[tokio::test]
    async fn check_and_remind_skips_when_only_completed_big_rocks() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 插入一个已完成的大石头任务（应被过滤掉）
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, is_completed, completed_at, created_at, updated_at)
             VALUES ('task-1', 'role', 'role-1', '已完成的大石头', 'Q1', 1, 1, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = check_and_remind_if_needed(&pool, &conv_pool, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }
}
