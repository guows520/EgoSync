use tauri::State;

use crate::db::app_settings;
use crate::db::pool::DbPool;
use crate::db::settings;
use crate::error::AppError;

#[tauri::command]
pub async fn app_is_first_launch(pool: State<'_, DbPool>) -> Result<bool, AppError> {
    let value = app_settings::get_setting(&pool, "onboarding_completed").await?;
    Ok(value.as_deref() != Some("true"))
}

#[tauri::command]
pub async fn app_complete_onboarding(pool: State<'_, DbPool>) -> Result<(), AppError> {
    app_settings::set_setting(&pool, "onboarding_completed", "true").await
}

#[tauri::command]
pub async fn app_is_llm_configured(pool: State<'_, DbPool>) -> Result<bool, AppError> {
    let count = settings::count_llm_configs(&pool).await?;
    Ok(count > 0)
}
