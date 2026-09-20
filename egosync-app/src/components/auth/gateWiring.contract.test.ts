// Story 16.1 评审修复：main.tsx 的 App-in-AuthGate 接线契约门禁。
//
// 「gate 先于任何事件订阅」硬约束的唯一落点是 main.tsx 的 JSX 结构
// （App 仅在 ready 态挂载 ⇒ 未认证零 useEngineEvent/SSE 订阅）——此前
// 只是纪律不是机制：重构把 <App /> 挪出 <AuthGate> 子树的话，全部既有
// 测试照绿（AuthGate 测试自带 children、App 测试不关心挂载位置），真实
// 浏览器未认证即挂 App ⇒ 业务 invoke 全 401 ⇒ SSE 重建循环回归。
//
// 本文件 source-scan 断言（与 csp.contract.test.ts / transport/
// contract.test.ts 同款范式）：
// 1. `<App />` 恰出现一次，且位于 `<AuthGate>…</AuthGate>` 子树内；
// 2. settings-demo 演示分支宿主门控（web 不渲染——未认证可达会满屏
//    401 错误态）。

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('入口接线契约：App 必须位于 AuthGate 子树内（gate 先于一切订阅）', () => {
  const mainSource = readFileSync(resolve(process.cwd(), 'src/main.tsx'), 'utf8');

  it('<App /> 恰出现一次（gate 外无游离渲染点）', () => {
    const occurrences = mainSource.match(/<App\s*\/>/g) ?? [];
    expect(occurrences, 'App 渲染点必须唯一（多渲染点 = 存在绕过 gate 的入口）')
      .toHaveLength(1);
  });

  it('<App /> 位于 <AuthGate>…</AuthGate> 子树内', () => {
    const gateOpen = mainSource.indexOf('<AuthGate');
    const gateClose = mainSource.indexOf('</AuthGate>');
    const appIdx = mainSource.indexOf('<App');

    expect(gateOpen, 'main.tsx 应挂 <AuthGate>').toBeGreaterThanOrEqual(0);
    expect(gateClose, 'main.tsx 的 AuthGate 应有闭合标签').toBeGreaterThan(gateOpen);
    // 契约核心：App 的挂载位置在 gate 内——挪出即红（未认证 ⇒ 零订阅
    // 的硬约束依赖此结构，浏览器 App 挂载即 gate 已判 ready）
    expect(appIdx).toBeGreaterThan(gateOpen);
    expect(appIdx).toBeLessThan(gateClose);
  });

  it('settings-demo 演示分支宿主门控：web 不渲染（isTauriHost 三元）', () => {
    // demo 留在 gate 外（独立演示页无认证语义），但仅桌面宿主渲染——
    // web 未认证访问 /settings-demo（SPA 回退供页）会满屏 401 错误态
    const demoGated = /\{showSettingsDemo\s*\?\s*\(isTauriHost\(\)\s*\?\s*<ButlerSettingsGroupedDemo\s*\/>\s*:\s*null\)\s*:/.test(
      mainSource
    );
    expect(demoGated, 'demo 分支应是 showSettingsDemo ? (isTauriHost() ? <Demo/> : null) : …').toBe(
      true
    );
  });
});
