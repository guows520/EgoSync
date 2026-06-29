//! Story 6.6: 大石头保护提醒服务 — 检测本周无进展的大石头任务并生成提醒
//!
//! 职责：
//! - 查询所有 `is_big_rock = true` 的未完成任务
//! - 过滤本周无进展（`updated_at` 早于本周一）的任务
//! - 工作日逐任务生成"轻触"级通知 + 写入管家对话消息 + emit `bigrock:protection` 事件
//! - 周五生成汇总提醒（"还有 N 个未完成"）
//! - 频率控制：每个大石头每日最多 1 次（DB 记录控制），已完成或当日有进展不提醒
//! - 错误只 `tracing::warn!`，不 panic，不阻塞调度器

use chrono::{Datelike, Local};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::models::task::CrossRoleTask;
use crate::services::notification_service;
use crate::services::suggestion_generator::NotificationLevel;

/// Tauri Event 名称
pub const BIGROCK_PROTECTION_EVENT: &str = "bigrock:protection";

/// 大石头保护提醒事件 payload，发给前端
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigRockProtectionPayload {
    pub task_id: String,
    pub task_title: String,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub message: String,
    pub notification_id: String,
}

/// 判断 `last_reminded_at`（ISO 8601 UTC）的本地日期是否与今天相同。
fn is_reminded_today(last_reminded_at: &str) -> bool {
    let now_local = Local::now().date_naive();
    match parse_iso_to_local_date(last_reminded_at) {
        Some(date) => date == now_local,
        None => false,
    }
}

/// 将 ISO 8601 UTC 字符串（如 `2026-06-22T12:30:00Z`）解析为本地日期。
fn parse_iso_to_local_date(s: &str) -> Option<chrono::NaiveDate> {
    let utc = chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
                .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
        })
        .ok()?;
    Some(utc.with_timezone(&chrono::Local).date_naive())
}

/// 计算本周一日期（本地时间）。周一 = `num_days_from_monday()` 为 0，周日为 6。
fn compute_this_week_monday() -> chrono::NaiveDate {
    let today = Local::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday() as i64;
    today - chrono::Duration::days(days_since_monday)
}

/// 判断 `updated_at`（ISO 8601 UTC）的本地日期是否在本周（本周一及之后）。
fn is_updated_this_week(updated_at: &str) -> bool {
    let monday = compute_this_week_monday();
    match parse_iso_to_local_date(updated_at) {
        Some(date) => date >= monday,
        None => false,
    }
}

/// 判断当前是否为工作日（周一~周五）。
fn is_weekday() -> bool {
    let weekday = Local::now().weekday();
    weekday.num_days_from_monday() < 5
}

/// 判断当前是否为周五。
fn is_friday() -> bool {
    Local::now().weekday().num_days_from_monday() == 4
}

/// 核心函数：检查本周无进展的大石头任务并生成保护提醒（AC1/AC3）。
///
/// - `pool`: 主数据库连接池（tasks / notifications / big_rock_protection_reminders）
/// - `conv_pool`: 对话数据库连接池（管家对话消息）
/// - `app_handle`: 可选，有则 emit `bigrock:protection` 事件给前端
///
/// 错误只 `tracing::warn!`，不阻塞其他任务的提醒生成。
pub async fn check_and_generate_protection_reminders(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<(), AppError> {
    // 非工作日跳过（AC1："工作日调度循环中检测"）
    if !is_weekday() {
        return Ok(());
    }

    check_and_generate_protection_reminders_inner(pool, conv_pool, app_handle).await
}

/// 核心逻辑（不含工作日守卫）— 供测试直接调用，避免日期依赖。
async fn check_and_generate_protection_reminders_inner(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<(), AppError> {
    let big_rock_tasks = db::tasks::list_all_tasks(pool, None, Some(true)).await?;

    // 过滤：未完成且本周无进展
    let stale_tasks: Vec<&CrossRoleTask> = big_rock_tasks
        .iter()
        .filter(|t| !t.is_completed && !is_updated_this_week(&t.updated_at))
        .collect();

    if stale_tasks.is_empty() {
        return Ok(());
    }

    for task in &stale_tasks {
        if let Err(e) = process_single_bigrock(pool, conv_pool, app_handle, task).await {
            tracing::warn!(
                task_id = %task.id,
                task_title = %task.title,
                error = %e,
                "大石头保护提醒生成失败（跳过该任务，继续处理其他任务）"
            );
        }
    }

    Ok(())
}

/// 处理单个大石头任务的保护提醒（AC3）。
async fn process_single_bigrock(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
    task: &CrossRoleTask,
) -> Result<(), AppError> {
    // a. 查询已有提醒记录
    let existing = db::big_rock_protection_reminders::get_reminder_for_task(pool, &task.id).await?;

    // b. 今日已提醒 → 跳过
    if let Some(ref reminder) = existing {
        if is_reminded_today(&reminder.last_reminded_at) {
            return Ok(());
        }
    }

    // c. 生成提醒文案（AC3 "当日有进展不提醒" 已由上游 `!is_updated_this_week` 过滤器保证，今天属于本周）
    let role_name = task.role_name.as_deref().unwrap_or("管家");
    let message = format!(
        "你的大石头'{}'这周还没动，要不要今天安排一点时间？",
        task.title
    );

    // e. 创建"轻触"通知 — 需要有 role_id 才能创建通知
    let mut delivered = false;
    let notification_id = if let Some(ref role_id) = task.role_id {
        match notification_service::create_notification_for_role(
            pool,
            role_id,
            NotificationLevel::Tap,
            &message,
        )
        .await
        {
            Ok(notification) => {
                tracing::info!(
                    task_id = %task.id,
                    notification_id = %notification.id,
                    level = %notification.level,
                    "大石头保护提醒通知已创建"
                );
                delivered = true;
                notification.id
            }
            Err(e) => {
                tracing::warn!(
                    task_id = %task.id,
                    role_id = %role_id,
                    error = %e,
                    "大石头保护提醒通知创建失败（继续写入对话消息）"
                );
                String::new()
            }
        }
    } else {
        tracing::info!(
            task_id = %task.id,
            "管家任务无 role_id，跳过通知创建"
        );
        String::new()
    };

    // f. 写入管家对话消息
    match db::conversations::get_or_create_butler_conversation(conv_pool).await {
        Ok(conv) => {
            match db::conversations::insert_message(conv_pool, &conv.id, "assistant", &message, true)
                .await
            {
                Ok(_) => {
                    delivered = true;
                }
                Err(e) => {
                    tracing::warn!(
                        task_id = %task.id,
                        conversation_id = %conv.id,
                        error = %e,
                        "大石头保护提醒对话消息写入失败"
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                task_id = %task.id,
                error = %e,
                "获取/创建管家对话失败"
            );
        }
    }

    // 通知与对话消息均未成功投递 → 不消耗提醒额度
    if !delivered {
        tracing::warn!(
            task_id = %task.id,
            task_title = %task.title,
            "大石头保护提醒通知与对话消息均投递失败，跳过计数自增（下一轮重试）"
        );
        return Ok(());
    }

    // g. 至少一项投递成功 → 自增提醒计数
    db::big_rock_protection_reminders::upsert_reminder(pool, &task.id).await?;

    // h. emit Tauri Event
    if let Some(handle) = app_handle {
        let payload = BigRockProtectionPayload {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            role_id: task.role_id.clone(),
            role_name: Some(role_name.to_string()),
            message: message.clone(),
            notification_id: notification_id.clone(),
        };
        if let Err(e) = handle.emit(BIGROCK_PROTECTION_EVENT, &payload) {
            tracing::warn!(
                task_id = %task.id,
                error = %e,
                "emit bigrock:protection 事件失败"
            );
        }
    }

    tracing::info!(
        task_id = %task.id,
        task_title = %task.title,
        role_name = %role_name,
        notification_id = %notification_id,
        "大石头保护提醒已生成"
    );

    Ok(())
}

/// 周五大石头未完成检查（AC4）。
///
/// 若为周五且有未完成大石头，生成汇总提醒"本周大石头还有 N 个未完成，周末要安排时间吗？"。
/// 返回 `Ok(true)` 表示已触发提醒，`Ok(false)` 表示跳过（非周五或无未完成大石头）。
pub async fn check_friday_bigrock_status(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<bool, AppError> {
    // 非周五返回
    if !is_friday() {
        return Ok(false);
    }

    let big_rock_tasks = db::tasks::list_all_tasks(pool, None, Some(true)).await?;

    // 过滤未完成
    let unfinished: Vec<&CrossRoleTask> = big_rock_tasks.iter().filter(|t| !t.is_completed).collect();

    if unfinished.is_empty() {
        return Ok(false);
    }

    let count = unfinished.len();
    let message = format!("本周大石头还有 {} 个未完成，周末要安排时间吗？", count);

    // 创建"轻触"通知（用第一个活跃角色的 ID）
    let notification_id = create_notification_if_possible(pool, &message).await;
    write_butler_message(conv_pool, &message).await;
    emit_protection_event(app_handle, &notification_id, &message);

    tracing::info!(
        unfinished_count = count,
        notification_id = %notification_id,
        "周五大石头未完成检查已触发"
    );

    Ok(true)
}

/// 使用第一个活跃角色的 ID 创建"轻触"通知。
async fn create_notification_if_possible(pool: &SqlitePool, message: &str) -> String {
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
        message,
    )
    .await
    {
        Ok(notification) => {
            tracing::info!(
                role_id = %first_role.id,
                notification_id = %notification.id,
                level = %notification.level,
                "周五大石头检查通知已创建"
            );
            notification.id
        }
        Err(e) => {
            tracing::warn!(
                role_id = %first_role.id,
                error = %e,
                "周五大石头检查通知创建失败（继续写入对话消息）"
            );
            String::new()
        }
    }
}

/// 写入管家对话消息。
async fn write_butler_message(conv_pool: &ConversationsPool, message: &str) {
    match db::conversations::get_or_create_butler_conversation(conv_pool).await {
        Ok(conv) => {
            match db::conversations::insert_message(
                conv_pool,
                &conv.id,
                "assistant",
                message,
                true,
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(
                        conversation_id = %conv.id,
                        "周五大石头检查管家对话消息已写入"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        conversation_id = %conv.id,
                        error = %e,
                        "周五大石头检查对话消息写入失败"
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "获取/创建管家对话失败");
        }
    }
}

/// emit `bigrock:protection` 事件给前端。
fn emit_protection_event(app_handle: Option<&AppHandle>, notification_id: &str, message: &str) {
    if let Some(handle) = app_handle {
        let payload = BigRockProtectionPayload {
            task_id: String::new(),
            task_title: String::new(),
            role_id: None,
            role_name: None,
            message: message.to_string(),
            notification_id: notification_id.to_string(),
        };
        if let Err(e) = handle.emit(BIGROCK_PROTECTION_EVENT, &payload) {
            tracing::warn!(error = %e, "emit bigrock:protection 事件失败");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_iso_to_local_date_parses_rfc3339() {
        let date = parse_iso_to_local_date("2026-06-22T12:30:00Z");
        assert!(date.is_some());
        assert_eq!(date.unwrap().format("%Y-%m-%d").to_string(), "2026-06-22");
    }

    #[test]
    fn parse_iso_to_local_date_returns_none_for_invalid() {
        assert!(parse_iso_to_local_date("not-a-date").is_none());
        assert!(parse_iso_to_local_date("").is_none());
    }

    #[test]
    fn is_reminded_today_returns_false_for_past_date() {
        assert!(!is_reminded_today("2020-01-01T00:00:00Z"));
    }

    #[test]
    fn is_reminded_today_returns_false_for_invalid() {
        assert!(!is_reminded_today("invalid"));
    }

    #[test]
    fn compute_this_week_monday_returns_monday() {
        let monday = compute_this_week_monday();
        // 本周一的 weekday 应该是 Monday (num_days_from_monday == 0)
        assert_eq!(monday.weekday().num_days_from_monday(), 0);
    }

    #[test]
    fn is_updated_this_week_returns_true_for_today() {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        assert!(is_updated_this_week(&now));
    }

    #[test]
    fn is_updated_this_week_returns_false_for_old_date() {
        assert!(!is_updated_this_week("2020-01-01T00:00:00Z"));
    }

    #[test]
    fn is_updated_this_week_returns_false_for_invalid() {
        assert!(!is_updated_this_week("invalid"));
    }

    #[test]
    fn bigrock_protection_event_name_is_correct() {
        assert_eq!(BIGROCK_PROTECTION_EVENT, "bigrock:protection");
    }

    #[test]
    fn payload_serializes_with_camel_case() {
        let payload = BigRockProtectionPayload {
            task_id: "task-1".to_string(),
            task_title: "测试".to_string(),
            role_id: Some("role-1".to_string()),
            role_name: Some("产品".to_string()),
            message: "test".to_string(),
            notification_id: "abc-123".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("taskId"));
        assert!(json.contains("taskTitle"));
        assert!(json.contains("roleId"));
        assert!(json.contains("roleName"));
        assert!(json.contains("notificationId"));
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

        sqlx::query(
            "CREATE TABLE big_rock_protection_reminders (
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
        .expect("failed to create big_rock_protection_reminders table");

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

    async fn insert_big_rock_task(
        pool: &SqlitePool,
        id: &str,
        role_id: Option<&str>,
        title: &str,
        is_completed: bool,
        updated_at: &str,
    ) {
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, is_completed, created_at, updated_at)
             VALUES (?1, 'role', ?2, ?3, 'Q1', 1, ?4, '2026-01-01T00:00:00Z', ?5)",
        )
        .bind(id)
        .bind(role_id)
        .bind(title)
        .bind(is_completed)
        .bind(updated_at)
        .execute(pool)
        .await
        .expect("failed to insert task");
    }

    #[tokio::test]
    async fn check_protection_generates_reminder_for_stale_bigrock() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 插入一个未完成、本周无进展的大石头（updated_at 为 2020 年）
        insert_big_rock_task(&pool, "task-1", Some("role-1"), "竞品分析", false, "2020-01-01T00:00:00Z").await;

        check_and_generate_protection_reminders_inner(&pool, &conv_pool, None)
            .await
            .expect("check failed");

        // 验证管家对话消息已写入
        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant' AND content LIKE '%大石头%'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 1);

        // 验证提醒记录已创建
        let reminder = db::big_rock_protection_reminders::get_reminder_for_task(&pool, "task-1")
            .await
            .unwrap()
            .expect("reminder should exist");
        assert_eq!(reminder.reminded_count, 1);
    }

    #[tokio::test]
    async fn check_protection_skips_when_bigrock_updated_this_week() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // updated_at 为今天（本周有进展）
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        insert_big_rock_task(&pool, "task-1", Some("role-1"), "竞品分析", false, &now).await;

        check_and_generate_protection_reminders_inner(&pool, &conv_pool, None)
            .await
            .expect("check failed");

        // 验证无管家对话消息
        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 0);
    }

    #[tokio::test]
    async fn check_protection_skips_completed_bigrock() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 已完成的大石头
        insert_big_rock_task(&pool, "task-1", Some("role-1"), "竞品分析", true, "2020-01-01T00:00:00Z").await;

        check_and_generate_protection_reminders_inner(&pool, &conv_pool, None)
            .await
            .expect("check failed");

        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 0);
    }

    #[tokio::test]
    async fn check_protection_skips_when_already_reminded_today() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        insert_big_rock_task(&pool, "task-1", Some("role-1"), "竞品分析", false, "2020-01-01T00:00:00Z").await;

        // 预先插入今日提醒记录
        db::big_rock_protection_reminders::upsert_reminder(&pool, "task-1")
            .await
            .unwrap();

        check_and_generate_protection_reminders_inner(&pool, &conv_pool, None)
            .await
            .expect("check failed");

        // 验证无新增管家对话消息
        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 0);
    }

    #[tokio::test]
    async fn check_friday_status_generates_reminder_for_unfinished() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 插入 2 个未完成大石头
        insert_big_rock_task(&pool, "task-1", Some("role-1"), "竞品分析", false, "2026-06-01T00:00:00Z").await;
        insert_big_rock_task(&pool, "task-2", Some("role-1"), "用户调研", false, "2026-06-01T00:00:00Z").await;

        let result = check_friday_bigrock_status(&pool, &conv_pool, None)
            .await
            .expect("friday check failed");

        // 注意：非周五时返回 false，周五时返回 true。测试环境无法控制实际日期，
        // 但无论是否周五，只要有未完成大石头且是周五，消息应包含"2 个未完成"。
        if is_friday() {
            assert_eq!(result, true);
            let msg_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM messages WHERE role = 'assistant' AND content LIKE '%2 个未完成%'",
            )
            .fetch_one(&*conv_pool)
            .await
            .unwrap();
            assert_eq!(msg_count, 1);
        } else {
            assert_eq!(result, false);
        }
    }

    #[tokio::test]
    async fn check_friday_status_skips_when_no_unfinished() {
        let (pool, conv_pool) = setup_test_db().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 只有一个已完成的大石头
        insert_big_rock_task(&pool, "task-1", Some("role-1"), "竞品分析", true, "2026-06-01T00:00:00Z").await;

        let result = check_friday_bigrock_status(&pool, &conv_pool, None)
            .await
            .expect("friday check failed");

        assert_eq!(result, false);

        let msg_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE role = 'assistant'",
        )
        .fetch_one(&*conv_pool)
        .await
        .unwrap();
        assert_eq!(msg_count, 0);
    }
}
