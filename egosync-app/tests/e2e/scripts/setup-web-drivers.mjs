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
// 用法：cd egosync-app/tests/e2e && npm run setup:web-drivers
// 覆盖项：E2E_WEB_CHROME_BIN（chrome 可执行文件路径）

import { existsSync, mkdirSync, chmodSync, statSync, rmSync } from 'node:fs';
import { writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const CHROMEDRIVER_VERSION = '148.0.7778.97';
const e2eRoot = resolve(import.meta.dirname, '..');
const chromedriverDir = join(e2eRoot, '.chromedriver');
const chromedriverBinary = join(chromedriverDir, 'chromedriver-linux64', 'chromedriver');
const defaultChromeBinary = join(
  process.env.HOME ?? '', '.cache', 'puppeteer', 'chrome',
  `linux-148.0.7778.97`, 'chrome-linux64', 'chrome',
);
const chromeBinary = process.env.E2E_WEB_CHROME_BIN ?? defaultChromeBinary;

/** 下载到文件（钉版 zip 约 10MB，整段缓冲即可）。 */
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

function ensureChrome() {
  if (!existsSync(chromeBinary)) {
    console.warn(
      `⚠ Chrome for Testing 148 未找到: ${chromeBinary}\n` +
      `  可经 E2E_WEB_CHROME_BIN 指定本机 chrome 路径，或经 puppeteer 安装：\n` +
      `    npx puppeteer browsers install chrome@148\n` +
      `  （版本须与 chromedriver ${CHROMEDRIVER_VERSION} 匹配）`,
    );
    return;
  }
  console.log(`✓ chrome 已就位: ${chromeBinary}`);
}

ensureChromedriver()
  .then(ensureChrome)
  .catch(err => {
    console.error(`✗ 供给失败: ${err.message}`);
    process.exit(1);
  });
