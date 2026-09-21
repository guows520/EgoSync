//! 桌面模式壳命令（Story 16.3，FR-48）：双模式常驻注册，零引擎 state
//! 依赖——远程模式（引擎装配整体跳过、state 未 manage）下照常可调用。
//!
//! 三命令（切换 = 重启式裁决的落点）：
//! - [`desktop_get_boot_config`]：前端引导期读取（mode/remoteUrl/
//!   remoteToken——远程态才读 keyring，本地引导零 keyring I/O）；
//! - [`remote_mode_save_config`]：持久化模式配置（mode 文件 + keyring
//!   令牌——NFR-C5：切换路径只触碰模式文件、keyring，无任何数据
//!   迁移/合并/同步面）；
//! - [`remote_mode_restart`]：显式退出清理（watchdog cancel +
//!   sidecar.stop，幂等）后 `app.restart()` 进新模式——重启前无论
//!   `RunEvent::Exit` 是否触发，清理已先执行（8.6 幂等保证）。
//!
//! 这些是**宿主壳命令**：前端经 `@tauri-apps/api/core` 直连 invoke
//! （不经 transport 通道——远程模式下业务 invoke 走远端 HTTP，壳命令
//! 必须始终走本机 Tauri IPC）。
//!
//! [T7 修订]：三命令全部 async 化——同步命令在 Tauri 2 于主线程执行，
//! 文件 + keyring I/O 会阻塞引导关键路径（渲染前 desktop_get_boot_config
//! 的往返即首屏延迟）；async 命令在异步运行时任务上执行，I/O 移出主线程。

use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::services::desktop_mode;
use crate::services::sidecar::SidecarManager;

use std::sync::Arc;

/// 引导配置（camelCase——与前端 `DesktopBootConfig` 同形契约）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBootConfig {
    /// `"local"` | `"remote"`。
    pub mode: String,
    /// 远程实例 base URL（模式文件持久值；local 态保留作预填）。
    pub remote_url: Option<String>,
    /// 远程实例令牌（**仅 remote 态读 keyring**——本地引导零 keyring
    /// I/O，本地设置 tab 不回显令牌本体）。
    pub remote_token: Option<String>,
}

/// 读引导配置：mode + remoteUrl（来自模式文件）+ remoteToken（仅远程态）。
///
/// 文件缺失/损坏 ⇒ local + 空配置（fail-safe——与进程引导同语义）。
/// [T2 修订] 裁决与 `read_mode` 同源（`resolve_mode`）：mode=remote 而
/// remoteUrl 缺失/空白 ⇒ 按 local 返回——前端 boot 注入与进程引导恒一致，
/// 杜绝「Rust 进远程 builder / 前端回退 TauriTransport 直通」的砖死会话。
#[tauri::command]
pub async fn desktop_get_boot_config(app: AppHandle) -> Result<DesktopBootConfig, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::SidecarError(format!("获取应用数据目录失败: {}", e)))?;
    let file = desktop_mode::read_mode_file(&app_data_dir);
    let mode = file
        .as_ref()
        .map(desktop_mode::resolve_mode)
        .unwrap_or(desktop_mode::DesktopMode::Local);

    match mode {
        desktop_mode::DesktopMode::Remote => {
            // [评审轮2 U23] 令牌臂裁决抽 `resolve_boot_token` 纯函数
            //（services 层直测——本地态零 keyring I/O / 出错 fail-safe
            // 的冻结款此前无测试；此处只传加载器）
            let remote_token =
                desktop_mode::resolve_boot_token(mode, desktop_mode::load_remote_token);
            Ok(DesktopBootConfig {
                mode: "remote".to_string(),
                remote_url: file.and_then(|f| f.remote_url),
                remote_token,
            })
        }
        desktop_mode::DesktopMode::Local => Ok(DesktopBootConfig {
            mode: "local".to_string(),
            remote_url: file.and_then(|f| f.remote_url),
            remote_token: None,
        }),
    }
}

/// 持久化模式配置（模式文件 + keyring 令牌）。
///
/// - `mode="remote"`：`remote_url` 必填且须 http/https；`token` 提供即
///   写 keyring（切换前的「测试连接通过」由前端守卫——本命令不重验，
///   重验由前端 boot 后 getAuthStatus 承担）；
/// - `mode="local"`：`remote_url` 可保留（下次切换预填）；keyring 令牌
///   **保留不清除**（用户自己的钥匙串——切回远程免重录；无安全恶化）。
///
/// [T12 修订] 校验与归一抽 `validate_save_request` 纯函数（services 层
/// 单测直测——未知 mode/空令牌/URL 形态/local 保留 URL 四分支）；命令层
/// 只做参数解析 → 调 Service → 落盘（Command 层业务逻辑禁令）。
#[tauri::command]
pub async fn remote_mode_save_config(
    app: AppHandle,
    mode: String,
    remote_url: Option<String>,
    token: Option<String>,
) -> Result<(), AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::SidecarError(format!("获取应用数据目录失败: {}", e)))?;

    let (file, token_to_save) = desktop_mode::validate_save_request(
        &mode,
        remote_url.as_deref(),
        token.as_deref(),
    )?;

    if let Some(token) = token_to_save {
        desktop_mode::save_remote_token(&token)?;
    }
    desktop_mode::write_mode_file(&app_data_dir, &file)?;
    Ok(())
}

/// 重启进新模式：显式退出清理（既有 `RunEvent::Exit` 路径同款——
/// watchdog cancel + sidecar.stop）后 `app.restart()`。
///
/// 幂等清理：本地态两处清理（此处 + 可能的 Exit 事件）叠加无害；
/// 远程态 state 未 manage——`try_state` 探测跳过（绝不 panic）。
/// `restart()` 不返回（进程退出重启）——`Ok(())` 仅为类型收口。
#[tauri::command]
pub async fn remote_mode_restart(app: AppHandle) -> Result<(), AppError> {
    // watchdog / 委派监听 / 调度器取消（未 manage ⇒ 远程态零清理面）
    if let Some(cancel) = app.try_state::<CancellationToken>() {
        cancel.cancel();
    }
    // sidecar 停机（未 manage ⇒ 远程态跳过）
    if let Some(sidecar) = app.try_state::<Arc<Mutex<SidecarManager>>>() {
        if let Err(e) = sidecar.lock().await.stop().await {
            tracing::warn!("Failed to stop opencode sidecar cleanly: {}", e);
        }
    }
    tracing::info!("模式切换：重启应用（desktop-mode.json 已先行持久化）");
    app.restart();
}
