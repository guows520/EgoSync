import { spawn, execSync, ChildProcess } from 'child_process';
import { existsSync, mkdirSync, rmSync, openSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { homedir } from 'os';
import { fileURLToPath } from 'url';
import net from 'net';
import { browser } from '@wdio/globals';

const __dirname = dirname(fileURLToPath(import.meta.url));

const isWindows = process.platform === 'win32';
const isLinux = process.platform === 'linux';

const binaryName = isWindows ? 'egosync.exe' : 'egosync';
const binaryPath = resolve(__dirname, '..', '..', 'src-tauri', 'target', 'release', binaryName);

const screenshotsDir = resolve(__dirname, 'screenshots');
const logsDir = resolve(__dirname, 'logs');
const a11yReportsDir = resolve(__dirname, 'reports', 'accessibility');
const perfReportsDir = resolve(__dirname, 'reports', 'performance');

// 等待 tauri-driver 在指定端口就绪，避免第一个 worker 连接时 ECONNREFUSED
function waitForDriverPort(port: number, timeout = 15000): Promise<void> {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const tryConnect = () => {
      const socket = net.connect({ host: '127.0.0.1', port });
      socket.setTimeout(1000);
      socket.on('connect', () => {
        socket.destroy();
        resolve();
      });
      socket.on('error', () => {
        socket.destroy();
        if (Date.now() - start > timeout) {
          reject(new Error(`tauri-driver port ${port} not ready within ${timeout}ms`));
        } else {
          setTimeout(tryConnect, 500);
        }
      });
      socket.on('timeout', () => {
        socket.destroy();
        if (Date.now() - start > timeout) {
          reject(new Error(`tauri-driver port ${port} not ready within ${timeout}ms`));
        } else {
          setTimeout(tryConnect, 500);
        }
      });
    };
    tryConnect();
  });
}

// 应用数据目录由 tauri.conf.json 的 identifier 决定（com.egosync.desktop），
// Tauri 2.x 的 app_data_dir() 据此解析，应用 DB 实际写入此目录（见 src-tauri/src/lib.rs）。
function getAppDataDir(): string {
  if (isWindows) {
    return join(process.env.APPDATA || join(homedir(), 'AppData', 'Roaming'), 'com.egosync.desktop');
  }
  return join(homedir(), '.config', 'com.egosync.desktop');
}

let tauriDriverProcess: ChildProcess | null = null;

export const config: WebdriverIO.Config = {
  hostname: '127.0.0.1',
  port: 4444,
  specs: ['./specs/**/*.ts'],
  suites: {
    ci: ['./specs/*.spec.ts'],
    perf: ['./specs/performance.spec.ts'],
  },
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'wry',
      'wdio:enforceWebDriverClassic': true,
      'tauri:options': {
        application: binaryPath,
      },
    } as any,
  ],
  logLevel: 'warn',
  waitforTimeout: 30000,
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  reporters: ['spec'],

  onPrepare: () => {
    if (existsSync(screenshotsDir)) {
      rmSync(screenshotsDir, { recursive: true, force: true });
    }
    mkdirSync(screenshotsDir, { recursive: true });
    if (existsSync(logsDir)) {
      rmSync(logsDir, { recursive: true, force: true });
    }
    mkdirSync(logsDir, { recursive: true });
    if (existsSync(a11yReportsDir)) {
      rmSync(a11yReportsDir, { recursive: true, force: true });
    }
    mkdirSync(a11yReportsDir, { recursive: true });
    if (existsSync(perfReportsDir)) {
      rmSync(perfReportsDir, { recursive: true, force: true });
    }
    mkdirSync(perfReportsDir, { recursive: true });
  },

  beforeSession: async () => {
    // 杀掉残留的 app 进程，释放 DB 文件锁
    try {
      if (isWindows) {
        execSync('taskkill /f /im egosync.exe 2>nul', { stdio: 'ignore' });
      } else {
        execSync('pkill -f egosync 2>/dev/null', { stdio: 'ignore' });
      }
    } catch {
      // ignore if no process found
    }

    const appDataDir = getAppDataDir();
    const dbFiles = ['egosync.db', 'conversations.db'];
    for (const dbFile of dbFiles) {
      const dbPath = join(appDataDir, dbFile);
      if (existsSync(dbPath)) {
        rmSync(dbPath, { force: true });
      }
    }

    // tauri-driver 输出落盘为日志文件，供 CI 失败时上传（满足 AC3「截图+日志」）
    const driverLogFd = openSync(join(logsDir, 'tauri-driver.log'), 'a');
    if (isLinux) {
      // xvfb 无 GPU 环境中 WebKitGTK DMABUF 渲染器和合成模式均无法工作，需全部禁用
      // 见 https://v2.tauri.app/develop/debug/linux-graphics/
      process.env.WEBKIT_DISABLE_DMABUF_RENDERER = '1';
      process.env.WEBKIT_DISABLE_COMPOSITING_MODE = '1';
      process.env.LIBGL_ALWAYS_SOFTWARE = '1';
      tauriDriverProcess = spawn('tauri-driver', [], {
        stdio: ['ignore', driverLogFd, driverLogFd],
        detached: true,
        shell: true,
      });
    } else if (isWindows) {
      const msedgedriverPath = join(process.env.LOCALAPPDATA || '', 'msedgedriver', 'msedgedriver.exe');
      const driverArgs = existsSync(msedgedriverPath)
        ? ['--native-driver', msedgedriverPath]
        : [];
      tauriDriverProcess = spawn('tauri-driver.exe', driverArgs, {
        stdio: ['ignore', driverLogFd, driverLogFd],
        detached: true,
        shell: true,
      });
    }

    // 等待 driver 端口就绪，避免 wdio 连接时 ECONNREFUSED
    if (tauriDriverProcess) {
      try {
        await waitForDriverPort(4444);
      } catch (e) {
        console.error(`[wdio.conf] ${e}`);
      }
    }
  },

  afterSession: () => {
    if (tauriDriverProcess) {
      try {
        tauriDriverProcess.kill();
      } catch {
        // ignore
      }
      tauriDriverProcess = null;
    }
    // 杀掉 app 进程，释放 DB 文件锁供下一个 spec 使用
    try {
      if (isWindows) {
        execSync('taskkill /f /im egosync.exe 2>nul', { stdio: 'ignore' });
      } else {
        execSync('pkill -f egosync 2>/dev/null', { stdio: 'ignore' });
      }
    } catch {
      // ignore
    }
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
