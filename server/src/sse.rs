//! SSE 事件面（Story 15.4）：`GET /api/events` 广播扇出。
//!
//! [`SseEventBus`] 实现 engine `EngineEvents` 接缝——引擎侧全部事件发射
//! 经此进入 tokio broadcast 通道；本路由订阅扇出为 SSE 流：
//! - 事件名 = Event 字段、data = payload JSON（与 emit 同构）；
//! - KeepAlive 30s 心跳（EventSource 平台超时倒逼）；
//! - broadcast 滞后（Lagged）即断开该连接（慢客户端兜底，EventSource
//!   自动重连兜住恢复）。
//!
//! Story 16.3：`POST /api/events/ticket`（Bearer）签发一次性 30s 票据，
//! `GET /api/events?ticket=` 凭票据建流——EventSource 无法携带自定义头，
//! 票据是令牌不入 URL/日志的短时替身（单次使用 + 30s TTL）。Cookie
//! 路径（`GET /api/events` 无票据参数）零变化。

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use egosync_engine::services::event_bus::EngineEvents;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_stream::wrappers::BroadcastStream;

use crate::AppState;

/// SSE 心跳间隔（F11）。
pub const SSE_KEEPALIVE_SECS: u64 = 30;

/// SSE 票据 TTL（Story 16.3 冻结款：30 秒一次性短时票据）。
pub const SSE_TICKET_TTL: Duration = Duration::from_secs(30);

/// 广播消息（事件名 + 已序列化 payload）。
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: String,
    pub payload: serde_json::Value,
}

/// SSE 广播通道容量：超出即视为慢客户端（Lagged → 断开该连接）。
pub const BROADCAST_CAPACITY: usize = 256;

/// 服务端事件总线：EngineEvents → broadcast 扇出。
pub struct SseEventBus {
    tx: tokio::sync::broadcast::Sender<SseEvent>,
}

impl SseEventBus {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(BROADCAST_CAPACITY);
        Self { tx }
    }

    /// 广播端句柄（AppState.events_tx 同源）。
    pub fn sender(&self) -> &tokio::sync::broadcast::Sender<SseEvent> {
        &self.tx
    }
}

impl Default for SseEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineEvents for SseEventBus {
    fn emit(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        // 无订阅者时 send 返回 Err——事件丢弃是广播语义的正常态
        // （首个 SSE 客户端连上前桌面时代行为一致：无人监听即无人消费）。
        let _ = self.tx.send(SseEvent {
            event: event.to_string(),
            payload,
        });
        Ok(())
    }
}

/// 广播订阅流 → SSE 事件流（滞后/Closed 即终止——慢客户端断开语义）。
///
/// 独立成函数供单元测试直接驱动（滞后断开不可经 TCP 稳定复现——内核
/// 缓冲会吸收慢消费，测试直接对 Stream 语义断言）。
///
/// 终止语义：`take_while` 在首个 `Err`（Lagged/Closed）处结束流——
/// `filter_map(None)` 只会丢弃该项继续消费（静默跳帧），不构成断开。
pub fn sse_event_stream(
    rx: tokio::sync::broadcast::Receiver<SseEvent>,
) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    BroadcastStream::new(rx)
        .take_while(|msg| futures_util::future::ready(msg.is_ok()))
        .filter_map(|msg| async move {
            match msg {
                Ok(ev) => Some(Ok(Event::default()
                    .event(ev.event)
                    .data(ev.payload.to_string()))),
                // take_while 已保证 Err 不达此处（防御性兜底）
                Err(_) => None,
            }
        })
}

/// `GET /api/events`：订阅广播扇出为 SSE（认证中间件守门，未认证 401）。
pub async fn events_handler(State(state): State<Arc<AppState>>) -> Response {
    let stream = sse_event_stream(state.events_tx.subscribe());

    (
        StatusCode::OK,
        [
            (header::CACHE_CONTROL, "no-cache"),
            // SSE 长连接禁用代理缓冲（自托管反代场景兼容）
            (header::HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        Sse::new(stream).keep_alive(
            KeepAlive::new().interval(Duration::from_secs(SSE_KEEPALIVE_SECS)),
        ),
    )
        .into_response()
}

// ── Story 16.3：SSE 一次性票据 ─────────────────────────────────────────────

/// SSE 票据表：ticket → 过期时刻（内存态；签发时顺手清扫过期项防泄漏）。
///
/// 单次使用：`consume` 即移除（校验与消费原子化——并发同票据只有首个
/// 成功）。票据值 = `sse_` + uuid v4（122 bit 熵，URL 安全字符）。
pub struct SseTicketStore {
    ttl: Duration,
    inner: Mutex<HashMap<String, Instant>>,
}

impl SseTicketStore {
    /// 以给定 TTL 构造（生产 = [`SSE_TICKET_TTL`]；测试可注入短 TTL 驱动
    /// 过期路径）。
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 签发一枚新票据（过期项顺手清扫——未消费票据最多存活 TTL）。
    pub fn issue(&self) -> String {
        let ticket = format!("sse_{}", uuid::Uuid::new_v4());
        let now = Instant::now();
        let mut inner = self.inner.lock().expect("SSE 票据表锁中毒");
        inner.retain(|_, expires_at| *expires_at > now);
        inner.insert(ticket.clone(), now + self.ttl);
        ticket
    }

    /// 校验并消费票据（单次使用）：存在且未过期 ⇒ true 并移除；否则
    /// false（不存在/已用/已过期同形失败——不泄露票据状态）。
    pub fn consume(&self, ticket: &str) -> bool {
        let mut inner = self.inner.lock().expect("SSE 票据表锁中毒");
        match inner.remove(ticket) {
            Some(expires_at) => expires_at > Instant::now(),
            None => false,
        }
    }

    /// 存量票据数（测试观测用）。
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("SSE 票据表锁中毒").len()
    }
}

/// `POST /api/events/ticket`（Story 16.3）：认证（Bearer/cookie——挂
/// require_auth 组）后签发一次性 SSE 票据。
///
/// 响应 `{ticket}`；票据 30s TTL、单次使用——EventSource 携 `?ticket=`
/// 建流，令牌本体不入 URL/日志。
pub async fn ticket_handler(State(state): State<Arc<AppState>>) -> Response {
    let ticket = state.sse_tickets.issue();
    (StatusCode::OK, Json(serde_json::json!({ "ticket": ticket }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 票据单次使用：首 consume 通过、二次拒绝（不存在同形）。
    #[test]
    fn ticket_is_single_use() {
        let store = SseTicketStore::new(SSE_TICKET_TTL);
        let ticket = store.issue();
        assert!(store.consume(&ticket), "首次使用必须通过");
        assert!(!store.consume(&ticket), "二次使用必须拒绝（单次使用）");
        assert_eq!(store.len(), 0, "消费后票据表应清空");
    }

    /// 票据 TTL 过期：短 TTL 构造 + 越时后消费拒绝（过期票据不建流）。
    #[tokio::test]
    async fn ticket_expires_after_ttl() {
        let store = SseTicketStore::new(Duration::from_millis(20));
        let ticket = store.issue();
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(!store.consume(&ticket), "过期票据必须拒绝（30s TTL 冻结款的最小化验证）");
    }

    /// 票据值形状：`sse_` 前缀 + uuid（URL 安全，无令牌本体）。
    #[test]
    fn ticket_value_shape() {
        let store = SseTicketStore::new(SSE_TICKET_TTL);
        let ticket = store.issue();
        assert!(ticket.starts_with("sse_"), "票据前缀: {}", ticket);
        assert!(ticket.len() > "sse_".len() + 30, "票据应携带 uuid 熵: {}", ticket);
    }
}
