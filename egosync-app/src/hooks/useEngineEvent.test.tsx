// Story 15.5 评审 G4：useEngineEvent 真实行为钉。
//
// 迁移面 22 个消费点的测试全部 mock 本 hook——此前删 deps 展开或清理
// 函数不会红任何测试，核心「桌面行为零变化」保证无测试锚。本文件以
// 可控假 transport 驱动真 hook：订阅于挂载、卸载即退订、deps 变更重
// 订阅（handler 闭包刷新）、payload 透传。
//
// 事件名取自生成物（零手写——对等面守门同款纪律）。

import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const fakeOn = vi.hoisted(() => vi.fn());
const fakeGetTransport = vi.hoisted(() => vi.fn());
vi.mock('@/transport', () => ({
  getTransport: fakeGetTransport,
}));

import { ENGINE_EVENT_NAMES } from '../transport/events';
import { useEngineEvent } from './useEngineEvent';

/** 驱动用事件名（生成物机械取用，零手写）。 */
const SAMPLE_EVENT = ENGINE_EVENT_NAMES[0]!;
const STREAM_EVENT = ENGINE_EVENT_NAMES.find((n) => n === 'llm:stream') ?? ENGINE_EVENT_NAMES[1]!;

type Handler = (payload: unknown) => void;

describe('useEngineEvent', () => {
  let unlisten: ReturnType<typeof vi.fn>;

  /** 抓取最近一次订阅的 handler 并模拟一次事件投递。 */
  function deliver(payload: unknown) {
    const calls = fakeOn.mock.calls as unknown as Array<[string, Handler]>;
    const handler = calls[calls.length - 1]![1];
    handler(payload);
  }

  beforeEach(() => {
    unlisten = vi.fn();
    fakeOn.mockReset().mockReturnValue(unlisten);
    fakeGetTransport.mockReset().mockReturnValue({ on: fakeOn });
  });

  it('挂载即经 getTransport().on 订阅（事件名 + handler 原样透传）', () => {
    const handler = vi.fn();
    renderHook(() => useEngineEvent(SAMPLE_EVENT, handler));
    expect(fakeOn).toHaveBeenCalledTimes(1);
    expect(fakeOn).toHaveBeenCalledWith(SAMPLE_EVENT, handler);
  });

  it('payload 透传给 handler（事件投递穿 hook 到消费方）', () => {
    const handler = vi.fn();
    renderHook(() => useEngineEvent(STREAM_EVENT, handler));
    deliver({ token: '你', done: false });
    expect(handler).toHaveBeenCalledWith({ token: '你', done: false });
  });

  it('卸载即退订（unlisten 调用恰一次）', () => {
    const handler = vi.fn();
    const { unmount } = renderHook(() => useEngineEvent(SAMPLE_EVENT, handler));
    expect(unlisten).not.toHaveBeenCalled();
    unmount();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('deps 变更 ⇒ 旧订阅退订、新订阅建立（handler 闭包刷新——ChatStream 切会话即此路径）', () => {
    const handler = vi.fn();
    const { rerender, unmount } = renderHook(
      ({ id }) => useEngineEvent(STREAM_EVENT, handler, [id]),
      { initialProps: { id: 'conv-1' } }
    );
    expect(fakeOn).toHaveBeenCalledTimes(1);
    rerender({ id: 'conv-2' });
    expect(unlisten).toHaveBeenCalledTimes(1); // 旧订阅退订
    expect(fakeOn).toHaveBeenCalledTimes(2); // 新订阅建立
    // 新闭包生效：投递走最新一次订阅的 handler
    deliver({ token: 'x', done: true });
    expect(handler).toHaveBeenCalledWith({ token: 'x', done: true });
    unmount();
    expect(unlisten).toHaveBeenCalledTimes(2);
  });

  it('deps 不变 ⇒ 不重订阅（effect 稳定，零重复投递）', () => {
    const handler = vi.fn();
    const { rerender } = renderHook(
      ({ id }) => useEngineEvent(SAMPLE_EVENT, handler, [id]),
      { initialProps: { id: 'same' } }
    );
    rerender({ id: 'same' });
    expect(fakeOn).toHaveBeenCalledTimes(1);
    expect(unlisten).not.toHaveBeenCalled();
  });
});
