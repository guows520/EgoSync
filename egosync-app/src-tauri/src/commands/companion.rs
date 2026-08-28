//! 手机伴侣命令层（Story 12.2）——薄层：参数解析 → service 调用 → 返回，零业务逻辑。

use std::sync::Arc;

use tauri::State;

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::companion::{CompanionStatus, PairedDevice, QrPayload};
use crate::services::companion_connection::{self, CompanionState};
use crate::services::companion_pairing;

#[tauri::command]
pub async fn pairing_generate_qr(
    pool: State<'_, DbPool>,
    companion: State<'_, Arc<CompanionState>>,
) -> Result<QrPayload, AppError> {
    let payload = companion_pairing::generate_qr()?;
    // 开窗即同步 NSD 广播（首配发现路径），窗口过期未消费时自动回收
    companion_connection::open_pairing_window_and_sync(
        &pool,
        &companion,
        payload.pairing_nonce.clone(),
    )
    .await;
    Ok(payload)
}

#[tauri::command]
pub async fn pairing_confirm(
    pool: State<'_, DbPool>,
    companion: State<'_, Arc<CompanionState>>,
) -> Result<PairedDevice, AppError> {
    let device = companion_pairing::confirm_pending(&pool, &companion.pending).await?;
    // 换绑确认后终止旧设备的活跃会话（「移除即拒绝」须覆盖存量连接）
    companion.terminate_active_session(None).await;
    companion.emit_event(
        companion_pairing::EVENT_PAIRED,
        companion_pairing::paired_event_payload(&device),
    );
    Ok(device)
}

#[tauri::command]
pub async fn paired_device_list(pool: State<'_, DbPool>) -> Result<Vec<PairedDevice>, AppError> {
    crate::db::paired_devices::get_all(&pool).await
}

#[tauri::command]
pub async fn paired_device_remove(
    pool: State<'_, DbPool>,
    companion: State<'_, Arc<CompanionState>>,
    device_id: String,
) -> Result<(), AppError> {
    companion_connection::remove_paired_device(&pool, &companion, &device_id).await
}

#[tauri::command]
pub async fn companion_get_status(
    pool: State<'_, DbPool>,
    companion: State<'_, Arc<CompanionState>>,
) -> Result<CompanionStatus, AppError> {
    companion_connection::get_status(&pool, &companion).await
}
