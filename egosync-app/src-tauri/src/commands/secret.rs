//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::secret`），
//! 壳侧薄化为 wrapper（经注入的 SecretStore 接缝，桌面注入 keyring 实现）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;

#[tauri::command]
pub async fn secret_store_save(
    ctx: State<'_, Arc<EngineCtx>>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    egosync_engine::commands::secret::secret_store_save(&ctx, key, value).await
}

#[tauri::command]
pub async fn secret_store_load(
    ctx: State<'_, Arc<EngineCtx>>,
    key: String,
) -> Result<Option<String>, AppError> {
    egosync_engine::commands::secret::secret_store_load(&ctx, key).await
}

#[tauri::command]
pub async fn secret_store_delete(
    ctx: State<'_, Arc<EngineCtx>>,
    key: String,
) -> Result<(), AppError> {
    egosync_engine::commands::secret::secret_store_delete(&ctx, key).await
}
