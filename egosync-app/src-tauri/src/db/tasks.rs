use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::task::{CreateTaskInput, Task, UpdateTaskInput};

const TASK_SELECT_COLUMNS: &str = "id, role_id, title, deadline, quadrant, is_big_rock, is_completed, completed_at, sort_order, protection_status, confidence, created_at, updated_at, deleted_at";
const ALLOWED_QUADRANTS: &[&str] = &["Q1", "Q2", "Q3", "Q4"];

pub async fn create_task(pool: &SqlitePool, input: &CreateTaskInput) -> Result<Task, AppError> {
    let title = normalized_title(&input.title)?;
    let quadrant = normalized_quadrant(input.quadrant.as_deref())?;
    let is_big_rock = input.is_big_rock.unwrap_or(false);
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();
    let sort_order = next_sort_order(pool, &input.role_id).await?;

    sqlx::query(
        "INSERT INTO tasks (id, role_id, title, deadline, quadrant, is_big_rock, is_completed, sort_order, protection_status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 'normal', ?8, ?9)",
    )
    .bind(&id)
    .bind(&input.role_id)
    .bind(title)
    .bind(input.deadline.as_deref())
    .bind(quadrant)
    .bind(is_big_rock)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建任务失败: {}", e)))?;

    get_active_task(pool, &id).await
}

pub async fn list_tasks_by_role(pool: &SqlitePool, role_id: &str) -> Result<Vec<Task>, AppError> {
    sqlx::query_as::<_, Task>(&format!(
        "SELECT {} FROM tasks WHERE role_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC, created_at ASC",
        TASK_SELECT_COLUMNS
    ))
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
    get_active_task(pool, id).await?;

    let title = match input.title.as_deref() {
        Some(title) => Some(normalized_title(title)?),
        None => None,
    };
    let quadrant = match input.quadrant.as_deref() {
        Some(quadrant) => Some(normalized_quadrant(Some(quadrant))?.to_string()),
        None => None,
    };
    let deadline = input.deadline.as_ref().map(|value| value.as_deref());
    let should_update_deadline = input.deadline.is_some();
    let now = crate::db::settings::chrono_now_pub();

    let result = sqlx::query(
        "UPDATE tasks
         SET title = COALESCE(?1, title),
             deadline = CASE WHEN ?2 THEN ?3 ELSE deadline END,
             quadrant = COALESCE(?4, quadrant),
             is_big_rock = COALESCE(?5, is_big_rock),
             updated_at = ?6
         WHERE id = ?7 AND deleted_at IS NULL",
    )
    .bind(title)
    .bind(should_update_deadline)
    .bind(deadline.flatten())
    .bind(quadrant.as_deref())
    .bind(input.is_big_rock)
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

async fn next_sort_order(pool: &SqlitePool, role_id: &str) -> Result<i32, AppError> {
    let max_order = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(sort_order) FROM tasks WHERE role_id = ?1 AND deleted_at IS NULL",
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("计算任务排序失败: {}", e)))?;

    Ok(max_order.map_or(0, |order| order + 1))
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
                role_id TEXT NOT NULL,
                title TEXT NOT NULL,
                deadline TEXT,
                quadrant TEXT NOT NULL DEFAULT 'Q2' CHECK (quadrant IN ('Q1', 'Q2', 'Q3', 'Q4')),
                is_big_rock INTEGER NOT NULL DEFAULT 0,
                is_completed INTEGER NOT NULL DEFAULT 0,
                completed_at TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                protection_status TEXT NOT NULL DEFAULT 'normal',
                confidence REAL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT,
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
                role_id: "role-a".to_string(),
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
                role_id: "role-b".to_string(),
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
        assert_eq!(tasks[0].role_id, "role-a");
        assert_eq!(tasks[0].title, "准备季度规划");
        assert_eq!(tasks[0].deadline.as_deref(), Some("2026-06-30"));
        assert_eq!(tasks[0].quadrant, "Q1");
        assert!(tasks[0].is_big_rock);
        assert!(!tasks[0].is_completed);
        assert_eq!(tasks[0].sort_order, 0);
        assert_eq!(tasks[0].protection_status, "normal");
    }

    #[tokio::test]
    async fn update_task_changes_only_provided_fields() {
        let pool = setup_test_db().await;
        let task = create_task(
            &pool,
            &CreateTaskInput {
                role_id: "role-a".to_string(),
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
        assert_ne!(updated.updated_at, task.updated_at);

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
                role_id: "role-a".to_string(),
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
                role_id: "role-a".to_string(),
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
                role_id: "role-a".to_string(),
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
}
