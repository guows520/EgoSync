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
  /** 与 WHATWG EventSource.CLOSED 对齐（致命关闭判定用）。 */
  static readonly CLOSED = 2;
  static instances: FakeEventSource[] = [];
  listeners = new Map<string, Array<(ev: { data: string }) => void>>();
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  /** 默认 OPEN（1）：既有用例的 onerror 均为非致命（原生重连语义）。 */
  readyState = 1;
  /** Story 16.1：close() 调用标记（无订阅即关断言）。 */
  closed = false;
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
  close() {
    this.closed = true;
  }
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

  // ── Story 16.1：401 全局拦截 / 认证态类型化与缓存失效 / 无订阅即关 ──

  it('invoke 401：reject HttpTransportError(401) 且发射 auth:unauthorized 前端本地事件', async () => {
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    const unauthorized: unknown[] = [];
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      return Promise.resolve(jsonResponse(401, { error: 'unauthorized' }));
    });
    const transport = new HttpTransport();
    const unlisten = transport.on('auth:unauthorized', (payload) => unauthorized.push(payload));

    const command = WEB_OK_COMMANDS[0]!;
    await expect(transport.invoke(command)).rejects.toMatchObject({ status: 401 });
    // 前端本地事件同步发射（AuthGate 全局拦截的信号源；不建 EventSource）
    expect(unauthorized).toHaveLength(1);
    expect(FakeEventSource.instances).toHaveLength(0);
    unlisten();
  });

  it('getAuthStatus 类型化 {setupRequired, authenticated}；invalidateAuthStatusCache 后重新请求', async () => {
    let setupRequired = true;
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string) => {
      fetchCalls.push({ url, init: {} });
      return Promise.resolve(jsonResponse(200, { setupRequired, authenticated: false }));
    });
    const transport = new HttpTransport();

    const first = await transport.getAuthStatus();
    // 类型化断言：形状即协议（setupRequired/authenticated 布尔字段）
    expect(first).toEqual({ setupRequired: true, authenticated: false });

    // 缓存命中：失效前零额外请求
    await transport.getAuthStatus();
    expect(fetchCalls.filter((c) => c.url === '/api/auth/status')).toHaveLength(1);

    // 失效（登录/setup 成功与登出路径）→ 下次调用真实重发
    transport.invalidateAuthStatusCache();
    setupRequired = false;
    const second = await transport.getAuthStatus();
    expect(second).toEqual({ setupRequired: false, authenticated: false });
    expect(fetchCalls.filter((c) => c.url === '/api/auth/status')).toHaveLength(2);
  });

  it('无订阅即关：全部事件订阅撤空 → 关闭 EventSource + 取消重建定时器；再订阅懒重建', () => {
    vi.useFakeTimers();
    try {
      vi.unstubAllGlobals();
      vi.stubGlobal('EventSource', FakeEventSource);
      vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
        fetchCalls.push({ url, init });
        return Promise.resolve(jsonResponse(200, { ok: true }));
      });
      const transport = new HttpTransport();

      const unlisten = transport.on(SAMPLE_EVENT, () => {});
      const source = FakeEventSource.instances[0]!;
      expect(source.closed).toBe(false);

      // 致命关闭（readyState=CLOSED）：eventSource 置空 + 5s 重建定时器入队
      source.readyState = FakeEventSource.CLOSED;
      source.onerror!();

      // 最后一个订阅撤空 → 无订阅即关：丢弃已死连接引用 + 取消重建定时器
      unlisten();
      expect(transport['eventSource']).toBeNull();

      // 重建定时器已取消：退避期满不再新建连接（无 401 重建循环）
      vi.advanceTimersByTime(10_000);
      expect(FakeEventSource.instances).toHaveLength(1);

      // 再订阅：懒重建新连接；撤空后该连接被显式 close
      const unlisten2 = transport.on(SAMPLE_EVENT, () => {});
      expect(FakeEventSource.instances).toHaveLength(2);
      unlisten2();
      expect(FakeEventSource.instances[1]!.closed).toBe(true);
    } finally {
      vi.useRealTimers();
    }
  });

  it('无订阅即关：仅部分事件撤订阅（其余仍在）不关闭连接', () => {
    const transport = new HttpTransport();
    const unlistenFirst = transport.on(SAMPLE_EVENT, () => {});
    const unlistenSecond = transport.on(ENGINE_EVENT_NAMES[1]!, () => {});
    const source = FakeEventSource.instances[0]!;

    // 撤掉其一：另一事件仍在订阅 → 连接保持
    unlistenFirst();
    expect(source.closed).toBe(false);
    expect(transport['eventSource']).toBe(source);

    // 全部撤空 → 关闭
    unlistenSecond();
    expect(source.closed).toBe(true);
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

  it('invoke：响应体读取中断（text() reject）⇒ HttpTransportError(0, null)（网络失败同形状——评审 G3）', async () => {
    vi.stubGlobal('fetch', () =>
      Promise.resolve({
        status: 200,
        headers: { get: () => null },
        text: () => Promise.reject(new TypeError('network error')),
      } as unknown as Response)
    );
    const transport = new HttpTransport();
    await expect(transport.invoke(WEB_OK_COMMANDS[0]!)).rejects.toMatchObject({
      name: 'HttpTransportError',
      status: 0,
      body: null,
    });
  });

  it('状态订阅者抛错被隔离：其余订阅者照常收状态、恢复重放照常触发（评审 G8）', async () => {
    const transport = new HttpTransport();
    const good = vi.fn();
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      transport.onConnectionStateChange(() => {
        throw new Error('buggy subscriber');
      });
      transport.onConnectionStateChange(good);
      transport.on(SAMPLE_EVENT, () => {});
      const source = FakeEventSource.instances[0]!;
      source.onopen!();
      expect(good).toHaveBeenCalledWith('online');
      source.onerror!();
      source.onopen!(); // 恢复：坏订阅者不得断掉恢复链
      expect(good).toHaveBeenCalledWith('online');
      await vi.waitFor(() => {
        expect(fetchCalls.length).toBe(REPLAY_WHITELIST.length);
      });
    } finally {
      spy.mockRestore();
    }
  });

  it('致命关闭（非 200，浏览器不原生重连）⇒ 退避后重建连接，恢复链（状态回 online + 重放）复活（评审 G2）', async () => {
    vi.useFakeTimers();
    try {
      const transport = new HttpTransport();
      const states: string[] = [];
      transport.onConnectionStateChange(state => states.push(state));
      transport.on(SAMPLE_EVENT, () => {});
      const source = FakeEventSource.instances[0]!;
      source.onopen!();
      source.readyState = 2; // CLOSED：WHATWG 对非 200 的永久关闭
      source.onerror!();
      expect(states).toEqual(['connecting', 'online', 'reconnecting']);
      expect(FakeEventSource.instances).toHaveLength(1); // 未即时重建（退避）
      vi.advanceTimersByTime(4999);
      expect(FakeEventSource.instances).toHaveLength(1); // 退避期内不重建
      vi.advanceTimersByTime(1);
      expect(FakeEventSource.instances).toHaveLength(2); // 退避到期重建
      const rebuilt = FakeEventSource.instances[1]!;
      rebuilt.onopen!();
      expect(states).toEqual(['connecting', 'online', 'reconnecting', 'online']);
      // 恢复重放照常触发（致命关闭后 onopen-after-error 路径复活——纯微任务链）
      const reconnected = waitForReconnected(transport);
      await reconnected;
      expect(fetchCalls.length).toBe(REPLAY_WHITELIST.length);
    } finally {
      vi.useRealTimers();
    }
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
