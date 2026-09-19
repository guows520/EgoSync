// Story 15.5：TauriTransport —— 桌面宿主实现（现状基线，行为逐字节一致）。
//
// - invoke：逐字节透传 @tauri-apps/api/core 的 invoke；
// - on：把 listen 的 Promise<unlisten> 异步语义包成同步 unlisten，静默容错
//   与卸载竞态防护原样平移（useTauriEvent 现状语义，见各注释）；
// - 连接状态恒 online：onConnectionStateChange 初始回调一次后永不变化；
// - emitFrontendEvent：走 @tauri-apps/api/event 的 emit（skill-scope-updated
//   前端→前端事件桌面原样）。

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { emit as tauriEmit, listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { TRANSPORT_CAPABILITIES } from './capabilities';
import type { ConnectionState, Transport } from './types';

export class TauriTransport implements Transport {
  readonly capabilities = TRANSPORT_CAPABILITIES;

  /** 逐字节透传（cmd + 可选参数原样转发，类型参数原样保留）。 */
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return tauriInvoke<T>(command, args);
  }

  /**
   * 事件订阅：listen 异步语义平移为同步 unlisten。
   *
   * 与 useTauriEvent 现状逐语义一致：
   * - listen Promise 静默容错（.catch(() => {})——事件插件不可用不炸宿主）；
   * - 已卸载后 listen 才 resolve：立即调用 fn() 释放（防泄漏）；
   * - unlisten 调用容错（Promise.resolve(...).catch(() => {})）；
   * - 回调门控：disposed 后到达的事件不再投递（对应旧 hook 的 mounted 门）。
   */
  on<T>(event: string, handler: (payload: T) => void): UnlistenFn {
    let unlisten: UnlistenFn | null = null;
    let disposed = false;

    listen<T>(event, (e) => {
      if (!disposed) {
        handler(e.payload);
      }
    }).then((fn) => {
      if (!disposed) {
        unlisten = fn;
      } else {
        // 卸载竞态：订阅方已离开，立即释放监听（防泄漏）
        Promise.resolve(fn()).catch(() => {});
      }
    }).catch(() => {});

    return () => {
      disposed = true;
      Promise.resolve(unlisten?.()).catch(() => {});
    };
  }

  /** 桌面宿主与引擎同进程：恒 online（初始回调一次后永不变化）。 */
  onConnectionStateChange(handler: (state: ConnectionState) => void): UnlistenFn {
    handler('online');
    return () => {};
  }

  /**
   * 前端本地事件：桌面走 event 插件 emit 原样（ChatStream listen 同源）。
   * 直返 emit 的 Promise——与基线 `emit('skill-scope-updated', payload)`
   * 同形，调用方 catch 兜底可达（评审 G1：void 丢弃曾令兜底成死代码）。
   */
  emitFrontendEvent(event: string, payload?: unknown): Promise<void> {
    return tauriEmit(event, payload);
  }
}
