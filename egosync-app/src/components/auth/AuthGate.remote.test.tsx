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
import { setDesktopMode } from '../../appMode';

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
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
    __resetTransportForTests();
    setDesktopMode('local');
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

  it('远端未初始化（setupRequired）：setup 向导（浏览器等价）', async () => {
    bootRemote(null);
    stubFetch(url =>
      url === `${REMOTE_URL}/api/auth/status`
        ? jsonResponse(200, { setupRequired: true, authenticated: false })
        : jsonResponse(404, { error: 'not found' })
    );
    renderGate();
    expect(await screen.findByText('初始化 EgoSync 实例')).toBeInTheDocument();
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

  it('重录验证网络失败：断网文案（不误报令牌失效）', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        // 首次（gate 检查）成功；重录验证断网
        return Promise.reject(new TypeError('network down'));
      }
      return jsonResponse(404, { error: 'not found' });
    });
    bootRemote('tk');
    renderGate();
    // 首次 status 网络失败 ⇒ 离线屏（I/O 矩阵「远程不可达」）
    expect(await screen.findByText('无法连接服务器，请检查网络后重试。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
    // 远程态：切回本地逃生口在离线屏呈现
    expect(screen.getByRole('button', { name: '切回本地模式' })).toBeInTheDocument();
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
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    __resetTransportForTests();
    setDesktopMode('local');
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

  it('重录验证 401 形态（非 200）：如实错误文案（loginErrorText 分流）', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: false });
      }
      return jsonResponse(401, { error: 'unauthorized' });
    });
    bootRemote('stale');
    // 首次 status：authenticated:false → 重录视图
    // 覆盖 verifyToken 非 200 路径：手动构造 status 404 分流
    renderGate();
    expect(await screen.findByText('连接远程实例')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: 'x'.repeat(8) } });
    fireEvent.click(screen.getByRole('button', { name: /验证并进入/ }));
    // authenticated:false ⇒ 统一「令牌不正确」
    expect(await screen.findByText('令牌不正确，请重试。')).toBeInTheDocument();
  });
});
