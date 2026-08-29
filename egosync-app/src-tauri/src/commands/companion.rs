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
    let payload = companion_pairing::generate_qr(&pool).await?;
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

/// 读取中继服务器地址（Story 12.4；`None` = 未配置，中继承载禁用）。
#[tauri::command]
pub async fn companion_get_relay_addr(pool: State<'_, DbPool>) -> Result<Option<String>, AppError> {
    companion_pairing::get_relay_addr(&pool).await
}

/// 配置中继服务器地址（如 `ws://relay.example.com:7333`）；`None`/空白清除。
/// 生效无需重启：中继客户端 ≤5s 慢轮询复查配置与门控。
#[tauri::command]
pub async fn companion_set_relay_addr(
    pool: State<'_, DbPool>,
    relay_addr: Option<String>,
) -> Result<(), AppError> {
    // P6：笔误不得静默入库进无限重连循环——结构校验（scheme/host/port/无路径）
    if let Some(addr) = relay_addr.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        validate_relay_addr(addr)?;
        companion_pairing::set_relay_addr(&pool, Some(addr)).await
    } else {
        companion_pairing::set_relay_addr(&pool, None).await
    }
}

/// 中继地址结构校验（参数解析级，薄层不含业务）：ws(s):// + 非空 host[:port]，
/// 拒绝空白字符与显式路径（服务端自动追加 /relay，携带路径会拼出 `//relay/relay`）。
fn validate_relay_addr(addr: &str) -> Result<(), AppError> {
    const SCHEME_ERR: &str = "中继服务器地址必须以 ws:// 或 wss:// 开头";
    let rest = addr
        .strip_prefix("ws://")
        .or_else(|| addr.strip_prefix("wss://"))
        .ok_or_else(|| AppError::ValidationError(SCHEME_ERR.to_string()))?;
    if rest.contains(char::is_whitespace) {
        return Err(AppError::ValidationError(
            "中继服务器地址不允许包含空白字符".to_string(),
        ));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return Err(AppError::ValidationError(
            "中继服务器地址缺少主机名".to_string(),
        ));
    }
    if authority.contains('@') {
        return Err(AppError::ValidationError(
            "中继服务器地址暂不支持用户信息（user@host）".to_string(),
        ));
    }
    // 端口合法性：host:port 中 port 必须为 u16（顺带拒绝未加括号的 IPv6 多冒号）
    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() || port.parse::<u16>().is_err() {
            return Err(AppError::ValidationError(
                "中继服务器地址的端口无效".to_string(),
            ));
        }
    }
    // 显式路径拒绝（含根路径以外的任何路径/查询/片段）
    if let Some(path) = rest.strip_prefix(authority) {
        if !path.is_empty() && path != "/" {
            return Err(AppError::ValidationError(
                "中继服务器地址不应包含路径（中继端点 /relay 由客户端自动追加）".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_addr_validation_accepts_wellformed() {
        assert!(validate_relay_addr("ws://relay.example.com:7333").is_ok());
        assert!(validate_relay_addr("wss://relay.example.com").is_ok());
        assert!(validate_relay_addr("ws://192.168.1.10:7333").is_ok());
        assert!(validate_relay_addr("ws://relay.example.com:7333/").is_ok()); // 根路径等价无路径
    }

    #[test]
    fn relay_addr_validation_rejects_garbage() {
        // WHY（P6）：裸 scheme/拼错地址此前会静默入库并进入无限重连循环，
        // 用户侧只表现为「离网连不上」而无任何错误反馈——入口必须拒绝。
        assert!(validate_relay_addr("ws://").is_err());
        assert!(validate_relay_addr("ws:// ").is_err()); // 空白主机名
        assert!(validate_relay_addr("ws://foo bar").is_err());
        assert!(validate_relay_addr("ws://host:7333/relay").is_err()); // 显式路径
        assert!(validate_relay_addr("ws://host:port").is_err()); // 非数字端口
        assert!(validate_relay_addr("http://host:7333").is_err()); // scheme 错误
        assert!(validate_relay_addr("ws://u:p@host:7333").is_err()); // userinfo
    }
}
