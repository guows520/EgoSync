//! Story 17.3 集成测试：逻辑级备份端点（/api/export + /api/import）。
//!
//! I/O 矩阵数据主权主战场（api_test.rs I/O 矩阵组织先例）：
//!
//! | 场景 | 预期 |
//! |------|------|
//! | 已认证 GET /api/export | 200 完整导出包（与桌面同格式 exportVersion="1.0"）+ attachment 头；密钥值零泄漏 |
//! | 未认证导出/导入 | 401 不触碰数据 |
//! | 导入恢复（服务端导出包） | 200 ImportResult+missingSecrets；核心数据抽查逐项一致；data:imported SSE 广播 |
//! | 导入后密钥缺失 | 报告逐项列出缺失 ref+重录文案，导入本身成功 |
//! | 桌面导出包 → 云端导入 | 同引擎同格式 ⇒ 互导成立（经引擎 export_json 真实桌面路径产包） |
//! | 云端导出包 → 桌面导入 | 经引擎 import_all 真实桌面路径恢复 |
//! | 损坏包 / 版本不符 | 200+AppError 单键 map+判别头；库内容不变（不半写） |
//!
//! 双向覆盖口径（AC「跨形态互导」）：桌面导出=引擎 `export_json` 写盘
//! （桌面壳 data_export 的引擎侧主体），云端导入=本路由——同一对引擎
//! 纯逻辑的 HTTP 面组合即「桌面→云端」；反向同理由 `/api/export` body
//! 落盘喂 `import_all`（桌面壳 data_import 的引擎侧主体）。

mod common;

use common::{login, Client, InProcessServer};
use egosync_engine::db::pool::{init_conversations_db, init_db};
use egosync_engine::services::data_export::{export_json, import_all};
use egosync_server::bootstrap::build_test_state;
use egosync_server::routes::{APP_ERROR_HEADER, APP_ERROR_HEADER_VALUE};
use serde_json::Value;

/// 临时数据目录（api_test.rs 同款：TempDir::keep 保持存活）。
fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("创建临时目录");
    dir.keep().join(tag)
}

/// 测试种子数据常量（导入后逐项抽查的锚点）。
const ROLE_ID: &str = "role-roundtrip-1";
const ROLE_NAME: &str = "产品经理";
const TASK_TITLE: &str = "写 PRD";
const MEMORY_CONTENT: &str = "喜欢简洁的设计";
const CONFIG_NAME: &str = "DeepSeek 主力";
const API_KEY_REF: &str = "llm_roundtrip_api_key";
const SECRET_VALUE: &str = "sk-must-never-leak";
const CONV_TITLE: &str = "与管家的第一次对话";
const MSG_CONTENT: &str = "帮我整理本周任务";

/// 向主库+对话库播种最小核心数据（直接 SQL——与 import_json_data 的
/// INSERT 列集一致）。
async fn seed_core_data(
    state: &std::sync::Arc<egosync_server::AppState>,
) {
    sqlx::query(
        "INSERT INTO roles (id, name, icon, color, goal, personality_prompt, status, energy, energy_updated_at, skills_config, proactivity_level, archived_at, created_at, updated_at)
         VALUES (?1, ?2, '🎯', '#FF0000', '打造好产品', '', 'active', 80, NULL, '{}', 'moderate', NULL, '2026-09-21T00:00:00Z', '2026-09-21T00:00:00Z')",
    )
    .bind(ROLE_ID)
    .bind(ROLE_NAME)
    .execute(&state.ctx.pool)
    .await
    .expect("seed role");

    sqlx::query(
        "INSERT INTO tasks (id, owner_type, role_id, title, deadline, quadrant, is_big_rock, is_completed, completed_at, sort_order, protection_status, confidence, manual_override, classification_reason, created_at, updated_at, deleted_at)
         VALUES ('task-rt-1', 'role', ?1, ?2, NULL, 'Q1', 1, 0, NULL, 0, 'normal', NULL, 0, NULL, '2026-09-21T00:00:00Z', '2026-09-21T00:00:00Z', NULL)",
    )
    .bind(ROLE_ID)
    .bind(TASK_TITLE)
    .execute(&state.ctx.pool)
    .await
    .expect("seed task");

    sqlx::query(
        "INSERT INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids, created_at)
         VALUES ('mem-rt-1', ?1, 'preference', ?2, 'conv-rt-1', 'msg-rt-1', '2026-09-21T00:00:00Z')",
    )
    .bind(ROLE_ID)
    .bind(MEMORY_CONTENT)
    .execute(&state.ctx.pool)
    .await
    .expect("seed memory");

    sqlx::query(
        "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, is_default, created_at, updated_at)
         VALUES ('cfg-rt-1', ?1, 'deepseek', 'https://api.deepseek.com/v1', 'deepseek-chat', ?2, 1, '2026-09-21T00:00:00Z', '2026-09-21T00:00:00Z')",
    )
    .bind(CONFIG_NAME)
    .bind(API_KEY_REF)
    .execute(&state.ctx.pool)
    .await
    .expect("seed llm_config");

    sqlx::query(
        "INSERT INTO conversations (id, role_id, title, started_at, updated_at)
         VALUES ('conv-rt-1', ?1, ?2, '2026-09-21T00:00:00Z', '2026-09-21T00:00:00Z')",
    )
    .bind(ROLE_ID)
    .bind(CONV_TITLE)
    .execute(&*state.ctx.conv_pool)
    .await
    .expect("seed conversation");

    sqlx::query(
        "INSERT INTO messages (id, conversation_id, role, content, thinking_content, is_complete, created_at, routing_metadata)
         VALUES ('msg-rt-1', 'conv-rt-1', 'user', ?1, '', 1, '2026-09-21T00:00:00Z', NULL)",
    )
    .bind(MSG_CONTENT)
    .execute(&*state.ctx.conv_pool)
    .await
    .expect("seed message");
}

/// 导出→断言包形状与零密钥泄漏（多测试复用）。
async fn export_and_assert_package(server: &InProcessServer, session: &str) -> String {
    let client = Client::new();
    let res = client
        .get(&server.url("/api/export"), Some(session), None)
        .await;
    assert_eq!(res.status(), 200, "已认证导出必须 200");
    assert_eq!(
        res.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
        "application/json",
        "导出 body 为 JSON"
    );
    let disposition = res
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        disposition.starts_with("attachment; filename=\"egosync-export-")
            && disposition.ends_with(".json\""),
        "attachment 文件名（与桌面 export_json 落盘命名同构）: {disposition}"
    );
    // 错误判别头不得出现（成功响应非 AppError）
    assert!(
        res.headers().get(APP_ERROR_HEADER).is_none(),
        "成功导出不得携带 AppError 判别头"
    );

    let body = res.text().await.expect("导出 body 文本");
    let package: Value = serde_json::from_str(&body).expect("导出包必须为有效 JSON");
    assert_eq!(
        package["exportVersion"], "1.0",
        "与桌面导出包同版本口径（exportVersion 字段 camelCase）"
    );
    assert_eq!(package["roles"][0]["id"], ROLE_ID, "角色段在场");
    assert_eq!(package["roles"][0]["name"], ROLE_NAME);
    assert_eq!(package["tasks"][0]["title"], TASK_TITLE, "任务段在场");
    assert_eq!(
        package["memories"][0]["content"], MEMORY_CONTENT,
        "记忆段在场"
    );
    assert_eq!(
        package["llmConfigs"][0]["apiKeyRef"], API_KEY_REF,
        "密钥引用随包导出（llmConfigs[].apiKeyRef）"
    );
    assert_eq!(
        package["conversations"][0]["title"], CONV_TITLE,
        "会话段在场（对话库并入同一包）"
    );
    // 密钥值零泄漏（响应形状测试断言风格沿用 17.1：值永不出现在任何响应）
    assert!(
        !body.contains(SECRET_VALUE),
        "导出包绝不含密钥值（仅 apiKeyRef 引用）"
    );
    assert!(
        !body.contains("apiKey\":"),
        "导出包不得出现 apiKey 值字段（仅 apiKeyRef）"
    );
    body
}

// ── I/O 矩阵：已认证导出（含密钥零泄漏）──

/// 已认证 GET /api/export：完整包（exportVersion="1.0" + 双库数据 +
/// 密钥引用）+ attachment 头；secrets.json 在场的密钥值零泄漏。
#[tokio::test]
async fn export_returns_desktop_format_package_without_secret_values() {
    let state = build_test_state(temp_data_dir("backup-export"), Some("backup-token".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state).await;
    // 密钥落 secrets.json（在场）——导出仍只有 ref，值不得出现
    state
        .ctx
        .secrets
        .save_secret(API_KEY_REF, SECRET_VALUE)
        .expect("写入种子密钥");

    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "backup-token")
        .await
        .expect("登录");

    export_and_assert_package(&server, &session).await;
}

// ── I/O 矩阵：未认证 401（不触碰数据）──

#[tokio::test]
async fn export_and_import_require_authentication() {
    let state = build_test_state(temp_data_dir("backup-auth"), Some("backup-token".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state).await;
    let server = InProcessServer::start(state).await;
    let client = Client::new();

    let res = client.get(&server.url("/api/export"), None, None).await;
    assert_eq!(res.status(), 401, "未认证导出必须 401");
    let err: Value = res.json().await.expect("401 body");
    assert_eq!(err["error"], "unauthorized", "401 统一形状");

    // 未认证导入：401（Bearer/Cookie 双通道均缺）
    let res = client
        .post_json(
            &server.url("/api/import"),
            Some(&serde_json::json!({})),
            None,
            None,
        )
        .await;
    assert_eq!(res.status(), 401, "未认证导入必须 401");

    // Bearer 通道同样可达（与 cmd 路由一致的认证面）
    let res = client
        .get_bearer(&server.url("/api/export"), Some("backup-token"), None)
        .await;
    assert_eq!(res.status(), 200, "Bearer 认证导出可达");

    // 数据未被触碰（未认证请求零副作用）
    let roles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
        .fetch_one(&server.state.ctx.pool)
        .await
        .expect("count roles");
    assert_eq!(roles, 1, "未认证请求不得触碰数据");
}

// ── I/O 矩阵：导入恢复（服务端包 → 新实例）+ 密钥缺失报告 + SSE ──

/// 已认证 POST /api/import：核心数据逐项一致恢复；missingSecrets 报告
/// （新实例无密钥——报告含 ref+重录文案，导入成功不受阻塞）；
/// `data:imported` 经 SSE 广播（engine 常量事件名 + ImportResult payload）。
#[tokio::test]
async fn import_restores_core_data_and_reports_missing_secrets() {
    // 实例 A：播种 + 导出
    let state_a = build_test_state(temp_data_dir("backup-rt-a"), Some("backup-token".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state_a).await;
    let server_a = InProcessServer::start(state_a).await;
    let client = Client::new();
    let session_a = login(&client, &server_a.url(""), "backup-token")
        .await
        .expect("登录 A");
    let package = export_and_assert_package(&server_a, &session_a).await;
    drop(server_a);

    // 实例 B：全新数据目录（secrets.json 不存在 ⇒ 探测应报缺失）
    let state_b = build_test_state(temp_data_dir("backup-rt-b"), Some("backup-token".into()))
        .await
        .expect("测试状态装配");
    let server_b = InProcessServer::start(state_b).await;
    let session_b = login(&client, &server_b.url(""), "backup-token")
        .await
        .expect("登录 B");

    // SSE 订阅先建立（广播通道——导入事件扇出观测）
    let mut sse_rx = server_b.state.events_tx.subscribe();

    let res = client
        .post_json(
            &server_b.url("/api/import"),
            Some(&serde_json::from_str::<Value>(&package).expect("包 JSON")),
            Some(&session_b),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "导入成功必须 200");
    assert!(
        res.headers().get(APP_ERROR_HEADER).is_none(),
        "成功导入不得携带 AppError 判别头"
    );
    let body: Value = res.json().await.expect("导入响应 body");

    // ImportResult（camelCase 计数）
    assert_eq!(body["imported"]["rolesCount"], 1, "角色数");
    assert_eq!(body["imported"]["tasksCount"], 1, "任务数");
    assert_eq!(body["imported"]["memoriesCount"], 1, "记忆数");
    assert_eq!(body["imported"]["conversationsCount"], 1, "会话数");
    assert_eq!(body["imported"]["messagesCount"], 1, "消息数");

    // 密钥可达性报告：逐项列出缺失 ref + 重录路径文案（missing_api_key_error 同款）
    let missing = body["missingSecrets"]
        .as_array()
        .expect("missingSecrets 为数组");
    assert_eq!(missing.len(), 1, "仅一条配置缺失: {missing:?}");
    assert_eq!(missing[0]["configName"], CONFIG_NAME, "报告含配置名");
    assert_eq!(missing[0]["apiKeyRef"], API_KEY_REF, "报告含密钥引用");
    assert!(
        missing[0]["message"]
            .as_str()
            .expect("message 为字符串")
            .contains("设置 → 模型服务配置"),
        "报告含重录路径文案: {}",
        missing[0]["message"]
    );
    assert!(
        missing[0]["message"]
            .as_str()
            .expect("message 为字符串")
            .contains(CONFIG_NAME),
        "文案含配置名（missing_api_key_error 同款）"
    );

    // 核心数据抽查（新实例逐项一致）
    let (role_name, task_title, memory_content, config_ref): (String, String, String, String) =
        sqlx::query_as(
            "SELECT r.name, t.title, m.content, c.api_key_ref \
             FROM roles r JOIN tasks t ON t.role_id = r.id \
             JOIN memories m ON m.role_id = r.id \
             JOIN llm_configs c ON c.id = 'cfg-rt-1' WHERE r.id = ?1",
        )
        .bind(ROLE_ID)
        .fetch_one(&server_b.state.ctx.pool)
        .await
        .expect("导入后主库抽查");
    assert_eq!(role_name, ROLE_NAME);
    assert_eq!(task_title, TASK_TITLE);
    assert_eq!(memory_content, MEMORY_CONTENT);
    assert_eq!(config_ref, API_KEY_REF, "配置（含密钥引用）随包恢复");

    let (conv_title, msg_content): (String, String) = sqlx::query_as(
        "SELECT cv.title, ms.content FROM conversations cv \
         JOIN messages ms ON ms.conversation_id = cv.id WHERE cv.id = 'conv-rt-1'",
    )
    .fetch_one(&*server_b.state.ctx.conv_pool)
    .await
    .expect("导入后对话库抽查");
    assert_eq!(conv_title, CONV_TITLE);
    assert_eq!(msg_content, MSG_CONTENT);

    // SSE：data:imported 已广播（engine 常量事件名 + ImportResult payload）
    let event = sse_rx
        .try_recv()
        .expect("导入成功必须广播 data:imported");
    assert_eq!(
        event.event,
        egosync_engine::events::DATA_IMPORTED_EVENT,
        "事件名复用 engine 常量（与桌面壳同事件名）"
    );
    assert_eq!(event.event, "data:imported");
    assert_eq!(event.payload["rolesCount"], 1, "payload 为 ImportResult");
}

// ── I/O 矩阵：跨形态互导（双向）──

/// 桌面导出包 → 云端导入：经引擎 `export_json`（桌面壳 data_export 的
/// 引擎侧主体，真实桌面落盘路径）产包，POST /api/import 恢复。
#[tokio::test]
async fn desktop_export_package_imports_via_api() {
    // 源实例：播种后经桌面引擎路径导出到磁盘
    let state = build_test_state(temp_data_dir("backup-cross-desktop"), Some("t".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state).await;
    let out_dir = tempfile::tempdir().expect("导出目录");
    let desktop_file = export_json(&state.ctx.pool, &state.ctx.conv_pool, out_dir.path())
        .await
        .expect("桌面引擎导出");
    let desktop_package = std::fs::read_to_string(&desktop_file).expect("读桌面导出包");

    // 目标实例（新数据目录）经 /api/import 导入
    let state_b = build_test_state(temp_data_dir("backup-cross-target"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state_b).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    let res = client
        .post_json(
            &server.url("/api/import"),
            Some(&serde_json::from_str::<Value>(&desktop_package).expect("桌面包 JSON")),
            Some(&session),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "桌面包导入云端必须 200");
    let body: Value = res.json().await.expect("导入响应");
    assert_eq!(body["imported"]["rolesCount"], 1, "桌面包角色恢复");
    assert_eq!(body["imported"]["conversationsCount"], 1, "桌面包会话恢复");
    assert_eq!(
        body["missingSecrets"]
            .as_array()
            .expect("missingSecrets")
            .len(),
        1,
        "桌面 keyring 不随包迁移 ⇒ 密钥缺失报告"
    );
}

/// 云端导出包 → 桌面导入：`GET /api/export` body 落盘，经引擎
/// `import_all`（桌面壳 data_import 的引擎侧主体）恢复到全新双库。
#[tokio::test]
async fn api_export_package_imports_via_desktop_engine() {
    let state = build_test_state(temp_data_dir("backup-cross-cloud"), Some("t".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state).await;
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");
    let package = export_and_assert_package(&server, &session).await;

    // 桌面恢复路径：全新双库（模拟新桌面实例）+ import_all
    let dir = tempfile::tempdir().expect("桌面数据目录");
    let desktop_pool = init_db(&dir.path().join("egosync.db"))
        .await
        .expect("桌面主库");
    let desktop_conv = init_conversations_db(&dir.path().join("conversations.db"))
        .await
        .expect("桌面对话库");
    let package_path = dir.path().join("cloud-export.json");
    std::fs::write(&package_path, &package).expect("落盘云端导出包");

    let result = import_all(&desktop_pool, &desktop_conv, &package_path)
        .await
        .expect("桌面引擎导入云端包");

    assert_eq!(result.roles_count, 1, "角色恢复");
    assert_eq!(result.tasks_count, 1, "任务恢复");
    assert_eq!(result.memories_count, 1, "记忆恢复");
    assert_eq!(result.conversations_count, 1, "会话恢复");
    assert_eq!(result.messages_count, 1, "消息恢复");

    let role_name: String = sqlx::query_scalar("SELECT name FROM roles WHERE id = ?1")
        .bind(ROLE_ID)
        .fetch_one(&desktop_pool)
        .await
        .expect("抽查角色");
    assert_eq!(role_name, ROLE_NAME, "云端包经桌面路径恢复一致");
}

// ── I/O 矩阵：损坏包 AppError 不半写 ──

/// 损坏包（非法 JSON）与版本不符包：200+AppError 单键 map+判别头，
/// 库内容不变（解析在事务开启之前——失败不半写）。
#[tokio::test]
async fn corrupted_package_returns_app_error_without_partial_write() {
    let state = build_test_state(temp_data_dir("backup-corrupt"), Some("t".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state).await;
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录");

    // 非法 JSON body
    let res = client
        .post_bytes(&server.url("/api/import"), b"not-a-json-package".to_vec(), Some(&session))
        .await;
    assert_eq!(res.status(), 200, "错误白名单：AppError 走 200");
    assert_eq!(
        res.headers()
            .get(APP_ERROR_HEADER)
            .and_then(|v| v.to_str().ok()),
        Some(APP_ERROR_HEADER_VALUE),
        "AppError 判别头在场"
    );
    let body: Value = res.json().await.expect("错误 body");
    let map = body.as_object().expect("单键 map");
    assert_eq!(map.len(), 1, "AppError 单键");
    assert!(
        map.contains_key("ValidationError"),
        "损坏包走 ValidationError（实得键: {map:?}）"
    );

    // 版本不符包（exportVersion="0.9"）：同 AppError 路径
    let bad_version = serde_json::json!({
        "roles": [], "tasks": [], "memories": [], "suggestions": [],
        "notifications": [], "mission": null, "conflicts": [], "briefings": [],
        "weeklyReviews": [], "llmConfigs": [], "appSettings": [], "mcpServers": [],
        "skills": [], "skillBindings": [], "q2Reminders": [],
        "bigRockProtectionReminders": [], "forgottenMemorySources": [],
        "roleMcpServerBindings": [], "butlerMcpServers": [], "pairedDevices": [],
        "conversations": [], "messages": [],
        "exportedAt": "2026-09-21T00:00:00Z", "exportVersion": "0.9",
    });
    let res = client
        .post_json(&server.url("/api/import"), Some(&bad_version), Some(&session), None)
        .await;
    assert_eq!(res.status(), 200, "版本不符同走 200+AppError");
    assert_eq!(
        res.headers()
            .get(APP_ERROR_HEADER)
            .and_then(|v| v.to_str().ok()),
        Some(APP_ERROR_HEADER_VALUE),
        "AppError 判别头在场"
    );
    let body: Value = res.json().await.expect("版本不符 body");
    assert!(
        body.as_object()
            .expect("单键 map")
            .contains_key("ValidationError"),
        "版本不符走 ValidationError: {body}"
    );

    // 库内容不变（不半写：播种数据原样在场）
    let (roles, tasks, memories): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM roles), (SELECT COUNT(*) FROM tasks), (SELECT COUNT(*) FROM memories)",
    )
    .fetch_one(&server.state.ctx.pool)
    .await
    .expect("导入失败后抽查");
    assert_eq!((roles, tasks, memories), (1, 1, 1), "失败导入不得半写");
}

// ── 导入响应中的 SSE 广播（无订阅者路径）──

/// 无在线客户端时导入照常成功（send 无订阅者 Err 是广播语义正常态——
/// 事件丢弃不构成错误）。
#[tokio::test]
async fn import_succeeds_without_sse_subscribers() {
    let state_a = build_test_state(temp_data_dir("backup-nosub-a"), Some("t".into()))
        .await
        .expect("测试状态装配");
    seed_core_data(&state_a).await;
    let server_a = InProcessServer::start(state_a).await;
    let client = Client::new();
    let session_a = login(&client, &server_a.url(""), "t").await.expect("登录");
    let package = export_and_assert_package(&server_a, &session_a).await;
    drop(server_a);

    let state_b = build_test_state(temp_data_dir("backup-nosub-b"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server_b = InProcessServer::start(state_b).await;
    let session_b = login(&client, &server_b.url(""), "t").await.expect("登录");
    // 不订阅 events_tx（无接收者）——导入仍须 200
    let res = client
        .post_json(
            &server_b.url("/api/import"),
            Some(&serde_json::from_str::<Value>(&package).expect("包 JSON")),
            Some(&session_b),
            None,
        )
        .await;
    assert_eq!(res.status(), 200, "无订阅者不影响导入");
    let body: Value = res.json().await.expect("导入响应");
    assert_eq!(body["imported"]["rolesCount"], 1);
}

// ── 评审修复（并发互斥）：并发导入不混装 ──
//
// 导入是双库全量替换（主库/对话库各自单事务——引擎既有语义）。没有
// 互斥时两个并发导入交错提交会让主库与对话库来自不同导出包（SQLite
// 只保证单库原子，不保证跨请求串行）。backup.rs 的 import_lock 保证
// 导入串行 + 导出快照不落中间态；本测试并发提交两个可区分的包，
// 断言最终库内容是**单一包的完整数据**（角色/任务/记忆/会话内部一致），
// 绝不出现「角色来自 A、任务来自 B」的混装。

#[tokio::test]
async fn concurrent_imports_do_not_mix_packages() {
    // 两个源实例：同构播种后改名为可区分的包 A / 包 B
    let mut packages = Vec::new();
    for tag in ["A", "B"] {
        let dir = temp_data_dir(&format!("backup-conc-{tag}"));
        let state = build_test_state(dir, Some("t".into()))
            .await
            .expect("测试状态装配");
        seed_core_data(&state).await;
        // 包标识：角色/任务/记忆/会话标题带上包标签（ID 不变——跨包可比）
        sqlx::query("UPDATE roles SET name = ?1 WHERE id = ?2")
            .bind(format!("并发导入角色-{tag}"))
            .bind(ROLE_ID)
            .execute(&state.ctx.pool)
            .await
            .expect("标记角色");
        sqlx::query("UPDATE tasks SET title = ?1 WHERE id = 'task-rt-1'")
            .bind(format!("并发导入任务-{tag}"))
            .execute(&state.ctx.pool)
            .await
            .expect("标记任务");
        sqlx::query("UPDATE memories SET content = ?1 WHERE id = 'mem-rt-1'")
            .bind(format!("并发导入记忆-{tag}"))
            .execute(&state.ctx.pool)
            .await
            .expect("标记记忆");
        sqlx::query("UPDATE conversations SET title = ?1 WHERE id = 'conv-rt-1'")
            .bind(format!("并发导入会话-{tag}"))
            .execute(&*state.ctx.conv_pool)
            .await
            .expect("标记会话");
        let server = InProcessServer::start(state).await;
        let client = Client::new();
        let session = login(&client, &server.url(""), "t").await.expect("登录源");
        // 最小导出（深度包断言由专用用例覆盖；此处只需 200 + 包文本——
        // 角色已改名，export_and_assert_package 的固定名断言不适用）
        let res = client
            .get(&server.url("/api/export"), Some(&session), None)
            .await;
        assert_eq!(res.status(), 200, "源实例导出须 200");
        packages.push(res.text().await.expect("包文本"));
        drop(server);
    }
    let [package_a, package_b]: [String; 2] = packages.try_into().expect("恰好两个包");

    // 目标实例：并发提交两个导入（tokio::join!——两请求同时在途）
    let state = build_test_state(temp_data_dir("backup-conc-target"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let session = login(&client, &server.url(""), "t").await.expect("登录目标");

    let url = server.url("/api/import");
    let body_a: Value = serde_json::from_str(&package_a).expect("包 A JSON");
    let body_b: Value = serde_json::from_str(&package_b).expect("包 B JSON");
    let session_ref = &session;
    let (res_a, res_b) = tokio::join!(
        client.post_json(&url, Some(&body_a), Some(session_ref), None),
        client.post_json(&url, Some(&body_b), Some(session_ref), None),
    );
    assert_eq!(res_a.status(), 200, "并发导入 A 须成功");
    assert_eq!(res_b.status(), 200, "并发导入 B 须成功");

    // 最终库内容：单一包的完整数据（互斥下 = 后获取锁的那个包）。
    // 断言内部一致性——角色与任务/记忆/会话必须来自同一包，绝不混装。
    let (role_name, task_title, memory_content): (String, String, String) = sqlx::query_as(
        "SELECT r.name, t.title, m.content FROM roles r \
         JOIN tasks t ON t.role_id = r.id \
         JOIN memories m ON m.role_id = r.id WHERE r.id = ?1",
    )
    .bind(ROLE_ID)
    .fetch_one(&server.state.ctx.pool)
    .await
    .expect("主库一致性抽查");
    let tag_of = |name: &str| -> String {
        ["A", "B"]
            .iter()
            .find(|t| name.ends_with(*t))
            .map(|t| t.to_string())
            .unwrap_or_else(|| panic!("角色名不带包标签: {name}"))
    };
    let role_tag = tag_of(&role_name);
    assert_eq!(
        task_title,
        format!("并发导入任务-{role_tag}"),
        "任务与角色必须同包（混装=互斥失效）"
    );
    assert_eq!(
        memory_content, format!("并发导入记忆-{role_tag}"),
        "记忆与角色必须同包"
    );
    let conv_title: String =
        sqlx::query_scalar("SELECT title FROM conversations WHERE id = 'conv-rt-1'")
            .fetch_one(&*server.state.ctx.conv_pool)
            .await
            .expect("会话抽查");
    assert_eq!(
        conv_title,
        format!("并发导入会话-{role_tag}"),
        "对话库与主库必须同包（跨库混装=互斥失效）"
    );
}
