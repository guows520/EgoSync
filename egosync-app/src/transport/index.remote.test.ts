// Story 16.3：传输三路解析矩阵（spec Tasks「测试三面」之 transport 解析矩阵）。
//
// 三路：浏览器（相对路径 HttpTransport）/ 桌面 local（TauriTransport）/
// 桌面 remote（HttpTransport 绝对 base + Bearer）。模式进程内恒定：
// setTransportBoot 渲染前注入一次；重复注入拒绝（防热换）。
//
// [T4 修订] 模式态单一事实源 = appMode：transport 侧 getDesktopMode 重复
// 导出已撤销——本文件的模式断言改经 `../appMode` 取值，注入方式与
// main.tsx 同款（setTransportBoot + setDesktopMode 成对）。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  __resetTransportForTests,
  getTransport,
  getTransportBoot,
  isTauriHost,
  setTransportBoot,
  updateRemoteToken,
} from './index';
import {
  __resetDesktopModeForTests,
  getDesktopMode,
  setDesktopMode,
} from '../appMode';
import { HttpTransport } from './http';
import { TauriTransport } from './tauri';

/** 全局 Tauri 桩快照（beforeEach 删 / afterEach 恢复——AuthGate.test 同款）。 */
const tauriInternals = (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

function asTauri() {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = tauriInternals;
}
function asBrowser() {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
}

/** 与 main.tsx 引导同款：transport boot 与 appMode 成对注入。 */
function boot(config: Parameters<typeof setTransportBoot>[0]) {
  setTransportBoot(config);
  setDesktopMode(config.mode === 'remote' ? 'remote' : 'local');
}

describe('传输三路解析矩阵（Story 16.3）', () => {
  beforeEach(() => {
    __resetTransportForTests();
    __resetDesktopModeForTests();
  });
  afterEach(() => {
    // 宿主复位先行（appMode 缺省态按宿主推导——上一用例删桩的浏览器
    // 环境若残留，复位会把 'browser' 钉进模块态）
    asTauri();
    __resetTransportForTests();
    __resetDesktopModeForTests();
    vi.unstubAllGlobals();
  });

  it('浏览器宿主（无注入）：HttpTransport 相对路径（既有行为零回归）', () => {
    asBrowser();
    __resetDesktopModeForTests();
    const transport = getTransport();
    expect(transport).toBeInstanceOf(HttpTransport);
    expect((transport as HttpTransport)['remote']).toBeNull();
    expect(getDesktopMode()).toBe('browser');
    expect(getTransportBoot()).toBeNull();
  });

  it('桌面宿主（无注入）：TauriTransport（缺省 fail-safe local——desktop-mode.json 缺失同语义）', () => {
    asTauri();
    const transport = getTransport();
    expect(transport).toBeInstanceOf(TauriTransport);
    expect(getDesktopMode()).toBe('local');
    expect(getTransportBoot()).toBeNull();
  });

  it('桌面 remote（注入 mode:remote + URL + 令牌）：HttpTransport 指向远端', () => {
    asTauri();
    boot({ mode: 'remote', remoteUrl: 'https://cloud.example.com', remoteToken: 'tk' });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(HttpTransport);
    const remote = (transport as HttpTransport)['remote'];
    expect(remote).toMatchObject({ kind: 'remote', baseUrl: 'https://cloud.example.com', token: 'tk' });
    expect(getDesktopMode()).toBe('remote');
  });

  it('桌面 remote 缺令牌：仍构造远程实例（空令牌——Bearer 判定 authenticated:false，不回退 TauriTransport 直通）', () => {
    asTauri();
    boot({ mode: 'remote', remoteUrl: 'https://x.example.com', remoteToken: null });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(HttpTransport);
    const remote = (transport as HttpTransport)['remote'];
    expect(remote).toMatchObject({ kind: 'remote', baseUrl: 'https://x.example.com', token: '' });
    expect(getDesktopMode()).toBe('remote');
  });

  it('桌面 remote 缺 URL（[T2] 读侧统一裁决后为纵深防御态）：回退 TauriTransport（fail-safe 有数据一侧）', () => {
    // [T2] Rust 读侧（read_mode/desktop_get_boot_config 同源）已把
    // remote 缺 URL 裁决为 local——本形态对真实 boot 通道不可达；传输
    // 层回退保留为纵深防御（模式态如实呈现 remote，UI 层引导处置）。
    asTauri();
    boot({ mode: 'remote', remoteUrl: null, remoteToken: null });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(TauriTransport);
    expect(getDesktopMode()).toBe('remote');
  });

  it('桌面 local（显式注入）：TauriTransport（模式注入后仍本地）', () => {
    asTauri();
    boot({ mode: 'local', remoteUrl: 'https://saved.example.com', remoteToken: null });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(TauriTransport);
    expect(getDesktopMode()).toBe('local');
    expect(getTransportBoot()?.remoteUrl).toBe('https://saved.example.com');
  });

  it('浏览器宿主注入 boot：传输仍按宿主裁决（浏览器不受桌面 boot 影响）', () => {
    asBrowser();
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://x.example.com', remoteToken: 'tk' });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(HttpTransport);
    expect((transport as HttpTransport)['remote']).toBeNull();
  });

  it('进程内单例：getTransport 复用首建实例（模式恒定）', () => {
    asTauri();
    boot({ mode: 'remote', remoteUrl: 'https://a.example.com', remoteToken: 'tk' });
    expect(getTransport()).toBe(getTransport());
  });

  it('setTransportBoot 重复注入拒绝（模式恒定——切换必经重启）', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      asTauri();
      setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
      setTransportBoot({ mode: 'remote', remoteUrl: 'https://b.example.com', remoteToken: 'tk2' });
      // 首值生效（local）；二值拒绝
      expect(getTransportBoot()?.mode).toBe('local');
      expect(spy).toHaveBeenCalledTimes(1);
    } finally {
      spy.mockRestore();
    }
  });

  it('setDesktopMode 重复注入拒绝（[T4] 语义对齐 setTransportBoot）', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      asTauri();
      setDesktopMode('remote');
      setDesktopMode('local');
      // 首值生效（remote）；二值拒绝（console.error + 忽略——不静默覆写）
      expect(getDesktopMode()).toBe('remote');
      expect(spy).toHaveBeenCalledTimes(1);
    } finally {
      spy.mockRestore();
    }
  });

  it('updateRemoteToken：仅 remote boot 生效——boot 记录同步 + 传输换令牌', () => {
    asTauri();
    boot({ mode: 'remote', remoteUrl: 'https://c.example.com', remoteToken: 'old' });
    const transport = getTransport() as HttpTransport;
    updateRemoteToken('new');
    expect(getTransportBoot()?.remoteToken).toBe('new');
    expect((transport['remote'] as { token: string }).token).toBe('new');
    // 就地换令牌——不重建实例（订阅面不断裂）
    expect(transport).toBe(getTransport());
  });

  it('updateRemoteToken（非 remote boot）：no-op', () => {
    asTauri();
    boot({ mode: 'local', remoteUrl: null, remoteToken: null });
    updateRemoteToken('x');
    expect(getTransportBoot()?.mode).toBe('local');
    expect(getTransportBoot()?.remoteToken).toBeNull();
  });

  it('reset 后可重新注入（测试隔离语义：boot/实例/模式态同复位）', () => {
    asTauri();
    boot({ mode: 'remote', remoteUrl: 'https://d.example.com', remoteToken: 'tk' });
    __resetTransportForTests();
    __resetDesktopModeForTests();
    expect(getTransportBoot()).toBeNull();
    boot({ mode: 'local', remoteUrl: null, remoteToken: null });
    expect(getTransportBoot()?.mode).toBe('local');
    expect(getDesktopMode()).toBe('local');
  });

  it('isTauriHost 探测不受 boot 影响（宿主与模式正交）', () => {
    asBrowser();
    expect(isTauriHost()).toBe(false);
    setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
    expect(isTauriHost()).toBe(false);
    asTauri();
    expect(isTauriHost()).toBe(true);
  });
});
