import { describe, it, before } from 'mocha';
import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady, openAddRoleModal, openSettings, seedCompleteOnboarding, seedRole } from '../helpers/app-helper.js';

// 当前激活的角色名（创建→改名后会变化），用于定位与断言
const ORIGINAL_NAME = 'E2E测试角色';
const RENAMED_NAME = 'E2E改名后角色';

// 读取角色头部 h2 文本（绕过部分 WebView getText 兼容性问题）
async function currentRoleHeaderName(): Promise<string> {
  return browser.execute(() => {
    const header = document.querySelector('header h2');
    return header?.textContent ?? '';
  });
}

// 进入目标角色视图：依次点击侧边栏角色图标，直到头部 h2 命中目标名
async function enterRoleByName(name: string): Promise<boolean> {
  const icons = await $$('[data-role-icon] button');
  const count = await icons.length;
  for (let i = 0; i < count; i++) {
    await icons[i].click();
    await browser.pause(500);
    if ((await currentRoleHeaderName()).includes(name)) return true;
  }
  return false;
}

describe('角色 CRUD 旅程', () => {
  before(async () => {
    await waitForAppReady();
    await seedCompleteOnboarding();
    // 预置一个保底角色：删除测试角色时需满足后端「至少保留一个角色」约束
    await seedRole('保底角色');
    await browser.refresh();
    await waitForAppReady();
  });

  it('AC2.3-创建：填写表单并创建角色', async () => {
    await openAddRoleModal();
    const modalHeader = await $('h2=添加新角色');
    await modalHeader.waitForDisplayed({ timeout: 5000 });
    const nameInput = await $('#add-role-name');
    await nameInput.setValue(ORIGINAL_NAME);
    const createButton = await $('button=创建角色');
    await createButton.click();
    await browser.pause(2000);
    // 应至少有保底角色 + 新建角色两个图标
    const roleIcons = await $$('[data-role-icon]');
    expect(await roleIcons.length).toBeGreaterThanOrEqual(2);
  });

  it('AC2.3-编辑名称：在设置 Tab 修改名称并保存，头部随之更新', async () => {
    const entered = await enterRoleByName(ORIGINAL_NAME);
    expect(entered).toBe(true);

    const settingsTab = await $('button=设置');
    await settingsTab.click();
    await browser.pause(500);

    const nameInput = await $('#role-name');
    await nameInput.waitForDisplayed({ timeout: 5000 });
    await nameInput.setValue(RENAMED_NAME);
    const saveButton = await $('button=保存更改');
    await saveButton.click();
    await browser.pause(2000);

    expect(await currentRoleHeaderName()).toBe(RENAMED_NAME);
  });

  it('AC2.3-归档：右键菜单归档后该角色从侧边栏消失', async () => {
    const before = await (await $$('[data-role-icon]')).length;
    const entered = await enterRoleByName(RENAMED_NAME);
    expect(entered).toBe(true);

    // 右键当前激活角色对应的图标
    const icons = await $$('[data-role-icon] button');
    const count = await icons.length;
    for (let i = 0; i < count; i++) {
      await icons[i].click();
      await browser.pause(300);
      if ((await currentRoleHeaderName()).includes(RENAMED_NAME)) {
        await icons[i].click({ button: 'right' });
        break;
      }
    }
    await browser.pause(500);
    const archiveItem = await $('button*=归档');
    await archiveItem.click();
    await browser.pause(500);
    const confirmArchive = await $('button=确认归档');
    await confirmArchive.click();
    await browser.pause(2000);

    const after = await (await $$('[data-role-icon]')).length;
    expect(after).toBe(before - 1);
  });

  it('AC2.3-恢复：在设置「已归档角色」中重新启用，角色重新出现', async () => {
    const before = await (await $$('[data-role-icon]')).length;
    await openSettings();
    await browser.pause(800);

    const restoreButton = await $('button=重新启用');
    await restoreButton.waitForDisplayed({ timeout: 5000 });
    await restoreButton.click();
    await browser.pause(2000);

    // 关闭设置回到主界面（再次点击侧边栏管家）
    const butlerButton = await $('button[title="管家"]');
    await butlerButton.click();
    await browser.pause(800);

    const after = await (await $$('[data-role-icon]')).length;
    expect(after).toBe(before + 1);
  });

  it('AC2.3-删除：右键删除并输入角色名确认后从侧边栏移除', async () => {
    const before = await (await $$('[data-role-icon]')).length;
    const entered = await enterRoleByName(RENAMED_NAME);
    expect(entered).toBe(true);

    const icons = await $$('[data-role-icon] button');
    const count = await icons.length;
    for (let i = 0; i < count; i++) {
      await icons[i].click();
      await browser.pause(300);
      if ((await currentRoleHeaderName()).includes(RENAMED_NAME)) {
        await icons[i].click({ button: 'right' });
        break;
      }
    }
    await browser.pause(500);
    const deleteItem = await $('button*=删除');
    await deleteItem.click();
    await browser.pause(500);

    // 删除确认需输入角色名（input placeholder 即角色名）
    const confirmInput = await $(`input[placeholder="${RENAMED_NAME}"]`);
    await confirmInput.waitForDisplayed({ timeout: 5000 });
    await confirmInput.setValue(RENAMED_NAME);
    const confirmDelete = await $('button=确认删除');
    await confirmDelete.click();
    await browser.pause(2000);

    const after = await (await $$('[data-role-icon]')).length;
    expect(after).toBe(before - 1);
  });
});
