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
  readServerPid,
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

  it('存活态重启真实置换服务端实例（旧 PID 已死、PID 文件指向新实例）', async () => {
    // 评审补丁（17.3 分诊 P1，验证缺口层发现）：restartWebServer 的
    // 「先杀旧实例」修复此前无任何会失败的断言保护——把它整体还原为旧
    // 实现（不杀只起）时，reconnect 冒烟与 resident-loop「同分钟重启」
    // 全部照绿（后者退回历史性空洞通过）。本用例对**存活**服务端执行
    // 重启并断言置换真实发生——孤儿僵尸服务器问题（17.3 验证期根因）
    // 若复发，此处必红。零时钟依赖，属冒烟套件选入标准。
    const oldPid = readServerPid();
    if (oldPid == null) throw new Error('服务端应存活（上一用例已重启并回到 online）');

    await restartWebServer();

    const newPid = readServerPid();
    if (newPid == null) throw new Error('重启后 PID 文件应指向新实例');
    if (newPid === oldPid) throw new Error(`重启必须产生新 PID（同 PID = 未真正重启）: ${newPid}`);

    // 旧 PID 必须已死：signal 0 探活抛 ESRCH 即死透
    let oldPidAlive = true;
    try {
      process.kill(oldPid!, 0);
    } catch {
      oldPidAlive = false;
    }
    expect(oldPidAlive).toBe(false, `旧实例 ${oldPid} 必须被杀死（孤儿 = 下一轮 onPrepare 污染源）`);

    // 置换后连接面照常恢复（新实例接客——与本 spec 主链路闭环）
    await waitForConnectionState('online', 60000);
  });
});
