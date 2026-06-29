import { $, browser } from '@wdio/globals';
import { existsSync, mkdirSync, writeFileSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const reportsDir = resolve(__dirname, '..', 'reports', 'performance');

// 性能阈值基线（CI runner 宽松，本地 SSD 严格）
// 超阈值时警告不阻断（AC #6），仅在报告中标记 exceededThreshold。
const THRESHOLDS = {
  coldStartMs: 10_000, // CI: ≤ 10s
  onboardingInteractiveMs: 15_000, // CI: ≤ 15s
  rssMb: 300, // CI: ≤ 300MB
  streamRenderMs: 50, // 三平台差异 ≤ 50ms（记录用，不阻断）
} as const;

interface ColdStartResult {
  coldStartMs: number;
  runnerOs: string;
  timestamp: string;
  exceededThreshold: boolean;
  threshold: number;
}

interface MemorySnapshot {
  rssMb: number;
  jsHeapUsedMb: number | null;
  processUptimeSecs: number;
  sidecarRssMb: number | null;
  timestamp: string;
  exceededThreshold: boolean;
  threshold: number;
}

interface OnboardingInteractiveResult {
  interactiveMs: number;
  runnerOs: string;
  timestamp: string;
  exceededThreshold: boolean;
  threshold: number;
}

interface StreamRenderResult {
  emitToRenderMs: number;
  tokenCount: number;
  runnerOs: string;
  timestamp: string;
  exceededThreshold: boolean;
  threshold: number;
}

function ensureReportsDir(): void {
  if (!existsSync(reportsDir)) {
    mkdirSync(reportsDir, { recursive: true });
  }
}

function runnerOs(): string {
  return (browser.capabilities as any)?.platformName
    || (browser.capabilities as any)?.['platform']
    || process.platform
    || 'unknown';
}

function checkThreshold(value: number, threshold: number): boolean {
  return value > threshold;
}

/**
 * 测量冷启动时间：从 session 创建到 body 可交互的时间差。
 * 复用 waitForAppReady 的等待逻辑，但在等待前后记录时间戳。
 */
export async function measureColdStart(): Promise<ColdStartResult> {
  ensureReportsDir();
  const start = Date.now();
  await browser.waitUntil(
    async () => await $('body').isExisting(),
    { timeout: 30000, timeoutMsg: 'App window did not load within 30s' },
  );
  await browser.pause(500);
  const coldStartMs = Date.now() - start;
  const exceeded = checkThreshold(coldStartMs, THRESHOLDS.coldStartMs);
  if (exceeded) {
    console.warn(`[perf] cold start ${coldStartMs}ms exceeds threshold ${THRESHOLDS.coldStartMs}ms (warning only, not blocking)`);
  }
  return {
    coldStartMs,
    runnerOs: runnerOs(),
    timestamp: new Date().toISOString(),
    exceededThreshold: exceeded,
    threshold: THRESHOLDS.coldStartMs,
  };
}

/**
 * 通过 Tauri IPC 读取进程内存快照（Rust 端 app_performance_snapshot command）。
 * WebView 无 process.memoryUsage()，必须通过 Rust 端 sysinfo crate 读取 RSS。
 */
export async function measureProcessMemory(): Promise<MemorySnapshot> {
  ensureReportsDir();
  const snapshot = await browser.executeAsync(async (done: (val: unknown) => void) => {
    try {
      const tauriInvoke = (window as any).__TAURI_INTERNALS__?.invoke;
      if (!tauriInvoke) {
        done({ __error: '__TAURI_INTERNALS__.invoke not found on window' });
        return;
      }
      const result = await tauriInvoke('app_performance_snapshot');
      done(result);
    } catch (e) {
      done({ __error: String(e) });
    }
  }) as {
    rssMb: number;
    jsHeapUsedMb: number | null;
    processUptimeSecs: number;
    sidecarRssMb: number | null;
  } & { __error?: string };

  if (snapshot && snapshot.__error) {
    throw new Error(`app_performance_snapshot failed: ${snapshot.__error}`);
  }
  if (!snapshot || snapshot.rssMb == null || !Number.isFinite(snapshot.rssMb)) {
    throw new Error('app_performance_snapshot returned invalid snapshot');
  }

  const exceeded = checkThreshold(snapshot.rssMb, THRESHOLDS.rssMb);
  if (exceeded) {
    console.warn(`[perf] RSS ${snapshot.rssMb}MB exceeds threshold ${THRESHOLDS.rssMb}MB (warning only, not blocking)`);
  }

  return {
    rssMb: snapshot.rssMb,
    jsHeapUsedMb: snapshot.jsHeapUsedMb,
    processUptimeSecs: snapshot.processUptimeSecs,
    sidecarRssMb: snapshot.sidecarRssMb,
    timestamp: new Date().toISOString(),
    exceededThreshold: exceeded,
    threshold: THRESHOLDS.rssMb,
  };
}

/**
 * 测量 Onboarding 可交互时间：从当前时刻到 onboarding 视图可交互元素可见的时间差。
 * CI 无 LLM 配置时显示"配置 AI 模型"按钮界面（无 input），因此等待 input 或按钮任一可见。
 */
export async function measureOnboardingInteractive(): Promise<OnboardingInteractiveResult> {
  ensureReportsDir();
  const start = Date.now();
  await browser.waitUntil(
    async () => {
      // CI 无 LLM 配置时显示"配置 AI 模型"按钮；配置完成后显示 input
      const input = await $('input[type="text"]');
      if (await input.isDisplayed()) return true;
      const configButton = await $('button=配置 AI 模型');
      return await configButton.isDisplayed();
    },
    { timeout: 30000, timeoutMsg: 'Onboarding interactive element not visible within 30s' },
  );
  const interactiveMs = Date.now() - start;
  const exceeded = checkThreshold(interactiveMs, THRESHOLDS.onboardingInteractiveMs);
  if (exceeded) {
    console.warn(`[perf] onboarding interactive ${interactiveMs}ms exceeds threshold ${THRESHOLDS.onboardingInteractiveMs}ms (warning only, not blocking)`);
  }
  return {
    interactiveMs,
    runnerOs: runnerOs(),
    timestamp: new Date().toISOString(),
    exceededThreshold: exceeded,
    threshold: THRESHOLDS.onboardingInteractiveMs,
  };
}

/**
 * 测量流式渲染延迟：通过 app_emit_test_stream command（perf-test feature gate）
 * 注入 mock llm:stream 事件，测量从 emit 到 DOM 中出现 token 文本的时间差。
 *
 * 此方案测量「Tauri Event → 前端监听 → React 状态更新 → DOM 渲染」端到端延迟，
 * 不测量 LLM 网络延迟（Provider 责任，非应用可控）。
 */
export async function measureStreamRenderLatency(
  tokens: string[] = ['你', '好', '，', '测', '试'],
): Promise<StreamRenderResult> {
  if (tokens.length === 0) {
    throw new Error('tokens cannot be empty');
  }
  ensureReportsDir();

  // 追加一个独特标记 token 作为检测 sentinel，避免 DOM 中已存在相同字符导致误匹配
  const sentinel = `perf-token-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  const tokensWithSentinel = [...tokens, sentinel];
  const tokenCount = tokens.length;

  // 通过 IPC 调用 app_emit_test_stream，在 Rust 端 emit llm:stream 事件
  // 测量从 emit 调用到 DOM 中出现最后一个 token 的时间差
  // 需要传入当前 conversation id，否则 ChatStream 的 handleStreamEvent 会过滤掉不匹配的事件
  const result = await browser.executeAsync(async (
    tok: string[],
    done: (val: unknown) => void,
  ) => {
    try {
      const tauriInvoke = (window as any).__TAURI_INTERNALS__?.invoke;
      if (!tauriInvoke) {
        done({ __error: '__TAURI_INTERNALS__.invoke not found on window' });
        return;
      }
      // 获取当前管家 conversation id，使流式事件能被 ChatStream 接收
      let convId: string | undefined;
      try {
        const conv = await tauriInvoke('chat_get_butler_conversation');
        convId = conv?.id;
      } catch {
        // 获取失败时回退到默认 "perf-test"
      }
      const start = Date.now();
      await tauriInvoke('app_emit_test_stream', {
        tokens: tok,
        conversationId: convId,
      });
      // 轮询 DOM 直到出现最后一个 sentinel token 文本（最多 5s）
      const lastToken = tok[tok.length - 1];
      const deadline = Date.now() + 5000;
      while (Date.now() < deadline) {
        const bodyText = document.body.innerText || '';
        if (bodyText.includes(lastToken)) {
          done({ emitToRenderMs: Date.now() - start });
          return;
        }
        await new Promise(r => setTimeout(r, 10));
      }
      done({ __error: 'token did not appear in DOM within 5s' });
    } catch (e) {
      done({ __error: String(e) });
    }
  }, tokens) as { emitToRenderMs: number } & { __error?: string };

  if (result && result.__error) {
    throw new Error(`measureStreamRenderLatency failed: ${result.__error}`);
  }

  const exceeded = checkThreshold(result.emitToRenderMs, THRESHOLDS.streamRenderMs);
  if (exceeded) {
    console.warn(`[perf] stream render ${result.emitToRenderMs}ms exceeds threshold ${THRESHOLDS.streamRenderMs}ms (warning only, not blocking)`);
  }

  return {
    emitToRenderMs: result.emitToRenderMs,
    tokenCount,
    runnerOs: runnerOs(),
    timestamp: new Date().toISOString(),
    exceededThreshold: exceeded,
    threshold: THRESHOLDS.streamRenderMs,
  };
}

/**
 * 将性能数据 JSON 落盘到 reports/performance/ 目录。
 */
export function writePerfReport(name: string, data: unknown): void {
  ensureReportsDir();
  const reportPath = join(reportsDir, `${name}.json`);
  writeFileSync(reportPath, JSON.stringify(data, null, 2));
  console.log(`[perf] report saved: ${reportPath}`);
}
