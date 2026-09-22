//! 安全中间件（Story 15.4，16.1 补齐响应头，17.1 反代感知）：CSP +
//! 安全响应头全响应下发 + 跨源显式拒绝 + panic 兜底。
//!
//! - CSP（架构 ⑧ 冻结内容 + 16.1 脚本 hash 裁量 + img-src/font-src data:
//!   裁量）：`default-src 'self'; script-src 'self' 'sha256-…'; style-src
//!   'self' 'unsafe-inline'; font-src 'self' data:; connect-src 'self';
//!   img-src 'self' data:`——全响应下发（含 healthz / 401 / 404 / 静态
//!   HTML）。`script-src` 追加 hash-source 放行 index.html 的内联防
//!   FOUC 主题脚本（禁止整体放开 unsafe-inline；hash 与构建产物 byte
//!   对 byte 钉死——前端 security 契约源码扫描测试守门）。`img-src` 的
//!   `data:` 放行 Vite 内联 SVG（logo/启动图标——见 [`CSP_POLICY`] 注
//!   释）；`font-src` 的 `data:` 放行 Vite 内联的小体积 woff2 字体分片
//!   （同机制——见 [`CSP_POLICY`] 注释）。16.1 曾放行的两 Google 字体
//!   域已随 Story 17.1 字体自托管（fontsource npm 包，woff2 随 dist
//!   分发）撤除——零第三方域不变（离线/内网一致性与隐私）。
//! - 安全响应头（15-4 遗留补齐，16.1）：`X-Content-Type-Options: nosniff`
//!   （MIME 嗅探防护）、`X-Frame-Options: DENY`（frame 嵌入防护）、
//!   `Referrer-Policy: no-referrer`（引用泄露防护）——全响应叠加。
//! - 跨源（架构 ⑧ + 17.1 反代感知）：`Origin` 头与宿主不同源 ⇒ 403
//!   显式拒绝，不下发任何 CORS 放行头。无 Origin 头的请求（curl /
//!   服务间调用）放行。**Story 17.1 TLS 反代缺口修复**：
//!   `EGOSYNC_BEHIND_PROXY=1` 时同源判定感知 `X-Forwarded-Proto`
//!   （生效 scheme ⇒ Host 缺省端口按该 scheme 归一）——修复 Caddy
//!   TLS 后 `Origin: https://dom.com`（缺省 443）对 `Host: dom.com`
//!   （原按 http 缺省 80 比对）全量误拒的 403 病灶。门控而非无条件
//!   信任：直连暴露（未设 env）下 X-Forwarded-\* 一律忽略（行为与
//!   15.4 逐字节一致——防伪造头信任）。
//! - panic 兜底：命令 handler panic 经 CatchPanicLayer → 500（进程级 5xx
//!   白名单内），响应体不携带 panic 细节（防信息泄露）。

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use serde_json::json;

use crate::AppState;

/// CSP 头值（架构 ⑧ 冻结内容 + 16.1 脚本 hash-source 裁量 + img-src /
/// font-src data: 裁量；字体源已随 17.1 自托管撤除——零第三方域）。
///
/// script hash 值为 `egosync-app/dist/index.html` 内联防 FOUC 脚本的 SHA256
/// base64（脚本本体 byte 对 byte——vite 构建原样保留 index.html 内联
/// 脚本；编辑该脚本须同步重算本值，前端 security 契约源码扫描测试守门）。
/// 字体经 fontsource npm 包自托管（woff2 随 dist 分发），桌面（tauri
/// csp:null）与 web 同链本地加载——视觉零分叉且离线一致。
///
/// `img-src 'self' data:`（2026-09-22 修复）：Vite 构建把小体积 SVG 内联为
/// `data:image/svg+xml` URI（index.html 启动图标 + Logo 组件——均随 dist
/// 分发，无外链）；未声明 img-src 时图片回退 `default-src 'self'`，`data:`
/// URI 不在 `'self'` 语义内 ⇒ 浏览器拦截（web 宿主 logo 不显示——e2e
/// chrome 日志实证 CSP violation）。放行范围刻意仅限图片侧 `data:`：
/// SVG 作 `<img>` 载入时浏览器禁用其内嵌脚本，无执行面；script/style/
/// connect 面零改动。契约测试同步钉死本指令（img-src 缺席即红）。
///
/// `font-src 'self' data:`（2026-09-22 修复，同机制）：Vite 对小于内联
/// 阈值的字体分片同样内联为 `data:font/woff2` URI（fontsource 分片中的
/// 小体积者——dist 产物 CSS 实证）；`font-src 'self'` 拦 data: URI ⇒
/// 字体回退系统字体（e2e chrome 日志 SEVERE 级 CSP violation 实证）。
/// 放行仅追加 `data:` scheme，零第三方域不变；契约测试同步钉死本指令。
pub const CSP_POLICY: &str = "default-src 'self'; script-src 'self' \
    'sha256-t7EoxfYkO3wNL2nCWqo4+0sb9alMtMK3TJiFg8kQUkQ='; style-src 'self' \
    'unsafe-inline'; font-src 'self' data:; connect-src 'self'; img-src 'self' data:";

/// 全响应下发 CSP + 安全响应头（nosniff / X-Frame-Options / Referrer-Policy）。
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP_POLICY),
    );
    // 15-4 遗留安全头补齐（16.1）：nosniff / frame 嵌入防护 / 引用泄露防护
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    res
}

/// 跨源显式拒绝：`Origin` 存在且与请求宿主不同源 ⇒ 403（无 CORS 放行头）。
///
/// 同源判定：Origin 的 host[:port] 归一化后与 Host 头一致（scheme 按生效
/// scheme 语义——见 [`effective_scheme`]；`Origin: null` 视为跨源拒绝）。
/// 端口缺省按生效 scheme 归一（http→80 / https→443；反代后生效 scheme
/// 取 `X-Forwarded-Proto` 首值——Story 17.1）。
///
/// Origin 存在而 Host 缺失/不可解析 ⇒ 直接 403（二轮评审 #1：原
/// `(Some, Some)` 匹配式在该形态下整体跳过检查放行——跨源拒绝冻结
/// 语义被旁路；同源判定需要宿主，宿主不可得时按不可证明同源处理）。
///
/// Story 16.3 白名单豁免：桌面 webview 宿主源（`tauri://localhost` /
/// `http://tauri.localhost`）显式放行——CORS 层（crate::cors）为白名单
/// 成员回写放行头；白名单判定两侧同源引用（单一名单，无第二清单）。
pub async fn reject_cross_origin(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
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
    // 反代感知（17.1）：仅 BEHIND_PROXY=1 时读取（直连暴露下忽略——
    // 客户端可伪造的头不构成信任依据）
    let forwarded_proto = if state.behind_proxy {
        req.headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    } else {
        None
    };

    match (origin, host) {
        // Origin 存在：必须有可解析 Host 且（同源或桌面白名单），否则拒绝
        (Some(origin), Some(host)) => {
            let scheme = effective_scheme(forwarded_proto.as_deref());
            if !is_same_origin(&origin, &host, &scheme)
                && !crate::cors::is_allowlisted_tauri_origin(&origin)
            {
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

/// 生效 scheme（Story 17.1）：同源判定的 Host 缺省端口归一依据。
///
/// `X-Forwarded-Proto` 首值（逗号分隔代理链取最左——最外层反代写入的
/// 就是客户端真实访问 scheme）；头缺失/未启用门控时为直连 HTTP（本
/// server 只讲 HTTP，TLS 终结在反代——架构冻结）。
///
/// 调用方（[`reject_cross_origin`]）已保证：直连暴露（BEHIND_PROXY 未
/// 启用）时不会传入该头的值——本函数对伪造头免疫由门控保证。
fn effective_scheme(forwarded_proto: Option<&str>) -> &str {
    match forwarded_proto
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("https") => "https",
        _ => "http",
    }
}

/// Origin（`scheme://host[:port]`）与 Host（`host[:port]`）是否同源。
///
/// `effective_scheme` 是 Host 侧的缺省端口归一依据（http→80 / https→443
/// ——反代 TLS 后 Host 无端口时按 443 比对，修复 15.4「Origin 缺省 443
/// 对 Host 缺省 80」的 403 病灶）；Origin 侧缺省端口按其自身 scheme。
/// Host 拆分括号感知 IPv6 字面量（`[::1]:8080` → host `::1` + port）。
fn is_same_origin(origin: &str, host: &str, effective_scheme: &str) -> bool {
    // "null" Origin（沙箱 iframe / file://）不同源
    if origin.eq_ignore_ascii_case("null") {
        return false;
    }
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    let (origin_host, origin_port) = split_host_port(rest);
    let (req_host, req_port) = split_host_port(host);
    let origin_scheme_port: u16 = match scheme.to_ascii_lowercase().as_str() {
        "http" => 80,
        "https" => 443,
        _ => return false,
    };
    if origin_host != req_host {
        return false;
    }
    // Host 缺省端口按生效 scheme 归一（直连 http=80——15.4 语义；
    // 反代 TLS https=443——17.1 修复）；Origin 缺省端口按自身 scheme。
    let host_default_port: u16 = if effective_scheme == "https" { 443 } else { 80 };
    origin_port.unwrap_or(origin_scheme_port) == req_port.unwrap_or(host_default_port)
}

/// 拆 `host[:port]`（Story 17.1：括号感知 IPv6 字面量）。
///
/// - `[::1]:8080` / `[::1]` → host 去括号小写（`::1`）+ 可选端口；
/// - 其余沿 `rsplit_once(':')` 且端口段全数字（IPv4/域名——端口段
///   含非数字即整体视为 host，兜底裸 IPv6 畸形输入）。
fn split_host_port(authority: &str) -> (String, Option<u16>) {
    // IPv6 字面量：方括号包 host，`]` 后 `:port` 为端口
    if let Some(rest) = authority.strip_prefix('[') {
        if let Some((host, tail)) = rest.split_once(']') {
            let port = tail.strip_prefix(':').and_then(|p| p.parse().ok());
            return (host.to_ascii_lowercase(), port);
        }
        // 括号未闭合的畸形输入：整体按 host 处理（无端口）
        return (authority.to_ascii_lowercase(), None);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── I/O 矩阵前三行 + IPv6 行（Story 17.1）──

    /// 行 1：反代后同源请求——`Origin: https://dom.com` + `Host: dom.com`
    /// + XFP https + 生效 scheme https ⇒ 放行（修复前 443 对 80 误拒）。
    #[test]
    fn same_origin_behind_tls_proxy_with_default_ports() {
        let scheme = effective_scheme(Some("https"));
        assert!(
            is_same_origin("https://dom.com", "dom.com", scheme),
            "反代 TLS：Origin 缺省 443 对 Host 缺省 443 ⇒ 同源"
        );
        // XFP 代理链取首值
        assert_eq!(effective_scheme(Some("https, http")), "https");
    }

    /// 行 2：直连 HTTP（dev/e2e）——生效 scheme http，现行为逐字节不变
    ///（http 缺省 80 比对；https Origin 对直连 Host 缺省 80 仍不同源）。
    #[test]
    fn same_origin_direct_http_semantics_unchanged() {
        let scheme = effective_scheme(None);
        assert_eq!(scheme, "http", "无 XFP ⇒ 直连 http");
        // http 同源（带端口）
        assert!(is_same_origin("http://127.0.0.1:8080", "127.0.0.1:8080", scheme));
        // http 同源（双方缺省 80）
        assert!(is_same_origin("http://dom.com", "dom.com", scheme));
        // 15.4 语义保持：https Origin 与直连 HTTP 宿主不同源
        assert!(
            !is_same_origin("https://dom.com", "dom.com", scheme),
            "直连下 https Origin（443）对 Host（80）仍不同源"
        );
        // 跨源 host ⇒ 拒绝
        assert!(!is_same_origin("http://evil.example", "dom.com", scheme));
        // Origin: null ⇒ 跨源
        assert!(!is_same_origin("null", "dom.com", scheme));
    }

    /// 行 3：伪造 X-Forwarded-\*——直连暴露下头被忽略（门控在
    /// [`reject_cross_origin`] 中间件层：behind_proxy=false 时不读头，
    /// 生效 scheme 恒为 http）。本用例钉死「头值不会凭空改变判定」。
    #[test]
    fn forged_forwarded_proto_ignored_without_gate() {
        // 门控关闭 ⇒ 调用方不传头值（effective_scheme(None)）
        let scheme = effective_scheme(None);
        assert!(
            !is_same_origin("https://dom.com", "dom.com", scheme),
            "直连暴露下伪造 XFP=https 不得放行 https Origin"
        );
    }

    /// 行 4：IPv6 字面量——括号感知拆分，同源判定正确。
    #[test]
    fn ipv6_literal_authority_split_and_match() {
        // 拆分形状
        assert_eq!(split_host_port("[::1]:8080"), ("::1".to_string(), Some(8080)));
        assert_eq!(split_host_port("[::1]"), ("::1".to_string(), None));
        assert_eq!(
            split_host_port("[2001:db8::1]:443"),
            ("2001:db8::1".to_string(), Some(443))
        );
        // 同源判定：Origin 与 Host 同为 [::1]:8080
        assert!(is_same_origin("http://[::1]:8080", "[::1]:8080", "http"));
        // 反代 TLS + IPv6（双方缺省 443）
        assert!(is_same_origin("https://[::1]", "[::1]", "https"));
        // 端口不同 ⇒ 不同源
        assert!(!is_same_origin("http://[::1]:8080", "[::1]:9090", "http"));
        // 非 IPv6 路径回归：域名 + 端口
        assert_eq!(split_host_port("dom.com:8080"), ("dom.com".to_string(), Some(8080)));
        assert_eq!(split_host_port("DOM.com"), ("dom.com".to_string(), None));
    }

    /// 反代 TLS 的边界：Origin 显式端口与 Host 显式端口仍须精确比对
    ///（缺省归一只补未写出的端口，不吞显式差异）。
    #[test]
    fn proxy_mode_explicit_ports_still_compared() {
        assert!(is_same_origin("https://dom.com:8443", "dom.com:8443", "https"));
        assert!(
            !is_same_origin("https://dom.com:8443", "dom.com", "https"),
            "显式 8443 对缺省 443 ⇒ 不同源"
        );
        // XFP=http 的反代（明文入口）：缺省 80 比对
        assert!(is_same_origin("http://dom.com", "dom.com", "http"));
    }
}
