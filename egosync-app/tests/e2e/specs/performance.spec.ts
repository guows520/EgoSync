import { describe, it, before } from 'mocha';
import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady, seedCompleteOnboarding, seedRole, invoke } from '../helpers/app-helper.js';
import {
  measureColdStart,
  measureProcessMemory,
  measureOnboardingInteractive,
  measureStreamRenderLatency,
  writePerfReport,
} from '../helpers/perf-helper.js';

describe('性能基准测量', () => {
  before(async () => {
    await waitForAppReady();
  });

  it('AC4 — 冷启动到可交互时间应在阈值内（CI ≤ 10s，警告不阻断）', async () => {
    const result = await measureColdStart();
    writePerfReport('cold-start', result);
    // 验证测量有效（coldStartMs > 0），不因超阈值失败（AC #6 警告不阻断）
    expect(result.coldStartMs).toBeGreaterThan(0);
    console.log(`[perf] cold start: ${result.coldStartMs}ms (threshold ${result.threshold}ms, exceeded=${result.exceededThreshold})`);
  });

  it('AC2 — Onboarding 可交互时间应在阈值内（CI ≤ 15s，警告不阻断）', async () => {
    // 冷启动后 onboarding 视图应已显示，测量 input 可见时间
    const result = await measureOnboardingInteractive();
    writePerfReport('onboarding-interactive', result);
    expect(result.interactiveMs).toBeGreaterThan(0);
    console.log(`[perf] onboarding interactive: ${result.interactiveMs}ms (threshold ${result.threshold}ms, exceeded=${result.exceededThreshold})`);
  });

  it('AC5 — 稳态内存（3 角色 + 100 记忆）应在阈值内（CI ≤ 300MB，警告不阻断）', async () => {
    await seedCompleteOnboarding();
    // 通过 perf-test feature gate 的 seed command 注入 3 角色 + 100 记忆
    await invoke('app_seed_perf_data', { roleCount: 3, memoryCount: 100 });
    await browser.refresh();
    await waitForAppReady();

    const result = await measureProcessMemory();
    writePerfReport('steady-state-memory', result);
    expect(result.rssMb).toBeGreaterThan(0);
    console.log(`[perf] steady-state RSS: ${result.rssMb}MB (threshold ${result.threshold}MB, exceeded=${result.exceededThreshold})`);
  });

  it('AC3 — 流式 token 渲染延迟应在阈值内（三平台差异 ≤ 50ms，警告不阻断）', async () => {
    // 需要进入管家对话视图以激活 llm:stream 监听
    await seedCompleteOnboarding();
    // 导航到管家视图
    const butlerButton = await $('button[title="管家"]');
    if (await butlerButton.isExisting()) {
      await butlerButton.click();
      await browser.pause(500);
    }

    const result = await measureStreamRenderLatency(['你', '好', '，', '测', '试']);
    writePerfReport('stream-render-latency', result);
    expect(result.emitToRenderMs).toBeGreaterThan(0);
    console.log(`[perf] stream render: ${result.emitToRenderMs}ms (threshold ${result.threshold}ms, exceeded=${result.exceededThreshold})`);
  });
});
