// Story 16.2：useConnectionState 三态矩阵——真实行为钉。
//
// 契约（http.ts/tauri.ts 既有实现）：
// - 订阅即回调当前状态（HttpTransport 初始 connecting / 恢复后 online；
//   TauriTransport 恒 online）；
// - 此后每次迁移回调新状态；
// - 卸载即撤订阅（此后的迁移不再投递）。
//
// 测试以可控假 transport 驱动真 hook，不依赖宿主探测。

import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectionState, UnlistenFn } from '@/transport';

const fakeOnConnectionStateChange = vi.hoisted(() => vi.fn());
const fakeGetTransport = vi.hoisted(() => vi.fn());
const fakeIsTauriHost = vi.hoisted(() => vi.fn());
vi.mock('@/transport', () => ({
  getTransport: fakeGetTransport,
  isTauriHost: fakeIsTauriHost,
}));

import { useConnectionState } from './useConnectionState';

describe('useConnectionState（三态矩阵）', () => {
  let handlers: Array<(state: ConnectionState) => void>;

  /** 模拟传输层状态迁移（向全部订阅者扇出）。 */
  function transition(state: ConnectionState) {
    for (const handler of [...handlers]) {
      handler(state);
    }
  }

  beforeEach(() => {
    handlers = [];
    fakeIsTauriHost.mockReset().mockReturnValue(false);
    fakeOnConnectionStateChange.mockReset().mockImplementation(
      (handler: (state: ConnectionState) => void): UnlistenFn => {
        handlers.push(handler);
        return () => {
          handlers = handlers.filter(h => h !== handler);
        };
      },
    );
    fakeGetTransport.mockReset().mockReturnValue({
      onConnectionStateChange: fakeOnConnectionStateChange,
    });
  });

  it('订阅即回调当前状态（连接中）', () => {
    fakeOnConnectionStateChange.mockImplementationOnce(
      (handler: (state: ConnectionState) => void) => {
        handler('connecting');
        handlers.push(handler);
        return () => {};
      },
    );
    const { result } = renderHook(() => useConnectionState());
    expect(result.current).toBe('connecting');
  });

  it('connecting → online → reconnecting → online 全迁移如实呈现', () => {
    const { result } = renderHook(() => useConnectionState());
    // 浏览器宿主的诚实初始态：连接尚未建立 → connecting（订阅回调前）
    expect(result.current).toBe('connecting');

    act(() => transition('connecting'));
    expect(result.current).toBe('connecting');

    act(() => transition('online'));
    expect(result.current).toBe('online');

    act(() => transition('reconnecting'));
    expect(result.current).toBe('reconnecting');

    act(() => transition('online'));
    expect(result.current).toBe('online');
  });

  it('Tauri 宿主初始即 online（传输恒在线的桌面语义）', () => {
    fakeIsTauriHost.mockReturnValue(true);
    const { result } = renderHook(() => useConnectionState());
    expect(result.current).toBe('online');
  });

  it('卸载即撤订阅（此后的迁移不再投递）', () => {
    const { result, unmount } = renderHook(() => useConnectionState());
    act(() => transition('reconnecting'));
    expect(result.current).toBe('reconnecting');

    unmount();
    act(() => transition('online'));
    // 卸载后不更新（不抛错、不残留）
    expect(result.current).toBe('reconnecting');
  });

  it('多实例独立订阅（各自收到同一迁移）', () => {
    const first = renderHook(() => useConnectionState());
    const second = renderHook(() => useConnectionState());
    expect(handlers.length).toBe(2);

    act(() => transition('reconnecting'));
    expect(first.result.current).toBe('reconnecting');
    expect(second.result.current).toBe('reconnecting');
  });
});
