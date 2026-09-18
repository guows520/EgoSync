//! Story 15.4 对等断言（「靠机制不靠纪律」的测试侧守门）：
//!
//! 1. 壳 lib.rs generate_handler 源码扫描集合 == commands.json 工件
//!    web-ok ∪ desktop-only(15) ∪ perf-test 门控(2)（lib.rs:567-595
//!    源码扫描先例）；
//! 2. 生成的 dispatch registry（WEB_OK_COMMANDS）与工件同源零漂移；
//! 3. desktop-only / perf-test 门控命令不在 server 路由面（物理 404 输入）；
//! 4. 无 dev 免认证旁路（探针串见测试内——源码零字面量防自噬）；
//! 5. engine 零 `tauri::` 引用（含新 commands 模块——CI 同口径的测试侧
//!    先行断言）。

use std::collections::BTreeSet;
use std::path::PathBuf;

/// 仓库根目录（server/ 的上级）。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// 从壳 lib.rs 源码扫描 generate_handler 注册的命令全名单。
fn scan_generate_handler() -> BTreeSet<String> {
    let lib_rs = std::fs::read_to_string(
        repo_root().join("egosync-app/src-tauri/src/lib.rs"),
    )
    .expect("读取壳 lib.rs 失败");
    let mut names = BTreeSet::new();
    for line in lib_rs.lines() {
        let trimmed = line.trim();
        // 形如 `commands::domain::command,`（含 #[cfg(feature = "perf-test")]
        // 门控行——其后的条目同样计入注册面）
        if let Some(rest) = trimmed.strip_prefix("commands::") {
            if let Some(entry) = rest.strip_suffix(',') {
                let parts: Vec<&str> = entry.split("::").collect();
                if parts.len() == 2 && parts.iter().all(|p| !p.is_empty()) {
                    names.insert(parts[1].to_string());
                }
            }
        }
    }
    assert!(
        !names.is_empty(),
        "generate_handler 源码扫描必须提取到命令（提取器失效）"
    );
    names
}

/// 解析 commands.json 工件的 web-ok 命令名集合。
fn artifact_web_ok_names() -> BTreeSet<String> {
    let text = std::fs::read_to_string(repo_root().join("crates/egosync-engine/commands.json"))
        .expect("读取 commands.json 失败");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("commands.json 解析失败");
    let commands = doc["commands"]
        .as_array()
        .expect("commands.json 必须含 commands 数组");
    let mut names = BTreeSet::new();
    for entry in commands {
        let name = entry["name"].as_str().expect("命令名");
        assert_eq!(
            entry["capability"].as_str(),
            Some("web-ok"),
            "工件命令 {} 的 capability 必须为 web-ok",
            name
        );
        names.insert(name.to_string());
    }
    assert_eq!(
        doc["count"].as_u64(),
        Some(commands.len() as u64),
        "count 字段与 commands 数组长度一致"
    );
    names
}

#[test]
fn generate_handler_scan_matches_artifact_union_gated() {
    let scanned = scan_generate_handler();
    let artifact = artifact_web_ok_names();

    let desktop_only: BTreeSet<String> =
        egosync_engine::capabilities::DESKTOP_ONLY_COMMANDS
            .iter()
            .map(|s| s.to_string())
            .collect();
    let perf_gated: BTreeSet<String> =
        egosync_engine::capabilities::PERF_TEST_GATED_COMMANDS
            .iter()
            .map(|s| s.to_string())
            .collect();

    // 名单完整性先钉死（desktop-only 15 / 门控 2 / 工件 103）
    // （评审回环裁决 A：secret_store_save/load/delete 划归 desktop-only）
    assert_eq!(desktop_only.len(), 15, "desktop-only 名单必须为 15 条");
    assert_eq!(perf_gated.len(), 2, "perf-test 门控名单必须为 2 条");
    assert_eq!(artifact.len(), 103, "web-ok 命令必须为 103 条");

    let mut expected = artifact.clone();
    expected.extend(desktop_only);
    expected.extend(perf_gated);

    assert_eq!(
        scanned, expected,
        "generate_handler 注册面必须 == 工件 web-ok ∪ desktop-only(15) ∪ 门控\n\
         仅在壳侧: {:?}\n仅在期望面: {:?}",
        scanned.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&scanned).collect::<Vec<_>>(),
    );
}

#[test]
fn dispatch_registry_matches_artifact_exactly() {
    let artifact = artifact_web_ok_names();
    let dispatch: BTreeSet<String> = egosync_server::dispatch_gen::WEB_OK_COMMANDS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        artifact, dispatch,
        "生成的 dispatch registry 与 commands.json 工件必须同源零漂移"
    );
}

#[test]
fn desktop_only_and_gated_never_in_dispatch() {
    let dispatch: BTreeSet<String> = egosync_server::dispatch_gen::WEB_OK_COMMANDS
        .iter()
        .map(|s| s.to_string())
        .collect();
    for name in egosync_engine::capabilities::DESKTOP_ONLY_COMMANDS {
        assert!(!dispatch.contains(*name), "desktop-only {} 不得入路由面", name);
    }
    for name in egosync_engine::capabilities::PERF_TEST_GATED_COMMANDS {
        assert!(!dispatch.contains(*name), "门控 {} 不得入路由面", name);
    }
}

/// 目录树文本文件扫描（扩展名白名单——跳过 target/node_modules/生成物）。
fn scan_tree_for(dir: &str, needle: &str, hits: &mut Vec<String>) {
    let root = repo_root().join(dir);
    let mut stack = vec![root];
    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() {
                // 跳过构建产物/依赖目录（它们非源码面）
                if matches!(name, "target" | "node_modules" | "dist" | ".git") {
                    continue;
                }
                stack.push(p);
            } else {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "rs" | "ts" | "tsx" | "js" | "yml" | "yaml" | "toml" | "sh") {
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        if text.contains(needle) {
                            hits.push(p.display().to_string());
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn no_dev_auth_bypass_anywhere() {
    // 探针串运行期拼接（源码零字面量——扫描器不得自噬）
    let needles = [
        format!("{}{}", "EGOSYNC_DEV_", "NO_AUTH"),
        format!("{}{}", "DEV_", "NO_AUTH"),
    ];
    let mut hits = Vec::new();
    for needle in &needles {
        for dir in ["server", "egosync-app", "crates"] {
            scan_tree_for(dir, needle, &mut hits);
        }
    }
    assert!(hits.is_empty(), "存在 dev 免认证旁路痕迹: {:?}", hits);
}

#[test]
fn engine_source_has_no_tauri_references() {
    let root = repo_root().join("crates/egosync-engine/src");
    let mut stack = vec![root];
    let mut hits = Vec::new();
    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    if text.contains("tauri::") {
                        hits.push(p.display().to_string());
                    }
                }
            }
        }
    }
    assert!(hits.is_empty(), "engine 源码零 tauri:: 引用被违反: {:?}", hits);
}
