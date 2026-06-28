import { $, $$, browser } from '@wdio/globals';

export async function waitForAppReady(): Promise<void> {
  await browser.waitUntil(
    async () => await $('body').isExisting(),
    { timeout: 30000, timeoutMsg: 'App window did not load within 30s' },
  );
  await browser.pause(1000);
}

export async function navigateToButler(): Promise<void> {
  const butlerButton = await $('button[title="管家"]');
  await butlerButton.click();
  await browser.pause(500);
}

export async function navigateToRole(roleId: string): Promise<void> {
  const roleIcons = await $$('[data-role-icon]');
  for (const icon of roleIcons) {
    const attr = await icon.getAttribute('data-role-icon');
    if (attr === roleId) {
      await icon.click();
      await browser.pause(500);
      return;
    }
  }
  throw new Error(`Role ${roleId} not found in sidebar`);
}

export async function openAddRoleModal(): Promise<void> {
  const addRoleButton = await $('button[title="添加角色"]');
  await addRoleButton.click();
  await browser.pause(500);
}

export async function openSettings(): Promise<void> {
  const settingsButton = await $('button[title="设置"]');
  await settingsButton.click();
  await browser.pause(500);
}

export async function clickTab(tabLabel: string): Promise<void> {
  const tabButton = await $(`button=${tabLabel}`);
  await tabButton.click();
  await browser.pause(500);
}

export async function fillInput(selector: string, value: string): Promise<void> {
  const input = await $(selector);
  await input.setValue(value);
}

export async function clickButton(text: string): Promise<void> {
  const button = await $(`button=${text}`);
  await button.click();
  await browser.pause(500);
}

// ── IPC-based seeding via browser.execute() ──

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const result = await browser.executeAsync(async (cmd: string, argObj: Record<string, unknown> | undefined, done: (val: unknown) => void) => {
    try {
      const tauriInvoke = (window as any).__TAURI_INTERNALS__?.invoke;
      if (!tauriInvoke) {
        done({ __error: '__TAURI_INTERNALS__.invoke not found on window' });
        return;
      }
      const result = argObj ? await tauriInvoke(cmd, argObj) : await tauriInvoke(cmd);
      done(result);
    } catch (e) {
      done({ __error: String(e) });
    }
  }, command, args) as T;
  // 检查错误
  if (result && typeof result === 'object' && '__error' in result) {
    throw new Error(`IPC ${command} failed: ${(result as any).__error}`);
  }
  return result;
}

export async function seedCompleteOnboarding(): Promise<void> {
  await invoke('app_complete_onboarding');
  await browser.pause(500);
  // 刷新页面让前端重新读取 DB 状态，从 onboarding 视图切换到管家视图
  await browser.refresh();
  await waitForAppReady();
}

export async function seedRole(name: string, icon: string = '🎯', color: string = '#6366F1'): Promise<string> {
  const role = await invoke<{ id: string }>('role_create', {
    input: { name, icon, color },
  });
  return role.id;
}

export async function seedTask(
  roleId: string,
  title: string,
  quadrant: string = 'Q2',
  isBigRock: boolean = false,
  deadline: string | null = null,
): Promise<string> {
  const task = await invoke<{ id: string }>('task_create', {
    input: {
      ownerType: 'role',
      roleId,
      title,
      quadrant,
      isBigRock,
      deadline,
    },
  });
  return task.id;
}
