import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding } from '../helpers/app-helper.js';

// ⚠️ 限制说明（AC2.4）：CI 环境无 LLM API Key，opencode sidecar 为占位文件，
// chat_send_message 会立即失败并复位流式状态（见 ChatStream.tsx handleSend 的 catch 分支）。
// 因此「输入禁用/流式光标/停止按钮」等真实流式 UI 在 CI 无法稳定复现（亚秒级竞态）。
// 本 spec 仅验证流式相关的静态 UI 契约（发送按钮存在、点击后输入框仍可用）。
// 真实流式行为验证作为 V2 待办（接入真实 LLM 或注入 mock sidecar 后补齐）。
describe('LLM 流式响应旅程（CI 无 LLM：仅验证静态 UI 契约）', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    await navigateToButler();
  });

  it('管家视图应显示聊天输入框', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.waitForDisplayed({ timeout: 15000 });
    expect(await chatInput.isDisplayed()).toBe(true);
  });

  it('非流式态应显示发送按钮（流式 UI 的静态契约）', async () => {
    const sendButton = await $('button[aria-label="发送"]');
    await sendButton.waitForExist({ timeout: 10000 });
    expect(await sendButton.isExisting()).toBe(true);
  });

  it('点击发送后输入框应保持可用（CI 无 LLM 时发送失败会复位，不应卡死）', async () => {
    const chatInput = await $('input[type="text"]');
    await chatInput.setValue('测试流式消息');
    const sendButton = await $('button[aria-label="发送"]');
    await sendButton.click();
    await browser.pause(2000);
    // 发送失败复位后输入框应恢复可输入（disabled 复位），验证不卡死
    expect(await chatInput.isEnabled()).toBe(true);
  });
});
