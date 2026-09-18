use sqlx::SqlitePool;

use crate::db::conversations;
use crate::db::memories;
use crate::db::pool::ConversationsPool;
use crate::db::roles;
use crate::db::tasks;
use crate::error::AppError;
use crate::models::dashboard::{DashboardMetrics, DashboardMetricsQuery, DashboardMetricsScope, DashboardStatus};

pub async fn get_dashboard_status(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
) -> Result<Vec<DashboardStatus>, AppError> {
    let roles = roles::list_active_roles(pool).await?;

    if roles.is_empty() {
        return Ok(Vec::new());
    }

    let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();

    let last_active_map = conversations::get_last_active_for_roles(conv_pool, &role_ids)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("批量查询角色最近活跃时间失败: {}", e);
            std::collections::HashMap::new()
        });

    let task_stats_map = tasks::get_task_stats_for_roles(pool, &role_ids)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("批量查询任务统计失败: {}", e);
            std::collections::HashMap::new()
        });

    let mut statuses: Vec<DashboardStatus> = roles
        .iter()
        .map(|role| {
            let stats = task_stats_map.get(&role.id).cloned().unwrap_or_default();
            let has_urgent = stats.urgent_count > 0;
            DashboardStatus {
                role_id: role.id.clone(),
                role_name: role.name.clone(),
                role_icon: role.icon.clone(),
                role_color: role.color.clone(),
                energy: role.energy,
                pending_tasks_count: stats.pending_count,
                last_active_at: last_active_map.get(&role.id).cloned(),
                has_urgent,
            }
        })
        .collect();

    statuses.sort_by(|a, b| {
        match (a.has_urgent, b.has_urgent) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_low = a.energy < 40;
                let b_low = b.energy < 40;
                match (a_low, b_low) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            }
        }
    });

    Ok(statuses)
}

pub async fn get_dashboard_metrics(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    query: DashboardMetricsQuery,
) -> Result<DashboardMetrics, AppError> {
    let (owner_condition, role_id) = match &query.scope {
        DashboardMetricsScope::All => ("all", None),
        DashboardMetricsScope::Butler => ("butler", None),
        DashboardMetricsScope::Role { role_id } => {
            roles::get_role(pool, role_id)
                .await
                .map_err(|_| AppError::ValidationError(format!("角色不存在: {}", role_id)))?;
            ("role", Some(role_id.as_str()))
        }
    };

    // 单次解析并规范化为 UTC（SQLite 文本比较要求统一 UTC Z 格式，AC-7/AC-9）
    let start_normalized = query
        .start_at
        .as_ref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .map_err(|_| AppError::ValidationError("startAt 无法解析为 RFC 3339 时间".to_string()))
        })
        .transpose()?;

    let end_normalized = query
        .end_at
        .as_ref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .map_err(|_| AppError::ValidationError("endAt 无法解析为 RFC 3339 时间".to_string()))
        })
        .transpose()?;

    if let (Some(start), Some(end)) = (start_normalized.as_ref(), end_normalized.as_ref()) {
        if start >= end {
            return Err(AppError::ValidationError(
                "startAt 必须早于 endAt".to_string(),
            ));
        }
    }

    let start_ref = start_normalized.as_deref();
    let end_ref = end_normalized.as_deref();

    let (task_count, pending_task_count, memory_count) = {
        let pool_clone = pool.clone();
        let oc = owner_condition.to_string();
        let rid = role_id.map(|s| s.to_string());
        let sa = start_ref.map(|s| s.to_string());
        let ea = end_ref.map(|s| s.to_string());

        let task_fut = {
            let pool = pool_clone.clone();
            let oc = oc.clone();
            let rid = rid.clone();
            let sa = sa.clone();
            let ea = ea.clone();
            async move {
                tasks::count_tasks_for_metrics(&pool, &oc, rid.as_deref(), sa.as_deref(), ea.as_deref())
                    .await
            }
        };

        let memory_fut = {
            let pool = pool_clone.clone();
            let oc = oc.clone();
            let rid = rid.clone();
            let sa = sa.clone();
            let ea = ea.clone();
            async move {
                memories::count_memories_for_metrics(&pool, &oc, rid.as_deref(), sa.as_deref(), ea.as_deref())
                    .await
            }
        };

        let ((tc, ptc), mc) = tokio::try_join!(task_fut, memory_fut)?;
        (tc, ptc, mc)
    };

    let conversation_count = conversations::count_conversations_for_metrics(
        conv_pool,
        owner_condition,
        role_id,
        start_ref,
        end_ref,
    )
    .await?;

    let generated_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    tracing::info!(
        "仪表盘指标聚合完成: scope={}, time_range={:?}-{:?}, tasks={}, memories={}, conversations={}, pending={}",
        owner_condition,
        start_ref,
        end_ref,
        task_count,
        memory_count,
        conversation_count,
        pending_task_count
    );

    Ok(DashboardMetrics {
        task_count,
        memory_count,
        conversation_count,
        pending_task_count,
        generated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_main_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test main db");

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
            "CREATE TABLE memories (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT,
                category TEXT NOT NULL,
                content TEXT NOT NULL,
                source_conversation_id TEXT NOT NULL,
                source_message_ids TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create memories table");

        pool
    }

    async fn setup_conv_pool() -> ConversationsPool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create test conv db");

        let schema = include_str!("../../migrations/002_conversations.sql");
        sqlx::raw_sql(schema)
            .execute(&pool)
            .await
            .expect("Failed to run migrations");

        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("Failed to add title column");

        ConversationsPool(pool)
    }

    #[tokio::test]
    async fn dashboard_sorts_urgent_first_then_low_energy() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        // role-normal: energy 80, no urgent
        sqlx::query("INSERT INTO roles (id, name, energy) VALUES ('r-normal', '正常角色', 80)")
            .execute(&pool)
            .await
            .expect("insert r-normal");

        // role-low: energy 20, no urgent
        sqlx::query("INSERT INTO roles (id, name, energy) VALUES ('r-low', '低能量角色', 20)")
            .execute(&pool)
            .await
            .expect("insert r-low");

        // role-urgent: energy 50, has Q1 task
        sqlx::query("INSERT INTO roles (id, name, energy) VALUES ('r-urgent', '紧急角色', 50)")
            .execute(&pool)
            .await
            .expect("insert r-urgent");

        // 给 r-urgent 添加一个 Q1 未完成任务
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at)
             VALUES ('t-q1', 'role', 'r-urgent', '紧急任务', 'Q1', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("insert Q1 task");

        // 给 r-normal 添加一个 Q2 未完成任务
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at)
             VALUES ('t-q2', 'role', 'r-normal', '普通任务', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("insert Q2 task");

        let statuses = get_dashboard_status(&pool, &conv_pool).await.expect("get dashboard");

        assert_eq!(statuses.len(), 3);
        // 紧急角色排第一
        assert_eq!(statuses[0].role_id, "r-urgent");
        assert!(statuses[0].has_urgent);
        // 低能量角色排第二
        assert_eq!(statuses[1].role_id, "r-low");
        assert!(!statuses[1].has_urgent);
        // 正常角色排第三
        assert_eq!(statuses[2].role_id, "r-normal");
        assert!(!statuses[2].has_urgent);
    }

    #[tokio::test]
    async fn dashboard_empty_roles_returns_empty() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        let statuses = get_dashboard_status(&pool, &conv_pool).await.expect("get dashboard");
        assert!(statuses.is_empty());
    }

    #[tokio::test]
    async fn dashboard_aggregates_pending_count_and_last_active() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        sqlx::query("INSERT INTO roles (id, name, energy) VALUES ('r1', '角色1', 60)")
            .execute(&pool)
            .await
            .expect("insert r1");

        // 2 个未完成 Q2 任务
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at)
             VALUES ('t1', 'role', 'r1', '任务1', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("insert t1");
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at)
             VALUES ('t2', 'role', 'r1', '任务2', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("insert t2");

        // 创建一条对话
        sqlx::query("INSERT INTO conversations (id, role_id, started_at, updated_at) VALUES ('c1', 'r1', '2026-06-01T00:00:00Z', '2026-06-20T10:00:00Z')")
            .execute(&*conv_pool)
            .await
            .expect("insert conversation");

        let statuses = get_dashboard_status(&pool, &conv_pool).await.expect("get dashboard");
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].pending_tasks_count, 2);
        assert!(!statuses[0].has_urgent);
        assert_eq!(
            statuses[0].last_active_at.as_deref(),
            Some("2026-06-20T10:00:00Z")
        );
    }

    #[tokio::test]
    async fn metrics_all_scope_aggregates_counts() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '角色1')")
            .execute(&pool).await.unwrap();

        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t1', 'role', 'r1', '任务1', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t2', 'butler', NULL, '管家任务', 'Q2', 1, '2026-06-02T00:00:00Z', '2026-06-02T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO memories (id, role_id, category, content, source_conversation_id, created_at) VALUES ('m1', NULL, 'fact', '全局记忆', 'conv-1', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO conversations (id, role_id, started_at, updated_at) VALUES ('c1', 'r1', '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&*conv_pool).await.unwrap();

        let metrics = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: None,
                end_at: None,
            },
        )
        .await
        .expect("get metrics");

        assert_eq!(metrics.task_count, 2);
        assert_eq!(metrics.pending_task_count, 1);
        assert_eq!(metrics.memory_count, 1);
        assert_eq!(metrics.conversation_count, 1);
        assert!(!metrics.generated_at.is_empty());
    }

    #[tokio::test]
    async fn metrics_butler_scope_filters_correctly() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('tb1', 'butler', NULL, '管家任务', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('tr1', 'role', 'r1', '角色任务', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO memories (id, role_id, category, content, source_conversation_id, created_at) VALUES ('m1', NULL, 'fact', '全局', 'conv-1', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO conversations (id, role_id, started_at, updated_at) VALUES ('c1', NULL, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&*conv_pool).await.unwrap();

        let metrics = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::Butler,
                start_at: None,
                end_at: None,
            },
        )
        .await
        .expect("get metrics");

        assert_eq!(metrics.task_count, 1, "仅 butler 任务");
        assert_eq!(metrics.memory_count, 1, "仅全局记忆");
        assert_eq!(metrics.conversation_count, 1, "仅管家会话");
    }

    #[tokio::test]
    async fn metrics_role_scope_filters_correctly() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        sqlx::query("INSERT INTO roles (id, name) VALUES ('r1', '角色1')")
            .execute(&pool).await.unwrap();

        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t-r1', 'role', 'r1', '角色1任务', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO memories (id, role_id, category, content, source_conversation_id, created_at) VALUES ('m-r1', 'r1', 'fact', '角色1记忆', 'conv-1', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO conversations (id, role_id, started_at, updated_at) VALUES ('c-r1', 'r1', '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&*conv_pool).await.unwrap();

        let metrics = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::Role { role_id: "r1".to_string() },
                start_at: None,
                end_at: None,
            },
        )
        .await
        .expect("get metrics");

        assert_eq!(metrics.task_count, 1);
        assert_eq!(metrics.memory_count, 1);
        assert_eq!(metrics.conversation_count, 1);
    }

    #[tokio::test]
    async fn metrics_role_scope_nonexistent_role_returns_validation_error() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        let result = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::Role { role_id: "nonexistent".to_string() },
                start_at: None,
                end_at: None,
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "应为 ValidationError");
    }

    #[tokio::test]
    async fn metrics_invalid_time_format_returns_validation_error() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        let result = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: Some("not-a-date".to_string()),
                end_at: None,
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "应为 ValidationError");
    }

    #[tokio::test]
    async fn metrics_start_ge_end_returns_validation_error() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        let result = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: Some("2026-06-10T00:00:00Z".to_string()),
                end_at: Some("2026-06-10T00:00:00Z".to_string()),
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "start >= end 应为 ValidationError");
    }

    #[tokio::test]
    async fn metrics_time_range_filters_half_open() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t1', 'role', NULL, '任务1', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t2', 'role', NULL, '任务2', 'Q2', 0, '2026-06-05T00:00:00Z', '2026-06-05T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t3', 'role', NULL, '任务3', 'Q2', 0, '2026-06-10T00:00:00Z', '2026-06-10T00:00:00Z')")
            .execute(&pool).await.unwrap();

        let metrics = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: Some("2026-06-01T00:00:00Z".to_string()),
                end_at: Some("2026-06-10T00:00:00Z".to_string()),
            },
        )
        .await
        .expect("get metrics");

        assert_eq!(metrics.task_count, 2, "半开区间：t3 在 end 边界外");
    }

    #[tokio::test]
    async fn metrics_pending_le_total_invariant() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;

        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t1', 'role', NULL, '未完成', 'Q2', 0, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, created_at, updated_at) VALUES ('t2', 'role', NULL, '已完成', 'Q2', 1, '2026-06-02T00:00:00Z', '2026-06-02T00:00:00Z')")
            .execute(&pool).await.unwrap();

        let metrics = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: None,
                end_at: None,
            },
        )
        .await
        .expect("get metrics");

        assert!(metrics.pending_task_count <= metrics.task_count, "pending <= total 不变量");
    }

    #[tokio::test]
    async fn metrics_main_db_failure_propagates() {
        // AC-12：主库查询失败时整个请求失败，不返回部分默认值
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create empty main pool");
        // 不创建 tasks/memories 表，查询必然失败
        let conv_pool = setup_conv_pool().await;

        let result = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: None,
                end_at: None,
            },
        )
        .await;

        assert!(result.is_err(), "主库失败时整个请求必须失败（AC-12）");
    }

    #[tokio::test]
    async fn metrics_conv_db_failure_propagates() {
        // AC-12：会话库查询失败时整个请求失败，不返回部分默认值
        let pool = setup_main_pool().await;
        // 不创建 conversations 表，会话查询必然失败
        let conv_pool = ConversationsPool(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .expect("create empty conv pool"),
        );

        let result = get_dashboard_metrics(
            &pool,
            &conv_pool,
            DashboardMetricsQuery {
                scope: DashboardMetricsScope::All,
                start_at: None,
                end_at: None,
            },
        )
        .await;

        assert!(result.is_err(), "会话库失败时整个请求必须失败（AC-12）");
    }
}
