//! Story 4.1: 角色后台调度器 — 按固定时间点运行工作循环
//!
//! 职责：
//! - 在 Tauri 启动时 spawn 一个后台 tokio task，使用 60 秒基础 tick
//! - 每次 tick 查询活跃角色列表，根据 `proactivity_level` 决定是否触发工作循环
//! - `passive` 跳过，`moderate` / `proactive` 按用户配置的 HH:MM 时间点触发
//! - 时间点基于本地时间（chrono::Local），同一角色同一时间点只触发一次
//! - 用户可通过 `scheduler_get_times` / `scheduler_set_times` 命令自定义时间点
//! - 每个角色独立 `tokio::spawn` 执行工作循环，互不阻塞
//! - 动态读取数据库，角色 CRUD 和 proactivity 变更立即生效
//! - 错误只 `tracing::warn!`，绝不 panic，绝不阻塞 Tauri setup
//!
//! Story 4.2: `run_work_loop_for_role` 调用建议生成服务，写入 pending 建议。

use std::collections::HashMap;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::time::Instant;

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::models::notification::NotificationNewPayload;
use crate::models::role::Role;
use crate::services::notification_service;
use crate::services::suggestion_generator::NotificationLevel;

/// `moderate` 角色的默认触发时间点（本地时间）：09:00、14:00、21:00（每日 3 次）
pub const DEFAULT_MODERATE_TIMES: &[&str] = &["09:00", "14:00", "21:00"];

/// `proactive` 角色的默认触发时间点（本地时间）：09:00、11:00、14:00、16:00、21:00（每日 5 次）
pub const DEFAULT_PROACTIVE_TIMES: &[&str] = &["09:00", "11:00", "14:00", "16:00", "21:00"];

/// app_settings 中存储 moderate 时间点的 key
pub const MODERATE_TIMES_KEY: &str = "scheduler.moderate_times";

/// app_settings 中存储 proactive 时间点的 key
pub const PROACTIVE_TIMES_KEY: &str = "scheduler.proactive_times";

/// 调度器基础 tick 间隔：60 秒
const BASE_TICK_SECS: u64 = 60;

/// Story 6.1: 简报时间读取失败时的降级默认值
const DEFAULT_BRIEFING_TIME_FALLBACK: &str = "08:00";

/// 每档最多允许的时间点数量
const MAX_TIMES_PER_LEVEL: usize = 12;

/// 根据 `proactivity_level` 返回默认触发时间点。
///
/// `passive` → `None`（跳过），`moderate` → 默认 3 个时间点，`proactive` → 默认 5 个时间点。
/// 未知值 → `None`（安全降级，跳过）。
pub fn default_times_for_proactivity(level: &str) -> Option<&'static [&'static str]> {
    match level {
        "moderate" => Some(DEFAULT_MODERATE_TIMES),
        "proactive" => Some(DEFAULT_PROACTIVE_TIMES),
        _ => None,
    }
}

/// 从数据库读取指定 proactivity_level 的触发时间点列表。
/// 读不到则返回默认值。
pub async fn get_trigger_times(
    pool: &SqlitePool,
    level: &str,
) -> Result<Option<Vec<String>>, AppError> {
    let key = match level {
        "moderate" => MODERATE_TIMES_KEY,
        "proactive" => PROACTIVE_TIMES_KEY,
        _ => return Ok(None),
    };

    match db::app_settings::get_setting(pool, key).await? {
        Some(raw) => {
            let times: Vec<String> = serde_json::from_str(&raw)
                .map_err(|e| AppError::DbError(format!("解析调度时间点失败: {}", e)))?;
            Ok(Some(times))
        }
        None => Ok(Some(
            default_times_for_proactivity(level)
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )),
    }
}

/// 校验时间点格式："HH:MM"，HH 00-23，MM 00-59。
fn validate_time_format(time: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 2 {
        return Err(AppError::ValidationError(format!(
            "时间格式错误：{}，应为 HH:MM",
            time
        )));
    }
    let h: u32 = parts[0]
        .parse()
        .map_err(|_| AppError::ValidationError(format!("小时解析失败: {}", time)))?;
    let m: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::ValidationError(format!("分钟解析失败: {}", time)))?;
    if h > 23 || m > 59 {
        return Err(AppError::ValidationError(format!(
            "时间超出范围: {}",
            time
        )));
    }
    Ok(())
}

/// 校验时间点列表：格式正确、无重复、不超过上限、排序。
pub fn validate_times(times: &[String]) -> Result<Vec<String>, AppError> {
    if times.len() > MAX_TIMES_PER_LEVEL {
        return Err(AppError::ValidationError(format!(
            "时间点数量超过上限 {}",
            MAX_TIMES_PER_LEVEL
        )));
    }
    let mut seen = std::collections::HashSet::new();
    for t in times {
        validate_time_format(t)?;
        if !seen.insert(t.clone()) {
            return Err(AppError::ValidationError(format!(
                "重复的时间点: {}",
                t
            )));
        }
    }
    let mut sorted = times.to_vec();
    sorted.sort();
    Ok(sorted)
}

/// 将当前本地时间格式化为 "HH:MM"。
fn current_hhmm(now: &chrono::DateTime<Local>) -> String {
    format!("{:02}:{:02}", now.hour(), now.minute())
}

/// 生成去重键："YYYY-MM-DD HH:MM"，确保同一角色同一时间点只触发一次。
fn trigger_key(date: chrono::NaiveDate, hhmm: &str) -> String {
    format!("{} {}", date, hhmm)
}

/// 工作循环 — 调用建议生成服务，将生成的建议写入 DB。
///
/// 永不向上抛错：内部所有失败都降级为 `tracing::warn!` + 返回 `Ok(())`。
pub async fn run_work_loop_for_role(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    role: &Role,
    app_handle: Option<&AppHandle>,
) -> Result<(), AppError> {
    tracing::info!(
        role_id = %role.id,
        role_name = %role.name,
        proactivity_level = %role.proactivity_level,
        "工作循环触发"
    );

    let suggestions = match crate::services::suggestion_generator::generate_suggestions(pool, role)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                role_name = %role.name,
                error = %e,
                "建议生成失败（降级完成本轮循环）"
            );
            return Ok(());
        }
    };

    let suggestions = crate::services::suggestion_generator::filter_suggestions_by_proactivity(
        suggestions,
        &role.proactivity_level,
    );

    if suggestions.is_empty() {
        tracing::info!(
            role_id = %role.id,
            role_name = %role.name,
            "本次无建议生成"
        );
        return Ok(());
    }

    // 获取管家会话 ID，将建议绑定到当前管家会话
    let butler_conv_id = match crate::db::conversations::get_or_create_butler_conversation(conv_pool).await {
        Ok(conv) => Some(conv.id),
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                error = %e,
                "获取管家会话失败，建议将不绑定会话"
            );
            None
        }
    };
    let suggestions: Vec<_> = suggestions
        .into_iter()
        .map(|mut s| {
            s.conversation_id = butler_conv_id.clone();
            s
        })
        .collect();

    let mut written = 0;
    for input in &suggestions {
        match db::suggestions::create_suggestion(pool, input).await {
            Ok(s) => {
                written += 1;
                tracing::info!(
                    role_id = %role.id,
                    role_name = %role.name,
                    suggestion_id = %s.id,
                    title = %s.title,
                    priority = %s.priority,
                    "建议已写入"
                );

                // Story 4.5: 为每条建议生成对应级别的通知
                let notification_level = match s.priority.as_str() {
                    "high" => NotificationLevel::Knock,
                    "medium" => NotificationLevel::Tap,
                    _ => NotificationLevel::Whisper,
                };

                match notification_service::create_notification_for_role(
                    pool,
                    &role.id,
                    notification_level,
                    &s.title,
                )
                .await
                {
                    Ok(notification) => {
                        tracing::info!(
                            role_id = %role.id,
                            suggestion_id = %s.id,
                            notification_id = %notification.id,
                            requested = ?notification_level,
                            actual_level = %notification.level,
                            "通知已创建"
                        );

                        // emit Tauri Event（如果有 AppHandle）
                        if let Some(handle) = app_handle {
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
                            let _ = handle.emit("notification:new", &payload);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            role_id = %role.id,
                            suggestion_id = %s.id,
                            error = %e,
                            "通知创建失败（不影响建议写入）"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    role_id = %role.id,
                    role_name = %role.name,
                    error = %e,
                    title = %input.title,
                    "写入建议失败（跳过该条，继续写入其他建议）"
                );
            }
        }
    }

    tracing::info!(
        role_id = %role.id,
        role_name = %role.name,
        total = suggestions.len(),
        written,
        "工作循环建议写入完成"
    );

    // Story 4.8: 工作循环完成后重新计算角色能量值
    if let Err(e) = crate::services::energy_calculator::calculate_and_update_energy(
        pool, conv_pool, role,
    ).await {
        tracing::warn!(
            role_id = %role.id,
            role_name = %role.name,
            error = %e,
            "能量值计算失败（不影响工作循环结果）"
        );
    }

    Ok(())
}

/// 启动后台调度器。在 Tauri `setup` 中调用。
///
/// 设计模式：短 tick（60 秒）+ 固定时间点 + 去重键。
/// 每次 tick 查询活跃角色列表，检查当前本地时间是否命中该角色的触发时间点。
/// 新建/归档/删除角色和 proactivity_level 变更自动生效。
/// 时间点可由用户通过 `scheduler_set_times` 命令自定义，下次 tick 自动生效。
///
/// 首次启动延迟：消耗 `interval.tick()` 的首次立即返回，避免启动时并发太多后台任务。
/// 同一角色同一时间点（同一日期+同一 HH:MM）只触发一次，跨天自动重置。
pub fn spawn_scheduler(pool: SqlitePool, conv_pool: crate::db::pool::ConversationsPool, app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(BASE_TICK_SECS));
        // 消耗首次立即 tick，避免启动时并发太多后台任务
        interval.tick().await;

        // role_id → "YYYY-MM-DD HH:MM" 去重键
        let mut last_triggered_map: HashMap<String, String> = HashMap::new();

        // Story 6.1: 简报触发去重 — 记录上次触发日期，每天只触发一次
        let mut last_briefing_trigger_date: Option<String> = None;

        // Story 6.3: 大石头提醒去重 — 记录上次触发的 ISO 周，每周只触发一次
        let mut last_bigrock_trigger_week: Option<String> = None;

        // Story 6.2 (AC7): 启动时读取节奏化时间配置（review/bigrock 为预留读取，
        // 实际触发逻辑在 Story 6.3/6.4 实现）
        match get_review_schedule(&pool).await {
            Ok((day, time)) => tracing::debug!(review_day = %day, review_time = %time, "调度器启动读取周复盘时间配置"),
            Err(e) => tracing::warn!(error = %e, "调度器启动读取周复盘时间配置失败（降级继续）"),
        }
        match get_bigrock_reminder_schedule(&pool).await {
            Ok((day, time)) => tracing::debug!(bigrock_reminder_day = %day, bigrock_reminder_time = %time, "调度器启动读取大石头规划提醒时间配置"),
            Err(e) => tracing::warn!(error = %e, "调度器启动读取大石头规划提醒时间配置失败（降级继续）"),
        }

        loop {
            // 每次 tick 动态读取活跃角色列表（不缓存）
            let roles = match db::roles::list_active_roles(&pool).await {
                Ok(roles) => roles,
                Err(e) => {
                    tracing::warn!(error = %e, "调度器查询活跃角色失败（降级继续）");
                    interval.tick().await;
                    continue;
                }
            };

            let now_local = Local::now();
            let current_hhmm = current_hhmm(&now_local);
            let current_key = trigger_key(now_local.date_naive(), &current_hhmm);

            // 当前活跃角色 id 集合，用于 tick 末尾清理过期状态
            let active_ids: std::collections::HashSet<&str> =
                roles.iter().map(|r| r.id.as_str()).collect();

            for role in &roles {
                // 从 DB 读取该角色 proactivity_level 对应的触发时间点
                let trigger_times = match get_trigger_times(&pool, &role.proactivity_level).await {
                    Ok(Some(times)) => times,
                    Ok(None) => continue, // passive 或未知值，跳过
                    Err(e) => {
                        tracing::warn!(
                            role_id = %role.id,
                            error = %e,
                            "读取调度时间点失败（降级跳过）"
                        );
                        continue;
                    }
                };

                // 检查当前 HH:MM 是否在触发时间点列表中
                if !trigger_times.contains(&current_hhmm) {
                    continue;
                }

                // 同一角色同一时间点只触发一次
                if last_triggered_map.get(&role.id) == Some(&current_key) {
                    continue;
                }

                // 标记已触发
                last_triggered_map.insert(role.id.clone(), current_key.clone());

                // 每个角色独立 spawn 执行，互不阻塞
                let pool_clone = pool.clone();
                let conv_pool_clone = conv_pool.clone();
                let role_clone = role.clone();
                let handle_clone = app_handle.clone();
                tokio::spawn(async move {
                    // 墙钟时间戳（ISO 8601），便于日志排查实际触发时刻
                    let triggered_at = crate::db::settings::chrono_now_pub();
                    let start = Instant::now();
                    let result = run_work_loop_for_role(&pool_clone, &conv_pool_clone, &role_clone, Some(&handle_clone)).await;
                    let elapsed = start.elapsed().as_millis() as u64;

                    match result {
                        Ok(()) => {
                            tracing::info!(
                                role_id = %role_clone.id,
                                role_name = %role_clone.name,
                                triggered_at = %triggered_at,
                                duration_ms = elapsed,
                                "工作循环完成"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                role_id = %role_clone.id,
                                role_name = %role_clone.name,
                                triggered_at = %triggered_at,
                                error = %e,
                                duration_ms = elapsed,
                                "工作循环失败"
                            );
                        }
                    }
                });
            }

            // 清理已归档/删除角色的状态，避免内存随角色 churn 无界增长
            last_triggered_map.retain(|id, _| active_ids.contains(id.as_str()));

            // Story 6.1: 晨间简报触发 — 读取 app_settings 中的 briefing_time，
            // 当前 HH:MM 匹配且当天尚未触发时，spawn 异步生成简报
            let today_date = now_local.date_naive().format("%Y-%m-%d").to_string();
            if last_briefing_trigger_date.as_deref() != Some(&today_date) {
                let briefing_time = match crate::services::briefing_generator::get_briefing_time(&pool).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!(error = %e, "读取简报时间设置失败（降级跳过）");
                        DEFAULT_BRIEFING_TIME_FALLBACK.to_string()
                    }
                };
                if current_hhmm == briefing_time {
                    last_briefing_trigger_date = Some(today_date.clone());
                    let pool_clone = pool.clone();
                    let conv_pool_clone = conv_pool.clone();
                    let handle_clone = app_handle.clone();
                    tokio::spawn(async move {
                        match crate::services::briefing_generator::generate_briefing_if_needed(
                            &pool_clone,
                            &conv_pool_clone,
                            Some(&handle_clone),
                        )
                        .await
                        {
                            Ok(true) => {
                                tracing::info!(date = %today_date, "调度器触发简报生成完成");
                            }
                            Ok(false) => {
                                tracing::info!(date = %today_date, "简报已存在或被跳过");
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, date = %today_date, "调度器触发简报生成失败");
                            }
                        }
                    });
                }
            }

            // Story 4.6: 每次 tick 都检查 Q2 保护提醒（不受触发时间点限制）
            // 频率控制由 q2_reminders 表的 last_reminded_at 管理（每日 ≤ 1 次）
            if let Err(e) = crate::services::q2_protection_reminder::check_and_generate_reminders(
                &pool,
                &conv_pool,
                Some(&app_handle),
            )
            .await
            {
                tracing::warn!(error = %e, "Q2 保护提醒检查失败（降级继续）");
            }

            // Story 6.3: 大石头规划提醒触发检查
            // 读取 bigrock_reminder_day/time 配置，匹配星期 + HH:MM 时触发，每周只触发一次
            let current_week = iso_week_key(&now_local);
            if last_bigrock_trigger_week.as_deref() != Some(&current_week) {
                let (bigrock_day, bigrock_time) = match get_bigrock_reminder_schedule(&pool).await {
                    Ok((d, t)) => (d, t),
                    Err(e) => {
                        tracing::warn!(error = %e, "读取大石头提醒时间配置失败（降级跳过）");
                        (String::new(), String::new())
                    }
                };
                // 当前星期（1=周一 ~ 7=周日）
                let current_day = (now_local.weekday().num_days_from_monday() + 1).to_string();
                if current_day == bigrock_day && current_hhmm == bigrock_time {
                    last_bigrock_trigger_week = Some(current_week.clone());
                    let pool_clone = pool.clone();
                    let conv_pool_clone = conv_pool.clone();
                    let handle_clone = app_handle.clone();
                    tokio::spawn(async move {
                        match crate::services::bigrock_reminder::check_and_remind_if_needed(
                            &pool_clone,
                            &conv_pool_clone,
                            Some(&handle_clone),
                        )
                        .await
                        {
                            Ok(true) => {
                                tracing::info!(week = %current_week, "调度器触发大石头规划提醒完成");
                            }
                            Ok(false) => {
                                tracing::info!(week = %current_week, "本周已有大石头，跳过提醒");
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, week = %current_week, "调度器触发大石头规划提醒失败");
                            }
                        }
                    });
                }
            }

            interval.tick().await;
        }
    });
}

/// 读取周复盘时间配置（Story 6.4 实现触发逻辑）
pub async fn get_review_schedule(pool: &SqlitePool) -> Result<(String, String), AppError> {
    use crate::commands::settings::{DEFAULT_REVIEW_DAY, DEFAULT_REVIEW_TIME, KEY_REVIEW_DAY, KEY_REVIEW_TIME};
    let day = db::app_settings::get_setting(pool, KEY_REVIEW_DAY)
        .await?
        .unwrap_or_else(|| DEFAULT_REVIEW_DAY.to_string());
    let time = db::app_settings::get_setting(pool, KEY_REVIEW_TIME)
        .await?
        .unwrap_or_else(|| DEFAULT_REVIEW_TIME.to_string());
    Ok((day, time))
}

/// 读取大石头规划提醒时间配置（Story 6.3 实现触发逻辑）
pub async fn get_bigrock_reminder_schedule(pool: &SqlitePool) -> Result<(String, String), AppError> {
    use crate::commands::settings::{
        DEFAULT_BIGROCK_REMINDER_DAY, DEFAULT_BIGROCK_REMINDER_TIME, KEY_BIGROCK_REMINDER_DAY,
        KEY_BIGROCK_REMINDER_TIME,
    };
    let day = db::app_settings::get_setting(pool, KEY_BIGROCK_REMINDER_DAY)
        .await?
        .unwrap_or_else(|| DEFAULT_BIGROCK_REMINDER_DAY.to_string());
    let time = db::app_settings::get_setting(pool, KEY_BIGROCK_REMINDER_TIME)
        .await?
        .unwrap_or_else(|| DEFAULT_BIGROCK_REMINDER_TIME.to_string());
    Ok((day, time))
}

/// 生成 ISO 周编号去重键："YYYY-Www"（如 "2026-W23"），确保每周只触发一次。
fn iso_week_key(now: &chrono::DateTime<Local>) -> String {
    let iso_week = now.date_naive().iso_week();
    format!("{}-W{:02}", iso_week.year(), iso_week.week())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_times_passive_returns_none() {
        assert_eq!(default_times_for_proactivity("passive"), None);
    }

    #[test]
    fn default_times_moderate_returns_3_slots() {
        assert_eq!(
            default_times_for_proactivity("moderate"),
            Some(&["09:00", "14:00", "21:00"][..])
        );
    }

    #[test]
    fn default_times_proactive_returns_5_slots() {
        assert_eq!(
            default_times_for_proactivity("proactive"),
            Some(&["09:00", "11:00", "14:00", "16:00", "21:00"][..])
        );
    }

    #[test]
    fn default_times_unknown_returns_none() {
        assert_eq!(default_times_for_proactivity("unknown"), None);
        assert_eq!(default_times_for_proactivity(""), None);
    }

    #[test]
    fn validate_times_accepts_valid_input() {
        let input = vec!["09:00".to_string(), "14:00".to_string(), "21:00".to_string()];
        let result = validate_times(&input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn validate_times_rejects_bad_format() {
        assert!(validate_times(&vec!["0900".to_string()]).is_err());
        assert!(validate_times(&vec!["25:00".to_string()]).is_err());
        assert!(validate_times(&vec!["09:60".to_string()]).is_err());
        assert!(validate_times(&vec!["".to_string()]).is_err());
        assert!(validate_times(&vec!["abc".to_string()]).is_err());
    }

    #[test]
    fn validate_times_rejects_duplicates() {
        let input = vec!["09:00".to_string(), "09:00".to_string()];
        assert!(validate_times(&input).is_err());
    }

    #[test]
    fn validate_times_rejects_too_many() {
        let input: Vec<String> = (0..13).map(|h| format!("{:02}:00", h)).collect();
        assert!(validate_times(&input).is_err());
    }

    #[test]
    fn validate_times_sorts_output() {
        let input = vec!["21:00".to_string(), "09:00".to_string(), "14:00".to_string()];
        let result = validate_times(&input).unwrap();
        assert_eq!(result, vec!["09:00", "14:00", "21:00"]);
    }

    #[test]
    fn current_hhmm_formats_correctly() {
        let now = Local::now();
        let hhmm = current_hhmm(&now);
        assert_eq!(hhmm.len(), 5);
        assert_eq!(hhmm.chars().nth(2), Some(':'));
    }

    #[test]
    fn trigger_key_distinguishes_different_times_same_date() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        assert_ne!(trigger_key(date, "09:00"), trigger_key(date, "14:00"));
    }

    #[test]
    fn trigger_key_distinguishes_same_time_different_dates() {
        let d1 = chrono::NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let d2 = chrono::NaiveDate::from_ymd_opt(2026, 6, 22).unwrap();
        assert_ne!(trigger_key(d1, "09:00"), trigger_key(d2, "09:00"));
    }

    #[tokio::test]
    async fn get_trigger_times_returns_default_when_no_db_setting() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
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

        let times = get_trigger_times(&pool, "moderate").await.unwrap();
        assert_eq!(
            times,
            Some(vec![
                "09:00".to_string(),
                "14:00".to_string(),
                "21:00".to_string(),
            ])
        );
    }

    #[tokio::test]
    async fn get_trigger_times_returns_db_value_when_set() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
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

        db::app_settings::set_setting(
            &pool,
            MODERATE_TIMES_KEY,
            r#"["08:30","12:00","18:30","22:00"]"#,
        )
        .await
        .unwrap();

        let times = get_trigger_times(&pool, "moderate").await.unwrap();
        assert_eq!(
            times,
            Some(vec![
                "08:30".to_string(),
                "12:00".to_string(),
                "18:30".to_string(),
                "22:00".to_string(),
            ])
        );
    }

    #[tokio::test]
    async fn get_trigger_times_passive_returns_none() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        let times = get_trigger_times(&pool, "passive").await.unwrap();
        assert_eq!(times, None);
    }

    #[tokio::test]
    async fn run_work_loop_for_role_returns_ok_without_provider() {
        let role = Role {
            id: "test-role-id".to_string(),
            name: "测试角色".to_string(),
            icon: "🎯".to_string(),
            color: "#6366F1".to_string(),
            goal: "测试目标".to_string(),
            personality_prompt: String::new(),
            status: "active".to_string(),
            energy: 100,
            energy_updated_at: None,
            skills_config: "{}".to_string(),
            proactivity_level: "moderate".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

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

        sqlx::query("INSERT INTO roles (id, name) VALUES ('test-role-id', '测试角色')")
            .execute(&pool)
            .await
            .expect("failed to insert role");

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

        let conv_pool = crate::db::pool::ConversationsPool(
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
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&*conv_pool)
        .await
        .expect("failed to create conversations table");

        let result = run_work_loop_for_role(&pool, &conv_pool, &role, None).await;
        assert!(result.is_ok());
    }

    // Story 6.3: ISO 周去重键和星期匹配测试

    #[test]
    fn iso_week_key_formats_correctly() {
        // 2026-06-02 是周一，属于 2026-W23
        let dt = chrono::NaiveDate::from_ymd_opt(2026, 6, 2)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        let key = iso_week_key(&dt);
        assert!(key.starts_with("2026-W"));
    }

    #[test]
    fn iso_week_key_distinguishes_different_weeks() {
        let dt1 = chrono::NaiveDate::from_ymd_opt(2026, 6, 2)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        let dt2 = chrono::NaiveDate::from_ymd_opt(2026, 6, 9)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        assert_ne!(iso_week_key(&dt1), iso_week_key(&dt2));
    }

    #[test]
    fn iso_week_key_same_week_different_days() {
        // 2026-06-02 (周二) 和 2026-06-05 (周五) 同属 W23
        let dt1 = chrono::NaiveDate::from_ymd_opt(2026, 6, 2)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        let dt2 = chrono::NaiveDate::from_ymd_opt(2026, 6, 5)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        assert_eq!(iso_week_key(&dt1), iso_week_key(&dt2));
    }

    #[test]
    fn weekday_num_days_from_monday_matches_settings_encoding() {
        // 2026-06-01 是周一 → num_days_from_monday() = 0 → +1 = 1
        let monday = chrono::NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        assert_eq!(monday.weekday().num_days_from_monday() + 1, 1);

        // 2026-06-07 是周日 → num_days_from_monday() = 6 → +1 = 7
        let sunday = chrono::NaiveDate::from_ymd_opt(2026, 6, 7)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap();
        assert_eq!(sunday.weekday().num_days_from_monday() + 1, 7);
    }
}
