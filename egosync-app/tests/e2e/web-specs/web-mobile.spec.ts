// Story 16.4：Web 移动端形态 + PWA 端到端旅程（375×812 = iPhone 逻辑分辨率）。
//
// 覆盖（spec I/O 矩阵移动侧 + AC1/AC2/AC3/AC4/AC5/AC6/AC8/AC9）：
// 1. 登录后 375px：侧栏退场（max-md:hidden）、底部 tab（管家/角色/设置）
//    呈现、管家主区全屏、管家视图头部通知铃铛/连接状态（侧栏控件搬迁）；
// 2. 管家对话：输入框 + 发送按钮可达且 ≥44×44px（虚拟键盘场景的触控红线）；
// 3. 角色 tab 两级：列表根（管家固定首行 + 角色行 + 「＋」）→ 点角色进
//    详情 → 「⋯」切换角色回根；
// 4. 任务面：象限分组头「排序」开关 → ↑/↓ 调序（服务端事实源断言新序）；
// 5. 仪表盘/通知面 + 375px 全程无横向溢出（矩形级，无视裁剪）；
// 6. 设置 tab：7 项入口 + 打开桌面同款 GlobalSettingsModal（移动全屏）；
// 7. 断点互斥：768px 侧栏回归、底栏退场（BREAKPOINT_EDGE）；
// 8. PWA：manifest 字段（name/start_url/display/theme_color/icons maskable）
//    + Service Worker 注册 + sw.js 伺服 200。
//
// 视口纪律：登录态在桌面宽度下锚定（openWebAppAndLogin 的就绪探测依赖
// 可见徽标），再切 375（CSS 媒体查询即时重排）；after 还原桌面宽度——
// 本 spec 在 glob 字母序中先于 web-reconnect/resident-loop/streaming 执行，
// 375 泄漏会让后续 spec 的侧栏可见性断言结构性失败。

import { describe, it, before, after } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import {
  openWebAppAndLogin,
  waitForConnectionState,
  saveWebSmokeScreenshot,
  webInvoke,
  seedRoleViaCommand,
  killWebServer,
  restartWebServer,
} from '../helpers/web-helper.js';
import { webServerUrl } from '../wdio.web.conf.js';

/** 375×812：iPhone 逻辑分辨率（DevTools 模拟 CSS 像素）。 */
const MOBILE = { width: 375, height: 812 };

describe('Web 移动端形态与 PWA（Story 16.4，375×812）', () => {
  /** 本 spec 造的角色/任务（跨用例共享——两级角色 tab 与任务面串联）。 */
  let seededRoleId = '';
  let titleA = '';
  let titleB = '';

  before(async () => {
    // 桌面宽度下完成登录锚定，再切移动视口（见文件头「视口纪律」）
    await openWebAppAndLogin();
    await browser.setWindowSize(MOBILE.width, MOBILE.height);
    // 轮询 innerWidth 落定（VM 高负载下 setWindowSize 有延迟；媒体查询的重排
    // 以浏览器报告的视口为准）。验收口径按 FR-45 的 375～430 CSS px 区间
    // （全量套件实测：紧邻 cargo 套件的高负载窗口曾把精确 375 断言拖过 15s
    // 超时——区间口径既守住验收又不把环境抖动当失败）。
    await browser.waitUntil(
      async () => {
        const w = (await browser.execute(() => window.innerWidth)) as number;
        return w >= 320 && w <= 430;
      },
      { timeout: 30000, timeoutMsg: '视口 30 秒内未落到 320～430px 移动区间' },
    );
    await browser.pause(300);
  });

  after(async () => {
    // 还原桌面宽度（≥768 ⇒ 侧栏可见）——后续 spec 的桌面链路依赖；
    // 同样等视口落定再收尾（未落定 = 下一个 spec 的结构性失败源）
    await browser.setWindowSize(1280, 800);
    await browser.waitUntil(
      async () => (await browser.execute(() => window.innerWidth)) >= 768,
      { timeout: 15000, timeoutMsg: '视口 15 秒内未还原到桌面宽度' },
    );
  });

  it('375px：侧栏退场、底部 tab 呈现、管家主区全屏（含铃铛/连接状态搬迁）', async () => {
    // 侧栏 max-md:hidden ⇒ 不可见（DOM 仍在——互斥是 CSS 类对，见单测）
    const sidebar = await $('aside[aria-label="角色导航"]');
    expect(await sidebar.isExisting()).toBe(true);
    expect(await sidebar.isDisplayed()).toBe(false);

    // 底部 tab 三件套在场
    const tabBar = await $('[data-testid="bottom-tab-bar"]');
    expect(await tabBar.isDisplayed()).toBe(true);
    for (const key of ['butler', 'roles', 'settings'] as const) {
      expect(await $(`[data-testid="bottom-tab-${key}"]`).isDisplayed()).toBe(true);
    }
    // 激活态：管家 tab aria-current
    expect(await $('[data-testid="bottom-tab-butler"]').getAttribute('aria-current')).toBe('page');

    // 侧栏专属控件搬迁（人工裁决 2026-09-25）：通知铃铛 + 连接状态 → 管家视图头部
    expect(await $('[data-testid="butler-notif-bell"]').isDisplayed()).toBe(true);
    expect(await $('header [data-testid="connection-status"]').isDisplayed()).toBe(true);

    // 管家主区全屏：聊天输入框在场且贴底可达
    const chatInput = await $('textarea, input[type="text"]');
    expect(await chatInput.isDisplayed()).toBe(true);

    await saveWebSmokeScreenshot('web-mobile-01-375px-butler');
  });

  it('管家对话：发送按钮 ≥44×44px 且在视口内（触控目标红线 + 键盘可达）', async () => {
    const sendButton = await $('button[aria-label="发送"]');
    await sendButton.waitForDisplayed({ timeout: 15000 });

    // 触控目标 ≥44×44px（AC3/C2）
    const size = await browser.execute((el: HTMLElement) => {
      const rect = el.getBoundingClientRect();
      return { width: rect.width, height: rect.height, bottom: rect.bottom, right: rect.right };
    }, sendButton);
    expect(size.width).toBeGreaterThanOrEqual(44);
    expect(size.height).toBeGreaterThanOrEqual(44);
    // 输入区贴底 + 虚拟键盘场景可达性：发送按钮在视口内（高度以浏览器
    // 报告的 innerHeight 为准——setWindowSize 高度可差 1px）
    const viewportHeight = (await browser.execute(() => window.innerHeight)) as number;
    expect(size.bottom).toBeLessThanOrEqual(viewportHeight + 0.5);
    expect(size.right).toBeLessThanOrEqual(MOBILE.width + 0.5);

    // 经 UI 发送一条消息（发送链路在移动视口可用）
    const messageText = `移动端走查消息 ${new Date().toISOString()}`;
    const chatInput = await $('textarea, input[type="text"]');
    await chatInput.setValue(messageText);
    await browser.keys('Enter');
    await browser.waitUntil(
      async () => (await $(`span*=${messageText}`)).isExisting(),
      { timeout: 30000, timeoutMsg: '移动视口发送后用户气泡未呈现' },
    );
  });

  it('角色 tab 两级：列表根（管家首行+角色行+＋）→ 点选进详情 → 「⋯」回根', async () => {
    // 造数据：真实角色 + 两个 Q1 任务（任务面消费）
    seededRoleId = await seedRoleViaCommand(`移动走查角色-${Date.now().toString().slice(-6)}`);
    const stamp = Date.now();
    titleA = `e2e-mobile-sort-a-${stamp}`;
    titleB = `e2e-mobile-sort-b-${stamp}`;
    await webInvoke('task_create', {
      input: { ownerType: 'role', roleId: seededRoleId, title: titleA, quadrant: 'Q1' },
    });
    await webInvoke('task_create', {
      input: { ownerType: 'role', roleId: seededRoleId, title: titleB, quadrant: 'Q1' },
    });

    // 刷新让应用拾取新角色（roles 挂载时加载）——Cookie 持久免重登
    await browser.refresh();
    await waitForConnectionState('online', 60000);

    // 第一级：底部「角色」tab → 列表根
    await $('[data-testid="bottom-tab-roles"]').click();
    await $('[data-testid="role-list-butler"]').waitForDisplayed({ timeout: 15000 });
    expect(await $(`[data-testid="role-list-role-${seededRoleId}"]`).isDisplayed()).toBe(true);
    expect(await $('[data-testid="role-list-add"]').isDisplayed()).toBe(true);
    await saveWebSmokeScreenshot('web-mobile-02-375px-role-list');

    // 第二级：点角色行 → 进详情（RoleHeader 头部 + 「⋯」菜单）
    await $(`[data-testid="role-list-role-${seededRoleId}"]`).click();
    await $('[data-testid="role-more-menu"]').waitForDisplayed({ timeout: 15000 });
    await saveWebSmokeScreenshot('web-mobile-03-375px-role-detail');

    // 「⋯」→ 切换角色 ⇒ 回列表根（线框屏 2b）
    await $('[data-testid="role-more-menu"]').click();
    await $('[data-testid="role-menu-switch"]').waitForDisplayed({ timeout: 10000 });
    await $('[data-testid="role-menu-switch"]').click();
    await browser.waitUntil(
      async () => (await $('[data-testid="role-list-butler"]')).isDisplayed(),
      { timeout: 10000, timeoutMsg: '「⋯」切换角色未回列表根' },
    );
  });

  it('任务面：象限分组头「排序」开关 → ↑/↓ 调序（DOM 序翻转断言）', async () => {
    // 重新进入角色详情 → 打开「任务」tab（RoleHeader 顶部 tab，小屏可横滚）
    await $(`[data-testid="role-list-role-${seededRoleId}"]`).click();
    await $('[data-testid="role-more-menu"]').waitForDisplayed({ timeout: 15000 });
    await $('button*=任务').click();
    await browser.waitUntil(
      async () => (await $('[data-testid^="sort-mode-"]')).isExisting(),
      { timeout: 15000, timeoutMsg: '任务面「排序」开关未呈现（四象限分组的排序入口缺席）' },
    );

    // 排序模式开：拖拽把手退场、↑/↓ 登场
    await $('[data-testid^="sort-mode-"]').click();
    const moveDown = await $(`button[aria-label="下移 ${titleA}"]`);
    await moveDown.waitForDisplayed({ timeout: 10000 });
    // 拖拽把手应已隐藏（useSortable disabled ⇒ 不渲染）
    expect(await $('button[aria-label^="拖动排序 "]').isExisting()).toBe(false);

    // DOM 序锚点：任务标题 p 元素（唯一前缀过滤，不依赖卡片内部结构）
    const titlesInDom = () => browser.execute(() => {
      const prefix = 'e2e-mobile-sort-';
      return Array.from(document.querySelectorAll('p'))
        .map(p => p.textContent ?? '')
        .filter(t => t.startsWith(prefix));
    });
    expect(await titlesInDom()).toEqual([titleA, titleB]);

    // ↑/↓ 调序：A 下移一位 ⇒ 乐观更新 + 服务端持久化后 DOM 序翻转
    await moveDown.click();
    await browser.waitUntil(
      async () => (await titlesInDom()).join(',') === [titleB, titleA].join(','),
      { timeout: 20000, timeoutMsg: '下移后任务 DOM 序未翻转（重排链路断）' },
    );

    // 375px 矩形级无横向溢出（排序模式开启态：↑/↓ + 编辑/删除 + 44px
    // 触控目标同屏，卡片集群不得撑出视口——评审修复补齐）
    expect(await collectHorizontalOverflow()).toEqual([]);

    await saveWebSmokeScreenshot('web-mobile-04-375px-task-sort');
  });

  it('仪表盘 + 通知面走查：375px 全程无横向溢出（矩形级）', async () => {
    // 回管家视图 → 仪表盘 tab（小屏工作区 h-[42%] 底栏面板）
    await $('[data-testid="bottom-tab-butler"]').click();
    await browser.waitUntil(
      async () => (await $('textarea, input[type="text"]')).isDisplayed(),
      { timeout: 15000 },
    );
    await $('button*=仪表盘').click();
    await browser.pause(800);

    // 矩形级无溢出（无视 overflow 裁剪——真实溢出即报）
    const assertNoHorizontalOverflow = async () => {
      const result = await browser.execute(() => {
        const vw = window.innerWidth;
        const offenders: string[] = [];
        for (const el of document.body.querySelectorAll<HTMLElement>('*')) {
          const rect = el.getBoundingClientRect();
          if (rect.width === 0 || rect.height === 0) continue;
          if (rect.left < -0.5 || rect.right > vw + 0.5) {
            const cls = (el.getAttribute('class') ?? '').slice(0, 60);
            offenders.push(`${el.tagName.toLowerCase()}[${cls}] left=${rect.left.toFixed(1)} right=${rect.right.toFixed(1)}`);
            if (offenders.length >= 5) break;
          }
        }
        return { offenders, scrollWidth: document.documentElement.scrollWidth, clientWidth: vw };
      });
      return result;
    };

    const withDashboard = await assertNoHorizontalOverflow();
    expect(withDashboard.offenders).toEqual([]);

    // 通知面板（铃铛搬迁后移动端也能开）
    await $('[data-testid="butler-notif-bell"]').click();
    await browser.pause(400);
    const panel = await $('[aria-label="通知中心"]');
    expect(await panel.isDisplayed()).toBe(true);
    const withPanel = await assertNoHorizontalOverflow();
    expect(withPanel.offenders).toEqual([]);
    await clickByJs('button[aria-label="关闭通知中心"]');
    await browser.pause(300);
  });

  it('设置 tab：7 项入口 + 打开桌面同款 GlobalSettingsModal（移动全屏）', async () => {
    await $('[data-testid="bottom-tab-settings"]').click();
    await $('[data-testid="settings-row-llm"]').waitForDisplayed({ timeout: 15000 });

    // 7 项：模型服务 / MCP / 调度 / 数据 / 通知 / 主题 / 登出
    for (const key of ['llm', 'mcp', 'scheduler', 'data', 'notification']) {
      expect(await $(`[data-testid="settings-row-${key}"]`).isDisplayed()).toBe(true);
    }
    expect(await $('[data-testid="settings-theme-light"]').isDisplayed()).toBe(true);
    expect(await $('[data-testid="settings-logout"]').isDisplayed()).toBe(true);

    // 打开桌面同款 GlobalSettingsModal（initialTab=llm 落点）
    await $('[data-testid="settings-row-llm"]').click();
    // slide-in-from-right 300ms 动画期内的 transform 会让 left 非终值——
    // 等动画落定再测几何
    await browser.pause(700);
    await browser.waitUntil(
      async () => (await $('h3*=LLM Provider')).isExisting(),
      { timeout: 15000, timeoutMsg: 'GlobalSettingsModal 未在移动端全屏打开' },
    );

    // 移动全屏：模态左缘贴 0（desktop left-16 的 max-md:left-0 覆盖）
    const modalLeft = await browser.execute(() => {
      const heading = Array.from(document.querySelectorAll('h3')).find(h => h.textContent?.includes('LLM Provider'));
      const shell = heading?.closest('.fixed');
      return shell ? shell.getBoundingClientRect().left : null;
    });
    expect(modalLeft).toBe(0);

    // 无横向溢出（矩形级——带 class 诊断，修复过程可定位）
    expect(await collectHorizontalOverflow()).toEqual([]);

    await clickByJs('button[aria-label="关闭全局设置"]');
    await browser.pause(300);

    // 「通知」入口落点 = 桌面调度时间 tab（敲门通知声音设置所在）
    await $('[data-testid="settings-row-notification"]').click();
    await browser.pause(700);
    await browser.waitUntil(
      async () => (await $('h3*=调度时间配置')).isExisting(),
      { timeout: 15000, timeoutMsg: '「通知」入口未落到调度时间 tab' },
    );
    await clickByJs('button[aria-label="关闭全局设置"]');
    await browser.pause(300);

    await saveWebSmokeScreenshot('web-mobile-05-375px-settings');
  });

  it('断点互斥：768px 侧栏回归、底栏退场（BREAKPOINT_EDGE）', async () => {
    // setWindowSize 在 VM 高负载下可能延迟生效（全量套件实测：500ms
    // 竞态）——按区间轮询 innerWidth 落定再做可见性断言（几何状态由
    // 浏览器自身报告；区间而非精确值——headless 视口与请求尺寸可差 1px）
    const waitForViewport = async (width: number) => {
      await browser.setWindowSize(width, 812);
      const inRange = (w: number) => (width >= 768 ? w >= 768 : w >= 320 && w <= 430);
      await browser.waitUntil(
        async () => {
          const w = (await browser.execute(() => window.innerWidth)) as number;
          return inRange(w);
        },
        { timeout: 15000, timeoutMsg: `视口 15 秒内未落到目标区间（请求 ${width}px）` },
      );
      await browser.pause(200);
    };

    await waitForViewport(1280);
    const sidebar = await $('aside[aria-label="角色导航"]');
    expect(await sidebar.isDisplayed()).toBe(true);
    expect(await $('[data-testid="bottom-tab-bar"]').isDisplayed()).toBe(false);

    // 回 375：互斥反转，无残留（元素重查——布局重建后句柄可能失效）
    await waitForViewport(MOBILE.width);
    const sidebarBack = await $('aside[aria-label="角色导航"]');
    expect(await sidebarBack.isDisplayed()).toBe(false);
    expect(await $('[data-testid="bottom-tab-bar"]').isDisplayed()).toBe(true);
  });

  it('PWA：manifest 字段齐备（name/start_url/display/theme_color/icons maskable）', async () => {
    const manifest = await browser.execute(async () => {
      const res = await fetch('/manifest.webmanifest');
      const json = await res.json();
      return { status: res.status, json };
    });

    expect(manifest.status).toBe(200);
    expect(manifest.json.name).toBe('EgoSync 数字分身');
    expect(manifest.json.short_name).toBe('EgoSync');
    expect(manifest.json.start_url).toBe('/');
    expect(manifest.json.display).toBe('standalone');
    expect(manifest.json.theme_color).toBe('#4F46E5');
    const icons = manifest.json.icons as Array<{ src: string; sizes: string; purpose?: string }>;
    expect(icons.length).toBeGreaterThanOrEqual(2);
    // purpose "any maskable"：常规 + maskable 双声明（评审修复——原 'maskable'
    // 单声明会让部分启动器拒绝非安全区用途）
    expect(icons.some(i => i.sizes === '192x192' && i.purpose === 'any maskable')).toBe(true);
    expect(icons.some(i => i.sizes === '512x512' && i.purpose === 'any maskable')).toBe(true);
    // id 与 start_url 一致（安装身份锚点）；不锁 orientation
    expect(manifest.json.id).toBe('/');
    expect(manifest.json.orientation).toBeUndefined();

    // CSP 增量运行时实证：响应头含 manifest-src/worker-src 'self'（漏配 ⇒
    // 浏览器拒载 manifest / SW 注册失败——契约测试源码侧 + 此处伺服侧双守）
    const csp = await browser.execute(async () => {
      const res = await fetch('/');
      return res.headers.get('content-security-policy') ?? '';
    });
    expect(csp).toContain("manifest-src 'self'");
    expect(csp).toContain("worker-src 'self'");
  });

  it('PWA：Service Worker 注册且 sw.js 可伺服（离线外壳前提）', async () => {
    const swStatus = await browser.execute(async () => {
      const res = await fetch('/sw.js');
      return res.status;
    });
    expect(swStatus).toBe(200);

    await browser.waitUntil(async () => {
      const reg = await browser.execute(async () => {
        const registration = await navigator.serviceWorker.getRegistration();
        return registration ? { scope: registration.scope, hasActive: Boolean(registration.active) } : null;
      });
      return reg !== null;
    }, {
      timeout: 30000,
      timeoutMsg: 'Service Worker 30 秒内未注册（生产构建 + 浏览器宿主才注册）',
    });
  });

  it('断网刷新：SW 外壳离线可开 + 离线屏诚实明示（OFFLINE 矩阵行）', async () => {
    // 杀真服务端（SIGKILL——与 web-reconnect 同机制；CDP Network 断链在
    // WDIO v9 不可用）：SW navigation 走 stale-while-revalidate ⇒ index.html
    // 从缓存供出，外壳仍可打开；/api/* 全断 ⇒ AuthGate 呈离线屏（NFR-C7
    // 红线：不为离线假装有数据——刷新=服务端重取语义不变）。
    // 保底纪律：断言任何一步失败，finally 都补重启服务端——否则后续 spec
    // 集体挂在启动（16.4 实测教训：首版断言张冠李戴致 restart 未执行）。
    await killWebServer();
    try {
      await browser.refresh();

      // ① 外壳离线可用：应用自身 UI 呈现（离线屏文案=诚实断线态，而非
      //    浏览器错误页/白屏——证明 SW 缓存的外壳把应用拉起来了）
      await browser.waitUntil(
        async () => (await $('//*[contains(text(), "无法连接服务器")]')).isExisting(),
        { timeout: 30000, timeoutMsg: '断网刷新后应用外壳未呈现（SW 外壳缓存未生效或离线屏缺席）' },
      );
      await saveWebSmokeScreenshot('web-mobile-06-375px-offline-shell');

      // ② 服务端回归：离线屏「重试」⇒ 回已认证外壳 + 在线徽标
      await restartWebServer();
      await (await $('button*=重试')).click();
      await browser.waitUntil(
        async () => (await $('[data-testid="bottom-tab-bar"]')).isDisplayed(),
        { timeout: 60000, timeoutMsg: '服务端恢复后应用未回到已认证外壳' },
      );
      await waitForConnectionState('online', 60000);
    } finally {
      // 保底：异常路径下服务端可能仍是死的——探活并按需补重启（正常路径
      // ② 已重启，此检查命中即跳过，不会二次重启）
      let alive = false;
      try {
        const res = await fetch(`${webServerUrl()}/healthz`, { signal: AbortSignal.timeout(2000) });
        alive = res.ok;
      } catch {
        alive = false;
      }
      if (!alive) {
        await restartWebServer();
      }
    }
  });
  // Story 16.4 D2 收口（2026-09-25 人类指令）：移动端归档对等入口——桌面右键
  // 菜单的归档/删除随侧栏退场消失，由角色详情「⋯」菜单补齐。自持一次性角色，
  // 不动其他用例的 seededRoleId；删除路径的输入名校验由 vitest 行为钉死
  // （RoleHeader.archiveDelete.test.tsx），e2e 只走归档真实落库链路。
  it('D2 收口：移动端「⋯」→ 归档角色（确认弹窗）→ 角色从列表消失且回管家', async () => {
    const throwawayId = await seedRoleViaCommand(`移动归档角色-${Date.now().toString().slice(-6)}`);
    await browser.refresh();
    await waitForConnectionState('online', 60000);

    // 角色 tab → 列表根 → 点进该角色详情
    await $('[data-testid="bottom-tab-roles"]').click();
    await $('[data-testid="role-list-butler"]').waitForDisplayed({ timeout: 15000 });
    await $(`[data-testid="role-list-role-${throwawayId}"]`).click();
    await $('[data-testid="role-more-menu"]').waitForDisplayed({ timeout: 15000 });

    // 「⋯」→ 归档 → 确认弹窗 → 确认归档
    await $('[data-testid="role-more-menu"]').click();
    await $('[data-testid="role-menu-archive"]').waitForDisplayed({ timeout: 10000 });
    await $('[data-testid="role-menu-archive"]').click();
    await $('[data-testid="role-confirm-archive"]').waitForDisplayed({ timeout: 10000 });
    await saveWebSmokeScreenshot('web-mobile-07-375px-role-archive');
    await $('[data-testid="role-confirm-archive"]').click();

    // 归档当前角色后 App 处理器切回管家视图。注意选择器：connection-status
    // 在侧栏（max-md:hidden）与管家头部各有一份，$() 取首个匹配即移动端不可见
    // 的侧栏副本——故以管家头部独有的通知铃铛为视图判据（16.4 搬迁证据）。
    await browser.waitUntil(
      async () => (await $('[data-testid="butler-notif-bell"]')).isDisplayed(),
      { timeout: 15000, timeoutMsg: '归档后未回到管家视图' },
    );

    // 角色列表不再含该角色（已归档角色只出现在管家→设置的归档分区）
    await $('[data-testid="bottom-tab-roles"]').click();
    await $('[data-testid="role-list-butler"]').waitForDisplayed({ timeout: 15000 });
    expect(await $(`[data-testid="role-list-role-${throwawayId}"]`).isExisting()).toBe(false);
  });
});

/** JS click（绕开命中测试——面板/顶栏元素被覆盖时同款手段，web-streaming 先例）。 */
async function clickByJs(selector: string): Promise<void> {
  const ok = await browser.execute(sel => {
    const el = document.querySelector(sel);
    if (!(el instanceof HTMLElement)) return false;
    el.click();
    return true;
  }, selector);
  if (!ok) throw new Error(`JS click 目标不存在: ${selector}`);
}

/** 矩形级横向溢出名单（无视 overflow 裁剪——真实溢出即报；带 class 诊断）。 */
async function collectHorizontalOverflow(): Promise<string[]> {
  return browser.execute(() => {
    const vw = window.innerWidth;
    const offenders: string[] = [];
    for (const el of document.body.querySelectorAll<HTMLElement>('*')) {
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      if (rect.left < -0.5 || rect.right > vw + 0.5) {
        const cls = (el.getAttribute('class') ?? '').slice(0, 60);
        offenders.push(`${el.tagName.toLowerCase()}[${cls}] left=${rect.left.toFixed(1)} right=${rect.right.toFixed(1)}`);
        if (offenders.length >= 5) break;
      }
    }
    return offenders;
  });
}
