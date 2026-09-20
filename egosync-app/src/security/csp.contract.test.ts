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
// 3. 字体源放行（16.1 评审修复）：index.html 外链 Google Fonts——桌面
//    宿主（tauri csp:null）正常加载，web 不放行则字体回退系统字体，
//    产生宿主门控项之外的视觉分叉；style-src 放行 css 源、font-src
//    放行字体文件源，与桌面加载同链对齐；
// 4. 已构建产物存在时：dist/index.html 的内联脚本与源 byte 对 byte 一致
//    （Vite 原样保留前提的回归门），且无其他 inline script。

import { existsSync, readFileSync } from 'node:fs';
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

    // 字体源放行（16.1 评审修复——index.html:7 外链 Google Fonts）：
    // style-src 尾追 css 源、font-src 新增字体文件源；从 CSP 撤掉任一
    // ⇒ web 字体回退系统字体，与桌面（tauri csp:null）视觉分叉
    expect(policy).toContain("style-src 'self' 'unsafe-inline' https://fonts.googleapis.com");
    expect(policy).toContain("font-src 'self' https://fonts.gstatic.com");
    // 字体放行仅此两源——不引入其余第三方域（隐私面最小化）
    const fontSources = policy.match(/https:\/\/[a-z.]+/g) ?? [];
    expect([...new Set(fontSources)]).toEqual([
      'https://fonts.googleapis.com',
      'https://fonts.gstatic.com',
    ]);
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
  });
});
