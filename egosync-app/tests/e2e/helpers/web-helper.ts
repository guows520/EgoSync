// Story 16.2：Web 模式 e2e 助手——浏览器宿主（HttpTransport）专用。
//
// 与桌面链路 app-helper 的关键差异：
// - 探测靠 DOM 标记而非 __TAURI_INTERNALS__（web 宿主无 Tauri 桥）：
//   ready = connection-status 徽标 data-state="online"；
// - IPC 走 REST：POST /api/cmd/{command}（camelCase 参数，200 + 判别头
//   X-Egosync-App-Error 表示业务错误）——与桌面 invoke 错误通道同构；
// - 服务端进程控制：PID 文件（onPrepare 在 launcher 进程起的 detached
//   实例，worker 进程读 PID kill；重启用同 env 重放）。

import { spawn, ChildProcess } from 'child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync, openSync } from 'fs';
import { resolve, dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { browser } from '@wdio/globals';
import { webServerEnv, webServerUrl, waitForPort } from '../wdio.web.conf';

const __dirname = dirname(fileURLToPath(import.meta.url));
// __dirname = egosync-app/tests/e2e/helpers —— e2e 根目录上一级；
// 仓库根 = egosync-app/tests/e2e 再上三级（server/ 与 _bmad-output/ 在仓库根）
const e2eRoot = resolve(__dirname, '..');
const repoRoot = resolve(e2eRoot, '..', '..', '..');
const serverBinary = resolve(repoRoot, 'server', 'target', 'debug', 'egosync-server');
const pidFile = resolve(e2eRoot, 'logs', 'web', 'web-server.pid');
/** 首个页面加载标记——后续 spec 加载前静默等窗口老化（限流预算隔离）。 */
const authWindowMarker = resolve(e2eRoot, 'logs', 'web', 'auth-window-opened');

export const E2E_WEB_TOKEN = 'e2e-web-fixed-token';

/**
 * 冒烟截图输出（运行时产物）。评审修复：直写 _bmad-output 受控目录会令
 * 每轮 test:web 都弄脏工作树（覆盖已提交的验证证据）；运行时截图落
 * logs/web/screenshots/，_bmad-output 内的四张留档冻结为本故事验证证据。
 */
const screenshotsDir = resolve(e2eRoot, 'logs', 'web', 'screenshots');

export async function saveWebSmokeScreenshot(name: string): Promise<void> {
  mkdirSync(screenshotsDir, { recursive: true });
  const path = join(screenshotsDir, `${name}.png`);
  try {
    await browser.saveScreenshot(path);
    console.log(`[web-smoke] screenshot saved: ${path}`);
  } catch (e) {
    console.log(`[web-smoke] screenshot failed (${name}): ${e}`);
  }
}

/**
 * 读当前服务端 PID（onPrepare 起服时写入；重启后由本助手覆写）。
 */
function readServerPid(): number | null {
  if (!existsSync(pidFile)) return null;
  const pid = Number(readFileSync(pidFile, 'utf-8').trim());
  return Number.isFinite(pid) ? pid : null;
}

/**
 * 终止当前服务端实例（SIGKILL——直接模拟断线，绕过优雅退出）。
 */
export async function killWebServer(): Promise<void> {
  const pid = readServerPid();
  if (pid == null) throw new Error('server pid file not found——onPrepare 未起服？');
  process.kill(pid, 'SIGKILL');
  // 等待端口真正关闭（进程死 ≠ socket 释放）
  const url = webServerUrl();
  await browser.waitUntil(
    async () => {
      try {
        await fetch(`${url}/healthz`, { signal: AbortSignal.timeout(500) });
        return false;
      } catch {
        return true;
      }
    },
    { timeout: 10000, timeoutMsg: 'server port still open after SIGKILL' },
  );
}

/**
 * 以同 env 重启服务端（reconnect 场景：服务端回来了）。
 * 返回子进程句柄仅用于防 GC；PID 落盘供后续控制/清理。
 */
export async function restartWebServer(): Promise<void> {
  const env = webServerEnv();
  const serverLogFd = openSync(resolve(e2eRoot, 'logs', 'web', 'server.log'), 'a');
  const child: ChildProcess = spawn(serverBinary, [], {
    env,
    stdio: ['ignore', serverLogFd, serverLogFd],
    detached: true,
  });
  // 评审修复：spawn 失败（ENOENT/EACCES）时 'error' 事件若无监听即
  // 未处理异常——worker 以含糊崩溃收场且 pidFile 落 'undefined'。
  const spawnFailed = new Promise<never>((_, reject) => {
    child.once('error', err => reject(new Error(`server restart spawn 失败: ${err.message}`)));
  });
  if (!child.pid) throw new Error('server restart spawn 失败：无 PID');
  writeFileSync(pidFile, String(child.pid));
  const { EGOSYNC_PORT } = env;
  if (!EGOSYNC_PORT) throw new Error('EGOSYNC_PORT missing in web server env');
  await Promise.race([waitForPort(Number(EGOSYNC_PORT), 20000), spawnFailed]);
}

/**
 * 打开应用并等待就绪：
 * 1. 页面加载（SSE 长连接 ⇒ 不能用 networkidle，DOM 标记驱动）；
 * 2. 未登录则提交固定令牌登录（spec1 之后 Cookie 持久，走直达路径）；
 * 3. connection-status 徽标 data-state="online"（三态机落位锚点）。
 *
 * 认证限流预算（/api/auth/* + /api/setup 共 5 次/min/IP 滑动窗口）：
 * 首个 spec 标记窗口开启；后续 spec 加载前静默 61s 等待上一窗口条目
 * 老化——每个 spec 的 status（+登录 spec 的 login，+流式 spec 的刷新
 * status）≤3 次恒在预算内（结构化确定性，不依赖精确计数）。
 */
export async function openWebAppAndLogin(): Promise<void> {
  // 非首个 spec：静默 61s——上一 spec 的窗口条目全部老化出窗（本 spec 的
  // status + 可能的 login + 流式 spec 的刷新 status ≤3 次，恒在 5/min 内）
  if (existsSync(authWindowMarker)) {
    await browser.pause(61000);
  }
  await browser.url(webServerUrl());
  writeFileSync(authWindowMarker, String(Date.now()));

  // 登录视图存在则登录（首次 spec）；Cookie 持久后直达 App。
  // 评审修复：此前 browser.url() 后立即一次性 isExisting——冷加载慢时
  // React 尚未挂载，登录表单缺席被误判为「无需登录」，随后徽标 30s
  // 超时并报出误导性错误。改为有界等待「登录表单或徽标」二选一落地，
  // 再按实际在场者分流。
  const loginInput = await browser.$('#egosync-login-token');
  const badge = await browser.$('[data-testid="connection-status"]');
  await browser.waitUntil(async () => {
    if (await loginInput.isExisting()) return true;
    return badge.isExisting();
  }, { timeout: 15000, timeoutMsg: '登录视图或应用徽标 15 秒内均未出现（应用启动异常）' });
  if (await loginInput.isExisting()) {
    await loginInput.setValue(E2E_WEB_TOKEN);
    const submit = await browser.$('button[type="submit"]');
    await submit.click();
  }

  // App 就绪 + 连接状态徽标落位 online（badge 已在上面解析）
  await badge.waitForDisplayed({ timeout: 30000 });
  await browser.waitUntil(
    async () => (await badge.getAttribute('data-state')) === 'online',
    { timeout: 30000, timeoutMsg: 'connection-status did not reach online within 30s' },
  );
}

/**
 * 等待连接状态迁移到目标态（断线/恢复断言）。
 */
export async function waitForConnectionState(
  state: 'connecting' | 'online' | 'reconnecting',
  timeout = 30000,
): Promise<void> {
  const badge = await browser.$('[data-testid="connection-status"]');
  await browser.waitUntil(
    async () => (await badge.getAttribute('data-state')) === state,
    { timeout, timeoutMsg: `connection-status did not reach ${state}` },
  );
}

/** 重连横幅存在性（显著断线提示的 DOM 锚点）。 */
export async function reconnectBannerExists(): Promise<boolean> {
  const banner = await browser.$('[data-testid="connection-banner"]');
  return banner.isExisting();
}

/**
 * REST invoke：浏览器上下文内 fetch POST /api/cmd/{command}。
 *
 * 与桌面 invoke 错误通道同构解包：非 200 或判别头 X-Egosync-App-Error
 * ⇒ 抛错（含响应体）；否则返回解析值。credentials same-origin 带会话
 * Cookie（绕过 UI 直接造数据——seeding 同款纪律）。
 */
export async function webInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const result = (await browser.executeAsync(
    async (cmd: string, argObj: Record<string, unknown> | undefined, done: (val: unknown) => void) => {
      try {
        const res = await fetch(`/api/cmd/${cmd}`, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(argObj ?? {}),
        });
        const text = await res.text();
        let parsed: unknown;
        try {
          parsed = JSON.parse(text);
        } catch {
          parsed = text;
        }
        if (res.status !== 200) {
          done({ __error: `HTTP ${res.status}`, __body: parsed });
          return;
        }
        if (res.headers.get('x-egosync-app-error') != null) {
          done({ __error: 'AppError', __body: parsed });
          return;
        }
        done({ __value: parsed });
      } catch (e) {
        done({ __error: String(e) });
      }
    },
    command,
    args,
  )) as { __value?: T; __error?: string; __body?: unknown };

  if (result && typeof result === 'object' && '__error' in result) {
    throw new Error(`webInvoke ${command} failed: ${result.__error} ${JSON.stringify(result.__body)}`);
  }
  return (result as { __value: T }).__value as T;
}

/** 经 UI 创建角色并返回 roleId（web-ok 面造数据 + 真实 UI 路径双覆盖）。 */
export async function seedRoleViaCommand(name: string): Promise<string> {
  const role = await webInvoke<{ id: string }>('role_create', {
    input: { name, icon: 'briefcase', color: '#4F46E5' },
  });
  return role.id;
}

/** 发送一条管家消息（chat_send_message：插 user 行 + assistant 占位行）。 */
export async function sendButlerMessage(content: string): Promise<void> {
  // 无 LLM 环境：token 不落库，assistant 行恒 is_complete=false——
  // 刷新恢复用例的确定性锚点（「生成中」占位）
  await webInvoke('chat_send_message', { input: { message: content } });
}
