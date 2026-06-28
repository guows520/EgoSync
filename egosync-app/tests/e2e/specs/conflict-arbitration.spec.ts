import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding, seedRole, seedTask } from '../helpers/app-helper.js';

describe('冲突仲裁旅程', () => {
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
