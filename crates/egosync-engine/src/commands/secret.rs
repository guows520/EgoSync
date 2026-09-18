//! secret 域命令体（Story 15.4 自壳 `commands/secret.rs` 平移，业务逻辑零
//! 改动；原直调壳侧 keyring 自由函数改经注入的 SecretStore 接缝
//! ——桌面壳注入 KeyringSecretStore 即同款自由函数，spawn_blocking 语义保持）。

use crate::commands::ctx::EngineCtx;
use crate::error::AppError;

pub async fn secret_store_save(
    ctx: &EngineCtx,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let secrets = ctx.secrets.clone();
    tokio::task::spawn_blocking(move || secrets.save_secret(&key, &value))
        .await
        .map_err(|e| AppError::KeyringError(format!("task join error: {}", e)))?
}

pub async fn secret_store_load(ctx: &EngineCtx, key: String) -> Result<Option<String>, AppError> {
    let secrets = ctx.secrets.clone();
    tokio::task::spawn_blocking(move || secrets.load_secret(&key))
        .await
        .map_err(|e| AppError::KeyringError(format!("task join error: {}", e)))?
}

pub async fn secret_store_delete(ctx: &EngineCtx, key: String) -> Result<(), AppError> {
    let secrets = ctx.secrets.clone();
    tokio::task::spawn_blocking(move || secrets.delete_secret(&key))
        .await
        .map_err(|e| AppError::KeyringError(format!("task join error: {}", e)))?
}
