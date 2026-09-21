//! CORS 白名单层（Story 16.3，FR-48）：桌面 webview 跨源访问通道。
//!
//! 桌面远程模式的 webview 宿主源（Windows WebView2 `http://tauri.localhost`；
//! macOS/Linux `tauri://localhost`）与远端实例必然跨源——本层对这两个
//! Origin 显式放行：
//! - **preflight**（OPTIONS + 白名单 Origin）：直接 204 + CORS 放行头
//!   （`Authorization` 触发预检——fetch 携 Bearer 的必要条件）；
//! - **实际响应**（含 401/429/404/5xx 与 SSE 流）：回写
//!   `Access-Control-Allow-Origin: <origin>` + `Vary: Origin`——CORS 下
//!   无 ACAO 的错误响应对 fetch 呈网络错误形态，桌面将无法区分 401 与
//!   断网（I/O 矩阵「网络错误≠401」严格分流的必要条件）；
//!   并回写 `Access-Control-Expose-Headers: x-egosync-app-error`——
//!   非安全列表响应头对跨源 JS 默认不可读，不 expose 则桌面读不到
//!   业务错误判别头（200+错误体形态被当成功值 resolve）。
//!
//! 非白名单跨源 ⇒ 不经本层放行（[`crate::security::reject_cross_origin`]
//! 403 显式拒绝的冻结语义零变化——白名单判定两侧同源引用）。
//! Cookie 语义不动：本层不下发 `Access-Control-Allow-Credentials`（桌面
//! 远程走 Bearer，浏览器 Cookie 同源直连不跨源）。

use axum::extract::Request;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::routes::APP_ERROR_HEADER;

/// 桌面 webview 宿主源白名单（Tauri 2 跨平台形态）。
pub const TAURI_ORIGIN_ALLOWLIST: &[&str] = &["tauri://localhost", "http://tauri.localhost"];

/// 是否白名单桌面宿主源（security::reject_cross_origin 与本层同源判定）。
pub fn is_allowlisted_tauri_origin(origin: &str) -> bool {
    TAURI_ORIGIN_ALLOWLIST.contains(&origin)
}

/// CORS 预检放行头集合：桌面远程客户端实际使用的方法（GET/POST/OPTIONS）
/// 与头（Authorization=Bearer 通道、Content-Type=JSON body）。
const ALLOW_METHODS: &str = "GET, POST, OPTIONS";
const ALLOW_HEADERS: &str = "Authorization, Content-Type";

/// CORS 白名单中间件：白名单 Origin ⇒ preflight 短路 / 响应回写放行头；
/// 其余请求原样透传（跨源拒绝归 [`crate::security::reject_cross_origin`]）。
pub async fn cors_allowlist(req: Request, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .filter(|o| is_allowlisted_tauri_origin(o))
        .map(|s| s.to_string());

    let Some(origin) = origin else {
        // 无 Origin / 非白名单 Origin：本层零动作（浏览器同源与拒绝路径
        // 的既有行为零变化）
        return next.run(req).await;
    };

    // preflight（OPTIONS）：短路 204 + 放行头（不得落入业务路由——
    // OPTIONS 不是任何业务端点的方法语义）
    if req.method() == Method::OPTIONS {
        return preflight_response(&origin);
    }

    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    // 回显具体 Origin（非 `*`——白名单成员且无需凭证模式）
    if let Ok(value) = HeaderValue::from_str(&origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    // [评审轮2 U2] 业务错误判别头对跨源 JS 可读：CORS 下非安全列表
    // 响应头默认不可读——不 expose 则远程桌面（http.ts）读不到
    // `x-egosync-app-error`，200+错误体形态（AppError 单键对象）被当
    // 成功值 resolve（错误语义整体断裂——桌面=浏览器等价性恢复）。
    // 仅需加在实际响应上（预检响应无关——preflight 不携带业务头语义）。
    headers.insert(
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        HeaderValue::from_static(APP_ERROR_HEADER),
    );
    res
}

/// CORS 预检响应：204 + 放行头（方法/头/缓存窗口）。
fn preflight_response(origin: &str) -> Response {
    let mut res = StatusCode::NO_CONTENT.into_response();
    let headers = res.headers_mut();
    if let Ok(value) = HeaderValue::from_str(origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static(ALLOW_METHODS),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static(ALLOW_HEADERS),
    );
    // 预检缓存 10 分钟（一生低频切换的桌面客户端，免每请求双跳）
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("600"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    res
}
