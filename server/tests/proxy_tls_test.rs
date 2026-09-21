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
//! Story 17.3（17.1 评审 #4 收口）：限流键 XFF 感知——反代态取
//! X-Forwarded-For 最右值（单可信跳语义），不同客户端分桶互不挤兑；
//! 无 XFF 回落 socket IP；直连态伪造 XFF 被忽略。
//!
//! Host 头经 reqwest 显式覆写（hyper 尊重用户提供的 Host）；直连/反代
//! 两态各起独立 server（behind_proxy 是装配期门控，非请求期开关）。

mod common;

use common::InProcessServer;
use egosync_server::bootstrap::{build_test_state, build_test_state_with_proxy};
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN};
use serde_json::json;

/// 登录失败响应（不消费 Set-Cookie——计数面观察）。
async fn failed_login(
    client: &reqwest::Client,
    url: &str,
    xff: Option<&str>,
) -> reqwest::StatusCode {
    let mut req = client.post(url).json(&json!({"token": "wrong"}));
    if let Some(xff) = xff {
        req = req.header("x-forwarded-for", xff);
    }
    let res = req.send().await.expect("失败 login 请求");
    res.status()
}

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

// ── Story 17.3：限流键 XFF 感知（17.1 评审 #4 收口）──

/// 反代态（BEHIND_PROXY=1）：登录失败按 XFF 最右值分桶——客户端 A（伪造
/// 多跳链，最右=真实 A）5 次失败后超限 429，客户端 B（不同最右值）不受
/// 挤兑仍 401；无 XFF 回落 socket IP。
#[tokio::test]
async fn behind_proxy_login_rate_limit_buckets_by_rightmost_xff() {
    let state = build_test_state_with_proxy(temp_data_dir("xff-login"), Some("t".into()), true)
        .await
        .expect("反代态测试装配");
    let server = InProcessServer::start(state).await;
    let client = reqwest::Client::new();
    let login_url = server.url("/api/auth/login");

    // 客户端 A：伪造多跳链 `1.2.3.4, 198.51.100.10`（模拟客户端伪造左侧 +
    // caddy 追加真实客户端 IP=最右值）；5 次失败（401）
    for i in 1..=5 {
        let status = failed_login(&client, &login_url, Some("1.2.3.4, 198.51.100.10")).await;
        assert_eq!(status, 401, "客户端 A 第 {i} 次失败应为 401（未超限）");
    }
    // 客户端 A 第 6 次（链左侧再变也不影响——桶键恒最右值）⇒ 429
    let status = failed_login(&client, &login_url, Some("9.9.9.9, 198.51.100.10")).await;
    assert_eq!(
        status, 429,
        "同一最右值的第 6 次失败必须 429（左侧伪造不改变桶键）"
    );

    // 客户端 B（不同最右值）：不受 A 的失败挤兑 ⇒ 仍 401（而非 429）
    let status = failed_login(&client, &login_url, Some("5.6.7.8, 198.51.100.20")).await;
    assert_eq!(
        status, 401,
        "反代态限流按客户端分桶——合法不同 IP 用户互不挤兑（17.1 评审 #4 收口）"
    );

    // 无 XFF（直连 app 端口的调试路径）⇒ 回落 socket IP：与前述 XFF 桶独立
    let status = failed_login(&client, &login_url, None).await;
    assert_eq!(
        status, 401,
        "无 XFF 回落 socket IP（独立桶——caddy 链路故障不殃及直连调试）"
    );
}

/// 直连态（门控未启用）：伪造 XFF 不得改变限流桶——恒 socket IP（5 次
/// 失败后第 6 次 429，与 15.4 行为一致——直连语义零变化）。
#[tokio::test]
async fn direct_mode_forged_xff_does_not_change_rate_limit_bucket() {
    let state = build_test_state(temp_data_dir("xff-direct"), Some("t".into()))
        .await
        .expect("直连态测试装配");
    let server = InProcessServer::start(state).await;
    let client = reqwest::Client::new();
    let login_url = server.url("/api/auth/login");

    // 5 次失败（每次伪造不同 XFF——门控未启用时头一律忽略）
    for i in 1..=5 {
        let status = failed_login(&client, &login_url, Some("198.51.100.30")).await;
        assert_eq!(status, 401, "直连态第 {i} 次失败应为 401");
    }
    // 第 6 次（再换伪造 XFF）⇒ 429——桶键恒 socket IP，伪造不得换桶
    let status = failed_login(&client, &login_url, Some("198.51.100.99")).await;
    assert_eq!(
        status, 429,
        "直连态伪造 XFF 不得绕过/转移限流桶（头被忽略，恒 socket IP）"
    );
}

/// 反代态 Bearer 失败面（T5 修订限流器）：同按 XFF 最右值分桶——客户端 A
/// Bearer 失败 5 次超限，客户端 B 持正确令牌不受影响（成功不计数）。
#[tokio::test]
async fn behind_proxy_bearer_failures_bucket_by_rightmost_xff() {
    let state = build_test_state_with_proxy(temp_data_dir("xff-bearer"), Some("t".into()), true)
        .await
        .expect("反代态测试装配");
    let server = InProcessServer::start(state).await;
    let client = reqwest::Client::new();
    let cmd_url = server.url("/api/cmd/role_list");

    // 客户端 A（最右 198.51.100.10）：Bearer 失败 5 次（401）
    for i in 1..=5 {
        let res = client
            .post(&cmd_url)
            .header("x-forwarded-for", "1.2.3.4, 198.51.100.10")
            .bearer_auth("wrong-token")
            .json(&json!({}))
            .send()
            .await
            .expect("A Bearer 失败请求");
        assert_eq!(res.status(), 401, "客户端 A 第 {i} 次 Bearer 失败应为 401");
    }
    // 客户端 A 第 6 次 ⇒ 429
    let res = client
        .post(&cmd_url)
        .header("x-forwarded-for", "9.9.9.9, 198.51.100.10")
        .bearer_auth("wrong-token-again")
        .json(&json!({}))
        .send()
        .await
        .expect("A 超限请求");
    assert_eq!(res.status(), 429, "同最右值 Bearer 失败超限须 429");

    // 客户端 B（不同最右值 + 正确令牌）：成功访问不受 A 失败挤兑
    let res = client
        .post(&cmd_url)
        .header("x-forwarded-for", "5.6.7.8, 198.51.100.20")
        .bearer_auth("t")
        .json(&json!({}))
        .send()
        .await
        .expect("B 正确令牌请求");
    assert_eq!(
        res.status(), 200,
        "合法客户端不受他人失败挤兑（仅计失败、成功不计数）"
    );
}

/// 反代态 SSE 流端点（`GET /api/events` 的 `require_auth_with_sse_ticket`
/// Bearer 分支）：同按 XFF 最右值分桶——评审补丁（17.3 分诊 P2，验证缺口
/// 层发现：SSE 面是 17.1 #4 收口的四个应用点中唯一无反代态测试的一个，
/// 键回退为 socket IP 时全部既有测试照绿）。客户端 A 伪造多跳 Bearer
/// 失败 5 次超限 429，客户端 B（不同最右值）不受挤兑仍 401。
#[tokio::test]
async fn behind_proxy_sse_bearer_failures_bucket_by_rightmost_xff() {
    let state = build_test_state_with_proxy(temp_data_dir("xff-sse"), Some("t".into()), true)
        .await
        .expect("反代态测试装配");
    let server = InProcessServer::start(state).await;
    let client = reqwest::Client::new();
    let events_url = server.url("/api/events");

    // 客户端 A（最右 198.51.100.10）：SSE 流端点 Bearer 失败 5 次（401）
    for i in 1..=5 {
        let res = client
            .get(&events_url)
            .header("x-forwarded-for", "1.2.3.4, 198.51.100.10")
            .bearer_auth("wrong-token")
            .send()
            .await
            .expect("A SSE Bearer 失败请求");
        assert_eq!(res.status(), 401, "客户端 A 第 {i} 次 SSE Bearer 失败应为 401");
    }
    // 客户端 A 第 6 次 ⇒ 429（同最右值超限——左侧伪造不换桶）
    let res = client
        .get(&events_url)
        .header("x-forwarded-for", "9.9.9.9, 198.51.100.10")
        .bearer_auth("wrong-token-again")
        .send()
        .await
        .expect("A SSE 超限请求");
    assert_eq!(res.status(), 429, "SSE 流端点同最右值 Bearer 失败超限须 429");

    // 客户端 B（不同最右值）：不受 A 失败挤兑 ⇒ 401（非 429）
    let res = client
        .get(&events_url)
        .header("x-forwarded-for", "5.6.7.8, 198.51.100.20")
        .bearer_auth("also-wrong")
        .send()
        .await
        .expect("B SSE 请求");
    assert_eq!(
        res.status(), 401,
        "SSE 面反代限流按客户端分桶——远程桌面事件流不受他人失败挤兑"
    );
}
