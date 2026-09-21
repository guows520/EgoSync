//! Story 17.1 集成测试：TLS 反代感知（同源判定 + Cookie Secure）。
//!
//! I/O 矩阵前三行的中间件级验证（单测钉纯函数形状，本文件钉端到端
//! 行为——生产路由 + 真实 TCP）：
//!
//! | 场景 | 预期 |
//! |------|------|
//! | 反代后同源请求（BEHIND_PROXY=1 + XFP=https + https Origin + 无端口 Host） | 放行（修复前 403）+ login Set-Cookie 带 Secure |
//! | 直连 HTTP（未设门控） | 行为逐字节不变（无 Secure；http 缺省 80 比对） |
//! | 伪造 X-Forwarded-\*（直连暴露 + 客户端伪造头） | 头被忽略（403 不放行） |
//!
//! Host 头经 reqwest 显式覆写（hyper 尊重用户提供的 Host）；直连/反代
//! 两态各起独立 server（behind_proxy 是装配期门控，非请求期开关）。

mod common;

use common::InProcessServer;
use egosync_server::bootstrap::{build_test_state, build_test_state_with_proxy};
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN};

/// 临时数据目录（与 api_test.rs 同款：TempDir::keep 保持存活）。
fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("创建临时目录");
    dir.keep().join(tag)
}

/// 构造 `Origin` + `X-Forwarded-Proto` 头集（Host 经请求级覆写）。
fn proxy_headers(origin: &str, xfp: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, HeaderValue::from_str(origin).expect("Origin 头"));
    if let Some(xfp) = xfp {
        headers.insert(
            "x-forwarded-proto",
            HeaderValue::from_str(xfp).expect("XFP 头"),
        );
    }
    headers
}

/// 反代后同源请求：`https Origin`（缺省 443）对无端口 `Host` 放行
///（修复前按 http 缺省 80 比对 ⇒ 全量 403），login Set-Cookie 带
/// `Secure`；跨源 host 仍 403；代理链 XFP 取首值。
#[tokio::test]
async fn behind_proxy_same_origin_and_secure_cookie() {
    let state = build_test_state_with_proxy(temp_data_dir("proxy-tls"), Some("t".into()), true)
        .await
        .expect("反代态测试装配");
    let server = InProcessServer::start(state).await;
    let client = reqwest::Client::new();
    let login_url = server.url("/api/auth/login");

    // ── 同源 https 放行 + Set-Cookie Secure（矩阵行 1）──
    let res = client
        .post(&login_url)
        .header("host", "dom.com")
        .headers(proxy_headers("https://dom.com", Some("https")))
        .json(&serde_json::json!({"token": "t"}))
        .send()
        .await
        .expect("反代后同源 login");
    assert_eq!(res.status(), 200, "XFP=https 缺省端口归一 443 ⇒ 放行（修复前 403）");
    let set_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("login 必须换发 Set-Cookie")
        .to_string();
    assert!(set_cookie.contains("egosync_session="), "Cookie 形状: {set_cookie}");
    assert!(
        set_cookie.contains("Secure"),
        "BEHIND_PROXY + XFP=https ⇒ Set-Cookie 须含 Secure: {set_cookie}"
    );
    assert!(set_cookie.contains("HttpOnly"), "既有属性保持: {set_cookie}");
    assert!(set_cookie.contains("SameSite=Strict"), "既有属性保持: {set_cookie}");

    // ── 跨源 host 仍拒绝（反代感知只修缺省端口归一，不弱化跨源拒绝）──
    let res = client
        .post(&login_url)
        .header("host", "dom.com")
        .headers(proxy_headers("https://evil.example", Some("https")))
        .json(&serde_json::json!({"token": "t"}))
        .send()
        .await
        .expect("反代后跨源 login");
    assert_eq!(res.status(), 403, "跨源 host 在反代态仍 403");

    // ── XFP 代理链取首值（`https, http` ⇒ https 归一）──
    let res = client
        .post(&login_url)
        .header("host", "dom.com")
        .headers(proxy_headers("https://dom.com", Some("https, http")))
        .json(&serde_json::json!({"token": "t"}))
        .send()
        .await
        .expect("代理链 XFP login");
    assert_eq!(res.status(), 200, "XFP 代理链首值 https ⇒ 放行");

    // ── 反代态但 XFP 缺失：https Origin 无法证明同源 ⇒ 403（诚实拒绝，
    //    与「门控开启即放行一切」的弱化实现相区分）──
    let res = client
        .post(&login_url)
        .header("host", "dom.com")
        .headers(proxy_headers("https://dom.com", None))
        .json(&serde_json::json!({"token": "t"}))
        .send()
        .await
        .expect("反代态无 XFP login");
    assert_eq!(res.status(), 403, "XFP 缺失时缺省 http（80）⇒ https Origin 拒绝");

    // ── logout 过期 Cookie 同属性追加 Secure（签发/过期须对称）──
    let res = client
        .post(server.url("/api/auth/logout"))
        .header("host", "dom.com")
        .headers(proxy_headers("https://dom.com", Some("https")))
        .send()
        .await
        .expect("反代后 logout");
    assert_eq!(res.status(), 200, "logout 幂等 200");
    let set_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("logout 必须回过期 Set-Cookie")
        .to_string();
    assert!(
        set_cookie.contains("Max-Age=0") && set_cookie.contains("Secure"),
        "logout 过期 Cookie 须含 Max-Age=0 + Secure: {set_cookie}"
    );
}

/// 直连 HTTP（dev/e2e 形态）：行为逐字节不变——https Origin 对直连
/// Host 仍 403（矩阵行 2），客户端伪造 XFP=https 被忽略（矩阵行 3），
/// login Set-Cookie 无 Secure。
#[tokio::test]
async fn direct_http_unchanged_and_forged_xfp_ignored() {
    let state = build_test_state(temp_data_dir("proxy-direct"), Some("t".into()))
        .await
        .expect("直连态测试装配");
    let server = InProcessServer::start(state).await;
    let client = reqwest::Client::new();
    let login_url = server.url("/api/auth/login");

    // 直连同源（http + 端口 Host）：放行，Set-Cookie 无 Secure
    let same_origin = format!("http://{}", server.addr);
    let res = client
        .post(&login_url)
        .header(ORIGIN, &same_origin)
        .json(&serde_json::json!({"token": "t"}))
        .send()
        .await
        .expect("直连同源 login");
    assert_eq!(res.status(), 200, "直连同源放行（既有行为）");
    let set_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("login Set-Cookie")
        .to_string();
    assert!(
        !set_cookie.contains("Secure"),
        "直连（未设门控）Set-Cookie 不得带 Secure: {set_cookie}"
    );

    // 伪造 XFP=https（矩阵行 3）：门控未启用 ⇒ 头被忽略 ⇒ https Origin
    // 对直连 Host（缺省 80）拒绝（与 15.4 行为一致）
    let res = client
        .post(&login_url)
        .header("host", "dom.com")
        .headers(proxy_headers("https://dom.com", Some("https")))
        .json(&serde_json::json!({"token": "t"}))
        .send()
        .await
        .expect("直连 + 伪造 XFP login");
    assert_eq!(
        res.status(),
        403,
        "直连暴露下伪造 X-Forwarded-Proto 不得被信任（头被忽略）"
    );
}
