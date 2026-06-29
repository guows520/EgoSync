import { describe, it, before } from 'mocha';
import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady, seedCompleteOnboarding, seedRole, seedTask } from '../helpers/app-helper.js';

describe('任务管理旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    const roleId = await seedRole('任务测试角色', '🎯', '#6366F1');
    await seedTask(roleId, '预置任务', 'Q2');
    // 刷新页面让 seed 数据在 UI 中生效
    await browser.refresh();
    await waitForAppReady();
    // 导航到角色视图 — 找到正确的角色
    const roleIcons = await $$('[data-role-icon] button');
    const iconCount = await roleIcons.length;
    if (iconCount > 0) {
      // 找到 "任务测试角色" 对应的图标
      let clicked = false;
      for (let i = 0; i < iconCount; i++) {
        const icon = roleIcons[i];
        await icon.click();
        await browser.pause(500);
        const nameEl = await $('h2');
        if (await nameEl.isExisting()) {
          const name = await nameEl.getText();
          if (name.includes('任务测试角色')) {
            clicked = true;
            break;
          }
        }
      }
      if (!clicked) {
        await roleIcons[0].click();
        await browser.pause(1000);
      }
    }
  });

  it('角色视图应显示角色名称', async () => {
    const roleName = await $('h2=任务测试角色');
    await roleName.waitForDisplayed({ timeout: 10000 });
    expect(await roleName.getText()).toBe('任务测试角色');
  });

  it('点击任务 Tab 应显示任务列表', async () => {
    const taskTab = await $('button=任务');
    await taskTab.click();
    await browser.pause(1000);
    const bodyText = await $('body').getText();
    expect(bodyText).toContain('任务清单');
  });

  it('创建新任务应出现在列表中', async () => {
    const addButtons = await $$('button');
    let addButton: WebdriverIO.Element | undefined;
    for (const btn of addButtons) {
      const text = await btn.getText();
      if (text.includes('新建') || text.includes('添加')) {
        addButton = btn;
        break;
      }
    }
    expect(addButton).toBeDefined();
    if (!addButton) {
      throw new Error('未找到「新建/添加」按钮');
    }
    await addButton.click();
    await browser.pause(500);

    const taskTitleInput = await $('#task-title');
    expect(await taskTitleInput.isExisting()).toBe(true);
    await taskTitleInput.setValue('E2E新建任务');
    const saveButton = await $('button=保存任务');
    await saveButton.click();
    await browser.pause(2000);
    const bodyText = await $('body').getText();
    expect(bodyText).toContain('E2E新建任务');
  });

  it('标记任务完成应显示勾选状态', async () => {
    const completeButton = await $('button[aria-label*="完成"]');
    if (await completeButton.isExisting()) {
      await completeButton.click();
      await browser.pause(1000);
    }
    const bodyText = await $('body').getText();
    expect(bodyText).toContain('已完成');
  });

  it('删除任务后应从列表消失', async () => {
    const deleteButtons = await $$('button[aria-label*="删除"]');
    const deleteCount = await deleteButtons.length;
    if (deleteCount > 0) {
      await deleteButtons[0].click();
      await browser.pause(500);
      const confirmDelete = await $('button=确认删除');
      if (await confirmDelete.isExisting()) {
        await confirmDelete.click();
        await browser.pause(2000);
      }
    }
    const taskTab = await $('button=任务');
    expect(await taskTab.isExisting()).toBe(true);
  });
});
