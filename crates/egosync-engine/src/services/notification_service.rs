/// Story 4.5: 通知创建服务 — 降级逻辑 + DB 写入
///
/// 两层降级：
/// 1. proactivity 约束 — `max_notification_level_for_proactivity`
/// 2. 每日敲门上限 — 当天 >= 3 次 knock 则降级为 tap

use sqlx::SqlitePool;

use crate::db;
use crate::error::AppError;
use crate::models::notification::{CreateNotificationInput, Notification};
use crate::services::suggestion_generator::{
    max_notification_level_for_proactivity, NotificationLevel,
};

/// 每日敲门通知上限。
const DAILY_KNOCK_LIMIT: i64 = 3;

/// 将 `NotificationLevel` 转为 DB 字符串。
fn level_to_str(level: NotificationLevel) -> &'static str {
    match level {
        NotificationLevel::Whisper => "whisper",
        NotificationLevel::Tap => "tap",
        NotificationLevel::Knock => "knock",
    }
}

/// 将字符串转为 `NotificationLevel`，未知值降级为 Whisper。
fn str_to_level(s: &str) -> NotificationLevel {
    match s {
        "knock" => NotificationLevel::Knock,
        "tap" => NotificationLevel::Tap,
        _ => NotificationLevel::Whisper,
    }
}

/// 比较两个级别，返回较低的（降级用）。
fn min_level(a: NotificationLevel, b: NotificationLevel) -> NotificationLevel {
    let rank = |l: NotificationLevel| match l {
        NotificationLevel::Whisper => 0,
        NotificationLevel::Tap => 1,
        NotificationLevel::Knock => 2,
    };
    if rank(a) <= rank(b) {
        a
    } else {
        b
    }
}

/// 为角色创建通知，应用两层降级逻辑。
///
/// 1. proactivity 约束：根据角色 `proactivity_level` 限制最高通知级别
/// 2. 每日敲门上限：当天已有 >= 3 次 knock 则降级为 tap
///
/// 返回创建的 `Notification`（含最终级别）。
pub async fn create_notification_for_role(
    pool: &SqlitePool,
    role_id: &str,
    requested_level: NotificationLevel,
    content: &str,
) -> Result<Notification, AppError> {
    // 读取角色 proactivity_level
    let role = db::roles::get_role(pool, role_id).await?;

    // 第一层：proactivity 约束
    let max_allowed = max_notification_level_for_proactivity(&role.proactivity_level);
    let level_after_proactivity = min_level(requested_level, max_allowed);

    if level_after_proactivity != requested_level {
        tracing::info!(
            role_id = %role_id,
            requested = ?requested_level,
            degraded_to = ?level_after_proactivity,
            proactivity_level = %role.proactivity_level,
            "通知级别因 proactivity 约束降级"
        );
    }

    // 第二层：每日敲门上限
    let mut final_level = level_after_proactivity;
    if final_level == NotificationLevel::Knock {
        let knock_count = db::notifications::count_knock_today(pool).await?;
        if knock_count >= DAILY_KNOCK_LIMIT {
            final_level = NotificationLevel::Tap;
            tracing::info!(
                role_id = %role_id,
                knock_count,
                limit = DAILY_KNOCK_LIMIT,
                "通知级别因每日敲门上限降级为 tap"
            );
        }
    }

    let input = CreateNotificationInput {
        role_id: role_id.to_string(),
        level: level_to_str(final_level).to_string(),
        content: content.to_string(),
    };

    db::notifications::create_notification(pool, &input).await
}

/// 将级别字符串转为 `NotificationLevel`，供 command 层使用。
pub fn parse_level(s: &str) -> NotificationLevel {
    str_to_level(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_to_str_round_trips() {
        assert_eq!(level_to_str(NotificationLevel::Whisper), "whisper");
        assert_eq!(level_to_str(NotificationLevel::Tap), "tap");
        assert_eq!(level_to_str(NotificationLevel::Knock), "knock");
    }

    #[test]
    fn str_to_level_known_values() {
        assert_eq!(str_to_level("whisper"), NotificationLevel::Whisper);
        assert_eq!(str_to_level("tap"), NotificationLevel::Tap);
        assert_eq!(str_to_level("knock"), NotificationLevel::Knock);
    }

    #[test]
    fn str_to_level_unknown_defaults_to_whisper() {
        assert_eq!(str_to_level("invalid"), NotificationLevel::Whisper);
        assert_eq!(str_to_level(""), NotificationLevel::Whisper);
    }

    #[test]
    fn min_level_returns_lower() {
        assert_eq!(
            min_level(NotificationLevel::Knock, NotificationLevel::Whisper),
            NotificationLevel::Whisper
        );
        assert_eq!(
            min_level(NotificationLevel::Knock, NotificationLevel::Tap),
            NotificationLevel::Tap
        );
        assert_eq!(
            min_level(NotificationLevel::Tap, NotificationLevel::Tap),
            NotificationLevel::Tap
        );
        assert_eq!(
            min_level(NotificationLevel::Whisper, NotificationLevel::Knock),
            NotificationLevel::Whisper
        );
    }

    #[tokio::test]
    async fn create_notification_proactivity_constraint_degrades_knock_to_tap() {
        let pool = setup_test_db_with_role("moderate").await;

        let n = create_notification_for_role(
            &pool,
            "role-a",
            NotificationLevel::Knock,
            "测试敲门",
        )
        .await
        .expect("create");

        assert_eq!(n.level, "tap", "moderate 角色的 knock 应降级为 tap");
    }

    #[tokio::test]
    async fn create_notification_proactivity_constraint_degrades_knock_to_whisper() {
        let pool = setup_test_db_with_role("passive").await;

        let n = create_notification_for_role(
            &pool,
            "role-a",
            NotificationLevel::Knock,
            "测试敲门",
        )
        .await
        .expect("create");

        assert_eq!(n.level, "whisper", "passive 角色的 knock 应降级为 whisper");
    }

    #[tokio::test]
    async fn create_notification_proactive_allows_knock() {
        let pool = setup_test_db_with_role("proactive").await;

        let n = create_notification_for_role(
            &pool,
            "role-a",
            NotificationLevel::Knock,
            "测试敲门",
        )
        .await
        .expect("create");

        assert_eq!(n.level, "knock", "proactive 角色允许 knock");
    }

    #[tokio::test]
    async fn create_notification_daily_knock_limit_degrades_to_tap() {
        let pool = setup_test_db_with_role("proactive").await;

        // 创建 3 次 knock（proactive 角色允许）
        for i in 0..3 {
            create_notification_for_role(
                &pool,
                "role-a",
                NotificationLevel::Knock,
                &format!("敲门 #{}", i),
            )
            .await
            .expect("create");
        }

        // 第 4 次 knock 应降级为 tap
        let n = create_notification_for_role(
            &pool,
            "role-a",
            NotificationLevel::Knock,
            "敲门 #4",
        )
        .await
        .expect("create");

        assert_eq!(n.level, "tap", "第 4 次 knock 应降级为 tap");
    }

    #[tokio::test]
    async fn create_notification_daily_knock_limit_does_not_affect_tap() {
        let pool = setup_test_db_with_role("proactive").await;

        // 填满 3 次 knock
        for i in 0..3 {
            create_notification_for_role(
                &pool,
                "role-a",
                NotificationLevel::Knock,
                &format!("敲门 #{}", i),
            )
            .await
            .expect("create");
        }

        // tap 不受敲门上限影响
        let n = create_notification_for_role(
            &pool,
            "role-a",
            NotificationLevel::Tap,
            "轻触通知",
        )
        .await
        .expect("create");

        assert_eq!(n.level, "tap");
    }

    async fn setup_test_db_with_role(proactivity_level: &str) -> sqlx::SqlitePool {
        use sqlx::sqlite::SqlitePoolOptions;

        let pool = SqlitePoolOptions::new()
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

        sqlx::query("INSERT INTO roles (id, name, icon, color, proactivity_level) VALUES ('role-a', '产品', '🎯', '#6366F1', ?1)")
            .bind(proactivity_level)
            .execute(&pool)
            .await
            .expect("failed to insert role");

        sqlx::query(
            "CREATE TABLE notifications (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT NOT NULL,
                level TEXT NOT NULL CHECK (level IN ('whisper', 'tap', 'knock')),
                content TEXT NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create notifications table");

        pool
    }
}
