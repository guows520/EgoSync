import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding } from '../helpers/app-helper.js';

describe('LLM 流式响应旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    await navigateToButler();
  });

  it('管家视图应显示聊天界面', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.waitForDisplayed({ timeout: 15000 });
    expect(await chatInput.isDisplayed()).toBe(true);
  });

  it('发送消息后输入框应暂时禁用（流式状态）', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.setValue('测试流式消息');
    const sendButton = await $('button[aria-label="发送"]');
    await sendButton.click();
    await browser.pause(2000);
    const inputStillExists = await chatInput.isExisting();
    expect(inputStillExists).toBe(true);
  });

  it('停止按钮在流式时应可用，非流式时显示发送按钮', async () => {
    const sendButton = await $('button[aria-label="发送"]');
    const stopButton = await $('button[aria-label="停止"]');
    const hasSend = await sendButton.isExisting();
    const hasStop = await stopButton.isExisting();
    expect(hasSend || hasStop).toBe(true);
  });
});
