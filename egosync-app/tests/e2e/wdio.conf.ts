import { spawn, execSync, ChildProcess } from 'child_process';
import { existsSync, mkdirSync, rmSync, openSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { homedir } from 'os';
import { fileURLToPath } from 'url';
import { browser } from '@wdio/globals';

const __dirname = dirname(fileURLToPath(import.meta.url));

const isWindows = process.platform === 'win32';
const isLinux = process.platform === 'linux';

const binaryName = isWindows ? 'egosync.exe' : 'egosync';
const binaryPath = resolve(__dirname, '..', '..', 'src-tauri', 'target', 'release', binaryName);

const screenshotsDir = resolve(__dirname, 'screenshots');
const logsDir = resolve(__dirname, 'logs');

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
  },

  beforeSession: () => {
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
      tauriDriverProcess = spawn('tauri-driver', [], {
        stdio: ['ignore', driverLogFd, driverLogFd],
        detached: true,
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
