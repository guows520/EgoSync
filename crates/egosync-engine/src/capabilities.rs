//! Story 15.4: 命令能力门控事实源。
//!
//! desktop-only 名单（15 条）与 perf-test 门控名单（2 条）在此落死；
//! commands.json 工件只含 web-ok 命令（desktop-only 物理排除），
//! server 路由据 `is_web_command` 判 404，前端 capabilities（15.5）
//! 据本模块与工件合并生成。
//!
//! 名单变更必须与壳 lib.rs generate_handler / server 路由对等断言同步
//! （对等测试守门：generate_handler 源码扫描集合 == 工件 web-ok ∪
//! desktop-only ∪ perf-test 门控）。

/// desktop-only 命令（15 条）：仅在桌面壳注册，server 物理不路由。
///
/// - 4 个 rfd 对话框命令（chat_pick_working_directory / skill_pick_custom_directory
///   / pick_import_file / data_export）——无头环境无文件选择对话框；
/// - data_import——桌面导入链经 rfd 选取文件（云端走 /api/import HTTP 流，17.3）；
/// - 7 个 companion_* 命令——桌面宿主专属手机伴侣连接语义；
/// - 3 个 secret_store_* 命令（save/load/delete）——密钥原文读写语义，
///   server 路由即违反「任何 API 响应不含 key 字段」冻结款（评审回环
///   裁决 A：secret_store_load 返回 Option&lt;String&gt; 原文，划归桌面专属；
///   命令体驻 engine 供桌面 wrapper 调用）。
pub const DESKTOP_ONLY_COMMANDS: &[&str] = &[
    "chat_pick_working_directory",
    "skill_pick_custom_directory",
    "pick_import_file",
    "data_export",
    "data_import",
    "pairing_generate_qr",
    "pairing_confirm",
    "paired_device_list",
    "paired_device_remove",
    "companion_get_status",
    "companion_get_relay_addr",
    "companion_set_relay_addr",
    "secret_store_save",
    "secret_store_load",
    "secret_store_delete",
];

/// perf-test feature 门控命令（2 条）：仅在 `--features perf-test` 构建注册，
/// 常规 generate_handler 源码扫描含 cfg 门控条目，对等断言单独并入。
pub const PERF_TEST_GATED_COMMANDS: &[&str] = &["app_emit_test_stream", "app_seed_perf_data"];

/// 查询某命令是否 desktop-only。
pub fn is_desktop_only(command: &str) -> bool {
    DESKTOP_ONLY_COMMANDS.contains(&command)
}

/// 查询某命令是否 perf-test 门控。
pub fn is_perf_test_gated(command: &str) -> bool {
    PERF_TEST_GATED_COMMANDS.contains(&command)
}

/// 查询某命令是否 web-ok（双宿主可用）。
///
/// 二轮评审修复 #9：经 [`capability_of`] 发源（== `Some(Web)`）——
/// 与 server `dispatch_gen::WEB_OK_COMMANDS`（同一 commands.json 工件）
/// 语义一致。
pub fn is_web_command(command: &str) -> bool {
    matches!(capability_of(command), Some(Capability::Web))
}

/// 命令能力类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// 桌面与 server 双宿主可用。
    Web,
    /// 仅桌面壳注册（server 物理不路由，UI 按 capability 隐入口）。
    DesktopOnly,
    /// perf-test feature 门控（仅基准构建）。
    PerfTestGated,
}

/// web-ok 名单（commands.json 工件编译期嵌入 + 首用解析一次）——
/// web-ok 的机械事实源与 server `dispatch_gen::WEB_OK_COMMANDS` 同源。
static WEB_OK_COMMANDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

fn web_ok_commands() -> &'static [String] {
    WEB_OK_COMMANDS.get_or_init(|| {
        let artifact: serde_json::Value =
            serde_json::from_str(include_str!("../commands.json"))
                .expect("commands.json 工件解析失败（生成物损坏？重跑 gen_commands）");
        artifact["commands"]
            .as_array()
            .expect("commands.json 工件缺 commands 数组")
            .iter()
            .map(|c| {
                c["name"]
                    .as_str()
                    .expect("工件条目缺 name（生成物损坏？）")
                    .to_string()
            })
            .collect()
    })
}

/// 查询某命令能力类别；未在任何名单/工件的未知命令返回 `None`。
///
/// 二轮评审修复 #9：`Some(Capability::Web)` 在此发源（经工件名单）——
/// 旧版从不发源 Web，15.5 若按 `capability_of==Web` 门控会拒掉全部
/// web-ok 命令（分裂脑）。三分名单互斥：工件（103）/ desktop-only
/// （15）/ 门控（2），由对等断言钉死与 generate_handler 扫描一致。
pub fn capability_of(command: &str) -> Option<Capability> {
    if is_desktop_only(command) {
        Some(Capability::DesktopOnly)
    } else if is_perf_test_gated(command) {
        Some(Capability::PerfTestGated)
    } else if web_ok_commands().iter().any(|n| n == command) {
        Some(Capability::Web)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 名单值钉：数量与成员一旦漂移，对等断言（generate_handler 扫描 ==
    /// 工件 ∪ 名单）会红——此测试给出可读的第一现场。
    #[test]
    fn desktop_only_list_pins_fifteen_members() {
        assert_eq!(DESKTOP_ONLY_COMMANDS.len(), 15, "desktop-only 名单必须恰 15 条");
        for name in [
            "chat_pick_working_directory",
            "skill_pick_custom_directory",
            "pick_import_file",
            "data_export",
            "data_import",
            "pairing_generate_qr",
            "pairing_confirm",
            "paired_device_list",
            "paired_device_remove",
            "companion_get_status",
            "companion_get_relay_addr",
            "companion_set_relay_addr",
            "secret_store_save",
            "secret_store_load",
            "secret_store_delete",
        ] {
            assert!(is_desktop_only(name), "{} 应为 desktop-only", name);
            assert_eq!(capability_of(name), Some(Capability::DesktopOnly));
        }
    }

    #[test]
    fn perf_test_gated_list_pins_two_members() {
        assert_eq!(PERF_TEST_GATED_COMMANDS.len(), 2);
        assert!(is_perf_test_gated("app_emit_test_stream"));
        assert!(is_perf_test_gated("app_seed_perf_data"));
    }

    /// 值钉（二轮评审修复 #9）：三类名单各发源正确、未知命令 None——
    /// 15.5 按 `capability_of==Web` 门控依赖此语义（旧版从不发源 Web）。
    #[test]
    fn capability_of_pins_all_variants() {
        // web-ok 名字 → Some(Web)（自工件发源）
        assert_eq!(capability_of("task_create"), Some(Capability::Web));
        assert_eq!(capability_of("chat_send_message"), Some(Capability::Web));
        assert!(is_web_command("task_create"));
        assert!(!is_desktop_only("chat_send_message"));
        assert!(!is_perf_test_gated("memory_list"));
        // desktop-only → Some(DesktopOnly)
        assert_eq!(capability_of("data_export"), Some(Capability::DesktopOnly));
        assert!(!is_web_command("data_export"));
        // 门控 → Some(PerfTestGated)
        assert_eq!(
            capability_of("app_emit_test_stream"),
            Some(Capability::PerfTestGated)
        );
        assert!(!is_web_command("app_emit_test_stream"));
        // 未知 → None（不在任何名单/工件）
        assert_eq!(capability_of("no_such_command_xyz"), None);
        assert!(!is_web_command("no_such_command_xyz"));
    }

    /// 评审回环裁决 A 钉子：secret_store_* 三命令划归 desktop-only，
    /// server 物理不路由（密钥原文命令——任何 API 响应不含 key 字段）。
    #[test]
    fn secret_store_commands_are_desktop_only() {
        for name in ["secret_store_save", "secret_store_load", "secret_store_delete"] {
            assert!(is_desktop_only(name), "{} 必须为 desktop-only（裁决 A）", name);
        }
    }
}
