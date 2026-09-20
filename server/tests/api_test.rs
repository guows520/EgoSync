//! Story 15.4 server 集成测试：I/O 矩阵全覆盖（InProcessServer 先例）。
//!
//! 覆盖：healthz 两级 / env 态与库态 / 优先级切换 + Cookie 跨重启 /
//! 401 形状 / desktop-only 与未知命令 404 / 错误白名单 13 variant /
//! camelCase 参数 / 限流 429 / 跨源 403 / CSP 全响应 / body 50MB 超限 413 /
//! SSE 事件与滞后断开 / 密钥零泄漏。
//!（SecretStore 文件优先/env 兜底等四断言在 tests/secret_store_test.rs。）


mod common;

use common::{login, Client, InProcessServer};
use egosync_engine::error::AppError;
use egosync_server::auth::SESSION_COOKIE;
use egosync_server::sse::SSE_KEEPALIVE_SECS;
use egosync_server::bootstrap::{build_app_state, build_test_state};
use egosync_server::{security::CSP_POLICY, MAX_BODY_BYTES};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;

/// 临时数据目录（测试期间存活；TempDir::keep 保持目录不被清理）。
fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let path = dir.keep();
    path.join(tag)
}

// ── 存活/深度探针 ──

#[tokio::test]
async fn healthz_alive_without_auth() {
    let state = build_test_state(temp_data_dir("healthz"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("healthz body");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn healthz_deep_reports_db_and_opencode_flags() {
    let state = build_test_state(temp_data_dir("healthz-deep"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 测试态 sidecar 未启动：opencode:false ⇒ 503（5xx 家族——巡检语义）
    let res = client
        .get(&server.url("/healthz?deep=1"), None, None)
        .await;
    assert_eq!(res.status(), 503);
    let body: Value = res.json().await.expect("healthz deep body");
    assert_eq!(body["db"], true, "双池 SELECT 1 应通过");
    assert_eq!(body["opencode"], false, "sidecar 未启动应如实降级");
}

// ── env 态引导（优先级冻结）──

#[tokio::test]
async fn env_mode_setup_is_404_and_login_compares_env_only() {
    let state = build_test_state(temp_data_dir("env-mode"), Some("env-secret-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // env 态：setup 路由不挂载 ⇒ 404
    let body = json!({"token": "anything"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&body), None, None)
        .await;
    assert_eq!(res.status(), 404, "env 态 setup 物理不挂载");

    // auth/status：setupRequired=false（env 已就位）
    let res = client.get(&server.url("/api/auth/status"), None, None).await;
    assert_eq!(res.status(), 200);
    let status: Value = res.json().await.expect("auth status body");
    assert_eq!(status["setupRequired"], false);

    // 错误令牌 ⇒ 统一 401
    let wrong = json!({"token": "wrong"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&wrong), None, None)
        .await;
    assert_eq!(res.status(), 401);
    let err: Value = res.json().await.expect("401 body");
    assert_eq!(err["error"], "unauthorized", "401 统一形状不泄露存在性");

    // 正确令牌 ⇒ Set-Cookie（httpOnly SameSite=Strict）
    let right = json!({"token": "env-secret-token"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&right), None, None)
        .await;
    assert_eq!(res.status(), 200);
    let set_cookie = res
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("登录成功必须下发会话 Cookie")
        .to_str()
        .expect("Set-Cookie 头解码");
    assert!(set_cookie.contains("HttpOnly"), "httpOnly 必须下发: {}", set_cookie);
    assert!(set_cookie.contains("SameSite=Strict"), "SameSite=Strict 必须下发: {}", set_cookie);
    assert!(set_cookie.starts_with(SESSION_COOKIE), "Cookie 名必须为 {}: {}", SESSION_COOKIE, set_cookie);
    // 16.1（boss 2026-09-20 裁决）：30 天持久会话——Max-Age=2592000 必须下发
    assert!(
        set_cookie.contains("Max-Age=2592000"),
        "30 天持久 Cookie（Max-Age=2592000）必须下发: {}",
        set_cookie
    );

    // 畸形 body ⇒ 同样 401（不泄露）
    let res = client
        .post_json::<serde_json::Value>(&server.url("/api/auth/login"), None, None, None)
        .await;
    assert_eq!(res.status(), 401);
}

// ── 库态首访 setup ──

#[tokio::test]
async fn db_mode_first_visit_setup_then_unavailable() {
    let data_dir = temp_data_dir("db-mode");
    let state = build_test_state(data_dir.clone(), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 首访：auth/status 显示 setupRequired
    let res = client.get(&server.url("/api/auth/status"), None, None).await;
    let status: Value = res.json().await.expect("auth status body");
    assert_eq!(status["setupRequired"], true);
    assert_eq!(status["authenticated"], false);

    // 空令牌 ⇒ 401（错误白名单冻结口径：非 200 仅 401/429/404/5xx，
    // 认证失败一律 401 统一形状）
    let empty = json!({"token": ""});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&empty), None, None)
        .await;
    assert_eq!(res.status(), 401);

    // 首访 setup 成功
    let token = json!({"token": "my-strong-token"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&token), None, None)
        .await;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("setup body");
    assert_eq!(body["ok"], true);

    // setup 随即不可用（不依赖重启）
    let again = json!({"token": "another"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&again), None, None)
        .await;
    assert_eq!(res.status(), 404, "已初始化后 setup ⇒ 404");

    // setup 后 login 用库内哈希通过（错误令牌 ⇒ 401 已由 env 态测试覆盖；
    // 本测试的 auth 面请求预算恰 5 次——限流窗口内不再追加）
    let ok = json!({"token": "my-strong-token"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&ok), None, None)
        .await;
    assert_eq!(res.status(), 200);
    assert!(res.headers().contains_key(reqwest::header::SET_COOKIE));
}

// ── 优先级切换 + Cookie 跨重启（auth_sessions 持久化）──

#[tokio::test]
async fn cookie_survives_restart_and_credential_mode_switch() {
    let data_dir = temp_data_dir("switch");

    // 库态：setup + login → 捕获 Cookie
    let state = build_test_state(data_dir.clone(), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let setup = json!({"token": "original-token"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&setup), None, None)
        .await;
    assert_eq!(res.status(), 200);
    let session = login(&client, &server.url(""), "original-token")
        .await
        .expect("库态登录成功");
    drop(server);

    // 同数据目录重启为 env 态：旧 Cookie 仍有效（会话表持久化）
    let state = build_test_state(data_dir.clone(), Some("new-env-token".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;

    // 旧 Cookie 上的业务请求可通过（authenticated）
    let res = client
        .post_json(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "已发 Cookie 不随切换失效");

    // env 态 login：仅比对 env（旧库令牌 ⇒ 401；env 令牌 ⇒ 200）
    let old = json!({"token": "original-token"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&old), None, None)
        .await;
    assert_eq!(res.status(), 401, "env 态忽略库内哈希（无 fail-open）");
    let env = json!({"token": "new-env-token"});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&env), None, None)
        .await;
    assert_eq!(res.status(), 200);
}

// ── 未认证业务面 ──

#[tokio::test]
async fn unauthenticated_business_endpoints_return_401() {
    let state = build_test_state(temp_data_dir("unauth"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // /api/cmd/* 无 Cookie ⇒ 401
    let res = client
        .post_json(&server.url("/api/cmd/role_list"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 401);

    // 伪造 Cookie ⇒ 401
    let res = client
        .post_json(
            &server.url("/api/cmd/role_list"),
            Some(&json!({})),
            Some("egosync_session=00000000-0000-4000-8000-000000000000"),
            None,
        )
        .await;
    assert_eq!(res.status(), 401);

    // SSE 无 Cookie ⇒ 401（EventSource 平台约束下的认证豁免面仅 healthz）
    let res = client.get(&server.url("/api/events"), None, None).await;
    assert_eq!(res.status(), 401);
}

// ── desktop-only / 未知命令物理不路由 ──

#[tokio::test]
async fn desktop_only_and_unknown_commands_are_404() {
    let state = build_test_state(temp_data_dir("routing"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    for command in [
        "pairing_generate_qr",
        "pairing_confirm",
        "paired_device_list",
        "paired_device_remove",
        "companion_get_status",
        "companion_get_relay_addr",
        "companion_set_relay_addr",
        "chat_pick_working_directory",
        "skill_pick_custom_directory",
        "pick_import_file",
        "data_export",
        "data_import",
        // 评审回环裁决 A：密钥原文命令划归 desktop-only，server 物理不路由
        "secret_store_save",
        "secret_store_load",
        "secret_store_delete",
        // 2 条 perf-test 门控命令同不入 server 路由面
        "app_emit_test_stream",
        "app_seed_perf_data",
        // 未知命令
        "no_such_cmd",
    ] {
        let res = client
            .post_json(
                &server.url(&format!("/api/cmd/{}", command)),
                Some(&json!({})),
                Some(&session),
                None,
            )
            .await;
        assert_eq!(res.status(), 404, "{} 应物理不路由", command);
    }
}

// ── 错误白名单（架构 ②：13 variant 一律 200 + 原样单键 map）──

/// 从 error.rs 源码机械统计 AppError variant 声明数（二轮评审修复 #10：
/// 手写清单与源码脱钩——新增 variant 不自动进覆盖）。
///
/// 扫描口径：`pub enum AppError {` 与首个列 0 `}` 之间的块内，形如
/// `Identifier(`（大驼峰后紧跟括号）的行即 variant 声明——`#[error]`
/// 属性行与 `///` 文档行不匹配。**格式变化导致定位失败 ⇒ panic**
/// （宁可响亮失败，不静默跳过）。
fn count_app_error_variants_in_source() -> usize {
    let src = include_str!("../../crates/egosync-engine/src/error.rs");
    let start = src
        .find("pub enum AppError {")
        .expect("error.rs 未找到 'pub enum AppError {'（格式变化？——panic 而非静默跳过）");
    let rest = &src[start + "pub enum AppError {".len()..];
    let end = rest
        .find("\n}")
        .expect("error.rs AppError 枚举块未闭合（格式变化？）");
    rest[..end]
        .lines()
        .filter(|line| {
            let t = line.trim();
            let ident: String = t
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            !ident.is_empty()
                && ident.chars().next().unwrap().is_ascii_uppercase()
                && t[ident.len()..].starts_with('(')
        })
        .count()
}

#[tokio::test]
async fn all_app_error_variants_map_to_200_single_key_json() {
    // 错误白名单的对等覆盖：对 13 个现役 variant 逐一构造实例，经与
    // cmd_handler 相同的「200 + Json(AppError)」序列化通道断言形状。
    // （Variant 的深层业务触发路径不可组合穷举——序列化通道等价即
    // 白名单口径等价；真实命令错误另见下条测试。）
    let variants: Vec<(&str, AppError)> = vec![
        ("NotFound", AppError::NotFound("x".into())),
        ("LlmError", AppError::LlmError("x".into())),
        ("DbError", AppError::DbError("x".into())),
        ("ValidationError", AppError::ValidationError("x".into())),
        ("KeyringError", AppError::KeyringError("x".into())),
        ("SidecarError", AppError::SidecarError("x".into())),
        (
            "RuntimeRefreshError",
            AppError::RuntimeRefreshError("x".into()),
        ),
        ("SkillNotFound", AppError::SkillNotFound("x".into())),
        (
            "SkillNotAddedToScope",
            AppError::SkillNotAddedToScope("x".into()),
        ),
        ("SkillDisabled", AppError::SkillDisabled("x".into())),
        ("PairingError", AppError::PairingError("x".into())),
        ("ConnectionError", AppError::ConnectionError("x".into())),
        ("ProtocolError", AppError::ProtocolError("x".into())),
    ];
    // 二轮评审修复 #10：手写清单长度必须等于源码 variant 声明数——
    // error.rs 新增 variant 而清单未跟 ⇒ 此处先红（可读第一现场）
    let source_count = count_app_error_variants_in_source();
    assert_eq!(
        variants.len(),
        source_count,
        "手写 variant 清单({})与 error.rs 源码声明数({})不一致——新增 variant 必须同步本清单",
        variants.len(),
        source_count
    );

    for (name, err) in variants {
        let response = egosync_server::auth::app_error_to_response(err);
        let (parts, body) = response.into_parts();
        assert_eq!(parts.status, 200, "{} 必须映射 200", name);
        // Story 15.5：AppError 响应必须携带判别头（HTTP 通道错误信号——
        // 前端 HttpTransport 据此把 200 body 解析值转为 reject）
        assert_eq!(
            parts.headers.get(egosync_server::routes::APP_ERROR_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some(egosync_server::routes::APP_ERROR_HEADER_VALUE),
            "{} 必须携带 X-Egosync-App-Error 判别头",
            name
        );
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("body 读取");
        let value: Value = serde_json::from_slice(&bytes).expect("单键 map JSON");
        let map = value.as_object().expect("错误响应为 JSON 对象");
        assert_eq!(map.len(), 1, "{} 必须单键", name);
        assert!(map.contains_key(name), "键名必须为 variant 名 {}", name);
        assert_eq!(map[name], "x", "值为原样错误文案");
    }
}

#[tokio::test]
async fn real_command_errors_return_200_with_variant_shape() {
    let state = build_test_state(temp_data_dir("cmd-error"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // NotFound 变体：不存在的记忆源消息
    let res = client
        .post_json(
            &server.url("/api/cmd/memory_get_source_messages"),
            Some(&json!({"memoryId": "no-such-memory"})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "业务错误一律 200");
    // Story 15.5：真实错误路径必须携带判别头（生产 cmd_handler Err 分支）
    assert_eq!(
        res.headers()
            .get(egosync_server::routes::APP_ERROR_HEADER)
            .and_then(|v| v.to_str().ok()),
        Some(egosync_server::routes::APP_ERROR_HEADER_VALUE),
        "AppError 响应必须携带判别头"
    );
    let body: Value = res.json().await.expect("错误 body");
    assert!(
        body.get("NotFound").is_some() || body.get("DbError").is_some(),
        "单键 map 形状: {}",
        body
    );

    // ValidationError 变体：必填参数缺失（类型不符）
    let res = client
        .post_json(
            &server.url("/api/cmd/task_create"),
            Some(&json!({"wrongField": true})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get(egosync_server::routes::APP_ERROR_HEADER)
            .and_then(|v| v.to_str().ok()),
        Some(egosync_server::routes::APP_ERROR_HEADER_VALUE),
        "参数反序列化错误必须携带判别头"
    );
    let body: Value = res.json().await.expect("错误 body");
    assert!(
        body.get("ValidationError").is_some(),
        "参数反序列化失败必须 ValidationError 单键: {}",
        body
    );

    // 非 JSON body ⇒ 200 + ValidationError + 判别头（错误白名单口径）
    let res = client
        .post_bytes(&server.url("/api/cmd/role_list"), b"not json".to_vec(), Some(&session))
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get(egosync_server::routes::APP_ERROR_HEADER)
            .and_then(|v| v.to_str().ok()),
        Some(egosync_server::routes::APP_ERROR_HEADER_VALUE),
        "非 JSON body 错误必须携带判别头"
    );
    let body: Value = res.json().await.expect("错误 body");
    assert!(
        body.get("ValidationError").is_some(),
        "非 JSON body 必须 ValidationError: {}",
        body
    );

    // 成功路径对照：零参命令 200 且不带判别头（判别信号的正反两面——
    // 前端 HttpTransport 只对带头响应走 reject 通道）
    let res = client
        .post_json(&server.url("/api/cmd/role_list"), Some(&json!({})), Some(&session), None)
        .await;
    assert_eq!(res.status(), 200);
    assert!(
        !res.headers()
            .contains_key(egosync_server::routes::APP_ERROR_HEADER),
        "成功响应不得携带判别头（免误判）"
    );
}

// ── camelCase 参数（与 invoke 同构）──

#[tokio::test]
async fn camel_case_params_unpack_like_invoke() {
    let state = build_test_state(temp_data_dir("camel"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // app_set_setting {key, value} → 写入后 app_get_setting 读回
    let res = client
        .post_json(
            &server.url("/api/cmd/app_set_setting"),
            Some(&json!({"key": "testKey", "value": "testValue"})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);

    let res = client
        .post_json(
            &server.url("/api/cmd/app_get_setting"),
            Some(&json!({"key": "testKey"})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("读回值");
    assert_eq!(body, "testValue");

    // Option 缺省参数：task_list_all 不带可选过滤 → 200
    let res = client
        .post_json(
            &server.url("/api/cmd/task_list_all"),
            Some(&json!({})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);

    // 空 body（无 JSON）零参命令 → 200
    let res = client
        .post_bytes(&server.url("/api/cmd/role_list"), Vec::new(), Some(&session))
        .await;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.expect("role_list body");
    assert_eq!(body, json!([]));
}

// ── 限流（5/min/IP）──

#[tokio::test]
async fn auth_surface_rate_limited_after_five_per_minute() {
    let state = build_test_state(temp_data_dir("rate"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 前 5 次请求（含 auth/status 与 login 混合计数）放行
    for i in 0..5 {
        let res = client.get(&server.url("/api/auth/status"), None, None).await;
        assert_eq!(res.status(), 200, "第 {} 次应放行", i + 1);
    }
    // 第 6 次 ⇒ 429
    let res = client.get(&server.url("/api/auth/status"), None, None).await;
    assert_eq!(res.status(), 429, "5 次/分钟后超限");
}

// ── 跨源拒绝 + CSP 全响应 ──

#[tokio::test]
async fn cross_origin_rejected_and_csp_on_all_responses() {
    let state = build_test_state(temp_data_dir("origin"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 跨源 Origin ⇒ 403（无 CORS 放行头）
    let res = client
        .get(&server.url("/healthz"), None, Some("http://evil.example"))
        .await;
    assert_eq!(res.status(), 403);
    assert!(
        !res.headers().contains_key("access-control-allow-origin"),
        "不得下发 CORS 放行头"
    );
    assert_eq!(
        res.headers().get(reqwest::header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok()),
        Some(CSP_POLICY),
        "403 响应仍下发 CSP"
    );

    // 同源 Origin（与 Host 一致）放行
    let same_origin = format!("http://{}", server.addr);
    let res = client
        .get(&server.url("/healthz"), None, Some(&same_origin))
        .await;
    assert_eq!(res.status(), 200, "同源放行");

    // CSP 全响应下发（healthz / 401 / 404）
    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(
        res.headers().get(reqwest::header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok()),
        Some(CSP_POLICY)
    );
    let res = client
        .post_json(&server.url("/api/cmd/role_list"), Some(&json!({})), None, None)
        .await;
    assert_eq!(res.status(), 401);
    assert_eq!(
        res.headers().get(reqwest::header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok()),
        Some(CSP_POLICY)
    );
    let session = login(&client, &server.url(""), "t").await.expect("登录");
    let res = client
        .post_json(
            &server.url("/api/cmd/no_such"),
            Some(&json!({})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 404);
    assert_eq!(
        res.headers().get(reqwest::header::CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok()),
        Some(CSP_POLICY)
    );
}

// ── body 上限 50MB（传输层 413）──

#[tokio::test]
async fn oversized_body_rejected_with_413() {
    let state = build_test_state(temp_data_dir("body-limit"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // 上限内（50MB 整）放行到解析层（ValidationError 或业务 200——都非 413）
    let ok_size = MAX_BODY_BYTES;
    let res = client
        .post_bytes(
            &server.url("/api/cmd/role_list"),
            vec![b'x'; ok_size],
            Some(&session),
        )
        .await;
    assert_ne!(res.status(), 413, "恰在上限的 body 不应 413（后续按内容解析）");

    // 超限（50MB+1）⇒ 413（传输层原语）
    let res = client
        .post_bytes(
            &server.url("/api/cmd/role_list"),
            vec![b'x'; MAX_BODY_BYTES + 1],
            Some(&session),
        )
        .await;
    assert_eq!(res.status(), 413, "body >50MB 必须 413");
}

// ── SSE：事件到达 + 滞后断开 + KeepAlive 配置 ──

#[tokio::test]
async fn sse_delivers_events_with_name_and_payload() {
    let state = build_test_state(temp_data_dir("sse"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state.clone()).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // 先连接（订阅在 handler 开头生效——响应头到达即订阅完成；
    // broadcast 无重放，事件必须在订阅后发射）
    let res = client.get(&server.url("/api/events"), Some(&session), None).await;
    assert_eq!(res.status(), 200);

    state
        .ctx
        .bus
        .emit("test:ping", json!({"hello": "world"}))
        .expect("SSE 事件发射");

    // 读取首个事件帧：event: 名称 / data: payload JSON（与 emit 同构）
    let mut stream = res.bytes_stream();
    let mut collected = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut buf = Vec::new();
    while std::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                buf.extend_from_slice(&chunk);
                if buf.windows(2).any(|w| w == b"\n\n") {
                    // 帧分隔到达
                    collected = buf.clone();
                    break;
                }
            }
            Ok(Some(Err(_))) | Ok(None) => break,
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&collected);
    assert!(
        text.contains("event: test:ping"),
        "SSE 帧必须携带事件名: {:?}",
        text
    );
    assert!(
        text.contains("\"hello\":\"world\""),
        "SSE data 必须为 payload JSON: {:?}",
        text
    );
}

#[tokio::test]
async fn sse_lagged_client_is_disconnected() {
    // 滞后断开语义直接对 Stream 断言：经 TCP 复现不可稳定（内核 socket
    // 缓冲吸收慢消费），而 handler 用的正是本函数产物——语义等价。
    let (tx, rx) = tokio::sync::broadcast::channel(
        egosync_server::sse::BROADCAST_CAPACITY,
    );
    let mut stream = Box::pin(egosync_server::sse::sse_event_stream(rx));

    // 未消费时灌超容量事件（容量 256，发 300）：首次 poll 即 Lagged
    for i in 0..300 {
        tx.send(egosync_server::sse::SseEvent {
            event: "test:flood".to_string(),
            payload: json!({"n": i}),
        })
        .expect("广播发送");
    }

    // 滞后 ⇒ 流终止（连接断开的 Stream 侧语义），不再产出后续事件
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut ended = false;
    while std::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
            Ok(None) => {
                ended = true;
                break;
            }
            Ok(Some(_)) => continue, // 可能先吐若干缓冲帧；滞后帧后终止
            Err(_) => continue,
        }
    }
    assert!(ended, "滞后客户端的流必须终止（断开语义）");

    // 对照组：容量内的正常消费不终止
    let (tx2, rx2) = tokio::sync::broadcast::channel(
        egosync_server::sse::BROADCAST_CAPACITY,
    );
    let mut stream2 = Box::pin(egosync_server::sse::sse_event_stream(rx2));
    for i in 0..10 {
        tx2.send(egosync_server::sse::SseEvent {
            event: "test:normal".to_string(),
            payload: json!({"n": i}),
        })
        .expect("广播发送");
    }
    let first = tokio::time::timeout(Duration::from_secs(2), stream2.next())
        .await
        .expect("正常帧必须到达")
        .expect("对照组流未关闭")
        .expect("对照组读取成功");
    let _ = first; // 出帧即通过（Event 类型无 is_empty——到达本身即断言）
}

#[test]
fn sse_keepalive_interval_is_30_seconds() {
    // KeepAlive 心跳为运行期长等待，集成测试钉配置值（F11 契约）
    assert_eq!(SSE_KEEPALIVE_SECS, 30);
}

// ── 密钥零泄漏 ──

#[tokio::test]
async fn llm_config_responses_never_leak_raw_keys() {
    let state = build_test_state(temp_data_dir("key-leak"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state.clone()).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // 经服务层直接落库一条带密钥的 LLM 配置（create 命令的 refresh_runtime
    // 会尝试重启 sidecar——测试环境无 opencode 二进制，该失败为既有桌面
    // 行为而非泄漏面；密钥泄漏断言的对象是 HTTP 响应面，服务层植入等价）
    let input = egosync_engine::models::settings::CreateLlmConfigInput {
        name: "测试网关".to_string(),
        provider: "openai_compatible".to_string(),
        base_url: "https://llm.example.com/v1".to_string(),
        model: "gpt-test".to_string(),
        api_key: "sk-super-secret-value-123".to_string(),
        network_location: egosync_engine::models::settings::NetworkLocation::Internal,
    };
    egosync_engine::services::llm_config::create_config(
        &state.ctx.pool,
        state.ctx.secrets.as_ref(),
        input,
    )
    .await
    .expect("服务层配置植入");

    // 列表响应不含原始密钥；含 api_key_ref
    let res = client
        .post_json(
            &server.url("/api/cmd/llm_config_list"),
            Some(&json!({})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);
    let text = res.text().await.expect("列表文本");
    assert!(
        !text.contains("sk-super-secret-value-123"),
        "任何响应不得含原始密钥"
    );
    assert!(text.contains("api_key_ref") || text.contains("apiKeyRef"), "应携带密钥引用");

    // （清理交由临时数据目录——随测试进程弃置）
}

// ── handler panic 兜底（CatchPanicLayer → 500，进程级 5xx 白名单内） ──

#[tokio::test]
async fn panic_in_handler_returns_500_without_details() {
    // 经生产同款 layer 组装（CatchPaniLayer::custom(security::panic_response)
    // ——与 build_router 叠放同一构造），触发 handler panic 断言 500 且
    // 响应体不携带 panic 细节（防信息泄露）。
    use axum::routing::get;
    use tower_http::catch_panic::CatchPanicLayer;

    async fn boom() -> &'static str {
        panic!("boom-detail-must-not-leak");
    }

    let app = axum::Router::new()
        .route("/boom", get(boom))
        .layer(CatchPanicLayer::custom(
            egosync_server::security::panic_response,
        ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind panic test server");
    let addr = listener.local_addr().expect("panic test addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve panic test");
    });

    let res = reqwest::get(format!("http://{}/boom", addr))
        .await
        .expect("请求 panic 路由");
    assert_eq!(res.status(), 500, "handler panic 必须 500（进程级 5xx）");
    let body = res.text().await.expect("panic body");
    assert!(
        !body.contains("boom-detail-must-not-leak"),
        "panic 细节不得泄露: {}",
        body
    );
    handle.abort();
}

// ── opencode 缺失的服务降级（I/O 矩阵：chat 异步流式模型）──

#[tokio::test]
async fn chat_command_degrades_gracefully_without_opencode() {
    let state = build_test_state(temp_data_dir("no-sidecar"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state.clone()).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // 先订阅 SSE（broadcast 无重放），再发送消息
    let sse_res = client
        .get(&server.url("/api/events"), Some(&session), None)
        .await;
    assert_eq!(sse_res.status(), 200);

    // chat_send_message 是异步流式模型（与桌面 invoke 逐字节一致）：
    // 命令同步返回 200 + user 消息；sidecar/LLM 失败发生在 run_stream
    // 任务内，经 llm:stream 兜底 done 帧呈现——服务存活、错误在带内。
    let res = client
        .post_json(
            &server.url("/api/cmd/chat_send_message"),
            Some(&json!({"request": {"content": "hi"}})),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "sidecar 缺失不致命，命令同步面正常");
    let message: Value = res.json().await.expect("user 消息 body");
    assert_eq!(message["role"], "user", "同步返回 user 消息（流式模型契约）");

    // SSE 侧收到 message:saved + llm:stream 兜底 done 帧（降级呈现路径）
    let mut stream = sse_res.bytes_stream();
    let mut buf = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut saw_message_saved = false;
    let mut saw_llm_done = false;
    while std::time::Instant::now() < deadline && !(saw_message_saved && saw_llm_done) {
        match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                buf.extend_from_slice(&chunk);
                let text = String::from_utf8_lossy(&buf);
                if text.contains("event: message:saved") {
                    saw_message_saved = true;
                }
                if text.contains("event: llm:stream") && text.contains("\"done\":true") {
                    saw_llm_done = true;
                }
            }
            Ok(Some(Err(_))) | Ok(None) => break,
            Err(_) => continue,
        }
    }
    assert!(saw_message_saved, "SSE 应收到 message:saved");
    assert!(
        saw_llm_done,
        "SSE 应收到 llm:stream 兜底 done 帧（sidecar 缺失的带内降级）: {}",
        String::from_utf8_lossy(&buf)
    );
}

// ── 生产引导冒烟（评审修复 #13：build_app_state 此前零自动化）──

#[tokio::test]
async fn build_app_state_production_bootstrap_smoke() {
    // 生产同款引导（非 build_test_state）：完整桌面序列——双池、
    // AgentConfig 同步、custom tools 写盘、sidecar（PATH 无 opencode
    // ⇒ 优雅降级）、delegate 监听、EventRouter、调度器。
    let state = build_app_state(
        temp_data_dir("prod-bootstrap"),
        Some("prod-smoke-token".into()),
    )
    .await
    .expect("生产引导必须成功");

    let server = InProcessServer::start(state.clone()).await;
    let client = Client::new();

    // healthz deep：双池 SELECT 1 必过；opencode 位按环境推导（二轮评审
    // 修复 #15：装有 opencode 的机器上「无二进制 ⇒ 503」断言会假红，
    // 健康分支也永不被断言——期望值随 PATH 实际状态取两侧）
    let opencode_available = std::process::Command::new("which")
        .arg("opencode")
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    let res = client.get(&server.url("/healthz?deep=1"), None, None).await;
    let status = res.status();
    let body: Value = res.json().await.expect("deep body");
    assert_eq!(body["db"], true, "生产引导双池必须健康");
    if opencode_available {
        assert_eq!(status, 200, "PATH 有 opencode ⇒ deep 探针健康态");
        assert_eq!(body["opencode"], true, "opencode 可用 ⇒ 健康位");
    } else {
        assert_eq!(status, 503, "PATH 无 opencode ⇒ deep 探针降级态");
        assert_eq!(body["opencode"], false, "无 opencode 二进制 ⇒ 降级位");
    }

    // login：env 令牌换 Cookie（生产引导后认证面全链）
    let session = login(&client, &server.url(""), "prod-smoke-token")
        .await
        .expect("生产引导后 env 令牌登录");
    assert!(session.starts_with("egosync_session="), "Cookie 形状");

    // 收尾：取消 token（生产引导的后台任务收尾通道——调度器/
    // watchdog/delegate 随取消退出，不留悬挂任务）
    state.cancel.cancel();
    // InProcessServer::drop 会 abort serve 任务
}

// ── setup 令牌强度下限（评审修复 #4）──

#[tokio::test]
async fn setup_rejects_short_token_with_401() {
    let state = build_test_state(temp_data_dir("short-token"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 首访态确认
    let res = client.get(&server.url("/api/auth/status"), None, None).await;
    let status: Value = res.json().await.expect("auth status body");
    assert_eq!(status["setupRequired"], true);

    // 短令牌（<8 字符 trim 后）⇒ 401 统一形状（白名单口径：认证失败）
    // ——且不消费 setup_available（后续 setup 仍可用）
    let short = json!({"token": "abc"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&short), None, None)
        .await;
    assert_eq!(res.status(), 401, "短令牌必须 401");
    let err: Value = res.json().await.expect("401 body");
    assert_eq!(err["error"], "unauthorized", "统一形状不泄露细节");

    // 空白令牌（trim 后为空）同路径 401
    let blank = json!({"token": "   "});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&blank), None, None)
        .await;
    assert_eq!(res.status(), 401);

    // 恰 8 字符（边界含）⇒ 放行（≥ 下限）
    let ok = json!({"token": "12345678"});
    let res = client
        .post_json(&server.url("/api/setup"), Some(&ok), None, None)
        .await;
    assert_eq!(res.status(), 200, "恰 8 字符在下限之上应通过");
}

// ── setup 并发防护（评审修复 #3：锁内重检-写入-翻转）──

#[tokio::test]
async fn concurrent_setup_writes_exactly_once_no_overwrite() {
    let state = build_test_state(temp_data_dir("setup-race"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 两枚不同令牌并发双发：恰一个 200（先入锁者），另一个 404（锁内
    // 重检发现已初始化）——后写不得覆盖前令牌
    let a = json!({"token": "token-alpha-111"});
    let b = json!({"token": "token-bravo-222"});
    let url_a = server.url("/api/setup");
    let url_b = server.url("/api/setup");
    let (res_a, res_b) = tokio::join!(
        client.post_json(&url_a, Some(&a), None, None),
        client.post_json(&url_b, Some(&b), None, None),
    );
    let code_a = res_a.status();
    let code_b = res_b.status();
    let mut codes = [code_a, code_b];
    codes.sort();
    assert_eq!(
        codes,
        [reqwest::StatusCode::OK, reqwest::StatusCode::NOT_FOUND],
        "并发双 setup 必须恰一个成功（实得 {} / {}）",
        code_a,
        code_b
    );

    // 胜者令牌可登录；败者令牌不得可用（未被写入——无覆盖）
    let winner = if code_a == reqwest::StatusCode::OK {
        "token-alpha-111"
    } else {
        "token-bravo-222"
    };
    let loser = if winner == "token-alpha-111" {
        "token-bravo-222"
    } else {
        "token-alpha-111"
    };
    let ok = json!({"token": winner});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&ok), None, None)
        .await;
    assert_eq!(res.status(), 200, "胜者令牌必须可登录");
    let bad = json!({"token": loser});
    let res = client
        .post_json(&server.url("/api/auth/login"), Some(&bad), None, None)
        .await;
    assert_eq!(res.status(), 401, "败者令牌必须不可登录（未被覆盖写入）");
}

// ── 跨源判定边界（二轮评审修复 #1）──

#[tokio::test]
async fn origin_without_host_header_is_rejected() {
    // Origin 存在而 Host 缺失 ⇒ 403：旧 `(Some, Some)` 匹配式在该形态下
    // 整体跳过检查放行——跨源拒绝冻结语义被旁路。
    // HTTP/1.1 客户端（reqwest）必发 Host——用裸 TCP 手工构造无 Host
    // 请求（HTTP/1.0 形态合法无 Host 头）。
    let state = build_test_state(temp_data_dir("no-host"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(server.addr)
        .await
        .expect("裸 TCP 连接");
    // 无 Host 头；Origin 指向第三方站点
    stream
        .write_all(b"GET /healthz HTTP/1.0\r\nOrigin: http://evil.example\r\n\r\n")
        .await
        .expect("写手工请求");
    let mut raw = String::new();
    // 读到响应头结束即止（无 body 长度依赖）
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let mut buf = [0u8; 512];
            let n = stream.read(&mut buf).await.expect("读响应");
            raw.push_str(&String::from_utf8_lossy(&buf[..n]));
            if raw.contains("\r\n\r\n") || n == 0 {
                break;
            }
        }
    })
    .await
    .expect("响应不超时");
    // 状态行版本与请求对齐（HTTP/1.0 请求得 HTTP/1.0 响应）——只断言码
    let status_line = raw.lines().next().unwrap_or("");
    assert!(
        status_line.contains(" 403 "),
        "Origin 无 Host 必须被拒（403），实得响应行: {:?}",
        status_line
    );
    assert!(
        !raw.contains("Access-Control-Allow"),
        "拒绝响应不得携带 CORS 放行头"
    );
}

// ── deep 参数归一（二轮评审修复 #8）──

#[tokio::test]
async fn deep_true_alias_triggers_deep_probe() {
    // `?deep=true`（监控方常见写法）曾静默拿到浅探针假 200——归一后
    // 必须走深探针（按环境 503 降级或 200 健康，flags 与状态码一致）
    let state = build_test_state(temp_data_dir("deep-true"), None)
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    // 期望按环境推导（装有 opencode 的机器 ⇒ 200 健康）——
    // 断言核心是「走的是深探针」而非浅探针假 200
    let opencode_available = std::process::Command::new("which")
        .arg("opencode")
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    let res = client.get(&server.url("/healthz?deep=true"), None, None).await;
    let status = res.status();
    let body: Value = res.json().await.expect("deep body");
    assert_eq!(body["db"], true, "深探针必须报告 db 位");
    if opencode_available {
        assert_eq!(status, 200, "PATH 有 opencode ⇒ deep=true 走深探针（200）");
        assert_eq!(body["status"], "ok", "健康状态字段");
    } else {
        assert_eq!(status, 503, "PATH 无 opencode ⇒ deep=true 仍走深探针（503）");
        assert_eq!(body["status"], "degraded", "深探针状态字段");
        assert_eq!(body["opencode"], false, "深探针必须报告 opencode 位");
    }
    // 浅探针对照：不带参数 ⇒ 200 单字段（deep=true 的对照面）
    let res = client.get(&server.url("/healthz"), None, None).await;
    assert_eq!(res.status(), 200, "无参数浅探针恒 200");
    let shallow: Value = res.json().await.expect("shallow body");
    assert_eq!(shallow["status"], "ok", "浅探针无 flags");
}
