//! 安全中间件（Story 15.4）：CSP 全响应下发 + 跨源显式拒绝 + panic 兜底。
//!
//! - CSP（架构 ⑧）：`default-src 'self'; script-src 'self'; style-src 'self'
//!   'unsafe-inline'; connect-src 'self'`——全响应下发（含 healthz / 401 /
//!   404；静态资源归 16.1）。
//! - 跨源（架构 ⑧）：`Origin` 头与宿主不同源 ⇒ 403 显式拒绝，不下发任何
//!   CORS 放行头。无 Origin 头的请求（curl / 服务间调用）放行。
//! - panic 兜底：命令 handler panic 经 CatchPanicLayer → 500（进程级 5xx
//!   白名单内），响应体不携带 panic 细节（防信息泄露）。

use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

/// CSP 头值（架构 ⑧ 冻结内容）。
pub const CSP_POLICY: &str =
    "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'";

/// 全响应下发 CSP 头。
pub async fn csp_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP_POLICY),
    );
    res
}

/// 跨源显式拒绝：`Origin` 存在且与请求宿主不同源 ⇒ 403（无 CORS 放行头）。
///
/// 同源判定：Origin 的 host[:port] 归一化后与 Host 头一致（scheme 按直连
/// HTTP 语义；`Origin: null` 视为跨源拒绝）。端口缺省按 scheme 归一
/// （http→80）。
///
/// Origin 存在而 Host 缺失/不可解析 ⇒ 直接 403（二轮评审 #1：原
/// `(Some, Some)` 匹配式在该形态下整体跳过检查放行——跨源拒绝冻结
/// 语义被旁路；同源判定需要宿主，宿主不可得时按不可证明同源处理）。
pub async fn reject_cross_origin(req: Request, next: Next) -> Response {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match (origin, host) {
        // Origin 存在：必须有可解析 Host 且同源，否则拒绝
        (Some(origin), Some(host)) => {
            if !is_same_origin(&origin, &host) {
                // 显式拒绝：不带任何 CORS 放行头（不回 Origin/ACAO/ACAC）。
                return cross_origin_rejected();
            }
        }
        // Host 缺失/非 UTF-8 ⇒ 无法证明同源 ⇒ 拒绝（不旁路检查）
        (Some(_), None) => return cross_origin_rejected(),
        // 无 Origin 头的请求（curl / 服务间调用）放行
        (None, _) => {}
    }
    next.run(req).await
}

/// 跨源拒绝响应（403 统一形状）。
fn cross_origin_rejected() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error": "cross-origin request rejected"})),
    )
        .into_response()
}

/// Origin（`scheme://host[:port]`）与 Host（`host[:port]`）是否同源。
fn is_same_origin(origin: &str, host: &str) -> bool {
    // "null" Origin（沙箱 iframe / file://）不同源
    if origin.eq_ignore_ascii_case("null") {
        return false;
    }
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    let (origin_host, origin_port) = split_host_port(rest);
    let (req_host, req_port) = split_host_port(host);
    // 15.4 直连无 TLS：https Origin 与 HTTP 宿主不同源（17.1 反代定稿后复核）
    let scheme_port: u16 = match scheme {
        "http" => 80,
        "https" => 443,
        _ => return false,
    };
    if origin_host != req_host {
        return false;
    }
    origin_port.unwrap_or(scheme_port) == req_port.unwrap_or(80)
}

/// 拆 `host[:port]`（IPv6 字面量本形态不用于同源判定路径，简化处理）。
fn split_host_port(authority: &str) -> (String, Option<u16>) {
    match authority.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => {
            (h.to_ascii_lowercase(), p.parse().ok())
        }
        _ => (authority.to_ascii_lowercase(), None),
    }
}

/// handler panic 兜底响应（CatchPanicLayer）：500 + 不泄露 panic 细节。
pub fn panic_response(_panic: Box<dyn std::any::Any + Send>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": "internal server error"})),
    )
        .into_response()
}
