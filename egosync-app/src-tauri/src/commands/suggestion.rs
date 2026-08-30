use tauri::State;

use crate::db::pool::DbPool;
use crate::db::suggestions;
use crate::db::tasks;
use crate::error::AppError;
use crate::models::suggestion::{Suggestion, SuggestionWithRole};
use crate::models::task::{CreateTaskInput, TaskOwnerType};

const ALLOWED_REJECT_REASONS: &[&str] = &["irrelevant", "bad_timing", "already_done", "other"];

#[tauri::command]
pub async fn suggestion_list_pending(conversation_id: String, pool: State<'_, DbPool>) -> Result<Vec<SuggestionWithRole>, AppError> {
    suggestions::list_pending_suggestions(&pool, &conversation_id).await
}

#[tauri::command]
pub async fn suggestion_confirm(id: String, pool: State<'_, DbPool>) -> Result<Suggestion, AppError> {
    confirm_and_create_task(&pool, &id).await
}

#[tauri::command]
pub async fn suggestion_reject(
    id: String,
    reason: String,
    pool: State<'_, DbPool>,
) -> Result<Suggestion, AppError> {
    reject_with_reason(&pool, &id, &reason).await
}

/// 确认建议并创建对应任务。先建任务再 confirm + 回填，避免「已确认但无任务」的静默丢失。
///
/// Story 13.3 受控例外 A：pub 化（纯可见性变更，零行为改动）——
/// companion dispatch 复用唯一入口，避免重写确认编排造成漂移。
pub async fn confirm_and_create_task(pool: &DbPool, id: &str) -> Result<Suggestion, AppError> {
    // 先校验建议处于 pending 状态，避免对已处理建议创建任务
    let suggestion = suggestions::get_suggestion(pool, id).await?;
    if suggestion.status != "pending" {
        return Err(AppError::NotFound(format!("待处理建议 {} 不存在或已处理", id)));
    }

    // 先创建任务：若任务创建失败，建议保持 pending，用户可重试
    let task = tasks::create_task(
        pool,
        &CreateTaskInput {
            owner_type: Some(TaskOwnerType::Role),
            role_id: Some(suggestion.role_id.clone()),
            title: suggestion.title.clone(),
            deadline: None,
            quadrant: None,
            is_big_rock: None,
        },
    )
    .await?;

    // 任务创建成功后再标记 confirmed（pending 守卫防止并发重复处理）
    let confirmed = suggestions::confirm_suggestion(pool, id).await?;
    suggestions::set_converted_task_id(pool, &confirmed.id, &task.id).await?;

    suggestions::get_suggestion(pool, &confirmed.id).await
}

/// 校验拒绝原因后写入拒绝状态。
///
/// Story 13.3 受控例外 A：pub 化（纯可见性变更，零行为改动）。
pub async fn reject_with_reason(pool: &DbPool, id: &str, reason: &str) -> Result<Suggestion, AppError> {
    validate_reject_reason(reason)?;
    suggestions::reject_suggestion(pool, id, reason.trim()).await
}

fn validate_reject_reason(reason: &str) -> Result<(), AppError> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("拒绝原因不能为空".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> DbPool {
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
                conversation_id TEXT,
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

    async fn insert_suggestion(pool: &DbPool, id: &str, status: &str) {
        sqlx::query(
            "INSERT INTO suggestions (id, role_id, title, content, priority, status, created_at)
             VALUES (?1, 'role-a', '写需求文档', '内容', 'medium', ?2, '2026-06-20T10:00:00Z')",
        )
        .bind(id)
        .bind(status)
        .execute(pool)
        .await
        .expect("failed to insert suggestion");
    }

    #[tokio::test]
    async fn confirm_creates_task_and_sets_converted_task_id() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "pending").await;

        let confirmed = confirm_and_create_task(&pool, "s1").await.expect("confirm");
        assert_eq!(confirmed.status, "confirmed");
        let task_id = confirmed.converted_task_id.expect("converted task id set");

        let task_title: String = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ?1")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .expect("task created");
        assert_eq!(task_title, "写需求文档");
    }

    #[tokio::test]
    async fn confirm_already_confirmed_returns_not_found_and_creates_no_task() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "confirmed").await;

        let result = confirm_and_create_task(&pool, "s1").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .expect("count tasks");
        assert_eq!(task_count, 0);
    }

    #[tokio::test]
    async fn confirm_nonexistent_returns_not_found() {
        let pool = setup_test_db().await;
        let result = confirm_and_create_task(&pool, "missing").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn reject_with_valid_reason_updates_status_and_reason() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "pending").await;

        let rejected = reject_with_reason(&pool, "s1", "bad_timing").await.expect("reject");
        assert_eq!(rejected.status, "rejected");
        assert_eq!(rejected.rejection_reason.as_deref(), Some("bad_timing"));
    }

    #[tokio::test]
    async fn reject_with_custom_reason_updates_status_and_reason() {
        let pool = setup_test_db().await;
        insert_suggestion(&pool, "s1", "pending").await;

        let rejected = reject_with_reason(&pool, "s1", "时间不够").await.expect("reject");
        assert_eq!(rejected.status, "rejected");
        assert_eq!(rejected.rejection_reason.as_deref(), Some("时间不够"));
    }

    #[test]
    fn validate_reject_reason_rejects_empty() {
        let result = validate_reject_reason("  ");
        assert!(matches!(result, Err(AppError::ValidationError(msg)) if msg == "拒绝原因不能为空"));
    }

    #[test]
    fn validate_reject_reason_accepts_valid() {
        for reason in ["irrelevant", "bad_timing", "already_done", "other"] {
            assert!(validate_reject_reason(reason).is_ok(), "应接受 {}", reason);
        }
    }

    #[test]
    fn validate_reject_reason_accepts_custom_value() {
        assert!(validate_reject_reason("时间不够").is_ok());
        assert!(validate_reject_reason("unknown_reason").is_ok());
    }
}
