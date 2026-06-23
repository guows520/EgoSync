use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::suggestion::{CreateSuggestionInput, Suggestion, SuggestionWithRole};

const SUGGESTION_SELECT_COLUMNS: &str = "id, role_id, title, content, priority, status, rejection_reason, converted_task_id, created_at";

pub async fn create_suggestion(
    pool: &SqlitePool,
    input: &CreateSuggestionInput,
) -> Result<Suggestion, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO suggestions (id, role_id, title, content, priority, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
    )
    .bind(&id)
    .bind(&input.role_id)
    .bind(&input.title)
    .bind(&input.content)
    .bind(&input.priority)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建建议失败: {}", e)))?;

    get_suggestion(pool, &id).await
}

pub async fn get_suggestion(pool: &SqlitePool, id: &str) -> Result<Suggestion, AppError> {
    sqlx::query_as::<_, Suggestion>(&format!(
        "SELECT {} FROM suggestions WHERE id = ?1",
        SUGGESTION_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询建议失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("建议 {} 不存在", id)))
}

pub async fn list_recent_suggestions(
    pool: &SqlitePool,
    role_id: &str,
    since_iso: &str,
) -> Result<Vec<Suggestion>, AppError> {
    sqlx::query_as::<_, Suggestion>(&format!(
        "SELECT {} FROM suggestions WHERE role_id = ?1 AND created_at >= ?2 ORDER BY created_at DESC",
        SUGGESTION_SELECT_COLUMNS
    ))
    .bind(role_id)
    .bind(since_iso)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询近期建议失败: {}", e)))
}

pub async fn list_pending_suggestions(pool: &SqlitePool) -> Result<Vec<SuggestionWithRole>, AppError> {
    sqlx::query_as::<_, SuggestionWithRole>(
        "SELECT s.id, s.role_id, s.title, s.content, s.priority, s.status,
                s.rejection_reason, s.converted_task_id, s.created_at,
                r.name AS role_name, r.icon AS role_icon, r.color AS role_color
         FROM suggestions s
         INNER JOIN roles r ON s.role_id = r.id
         WHERE s.status = 'pending'
         ORDER BY s.created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询待处理建议失败: {}", e)))
}

pub async fn confirm_suggestion(pool: &SqlitePool, id: &str) -> Result<Suggestion, AppError> {
    let result = sqlx::query("UPDATE suggestions SET status = 'confirmed' WHERE id = ?1 AND status = 'pending'")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("确认建议失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("待处理建议 {} 不存在或已处理", id)));
    }

    get_suggestion(pool, id).await
}

pub async fn reject_suggestion(pool: &SqlitePool, id: &str, reason: &str) -> Result<Suggestion, AppError> {
    let result = sqlx::query(
        "UPDATE suggestions SET status = 'rejected', rejection_reason = ?1 WHERE id = ?2 AND status = 'pending'",
    )
    .bind(reason)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("拒绝建议失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("待处理建议 {} 不存在或已处理", id)));
    }

    get_suggestion(pool, id).await
}

pub async fn list_rejected_suggestions(
    pool: &SqlitePool,
    role_id: &str,
    since_iso: &str,
) -> Result<Vec<Suggestion>, AppError> {
    sqlx::query_as::<_, Suggestion>(&format!(
        "SELECT {} FROM suggestions WHERE role_id = ?1 AND status = 'rejected' AND created_at >= ?2 ORDER BY created_at DESC",
        SUGGESTION_SELECT_COLUMNS
    ))
    .bind(role_id)
    .bind(since_iso)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询已拒绝建议失败: {}", e)))
}

pub async fn set_converted_task_id(
    pool: &SqlitePool,
    suggestion_id: &str,
    task_id: &str,
) -> Result<(), AppError> {
    let result = sqlx::query("UPDATE suggestions SET converted_task_id = ?1 WHERE id = ?2")
        .bind(task_id)
        .bind(suggestion_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("写入 converted_task_id 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("建议 {} 不存在", suggestion_id)));
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

        sqlx::query("INSERT INTO roles (id, name, icon, color) VALUES ('role-a', '产品', '🎯', '#6366F1')")
            .execute(&pool)
            .await
            .expect("failed to insert role-a");
        sqlx::query("INSERT INTO roles (id, name, icon, color) VALUES ('role-b', '学习', '📚', '#10B981')")
            .execute(&pool)
            .await
            .expect("failed to insert role-b");

        sqlx::query(
            "CREATE TABLE suggestions (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                priority TEXT NOT NULL CHECK (priority IN ('high', 'medium', 'low')),
                status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'rejected')),
                rejection_reason TEXT,
                converted_task_id TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create suggestions table");

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

        pool
    }

    async fn insert_suggestion(
        pool: &SqlitePool,
        id: &str,
        role_id: &str,
        title: &str,
        status: &str,
        created_at: &str,
    ) {
        sqlx::query(
            "INSERT INTO suggestions (id, role_id, title, content, priority, status, created_at)
             VALUES (?1, ?2, ?3, '内容', 'medium', ?4, ?5)",
        )
        .bind(id)
        .bind(role_id)
        .bind(title)
        .bind(status)
        .bind(created_at)
        .execute(pool)
        .await
        .expect("failed to insert suggestion");
    }

    #[tokio::test]
    async fn list_pending_returns_only_pending_with_role_info() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "pending", "2026-06-20T10:00:00Z").await;
        insert_suggestion(&pool, "s2", "role-b", "建议B", "pending", "2026-06-21T10:00:00Z").await;
        insert_suggestion(&pool, "s3", "role-a", "建议C", "confirmed", "2026-06-19T10:00:00Z").await;
        insert_suggestion(&pool, "s4", "role-b", "建议D", "rejected", "2026-06-18T10:00:00Z").await;

        let pending = list_pending_suggestions(&pool).await.expect("list pending");
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].id, "s2");
        assert_eq!(pending[0].role_name, "学习");
        assert_eq!(pending[0].role_icon, "📚");
        assert_eq!(pending[0].role_color, "#10B981");
        assert_eq!(pending[1].id, "s1");
        assert_eq!(pending[1].role_name, "产品");
    }

    #[tokio::test]
    async fn list_pending_empty_when_no_pending() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "confirmed", "2026-06-20T10:00:00Z").await;
        let pending = list_pending_suggestions(&pool).await.expect("list pending");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn confirm_suggestion_updates_status() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "pending", "2026-06-20T10:00:00Z").await;

        let confirmed = confirm_suggestion(&pool, "s1").await.expect("confirm");
        assert_eq!(confirmed.status, "confirmed");
        assert_eq!(confirmed.id, "s1");
    }

    #[tokio::test]
    async fn confirm_suggestion_already_confirmed_returns_not_found() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "confirmed", "2026-06-20T10:00:00Z").await;

        let result = confirm_suggestion(&pool, "s1").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn confirm_suggestion_nonexistent_returns_not_found() {
        let pool = setup_test_db().await;
        let result = confirm_suggestion(&pool, "missing").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn reject_suggestion_updates_status_and_reason() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "pending", "2026-06-20T10:00:00Z").await;

        let rejected = reject_suggestion(&pool, "s1", "irrelevant").await.expect("reject");
        assert_eq!(rejected.status, "rejected");
        assert_eq!(rejected.rejection_reason.as_deref(), Some("irrelevant"));
    }

    #[tokio::test]
    async fn reject_suggestion_already_rejected_returns_not_found() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "rejected", "2026-06-20T10:00:00Z").await;

        let result = reject_suggestion(&pool, "s1", "bad_timing").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_rejected_filters_by_role_and_time() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "rejected", "2026-06-20T10:00:00Z").await;
        insert_suggestion(&pool, "s2", "role-a", "建议B", "rejected", "2026-06-10T10:00:00Z").await;
        insert_suggestion(&pool, "s3", "role-b", "建议C", "rejected", "2026-06-20T10:00:00Z").await;
        insert_suggestion(&pool, "s4", "role-a", "建议D", "pending", "2026-06-20T10:00:00Z").await;

        let rejected = list_rejected_suggestions(&pool, "role-a", "2026-06-15T00:00:00Z")
            .await
            .expect("list rejected");
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].id, "s1");
        assert_eq!(rejected[0].title, "建议A");
    }

    #[tokio::test]
    async fn list_rejected_empty_when_no_rejected() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "pending", "2026-06-20T10:00:00Z").await;

        let rejected = list_rejected_suggestions(&pool, "role-a", "2026-01-01T00:00:00Z")
            .await
            .expect("list rejected");
        assert!(rejected.is_empty());
    }

    #[tokio::test]
    async fn set_converted_task_id_updates_field() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "role-a", "建议A", "confirmed", "2026-06-20T10:00:00Z").await;

        set_converted_task_id(&pool, "s1", "task-123").await.expect("set converted task id");

        let suggestion = get_suggestion(&pool, "s1").await.expect("get suggestion");
        assert_eq!(suggestion.converted_task_id.as_deref(), Some("task-123"));
    }

    #[tokio::test]
    async fn set_converted_task_id_nonexistent_returns_not_found() {
        let pool = setup_test_db().await;
        let result = set_converted_task_id(&pool, "missing", "task-123").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
