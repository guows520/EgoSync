//! egosync-server —— EgoSync 云端自托管 server binary（Story 15.4）。
//!
//! 形态：单实例单用户自托管（无多租户/注册体系抽象），与 relay-server
//! 同栈同布局（axum 0.8 + tokio 1、`build_router` 供测试进程内复用、
//! main.rs 仅 env+tracing+优雅退出）。业务面唯一入口 `POST /api/cmd/{command}`
//! （camelCase 参数与桌面 invoke 同构），事件面 `GET /api/events`（SSE）。
//!
//! 认证（架构决策 #5）：env `EGOSYNC_TOKEN` 存在 ⇒ setup 不挂载、login 仅
//! 常时比对 env；env 不存在 ⇒ 库内 Argon2id 哈希（首访 `/api/setup` 写入
//! `app_settings` kv）。两态切换须重启进程；已发 Cookie 存于 `auth_sessions`
//! 表，跨重启/跨切换不失效。无任何 dev 免认证旁路。
//!
//! 错误白名单（架构 ②）：AppError 全 variant 一律 `200 + 原样单键 map`；
//! 非 200 仅 401 / 429 / 404 / 进程级 5xx（handler panic 经 CatchPanicLayer
//! → 500）；传输面前置拒绝（跨源 403 / 超限 413）为显式登记的传输面扩展。

pub mod auth;
pub mod bootstrap;
pub mod dispatch_gen;
pub mod healthz;
pub mod idle_timeout;
pub mod routes;
pub mod secret_store;
pub mod security;
pub mod sse;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::auth::AuthState;
use crate::sse::SseEvent;

/// 请求体上限（F11）：显式覆盖 axum 默认 2MB——导入/大 payload 需要。
pub const MAX_BODY_BYTES: usize = 50 * 1024 * 1024;

/// 路由级共享状态：EngineCtx 单容器 + 认证态 + 事件扇出 + 生命周期句柄。
pub struct AppState {
    /// 引擎命令容器（全部 web-ok 命令体的依赖注入点）。
    pub ctx: Arc<egosync_engine::commands::ctx::EngineCtx>,
    /// 认证态（env/库态 + setup 可用位 + 会话表 + 限流器）。
    pub auth: AuthState,
    /// SSE 事件扇出通道（SseEventBus 的广播端）。
    pub events_tx: broadcast::Sender<SseEvent>,
    /// sidecar 句柄（healthz deep / 退出清理）。
    pub sidecar: Arc<tokio::sync::Mutex<egosync_engine::services::sidecar::SidecarManager>>,
    /// 全局取消令牌（watchdog / delegate 监听 / 退出清理）。
    pub cancel: CancellationToken,
}

/// 构建完整路由（生产与测试共用；测试经 [`bootstrap::build_test_state`]
/// 注入轻量状态）。
pub fn build_router(state: Arc<AppState>) -> Router {
    // 认证面前的公开路由（限流 5/min/IP：/api/auth/* + /api/setup）
    let public = Router::new()
        .route("/api/auth/status", get(auth::auth_status))
        .route("/api/auth/login", post(auth::login))
        .route("/api/setup", post(auth::setup))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::rate_limit,
        ))
        .with_state(state.clone());

    // 业务路由（认证中间件守门——未认证含 SSE 一律 401）
    let authed = Router::new()
        .route("/api/cmd/{command}", post(routes::cmd_handler))
        .route("/api/events", get(sse::events_handler))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ))
        .with_state(state.clone());

    // 探针（认证豁免面：仅 healthz；静态资源归 16.1）
    let probes = Router::new()
        .route("/healthz", get(healthz::healthz))
        .with_state(state.clone());

    Router::new()
        .merge(probes)
        .merge(public)
        .merge(authed)
        // 中间件叠放（后加者为外层）：CatchPanic 最外兜底 → CSP 全响应
        // （须在跨源拒绝之外——403 也下发 CSP）→ 跨源拒绝 → body 上限。
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(axum::middleware::from_fn(security::reject_cross_origin))
        .layer(axum::middleware::from_fn(security::csp_headers))
        .layer(tower_http::catch_panic::CatchPanicLayer::custom(
            security::panic_response,
        ))
}
