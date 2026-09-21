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
//!
//! Story 17.2（FR-47）：触发去重持久化——原五处循环局部内存去重
//! （last_triggered_map / last_briefing_trigger_date / last_bigrock_trigger_week /
//! last_review_trigger_week / last_bigrock_friday_check_date，重启即失忆——云端
//! 7×24 场景同分钟重启会重复触发）统一收纳入 `scheduler_triggers` 表，
//! 经 `db::scheduler_triggers` 唯一存取层读写：重启不重复、停机跨过的计划
//! 时刻跳过不补发（诚实代价）、TZ 变更后首个周期至多一次跳过或重复
//! （键含时区维度的已裁决权衡）。
//!
//! 时间源三分表（架构 ⑨ 冻结，本文件的三处落点）：
//! - 持久化：`scheduler_triggers.last_triggered_at` 一律 `chrono_now_pub` 的
//!   UTC RFC3339 串（禁改 Local）；
//! - 调度判定：本文件全部时间比较一律容器 `Local::now()` 语义零改动（`TZ`
//!   env 生效），派生维度见 `TriggerClock`；
//! - 前端渲染：浏览器 TZ（组件不动，本文件不涉）。

use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike};
use sqlx::SqlitePool;
use tokio::time::Instant;

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::db::scheduler_triggers::{
    has_triggered, record_trigger, JOB_BIGROCK_FRIDAY_CHECK, JOB_BIGROCK_PLANNING, JOB_BRIEFING,
    JOB_WEEKLY_REVIEW, JOB_WORK_LOOP, SCOPE_GLOBAL,
};
use crate::error::AppError;
use crate::events::NOTIFICATION_NEW_EVENT;
use crate::models::notification::NotificationNewPayload;
use crate::models::role::Role;
use crate::services::event_bus::EngineEvents;
use crate::services::notification_service;
use crate::services::secret_store::SecretStore;
use crate::services::suggestion_generator::NotificationLevel;

/// `moderate` 角色的默认触发时间点（本地时间）：09:00、14:00、21:00（每日 3 次）
pub const DEFAULT_MODERATE_TIMES: &[&str] = &["09:00", "14:00", "21:00"];

/// `proactive` 角色的默认触发时间点（本地时间）：09:00、11:00、14:00、16:00、21:00（每日 5 次）
pub const DEFAULT_PROACTIVE_TIMES: &[&str] = &["09:00", "11:00", "14:00", "16:00", "21:00"];

/// app_settings 中存储 moderate 时间点的 key
pub const MODERATE_TIMES_KEY: &str = "scheduler.moderate_times";

/// app_settings 中存储 proactive 时间点的 key
pub const PROACTIVE_TIMES_KEY: &str = "scheduler.proactive_times";

// Story 15.2：以下 8 个调度常量自桌面壳 commands/settings.rs 下沉本文件
// （engine 不能反向引用壳），壳侧改 use 回引，值不变。
pub const DEFAULT_REVIEW_DAY: &str = "7";
pub const DEFAULT_REVIEW_TIME: &str = "20:00";
pub const DEFAULT_BIGROCK_REMINDER_DAY: &str = "1";
pub const DEFAULT_BIGROCK_REMINDER_TIME: &str = "09:00";

pub const KEY_REVIEW_DAY: &str = "review_day";
pub const KEY_REVIEW_TIME: &str = "review_time";
pub const KEY_BIGROCK_REMINDER_DAY: &str = "bigrock_reminder_day";
pub const KEY_BIGROCK_REMINDER_TIME: &str = "bigrock_reminder_time";

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

/// Story 17.2: 触发时钟——从注入的 `now: DateTime<Local>` 派生调度判定所需的
/// 全部键维度（表读判定与置位共用）。
///
/// 抽为显式结构是为让判定逻辑可注入时间做引擎测试（同分钟重启 / 停机跳过 /
/// TZ 变更至多一次），运行态由循环内的 `Local::now()` 构造一次、全 tick 复用，
/// 判定语义与既有 `Local::now()` 零改动。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TriggerClock {
    /// "HH:MM"（Local，分钟级精确匹配基准）
    pub hhmm: String,
    /// "YYYY-MM-DD"（Local）
    pub date: String,
    /// "YYYY-Www" ISO 周（Local）
    pub iso_week: String,
    /// "+HH:MM"（Local 的 UTC 偏移，`%:z`）——触发键时区维度：TZ env 变更或
    /// DST 偏移变化 ⇒ 键变化 ⇒ 首个周期至多一次跳过或重复（已裁决权衡）
    pub tz_offset: String,
    /// 1=周一 … 7=周日（Local，与 app_settings 中 day 配置同编码）
    pub weekday: u32,
}

impl TriggerClock {
    pub fn from_now(now: &chrono::DateTime<Local>) -> Self {
        Self {
            hhmm: current_hhmm(now),
            date: now.date_naive().format("%Y-%m-%d").to_string(),
            iso_week: iso_week_key(now),
            tz_offset: now.format("%:z").to_string(),
            weekday: now.weekday().num_days_from_monday() + 1,
        }
    }

    /// work_loop 的 cycle：沿用 `trigger_key` 原样 "YYYY-MM-DD HH:MM"——
    /// 同分钟重启去重精确成立（60s tick 内同分钟至多采样一次，重启后同分钟
    /// 命中同 cycle 即跳过），同日不同时刻照常触发。
    pub fn work_loop_cycle(&self) -> String {
        trigger_key(
            chrono::NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
                .expect("date 由本结构构造，格式恒合法"),
            &self.hhmm,
        )
    }
}

// ── Story 17.2: 判定函数（可注入 clock，读 scheduler_triggers 表） ──────────
// 契约：Ok(true) = 本周期可触发（调用方置位后 spawn）；Ok(false) = 已触发或
// 时间不匹配（跳过）；Err = 表读失败（调用方 warn 降级跳过本 tick，不 panic）。

/// 工作循环判定：当前 HH:MM 命中该角色的触发时间点，且该 (role, 时刻) 未触发过。
pub(crate) async fn should_trigger_work_loop(
    pool: &SqlitePool,
    clock: &TriggerClock,
    role_id: &str,
    trigger_times: &[String],
) -> Result<bool, AppError> {
    if !trigger_times.contains(&clock.hhmm) {
        return Ok(false);
    }
    // has_triggered = 本周期已触发 ⇒ 应触发取反（行缺失/周期落后 ⇒ 可触发）
    Ok(!has_triggered(
        pool,
        JOB_WORK_LOOP,
        role_id,
        &clock.work_loop_cycle(),
        &clock.tz_offset,
    )
    .await?)
}

/// 晨间简报判定：当前 HH:MM 等于 briefing_time，且当日未触发过。
/// （停机跨过计划时刻 ⇒ HH:MM 不再匹配 ⇒ 跳过不补发——诚实代价在此落死）
pub(crate) async fn should_trigger_briefing(
    pool: &SqlitePool,
    clock: &TriggerClock,
    briefing_time: &str,
) -> Result<bool, AppError> {
    if clock.hhmm != briefing_time {
        return Ok(false);
    }
    Ok(!has_triggered(pool, JOB_BRIEFING, SCOPE_GLOBAL, &clock.date, &clock.tz_offset).await?)
}

/// 周期性 job（大石头规划提醒 / 周复盘）判定：星期与 HH:MM 均匹配，且本周未触发过。
pub(crate) async fn should_trigger_weekly(
    pool: &SqlitePool,
    clock: &TriggerClock,
    job: &str,
    day: &str,
    time: &str,
) -> Result<bool, AppError> {
    if clock.weekday.to_string() != day || clock.hhmm != time {
        return Ok(false);
    }
    Ok(!has_triggered(pool, job, SCOPE_GLOBAL, &clock.iso_week, &clock.tz_offset).await?)
}

/// 周五大石头未完成检查判定：当天为周五，且当日未检查过
/// （沿用既有语义：周五任意 tick 触发一次，无 HH:MM 匹配）。
pub(crate) async fn should_trigger_friday_check(
    pool: &SqlitePool,
    clock: &TriggerClock,
) -> Result<bool, AppError> {
    if clock.weekday != 5 {
        return Ok(false);
    }
    Ok(!has_triggered(
        pool,
        JOB_BIGROCK_FRIDAY_CHECK,
        SCOPE_GLOBAL,
        &clock.date,
        &clock.tz_offset,
    )
    .await?)
}

/// 工作循环 — 调用建议生成服务，将生成的建议写入 DB。
///
/// 永不向上抛错：内部所有失败都降级为 `tracing::warn!` + 返回 `Ok(())`。
pub async fn run_work_loop_for_role(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    role: &Role,
    events: Option<&dyn EngineEvents>,
    secret: &dyn SecretStore,
) -> Result<(), AppError> {
    tracing::info!(
        role_id = %role.id,
        role_name = %role.name,
        proactivity_level = %role.proactivity_level,
        "工作循环触发"
    );

    let suggestions = match crate::services::suggestion_generator::generate_suggestions(
        pool, role, secret,
    )
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

                        // emit 事件（如果有事件总线）
                        if let Some(bus) = events {
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
                            let _ = serde_json::to_value(&payload)
                                .map_err(|e| e.to_string())
                                .and_then(|payload| bus.emit(NOTIFICATION_NEW_EVENT, payload));
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
/// 设计模式：短 tick（60 秒）+ 固定时间点 + 持久化去重键。
/// 每次 tick 查询活跃角色列表，检查当前本地时间是否命中该角色的触发时间点。
/// 新建/归档/删除角色和 proactivity_level 变更自动生效。
/// 时间点可由用户通过 `scheduler_set_times` 命令自定义，下次 tick 自动生效。
///
/// 首次启动延迟：消耗 `interval.tick()` 的首次立即返回，避免启动时并发太多后台任务。
/// 同一角色同一时间点（同一日期+同一 HH:MM）只触发一次，跨天自动重置；
/// 去重状态持久化于 `scheduler_triggers` 表（Story 17.2）——同分钟重启不重复，
/// 停机跨过计划时刻跳过不补发。
///
/// Story 15.2 接缝二/四：事件经 EngineEvents 注入；调用方（桌面壳 setup
/// 同步上下文）注入宿主 runtime Handle 派生任务——裸 tokio::spawn 在无
/// reactor 上下文会 panic（v0.1.6-alpha.1 历史事故）。
pub fn spawn_scheduler(
    pool: SqlitePool,
    conv_pool: crate::db::pool::ConversationsPool,
    events: Arc<dyn EngineEvents>,
    secret: Arc<dyn SecretStore>,
    handle: tokio::runtime::Handle,
) {
    handle.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(BASE_TICK_SECS));
        // 消耗首次立即 tick，避免启动时并发太多后台任务
        interval.tick().await;

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

            // 调度判定时间源：容器 Local（三分表 ②，TZ env 生效）
            let now_local = Local::now();
            let clock = TriggerClock::from_now(&now_local);

            // 当前活跃角色 id 集合，用于 tick 末尾清理过期触发状态
            let active_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();

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

                // Story 17.2: 同一角色同一时间点只触发一次——判定读
                // scheduler_triggers 表（重启不失忆）；表读失败 warn 跳过本 tick
                let should = match should_trigger_work_loop(
                    &pool,
                    &clock,
                    &role.id,
                    &trigger_times,
                )
                .await
                {
                    Ok(should) => should,
                    Err(e) => {
                        tracing::warn!(
                            role_id = %role.id,
                            error = %e,
                            "读取工作循环触发状态失败（降级跳过本 tick）"
                        );
                        continue;
                    }
                };
                if !should {
                    continue;
                }

                // 标记已触发（置位保持 spawn 前——deferred-work #225：生成的
                // 瞬时失败当天不重试；置位失败仅跳过本 tick——无写入记录，
                // 下 tick 会重新判定并重试，与简报分支同语义）
                // 评审修复：原注释「烧掉本槽位」与实际行为相反。
                if let Err(e) = record_trigger(
                    &pool,
                    JOB_WORK_LOOP,
                    &role.id,
                    &clock.work_loop_cycle(),
                    &clock.tz_offset,
                )
                .await
                {
                    tracing::warn!(
                        role_id = %role.id,
                        error = %e,
                        "记录工作循环触发状态失败（降级跳过本 tick）"
                    );
                    continue;
                }

                // 每个角色独立 spawn 执行，互不阻塞
                let pool_clone = pool.clone();
                let conv_pool_clone = conv_pool.clone();
                let role_clone = role.clone();
                let events_clone = events.clone();
                let secret_clone = secret.clone();
                tokio::spawn(async move {
                    // 墙钟时间戳（ISO 8601），便于日志排查实际触发时刻
                    let triggered_at = crate::db::settings::chrono_now_pub();
                    let start = Instant::now();
                    let result = run_work_loop_for_role(
                        &pool_clone,
                        &conv_pool_clone,
                        &role_clone,
                        Some(events_clone.as_ref()),
                        secret_clone.as_ref(),
                    )
                    .await;
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

            // 清理已归档/删除角色的触发状态，避免表内随角色 churn 无界积行
            // （原 last_triggered_map.retain 语义——仅 work_loop 行，走表 prune）
            if let Err(e) = db::scheduler_triggers::prune_work_loop_rows(&pool, &active_ids).await
            {
                tracing::warn!(error = %e, "清理过期工作循环触发状态失败（降级继续）");
            }

            // Story 6.1: 晨间简报触发 — 读取 app_settings 中的 briefing_time，
            // 当前 HH:MM 匹配且当天尚未触发时，spawn 异步生成简报
            // （去重改读 scheduler_triggers：Story 17.2）
            let briefing_time = match crate::services::briefing_generator::get_briefing_time(&pool)
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(error = %e, "读取简报时间设置失败（降级跳过）");
                    DEFAULT_BRIEFING_TIME_FALLBACK.to_string()
                }
            };
            match should_trigger_briefing(&pool, &clock, &briefing_time).await {
                Ok(true) => {
                    // 置位保持 spawn 前；置位失败跳过本 tick（不冒重复触发风险）
                    match record_trigger(
                        &pool,
                        JOB_BRIEFING,
                        SCOPE_GLOBAL,
                        &clock.date,
                        &clock.tz_offset,
                    )
                    .await
                    {
                        Ok(()) => {
                            let pool_clone = pool.clone();
                            let conv_pool_clone = conv_pool.clone();
                            let events_clone = events.clone();
                            let secret_clone = secret.clone();
                            let today_date = clock.date.clone();
                            tokio::spawn(async move {
                                match crate::services::briefing_generator::generate_briefing_if_needed(
                                    &pool_clone,
                                    &conv_pool_clone,
                                    Some(events_clone.as_ref()),
                                    secret_clone.as_ref(),
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
                        Err(e) => {
                            tracing::warn!(error = %e, "记录简报触发状态失败（降级跳过本 tick）");
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "读取简报触发状态失败（降级跳过本 tick）");
                }
            }

            // Story 4.6: 每次 tick 都检查 Q2 保护提醒（不受触发时间点限制）
            // 频率控制由 q2_reminders 表的 last_reminded_at 管理（每日 ≤ 1 次）
            if let Err(e) = crate::services::q2_protection_reminder::check_and_generate_reminders(
                &pool,
                &conv_pool,
                Some(events.as_ref()),
            )
            .await
            {
                tracing::warn!(error = %e, "Q2 保护提醒检查失败（降级继续）");
            }

            // Story 6.6: 大石头保护提醒检查（每次 tick 都检查，频率由 DB 记录控制）
            if let Err(e) = crate::services::bigrock_protection::check_and_generate_protection_reminders(
                &pool,
                &conv_pool,
                Some(events.as_ref()),
            )
            .await
            {
                tracing::warn!(error = %e, "大石头保护提醒检查失败（降级继续）");
            }

            // Story 6.3: 大石头规划提醒触发检查
            // 读取 bigrock_reminder_day/time 配置，匹配星期 + HH:MM 时触发，每周只触发一次
            // （去重改读 scheduler_triggers：Story 17.2）
            let (bigrock_day, bigrock_time) = match get_bigrock_reminder_schedule(&pool).await {
                Ok((d, t)) => (d, t),
                Err(e) => {
                    tracing::warn!(error = %e, "读取大石头提醒时间配置失败（降级跳过）");
                    (String::new(), String::new())
                }
            };
            match should_trigger_weekly(
                &pool,
                &clock,
                JOB_BIGROCK_PLANNING,
                &bigrock_day,
                &bigrock_time,
            )
            .await
            {
                Ok(true) => {
                    match record_trigger(
                        &pool,
                        JOB_BIGROCK_PLANNING,
                        SCOPE_GLOBAL,
                        &clock.iso_week,
                        &clock.tz_offset,
                    )
                    .await
                    {
                        Ok(()) => {
                            let pool_clone = pool.clone();
                            let conv_pool_clone = conv_pool.clone();
                            let events_clone = events.clone();
                            let week_clone = clock.iso_week.clone();
                            tokio::spawn(async move {
                                match crate::services::bigrock_reminder::check_and_remind_if_needed(
                                    &pool_clone,
                                    &conv_pool_clone,
                                    Some(events_clone.as_ref()),
                                )
                                .await
                                {
                                    Ok(true) => {
                                        tracing::info!(week = %week_clone, "调度器触发大石头规划提醒完成");
                                    }
                                    Ok(false) => {
                                        tracing::info!(week = %week_clone, "本周已有大石头，跳过提醒");
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, week = %week_clone, "调度器触发大石头规划提醒失败");
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "记录大石头规划提醒触发状态失败（降级跳过本 tick）");
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "读取大石头规划提醒触发状态失败（降级跳过本 tick）");
                }
            }

            // Story 6.4: 周复盘触发检查
            // 读取 review_day/time 配置，匹配星期 + HH:MM 时触发，每周只触发一次
            // （去重改读 scheduler_triggers：Story 17.2）
            let (review_day, review_time) = match get_review_schedule(&pool).await {
                Ok((d, t)) => (d, t),
                Err(e) => {
                    tracing::warn!(error = %e, "读取周复盘时间配置失败（降级跳过）");
                    (String::new(), String::new())
                }
            };
            match should_trigger_weekly(
                &pool,
                &clock,
                JOB_WEEKLY_REVIEW,
                &review_day,
                &review_time,
            )
            .await
            {
                Ok(true) => {
                    match record_trigger(
                        &pool,
                        JOB_WEEKLY_REVIEW,
                        SCOPE_GLOBAL,
                        &clock.iso_week,
                        &clock.tz_offset,
                    )
                    .await
                    {
                        Ok(()) => {
                            let pool_clone = pool.clone();
                            let conv_pool_clone = conv_pool.clone();
                            let events_clone = events.clone();
                            let secret_clone = secret.clone();
                            let week_clone = clock.iso_week.clone();
                            tokio::spawn(async move {
                                match crate::services::review_generator::generate_review_if_needed(
                                    &pool_clone,
                                    &conv_pool_clone,
                                    Some(events_clone.as_ref()),
                                    secret_clone.as_ref(),
                                )
                                .await
                                {
                                    Ok(true) => {
                                        tracing::info!(week = %week_clone, "调度器触发周复盘生成完成");
                                    }
                                    Ok(false) => {
                                        tracing::info!(week = %week_clone, "本周复盘已存在或被跳过");
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, week = %week_clone, "调度器触发周复盘生成失败");
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "记录周复盘触发状态失败（降级跳过本 tick）");
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "读取周复盘触发状态失败（降级跳过本 tick）");
                }
            }

            // Story 6.6: 周五大石头未完成检查（当天为周五且尚未检查过——
            // 去重改读 scheduler_triggers：Story 17.2；无 HH:MM 匹配，周五首个
            // tick 触发一次）
            match should_trigger_friday_check(&pool, &clock).await {
                Ok(true) => {
                    match record_trigger(
                        &pool,
                        JOB_BIGROCK_FRIDAY_CHECK,
                        SCOPE_GLOBAL,
                        &clock.date,
                        &clock.tz_offset,
                    )
                    .await
                    {
                        Ok(()) => {
                            let pool_clone = pool.clone();
                            let conv_pool_clone = conv_pool.clone();
                            let events_clone = events.clone();
                            let today_date = clock.date.clone();
                            tokio::spawn(async move {
                                match crate::services::bigrock_protection::check_friday_bigrock_status(
                                    &pool_clone,
                                    &conv_pool_clone,
                                    Some(events_clone.as_ref()),
                                )
                                .await
                                {
                                    Ok(true) => tracing::info!(date = %today_date, "周五大石头未完成检查已触发"),
                                    Ok(false) => tracing::info!(date = %today_date, "周五大石头检查跳过（无未完成或非周五）"),
                                    Err(e) => tracing::warn!(error = %e, date = %today_date, "周五大石头未完成检查失败"),
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "记录周五大石头检查触发状态失败（降级跳过本 tick）");
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "读取周五大石头检查触发状态失败（降级跳过本 tick）");
                }
            }

            interval.tick().await;
        }
    });
}

/// 读取周复盘时间配置（Story 6.4 实现触发逻辑）
pub async fn get_review_schedule(pool: &SqlitePool) -> Result<(String, String), AppError> {
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

    /// 测试用 SecretStore 桩：该测试走「无默认 provider → 降级空建议」路径，
    /// 不触达密钥读取，仅满足接缝签名（load 恒返回 None）。
    struct TestSecretStore;

    impl crate::services::secret_store::SecretStore for TestSecretStore {
        fn save_secret(&self, _key: &str, _value: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn load_secret(&self, _key: &str) -> Result<Option<String>, AppError> {
            Ok(None)
        }
        fn delete_secret(&self, _key: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

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

        let result = run_work_loop_for_role(&pool, &conv_pool, &role, None, &TestSecretStore).await;
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

    // ---- Story 17.2（FR-47）：触发判定表驱动测试 ----
    // 判定逻辑的可注入时钟在测试内构造（运行态由循环内 Local::now() 构造）；
    // 期望值的 tz 维度一律从同一 datetime 派生，测试不依赖本机时区。

    async fn setup_triggers_db() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE scheduler_triggers (
                job TEXT NOT NULL,
                scope TEXT NOT NULL,
                cycle TEXT NOT NULL,
                tz_offset TEXT NOT NULL,
                last_triggered_at TEXT NOT NULL,
                trigger_count INTEGER NOT NULL DEFAULT 1,
                UNIQUE(job, scope, tz_offset)
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create scheduler_triggers table");

        pool
    }

    fn local_dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::DateTime<Local> {
        chrono::NaiveDate::from_ymd_opt(y, m, d)
            .expect("valid date")
            .and_hms_opt(h, min, 0)
            .expect("valid time")
            .and_local_timezone(Local)
            .unwrap()
    }

    #[tokio::test]
    async fn work_loop_same_minute_restart_does_not_retrigger() {
        // FR-47 验收：简报/工作循环触发后同分钟重启，循环再跑不再触发当日该时刻
        let pool = setup_triggers_db().await;
        let now = local_dt(2026, 9, 21, 9, 0);
        let clock = TriggerClock::from_now(&now);
        let times = vec!["09:00".to_string(), "14:00".to_string()];

        // 首个周期：未触发过 ⇒ 可触发
        assert!(should_trigger_work_loop(&pool, &clock, "role-1", &times)
            .await
            .unwrap());
        record_trigger(&pool, JOB_WORK_LOOP, "role-1", &clock.work_loop_cycle(), &clock.tz_offset)
            .await
            .unwrap();

        // 同分钟重启（进程内内存态已失忆，表仍在）：同 cycle ⇒ 不再触发
        assert!(!should_trigger_work_loop(&pool, &clock, "role-1", &times)
            .await
            .unwrap());

        // 同日不同时刻（下一触发点 14:00）照常触发——cycle 含 HH:MM，非按日一刀切
        let afternoon = local_dt(2026, 9, 21, 14, 0);
        let afternoon_clock = TriggerClock::from_now(&afternoon);
        assert!(should_trigger_work_loop(&pool, &afternoon_clock, "role-1", &times)
            .await
            .unwrap());

        // 跨天同时刻照常触发
        let tomorrow = local_dt(2026, 9, 22, 9, 0);
        let tomorrow_clock = TriggerClock::from_now(&tomorrow);
        assert!(should_trigger_work_loop(&pool, &tomorrow_clock, "role-1", &times)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn work_loop_skips_when_downtime_crossed_planned_time() {
        // FR-47 验收：停机跨过计划时刻 ⇒ 跳过不补发（诚实代价）
        let pool = setup_triggers_db().await;
        // 09:00 计划时刻停机错过，09:05 恢复
        let now = local_dt(2026, 9, 21, 9, 5);
        let clock = TriggerClock::from_now(&now);
        let times = vec!["09:00".to_string()];

        assert!(!should_trigger_work_loop(&pool, &clock, "role-1", &times)
            .await
            .unwrap(), "分钟级精确匹配 ⇒ 错过即跳过，不补发不堆积");
    }

    #[tokio::test]
    async fn briefing_same_minute_restart_does_not_retrigger() {
        let pool = setup_triggers_db().await;
        let now = local_dt(2026, 9, 21, 8, 0);
        let clock = TriggerClock::from_now(&now);

        assert!(should_trigger_briefing(&pool, &clock, "08:00").await.unwrap());
        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, &clock.date, &clock.tz_offset)
            .await
            .unwrap();

        // 同分钟重启：当日已触发 ⇒ 跳过
        assert!(!should_trigger_briefing(&pool, &clock, "08:00").await.unwrap());

        // 当天晚些时候（简报时间已过）不因去重放开而补发
        let later = local_dt(2026, 9, 21, 10, 30);
        let later_clock = TriggerClock::from_now(&later);
        assert!(!should_trigger_briefing(&pool, &later_clock, "08:00").await.unwrap());
    }

    #[tokio::test]
    async fn briefing_skips_when_downtime_crossed_planned_time() {
        // 昨日已触发（表行 cycle=昨日），今日 08:00 停机错过、09:00 恢复 ⇒ 不补发
        let pool = setup_triggers_db().await;
        let yesterday = local_dt(2026, 9, 20, 8, 0);
        let yesterday_clock = TriggerClock::from_now(&yesterday);
        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, &yesterday_clock.date, &yesterday_clock.tz_offset)
            .await
            .unwrap();

        let now = local_dt(2026, 9, 21, 9, 0);
        let clock = TriggerClock::from_now(&now);
        assert!(!should_trigger_briefing(&pool, &clock, "08:00")
            .await
            .unwrap(), "HH:MM 不匹配 ⇒ 跳过（即便当日去重行不存在）");
    }

    #[tokio::test]
    async fn tz_change_allows_at_most_one_retrigger() {
        // FR-47 验收：TZ env 变更后首个周期至多一次跳过或重复（键含 tz 维度）
        let pool = setup_triggers_db().await;
        let now = local_dt(2026, 9, 21, 8, 0);
        let clock = TriggerClock::from_now(&now);

        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, &clock.date, &clock.tz_offset)
            .await
            .unwrap();
        assert!(!should_trigger_briefing(&pool, &clock, "08:00").await.unwrap());

        // TZ 变更（或 DST 偏移变化）⇒ tz_offset 键变化 ⇒ 新键无行 ⇒ 判定可触发
        //（"至多一次重复"的来源；此后新键行接管去重，不再累积重复）
        let changed_tz = if clock.tz_offset == "+08:00" { "+00:00" } else { "+08:00" };
        let clock_after_tz_change = TriggerClock {
            tz_offset: changed_tz.to_string(),
            ..clock.clone()
        };
        assert!(should_trigger_briefing(&pool, &clock_after_tz_change, "08:00")
            .await
            .unwrap());
        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, &clock_after_tz_change.date, &clock_after_tz_change.tz_offset)
            .await
            .unwrap();
        assert!(!should_trigger_briefing(&pool, &clock_after_tz_change, "08:00")
            .await
            .unwrap(), "新键置位后同周期不再重复");
    }

    #[tokio::test]
    async fn weekly_job_dedupes_within_iso_week_and_triggers_next_week() {
        let pool = setup_triggers_db().await;
        // 2026-09-21 是周一
        let monday = local_dt(2026, 9, 21, 20, 0);
        assert_eq!(monday.weekday().num_days_from_monday() + 1, 1);
        let clock = TriggerClock::from_now(&monday);

        assert!(should_trigger_weekly(&pool, &clock, JOB_WEEKLY_REVIEW, "1", "20:00")
            .await
            .unwrap());
        record_trigger(&pool, JOB_WEEKLY_REVIEW, SCOPE_GLOBAL, &clock.iso_week, &clock.tz_offset)
            .await
            .unwrap();

        // 同周一同星期+同时刻再次判定：去重分支真路径（星期/时刻门均放行，
        // 仅 cycle 已置位——FR-47「同分钟重启不重复触发」的判定级锚点；
        // 评审修复：原周三断言被星期门短路，去重分支零执行）
        assert!(!should_trigger_weekly(&pool, &clock, JOB_WEEKLY_REVIEW, "1", "20:00")
            .await
            .unwrap(), "同 ISO 周内同星期+时刻（cycle 已置位）不再触发");
        // 非配置星期不触发（星期门短路，与去重无关）
        let wednesday = local_dt(2026, 9, 23, 20, 0);
        let wednesday_clock = TriggerClock::from_now(&wednesday);
        assert!(!should_trigger_weekly(&pool, &wednesday_clock, JOB_WEEKLY_REVIEW, "1", "20:00")
            .await
            .unwrap(), "非配置星期不触发");

        // 次周同一时刻照常触发
        let next_monday = local_dt(2026, 9, 28, 20, 0);
        let next_clock = TriggerClock::from_now(&next_monday);
        assert!(should_trigger_weekly(&pool, &next_clock, JOB_WEEKLY_REVIEW, "1", "20:00")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn weekly_job_requires_both_day_and_time_match() {
        let pool = setup_triggers_db().await;
        let monday = local_dt(2026, 9, 21, 20, 0);
        let clock = TriggerClock::from_now(&monday);

        // 星期不匹配
        assert!(!should_trigger_weekly(&pool, &clock, JOB_BIGROCK_PLANNING, "2", "20:00")
            .await
            .unwrap());
        // 时刻不匹配
        assert!(!should_trigger_weekly(&pool, &clock, JOB_BIGROCK_PLANNING, "1", "09:00")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn friday_check_dedupes_per_day() {
        let pool = setup_triggers_db().await;
        // 2026-09-25 是周五
        let friday = local_dt(2026, 9, 25, 10, 0);
        assert_eq!(friday.weekday().num_days_from_monday() + 1, 5);
        let clock = TriggerClock::from_now(&friday);

        assert!(should_trigger_friday_check(&pool, &clock).await.unwrap());
        record_trigger(&pool, JOB_BIGROCK_FRIDAY_CHECK, SCOPE_GLOBAL, &clock.date, &clock.tz_offset)
            .await
            .unwrap();
        assert!(!should_trigger_friday_check(&pool, &clock).await.unwrap());

        // 非周五不触发
        let monday = local_dt(2026, 9, 21, 10, 0);
        let monday_clock = TriggerClock::from_now(&monday);
        assert!(!should_trigger_friday_check(&pool, &monday_clock).await.unwrap());
    }

    #[test]
    fn trigger_clock_derives_all_dimensions_consistently() {
        let now = local_dt(2026, 9, 21, 9, 5);
        let clock = TriggerClock::from_now(&now);
        assert_eq!(clock.hhmm, "09:05");
        assert_eq!(clock.date, "2026-09-21");
        assert_eq!(clock.iso_week, iso_week_key(&now));
        assert_eq!(clock.tz_offset, now.format("%:z").to_string());
        assert_eq!(clock.weekday, 1);
        assert_eq!(clock.work_loop_cycle(), "2026-09-21 09:05");
    }
}
