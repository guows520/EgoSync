//! Story 15.5：server 侧工件驱动的全量对等——与前端 vitest parity.test.ts
//! 消费同一 `crates/egosync-engine/commands.json` 工件（单源双消费：
//! 任何单侧契约漂移在 CI 变红）。
//!
//! 覆盖（spec 任务口径）：
//! 1. include_str! 工件遍历全部 103 条 web-ok 命令：`POST /api/cmd/{name}`
//!    body `{}` ⇒ 非 404、状态 200、body 合法 JSON；
//! 2. 带判别头（`X-Egosync-App-Error: 1`）的响应 body 必为 13 variant
//!    单键 map（判别头 = 错误分类器；variant 清单自 error.rs 源码机械枚举）；
//! 3. llm_config_* 响应递归扫描无 `api_key` 键（密钥零泄漏）；
//! 4. 每命令 HTTP body 字节 == `to_string(parse(body))`（serde_json 规范
//!    编码往返一致——对象键序依赖 preserve_order，即其生效的逐命令证据）；
//! 5. 键序等价属性测试（preserve_order 生效证明）。
//!
//! 禁止二次 dispatch 比对（性能快照/UUID 类命令非确定性会误红——本套件
//! 只断言传输面形状，不断言业务语义）。

mod common;

use common::{login, Client, InProcessServer};
use egosync_server::bootstrap::build_test_state;
use egosync_server::routes::{APP_ERROR_HEADER, APP_ERROR_HEADER_VALUE};
use serde_json::{json, Value};

/// 临时数据目录（测试期间存活；TempDir::keep 保持目录不被清理）。
fn temp_data_dir(tag: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let path = dir.keep();
    path.join(tag)
}

/// 从 error.rs 源码机械枚举 AppError variant 名（与 api_test 同口径；
/// 格式变化导致定位失败 ⇒ panic——宁可响亮失败，不静默跳过）。
fn app_error_variants_in_source() -> Vec<String> {
    let src = include_str!("../../crates/egosync-engine/src/error.rs");
    let start = src
        .find("pub enum AppError {")
        .expect("error.rs 未找到 'pub enum AppError {'（格式变化？）");
    let rest = &src[start + "pub enum AppError {".len()..];
    let end = rest
        .find("\n}")
        .expect("error.rs AppError 枚举块未闭合（格式变化？）");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            let ident: String = t
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let is_variant = !ident.is_empty()
                && ident.chars().next().unwrap().is_ascii_uppercase()
                && t[ident.len()..].starts_with('(');
            is_variant.then_some(ident)
        })
        .collect()
}

/// 递归扫描 JSON 值：任意层级是否出现 `api_key` 键（密钥零泄漏断言）。
fn json_contains_api_key(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| k == "api_key" || json_contains_api_key(v)),
        Value::Array(items) => items.iter().any(json_contains_api_key),
        _ => false,
    }
}

#[tokio::test]
async fn all_web_ok_commands_route_with_canonical_json_bodies() {
    let variants = app_error_variants_in_source();
    assert_eq!(variants.len(), 13, "error.rs 现役 AppError variant 必须为 13");

    // 工件单源：与前端 vitest parity.test.ts 读的是同一份 commands.json
    let artifact: Value =
        serde_json::from_str(include_str!("../../crates/egosync-engine/commands.json"))
            .expect("commands.json 解析失败");
    let commands = artifact["commands"]
        .as_array()
        .expect("commands.json 必须含 commands 数组");
    assert_eq!(commands.len(), 103, "web-ok 命令必须为 103 条");
    assert_eq!(
        artifact["count"].as_u64(),
        Some(commands.len() as u64),
        "count 字段与 commands 数组长度一致"
    );

    let state = build_test_state(temp_data_dir("cmd-parity"), Some("t".into()))
        .await
        .expect("测试状态装配");
    let server = InProcessServer::start(state).await;
    let client = Client::new();
    let mut session = login(&client, &server.url(""), "t").await.expect("登录");

    for entry in commands {
        let name = entry["name"].as_str().expect("命令名");
        let url = server.url(&format!("/api/cmd/{}", name));

        let mut res = client
            .post_json(&url, Some(&json!({})), Some(&session), None)
            .await;
        // data_destroy 清空 auth_sessions（「全部销毁」冻结语义——含会话表）：
        // 401 时重登一次并重试；命令本身的断言口径不变。env 态令牌不受
        // app_settings 清空影响（login 仅比对 env）。
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
            session = login(&client, &server.url(""), "t")
                .await
                .expect("data_destroy 后重登");
            res = client
                .post_json(&url, Some(&json!({})), Some(&session), None)
                .await;
        }

        assert_ne!(res.status(), 404, "{} 必须在 server 路由面（非 404）", name);
        assert_eq!(
            res.status(),
            200,
            "{} 必须响应 200（成功或 AppError 单键 map）",
            name
        );

        let has_header = res.headers().get(APP_ERROR_HEADER).is_some();
        if has_header {
            assert_eq!(
                res.headers().get(APP_ERROR_HEADER)
                    .and_then(|v| v.to_str().ok()),
                Some(APP_ERROR_HEADER_VALUE),
                "{} 判别头值冻结为 1",
                name
            );
        }
        let body_text = res.text().await.expect("body 文本");

        // body 必须为合法 JSON（成功值或错误单键 map）
        let body: Value = serde_json::from_str(&body_text)
            .unwrap_or_else(|e| panic!("{} 的 body 必须为合法 JSON: {} ({})", name, e, body_text));

        // 判别头 = 错误分类器：带头响应 body 必为 13 variant 单键 map
        if has_header {
            let map = body
                .as_object()
                .unwrap_or_else(|| panic!("{} 的错误 body 必须为对象: {}", name, body_text));
            assert_eq!(map.len(), 1, "{} 的错误 body 必须单键: {}", name, body_text);
            let key = map.keys().next().expect("单键必存在");
            assert!(
                variants.iter().any(|v| v == key),
                "{} 判别头响应的单键 {} 必须是 13 variant 之一",
                name,
                key
            );
        }

        // 规范编码往返一致：HTTP body 字节 == to_string(parse(body))。
        // 对象键序依赖 serde_json preserve_order（IndexMap 保插入序）——
        // 这是 preserve_order 在全部 103 条命令上生效的逐命令证据
        //（Value 往返不改变键序 ⇒ server body 与桌面 invoke 直接序列化同构）。
        let canonical = serde_json::to_string(&body)
            .unwrap_or_else(|e| panic!("{} body 再序列化失败: {}", name, e));
        assert_eq!(
            body_text, canonical,
            "{} 的 HTTP body 必须是 serde_json 规范编码（键序往返一致）",
            name
        );

        // 密钥零泄漏：llm_config_* 响应任意层级无 api_key 键（只有 api_key_ref）
        if name.starts_with("llm_config_") {
            assert!(
                !json_contains_api_key(&body),
                "{} 的响应任意层级不得出现 api_key 键: {}",
                name,
                body_text
            );
        }
    }
}

/// 键序等价属性测试（preserve_order 生效证明）：构造字段声明序非字母序的
/// 样本，`to_string(to_value(x))` 必须与直接 `to_string(x)` 逐字节一致——
/// 默认 BTreeMap（字母序）下两者不同，preserve_order（IndexMap 保插入序
/// = 结构体字段序）下恒等。与 engine error.rs 同款属性双侧钉死。
#[test]
fn serde_json_preserve_order_roundtrip_property() {
    #[derive(serde::Serialize)]
    struct Sample {
        zebra: u32,
        alpha: String,
        middle: bool,
    }
    let sample = Sample {
        zebra: 1,
        alpha: "a".to_string(),
        middle: true,
    };
    let direct = serde_json::to_string(&sample).expect("直接序列化");
    let roundtrip =
        serde_json::to_string(&serde_json::to_value(&sample).expect("转 Value")).expect("Value 序列化");
    assert_eq!(
        direct, roundtrip,
        "Value 往返不得改变键序（preserve_order 生效证据）：直接={} 往返={}",
        direct, roundtrip
    );
    assert_eq!(direct, r#"{"zebra":1,"alpha":"a","middle":true}"#);
}
