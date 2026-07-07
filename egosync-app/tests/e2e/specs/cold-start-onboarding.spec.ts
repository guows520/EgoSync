import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/app-helper.js';

describe('冷启动引导旅程', () => {
  before(async () => {
    await waitForAppReady();
  });

  it('首次启动应显示 Onboarding 视图', async () => {
    // 等待 onboarding 视图加载 — 可能是 "正在准备..." 或 "欢迎使用 EgoSync"
    await browser.waitUntil(
      async () => {
        const preparing = await $('div=正在准备...');
        const welcome = await $('h2=欢迎使用 EgoSync');
        const butlerHeader = await $('h2=分身管家');
        return (await preparing.isExisting()) || (await welcome.isExisting()) || (await butlerHeader.isExisting());
      },
      { timeout: 20000, timeoutMsg: 'Onboarding view did not load' },
    );
    const body = await $('body');
    expect(await body.isDisplayed()).toBe(true);
  });

  it('应显示欢迎文案或配置入口', async () => {
    const configButton = await $('button=配置 AI 模型');
    const chatInput = await $('input[type="text"]');
    const hasConfig = await configButton.isExisting();
    const hasChat = await chatInput.isExisting();
    expect(hasConfig || hasChat).toBe(true);
  });

  it('输入框应可输入文字', async () => {
    const chatInput = await $('input[type="text"]');
    if (await chatInput.isExisting()) {
      await chatInput.setValue('测试用户');
      const value = await chatInput.getValue();
      expect(value).toBe('测试用户');
    }
  });

  it('应用窗口标题应为 EgoSync', async () => {
    const title = await browser.getTitle();
    expect(title).toContain('EgoSync');
  });
});
