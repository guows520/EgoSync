// Story 16.1：CSP hash-source 契约耦合门禁（前端 index.html ⇄ server security.rs）。
//
// server 的 CSP_POLICY 对 script-src 只放行 `'self'` + 单条 hash-source
//（index.html 内联防 FOUC 主题脚本的 SHA256 base64）。此前「hash 与构建
// 产物 byte 对 byte 一致」只是纪律不是机制——改脚本内容/缩进或 Vite 构建
// 行为变化都会让 hash 静默失效（浏览器直接拒执行主题脚本 ⇒ 深浅色闪烁
// 回归）。本文件 source-scan 双侧断言（与 transport/contract.test.ts 的
// APP_ERROR_HEADER 门禁同款范式）：
//
// 1. index.html 源内联脚本的 sha256-base64 == security.rs CSP_POLICY 内
//    嵌的 hash-source（编辑脚本须同步重算 Rust 侧值，反之亦然）；
// 2. CSP 不含 script-src 'unsafe-inline'（16.1 script 侧唯一放行项是
//    hash-source——禁止整体放开，见 security.rs 头注释与架构 ⑧）；
// 3. 零第三方域（Story 17.1，人工裁决 B）：16.1 曾放行的两 Google 字体
//    域已随字体自托管（@fontsource-variable npm 包，woff2 随 dist 分发）
//    撤除——font-src/style-src 回归 `'self'`；断言收紧为整个 policy
//    不含任何 http(s) 外链源（离线/内网一致性与隐私）；
// 4. 已构建产物存在时：dist/index.html 的内联脚本与源 byte 对 byte 一致
//    （Vite 原样保留前提的回归门），且无其他 inline script、无 Google
//    Fonts 外链残留。

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

/** 提取 html 中首个非 module、无 src 的内联 <script> 本体。 */
function extractInlineScript(html: string): string | null {
  const m = html.match(/<script>([\s\S]*?)<\/script>/);
  return m ? m[1]! : null;
}

/** 把 Rust 字符串字面量里的 `\`+换行+行首空白 续行拼接还原成有效串。 */
function rustUnwrapContinuations(raw: string): string {
  return raw.replace(/\\\r?\n\s*/g, '');
}

describe('CSP hash-source 契约耦合（index.html ⇄ server security.rs）', () => {
  const indexHtml = readFileSync(resolve(process.cwd(), 'index.html'), 'utf8');
  const securityRs = readFileSync(
    resolve(process.cwd(), '..', 'server/src/security.rs'),
    'utf8'
  );

  const inlineScript = extractInlineScript(indexHtml);
  const policyMatch = securityRs.match(
    /pub const CSP_POLICY: &str = "([^"]+)"/
  );

  it('index.html 内联脚本存在且非空（防 FOUC 主题脚本本体）', () => {
    expect(inlineScript, 'index.html 应含非 module 内联 <script>').not.toBeNull();
    expect(Buffer.byteLength(inlineScript!)).toBeGreaterThan(0);
  });

  it('security.rs 应含 CSP_POLICY 常量（Rust 续行拼接后可解析）', () => {
    expect(policyMatch, 'security.rs 应含 CSP_POLICY 字符串常量').not.toBeNull();
  });

  it('内联脚本 sha256-base64 == CSP_POLICY 的 hash-source（byte 对 byte 钉死）', () => {
    const actual = createHash('sha256').update(inlineScript!).digest('base64');
    const policy = rustUnwrapContinuations(policyMatch![1]!);
    const hashInPolicy = policy.match(/'sha256-([A-Za-z0-9+/=]+)'/);

    expect(hashInPolicy, 'CSP_POLICY 的 script-src 应含 sha256 hash-source').not.toBeNull();
    // 契约核心：脚本本体（含缩进换行，byte 对 byte）hash 与放行值一致。
    // 不一致 ⇒ 浏览器拒执行主题脚本（深浅色 FOUC 回归）——编辑任一侧
    // 须同步另一侧（重算 hash）。
    expect(actual).toBe(hashInPolicy![1]!);
  });

  it('CSP_POLICY 保持冻结结构：script-src 无 unsafe-inline，指令集完整', () => {
    const policy = rustUnwrapContinuations(policyMatch![1]!);

    // script 侧唯一获批的放行项是 hash-source——整体放开 unsafe-inline
    // 被明确禁止（架构 ⑧ + 16.1 Design Notes 裁量）
    const scriptSrc = policy.match(/script-src ([^;]+)/)![1]!;
    expect(scriptSrc).not.toContain('unsafe-inline');
    expect(scriptSrc).toContain("'self'");

    // 冻结内容（架构 ⑧）：default-src 'self'; style-src 'self' 'unsafe-inline'
    //（React 运行时注入内联样式需要）; connect-src 'self'（同源 API/SSE）
    expect(policy).toContain("default-src 'self'");
    expect(policy).toContain("style-src 'self' 'unsafe-inline'");
    expect(policy).toContain("connect-src 'self'");

    // 零第三方域（Story 17.1，人工裁决 B）：16.1 曾放行的两 Google 字体域
    // 已随自托管撤除——font-src/style-src 回归 'self'；字体经
    // @fontsource-variable npm 包随 dist 分发（woff2 本地命中）
    expect(policy).toContain("font-src 'self'");
    // 整个 policy 不得含任何第三方域（http/https 外链源）——隐私面
    // 最小化 + 离线一致；fontsource 的 woff2 与 css 都在 'self' 之下。
    // i 标志（17.1 评审 #15）：CSP 域名源大小写不敏感，`https://FONTS.`
    // 之类大写源不得绕过门禁
    const externalSources = policy.match(/https?:\/\/[a-z.]+/gi) ?? [];
    expect(externalSources, `CSP 必须零第三方域，实得: ${externalSources.join(', ')}`).toEqual([]);
  });

  it('已构建产物：dist/index.html 内联脚本与源一致，且无其他 inline script', () => {
    const distPath = resolve(process.cwd(), 'dist/index.html');
    if (!existsSync(distPath)) {
      // 构建产物不存在（纯测试环境）——源侧契约已由上面的用例守门
      //（npm run build 后本用例实跑产物侧校验）
      return;
    }
    const distHtml = readFileSync(distPath, 'utf8');

    // 内联脚本 byte 对 byte 一致（Vite 原样保留 index.html——若构建管线
    // 改为压缩/改写内联脚本，此处立即红：CSP hash 会对不上实际产物）
    const distScript = extractInlineScript(distHtml);
    expect(distScript, 'dist/index.html 应保留内联主题脚本').not.toBeNull();
    expect(distScript!).toBe(inlineScript!);

    // 除防 FOUC 脚本外无其他 inline script（有 ⇒ CSP 拒执行且未在
    // hash-source 放行清单内——须外置或补 hash 裁量）
    const inlineCount = (distHtml.match(/<script>[\s\S]*?<\/script>/g) ?? []).length;
    expect(inlineCount).toBe(1);
    // module 入口（src/main.tsx → 构建后带 src 的外置 chunk）不算 inline
    expect(distHtml).toMatch(/<script[^>]+src=/);
    // Google Fonts 外链清零（17.1 自托管替换点回归门——外链回潮即红）
    expect(distHtml).not.toContain('fonts.googleapis.com');
    expect(distHtml).not.toContain('fonts.gstatic.com');

    // 字体产物正向断言（17.1 评审 #30）：上面的「外链缺席」对「什么
    // 都没有」天然成立——若 fontsource import 被删，字体整体静默消失
    // 仍全绿。此处断言 dist/assets 实际含 woff2 分片（@fontsource-
    // variable unicode-range 切片），字体丢失立即红。
    const assetsDir = resolve(process.cwd(), 'dist/assets');
    const woff2 = readdirSync(assetsDir).filter((f) => f.endsWith('.woff2'));
    expect(woff2.length, 'dist/assets 应含自托管 woff2 字体分片').toBeGreaterThan(0);
    // CSS 侧同样钉住：产物 CSS 引用 woff2（字体声明被打包进产物）
    const cssFiles = readdirSync(assetsDir).filter((f) => f.endsWith('.css'));
    const cssText = cssFiles.map((f) => readFileSync(resolve(assetsDir, f), 'utf8')).join('\n');
    expect(cssText).toMatch(/\.woff2/);
  });
});
