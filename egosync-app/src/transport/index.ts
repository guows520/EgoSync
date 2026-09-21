// Story 15.5：传输抽象入口——宿主探测 + 懒单例 + 模块级便捷再导出。
//
// services 层经 `import { invoke } from '@/transport'` 与宿主解耦：组件/
// service 代码零感知双宿主（单一构建产物双宿主复用，不做双构建）。
//
// 宿主探测：`window.__TAURI_INTERNALS__` 存在 ⇒ TauriTransport（桌面），
// 不存在 ⇒ HttpTransport（浏览器）；测试环境经 test-setup 的全局桩默认
// 走 Tauri 分支。
//
// Story 16.3 三路解析（桌面远程模式）：
// - 浏览器宿主 ⇒ HttpTransport()（相对路径——既有行为）；
// - 桌面 local ⇒ TauriTransport（本地引擎——既有行为）；
// - 桌面 remote ⇒ HttpTransport({kind:'remote', baseUrl, token})（绝对
//   URL + Bearer + SSE 票据——远端实例客户端，语义与浏览器等价）。
//
// 模式在进程生命周期内恒定（切换必经重启）：setTransportBoot 在渲染前
// 由 main.tsx 引导期注入一次，之后只读。测试默认 boot=local
// （test-setup 桩不变）。

import { HttpTransport } from './http';
import { TauriTransport } from './tauri';
import type { AuthStatus, DesktopBootConfig, Transport } from './types';

export type {
  AuthStatus,
  ConnectionState,
  DesktopBootConfig,
  Transport,
  TransportCapabilities,
  UnlistenFn,
} from './types';
export { HttpTransportError } from './types';
export { FRONTEND_LOCAL_EVENTS, isFrontendLocalEvent } from './localEvents';
export type { TransportReconnectedPayload } from './localEvents';
export { TRANSPORT_CAPABILITIES, isDesktopOnly, isWebCommand } from './capabilities';

/** Tauri 宿主探测（WebView 注入 `__TAURI_INTERNALS__`）。 */
export function isTauriHost(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * 进程内模式态（Story 16.3）：渲染前引导期注入，之后恒定只读。
 *
 * - `null`（缺省）= 未注入引导——按纯浏览器宿主解析（浏览器/测试语义；
 *   Tauri 宿主未注入即 local——desktop-mode.json 缺失 fail-safe 同语义）；
 * - `mode:'remote'` ⇒ 桌面远程模式（HttpTransport 指向远端）；
 * - `mode:'local'` ⇒ 桌面本地模式（TauriTransport）。
 */
let boot: DesktopBootConfig | null = null;

/**
 * 注入引导配置（Story 16.3）——main.tsx 渲染前调用（Tauri 宿主经
 * desktop_get_boot_config 壳命令取得；浏览器宿主零调用）。
 *
 * 进程生命周期内只调用一次（模式恒定；重复注入视为程序错误——静默
 * 忽略首值之外的后续注入，防热换传输违背冻结款）。
 */
export function setTransportBoot(config: DesktopBootConfig): void {
  if (boot !== null) {
    console.error('setTransportBoot 重复调用被忽略（模式进程内恒定——切换必经重启）');
    return;
  }
  boot = config;
}

/** 读当前引导配置（测试与 UI 消费；未注入 ⇒ null）。 */
export function getTransportBoot(): DesktopBootConfig | null {
  return boot;
}

// [T4 修订] 桌面模式的 getDesktopMode 重复导出已撤销——appMode.ts 为
// 模式态单一事实源（消费面统一 `@/appMode` 的 getDesktopMode/
// isLocalDesktop/isRemoteDesktop；transport 侧仅保留 boot 记录供传输
// 解析与 remoteUrl 预填消费）。防模式双源漂移：未来调用点漏配一半即
// 门控与传输不一致。
let instance: Transport | null = null;

/**
 * 传输懒单例：首个调用时按「引导模式 × 宿主探测」三路选定实现，进程内
 * 复用（模式恒定——单例与模式同生命周期）。
 */
export function getTransport(): Transport {
  if (!instance) {
    if (isTauriHost()) {
      // 桌面宿主：引导模式裁决（remote = HttpTransport 指向远端实例）。
      // remote 态缺令牌仍构造远程实例（空令牌 ⇒ Bearer 判定 authenticated:
      // false ⇒ AuthGate 进令牌重录——**不得**回退 TauriTransport：其
      // getAuthStatus 直通 authenticated:true 会带坏传输直进应用）；
      // 缺 URL（模式文件损坏态）无远端可指——回退本地（fail-safe 有
      // 数据的一侧，desktop-mode.json 损坏同语义）。
      instance =
        boot?.mode === 'remote' && boot.remoteUrl
          ? new HttpTransport({
              kind: 'remote',
              baseUrl: boot.remoteUrl,
              token: boot.remoteToken ?? '',
            })
          : new TauriTransport();
    } else {
      instance = new HttpTransport();
    }
  }
  return instance;
}

/**
 * 认证态发现（Story 16.1）：浏览器宿主经 HttpTransport 缓存通道
 * （限流预算保护——见 http.ts getAuthStatus）；Tauri 宿主直通值
 * （本地引擎无认证面；AuthGate 在此之前已直通，本分支为防御性兜底）。
 *
 * Story 16.3：桌面 remote 同走 HttpTransport 缓存通道（Bearer 判定）；
 * 直通仅 desktop-local。
 */
export async function getAuthStatus(): Promise<AuthStatus> {
  const transport = getTransport();
  if (transport instanceof HttpTransport) {
    return transport.getAuthStatus();
  }
  return { setupRequired: false, authenticated: true };
}

/**
 * 认证态缓存失效（Story 16.1）：登录/setup 成功与登出后调用（15-5 G11
 * 遗嘱）。Tauri 宿主无缓存（直通值恒定）——no-op。
 */
export function invalidateAuthStatusCache(): void {
  const transport = getTransport();
  if (transport instanceof HttpTransport) {
    transport.invalidateAuthStatusCache();
  }
}

/**
 * 远程令牌更新（Story 16.3 令牌重录路径）：验证通过并写入 keyring 后
 * 调用——传输实例**就地换令牌**（不重建：AuthGate 的 auth:unauthorized
 * 订阅挂在既有实例上，重建会割裂订阅面）+ 认证缓存失效。
 *
 * 仅远程桌面（HttpTransport remote 实例）有意义；其他宿主 no-op。
 * 同步 boot 记录（getTransportBoot 消费面一致）。
 */
export function updateRemoteToken(token: string): void {
  if (boot?.mode === 'remote') {
    boot = { ...boot, remoteToken: token };
  }
  const transport = getTransport();
  if (transport instanceof HttpTransport) {
    transport.updateToken(token);
  }
}

/** 模块级 invoke 再导出（services 层唯一入口，签名与 @tauri-apps/api/core 同形）。 */
export function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return getTransport().invoke<T>(command, args);
}

/**
 * 前端本地事件发射（skill-scope-updated 等）：按宿主路由——Tauri 走
 * event 插件 emit 原样（rejection 透传给调用方 catch），浏览器走进程内
 * 总线（不进 SSE 契约、恒 resolve）。
 */
export function emitFrontendEvent(event: string, payload?: unknown): Promise<void> {
  return getTransport().emitFrontendEvent(event, payload);
}

/**
 * 测试专用：重置传输单例（跨宿主探测切换的测试隔离）。
 * 生产代码不得调用——传输是应用级单例。
 */
export function __resetTransportForTests(): void {
  instance = null;
  boot = null;
}
