use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::suggestion::{CreateSuggestionInput, Suggestion};

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
