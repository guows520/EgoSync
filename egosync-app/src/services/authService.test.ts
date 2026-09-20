// Story 16.1：authService REST 直连形状测试（评审修复——补 logout）。
//
// 认证面不进 invoke 通道（独立 REST 契约）：login/setup/logout 的
// URL / method / credentials / body 形状此前无任何测试钉住（Sidebar 测试
// mock 掉服务层，AuthGate 登出用例直接发事件）——本文件以 stubGlobal
// fetch 断言真实调用形状（AuthGate.test.tsx 同款假 Response 桩）。
//
// 浏览器分支范式 = 删桩-恢复-`__resetTransportForTests()` 三件套。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetTransportForTests, HttpTransportError } from '@/transport';
import { authService } from './authService';

/** 全局 Tauri 桩快照（beforeEach 删 / afterEach 恢复）。 */
const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

/** 假 fetch Response（authService 只消费 status / text）。 */
function jsonResponse(status: number, body: unknown): Response {
  const text = JSON.stringify(body);
  return {
    status,
    ok: status >= 200 && status < 300,
    headers: { get: () => null },
    text: () => Promise.resolve(text),
  } as unknown as Response;
}

/** 按 URL 分流的 fetch 桩；返回调用记录。 */
function stubFetch(
  handler: (url: string, init?: RequestInit) => Response | Promise<Response>
): Array<{ url: string; init?: RequestInit }> {
  const calls: Array<{ url: string; init?: RequestInit }> = [];
  vi.stubGlobal('fetch', (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    const result = handler(url, init);
    return result instanceof Promise ? result : Promise.resolve(result);
  });
  return calls;
}

describe('authService REST 直连形状（logout / login / setup）', () => {
  beforeEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetTransportForTests();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
  });

  it('logout：POST /api/auth/logout，credentials same-origin，body 空对象', async () => {
    const calls = stubFetch(() => jsonResponse(200, { ok: true }));

    await authService.logout();

    // 形状断言：此前 logout 的真实 REST 调用零测试执行（Sidebar 测试
    // mock 掉服务层）——URL/method/credentials/body 全部钉死
    expect(calls).toHaveLength(1);
    const call = calls[0];
    expect(call.url).toBe('/api/auth/logout');
    expect(call.init?.method).toBe('POST');
    expect(call.init?.credentials).toBe('same-origin');
    expect(JSON.parse(String(call.init?.body))).toEqual({});
    expect((call.init?.headers as Record<string, string>)['Content-Type']).toBe(
      'application/json'
    );
  });

  it('login：POST /api/auth/login，credentials same-origin，body {token}', async () => {
    const calls = stubFetch(() => jsonResponse(200, { ok: true }));

    await authService.login('the-token');

    expect(calls).toHaveLength(1);
    const call = calls[0];
    expect(call.url).toBe('/api/auth/login');
    expect(call.init?.method).toBe('POST');
    expect(call.init?.credentials).toBe('same-origin');
    expect(JSON.parse(String(call.init?.body))).toEqual({ token: 'the-token' });
  });

  it('setup：POST /api/setup，body {token}', async () => {
    const calls = stubFetch(() => jsonResponse(200, { ok: true }));

    await authService.setup('setup-token');

    expect(calls).toHaveLength(1);
    const call = calls[0];
    expect(call.url).toBe('/api/setup');
    expect(call.init?.method).toBe('POST');
    expect(call.init?.credentials).toBe('same-origin');
    expect(JSON.parse(String(call.init?.body))).toEqual({ token: 'setup-token' });
  });

  it('logout 非 200：reject HttpTransportError{status, body}', async () => {
    stubFetch(() => jsonResponse(429, { error: 'rate limit exceeded' }));

    // Sidebar 429 分流依赖此错误形状（status 字段）
    await expect(authService.logout()).rejects.toMatchObject({
      status: 429,
      body: { error: 'rate limit exceeded' },
    });
    await expect(authService.logout()).rejects.toBeInstanceOf(HttpTransportError);
  });

  it('网络失败：reject HttpTransportError{status:0, body:null}', async () => {
    stubFetch(() => Promise.reject(new TypeError('network down')));

    await expect(authService.logout()).rejects.toMatchObject({ status: 0, body: null });
  });
});
