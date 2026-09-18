use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::AppError;
use crate::models::task::{ProtectionStatus, Task};
use crate::models::task_decomposition::{
    CreateTaskDecompositionProposalInput, TaskDecompositionItem, TaskDecompositionProposal,
    TaskDecompositionProposalWithRole,
};

const TASK_SELECT_COLUMNS: &str = "id, owner_type, role_id, title, deadline, quadrant, is_big_rock, is_completed, completed_at, sort_order, protection_status, confidence, manual_override, classification_reason, created_at, updated_at, deleted_at";

#[derive(sqlx::FromRow)]
struct ProposalRow {
    id: String,
    role_id: String,
    source_conversation_id: String,
    task_summary: String,
    status: String,
    created_at: String,
    resolved_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ProposalWithRoleRow {
    id: String,
    role_id: String,
    source_conversation_id: String,
    task_summary: String,
    status: String,
    created_at: String,
    resolved_at: Option<String>,
    role_name: String,
    role_icon: String,
    role_color: String,
}

pub async fn create_proposal(
    pool: &SqlitePool,
    input: &CreateTaskDecompositionProposalInput,
) -> Result<TaskDecompositionProposal, AppError> {
    let role_id = required_text(&input.role_id, "角色不能为空")?;
    let conversation_id = required_text(&input.source_conversation_id, "来源会话不能为空")?;
    let task_summary = normalize_title(&input.task_summary)?;
    let items = normalize_items(&input.items)?;
    if items.len() < 2 {
        return Err(AppError::ValidationError(
            "拆分提案必须至少包含两个任务".to_string(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启拆分提案事务失败: {}", e)))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();
    let items_fingerprint = serde_json::to_string(&items)
        .map_err(|e| AppError::DbError(format!("序列化拆分子项失败: {}", e)))?;
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO task_decomposition_proposals
         (id, role_id, source_conversation_id, task_summary, items_fingerprint, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
    )
    .bind(&id)
    .bind(role_id)
    .bind(conversation_id)
    .bind(&task_summary)
    .bind(&items_fingerprint)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::DbError(format!("创建拆分提案失败: {}", e)))?;

    let proposal_id = if inserted.rows_affected() == 1 {
        for (position, item) in items.iter().enumerate() {
            sqlx::query(
                "INSERT INTO task_decomposition_items
                 (id, proposal_id, position, title, deadline)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&id)
            .bind(position as i64)
            .bind(&item.title)
            .bind(item.deadline.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DbError(format!("保存拆分子项失败: {}", e)))?;
        }
        id
    } else {
        sqlx::query_scalar::<_, String>(
            "SELECT id FROM task_decomposition_proposals
             WHERE source_conversation_id = ?1 AND role_id = ?2
               AND task_summary = ?3 AND items_fingerprint = ?4 AND status = 'pending'",
        )
        .bind(conversation_id)
        .bind(role_id)
        .bind(&task_summary)
        .bind(&items_fingerprint)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("查询重复拆分提案失败: {}", e)))?
    };

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交拆分提案事务失败: {}", e)))?;
    get_proposal(pool, &proposal_id).await
}

pub async fn list_pending_by_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<TaskDecompositionProposalWithRole>, AppError> {
    let rows = sqlx::query_as::<_, ProposalWithRoleRow>(
        "SELECT p.id, p.role_id, p.source_conversation_id, p.task_summary,
                p.status, p.created_at, p.resolved_at,
                r.name AS role_name, r.icon AS role_icon, r.color AS role_color
         FROM task_decomposition_proposals p
         INNER JOIN roles r ON r.id = p.role_id
         WHERE p.source_conversation_id = ?1 AND p.status = 'pending'
         ORDER BY p.created_at ASC",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询待处理拆分提案失败: {}", e)))?;

    let mut proposals = Vec::with_capacity(rows.len());
    for row in rows {
        let items = list_items(pool, &row.id).await?;
        proposals.push(TaskDecompositionProposalWithRole {
            proposal: TaskDecompositionProposal {
                id: row.id,
                role_id: row.role_id,
                source_conversation_id: row.source_conversation_id,
                task_summary: row.task_summary,
                items,
                status: row.status,
                created_at: row.created_at,
                resolved_at: row.resolved_at,
            },
            role_name: row.role_name,
            role_icon: row.role_icon,
            role_color: row.role_color,
        });
    }
    Ok(proposals)
}

pub async fn accept_proposal(pool: &SqlitePool, id: &str) -> Result<Vec<Task>, AppError> {
    Ok(accept_proposal_once(pool, id).await?.0)
}

pub async fn accept_proposal_once(
    pool: &SqlitePool,
    id: &str,
) -> Result<(Vec<Task>, bool), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启接受拆分事务失败: {}", e)))?;
    let proposal = get_proposal_row_tx(&mut tx, id).await?;
    if proposal.status == "accepted" {
        let tasks = accepted_tasks_tx(&mut tx, id).await?;
        tx.commit()
            .await
            .map_err(|e| AppError::DbError(format!("提交重复接受事务失败: {}", e)))?;
        return Ok((tasks, false));
    }
    if proposal.status != "pending" {
        return Err(AppError::ValidationError(
            "该拆分提案已选择保持单任务".to_string(),
        ));
    }

    let now = crate::db::settings::chrono_now_pub();
    let claimed = sqlx::query(
        "UPDATE task_decomposition_proposals
         SET status = 'accepted', resolved_at = ?1
         WHERE id = ?2 AND status = 'pending'",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::DbError(format!("锁定拆分提案失败: {}", e)))?;
    if claimed.rows_affected() != 1 {
        return Err(AppError::ValidationError("拆分提案已被处理".to_string()));
    }

    let items = list_item_rows_tx(&mut tx, id).await?;
    let mut next_order = next_sort_order_tx(&mut tx, &proposal.role_id).await?;
    let mut tasks = Vec::with_capacity(items.len());
    for (item_id, item) in items {
        let task = insert_role_task_tx(
            &mut tx,
            &proposal.role_id,
            &item.title,
            item.deadline.as_deref(),
            next_order,
            &now,
        )
        .await?;
        next_order += 1;
        sqlx::query("UPDATE task_decomposition_items SET created_task_id = ?1 WHERE id = ?2")
            .bind(&task.id)
            .bind(item_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DbError(format!("关联拆分任务失败: {}", e)))?;
        tasks.push(task);
    }

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交接受拆分事务失败: {}", e)))?;
    Ok((tasks, true))
}

pub async fn keep_single_proposal(pool: &SqlitePool, id: &str) -> Result<Task, AppError> {
    Ok(keep_single_proposal_once(pool, id).await?.0)
}

pub async fn keep_single_proposal_once(
    pool: &SqlitePool,
    id: &str,
) -> Result<(Task, bool), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启保持单任务事务失败: {}", e)))?;
    let proposal = get_proposal_row_tx(&mut tx, id).await?;
    if proposal.status == "kept_single" {
        let task_id = sqlx::query_scalar::<_, Option<String>>(
            "SELECT single_task_id FROM task_decomposition_proposals WHERE id = ?1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("查询单任务结果失败: {}", e)))?
        .ok_or_else(|| AppError::DbError("保持单任务结果缺少任务 ID".to_string()))?;
        let task = get_task_tx(&mut tx, &task_id).await?;
        tx.commit()
            .await
            .map_err(|e| AppError::DbError(format!("提交重复保持单任务事务失败: {}", e)))?;
        return Ok((task, false));
    }
    if proposal.status != "pending" {
        return Err(AppError::ValidationError(
            "该拆分提案已接受拆分".to_string(),
        ));
    }

    let now = crate::db::settings::chrono_now_pub();
    let claimed = sqlx::query(
        "UPDATE task_decomposition_proposals
         SET status = 'kept_single', resolved_at = ?1
         WHERE id = ?2 AND status = 'pending'",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::DbError(format!("锁定拆分提案失败: {}", e)))?;
    if claimed.rows_affected() != 1 {
        return Err(AppError::ValidationError("拆分提案已被处理".to_string()));
    }

    let sort_order = next_sort_order_tx(&mut tx, &proposal.role_id).await?;
    let task = insert_role_task_tx(
        &mut tx,
        &proposal.role_id,
        &proposal.task_summary,
        None,
        sort_order,
        &now,
    )
    .await?;
    sqlx::query("UPDATE task_decomposition_proposals SET single_task_id = ?1 WHERE id = ?2")
        .bind(&task.id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("关联单任务结果失败: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交保持单任务事务失败: {}", e)))?;
    Ok((task, true))
}

pub async fn get_proposal(
    pool: &SqlitePool,
    id: &str,
) -> Result<TaskDecompositionProposal, AppError> {
    let row = sqlx::query_as::<_, ProposalRow>(
        "SELECT id, role_id, source_conversation_id, task_summary,
                status, created_at, resolved_at
         FROM task_decomposition_proposals WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询拆分提案失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("拆分提案 {} 不存在", id)))?;
    let items = list_items(pool, id).await?;
    Ok(row.into_proposal(items))
}

impl ProposalRow {
    fn into_proposal(self, items: Vec<TaskDecompositionItem>) -> TaskDecompositionProposal {
        TaskDecompositionProposal {
            id: self.id,
            role_id: self.role_id,
            source_conversation_id: self.source_conversation_id,
            task_summary: self.task_summary,
            items,
            status: self.status,
            created_at: self.created_at,
            resolved_at: self.resolved_at,
        }
    }
}

async fn list_items(
    pool: &SqlitePool,
    proposal_id: &str,
) -> Result<Vec<TaskDecompositionItem>, AppError> {
    sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT title, deadline FROM task_decomposition_items
         WHERE proposal_id = ?1 ORDER BY position ASC",
    )
    .bind(proposal_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(title, deadline)| TaskDecompositionItem { title, deadline })
            .collect()
    })
    .map_err(|e| AppError::DbError(format!("查询拆分子项失败: {}", e)))
}

async fn get_proposal_row_tx(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<ProposalRow, AppError> {
    sqlx::query_as::<_, ProposalRow>(
        "SELECT id, role_id, source_conversation_id, task_summary,
                status, created_at, resolved_at
         FROM task_decomposition_proposals WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| AppError::DbError(format!("查询拆分提案失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("拆分提案 {} 不存在", id)))
}

async fn list_item_rows_tx(
    tx: &mut Transaction<'_, Sqlite>,
    proposal_id: &str,
) -> Result<Vec<(String, TaskDecompositionItem)>, AppError> {
    sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT id, title, deadline FROM task_decomposition_items
         WHERE proposal_id = ?1 ORDER BY position ASC",
    )
    .bind(proposal_id)
    .fetch_all(&mut **tx)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(id, title, deadline)| (id, TaskDecompositionItem { title, deadline }))
            .collect()
    })
    .map_err(|e| AppError::DbError(format!("查询拆分子项失败: {}", e)))
}

async fn next_sort_order_tx(
    tx: &mut Transaction<'_, Sqlite>,
    role_id: &str,
) -> Result<i32, AppError> {
    let max_order = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(sort_order) FROM tasks
         WHERE owner_type = 'role' AND role_id = ?1 AND deleted_at IS NULL",
    )
    .bind(role_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::DbError(format!("计算拆分任务排序失败: {}", e)))?;
    Ok(max_order.map_or(0, |value| value + 1))
}

async fn insert_role_task_tx(
    tx: &mut Transaction<'_, Sqlite>,
    role_id: &str,
    title: &str,
    deadline: Option<&str>,
    sort_order: i32,
    now: &str,
) -> Result<Task, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tasks
         (id, owner_type, role_id, title, deadline, quadrant, is_big_rock,
          is_completed, sort_order, protection_status, manual_override,
          created_at, updated_at)
         VALUES (?1, 'role', ?2, ?3, ?4, 'Q2', 0, 0, ?5, ?6, 0, ?7, ?7)",
    )
    .bind(&id)
    .bind(role_id)
    .bind(title)
    .bind(deadline)
    .bind(sort_order)
    .bind(ProtectionStatus::Normal.as_str())
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::DbError(format!("创建拆分任务失败: {}", e)))?;
    get_task_tx(tx, &id).await
}

async fn get_task_tx(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
        TASK_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| AppError::DbError(format!("查询拆分任务失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("任务 {} 不存在", id)))
}

async fn accepted_tasks_tx(
    tx: &mut Transaction<'_, Sqlite>,
    proposal_id: &str,
) -> Result<Vec<Task>, AppError> {
    let task_columns = format!("t.{}", TASK_SELECT_COLUMNS.replace(", ", ", t."));
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks t
         INNER JOIN task_decomposition_items i ON i.created_task_id = t.id
         WHERE i.proposal_id = ?1 AND t.deleted_at IS NULL
         ORDER BY i.position ASC",
        task_columns
    ))
    .bind(proposal_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| AppError::DbError(format!("查询已接受拆分任务失败: {}", e)))
}

fn required_text<'a>(value: &'a str, message: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::ValidationError(message.to_string()));
    }
    Ok(value)
}

fn normalize_title(value: &str) -> Result<String, AppError> {
    let title = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return Err(AppError::ValidationError("任务标题不能为空".to_string()));
    }
    Ok(title)
}

fn normalize_items(
    items: &[TaskDecompositionItem],
) -> Result<Vec<TaskDecompositionItem>, AppError> {
    let mut normalized = Vec::with_capacity(items.len());
    let mut seen = std::collections::HashSet::with_capacity(items.len());
    for item in items {
        let title = normalize_title(&item.title)?;
        let deadline = item
            .deadline
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let key = (title.to_lowercase(), deadline.clone());
        if seen.insert(key) {
            normalized.push(TaskDecompositionItem { title, deadline });
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_task_decomposition_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create test db");
        sqlx::raw_sql(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE roles (
                 id TEXT PRIMARY KEY NOT NULL,
                 name TEXT NOT NULL,
                 icon TEXT NOT NULL DEFAULT 'target',
                 color TEXT NOT NULL DEFAULT '#4F46E5'
             );
             CREATE TABLE tasks (
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
                 deleted_at TEXT,
                 FOREIGN KEY (role_id) REFERENCES roles(id)
             );
             INSERT INTO roles (id, name, icon, color)
             VALUES ('role-a', '家庭', 'home', '#10B981');",
        )
        .execute(&pool)
        .await
        .expect("create base tables");
        sqlx::raw_sql(include_str!(
            "../../migrations/027_task_decomposition_proposals.sql"
        ))
        .execute(&pool)
        .await
        .expect("create proposal tables");
        pool
    }

    fn task_decomposition_input() -> CreateTaskDecompositionProposalInput {
        CreateTaskDecompositionProposalInput {
            role_id: "role-a".to_string(),
            source_conversation_id: "conv-butler".to_string(),
            task_summary: " 安排   家长会 ".to_string(),
            items: vec![
                TaskDecompositionItem {
                    title: " 提前   下班 ".to_string(),
                    deadline: Some("2026-07-17T14:00:00".to_string()),
                },
                TaskDecompositionItem {
                    title: "准备材料".to_string(),
                    deadline: None,
                },
            ],
        }
    }

    #[tokio::test]
    async fn task_decomposition_create_persists_normalized_pending_without_tasks() {
        let pool = setup_task_decomposition_db().await;
        let proposal = create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create proposal");

        assert_eq!(proposal.status, "pending");
        assert_eq!(proposal.task_summary, "安排 家长会");
        assert_eq!(proposal.items[0].title, "提前 下班");
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .expect("count tasks");
        assert_eq!(task_count, 0, "pending 提案不得提前创建任务");
    }

    #[tokio::test]
    async fn task_decomposition_create_is_idempotent_for_same_pending_batch() {
        let pool = setup_task_decomposition_db().await;
        let first = create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create first");
        let second = create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create duplicate");

        assert_eq!(first.id, second.id);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_decomposition_proposals")
            .fetch_one(&pool)
            .await
            .expect("count proposals");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn task_decomposition_list_pending_is_scoped_to_source_conversation() {
        let pool = setup_task_decomposition_db().await;
        create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create proposal");

        let matching = list_pending_by_conversation(&pool, "conv-butler")
            .await
            .expect("list matching");
        let other = list_pending_by_conversation(&pool, "conv-other")
            .await
            .expect("list other");
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].role_name, "家庭");
        assert!(other.is_empty());
    }

    #[tokio::test]
    async fn task_decomposition_accept_creates_all_atomically_and_is_idempotent() {
        let pool = setup_task_decomposition_db().await;
        let proposal = create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create proposal");

        let first = accept_proposal(&pool, &proposal.id)
            .await
            .expect("accept proposal");
        let repeated = accept_proposal(&pool, &proposal.id)
            .await
            .expect("repeat accept");

        assert_eq!(first.len(), 2);
        assert_eq!(
            first.iter().map(|task| &task.id).collect::<Vec<_>>(),
            repeated.iter().map(|task| &task.id).collect::<Vec<_>>()
        );
        assert_eq!(first[0].deadline.as_deref(), Some("2026-07-17T14:00:00"));
        let stored = get_proposal(&pool, &proposal.id)
            .await
            .expect("get proposal");
        assert_eq!(stored.status, "accepted");
        assert!(stored.resolved_at.is_some());
    }

    #[tokio::test]
    async fn task_decomposition_keep_single_uses_original_summary_and_is_idempotent() {
        let pool = setup_task_decomposition_db().await;
        let proposal = create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create proposal");

        let first = keep_single_proposal(&pool, &proposal.id)
            .await
            .expect("keep single");
        let repeated = keep_single_proposal(&pool, &proposal.id)
            .await
            .expect("repeat keep single");

        assert_eq!(first.id, repeated.id);
        assert_eq!(first.title, "安排 家长会");
        assert_eq!(first.deadline, None);
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .expect("count tasks");
        assert_eq!(task_count, 1);
    }

    #[tokio::test]
    async fn task_decomposition_rejects_opposite_terminal_action() {
        let pool = setup_task_decomposition_db().await;
        let proposal = create_proposal(&pool, &task_decomposition_input())
            .await
            .expect("create proposal");
        accept_proposal(&pool, &proposal.id)
            .await
            .expect("accept proposal");

        let result = keep_single_proposal(&pool, &proposal.id).await;
        assert!(matches!(result, Err(AppError::ValidationError(_))));
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .expect("count tasks");
        assert_eq!(task_count, 2);
    }
}
