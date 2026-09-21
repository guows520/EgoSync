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
//! 静态服务（Story 16.1）：`build_router` 尾部 fallback 挂载
//! `egosync-app/dist`（单一构建产物双宿主复用）+ SPA 回退；目录经参数
//! 注入（main.rs 读 env `EGOSYNC_STATIC_DIR` / 默认路径；`None` = API-only），
//! 测试经 fixture 目录注入。
//!
//! 错误白名单（架构 ②）：AppError 全 variant 一律 `200 + 原样单键 map`；
//! 非 200 仅 401 / 429 / 404 / 进程级 5xx（handler panic 经 CatchPanicLayer
//! → 500）；传输面前置拒绝（跨源 403 / 超限 413）为显式登记的传输面扩展。

pub mod auth;
pub mod bootstrap;
pub mod cors;
pub mod dispatch_gen;
pub mod healthz;
pub mod idle_timeout;
pub mod routes;
pub mod secret_store;
pub mod security;
pub mod sse;
pub mod static_files;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::auth::AuthState;
use crate::sse::{SseEvent, SseTicketStore};

/// 请求体上限（F11）：显式覆盖 axum 默认 2MB——导入/大 payload 需要。
pub const MAX_BODY_BYTES: usize = 50 * 1024 * 1024;

/// 路由级共享状态：EngineCtx 单容器 + 认证态 + 事件扇出 + 生命周期句柄。
pub struct AppState {
    /// 引擎命令容器（全部 web-ok 命令体的依赖注入点）。
    pub ctx: Arc<egosync_engine::commands::ctx::EngineCtx>,
    /// 认证态（env/库态 + setup 可用位 + 会话表 + 限流器）。
    pub auth: AuthState,
    /// [T5 修订] Bearer 失败面限流器（仅计验证失败；5 次/分钟/IP 滑动
    /// 窗口）——独立于 auth 路由限流器（`auth.rate`）：auth 面预算不被
    /// Bearer 失败消耗，Bearer 失败预算不被登录轮询消耗。
    pub bearer_rate: crate::auth::RateLimiter,
    /// SSE 事件扇出通道（SseEventBus 的广播端）。
    pub events_tx: broadcast::Sender<SseEvent>,
    /// SSE 一次性票据表（Story 16.3：桌面远程 EventSource 通道）。
    pub sse_tickets: SseTicketStore,
    /// sidecar 句柄（healthz deep / 退出清理）。
    pub sidecar: Arc<tokio::sync::Mutex<egosync_engine::services::sidecar::SidecarManager>>,
    /// 全局取消令牌（watchdog / delegate 监听 / 退出清理）。
    pub cancel: CancellationToken,
    /// 反代感知门控（Story 17.1）：`EGOSYNC_BEHIND_PROXY=1` ⇒
    /// security 同源判定读取 X-Forwarded-Proto（TLS 反代缺省端口归一）
    /// + auth 在 XFP=https 时 Cookie 加 `Secure`。未启用时
    /// X-Forwarded-\* 一律忽略（直连语义与 15.4 逐字节一致——dev/e2e
    /// 零影响，防直连暴露下的伪造信任）。
    pub behind_proxy: bool,
}

/// 构建完整路由（生产与测试共用；测试经 [`bootstrap::build_test_state`]
/// 注入轻量状态）。
///
/// `static_dir`（Story 16.1）：静态目录（`egosync-app/dist`）——`Some` 挂
/// ServeDir + SPA 回退，`None` API-only；中间件叠放对 fallback 同样生效
/// （CSP / 安全头 / 跨源 / body 上限天然覆盖静态响应）。
pub fn build_router(state: Arc<AppState>, static_dir: Option<std::path::PathBuf>) -> Router {
    // 认证面前的公开路由（限流 5/min/IP：/api/auth/* + /api/setup）
    let public = Router::new()
        .route("/api/auth/status", get(auth::auth_status))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/setup", post(auth::setup))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::rate_limit,
        ))
        .with_state(state.clone());

    // 业务路由（认证中间件守门——未认证含 SSE 一律 401；Bearer 叠加通道
    // 见 auth::require_auth）
    let authed = Router::new()
        .route("/api/cmd/{command}", post(routes::cmd_handler))
        .route(
            "/api/events/ticket",
            post(sse::ticket_handler),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ))
        .with_state(state.clone());

    // SSE 事件流（Story 16.3 三通道认证：Bearer / 一次性票据 / 会话
    // Cookie——票据只在 `/api/events` 接受，EventSource 无法带自定义头）
    let sse_stream = Router::new()
        .route("/api/events", get(sse::events_handler))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth_with_sse_ticket,
        ))
        .with_state(state.clone());

    // 探针（认证豁免面：仅 healthz）
    let probes = Router::new()
        .route("/healthz", get(healthz::healthz))
        .with_state(state.clone());

    // 静态服务（16.1）：`/` 显式路由直出 index.html（ServeDir 的
    // append_index 不支持自定义响应头——no-cache 须显式路由）+ 尾部
    // fallback——仅未命中任何路由的请求到达；/api/* 未知路径在 fallback
    // 内守卫为 JSON 404（不落入 SPA 面）
    let base = Router::new()
        .merge(probes)
        .merge(public)
        .merge(authed)
        .merge(sse_stream);
    let router = match static_dir {
        Some(dir) => {
            // index.html 路径句柄（`/` 直出与 SPA 回退共用 + no-cache）
            let index_path = Arc::new(dir.join("index.html"));
            base.route(
                "/",
                get(move || static_files::index_response(Some(index_path.clone()))),
            )
            .fallback_service(static_files::serve_dir_fallback(dir))
        }
        None => base.fallback_service(static_files::api_only_fallback()),
    };

    router
        // 中间件叠放（后加者为外层）：CatchPanic 最外兜底 → CSP+安全头全响应
        // （须在跨源拒绝之外——403 也下发）→ CORS 白名单层（16.3：桌面
        // webview 跨源放行 + 预检短路；须在跨源拒绝之外——白名单 Origin 的
        // 响应/预检需要本层回写放行头，且预检不得落入拒绝层）→ 跨源拒绝
        // （17.1 起带 state：BEHIND_PROXY 门控的 XFP 感知）→ body 上限。
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::reject_cross_origin,
        ))
        .layer(axum::middleware::from_fn(cors::cors_allowlist))
        .layer(axum::middleware::from_fn(security::security_headers))
        .layer(tower_http::catch_panic::CatchPanicLayer::custom(
            security::panic_response,
        ))
}
