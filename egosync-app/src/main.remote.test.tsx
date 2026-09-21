// Story 16.3：main.tsx 渲染前异步引导测试（spec Tasks「测试三面」之
// 引导/切换序）。
//
// 覆盖：
// - Tauri 宿主：desktop_get_boot_config 壳命令 → setTransportBoot +
//   appMode 注入（remote / local 两态）→ render；
// - 壳命令失败（IPC 异常）：fail-safe 回退 local（与 desktop-mode.json
//   缺失同语义——不让桌面因引导失败拒启）；
// - 浏览器宿主：零壳调用（直接 render，transport 走相对路径）。
//
// 范式：vi.resetModules + 动态 import('./main')（入口模块 import 时执行
// bootstrap().finally(render)）。**断言模块同样动态解析**——resetModules
// 后静态导入与 main.tsx 的模块图是不同实例（单例态互不可见），动态
// import 与 main.tsx 共享同一新实例；**宿主环境先行**（appMode 缺省态在
// 模块加载时按 isTauriHost() 求值——先删桩再导入）。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

/** 壳命令 mock。 */
const shellMock = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shellMock.invoke }));

/** 窗口 API mock（[评审轮2 U1] 远程引导显示窗口——jsdom 无窗口 API，
 * 与 App.test.tsx 同款桩）。 */
const windowMock = vi.hoisted(() => ({ show: vi.fn(() => Promise.resolve()) }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ show: windowMock.show }),
}));

/** ReactDOM.createRoot mock（捕获 render 的元素树）。 */
const domMock = vi.hoisted(() => ({ render: vi.fn() }));
vi.mock('react-dom/client', () => ({
  default: { createRoot: () => ({ render: domMock.render }) },
}));

/** 壳命令应答（每用例按需编程）。 */
let bootConfigAnswer: unknown = null;

vi.mock('./services/desktopModeService', () => ({
  desktopGetBootConfig: () => shellMock.invoke('desktop_get_boot_config', bootConfigAnswer),
  remoteModeSaveConfig: (args: unknown) => shellMock.invoke('remote_mode_save_config', args),
  remoteModeRestart: () => shellMock.invoke('remote_mode_restart'),
}));

type TransportModule = typeof import('@/transport');
type TransportHttpModule = typeof import('@/transport/http');
type AppModeModule = typeof import('@/appMode');

/** resetModules + 动态解析断言模块（与后续 import('./main') 同模块图）。 */
async function resolveModules(): Promise<{
  transport: TransportModule;
  http: TransportHttpModule;
  appMode: AppModeModule;
}> {
  vi.resetModules();
  const [transport, http, appMode] = await Promise.all([
    import('@/transport'),
    import('@/transport/http'),
    import('@/appMode'),
  ]);
  return { transport, http, appMode };
}

/** 注入 index.html 形态的 splash 节点（[评审轮2 U21] splash 语义断言）。 */
function injectSplash(): HTMLElement {
  document.getElementById('egosync-splash')?.remove();
  const splash = document.createElement('div');
  splash.id = 'egosync-splash';
  document.body.appendChild(splash);
  return splash;
}

describe('main.tsx 渲染前异步引导（Story 16.3）', () => {
  beforeEach(() => {
    domMock.render.mockClear();
    shellMock.invoke.mockReset();
    windowMock.show.mockClear();
    document.getElementById('egosync-splash')?.remove();
    bootConfigAnswer = null;
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
  });
  afterEach(() => {
    document.getElementById('egosync-splash')?.remove();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
  });

  it('Tauri 宿主 remote 引导：壳命令 → boot 注入（remote/URL/token）→ render', async () => {
    shellMock.invoke.mockImplementation((cmd: string) => {
      if (cmd === 'desktop_get_boot_config') {
        return Promise.resolve({
          mode: 'remote',
          remoteUrl: 'https://instance.example.com',
          remoteToken: 'stored-token',
        });
      }
      return Promise.resolve(null);
    });

    const mods = await resolveModules();
    const splash = injectSplash();
    await import('./main');
    await vi.waitFor(() => {
      expect(domMock.render).toHaveBeenCalled();
    });

    const boot = mods.transport.getTransportBoot();
    expect(boot).toMatchObject({
      mode: 'remote',
      remoteUrl: 'https://instance.example.com',
      remoteToken: 'stored-token',
    });
    expect(mods.appMode.getDesktopMode()).toBe('remote');
    // 传输按 remote 引导构造（远端客户端）
    const instance = mods.transport.getTransport();
    const HttpTransportCtor = mods.http.HttpTransport;
    expect(instance).toBeInstanceOf(HttpTransportCtor);
    expect((instance as InstanceType<typeof HttpTransportCtor>)['remote']).toMatchObject({
      baseUrl: 'https://instance.example.com',
      token: 'stored-token',
    });
    // 壳命令调用面：仅 desktop_get_boot_config
    expect(shellMock.invoke).toHaveBeenCalledWith('desktop_get_boot_config', null);
    // [评审轮2 U1] 远程桌面：引导完成即显示窗口——认证界面发生在 App
    // 挂载之前，App.tsx 的 show() 不可达（gate 未过 ⇒ 窗口永隐藏）
    expect(windowMock.show).toHaveBeenCalledTimes(1);
    // [评审轮2 U12/U21] 正式路径 splash 不由 main.tsx 撤除（AuthGate/
    // App 按各自语义移除——checking 期间品牌屏保持覆盖）
    expect(splash.classList.contains('hidden')).toBe(false);
    expect(splash.parentElement).toBe(document.body);
  });

  it('Tauri 宿主 local 引导：boot 注入 local → render（本地引擎路径）', async () => {
    shellMock.invoke.mockImplementation((cmd: string) => {
      if (cmd === 'desktop_get_boot_config') {
        return Promise.resolve({ mode: 'local', remoteUrl: null, remoteToken: null });
      }
      return Promise.resolve(null);
    });

    const mods = await resolveModules();
    const splash = injectSplash();
    await import('./main');
    await vi.waitFor(() => {
      expect(domMock.render).toHaveBeenCalled();
    });
    expect(mods.transport.getTransportBoot()).toMatchObject({ mode: 'local', remoteUrl: null, remoteToken: null });
    expect(mods.appMode.getDesktopMode()).toBe('local');
    // [评审轮2 U1] 本地态不在引导期显示窗口（App ready 后首显——防首帧
    // 闪白，既有设计零变化）
    expect(windowMock.show).not.toHaveBeenCalled();
    // [评审轮2 U12/U21] splash 不由 main.tsx 撤除（App ready 时自移除）
    expect(splash.classList.contains('hidden')).toBe(false);
  });

  it('壳命令失败：fail-safe 回退 local（desktop-mode.json 缺失同语义——不拒启）', async () => {
    shellMock.invoke.mockRejectedValue(new Error('IPC broken'));

    const mods = await resolveModules();
    await import('./main');
    await vi.waitFor(() => {
      expect(domMock.render).toHaveBeenCalled();
    });
    expect(mods.transport.getTransportBoot()).toMatchObject({ mode: 'local', remoteUrl: null, remoteToken: null });
    expect(mods.appMode.getDesktopMode()).toBe('local');
  });

  it('浏览器宿主：零壳调用 → render（相对路径 transport——既有行为）', async () => {
    // 宿主环境先行：删桩后解析模块（appMode 缺省态按浏览器求值）
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

    const mods = await resolveModules();
    const splash = injectSplash();
    await import('./main');
    await vi.waitFor(() => {
      expect(domMock.render).toHaveBeenCalled();
    });
    expect(shellMock.invoke).not.toHaveBeenCalled();
    expect(mods.transport.getTransportBoot()).toBeNull();
    expect(mods.appMode.getDesktopMode()).toBe('browser');
    // [评审轮2 U21] 浏览器宿主：main.tsx 不撤 splash（16.1 语义——
    // AuthGate 离开 checking 时自移除）也零窗口调用
    expect(splash.classList.contains('hidden')).toBe(false);
    expect(windowMock.show).not.toHaveBeenCalled();
  });
});
