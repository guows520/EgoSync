use serde::Serialize;
use tauri::State;

use crate::db::app_settings;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::services::scheduler;

#[derive(Debug, Clone, Serialize)]
pub struct SchedulerTimes {
    pub moderate: Vec<String>,
    pub proactive: Vec<String>,
}

#[tauri::command]
pub async fn scheduler_get_times(pool: State<'_, DbPool>) -> Result<SchedulerTimes, AppError> {
    let moderate = scheduler::get_trigger_times(&pool, "moderate")
        .await?
        .unwrap_or_default();
    let proactive = scheduler::get_trigger_times(&pool, "proactive")
        .await?
        .unwrap_or_default();
    Ok(SchedulerTimes { moderate, proactive })
}

#[tauri::command]
pub async fn scheduler_set_times(
    moderate: Vec<String>,
    proactive: Vec<String>,
    pool: State<'_, DbPool>,
) -> Result<SchedulerTimes, AppError> {
    let moderate_validated = scheduler::validate_times(&moderate)?;
    let proactive_validated = scheduler::validate_times(&proactive)?;

    let moderate_json = serde_json::to_string(&moderate_validated)
        .map_err(|e| AppError::DbError(format!("序列化时间点失败: {}", e)))?;
    let proactive_json = serde_json::to_string(&proactive_validated)
        .map_err(|e| AppError::DbError(format!("序列化时间点失败: {}", e)))?;

    app_settings::set_setting(&pool, scheduler::MODERATE_TIMES_KEY, &moderate_json).await?;
    app_settings::set_setting(&pool, scheduler::PROACTIVE_TIMES_KEY, &proactive_json).await?;

    Ok(SchedulerTimes {
        moderate: moderate_validated,
        proactive: proactive_validated,
    })
}
