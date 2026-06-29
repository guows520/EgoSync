use tauri::{AppHandle, State};

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::briefing::Briefing;
use crate::services::briefing_generator;

#[tauri::command]
pub async fn briefing_get_latest(pool: State<'_, DbPool>) -> Result<Option<Briefing>, AppError> {
    crate::db::briefings::get_latest_briefing(&pool).await
}

#[tauri::command]
pub async fn briefing_generate_now(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
    app_handle: AppHandle,
) -> Result<bool, AppError> {
    briefing_generator::generate_briefing_if_needed(&pool, &conv_pool, Some(&app_handle)).await
}
