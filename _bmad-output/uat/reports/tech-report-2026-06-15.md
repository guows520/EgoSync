# UAT 技术诊断报告 — 2026-06-15

**验收范围:** 全部 25 story / 140 用例　**执行模式:** dev
**关联业务报告:** _bmad-output/uat/reports/business-report-2026-06-15.md（按用例编号关联）

> 本报告供 `bmad-investigate` 立案与 `bmad-quick-dev` / `bmad-dev-story` 修复使用。
> UAT 为黑盒验证：提供业务证据与接口锚点，不做代码 path:line 归因（由 bmad-investigate 完成）。

---

## ✅ 修复闭环状态（2026-06-15 更新）

本报告原记录的 2 个 Fail 已经 `bmad-investigate` 立案 → `bmad-quick-dev` 修复 → 复测通过，**当前 Fail 数 = 0**。

| 用例 | 原结论 | 现结论 | 修复摘要 | 复测证据 |
|---|---|---|---|---|
| UAT-UI-004 | Fail | ✅ Pass | `tsconfig.json` 增加 `exclude: ["src/**/*.test.tsx","src/**/*.test.ts"]`（方案 A），测试文件移出生产 tsc | `npm run build` exit 0，tsc 零错误，产物生成 |
| UAT-UI-005 | Fail | ✅ Pass | App.tsx 抽取 5 个 hook（useThemeToggle/useSourceNavigation/useRoleLifecycle/useButlerProposal/useModalStack）+ 2 个展示组件（AppMainContent/AppModalStack），281→99 行 | App.tsx 99 行 ≤100，Vitest 164 通过，build exit 0 |

> 立案与根因详情见 `_bmad-output/implementation-artifacts/investigations/uat-ui-004-005-build-gate-investigation.md`。
> 以下五要素 DNA 为原始失败记录，保留供追溯。

---

## 失败用例（每条含五要素证据 DNA）— 已修复，保留追溯

### UAT-UI-004 — 拆分后类型检查、测试与构建全部通过

1. **是什么失败**
   - 用例编号: UAT-UI-004
   - story_key: 1-3-component-domain-split-visual-zero-regression
   - 业务场景: 重构后应类型检查零错误、测试通过、生产构建成功

2. **怎么复现**
   - 测试步骤: 进入 egosync-app/ → 运行 `npm run build`（= `tsc && vite build`）
   - 数据快照ID / 专属实体ID: 无（read-only，源码仓库 main 分支）
   - 执行模式: auto　破坏性: read-only

3. **期望 vs 实际**
   - 期望: tsc 类型检查零错误，build 退出码 0 并产出 dist 产物
   - 实际: `npm run build` 退出码 **2**，阻断在 tsc 阶段；`vite build` 单独执行退出码 0（产物可生成）
   - 精确 diff: 期望 exit=0 / 实际 exit=2；失败点为 tsc 而非 vite

4. **第一现场证据（stronghold 候选）**
   - 报错输出:
     ```
     src/components/settings/GlobalSettingsModal.test.tsx(1,30): error TS2307: Cannot find module 'node:fs' or its corresponding type declarations.
     src/components/settings/GlobalSettingsModal.test.tsx(2,25): error TS2307: Cannot find module 'node:path' or its corresponding type declarations.
     src/components/settings/GlobalSettingsModal.test.tsx(275,38): error TS2591: Cannot find name 'process'. Do you need to install type definitions for node?
     ```
   - 关键线索: `egosync-app/tsconfig.json` 的 `include: ['src']` 纳入测试文件，但 `compilerOptions.types` 仅含 `['vitest/globals','@testing-library/jest-dom']`，未含 `node`，导致测试文件中的 `node:fs/node:path/process` 类型缺失
   - 日志时间戳: 2026-06-15 本轮执行

5. **接口锚点**
   - 构建命令: `npm run build` → `tsc && vite build`
   - 涉及文件: egosync-app/tsconfig.json、egosync-app/src/components/settings/GlobalSettingsModal.test.tsx

---

### UAT-UI-005 — 组件按域分布且入口文件精简

1. **是什么失败**
   - 用例编号: UAT-UI-005
   - story_key: 1-3-component-domain-split-visual-zero-regression
   - 业务场景: 组件按域分目录，App.tsx 仅承担路由/状态且 ≤100 行

2. **怎么复现**
   - 测试步骤: 检查 src/components/ 域目录数 + lib/utils.ts + constants/mockData.ts TODO + 统计 src/App.tsx 行数
   - 数据快照ID / 专属实体ID: 无（read-only）
   - 执行模式: auto　破坏性: read-only

3. **期望 vs 实际**
   - 期望: App.tsx ≤ 100 行
   - 实际: App.tsx **281 行**
   - 精确 diff: 行数 281 vs 上限 100，超出 181 行；其余三项（7 域目录✅ / lib/utils.ts cn()✅ / mockData.ts 含 TODO✅）均达标

4. **第一现场证据（stronghold 候选）**
   - 实测: `(Get-Content src/App.tsx).Count` = 281
   - 域目录: butler/chat/layout/modals/notifications/onboarding/role/settings（8 个，含 layout，满足'7 个域目录'要求）
   - 日志时间戳: 2026-06-15 本轮执行

5. **接口锚点**
   - 涉及文件: egosync-app/src/App.tsx

---

## 阻塞用例（Blocked）

| 用例编号 | story_key | 阻塞原因 | 阻塞证据 |
|---|---|---|---|
| UAT-APP-002 | 1-1-tauri-desktop-app-existing-ui | 缺未装过 EgoSync 的干净 Windows 测试机/全新 VM 快照 | 数据需求表#1，无法验证安装包独立双击启动 |
| UAT-APP-003 | 1-1-tauri-desktop-app-existing-ui | 缺中端 Win / M1 Mac 基准硬件 | 数据需求表#2，云虚机不能代表真实冷启动性能 |
| UAT-QA-004 | 1-2-rust-frontend-test-infrastructure | 缺可访问 GitHub 仓库 + 启用 Actions（含额度） | 数据需求表#5，无法触发三平台 matrix CI |
| UAT-THEME-005 | 1-4-theme-toggle-design-tokens | 缺 axe DevTools 等对比度检测工具 | 数据需求表#7，无法客观测量 WCAG 对比度 |
| UAT-THEME-006 | 1-4-theme-toggle-design-tokens | 缺 Story1.3 完成态（token 化前）浅色基线截图 | 数据需求表#4，无视觉零回归对比基准 |
| UAT-UI-001 | 1-3-component-domain-split-visual-zero-regression | 缺拆分前 6 张基线截图 | 数据需求表#3，无像素级对比基准 |

## 待人工验收（129，非失败非阻塞）

本轮 dev 模式 Stage B 人工验收尚未执行，129 个用例处于 Pending 状态，逐条清单见 `manual-acceptance-queue.md`。其中 33 个原标记 auto 因项目无 E2E 脚本降级人工（建议后续补 Playwright E2E 以恢复自动化判定）。

## 交接建议

- 建议对 **UAT-UI-004 / UAT-UI-005** 两个 Fail 项运行 `bmad-investigate`，以'接口锚点 + 第一现场证据'为 stronghold 立案。
  - UAT-UI-004 修复方向参考：将测试文件排除出生产 tsc（如 tsconfig `exclude` 测试文件或拆分 tsconfig），或在 `types` 增加 `node`。
  - UAT-UI-005 修复方向参考：将 App.tsx 中的视图/逻辑进一步拆分到域组件，使入口降到 ≤100 行。
- 根因确认后由 `bmad-quick-dev` 或 `bmad-dev-story` 修复，修复后重跑本范围（至少 UAT-UI-004/005）。
- 6 个 Blocked 项待 boss 按数据需求表补齐环境/物料后解阻并补验。
