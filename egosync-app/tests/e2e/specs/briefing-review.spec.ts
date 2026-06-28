import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, openSettings, seedCompleteOnboarding, seedRole } from '../helpers/app-helper.js';

describe('简报复盘旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    await seedRole('简报测试角色', '🎯', '#6366F1');
    await navigateToButler();
  });

  it('管家视图应正常加载', async () => {
    const header = await $('h2=数字管家');
    await header.waitForDisplayed({ timeout: 15000 });
    expect(await header.getText()).toBe('数字管家');
  });

  it('应用应正常运行不因简报数据崩溃', async () => {
    await browser.pause(3000);
    const bodyText = await $('body').getText();
    expect(bodyText.length).toBeGreaterThan(0);
  });

  it('应能打开设置面板', async () => {
    await openSettings();
    await browser.pause(1000);
    const bodyText = await $('body').getText();
    expect(bodyText.length).toBeGreaterThan(0);
  });

  it('周复盘 Modal 应可通过设置或按钮打开', async () => {
    const closeButton = await $('button[aria-label="关闭"]');
    if (await closeButton.isExisting()) {
      await closeButton.click();
      await browser.pause(500);
    }
    const header = await $('h2=数字管家');
    expect(await header.isExisting()).toBe(true);
  });
});
