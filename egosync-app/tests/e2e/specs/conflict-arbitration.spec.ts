import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding, seedRole, seedTask } from '../helpers/app-helper.js';

// ⚠️ 限制说明（AC2.6）：冲突仲裁特性（Stories 5-3~5-6：时间冲突检测/三步仲裁/仲裁 Modal）
// 已在 sprint-status 中标记为 deferred-v2，应用当前未实现仲裁 Modal 及三步展示。
// 因此本 spec 无法验证「打开仲裁 Modal → 三步展示」，仅验证：预置冲突任务数据后
// 应用不崩溃、核心导航（管家/通知）仍可用。仲裁旅程的真实验证待 V2 仲裁特性落地后补齐。
describe('冲突仲裁旅程（仲裁特性 deferred-v2：仅验证健壮性）', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();

    const roleA = await seedRole('角色A', '🎯', '#6366F1');
    const roleB = await seedRole('角色B', '💡', '#F59E0B');

    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    tomorrow.setHours(14, 0, 0, 0);
    const conflictTime = tomorrow.toISOString();

    await seedTask(roleA, '角色A的会议', 'Q1', false, conflictTime);
    await seedTask(roleB, '角色B的会议', 'Q1', false, conflictTime);

    await navigateToButler();
  });

  it('管家视图应正常加载', async () => {
    const header = await $('h2=数字管家');
    await header.waitForDisplayed({ timeout: 15000 });
    expect(await header.getText()).toBe('数字管家');
  });

  it('应用应正常运行不因冲突数据崩溃', async () => {
    await browser.pause(2000);
    const bodyText = await $('body').getText();
    expect(bodyText.length).toBeGreaterThan(0);
  });

  it('通知面板应可打开', async () => {
    const notifButton = await $('button[title="通知"]');
    if (await notifButton.isExisting()) {
      await notifButton.click();
      await browser.pause(1000);
      await notifButton.click();
    }
    expect(await notifButton.isExisting()).toBe(true);
  });
});
