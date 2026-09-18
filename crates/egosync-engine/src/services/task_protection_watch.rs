//! Story 3.5: 每小时 Q2 保护检查后台任务
//!
//! 职责：
//! - 在 Tauri 启动时 spawn 一个后台 tokio task，内部使用 `tokio::interval(3600s)`
//! - 每次 tick 对所有活跃任务执行幂等重算：
//!   - 先 `clear_protection_for_resolved`：把不再符合 at_risk 的任务（非 Q2 / 已完成 / 近期处理过）清回 `normal`
//!   - 再 `mark_stale_q2_at_risk`：把连续过期未处理的 Q2 任务标记为 `at_risk`
//! - 错误只 `tracing::warn!`，绝不 panic，绝不阻塞 Tauri 启动
//!
//! 关键正确性：标记 / 清除都**不刷新 `updated_at`**（保护状态由 `updated_at` 距今天数派生）。
//! 单测通过直接调用 `recompute_protection_status` 验证逻辑，无需等待一小时。

use sqlx::SqlitePool;

use crate::db;
use crate::error::AppError;

/// Q2 任务连续未处理多少天后标记为 `at_risk`。
pub const AT_RISK_DAYS: i64 = 3;

/// 对所有活跃任务执行一次幂等的保护状态重算。返回本次被标记为 `at_risk` 的任务数量。
///
/// 顺序：先 `clear_protection_for_resolved`（恢复已解除的），再 `mark_stale_q2_at_risk`（标记过期的）。
/// 两步命中集互不重叠，但固定顺序便于测试推理。
pub async fn recompute_protection_status(pool: &SqlitePool) -> Result<u64, AppError> {
    let threshold = compute_at_risk_threshold();
    let cleared = db::tasks::clear_protection_for_resolved(pool, &threshold).await?;
    let marked = db::tasks::mark_stale_q2_at_risk(pool, &threshold).await?;

    if marked > 0 || cleared > 0 {
        tracing::info!(marked, cleared, "Q2 保护检查完成（mark/clear）");
    }
    Ok(marked)
}

/// 计算 `now - AT_RISK_DAYS` 的完整 ISO 8601 时间戳（`YYYY-MM-DDTHH:MM:SSZ`）。
/// 与 `db::settings::chrono_now_pub()` 同款格式，可与 `updated_at` 直接字符串比较。
fn compute_at_risk_threshold() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let threshold_secs = now_secs - AT_RISK_DAYS * 86400;
    let threshold_secs = threshold_secs.max(0);
    let days = threshold_secs / 86400;
    let rem = threshold_secs % 86400;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;
    let (y, m, d) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// 启动每小时 Q2 保护检查后台任务。在 Tauri `setup` 中调用。
///
/// Story 15.2 接缝四：调用方（桌面壳 setup 同步上下文）注入宿主
/// runtime Handle 派生任务——裸 tokio::spawn 在无 reactor 上下文会
/// panic（v0.1.6-alpha.1 历史事故）。
///
/// 首次 tick 立即执行（便于启动后快速检查），之后每 3600 秒一次。错误只 warn，不 panic。
pub fn spawn_hourly_watch(pool: SqlitePool, handle: tokio::runtime::Handle) {
    handle.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        // 首次 tick 立即返回
        interval.tick().await;

        loop {
            if let Err(e) = recompute_protection_status(&pool).await {
                tracing::warn!(error = %e, "Q2 保护检查后台任务出错（降级继续）");
            }
            interval.tick().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_at_risk_threshold_is_full_iso_timestamp() {
        let threshold = compute_at_risk_threshold();
        // 格式 YYYY-MM-DDTHH:MM:SSZ，长度 20
        assert_eq!(threshold.len(), 20, "时间戳长度应为 20，得到 {}", threshold);
        assert_eq!(threshold.chars().nth(4), Some('-'));
        assert_eq!(threshold.chars().nth(7), Some('-'));
        assert_eq!(threshold.chars().nth(10), Some('T'));
        assert_eq!(threshold.chars().nth(13), Some(':'));
        assert_eq!(threshold.chars().nth(16), Some(':'));
        assert_eq!(threshold.chars().nth(19), Some('Z'));
    }

    #[test]
    fn days_to_ymd_handles_epoch_origin() {
        // 1970-01-01 对应 Unix days = 0
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    // ---- Story 15.1：自 db/tasks.rs 迁入的端到端重算测试 ----
    // 原测试位于引擎 crate 的 db/tasks.rs 内联测试中，因调用本模块（桌面壳留守
    // service）而随 Story 15.1 迁移边界迁至本文件，断言与辅助函数保持原样。

    async fn setup_protection_test_db() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
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

        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-a', '产品')")
            .execute(&pool)
            .await
            .expect("failed to insert role-a");
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-b', '学习')")
            .execute(&pool)
            .await
            .expect("failed to insert role-b");

        sqlx::query(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY NOT NULL,
                owner_type TEXT NOT NULL DEFAULT 'role' CHECK (owner_type IN ('role', 'butler')),
                role_id TEXT,
                title TEXT NOT NULL,
                deadline TEXT,
                quadrant TEXT NOT NULL DEFAULT 'Q2' CHECK (quadrant IN ('Q1', 'Q2', 'Q3', 'Q4')),
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
                deleted_at TEXT,
                CHECK ((owner_type = 'role' AND role_id IS NOT NULL) OR (owner_type = 'butler' AND role_id IS NULL)),
                FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create tasks table");

        pool
    }

    /// 直接 INSERT 一条任务，精确控制 quadrant / is_completed / protection_status / updated_at，
    /// 避免依赖真实时钟与 create_task 的默认值。
    #[allow(clippy::too_many_arguments)]
    async fn insert_protection_task(
        pool: &SqlitePool,
        id: &str,
        quadrant: &str,
        is_completed: bool,
        protection_status: &str,
        updated_at: &str,
    ) {
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, protection_status, created_at, updated_at)
             VALUES (?1, 'role', 'role-a', ?2, ?3, ?4, ?5, '2026-01-01T00:00:00Z', ?6)",
        )
        .bind(id)
        .bind(format!("任务 {}", id))
        .bind(quadrant)
        .bind(is_completed as i32)
        .bind(protection_status)
        .bind(updated_at)
        .execute(pool)
        .await
        .expect("insert protection task");
    }

    async fn protection_status_of(pool: &SqlitePool, id: &str) -> String {
        sqlx::query_scalar::<_, String>("SELECT protection_status FROM tasks WHERE id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("fetch protection_status")
    }

    #[tokio::test]
    async fn recompute_protection_status_clears_then_marks_in_one_pass() {
        // 端到端验证 service 层 recompute_protection_status 在单次调用内
        // 先 clear（恢复已解除的）再 mark（标记过期的），且组合幂等：
        // 已是 at_risk 且仍过期的任务保持不变、不重复计入返回值。
        // 使用 2020/2099 这类远离 now±3天 阈值的固定时间戳，避免依赖真实时钟。
        let pool = setup_protection_test_db().await;
        // 过期、未完成、Q2、当前 normal → 应被 mark 为 at_risk（计入返回值）
        insert_protection_task(&pool, "q2-stale-normal", "Q2", false, "normal", "2020-01-01T00:00:00Z").await;
        // 过期、未完成、Q2、当前已是 at_risk → 保持 at_risk，不被 clear，不重复计数
        insert_protection_task(&pool, "q2-stale-at-risk", "Q2", false, "at_risk", "2020-01-01T00:00:00Z").await;
        // at_risk 的 Q1（已非 Q2）→ 应被 clear 回 normal
        insert_protection_task(&pool, "q1-at-risk", "Q1", false, "at_risk", "2020-01-01T00:00:00Z").await;
        // 近期 Q2 normal → 不动
        insert_protection_task(&pool, "q2-recent", "Q2", false, "normal", "2099-01-01T00:00:00Z").await;

        let marked = recompute_protection_status(&pool)
            .await
            .expect("recompute protection status");

        assert_eq!(marked, 1, "仅本次新标记的过期 Q2 计入返回值（已 at_risk 的不重复计数）");
        assert_eq!(protection_status_of(&pool, "q2-stale-normal").await, "at_risk");
        assert_eq!(protection_status_of(&pool, "q2-stale-at-risk").await, "at_risk", "仍过期保持 at_risk");
        assert_eq!(protection_status_of(&pool, "q1-at-risk").await, "normal", "非 Q2 被清回 normal");
        assert_eq!(protection_status_of(&pool, "q2-recent").await, "normal", "近期 Q2 不标记");
    }
}
