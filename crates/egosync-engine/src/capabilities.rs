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

/// 查询某命令能力类别；未知命令返回 `None`。
///
/// 注意：web-ok 的判定以 commands.json 工件为准（机械导出），本函数
/// 只覆盖 desktop-only / 门控名单——`Some(Capability::Web)` 不在此发源。
pub fn capability_of(command: &str) -> Option<Capability> {
    if is_desktop_only(command) {
        Some(Capability::DesktopOnly)
    } else if is_perf_test_gated(command) {
        Some(Capability::PerfTestGated)
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

    /// web-ok 命令不在任何门控名单（返回 None —— 能力类别由工件发源）。
    #[test]
    fn web_ok_commands_are_not_gated() {
        assert_eq!(capability_of("task_create"), None);
        assert_eq!(capability_of("chat_send_message"), None);
        assert!(!is_desktop_only("chat_send_message"));
        assert!(!is_perf_test_gated("memory_list"));
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
