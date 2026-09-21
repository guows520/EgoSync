// Story 16.3：远程模式设置 tab 流程测试（spec Tasks「测试三面」之设置 tab 流程）。
//
// 覆盖（I/O 矩阵「本地→远程切换」行 + AC 诚实代价文案）：
// - local 态：URL+令牌+测试连接+切换；测试失败（令牌无效/setupRequired/
//   断网）⇒ 切换按钮禁用+原因；测试通过 ⇒ 可切；
// - 切换确认：诚实代价文案呈现（本地引擎停机/零本地写入/永不合并同步/
//   重启）；确认 ⇒ save(remote)+restart 调用序；取消 ⇒ 零调用；
// - remote 态：连接信息 + 切回本地（确认 ⇒ save(local, 保留 URL)+restart）；
// - 令牌不落明文回显（password input）。

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RemoteModeSection } from './RemoteModeSection';
import { __resetTransportForTests, setTransportBoot } from '@/transport';
import { setDesktopMode, __resetDesktopModeForTests } from '../../appMode';

const REMOTE_URL = 'https://instance.example.com';

/** 壳命令 mock（desktopModeService 直连 @tauri-apps/api/core）。 */
const shellMocks = vi.hoisted(() => ({
  saveConfig: vi.fn(),
  restart: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === 'remote_mode_save_config') return shellMocks.saveConfig(args) ?? Promise.resolve();
    if (cmd === 'remote_mode_restart') return shellMocks.restart() ?? Promise.resolve();
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

function fillLocalForm(url: string, token: string) {
  fireEvent.change(screen.getByLabelText('远程实例地址'), { target: { value: url } });
  fireEvent.change(screen.getByLabelText('访问令牌'), { target: { value: token } });
}

describe('RemoteModeSection（local 态——配置与切换）', () => {
  beforeEach(() => {
    __resetTransportForTests();
    __resetDesktopModeForTests();
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
    // 默认 Tauri 桩（桌面宿主）+ local 引导（URL 预填）
    setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
    setDesktopMode('local');
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });

  it('local 态表单：URL/令牌输入 + 切换按钮禁用（未测试连接前）+ 提示', () => {
    render(<RemoteModeSection />);
    expect(screen.getByTestId('remote-mode-section')).toHaveAttribute('data-mode', 'local');
    expect(screen.getByLabelText('远程实例地址')).toBeInTheDocument();
    expect(screen.getByLabelText('访问令牌')).toBeInTheDocument();
    const switchBtn = screen.getByRole('button', { name: /切换到远程模式/ });
    expect(switchBtn).toBeDisabled();
    expect(screen.getByTestId('remote-mode-switch-hint')).toHaveTextContent('填写实例地址与令牌并测试连接通过后');
  });

  it('URL 预填：模式文件持久值回填输入框', () => {
    // beforeEach 已注入 boot(null)——重注入前复位（boot 进程内恒定的测试语义）
    __resetTransportForTests();
    setTransportBoot({ mode: 'local', remoteUrl: REMOTE_URL, remoteToken: null });
    setDesktopMode('local');
    render(<RemoteModeSection />);
    expect(screen.getByLabelText('远程实例地址')).toHaveValue(REMOTE_URL);
  });

  it('令牌不落明文回显：password input', () => {
    render(<RemoteModeSection />);
    expect(screen.getByLabelText('访问令牌')).toHaveAttribute('type', 'password');
  });

  it('测试连接成功：结果呈现 + 切换按钮启用（Bearer 判定走绝对 URL）', async () => {
    const calls = stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: true });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'primary-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));

    const result = await screen.findByTestId('remote-mode-test-result');
    expect(result).toHaveTextContent('连接成功：令牌有效。');
    expect(screen.getByRole('button', { name: /切换到远程模式/ })).toBeEnabled();

    // Bearer 判定走绝对 URL + 候选令牌头
    const call = calls.find(c => c.url === `${REMOTE_URL}/api/auth/status`);
    expect(call).toBeDefined();
    expect((call!.init!.headers as Record<string, string>)['Authorization']).toBe('Bearer primary-token');
  });

  it('测试失败（令牌无效）：切换禁用 + 原因（I/O 矩阵「测试失败→切换按钮禁用+原因」）', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: false });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'wrong-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));

    const result = await screen.findByTestId('remote-mode-test-result');
    expect(result).toHaveTextContent('令牌无效或权限不足');
    expect(screen.getByRole('button', { name: /切换到远程模式/ })).toBeDisabled();
    expect(screen.getByTestId('remote-mode-switch-hint')).toHaveTextContent('暂不可切换');
  });

  it('测试失败（实例未初始化 setupRequired）：如实原因（需先完成初始化）', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: true, authenticated: false });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'any-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));

    const result = await screen.findByTestId('remote-mode-test-result');
    expect(result).toHaveTextContent('尚未初始化');
    expect(screen.getByRole('button', { name: /切换到远程模式/ })).toBeDisabled();
  });

  it('测试网络失败：无法连接文案（不误报令牌问题——网络错误≠401 严格分流）', async () => {
    stubFetch(() => Promise.reject(new TypeError('network down')));
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'any-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));

    const result = await screen.findByTestId('remote-mode-test-result');
    expect(result).toHaveTextContent('无法连接到实例');
    expect(screen.getByRole('button', { name: /切换到远程模式/ })).toBeDisabled();
  });

  it('切换流程：诚实代价确认（四条冻结款文案）→ save(remote)+restart 调用序', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: true });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'primary-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));
    await screen.findByTestId('remote-mode-test-result');

    fireEvent.click(screen.getByRole('button', { name: /切换到远程模式/ }));
    const confirm = await screen.findByTestId('remote-mode-switch-confirm');
    // 诚实代价文案（AC 冻结款——引擎停机/零本地写入/永不合并/重启）
    expect(confirm.textContent).toContain('本地引擎将完全停止');
    expect(confirm.textContent).toContain('不会在本地写入任何业务数据');
    expect(confirm.textContent).toContain('绝不会合并或同步');
    expect(confirm.textContent).toContain('重启');

    fireEvent.click(screen.getByRole('button', { name: /确认切换并重启/ }));
    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalledWith({
        mode: 'remote',
        remoteUrl: REMOTE_URL,
        token: 'primary-token',
      });
      expect(shellMocks.restart).toHaveBeenCalled();
    });
    const saveOrder = shellMocks.saveConfig.mock.invocationCallOrder[0]!;
    const restartOrder = shellMocks.restart.mock.invocationCallOrder[0]!;
    // 持久化先行——重启进新模式的前提
    expect(saveOrder).toBeLessThan(restartOrder);
  });

  it('切换取消：零变更零重启（I/O 矩阵「切换取消」行）', async () => {
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: true });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'primary-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));
    await screen.findByTestId('remote-mode-test-result');

    fireEvent.click(screen.getByRole('button', { name: /切换到远程模式/ }));
    await screen.findByTestId('remote-mode-switch-confirm');
    fireEvent.click(screen.getByRole('button', { name: '取消' }));

    // 确认框关闭；save/restart 零调用
    expect(screen.queryByTestId('remote-mode-switch-confirm')).not.toBeInTheDocument();
    expect(shellMocks.saveConfig).not.toHaveBeenCalled();
    expect(shellMocks.restart).not.toHaveBeenCalled();
  });

  it('切换失败（[评审轮2 U24]）：save reject ⇒ 错误可见 + 按钮复位（不卡「正在重启...」）', async () => {
    // T3 同款吞错 bug 在设置 tab 路径的防线——keyring 不可用等失败
    // 必须复位可重试，确认框关闭，错误如实呈现
    stubFetch(url => {
      if (url === `${REMOTE_URL}/api/auth/status`) {
        return jsonResponse(200, { setupRequired: false, authenticated: true });
      }
      return jsonResponse(404, { error: 'not found' });
    });
    shellMocks.saveConfig.mockRejectedValueOnce(new Error('keyring 不可用'));
    render(<RemoteModeSection />);
    fillLocalForm(REMOTE_URL, 'primary-token');
    fireEvent.click(screen.getByRole('button', { name: /测试连接/ }));
    await screen.findByTestId('remote-mode-test-result');

    fireEvent.click(screen.getByRole('button', { name: /切换到远程模式/ }));
    await screen.findByTestId('remote-mode-switch-confirm');
    fireEvent.click(screen.getByRole('button', { name: /确认切换并重启/ }));

    expect(await screen.findByText(/切换失败：keyring 不可用/)).toBeInTheDocument();
    // 确认框关闭（错误态呈现，不再停留确认界面）
    expect(screen.queryByTestId('remote-mode-switch-confirm')).not.toBeInTheDocument();
    // 按钮复位：不卡在禁用态「正在重启...」（可重试）
    expect(screen.getByRole('button', { name: /切换到远程模式/ })).toBeEnabled();
    expect(shellMocks.restart).not.toHaveBeenCalled();
  });

  it('URL 形态预校验：无 scheme ⇒ 测试按钮禁用', () => {
    render(<RemoteModeSection />);
    fillLocalForm('instance.example.com', 'token');
    expect(screen.getByRole('button', { name: /测试连接/ })).toBeDisabled();
  });
});

describe('RemoteModeSection（remote 态——连接信息与切回）', () => {
  beforeEach(() => {
    __resetTransportForTests();
    __resetDesktopModeForTests();
    shellMocks.saveConfig.mockReset().mockResolvedValue(undefined);
    shellMocks.restart.mockReset().mockResolvedValue(undefined);
    setTransportBoot({ mode: 'remote', remoteUrl: REMOTE_URL, remoteToken: 'stored' });
    setDesktopMode('remote');
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });

  it('remote 态：连接信息呈现（URL + 运行中说明）；不渲染配置表单', () => {
    render(<RemoteModeSection />);
    expect(screen.getByTestId('remote-mode-section')).toHaveAttribute('data-mode', 'remote');
    expect(screen.getByTestId('remote-mode-current-url')).toHaveTextContent(REMOTE_URL);
    expect(screen.getByText('远程模式运行中')).toBeInTheDocument();
    // 不渲染配置表单（模式恒定——不可在本态改目标）
    expect(screen.queryByLabelText('远程实例地址')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('访问令牌')).not.toBeInTheDocument();
  });

  it('切回本地：确认文案（本地引擎重启/不合并同步）→ save(local, 保留 URL)+restart', async () => {
    render(<RemoteModeSection />);
    fireEvent.click(screen.getByRole('button', { name: /切回本地模式/ }));

    const confirmText = await screen.findByText(/确认切回本地模式/);
    expect(confirmText.textContent).toContain('以本地引擎启动');
    expect(confirmText.textContent).toContain('不会做任何合并或同步');

    fireEvent.click(screen.getByRole('button', { name: /确认并重启/ }));
    await waitFor(() => {
      expect(shellMocks.saveConfig).toHaveBeenCalledWith({
        mode: 'local',
        remoteUrl: REMOTE_URL,
        token: null,
      });
      expect(shellMocks.restart).toHaveBeenCalled();
    });
  });

  it('切回本地取消：零调用', async () => {
    render(<RemoteModeSection />);
    fireEvent.click(screen.getByRole('button', { name: /切回本地模式/ }));
    await screen.findByText(/确认切回本地模式/);
    fireEvent.click(screen.getByRole('button', { name: '取消' }));

    expect(shellMocks.saveConfig).not.toHaveBeenCalled();
    expect(shellMocks.restart).not.toHaveBeenCalled();
  });

  it('切回本地失败（[评审轮2 U24]）：save reject ⇒ 错误可见 + 按钮复位', async () => {
    shellMocks.saveConfig.mockRejectedValueOnce({ SidecarError: 'keyring 不可用' });
    render(<RemoteModeSection />);
    fireEvent.click(screen.getByRole('button', { name: /切回本地模式/ }));
    await screen.findByText(/确认切回本地模式/);
    fireEvent.click(screen.getByRole('button', { name: /确认并重启/ }));

    // [评审轮2 U15] AppError 单键对象 reject ⇒ 首值可见（非 [object Object]）
    expect(await screen.findByText(/切回本地失败：keyring 不可用/)).toBeInTheDocument();
    // 按钮复位（不卡「正在重启...」永久禁用——可重试）
    expect(screen.getByRole('button', { name: /切回本地模式/ })).toBeEnabled();
    expect(shellMocks.restart).not.toHaveBeenCalled();
  });
});
