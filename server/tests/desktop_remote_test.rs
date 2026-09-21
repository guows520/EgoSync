//! Story 16.3 server 集成测试：桌面远程模式叠加通道全覆盖。
//!
//! 三面（与 spec Verification 对齐）：
//! - **Bearer**：`/api/cmd/*` 与 `/api/auth/status` 接受 `Authorization:
//!   Bearer <主令牌>`（env/Argon2id 校验，非会话表）；无效 Bearer ⇒ 401；
//!   Cookie 路径零变化（既有 api_test.rs 全量回归）；
//!   [T5 修订] Bearer 失败面限流：仅计失败、5 次/分钟/IP 滑动窗口、
//!   第 6 次 429、成功不计数不限流、计数器独立于 auth 路由限流器；
//! - **票据**：`POST /api/events/ticket`（Bearer）签发一次性 30s 票据；
//!   `GET /api/events?ticket=` 校验后建流；单次使用（二次 401）；
//! - **CORS**：`tauri://localhost` / `http://tauri.localhost` 白名单——
//!   preflight 放行头、错误响应（401）携带 ACAO（网络错误≠401 分流的
//!   必要条件）、非白名单跨源仍 403、无 Origin 请求零 CORS 头。

mod common;

use common::{Client, InProcessServer};
use egosync_server::cors::TAURI_ORIGIN_ALLOWLIST;
use egosync_server::sse::SSE_TICKET_TTL;
use egosync_server::bootstrap::build_test_state;
use serde_json::{json, Value};
use std::time::Duration;

/// 临时数据目录（同 api_test.rs 范式）。
fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let path = dir.keep();
    path.join(tag)
}

// ── Bearer 叠加认证 ───────────────────────────────────────────────────────

#[tokio::test]
async fn bearer_token_authenticates_business_endpoints() {
    let state = build_test_state(temp_data_dir("bearer-cmd"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 有效 Bearer ⇒ 业务命令 200（无 Cookie——桌面远程客户端形态）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("primary-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "有效 Bearer 必须放行业务命令");

    // 无效 Bearer ⇒ 401 统一形状（不回落 Cookie——显式出示凭据结果确定）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("wrong-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 401, "无效 Bearer 必须 401");
    let err: Value = res.json().await.expect("401 body");
    assert_eq!(err["error"], "unauthorized", "401 统一形状");

    // 空白 scheme 非 Bearer ⇒ 无 Authorization 语义 ⇒ 401（无 Cookie 回落）
    let res = client
        .post_json(&server.url("/api/cmd/role_list"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn bearer_token_authenticates_sse_stream_endpoint() {
    let state = build_test_state(temp_data_dir("bearer-sse"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // GET /api/events 携 Bearer ⇒ 200 建流（全部 /api/* 接受 Bearer）
    let res = client
        .get_bearer(&server.url("/api/events"), Some("primary-token"), None)
        .await;
    assert_eq!(res.status(), 200, "Bearer 通道必须可达 SSE 流");
}

#[tokio::test]
async fn bearer_on_auth_status_reports_authenticated() {
    let state = build_test_state(temp_data_dir("bearer-status"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 有效 Bearer ⇒ authenticated:true（桌面远程连接测试/引导探测通道）
    let res = client
        .get_bearer(&server.url("/api/auth/status"), Some("primary-token"), None)
        .await;
    assert_eq!(res.status(), 200);
    let status: Value = res.json().await.expect("status body");
    assert_eq!(status["authenticated"], true, "有效 Bearer ⇒ authenticated");
    assert_eq!(status["setupRequired"], false);

    // 无效 Bearer ⇒ authenticated:false（200 形状不变——错误文案归前端分流）
    let res = client
        .get_bearer(&server.url("/api/auth/status"), Some("wrong-token"), None)
        .await;
    assert_eq!(res.status(), 200);
    let status: Value = res.json().await.expect("status body");
    assert_eq!(status["authenticated"], false, "无效 Bearer ⇒ 未认证态");

    // 无 Bearer ⇒ Cookie 路径（无 Cookie ⇒ authenticated:false）零变化
    let res = client.get(&server.url("/api/auth/status"), None, None).await;
    assert_eq!(res.status(), 200);
    let status: Value = res.json().await.expect("status body");
    assert_eq!(status["authenticated"], false);
}

#[tokio::test]
async fn bearer_validates_db_mode_argon2id_primary_token() {
    // 库态实例（无 env）：Bearer 走 Argon2id 主令牌校验（非会话表）
    let state = build_test_state(temp_data_dir("bearer-db"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 首访 setup 写入库态主令牌
    let setup = json!({"token": "db-primary-token"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&setup), None, None)
        .await;
    assert_eq!(res.status(), 200);

    // 有效 Bearer（库态 Argon2id 校验通过）⇒ 业务命令放行
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("db-primary-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "库态主令牌 Bearer 必须放行");

    // 无效 Bearer ⇒ 401
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("not-the-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 401);

    // Bearer ≠ 会话表：login 换发的会话 Cookie 不赋予 Bearer 通道语义
    //（对照——Cookie 自身照常可用，api_test.rs 已覆盖）
}

// ── SSE 一次性票据 ────────────────────────────────────────────────────────

#[tokio::test]
async fn ticket_flow_bearer_issue_then_sse_consume() {
    let state = build_test_state(temp_data_dir("ticket-flow"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 未认证签发票据 ⇒ 401（挂 require_auth 组）
    let res = client
        .post_json_bearer(&server.url("/api/events/ticket"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 401, "票据签发必须认证");

    // Bearer 签发 ⇒ 200 {ticket}
    let res = client
        .post_json_bearer(
            &server.url("/api/events/ticket"),
            Some(&json!({})),
            Some("primary-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("ticket body");
    let ticket = body["ticket"].as_str().expect("ticket 字段").to_string();
    assert!(!ticket.is_empty(), "票据值非空");

    // 凭票据建流（无 Bearer/Cookie）⇒ 200 SSE
    let res = client
        .get(&server.url(&format!("/api/events?ticket={}", ticket)), None, None)
        .await;
    assert_eq!(res.status(), 200, "一次性票据必须可建 SSE 流");
    assert_eq!(
        res.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default(),
        "text/event-stream",
        "SSE 流内容类型"
    );
}

#[tokio::test]
async fn ticket_is_single_use_second_consume_is_401() {
    let state = build_test_state(temp_data_dir("ticket-single"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    let res = client
        .post_json_bearer(
            &server.url("/api/events/ticket"),
            Some(&json!({})),
            Some("primary-token"),
            None,
        )
        .await;
    let body: Value = res.json().await.expect("ticket body");
    let ticket = body["ticket"].as_str().expect("ticket 字段").to_string();

    // 首次消费 ⇒ 200
    let res = client
        .get(&server.url(&format!("/api/events?ticket={}", ticket)), None, None)
        .await;
    assert_eq!(res.status(), 200);

    // 二次消费 ⇒ 401（单次使用——重放票据不得建流）
    let res = client
        .get(&server.url(&format!("/api/events?ticket={}", ticket)), None, None)
        .await;
    assert_eq!(res.status(), 401, "票据单次使用：二次必须 401");

    // 伪造票据 ⇒ 401
    let res = client
        .get(&server.url("/api/events?ticket=sse_forged-not-a-real-ticket"), None, None)
        .await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn ticket_ttl_is_30_seconds_frozen() {
    // TTL 冻结款钉配置值（运行期 30s 长等待不可测——值断言 + store 级
    // 过期语义单测见 sse.rs tests）
    assert_eq!(SSE_TICKET_TTL, Duration::from_secs(30));
}

#[tokio::test]
async fn cookie_sse_path_unchanged_without_ticket_param() {
    // Cookie 路径零变化：无 ticket 参数的 /api/events 仍走会话 Cookie 校验
    let state = build_test_state(temp_data_dir("ticket-cookie"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 无 Cookie 无票据 ⇒ 401（既有语义）
    let res = client.get(&server.url("/api/events"), None, None).await;
    assert_eq!(res.status(), 401);

    // login 换 Cookie ⇒ 200 建流（EventSource 浏览器既有路径）
    let session = common::login(&client, &server.url(""), "primary-token")
        .await
        .expect("登录");
    let res = client.get(&server.url("/api/events"), Some(&session), None).await;
    assert_eq!(res.status(), 200, "Cookie 通道必须照常建流");
}

// ── [T5 修订] Bearer 失败面限流（仅计失败；5 次/分钟/IP；超限 429） ──

#[tokio::test]
async fn bearer_failures_rate_limited_after_five_per_minute() {
    let state = build_test_state(temp_data_dir("bearer-rate"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 前 5 次错误 Bearer ⇒ 401（逐次计入失败窗口）
    for i in 0..5 {
        let res = client
            .post_json_bearer(
                &server.url("/api/cmd/role_list"),
                Some(&json!({})),
                Some("wrong-token"),
                None,
            )
            .await;
        assert_eq!(res.status(), 401, "第 {} 次错误 Bearer 应 401", i + 1);
    }
    // 第 6 次 ⇒ 429 统一形状（与登录面限流同款）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("wrong-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 429, "5 次失败/分钟后超限");
    let body: Value = res.json().await.expect("429 body");
    assert_eq!(body["error"], "rate limit exceeded", "429 统一形状");

    // /api/events/ticket 同计数器（任务面「均覆盖」钉）：超限后错误
    // Bearer 同 429，不因换端点旁路
    let res = client
        .post_json_bearer(
            &server.url("/api/events/ticket"),
            Some(&json!({})),
            Some("wrong-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 429, "票据签发面同计数器超限");

    // SSE 流端点（require_auth_with_sse_ticket 的 Bearer 分支）同计数器
    let res = client
        .get_bearer(&server.url("/api/events"), Some("wrong-token"), None)
        .await;
    assert_eq!(res.status(), 429, "SSE 流端点 Bearer 失败同计数器超限");

    // 计数器独立性：auth 路由限流器未被 Bearer 失败消耗（/api/auth/status
    // 首次访问仍 200——两计数器互不侵占预算）
    let res = client.get(&server.url("/api/auth/status"), None, None).await;
    assert_eq!(res.status(), 200, "Bearer 失败计数独立于 auth 路由限流器");
}

#[tokio::test]
async fn bearer_successes_do_not_count_or_limit() {
    let state = build_test_state(temp_data_dir("bearer-ok"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 10 次成功 Bearer 访问（远超限流阈值）——全部放行（成功不计数、
    // 不限流：令牌正确者不受任何失败计数影响）
    for i in 0..10 {
        let res = client
            .post_json_bearer(
                &server.url("/api/cmd/role_list"),
                Some(&json!({})),
                Some("primary-token"),
                None,
            )
            .await;
        assert_eq!(res.status(), 200, "第 {} 次成功访问应放行", i + 1);
    }
    // 成功访问不消耗失败预算：首次失败仍 401（非 429）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("wrong-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 401, "成功访问不计数——失败预算未被消耗");

    // 成功访问在失败后仍放行（「仅计失败」的另一面：失败窗口不阻断
    // 有效令牌——校验先行，仅失败路径计数）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("primary-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "有效 Bearer 在失败计数存在时仍放行");
}

// ── CORS 白名单（桌面 webview 跨源） ──────────────────────────────────────

#[tokio::test]
async fn cors_preflight_for_tauri_origins_is_allowed() {
    let state = build_test_state(temp_data_dir("cors-preflight"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    for origin in TAURI_ORIGIN_ALLOWLIST {
        // preflight：204 + ACAO 回显 + 方法/头放行（Authorization 触发预检
        // 的必要条件）
        let res = client
            .options(&server.url("/api/cmd/role_list"), Some(origin))
            .await;
        assert_eq!(res.status(), 204, "preflight {} 应 204", origin);
        assert_eq!(
            res.headers()
                .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|v| v.to_str().ok()),
            Some(*origin),
            "ACAO 必须回显白名单 Origin {}",
            origin
        );
        let allow_headers = res
            .headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_HEADERS)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(
            allow_headers.to_ascii_lowercase().contains("authorization"),
            "放行头必须含 Authorization: {}",
            allow_headers
        );
        assert!(
            allow_headers.to_ascii_lowercase().contains("content-type"),
            "放行头必须含 Content-Type: {}",
            allow_headers
        );
    }

    // 非白名单跨源 preflight ⇒ 403（拒绝语义不被 CORS 层旁路）
    let res = client
        .options(&server.url("/api/cmd/role_list"), Some("http://evil.example"))
        .await;
    assert_eq!(res.status(), 403, "非白名单 preflight 必须拒绝");
}

#[tokio::test]
async fn cors_headers_on_responses_for_tauri_origins() {
    let state = build_test_state(temp_data_dir("cors-resp"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let origin = "http://tauri.localhost";

    // 实际请求（认证失败 401）：响应必须携带 ACAO——CORS 下无 ACAO 的
    // 错误响应对 fetch 呈网络错误形态，桌面无法区分 401 与断网
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("wrong-token"),
            Some(origin),
        )
        .await;
    assert_eq!(res.status(), 401);
    assert_eq!(
        res.headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some(origin),
        "401 响应必须携带 ACAO（网络错误≠401 分流的必要条件）"
    );

    // 成功响应同样携带 ACAO + Vary: Origin + 错误判别头 expose
    //（[评审轮2 U2] 非安全列表响应头对跨源 JS 默认不可读——不 expose
    // 则桌面 http.ts 读不到 x-egosync-app-error，业务错误整体呈成功形态）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("primary-token"),
            Some(origin),
        )
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some(origin)
    );
    assert!(res.headers().contains_key(reqwest::header::VARY));
    assert_eq!(
        res.headers()
            .get(reqwest::header::ACCESS_CONTROL_EXPOSE_HEADERS)
            .and_then(|v| v.to_str().ok()),
        Some("x-egosync-app-error"),
        "白名单源响应必须 expose 业务错误判别头（跨源可读——桌面错误分流的必要条件）"
    );
    // 401 响应同样 expose（错误形态优先——桌面须能区分 401 与断网）
    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("wrong-token"),
            Some(origin),
        )
        .await;
    assert_eq!(res.status(), 401);
    assert_eq!(
        res.headers()
            .get(reqwest::header::ACCESS_CONTROL_EXPOSE_HEADERS)
            .and_then(|v| v.to_str().ok()),
        Some("x-egosync-app-error"),
        "错误响应同样 expose 判别头"
    );
}

#[tokio::test]
async fn cors_no_origin_responses_have_zero_cors_headers() {
    // 浏览器/服务间路径零变化：无 Origin 请求的响应不得携带 CORS 头
    let state = build_test_state(temp_data_dir("cors-none"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(res.status(), 200);
    assert!(
        !res.headers().contains_key(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN),
        "无 Origin 请求不得携带 ACAO（浏览器路径零变化）"
    );

    let res = client
        .post_json_bearer(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("primary-token"),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);
    assert!(
        !res.headers().contains_key(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN),
        "无 Origin 请求不得携带 ACAO"
    );
    assert!(
        !res.headers()
            .contains_key(reqwest::header::ACCESS_CONTROL_EXPOSE_HEADERS),
        "无 Origin 请求不得携带 expose 头（[评审轮2 U2] 浏览器路径零变化）"
    );
}

#[tokio::test]
async fn cors_sse_stream_response_carries_allow_origin() {
    // SSE 响应覆盖：票据建流的 EventSource 跨源可读（ACAO 必达）
    let state = build_test_state(temp_data_dir("cors-sse"), Some("primary-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let origin = "tauri://localhost";

    let res = client
        .post_json_bearer(
            &server.url("/api/events/ticket"),
            Some(&json!({})),
            Some("primary-token"),
            Some(origin),
        )
        .await;
    assert_eq!(res.status(), 200, "票据签发（跨源 Bearer）");
    let body: Value = res.json().await.expect("ticket body");
    let ticket = body["ticket"].as_str().expect("ticket 字段");

    let res = client
        .get(&server.url(&format!("/api/events?ticket={}", ticket)), None, Some(origin))
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get(reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some(origin),
        "SSE 流响应必须携带 ACAO（EventSource 跨源可读）"
    );
}
