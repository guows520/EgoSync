// Story 16.2：Web 模式 SSE 单连接多路复用。
//
// 验证链路：REST POST /api/cmd/notification_create（造数据）→ 服务端
// emit notification:new → SSE /api/events 推送 → 前端通知面板实时呈现。
// Cookie 会话由 spec1 登录持久（user-data-dir），本 spec 仅 1 次 status。

import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { openWebAppAndLogin, waitForConnectionState, webInvoke } from '../helpers/web-helper.js';

describe('Web 模式 SSE 事件推送（Story 16.2）', () => {
  before(async () => {
    await openWebAppAndLogin();
  });

  it('服务端命令产生的事件经 SSE 实时到达通知面板', async () => {
    // 1. 先造一个角色（notification_create 校验 role_id 存在）
    const role = await webInvoke<{ id: string; name: string }>('role_create', {
      input: { name: 'SSE验证角色', icon: 'briefcase', color: '#4F46E5' },
    });
    expect(role.id).toBeTruthy();

    // 2. 打开通知面板（侧栏铃铛）
    const bellButton = await $('button[title="通知"]');
    await bellButton.waitForDisplayed({ timeout: 15000 });
    await bellButton.click();
    const panel = await $('[aria-label="通知中心"]');
    await panel.waitForDisplayed({ timeout: 10000 });
    expect(await panel.isDisplayed()).toBe(true);

    // 3. 经 REST 造一条通知 → SSE notification:new → 面板实时出现内容
    const marker = `SSE实时推送验证-${Date.now()}`;
    await webInvoke('notification_create', {
      input: { roleId: role.id, level: 'tap', content: marker },
    });

    const notificationItem = await $(`p=${marker}`);
    await notificationItem.waitForDisplayed({ timeout: 15000 });
    expect(await notificationItem.isDisplayed()).toBe(true);
  });
});
