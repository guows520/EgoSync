//! 手机伴侣配对相关模型（Story 12.2）。
//!
//! `PairedDevice` 对应 `paired_devices` 表（单对单语义由
//! [`crate::db::paired_devices::upsert_single_device`] 落地）；
//! `QrPayload` 是配对二维码内容契约；`CompanionStatus` 供
//! `companion_get_status` 返回连接状态快照。

use serde::{Deserialize, Serialize};

/// 已配对设备记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub id: String,
    pub device_name: String,
    pub device_pubkey: String,
    pub paired_at: String,
    pub last_seen_at: String,
}

/// 配对二维码 payload（前端负责渲染二维码图，后端只产出数据）。
///
/// `relay_addr` 为桌面配置的中继服务器地址（`companion_relay_addr`，
/// 未配置/空白为 `None`）：手机据此在局域网发现失败时回退中继完成
/// 首配（Story 12.5，需桌面确认）与会话连接。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrPayload {
    pub relay_addr: Option<String>,
    pub desktop_static_pubkey: String,
    pub relay_id: String,
    pub pairing_nonce: String,
}

/// 伴侣连接状态快照（`companion_get_status` 返回值）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionStatus {
    pub listening: bool,
    pub port: Option<u16>,
    pub connected: Option<ConnectedDeviceInfo>,
    pub paired_device: Option<PairedDevice>,
    /// 换绑 pending 单槽（新公钥请求替换已配对记录时可见）。
    pub pending_pairing: Option<PendingPairing>,
}

/// 当前已连接设备摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedDeviceInfo {
    pub device_id: String,
    pub device_name: String,
    /// 连接来源：直连（LAN）或中继（本 story 恒 direct）。
    pub origin: String,
    pub since: String,
}

/// 换绑 pending 信息（等待 `pairing_confirm` 确认）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPairing {
    pub device_name: String,
    pub device_pubkey: String,
    /// pending 创建时间（ISO 8601），120 秒超时作废。
    pub created_at: String,
}
