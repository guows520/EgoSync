use tauri::State;

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
    bus: State<'_, crate::services::tauri_event_bus::TauriEventBus>,
    secrets: State<'_, crate::services::secret_store_keyring::KeyringSecretStore>,
) -> Result<bool, AppError> {
    // Story 15.2：事件/密钥经 EngineEvents / SecretStore 接缝注入
    briefing_generator::generate_briefing_if_needed(
        &pool,
        &conv_pool,
        Some(bus.inner()),
        secrets.inner(),
    )
    .await
}
