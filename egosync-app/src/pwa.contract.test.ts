// Story 16.4：PWA 手写件契约（manifest + Service Worker）源码扫描守门。
//
// 与 e2e（web-mobile.spec.ts 的浏览器侧断言）互补：CI 无浏览器时用源码
// 扫描钉死两条红线——
// 1. manifest 字段齐备（name/short_name/start_url/display=standalone/
//    theme_color/192+512 maskable 图标）；
// 2. Service Worker 缓存纪律（NFR-C3）：仅外壳缓存——/api/* 零缓存旁路、
//    无 localStorage/IndexedDB 业务缓存（业务数据零落盘红线，源码层禁入）；
// 3. index.html 已挂 manifest link + theme-color meta（安装入口在场）。

import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

function readPublic(file: string): string {
  const path = resolve(process.cwd(), 'public', file);
  expect(existsSync(path), `public/${file} 应存在（PWA 三件套手写落地）`).toBe(true);
  return readFileSync(path, 'utf8');
}

describe('PWA 契约（Story 16.4：manifest + Service Worker）', () => {
  it('manifest.webmanifest 字段齐备（安装元数据：名称/启动/standalone/主题色/maskable 图标）', () => {
    const manifest = JSON.parse(readPublic('manifest.webmanifest'));

    expect(manifest.name).toBe('EgoSync 数字分身');
    expect(manifest.short_name).toBe('EgoSync');
    // id 与 start_url 一致（安装身份锚点——PWA 最佳实践）
    expect(manifest.id).toBe('/');
    expect(manifest.start_url).toBe('/');
    expect(manifest.scope).toBe('/');
    expect(manifest.display).toBe('standalone');
    // 不锁 orientation：横竖屏随设备（宽度断点体系的前提——UX 未授权锁定）
    expect(manifest.orientation).toBeUndefined();
    // theme-color 取设计 token #4F46E5（与 maskable 图标底色同源）
    expect(manifest.theme_color).toBe('#4F46E5');
    const icons = manifest.icons as Array<{ src: string; sizes: string; type: string; purpose?: string }>;
    // purpose "any maskable"：常规用途 + maskable 安全区适配双声明
    expect(icons.some(i => i.sizes === '192x192' && i.type === 'image/png' && i.purpose === 'any maskable')).toBe(true);
    expect(icons.some(i => i.sizes === '512x512' && i.type === 'image/png' && i.purpose === 'any maskable')).toBe(true);
  });

  it('Service Worker 缓存纪律：/api/* 零缓存旁路 + 业务数据零落盘（NFR-C3 红线）', () => {
    const sw = readPublic('sw.js');

    // 业务 API 直通零缓存（含 /api/cmd 与 SSE 的 fetch 形态）
    expect(sw).toContain("pathname === '/api' || url.pathname.startsWith('/api/')");
    // 外壳缓存纪律在注释与实现两侧都在场（防无意识重构漂移）
    expect(sw).toContain('stale-while-revalidate');
    expect(sw).toContain('cache-first');
    expect(sw).toContain('egosync-shell-v1');
    expect(sw).toContain("'/index.html'");

    // 业务数据红线：SW **代码**（剥离注释后）禁出现 localStorage/IndexedDB
    // ——业务缓存只允许服务端；主题偏好等非业务态归 index.html FOUC 脚本，
    // 不经 SW。注释里提到这些词是纪律说明，不算违规。
    const swCode = sw.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
    expect(swCode).not.toContain('localStorage');
    expect(swCode).not.toContain('indexedDB');
    expect(swCode).not.toContain('caches.open("egosync-data');
  });

  it('index.html 挂 manifest link + apple meta + theme-color（安装入口在场，FOUC 脚本零改动）', () => {
    const html = readFileSync(resolve(process.cwd(), 'index.html'), 'utf8');

    expect(html).toContain('<link rel="manifest" href="/manifest.webmanifest" />');
    expect(html).toContain('<link rel="apple-touch-icon" href="/icons/apple-touch-icon.png" />');
    expect(html).toContain('<meta name="apple-mobile-web-app-capable" content="yes" />');
    expect(html).toContain('<meta name="theme-color" content="#4F46E5" />');

    // FOUC 内联脚本仍在（CSP hash byte 契约的另一半由 csp.contract.test.ts 守）
    expect(html).toContain("localStorage.getItem('egosync-theme')");
  });

  it('SW 注册失败静默降级（SW_FAIL 矩阵行）：门控 + try/catch + 非阻塞', () => {
    const main = readFileSync(resolve(process.cwd(), 'src', 'main.tsx'), 'utf8');

    // 门控：仅生产构建 + 浏览器宿主（dev 热更新/桌面壳不经 SW——桌面零影响）
    expect(main).toContain('import.meta.env.PROD');
    expect(main).toContain('isTauriHost()');
    // 双重静默：Promise 拒（.catch）不可逃逸 + 同步 throw（try/catch）不可
    // 逃逸——隐私模式等失败不阻断引导（I/O 矩阵 SW_FAIL 行的预期行为）
    expect(main).toMatch(/try\s*\{[\s\S]*?serviceWorker\.register[\s\S]*?\}\s*catch/);
    expect(main).toMatch(/register\('\/sw\.js'\)\s*\.\s*catch/);
    // fire-and-forget：注册不 await——失败不卡引导、成功不抢首屏资源
    expect(main).toContain('void navigator.serviceWorker.register');
  });
});
