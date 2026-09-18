//! SSE 事件面（Story 15.4）：`GET /api/events` 广播扇出。
//!
//! [`SseEventBus`] 实现 engine `EngineEvents` 接缝——引擎侧全部事件发射
//! 经此进入 tokio broadcast 通道；本路由订阅扇出为 SSE 流：
//! - 事件名 = Event 字段、data = payload JSON（与 emit 同构）；
//! - KeepAlive 30s 心跳（EventSource 平台超时倒逼）；
//! - broadcast 滞后（Lagged）即断开该连接（慢客户端兜底，EventSource
//!   自动重连兜住恢复）。

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use egosync_engine::services::event_bus::EngineEvents;
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;

use crate::AppState;

/// SSE 心跳间隔（F11）。
pub const SSE_KEEPALIVE_SECS: u64 = 30;

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
