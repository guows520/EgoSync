// Story 16.3：AuthGate 远程桌面分支矩阵（spec Tasks「测试三面」之 AuthGate 远程分支）。
//
// 覆盖：
// - 远程桌面去直通：初始 checking（与本地桌面直通 ready 相对）；
// - Bearer status 分流：authenticated → ready / setupRequired → setup /
//   未认证（keyring 令牌缺失/失效）→ RemoteLoginView（令牌重录）；
// - 令牌重录全链路：验证（Bearer status）→ keyring 保存（壳命令）→
//   传输换令牌 → 重验 → ready；
// - 重录 401：如实错误文案（可重试）；
// - 网络失败：离线屏 + 重试 + 「切回本地」逃生口（仅远程态）；
// - 会话中 401（auth:unauthorized）→ 回令牌重录视图；
// - 切回本地：save(mode=local, 保留 URL) + restart 调用序；
// - 本地桌面直通零回归（既有语义）。
//
// 桌面远程形态构造 = 默认 Tauri 桩 + setTransportBoot(remote) +
// setDesktopMode(remote)（main.tsx 引导注入的等价态）。

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthGate } from './AuthGate';
import {
  __resetTransportForTests,
  getTransport,
  getTransportBoot,
  setTransportBoot,
} from '@/transport';
import { setDesktopMode, __resetDesktopModeForTests } from '../../appMode';

const REMOTE_URL = 'https://instance.example.com';

/** 全局 Tauri 桩快照。 */
const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

/** 壳命令 mock（desktopModeService 直连 @tauri-apps/api/core）。 */
const shellMocks = vi.hoisted(() => ({
  saveConfig: vi.fn(),
  restart: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === 'remote_mode_save_config') return shellMocks.saveConfig(args) ?? Promise.resolve();
    if (cmd === 'remote_mode_restart') return shellMocks.restart() ?? Promise.resolve();
    if (cmd === 'desktop_get_boot_config') {
      return Promise.resolve({ mode: 'remote', remoteUrl: REMOTE_URL, remoteToken: null });
    }
    return Promise.reject(new Error(`unexpected shell command: ${cmd}`));
  },
}));

function jsonResponse(status: number, body: unknown): Response {
  const text = JSON.stringify(body);
  return {
    status,
    ok: status >= 200 && status < 300,
    headers: { get: () => null },
    text: () => Promise.resolve(text),
    json: () => Promise.resolve(body),
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

const APP_MARKER = 'gate-app-marker';

function renderGate() {
  return render(
    <AuthGate>
      <div data-testid={APP_MARKER}>app</div>
    </AuthGate>
  );
}

/** 注入远程桌面引导态（main.tsx 等价）。 */
function bootRemote(token: string | null) {
  setTransportBoot({ mode: 'remote', remoteUrl: REMOTE_URL, remoteToken: token });
  setDesktopMode('remote');
}

describe('AuthGate 远程桌面分支（Story 16.3）', () => {
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    // [T4] setDesktopMode 一次性注入——跨用例须复位注入位
    __resetDesktopModeForTests();
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });

  it('远程桌面去直通：初始 checking（不是本地桌面的直通 ready）', async () => {
    bootRemote('tk');
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: false, authenticated: true })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    // checking 兜底文案先呈现（status 拉取中）
    expect(screen.getByText('正在检查登录状态...')).toBeInTheDocument();
    // Bearer 有效 ⇒ ready（children 挂载）
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
    const boot = getTransportBoot();
    expect(boot?.mode).toBe('remote');
    // getAuthStatus 走绝对 URL + Bearer（transport 通道）
    expect(getTransport()).toBeDefined();
  });

  it('远端未初始化（setupRequired）：[T1] 远程 setup 视图（非浏览器 SetupView）+ 逃生口', async () => {
    bootRemote(null);
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: true, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    // 远程 setup 视图呈现（绝对 URL 直连面——不复用相对路径 SetupView）
    expect(await screen.findByTestId('remote-setup-view')).toBeInTheDocument();
    expect(screen.getByText('初始化远程 EgoSync 实例')).toBeInTheDocument();
    expect(screen.getByTestId('remote-setup-url')).toHaveTextContent(REMOTE_URL);
    // setup 态含「切回本地」逃生口（I/O 矩阵「可中途切回本地」）
    expect(screen.getByRole('button', { name: '切回本地模式' })).toBeInTheDocument();
  });

  it('keyring 令牌缺失/失效（authenticated:false）：令牌重录视图（RemoteLoginView）', async () => {
    bootRemote(null);
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: false, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();
    expect(screen.getByTestId('remote-login-url')).toHaveTextContent(REMOTE_URL);
    expect(screen.getByRole('button', { name: '切回本地模式' })).toBeInTheDocument();
  });

  it('令牌重录全链路：验证 → keyring 保存 → 重验 → ready', async () => {
    // 引导令牌失效：首次 status authenticated:false
    let authenticated = false;
    const calls = stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    bootRemote('stale-token');
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    // 重录新令牌：验证即通过
    authenticated = true;
    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'new-token' } });
    fireEvent.click(screen.getByRole('button', { name: /验证并进入/ }));

    // 壳命令保存（keyring + 模式文件幂等重写）
    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalledWith({
        mode: 'remote',
        remoteUrl: REMOTE_URL,
        token: 'new-token',
      });
    });
    // 重验走传输通道（Bearer status——getAuthStatus 绝对 URL）
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
    const statusCalls = calls.filter(c => c.url === `${REMOTE_URL}/api/auth/status`);
    // 验证 + 重验（新令牌的 Bearer 判定）
    expect(statusCalls.length).toBeGreaterThanOrEqual(2);
    // 传输实例换令牌（就地——后续 invoke 携新令牌）
    expect(getTransportBoot()?.remoteToken).toBe('new-token');
  });

  it('重录令牌无效（authenticated:false）：如实错误文案，可重试', async () => {
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: false, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    bootRemote(null);
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'wrong-token' } });
    fireEvent.click(screen.getByRole('button', { name: /验证并进入/ }));

    expect(await screen.findByText('令牌不正确，请重试。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /验证并进入/ })).toBeInTheDocument();
    // 未保存（keyring 零残留——失败可重试）
    expect(shellMocks.saveConfig).not.toHaveBeenCalled();
  });

  it('引导检查网络失败 ⇒ 离线屏 + 重试 + 切回本地逃生口（I/O 矩阵「远程不可达」）', async () => {
    // [评审轮2 U13 改名] 原名「重录验证网络失败」不副实：桩对全部
    // status 请求 reject——实际跑的是 gate 首查失败 → 离线屏（重录视图
    // 内 verifyToken 网络失败分支由下方独立用例覆盖）
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return Promise.reject(new TypeError('network down'));
      }
      return jsonResponse(404, { error: 'not found' });
    });
    bootRemote('tk');
    renderGate();
    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
    // 远程态：切回本地逃生口在离线屏呈现
    expect(screen.getByRole('button', { name: '切回本地模式' })).toBeInTheDocument();
    // [评审轮2 U11] 离线屏展示远端地址（配错/失效 URL 时可就地诊断）
    expect(screen.getByTestId('auth-offline-url')).toHaveTextContent(REMOTE_URL);
  });

  it('重录验证网络失败：登录视图内断网文案（不误报令牌失效）', async () => {
    // [评审轮2 U13] 真分支：gate 首查成功（authenticated:false → 重录
    // 视图）后，候选令牌的 verifyToken 直连断网 ⇒ loginErrorText 分流
    // 为网络不可达文案（HttpTransportError(0)——网络错误≠401 严格分流）
    let statusCallCount = 0;
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        statusCallCount += 1;
        if (statusCallCount === 1) {
          // gate 首查：令牌缺失 ⇒ 重录视图
          return jsonResponse(200, { setupRequired: false, authenticated: false });
        }
        // 重录提交后的候选令牌验证：断网
        return Promise.reject(new TypeError('network down'));
      }
      return jsonResponse(404, { error: 'not found' });
    });
    bootRemote(null);
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'candidate-token' } });
    fireEvent.click(screen.getByRole('button', { name: /验证并进入/ }));

    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();
    // 未保存（keyring 零残留——失败可重试）
    expect(shellMocks.saveConfig).not.toHaveBeenCalled();
    // 表单仍在（可重试）
    expect(screen.getByRole('button', { name: /验证并进入/ })).toBeEnabled();
  });

  it('离线屏切回本地：save(mode=local, 保留 URL) + restart（逃生口全链路）', async () => {
    stubFetch(() => Promise.reject(new TypeError('network down')));
    bootRemote('tk');
    renderGate();
    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '切回本地模式' }));
    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalledWith({
        mode: 'local',
        remoteUrl: REMOTE_URL,
        token: null,
      });
      expect(shellMocks.restart).toHaveBeenCalled();
    });
    // 调用序：先 save 后 restart（持久化先行——重启进新模式的前提）
    const saveOrder = shellMocks.saveConfig.mock.invocationCallOrder[0]!;
    const restartOrder = shellMocks.restart.mock.invocationCallOrder[0]!;
    expect(saveOrder).toBeLessThan(restartOrder);
  });

  it('会话中 invoke 401：auth:unauthorized → 回令牌重录视图（App 卸载）', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: true });
      }
      // 业务 invoke 401（令牌在远端被轮换）
      return jsonResponse(401, { error: 'unauthorized' });
    });
    bootRemote('tk');
    renderGate();
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();

    // 模拟远端令牌失效后的业务 invoke（http.ts 401 → auth:unauthorized）
    await act(async () => {
      await expect(getTransport().invoke('role_list')).rejects.toMatchObject({ status: 401 });
    });

    // 回令牌重录视图（children 卸载）
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();
    expect(screen.queryByTestId(APP_MARKER)).not.toBeInTheDocument();
  });

  it('本地桌面直通零回归：boot local ⇒ 初始 ready、零 fetch', () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
    setDesktopMode('local');
    renderGate();
    expect(screen.getByTestId(APP_MARKER)).toBeInTheDocument();
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

describe('AuthGate 令牌重录视图（RemoteLoginView 内部矩阵）', () => {
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    __resetDesktopModeForTests();
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });

  it('重录保存后重验失败（远端令牌轮换竞态）：如实呈现，不进应用', async () => {
    // 呼叫序：#1 gate 初始检查（旧令牌）⇒ false；#2 候选令牌直连验证 ⇒ true；
    // #3 保存后重验（新令牌经传输）⇒ false（验证与保存之间远端轮换的竞态）
    let statusCallCount = 0;
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        statusCallCount += 1;
        const authenticated = statusCallCount === 2;
        return jsonResponse(200, { setupRequired: false, authenticated });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    bootRemote('stale');
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'good-token' } });
    fireEvent.click(screen.getByRole('button', { name: /验证并进入/ }));

    // 保存被调用（验证通过即持久化）；重验失败 ⇒ 如实错误，不进应用
    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalled();
    });
    expect(await screen.findByText('令牌保存后重验失败，请重试。')).toBeInTheDocument();
    expect(screen.queryByTestId(APP_MARKER)).not.toBeInTheDocument();
    // 可重试（表单仍在）
    expect(screen.getByRole('button', { name: /验证并进入/ })).toBeInTheDocument();
  });

  it('重录验证非 200（[T10] 分流用例）：服务端 5xx 文案（verifyToken 上抛路径）', async () => {
    // 呼叫序：#1 gate 初始检查 ⇒ 200（authenticated:false → 重录视图）；
    // #2 候选令牌直连验证 ⇒ **非 200**（500）——verifyToken 上抛
    // HttpTransportError(500)，loginErrorText 分流为服务端 5xx 文案。
    //（原桩恒返 200 只覆盖 authenticated:false 分流——非 200 路径零覆盖，
    // 测试名不副实。）
    let statusCallCount = 0;
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        statusCallCount += 1;
        if (statusCallCount === 1) {
          return jsonResponse(200, { setupRequired: false, authenticated: false });
        }
        return jsonResponse(500, { error: 'internal server error' });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    bootRemote('stale');
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'x'.repeat(8) } });
    fireEvent.click(screen.getByRole('button', { name: /验证并进入/ }));
    // 非 200 ⇒ loginErrorText 服务端错误文案（与「令牌不正确」分流的
    // 200-authenticated:false 路径文案不同——两分支可区分）
    expect(await screen.findByText('服务暂时不可用，请稍后重试。')).toBeInTheDocument();
    // 未保存（keyring 零残留——失败可重试）
    expect(shellMocks.saveConfig).not.toHaveBeenCalled();
  });

  it('切回本地失败（[T3]）：错误就地可见 + 按钮复位（不永久禁用）', async () => {
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: false, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    shellMocks.saveConfig.mockRejectedValueOnce(new Error('keyring 不可用'));
    bootRemote(null);
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '切回本地模式' }));
    // [T3] handleSwitchToLocal rethrow ⇒ RemoteLoginView 侧 catch 生效：
    // 错误就地可见（login 视图 error 呈现，非仅 login 态不可见的
    // offlineMessage）
    expect(await screen.findByText(/切回本地失败：keyring 不可用/)).toBeInTheDocument();
    // finally 复位：按钮不再停在「正在切回本地...」永久禁用
    expect(screen.getByRole('button', { name: '切回本地模式' })).not.toBeDisabled();
  });

  it('切回本地失败（[评审轮2 U15]）：AppError 单键对象 reject ⇒ 首值可见（非 [object Object]）', async () => {
    // Tauri reject 的实际形状 = 序列化 AppError 单键对象（本仓
    // GlobalSettingsModal.test 的 mock 形状为证）——String(e) 会呈
    // [object Object]，keyring/校验真实原因不可见
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: false, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    shellMocks.saveConfig.mockRejectedValueOnce({ SidecarError: 'keyring 不可用' });
    bootRemote(null);
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '切回本地模式' }));
    expect(await screen.findByText(/切回本地失败：keyring 不可用/)).toBeInTheDocument();
    expect(screen.queryByText(/切回本地失败：\[object Object\]/)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '切回本地模式' })).not.toBeDisabled();
  });
});

// ── [T1 修订] 远程 setup 流（RemoteSetupView——冻结矩阵第 5 行闭合） ──
// 提交链五钉：绝对 URL、成功落 keyring、重验进 ready、失败如实呈现、
// 逃生口可切。

describe('AuthGate 远程 setup 流（RemoteSetupView）', () => {
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    __resetDesktopModeForTests();
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });

  /** 进入 setup 态：status 首查 setupRequired:true（未初始化实例）。
   * 返回 fetch 调用记录（提交链断言消费）。 */
  function bootSetupState(
    handler: (url: string, init?: RequestInit) => Response | Promise<Response>
  ): Array<{ url: string; init?: RequestInit }> {
    const calls = stubFetch(handler);
    bootRemote(null);
    renderGate();
    return calls;
  }

  function fillSetupForm(token: string) {
    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: token } });
    fireEvent.change(screen.getByLabelText('确认令牌'), { target: { value: token } });
  }

  it('setup 提交链：绝对 URL 直连（无 Bearer）→ keyring 保存 → 重验进 ready', async () => {
    let statusCallCount = 0;
    const calls = bootSetupState((url, init) => {
      if (url === `${REMOTE_URL}/api/setup`) {
        // 绝对 URL 直连 + 无 Authorization 头（未初始化实例无令牌可验）
        expect(init?.method).toBe('POST');
        const headers = (init?.headers ?? {}) as Record<string, string>;
        expect(headers['Authorization']).toBeUndefined();
        expect(init?.body).toBe(JSON.stringify({ token: 'setup-token-1' }));
        return jsonResponse(200, { ok: true });
      }
      if (url === `${REMOTE_URL}/api/auth/status`) {
        statusCallCount += 1;
        // #1 gate 首查（空令牌）⇒ setupRequired；#2 重验（新令牌 Bearer）⇒ 通过
        const isRecheck = statusCallCount >= 2;
        return jsonResponse(200, { setupRequired: !isRecheck, authenticated: isRecheck });
      }
      return jsonResponse(404, { error: 'not found' });
    });

    expect(await screen.findByTestId('remote-setup-view')).toBeInTheDocument();
    fillSetupForm('setup-token-1');
    fireEvent.click(screen.getByRole('button', { name: /完成初始化并进入/ }));

    // 令牌即主令牌：写 keyring（mode 文件幂等重写 remote + URL）
    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalledWith({
        mode: 'remote',
        remoteUrl: REMOTE_URL,
        token: 'setup-token-1',
      });
    });
    // 重验通过（getAuthStatus 以新令牌 Bearer 判定）⇒ ready（children 挂载）
    expect(await screen.findByTestId(APP_MARKER)).toBeInTheDocument();
    // 传输换令牌（就地）——重验请求携带新令牌 Bearer
    const statusCalls = calls.filter(c => c.url === `${REMOTE_URL}/api/auth/status`);
    expect(statusCalls.length).toBeGreaterThanOrEqual(2);
    const recheck = statusCalls[statusCalls.length - 1]!;
    expect((recheck.init!.headers as Record<string, string>)['Authorization']).toBe(
      'Bearer setup-token-1'
    );
    expect(getTransportBoot()?.remoteToken).toBe('setup-token-1');
  });

  it('setup 404（实例已初始化/env 锁定）：如实呈现 + 返回登录入口切到令牌重录', async () => {
    bootSetupState(url => {
      if (url === `${REMOTE_URL}/api/setup`) {
        return jsonResponse(404, { error: 'not found' });
      }
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: true, authenticated: false });
      }
      return jsonResponse(404, { error: 'not found' });
    });

    expect(await screen.findByTestId('remote-setup-view')).toBeInTheDocument();
    fillSetupForm('setup-token-1');
    fireEvent.click(screen.getByRole('button', { name: /完成初始化并进入/ }));

    // 统一锁定说明（env 锁定态与已初始化态不可区分——不泄露实例状态）
    expect(
      await screen.findByText('实例已初始化或已由环境变量锁定，无法再次初始化。')
    ).toBeInTheDocument();
    // keyring 零残留（失败路径不保存）
    expect(shellMocks.saveConfig).not.toHaveBeenCalled();

    // 返回登录入口 → 令牌重录视图（RemoteLoginView）
    fireEvent.click(screen.getByRole('button', { name: '返回登录' }));
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();
  });

  it('setup 网络失败：断网文案如实呈现（不误报令牌问题）', async () => {
    bootSetupState(url => {
      if (url === `${REMOTE_URL}/api/setup`) {
        return Promise.reject(new TypeError('network down'));
      }
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: true, authenticated: false });
      }
      return jsonResponse(404, { error: 'not found' });
    });

    expect(await screen.findByTestId('remote-setup-view')).toBeInTheDocument();
    fillSetupForm('setup-token-1');
    fireEvent.click(screen.getByRole('button', { name: /完成初始化并进入/ }));

    // 网络错误≠令牌问题（严格分流的 setup 侧落点）
    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();
    expect(shellMocks.saveConfig).not.toHaveBeenCalled();
    // 失败可重试（表单仍在）
    expect(screen.getByRole('button', { name: /完成初始化并进入/ })).toBeInTheDocument();
  });

  it('setup 态切回本地逃生口：save(mode=local, 保留 URL) + restart（I/O 矩阵「可中途切回本地」）', async () => {
    bootSetupState(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: true, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );

    expect(await screen.findByTestId('remote-setup-view')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '切回本地模式' }));

    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalledWith({
        mode: 'local',
        remoteUrl: REMOTE_URL,
        token: null,
      });
      expect(shellMocks.restart).toHaveBeenCalled();
    });
    // 调用序：先 save 后 restart（持久化先行——重启进新模式的前提）
    const saveOrder = shellMocks.saveConfig.mock.invocationCallOrder[0]!;
    const restartOrder = shellMocks.restart.mock.invocationCallOrder[0]!;
    expect(saveOrder).toBeLessThan(restartOrder);
  });
});
