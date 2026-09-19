// Story 15.5：TauriTransport 单元钉语义——与 useTauriEvent 现状逐语义一致。
//
// 覆盖：invoke 透传 / 同步 unlisten / mounted 竞态（卸载后 listen 才
// resolve ⇒ 立即释放）/ dispose 后事件不再投递 / listen 静默容错 /
// 连接状态恒 online 初始回调一次 / emit 透传 / 宿主探测与懒单例。

import { describe, expect, it, vi, beforeEach } from 'vitest';

// @tauri-apps/api 模块桩（vi.hoisted 供提升后的 mock 工厂引用）
const coreInvoke = vi.hoisted(() => vi.fn());
const eventListen = vi.hoisted(() => vi.fn());
const eventEmit = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: coreInvoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: eventListen,
  emit: eventEmit,
}));

import { __resetTransportForTests, getTransport, isTauriHost } from '.';
import { HttpTransport } from './http';
import { TauriTransport } from './tauri';

describe('TauriTransport', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    __resetTransportForTests();
  });

  it('invoke 逐字节透传 @tauri-apps/api/core（参数与返回值原样）', async () => {
    const value = { id: 'r1', name: '产品' };
    coreInvoke.mockResolvedValue(value);
    const transport = new TauriTransport();
    const result = await transport.invoke('role_list', { roleId: 'r1' });
    expect(result).toBe(value);
    expect(coreInvoke).toHaveBeenCalledTimes(1);
    expect(coreInvoke).toHaveBeenCalledWith('role_list', { roleId: 'r1' });
  });

  it('invoke 无参数调用形态原样透传（args undefined）', async () => {
    coreInvoke.mockResolvedValue([]);
    await new TauriTransport().invoke('role_list');
    expect(coreInvoke).toHaveBeenCalledWith('role_list', undefined);
  });

  it('on 返回同步 unlisten（listen 未 resolve 前即可调用）', () => {
    eventListen.mockReturnValue(new Promise(() => {})); // 永不 resolve
    const transport = new TauriTransport();
    const unlisten = transport.on('notification:new', () => {});
    expect(typeof unlisten).toBe('function');
    expect(() => unlisten()).not.toThrow();
  });

  it('mounted 竞态：卸载后 listen 才 resolve ⇒ fn 立即释放（防泄漏）', async () => {
    let resolveListen: (fn: () => void) => void = () => {};
    eventListen.mockReturnValue(
      new Promise<(fn: () => void) => void>(resolve => {
        resolveListen = resolve;
      })
    );
    const transport = new TauriTransport();
    const unlisten = transport.on('q2:reminder', () => {});
    unlisten(); // 卸载在 listen resolve 之前
    const release = vi.fn();
    resolveListen(release);
    await Promise.resolve(); // 微任务冲洗
    await Promise.resolve();
    expect(release).toHaveBeenCalledTimes(1);
  });

  it('dispose 后到达的事件不再投递（旧 hook 的 mounted 门语义）', async () => {
    let rawCallback: ((e: { payload: unknown }) => void) | null = null;
    eventListen.mockImplementation(
      (_event: string, cb: (e: { payload: unknown }) => void) => {
        rawCallback = cb;
        return Promise.resolve(() => {});
      }
    );
    const handler = vi.fn();
    const transport = new TauriTransport();
    const unlisten = transport.on('llm:stream', handler);
    await Promise.resolve();
    unlisten();
    rawCallback!({ payload: { token: 'x' } });
    expect(handler).not.toHaveBeenCalled();
  });

  it('listen 订阅失败静默容错（不炸宿主）', async () => {
    eventListen.mockRejectedValue(new Error('plugin unavailable'));
    const handler = vi.fn();
    const transport = new TauriTransport();
    const unlisten = transport.on('notification:new', handler);
    await Promise.resolve();
    await Promise.resolve();
    expect(handler).not.toHaveBeenCalled();
    expect(() => unlisten()).not.toThrow();
  });

  it('连接状态恒 online：onConnectionStateChange 初始回调一次后永不变化', () => {
    const transport = new TauriTransport();
    const states: string[] = [];
    const unlisten = transport.onConnectionStateChange(state => states.push(state));
    expect(states).toEqual(['online']);
    unlisten();
    expect(states).toEqual(['online']);
  });

  it('emitFrontendEvent 走 @tauri-apps/api/event emit 透传', async () => {
    const payload = { roleId: 'r1', skillId: 's1' };
    const transport = new TauriTransport();
    await transport.emitFrontendEvent('skill-scope-updated', payload);
    expect(eventEmit).toHaveBeenCalledWith('skill-scope-updated', payload);
  });

  it('emitFrontendEvent 直返 emit 的 Promise：rejection 透传给调用方 catch（基线语义——评审 G1）', async () => {
    eventEmit.mockRejectedValueOnce(new Error('event plugin unavailable'));
    const transport = new TauriTransport();
    await expect(
      transport.emitFrontendEvent('skill-scope-updated', { roleId: 'r1' })
    ).rejects.toThrow('event plugin unavailable');
  });
});

describe('宿主探测与懒单例（getTransport）', () => {
  beforeEach(() => {
    __resetTransportForTests();
  });

  it('__TAURI_INTERNALS__ 存在（test-setup 全局桩）⇒ TauriTransport，且单例复用', () => {
    expect(isTauriHost()).toBe(true);
    const first = getTransport();
    expect(first).toBeInstanceOf(TauriTransport);
    expect(getTransport()).toBe(first);
  });

  it('__TAURI_INTERNALS__ 缺失 ⇒ HttpTransport（懒单例按宿主选定）', () => {
    const w = window as unknown as Record<string, unknown>;
    const internals = w.__TAURI_INTERNALS__;
    delete w.__TAURI_INTERNALS__;
    try {
      expect(isTauriHost()).toBe(false);
      const transport = getTransport();
      expect(transport).toBeInstanceOf(HttpTransport);
      expect(getTransport()).toBe(transport);
    } finally {
      w.__TAURI_INTERNALS__ = internals;
      __resetTransportForTests();
    }
  });
});
