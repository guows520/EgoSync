use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::task::{CreateTaskInput, CrossRoleTask, ProtectionStatus, Task, TaskOwnerType, UpdateTaskInput};

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
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(&id)
    .bind(owner_type)
    .bind(role_id)
    .bind(title)
    .bind(input.deadline.as_deref())
    .bind(quadrant)
    .bind(is_big_rock)
    .bind(sort_order)
    .bind(ProtectionStatus::Normal.as_str())
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

pub async fn list_all_tasks(
    pool: &SqlitePool,
    quadrant: Option<&str>,
    is_big_rock: Option<bool>,
) -> Result<Vec<CrossRoleTask>, AppError> {
    sqlx::query_as::<_, CrossRoleTask>(
        "SELECT t.id, t.owner_type, t.role_id, t.title, t.deadline, t.quadrant,
                t.is_big_rock, t.is_completed, t.completed_at, t.sort_order,
                t.protection_status, t.confidence, t.manual_override,
                t.classification_reason, t.created_at, t.updated_at, t.deleted_at,
                r.name AS role_name, r.color AS role_color
         FROM tasks t
         LEFT JOIN roles r ON t.role_id = r.id
         WHERE t.deleted_at IS NULL
           AND (t.owner_type = 'butler' OR r.status = 'active')
           AND (?1 IS NULL OR t.quadrant = ?1)
           AND (?2 IS NULL OR t.is_big_rock = ?2)
         ORDER BY t.quadrant ASC, t.is_completed ASC, t.is_big_rock DESC,
                  t.owner_type ASC, t.sort_order ASC",
    )
    .bind(quadrant)
    .bind(is_big_rock)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询全量任务失败: {}", e)))
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
             protection_status = 'normal',
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

    // Story 4.6: 用户编辑任务即处理，清除 Q2 提醒记录
    let _ = crate::db::q2_reminders::delete_reminder_for_task(pool, id).await;

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
             protection_status = 'normal',
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

    // Story 4.6: 用户完成任务即处理，清除 Q2 提醒记录
    let _ = crate::db::q2_reminders::delete_reminder_for_task(pool, id).await;

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

/// Story 4.6：查询所有 `protection_status = 'at_risk'` AND `quadrant = 'Q2'` AND `is_completed = 0`
/// AND `deleted_at IS NULL` 的任务，JOIN `roles` 获取角色信息（需返回角色名用于提醒文案）。
pub async fn list_at_risk_q2_tasks(pool: &SqlitePool) -> Result<Vec<CrossRoleTask>, AppError> {
    sqlx::query_as::<_, CrossRoleTask>(
        "SELECT t.id, t.owner_type, t.role_id, t.title, t.deadline, t.quadrant,
                t.is_big_rock, t.is_completed, t.completed_at, t.sort_order,
                t.protection_status, t.confidence, t.manual_override,
                t.classification_reason, t.created_at, t.updated_at, t.deleted_at,
                r.name AS role_name, r.color AS role_color
         FROM tasks t
         LEFT JOIN roles r ON t.role_id = r.id
         WHERE t.protection_status = ?
           AND t.quadrant = 'Q2'
           AND t.is_completed = 0
           AND t.deleted_at IS NULL
         ORDER BY t.updated_at ASC",
    )
    .bind(ProtectionStatus::AtRisk.as_str())
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 at_risk Q2 任务失败: {}", e)))
}

/// Story 3.5：将「连续过期未处理」的 Q2 任务标记为 `at_risk`。
///
/// 命中条件：quadrant = 'Q2'、未完成、未软删除、`updated_at <= threshold`（即距今 ≥ 阈值天数），
/// 且当前不是 `at_risk`（避免无意义写入）。`threshold` 为完整 ISO 时间戳字符串
/// （格式同 `chrono_now_pub()`：`YYYY-MM-DDTHH:MM:SSZ`），可与 `updated_at` 直接字符串比较。
///
/// **关键：不刷新 `updated_at`**。保护状态完全由 `updated_at` 距今天数派生，若标记时刷新时间戳，
/// 下一轮重算会立即把它判回 `normal`，导致永远标不上 / 状态抖动。
/// 返回被标记的行数。
pub async fn mark_stale_q2_at_risk(
    pool: &SqlitePool,
    threshold: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE tasks
         SET protection_status = ?1
         WHERE quadrant = 'Q2'
           AND is_completed = 0
           AND deleted_at IS NULL
           AND updated_at <= ?2
           AND protection_status != ?3",
    )
    .bind(ProtectionStatus::AtRisk.as_str())
    .bind(threshold)
    .bind(ProtectionStatus::AtRisk.as_str())
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("标记 Q2 任务 at_risk 失败: {}", e)))?;

    Ok(result.rows_affected())
}

/// Story 3.5：将「已被处理 / 不再符合 at_risk 条件」的任务恢复为 `normal`。
///
/// 命中条件：当前为 `at_risk`、未软删除，且满足以下任一「已解除」情形：
/// 非 Q2（如临期升入 Q1）、已完成、或 `updated_at > threshold`（近期被处理过）。
/// 同样**不刷新 `updated_at`**（保护状态是派生量，不应改写交互时间）。返回被恢复的行数。
pub async fn clear_protection_for_resolved(
    pool: &SqlitePool,
    threshold: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE tasks
         SET protection_status = ?1
         WHERE protection_status = ?2
           AND deleted_at IS NULL
           AND (quadrant != 'Q2' OR is_completed = 1 OR updated_at > ?3)",
    )
    .bind(ProtectionStatus::Normal.as_str())
    .bind(ProtectionStatus::AtRisk.as_str())
    .bind(threshold)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("恢复任务 protection_status 失败: {}", e)))?;

    Ok(result.rows_affected())
}

/// 仪表盘批量任务统计：一次 GROUP BY 查询获取所有角色的 pending 任务数和紧急任务数。
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    pub pending_count: i64,
    pub urgent_count: i64,
}

pub async fn get_task_stats_for_roles(
    pool: &SqlitePool,
    role_ids: &[String],
) -> Result<std::collections::HashMap<String, TaskStats>, AppError> {
    if role_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let placeholders = role_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT role_id, COUNT(*) as pending_count,
                SUM(CASE WHEN quadrant = 'Q1' THEN 1 ELSE 0 END) as urgent_count
         FROM tasks
         WHERE owner_type = 'role' AND is_completed = 0 AND deleted_at IS NULL
           AND role_id IN ({})
         GROUP BY role_id",
        placeholders
    );

    let mut query = sqlx::query_as::<_, (String, i64, Option<i64>)>(&sql);
    for id in role_ids {
        query = query.bind(id);
    }

    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::DbError(format!("批量查询任务统计失败: {}", e)))?;

    let mut stats = std::collections::HashMap::new();
    for (role_id, pending_count, urgent_count) in rows {
        stats.insert(
            role_id,
            TaskStats {
                pending_count,
                urgent_count: urgent_count.unwrap_or(0),
            },
        );
    }

    Ok(stats)
}

fn compute_7_days_ago_threshold() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let threshold_secs = now_secs - 7 * 86400;
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

pub async fn count_completed_tasks_in_last_7_days_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<i64, AppError> {
    let threshold = compute_7_days_ago_threshold();
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tasks
         WHERE owner_type = 'role' AND role_id = ?1
           AND is_completed = 1 AND completed_at IS NOT NULL
           AND completed_at >= ?2
           AND deleted_at IS NULL",
    )
    .bind(role_id)
    .bind(&threshold)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询最近7天完成任务数失败: {}", e)))?;
    Ok(count)
}

pub async fn count_total_tasks_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tasks
         WHERE owner_type = 'role' AND role_id = ?1
           AND deleted_at IS NULL",
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色任务总数失败: {}", e)))?;
    Ok(count)
}

pub async fn count_at_risk_q2_tasks_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tasks
         WHERE owner_type = 'role' AND role_id = ?1
           AND protection_status = 'at_risk'
           AND quadrant = 'Q2'
           AND is_completed = 0
           AND deleted_at IS NULL",
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 at_risk Q2 任务数失败: {}", e)))?;
    Ok(count)
}

pub async fn count_active_big_rocks_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tasks
         WHERE owner_type = 'role' AND role_id = ?1
           AND is_big_rock = 1
           AND is_completed = 0
           AND deleted_at IS NULL",
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询活跃大石头数量失败: {}", e)))?;
    Ok(count)
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

        // Story 4.6: q2_reminders 表（update_task / set_task_completion 清除提醒记录测试需要）
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS q2_reminders (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                reminded_count INTEGER NOT NULL DEFAULT 1,
                last_reminded_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                UNIQUE(task_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create q2_reminders table");

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

    // ---- Story 3.5: Q2 保护状态 ----

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

    async fn updated_at_of(pool: &SqlitePool, id: &str) -> String {
        sqlx::query_scalar::<_, String>("SELECT updated_at FROM tasks WHERE id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("fetch updated_at")
    }

    const THRESHOLD: &str = "2026-06-10T00:00:00Z";
    const STALE: &str = "2026-01-01T00:00:00Z"; // <= THRESHOLD，视为过期
    const RECENT: &str = "2026-12-31T00:00:00Z"; // > THRESHOLD，视为近期

    #[tokio::test]
    async fn mark_stale_q2_at_risk_marks_only_stale_incomplete_q2() {
        let pool = setup_test_db().await;
        insert_protection_task(&pool, "q2-stale", "Q2", false, "normal", STALE).await;
        insert_protection_task(&pool, "q2-recent", "Q2", false, "normal", RECENT).await;
        insert_protection_task(&pool, "q2-done", "Q2", true, "normal", STALE).await;
        insert_protection_task(&pool, "q1-stale", "Q1", false, "normal", STALE).await;

        let marked = mark_stale_q2_at_risk(&pool, THRESHOLD).await.expect("mark");

        assert_eq!(marked, 1, "仅过期、未完成的 Q2 任务被标记");
        assert_eq!(protection_status_of(&pool, "q2-stale").await, "at_risk");
        assert_eq!(protection_status_of(&pool, "q2-recent").await, "normal", "近期 Q2 不标记");
        assert_eq!(protection_status_of(&pool, "q2-done").await, "normal", "已完成 Q2 不标记");
        assert_eq!(protection_status_of(&pool, "q1-stale").await, "normal", "非 Q2 不标记");
    }

    #[tokio::test]
    async fn mark_stale_q2_at_risk_does_not_refresh_updated_at() {
        let pool = setup_test_db().await;
        insert_protection_task(&pool, "q2-stale", "Q2", false, "normal", STALE).await;

        mark_stale_q2_at_risk(&pool, THRESHOLD).await.expect("mark");

        assert_eq!(protection_status_of(&pool, "q2-stale").await, "at_risk");
        assert_eq!(
            updated_at_of(&pool, "q2-stale").await,
            STALE,
            "标记 at_risk 不得刷新 updated_at"
        );
    }

    #[tokio::test]
    async fn clear_protection_for_resolved_restores_resolved_tasks() {
        let pool = setup_test_db().await;
        // at_risk 的 Q1（已非 Q2）应恢复 normal
        insert_protection_task(&pool, "q1-at-risk", "Q1", false, "at_risk", STALE).await;
        // at_risk 且近期处理过的 Q2 应恢复 normal
        insert_protection_task(&pool, "q2-recent-at-risk", "Q2", false, "at_risk", RECENT).await;
        // at_risk 且已完成的 Q2 应恢复 normal
        insert_protection_task(&pool, "q2-done-at-risk", "Q2", true, "at_risk", STALE).await;
        // at_risk 且仍过期的 Q2 不应被清除（保持 at_risk）
        insert_protection_task(&pool, "q2-stale-at-risk", "Q2", false, "at_risk", STALE).await;

        let cleared = clear_protection_for_resolved(&pool, THRESHOLD).await.expect("clear");

        assert_eq!(cleared, 3, "三条已解除的应被恢复");
        assert_eq!(protection_status_of(&pool, "q1-at-risk").await, "normal");
        assert_eq!(protection_status_of(&pool, "q2-recent-at-risk").await, "normal");
        assert_eq!(protection_status_of(&pool, "q2-done-at-risk").await, "normal");
        assert_eq!(
            protection_status_of(&pool, "q2-stale-at-risk").await,
            "at_risk",
            "仍过期的 Q2 保持 at_risk"
        );
    }

    #[tokio::test]
    async fn clear_protection_for_resolved_does_not_refresh_updated_at() {
        let pool = setup_test_db().await;
        insert_protection_task(&pool, "q1-at-risk", "Q1", false, "at_risk", RECENT).await;

        clear_protection_for_resolved(&pool, THRESHOLD).await.expect("clear");

        assert_eq!(protection_status_of(&pool, "q1-at-risk").await, "normal");
        assert_eq!(
            updated_at_of(&pool, "q1-at-risk").await,
            RECENT,
            "恢复 normal 不得刷新 updated_at"
        );
    }

    #[tokio::test]
    async fn update_task_resets_protection_status_to_normal() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "Q2 任务").await;
        // 模拟该任务已被标记为 at_risk
        sqlx::query("UPDATE tasks SET protection_status = 'at_risk' WHERE id = ?1")
            .bind(&task.id)
            .execute(&pool)
            .await
            .expect("set at_risk");

        let updated = update_task(
            &pool,
            &task.id,
            &UpdateTaskInput {
                title: Some("改个标题".to_string()),
                deadline: None,
                quadrant: None,
                is_big_rock: None,
            },
        )
        .await
        .expect("update task");

        assert_eq!(updated.protection_status, "normal", "编辑即处理，应复位 normal");
    }

    #[tokio::test]
    async fn set_task_completion_resets_protection_status_to_normal() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "Q2 任务").await;
        sqlx::query("UPDATE tasks SET protection_status = 'at_risk' WHERE id = ?1")
            .bind(&task.id)
            .execute(&pool)
            .await
            .expect("set at_risk");

        let completed = set_task_completion(&pool, &task.id, true)
            .await
            .expect("complete task");

        assert_eq!(completed.protection_status, "normal", "完成即交互，应复位 normal");
    }

    #[tokio::test]
    async fn recompute_protection_status_clears_then_marks_in_one_pass() {
        // 端到端验证 service 层 recompute_protection_status 在单次调用内
        // 先 clear（恢复已解除的）再 mark（标记过期的），且组合幂等：
        // 已是 at_risk 且仍过期的任务保持不变、不重复计入返回值。
        // 使用 2020/2099 这类远离 now±3天 阈值的固定时间戳，避免依赖真实时钟。
        let pool = setup_test_db().await;
        // 过期、未完成、Q2、当前 normal → 应被 mark 为 at_risk（计入返回值）
        insert_protection_task(&pool, "q2-stale-normal", "Q2", false, "normal", "2020-01-01T00:00:00Z").await;
        // 过期、未完成、Q2、当前已是 at_risk → 保持 at_risk，不被 clear，不重复计数
        insert_protection_task(&pool, "q2-stale-at-risk", "Q2", false, "at_risk", "2020-01-01T00:00:00Z").await;
        // at_risk 的 Q1（已非 Q2）→ 应被 clear 回 normal
        insert_protection_task(&pool, "q1-at-risk", "Q1", false, "at_risk", "2020-01-01T00:00:00Z").await;
        // 近期 Q2 normal → 不动
        insert_protection_task(&pool, "q2-recent", "Q2", false, "normal", "2099-01-01T00:00:00Z").await;

        let marked = crate::services::task_protection_watch::recompute_protection_status(&pool)
            .await
            .expect("recompute protection status");

        assert_eq!(marked, 1, "仅本次新标记的过期 Q2 计入返回值（已 at_risk 的不重复计数）");
        assert_eq!(protection_status_of(&pool, "q2-stale-normal").await, "at_risk");
        assert_eq!(protection_status_of(&pool, "q2-stale-at-risk").await, "at_risk", "仍过期保持 at_risk");
        assert_eq!(protection_status_of(&pool, "q1-at-risk").await, "normal", "非 Q2 被清回 normal");
        assert_eq!(protection_status_of(&pool, "q2-recent").await, "normal", "近期 Q2 不标记");
    }

    // ---- Story 3.7: list_all_tasks 跨 owner 查询 ----

    /// 直接 INSERT 一条 butler 任务，精确控制 quadrant/is_completed/is_big_rock/sort_order。
    async fn insert_butler_task_raw(
        pool: &SqlitePool,
        id: &str,
        quadrant: &str,
        is_completed: bool,
        is_big_rock: bool,
        sort_order: i32,
    ) {
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, is_big_rock, sort_order, created_at, updated_at)
             VALUES (?1, 'butler', NULL, ?2, ?3, ?4, ?5, ?6, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .bind(id)
        .bind(format!("管家任务 {}", id))
        .bind(quadrant)
        .bind(is_completed as i32)
        .bind(is_big_rock as i32)
        .bind(sort_order)
        .execute(pool)
        .await
        .expect("insert butler task");
    }

    /// 直接 INSERT 一条角色任务，精确控制各字段。
    async fn insert_role_task_raw(
        pool: &SqlitePool,
        id: &str,
        role_id: &str,
        quadrant: &str,
        is_completed: bool,
        is_big_rock: bool,
        sort_order: i32,
    ) {
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, is_big_rock, sort_order, created_at, updated_at)
             VALUES (?1, 'role', ?2, ?3, ?4, ?5, ?6, ?7, '2026-06-01T00:00:00Z', '2026-06-01T00:00:00Z')",
        )
        .bind(id)
        .bind(role_id)
        .bind(format!("角色任务 {}", id))
        .bind(quadrant)
        .bind(is_completed as i32)
        .bind(is_big_rock as i32)
        .bind(sort_order)
        .execute(pool)
        .await
        .expect("insert role task");
    }

    #[tokio::test]
    async fn list_all_tasks_includes_butler_and_role_tasks() {
        let pool = setup_test_db().await;
        insert_role_task_raw(&pool, "r-q1", "role-a", "Q1", false, false, 0).await;
        insert_butler_task_raw(&pool, "b-q2", "Q2", false, false, 0).await;

        let all = list_all_tasks(&pool, None, None).await.expect("list all");

        assert_eq!(all.len(), 2);
        let role_task = all.iter().find(|t| t.id == "r-q1").expect("role task");
        assert_eq!(role_task.owner_type, "role");
        assert_eq!(role_task.role_name.as_deref(), Some("产品"));
        assert_eq!(role_task.role_color.as_deref(), Some("#4F46E5"));

        let butler_task = all.iter().find(|t| t.id == "b-q2").expect("butler task");
        assert_eq!(butler_task.owner_type, "butler");
        assert_eq!(butler_task.role_name, None);
        assert_eq!(butler_task.role_color, None);
    }

    #[tokio::test]
    async fn list_all_tasks_includes_completed_tasks() {
        let pool = setup_test_db().await;
        insert_role_task_raw(&pool, "r-done", "role-a", "Q1", true, false, 0).await;
        insert_role_task_raw(&pool, "r-todo", "role-a", "Q1", false, false, 1).await;

        let all = list_all_tasks(&pool, None, None).await.expect("list all");

        assert_eq!(all.len(), 2, "已完成任务应包含在结果中");
        let todo_task = all.iter().find(|t| t.id == "r-todo").expect("todo task");
        let done_task = all.iter().find(|t| t.id == "r-done").expect("done task");
        assert!(!todo_task.is_completed);
        assert!(done_task.is_completed);
    }

    #[tokio::test]
    async fn list_all_tasks_filters_by_quadrant() {
        let pool = setup_test_db().await;
        insert_role_task_raw(&pool, "r-q1", "role-a", "Q1", false, false, 0).await;
        insert_role_task_raw(&pool, "r-q2", "role-a", "Q2", false, false, 0).await;
        insert_butler_task_raw(&pool, "b-q1", "Q1", false, false, 0).await;

        let q1_only = list_all_tasks(&pool, Some("Q1"), None).await.expect("list Q1");
        assert_eq!(q1_only.len(), 2);
        assert!(q1_only.iter().all(|t| t.quadrant == "Q1"));
    }

    #[tokio::test]
    async fn list_all_tasks_filters_by_is_big_rock() {
        let pool = setup_test_db().await;
        insert_role_task_raw(&pool, "r-rock", "role-a", "Q2", false, true, 0).await;
        insert_role_task_raw(&pool, "r-normal", "role-a", "Q2", false, false, 1).await;

        let rocks = list_all_tasks(&pool, None, Some(true)).await.expect("list big rocks");
        assert_eq!(rocks.len(), 1);
        assert_eq!(rocks[0].id, "r-rock");
        assert!(rocks[0].is_big_rock);
    }

    #[tokio::test]
    async fn list_all_tasks_excludes_archived_role_tasks() {
        let pool = setup_test_db().await;
        // 插入一个归档角色 + 其任务
        sqlx::query("INSERT INTO roles (id, name, status) VALUES ('role-c', '已归档角色', 'archived')")
            .execute(&pool)
            .await
            .expect("insert archived role");
        insert_role_task_raw(&pool, "r-archived", "role-c", "Q1", false, false, 0).await;
        insert_role_task_raw(&pool, "r-active", "role-a", "Q1", false, false, 0).await;
        insert_butler_task_raw(&pool, "b-active", "Q2", false, false, 0).await;

        let all = list_all_tasks(&pool, None, None).await.expect("list all");

        let ids: Vec<&str> = all.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"r-active"), "活跃角色任务应包含");
        assert!(ids.contains(&"b-active"), "管家任务应包含");
        assert!(!ids.contains(&"r-archived"), "归档角色任务应被排除");
    }

    #[tokio::test]
    async fn list_all_tasks_orders_by_quadrant_then_completed_then_big_rock_then_owner_then_sort() {
        let pool = setup_test_db().await;
        // Q1 未完成大石头 (role-a) — 应排第一
        insert_role_task_raw(&pool, "r-q1-rock", "role-a", "Q1", false, true, 0).await;
        // Q1 未完成普通 (role-a, sort_order=0) — 应排第二
        insert_role_task_raw(&pool, "r-q1-normal", "role-a", "Q1", false, false, 0).await;
        // Q1 已完成 — 应排第三（is_completed ASC: 0在前, 1在后）
        insert_role_task_raw(&pool, "r-q1-done", "role-a", "Q1", true, false, 1).await;
        // Q2 butler 未完成 — Q2 在 Q1 之后
        insert_butler_task_raw(&pool, "b-q2", "Q2", false, false, 0).await;
        // Q2 role-b 未完成 — owner_type 'butler' < 'role' 字典序，但 'butler' 排在前
        insert_role_task_raw(&pool, "r-q2-b", "role-b", "Q2", false, false, 0).await;

        let all = list_all_tasks(&pool, None, None).await.expect("list all");
        let ids: Vec<&str> = all.iter().map(|t| t.id.as_str()).collect();

        // Q1 在 Q2 前
        assert_eq!(ids[0], "r-q1-rock", "Q1 大石头优先");
        assert_eq!(ids[1], "r-q1-normal", "Q1 普通未完成");
        assert_eq!(ids[2], "r-q1-done", "Q1 已完成排最后");
        // Q2: butler 排在 role 前 (owner_type ASC: 'butler' < 'role')
        assert_eq!(ids[3], "b-q2", "Q2 butler 在 role 前");
        assert_eq!(ids[4], "r-q2-b", "Q2 role-b 在 butler 后");
    }

    // ---- Story 4.6: Q2 保护提醒 ----

    #[tokio::test]
    async fn list_at_risk_q2_tasks_returns_only_at_risk_incomplete_q2() {
        let pool = setup_test_db().await;
        // at_risk Q2 未完成 → 应返回
        insert_protection_task(&pool, "q2-at-risk", "Q2", false, "at_risk", STALE).await;
        // normal Q2 未完成 → 排除
        insert_protection_task(&pool, "q2-normal", "Q2", false, "normal", STALE).await;
        // at_risk Q2 已完成 → 排除
        insert_protection_task(&pool, "q2-done", "Q2", true, "at_risk", STALE).await;
        // at_risk Q1 未完成 → 排除
        insert_protection_task(&pool, "q1-at-risk", "Q1", false, "at_risk", STALE).await;

        let tasks = list_at_risk_q2_tasks(&pool).await.expect("list at_risk q2");

        let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["q2-at-risk"], "仅 at_risk 未完成 Q2 返回");
        assert_eq!(tasks[0].role_name.as_deref(), Some("产品"), "应 JOIN 角色名");
    }

    #[tokio::test]
    async fn update_task_clears_q2_reminder_record() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "Q2 任务").await;

        // 写入一条提醒记录
        crate::db::q2_reminders::upsert_reminder(&pool, &task.id)
            .await
            .expect("upsert reminder");

        // 确认记录存在
        let before = crate::db::q2_reminders::get_reminder_for_task(&pool, &task.id)
            .await
            .expect("get before");
        assert!(before.is_some());

        // 更新任务 → 应清除提醒记录
        update_task(
            &pool,
            &task.id,
            &UpdateTaskInput {
                title: Some("改标题".to_string()),
                deadline: None,
                quadrant: None,
                is_big_rock: None,
            },
        )
        .await
        .expect("update task");

        let after = crate::db::q2_reminders::get_reminder_for_task(&pool, &task.id)
            .await
            .expect("get after");
        assert!(after.is_none(), "更新任务后应清除 Q2 提醒记录");
    }

    #[tokio::test]
    async fn toggle_task_complete_clears_q2_reminder_record() {
        let pool = setup_test_db().await;
        let task = create_simple_task(&pool, "role-a", "Q2 任务").await;

        // 写入一条提醒记录
        crate::db::q2_reminders::upsert_reminder(&pool, &task.id)
            .await
            .expect("upsert reminder");

        // 完成任务 → 应清除提醒记录
        set_task_completion(&pool, &task.id, true)
            .await
            .expect("complete task");

        let after = crate::db::q2_reminders::get_reminder_for_task(&pool, &task.id)
            .await
            .expect("get after complete");
        assert!(after.is_none(), "完成任务后应清除 Q2 提醒记录");
    }

    #[tokio::test]
    async fn get_task_stats_for_roles_counts_pending_and_urgent() {
        let pool = setup_test_db().await;

        // role-a: 2 pending (1 Q1 urgent + 1 Q2), 1 completed
        create_task(
            &pool,
            &CreateTaskInput {
                owner_type: None,
                role_id: Some("role-a".to_string()),
                title: "紧急任务".to_string(),
                deadline: None,
                quadrant: Some("Q1".to_string()),
                is_big_rock: None,
            },
        )
        .await
        .expect("create Q1 task");
        create_simple_task(&pool, "role-a", "普通任务").await;
        let completed = create_simple_task(&pool, "role-a", "已完成任务").await;
        set_task_completion(&pool, &completed.id, true)
            .await
            .expect("complete task");

        // role-b: 1 pending Q2
        create_simple_task(&pool, "role-b", "学习任务").await;

        let stats = get_task_stats_for_roles(
            &pool,
            &["role-a".to_string(), "role-b".to_string()],
        )
        .await
        .expect("get stats");

        let a = stats.get("role-a").expect("role-a stats");
        assert_eq!(a.pending_count, 2, "role-a 应有 2 个待办");
        assert_eq!(a.urgent_count, 1, "role-a 应有 1 个紧急 Q1 任务");

        let b = stats.get("role-b").expect("role-b stats");
        assert_eq!(b.pending_count, 1, "role-b 应有 1 个待办");
        assert_eq!(b.urgent_count, 0, "role-b 无紧急任务");
    }

    #[tokio::test]
    async fn get_task_stats_for_roles_empty_returns_empty() {
        let pool = setup_test_db().await;
        let stats = get_task_stats_for_roles(&pool, &[]).await.expect("get stats");
        assert!(stats.is_empty());
    }

    // ---- Story 4.8: 任务统计查询函数测试 ----

    #[allow(clippy::too_many_arguments)]
    async fn insert_energy_test_task(
        pool: &SqlitePool,
        id: &str,
        role_id: &str,
        quadrant: &str,
        is_completed: bool,
        completed_at: Option<&str>,
        is_big_rock: bool,
        protection_status: &str,
        deleted_at: Option<&str>,
    ) {
        sqlx::query(
            "INSERT INTO tasks (id, owner_type, role_id, title, quadrant, is_completed, completed_at, is_big_rock, protection_status, sort_order, created_at, updated_at, deleted_at)
             VALUES (?1, 'role', ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', ?9)",
        )
        .bind(id)
        .bind(role_id)
        .bind(format!("任务 {}", id))
        .bind(quadrant)
        .bind(is_completed as i32)
        .bind(completed_at)
        .bind(is_big_rock as i32)
        .bind(protection_status)
        .bind(deleted_at)
        .execute(pool)
        .await
        .expect("insert energy test task");
    }

    #[tokio::test]
    async fn count_completed_tasks_in_last_7_days_counts_correctly() {
        let pool = setup_test_db().await;
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let old = "2020-01-01T00:00:00Z";

        insert_energy_test_task(&pool, "t-recent", "role-a", "Q1", true, Some(&now), false, "normal", None).await;
        insert_energy_test_task(&pool, "t-old", "role-a", "Q1", true, Some(old), false, "normal", None).await;
        insert_energy_test_task(&pool, "t-incomplete", "role-a", "Q1", false, None, false, "normal", None).await;
        insert_energy_test_task(&pool, "t-deleted", "role-a", "Q1", true, Some(&now), false, "normal", Some("2026-01-01T00:00:00Z")).await;

        let count = count_completed_tasks_in_last_7_days_for_role(&pool, "role-a")
            .await
            .expect("count");
        assert_eq!(count, 1, "仅最近7天完成且未删除的任务应被统计");
    }

    #[tokio::test]
    async fn count_total_tasks_for_role_counts_all_undeleted() {
        let pool = setup_test_db().await;
        insert_energy_test_task(&pool, "t1", "role-a", "Q1", false, None, false, "normal", None).await;
        insert_energy_test_task(&pool, "t2", "role-a", "Q2", true, Some("2026-06-01T00:00:00Z"), false, "normal", None).await;
        insert_energy_test_task(&pool, "t3", "role-a", "Q2", false, None, true, "normal", None).await;
        insert_energy_test_task(&pool, "t4", "role-a", "Q1", false, None, false, "normal", Some("2026-01-01T00:00:00Z")).await;

        let count = count_total_tasks_for_role(&pool, "role-a")
            .await
            .expect("count");
        assert_eq!(count, 3, "已删除任务应被排除");
    }

    #[tokio::test]
    async fn count_at_risk_q2_tasks_for_role_counts_correctly() {
        let pool = setup_test_db().await;
        insert_energy_test_task(&pool, "q2-atrisk", "role-a", "Q2", false, None, false, "at_risk", None).await;
        insert_energy_test_task(&pool, "q2-normal", "role-a", "Q2", false, None, false, "normal", None).await;
        insert_energy_test_task(&pool, "q1-atrisk", "role-a", "Q1", false, None, false, "at_risk", None).await;
        insert_energy_test_task(&pool, "q2-done", "role-a", "Q2", true, Some("2026-06-01T00:00:00Z"), false, "at_risk", None).await;
        insert_energy_test_task(&pool, "q2-atrisk-del", "role-a", "Q2", false, None, false, "at_risk", Some("2026-01-01T00:00:00Z")).await;

        let count = count_at_risk_q2_tasks_for_role(&pool, "role-a")
            .await
            .expect("count");
        assert_eq!(count, 1, "仅 at_risk + Q2 + 未完成 + 未删除");
    }

    #[tokio::test]
    async fn count_active_big_rocks_for_role_counts_correctly() {
        let pool = setup_test_db().await;
        insert_energy_test_task(&pool, "br1", "role-a", "Q2", false, None, true, "normal", None).await;
        insert_energy_test_task(&pool, "br2", "role-a", "Q2", false, None, true, "normal", None).await;
        insert_energy_test_task(&pool, "br-done", "role-a", "Q2", true, Some("2026-06-01T00:00:00Z"), true, "normal", None).await;
        insert_energy_test_task(&pool, "br-del", "role-a", "Q2", false, None, true, "normal", Some("2026-01-01T00:00:00Z")).await;
        insert_energy_test_task(&pool, "normal", "role-a", "Q2", false, None, false, "normal", None).await;

        let count = count_active_big_rocks_for_role(&pool, "role-a")
            .await
            .expect("count");
        assert_eq!(count, 2, "仅 is_big_rock + 未完成 + 未删除");
    }
}
