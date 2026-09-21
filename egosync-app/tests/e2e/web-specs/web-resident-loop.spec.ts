// Story 17.2：常驻工作循环硬化（FR-47）端到端验证。
//
// 三条链路：
// ① 常驻生成：登录 → in-spec 起 LLM SSE stub（openai 兼容、127.0.0.1 内网
//   直连绕代理）→ llm_config_create/set_default → briefing_time 与
//   moderate/proactive 触发时间键播种为「下一分钟」→ 建带 goal 角色 →
//   about:blank 断连（零客户端连接，服务端自行运转）→ 目标分钟完整过去
//   （60s tick 必命中）→ 回连断言：当日简报已生成落库且聊天流可见
//   （briefing_get_latest + 页面文本）；建议 status='pending'（API +
//   ActionCard 确认/拒绝按钮在场——FR-11 确认纪律不因常驻豁免）。
// ② 同分钟重启不重复触发（执行级验收）：独立角色 + 独立建议标题（stub
//   可变标题），目标分钟内轮询到建议落地后立即 kill+重启服务端——新进程
//   首 tick 立即执行且仍处于同一分钟：持久化去重必须挡住重触发（基线
//   内存实现重启即失忆会二连发）→ 分钟完整过去后断言仍恰好 1 条。
// ③ 清理：先回应用页（失败路径可能停在 about:blank，相对 URL fetch 不可
//   解析），快照优先复位调度设置（scheduler_get_times/settings_get_schedule
//   读原值，不硬编码引擎默认值），删配置/删角色（唯一活跃角色守卫失败
//   降级归档）。
//
// 已知窗口（诚实登记）：
// - 若本轮服务器首 tick 恰落当地 08:00 分（默认简报时刻），当日 cycle 已
//   被默认时刻消费——测试 ① 开头检测到当日简报已存在即跳过（环境窗口非
//   回归）；测试 ② 不依赖简报 cycle，不受影响。
// - 测试 ② 的重启若因 tick 落点过晚（分钟末）未能在同分钟内完成，断言
//   退化为恒真（下一分钟本就不匹配时间键）——落点均匀分布下约 9 成运行
//   走真路径，届时以 console 输出登记。

import { describe, it, before, after } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import http from 'node:http';
import { openWebAppAndLogin, webInvoke, restartWebServer } from '../helpers/web-helper.js';
import { webServerUrl } from '../wdio.web.conf';

const STUB_PORT = 18081;
const STUB_BASE_URL = `http://127.0.0.1:${STUB_PORT}/v1`;
const RUN = Date.now();
const BRIEFING_MARKER = `E2E常驻简报验证-${RUN}`;
const SUGGESTION_TITLE = `E2E常驻建议验证-${RUN}`;
const RESTART_TITLE = `E2E重启去重验证-${RUN}`;

/** stub 建议响应的当前标题（可变——重启腿用独立标题区分首轮建议）。 */
let currentSuggestionTitle = SUGGESTION_TITLE;

let stubServer: http.Server | null = null;
let configId = '';
let residentRoleId = '';
let restartRoleId = '';
let butlerConvId = '';
/** 清理复位快照（读当前值，避免硬编码引擎默认值漂移）。 */
let savedTimes: { moderate: string[]; proactive: string[] } | null = null;
let savedBriefingTime = '';

/** openai 兼容 SSE stub：按 system prompt 判别简报/建议两种调用形态。 */
function startLlmStub(): Promise<void> {
  return new Promise((resolve, reject) => {
    let listening = false;
    stubServer = http.createServer((req, res) => {
      // 评审修复：客户端中途断开/流错误若无监听即未处理异常，崩掉 wdio worker
      req.on('error', () => {});
      res.on('error', () => {});
      const chunks: Buffer[] = [];
      req.on('data', (c: Buffer) => chunks.push(c));
      req.on('end', () => {
        const body = Buffer.concat(chunks).toString('utf8');
        let isSuggestion = false;
        try {
          const messages = (JSON.parse(body) as { messages?: Array<{ content?: string }> }).messages;
          isSuggestion = (messages?.[0]?.content ?? '').includes('主动建议生成器');
        } catch {
          // 判别失败按简报路径兜底（两条路径均为固定文本响应）
        }
        const content = isSuggestion
          ? JSON.stringify({
              suggestions: [
                {
                  title: currentSuggestionTitle,
                  content: `无人值守期间由调度循环生成的建议（${RUN}），等待用户确认`,
                  priority: 'medium',
                },
              ],
            })
          : `${BRIEFING_MARKER}：管家晨间简报按计划时刻自动生成，无任何客户端连接。`;
        res.writeHead(200, { 'Content-Type': 'text/event-stream' });
        // 分两片 delta 模拟流式，[DONE] 收尾（openai.rs 解析协议）
        const half = Math.ceil(content.length / 2);
        for (const piece of [content.slice(0, half), content.slice(half)]) {
          res.write(`data: ${JSON.stringify({ choices: [{ index: 0, delta: { content: piece } }] })}\n\n`);
        }
        res.write(`data: ${JSON.stringify({ choices: [{ index: 0, delta: {}, finish_reason: 'stop' }] })}\n\n`);
        res.write('data: [DONE]\n\n');
        res.end();
      });
    });
    // 评审修复：端口被占等 listen 错误若无监听即未处理异常
    stubServer.on('error', (err) => {
      if (!listening) reject(new Error(`LLM stub 监听 127.0.0.1:${STUB_PORT} 失败: ${err.message}`));
    });
    stubServer.listen(STUB_PORT, '127.0.0.1', () => {
      listening = true;
      resolve();
    });
  });
}

const pad = (n: number) => String(n).padStart(2, '0');
const fmtHHMM = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;
const fmtDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/**
 * 计划时刻 = 下一个分钟边界（≥20s 播种守卫——评审修复：原 10s 守卫在慢
 * 环境下播种可能溢入目标分钟）。返回分钟起点 epoch 供溢出复核与断连计时。
 */
function nextTargetMinute(): { hhmm: string; date: Date; minuteStartEpoch: number; msToStart: number } {
  let msToBoundary = 60_000 - (Date.now() % 60_000);
  if (msToBoundary < 20_000) msToBoundary += 60_000;
  const minuteStartEpoch = Date.now() + msToBoundary;
  const at = new Date(minuteStartEpoch + 5_000); // 分钟内采样时刻（HH:MM 即分钟值）
  return { hhmm: fmtHHMM(at), date: at, minuteStartEpoch, msToStart: msToBoundary };
}

/** 回连：导航回应用 + 连接徽标 online（Cookie 持久免重登）。 */
async function reconnectAndWaitOnline(): Promise<void> {
  await browser.url(webServerUrl());
  const badge = await $('[data-testid="connection-status"]');
  await badge.waitForDisplayed({ timeout: 30_000 });
  await browser.waitUntil(async () => (await badge.getAttribute('data-state')) === 'online', {
    timeout: 30_000,
    timeoutMsg: '回连后连接徽标未回到 online',
  });
}

describe('Web 模式常驻工作循环（Story 17.2 / FR-47）', () => {
  before(async () => {
    await startLlmStub();
    await openWebAppAndLogin();
  });

  it('计划时刻无任何客户端连接，简报按时生成落库并留存', async function () {
    // 超时按实际等待派生（评审修复：原固定 150s 低于内部等待最坏和）
    this.timeout(300_000);

    // 08:00 守卫（评审修复）：服务器首 tick 若已落当地 08:00 分（默认时刻），
    // 当日简报 cycle 已被消费，标记简报无法再生成——环境窗口非回归，跳过
    const preexisting = await webInvoke<{ date: string } | null>('briefing_get_latest');
    if (preexisting !== null && preexisting.date === fmtDate(new Date())) {
      console.warn(`[resident-loop] 服务器启动期默认 08:00 简报已消费当日 cycle（${preexisting.date}），跳过测试 ①`);
      return this.skip();
    }

    // 快照调度设置（清理复位用，不硬编码引擎默认值）
    savedTimes = await webInvoke<{ moderate: string[]; proactive: string[] }>('scheduler_get_times');
    const savedSchedule = await webInvoke<{ briefingTime: string }>('settings_get_schedule');
    savedBriefingTime = savedSchedule.briefingTime;

    // ① 播种（与时间无关的先做——压缩设置写入到断连之间的竞态窗口）：
    //    LLM stub 设为默认配置（internal ⇒ 内网直连绕代理；密钥经
    //    ServerSecretStore 落 secrets.json——命令写入路径与真实用户一致）
    const created = await webInvoke<{ id: string }>('llm_config_create', {
      input: {
        name: `e2e常驻stub-${RUN}`,
        provider: 'openai_compatible',
        baseUrl: STUB_BASE_URL,
        model: 'stub-model',
        apiKey: 'e2e-stub-key',
        networkLocation: 'internal',
      },
    });
    configId = created.id;
    await webInvoke('llm_config_set_default', { id: configId });

    const conv = await webInvoke<{ id: string }>('chat_get_butler_conversation');
    butlerConvId = conv.id;

    // 角色必须带核心目标：建议生成对「无目标、无任务、无记忆」的角色有既有
    // 短路（不凭空捏造建议——suggestion_generator 设计语义），带 goal 才走 LLM
    const role = await webInvoke<{ id: string }>('role_create', {
      input: {
        name: `常驻验证角色-${RUN}`,
        icon: 'briefcase',
        color: '#4F46E5',
        goal: '每周完成一次深度整理并输出周报，保护深度工作时段',
      },
    });
    residentRoleId = role.id;

    // ② 时间相关播种（仅 2 次调用）+ 溢出守卫（评审修复：慢环境复核）
    let target = nextTargetMinute();
    const seedTimes = async () => {
      await webInvoke('settings_update_schedule', { briefingTime: target.hhmm });
      await webInvoke('scheduler_set_times', { moderate: [target.hhmm], proactive: [target.hhmm] });
    };
    await seedTimes();
    if (Date.now() >= target.minuteStartEpoch) {
      // 播种溢入目标分钟 → 顺延一分钟重设（目标分钟的 tick 可能已按旧键错过）
      target = nextTargetMinute();
      await seedTimes();
    }

    // ③ 断连：页面离开应用 ⇒ 零客户端连接（SSE 断开，服务端自行运转）
    await browser.url('about:blank');

    // ④ 目标分钟完整过去（60s tick 必命中；+5s 收生成余量）
    await browser.pause(target.msToStart + 65_000);

    // ⑤ 回连（Cookie 持久免重登；徽标回 online）
    await reconnectAndWaitOnline();

    // ⑥ 断言：当日简报已生成（触发侧 + 生成侧全链路）
    //（todayStr 取目标分钟日期而非回连时刻——评审修复：跨午夜运行时两者可能差一天）
    const todayStr = fmtDate(target.date);
    await browser.waitUntil(
      async () => {
        const b = await webInvoke<{ date: string; content: string } | null>('briefing_get_latest');
        return b !== null && b.date === todayStr && b.content.includes(BRIEFING_MARKER);
      },
      {
        timeout: 30_000,
        timeoutMsg: `计划分钟已过、零客户端期间简报未生成（briefing_get_latest 未命中 ${BRIEFING_MARKER}）`,
      },
    );

    // ⑦ 用户「次日打开 WEB 端可见」：简报以管家消息留存于聊天流
    //（等待式：徽标 online ≠ ChatStream 历史加载完成，innerText 须等渲染落地）
    await browser.waitUntil(
      async () => {
        const visible = await browser.execute(
          (marker: string) => document.body.innerText.includes(marker),
          BRIEFING_MARKER,
        );
        return visible;
      },
      {
        timeout: 20_000,
        timeoutMsg: `简报消息未在管家聊天流渲染（${BRIEFING_MARKER}）`,
      },
    );
  });

  it('常驻生成的建议处于待确认态，不自动执行（FR-11）', async () => {
    if (!residentRoleId) return; // 测试 ① 环境跳过时本轮无建议可断言

    // API 层：pending 列表命中本条建议
    let pending: Array<{ id: string; title: string; status: string }> = [];
    await browser.waitUntil(
      async () => {
        pending = await webInvoke('suggestion_list_pending', { conversationId: butlerConvId });
        return Array.isArray(pending) && pending.some((s) => s.title === SUGGESTION_TITLE);
      },
      {
        timeout: 30_000,
        timeoutMsg: `常驻生成的建议未出现或未入 pending 列表（${SUGGESTION_TITLE}）`,
      },
    );
    const hit = pending.find((s) => s.title === SUGGESTION_TITLE);
    expect(hit).toBeTruthy();
    expect(hit!.status).toBe('pending');

    // UI 层：管家视图 ActionCard 待确认态（确认/拒绝按钮在场 = 等用户裁决）
    //（等待式 75s：并行 worker（如 web-streaming 的消息用例）在同一管家会话
    // 触发流式时，ChatStream 的 showActions=!isStreaming 会暂隐建议块——
    // 流式结束即恢复渲染，非本用例回归；单跑时数秒内即出现）
    const card = await $(`article[aria-label^="${SUGGESTION_TITLE}"]`);
    await browser.waitUntil(async () => card.isDisplayed(), {
      timeout: 75_000,
      timeoutMsg: `建议卡片未渲染为待确认态（${SUGGESTION_TITLE}）`,
    });
    const buttons = await card.$$('button');
    const texts: string[] = [];
    for (const btn of buttons) texts.push(await btn.getText());
    expect(texts).toContain('确认');
    expect(texts).toContain('拒绝');
  });

  it('同分钟重启服务端不重复触发工作循环（FR-47 执行级验收）', async function () {
    this.timeout(240_000);

    // 独立角色 + 独立建议标题（stub 可变标题，与测试 ① 的建议区分）
    currentSuggestionTitle = RESTART_TITLE;
    const role = await webInvoke<{ id: string }>('role_create', {
      input: {
        name: `重启去重角色-${RUN}`,
        icon: 'target',
        color: '#0EA5E9',
        goal: '每日复盘一次执行偏差并记录改进项',
      },
    });
    restartRoleId = role.id;

    // 时间键播种（仅 moderate/proactive——简报日 cycle 已消费，无需也不应再设）
    let target = nextTargetMinute();
    const seedTimes = async () => {
      await webInvoke('scheduler_set_times', { moderate: [target.hhmm], proactive: [target.hhmm] });
    };
    await seedTimes();
    if (Date.now() >= target.minuteStartEpoch) {
      target = nextTargetMinute();
      await seedTimes();
    }

    // 断连至分钟初，回连轮询触发（tick 均匀落点，分钟内必有一次）
    await browser.url('about:blank');
    await browser.pause(target.msToStart + 2_000);
    await reconnectAndWaitOnline();

    const minuteStartEpoch = target.minuteStartEpoch;
    await browser.waitUntil(
      async () => {
        const list = await webInvoke<Array<{ roleId: string; title: string }>>('suggestion_list_pending', {
          conversationId: butlerConvId,
        });
        return list.some((s) => s.roleId === restartRoleId && s.title === RESTART_TITLE);
      },
      {
        timeout: 70_000,
        timeoutMsg: `目标分钟内建议未落地（role=${restartRoleId}，title=${RESTART_TITLE}）`,
      },
    );
    // 此刻仍在目标分钟内（轮询起点为分钟初，tick 落点 + 轮询间隔 ≪ 60s）

    // 同分钟重启：kill + 同 env 重启（新进程首 tick 立即执行——tokio interval
    // 首次消耗后循环体即刻跑，仍处于同一分钟：持久化去重必须挡住重触发；
    // 基线内存实现重启即失忆，会向 stub 二次请求并二连发建议）
    await restartWebServer();
    const restartedInMinute = Date.now() < minuteStartEpoch + 60_000;
    if (!restartedInMinute) {
      console.warn('[resident-loop] 重启未能在目标分钟内完成（tick 落点过晚），本轮断言退化为恒真');
    }

    // 分钟完整过去（新进程若仍在分钟内，其首 tick 已做过去重判定）
    const remaining = minuteStartEpoch + 60_000 - Date.now() + 5_000;
    if (remaining > 0) await browser.pause(remaining);

    // 断言：仍恰好 1 条（重启不重触发；基线为 2 条）
    await reconnectAndWaitOnline();
    const list = await webInvoke<Array<{ roleId: string; title: string }>>('suggestion_list_pending', {
      conversationId: butlerConvId,
    });
    const hits = list.filter((s) => s.roleId === restartRoleId && s.title === RESTART_TITLE);
    expect(hits.length).toBe(1);
  });

  after(async () => {
    // 清理（尽力而为）：先回应用页（评审修复：失败路径停在 about:blank 时
    // 相对 URL fetch 不可解析，清理会静默全灭并污染同服务器后续 spec）
    try {
      await browser.url(webServerUrl());
    } catch {
      /* 忽略——服务端可能已被测试 ③ 重启，页面加载失败不阻断清理 */
    }
    // 复位调度设置（快照优先；无快照时说明测试 ① 未跑到快照点，本就未改动）
    try {
      if (savedTimes) {
        await webInvoke('scheduler_set_times', { moderate: savedTimes.moderate, proactive: savedTimes.proactive });
      }
    } catch {
      /* 忽略 */
    }
    try {
      if (savedBriefingTime) {
        await webInvoke('settings_update_schedule', { briefingTime: savedBriefingTime });
      }
    } catch {
      /* 忽略 */
    }
    if (configId) {
      try {
        await webInvoke('llm_config_delete', { id: configId });
      } catch {
        /* 忽略 */
      }
    }
    // 删角色（建议随 role_id ON DELETE CASCADE 一并清除；唯一活跃角色守卫
    // 失败则降级归档）
    for (const id of [residentRoleId, restartRoleId]) {
      if (!id) continue;
      try {
        await webInvoke('role_delete', { id });
      } catch {
        try {
          await webInvoke('role_archive', { id });
        } catch {
          /* 忽略 */
        }
      }
    }
    await new Promise<void>((resolve) => {
      if (!stubServer) return resolve();
      stubServer.close(() => resolve());
    });
  });
});
