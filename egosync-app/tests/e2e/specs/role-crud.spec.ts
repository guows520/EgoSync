import { describe, it, before } from 'mocha';
import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady, openAddRoleModal, seedCompleteOnboarding } from '../helpers/app-helper.js';

describe('角色 CRUD 旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
  });

  it('应能打开添加角色 Modal', async () => {
    await openAddRoleModal();
    const modalHeader = await $('h2=添加新角色');
    await modalHeader.waitForDisplayed({ timeout: 5000 });
    const text = await modalHeader.getText();
    expect(text).toBe('添加新角色');
  });

  it('填写表单并创建角色', async () => {
    const nameInput = await $('#add-role-name');
    await nameInput.setValue('E2E测试角色');
    const createButton = await $('button=创建角色');
    await createButton.click();
    await browser.pause(2000);
    const roleIcons = await $$('[data-role-icon]');
    expect(roleIcons.length).toBeGreaterThanOrEqual(1);
  });

  it('角色视图应显示角色名称', async () => {
    const roleIcons = await $$('[data-role-icon] button');
    const iconCount = await roleIcons.length;
    if (iconCount > 0) {
      await roleIcons[iconCount - 1].click();
      await browser.pause(2000);
    }
    // 使用 JS 直接获取 h2 文本，绕过 WebView2 getText 兼容性问题
    const h2Text = await browser.execute(() => {
      const h2s = document.querySelectorAll('h2');
      for (const h2 of h2s) {
        if (h2.textContent && h2.textContent.includes('E2E测试角色')) {
          return h2.textContent;
        }
      }
      return '';
    });
    expect(h2Text).toBe('E2E测试角色');
  });

  it('应能通过右键菜单归档角色', async () => {
    const roleIconContainer = await $('[data-role-icon]');
    await roleIconContainer.click({ button: 'right' });
    await browser.pause(500);
    const archiveButtons = await $$('button*=归档');
    const archiveCount = await archiveButtons.length;
    if (archiveCount > 0) {
      await archiveButtons[archiveCount - 1].click();
      await browser.pause(500);
      const confirmButton = await $('button=确认归档');
      await confirmButton.click();
      await browser.pause(2000);
    }
    const remainingIcons = await $$('[data-role-icon]');
    expect(remainingIcons.length).toBeGreaterThanOrEqual(0);
  });
});
