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
/// 首次 tick 立即执行（便于启动后快速检查），之后每 3600 秒一次。错误只 warn，不 panic。
pub fn spawn_hourly_watch(pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
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
}
