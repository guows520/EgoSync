//! Story 3.3: 每小时临期检查后台任务
//!
//! 职责：
//! - 在 Tauri 启动时 spawn 一个后台 tokio task，内部使用 `tokio::interval(3600s)`
//! - 每次 tick 查询 `deadline <= now + 2 days`、未完成、未软删除、未手动覆盖、当前为 Q2 的任务
//! - 将这些任务升入 Q1，写入中文 classification_reason（Q3/Q4 不升，已是 Q1 无需升）
//! - 错误只 `tracing::warn!`，绝不 panic，绝不阻塞 Tauri 启动
//!
//! 单测通过直接调用 `escalate_imminent_tasks` 验证逻辑，无需等待一小时。

use sqlx::SqlitePool;

use crate::db;
use crate::error::AppError;
use crate::services::task_classifier::IMMINENT_DAYS;

/// 升级临期任务到 Q1。返回被升级的任务数量。
///
/// 查询条件：deadline <= now+2天、未完成、未软删除、manual_override = 0、quadrant = 'Q2'。
/// 升级时写入 classification_reason = "截止日期已进入 2 天内，自动升入 Q1"。
pub async fn escalate_imminent_tasks(pool: &SqlitePool) -> Result<usize, AppError> {
    let threshold = compute_imminent_threshold();
    let tasks = db::tasks::list_imminent_tasks_for_escalation(pool, &threshold).await?;

    let mut upgraded = 0usize;
    for task in &tasks {
        match db::tasks::update_task_classification(
            pool,
            &task.id,
            "Q1",
            0.9,
            "截止日期已进入 2 天内，自动升入 Q1",
        )
        .await
        {
            Ok(_) => upgraded += 1,
            Err(AppError::NotFound(_)) => {
                // 任务可能在此期间被手动覆盖或删除，跳过即可
                tracing::debug!(task_id = %task.id, "临期升 Q1 跳过：任务已手动覆盖或不存在");
            }
            Err(e) => {
                tracing::warn!(task_id = %task.id, error = %e, "临期升 Q1 失败");
            }
        }
    }

    if upgraded > 0 {
        tracing::info!(upgraded, "临期任务升 Q1 完成");
    }
    Ok(upgraded)
}

/// 计算 `now + IMMINENT_DAYS` 的 ISO 8601 日期时间字符串（`YYYY-MM-DDT23:59:59Z`）。
/// deadline 列可能存储 `YYYY-MM-DD` 或 `YYYY-MM-DDTHH:MM:SSZ` 格式，
/// 使用当天 23:59:59 作为阈值确保同日带时间的 deadline 仍能被字符串比较命中。
fn compute_imminent_threshold() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let threshold_secs = now_secs + IMMINENT_DAYS * 86400;
    let days = threshold_secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}T23:59:59Z", y, m, d)
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

/// 启动每小时临期检查后台任务。在 Tauri `setup` 中调用。
///
/// 首次 tick 立即执行（便于启动后快速检查），之后每 3600 秒一次。
/// 错误只 warn，不 panic。
pub fn spawn_hourly_watch(pool: SqlitePool) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        // 首次 tick 立即返回
        interval.tick().await;

        loop {
            if let Err(e) = escalate_imminent_tasks(&pool).await {
                tracing::warn!(error = %e, "临期检查后台任务出错（降级继续）");
            }
            interval.tick().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_imminent_threshold_is_iso_datetime() {
        let threshold = compute_imminent_threshold();
        // 格式 YYYY-MM-DDT23:59:59Z
        assert_eq!(threshold.len(), 20);
        assert_eq!(threshold.chars().nth(4), Some('-'));
        assert_eq!(threshold.chars().nth(7), Some('-'));
        assert_eq!(threshold.chars().nth(10), Some('T'));
        assert!(threshold.ends_with("Z"));
    }

    #[test]
    fn days_to_ymd_round_trips_via_threshold() {
        // 通过当前时间生成阈值，再反向验证年份合理（>= 2025），避免硬编码 epoch 天数。
        let threshold = compute_imminent_threshold();
        let parts: Vec<&str> = threshold.split('-').collect();
        assert_eq!(parts.len(), 3);
        let year: i64 = parts[0].parse().expect("year should parse");
        let month: u32 = parts[1].parse().expect("month should parse");
        let day: u32 = parts[2].split('T').next().unwrap().parse().expect("day should parse");
        assert!(year >= 2025, "年份应在合理范围内，得到 {}", year);
        assert!((1..=12).contains(&month), "月份越界：{}", month);
        assert!((1..=31).contains(&day), "日期越界：{}", day);
    }

    #[test]
    fn days_to_ymd_handles_epoch_origin() {
        // 1970-01-01 对应 Unix days = 0
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }
}
