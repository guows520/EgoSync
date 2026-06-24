use tauri::State;

use crate::db::mission as mission_db;
use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::mission::Mission;
use crate::services::mission_inferrer::{self, InferredValues, InferenceEligibility};

fn validate_format(format: &str) -> Result<(), AppError> {
    if format != "free" && format != "structured" {
        return Err(AppError::ValidationError(format!(
            "无效的 format 值: {}，只允许 'free' 或 'structured'",
            format
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn mission_get(pool: State<'_, DbPool>) -> Result<Option<Mission>, AppError> {
    mission_db::get_mission(&pool).await
}

#[tauri::command]
pub async fn mission_update(
    content: Option<String>,
    format: String,
    pool: State<'_, DbPool>,
) -> Result<Mission, AppError> {
    validate_format(&format)?;
    mission_db::upsert_mission(&pool, content.as_deref(), &format).await
}

#[tauri::command]
pub async fn mission_infer(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Option<InferredValues>, AppError> {
    let outcome = mission_inferrer::infer_values(&pool, &conv_pool).await?;
    Ok(outcome.values)
}

#[tauri::command]
pub async fn mission_infer_eligibility(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<InferenceEligibility, AppError> {
    mission_inferrer::check_eligibility(&pool, &conv_pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_format_free() {
        assert!(validate_format("free").is_ok());
    }

    #[test]
    fn test_validate_format_structured() {
        assert!(validate_format("structured").is_ok());
    }

    #[test]
    fn test_validate_format_invalid_returns_validation_error() {
        let result = validate_format("invalid");
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[test]
    fn test_validate_format_empty_returns_validation_error() {
        let result = validate_format("");
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }
}
