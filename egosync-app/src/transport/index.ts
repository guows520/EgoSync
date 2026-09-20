// Story 15.5：传输抽象入口——宿主探测 + 懒单例 + 模块级便捷再导出。
//
// services 层经 `import { invoke } from '@/transport'` 与宿主解耦：组件/
// service 代码零感知双宿主（单一构建产物双宿主复用，不做双构建）。
//
// 宿主探测：`window.__TAURI_INTERNALS__` 存在 ⇒ TauriTransport（桌面），
// 不存在 ⇒ HttpTransport（浏览器）；测试环境经 test-setup 的全局桩默认
// 走 Tauri 分支。

import { HttpTransport } from './http';
import { TauriTransport } from './tauri';
import type { AuthStatus, Transport } from './types';

export type {
  AuthStatus,
  ConnectionState,
  Transport,
  TransportCapabilities,
  UnlistenFn,
} from './types';
export { HttpTransportError } from './types';
export { FRONTEND_LOCAL_EVENTS, isFrontendLocalEvent } from './localEvents';
export { TRANSPORT_CAPABILITIES, isDesktopOnly, isWebCommand } from './capabilities';

/** Tauri 宿主探测（WebView 注入 `__TAURI_INTERNALS__`）。 */
export function isTauriHost(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

let instance: Transport | null = null;

/** 传输懒单例：首个调用时按宿主探测选定实现，进程内复用。 */
export function getTransport(): Transport {
  if (!instance) {
    instance = isTauriHost() ? new TauriTransport() : new HttpTransport();
  }
  return instance;
}

/**
 * 认证态发现（Story 16.1）：浏览器宿主经 HttpTransport 缓存通道
 * （限流预算保护——见 http.ts getAuthStatus）；Tauri 宿主直通值
 * （本地引擎无认证面；AuthGate 在此之前已直通，本分支为防御性兜底）。
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
}
