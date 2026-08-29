//! 纯二进制双向透传（AC2）+ 连接生命周期编排。
//!
//! 生命周期：鉴权（auth）→ 登记槽位（registry）→ 双向转发循环 → 统一清理。
//! 转发纪律：任一端收到的 binary WS 消息**原样**（不解码、不解析、不修改字节）
//! 发往对端；text 消息在转发态被忽略并 warn；转发统计只记字节数（debug 级）。
//!
//! 评审整改（本 story Review Findings）：
//! - **有界转发通道**：`FORWARD_CHANNEL_CAPACITY` 上限，满即断连——
//!   慢/半死对端不再造成无界内存积压与任务挂死（AC5）；
//! - **写超时**：对本端 WS 客户端的写超过 `WRITE_TIMEOUT_SECS` 判死断连；
//! - **服务端主动保活**：周期 Ping + `KEEPALIVE_IDLE_SECS` 空闲上限——
//!   无 FIN 的死对端（NAT 超时/断电）也会被回收，槽位不再永久滞留；
//! - **优雅退出**：订阅 registry 关闭广播，收到即向客户端发 Close 后退出。

use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::mpsc;

use crate::auth;
use crate::registry::{self, Slot};
use crate::AppState;

/// 转发通道容量（有界）：每连接最多积压 16 条消息——
/// 超过即对端消费停滞，断连（断线即丢语义，不做无界积压）。
pub const FORWARD_CHANNEL_CAPACITY: usize = 16;

/// 对本端 WS 客户端单次写的超时上限（秒）：超过判死断连。
pub const WRITE_TIMEOUT_SECS: u64 = 10;

/// 保活 Ping 周期（秒）：周期性探测对端存活（触发对端协议栈自动回 Pong）。
pub const KEEPALIVE_PING_SECS: u64 = 20;

/// 空闲上限（秒）：期间无任何收发活动（含 Pong）即判僵尸连接断连。
pub const KEEPALIVE_IDLE_SECS: u64 = 60;

/// `/relay` WS upgrade 后的连接处理入口。
pub async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let registry = state.registry;

    // 1. 挑战-应答鉴权（整体超时预算）：失败即关连接、不登记（防 ID 抢占）
    let client = match auth::authenticate(&mut socket, state.auth_timeout).await {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(?err, "registration rejected");
            return;
        }
    };
    let relay_id = client.relay_id;
    let role = client.role;

    // 2. 登记槽位（单槽替换语义由 Registry 保证；phone 槽公钥绑定防抢占 D1-a）
    let (tx, mut rx) = mpsc::channel::<Message>(FORWARD_CHANNEL_CAPACITY);
    let conn_id = registry::next_conn_id();
    let slot = Slot {
        conn_id,
        sender: tx,
        registered_at: Instant::now(),
        remote_static_pubkey: client.remote_static_pubkey,
    };
    match registry.register(&relay_id, role, slot).await {
        Ok(_peer) => {} // 先到者无对端（peer sender 缓存优化见 deferred-work）
        Err(err) => {
            // phone 槽已被其他公钥占用：拒绝且不登记（只记事件类别，不记密钥材料）
            tracing::warn!(relay_id = %relay_id, ?err, "registration rejected: slot bound to another key");
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    }
    tracing::info!(relay_id = %relay_id, role = ?role, "registered");

    // 3. 双向转发循环（含保活与优雅退出分支）
    let mut shutdown_rx = registry.shutdown_rx();
    let mut keepalive = tokio::time::interval(state.keepalive_ping);
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_activity = Instant::now();

    loop {
        tokio::select! {
            // 对端 → 本连接（经通道推送，需写给本端 WS 客户端）
            maybe = rx.recv() => {
                match maybe {
                    Some(msg) => {
                        last_activity = Instant::now();
                        // 写超时：对本端写停滞（TCP 缓冲满且对端不收）判死断连
                        match tokio::time::timeout(
                            Duration::from_secs(WRITE_TIMEOUT_SECS),
                            socket.send(msg),
                        ).await {
                            Ok(Ok(())) => {}
                            Ok(Err(_)) | Err(_) => break, // 写失败/写超时 → 统一清理（P2）
                        }
                    }
                    None => {
                        // 本槽位被新连接替换（P6）：通知旧客户端后退出
                        let _ = socket.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
            // 本连接 → 对端（原样转发 binary）
            maybe = socket.recv() => {
                last_activity = Instant::now(); // 任何入站消息（含 Ping/Pong）都算存活证据
                match maybe {
                    Some(Ok(Message::Binary(data))) => {
                        let bytes = data.len();
                        match registry.peer_sender(&relay_id, role).await {
                            Some(peer) => {
                                // 有界通道满 = 对端消费停滞：断连（断线即丢，不做无界积压）
                                if peer.try_send(Message::Binary(data)).is_err() {
                                    break;
                                }
                                tracing::debug!(relay_id = %relay_id, role = ?role, bytes, "forwarded");
                            }
                            None => {
                                // 无离线投递（架构硬边界 #3）：对端不在即丢弃
                                tracing::debug!(relay_id = %relay_id, role = ?role, bytes, "peer absent, dropped");
                            }
                        }
                    }
                    Some(Ok(Message::Text(_))) => {
                        // 控制协议只在注册首消息出现一次；转发态 text 属协议违例
                        tracing::warn!(relay_id = %relay_id, role = ?role, "text message in forwarding state ignored");
                    }
                    Some(Ok(_)) => {} // Ping/Pong 由 tungstenite 协议栈自处理（回 Pong 已核实）
                    Some(Err(_)) | None => break, // WS 错误 / 客户端断开
                }
            }
            // 服务端主动保活：空闲超限判僵尸断连；否则发 Ping 探测（对端自动回 Pong）
            _ = keepalive.tick() => {
                if last_activity.elapsed() > state.keepalive_idle {
                    break; // 僵尸连接（NAT 超时/断电无 FIN）→ 统一清理回收槽位
                }
                if socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
            // 优雅退出广播（Docker stop）：向客户端发 Close 后退出，进程不挂到 SIGKILL
            _ = shutdown_rx.changed() => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
        }
    }

    // 4. 统一清理：清槽（conn_id 匹配才生效）+ 对端 close 通知（P2）
    if let Some(peer) = registry.disconnect(&relay_id, role, conn_id).await {
        // try_send：对端通道可能已满（其消费停滞），不阻塞退出路径
        let _ = peer.try_send(Message::Close(None));
    }
    tracing::info!(relay_id = %relay_id, role = ?role, "disconnected");
}
