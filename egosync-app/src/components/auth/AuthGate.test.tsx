// Story 16.1：AuthGate 全路径矩阵测试（I/O 矩阵逐行钉死）。
//
// 覆盖：首访 setup 引导 / env 锁定 404 统一说明 / 登录成功失败 / 429 退避 /
// 断网重试 / 401 全局拦截回登录 / 登出事件 / 缓存失效（15-5 G11 遗嘱）/
// 桌面宿主直通。
//
// 浏览器分支范式 = 删桩-恢复-`__resetTransportForTests()` 三件套
// （TitleBar.browser.test.tsx 同款）：删 `__TAURI_INTERNALS__` 走
// HttpTransport，fetch 经 vi.stubGlobal 注入假实现；children 用标记 div
// 替代真实 App（gate 语义与 App 内部解耦——App 桌面回归由 App.test.tsx
// 默认桩覆盖）。

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthGate } from './AuthGate';
import {
  __resetTransportForTests,
  getTransport,
  emitFrontendEvent,
} from '@/transport';

/** 全局 Tauri 桩快照（beforeEach 删 / afterEach 恢复）。 */
const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

/** 假 fetch Response（gate/authService 只消费 status / text）。 */
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

/** gate children 标记（不挂真实 App——零 invoke/事件订阅噪音）。 */
const APP_MARKER = 'gate-app-marker';

function renderGate() {
  return render(
    <AuthGate>
      <div data-testid={APP_MARKER}>app</div>
    </AuthGate>
  );
}

describe('AuthGate 全路径矩阵（浏览器宿主）', () => {
  beforeEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    __resetTransportForTests();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
  });

  // ── status 分流 ──

  it('首访未初始化：setupRequired:true → setup 向导', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(200, { setupRequired: true, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByText('初始化 EgoSync 实例')).toBeInTheDocument();
    expect(screen.queryByText('登录 EgoSync')).not.toBeInTheDocument();
  });

  it('已初始化未登录：setupRequired:false → 登录页（含次级「首次部署？」入口）', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(200, { setupRequired: false, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByText('登录 EgoSync')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /首次部署？/ })).toBeInTheDocument();
  });

  it('已认证（持久 Cookie）：直通渲染 children', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(200, { setupRequired: false, authenticated: true })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
    expect(screen.queryByText('登录 EgoSync')).not.toBeInTheDocument();
  });

  it('服务器不可达：断网文案 + 重试按钮（不白屏）', async () => {
    let fail = true;
    stubFetch(url => {
      if (url === '/api/auth/status' && fail) {
        return Promise.reject(new TypeError('network down'));
      }
      return jsonResponse(200, { setupRequired: false, authenticated: false });
    });
    renderGate();
    // 评审修复 #2：文案统一 authErrors 常量（带句号——与登录面同源）
    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();

    // 重试：恢复网络 → 登录页
    fail = false;
    fireEvent.click(screen.getByRole('button', { name: '重试' }));
    await waitFor(() => {
      expect(screen.getByText('登录 EgoSync')).toBeInTheDocument();
    });
  });

  it('status 检查撞 429：尝试过于频繁退避文案 + 重试按钮', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(429, { error: 'rate limit exceeded' })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByText('尝试过于频繁，请稍后再试')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
  });

  // ── setup 路径 ──

  it('setup 成功：自动携令牌 login → 进入应用', async () => {
    const calls = stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: true, authenticated: false });
      }
      if (url === '/api/setup') return jsonResponse(200, { ok: true });
      if (url === '/api/auth/login') return jsonResponse(200, { ok: true });
      return jsonResponse(404, { error: 'not found' });
    });
    renderGate();
    await screen.findByText('初始化 EgoSync 实例');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'first-visit-token' } });
    fireEvent.change(screen.getByLabelText('确认令牌'), { target: { value: 'first-visit-token' } });
    fireEvent.click(screen.getByRole('button', { name: /完成初始化并进入/ }));

    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
    const setupCall = calls.find(c => c.url === '/api/setup');
    const loginCall = calls.find(c => c.url === '/api/auth/login');
    expect(setupCall, 'setup 请求应发出').toBeDefined();
    expect(loginCall, 'setup 成功后应自动 login').toBeDefined();
    expect(JSON.parse(String(loginCall?.init?.body))).toEqual({ token: 'first-visit-token' });
  });

  it('setup 短令牌：前端预校验拦截（<8 字符不可提交）', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(200, { setupRequired: true, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    await screen.findByText('初始化 EgoSync 实例');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'short' } });
    fireEvent.change(screen.getByLabelText('确认令牌'), { target: { value: 'short' } });
    expect(screen.getByRole('button', { name: /完成初始化并进入/ })).toBeDisabled();
    expect(screen.getByText('令牌至少 8 位字符')).toBeInTheDocument();
  });

  it('env 锁定态 setup 尝试：404 → 统一锁定说明（不自动重试，可回登录）', async () => {
    const calls = stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: false, authenticated: false });
      }
      if (url === '/api/setup') return jsonResponse(404, { error: 'not found' });
      return jsonResponse(200, { ok: true });
    });
    renderGate();
    await screen.findByText('登录 EgoSync');

    // 次级入口进入 setup（已初始化实例的用户误入路径）
    fireEvent.click(screen.getByRole('button', { name: /首次部署？/ }));
    await screen.findByText('初始化 EgoSync 实例');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'locked-out-token' } });
    fireEvent.change(screen.getByLabelText('确认令牌'), { target: { value: 'locked-out-token' } });
    fireEvent.click(screen.getByRole('button', { name: /完成初始化并进入/ }));

    expect(
      await screen.findByText(/实例已初始化或已由环境变量锁定/)
    ).toBeInTheDocument();
    // 不自动重试：setup 恰一次请求；login 未被调用（不自动 login）
    expect(calls.filter(c => c.url === '/api/setup')).toHaveLength(1);
    expect(calls.filter(c => c.url === '/api/auth/login')).toHaveLength(0);

    // 返回登录入口
    fireEvent.click(screen.getByRole('button', { name: '返回登录' }));
    expect(await screen.findByText('登录 EgoSync')).toBeInTheDocument();
  });

  it('setup 成功但自动登录失败：如实呈现（登录面文案）+ 返回登录入口', async () => {
    stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: true, authenticated: false });
      }
      if (url === '/api/setup') return jsonResponse(200, { ok: true });
      // 自动登录失败（如服务端异常重启后令牌校验变化等边角）
      if (url === '/api/auth/login') return jsonResponse(401, { error: 'unauthorized' });
      return jsonResponse(404, { error: 'not found' });
    });
    renderGate();
    await screen.findByText('初始化 EgoSync 实例');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'first-visit-token' } });
    fireEvent.change(screen.getByLabelText('确认令牌'), { target: { value: 'first-visit-token' } });
    fireEvent.click(screen.getByRole('button', { name: /完成初始化并进入/ }));

    // 内层按登录面分流（401 → 令牌不正确）——不是 setup 面的
    // 「令牌长度不足」（评审修复：setupErrorText → loginErrorText）
    expect(
      await screen.findByText(/初始化已完成，但自动登录失败（令牌不正确，请重试。）/)
    ).toBeInTheDocument();
    // 显性返回登录入口（引导手动登录）；未进入应用
    expect(screen.getByRole('button', { name: '返回登录' })).toBeInTheDocument();
    expect(screen.queryByTestId(APP_MARKER)).not.toBeInTheDocument();
  });

  it('离开 checking 态：入口 splash 隐藏并延时移除（评审补：行为此前零断言）', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(200, { setupRequired: false, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    // 模拟 index.html 的入口遮罩（z-9999）——jsdom 中无此节点，须注入
    const splash = document.createElement('div');
    splash.id = 'egosync-splash';
    document.body.appendChild(splash);
    try {
      renderGate();
      await screen.findByText('登录 EgoSync');

      // 立即隐藏：遮罩不再盖住认证界面（视觉即时正确）
      expect(splash.classList.contains('hidden'), 'splash 应立即 hidden').toBe(true);
      // 400ms 过渡后从 DOM 移除（真实浏览器动画收尾；waitFor 留足裕量）
      await waitFor(() => expect(splash.isConnected).toBe(false), { timeout: 1500 });
    } finally {
      splash.remove();
    }
  });

  // ── 登录路径 ──

  it('登录成功：进入应用；缓存失效后重新挂载按新认证态分流', async () => {
    let authenticated = false;
    const calls = stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: false, authenticated });
      }
      if (url === '/api/auth/login') {
        authenticated = true;
        return jsonResponse(200, { ok: true });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    const { unmount } = renderGate();
    await screen.findByText('登录 EgoSync');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'correct-token' } });
    fireEvent.click(screen.getByRole('button', { name: '登录' }));
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();

    // 缓存失效（登录成功路径调用 invalidateAuthStatusCache）：重新挂载
    // 必须重发 status 请求并按新态（authenticated:true）直通——若缓存
    // 未失效，第二次挂载会复用 pre-login 的 authenticated:false 回登录页
    unmount();
    renderGate();
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
    expect(calls.filter(c => c.url === '/api/auth/status')).toHaveLength(2);
  });

  it('登录失败：401 → 统一失败文案（可重试），不泄露存在性', async () => {
    stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: false, authenticated: false });
      }
      if (url === '/api/auth/login') return jsonResponse(401, { error: 'unauthorized' });
      return jsonResponse(404, { error: 'not found' });
    });
    renderGate();
    await screen.findByText('登录 EgoSync');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'wrong-token' } });
    fireEvent.click(screen.getByRole('button', { name: '登录' }));

    expect(await screen.findByText('令牌不正确，请重试。')).toBeInTheDocument();
    // 表单仍在——可重试
    expect(screen.getByRole('button', { name: '登录' })).toBeInTheDocument();
  });

  it('登录撞限流：429 → 尝试过于频繁', async () => {
    stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: false, authenticated: false });
      }
      if (url === '/api/auth/login') return jsonResponse(429, { error: 'rate limit exceeded' });
      return jsonResponse(404, { error: 'not found' });
    });
    renderGate();
    await screen.findByText('登录 EgoSync');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'any-token' } });
    fireEvent.click(screen.getByRole('button', { name: '登录' }));

    expect(await screen.findByText('尝试过于频繁，请稍后再试')).toBeInTheDocument();
  });

  it('登录时断网：无法连接服务器', async () => {
    stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: false, authenticated: false });
      }
      return Promise.reject(new TypeError('network down'));
    });
    renderGate();
    await screen.findByText('登录 EgoSync');

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'any-token' } });
    fireEvent.click(screen.getByRole('button', { name: '登录' }));

    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();
  });

  // ── 401 全局拦截与登出（auth:unauthorized 事件面） ──

  it('会话中 invoke 401：auth:unauthorized → 回登录页 + 认证态缓存失效', async () => {
    let authenticated = true;
    stubFetch(url => {
      if (url === '/api/auth/status') {
        return jsonResponse(200, { setupRequired: false, authenticated });
      }
      if (url === '/api/cmd/role_list') {
        return jsonResponse(401, { error: 'unauthorized' });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    const { unmount } = renderGate();
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();

    // 模拟会话失效后的业务 invoke（http.ts 401 → 发射 auth:unauthorized）
    await act(async () => {
      await expect(getTransport().invoke('role_list')).rejects.toMatchObject({ status: 401 });
    });

    // 全局回登录页（内存态随 App 卸载清空——children 不再渲染）
    expect(await screen.findByText('登录 EgoSync')).toBeInTheDocument();
    expect(screen.queryByTestId(APP_MARKER)).not.toBeInTheDocument();

    // 缓存失效：重新挂载必须重查 status（authenticated 仍 true 时应回应用）
    authenticated = true;
    unmount();
    renderGate();
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
  });

  it('登出事件（Sidebar 发射）：回登录页，children 卸载', async () => {
    stubFetch(url =>
      url === '/api/auth/status'
        ? jsonResponse(200, { setupRequired: false, authenticated: true })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();

    // Sidebar 登出路径：authService.logout 后 emitFrontendEvent('auth:unauthorized')
    await act(async () => {
      await emitFrontendEvent('auth:unauthorized');
    });

    expect(await screen.findByText('登录 EgoSync')).toBeInTheDocument();
    expect(screen.queryByTestId(APP_MARKER)).not.toBeInTheDocument();
  });
});

describe('AuthGate 桌面宿主直通', () => {
  // 默认桩（__TAURI_INTERNALS__ 存在）——桌面分支零 fetch、零事件订阅
  it('桌面宿主：初始即 ready 直通渲染 children，零 fetch', () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    try {
      renderGate();
      expect(screen.getByTestId(APP_MARKER)).toBeInTheDocument();
      expect(screen.queryByText('登录 EgoSync')).not.toBeInTheDocument();
      expect(fetchSpy).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllGlobals();
      __resetTransportForTests();
    }
  });
});
