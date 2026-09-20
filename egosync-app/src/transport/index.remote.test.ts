// Story 16.3：传输三路解析矩阵（spec Tasks「测试三面」之 transport 解析矩阵）。
//
// 三路：浏览器（相对路径 HttpTransport）/ 桌面 local（TauriTransport）/
// 桌面 remote（HttpTransport 绝对 base + Bearer）。模式进程内恒定：
// setTransportBoot 渲染前注入一次；重复注入拒绝（防热换）；getDesktopMode
// 与 getTransportBoot 的派生语义；updateRemoteToken 就地换令牌。

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  __resetTransportForTests,
  getDesktopMode,
  getTransport,
  getTransportBoot,
  isTauriHost,
  setTransportBoot,
  updateRemoteToken,
} from './index';
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

describe('传输三路解析矩阵（Story 16.3）', () => {
  beforeEach(() => {
    __resetTransportForTests();
  });
  afterEach(() => {
    __resetTransportForTests();
    vi.unstubAllGlobals();
  });

  it('浏览器宿主（无注入）：HttpTransport 相对路径（既有行为零回归）', () => {
    asBrowser();
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
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://cloud.example.com', remoteToken: 'tk' });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(HttpTransport);
    const remote = (transport as HttpTransport)['remote'];
    expect(remote).toMatchObject({ kind: 'remote', baseUrl: 'https://cloud.example.com', token: 'tk' });
    expect(getDesktopMode()).toBe('remote');
  });

  it('桌面 remote 缺令牌：仍构造远程实例（空令牌——Bearer 判定 authenticated:false，不回退 TauriTransport 直通）', () => {
    asTauri();
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://x.example.com', remoteToken: null });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(HttpTransport);
    const remote = (transport as HttpTransport)['remote'];
    expect(remote).toMatchObject({ kind: 'remote', baseUrl: 'https://x.example.com', token: '' });
    expect(getDesktopMode()).toBe('remote');
  });

  it('桌面 remote 缺 URL（模式文件损坏态）：回退 TauriTransport（fail-safe 有数据一侧）', () => {
    asTauri();
    setTransportBoot({ mode: 'remote', remoteUrl: null, remoteToken: null });
    const transport = getTransport();
    expect(transport).toBeInstanceOf(TauriTransport);
    // 模式态如实呈现 remote（UI 层引导面处置——不在传输层伪造）
    expect(getDesktopMode()).toBe('remote');
  });

  it('桌面 local（显式注入）：TauriTransport（模式注入后仍本地）', () => {
    asTauri();
    setTransportBoot({ mode: 'local', remoteUrl: 'https://saved.example.com', remoteToken: null });
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
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://a.example.com', remoteToken: 'tk' });
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

  it('updateRemoteToken：仅 remote boot 生效——boot 记录同步 + 传输换令牌', () => {
    asTauri();
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://c.example.com', remoteToken: 'old' });
    const transport = getTransport() as HttpTransport;
    updateRemoteToken('new');
    expect(getTransportBoot()?.remoteToken).toBe('new');
    expect((transport['remote'] as { token: string }).token).toBe('new');
    // 就地换令牌——不重建实例（订阅面不断裂）
    expect(transport).toBe(getTransport());
  });

  it('updateRemoteToken（非 remote boot）：no-op', () => {
    asTauri();
    setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
    updateRemoteToken('x');
    expect(getTransportBoot()?.mode).toBe('local');
    expect(getTransportBoot()?.remoteToken).toBeNull();
  });

  it('reset 后可重新注入（测试隔离语义：boot 与实例同复位）', () => {
    asTauri();
    setTransportBoot({ mode: 'remote', remoteUrl: 'https://d.example.com', remoteToken: 'tk' });
    __resetTransportForTests();
    expect(getTransportBoot()).toBeNull();
    setTransportBoot({ mode: 'local', remoteUrl: null, remoteToken: null });
    expect(getTransportBoot()?.mode).toBe('local');
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
