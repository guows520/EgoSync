//! 手机伴侣连接服务（Story 12.2）。
//!
//! 职责：WS 监听、NSD/mDNS 广播、连接状态机。
//! 与 [`crate::services::companion_pairing`] 协作完成配对决策，
//! 事件经注入的 `Option<AppHandle>` 回调发射（无 AppHandle 时跳过，
//! 便于脱 UI 测试）。

use std::net::SocketAddr;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::WebSocketStream;

use companion_proto::frames::{decode_frame, encode_frame, Frame, PingPayload};
use companion_proto::PROTOCOL_VERSION;

use crate::db::paired_devices as paired_devices_db;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::companion::{CompanionStatus, ConnectedDeviceInfo};
use crate::services::companion_pairing::{
    connected_event_payload, connection_pubkey_allowed, decide_pairing, device_name_from_frame,
    disconnected_event_payload, generate_qr_payload, load_or_create_static_keypair,
    paired_event_payload, pairing_nonce_from_frame, pending_is_expired,
    validate_and_consume_window_nonce, validate_first_frame, HandshakeIo, PairingDecision,
    PairingWindow, DEFAULT_DEVICE_NAME, EVENT_CONNECTED, EVENT_DISCONNECTED, EVENT_PAIRED,
    PAIRING_WINDOW_TIMEOUT_SECS,
};
use crate::models::companion::{PairedDevice, PendingPairing};

/// NSD/mDNS 服务类型（三端一致）。
const NSD_SERVICE_TYPE: &str = "_egosync._tcp.local.";

/// Noise 握手与协议首帧等待超时（秒）——半开/恶意慢连接不得长期占用资源。
const HANDSHAKE_TIMEOUT_SECS: u64 = 10;

/// 会话帧循环空闲超时（秒）——超过此时长未收到任何帧（含 PING）视为断连。
const SESSION_IDLE_TIMEOUT_SECS: u64 = 120;

/// 握手后等待 app 层 Notice（deviceInfo / pairingAuth）的收集窗口（秒）。
const APP_NOTICE_WINDOW_SECS: u64 = 3;

/// 连接状态机（`Arc<RwLock<...>>` managed state）。
#[derive(Clone, Debug)]
pub enum CompanionConnectionState {
    Listening,
    Connected {
        device_id: String,
        device_name: String,
        origin: String,
        since: String,
        peer_addr: String,
    },
    /// WS 监听启动失败（降级）——状态查询如实反映，不伪装 Listening。
    Failed,
}

/// 当前活跃会话句柄：remove / confirm / 新连接可经 terminate 信号终止会话。
pub struct ActiveSession {
    pub device_id: String,
    terminate: tokio::sync::watch::Sender<bool>,
}

impl ActiveSession {
    fn terminate(&self) {
        let _ = self.terminate.send(true);
    }
}

struct NsdHandle {
    daemon: ServiceDaemon,
    full_name: String,
}

/// 桌面伴侣全局状态（`lib.rs` 中 `app.manage(Arc<CompanionState>)`）。
pub struct CompanionState {
    pub connection: std::sync::RwLock<CompanionConnectionState>,
    pub pending: tokio::sync::Mutex<Option<PendingPairing>>,
    pub pairing_window: tokio::sync::Mutex<Option<PairingWindow>>,
    pub desktop_static_priv: Vec<u8>,
    pub desktop_static_pub: Vec<u8>,
    pub relay_id: String,
    pub app_handle: Option<AppHandle>,
    nsd: tokio::sync::Mutex<Option<NsdHandle>>,
    listen_port: std::sync::atomic::AtomicU16,
    active_session: tokio::sync::Mutex<Option<ActiveSession>>,
}

impl CompanionState {
    /// 生产构造：从 keyring 加载/生成静态密钥。
    pub fn new(app_handle: Option<AppHandle>) -> Result<Self, AppError> {
        let (priv_key, pub_key) = load_or_create_static_keypair()?;
        Ok(Self::with_static_keypair(app_handle, priv_key, pub_key))
    }

    /// 测试构造：注入固定静态密钥（不触 keyring）。
    pub fn with_static_keypair_for_testing(
        app_handle: Option<AppHandle>,
        priv_key: Vec<u8>,
        pub_key: Vec<u8>,
    ) -> Self {
        Self::with_static_keypair(app_handle, priv_key, pub_key)
    }

    fn with_static_keypair(
        app_handle: Option<AppHandle>,
        priv_key: Vec<u8>,
        pub_key: Vec<u8>,
    ) -> Self {
        let relay_id = generate_qr_payload(&pub_key).relay_id;
        Self {
            connection: std::sync::RwLock::new(CompanionConnectionState::Listening),
            pending: tokio::sync::Mutex::new(None),
            pairing_window: tokio::sync::Mutex::new(None),
            desktop_static_priv: priv_key,
            desktop_static_pub: pub_key,
            relay_id,
            app_handle,
            nsd: tokio::sync::Mutex::new(None),
            listen_port: std::sync::atomic::AtomicU16::new(0),
            active_session: tokio::sync::Mutex::new(None),
        }
    }

    /// 打开配对窗口（`pairing_generate_qr` 命令调用）。
    pub async fn open_pairing_window(&self, nonce: String) {
        let now = chrono::Utc::now().timestamp();
        *self.pairing_window.lock().await = Some(PairingWindow {
            nonce,
            created_at_unix: now,
        });
    }

    /// 标记监听启动失败（状态查询如实反映，不再伪装 Listening）。
    pub fn set_failed(&self) {
        *self.connection.write().unwrap() = CompanionConnectionState::Failed;
    }

    /// 终止当前活跃会话：`device_id` 为 `None` 时终止任意当前会话。
    /// 槽位清理由会话任务自身完成（退出时校验仍为当前会话后才复位状态机），
    /// 避免此处过早清槽导致会话退出时误判为「已被取代」而跳过状态复位。
    pub async fn terminate_active_session(&self, device_id: Option<&str>) {
        let guard = self.active_session.lock().await;
        if let Some(session) = guard.as_ref() {
            let matched = device_id
                .map(|id| id == session.device_id)
                .unwrap_or(true);
            if matched {
                session.terminate();
            }
        }
    }

    fn now(&self) -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn listen_port(&self) -> u16 {
        self.listen_port
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 发射 companion 事件（AppHandle 缺省时跳过——脱 UI 测试路径）。
    pub fn emit_event(&self, event: &str, payload: serde_json::Value) {
        self.emit(event, payload);
    }

    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Some(handle) = &self.app_handle {
            if let Err(e) = handle.emit(event, payload) {
                tracing::warn!(event = event, error = %e, "companion 事件发射失败");
            }
        }
    }

    fn set_listening(&self) {
        *self.connection.write().unwrap() = CompanionConnectionState::Listening;
    }

    fn set_connected(
        &self,
        device_id: &str,
        device_name: &str,
        origin: &str,
        peer_addr: &str,
    ) {
        *self.connection.write().unwrap() = CompanionConnectionState::Connected {
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            origin: origin.to_string(),
            since: crate::db::settings::chrono_now_pub(),
            peer_addr: peer_addr.to_string(),
        };
    }
}

/// 状态查询（`companion_get_status` 数据源）。
pub async fn get_status(
    pool: &DbPool,
    state: &Arc<CompanionState>,
) -> Result<CompanionStatus, AppError> {
    let (listening, connected) = {
        let connection = state
            .connection
            .read()
            .map_err(|e| AppError::ConnectionError(format!("读取连接状态失败: {}", e)))?;
        match &*connection {
            CompanionConnectionState::Listening => (true, None),
            CompanionConnectionState::Failed => (false, None),
            CompanionConnectionState::Connected {
                device_id,
                device_name,
                since,
                ..
            } => (
                false,
                Some(ConnectedDeviceInfo {
                    device_id: device_id.clone(),
                    device_name: device_name.clone(),
                    origin: "direct".to_string(),
                    since: since.clone(),
                }),
            ),
        }
    };

    // DB 错误显式传播——吞掉会让状态面板谎报「暂未配对」；
    // get_all 为 paired_at DESC，首条即最新配对（pop() 会取到最旧）。
    let paired_device = paired_devices_db::get_all(pool)
        .await?
        .into_iter()
        .next();

    let pending = {
        let guard = state.pending.lock().await;
        if let Some(p) = &*guard {
            if !pending_is_expired(p, state.now()) {
                Some(p.clone())
            } else {
                None
            }
        } else {
            None
        }
    };

    let port = state.listen_port();
    Ok(CompanionStatus {
        listening,
        port: if port == 0 { None } else { Some(port) },
        connected,
        paired_device,
        pending_pairing: pending,
    })
}

// ---------------------------------------------------------------------------
// WS 监听与连接处理
// ---------------------------------------------------------------------------

/// 启动 WS 监听（动态端口）+ NSD 广播（若已有配对设备）。
///
/// 在 `lib.rs` setup 中以非阻塞 spawn 调用，失败仅 warn 降级。
pub async fn start_companion_listener(
    pool: DbPool,
    state: Arc<CompanionState>,
) -> Result<u16, AppError> {
    let listener = TcpListener::bind(("0.0.0.0", 0))
        .await
        .map_err(|e| AppError::ConnectionError(format!("WS 监听端口绑定失败: {}", e)))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::ConnectionError(format!("读取监听端口失败: {}", e)))?
        .port();
    state
        .listen_port
        .store(port, std::sync::atomic::Ordering::Relaxed);

    tracing::info!(port, "companion WS 监听已启动");

    // 已配对设备或配对窗口打开时注册 NSD 广播（P0b：窗口是内存态，
    // 冷启动时通常仅已配对设备生效）
    sync_nsd_registration(&pool, &state).await;

    let pool_for_loop = pool.clone();
    let state_for_loop = state.clone();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let pool_c = pool_for_loop.clone();
                    let state_c = state_for_loop.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(pool_c, state_c, stream, peer_addr).await
                        {
                            tracing::warn!(peer = %peer_addr, error = %e, "companion 连接处理失败");
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!(error = %e, "companion WS accept 失败，1s 后重试");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    Ok(port)
}

/// WS 二进制 IO 适配 `HandshakeIo`，复用同一 stream 进入传输态帧循环。
struct WsIo {
    ws: WebSocketStream<TcpStream>,
}

#[async_trait::async_trait]
impl crate::services::companion_pairing::HandshakeIo for WsIo {
    async fn recv(&mut self) -> Result<Vec<u8>, AppError> {
        match self.ws.next().await {
            Some(Ok(msg)) => match msg {
                tokio_tungstenite::tungstenite::Message::Binary(b) => Ok(b.to_vec()),
                tokio_tungstenite::tungstenite::Message::Close(_) => Err(
                    AppError::ConnectionError("对端关闭了连接".to_string()),
                ),
                _ => Err(AppError::ProtocolError("握手期仅接受二进制消息".to_string())),
            },
            Some(Err(e)) => Err(AppError::ConnectionError(format!("WS 读取失败: {}", e))),
            None => Err(AppError::ConnectionError("连接已关闭".to_string())),
        }
    }

    async fn send(&mut self, msg: &[u8]) -> Result<(), AppError> {
        self.ws
            .send(tokio_tungstenite::tungstenite::Message::binary(msg.to_vec()))
            .await
            .map_err(|e| AppError::ConnectionError(format!("WS 发送失败: {}", e)))?;
        Ok(())
    }
}

async fn handle_connection(
    pool: DbPool,
    state: Arc<CompanionState>,
    stream: TcpStream,
    peer_addr: SocketAddr,
) -> Result<(), AppError> {
    let ws = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| AppError::ConnectionError(format!("WS 握手失败: {}", e)))?;
    let mut io = WsIo { ws };

    // ── Noise XX 握手（整体超时：半开/恶意慢连接不得长期占用资源）──
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_TIMEOUT_SECS),
        crate::services::companion_pairing::run_responder_handshake(
            &state.desktop_static_priv,
            &mut io,
        ),
    )
    .await
    .map_err(|_| AppError::ConnectionError("手机伴侣握手超时".to_string()))??;
    let pubkey_hex =
        crate::services::companion_pairing::remote_pubkey_hex(&outcome.remote_static_pubkey);
    let mut transport = outcome.transport;

    // ── 早期准入：未配对 + 无 pending + 窗口关闭 → 立即拒绝 ──
    let now = state.now();
    let window = state.pairing_window.lock().await.clone();
    let allowed =
        connection_pubkey_allowed(&pool, &state.pending, &window, now, &pubkey_hex).await?;
    if !allowed {
        tracing::warn!(peer = %peer_addr, "未配对公钥尝试连接，已拒绝");
        return Ok(()); // 关闭连接
    }

    // ── 首帧必须为 HELLO（超时同握手期）──
    let first_frame = tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_TIMEOUT_SECS),
        recv_frame(&mut io, &mut transport),
    )
    .await
    .map_err(|_| AppError::ConnectionError("等待手机协议首帧超时".to_string()))??;
    validate_first_frame(&first_frame)?;

    // ── 收集 app 层 Notice：deviceInfo（设备名）/ pairingAuth（配对 nonce）──
    // 已配对 / pending 匹配的公钥无需 nonce（信任已建立）；全新公钥必须提交。
    let is_paired = paired_devices_db::get_by_pubkey(&pool, &pubkey_hex)
        .await?
        .is_some();
    let pending_match = {
        let guard = state.pending.lock().await;
        guard
            .as_ref()
            .map(|p| p.device_pubkey == pubkey_hex && !pending_is_expired(p, state.now()))
            .unwrap_or(false)
    };
    let need_nonce = !is_paired && !pending_match;

    let mut device_name = DEFAULT_DEVICE_NAME.to_string();
    let mut submitted_nonce: Option<String> = None;
    let mut name_received = false;
    let mut nonce_received = false;
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(APP_NOTICE_WINDOW_SECS);
    loop {
        if name_received && (!need_nonce || nonce_received) {
            break; // 应收集项已齐
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break; // 收集窗口截止
        }
        match tokio::time::timeout(remaining, recv_frame(&mut io, &mut transport)).await {
            Ok(Ok(frame)) => {
                if let Some(name) = device_name_from_frame(&frame) {
                    device_name = name;
                    name_received = true;
                }
                if let Some(nonce) = pairing_nonce_from_frame(&frame) {
                    submitted_nonce = Some(nonce);
                    nonce_received = true;
                }
                // 其余帧（如 PING）不在此应答，交由会话循环处理
            }
            // 对端已断开：停止收集，交由后续判定（nonce 已提交则配对仍可成立）
            Ok(Err(AppError::ConnectionError(_))) => break,
            Ok(Err(e)) => return Err(e),
            Err(_) => break,
        }
    }

    // ── nonce 校验：全新公钥必须提交与窗口一致的 nonce（「二维码单次有效」）──
    if need_nonce {
        let ok = {
            let mut window = state.pairing_window.lock().await;
            validate_and_consume_window_nonce(&mut window, state.now(), submitted_nonce.as_deref())
        };
        if !ok {
            tracing::warn!(peer = %peer_addr, "配对窗口 nonce 校验失败，已拒绝");
            return Ok(());
        }
        // 新扫码授权取代陈旧 pending（同公钥待确认槽以新时间重建，见 decide_pairing）
        *state.pending.lock().await = None;
    }

    // ── 配对决策 ──
    let decision = decide_pairing(&pool, &state.pending, &pubkey_hex, &device_name).await?;
    match decision {
        PairingDecision::AlreadyPaired { device_id, device_name } => {
            enter_session(pool, state, &mut io, &mut transport, device_id, device_name, peer_addr)
                .await
        }
        PairingDecision::FirstPairing { device } => {
            state.emit(EVENT_PAIRED, paired_event_payload(&device));
            // 配对成功后同步 NSD（首配即注册常驻广播）
            sync_nsd_registration(&pool, &state).await;
            enter_session(
                pool,
                state,
                &mut io,
                &mut transport,
                device.id,
                device.device_name,
                peer_addr,
            )
            .await
        }
        PairingDecision::PendingRebind { .. } => {
            // 换绑待确认：关闭连接，等待 pairing_confirm（手机重连即恢复）
            Ok(())
        }
    }
}

/// 帧循环：在 transport 会话内处理 HELLO / PING / Notice(deviceInfo)，
/// 其余帧类型收到即忽略并 warn（13.x 才消费）。
///
/// 会话生命周期：注册到 `active_session` 单槽——新连接进入时旧会话被终止
/// （僵尸连接不得污染状态机）；remove / confirm 经 terminate 信号终止会话；
/// 仅当退出会话仍是当前活跃会话时才回写 Listening 并发 disconnected 事件。
async fn enter_session(
    _pool: DbPool,
    state: Arc<CompanionState>,
    io: &mut WsIo,
    transport: &mut companion_proto::crypto::TransportSession,
    device_id: String,
    device_name: String,
    peer_addr: SocketAddr,
) -> Result<(), AppError> {
    let (term_tx, mut term_rx) = tokio::sync::watch::channel(false);
    {
        let mut guard = state.active_session.lock().await;
        if let Some(old) = guard.take() {
            old.terminate(); // 新连接取代旧会话
        }
        *guard = Some(ActiveSession {
            device_id: device_id.clone(),
            terminate: term_tx.clone(),
        });
    }
    state.set_connected(&device_id, &device_name, "direct", &peer_addr.to_string());
    state.emit(
        EVENT_CONNECTED,
        connected_event_payload(&device_id, &device_name),
    );

    loop {
        let frame = tokio::select! {
            _ = term_rx.changed() => break, // 被 remove / confirm / 新连接终止
            res = tokio::time::timeout(
                std::time::Duration::from_secs(SESSION_IDLE_TIMEOUT_SECS),
                recv_frame(io, transport),
            ) => match res {
                Ok(Ok(f)) => f,
                Ok(Err(AppError::ConnectionError(_))) => break,
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "companion 帧读取失败，断开连接");
                    break;
                }
                Err(_) => {
                    tracing::debug!(peer = %peer_addr, "会话空闲超时，断开连接");
                    break;
                }
            },
        };
        match &frame {
            Frame::Hello(_) => {
                tracing::debug!(peer = %peer_addr, "重复 HELLO 帧，忽略");
            }
            Frame::Ping(_) => {
                match encode_frame(&Frame::Ping(PingPayload {}), transport) {
                    Ok(pong) => {
                        // 应答失败与接收失败同路径清理——? 提前返回会绕过
                        // 状态机复位与 disconnected 事件发射
                        if let Err(e) = send_frame(io, &pong).await {
                            tracing::debug!(error = %e, "PONG 发送失败，断开连接");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "PONG 帧编码失败，断开连接");
                        break;
                    }
                }
            }
            Frame::Notice(_) => {
                if let Some(name) = device_name_from_frame(&frame) {
                    tracing::info!(new_name = %name, "收到对端设备名更新");
                    // 名字更新属 app 层契约，不在此写入（避免无窗口无 confirm 的隐性写入）
                }
            }
            other => {
                // 仅记帧类型判别式——payload 属解密后明文，不得入日志（NFR-M7）
                tracing::warn!(frame_type = frame_type_name(other), "companion 收到本 story 不处理的帧类型，忽略");
            }
        }
    }

    // 仅当本会话仍是当前活跃会话时才复位状态机（被新连接取代时不得回写，
    // 否则旧连接的退出会把在线的新连接错标为 Listening）
    let is_current = {
        let guard = state.active_session.lock().await;
        guard
            .as_ref()
            .map(|s| s.terminate.same_channel(&term_tx))
            .unwrap_or(false)
    };
    if is_current {
        *state.active_session.lock().await = None;
        state.set_listening();
        state.emit(
            EVENT_DISCONNECTED,
            disconnected_event_payload(&device_id, "连接已断开"),
        );
    }
    Ok(())
}

/// 帧类型判别式名称（仅类型、不含 payload——帧明文零输出纪律）。
fn frame_type_name(frame: &Frame) -> &'static str {
    match frame {
        Frame::Hello(_) => "hello",
        Frame::Snapshot(_) => "snapshot",
        Frame::StateDelta(_) => "state_delta",
        Frame::Command(_) => "command",
        Frame::CommandResult(_) => "command_result",
        Frame::StreamToken(_) => "stream_token",
        Frame::Notice(_) => "notice",
        Frame::Ping(_) => "ping",
    }
}

async fn recv_frame(
    io: &mut WsIo,
    transport: &mut companion_proto::crypto::TransportSession,
) -> Result<Frame, AppError> {
    let bytes = io.recv().await?;
    decode_frame(&bytes, transport).map_err(|_| {
        AppError::ProtocolError("手机伴侣协议帧无效".to_string())
    })
}

async fn send_frame(io: &mut WsIo, bytes: &[u8]) -> Result<(), AppError> {
    io.send(bytes).await
}

// ---------------------------------------------------------------------------
// NSD/mDNS 广播
// ---------------------------------------------------------------------------

/// 构造 NSD `ServiceInfo`（参数单测替代真实广播断言）。
fn build_service_info(
    state: &CompanionState,
    port: u16,
) -> Result<ServiceInfo, mdns_sd::Error> {
    let instance = format!("EgoSync-{}", &state.relay_id[..8]);
    // 主机名掺入 relay_id：同一局域网多台桌面时避免 mDNS 主机名冲突
    let host_name = format!("egosync-{}.local.", &state.relay_id[..8]);
    let mut props = std::collections::HashMap::new();
    props.insert("proto".to_string(), PROTOCOL_VERSION.to_string());
    ServiceInfo::new(
        NSD_SERVICE_TYPE,
        &instance,
        &host_name,
        "0.0.0.0",
        port,
        props,
    )
    .map(|si| si.enable_addr_auto())
}

/// 注册 NSD 广播（已注册则跳过）。
pub fn register_nsd(state: &Arc<CompanionState>) -> Result<(), AppError> {
    let port = state.listen_port();
    let mut guard = match state.nsd.try_lock() {
        Ok(g) => g,
        // 并发注册中等同成功：另一持有者正在注册同一服务
        Err(_) => return Ok(()),
    };
    if guard.is_some() {
        return Ok(());
    }
    let daemon = ServiceDaemon::new()
        .map_err(|e| AppError::ConnectionError(format!("NSD 守护进程启动失败: {}", e)))?;
    let service = build_service_info(&state, port).map_err(|e| {
        AppError::ConnectionError(format!("NSD ServiceInfo 构造失败: {}", e))
    })?;
    let full_name = service.get_fullname().to_string();
    daemon
        .register(service)
        .map_err(|e| AppError::ConnectionError(format!("NSD 注册失败: {}", e)))?;
    tracing::info!(%full_name, port, "NSD 广播已注册");
    *guard = Some(NsdHandle { daemon, full_name });
    Ok(())
}

/// 注销 NSD 广播（若已注册）。
async fn unregister_nsd(state: &Arc<CompanionState>) {
    let mut guard = state.nsd.lock().await;
    if let Some(handle) = guard.take() {
        if let Err(e) = handle.daemon.unregister(&handle.full_name) {
            tracing::warn!(error = %e, "NSD 注销失败");
        }
        let _ = handle.daemon.shutdown();
        tracing::info!("NSD 广播已注销（无配对设备且配对窗口关闭）");
    }
}

/// 按当前状态同步 NSD 注册：已配对 ≥1 或配对窗口打开 → 注册；否则注销。
///
/// 首配发现路径（P0b）：QR 生成（开窗）即广播——否则首配前手机无任何途径
/// 发现桌面（QR 不含地址）；窗口过期/被消费且无配对设备时回收广播。
pub async fn sync_nsd_registration(pool: &DbPool, state: &Arc<CompanionState>) {
    let devices_nonempty = match paired_devices_db::get_all(pool).await {
        Ok(devices) => !devices.is_empty(),
        Err(e) => {
            tracing::warn!(error = %e, "NSD 同步前查询配对设备失败");
            return;
        }
    };
    let window_open = {
        let window = state.pairing_window.lock().await;
        crate::services::companion_pairing::pairing_window_is_open(&window, state.now())
    };
    if devices_nonempty || window_open {
        if let Err(e) = register_nsd(state) {
            tracing::warn!(error = %e, "NSD 注册失败");
        }
    } else {
        unregister_nsd(state).await;
    }
}

/// 打开配对窗口并同步 NSD（`pairing_generate_qr` 命令入口）。
///
/// 窗口过期未被消费时由延迟任务回收广播；重新生成 QR 会开新窗口，
/// 回收任务按当时状态幂等重评估，不误伤新窗口。
pub async fn open_pairing_window_and_sync(
    pool: &DbPool,
    state: &Arc<CompanionState>,
    nonce: String,
) {
    state.open_pairing_window(nonce).await;
    sync_nsd_registration(pool, state).await;
    let pool_c = pool.clone();
    let state_c = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(
            PAIRING_WINDOW_TIMEOUT_SECS as u64 + 5,
        ))
        .await;
        sync_nsd_registration(&pool_c, &state_c).await;
    });
}

/// 移除配对并回收全部相关状态（`paired_device_remove` 命令入口）。
///
/// 「移除即拒绝」要求：删库之外同步清空配对窗口与 pending 槽（窗口残余期
/// 内任意新公钥不得自动再配对）、终止该设备的活跃会话（存量连接同被拒绝）、
/// 并按剩余状态回收 NSD 广播。
pub async fn remove_paired_device(
    pool: &DbPool,
    state: &Arc<CompanionState>,
    device_id: &str,
) -> Result<(), AppError> {
    paired_devices_db::remove(pool, device_id).await?;
    *state.pairing_window.lock().await = None;
    *state.pending.lock().await = None;
    state.terminate_active_session(Some(device_id)).await;
    sync_nsd_registration(pool, state).await;
    Ok(())
}

/// 数据销毁后的伴侣状态回收（`data_destroy` 命令入口）：
/// 终止会话、清 pending / 配对窗口、注销 NSD、删除 keyring 静态密钥并令
/// 内存缓存失效。
///
/// 注意：运行中的 WS 监听仍持旧静态密钥（不可变字段），重启应用后才加载
/// 新密钥——销毁后重新配对建议先重启。
pub async fn reset_after_data_destroy(pool: &DbPool, state: &Arc<CompanionState>) {
    state.terminate_active_session(None).await;
    *state.pending.lock().await = None;
    *state.pairing_window.lock().await = None;
    unregister_nsd(state).await;
    if let Err(e) =
        crate::services::secret_store::delete_secret(crate::services::companion_pairing::KEYRING_KEY)
    {
        tracing::warn!(error = %e, "销毁 companion 静态密钥失败");
    }
    crate::services::companion_pairing::invalidate_static_keypair_cache();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pool::init_db;
    use companion_proto::crypto::generate_static_keypair;
    use tempfile::tempdir;

    async fn test_pool() -> DbPool {
        let dir = tempdir().expect("create temp dir");
        let db_dir = dir.into_path();
        init_db(&db_dir.join("egosync.db"))
            .await
            .expect("init db with migrations")
    }

    #[test]
    fn service_info_carries_proto_txt_and_port() {
        // WHY: NSD 是手机"零配置发现桌面"的唯一入口——TXT proto 字段与
        // 端口漂移意味着手机连上的是错误版本或错误端口（握手必失败）。
        let (priv_key, pub_key) = generate_static_keypair().unwrap();
        let state = CompanionState::with_static_keypair_for_testing(None, priv_key, pub_key);
        state
            .listen_port
            .store(12345, std::sync::atomic::Ordering::Relaxed);
        let si = build_service_info(&state, 12345).expect("ServiceInfo 构造成功");
        assert_eq!(
            si.get_fullname(),
            format!("EgoSync-{}.{}", &state.relay_id[..8], NSD_SERVICE_TYPE)
        );
        // 主机名必须掺入实例标识——同网多台桌面共用固定主机名会引发
        // mDNS 冲突，手机可能解析到错误桌面
        assert_eq!(
            si.get_hostname(),
            format!("egosync-{}.local.", &state.relay_id[..8]),
            "主机名必须随 relay_id 唯一化"
        );
        let proto_val = si
            .get_property("proto")
            .expect("proto TXT 必须存在")
            .val()
            .map(|b| String::from_utf8_lossy(b).to_string());
        assert_eq!(proto_val, Some(PROTOCOL_VERSION.to_string()));
    }

    #[tokio::test]
    async fn status_reflects_listening_and_paired_device() {
        // WHY: 状态查询是前端"配对入口"可见性的唯一数据源——
        // listening/paired/pending 任一失真，用户都会做出错误判断（如以为
        // 已配对实际未配对）。
        let pool = test_pool().await;
        let (priv_key, pub_key) = generate_static_keypair().unwrap();
        let state = Arc::new(CompanionState::with_static_keypair_for_testing(
            None, priv_key, pub_key,
        ));
        let status = get_status(&pool, &state).await.expect("status");
        assert!(status.listening);
        assert!(status.connected.is_none());
        assert!(status.paired_device.is_none());

        paired_devices_db::upsert_single_device(&pool, &PairedDevice {
            id: "d1".to_string(),
            device_name: "手机A".to_string(),
            device_pubkey: "pub-a".to_string(),
            paired_at: "2026-08-28T10:00:00Z".to_string(),
            last_seen_at: "2026-08-28T10:00:00Z".to_string(),
        })
        .await
        .expect("upsert");
        let status = get_status(&pool, &state).await.expect("status");
        assert_eq!(status.paired_device.unwrap().device_pubkey, "pub-a");
    }

    #[tokio::test]
    async fn pairing_window_gates_unknown_pubkey_but_allows_paired() {
        // WHY: 移除即拒绝——窗口未打开时，未知公钥必须被拒；已配对公钥
        // 重连始终放行（信任持久化）。
        let pool = test_pool().await;
        let (priv_key, pub_key) = generate_static_keypair().unwrap();
        let state = Arc::new(CompanionState::with_static_keypair_for_testing(
            None, priv_key, pub_key,
        ));
        // 库为空、窗口关闭：未知公钥被拒
        assert!(
            !connection_pubkey_allowed(&pool, &state.pending, &None, state.now(), "pub-x")
                .await
                .unwrap(),
            "无配对无窗口时未知公钥必须被拒"
        );
        // 写入设备后窗口关闭：同公钥放行
        paired_devices_db::upsert_single_device(&pool, &PairedDevice {
            id: "d1".to_string(),
            device_name: "手机A".to_string(),
            device_pubkey: "pub-a".to_string(),
            paired_at: "2026-08-28T10:00:00Z".to_string(),
            last_seen_at: "2026-08-28T10:00:00Z".to_string(),
        })
        .await
        .expect("upsert");
        assert!(
            connection_pubkey_allowed(&pool, &state.pending, &None, state.now(), "pub-a")
                .await
                .unwrap(),
            "已配对公钥重连必须放行"
        );
    }
}
