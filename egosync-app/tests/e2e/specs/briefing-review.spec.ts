import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, navigateToButler, seedCompleteOnboarding, seedRole } from '../helpers/app-helper.js';

// ⚠️ 限制说明（AC2.7）：
// 1. 周复盘 Modal（WeeklyReviewModal）在应用中仅由后台 `bigrock:reminder` 调度事件触发
//    （App.tsx setIsReviewOpen），没有任何用户可点击的 UI 入口，E2E 无法通过点击打开。
// 2. 晨间简报内容由调度器在指定时间推送（依赖 LLM 生成），CI 无法即时产出真实简报内容。
// 因此本 spec 验证「简报/复盘」相关的可达成契约：管家仪表盘正常加载、管家设置中
// 晨间简报与周复盘的配置项可见可达。真实简报内容渲染与周复盘 Modal 验证为 V2 待办。
describe('简报复盘旅程（节奏化配置可达性 + 仪表盘加载）', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    await seedRole('简报测试角色', '🎯', '#6366F1');
    await browser.refresh();
    await waitForAppReady();
    await navigateToButler();
  });

  it('管家视图应正常加载', async () => {
    const header = await $('h2=数字管家');
    await header.waitForDisplayed({ timeout: 15000 });
    expect(await header.getText()).toBe('数字管家');
  });

  it('仪表盘应渲染角色状态总览', async () => {
    const dashboardTab = await $('button=仪表盘');
    await dashboardTab.click();
    await browser.pause(1500);
    const overview = await $('h3=角色状态总览');
    await overview.waitForDisplayed({ timeout: 10000 });
    expect(await overview.isDisplayed()).toBe(true);
  });

  it('管家设置应可见晨间简报与周复盘的节奏配置项', async () => {
    const settingsTab = await $('button=设置');
    await settingsTab.click();
    await browser.pause(1500);
    const briefingLabel = await $('label=晨间简报时间');
    await briefingLabel.waitForDisplayed({ timeout: 10000 });
    expect(await briefingLabel.isDisplayed()).toBe(true);

    const reviewLabel = await $('label=周复盘时间');
    expect(await reviewLabel.isExisting()).toBe(true);
  });
});
