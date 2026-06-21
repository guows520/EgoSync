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

use chrono::{Local, Timelike};
use sqlx::SqlitePool;
use tokio::time::Instant;

use crate::db;
use crate::error::AppError;
use crate::models::role::Role;

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
pub async fn run_work_loop_for_role(pool: &SqlitePool, role: &Role) -> Result<(), AppError> {
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

    if suggestions.is_empty() {
        tracing::info!(
            role_id = %role.id,
            role_name = %role.name,
            "本次无建议生成"
        );
        return Ok(());
    }

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
pub fn spawn_scheduler(pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(BASE_TICK_SECS));
        // 消耗首次立即 tick，避免启动时并发太多后台任务
        interval.tick().await;

        // role_id → "YYYY-MM-DD HH:MM" 去重键
        let mut last_triggered_map: HashMap<String, String> = HashMap::new();

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
                let role_clone = role.clone();
                tokio::spawn(async move {
                    // 墙钟时间戳（ISO 8601），便于日志排查实际触发时刻
                    let triggered_at = crate::db::settings::chrono_now_pub();
                    let start = Instant::now();
                    let result = run_work_loop_for_role(&pool_clone, &role_clone).await;
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

            interval.tick().await;
        }
    });
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

        let result = run_work_loop_for_role(&pool, &role).await;
        assert!(result.is_ok());
    }
}
