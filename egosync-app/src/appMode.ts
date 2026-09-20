// Story 16.3：模块级桌面模式态（FR-48 桌面客户端远程模式）。
//
// 模式在**进程生命周期内恒定**（切换必经重启——架构裁决，见 spec
// Boundaries「切换方式裁决」）：无 context/provider/热换，模块级单值即
// 全部状态。main.tsx 渲染前经 desktop_get_boot_config 壳命令注入；此后
// 全进程只读。
//
// 消费面（UI 门控）：desktop-only 业务能力按 `mode==='local'` 门控
// （远程态桌面 = 浏览器等价物——能力隐藏同浏览器分支）；连接状态初始值
// 按模式诚实给定（remote ⇒ connecting）。

import { isTauriHost } from './transport';
import type { DesktopBootConfig } from './transport';

/** 桌面模式（'browser' = 浏览器宿主——无桌面语义）。 */
export type DesktopMode = 'local' | 'remote' | 'browser';

// 缺省按宿主推导：Tauri 宿主 = 'local'（desktop-mode.json 缺失 fail-safe
// 同语义——回退到有本地数据的一侧；测试默认桩零注入即 local，既有桌面
// 分支测试零回归）；浏览器宿主 = 'browser'。
let desktopMode: DesktopMode = isTauriHost() ? 'local' : 'browser';

/** 引导期注入（main.tsx 渲染前调用一次；重复调用静默忽略——模式恒定）。 */
export function setDesktopMode(mode: DesktopMode): void {
  desktopMode = mode;
}

/** 读当前模式（UI 门控与连接状态消费）。 */
export function getDesktopMode(): DesktopMode {
  return desktopMode;
}

/**
 * 本地桌面判定（门控便捷谓词）：Tauri 宿主且 local 模式。
 *
 * desktop-only 命令面门控 `!isDesktopOnly(cmd) || isLocalDesktop()`：
 * - 浏览器宿主 ⇒ false（desktop-only 隐藏——既有语义）；
 * - 远程桌面 ⇒ false（desktop-only 隐藏——浏览器等价物，16.3 新增语义）；
 * - 本地桌面 ⇒ true（desktop-only 可见——既有语义零回归）。
 */
export function isLocalDesktop(): boolean {
  return isTauriHost() && desktopMode === 'local';
}

/**
 * 远程桌面判定：Tauri 宿主且 remote 模式（桌面壳仍在 + 远端实例客户端）。
 */
export function isRemoteDesktop(): boolean {
  return isTauriHost() && desktopMode === 'remote';
}

/** 由引导配置注入模式态（main.tsx 消费——setDesktopMode + setTransportBoot 的联动收口）。 */
export function applyBootConfig(config: DesktopBootConfig): void {
  setDesktopMode(config.mode === 'remote' ? 'remote' : isTauriHost() ? 'local' : 'browser');
}

/** 测试专用：复位模式态为宿主推导缺省（跨用例隔离——与 __resetTransportForTests 配对）。 */
export function __resetDesktopModeForTests(): void {
  desktopMode = isTauriHost() ? 'local' : 'browser';
}
