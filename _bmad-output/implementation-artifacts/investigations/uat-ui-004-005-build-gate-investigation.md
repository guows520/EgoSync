# Investigation: UAT-UI-004 / UAT-UI-005 构建门禁与入口精简双 Fail

## Hand-off Brief

1. **What happened.** `npm run build` 退出码 2（tsc 阶段中断），根因是 `tsconfig.json` 把唯一使用 node API 的测试文件 `GlobalSettingsModal.test.tsx` 纳入生产类型检查却未提供 `node` 类型（Confirmed）；同时 `src/App.tsx` 实测 280 行，超过 UAT-UI-005 的 ≤100 行上限（Confirmed）。
2. **Where the case stands.** 两个 Fail 的根因均已 Confirmed，证据齐全，无剩余调查缺口；属可直接修复状态。
3. **What's needed next.** 由 boss 确认修复方案后交 `bmad-quick-dev` 执行——UI-004 为配置层一行修复，UI-005 为入口组件机械拆分。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | UAT-UI-004 / UAT-UI-005（来自 UAT dev 模式 Stage A）                        |
| Date opened      | 2026-06-15                                                                  |
| Status           | Active                                                                      |
| System           | Windows / Node 25.4.0 / Vite 5 + Vitest 4 + TypeScript 5.2                  |
| Evidence sources | tech-report-2026-06-15.md、egosync-app/tsconfig.json、egosync-app/tsconfig.node.json、egosync-app/src/components/settings/GlobalSettingsModal.test.tsx、egosync-app/src/App.tsx |

## Problem Statement

UAT dev 模式 Stage A 自动判定出两个 Fail：
- **UAT-UI-004**：`npm run build`（= `tsc && vite build`）退出码 2，阻断在 tsc；`vite build` 单独退出码 0。
- **UAT-UI-005**：`src/App.tsx` 行数超过用例要求的 ≤100 行上限。

用户初始假设：UI-004 因 tsconfig `include:["src"]` 纳入测试文件、`types` 未含 `node`；UI-005 单纯行数超限。两条假设均需独立核实。

## Evidence Inventory

| Source   | Status    | Notes     |
| -------- | --------- | --------- |
| egosync-app/tsconfig.json | Available | `:22` types=["vitest/globals","@testing-library/jest-dom"]（无 node）；`:24` include=["src"] |
| egosync-app/tsconfig.node.json | Available | 仅 include=["vite.config.ts"]，compilerOptions 也无 node 类型 |
| GlobalSettingsModal.test.tsx | Available | `:1` import node:fs；`:2` import node:path；`:275` process.cwd() |
| 全量测试文件 node API 使用情况 | Available | 全项目仅 1 个测试文件使用 node 内置模块（即该文件） |
| egosync-app/src/App.tsx | Available | 实测 280 行（`wc -l` 计 280；含末行共 281 显示行） |
| npm run build 退出码 | Available | exit 2（本轮实跑） |
| vite build 单独退出码 | Available | exit 0，产出 dist 产物（本轮实跑） |

## Confirmed Findings

### Finding 1: tsconfig 生产编译纳入测试文件但缺 node 类型

**Evidence:** `egosync-app/tsconfig.json:24`（`include: ["src"]`）+ `egosync-app/tsconfig.json:22`（`types` 不含 `node`）+ `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx:1-2,275`

**Detail:** `tsc` 默认编译 `include` 命中的全部 `src/**`，包含 co-located 的 `*.test.tsx`。该测试文件在 `:1-2` 导入 `node:fs`/`node:path`、`:275` 用 `process.cwd()` 读取 `src/index.css` 以验证 loading 动画 keyframes（合法测试逻辑）。但 `compilerOptions.types` 白名单只放行 `vitest/globals` 与 `@testing-library/jest-dom`，等于关闭了 `@types/node` 的自动加载（`@types/node` 已在 devDependencies，但被 `types` 白名单挡住），导致 TS2307（node:fs/node:path 模块找不到）与 TS2591（process 未定义）。`tsconfig.node.json` 只管 `vite.config.ts`，不覆盖测试文件，无法兜底。

### Finding 2: vite build 与 tsc 解耦，产物生成不受影响

**Evidence:** 本轮实跑：`npx vite build` exit 0（1557 modules，产出 dist/index.html + assets）；`npm run build` exit 2

**Detail:** `build` 脚本是 `tsc && vite build`，`&&` 短路使 tsc 失败即中止、vite 不执行。vite 用 esbuild/oxc 转译不做类型检查，所以单独跑能成功。证明产物本身可构建，门禁失败纯粹来自类型检查阶段——这定位了失败边界，排除了"代码无法编译成产物"的可能。

### Finding 3: App.tsx 行数超限，但职责已属入口编排

**Evidence:** `egosync-app/src/App.tsx`（实测 280 行）+ UAT-UI-005 用例要求 ≤100 行

**Detail:** App.tsx 当前承担：11 个子组件 import、13 个 useState、3 个 refresh useCallback、role:proposed 事件监听 + 涌现角色提议确认/取消逻辑（:44-114，约 70 行）、角色归档/恢复/删除/更新 handler（:136-155）、来源导航 handler（:157-175）、主题切换（:177-184）、mainTint 色温计算（:189-199）、80 行 JSX 渲染树。职责仍属"路由/状态编排"，未混入业务逻辑，但量级超出精简入口标准。超限 180 行。其余三项（7 域目录 / lib/utils.ts cn() / constants/mockData.ts TODO）均达标，故 Fail 仅由行数单点触发。

## Deduced Conclusions

### Deduction 1: UI-004 是配置缺陷而非代码缺陷

**Based on:** Finding 1 + Finding 2

**Reasoning:** 产物可正常构建（F2），失败仅源于测试文件被纳入生产类型检查且缺 node 类型（F1）。测试文件的 node API 用法本身合法。问题出在 tsconfig 的编译范围/类型白名单配置，而非任何业务源码或测试逻辑错误。

**Conclusion:** 修复应落在 tsconfig 配置层，不应改动测试文件的合法逻辑，也不触碰业务代码。

### Deduction 2: UI-005 是结构度量缺陷，可机械拆分消除

**Based on:** Finding 3

**Reasoning:** App.tsx 内容已是合规的入口编排职责，超限来自多个可独立抽取的关注点（涌现提议逻辑、角色生命周期 handler、导航 handler、主题、色温）。这些都能抽成自定义 hook 或子模块而不改变行为。

**Conclusion:** 通过抽取 hook（如 useButlerProposal / useRoleLifecycle / useSourceNavigation / useThemeToggle）可把入口降到 ≤100 行，属低风险机械重构。

## Conclusion

**Confidence:** High（两个根因均 Confirmed，确定性复现：`npm run build` 必现 exit 2；`wc -l src/App.tsx` 必现 280）

- **UAT-UI-004 根因**：`tsconfig.json` 用 `include:["src"]` 将 co-located 测试文件纳入生产 `tsc`，同时 `types` 白名单未含 `node`，使唯一用 node API 的 `GlobalSettingsModal.test.tsx` 报 TS2307/TS2591。配置缺陷，非代码缺陷。
- **UAT-UI-005 根因**：`src/App.tsx` 280 行 > 100 行上限，关注点集中于单文件。结构度量缺陷，可机械拆分。

## Recommended Next Steps

### Fix direction

**UAT-UI-004（二选一，建议方案 A）：**
- **方案 A（推荐，最贴合用例语义）**：在 `tsconfig.json` 增加 `"exclude": ["src/**/*.test.tsx", "src/**/*.test.ts"]`，把测试文件移出生产类型检查。理由：用例 UI-004 验证的是"生产构建门禁"，测试文件不应进入生产 `tsc`；Vitest 自身用独立类型上下文（vitest/globals），不依赖生产 tsc 的类型检查。副作用最小，不动任何源码/测试逻辑，不放宽类型安全。
- **方案 B（备选）**：在 `compilerOptions.types` 加 `"node"`。理由：放行 @types/node 即可消除报错。但副作用是测试文件仍在生产编译范围内，且全局引入 node 类型可能让本应只在 Node 环境用的 API 在前端代码里也不报错，弱化边界。

**UAT-UI-005：**
- 将 App.tsx 的可抽取关注点提取为自定义 hook：
  - `useButlerProposal`（:44-114 涌现提议状态 + 事件监听 + confirm/cancel）→ 约省 60 行
  - `useRoleLifecycle`（:136-155 archive/restore/delete/update）→ 约省 18 行
  - `useSourceNavigation`（:157-175）→ 约省 16 行
  - `useThemeToggle`（:34-38,177-184）→ 约省 10 行
  - `mainTint` 计算（:189-199）可并入 useRoleLifecycle 或独立 util → 约省 10 行
- 抽取后入口仅保留 import + 组合 hook + JSX 渲染树，预计 ≤100 行。纯机械重构，行为不变，靠现有 Vitest（App.test.tsx 在列）回归保护。

### Diagnostic

无剩余诊断需求——两个根因已确定性复现，证据闭环。修复后验证：
- UI-004：`cd GUI && npm run build` 退出码应为 0 并产出 dist。
- UI-005：`(Get-Content src/App.tsx).Count` 应 ≤ 100，且 `npx vitest run` 全绿（确认拆分未回归）。

## Reproduction Plan

- **UI-004**：`cd GUI && npm run build` → 观察 exit 2，tsc 报 TS2307×2 + TS2591×1（指向 GlobalSettingsModal.test.tsx）。对照：`npx vite build` → exit 0。
- **UI-005**：`(Get-Content egosync-app/src/App.tsx).Count` → 280。

## Side Findings

- `GlobalSettingsModal.test.tsx:275` 用 `process.cwd()` 读 `src/index.css` 来断言 loading 动画 keyframes，属"用测试守护 CSS 资产"的有效用法；方案 A 修复后该测试仍由 Vitest 正常运行（Vitest 运行时有 node 环境），不受影响。
- `@types/node` 已在 `package.json` devDependencies（`^25.9.1`），证明依赖侧无缺失，问题纯在 tsconfig 的 `types` 白名单收口。

## Status

Resolved（根因 Confirmed → 修复已实施并复测通过，见 Follow-up: 2026-06-15）

## Follow-up: 2026-06-15

### 修复实施（boss 确认方案：UI-004 方案 A + UI-005 选项 1）

**UAT-UI-004 — tsconfig 排除测试文件（方案 A）**
- `egosync-app/tsconfig.json` 增加 `"exclude": ["src/**/*.test.tsx", "src/**/*.test.ts"]`
- 不动测试文件合法逻辑、不动业务源码、不放宽类型安全
- Vitest 用独立类型上下文运行，`GlobalSettingsModal.test.tsx:275` 的 `process.cwd()` 读 CSS 断言不受影响

**UAT-UI-005 — 抽取 5 个 hook + 2 个展示组件（选项 1，boss 追加批准第 5 个 hook）**
- 新建 hooks：`useThemeToggle` / `useSourceNavigation`（含 clearRole/clearButler）/ `useRoleLifecycle`（内置 onboarding 检测 + handleOnboardingComplete）/ `useButlerProposal` / `useModalStack`（6 浮层开关 + roleInitialTab）
- 新建展示组件：`components/layout/AppMainContent.tsx`（主区视图切换）/ `components/layout/AppModalStack.tsx`（6 modal 挂载）
- App.tsx 改为命名空间聚合调用（`m.` / `role.` / `nav.` / `proposal.`），消除解构行与内联 setter 闭包；mainTint useMemo 无损压缩
- 纯机械重构，行为不变

### 复测结果（实跑铁证）

| 用例 | 预期项 | 实测 | 判定 |
|---|---|---|---|
| UAT-UI-004 | tsc 零错误 / 测试全过 / 构建成功 | `npm run build` exit 0、164 测试通过、dist 产物生成 | ✅ Pass |
| UAT-UI-005 | ≥7 域目录 / cn()+TODO / App.tsx ≤100 | 8 域目录、cn()✅、mockData TODO✅、**App.tsx 99 行** | ✅ Pass |

- 全量前端测试：**164 通过 / 15 文件**，零回归（`App.test.tsx` 4 个用例含涌现提议 + 来源导航双向切换，均绿）
- 中途偏差修正：泛型标注 `useMemo<React.CSSProperties>` 不兼容 CSS 变量键（TS2345），已改回 `as React.CSSProperties` 断言

### Updated Conclusion

两个 Fail 均已修复并复测 Pass，置信度 High（确定性复现：`npm run build` exit 0、`App.tsx` 99 行）。无剩余缺口，案件闭环。
