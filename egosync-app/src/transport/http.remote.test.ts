// Story 16.3：HttpTransport 远程桌面通道钉语义（独立于既有 http.test.ts
// 的浏览器路径钉——既有文件零改动即浏览器行为回归守门）。
//
// 覆盖（spec Tasks「测试三面」之 http.ts remote 行为）：
// - 绝对 URL + Authorization 头（invoke / getAuthStatus / 票据签发）；
// - SSE 票据流程：订阅 ⇒ 签发（Bearer）⇒ ?ticket= 建流；onopen 后票据
//   清空（一次性——下次重建重走签发）；
// - 票据建流被拒 ⇒ 重取重试一次（不升级退避）；再失败 ⇒ 退避；
// - 远程退避 governor：1s→2s→4s…封顶 30s + 抖动（±20% 容差）；
// - onerror 全接管（native 重连复用已消费票据必然失败——立即关闭+重取）；
// - 无订阅即关：撤空 ⇒ 世代翻新（进行中的异步建流作废，防孤儿连接）；
// - updateToken：就地换令牌 + 认证缓存失效。
//
// EventSource/fetch 经 vi.stubGlobal 注入假实现（jsdom 无 EventSource）。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { REPLAY_WHITELIST } from './capabilities';
import { ENGINE_EVENT_NAMES } from './events';
import { HttpTransport } from './http';

const SAMPLE_EVENT = ENGINE_EVENT_NAMES[0]!;
const BASE_URL = 'https://instance.example.com';

class FakeEventSource {
  static readonly CLOSED = 2;
  static instances: FakeEventSource[] = [];
  listeners = new Map<string, Array<(ev: { data: string }) => void>>();
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 1;
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
    this.readyState = FakeEventSource.CLOSED;
  }
}

function jsonResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  const text = JSON.stringify(body);
  const lower = new Map(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v]));
  return {
    status,
    headers: { get: (name: string) => lower.get(name.toLowerCase()) ?? null },
    text: () => Promise.resolve(text),
    json: () => Promise.resolve(body),
  } as unknown as Response;
}

/** 构造远程实例（fetch 票据签发默认成功）。 */
function makeRemote(token = 'the-token'): HttpTransport {
  return new HttpTransport({ kind: 'remote', baseUrl: BASE_URL, token });
}

describe('HttpTransport 远程桌面通道', () => {
  let fetchCalls: Array<{ url: string; init: RequestInit }>;
  /** 可编程票据签发结果（默认成功发一枚新票）。 */
  let ticketResponse: () => Response;

  beforeEach(() => {
    FakeEventSource.instances = [];
    fetchCalls = [];
    ticketResponse = () => jsonResponse(200, { ticket: `sse_${Math.random().toString(36).slice(2)}` });
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      if (url.endsWith('/api/events/ticket')) return Promise.resolve(ticketResponse());
      return Promise.resolve(jsonResponse(200, { ok: true }));
    });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('invoke：绝对 URL + Bearer 头（相对路径语义不复用）', async () => {
    const transport = makeRemote('secret-token');
    const result = await transport.invoke('role_list');
    expect(result).toEqual({ ok: true });
    expect(fetchCalls).toHaveLength(1);
    expect(fetchCalls[0]!.url).toBe('https://instance.example.com/api/cmd/role_list');
    const headers = fetchCalls[0]!.init.headers as Record<string, string>;
    expect(headers['Authorization']).toBe('Bearer secret-token');
    expect(headers['Content-Type']).toBe('application/json');
  });

  it('invoke 401：reject + auth:unauthorized 全局事件（远程令牌失效信号）', async () => {
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', () => Promise.resolve(jsonResponse(401, { error: 'unauthorized' })));
    const transport = makeRemote();
    const unauthorized: unknown[] = [];
    transport.on('auth:unauthorized', payload => unauthorized.push(payload));
    await expect(transport.invoke('role_list')).rejects.toMatchObject({ status: 401 });
    expect(unauthorized).toHaveLength(1);
  });

  it('getAuthStatus：绝对 URL + Bearer 头（Bearer 判定通道）', async () => {
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      if (url.endsWith('/api/auth/status')) {
        return Promise.resolve(jsonResponse(200, { setupRequired: false, authenticated: true }));
      }
      return Promise.resolve(jsonResponse(200, { ok: true }));
    });
    const transport = makeRemote('primary-token');
    const status = await transport.getAuthStatus();
    expect(status).toEqual({ setupRequired: false, authenticated: true });
    const call = fetchCalls.find(c => c.url.endsWith('/api/auth/status'));
    expect(call).toBeDefined();
    expect((call!.init.headers as Record<string, string>)['Authorization']).toBe('Bearer primary-token');
  });

  it('SSE 票据流程：订阅 ⇒ Bearer 签发 ⇒ ?ticket= 建流；onopen 票据清空', async () => {
    const transport = makeRemote();
    transport.on(SAMPLE_EVENT, () => {});

    // 签发（Bearer 头）→ 建流（?ticket=）
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(1);
    });
    const ticketCall = fetchCalls.find(c => c.url.endsWith('/api/events/ticket'));
    expect(ticketCall).toBeDefined();
    expect(ticketCall!.init.method).toBe('POST');
    expect((ticketCall!.init.headers as Record<string, string>)['Authorization']).toBe('Bearer the-token');

    const source = FakeEventSource.instances[0]!;
    expect(source.url).toMatch(/^https:\/\/instance\.example\.com\/api\/events\?ticket=sse_/);
    // 票据不携带令牌本体（防令牌入 URL/日志）
    expect(source.url).not.toContain('the-token');

    // onopen：在线 + 票据清空（一次性——下次重建重走签发）
    source.onopen!();
    const states: string[] = [];
    transport.onConnectionStateChange(s => states.push(s));
    expect(transport['sseTicket']).toBeNull();
    void states;
  });

  it('建流后撤订阅重建：重走签发（票据一次性——不滞留）', async () => {
    const transport = makeRemote();
    const unlisten = transport.on(SAMPLE_EVENT, () => {});
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(1);
    });
    FakeEventSource.instances[0]!.onopen!();
    unlisten();

    // 再订阅 ⇒ 懒重建：新票据签发 + 新连接
    transport.on(SAMPLE_EVENT, () => {});
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(2);
    });
    const ticketCalls = fetchCalls.filter(c => c.url.endsWith('/api/events/ticket'));
    // 重建必须重走票据签发（一次性票据）
    expect(ticketCalls).toHaveLength(2);
  });

  it('onerror 全接管：立即关闭 + 重取票据重试一次（不升级退避）', async () => {
    const transport = makeRemote();
    transport.on(SAMPLE_EVENT, () => {});
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(1);
    });
    const first = FakeEventSource.instances[0]!;
    first.onopen!();

    // 网络错误（readyState 保持 OPEN——非致命形态）也须接管：票据一次性
    // 语义下原生重连复用已消费票据必然失败
    first.onerror!();
    // 远程路径 onerror 必须显式关闭（接管原生重连）
    expect(first.closed).toBe(true);

    // 立即重取重试（无退避定时器窗口）：新票据 + 新连接
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(2);
    });
    const ticketCalls = fetchCalls.filter(c => c.url.endsWith('/api/events/ticket'));
    expect(ticketCalls.length).toBeGreaterThanOrEqual(2);
  });

  it('签发网络失败 ⇒ 退避 governor 调度重建（REMOTE_OFFLINE 重连）', async () => {
    vi.useFakeTimers();
    try {
      ticketResponse = () => jsonResponse(500, { error: 'boom' });
      const transport = makeRemote();
      transport.on(SAMPLE_EVENT, () => {});
      // 首次签发 500 ⇒ 调度退避（不建流）
      await vi.advanceTimersByTimeAsync(0);
      expect(FakeEventSource.instances).toHaveLength(0);
      expect(transport['rebuildTimer']).not.toBeNull();

      // 退避到期：重试签发（仍 500 ⇒ 再退避）
      await vi.advanceTimersByTimeAsync(1300);
      const ticketCalls = fetchCalls.filter(c => c.url.endsWith('/api/events/ticket'));
      expect(ticketCalls.length).toBeGreaterThanOrEqual(2);
      expect(FakeEventSource.instances).toHaveLength(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it('退避 governor：中间档 1s→2s→4s 序列 + 封顶 30s + ±20% 抖动', async () => {
    vi.useFakeTimers();
    const randomSpy = vi.spyOn(Math, 'random').mockReturnValue(0.5); // 抖动中心（jitter 因子恰 1.0）
    const ticketFetchCount = () => fetchCalls.filter(c => c.url.endsWith('/api/events/ticket')).length;
    try {
      const transport = makeRemote();
      // 首连成功（默认签发桩）后转入持续失败：驱动连续退避轮次
      transport.on(SAMPLE_EVENT, () => {});
      await vi.advanceTimersByTimeAsync(0);
      expect(FakeEventSource.instances).toHaveLength(1);
      FakeEventSource.instances[0]!.onopen!();

      // 失败轮次 1：onerror ⇒ 立即重取票据一次（重试路径，无定时器）；
      // 重取失败 ⇒ 退避 attempt=1（1s 档）
      ticketResponse = () => jsonResponse(500, { error: 'boom' });
      FakeEventSource.instances[0]!.onerror!();
      await vi.advanceTimersByTimeAsync(0);
      await vi.advanceTimersByTimeAsync(0);
      expect(transport['rebuildTimer']).not.toBeNull();
      const baseline = ticketFetchCount();

      // [T11] 中间档断言（原 delays.push 为死代码——1s 与 30s 封顶之外的
      // 回归测不出）：999ms 未重试、恰 1000ms 重试（attempt=1 ⇒ 1s 档）
      await vi.advanceTimersByTimeAsync(999);
      expect(ticketFetchCount()).toBe(baseline);
      await vi.advanceTimersByTimeAsync(1);
      expect(ticketFetchCount()).toBe(baseline + 1);

      // 档位 2（2s）：1999ms 未重试、恰 2000ms 重试
      const second = ticketFetchCount();
      await vi.advanceTimersByTimeAsync(1999);
      expect(ticketFetchCount()).toBe(second);
      await vi.advanceTimersByTimeAsync(1);
      expect(ticketFetchCount()).toBe(second + 1);

      // 档位 3（4s）：3999ms 未重试、恰 4000ms 重试
      const third = ticketFetchCount();
      await vi.advanceTimersByTimeAsync(3999);
      expect(ticketFetchCount()).toBe(third);
      await vi.advanceTimersByTimeAsync(1);
      expect(ticketFetchCount()).toBe(third + 1);

      // 封顶验证：attempt 大值 ⇒ delay ≤ 30s * 1.2（36s 硬上限）
      transport['remoteBackoffAttempt'] = 50;
      const capped = transport['remoteBackoffDelay']();
      expect(capped).toBeLessThanOrEqual(36000);
      expect(capped).toBeGreaterThanOrEqual(24000); // 30s * 0.8
    } finally {
      randomSpy.mockRestore();
      vi.useRealTimers();
    }
  });

  it('无订阅即关：撤空 ⇒ 世代翻新（进行中的异步建流作废）', async () => {
    // 慢签发：票据签发由手动门放行
    let gateOpen = false;
    let pending: ((res: Response) => void) | null = null;
    vi.unstubAllGlobals();
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
      fetchCalls.push({ url, init });
      if (url.endsWith('/api/events/ticket')) {
        if (gateOpen) return Promise.resolve(ticketResponse());
        return new Promise<Response>(resolve => { pending = resolve; });
      }
      return Promise.resolve(jsonResponse(200, { ok: true }));
    });

    const transport = makeRemote();
    const unlisten = transport.on(SAMPLE_EVENT, () => {});
    // 签发挂起中撤空订阅：teardown 世代 +1
    unlisten();
    const generation = transport['sseGeneration'];
    expect(generation).toBeGreaterThan(0);

    // 放行签发：世代已翻新 ⇒ 不建流（无孤儿连接）
    gateOpen = true;
    pending!(ticketResponse());
    await vi.waitFor(() => new Promise(resolve => setTimeout(resolve, 0)));
    await Promise.resolve();
    await Promise.resolve();
    expect(FakeEventSource.instances).toHaveLength(0);
  });

  it('updateToken：就地换令牌 + 认证缓存失效 + 在手票据作废', async () => {
    const transport = makeRemote('old-token');
    // 预热缓存（旧令牌判定）
    await transport.getAuthStatus();

    transport.updateToken('new-token');
    expect((transport['remote'] as { token: string }).token).toBe('new-token');
    expect(transport['authStatusCache']).toBeUndefined();

    // 缓存失效 ⇒ 下次 getAuthStatus 以新令牌真实重发
    await transport.getAuthStatus();
    const calls = fetchCalls.filter(c => c.url.endsWith('/api/auth/status'));
    expect(calls).toHaveLength(2);
    expect((calls[1]!.init.headers as Record<string, string>)['Authorization']).toBe('Bearer new-token');
  });

  it('重放白名单 onopen-after-error 照常触发（远端恢复补齐——白名单命令原样）', async () => {
    const transport = makeRemote();
    const unlistenReconnected = transport.on('transport:reconnected', () => {});
    transport.on(SAMPLE_EVENT, () => {});
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(1);
    });
    const first = FakeEventSource.instances[0]!;
    first.onopen!();
    first.onerror!(); // ⇒ 立即重取重试（新连接）
    await vi.waitFor(() => {
      expect(FakeEventSource.instances).toHaveLength(2);
    });
    const rebuilt = FakeEventSource.instances[1]!;
    rebuilt.onopen!(); // onopen-after-error ⇒ 重放白名单

    await vi.waitFor(() => {
      const replayed = fetchCalls.filter(c => c.url.includes('/api/cmd/'));
      expect(replayed.length).toBe(REPLAY_WHITELIST.length);
    });
    unlistenReconnected();
  });
});
