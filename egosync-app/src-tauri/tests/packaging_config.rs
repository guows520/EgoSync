//! 桌面壳打包配置测试（Story 15.1）。
//!
//! 承接自引擎 crate 的 services/sidecar.rs：NSIS 安装前钩子断言读取桌面壳
//! tauri.conf.json，属宿主打包配置（engine 无 tauri.conf.json），随 Story 15.1
//! 迁移归属桌面壳。

/// WHY: 如果安装器不在覆盖 resources/opencode.exe 前停止旧 sidecar，
/// Windows 会因文件被运行中进程锁定而拒绝写入，导致升级失败。
/// 此测试验证 NSIS 安装前清理边界确实存在，而不是仅靠应用启动后端口清理。
#[test]
fn test_nsis_preinstall_hook_configured_and_path_based() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let conf_path = manifest_dir.join("tauri.conf.json");
    let conf = std::fs::read_to_string(&conf_path)
        .unwrap_or_else(|_| panic!("read tauri.conf.json: {}", conf_path.display()));

    let conf_value: serde_json::Value =
        serde_json::from_str(&conf).expect("tauri.conf.json must be valid JSON");

    let hooks_path = conf_value
        .pointer("/bundle/windows/nsis/installerHooks")
        .and_then(|v| v.as_str())
        .expect(
            "tauri.conf.json 必须配置 bundle.windows.nsis.installerHooks，\
             否则安装器不会在覆盖文件前清理旧 sidecar",
        );

    assert!(
        hooks_path.to_lowercase().ends_with(".nsh"),
        "installerHooks 必须指向 .nsh 文件: {}",
        hooks_path
    );

    let hook_file = manifest_dir.join(hooks_path);
    assert!(
        hook_file.exists(),
        "NSIS installer hook 文件必须存在: {}",
        hook_file.display()
    );

    let hook_src = std::fs::read_to_string(&hook_file)
        .unwrap_or_else(|_| panic!("read hook file: {}", hook_file.display()));

    assert!(
        hook_src.contains("NSIS_HOOK_PREINSTALL"),
        "hook 文件必须定义 NSIS_HOOK_PREINSTALL 宏，\
         该宏在 NSIS 复制文件前执行，是安装前清理的入口"
    );

    // 路径匹配而非端口匹配：仅按端口杀进程会误杀用户手动运行的无关 opencode 服务
    assert!(
        hook_src.to_lowercase().contains("executablepath")
            || hook_src.to_lowercase().contains("processpath")
            || hook_src.contains("Win32_Process"),
        "hook 必须按完整映像路径识别 EgoSync 进程，\
         不能仅按进程名或端口杀进程（会误杀同名无关进程）"
    );
}
