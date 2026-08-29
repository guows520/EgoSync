//! relay-server —— EgoSync 手机伴侣无状态加密中继。
//!
//! 架构边界（零知识纪律，NFR-M7）：
//! - 只见 `relay_id` 与密文帧：不解析、不修改、不落盘任何转发内容；
//! - 注册表只存在于内存（零数据库、零磁盘写、断线即丢）；
//! - tracing 日志只含 relay_id / 角色 / 字节数 / 事件类别，帧字节与密钥材料永不入日志。
//!
//! bin 入口见 `main.rs`（env 配置 + tracing init + 优雅退出）；
//! 集成测试经由 [`build_router`] / [`build_router_with`] 在进程内拉起完整路由
//! （后者允许注入紧凑的时限配置做回归测试）。

pub mod auth;
pub mod forward;
pub mod registry;

use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

/// 单条 WS 消息大小上限（字节）：贴着 E2E 密文帧上限
/// （`companion_proto::crypto::MAX_CIPHERTEXT_LEN = 64KB - 1`）留一倍裕量，
/// 在协议层拒绝超大帧，防内存滥用（默认 tungstenite 上限约 64MB）。
pub const MAX_MESSAGE_SIZE: usize = 128 * 1024;

/// 路由级共享状态：注册表 + 连接生命周期时限配置。
///
/// 时限默认值取各模块常量（生产配置）；测试经 [`build_router_with`]
/// 注入紧凑值做回归（鉴权总预算 / 保活 / 大帧拒绝）。
#[derive(Clone)]
pub struct AppState {
    pub registry: registry::Registry,
    /// 鉴权阶段整体超时预算（AC2 防御边界：未鉴权连接不得长期占用）。
    pub auth_timeout: Duration,
    /// 保活 Ping 周期（AC5 僵尸连接回收）。
    pub keepalive_ping: Duration,
    /// 空闲上限：期间无任何收发活动即判僵尸断连。
    pub keepalive_idle: Duration,
    /// 单条 WS 消息大小上限（协议层拒绝）。
    pub max_message_size: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            registry: registry::Registry::new(),
            auth_timeout: Duration::from_secs(auth::AUTH_TIMEOUT_SECS),
            keepalive_ping: Duration::from_secs(forward::KEEPALIVE_PING_SECS),
            keepalive_idle: Duration::from_secs(forward::KEEPALIVE_IDLE_SECS),
            max_message_size: MAX_MESSAGE_SIZE,
        }
    }
}

/// 构建完整路由（默认配置）：`GET /healthz` + `GET /relay`（WS upgrade）。
pub fn build_router() -> Router {
    build_router_with(AppState::default())
}

/// 构建完整路由（注入自定义状态，测试用）。
pub fn build_router_with(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/relay", get(relay_upgrade))
        .with_state(state)
}

/// AC1：探活端点，返回 200 + `{"status":"ok"}`。
async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

/// WS upgrade 入口：连接生命周期（鉴权 → 登记 → 转发）由 forward 模块编排。
async fn relay_upgrade(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.max_message_size(state.max_message_size)
        .on_upgrade(move |socket| forward::handle_socket(socket, state))
}
