import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding } from '../helpers/app-helper.js';

describe('管家对话旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    await navigateToButler();
  });

  it('管家视图应显示数字管家标题', async () => {
    const header = await $('h2=数字管家');
    await header.waitForDisplayed({ timeout: 15000 });
    const text = await header.getText();
    expect(text).toBe('数字管家');
  });

  it('应显示聊天输入框', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.waitForDisplayed({ timeout: 10000 });
    expect(await chatInput.isDisplayed()).toBe(true);
  });

  it('用户输入消息后应显示用户消息气泡', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.setValue('这是一条测试消息');
    const sendButton = await $('button[aria-label="发送"]');
    await sendButton.click();
    await browser.pause(1000);
    const bodyText = await $('body').getText();
    expect(bodyText).toContain('这是一条测试消息');
  });
});
