use crate::error::AppError;
use crate::services::secret_store;

#[tauri::command]
pub async fn secret_store_save(key: String, value: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || secret_store::save_secret(&key, &value))
        .await
        .map_err(|e| AppError::KeyringError(format!("task join error: {}", e)))?
}

#[tauri::command]
pub async fn secret_store_load(key: String) -> Result<Option<String>, AppError> {
    tokio::task::spawn_blocking(move || secret_store::load_secret(&key))
        .await
        .map_err(|e| AppError::KeyringError(format!("task join error: {}", e)))?
}

#[tauri::command]
pub async fn secret_store_delete(key: String) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || secret_store::delete_secret(&key))
        .await
        .map_err(|e| AppError::KeyringError(format!("task join error: {}", e)))?
}
