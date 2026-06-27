use tauri::{AppHandle, State};

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::weekly_review::WeeklyReview;
use crate::services::review_generator;

#[tauri::command]
pub async fn review_get_latest(pool: State<'_, DbPool>) -> Result<Option<WeeklyReview>, AppError> {
    crate::db::weekly_reviews::get_latest_weekly_review(&pool).await
}

#[tauri::command]
pub async fn review_get_by_week(
    pool: State<'_, DbPool>,
    week_start: String,
) -> Result<Option<WeeklyReview>, AppError> {
    crate::db::weekly_reviews::get_weekly_review_by_week_start(&pool, &week_start).await
}

#[tauri::command]
pub async fn review_generate_now(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
    app_handle: AppHandle,
) -> Result<bool, AppError> {
    review_generator::generate_review_if_needed(&pool, &conv_pool, Some(&app_handle)).await
}
