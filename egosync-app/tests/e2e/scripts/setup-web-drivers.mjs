// Story 16.2 评审修复（V2 最小件）：Web e2e 驱动供给脚本。
//
// 背景：wdio.web.conf.ts 钉版 chromedriver 148.0.7778.97 落在
// tests/e2e/.chromedriver/（已 gitignore——运行时下载物同 node_modules
// 语义），此前「如何获得」只存在于故事文档散文里，换机器不可复现。
// 本脚本把供给变成可执行物：
//   1. 钉版 chromedriver 148.0.7778.97（与 Chrome for Testing 148 精确
//      匹配）缺失时从 chrome-for-testing-public 官方源下载解压；
//   2. 校验 Chrome 可执行文件在场（缺省读 puppeteer 缓存布局；可用
//      E2E_WEB_CHROME_BIN 覆盖）；
//   3. 幂等——已就位时零下载零改动直接退出。
//
// Story 17.3（CI 供给闭环）：新增 Linux 下钉版 Chrome for Testing
// 148 下载——CI（server-ci.yml web-e2e-smoke job）此前只装 driver 不装
// 浏览器，浏览器供给靠散文。下载落**与 wdio.web.conf.ts 缺省解析一致的
// puppeteer 缓存布局**（~/.cache/puppeteer/chrome/linux-<版本>/...），
// 供给后无需任何 env 即被配置消费。非 Linux 平台保持原「提示 + 手工
// 供给」行为（macOS puppeteer 布局不同，本地开发者经 npx puppeteer /
// E2E_WEB_CHROME_BIN 供给——行为面不变）。
//
// 用法：cd egosync-app/tests/e2e && npm run setup:web-drivers
// 覆盖项：E2E_WEB_CHROME_BIN（chrome 可执行文件路径）

import { existsSync, mkdirSync, chmodSync, statSync, rmSync } from 'node:fs';
import { writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';

const CHROMEDRIVER_VERSION = '148.0.7778.97';
const e2eRoot = resolve(import.meta.dirname, '..');
const chromedriverDir = join(e2eRoot, '.chromedriver');
const chromedriverBinary = join(chromedriverDir, 'chromedriver-linux64', 'chromedriver');
const defaultChromeBinary = join(
  process.env.HOME ?? '', '.cache', 'puppeteer', 'chrome',
  `linux-${CHROMEDRIVER_VERSION}`, 'chrome-linux64', 'chrome',
);
const chromeBinary = process.env.E2E_WEB_CHROME_BIN ?? defaultChromeBinary;
// Chrome for Testing 钉版下载（与 chromedriver 同版本源；仅 linux——CI 面）
const CHROME_DOWNLOAD_VERSION = CHROMEDRIVER_VERSION;
const chromeCacheDir = join(
  process.env.HOME ?? '', '.cache', 'puppeteer', 'chrome',
  `linux-${CHROME_DOWNLOAD_VERSION}`,
);

/** 下载到文件（钉版 zip 约 10MB 级；chrome 约 160MB——整段缓冲即可）。 */
async function download(url, dest) {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`下载失败 HTTP ${res.status}: ${url}`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  writeFileSync(dest, buf);
  console.log(`  已下载 ${(buf.length / 1024 / 1024).toFixed(1)}MB`);
}

async function ensureChromedriver() {
  if (existsSync(chromedriverBinary)) {
    console.log(`✓ chromedriver ${CHROMEDRIVER_VERSION} 已就位: ${chromedriverBinary}`);
    return;
  }
  mkdirSync(chromedriverDir, { recursive: true });
  // 官方已知良好版本元数据（Linux64 钉版直链）
  const url =
    `https://storage.googleapis.com/chrome-for-testing-public/${CHROMEDRIVER_VERSION}/linux64/chromedriver-linux64.zip`;
  const zipPath = join(tmpdir(), `chromedriver-${CHROMEDRIVER_VERSION}.zip`);
  console.log(`↓ 下载 chromedriver ${CHROMEDRIVER_VERSION} …`);
  console.log(`  ${url}`);
  await download(url, zipPath);
  const zipSize = statSync(zipPath).size;
  if (zipSize < 1 * 1024 * 1024) {
    throw new Error(`下载物异常（${zipSize} 字节，疑似错误页而非 zip）`);
  }
  execFileSync('unzip', ['-o', zipPath, '-d', chromedriverDir]);
  rmSync(zipPath, { force: true });
  chmodSync(chromedriverBinary, 0o755);
  console.log(`✓ chromedriver 就位: ${chromedriverBinary}`);
}

/**
 * chrome 二进制完整性校验（评审补丁 17.3 分诊 P13）：存在 ≠ 可执行——
 * 上次解压被磁盘满打断会留下残缺二进制，existsSync 幂等短路永不自愈，
 * 启动时才爆晦涩错误。能实际跑 --version 才算就位。
 */
function chromeBinaryRuns(): boolean {
  try {
    execFileSync(chromeBinary, ['--version', '--no-sandbox'], { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Linux 下钉版 Chrome for Testing 供给（Story 17.3）：缺失时下载解压到
 * wdio.web.conf.ts 缺省解析的 puppeteer 缓存布局（供给后零 env 消费）。
 * 非 Linux：保持提示 + 手工供给（本地开发者语义不变）。
 */
async function ensureChrome() {
  if (existsSync(chromeBinary) && chromeBinaryRuns()) {
    console.log(`✓ chrome 已就位（--version 校验通过）: ${chromeBinary}`);
    return;
  }
  if (existsSync(chromeBinary)) {
    // 残缺二进制：清除缓存目录后走重新下载（无此分支则永不自愈）。
    // 仅自愈**缺省缓存布局**——E2E_WEB_CHROME_BIN 自定义路径（可能是
    // 系统 chrome）绝不清除，改为明确报错交人工处理。
    if (chromeBinary !== defaultChromeBinary) {
      throw new Error(`E2E_WEB_CHROME_BIN 指定的 chrome 无法执行（--version 失败）: ${chromeBinary}`);
    }
    console.warn(`⚠ chrome 二进制存在但无法执行（疑似上次解压中断的残缺产物）——清除重下: ${chromeBinary}`);
    rmSync(chromeCacheDir, { recursive: true, force: true });
  }
  if (process.platform !== 'linux') {
    console.warn(
      `⚠ Chrome for Testing 148 未找到: ${chromeBinary}\n` +
      `  非 Linux 平台不自动下载（布局差异）；可经 E2E_WEB_CHROME_BIN 指定本机\n` +
      `  chrome 路径，或经 puppeteer 安装：\n` +
      `    npx puppeteer browsers install chrome@148\n` +
      `  （版本须与 chromedriver ${CHROMEDRIVER_VERSION} 匹配）`,
    );
    return;
  }
  const url =
    `https://storage.googleapis.com/chrome-for-testing-public/${CHROME_DOWNLOAD_VERSION}/linux64/chrome-linux64.zip`;
  const zipPath = join(tmpdir(), `chrome-for-testing-${CHROME_DOWNLOAD_VERSION}.zip`);
  console.log(`↓ 下载 Chrome for Testing ${CHROME_DOWNLOAD_VERSION} …`);
  console.log(`  ${url}`);
  await download(url, zipPath);
  const zipSize = statSync(zipPath).size;
  if (zipSize < 50 * 1024 * 1024) {
    throw new Error(`下载物异常（${zipSize} 字节，疑似错误页而非 chrome zip）`);
  }
  // unzip 不递归建目录——父链 ~/.cache/puppeteer/chrome 不存在时直接失败
  // （本机与 CI ubuntu runner 同病：缓存目录并非预先存在）。chromedriver
  // 分支同款 mkdirSync 在先（:62）。
  mkdirSync(chromeCacheDir, { recursive: true });
  execFileSync('unzip', ['-o', zipPath, '-d', chromeCacheDir]);
  rmSync(zipPath, { force: true });
  chmodSync(chromeBinary, 0o755);
  if (!existsSync(chromeBinary)) {
    throw new Error(`解压后未找到 chrome 可执行文件: ${chromeBinary}`);
  }
  // 下载完整性二道关（P13）：能跑才算数——截断 zip 解压出的残缺二进制
  // 在此暴露为明确错误（而非 wdio 启动期的晦涩崩溃）
  if (!chromeBinaryRuns()) {
    throw new Error(`chrome 二进制无法执行（下载或解压损坏）: ${chromeBinary}\n` +
      `  清除缓存后重试: rm -rf "${dirname(chromeBinary)}"`);
  }
  console.log(`✓ Chrome for Testing 就位: ${chromeBinary}`);
}

ensureChromedriver()
  .then(ensureChrome)
  .catch(err => {
    console.error(`✗ 供给失败: ${err.message}`);
    process.exit(1);
  });
