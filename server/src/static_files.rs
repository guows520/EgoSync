//! 静态服务（Story 16.1）：`egosync-app/dist` 同源服务 + SPA 回退。
//!
//! server 以同一 Vite 构建产物服务浏览器入口（与桌面 frontendDist 同一
//! `npm run build` 产物——单一构建锚点，绝不复制出第二套前端构建）：
//! - 文件命中：[`tower_http::services::ServeDir`]（content-type / ETag /
//!   目录穿越防护由其保证；`/` 经 `append_index_html_on_directories`
//!   命中 dist/index.html）；
//! - 文件未命中（或目录穿越等无效路径）：[`SpaFallbackService`] 决策树——
//!   `/api/*` 未知路径 → JSON 404（API 面错误形状保持，绝不落入静态面）；
//!   末段含扩展名的路径（`/assets/missing.js` 等资产形态）→ 404；
//!   其余路由形态（`/unknown-path` 深链）→ 200 index.html（SPA 回退）；
//! - 目录未配置（env 未设且默认路径缺失）→ API-only + 启动期 tracing
//!   警告（见 main.rs `resolve_static_dir`），静态面一律 404，进程不崩溃。
//!
//! 挂载形态：`build_router` 尾部 `fallback_service`（仅未命中任何路由的
//! 请求到达）——CSP / 安全头 / 跨源拒绝 / body 上限 / CatchPanic 中间件
//! 天然覆盖静态响应（`Router::layer` 对 fallback 同样生效）。
//!
//! `call_fallback_on_method_not_allowed(true)`：POST 等非 GET/HEAD 请求
//! 也进入 fallback——`/api/*` 未知路径不受 ServeDir 的 405 短路影响，
//! 维持 404 语义（15.4 未匹配路由默认 404 的现状延续）。

use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;
use tower::Service;
use tower_http::services::ServeDir;

/// API-only 模式的 fallback（目录未配置）：`/api/*` 未知路径与其余路径
/// 一律 JSON 404——静态面整体不挂载（含文件命中也不服务）。
pub fn api_only_fallback() -> SpaFallbackService {
    SpaFallbackService { index_path: None }
}

/// 静态模式 fallback 服务链：ServeDir（文件命中）→ SpaFallback（未命中）。
///
/// `Router::fallback_service` 的挂载对象。
pub fn serve_dir_fallback(static_dir: PathBuf) -> ServeDir<SpaFallbackService> {
    let index_path = Arc::new(static_dir.join("index.html"));
    ServeDir::new(static_dir)
        .call_fallback_on_method_not_allowed(true)
        .fallback(SpaFallbackService {
            index_path: Some(index_path),
        })
}

/// SPA 回退 + /api 守卫 + 资产 404 决策树（见模块文档）。
#[derive(Clone)]
pub struct SpaFallbackService {
    /// index.html 路径（`None` = API-only 模式）。
    index_path: Option<Arc<PathBuf>>,
}

impl SpaFallbackService {
    /// 路径是否为资产形态（末段含扩展名）——SPA 路由段不含 `.`。
    fn is_asset_shaped(path: &str) -> bool {
        path.rsplit('/')
            .next()
            .is_some_and(|segment| segment.contains('.'))
    }
}

impl<B> Service<Request<B>> for SpaFallbackService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // 决策只需 URI path——先同步提取，请求体（可能 !Send 的 B）不进
        // async 块（future 恒 Send，无泛型 bound）
        let path = req.uri().path().to_string();
        let index_path = self.index_path.clone();
        Box::pin(async move { Ok(spa_fallback_response(&path, index_path).await) })
    }
}

/// fallback 决策树（async 主体——与 Service 实现解耦，逻辑单点）。
async fn spa_fallback_response(path: &str, index_path: Option<Arc<PathBuf>>) -> Response {
    // /api 未知路径：JSON 404（API 面错误形状——绝不落入静态/SPA 面）
    if path == "/api" || path.starts_with("/api/") {
        return not_found_json();
    }

    // API-only（目录未配置）：静态面一律 404
    let Some(index_path) = index_path else {
        return not_found_json();
    };

    // 资产形态（末段含扩展名）：缺失文件如实 404——SPA 回退只救路由深链
    if SpaFallbackService::is_asset_shaped(path) {
        return not_found_json();
    }

    // SPA 路由回退：200 index.html（no-cache——见 [`index_response`]）
    index_response(Some(index_path)).await
}

/// index.html 响应（`/` 直出与 SPA 回退共用）：200 + text/html +
/// `Cache-Control: no-cache`。
///
/// 重部署场景的关键防护：index.html 引用带 hash 的资产文件，浏览器凭
/// 启发式缓存（ServeDir 带 Last-Modified）继续使用旧 index 时，会请求
/// 已 404 的旧 hash 资产导致白屏——`no-cache` 保证每次进入都取最新
/// index（资产本身带内容 hash，可被安全长缓存）。
pub async fn index_response(index_path: Option<Arc<PathBuf>>) -> Response {
    let Some(index_path) = index_path else {
        return not_found_json();
    };
    match tokio::fs::read(index_path.as_path()).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => {
            // index.html 缺失（目录已配置但产物不全）：如实 404 + 运维可见
            tracing::error!("读取 index.html 失败（{}）: {}", index_path.display(), e);
            not_found_json()
        }
    }
}

/// JSON 404 统一形状（与 API 面既有错误形状一致：`{"error":"not found"}`）。
fn not_found_json() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": "not found"})),
    )
        .into_response()
}
