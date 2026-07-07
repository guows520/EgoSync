import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding } from '../helpers/app-helper.js';

describe('管家对话旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    await navigateToButler();
  });

  it('管家视图应显示分身管家标题', async () => {
    const header = await $('h2=分身管家');
    await header.waitForDisplayed({ timeout: 15000 });
    const text = await header.getText();
    expect(text).toBe('分身管家');
  });

  it('应显示聊天输入框', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.waitForDisplayed({ timeout: 10000 });
    expect(await chatInput.isDisplayed()).toBe(true);
  });

  // ⚠️ 限制说明：CI 无 LLM 时 chat_send_message 立即失败，乐观插入的用户气泡会被
  // catch 分支回滚移除（见 ChatStream.tsx handleSend）。因此不能断言「气泡持久存在」，
  // 改为验证「可输入并发送、发送后界面不卡死且输入框恢复可用」。真实对话往返验证为 V2 待办。
  it('用户可在输入框输入并点击发送，发送后界面不卡死', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.setValue('这是一条测试消息');
    expect(await chatInput.getValue()).toBe('这是一条测试消息');
    const sendButton = await $('button[aria-label="发送"]');
    await sendButton.click();
    // 发送链路完成（成功或失败复位）后，输入框应恢复可用
    // CI 无 LLM 时后端 spawn task 需要走完失败链路才 emit done，用 waitUntil 替代固定 pause
    await browser.waitUntil(async () => await chatInput.isEnabled(), {
      timeout: 30000,
      timeoutMsg: '发送后输入框未在 30 秒内恢复可用',
    });
    expect(await chatInput.isEnabled()).toBe(true);
  });
});
