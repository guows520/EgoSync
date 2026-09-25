//! Story 16.1 server 集成测试：静态服务（SPA 回退）+ logout + 安全头。
//!
//! 覆盖（I/O 矩阵逐行）：
//! - 静态服务：index 命中（no-cache）/ 深路径 SPA 回退（no-cache）/
//!   资产命中与缺失 404 / `/api/*` JSON 404 保持（GET 与 POST 皆不落入
//!   静态面）/ 路由优先于 fallback（healthz 不被吞）/ 未配置目录
//!   API-only；
//! - PWA 外壳（Story 16.4）：manifest / sw.js / 图标静态命中且带 CSP
//!   新指令（manifest-src / worker-src 'self'——只增不减）；
//! - logout：有效会话删行 + Cookie 过期（Max-Age=0 同属性）+ 幂等 200
//!   （无会话/已删会话/伪造值）+ 后续业务请求 401；
//! - 安全头：CSP + nosniff + X-Frame-Options: DENY + Referrer-Policy:
//!   no-referrer 全响应（HTML / JSON 404 / healthz）。
//!
//! fixture dist：测试自建临时目录（index.html + assets/app.js +
//! manifest/sw/icons）——不读
//! 工作区真实 `../egosync-app/dist`（测试面与前端构建产物解耦，cargo test
//! 无需前端产物即可运行）。
//!
//! 临时目录生命周期（评审修复）：[`TestDirs`] 守卫持有 TempDir——声明
//! 顺序保证先于 server 声明 ⇒ 后于 server drop（连接关闭后目录回收，
//! 不再以 `keep()` 永久泄漏 /tmp）。logout 断言拆两个测试：单 server 的
//! 认证面请求 4 次 + 2 次（limit 5/min/IP 留余量——后续追加断言不致
//! 撞 429 假红）。

mod common;

use common::{Client, InProcessServer};
use egosync_server::auth::SESSION_COOKIE;
use egosync_server::bootstrap::build_test_state;
use egosync_server::security::CSP_POLICY;
use serde_json::{json, Value};

/// 测试目录守卫：data 子目录（SQLite）+ 可选 fixture dist 子目录。
///
/// 拥有 [`tempfile::TempDir`]——守卫存活期 = 目录存活期，drop 时连同
/// 内容回收。**声明顺序即 drop 顺序**：守卫须在 server 之前声明（Rust
/// 逆序 drop ⇒ server/连接先关，目录最后删——Linux 上文件句柄未关也可
/// 安全 unlink，顺序仅为整洁）。
struct TestDirs {
    _guard: tempfile::TempDir,
    /// 数据目录（egosync.db / conversations.db 落点）。
    data: std::path::PathBuf,
    /// fixture dist（`with_fixture` 为 true 时 Some）。
    dist: Option<std::path::PathBuf>,
}

impl TestDirs {
    /// 新建守卫目录（`with_fixture` = true 时写 fixture dist）。
    fn new(tag: &str, with_fixture: bool) -> Self {
        let guard = tempfile::tempdir().expect("创建临时目录");
        let data = guard.path().join(tag);
        let dist = with_fixture.then(|| {
            let dist = guard.path().join(format!("{}-dist", tag));
            std::fs::create_dir_all(dist.join("assets")).expect("建 fixture assets 目录");
            std::fs::write(dist.join("index.html"), FIXTURE_INDEX_HTML)
                .expect("写 fixture index.html");
            std::fs::write(dist.join("assets").join("app.js"), "// fixture asset\n")
                .expect("写 fixture 资产");
            // Story 16.4：PWA 外壳文件（manifest / Service Worker / 图标）——
            // 与真实 public/ 布局同构，供静态命中 + CSP 新指令断言消费
            std::fs::write(dist.join("manifest.webmanifest"), "{\"name\":\"fixture\"}\n")
                .expect("写 fixture manifest");
            std::fs::write(dist.join("sw.js"), "// fixture service worker\n")
                .expect("写 fixture sw.js");
            std::fs::create_dir_all(dist.join("icons")).expect("建 fixture icons 目录");
            std::fs::write(dist.join("icons").join("icon-192.png"), "fixture-png\n")
                .expect("写 fixture icon-192.png");
            std::fs::write(dist.join("icons").join("icon-512.png"), "fixture-png\n")
                .expect("写 fixture icon-512.png");
            std::fs::write(dist.join("icons").join("apple-touch-icon.png"), "fixture-png\n")
                .expect("写 fixture apple-touch-icon.png");
            dist
        });
        Self { _guard: guard, data, dist }
    }
}

const FIXTURE_INDEX_HTML: &str = concat!(
    "<!doctype html>\n",
    "<html lang=\"zh-CN\">\n",
    "  <head><meta charset=\"UTF-8\" /><title>EgoSync</title></head>\n",
    "  <body>\n",
    "    <div id=\"root\"></div>\n",
    "    <div id=\"egosync-splash\"></div>\n",
    "    <script type=\"module\" src=\"/assets/app.js\"></script>\n",
    "  </body>\n",
    "</html>\n",
);

// ── 静态服务：index / 资产 / SPA 回退 / API 守卫 ──

#[tokio::test]
async fn static_serves_index_assets_and_spa_fallback() {
    let dirs = TestDirs::new("static", true);
    let state = build_test_state(dirs.data.clone(), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start_with_static(state, dirs.dist.clone()).await;
    let client = Client::new();

    // 根路径：index.html 显式路由直出（no-cache——重部署后旧 index 引用
    // 已 404 hash 资产会导致白屏，须每次进入取最新）
    let res = client.get(&server.url("/"), None, None).await;
    assert_eq!(res.status(), 200);
    assert!(
        res.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("text/html")),
        "index.html 必须 text/html"
    );
    assert_eq!(
        res.headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("no-cache"),
        "index.html 直出必须 no-cache"
    );
    let body = res.text().await.expect("index body");
    assert_eq!(body, FIXTURE_INDEX_HTML, "根路径必须命中同一构建产物 index.html");

    // 存量资产命中：assets/app.js（content-type 由 ServeDir 推导）
    let res = client.get(&server.url("/assets/app.js"), None, None).await;
    assert_eq!(res.status(), 200);
    assert!(
        res.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("javascript")),
        "JS 资产 content-type 必须为 javascript: {:?}",
        res.headers().get(reqwest::header::CONTENT_TYPE)
    );

    // 深路径 SPA 回退：未知非 API 路由形态 ⇒ 200 index.html（no-cache 同上）
    for path in ["/unknown-path", "/settings-demo", "/chat/some/deep/route"] {
        let res = client.get(&server.url(path), None, None).await;
        assert_eq!(res.status(), 200, "{} 应 SPA 回退 200", path);
        assert_eq!(
            res.headers()
                .get(reqwest::header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok()),
            Some("no-cache"),
            "SPA 回退 index.html 同样 no-cache（{}）",
            path
        );
        let body = res.text().await.expect("SPA 回退 body");
        assert_eq!(body, FIXTURE_INDEX_HTML, "{} 应回退到 index.html", path);
    }

    // 缺失资产：资产形态（末段含扩展名）如实 404——SPA 回退只救路由深链
    let res = client.get(&server.url("/assets/missing.js"), None, None).await;
    assert_eq!(res.status(), 404, "缺失资产必须 404");
    let res = client.get(&server.url("/missing.txt"), None, None).await;
    assert_eq!(res.status(), 404, "根下缺失的带扩展名路径同样 404");

    // API 未知路径：JSON 404 保持（GET 与 POST 都不得落入静态/SPA 面）
    let res = client.get(&server.url("/api/unknown"), None, None).await;
    assert_eq!(res.status(), 404);
    let err: Value = res.json().await.expect("API 404 body");
    assert_eq!(err["error"], "not found", "API 面错误形状保持");
    let res = client
        .post_json(&server.url("/api/unknown"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 404, "POST /api/* 未知路径同样 404（不被 ServeDir 405 短路）");
    let err: Value = res.json().await.expect("API 404 body");
    assert_eq!(err["error"], "not found");

    // 路由优先于 fallback：healthz 不被静态面吞掉
    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("healthz body");
    assert_eq!(body["status"], "ok");

    // 业务面（认证守门）不受静态挂载影响：无 Cookie ⇒ 401
    let res = client
        .post_json(&server.url("/api/cmd/role_list"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn static_dir_unconfigured_runs_api_only() {
    // dist 未配置（env 未设且默认路径不存在——InProcessServer 默认 None）：
    // API-only + 不崩溃（I/O 矩阵）
    let dirs = TestDirs::new("api-only", false);
    let state = build_test_state(dirs.data.clone(), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 静态面整体不挂载：根路径与其余非 API 路径 404
    let res = client.get(&server.url("/"), None, None).await;
    assert_eq!(res.status(), 404, "未配置目录时根路径 404（API-only）");
    let res = client.get(&server.url("/unknown-path"), None, None).await;
    assert_eq!(res.status(), 404);
    let err: Value = res.json().await.expect("404 body");
    assert_eq!(err["error"], "not found");

    // API 面照常：healthz 200 / login 200
    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(res.status(), 200);
    let body = json!({"token": "t"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&body), None, None)
        .await;
    assert_eq!(res.status(), 200, "API-only 模式认证面照常");
}

// ── PWA 外壳文件（Story 16.4）：manifest / sw.js / 图标静态命中 + CSP 新指令 ──

#[tokio::test]
async fn static_serves_pwa_shell_files_with_csp_new_directives() {
    let dirs = TestDirs::new("pwa-shell", true);
    let state = build_test_state(dirs.data.clone(), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start_with_static(state, dirs.dist.clone()).await;
    let client = Client::new();

    // CSP 含 16.4 增量指令（manifest-src / worker-src 'self'——PWA 安装
    // 与 SW 注册的 CSP 放行面；常量被删即红）
    assert!(
        CSP_POLICY.contains("manifest-src 'self'"),
        "CSP_POLICY 应含 manifest-src 'self'（PWA manifest 同源加载）"
    );
    assert!(
        CSP_POLICY.contains("worker-src 'self'"),
        "CSP_POLICY 应含 worker-src 'self'（Service Worker 同源注册）"
    );

    // 外壳文件逐一静态命中且全响应带 CSP（含 index/资产/api 同策略）
    for path in [
        "/manifest.webmanifest",
        "/sw.js",
        "/icons/icon-192.png",
        "/icons/icon-512.png",
        "/icons/apple-touch-icon.png",
    ] {
        let res = client.get(&server.url(path), None, None).await;
        assert_eq!(res.status(), 200, "{} 应静态命中", path);
        assert_eq!(
            res.headers()
                .get(reqwest::header::CONTENT_SECURITY_POLICY)
                .and_then(|v| v.to_str().ok()),
            Some(CSP_POLICY),
            "{} 必须带 CSP（静态响应同策略）",
            path
        );
    }
}

// ── logout：删行 + Cookie 过期 + 幂等 ──
// 拆两个测试（评审修复）：认证面限流 5/min/IP——单 server 请求 4 次 + 2 次
// 留余量（原单测 5 次恰好贴上限，追加断言即 429 假红）。

#[tokio::test]
async fn logout_deletes_session_expires_cookie_and_rejects_reuse() {
    let dirs = TestDirs::new("logout", false);
    let state = build_test_state(dirs.data.clone(), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 登录换会话（捕获完整 Set-Cookie 与会话 Cookie 值）
    let login_body = json!({"token": "t"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&login_body), None, None)
        .await;
    assert_eq!(res.status(), 200);
    let login_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("登录 Set-Cookie")
        .to_string();
    let session = login_cookie.split(';').next().unwrap_or("").trim().to_string();
    assert!(session.starts_with(SESSION_COOKIE));

    // 登出：200 + Cookie 过期（同属性 Max-Age=0）
    let res = client
        .post_json(&server.url("/api/auth/logout"), Some(&json!({})), Some(&session), None)
        .await;
    assert_eq!(res.status(), 200, "logout 携带有效会话必须 200");
    // 先取头再取 body（json() 消费 Response）
    let logout_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .expect("登出必须下发过期 Cookie")
        .to_string();
    let ok: Value = res.json().await.expect("logout body");
    assert_eq!(ok["ok"], true);
    assert!(
        logout_cookie.starts_with(&format!("{}=;", SESSION_COOKIE)),
        "登出 Cookie 必须清值: {}",
        logout_cookie
    );
    assert!(logout_cookie.contains("Max-Age=0"), "登出 Cookie 必须过期: {}", logout_cookie);
    assert!(logout_cookie.contains("HttpOnly"), "同属性 httpOnly: {}", logout_cookie);
    assert!(logout_cookie.contains("SameSite=Strict"), "同属性 SameSite=Strict: {}", logout_cookie);
    assert!(logout_cookie.contains("Path=/"), "同属性 Path=/: {}", logout_cookie);

    // 会话行已删：同一 Cookie 的后续业务请求 ⇒ 401（不可复用）
    let res = client
        .post_json(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 401, "登出后会话行必须已删（后续请求 401）");

    // 幂等：已删会话再次登出 ⇒ 仍 200（不 401——require_auth 误判路径）
    let res = client
        .post_json(&server.url("/api/auth/logout"), Some(&json!({})), Some(&session), None)
        .await;
    assert_eq!(res.status(), 200, "已登出会话再登出必须幂等 200");
}

#[tokio::test]
async fn logout_without_valid_session_is_idempotent() {
    let dirs = TestDirs::new("logout-idem", false);
    let state = build_test_state(dirs.data.clone(), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 幂等：无 Cookie 登出 ⇒ 200 + 过期 Cookie
    let res = client
        .post_json(&server.url("/api/auth/logout"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 200, "无会话登出必须幂等 200");
    assert!(
        res.headers().contains_key(reqwest::header::SET_COOKIE),
        "无会话登出也下发过期 Cookie"
    );

    // 伪造 Cookie 登出 ⇒ 幂等 200（DELETE 匹配 0 行，不泄露有效性）
    let res = client
        .post_json(
            &server.url("/api/auth/logout"),
            Some(&json!({})),
            Some("egosync_session=00000000-0000-4000-8000-000000000000"),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "伪造会话登出必须幂等 200");
}

// ── 安全头：CSP + nosniff + X-Frame-Options + Referrer-Policy 全响应 ──

#[tokio::test]
async fn security_headers_cover_static_and_api_responses() {
    let dirs = TestDirs::new("sec-headers", true);
    let state = build_test_state(dirs.data.clone(), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start_with_static(state, dirs.dist.clone()).await;
    let client = Client::new();

    let expect_security_headers = |res: &reqwest::Response| {
        assert_eq!(
            res.headers().get(reqwest::header::CONTENT_SECURITY_POLICY)
                .and_then(|v| v.to_str().ok()),
            Some(CSP_POLICY),
            "CSP 必须全响应下发"
        );
        assert_eq!(
            res.headers().get(reqwest::header::X_CONTENT_TYPE_OPTIONS)
                .and_then(|v| v.to_str().ok()),
            Some("nosniff"),
            "nosniff 必须全响应下发"
        );
        assert_eq!(
            res.headers().get(reqwest::header::X_FRAME_OPTIONS)
                .and_then(|v| v.to_str().ok()),
            Some("DENY"),
            "X-Frame-Options: DENY 必须全响应下发"
        );
        assert_eq!(
            res.headers().get(reqwest::header::REFERRER_POLICY)
                .and_then(|v| v.to_str().ok()),
            Some("no-referrer"),
            "Referrer-Policy: no-referrer 必须全响应下发"
        );
    };

    // 静态 HTML（index 命中 + SPA 回退）
    let res = client.get(&server.url("/"), None, None).await;
    assert_eq!(res.status(), 200);
    expect_security_headers(&res);

    // 静态资产
    let res = client.get(&server.url("/assets/app.js"), None, None).await;
    assert_eq!(res.status(), 200);
    expect_security_headers(&res);

    // API JSON 200（healthz）
    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(res.status(), 200);
    expect_security_headers(&res);

    // 404（静态资产缺失）
    let res = client.get(&server.url("/assets/missing.js"), None, None).await;
    assert_eq!(res.status(), 404);
    expect_security_headers(&res);

    // 401（未认证业务请求）
    let res = client
        .post_json(&server.url("/api/cmd/role_list"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 401);
    expect_security_headers(&res);
}
