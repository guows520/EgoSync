// Story 16.2：Web 模式流式旅程 + 刷新恢复。
//
// CI 无 LLM：chat_send_message 先插 user 行（complete）+ assistant 占位行
// （is_complete=false，token 仅内存、终止才落库）——错误路径下占位行
// 永远保持未完成。这正是「流式中途刷新」的确定性锚点：
// 刷新 → 历史重拉 → 未完成回复行呈「生成中」占位（非空气泡、非伪内容）。
// 与桌面 llm-streaming.spec 同语义（其限制说明同款来源）。

import { describe, it, before, after } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import {
  openWebAppAndLogin,
  waitForConnectionState,
  saveWebSmokeScreenshot,
  webInvoke,
} from '../helpers/web-helper.js';

describe('Web 模式流式 + 刷新恢复（Story 16.2）', () => {
  before(async () => {
    await openWebAppAndLogin();
  });

  it('登录后连接状态徽标呈 online（HttpTransport 三态机落位）', async () => {
    await waitForConnectionState('online');
    // 冒烟场景①留档：在线态（管家视图 + connection-status 徽标）
    await saveWebSmokeScreenshot('web-01-online');
  });

  it('登录后直达管家视图（onboarding 预置生效）且显示聊天输入框', async () => {
    const chatInput = await $('textarea, input[type="text"]');
    await chatInput.waitForDisplayed({ timeout: 15000 });
    expect(await chatInput.isDisplayed()).toBe(true);
  });

  it('发送消息后刷新页面：未完成回复呈「生成中」占位（刷新恢复）', async () => {
    // 本轮唯一消息内容（含时间戳）：DB 跨轮累计且每轮发同文案时，
    // 「找未完成助手行」的轮询会命中旧行而立即通过——刷新可能赶在
    // 本轮新行落库可见之前执行（第 3 轮实测复现）。唯一内容使
    // 轮询/断言锚点严格对准本轮行。
    const messageText = `帮我看一下今天的任务安排（${new Date().toISOString()}）`;
    const messagePrefix = '帮我看一下今天的任务安排';

    // 1. 经 UI 发送一条管家消息（chat_send_message：user 行 + assistant 占位行）
    const chatInput = await $('textarea, input[type="text"]');
    await chatInput.setValue(messageText);
    await browser.keys('Enter');

    // 2. 服务端事实源锚点：轮询 chat_get_history 直至本轮 user 行落库、
    //    且其后存在未完成助手行，再执行刷新。此前用「输入框恢复可用」
    //    作锚点是弱锚点——React 禁用输入与 done 帧复位之间的窗口可能被
    //    整段错过（首轮 poll 即通过），刷新会赶在落库可见前重拉历史
    //    → 占位行缺席 → 用例抖动。
    await browser.waitUntil(async () => {
      const conversations = await webInvoke<{ id: string }[]>('chat_list_conversations');
      if (!Array.isArray(conversations) || conversations.length === 0) return false;
      for (const conversation of conversations) {
        const history = await webInvoke<{ role: string; content: string; isComplete: boolean }[]>(
          'chat_get_history',
          { conversationId: conversation.id },
        );
        if (!Array.isArray(history)) continue;
        const userIndex = history.findIndex(m => m.role === 'user' && m.content === messageText);
        if (userIndex === -1) continue;
        if (history.slice(userIndex + 1).some(m => m.role === 'assistant' && !m.isComplete)) return true;
      }
      return false;
    }, {
      timeout: 30000,
      timeoutMsg: '服务端 30 秒内未见本轮未完成助手行（chat_send_message 落库异常）',
    });

    // 3. 模拟用户 F5（流式中途刷新）：历史重拉后占位行呈现
    await browser.refresh();
    await waitForConnectionState('online');

    // 4. 刷新恢复断言：未完成回复呈「生成中」占位（pending-generation-placeholder）。
    //    每轮 poll 重新 $() 查询——刷新后应用仍在启动/React 重渲染会替换 DOM 节点，
    //    持有单一元素引用会解析到已替换的旧节点（isDisplayed 恒 false 的间歇性抖动，
    //    三连跑实测复现两次；重查询模式免疫节点替换）。
    let lastDiagnostic = '';
    let pollCount = 0;
    await browser.waitUntil(async () => {
      const placeholder = await $('[data-testid="pending-generation-placeholder"]');
      const exists = await placeholder.isExisting();
      if (exists && (await placeholder.isDisplayed())) return true;
      // 诊断转储（每 10 轮探一次）：区分「应用半启动」（徽标/输入框缺席
      // ——会话解析失败）与「元素在场但不可见」两类根因；REST 直查区分
      // 「服务端有数据而前端没拿到」vs「服务端本身空」。
      pollCount += 1;
      if (pollCount % 10 !== 1) return false;
      lastDiagnostic = await browser.execute(async () => {
        const badge = document.querySelector('[data-testid="connection-status"]');
        const input = document.querySelector('textarea, input[type="text"]') as HTMLInputElement | null;
        const bodyLen = document.body.innerText?.length ?? 0;
        const chatMsgCount = document.querySelectorAll('[data-testid^="chat-message-"]').length;
        const scrollChildren = document.querySelector('[data-testid="chat-scroll-container"]')?.children.length ?? -1;
        let restProbe = 'n/a';
        try {
          const res = await fetch('/api/cmd/chat_get_butler_conversation', {
            method: 'POST',
            credentials: 'include',
            headers: { 'Content-Type': 'application/json' },
            body: '{}',
          });
          restProbe = `conv:${res.status}`;
          if (res.ok) {
            const data = await res.json();
            restProbe += `:${JSON.stringify(data).slice(0, 80)}`;
          }
        } catch (e) {
          restProbe = `error:${(e as Error).message}`;
        }
        return `badge=${badge?.getAttribute('data-state') ?? 'absent'} input=${input ? `present${input.disabled ? ':disabled' : ':enabled'}` : 'absent'} bodyTextLen=${bodyLen} chatMsg=${chatMsgCount} scrollChildren=${scrollChildren} rest=${restProbe}`;
      });
      return false;
    }, {
      // 45s 预算：本 VM（3.6GB + swap）跑三连套件时偶发渲染线程停摆 15~30s
      // （元素最终必然呈现——历史落库由前置 REST 轮询锚定；停摆是环境
      // 负载问题，非产品缺陷），放宽预算避免环境性抖动误报。
      timeout: 45000,
      timeoutMsg: '刷新恢复后「生成中」占位 45 秒内未呈现',
    }).catch(async err => {
      throw new Error(`${(err as Error).message}｜诊断: ${lastDiagnostic}`);
    });
    const placeholder = await $('[data-testid="pending-generation-placeholder"]');
    const text = await placeholder.getText();
    expect(text).toContain('生成中');

    // 5. 用户消息如实呈现（服务端是事实源；user 气泡内容为 <span> 纯文本；
    //    *= 部分匹配以兼容时间戳后缀）
    const userBubble = await $(`span*=${messagePrefix}`);
    expect(await userBubble.isExisting()).toBe(true);

    // 冒烟场景③留档：流式中途 F5——「生成中」占位 + 用户消息
    await saveWebSmokeScreenshot('web-03-refresh-resume');
  });

  it('375px 视口无横向溢出（响应式基线走查：管家视图 + 通知面板 + 模态）', async () => {
    // DevTools 模拟 iPhone 逻辑分辨率（CSS 像素 375 宽）
    // setWindowSize 在 VM 高负载下可能延迟生效（16.4 全量套件实测：500ms
    // 竞态；D2 收口后两次全量复跑均栽在未落定——通知面板仍按桌面宽
    // left=800~1180 定位被记入溢出名单）——按区间轮询 innerWidth 落定
    // 再做几何断言（范式照搬 web-mobile 的 waitForViewport；仅加固等待，
    // 断言口径不动）
    await browser.setWindowSize(375, 812);
    await browser.waitUntil(
      async () => {
        const w = (await browser.execute(() => window.innerWidth)) as number;
        return w >= 320 && w <= 430;
      },
      { timeout: 60000, timeoutMsg: '视口 60 秒内未落到 320～430px 移动区间' },
    );
    await browser.pause(300);

    // 评审修复：scrollWidth 断言在双层 overflow-hidden 裁剪下结构性恒真
    // （真实溢出被裁掉、fixed 元素不计入文档滚动溢出）——改为
    // getBoundingClientRect 逐容器断言视口内，并扩展覆盖面（通知面板 +
    // 共享 Modal；此前只测管家聊天视图，面板/模态收缩回归零守护）。
    const assertNoHorizontalOverflow = async () => {
      const result = await browser.execute(() => {
        const vw = window.innerWidth;
        const offenders: string[] = [];
        // 全文档可见元素的矩形级检查（无视裁剪——溢出即报）
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

    // ① 管家聊天视图基线（保留原 scrollWidth 断言 + 矩形级强化）
    const butler = await assertNoHorizontalOverflow();
    expect(butler.scrollWidth).toBeLessThanOrEqual(butler.clientWidth);
    expect(butler.offenders).toEqual([]);
    // 响应式留档：375px 管家视图
    await saveWebSmokeScreenshot('web-04-375px-butler');

    // ② 通知面板（w-full max-w-[380px]——响应式收缩路径）。
    //    title 属性恒为「通知」——aria-label 随未读数动态变化
    //    （events spec 留下未读后为「有新通知」，套件序下实测踩坑）。
    //    UI 导航点击走 JS click（绕开命中测试）：套件序下应用在 SSE/
    //    数据加载窗口的布局微移会让 WebDriver 命中测试瞬态拦截
    //    （实测 5 秒 3 连拦截），断言严格性不受影响。
    const clickByJs = async (selector: string) => {
      const ok = await browser.execute(sel => {
        const el = document.querySelector(sel);
        if (!(el instanceof HTMLElement)) return false;
        el.click();
        return true;
      }, selector);
      if (!ok) throw new Error(`JS click 目标不存在: ${selector}`);
    };

    await clickByJs('button[title="通知"]');
    await browser.pause(400);
    const panel = await $('[aria-label="通知中心"]');
    expect(await panel.isDisplayed()).toBe(true);
    const withPanel = await assertNoHorizontalOverflow();
    expect(withPanel.offenders).toEqual([]);
    // 面板关闭——移动端面板 w-full 全屏覆盖，侧栏铃铛被盖住，
    // 须经面板自己的关闭按钮（aria-label）关闭
    await clickByJs('button[aria-label="关闭通知中心"]');
    await browser.pause(300);

    // ③ 共享 Modal（添加角色 w-480px + MODAL_MOBILE_WIDTH 移动收缩）
    await clickByJs('button[aria-label="添加角色"]');
    const roleModal = await $('[role="dialog"][aria-label="添加新角色"]');
    await roleModal.waitForDisplayed({ timeout: 10000 });
    const withModal = await assertNoHorizontalOverflow();
    expect(withModal.offenders).toEqual([]);
    // 关闭模态（Escape）
    await browser.keys('Escape');
    await browser.pause(300);
  });

  after(async () => {
    // 还原桌面宽度（≥768 ⇒ 侧栏可见）——后续 spec 的桌面链路依赖；
    // 同样等视口落定再收尾（视口纪律对齐 web-mobile after 钩子：
    // 未落定 = 下一个 spec 的结构性失败源）
    await browser.setWindowSize(1280, 800);
    await browser.waitUntil(
      async () => (await browser.execute(() => window.innerWidth)) >= 768,
      { timeout: 30000, timeoutMsg: '视口 30 秒内未还原到桌面宽度' },
    );
  });
});
