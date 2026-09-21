// Story 16.2：Web 模式（浏览器宿主）端到端测试——wdio 独立配置。
//
// 与桌面链路（wdio.conf.ts，tauri-driver + wry WebView）零共享状态：
// - 浏览器：Chrome for Testing 148（puppeteer 缓存二进制）+ chromedriver
//   148（tests/e2e/.chromedriver 钉版下载，版本与浏览器精确匹配）；
// - 服务端：egosync-server debug 构建（EGOSYNC_* env 驱动，静态目录指向
//   egosync-app/dist）——onPrepare 起服、onComplete 关服，跨进程 PID 文件
//   控制（reconnect spec 在 worker 内 kill/重启同 env 实例）；
// - 认证限流预算（/api/auth/* + /api/setup 共 5 次/min/IP）：持久
//   user-data-dir 保 Cookie（会话 30 天）——spec1 走 status+login 共 2 次，
//   后续各 spec 仅 1 次 status，3 specs 共 4 次 < 5。
//
// 就绪探测：/healthz 200（服务端）+ DOM 标记（connection-status
// data-state="online"，前端 HttpTransport 三态机落位）。

import { spawn, execFileSync, ChildProcess } from 'child_process';
import { existsSync, mkdirSync, rmSync, writeFileSync, readFileSync, openSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import net from 'net';
import { browser } from '@wdio/globals';
import { DatabaseSync } from 'node:sqlite';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── 布局常量（相对 tests/e2e/） ────────────────────────────────────────────
const e2eRoot = __dirname;
const repoRoot = resolve(e2eRoot, '..', '..', '..');
const serverBinary = resolve(repoRoot, 'server', 'target', 'debug', 'egosync-server');
const staticDir = resolve(repoRoot, 'egosync-app', 'dist');
// Chrome 与 chromedriver 路径可经 env 覆盖（setup-web-drivers.mjs 之外的
// 供给路径——CI 或本机自定义安装位置），缺省为钉版布局（puppeteer 缓存
// Chrome for Testing 148 + tests/e2e/.chromedriver/ 钉版 driver）。
const chromeBinary = process.env.E2E_WEB_CHROME_BIN ?? join(
  process.env.HOME ?? '',
  '.cache', 'puppeteer', 'chrome', 'linux-148.0.7778.97', 'chrome-linux64', 'chrome',
);
const chromedriverBinary = process.env.E2E_WEB_CHROMEDRIVER_BIN ?? resolve(
  e2eRoot, '.chromedriver', 'chromedriver-linux64', 'chromedriver',
);
const dataDir = resolve(e2eRoot, 'logs', 'web-server-data');
const logsDir = resolve(e2eRoot, 'logs', 'web');
const screenshotsDir = resolve(e2eRoot, 'screenshots', 'web');
const pidFile = resolve(logsDir, 'web-server.pid');
/** 首个页面加载标记（限流窗口隔离——web-helper 读写；onPrepare 清残留）。 */
const authWindowMarker = resolve(logsDir, 'auth-window-opened');

// ── 服务端 env（固定端口，spec 内跨进程控制用同 env 重启） ────────────────
const WEB_PORT = 18080;
const WEB_TOKEN = 'e2e-web-fixed-token';

export function webServerEnv(): Record<string, string> {
  return {
    ...process.env,
    EGOSYNC_DATA_DIR: dataDir,
    EGOSYNC_TOKEN: WEB_TOKEN,
    EGOSYNC_STATIC_DIR: staticDir,
    EGOSYNC_HOST: '127.0.0.1',
    EGOSYNC_PORT: String(WEB_PORT),
  } as Record<string, string>;
}

export function webServerUrl(): string {
  return `http://127.0.0.1:${WEB_PORT}`;
}

/** 等待端口可连接（TCP 探测，间隔 200ms）。 */
export function waitForPort(port: number, timeout = 20000): Promise<void> {
  return new Promise((resolvePromise, reject) => {
    const start = Date.now();
    const tryConnect = () => {
      const socket = net.connect({ host: '127.0.0.1', port });
      socket.setTimeout(1000);
      socket.on('connect', () => {
        socket.destroy();
        resolvePromise();
      });
      socket.on('error', () => {
        socket.destroy();
        if (Date.now() - start > timeout) {
          reject(new Error(`port ${port} not ready within ${timeout}ms`));
        } else {
          setTimeout(tryConnect, 200);
        }
      });
      socket.on('timeout', () => {
        socket.destroy();
        if (Date.now() - start > timeout) {
          reject(new Error(`port ${port} not ready within ${timeout}ms`));
        } else {
          setTimeout(tryConnect, 200);
        }
      });
    };
    tryConnect();
  });
}

/** 等待端口拒绝连接（进程退出/优雅收尾探测，间隔 200ms）。 */
export function waitForPortClosed(port: number, timeout = 15000): Promise<void> {
  return new Promise((resolvePromise, reject) => {
    const start = Date.now();
    const tryConnect = () => {
      const socket = net.connect({ host: '127.0.0.1', port });
      socket.setTimeout(1000);
      socket.on('connect', () => {
        socket.destroy();
        if (Date.now() - start > timeout) {
          reject(new Error(`port ${port} still open within ${timeout}ms`));
        } else {
          setTimeout(tryConnect, 200);
        }
      });
      socket.on('error', () => {
        socket.destroy();
        resolvePromise();
      });
      socket.on('timeout', () => {
        // 评审修复：连接停摆（如 accept 积压）≠ 端口已关——此前直接按
        // 「已关闭」收场，随后的 node:sqlite 直写可能与活着的服务发生
        // 锁竞争（本套件「意外发现」里记录的正是这类连锁故障）。
        socket.destroy();
        if (Date.now() - start > timeout) {
          reject(new Error(`port ${port} connect stalled beyond ${timeout}ms`));
        } else {
          setTimeout(tryConnect, 200);
        }
      });
    };
    tryConnect();
  });
}

let serverProcess: ChildProcess | null = null;

/**
 * 探测端口占用者（ss -tlnp 解析 + /proc cmdline 校验）——无占用返回 null。
 * Story 17.3 验证期事故加固：孤儿 egosync-server 占端口时，waitForPort 会被
 * 僵尸应答误判为「新服务端就绪」，后续全线雪崩；本函数让 onPrepare 能在
 * 起服前发现并处置。ss/proc 不可用（非 Linux）时返回 null——退化为不检查。
 */
/** PID 身份核验（评审补丁 P12 余项，与 web-helper isEgoSyncServerPid 同纪律）：
 * PID 文件陈旧且 PID 被系统回收复用时，盲杀会命中无关进程——先验
 * /proc/{pid}/cmdline 确是 egosync-server；非 Linux /proc 缺席时维持旧杀语义。 */
function pidIsEgosyncServer(pid: number): boolean {
  if (!existsSync('/proc')) return true;
  try {
    return readFileSync(`/proc/${pid}/cmdline`, 'utf-8').includes('egosync-server');
  } catch {
    return false; // 进程已死——无需杀
  }
}

function findPortSquatter(port: number): { pid: number; isEgosyncServer: boolean } | null {
  try {
    const stdout = execFileSync('ss', ['-tlnp'], { encoding: 'utf-8', stdio: ['ignore', 'pipe', 'ignore'] });
    const line = stdout.split('\n').find(l => new RegExp(`:${port}\\b`).test(l));
    if (!line) return null;
    const pidMatch = line.match(/pid=(\d+)/);
    if (!pidMatch) return null;
    const pid = Number(pidMatch[1]);
    let isEgosyncServer = false;
    try {
      // 只对「确证是本仓库 egosync-server 二进制」的占用者自动清理——
      // 其它任何进程占用端口都只报错不杀，绝不误杀用户进程。
      const cmdline = readFileSync(`/proc/${pid}/cmdline`, 'utf-8');
      isEgosyncServer = cmdline.includes('egosync-server');
    } catch {
      // /proc 不可读——按未知占用者处理（只报错）
    }
    return { pid, isEgosyncServer };
  } catch {
    return null; // ss 不可用等——维持旧行为（不检查）
  }
}

export const config: WebdriverIO.Config = {
  // 留空 hostname/port ⇒ wdio startWebDriver 自动拉起 chromedriver
  // （CHROMEDRIVER_PATH 指向钉版二进制），worker 直连本机随机端口。
  // Web specs 与桌面 specs（specs/**/*.ts）分目录——桌面链路 wdio.conf.ts
  // 的 glob 零改动即不吞 web specs（桌面零回归约束）
  specs: ['./web-specs/*.spec.ts'],
  // Story 17.3：套件切分（Design Notes「冒烟套件切分」）——
  // - smoke（CI 冒烟，server-ci.yml web-e2e-smoke job 消费）：
  //   web-streaming + web-reconnect——无时钟对齐依赖（确定性），负载
  //   抖动敏感度低；
  // - full（本地/夜跑全量，`npm run test:web` 缺省即全部 specs）：
  //   加 web-events / web-resident-loop——含 60s tick 对齐与 LLM stub，
  //   留本地全量（`npm run test:web:smoke` 跑 smoke 子集）。
  // 用法：`npm run test:web:smoke`（--suite smoke）；不指定 suite 时跑
  // specs 全量（缺省行为零变化）。
  suites: {
    smoke: [
      './web-specs/web-streaming.spec.ts',
      './web-specs/web-reconnect.spec.ts',
    ],
  },
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'chrome',
      'goog:chromeOptions': {
        binary: chromeBinary,
        args: [
          // 持久 user-data-dir：跨 spec 保 Cookie（认证限流预算关键）
          `--user-data-dir=${resolve(e2eRoot, 'logs', 'web-user-data')}`,
          '--no-sandbox',
          '--disable-dev-shm-usage',
          '--disable-gpu',
          // 评审修复：显式 headless——此前依赖 wdio 自动 Xvfb 兜底（隐式
          // 显示依赖，CI/无显示环境的不确定性来源）
          '--headless=new',
        ],
      },
      'wdio:chromedriverOptions': {
        binary: chromedriverBinary,
      },
    } as any,
  ],
  logLevel: 'warn',
  waitforTimeout: 20000,
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    // 评审修复：61s 限流老化静默 + 30s 服务端锚点轮询 + 30s 刷新恢复等待
    // 在慢环境下可能叠加逼近 120s——放宽到 180s 杜绝 hook/test 环境性超时
    timeout: 180000,
  },
  reporters: ['spec'],

  onPrepare: async () => {
    // 前置校验：二进制缺失即刻失败（错误信息可定位）
    if (!existsSync(serverBinary)) {
      throw new Error(`server binary not found: ${serverBinary}（先 cd server && cargo build）`);
    }
    if (!existsSync(chromeBinary)) {
      throw new Error(`chrome binary not found: ${chromeBinary}`);
    }
    if (!existsSync(chromedriverBinary)) {
      throw new Error(
        `chromedriver not found: ${chromedriverBinary}（先 npm run setup:web-drivers 下载钉版，或经 E2E_WEB_CHROMEDRIVER_BIN 指定）`,
      );
    }
    if (!existsSync(join(staticDir, 'index.html'))) {
      throw new Error(`dist not built: ${staticDir}（先 cd egosync-app && npm run build）`);
    }

    // 输出目录
    for (const dir of [logsDir, screenshotsDir, resolve(e2eRoot, 'screenshots')]) {
      mkdirSync(dir, { recursive: true });
    }
    rmSync(join(e2eRoot, 'logs', 'web-user-data'), { recursive: true, force: true });
    mkdirSync(join(e2eRoot, 'logs', 'web-user-data'), { recursive: true });

    // 数据目录清空（全新库 ⇒ env 态 EGOSYNC_TOKEN 仅 login 无 setup）
    rmSync(dataDir, { recursive: true, force: true });
    mkdirSync(dataDir, { recursive: true });

    // 残留服务端进程清理（上一轮异常退出）——身份核验后才杀（评审补丁
    // P12 余项：PID 回收复用时盲杀会命中无关进程；核验失败仅清文件不杀）
    if (existsSync(pidFile)) {
      try {
        const pid = Number(readFileSync(pidFile, 'utf-8').trim());
        if (Number.isFinite(pid) && pidIsEgosyncServer(pid)) process.kill(pid, 'SIGKILL');
      } catch {
        // 已死/无权限——忽略
      }
      rmSync(pidFile, { force: true });
    }
    // 端口占用守卫（17.3 验证期事故加固）：PID 文件只追踪「最后写入」的
    // 实例——若存在未被追踪的孤儿 egosync-server 仍占着端口，起服后
    // waitForPort 会被僵尸应答误判为就绪。此处主动处置：孤儿 egosync-server
    // 直接清理（SIGKILL + 等端口释放）；其它占用者明确报错，绝不误杀。
    const squatter = findPortSquatter(WEB_PORT);
    if (squatter) {
      if (squatter.isEgosyncServer) {
        console.warn(
          `[wdio.web.conf] 端口 ${WEB_PORT} 被孤儿 egosync-server(pid=${squatter.pid})占用——已清理`,
        );
        try {
          process.kill(squatter.pid, 'SIGKILL');
        } catch {
          // ss 扫描与 kill 之间恰好退出——继续（waitForPortClosed 兜底）
        }
        await waitForPortClosed(WEB_PORT, 10000);
      } else {
        throw new Error(
          `端口 ${WEB_PORT} 被非 egosync-server 进程(pid=${squatter.pid})占用——请手工排查后重跑`,
        );
      }
    }
    // 限流窗口标记清残留（上一轮遗留会让首个 spec 白等 61s）
    rmSync(authWindowMarker, { force: true });

    // 起服（detached——onPrepare 在 launcher 进程，worker 内通过 PID 文件控制）
    const serverLogFd = openSync(join(logsDir, 'server.log'), 'a');
    const startServer = (): ChildProcess => {
      const child = spawn(serverBinary, [], {
        env: webServerEnv(),
        stdio: ['ignore', serverLogFd, serverLogFd],
        detached: true,
      });
      writeFileSync(pidFile, String(child.pid));
      return child;
    };
    serverProcess = startServer();
    console.log(`[wdio.web.conf] server pid=${serverProcess.pid} port=${WEB_PORT}`);

    // 首轮引导预置：首次 boot 建库（迁移）后直写 onboarding_completed=true
    // （node:sqlite，DB 侧零认证请求——限流预算保护：3 specs 共 5 次
    // /api/auth 请求，滑动窗口 60s 内自然老化），再重启使 spec1 登录后
    // 直达管家视图（无 LLM 环境下 onboarding UI 卡在「配置大模型服务」
    // 步骤无法走通，与桌面 e2e seedCompleteOnboarding 同语义）。
    await waitForPort(WEB_PORT, 20000);
    serverProcess.kill();
    serverProcess = null;
    // 等待端口真正关闭（优雅退出收尾 ≠ 进程即死）——随后 DB 直写零锁竞争
    await waitForPortClosed(WEB_PORT, 15000);
    const db = new DatabaseSync(join(dataDir, 'egosync.db'));
    db.prepare(
      "INSERT INTO app_settings (key, value) VALUES ('onboarding_completed', 'true') " +
      'ON CONFLICT(key) DO UPDATE SET value = excluded.value',
    ).run();
    db.close();
    serverProcess = startServer();
    await waitForPort(WEB_PORT, 20000);
    console.log(`[wdio.web.conf] server restarted pid=${serverProcess.pid}（onboarding 已预置）`);
  },

  onComplete: () => {
    if (serverProcess) {
      try {
        serverProcess.kill();
      } catch {
        // ignore
      }
      serverProcess = null;
    }
    // 兜底：reconnect spec 重启后的实例 PID 也写同一 pidFile——重读并清理
    try {
      const pid = Number(readFileSync(pidFile, 'utf-8').trim());
      if (Number.isFinite(pid)) process.kill(pid, 'SIGKILL');
    } catch {
      // 已死——忽略
    }
    rmSync(pidFile, { force: true });
  },

  afterTest: async function (_test, _context, result) {
    if (result.error) {
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const screenshotPath = join(screenshotsDir, `fail-${timestamp}.png`);
      try {
        await browser.saveScreenshot(screenshotPath);
        console.log(`Screenshot saved: ${screenshotPath}`);
      } catch (e) {
        console.log(`Failed to save screenshot: ${e}`);
      }
    }
  },
};
