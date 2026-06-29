import { describe, it, before } from 'mocha';
import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady, seedCompleteOnboarding, seedRole, seedTask, openAddRoleModal, openSettings } from '../helpers/app-helper.js';
import { runAxeScan } from '../helpers/a11y-helper.js';

describe('WCAG 2.1 AA 无障碍扫描', () => {
  before(async () => {
    await waitForAppReady();
  });

  it('冷启动 Onboarding 视图无 critical/serious axe 问题', async () => {
    await runAxeScan('onboarding');
  });

  it('管家主视图无 critical/serious axe 问题', async () => {
    await seedCompleteOnboarding();
    await runAxeScan('butler-main');
  });

  it('角色对话视图无 critical/serious axe 问题', async () => {
    await seedCompleteOnboarding();
    await seedRole('无障碍测试角色', 'briefcase', '#6366F1');
    await browser.refresh();
    await waitForAppReady();

    const roleIcons = await $$('[data-role-icon] button');
    const iconCount = await roleIcons.length;
    if (iconCount > 0) {
      await roleIcons[iconCount - 1].click();
      await browser.pause(1000);
    }
    await runAxeScan('role-chat');
  });

  it('任务管理视图无 critical/serious axe 问题', async () => {
    await seedCompleteOnboarding();
    const roleId = await seedRole('任务测试角色', 'briefcase', '#6366F1');
    await seedTask(roleId, '无障碍测试任务', 'Q2');
    await browser.refresh();
    await waitForAppReady();

    const roleIcons = await $$('[data-role-icon] button');
    const iconCount = await roleIcons.length;
    if (iconCount > 0) {
      await roleIcons[iconCount - 1].click();
      await browser.pause(1000);
    }

    const taskTab = await $('button=任务');
    if (await taskTab.isExisting()) {
      await taskTab.click();
      await browser.pause(1000);
    }
    await runAxeScan('role-tasks');
  });

  it('添加角色 Modal 无 critical/serious axe 问题', async () => {
    await seedCompleteOnboarding();
    await openAddRoleModal();
    await browser.pause(500);
    await runAxeScan('add-role-modal');
  });

  it('设置面板无 critical/serious axe 问题', async () => {
    await seedCompleteOnboarding();
    await openSettings();
    await browser.pause(500);
    await runAxeScan('settings');
  });
});
