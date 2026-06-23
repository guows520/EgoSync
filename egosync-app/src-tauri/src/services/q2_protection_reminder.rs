//! Story 4.6: Q2 保护提醒服务 — 检测 at_risk Q2 任务并生成提醒
//!
//! 职责：
//! - 查询所有 `protection_status = 'at_risk'` 的 Q2 未完成任务
//! - 对每个任务按频率限制（每日 ≤ 1 次、连续 3 天后停止）生成提醒
//! - 创建"轻触"级通知 + 写入管家对话消息
//! - emit Tauri Event `q2:reminder` 供前端实时更新

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::services::notification_service;
use crate::services::suggestion_generator::NotificationLevel;
use crate::services::task_protection_watch::AT_RISK_DAYS;

/// 连续提醒 3 天无响应后停止提醒（AC3）
const MAX_REMINDER_DAYS: i64 = 3;

/// Tauri Event 名称
pub const Q2_REMINDER_EVENT: &str = "q2:reminder";

/// Q2 提醒事件 payload，发给前端
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Q2ReminderPayload {
    pub task_id: String,
    pub task_title: String,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub message: String,
    pub notification_id: String,
}

/// 判断 `last_reminded_at`（ISO 8601 UTC）的本地日期是否与今天相同。
fn is_reminded_today(last_reminded_at: &str) -> bool {
    let now_local = chrono::Local::now().date_naive();
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

/// 核心函数：检查 at_risk Q2 任务并生成提醒。
///
/// - `pool`: 主数据库连接池（tasks / notifications / q2_reminders）
/// - `conv_pool`: 对话数据库连接池（管家对话消息）
/// - `app_handle`: 可选，有则 emit `q2:reminder` 事件给前端
///
/// 错误只 `tracing::warn!`，不阻塞其他任务的提醒生成。
pub async fn check_and_generate_reminders(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<(), AppError> {
    let at_risk_tasks = db::tasks::list_at_risk_q2_tasks(pool).await?;

    if at_risk_tasks.is_empty() {
        return Ok(());
    }

    for task in &at_risk_tasks {
        if let Err(e) = process_single_task(pool, conv_pool, app_handle, task).await {
            tracing::warn!(
                task_id = %task.id,
                task_title = %task.title,
                error = %e,
                "Q2 提醒生成失败（跳过该任务，继续处理其他任务）"
            );
        }
    }

    Ok(())
}

async fn process_single_task(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
    task: &crate::models::task::CrossRoleTask,
) -> Result<(), AppError> {
    // a. 查询已有提醒记录
    let existing = db::q2_reminders::get_reminder_for_task(pool, &task.id).await?;

    // b. 连续提醒 3 天无响应 → 跳过
    if let Some(ref reminder) = existing {
        if reminder.reminded_count >= MAX_REMINDER_DAYS {
            tracing::info!(
                task_id = %task.id,
                reminded_count = reminder.reminded_count,
                "Q2 任务已提醒 {} 次，停止提醒",
                reminder.reminded_count
            );
            return Ok(());
        }
    }

    // c. 今天已提醒过 → 跳过
    if let Some(ref reminder) = existing {
        if is_reminded_today(&reminder.last_reminded_at) {
            return Ok(());
        }
    }

    // d. 生成提醒
    // 生成提醒文案
    let role_name = task.role_name.as_deref().unwrap_or("管家");
    let message = format!(
        "你的'{}'任务已经 {} 天没动了，要不要今天安排一下？",
        task.title, AT_RISK_DAYS
    );

    // 创建"轻触"级通知 — 需要有 role_id 才能创建通知
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
                    "Q2 提醒通知已创建"
                );
                delivered = true;
                notification.id
            }
            Err(e) => {
                tracing::warn!(
                    task_id = %task.id,
                    role_id = %role_id,
                    error = %e,
                    "Q2 提醒通知创建失败（继续写入对话消息）"
                );
                String::new()
            }
        }
    } else {
        // butler 任务没有 role_id，跳过通知创建
        tracing::info!(
            task_id = %task.id,
            "管家任务无 role_id，跳过通知创建"
        );
        String::new()
    };

    // 写入管家对话消息
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
                        "Q2 提醒对话消息写入失败"
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

    // 通知与对话消息均未成功投递 → 不消耗提醒额度，下一轮重试
    if !delivered {
        tracing::warn!(
            task_id = %task.id,
            task_title = %task.title,
            "Q2 提醒通知与对话消息均投递失败，跳过计数自增（下一轮重试）"
        );
        return Ok(());
    }

    // 至少一项投递成功 → 自增提醒计数（AC3 频率/上限控制）
    db::q2_reminders::upsert_reminder(pool, &task.id).await?;

    // emit Tauri Event
    if let Some(handle) = app_handle {
        let payload = Q2ReminderPayload {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            role_id: task.role_id.clone(),
            role_name: Some(role_name.to_string()),
            message: message.clone(),
            notification_id: notification_id.clone(),
        };
        if let Err(e) = handle.emit(Q2_REMINDER_EVENT, &payload) {
            tracing::warn!(
                task_id = %task.id,
                error = %e,
                "emit q2:reminder 事件失败"
            );
        }
    }

    tracing::info!(
        task_id = %task.id,
        task_title = %task.title,
        role_name = %role_name,
        notification_id = %notification_id,
        "Q2 提醒已生成"
    );

    Ok(())
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
    fn parse_iso_to_local_date_parses_naive_format() {
        let date = parse_iso_to_local_date("2026-06-22T12:30:00Z");
        assert!(date.is_some());
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
    fn max_reminder_days_is_three() {
        assert_eq!(MAX_REMINDER_DAYS, 3);
    }

    #[test]
    fn q2_reminder_event_name_is_correct() {
        assert_eq!(Q2_REMINDER_EVENT, "q2:reminder");
    }
}
