//! 手机伴侣配对服务（Story 12.2）。
//!
//! 职责：静态密钥管理（keyring + 内存缓存）、QR payload 生成、
//! Noise XX responder 握手编排、配对决策（首配 / 免配对重连 / 换绑 pending）。
//!
//! 架构边界：本模块只读写 `db::paired_devices` 与自身状态，不触任何其他
//! service；全部核心函数为无 `AppHandle` 的纯函数（事件 payload 以返回值
//! 产出，由 command / connection 层负责 emit），保证可脱 UI 测试。

use companion_proto::crypto::{generate_static_keypair, HandshakeSession, TransportSession};
use companion_proto::frames::Frame;
use companion_proto::ProtoError;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::db::paired_devices as paired_devices_db;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::companion::{PairedDevice, PendingPairing, QrPayload};
use crate::services::secret_store;

/// keyring 中静态密钥对的 key 名（与 LLM API Key 同级管理）。
pub const KEYRING_KEY: &str = "companion_static_keypair";

/// 换绑 pending 有效期（秒）。
pub const PENDING_PAIRING_TIMEOUT_SECS: i64 = 120;

/// 握手后 app 层未提供设备名时的 fallback。
pub const DEFAULT_DEVICE_NAME: &str = "手机伴侣";

/// `companion:paired` 等事件的 payload 事实源（冻结契约，见 Dev Notes）。
pub const EVENT_PAIRED: &str = "companion:paired";
pub const EVENT_CONNECTED: &str = "companion:connected";
pub const EVENT_DISCONNECTED: &str = "companion:disconnected";

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn proto_err(e: ProtoError) -> AppError {
    // 用户可见文案为中文短句、不暴露技术细节；完整原因走 tracing
    tracing::warn!(error = %e, "companion 协议错误");
    AppError::ProtocolError("手机伴侣通信握手失败".to_string())
}

// ---------------------------------------------------------------------------
// 静态密钥管理
// ---------------------------------------------------------------------------

/// 静态密钥内存缓存。用 RwLock 而非 OnceLock：数据销毁删除 keyring 密钥后
/// 需可失效（P7），OnceLock 无法重置。
static KEYPAIR_CACHE: std::sync::RwLock<Option<(Vec<u8>, Vec<u8>)>> =
    std::sync::RwLock::new(None);

/// 加载（或首次生成并写入 keyring）桌面静态密钥对，hex 编码存储，内存缓存。
pub fn load_or_create_static_keypair() -> Result<(Vec<u8>, Vec<u8>), AppError> {
    if let Some(cached) = KEYPAIR_CACHE
        .read()
        .expect("companion 密钥缓存锁不应中毒")
        .clone()
    {
        return Ok(cached);
    }

    if let Some(stored) = secret_store::load_secret(KEYRING_KEY)? {
        if let Some(keypair) = parse_keypair_hex(&stored) {
            *KEYPAIR_CACHE.write().expect("companion 密钥缓存锁不应中毒") = Some(keypair.clone());
            return Ok(keypair);
        }
        tracing::warn!("companion 静态密钥存储格式非法，将重新生成");
    }

    let (priv_key, pub_key) = generate_static_keypair()
        .map_err(|_| AppError::PairingError("生成桌面配对密钥失败".to_string()))?;
    let encoded = format!("{}:{}", hex_encode(&priv_key), hex_encode(&pub_key));
    secret_store::save_secret(KEYRING_KEY, &encoded)?;
    let keypair = (priv_key, pub_key);
    *KEYPAIR_CACHE.write().expect("companion 密钥缓存锁不应中毒") = Some(keypair.clone());
    tracing::info!(pubkey = %hex_encode(&keypair.1), "companion 静态密钥已就绪");
    Ok(keypair)
}

/// 令内存密钥缓存失效（数据销毁删除 keyring 密钥后调用，下次加载生成新密钥）。
pub fn invalidate_static_keypair_cache() {
    *KEYPAIR_CACHE
        .write()
        .expect("companion 密钥缓存锁不应中毒") = None;
}

fn parse_keypair_hex(stored: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let (priv_hex, pub_hex) = stored.split_once(':')?;
    let priv_key = hex_decode(priv_hex)?;
    let pub_key = hex_decode(pub_hex)?;
    if priv_key.len() != 32 || pub_key.len() != 32 {
        return None;
    }
    Some((priv_key, pub_key))
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

// ---------------------------------------------------------------------------
// QR payload
// ---------------------------------------------------------------------------

/// 生成配对二维码 payload。
///
/// `relay_addr` 来自 app_settings（`companion_relay_addr`，Story 12.4）：
/// 未配置时为 `None`，前端/手机如实降级（离网即 Offline）。
pub fn generate_qr_payload(desktop_static_pubkey: &[u8], relay_addr: Option<String>) -> QrPayload {
    let pubkey_hex = hex_encode(desktop_static_pubkey);
    let relay_id = hex_encode(&Sha256::digest(desktop_static_pubkey))[..16].to_string();
    QrPayload {
        relay_addr,
        desktop_static_pubkey: pubkey_hex,
        relay_id,
        pairing_nonce: uuid::Uuid::new_v4().to_string(),
    }
}

/// 命令层入口：确保静态密钥存在、读取中继配置并产出 QR payload。
pub async fn generate_qr(pool: &DbPool) -> Result<QrPayload, AppError> {
    let (_priv_key, pub_key) = load_or_create_static_keypair()?;
    let relay_addr = get_relay_addr(pool).await?;
    Ok(generate_qr_payload(&pub_key, relay_addr))
}

// ---------------------------------------------------------------------------
// 中继服务器地址配置（Story 12.4，app_settings key-value 复用）
// ---------------------------------------------------------------------------

/// app_settings 中的中继服务器地址键。
pub const RELAY_ADDR_SETTING_KEY: &str = "companion_relay_addr";

/// 读取中继地址（空白视为未配置，返回 `None`）。
pub async fn get_relay_addr(pool: &DbPool) -> Result<Option<String>, AppError> {
    Ok(crate::db::app_settings::get_setting(pool, RELAY_ADDR_SETTING_KEY)
        .await?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty()))
}

/// 写入中继地址：`None`/空白清除配置（中继客户端在 ≤5s 内停止连接尝试）。
pub async fn set_relay_addr(pool: &DbPool, relay_addr: Option<&str>) -> Result<(), AppError> {
    let value = relay_addr.map(str::trim).filter(|v| !v.is_empty()).unwrap_or("");
    crate::db::app_settings::set_setting(pool, RELAY_ADDR_SETTING_KEY, value).await
}

// ---------------------------------------------------------------------------
// Noise XX responder 握手编排
// ---------------------------------------------------------------------------

/// 握手期与传输期的底层 IO 抽象（WS 二进制消息 / 测试通道）。
#[async_trait::async_trait]
pub trait HandshakeIo: Send {
    /// 接收一条二进制消息（握手期为单条 Noise 消息，无长度前缀）。
    async fn recv(&mut self) -> Result<Vec<u8>, AppError>;
    /// 发送一条二进制消息。
    async fn send(&mut self, msg: &[u8]) -> Result<(), AppError>;
}

/// responder 侧握手产物：对端静态公钥（设备身份）+ 传输态会话。
pub struct HandshakeOutcome {
    pub remote_static_pubkey: Vec<u8>,
    pub transport: TransportSession,
}

/// 运行 responder 侧 Noise XX 三消息交换（-> e；<- e,ee,s,es；-> s,se）。
///
/// 每条 Noise 握手消息对应一条底层二进制消息，无长度前缀。
pub async fn run_responder_handshake(
    desktop_priv: &[u8],
    io: &mut dyn HandshakeIo,
) -> Result<HandshakeOutcome, AppError> {
    let mut session = HandshakeSession::responder(desktop_priv).map_err(proto_err)?;
    let m1 = io.recv().await?;
    session.read_message(&m1).map_err(proto_err)?;
    let m2 = session.write_message(&[]).map_err(proto_err)?;
    io.send(&m2).await?;
    let m3 = io.recv().await?;
    session.read_message(&m3).map_err(proto_err)?;
    let remote_static_pubkey = session.remote_static_pubkey().ok_or_else(|| {
        AppError::ProtocolError("手机伴侣通信握手失败".to_string())
    })?;
    let transport = session.into_transport().map_err(proto_err)?;
    Ok(HandshakeOutcome {
        remote_static_pubkey,
        transport,
    })
}

/// 对端静态公钥的 hex 表示（配对决策 / 事件 payload 的设备身份键）。
pub fn remote_pubkey_hex(remote_static_pubkey: &[u8]) -> String {
    hex_encode(remote_static_pubkey)
}

// ---------------------------------------------------------------------------
// 握手后帧级约定（app 层契约，帧 schema 不变）
// ---------------------------------------------------------------------------

/// 握手完成后首帧必须是 HELLO（protocolVersion 校验由 `decode_frame` 内建）。
pub fn validate_first_frame(frame: &Frame) -> Result<(), AppError> {
    match frame {
        Frame::Hello(_) => Ok(()),
        _ => Err(AppError::ProtocolError(
            "手机伴侣协议首帧无效".to_string(),
        )),
    }
}

/// 从 Notice 帧 app 层 data 中提取设备名。
///
/// app 层约定：手机在 HELLO 后以 `Notice(data = {"type":"deviceInfo",
/// "deviceName":"..."})` 上报设备名；协议帧 schema 不变、无需 bump。
pub fn device_name_from_frame(frame: &Frame) -> Option<String> {
    if let Frame::Notice(notice) = frame {
        let value: serde_json::Value = serde_json::from_str(&notice.data).ok()?;
        if value.get("type")?.as_str()? == "deviceInfo" {
            return value.get("deviceName")?.as_str().map(String::from);
        }
    }
    None
}

/// 从 Notice 帧 app 层 data 中提取配对 nonce。
///
/// app 层约定（12.4 手机端契约）：手机在 HELLO 后以 `Notice(data =
/// {"type":"pairingAuth","nonce":"..."})` 提交 QR 中的 pairing_nonce；
/// 协议帧 schema 不变、无需 bump。
pub fn pairing_nonce_from_frame(frame: &Frame) -> Option<String> {
    if let Frame::Notice(notice) = frame {
        let value: serde_json::Value = serde_json::from_str(&notice.data).ok()?;
        if value.get("type")?.as_str()? == "pairingAuth" {
            return value.get("nonce")?.as_str().map(String::from);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 配对决策
// ---------------------------------------------------------------------------

/// `decide_pairing` 的决策结果。
#[derive(Debug, Clone)]
pub enum PairingDecision {
    /// 已配对设备免配对直接进入加密会话（AC4）。
    AlreadyPaired {
        device_id: String,
        device_name: String,
    },
    /// 首次配对：已写入 paired_devices，应发 `companion:paired`。
    FirstPairing { device: PairedDevice },
    /// 换绑：新公钥进入 pending 单槽，等待 `pairing_confirm`（120s 超时作废）。
    PendingRebind { pending: PendingPairing },
}

/// 换绑 pending 是否已过期。
pub fn pending_is_expired(pending: &PendingPairing, now_unix_secs: i64) -> bool {
    parse_iso_to_unix(&pending.created_at)
        .map(|created| now_unix_secs - created > PENDING_PAIRING_TIMEOUT_SECS)
        .unwrap_or(true) // 无法解析的时间视为已作废（显式失败，不静默放行）
}

fn parse_iso_to_unix(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|dt| dt.timestamp())
}

fn now_unix_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 配对决策：同公钥 → 免配对；无任何记录（首配）→ 直接写入；
/// 已有记录且公钥不同（换绑）→ pending 单槽等待确认。
pub async fn decide_pairing(
    pool: &DbPool,
    pending_slot: &Mutex<Option<PendingPairing>>,
    remote_pubkey_hex: &str,
    device_name: &str,
) -> Result<PairingDecision, AppError> {
    // 已配对公钥：免配对直接进入会话（AC4 前置）
    if let Some(existing) = paired_devices_db::get_by_pubkey(pool, remote_pubkey_hex).await? {
        paired_devices_db::update_last_seen(pool, remote_pubkey_hex).await?;
        return Ok(PairingDecision::AlreadyPaired {
            device_id: existing.id,
            device_name: existing.device_name,
        });
    }

    let now = crate::db::settings::chrono_now_pub();
    if paired_devices_db::get_all(pool).await?.is_empty() {
        // 首配：扫码即绑定（FR-40 无桌面确认环节）
        let device = PairedDevice {
            id: uuid::Uuid::new_v4().to_string(),
            device_name: device_name.to_string(),
            device_pubkey: remote_pubkey_hex.to_string(),
            paired_at: now.clone(),
            last_seen_at: now,
        };
        paired_devices_db::upsert_single_device(pool, &device).await?;
        return Ok(PairingDecision::FirstPairing { device });
    }

    // 换绑：pending 单槽（覆盖旧 pending），等待 pairing_confirm。
    // P9：同公钥重连保留原 created_at——120s 作废语义不得被重连无限续期
    //（新扫码走 nonce 校验路径，校验成功时清槽重建 pending，见 connection 层）。
    let pending = {
        let mut guard = pending_slot.lock().await;
        if let Some(existing) = guard.as_ref() {
            if existing.device_pubkey == remote_pubkey_hex {
                return Ok(PairingDecision::PendingRebind {
                    pending: existing.clone(),
                });
            }
        }
        let pending = PendingPairing {
            device_name: device_name.to_string(),
            device_pubkey: remote_pubkey_hex.to_string(),
            created_at: crate::db::settings::chrono_now_pub(),
        };
        *guard = Some(pending.clone());
        pending
    };
    tracing::info!(
        device_name = %pending.device_name,
        "新设备请求替换配对，进入待确认状态"
    );
    Ok(PairingDecision::PendingRebind { pending })
}

/// 确认换绑：pending 存在且未过期时写入 paired_devices（替换旧记录）。
pub async fn confirm_pending(
    pool: &DbPool,
    pending_slot: &Mutex<Option<PendingPairing>>,
) -> Result<PairedDevice, AppError> {
    let mut guard = pending_slot.lock().await;
    let Some(pending) = guard.clone() else {
        return Err(AppError::ValidationError(
            "当前没有等待确认的新设备".to_string(),
        ));
    };
    if pending_is_expired(&pending, now_unix_secs()) {
        *guard = None;
        return Err(AppError::ValidationError(
            "确认已超时，请让手机重新扫码配对".to_string(),
        ));
    }
    let now = crate::db::settings::chrono_now_pub();
    let device = PairedDevice {
        id: uuid::Uuid::new_v4().to_string(),
        device_name: pending.device_name,
        device_pubkey: pending.device_pubkey,
        paired_at: now.clone(),
        last_seen_at: now,
    };
    paired_devices_db::upsert_single_device(pool, &device).await?;
    *guard = None;
    Ok(device)
}

// ---------------------------------------------------------------------------
// 事件 payload（纯函数产出，command / connection 层负责 emit）
// ---------------------------------------------------------------------------

/// 配对窗口：`pairing_generate_qr` 时打开，未知公钥仅在此窗口内被接受。
///
/// 用途：保障"移除即拒绝"——移除已配对设备后，旧公钥重连因无窗口而被拒；
/// 用户重新生成 QR（表达配对意图）后才接受新公钥。
#[derive(Clone)]
pub struct PairingWindow {
    pub nonce: String,
    pub created_at_unix: i64,
}

/// 配对窗口有效期（秒）。
pub const PAIRING_WINDOW_TIMEOUT_SECS: i64 = 300;

/// 配对窗口是否仍在有效期内。
pub fn pairing_window_is_open(window: &Option<PairingWindow>, now_unix_secs: i64) -> bool {
    match window {
        Some(w) => now_unix_secs - w.created_at_unix <= PAIRING_WINDOW_TIMEOUT_SECS,
        None => false,
    }
}

/// 连接准入：公钥为已配对 / 在 pending 单槽中 / 配对窗口打开 ——任一为真即允许。
pub async fn connection_pubkey_allowed(
    pool: &DbPool,
    pending_slot: &Mutex<Option<PendingPairing>>,
    window: &Option<PairingWindow>,
    now_unix_secs: i64,
    remote_pubkey_hex: &str,
) -> Result<bool, AppError> {
    if paired_devices_db::get_by_pubkey(pool, remote_pubkey_hex).await?.is_some() {
        return Ok(true);
    }
    let pending = pending_slot.lock().await;
    if let Some(pending) = &*pending {
        if pending.device_pubkey == remote_pubkey_hex
            && !pending_is_expired(pending, now_unix_secs)
        {
            return Ok(true);
        }
    }
    Ok(pairing_window_is_open(window, now_unix_secs))
}

/// 校验并消费配对窗口 nonce（P0a）：窗口打开且提交的 nonce 与窗口 nonce 一致时
/// 清空窗口（「二维码单次有效」由此落地）并返回 true；其余情况返回 false 且不改动窗口。
pub fn validate_and_consume_window_nonce(
    window: &mut Option<PairingWindow>,
    now_unix_secs: i64,
    submitted_nonce: Option<&str>,
) -> bool {
    let Some(w) = window.as_ref() else {
        return false;
    };
    if !pairing_window_is_open(window, now_unix_secs) {
        return false;
    }
    match submitted_nonce {
        Some(nonce) if nonce == w.nonce => {
            *window = None;
            true
        }
        _ => false,
    }
}

/// `companion:paired` payload：`{ deviceId, deviceName, pubkeyPrefix }`。
pub fn paired_event_payload(device: &PairedDevice) -> serde_json::Value {
    serde_json::json!({
        "deviceId": device.id,
        "deviceName": device.device_name,
        "pubkeyPrefix": device.device_pubkey.chars().take(8).collect::<String>(),
    })
}

/// `companion:connected` payload：`{ deviceId, deviceName }`。
pub fn connected_event_payload(device_id: &str, device_name: &str) -> serde_json::Value {
    serde_json::json!({
        "deviceId": device_id,
        "deviceName": device_name,
    })
}

/// `companion:disconnected` payload：`{ deviceId, reason }`（reason 为中文短句）。
pub fn disconnected_event_payload(device_id: &str, reason: &str) -> serde_json::Value {
    serde_json::json!({
        "deviceId": device_id,
        "reason": reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pool::init_db;
    use companion_proto::frames::{HelloPayload, NoticePayload, PingPayload};
    use tempfile::tempdir;

    async fn test_pool() -> DbPool {
        let dir = tempdir().expect("create temp dir");
        let db_dir = dir.into_path();
        init_db(&db_dir.join("egosync.db"))
            .await
            .expect("init db with migrations")
    }

    fn pending_with_age(device_pubkey: &str, age_secs: i64) -> PendingPairing {
        PendingPairing {
            device_name: "新手机".to_string(),
            device_pubkey: device_pubkey.to_string(),
            created_at: (chrono::Utc::now() - chrono::Duration::seconds(age_secs))
                .to_rfc3339(),
        }
    }

    // ── QR payload ──

    #[test]
    fn qr_payload_fields_match_contract() {
        // WHY: QR 是手机侧唯一配对入口——任一字段缺失/错位，手机都无法
        // 完成握手，且用户无法从 UI 察觉（表现为"扫不上"）。
        let pubkey = [7u8; 32];
        // 未配置中继：relay_addr 必须为 None（前端/手机如实降级）
        let payload = generate_qr_payload(&pubkey, None);
        assert!(payload.relay_addr.is_none(), "未配置时 relay_addr 必须为空");
        assert_eq!(payload.desktop_static_pubkey.len(), 64, "pubkey hex 应为 64 字符");
        let expected_relay_id = hex_encode(&Sha256::digest(pubkey))[..16].to_string();
        assert_eq!(payload.relay_id, expected_relay_id, "relay_id 必须是 pubkey 哈希前 16 字符");
        assert!(!payload.pairing_nonce.is_empty(), "pairing_nonce 不能为空");
        // 配置了中继（Story 12.4）：地址必须逐字透出——手机扫码后经它注册中继
        let with_relay =
            generate_qr_payload(&pubkey, Some("ws://relay.example.com:7333".to_string()));
        assert_eq!(
            with_relay.relay_addr.as_deref(),
            Some("ws://relay.example.com:7333"),
            "relay_addr 必须原样透传，不得改写/补路径"
        );
    }

    #[test]
    fn qr_pairing_nonce_is_single_use() {
        // WHY: nonce 单次有效是"QR 不能被旁人重扫"的安全底线——
        // 两次生成产出相同 nonce 意味着重放窗口被打开。
        let pubkey = [1u8; 32];
        let a = generate_qr_payload(&pubkey, None);
        let b = generate_qr_payload(&pubkey, None);
        assert_ne!(a.pairing_nonce, b.pairing_nonce);
    }

    // ── 握手编排 ──

    /// mpsc 双通道实现的 HandshakeIo 测试替身 + 配对的双端通道。
    struct ChannelIo {
        incoming: tokio::sync::mpsc::Receiver<Vec<u8>>,
        outgoing: tokio::sync::mpsc::Sender<Vec<u8>>,
    }

    #[async_trait::async_trait]
    impl HandshakeIo for ChannelIo {
        async fn recv(&mut self) -> Result<Vec<u8>, AppError> {
            self.incoming
                .recv()
                .await
                .ok_or_else(|| AppError::ConnectionError("连接已关闭".to_string()))
        }
        async fn send(&mut self, msg: &[u8]) -> Result<(), AppError> {
            self.outgoing
                .send(msg.to_vec())
                .await
                .map_err(|_| AppError::ConnectionError("连接已关闭".to_string()))
        }
    }

    #[tokio::test]
    async fn responder_handshake_extracts_remote_static_pubkey() {
        // WHY: 对端静态公钥是配对决策的唯一设备身份——若 responder 拿不到
        // 发起方公钥，"免配对重连"与"移除即拒绝"全部无从谈起。
        let (desktop_priv, _desktop_pub) = generate_static_keypair().unwrap();
        let (phone_priv, phone_pub) = generate_static_keypair().unwrap();

        let (tx_to_responder, rx_to_responder) = tokio::sync::mpsc::channel(4);
        let (tx_to_phone, mut rx_to_phone) = tokio::sync::mpsc::channel(4);
        let mut io = ChannelIo {
            incoming: rx_to_responder,
            outgoing: tx_to_phone,
        };

        let phone_task = tokio::spawn(async move {
            let mut initiator = HandshakeSession::initiator(&phone_priv).unwrap();
            let m1 = initiator.write_message(&[]).unwrap();
            tx_to_responder.send(m1).await.unwrap();
            let m2 = initiator
                .read_message(&rx_to_phone.recv().await.unwrap())
                .unwrap();
            let m3 = initiator.write_message(&[]).unwrap();
            let _ = m2;
            tx_to_responder.send(m3).await.unwrap();
            initiator.into_transport().unwrap()
        });

        let outcome = run_responder_handshake(&desktop_priv, &mut io)
            .await
            .expect("responder handshake");
        assert_eq!(
            hex_encode(&outcome.remote_static_pubkey),
            hex_encode(&phone_pub),
            "提取的对端公钥必须与发起方真实公钥一致"
        );
        phone_task.await.expect("phone task");
    }

    #[tokio::test]
    async fn responder_handshake_rejects_garbage_first_message() {
        // WHY: 错误协议消息必须显式失败（ProtocolError）而非 panic 或
        // 静默继续——错误握手进会话等于加密边界形同虚设。
        let (desktop_priv, _) = generate_static_keypair().unwrap();
        let (tx_to_responder, rx_to_responder) = tokio::sync::mpsc::channel(4);
        let (tx_to_phone, _rx_to_phone) = tokio::sync::mpsc::channel(4);
        let mut io = ChannelIo {
            incoming: rx_to_responder,
            outgoing: tx_to_phone,
        };
        tx_to_responder.send(vec![0u8; 10]).await.unwrap();
        // 注：XX 首条消息（-> e）无 MAC，超长零字节会被 snow 当合法消息+payload
        // 接受；短于 32 字节临时公钥的消息才是确定性协议错误
        drop(tx_to_responder);
        let err = match run_responder_handshake(&desktop_priv, &mut io).await {
            Err(e) => e,
            Ok(_) => panic!("垃圾握手消息必须被拒绝"),
        };
        assert!(matches!(err, AppError::ProtocolError(_)), "实际: {err:?}");
    }

    // ── 帧级约定 ──

    #[test]
    fn first_frame_must_be_hello() {
        let hello = Frame::Hello(HelloPayload {
            protocol_version: companion_proto::PROTOCOL_VERSION,
        });
        assert!(validate_first_frame(&hello).is_ok());
        let ping = Frame::Ping(PingPayload {});
        assert!(validate_first_frame(&ping).is_err());
    }

    #[test]
    fn device_name_extracted_from_notice_data() {
        let notice = Frame::Notice(NoticePayload {
            data: r#"{"type":"deviceInfo","deviceName":"Pixel 8"}"#.to_string(),
        });
        assert_eq!(
            device_name_from_frame(&notice),
            Some("Pixel 8".to_string()),
            "app 层 deviceInfo 约定必须能提取设备名"
        );
        let other = Frame::Notice(NoticePayload {
            data: r#"{"type":"other"}"#.to_string(),
        });
        assert_eq!(device_name_from_frame(&other), None);
        let ping = Frame::Ping(PingPayload {});
        assert_eq!(device_name_from_frame(&ping), None);
    }

    #[test]
    fn pairing_nonce_extracted_from_notice_data() {
        // WHY: pairingAuth 是手机证明「扫过码」的唯一凭据——提取失败意味着
        // nonce 校验链路断裂，配对退化为纯时间窗口放行。
        let notice = Frame::Notice(NoticePayload {
            data: r#"{"type":"pairingAuth","nonce":"n-123"}"#.to_string(),
        });
        assert_eq!(pairing_nonce_from_frame(&notice), Some("n-123".to_string()));
        let device_info = Frame::Notice(NoticePayload {
            data: r#"{"type":"deviceInfo","deviceName":"Pixel"}"#.to_string(),
        });
        assert_eq!(pairing_nonce_from_frame(&device_info), None);
        let ping = Frame::Ping(PingPayload {});
        assert_eq!(pairing_nonce_from_frame(&ping), None);
    }

    #[test]
    fn window_nonce_validation_consumes_only_on_match() {
        // WHY: 「二维码单次有效」的闭环——命中即消费（重放无效）、
        // 未命中不得误伤窗口（用户手滑输错可重试）。
        let now = now_unix_secs();
        let mk_window = || {
            Some(PairingWindow {
                nonce: "n-secret".to_string(),
                created_at_unix: now,
            })
        };
        // 命中 → true 且窗口被消费
        let mut w = mk_window();
        assert!(validate_and_consume_window_nonce(&mut w, now, Some("n-secret")));
        assert!(w.is_none(), "nonce 命中后窗口必须被消费");
        // 不匹配 → false 且窗口保留
        let mut w = mk_window();
        assert!(!validate_and_consume_window_nonce(&mut w, now, Some("wrong")));
        assert!(w.is_some(), "校验失败不得消费窗口");
        // 缺失 → false
        let mut w = mk_window();
        assert!(!validate_and_consume_window_nonce(&mut w, now, None));
        assert!(w.is_some());
        // 过期 → false
        let mut w = Some(PairingWindow {
            nonce: "n-secret".to_string(),
            created_at_unix: now - PAIRING_WINDOW_TIMEOUT_SECS - 1,
        });
        assert!(!validate_and_consume_window_nonce(&mut w, now, Some("n-secret")));
        // 无窗口 → false
        let mut w = None;
        assert!(!validate_and_consume_window_nonce(&mut w, now, Some("n-secret")));
    }

    // ── 配对决策 ──

    #[tokio::test]
    async fn first_pairing_writes_device_directly() {
        // WHY: 首配"扫码即绑定"（FR-40）——若首配也要确认，配对流程
        // 与 PRD 的免输入承诺冲突。
        let pool = test_pool().await;
        let slot = Mutex::new(None);
        let decision = decide_pairing(&pool, &slot, "pub-first", "首台手机")
            .await
            .expect("first pairing");
        match decision {
            PairingDecision::FirstPairing { device } => {
                assert_eq!(device.device_pubkey, "pub-first");
                assert_eq!(device.device_name, "首台手机");
            }
            other => panic!("首配决策必须是 FirstPairing，实际: {other:?}"),
        }
        assert!(
            paired_devices_db::get_by_pubkey(&pool, "pub-first")
                .await
                .unwrap()
                .is_some(),
            "首配必须立即落库"
        );
    }

    #[tokio::test]
    async fn same_pubkey_reconnect_is_already_paired_without_new_record() {
        // WHY: "配对一次，之后免配对"——重连若再次写库/发 paired 事件，
        // 前端会弹出第二次配对提示，信任持久化在用户眼里就是坏的。
        let pool = test_pool().await;
        let slot = Mutex::new(None);
        decide_pairing(&pool, &slot, "pub-a", "手机A")
            .await
            .expect("first pairing");
        let decision = decide_pairing(&pool, &slot, "pub-a", "手机A")
            .await
            .expect("reconnect");
        match decision {
            PairingDecision::AlreadyPaired { .. } => {}
            other => panic!("同公钥重连必须是 AlreadyPaired，实际: {other:?}"),
        }
        assert_eq!(
            paired_devices_db::get_all(&pool).await.unwrap().len(),
            1,
            "重连不得产生第二条配对记录"
        );
    }

    #[tokio::test]
    async fn different_pubkey_goes_pending_then_confirm_replaces() {
        // WHY: "换绑需可见同意"——第三台设备静默顶替绑定等于任何拿到
        // QR 的人都能偷走配对位；pending+confirm 是最小可见同意闸门。
        let pool = test_pool().await;
        let slot = Mutex::new(None);
        decide_pairing(&pool, &slot, "pub-a", "手机A")
            .await
            .expect("first pairing");

        match decide_pairing(&pool, &slot, "pub-b", "手机B")
            .await
            .expect("rebind request")
        {
            PairingDecision::PendingRebind { pending } => {
                assert_eq!(pending.device_pubkey, "pub-b");
            }
            other => panic!("换绑请求必须是 PendingRebind，实际: {other:?}"),
        }
        // 换绑 pending 期间旧设备仍在库中（未确认前不破坏现有绑定）
        assert_eq!(paired_devices_db::get_all(&pool).await.unwrap().len(), 1);

        let confirmed = confirm_pending(&pool, &slot).await.expect("confirm");
        assert_eq!(confirmed.device_pubkey, "pub-b");
        let all = paired_devices_db::get_all(&pool).await.unwrap();
        assert_eq!(all.len(), 1, "确认后必须替换（单对单）");
        assert_eq!(all[0].device_pubkey, "pub-b");
    }

    #[tokio::test]
    async fn confirm_without_pending_is_validation_error() {
        let pool = test_pool().await;
        let slot = Mutex::new(None);
        let err = confirm_pending(&pool, &slot)
            .await
            .expect_err("无 pending 时 confirm 必须失败");
        assert!(matches!(err, AppError::ValidationError(_)), "实际: {err:?}");
    }

    #[tokio::test]
    async fn expired_pending_is_rejected_on_confirm() {
        // WHY: pending 120s 作废——无限期的待确认槽会让"曾经的扫码"
        // 在任意未来时刻被确认，等同重放窗口。
        let pool = test_pool().await;
        let slot = Mutex::new(Some(pending_with_age("pub-new", 999)));
        let err = confirm_pending(&pool, &slot)
            .await
            .expect_err("过期 pending 必须拒绝");
        assert!(matches!(err, AppError::ValidationError(_)), "实际: {err:?}");
        assert!(slot.lock().await.is_none(), "过期 pending 必须被清槽");
    }

    #[tokio::test]
    async fn pending_same_pubkey_reconnect_keeps_created_at() {
        // WHY: pending 120s 作废是重放闸门——若同公钥重连刷新 created_at，
        // 手机持续重连即可把"曾经的扫码"无限延长到任意未来时刻。
        let pool = test_pool().await;
        let slot = Mutex::new(None);
        decide_pairing(&pool, &slot, "pub-a", "手机A")
            .await
            .expect("first pairing");
        // 注入既有 pending（旧 created_at），模拟手机在 pending 期间重连
        *slot.lock().await = Some(pending_with_age("pub-b", 60));
        match decide_pairing(&pool, &slot, "pub-b", "手机B")
            .await
            .expect("pending reconnect")
        {
            PairingDecision::PendingRebind { pending } => {
                let age = now_unix_secs()
                    - parse_iso_to_unix(&pending.created_at).expect("created_at 可解析");
                assert!(age >= 55, "同公钥重连不得刷新 created_at，实际年龄 {age}s");
            }
            other => panic!("同公钥重连必须是 PendingRebind，实际: {other:?}"),
        }
    }

    #[test]
    fn pending_expiry_boundary() {
        let fresh = pending_with_age("pub-a", 10);
        assert!(!pending_is_expired(&fresh, now_unix_secs()));
        let stale = pending_with_age("pub-a", 999);
        assert!(pending_is_expired(&stale, now_unix_secs()));
    }

    // ── 事件 payload ──

    #[test]
    fn event_payloads_match_frozen_contract() {
        // WHY: 事件 payload 形状是前后端冻结契约——字段名漂移会导致
        // 前端监听刷新静默失效（UI 不更新但无报错）。
        let device = PairedDevice {
            id: "d1".to_string(),
            device_name: "手机A".to_string(),
            device_pubkey: "aabbccdd".to_string(),
            paired_at: "t".to_string(),
            last_seen_at: "t".to_string(),
        };
        let paired = paired_event_payload(&device);
        assert_eq!(paired["deviceId"], "d1");
        assert_eq!(paired["deviceName"], "手机A");
        assert_eq!(paired["pubkeyPrefix"], "aabbccdd");

        let connected = connected_event_payload("d1", "手机A");
        assert_eq!(connected["deviceId"], "d1");
        assert_eq!(connected["deviceName"], "手机A");

        let disconnected = disconnected_event_payload("d1", "连接已断开");
        assert_eq!(disconnected["deviceId"], "d1");
        assert_eq!(disconnected["reason"], "连接已断开");
    }
}
