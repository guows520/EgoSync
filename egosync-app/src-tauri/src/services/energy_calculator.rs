//! Story 4.8: 能量值计算引擎
//!
//! 职责：
//! - 读取角色任务/对话数据 → 公式计算 → 更新 roles 表 → 必要时生成低能量通知
//! - 公式: energy = 0.4 * task_completion_rate + 0.3 * recent_activity_score
//!                + 0.2 * goal_progress + 0.1 * (100 - at_risk_penalty)

use sqlx::SqlitePool;

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::models::role::Role;
use crate::services::notification_service;
use crate::services::suggestion_generator::NotificationLevel;

const WEIGHT_TASK_COMPLETION: f64 = 0.4;
const WEIGHT_RECENT_ACTIVITY: f64 = 0.3;
const WEIGHT_GOAL_PROGRESS: f64 = 0.2;
const WEIGHT_AT_RISK: f64 = 0.1;
const LOW_ENERGY_THRESHOLD: i32 = 40;
const AT_RISK_PENALTY_PER_TASK: i32 = 20;
const MAX_AT_RISK_PENALTY: i32 = 60;
const MAX_BIG_ROCKS: i32 = 3;

/// 根据最近活跃时间计算互动频率得分。
///
/// - 今天 → 100.0
/// - 1 天前 → 80.0
/// - 2 天前 → 65.0
/// - 3 天前 → 50.0
/// - 4-6 天前 → 30.0
/// - ≥ 7 天 → 10.0
/// - None / 解析失败 → 0.0
pub fn recent_activity_score_from_last_active(last_active_at: Option<&str>) -> f64 {
    let Some(ts) = last_active_at else {
        return 0.0;
    };

    let Some(last_date) = parse_iso_to_local_date(ts) else {
        return 0.0;
    };

    let today = chrono::Local::now().date_naive();
    activity_score_from_delta_days((today - last_date).num_days())
}

/// 将 ISO 8601 时间戳解析为本地时区下的日历日期。
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

/// 根据本地日历日期差映射互动频率得分（与 spec Task 5.5 一致）。
fn activity_score_from_delta_days(delta_days: i64) -> f64 {
    match delta_days {
        d if d <= 0 => 100.0,
        1 => 80.0,
        2 => 65.0,
        3 => 50.0,
        4..=6 => 30.0,
        _ => 10.0,
    }
}

/// 计算并更新角色能量值。
///
/// 读取任务统计 + 对话活跃时间 → 公式计算 → 写入 DB → 必要时生成低能量通知。
/// 错误只 warn 不阻塞调度器循环。
pub async fn calculate_and_update_energy(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    role: &Role,
) -> Result<(), AppError> {
    let old_energy = role.energy;

    let completed_7d = db::tasks::count_completed_tasks_in_last_7_days_for_role(pool, &role.id).await?;
    let total_tasks = db::tasks::count_total_tasks_for_role(pool, &role.id).await?;

    let task_completion_rate = if total_tasks == 0 {
        0.0
    } else {
        (completed_7d as f64 / total_tasks as f64) * 100.0
    };

    let last_active_map = db::conversations::get_last_active_for_roles(
        conv_pool,
        std::slice::from_ref(&role.id),
    )
    .await?;

    let last_active = last_active_map.get(&role.id).map(|s| s.as_str());
    let recent_activity_score = recent_activity_score_from_last_active(last_active);

    let active_big_rocks = db::tasks::count_active_big_rocks_for_role(pool, &role.id).await?;
    let goal_progress = ((1.0 - active_big_rocks as f64 / MAX_BIG_ROCKS as f64) * 100.0).clamp(0.0, 100.0);

    let at_risk_count = db::tasks::count_at_risk_q2_tasks_for_role(pool, &role.id).await?;
    let at_risk_penalty = (at_risk_count as i32 * AT_RISK_PENALTY_PER_TASK).min(MAX_AT_RISK_PENALTY);

    let energy_raw = WEIGHT_TASK_COMPLETION * task_completion_rate
        + WEIGHT_RECENT_ACTIVITY * recent_activity_score
        + WEIGHT_GOAL_PROGRESS * goal_progress
        + WEIGHT_AT_RISK * (100.0 - at_risk_penalty as f64);

    let energy = energy_raw.round() as i32;
    let energy = energy.clamp(0, 100);

    tracing::info!(
        role_id = %role.id,
        role_name = %role.name,
        old_energy,
        new_energy = energy,
        completed_7d,
        total_tasks,
        task_completion_rate,
        recent_activity_score,
        active_big_rocks,
        goal_progress,
        at_risk_count,
        at_risk_penalty,
        "能量值计算完成"
    );

    let now = crate::db::settings::chrono_now_pub();
    db::roles::update_energy(pool, &role.id, energy, &now).await?;

    // 首次计算（energy_updated_at 为空）只建立能量基线，不发低能量通知，
    // 避免新建/低活跃角色从默认 100 跌破阈值时误报"能量偏低"。
    let is_first_calculation = role.energy_updated_at.is_none();
    if !is_first_calculation && old_energy >= LOW_ENERGY_THRESHOLD && energy < LOW_ENERGY_THRESHOLD {
        let message = format!("你的{}角色能量值较低，可能需要关注", role.name);
        if let Err(e) = notification_service::create_notification_for_role(
            pool,
            &role.id,
            NotificationLevel::Tap,
            &message,
        )
        .await
        {
            tracing::warn!(
                role_id = %role.id,
                role_name = %role.name,
                error = %e,
                "低能量通知创建失败（不阻塞能量计算）"
            );
        } else {
            tracing::info!(
                role_id = %role.id,
                role_name = %role.name,
                energy,
                "低能量通知已生成"
            );
        }
    }

    Ok(())
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

        pool
    }

    async fn setup_test_conv_pool() -> ConversationsPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create conv test db");

        sqlx::query(
            "CREATE TABLE conversations (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT,
                title TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create conversations table");

        ConversationsPool(pool)
    }

    fn make_role(id: &str, name: &str, energy: i32) -> Role {
        Role {
            id: id.to_string(),
            name: name.to_string(),
            icon: "🎯".to_string(),
            color: "#6366F1".to_string(),
            goal: String::new(),
            personality_prompt: String::new(),
            status: "active".to_string(),
            energy,
            energy_updated_at: None,
            skills_config: "{}".to_string(),
            proactivity_level: "moderate".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    // ---- recent_activity_score_from_last_active tests ----

    #[test]
    fn recent_activity_score_today_returns_100() {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        assert_eq!(recent_activity_score_from_last_active(Some(&now)), 100.0);
    }

    #[test]
    fn recent_activity_score_yesterday_returns_80() {
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert_eq!(recent_activity_score_from_last_active(Some(&yesterday)), 80.0);
    }

    #[test]
    fn recent_activity_score_3_days_returns_50() {
        let three_days_ago = (chrono::Utc::now() - chrono::Duration::days(3))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert_eq!(recent_activity_score_from_last_active(Some(&three_days_ago)), 50.0);
    }

    #[test]
    fn recent_activity_score_7_days_returns_10() {
        let seven_days_ago = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert_eq!(recent_activity_score_from_last_active(Some(&seven_days_ago)), 10.0);
    }

    #[test]
    fn recent_activity_score_none_returns_0() {
        assert_eq!(recent_activity_score_from_last_active(None), 0.0);
    }

    #[test]
    fn recent_activity_score_invalid_returns_0() {
        assert_eq!(recent_activity_score_from_last_active(Some("invalid")), 0.0);
    }

    #[test]
    fn activity_score_delta_days_maps_calendar_buckets() {
        // 本地日历日期差 → 得分（含跨午夜边界：昨天=1天=80）
        assert_eq!(activity_score_from_delta_days(-1), 100.0); // 未来时间戳归为今天
        assert_eq!(activity_score_from_delta_days(0), 100.0);
        assert_eq!(activity_score_from_delta_days(1), 80.0);
        assert_eq!(activity_score_from_delta_days(2), 65.0);
        assert_eq!(activity_score_from_delta_days(3), 50.0);
        assert_eq!(activity_score_from_delta_days(4), 30.0);
        assert_eq!(activity_score_from_delta_days(6), 30.0);
        assert_eq!(activity_score_from_delta_days(7), 10.0);
        assert_eq!(activity_score_from_delta_days(30), 10.0);
    }

    // ---- calculate_and_update_energy integration tests ----

    #[tokio::test]
    async fn energy_no_tasks_no_conversation_returns_30() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = make_role("r1", "产品", 100);
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        let energy: i32 = sqlx::query_scalar("SELECT energy FROM roles WHERE id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        // task_completion=0, activity=0, goal_progress=100, at_risk=0
        // energy = 0.4*0 + 0.3*0 + 0.2*100 + 0.1*100 = 30
        assert_eq!(energy, 30);
    }

    #[tokio::test]
    async fn energy_with_completed_tasks_increases() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        // 2 tasks, 1 completed recently
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, completed_at, created_at, updated_at) VALUES ('t1', 'role', 'r1', 'task1', 'Q1', 1, ?1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t2', 'role', 'r1', 'task2', 'Q2', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .unwrap();

        let role = make_role("r1", "产品", 100);
        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        let energy: i32 = sqlx::query_scalar("SELECT energy FROM roles WHERE id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        // task_completion = 1/2 * 100 = 50
        // activity = 0, goal_progress = 100, at_risk = 0
        // energy = 0.4*50 + 0.3*0 + 0.2*100 + 0.1*100 = 50
        assert_eq!(energy, 50);
    }

    #[tokio::test]
    async fn energy_with_at_risk_q2_decreases() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, protection_status, created_at, updated_at) VALUES ('t1', 'role', 'r1', 'task1', 'Q2', 0, 'at_risk', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .execute(&pool)
            .await
            .unwrap();

        let role = make_role("r1", "产品", 100);
        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        let energy: i32 = sqlx::query_scalar("SELECT energy FROM roles WHERE id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        // task_completion=0, activity=0, goal_progress=100, at_risk_penalty=20
        // energy = 0.4*0 + 0.3*0 + 0.2*100 + 0.1*(100-20) = 28
        assert_eq!(energy, 28);
    }

    #[tokio::test]
    async fn energy_crossing_threshold_generates_notification() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 3 active big rocks → goal_progress = 0
        for i in 1..=3 {
            sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, is_completed, created_at, updated_at) VALUES (?1, 'role', 'r1', ?2, 'Q2', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
                .bind(format!("br{}", i))
                .bind(format!("大石头{}", i))
                .execute(&pool)
                .await
                .unwrap();
        }
        // 3 at_risk Q2 → at_risk_penalty = 60
        for i in 1..=3 {
            sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, protection_status, created_at, updated_at) VALUES (?1, 'role', 'r1', ?2, 'Q2', 0, 'at_risk', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
                .bind(format!("ar{}", i))
                .bind(format!("风险任务{}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // old_energy = 50 (>= 40), new should be < 40
        // energy_updated_at 已存在 → 非首次计算，应正常发通知
        let mut role = make_role("r1", "产品", 50);
        role.energy_updated_at = Some("2026-06-01T00:00:00Z".to_string());
        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        let energy: i32 = sqlx::query_scalar("SELECT energy FROM roles WHERE id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        // task_completion=0, activity=0, goal_progress=0, at_risk_penalty=60
        // energy = 0.4*0 + 0.3*0 + 0.2*0 + 0.1*(100-60) = 4
        assert!(energy < 40, "energy should be < 40, got {}", energy);

        let notif_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE role_id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(notif_count, 1, "应生成 1 条低能量通知");
    }

    #[tokio::test]
    async fn energy_staying_below_threshold_no_duplicate_notification() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // Setup to get energy < 40
        for i in 1..=3 {
            sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, is_completed, created_at, updated_at) VALUES (?1, 'role', 'r1', ?2, 'Q2', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
                .bind(format!("br{}", i))
                .bind(format!("大石头{}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // First call: old_energy = 50 → should trigger notification
        // energy_updated_at 已存在 → 非首次计算
        let mut role = make_role("r1", "产品", 50);
        role.energy_updated_at = Some("2026-06-01T00:00:00Z".to_string());
        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        // Second call: old_energy is now low (e.g. 6) → should NOT trigger
        let mut role_updated = make_role("r1", "产品", 6);
        role_updated.energy_updated_at = Some("2026-06-01T00:00:00Z".to_string());
        calculate_and_update_energy(&pool, &conv_pool, &role_updated)
            .await
            .expect("calc");

        let notif_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE role_id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(notif_count, 1, "不应重复生成低能量通知");
    }

    #[tokio::test]
    async fn energy_first_calculation_skips_low_energy_notification() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // 3 active big rocks + 3 at_risk Q2 → energy < 40
        for i in 1..=3 {
            sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_big_rock, is_completed, created_at, updated_at) VALUES (?1, 'role', 'r1', ?2, 'Q2', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
                .bind(format!("br{}", i))
                .bind(format!("大石头{}", i))
                .execute(&pool)
                .await
                .unwrap();
        }
        for i in 1..=3 {
            sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, protection_status, created_at, updated_at) VALUES (?1, 'role', 'r1', ?2, 'Q2', 0, 'at_risk', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
                .bind(format!("ar{}", i))
                .bind(format!("风险任务{}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // 首次计算：energy_updated_at = None，即使 energy < 40 也不应发通知
        let role = make_role("r1", "产品", 100);
        assert!(role.energy_updated_at.is_none());
        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        let energy: i32 = sqlx::query_scalar("SELECT energy FROM roles WHERE id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(energy < 40, "energy should be < 40, got {}", energy);

        let notif_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE role_id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(notif_count, 0, "首次计算应跳过低能量通知（仅建立基线）");
    }

    #[tokio::test]
    async fn energy_clamped_to_0_100() {
        let pool = setup_test_db().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '产品')")
            .execute(&pool)
            .await
            .unwrap();

        // Best case: 0 tasks, no big rocks, no at_risk, recent conversation
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        sqlx::query("INSERT INTO conversations (id, role_id, title, created_at, updated_at) VALUES ('c1', 'r1', 'chat', ?1, ?1)")
            .bind(&now)
            .execute(&*conv_pool)
            .await
            .unwrap();

        let role = make_role("r1", "产品", 50);
        calculate_and_update_energy(&pool, &conv_pool, &role)
            .await
            .expect("calc");

        let energy: i32 = sqlx::query_scalar("SELECT energy FROM roles WHERE id = 'r1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        // task_completion=0, activity=100, goal_progress=100, at_risk=0
        // energy = 0.4*0 + 0.3*100 + 0.2*100 + 0.1*100 = 60
        assert_eq!(energy, 60);
        assert!(energy >= 0 && energy <= 100, "energy should be clamped [0, 100]");
    }
}
