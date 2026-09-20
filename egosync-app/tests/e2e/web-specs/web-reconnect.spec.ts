// Story 16.2：Web 模式断线重连。
//
// 验证链路（HttpTransport 三态机 + 白名单重放）：
// 1. kill 服务端（SIGKILL——模拟断线）→ 徽标 data-state=reconnecting +
//    显著重连横幅出现；
// 2. 同 env 重启服务端 → SSE 重连 → 徽标回到 online + 横幅消失；
// 3. 重连后界面仍可继续工作（重放恢复会话列表/通知等白名单命令）。
//
// Cookie 会话存 DB（30 天）——服务端重启后免重登录（auth 面零请求）。
// 页面全程保持挂载（重连语义不依赖页面刷新）。

import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import {
  openWebAppAndLogin,
  waitForConnectionState,
  reconnectBannerExists,
  killWebServer,
  restartWebServer,
  webInvoke,
  saveWebSmokeScreenshot,
} from '../helpers/web-helper.js';

describe('Web 模式断线重连（Story 16.2）', () => {
  before(async () => {
    await openWebAppAndLogin();
  });

  it('服务端被 kill 后连接状态迁移到 reconnecting 并显示重连横幅', async () => {
    await waitForConnectionState('online');

    await killWebServer();

    await waitForConnectionState('reconnecting', 30000);
    expect(await reconnectBannerExists()).toBe(true);
    // 冒烟场景②留档：拔线重连——reconnecting 横幅 + 徽标警示态
    await saveWebSmokeScreenshot('web-02-reconnecting');
  });

  it('服务端重启后连接状态回到 online 且重连横幅消失', async () => {
    await restartWebServer();

    await waitForConnectionState('online', 60000);
    expect(await reconnectBannerExists()).toBe(false);
  });

  it('重连后界面仍可继续工作（白名单重放恢复 + 命令可用）', async () => {
    // 重连后应用命令面恢复（cookie 会话在 DB——服务端重启后免重登录）
    const marker = `重连后命令面验证-${Date.now()}`;
    await webInvoke('app_set_setting', { key: 'e2e-reconnect-check', value: marker });
    const value = await webInvoke<string>('app_get_setting', { key: 'e2e-reconnect-check' });
    expect(value).toBe(marker);

    // 界面核心元素仍在（页面全程未刷新）
    const badge = await $('[data-testid="connection-status"]');
    expect(await badge.isDisplayed()).toBe(true);
    expect(await badge.getAttribute('data-state')).toBe('online');
  });
});
