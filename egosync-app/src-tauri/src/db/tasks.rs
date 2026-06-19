use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::task::{CreateTaskInput, Task, TaskOwnerType, UpdateTaskInput};

const TASK_SELECT_COLUMNS: &str = "id, owner_type, role_id, title, deadline, quadrant, is_big_rock, is_completed, completed_at, sort_order, protection_status, confidence, manual_override, classification_reason, created_at, updated_at, deleted_at";
const ALLOWED_QUADRANTS: &[&str] = &["Q1", "Q2", "Q3", "Q4"];
const MAX_BIG_ROCKS_PER_OWNER: i32 = 3;
const BIG_ROCK_LIMIT_MESSAGE: &str = "每个任务清单每周最多 3 个大石头，请先取消一个再标记";

pub async fn create_task(pool: &SqlitePool, input: &CreateTaskInput) -> Result<Task, AppError> {
    let (owner_type, role_id) = normalized_owner(input.owner_type.as_ref(), input.role_id.as_deref())?;
    let title = normalized_title(&input.title)?;
    let quadrant = normalized_quadrant(input.quadrant.as_deref())?;
    let is_big_rock = input.is_big_rock.unwrap_or(false);
    if is_big_rock {
        let count = count_big_rocks_by_owner(pool, owner_type, role_id).await?;
        if count >= MAX_BIG_ROCKS_PER_OWNER {
            return Err(AppError::ValidationError(BIG_ROCK_LIMIT_MESSAGE.to_string()));
        }
    }
    // 若用户在 TaskModal 中显式选择了 quadrant，则标记 manual_override = true，
    // 后续自动分类与临期升 Q1 不会覆盖该任务。
    let manual_override = input.quadrant.is_some();
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();
    let sort_order = next_sort_order(pool, owner_type, role_id).await?;

    sqlx::query(
        "INSERT INTO tasks (id, owner_type, role_id, title, deadline, quadrant, is_big_rock, is_completed, sort_order, protection_status, manual_override, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, 'normal', ?9, ?10, ?11)",
    )
    .bind(&id)
    .bind(owner_type)
    .bind(role_id)
    .bind(title)
    .bind(input.deadline.as_deref())
    .bind(quadrant)
    .bind(is_big_rock)
    .bind(sort_order)
    .bind(manual_override)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建任务失败: {}", e)))?;

    get_active_task(pool, &id).await
}

pub async fn list_tasks_by_role(pool: &SqlitePool, role_id: &str) -> Result<Vec<Task>, AppError> {
    list_tasks_by_owner(pool, "role", Some(role_id)).await
}

pub async fn list_butler_tasks(pool: &SqlitePool) -> Result<Vec<Task>, AppError> {
    list_tasks_by_owner(pool, "butler", None).await
}

async fn list_tasks_by_owner(
    pool: &SqlitePool,
    owner_type: &str,
    role_id: Option<&str>,
) -> Result<Vec<Task>, AppError> {
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks
         WHERE owner_type = ?1
           AND ((?2 IS NULL AND role_id IS NULL) OR role_id = ?2)
           AND deleted_at IS NULL
         ORDER BY sort_order ASC, created_at ASC",
        TASK_SELECT_COLUMNS
    ))
    .bind(owner_type)
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询任务列表失败: {}", e)))
}

pub async fn update_task(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateTaskInput,
) -> Result<Task, AppError> {
    let existing = get_active_task(pool, id).await?;

    if input.is_big_rock == Some(true) && !existing.is_big_rock {
        let count = count_big_rocks_by_owner(pool, &existing.owner_type, existing.role_id.as_deref()).await?;
        if count >= MAX_BIG_ROCKS_PER_OWNER {
            return Err(AppError::ValidationError(BIG_ROCK_LIMIT_MESSAGE.to_string()));
        }
    }

    let title = match input.title.as_deref() {
        Some(title) => Some(normalized_title(title)?),
        None => None,
    };
    let quadrant = match input.quadrant.as_deref() {
        Some(quadrant) => Some(normalized_quadrant(Some(quadrant))?.to_string()),
        None => None,
    };
    // 若用户显式修改 quadrant，标记 manual_override = true，后续自动分类不再覆盖。
    let should_set_manual_override = quadrant.is_some();
    let deadline = input.deadline.as_ref().map(|value| value.as_deref());
    let should_update_deadline = input.deadline.is_some();
    let now = crate::db::settings::chrono_now_pub();

    let result = sqlx::query(
        "UPDATE tasks
         SET title = COALESCE(?1, title),
             deadline = CASE WHEN ?2 THEN ?3 ELSE deadline END,
             quadrant = COALESCE(?4, quadrant),
             is_big_rock = COALESCE(?5, is_big_rock),
             manual_override = CASE WHEN ?6 THEN 1 ELSE manual_override END,
             updated_at = ?7
         WHERE id = ?8 AND deleted_at IS NULL",
    )
    .bind(title)
    .bind(should_update_deadline)
    .bind(deadline.flatten())
    .bind(quadrant.as_deref())
    .bind(input.is_big_rock)
    .bind(should_set_manual_override)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新任务失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("任务 {} 不存在", id)));
    }

    get_active_task(pool, id).await
}

pub async fn soft_delete_task(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    get_active_task(pool, id).await?;

    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("删除任务失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("任务 {} 不存在", id)));
    }

    Ok(())
}

/// 批量重写某任务清单的 `sort_order`。
///
/// 入参 `task_ids` 为该 owner 完整任务的新顺序，`sort_order` 被重写为列表索引（0..n）。
/// 所有 id 必须存在、未软删除且同属一个 owner，否则在单事务内整体回滚并返回错误。
/// 重复 id 视为 `ValidationError`：避免同一 id 在事务内被多次 UPDATE 覆盖导致 `sort_order` 错乱。
pub async fn reorder_tasks(pool: &SqlitePool, task_ids: &[String]) -> Result<(), AppError> {
    if task_ids.is_empty() {
        return Ok(());
    }

    let mut seen = std::collections::HashSet::with_capacity(task_ids.len());
    for id in task_ids {
        if !seen.insert(id.as_str()) {
            return Err(AppError::ValidationError(format!(
                "任务 ID {} 重复出现",
                id
            )));
        }
    }

    let now = crate::db::settings::chrono_now_pub();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启排序事务失败: {}", e)))?;

    let mut scoped_owner: Option<(String, Option<String>)> = None;
    for (index, id) in task_ids.iter().enumerate() {
        let task_owner = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT owner_type, role_id FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("查询任务 owner 失败: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("任务 {} 不存在", id)))?;

        match &scoped_owner {
            None => scoped_owner = Some(task_owner),
            Some(owner) if owner != &task_owner => {
                return Err(AppError::ValidationError(
                    "排序任务必须属于同一个任务清单".to_string(),
                ));
            }
            _ => {}
        }

        sqlx::query("UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL")
            .bind(index as i32)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DbError(format!("更新任务排序失败: {}", e)))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交排序事务失败: {}", e)))?;

    Ok(())
}

/// 设置任务完成状态：完成时写 `completed_at = now`，取消时置 `NULL`，并同步 `updated_at`。
/// 不修改 `sort_order`（撤销后凭原 `sort_order` 回到分组内原位置）。不存在/已软删除返回 `NotFound`。
/// 幂等：若当前 `is_completed` 已等于目标值，直接返回当前 task，不刷新 `completed_at`/`updated_at`，
/// 保护「首次完成时刻」语义，避免 agent / 重复调用污染历史时间戳。
///
/// 方案 D：完成任务时自动撤销大石头标记（`is_big_rock = 0`），使已完成任务不再占用大石头名额。
/// 这样撤销完成后任务只是普通未完成任务，不会因恢复大石头身份而导致名额超限，彻底消除边界矛盾。
pub async fn set_task_completion(
    pool: &SqlitePool,
    id: &str,
    is_completed: bool,
) -> Result<Task, AppError> {
    let existing = get_active_task(pool, id).await?;
    if existing.is_completed == is_completed {
        return Ok(existing);
    }

    let now = crate::db::settings::chrono_now_pub();
    let completed_at = if is_completed { Some(now.as_str()) } else { None };

    let result = sqlx::query(
        "UPDATE tasks
         SET is_completed = ?1,
             completed_at = ?2,
             is_big_rock = CASE WHEN ?1 THEN 0 ELSE is_big_rock END,
             updated_at = ?3
         WHERE id = ?4 AND deleted_at IS NULL",
    )
    .bind(is_completed)
    .bind(completed_at)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新任务完成状态失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("任务 {} 不存在", id)));
    }

    get_active_task(pool, id).await
}

async fn get_active_task(pool: &SqlitePool, id: &str) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
        TASK_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询任务失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("任务 {} 不存在", id)))
}

/// 暴露给 services 层使用的 active task 查询。
pub async fn get_active_task_pub(pool: &SqlitePool, id: &str) -> Result<Task, AppError> {
    get_active_task(pool, id).await
}

pub async fn count_big_rocks_by_owner(
    pool: &SqlitePool,
    owner_type: &str,
    role_id: Option<&str>,
) -> Result<i32, AppError> {
    let count = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM tasks
         WHERE owner_type = ?1
           AND ((?2 IS NULL AND role_id IS NULL) OR role_id = ?2)
           AND is_big_rock = 1
           AND deleted_at IS NULL",
    )
    .bind(owner_type)
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询大石头数量失败: {}", e)))?;

    Ok(count)
}

async fn next_sort_order(
    pool: &SqlitePool,
    owner_type: &str,
    role_id: Option<&str>,
) -> Result<i32, AppError> {
    let max_order = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(sort_order) FROM tasks
         WHERE owner_type = ?1
           AND ((?2 IS NULL AND role_id IS NULL) OR role_id = ?2)
           AND deleted_at IS NULL",
    )
    .bind(owner_type)
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("计算任务排序失败: {}", e)))?;

    Ok(max_order.map_or(0, |order| order + 1))
}

fn normalized_owner<'a>(
    owner_type: Option<&TaskOwnerType>,
    role_id: Option<&'a str>,
) -> Result<(&'static str, Option<&'a str>), AppError> {
    match owner_type.unwrap_or(&TaskOwnerType::Role) {
        TaskOwnerType::Role => {
            let role_id = role_id
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| AppError::ValidationError("角色任务必须指定角色".to_string()))?;
            Ok(("role", Some(role_id)))
        }
        TaskOwnerType::Butler => Ok(("butler", None)),
    }
}

fn normalized_title(title: &str) -> Result<&str, AppError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::ValidationError("任务标题不能为空".to_string()));
    }
    Ok(title)
}

fn normalized_quadrant(quadrant: Option<&str>) -> Result<&str, AppError> {
    let quadrant = quadrant.unwrap_or("Q2");
    if !ALLOWED_QUADRANTS.contains(&quadrant) {
        return Err(AppError::ValidationError("四象限分类无效".to_string()));
    }
    Ok(quadrant)
}

/// 写入自动分类结果：更新 quadrant、confidence、classification_reason、updated_at。
/// 不修改 sort_order、is_completed、completed_at、manual_override。
/// 调用方应已确认 `manual_override = false`。
pub async fn update_task_classification(
    pool: &SqlitePool,
    id: &str,
    quadrant: &str,
    confidence: f64,
    classification_reason: &str,
) -> Result<Task, AppError> {
    if !ALLOWED_QUADRANTS.contains(&quadrant) {
        return Err(AppError::ValidationError("四象限分类无效".to_string()));
    }
    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE tasks
         SET quadrant = ?1,
             confidence = ?2,
             classification_reason = ?3,
             updated_at = ?4
         WHERE id = ?5 AND deleted_at IS NULL AND manual_override = 0",
    )
    .bind(quadrant)
    .bind(confidence)
    .bind(classification_reason)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("写入分类结果失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "任务 {} 不存在或已被手动覆盖",
            id
        )));
    }

    get_active_task(pool, id).await
}

/// 读取某 owner 最近 5 条同 owner 任务（排除当前任务、软删除任务），按 created_at DESC。
pub async fn list_recent_tasks_by_owner(
    pool: &SqlitePool,
    owner_type: &str,
    role_id: Option<&str>,
    exclude_task_id: &str,
    limit: u32,
) -> Result<Vec<Task>, AppError> {
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks
         WHERE owner_type = ?1
           AND ((?2 IS NULL AND role_id IS NULL) OR role_id = ?2)
           AND id != ?3
           AND deleted_at IS NULL
         ORDER BY created_at DESC
         LIMIT ?4",
        TASK_SELECT_COLUMNS
    ))
    .bind(owner_type)
    .bind(role_id)
    .bind(exclude_task_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询同任务清单历史任务失败: {}", e)))
}

pub async fn list_recent_tasks_by_role(
    pool: &SqlitePool,
    role_id: &str,
    exclude_task_id: &str,
    limit: u32,
) -> Result<Vec<Task>, AppError> {
    list_recent_tasks_by_owner(pool, "role", Some(role_id), exclude_task_id, limit).await
}

/// 查询临期且未手动覆盖的未完成任务：
/// deadline IS NOT NULL、deleted_at IS NULL、is_completed = 0、manual_override = 0、quadrant = 'Q2'。
/// 仅「重要不紧急(Q2)」的任务在临期时升入 Q1；Q3/Q4 本就不重要，临期也不应升为重要紧急。
/// `deadline_threshold` 为 ISO 8601 字符串（如 "2026-06-19"），返回 deadline <= threshold 的任务。
pub async fn list_imminent_tasks_for_escalation(
    pool: &SqlitePool,
    deadline_threshold: &str,
) -> Result<Vec<Task>, AppError> {
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks
         WHERE deadline IS NOT NULL
           AND deadline != ''
           AND deadline <= ?1
           AND deleted_at IS NULL
           AND is_completed = 0
           AND manual_override = 0
           AND quadrant = 'Q2'
         ORDER BY deadline ASC",
        TASK_SELECT_COLUMNS
    ))
    .bind(deadline_threshold)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询临期任务失败: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::TaskOwnerType;
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
                icon TEXT NOT NULL DEFAULT 'target',
                color TEXT NOT NULL DEFAULT '#4F46E5',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
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

    #[tokio::test]
    async fn create_and_list_tasks_stays_scoped_to_role() {
        let pool = setup_test_db().await;

        let first = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "准备季度规划".to_string(),
                deadline: Some("2026-06-30".to_string()),
                quadrant: Some("Q1".to_string()),
                is_big_rock: Some(true),
            },
        )
        .await
        .expect("create first task");
        create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-b".to_string()),
                title: "阅读论文".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(false),
            },
        )
        .await
        .expect("create second task");

        let tasks = list_tasks_by_role(&pool, "role-a").await.expect("list tasks");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, first.id);
        assert_eq!(tasks[0].role_id.as_deref(), Some("role-a"));
        assert_eq!(tasks[0].title, "准备季度规划");
        assert_eq!(tasks[0].deadline.as_deref(), Some("2026-06-30"));
        assert_eq!(tasks[0].quadrant, "Q1");
        assert!(tasks[0].is_big_rock);
        assert!(!tasks[0].is_completed);
        assert_eq!(tasks[0].sort_order, 0);
        assert_eq!(tasks[0].protection_status, "normal");
    }

    #[tokio::test]
    async fn imminent_escalation_only_returns_non_overridden_q2() {
        let pool = setup_test_db().await;

        // 直接 INSERT 以精确控制 quadrant 与 manual_override（create_task 显式 quadrant 会置 override）。
        let rows = [
            ("t-q2", "Q2", 0, "2026-06-18"),       // 应被返回：临期 Q2 未覆盖
            ("t-q2-override", "Q2", 1, "2026-06-18"), // 排除：手动覆盖
            ("t-q3", "Q3", 0, "2026-06-18"),       // 排除：Q3 不升
            ("t-q4", "Q4", 0, "2026-06-18"),       // 排除：Q4 不升
            ("t-q1", "Q1", 0, "2026-06-18"),       // 排除：已是 Q1
            ("t-q2-future", "Q2", 0, "2026-12-31"),// 排除：未临期
        ];
        for (id, quadrant, override_flag, deadline) in rows {
            sqlx::query(
                "INSERT INTO tasks (id, owner_type, role_id, title, deadline, quadrant, manual_override, created_at, updated_at)
                 VALUES (?1, 'role', 'role-a', ?2, ?3, ?4, ?5, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
            )
            .bind(id)
            .bind(format!("任务 {}", id))
            .bind(deadline)
            .bind(quadrant)
            .bind(override_flag)
            .execute(&pool)
            .await
            .expect("insert task");
        }

        let imminent = list_imminent_tasks_for_escalation(&pool, "2026-06-19")
            .await
            .expect("list imminent");

        let ids: Vec<&str> = imminent.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["t-q2"], "仅未手动覆盖的临期 Q2 任务应被升入 Q1");
    }

    #[tokio::test]
    async fn update_task_changes_only_provided_fields() {
        let pool = setup_test_db().await;
        let task = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "准备季度规划".to_string(),
                deadline: Some("2026-06-30".to_string()),
                quadrant: Some("Q1".to_string()),
                is_big_rock: Some(true),
            },
        )
        .await
        .expect("create task");

        let updated = update_task(
            &pool,
            &task.id,
            &UpdateTaskInput {
                title: Some("更新季度规划".to_string()),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: None,
            },
        )
        .await
        .expect("update task");

        assert_eq!(updated.title, "更新季度规划");
        assert_eq!(updated.deadline.as_deref(), Some("2026-06-30"));
        assert_eq!(updated.quadrant, "Q2");
        assert!(updated.is_big_rock);
        // updated_at 为非空时间戳（秒级精度，不依赖跨秒边界以避免 flaky）
        assert!(!updated.updated_at.is_empty());

        let cleared = update_task(
            &pool,
            &task.id,
            &UpdateTaskInput {
                title: None,
                deadline: Some(None),
                quadrant: None,
                is_big_rock: None,
            },
        )
        .await
        .expect("clear deadline");

        assert_eq!(cleared.deadline, None);
    }

    #[tokio::test]
    async fn soft_delete_hides_task_and_blocks_future_mutation() {
        let pool = setup_test_db().await;
        let task = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "准备季度规划".to_string(),
                deadline: None,
                quadrant: None,
                is_big_rock: None,
            },
        )
        .await
        .expect("create task");

        soft_delete_task(&pool, &task.id).await.expect("soft delete task");
        let tasks = list_tasks_by_role(&pool, "role-a").await.expect("list tasks");
        let update_result = update_task(
            &pool,
            &task.id,
            &UpdateTaskInput {
                title: Some("不应更新".to_string()),
                deadline: None,
                quadrant: None,
                is_big_rock: None,
            },
        )
        .await;
        let delete_result = soft_delete_task(&pool, &task.id).await;

        assert!(tasks.is_empty());
        assert!(matches!(update_result, Err(AppError::NotFound(_))));
        assert!(matches!(delete_result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn create_rejects_empty_title_and_invalid_quadrant() {
        let pool = setup_test_db().await;

        let empty_title = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "  ".to_string(),
                deadline: None,
                quadrant: None,
                is_big_rock: None,
            },
        )
        .await;
        let invalid_quadrant = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "准备季度规划".to_string(),
                deadline: None,
                quadrant: Some("Q5".to_string()),
                is_big_rock: None,
            },
        )
        .await;

        assert!(matches!(empty_title, Err(AppError::ValidationError(_))));
        assert!(matches!(invalid_quadrant, Err(AppError::ValidationError(_))));
    }

    async fn create_simple_task(pool: &SqlitePool, role_id: &str, title: &str) -> Task {
        create_task(
            pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(role_id.to_string()),
                title: title.to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: None,
            },
        )
        .await
        .expect("create task")
    }

    #[tokio::test]
    async fn reorder_tasks_rewrites_sort_order_by_index() {
        let pool = setup_test_db().await;
        let a = create_simple_task(&pool, "role-a", "任务A").await;
        let b = create_simple_task(&pool, "role-a", "任务B").await;
        let c = create_simple_task(&pool, "role-a", "任务C").await;

        reorder_tasks(&pool, &[c.id.clone(), a.id.clone(), b.id.clone()])
            .await
            .expect("reorder tasks");

        let tasks = list_tasks_by_role(&pool, "role-a").await.expect("list tasks");
        let ordered: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ordered, vec![c.id.as_str(), a.id.as_str(), b.id.as_str()]);
        assert_eq!(tasks[0].sort_order, 0);
        assert_eq!(tasks[1].sort_order, 1);
        assert_eq!(tasks[2].sort_order, 2);
    }

    #[tokio::test]
    async fn reorder_tasks_empty_list_is_noop() {
        let pool = setup_test_db().await;
        let result = reorder_tasks(&pool, &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn reorder_tasks_with_invalid_id_rolls_back_without_partial_write() {
        let pool = setup_test_db().await;
        let a = create_simple_task(&pool, "role-a", "任务A").await;
        let b = create_simple_task(&pool, "role-a", "任务B").await;

        // 故意把合法 id 放在非法 id 之前，验证不部分写入
        let result = reorder_tasks(
            &pool,
            &[b.id.clone(), "missing-id".to_string(), a.id.clone()],
        )
        .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));

        let tasks = list_tasks_by_role(&pool, "role-a").await.expect("list tasks");
        // 顺序应保持创建时的原样（a 在前，b 在后），sort_order 未被改写
        assert_eq!(tasks[0].id, a.id);
        assert_eq!(tasks[0].sort_order, 0);
        assert_eq!(tasks[1].id, b.id);
        assert_eq!(tasks[1].sort_order, 1);
    }

    #[tokio::test]
    async fn reorder_tasks_rejects_cross_role_ids() {
        let pool = setup_test_db().await;
        let a = create_simple_task(&pool, "role-a", "任务A").await;
        let b = create_simple_task(&pool, "role-b", "任务B").await;

        let result = reorder_tasks(&pool, &[a.id.clone(), b.id.clone()]).await;
        assert!(matches!(result, Err(AppError::ValidationError(_))));

        // role-b 的任务未被改动
        let tasks_b = list_tasks_by_role(&pool, "role-b").await.expect("list tasks");
        assert_eq!(tasks_b[0].sort_order, 0);
    }

    #[tokio::test]
    async fn reorder_tasks_is_scoped_to_role() {
        let pool = setup_test_db().await;
        let a1 = create_simple_task(&pool, "role-a", "A1").await;
        let a2 = create_simple_task(&pool, "role-a", "A2").await;
        let b1 = create_simple_task(&pool, "role-b", "B1").await;

        reorder_tasks(&pool, &[a2.id.clone(), a1.id.clone()])
            .await
            .expect("reorder role-a");

        let tasks_b = list_tasks_by_role(&pool, "role-b").await.expect("list role-b");
        assert_eq!(tasks_b.len(), 1);
        assert_eq!(tasks_b[0].id, b1.id);
        assert_eq!(tasks_b[0].sort_order, 0);
    }

    #[tokio::test]
    async fn set_task_completion_writes_and_clears_timestamp() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "任务A").await;
        assert!(!task.is_completed);
        assert_eq!(task.completed_at, None);

        let completed = set_task_completion(&pool, &task.id, true)
            .await
            .expect("complete task");
        assert!(completed.is_completed);
        assert!(completed.completed_at.is_some());
        assert_eq!(completed.sort_order, task.sort_order);
        // completed_at 与 updated_at 同步写入同一时间戳（秒级精度，不依赖跨秒边界）
        assert_eq!(completed.completed_at.as_deref(), Some(completed.updated_at.as_str()));

        let reverted = set_task_completion(&pool, &task.id, false)
            .await
            .expect("uncomplete task");
        assert!(!reverted.is_completed);
        assert_eq!(reverted.completed_at, None);
        assert_eq!(reverted.sort_order, task.sort_order);
    }

    #[tokio::test]
    async fn set_task_completion_on_deleted_task_returns_not_found() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "任务A").await;
        soft_delete_task(&pool, &task.id).await.expect("soft delete");

        let result = set_task_completion(&pool, &task.id, true).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn set_task_completion_idempotent_on_same_state() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "任务A").await;

        let completed = set_task_completion(&pool, &task.id, true)
            .await
            .expect("complete task");
        let first_completed_at = completed.completed_at.clone();
        let first_updated_at = completed.updated_at.clone();
        assert!(first_completed_at.is_some());

        // 再次调用 true → 幂等返回，不刷新时间戳
        let again = set_task_completion(&pool, &task.id, true)
            .await
            .expect("idempotent complete");
        assert_eq!(again.completed_at, first_completed_at);
        assert_eq!(again.updated_at, first_updated_at);

        // 撤销后再次撤销也幂等
        let reverted = set_task_completion(&pool, &task.id, false)
            .await
            .expect("uncomplete");
        assert!(reverted.completed_at.is_none());
        let reverted_updated = reverted.updated_at.clone();
        let again_uncomplete = set_task_completion(&pool, &task.id, false)
            .await
            .expect("idempotent uncomplete");
        assert_eq!(again_uncomplete.updated_at, reverted_updated);
    }

    #[tokio::test]
    async fn reorder_tasks_rejects_duplicate_ids() {
        let pool = setup_test_db().await;
        let a = create_simple_task(&pool, "role-a", "任务A").await;
        let b = create_simple_task(&pool, "role-a", "任务B").await;

        let result = reorder_tasks(
            &pool,
            &[a.id.clone(), b.id.clone(), a.id.clone()],
        )
        .await;

        assert!(matches!(result, Err(AppError::ValidationError(_))));

        // 顺序未被改动
        let tasks = list_tasks_by_role(&pool, "role-a").await.expect("list tasks");
        assert_eq!(tasks[0].id, a.id);
        assert_eq!(tasks[0].sort_order, 0);
        assert_eq!(tasks[1].id, b.id);
        assert_eq!(tasks[1].sort_order, 1);
    }

    async fn create_big_rock(pool: &SqlitePool, role_id: &str, title: &str) -> Task {
        create_task(
            pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some(role_id.to_string()),
                title: title.to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(true),
            },
        )
        .await
        .expect("create big rock task")
    }

    #[tokio::test]
    async fn create_fourth_big_rock_returns_validation_error() {
        let pool = setup_test_db().await;
        create_big_rock(&pool, "role-a", "大石头1").await;
        create_big_rock(&pool, "role-a", "大石头2").await;
        create_big_rock(&pool, "role-a", "大石头3").await;

        let result = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "大石头4".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(true),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[tokio::test]
    async fn completing_big_rock_clears_flag_and_frees_slot() {
        let pool = setup_test_db().await;
        let r1 = create_big_rock(&pool, "role-a", "大石头1").await;
        create_big_rock(&pool, "role-a", "大石头2").await;
        create_big_rock(&pool, "role-a", "大石头3").await;

        // 方案 D：完成其中一个大石头后，大石头标记被自动撤销
        let completed = set_task_completion(&pool, &r1.id, true)
            .await
            .expect("complete big rock");
        assert!(completed.is_completed);
        assert!(!completed.is_big_rock, "完成后大石头标记应被撤销");

        // 名额释放：进行中大石头从 3 降为 2
        let count = count_big_rocks_by_owner(&pool, "role", Some("role-a"))
            .await
            .expect("count big rocks");
        assert_eq!(count, 2);

        // 可再标记一个新的大石头，且不触发上限错误
        create_big_rock(&pool, "role-a", "大石头4").await;
        let count_after = count_big_rocks_by_owner(&pool, "role", Some("role-a"))
            .await
            .expect("count big rocks after");
        assert_eq!(count_after, 3);
    }

    #[tokio::test]
    async fn uncompleting_big_rock_stays_non_big_rock() {
        let pool = setup_test_db().await;
        let r1 = create_big_rock(&pool, "role-a", "大石头1").await;

        set_task_completion(&pool, &r1.id, true)
            .await
            .expect("complete big rock");
        // 撤销完成后任务恢复为普通未完成任务，不再是大石头，也不占名额
        let reverted = set_task_completion(&pool, &r1.id, false)
            .await
            .expect("uncomplete big rock");
        assert!(!reverted.is_completed);
        assert!(!reverted.is_big_rock, "撤销完成后不应恢复大石头身份");

        let count = count_big_rocks_by_owner(&pool, "role", Some("role-a"))
            .await
            .expect("count big rocks");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn update_non_big_rock_to_big_rock_at_limit_returns_validation_error() {
        let pool = setup_test_db().await;
        create_big_rock(&pool, "role-a", "大石头1").await;
        create_big_rock(&pool, "role-a", "大石头2").await;
        create_big_rock(&pool, "role-a", "大石头3").await;

        let normal_task = create_simple_task(&pool, "role-a", "普通任务").await;

        let result = update_task(
            &pool,
            &normal_task.id,
            &UpdateTaskInput {
                title: None,
                deadline: None,
                quadrant: None,
                is_big_rock: Some(true),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[tokio::test]
    async fn update_existing_big_rock_without_change_does_not_trigger_validation() {
        let pool = setup_test_db().await;
        let rock = create_big_rock(&pool, "role-a", "大石头1").await;
        create_big_rock(&pool, "role-a", "大石头2").await;
        create_big_rock(&pool, "role-a", "大石头3").await;

        let updated = update_task(
            &pool,
            &rock.id,
            &UpdateTaskInput {
                title: Some("大石头1-改名".to_string()),
                deadline: None,
                quadrant: None,
                is_big_rock: Some(true),
            },
        )
        .await
        .expect("update should succeed");

        assert_eq!(updated.title, "大石头1-改名");
        assert!(updated.is_big_rock);
    }

    #[tokio::test]
    async fn unmark_big_rock_does_not_trigger_validation_even_at_limit() {
        let pool = setup_test_db().await;
        let rock1 = create_big_rock(&pool, "role-a", "大石头1").await;
        create_big_rock(&pool, "role-a", "大石头2").await;
        create_big_rock(&pool, "role-a", "大石头3").await;

        let updated = update_task(
            &pool,
            &rock1.id,
            &UpdateTaskInput {
                title: None,
                deadline: None,
                quadrant: None,
                is_big_rock: Some(false),
            },
        )
        .await
        .expect("unmark should succeed");

        assert!(!updated.is_big_rock);
    }

    #[tokio::test]
    async fn big_rock_limit_is_independent_per_role() {
        let pool = setup_test_db().await;
        create_big_rock(&pool, "role-a", "大石头A1").await;
        create_big_rock(&pool, "role-a", "大石头A2").await;
        create_big_rock(&pool, "role-a", "大石头A3").await;

        let result = create_big_rock(&pool, "role-b", "大石头B1").await;
        assert!(result.is_big_rock);
    }

    async fn create_butler_task(pool: &SqlitePool, title: &str) -> Task {
        create_task(
            pool,
            &CreateTaskInput {
                owner_type: Some(TaskOwnerType::Butler),
                role_id: None,
                title: title.to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: None,
            },
        )
        .await
        .expect("create butler task")
    }

    #[tokio::test]
    async fn butler_tasks_are_listed_separately_from_role_tasks() {
        let pool = setup_test_db().await;
        create_simple_task(&pool, "role-a", "角色任务A").await;
        create_butler_task(&pool, "管家任务1").await;
        create_butler_task(&pool, "管家任务2").await;

        let butler_tasks = list_butler_tasks(&pool).await.expect("list butler tasks");
        assert_eq!(butler_tasks.len(), 2);
        assert!(butler_tasks.iter().all(|t| t.owner_type == "butler"));
        assert!(butler_tasks.iter().all(|t| t.role_id.is_none()));

        let role_tasks = list_tasks_by_role(&pool, "role-a").await.expect("list role tasks");
        assert_eq!(role_tasks.len(), 1);
        assert_eq!(role_tasks[0].owner_type, "role");
    }

    #[tokio::test]
    async fn butler_big_rock_limit_is_independent_from_roles() {
        let pool = setup_test_db().await;
        create_big_rock(&pool, "role-a", "角色大石头1").await;
        create_big_rock(&pool, "role-a", "角色大石头2").await;
        create_big_rock(&pool, "role-a", "角色大石头3").await;

        // 管家可以独立拥有 3 个大石头
        for i in 1..=3 {
            let b = create_task(
                &pool,
                &CreateTaskInput {
                    owner_type: Some(TaskOwnerType::Butler),
                    role_id: None,
                    title: format!("管家大石头{}", i),
                    deadline: None,
                    quadrant: Some("Q2".to_string()),
                    is_big_rock: Some(true),
                },
            )
            .await
            .expect(&format!("create butler big rock {}", i));
            assert!(b.is_big_rock);
        }

        // 第 4 个管家大石头应被拒绝
        let result = create_task(
            &pool,
            &CreateTaskInput {
                owner_type: Some(TaskOwnerType::Butler),
                role_id: None,
                title: "管家大石头4".to_string(),
                deadline: None,
                quadrant: Some("Q2".to_string()),
                is_big_rock: Some(true),
            },
        )
        .await;
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[tokio::test]
    async fn reorder_rejects_mixed_owner_tasks() {
        let pool = setup_test_db().await;
        let role_task = create_simple_task(&pool, "role-a", "角色任务").await;
        let butler_task = create_butler_task(&pool, "管家任务").await;

        let result = reorder_tasks(&pool, &[role_task.id.clone(), butler_task.id.clone()]).await;
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }
}
