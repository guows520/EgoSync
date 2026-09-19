// Story 15.5：事件面三集合同源断言 + 逐事件名双通道路由测试。
//
// 三集合（任何一侧漂移即红）：
// 1. events.rs 解析集——经生成器导出的单一解析器（scripts/gen-transport.mjs）
//    运行时解析 crates/egosync-engine/src/events.rs；
// 2. events.ts 生成导出集——ENGINE_EVENT_NAMES（构建期同源生成物）；
// 3. 测试枚举集——本文件值钉（27 名清单；新增/删除事件必须三方同步，
//    漏更新在此先红——可读第一现场，与 engine 侧值钉测试同款守卫）。

import { readFileSync } from 'node:fs';
import * as nodePath from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { parseEngineEvents } from '../../scripts/gen-transport.mjs';
import { ENGINE_EVENT_NAMES } from './events';
import { HttpTransport } from './http';
import { TauriTransport } from './tauri';

// vitest 运行 cwd = egosync-app（jsdom 全局 URL 补丁使 import.meta.url 相对解析不可用）
const repoRoot = nodePath.resolve(process.cwd(), '..');

function readEngineEventsRs(): string {
  return readFileSync(`${repoRoot}/crates/egosync-engine/src/events.rs`, 'utf8');
}

/** 工件解析集（events.rs 全量事件，驱动逐事件路由测试）。 */
const parsedEvents = parseEngineEvents(readEngineEventsRs()).map((e) => e.eventName);

/** 测试枚举集（值钉——27 名，events.rs 现役清单）。 */
const PINNED_EVENT_NAMES: readonly string[] = [
  'bigrock:protection',
  'bigrock:reminder',
  'briefing:generated',
  'review:generated',
  'q2:reminder',
  'notification:new',
  'task:classified',
  'skill-registry-updated',
  'llm:stream',
  'message:saved',
  'conversation:created',
  'conversation:deleted',
  'conversation:title-updated',
  'role:proposed',
  'role:delegated',
  'task:tool-action',
  'role:created',
  'role:updated',
  'role:archived',
  'role:restored',
  'role:deleted',
  'task:created',
  'task:updated',
  'task:deleted',
  'task:reordered',
  'notification:read',
  'data:imported',
];

// Tauri 分支 listen 透传验证：捕获每事件名的 listen 注册（vi.hoisted 供
// 提升后的 mock 工厂引用）
const listenEvents = vi.hoisted(() => [] as string[]);
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string) => {
    listenEvents.push(event);
    return Promise.resolve(() => {});
  },
  emit: vi.fn(),
}));

describe('事件三集合同源（engine 常量 == TS 生成 == 测试枚举）', () => {
  it('events.rs 解析集 == 测试枚举集（值钉）', () => {
    expect([...parsedEvents].sort()).toEqual([...PINNED_EVENT_NAMES].sort());
  });

  it('events.ts 生成导出集 == 测试枚举集（值钉）', () => {
    expect([...ENGINE_EVENT_NAMES].sort()).toEqual([...PINNED_EVENT_NAMES].sort());
  });

  it('事件数量恰 27（计数钉——防静默增删）', () => {
    expect(parsedEvents.length).toBe(27);
    expect(ENGINE_EVENT_NAMES.length).toBe(27);
    expect(PINNED_EVENT_NAMES.length).toBe(27);
    expect(new Set(parsedEvents).size).toBe(27);
  });
});

describe('逐事件名双通道路由（events.rs 全量事件逐条）', () => {
  it('Tauri 分支：每事件名经 @tauri-apps/api/event listen 透传订阅', () => {
    listenEvents.length = 0;
    const transport = new TauriTransport();
    for (const name of parsedEvents) {
      const unlisten = transport.on(name, () => {});
      expect(typeof unlisten).toBe('function');
      unlisten();
    }
    expect([...listenEvents].sort()).toEqual([...parsedEvents].sort());
  });

  it('HTTP 分支：每事件名经单条 EventSource 同名帧分发、payload JSON 同构', () => {
    class FakeEventSource {
      listeners = new Map<string, Array<(ev: { data: string }) => void>>();
      constructor(public url: string) {
        instances.push(this);
      }
      addEventListener(name: string, listener: (ev: { data: string }) => void) {
        const list = this.listeners.get(name) ?? [];
        list.push(listener);
        this.listeners.set(name, list);
      }
      removeEventListener() {}
      close() {}
    }
    const instances: FakeEventSource[] = [];
    vi.stubGlobal('EventSource', FakeEventSource);

    try {
      const transport = new HttpTransport();
      const received: Record<string, unknown> = {};
      for (const name of parsedEvents) {
        transport.on(name, payload => {
          received[name] = payload;
        });
      }
      // 单条 EventSource 多路复用（全部 27 事件共享一条连接）
      expect(instances).toHaveLength(1);
      expect(instances[0].url).toBe('/api/events');
      // 逐事件投递 SSE 帧（event: 字段路由由 EventSource 完成，data JSON 解析）
      for (const name of parsedEvents) {
        const data = JSON.stringify({ event: name, ok: true });
        for (const listener of instances[0].listeners.get(name) ?? []) {
          listener({ data });
        }
      }
      for (const name of parsedEvents) {
        expect(received[name]).toEqual({ event: name, ok: true });
      }
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
