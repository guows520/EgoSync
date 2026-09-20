// Story 16.3：桌面模式壳命令服务——直连 Tauri IPC（不经 transport 通道）。
//
// 三命令（Rust `commands/desktop_mode.rs` 同形契约）：
// - `desktop_get_boot_config`：引导期读取（mode/remoteUrl/remoteToken）；
// - `remote_mode_save_config`：持久化模式配置（desktop-mode.json + keyring）；
// - `remote_mode_restart`：显式退出清理后 `app.restart()` 进新模式。
//
// **必须直连 `@tauri-apps/api/core`**：远程模式下业务 invoke 经
// HttpTransport 走远端 HTTP——远端无桌面壳语境（模式配置是本机事实），
// 壳命令走本机 Tauri IPC 是唯一可达路径（且本地模式同样可用——双模式
// 常驻注册）。本服务仅桌面宿主调用（main.tsx 引导 / 设置 tab 切换流程）。

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { DesktopBootConfig } from '@/transport';

/** 引导配置读取（缺失/损坏 ⇒ mode:'local'——Rust 侧 fail-safe 同语义）。 */
export async function desktopGetBootConfig(): Promise<DesktopBootConfig> {
  return tauriInvoke<DesktopBootConfig>('desktop_get_boot_config');
}

/** 模式配置持久化（mode 文件 + keyring 令牌——NFR-C5：仅此两处落盘面）。 */
export async function remoteModeSaveConfig(params: {
  mode: 'local' | 'remote';
  remoteUrl?: string | null;
  token?: string | null;
}): Promise<void> {
  await tauriInvoke('remote_mode_save_config', {
    mode: params.mode,
    remoteUrl: params.remoteUrl ?? null,
    token: params.token ?? null,
  });
}

/** 重启进新模式（显式退出清理后 app.restart()——调用后进程退出，Promise 实际不返回）。 */
export async function remoteModeRestart(): Promise<void> {
  await tauriInvoke('remote_mode_restart');
}
