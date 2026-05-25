use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::db::app_settings;
use crate::db::pool::DbPool;
use crate::db::settings;
use crate::error::AppError;
use crate::models::agent::SidecarStatus;
use crate::services::sidecar::SidecarManager;

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

#[tauri::command]
pub async fn app_sidecar_status(
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<SidecarStatus, AppError> {
    let mut mgr = sidecar.lock().await;
    Ok(SidecarStatus {
        running: mgr.is_running(),
        port: mgr.port(),
        uptime_secs: mgr.uptime_secs(),
    })
}
