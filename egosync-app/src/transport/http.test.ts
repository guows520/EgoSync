// Story 15.5：HttpTransport 单元钉语义。
//
// 覆盖：SSE 帧解析 / 状态机转换（connecting→online→reconnecting→online）/
// 重放触发与不触发 / 写入类零重放 / 重放结果集含错误不中断 / 连续恢复
// 只重放一次（并发防护）/ auth status 缓存只请求一次 / FRONTEND_LOCAL_EVENTS
// 不建 EventSource。
//
// EventSource/fetch 经 vi.stubGlobal 注入假实现（jsdom 无 EventSource）；
// 事件名/命令名全部取自生成物（零手写——对等面守门同款纪律）。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { REPLAY_WHITELIST, WEB_OK_COMMANDS } from './capabilities';
import { ENGINE_EVENT_NAMES } from './events';
import { HttpTransport } from './http';

/** 驱动用事件名/命令名（生成物机械取用，零手写）。 */
const SAMPLE_EVENT = ENGINE_EVENT_NAMES[0]!;

class FakeEventSource {
  static instances: FakeEventSource[] = [];
  listeners = new Map<string, Array<(ev: { data: string }) => void>>();
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  constructor(public url: string) {
    FakeEventSource.instances.push(this);
  }
  addEventListener(name: string, listener: (ev: { data: string }) => void) {
    const list = this.listeners.get(name) ?? [];
    list.push(listener);
    this.listeners.set(name, list);
  }
  removeEventListener(name: string, listener: (ev: { data: string }) => void) {
    const list = this.listeners.get(name) ?? [];
    this.listeners.set(name, list.filter((l) => l !== listener));
  }
  close() {}
}

/** 假 fetch Response（transport 只消费 status / headers.get / text）。 */
function jsonResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  const text = JSON.stringify(body);
  const lower = new Map(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v]));
  return {
    status,
    headers: { get: (name: string) => lower.get(name.toLowerCase()) ?? null },
    text: () => Promise.resolve(text),
  } as unknown as Response;
}

describe('HttpTransport', () => {
  let fetchCalls: Array<{ url: string; init: RequestInit }>;

  beforeEach(() => {
    FakeEventSource.instances = [];
    fetchCalls = [];
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      return Promise.resolve(jsonResponse(200, { ok: true }));
    });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('invoke：POST /api/cmd/{cmd}、same-origin 凭证、空参⇒`{}` body', async () => {
    const transport = new HttpTransport();
    const command = WEB_OK_COMMANDS[0]!;
    const result = await transport.invoke(command);
    expect(result).toEqual({ ok: true });
    expect(fetchCalls).toHaveLength(1);
    expect(fetchCalls[0]!.url).toBe(`/api/cmd/${command}`);
    expect(fetchCalls[0]!.init.method).toBe('POST');
    expect(fetchCalls[0]!.init.credentials).toBe('same-origin');
    expect(fetchCalls[0]!.init.body).toBe('{}');
  });

  it('SSE 帧解析：data JSON 解析为 payload，同名 handler 全部扇出', () => {
    const transport = new HttpTransport();
    const first = vi.fn();
    const second = vi.fn();
    transport.on(SAMPLE_EVENT, first);
    transport.on(SAMPLE_EVENT, second);
    const source = FakeEventSource.instances[0]!;
    const payload = { token: 'x', done: false };
    for (const listener of source.listeners.get(SAMPLE_EVENT) ?? []) {
      listener({ data: JSON.stringify(payload) });
    }
    expect(first).toHaveBeenCalledWith(payload);
    expect(second).toHaveBeenCalledWith(payload);
  });

  it('SSE 帧解析：非法 JSON 帧静默丢弃（与 listen 容错语义对齐）', () => {
    const transport = new HttpTransport();
    const handler = vi.fn();
    transport.on(SAMPLE_EVENT, handler);
    const source = FakeEventSource.instances[0]!;
    for (const listener of source.listeners.get(SAMPLE_EVENT) ?? []) {
      listener({ data: 'not-json' });
    }
    expect(handler).not.toHaveBeenCalled();
  });

  it('状态机：初始 connecting；首 onopen 前的 onerror 保持 connecting；onopen ⇒ online', () => {
    const transport = new HttpTransport();
    const states: string[] = [];
    transport.onConnectionStateChange(state => states.push(state));
    transport.on(SAMPLE_EVENT, () => {});
    const source = FakeEventSource.instances[0]!;

    expect(states).toEqual(['connecting']);
    source.onerror!();
    expect(states).toEqual(['connecting']);
    source.onopen!();
    expect(states).toEqual(['connecting', 'online']);
  });

  it('状态机：online 后 onerror ⇒ reconnecting，onopen 恢复 online', async () => {
    const transport = new HttpTransport();
    const states: string[] = [];
    transport.onConnectionStateChange(state => states.push(state));
    transport.on(SAMPLE_EVENT, () => {});
    const source = FakeEventSource.instances[0]!;

    source.onopen!();
    source.onerror!();
    source.onerror!(); // 重连期连续 error 维持 reconnecting
    expect(states).toEqual(['connecting', 'online', 'reconnecting']);
    source.onopen!();
    expect(states).toEqual(['connecting', 'online', 'reconnecting', 'online']);
    // 排空本次 onopen-after-error 触发的 fire-and-forget 重放——
    // 防止其跨测试泄漏到后续用例的 fetch 桩与 transport:reconnected 总线
    await vi.waitFor(() => {
      expect(fetchCalls.length).toBe(REPLAY_WHITELIST.length);
    });
  });

  it('首个 onopen 不触发重放（无错误历史）', () => {
    const transport = new HttpTransport();
    transport.on(SAMPLE_EVENT, () => {});
    const source = FakeEventSource.instances[0]!;
    source.onopen!();
    expect(fetchCalls).toHaveLength(0);
  });

  it('onopen-after-error 触发白名单重放：`{}` 逐条、仅白名单命令（写入类零重放）', async () => {
    const transport = new HttpTransport();
    const unlisten = transport.on('transport:reconnected', () => {});
    transport.on(SAMPLE_EVENT, () => {});
    const source = FakeEventSource.instances[0]!;
    source.onopen!();
    source.onerror!();

    const done = waitForReconnected(transport);
    source.onopen!();
    await done;

    const replayed = fetchCalls.map((c) => c.url.replace('/api/cmd/', ''));
    expect([...replayed].sort()).toEqual([...REPLAY_WHITELIST].sort());
    expect(fetchCalls).toHaveLength(REPLAY_WHITELIST.length);
    // 白名单外命令（写入类等）零重放
    for (const command of WEB_OK_COMMANDS) {
      if (!REPLAY_WHITELIST.includes(command)) {
        expect(replayed).not.toContain(command);
      }
    }
    // 重放一律空参 `{}`
    for (const call of fetchCalls) {
      expect(call.init.body).toBe('{}');
    }
    unlisten();
  });

  it('重放结果集经 transport:reconnected 交付：含错误不中断整体（31 条全量入集）', async () => {
    const failing = REPLAY_WHITELIST[0]!;
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      if (url === `/api/cmd/${failing}`) {
        // 重放中单条 AppError：200 + 判别头
        return Promise.resolve(
          jsonResponse(200, { DbError: '数据库暂时不可用' }, { 'x-egosync-app-error': '1' })
        );
      }
      return Promise.resolve(jsonResponse(200, { replayed: true }));
    });

    const transport = new HttpTransport();
    let delivered: { results: Record<string, unknown> } | null = null;
    const unlisten = transport.on('transport:reconnected', payload => {
      delivered = payload as { results: Record<string, unknown> };
    });
    transport.on(SAMPLE_EVENT, () => {});
    const source = FakeEventSource.instances[0]!;
    source.onopen!();
    source.onerror!();
    source.onopen!();
    await vi.waitFor(() => {
      expect(delivered).not.toBeNull();
    });

    expect(Object.keys(delivered!.results).sort()).toEqual([...REPLAY_WHITELIST].sort());
    expect(delivered!.results[failing]).toEqual({ DbError: '数据库暂时不可用' });
    const successSample = REPLAY_WHITELIST[1]!;
    expect(delivered!.results[successSample]).toEqual({ replayed: true });
    unlisten();
  });

  it('连续恢复只重放一次（并发防护：重放进行中的 onopen 不再触发）', async () => {
    // 慢 fetch：首个重放请求由手动门放行，其后立即成功 ⇒ 重放批次可控推进
    let gateOpen = false;
    let pending: ((value: Response) => void) | null = null;
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      if (gateOpen) return Promise.resolve(jsonResponse(200, { ok: true }));
      return new Promise<Response>(resolve => {
        pending = resolve;
      });
    });

    const transport = new HttpTransport();
    transport.on(SAMPLE_EVENT, () => {});
    const source = FakeEventSource.instances[0]!;
    source.onopen!();
    source.onerror!();
    source.onopen!();
    // 第一轮重放挂起中（第 1 条 fetch pending），又来一轮 error→open：
    // 不得追加新重放（replaying 门常闭——连续恢复只重放一次）
    source.onerror!();
    source.onopen!();
    expect(fetchCalls).toHaveLength(1);
    // 开闸放行：挂起批次继续走完全部白名单
    gateOpen = true;
    pending!(jsonResponse(200, { ok: true }));
    await vi.waitFor(() => {
      expect(fetchCalls.length).toBe(REPLAY_WHITELIST.length);
    });
  });

  it('getAuthStatus 缓存：首次请求后缓存，第二次调用零额外请求', async () => {
    const transport = new HttpTransport();
    const first = await transport.getAuthStatus();
    const second = await transport.getAuthStatus();
    expect(second).toBe(first);
    const authCalls = fetchCalls.filter((c) => c.url === '/api/auth/status');
    expect(authCalls).toHaveLength(1);
    expect(authCalls[0]!.init.credentials).toBe('same-origin');
  });

  it('FRONTEND_LOCAL_EVENTS：进程内消化，不建 EventSource、不进传输契约', () => {
    const transport = new HttpTransport();
    const handler = vi.fn();
    const unlisten = transport.on('skill-scope-updated', handler);
    // 前端本地事件订阅不建 EventSource（EngineEventNames 里的引擎事件才建）
    expect(FakeEventSource.instances).toHaveLength(0);
    transport.emitFrontendEvent('skill-scope-updated', { roleId: 'r1' });
    expect(handler).toHaveBeenCalledWith({ roleId: 'r1' });
    unlisten();
    transport.emitFrontendEvent('skill-scope-updated', { roleId: 'r2' });
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('on 取消订阅：handler 移除且不再收帧', () => {
    const transport = new HttpTransport();
    const handler = vi.fn();
    const unlisten = transport.on(SAMPLE_EVENT, handler);
    const source = FakeEventSource.instances[0]!;
    const listener = (source.listeners.get(SAMPLE_EVENT) ?? [])[0]!;
    listener({ data: JSON.stringify({ n: 1 }) });
    expect(handler).toHaveBeenCalledTimes(1);
    unlisten();
    listener({ data: JSON.stringify({ n: 2 }) });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(source.listeners.get(SAMPLE_EVENT)).toHaveLength(0);
  });
});

/** 等待 transport:reconnected 交付（重放批次完成的信号）。 */
function waitForReconnected(transport: HttpTransport): Promise<void> {
  return new Promise(resolve => {
    const unlisten = transport.on('transport:reconnected', () => {
      unlisten();
      resolve();
    });
  });
}
