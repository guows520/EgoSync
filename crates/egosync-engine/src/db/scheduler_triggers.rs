//! Story 17.2（FR-47）：调度触发去重统一存取层 —— `scheduler_triggers` 表唯一入口。
//!
//! 职责：
//! - 收纳调度器全部触发判定（原五处循环局部内存去重：last_triggered_map /
//!   last_briefing_trigger_date / last_bigrock_trigger_week /
//!   last_review_trigger_week / last_bigrock_friday_check_date）——重启不失忆
//! - `has_triggered` 判定（cycle 相同即本周期已触发）；`record_trigger` 置位
//!   （spawn 前调用，LLM 瞬时失败当天烧掉槽位不重试——deferred-work #225 语义）
//! - count 按 job 语义：bigrock_protection 跨 cycle 递增累计（对齐旧
//!   big_rock_protection_reminders 表的 count++），其余 job 恒 1；同 cycle 重复
//!   调用幂等（计数不变）
//! - `prune_work_loop_rows` 仅清理已归档/删除角色的 work_loop 行（对齐既有
//!   retain 语义；勿伤 bigrock 的 task 作用域行）
//!
//! 时间源三分表（架构 ⑨ 冻结）在本模块的落点：
//! - 持久化：`last_triggered_at` 一律 `chrono_now_pub` 的 UTC RFC3339 串（禁 Local）；
//! - 调度判定：`cycle` / `tz_offset` 由调用方从容器 `Local::now()` 派生后传入
//!   （本模块不做任何时区推断——判定语义与 chrono::Local 单一来源绑定）；
//! - 前端渲染：不涉（浏览器 TZ，组件不动）。

use sqlx::SqlitePool;

use crate::error::AppError;

/// 工作循环（每角色，scope = role_id，cycle = "YYYY-MM-DD HH:MM"）
pub const JOB_WORK_LOOP: &str = "work_loop";
/// 晨间简报（全局，cycle = "YYYY-MM-DD"）
pub const JOB_BRIEFING: &str = "briefing";
/// 大石头规划提醒（全局，cycle = ISO 周 "YYYY-Www"）
pub const JOB_BIGROCK_PLANNING: &str = "bigrock_planning";
/// 周复盘（全局，cycle = ISO 周 "YYYY-Www"）
pub const JOB_WEEKLY_REVIEW: &str = "weekly_review";
/// 周五大石头未完成检查（全局，cycle = "YYYY-MM-DD"）
pub const JOB_BIGROCK_FRIDAY_CHECK: &str = "bigrock_friday_check";
/// 大石头保护提醒（每任务，scope = task_id，cycle = "YYYY-MM-DD"——
/// 替换迁移自 big_rock_protection_reminders，count++ 语义保留）
pub const JOB_BIGROCK_PROTECTION: &str = "bigrock_protection";

/// 全局作用域（非角色/任务维度的单例 job 统一用此 scope）
pub const SCOPE_GLOBAL: &str = "global";

/// 一行触发状态：`(job, scope, tz_offset)` 唯一键 + 最新周期信息。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, sqlx::FromRow)]
pub struct SchedulerTriggerRow {
    pub job: String,
    pub scope: String,
    pub cycle: String,
    pub tz_offset: String,
    pub last_triggered_at: String,
    pub trigger_count: i64,
}

/// 触发判定：该 `(job, scope, tz_offset)` 键在当前 cycle 是否已触发。
///
/// 行缺失（从未触发过或 TZ 变更后新键）或行内 cycle 落后于当前 ⇒ `Ok(false)`
/// （本周期可触发）；cycle 相同 ⇒ `Ok(true)`（本周期已触发）。
/// 判定只比较 cycle，不读 `last_triggered_at`——时间戳仅供排查与导出。
pub async fn has_triggered(
    pool: &SqlitePool,
    job: &str,
    scope: &str,
    cycle: &str,
    tz_offset: &str,
) -> Result<bool, AppError> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT cycle FROM scheduler_triggers
         WHERE job = ?1 AND scope = ?2 AND tz_offset = ?3",
    )
    .bind(job)
    .bind(scope)
    .bind(tz_offset)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询调度触发状态失败: {}", e)))?;

    Ok(existing.as_deref() == Some(cycle))
}

/// 置位（记录触发）：upsert 前移 cycle 与 `last_triggered_at`（UTC RFC3339）。
///
/// count 按 job 语义：
/// - `bigrock_protection`：跨 cycle 递增累计（对齐旧表 count++）；
/// - 其余 job：恒 1（单周期内至多触发一次，跨周期无需计数）；
/// - 同 cycle 重复调用幂等：计数不变（仅刷新 last_triggered_at）。
///
/// 调用时机保持 spawn 前置位——生成的瞬时失败不回滚本记录（当天烧掉槽位），
/// 生成侧另有 UNIQUE / 近 7 天标题查重兜底防重复落库。
pub async fn record_trigger(
    pool: &SqlitePool,
    job: &str,
    scope: &str,
    cycle: &str,
    tz_offset: &str,
) -> Result<(), AppError> {
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO scheduler_triggers (job, scope, cycle, tz_offset, last_triggered_at, trigger_count)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)
         ON CONFLICT(job, scope, tz_offset) DO UPDATE SET
             last_triggered_at = excluded.last_triggered_at,
             trigger_count = CASE
                 WHEN scheduler_triggers.cycle = excluded.cycle THEN scheduler_triggers.trigger_count
                 WHEN scheduler_triggers.job = ?6 THEN scheduler_triggers.trigger_count + 1
                 ELSE 1
             END,
             cycle = excluded.cycle",
    )
    .bind(job)
    .bind(scope)
    .bind(cycle)
    .bind(tz_offset)
    .bind(&now)
    .bind(JOB_BIGROCK_PROTECTION)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("记录调度触发状态失败: {}", e)))?;

    Ok(())
}

/// 查询一行触发状态（测试与诊断用；判定路径请走 `has_triggered`）。
pub async fn get_trigger(
    pool: &SqlitePool,
    job: &str,
    scope: &str,
    tz_offset: &str,
) -> Result<Option<SchedulerTriggerRow>, AppError> {
    sqlx::query_as::<_, SchedulerTriggerRow>(
        "SELECT job, scope, cycle, tz_offset, last_triggered_at, trigger_count
         FROM scheduler_triggers
         WHERE job = ?1 AND scope = ?2 AND tz_offset = ?3",
    )
    .bind(job)
    .bind(scope)
    .bind(tz_offset)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询调度触发状态失败: {}", e)))
}

/// 删除指定 `(job, scope)` 的全部触发行（含所有 tz 维度行）。
///
/// 任务删除/完成时清理 bigrock 保护记录的落点（对齐旧表
/// `delete_reminder_for_task` 语义）。删除失败由调用方 warn 不阻断。
pub async fn delete_trigger(
    pool: &SqlitePool,
    job: &str,
    scope: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM scheduler_triggers WHERE job = ?1 AND scope = ?2")
        .bind(job)
        .bind(scope)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除调度触发状态失败: {}", e)))?;

    Ok(result.rows_affected())
}

/// 清理已归档/删除角色的 work_loop 触发行（对齐调度循环原 `retain` 语义）。
///
/// 仅作用于 `job = 'work_loop'`——bigrock 等任务作用域行不经此路径
/// （由任务删除时 `delete_trigger` 显式清理），勿伤。
/// 活跃角色集为空时清空全部 work_loop 行（同原 retain 对空集的语义）。
pub async fn prune_work_loop_rows(
    pool: &SqlitePool,
    active_role_ids: &[String],
) -> Result<u64, AppError> {
    if active_role_ids.is_empty() {
        let result = sqlx::query("DELETE FROM scheduler_triggers WHERE job = ?1")
            .bind(JOB_WORK_LOOP)
            .execute(pool)
            .await
            .map_err(|e| AppError::DbError(format!("清理工作循环触发状态失败: {}", e)))?;
        return Ok(result.rows_affected());
    }

    let placeholders = (1..=active_role_ids.len())
        .map(|i| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "DELETE FROM scheduler_triggers WHERE job = ?1 AND scope NOT IN ({})",
        placeholders
    );

    let mut query = sqlx::query(&sql).bind(JOB_WORK_LOOP);
    for id in active_role_ids {
        query = query.bind(id);
    }
    let result = query
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("清理工作循环触发状态失败: {}", e)))?;

    Ok(result.rows_affected())
}

/// bigrock 保护提醒导出快照：每 task 一行（多 tz 行时取 `last_triggered_at`
/// 最新一条），供 data_export 以旧 JSON 形状（`bigRockProtectionReminders` 段）
/// 合成导出——表已替换迁移，导出形状不变以保跨形态互导。
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct BigrockProtectionSnapshot {
    pub scope: String,
    pub trigger_count: i64,
    pub last_triggered_at: String,
}

/// 列出 bigrock 保护提醒的导出快照（每 scope 取 `last_triggered_at` 最新行，
/// 并列时以 tz_offset 字典序定序——保证确定性）。
pub async fn list_bigrock_protection_latest(
    pool: &SqlitePool,
) -> Result<Vec<BigrockProtectionSnapshot>, AppError> {
    sqlx::query_as::<_, BigrockProtectionSnapshot>(
        "SELECT s.scope, s.trigger_count, s.last_triggered_at
         FROM scheduler_triggers s
         WHERE s.job = ?1
           AND NOT EXISTS (
               SELECT 1 FROM scheduler_triggers t
               WHERE t.job = ?1 AND t.scope = s.scope
                 AND (t.last_triggered_at > s.last_triggered_at
                      OR (t.last_triggered_at = s.last_triggered_at AND t.tz_offset > s.tz_offset))
           )
         ORDER BY s.scope ASC",
    )
    .bind(JOB_BIGROCK_PROTECTION)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询大石头保护提醒快照失败: {}", e)))
}

/// 导入落点：以导出包数据直写统一表（权威覆写——cycle / count 均以包内值为准，
/// 与 `record_trigger` 的增量语义不同）。cycle 由 `last_reminded_at` 按当前
/// Local 折算（见 `local_date_from_utc`）。
///
/// 接受任意 `Sqlite` executor（连接池或事务）——data_import 在主库事务内调用。
pub async fn upsert_imported_row<'e, E>(
    executor: E,
    job: &str,
    scope: &str,
    cycle: &str,
    tz_offset: &str,
    last_triggered_at: &str,
    trigger_count: i64,
) -> Result<(), AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT INTO scheduler_triggers (job, scope, cycle, tz_offset, last_triggered_at, trigger_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(job, scope, tz_offset) DO UPDATE SET
             cycle = excluded.cycle,
             last_triggered_at = excluded.last_triggered_at,
             trigger_count = excluded.trigger_count",
    )
    .bind(job)
    .bind(scope)
    .bind(cycle)
    .bind(tz_offset)
    .bind(last_triggered_at)
    .bind(trigger_count)
    .execute(executor)
    .await
    .map_err(|e| AppError::DbError(format!("导入调度触发状态失败: {}", e)))?;

    Ok(())
}

/// 将 UTC 时间戳（RFC3339 或 `"%Y-%m-%dT%H:%M:%SZ"`）解析为 Local 日期。
///
/// 供 bigrock 保护提醒的 cycle 派生（旧表 `last_reminded_at` 与统一表
/// `last_triggered_at` 均为 UTC RFC3339，cycle 语义 = 该时刻的 Local 日期）。
/// 无法解析返回 `None`（调用方决定回退语义——旧表"解析失败=今日未提醒"）。
pub fn parse_utc_to_local_date(s: &str) -> Option<chrono::NaiveDate> {
    let utc = chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
                .map(|dt| {
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                })
        })
        .ok()?;
    Some(utc.with_timezone(&chrono::Local).date_naive())
}

/// 将 UTC 时间戳折算为 Local 日期串（"YYYY-MM-DD"）；无法解析返回空串——
/// 空串永不匹配任何真实日期，即导入脏数据后视为"今日未提醒"（保持旧表
/// `is_reminded_today` 对解析失败的降级行为）。
pub fn local_date_from_utc(s: &str) -> String {
    parse_utc_to_local_date(s)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
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

    #[tokio::test]
    async fn has_triggered_returns_false_when_row_absent() {
        let pool = setup_test_db().await;
        assert!(!has_triggered(&pool, JOB_WORK_LOOP, "role-1", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn record_then_has_triggered_same_cycle_is_true() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_WORK_LOOP, "role-1", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap();
        assert!(has_triggered(&pool, JOB_WORK_LOOP, "role-1", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn has_triggered_false_when_cycle_advances() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_WORK_LOOP, "role-1", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap();
        // 同日不同时刻（下一触发点）照常触发
        assert!(!has_triggered(&pool, JOB_WORK_LOOP, "role-1", "2026-09-21 14:00", "+08:00")
            .await
            .unwrap());
        // 跨天照常触发
        assert!(!has_triggered(&pool, JOB_WORK_LOOP, "role-1", "2026-09-22 09:00", "+08:00")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn tz_offset_is_key_dimension() {
        // TZ 变更（或 DST 偏移变化）后首个周期：新键无行 ⇒ 判定未触发
        // （"至多一次重复"的键语义来源）
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, "2026-09-21", "+08:00")
            .await
            .unwrap();
        assert!(has_triggered(&pool, JOB_BRIEFING, SCOPE_GLOBAL, "2026-09-21", "+08:00")
            .await
            .unwrap());
        assert!(!has_triggered(&pool, JOB_BRIEFING, SCOPE_GLOBAL, "2026-09-21", "+00:00")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn same_cycle_record_is_idempotent_for_count() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();

        let row = get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "+08:00")
            .await
            .unwrap()
            .expect("row exists");
        assert_eq!(row.trigger_count, 1, "同 cycle 重复置位不得递增计数");
    }

    #[tokio::test]
    async fn bigrock_protection_count_increments_across_cycles() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-22", "+08:00")
            .await
            .unwrap();

        let row = get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "+08:00")
            .await
            .unwrap()
            .expect("row exists");
        assert_eq!(row.trigger_count, 2, "bigrock_protection 跨 cycle 递增累计（对齐旧表 count++）");
        assert_eq!(row.cycle, "2026-09-22", "行存最新 cycle");
    }

    #[tokio::test]
    async fn non_bigrock_count_stays_one_across_cycles() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, "2026-09-20", "+08:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, "2026-09-21", "+08:00")
            .await
            .unwrap();

        let row = get_trigger(&pool, JOB_BRIEFING, SCOPE_GLOBAL, "+08:00")
            .await
            .unwrap()
            .expect("row exists");
        assert_eq!(row.trigger_count, 1, "非 bigrock job 计数恒 1");
    }

    #[tokio::test]
    async fn delete_trigger_removes_all_tz_rows_for_scope_only() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+00:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-2", "2026-09-21", "+08:00")
            .await
            .unwrap();

        let deleted = delete_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1")
            .await
            .unwrap();
        assert_eq!(deleted, 2, "同 scope 的全部 tz 维度行一并删除");

        assert!(get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "+08:00")
            .await
            .unwrap()
            .is_none());
        assert!(get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-2", "+08:00")
            .await
            .unwrap()
            .is_some(), "其他 scope 不受影响");
    }

    #[tokio::test]
    async fn prune_removes_inactive_work_loop_rows_only() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_WORK_LOOP, "role-active", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_WORK_LOOP, "role-archived", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap();
        // bigrock 的 task 作用域行不得被 prune 误伤
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();

        let active = vec!["role-active".to_string()];
        let deleted = prune_work_loop_rows(&pool, &active).await.unwrap();
        assert_eq!(deleted, 1);

        assert!(get_trigger(&pool, JOB_WORK_LOOP, "role-active", "+08:00")
            .await
            .unwrap()
            .is_some());
        assert!(get_trigger(&pool, JOB_WORK_LOOP, "role-archived", "+08:00")
            .await
            .unwrap()
            .is_none());
        assert!(get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "+08:00")
            .await
            .unwrap()
            .is_some(), "prune 仅 work_loop 行，勿伤 bigrock task 作用域行");
    }

    #[tokio::test]
    async fn prune_with_empty_active_set_clears_all_work_loop_rows() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_WORK_LOOP, "role-1", "2026-09-21 09:00", "+08:00")
            .await
            .unwrap();
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();

        let deleted = prune_work_loop_rows(&pool, &[]).await.unwrap();
        assert_eq!(deleted, 1, "活跃角色集为空 ⇒ 清空全部 work_loop 行（原 retain 语义）");
        assert!(get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "+08:00")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn list_bigrock_protection_latest_picks_newest_row_per_scope() {
        let pool = setup_test_db().await;
        // task-1 两条 tz 行（模拟 TZ 变更历史）——取 last_triggered_at 最新
        sqlx::query(
            "INSERT INTO scheduler_triggers (job, scope, cycle, tz_offset, last_triggered_at, trigger_count)
             VALUES
                ('bigrock_protection', 'task-1', '2026-09-20', '+08:00', '2026-09-20T01:00:00Z', 3),
                ('bigrock_protection', 'task-1', '2026-09-21', '+00:00', '2026-09-21T01:00:00Z', 1),
                ('bigrock_protection', 'task-2', '2026-09-19', '+08:00', '2026-09-19T01:00:00Z', 7)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let snapshots = list_bigrock_protection_latest(&pool).await.unwrap();
        assert_eq!(snapshots.len(), 2, "每 scope 恰一行（保持旧表 UNIQUE(task_id) 的单行导出形状）");
        assert_eq!(snapshots[0].scope, "task-1");
        assert_eq!(snapshots[0].last_triggered_at, "2026-09-21T01:00:00Z");
        assert_eq!(snapshots[0].trigger_count, 1);
        assert_eq!(snapshots[1].scope, "task-2");
        assert_eq!(snapshots[1].trigger_count, 7);
    }

    #[tokio::test]
    async fn upsert_imported_row_overwrites_authoritatively() {
        let pool = setup_test_db().await;
        record_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "2026-09-21", "+08:00")
            .await
            .unwrap();

        upsert_imported_row(
            &pool,
            JOB_BIGROCK_PROTECTION,
            "task-1",
            "2026-01-01",
            "+08:00",
            "2026-01-01T00:00:00Z",
            5,
        )
        .await
        .unwrap();

        let row = get_trigger(&pool, JOB_BIGROCK_PROTECTION, "task-1", "+08:00")
            .await
            .unwrap()
            .expect("row exists");
        assert_eq!(row.cycle, "2026-01-01");
        assert_eq!(row.last_triggered_at, "2026-01-01T00:00:00Z");
        assert_eq!(row.trigger_count, 5, "导入为权威覆写，count 以包内值为准");
    }

    #[test]
    fn parse_utc_to_local_date_parses_rfc3339() {
        let date = parse_utc_to_local_date("2026-06-22T12:30:00Z");
        assert!(date.is_some());
        // UTC 12:30 在任何整半点时区折算均为同一天（东八区 20:30 / UTC-8 04:30）
        assert_eq!(date.unwrap().format("%Y-%m-%d").to_string(), "2026-06-22");
    }

    #[test]
    fn parse_utc_to_local_date_returns_none_for_invalid() {
        assert!(parse_utc_to_local_date("not-a-date").is_none());
        assert!(parse_utc_to_local_date("").is_none());
    }

    #[test]
    fn local_date_from_utc_falls_back_to_empty_string() {
        assert_eq!(local_date_from_utc("invalid"), "");
        // 合法值折算为 Local 日期串（空串永不匹配真实日期 ⇒ 视为未提醒）
        assert!(!local_date_from_utc("2026-06-22T12:30:00Z").is_empty());
    }

    #[test]
    fn job_constants_are_stable_strings() {
        // job 值入库持久化 + migration 034 数据迁移依赖 'bigrock_protection' 字面量——
        // 变更即破坏既有库兼容，钉死。
        assert_eq!(JOB_WORK_LOOP, "work_loop");
        assert_eq!(JOB_BRIEFING, "briefing");
        assert_eq!(JOB_BIGROCK_PLANNING, "bigrock_planning");
        assert_eq!(JOB_WEEKLY_REVIEW, "weekly_review");
        assert_eq!(JOB_BIGROCK_FRIDAY_CHECK, "bigrock_friday_check");
        assert_eq!(JOB_BIGROCK_PROTECTION, "bigrock_protection");
        assert_eq!(SCOPE_GLOBAL, "global");
    }
}
