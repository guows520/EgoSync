---
baseline_commit: e90c910
---

# Story 8.3: WCAG 2.1 AA 无障碍审计与修复

Status: done

## Story

As a 用户,
I want 应用符合无障碍标准,
so that 视觉障碍或行动不便的用户也能使用。

## 背景与现状（务必先读）

本 story 是 Epic 8（跨平台分发与 V1 加固）的第三个 story。Story 8.2 已建立 WebdriverIO + tauri-driver E2E 基础，本 story 在该基础上加入自动化无障碍审计，并修复当前前端中会阻断 WCAG 2.1 AA 的问题。

核心交付：
1. 在现有 E2E 框架中加入 axe 运行态扫描，覆盖 Tauri WebView 中可交互界面。
2. 加入 pa11y 扫描脚本，覆盖 Vite/HTML 可访问性基线。
3. 修复 0 个 critical/serious axe 问题所需的 UI 问题。
4. 补齐键盘导航、ARIA、焦点环、非颜色状态编码、reduced-motion 行为。
5. 将无障碍检查接入 CI，失败阻断发布。

### 已建成的基础（直接使用）

- `egosync-app/tests/e2e/` 已存在独立 E2E package，使用 WebdriverIO 9.x 与 Tauri WebDriver。
- `egosync-app/tests/e2e/wdio.conf.ts` 已配置 Windows/Linux tauri-driver、截图、日志、DB 清理。
- `.github/workflows/ci.yml` 已在 Windows/Linux 跑 E2E，macOS 跳过 Tauri WebDriver。
- `egosync-app/src/index.css` 已定义设计 token 与 reduced-motion media query，但 reduced-motion 分支仍保留 `breathe`、`animate-bounce-forever`、`animate-loading-spin` 动画，需修正。
- `egosync-app/src/components/layout/Sidebar.tsx` 已有 `role="navigation"` 与角色列表上下键导航，但管家/主题/设置按钮缺少显式 `aria-label`，确认弹窗缺少 `role="dialog"` 与 Escape 关闭。
- `egosync-app/src/components/layout/RoleSidebarIcon.tsx` 已有 `aria-label` 与焦点环，但能量状态小点仅靠颜色，需增加形状/文本辅助。
- `egosync-app/src/components/layout/Modal.tsx` 只有视觉遮罩，无 `role="dialog"`、`aria-modal`、Escape 关闭、焦点初始落点或背景说明。
- `egosync-app/src/components/chat/ChatInput.tsx` 已支持 Enter 发送、停止/发送按钮 `aria-label`，但 input 缺少清晰 `aria-label`。
- `egosync-app/src/components/butler/DashboardTab.tsx` 角色状态卡片主要靠颜色表达能量，需增加可读状态文本或图标（仅加文字/图标辅助，不改色值）。
- `egosync-app/src/components/modals/TaskModal.tsx` 四象限选项已有 `Q1/Q2/Q3/Q4` 前缀，保留该模式。

## Acceptance Criteria

1. Given 自动扫描，Then axe/pa11y 扫描无 critical/serious 问题（对比度规则 `color-contrast` 暂时禁用，推迟到后续版本），And 扫描报告可追溯。

2. Given 色盲友好，Then 所有状态指示不仅靠颜色区分，And 能量值同时具备文字/图标/形状辅助，And 四象限标签保留 Q1-Q4 文本前缀。

3. Given 键盘导航，Then Tab 键序覆盖 Sidebar → 管家对话 → 工作面板 → Modal，And Enter/Space 激活按钮，And Escape 关闭 Modal/确认弹窗，And 所有可交互元素有可见 `:focus-visible` 焦点环。

4. Given 屏幕阅读器，Then 所有交互元素有可理解的 `aria-label` 或可见文本名称，And 流式输出区域使用 `role="log" aria-live="polite"`，And Modal 使用 `role="dialog" aria-modal="true"` 与可访问标题。

5. Given 动效偏好，Then `prefers-reduced-motion: reduce` 时关闭呼吸、滑入、淡出、bounce、spin 等非必要动效和过渡，And 功能不受影响。

6. Given 审计工具，Then 使用 `@axe-core/webdriverio` 在 E2E 中扫描 Tauri WebView，And 使用 `pa11y` 扫描 Vite/HTML 基线，And CI 中无障碍检查失败时阻断发布并上传报告。

## Tasks / Subtasks

- [ ] Task 1: 建立无障碍自动扫描基础 (AC: #1, #6)
  - [ ] 1.1 在 `egosync-app/tests/e2e/package.json` 添加 `@axe-core/webdriverio`、`axe-core`、`pa11y` devDependencies。
  - [ ] 1.2 新增 E2E helper：封装 axe 扫描函数，接收页面名称，输出 violations 到 `tests/e2e/reports/accessibility/`。扫描时通过 `AxeBuilder.disableRules(['color-contrast'])` 禁用对比度规则（对比度修复推迟到后续版本，避免影响已确认的 UI 色值）。
  - [ ] 1.3 新增 `accessibility.spec.ts`，复用现有 `seedCompleteOnboarding()`、`seedRole()`、`seedTask()` 进入关键界面后运行 axe。
  - [ ] 1.4 新增 pa11y 脚本或 Node runner：启动 Vite/preview 后扫描至少首页 HTML 基线，标准使用 WCAG2AA，runner 使用 axe/htmlcs 组合。
  - [ ] 1.5 报告必须包含页面名、规则 id、impact、selector、失败 HTML 摘要；critical/serious 直接 fail。

- [ ] Task 2: 修复全局焦点与 reduced-motion (AC: #3, #5)
  - [ ] 2.1 修改 `egosync-app/src/index.css`：不要全局移除 `*:focus` 可见焦点；改为仅使用 `:focus:not(:focus-visible)` 隐藏鼠标焦点。
  - [ ] 2.2 增加全局 `:focus-visible` 2px offset 焦点环，符合 UX 规范。
  - [ ] 2.3 修正 `prefers-reduced-motion: reduce`：关闭 `breathe`、`animate-bounce-forever`、`animate-loading-spin` 等动画，不允许保留无限循环。
  - [ ] 2.4 确认动画关闭后按钮、输入、列表、Modal 功能不受影响。

- [ ] Task 3: 修复 Modal 与键盘关闭行为 (AC: #3, #4)
  - [ ] 3.1 更新 `egosync-app/src/components/layout/Modal.tsx`，支持 `ariaLabel` 或 `ariaLabelledBy`、`role="dialog"`、`aria-modal="true"`。
  - [ ] 3.2 `Modal` 监听 Escape 调用 `onClose`，并保持点击遮罩关闭现有行为。
  - [ ] 3.3 更新所有 `Modal` 调用点，为每个弹窗提供可访问标题或标签；不得留下无名称 dialog。
  - [ ] 3.4 修复 `Sidebar.tsx` 内联确认弹窗：添加 dialog 语义、Escape 关闭、输入框 label/aria-label。

- [ ] Task 4: 修复屏幕阅读器名称与状态语义 (AC: #2, #4)
  - [ ] 4.1 给 Sidebar 管家、添加角色、主题切换、设置按钮补齐显式 `aria-label`，不要只依赖 `title`。
  - [ ] 4.2 给 ChatInput 输入框添加与当前上下文一致的 `aria-label`。
  - [ ] 4.3 给 ChatStream 消息列表/流式输出容器添加 `role="log" aria-live="polite"`，避免把每个静态气泡都误标为 live region。
  - [ ] 4.4 对通知红点/绿点提供屏幕阅读器文本，不只依赖颜色。

- [ ] Task 5: 修复色盲友好状态表达 (AC: #2)
  - [ ] 5.1 `RoleSidebarIcon.tsx` 的能量状态小点增加形状或隐藏文本辅助，例如高/中/低能量分别具备不同 `data-energy-state`、图标或 `sr-only` 文案。
  - [ ] 5.2 `DashboardTab.tsx` 的能量百分比旁增加文字状态（高能量/中能量/低能量）或图标，不能只靠绿色/琥珀/红色。
  - [ ] 5.3 保持 `TaskModal.tsx` 四象限 `Q1/Q2/Q3/Q4` 文本前缀，不要改回纯颜色标签。
  - [ ] 5.4 避免将低能量表达设计成强负面红色唯一提示；优先文字+图标+中性色辅助。

- [ ] Task 6: CI 集成与验证 (AC: #1, #6)
  - [ ] 6.1 在 `.github/workflows/ci.yml` 的 E2E 依赖安装后运行无障碍 E2E spec（Windows/Linux）。
  - [ ] 6.2 在 CI 中运行 pa11y 基线扫描；若 Tauri WebDriver 不支持 macOS，pa11y 可作为 macOS 的 HTML 层替代检查。
  - [ ] 6.3 失败时上传 `egosync-app/tests/e2e/reports/accessibility/`。
  - [ ] 6.4 本地至少运行 `npm run test:frontend`；如环境具备 tauri-driver，再运行 `cd egosync-app/tests/e2e && npm test`。

## Dev Notes

### 技术决策

axe 与 pa11y 分工：
- `@axe-core/webdriverio` 用于 Tauri WebView 运行态扫描，直接复用 Story 8.2 的 WebdriverIO session。
- `pa11y` 用于 Vite/HTML 层基线扫描；它基于 Puppeteer/浏览器 URL，不应被强行用于已启动的 Tauri WebView。
- Story 8.3 AC 同时要求 axe-core + pa11y，因此两者都要进入自动化流程，但失败门槛不同：axe 负责真实应用状态，pa11y 负责静态/浏览器基线。

CI 平台约束：
- Tauri WebDriver 仍只在 Windows/Linux 跑；macOS 无 WKWebView driver。
- pa11y 可在普通浏览器页面运行，因此可考虑在所有平台或至少 Linux 执行。
- 不要修改 Story 8.2 已建立的 DB 清理、日志、截图机制；只扩展 reports 上传。

依赖约束：
- 使用现有 E2E 独立 package，不要把 axe/pa11y 依赖放入根 `egosync-app/package.json`。
- Web 搜索确认：`@axe-core/webdriverio` 当前 4.11.x，`axe-core` 当前 4.11.x，`pa11y` 当前 9.1.x。安装时使用兼容主版本即可，遵循 lockfile 固化。

### 当前必须关注的实现文件

更新文件：
- `egosync-app/tests/e2e/package.json` — 添加无障碍测试依赖与脚本。
- `egosync-app/tests/e2e/wdio.conf.ts` — 保持现有 driver/session 逻辑，可新增 report 目录清理与失败报告上传路径配合。
- `egosync-app/tests/e2e/helpers/app-helper.ts` — 可复用 seed/navigate helpers；新增 a11y helper 可放到单独文件，避免污染通用 helper。
- `egosync-app/tests/e2e/specs/accessibility.spec.ts` — 新增 axe 扫描 spec。
- `egosync-app/src/index.css` — 修复全局 focus 与 reduced-motion。
- `egosync-app/src/components/layout/Modal.tsx` — 增加 dialog 语义与 Escape。
- `egosync-app/src/components/layout/Sidebar.tsx` — 补 aria-label、确认弹窗语义、通知点文本。
- `egosync-app/src/components/layout/RoleSidebarIcon.tsx` — 能量状态非颜色编码。
- `egosync-app/src/components/chat/ChatInput.tsx` — 输入框 label。
- `egosync-app/src/components/chat/ChatStream.tsx` — 流式输出 live region。
- `egosync-app/src/components/butler/DashboardTab.tsx` — 能量状态文字/图标辅助。
- `.github/workflows/ci.yml` — 上传 a11y reports，并在合适位置运行扫描。

### 必须保留的既有行为

- 保留 Story 8.2 的 Windows/Linux E2E 运行模式，macOS 跳过 Tauri WebDriver。
- 保留 `com.egosync.desktop` app data 路径，不要回退到旧的 `com.egosync.app`。
- 保留 Sidebar 上下键切换角色的现有行为。
- 保留点击遮罩关闭 Modal 的现有体验。
- 保留 ChatInput Enter 发送、Shift+Enter 不发送的行为。
- 保留 TaskModal 的 `Q1/Q2/Q3/Q4` 文本前缀。

### 对比度推迟说明

对比度修复（`color-contrast` 规则）推迟到后续版本，原因：
- 当前 UI 色值已经过设计确认，修改 `text-slate-400`/`text-amber-600`/`text-red-500` 等约 20+ 处色值会带来可感知的视觉变化
- 涉及 `DashboardTab`、`Sidebar`、`GlobalSettingsModal` 等核心组件的辅助文字、功能色文字、非激活按钮
- 需要单独评估视觉影响后再统一调整

本 story 的 axe/pa11y 扫描通过 `disableRules(['color-contrast'])` 跳过对比度检查，其他无障碍规则照常执行。

### 反模式警告

- 不要为了通过 axe 扫描删除真实 UI 或隐藏问题节点。
- 不要全局 `outline: none !important`。
- 不要只给图标按钮加 `title`，必须有可访问名称。
- 不要把每个聊天气泡都设成 live region；流式输出容器应统一管理。
- 不要把 reduced-motion 写成“减慢动画”；AC 要求关闭非必要动效和过渡。
- 不要引入 Playwright/Cypress 重建测试框架；复用 WebdriverIO + tauri-driver。
- 不要把 pa11y 当作 Tauri WebView driver；它扫描 URL/HTML 基线。

### Previous Story Intelligence

Story 8.2 关键经验：
- E2E 已采用 IPC seeding，避免 `better-sqlite3` 原生依赖和 Windows C++ 工具链问题。
- DB 清理目录必须使用 `com.egosync.desktop`，否则测试隔离失效。
- 失败诊断已包含 screenshots/logs，本 story 只需新增 accessibility reports。
- LLM/仲裁/简报真实行为在 CI 中有能力边界；无障碍测试应优先扫描可稳定到达的界面状态。
- Windows `msedgedriver` 路径仍有待 CI 首跑验证，不要在本 story 中扩大该不确定性。

最近提交模式：
- `e90c910` 修复 E2E DB 目录、死代码、旅程验证收紧。
- `aa93922` 搭建 E2E 测试框架与侧边栏滚动修复。
- `77d329b` 同步 8-1 故事文件。
- `851011e` Windows sidecar 启动加 `CREATE_NO_WINDOW`。
- `825a7ac` bundle identifier 改为 `com.egosync.desktop`。

### Testing Requirements

- 前端单元测试：`cd egosync-app && npm run test:frontend`。
- E2E 测试：`cd egosync-app/tests/e2e && npm test`（需本机安装 tauri-driver，Windows 需 Edge Driver，Linux 需 WebKitWebDriver + xvfb）。
- 无障碍扫描：新增脚本后应能单独运行 axe/pa11y；失败报告落在 `egosync-app/tests/e2e/reports/accessibility/`。
- CI：Windows/Linux 的 E2E/a11y 失败必须阻断；macOS 不跑 Tauri WebDriver。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.3] — WCAG 2.1 AA AC 原文。
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8] — V1 加固上下文。
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Accessibility Strategy] — 对比度、键盘、屏幕阅读器、reduced-motion、测试策略。
- [Source: _bmad-output/planning-artifacts/epics.md#UX-DR22] — WCAG 2.1 AA、aria-label、live region、键盘导航要求。
- [Source: _bmad-output/project-context.md] — React/Tauri/测试/代码组织规则。
- [Source: _bmad-output/implementation-artifacts/8-2-e2e-test-core-journeys.md] — E2E 基础与偏离说明。
- [Source: egosync-app/tests/e2e/wdio.conf.ts] — 当前 WebdriverIO/Tauri Driver 配置。
- [Source: egosync-app/src/index.css] — 当前 focus 与 reduced-motion 实现。
- [Source: egosync-app/src/components/layout/Modal.tsx] — Modal 当前语义缺口。
- [Source: egosync-app/src/components/layout/Sidebar.tsx] — Sidebar 键盘导航与确认弹窗。
- [Source: egosync-app/src/components/layout/RoleSidebarIcon.tsx] — 角色图标与能量状态。
- [Source: egosync-app/src/components/chat/ChatStream.tsx] — 流式输出与聊天区域。
- [Source: egosync-app/src/components/chat/ChatInput.tsx] — 对话输入。
- [Source: egosync-app/src/components/butler/DashboardTab.tsx] — 仪表盘状态卡片。
- [Source: .github/workflows/ci.yml] — CI E2E 运行与 artifact 上传位置。
- [External: @axe-core/webdriverio npm] — WebdriverIO axe integration, AxeBuilder API。
- [External: pa11y npm] — WCAG2AA、runner axe/htmlcs、CI exit code 行为。

### Review Findings

- [x] [Review][Reverted] RoleSidebarIcon 能量状态形状区分已按用户要求还原，小点恢复为统一 rounded-full。
  - 位置：`egosync-app/src/components/layout/RoleSidebarIcon.tsx:23-27, 79-86`

- [x] [Review][Patch] 缺少 pa11y 扫描脚本文件（critical）`egosync-app/tests/e2e/package.json` 引用了 `"pa11y": "node scripts/pa11y-scan.mjs"`，但 `egosync-app/tests/e2e/scripts/` 目录为空，CI 会立即失败。
  - 位置：`egosync-app/tests/e2e/package.json:8`

- [x] [Review][Patch] CI 中 pa11y 扫描使用硬编码 `sleep 3` 等待 preview 服务启动，不可靠且可能静默误报。
  - 位置：`.github/workflows/ci.yml:117-118`

- [x] [Review][Patch] CI 中 pa11y 扫描 URL 硬编码为 `localhost:4173`，若 Vite preview 端口变更会失败。
  - 位置：`egosync-app/tests/e2e/scripts/pa11y-scan.mjs:18`（脚本尚未创建）

- [x] [Review][Patch] Modal.tsx 的 Escape 监听依赖 `onClose` props；若父组件每次渲染创建新函数引用，事件监听器会被反复挂载/卸载。
  - 位置：`egosync-app/src/components/layout/Modal.tsx:12-20`

- [x] [Review][Patch] Modal.tsx 打开时未将焦点移入 dialog、未实现焦点陷阱、关闭时未返回触发元素；违反 AC #3 焦点初始落点。
  - 位置：`egosync-app/src/components/layout/Modal.tsx:11-35`

- [x] [Review][Patch] Modal.tsx 的 `ariaLabel` 为可选，可能产生无名称 dialog；规范 Task 3.3 要求“不得留下无名称 dialog”。
  - 位置：`egosync-app/src/components/layout/Modal.tsx:8, 28`

- [x] [Review][Patch] 多个组件同时监听 document 级别 Escape，多个 Modal 同时打开时会产生冲突。
  - 位置：`egosync-app/src/components/layout/Modal.tsx:18` / `egosync-app/src/components/layout/Sidebar.tsx:289-298`

- [x] [Review][Patch] ButlerSettingsContent.tsx 中两个内联 Modal（编辑使命宣言、推断的使命宣言）缺少 Escape 关闭逻辑。
  - 位置：`egosync-app/src/components/butler/ButlerSettingsContent.tsx:1029-1153`

- [x] [Review][Patch] GlobalSettingsModal.tsx 是内联面板但未使用 Modal 组件，缺少 Escape 关闭与焦点管理。
  - 位置：`egosync-app/src/components/settings/GlobalSettingsModal.tsx:458-459`

- [x] [Review][Reverted] DashboardTab.tsx 能量状态文字+图标辅助已按用户要求还原，仅保留颜色+百分比表达。
  - 位置：`egosync-app/src/components/butler/DashboardTab.tsx:66-80`

- [x] [Review][Patch] ChatInput.tsx 的 placeholder 与 aria-label 回退逻辑不一致，placeholder 可能为英文而 aria-label 固定为中文。
  - 位置：`egosync-app/src/components/chat/ChatInput.tsx:31-32`

- [x] [Review][Dismiss] index.css 使用全局 `*:focus-visible` 选择符 + `!important` 是 spec 2.2 要求的“全局焦点环”，且项目内所有组件均为受控代码；无需改动。
  - 位置：`egosync-app/src/index.css:70-73`

- [x] [Review][Patch] DashboardTab.tsx / RoleSidebarIcon.tsx 未对 `role.energy` 做边界验证；null/undefined/NaN/负值/超 100 会生成无效 CSS 或错误状态。
  - 位置：`egosync-app/src/components/butler/DashboardTab.tsx:66-78` / `egosync-app/src/components/layout/RoleSidebarIcon.tsx:17-27`

- [x] [Review][Dismiss] Sidebar.tsx 内联删除确认弹窗输入框已固定为“DELETE”文本，无额外校验必要。
  - 位置：`egosync-app/src/components/layout/Sidebar.tsx:186-192`

- [x] [Review][Dismiss] CI 中 accessibility reports 上传条件 `failure() && os != macos` 已能在 Windows/Linux 的 E2E 或 pa11y 失败时触发；macOS 不跑 E2E/a11y，无需上传。
  - 位置：`.github/workflows/ci.yml:136-141`

- [x] [Review][Dismiss] Sidebar.tsx 的 `z-[200]` 用于下拉/确认弹窗层级，Modal 为 `z-50` 全屏遮罩，两者不冲突；无需改动。
  - 位置：`egosync-app/src/components/layout/Sidebar.tsx:236`

- [x] [Review][Patch] Modal.tsx 的 `ModalProps` 接口未导出，影响复用和扩展。
  - 位置：`egosync-app/src/components/layout/Modal.tsx:4-9`

- [x] [Review][Defer] NotificationPanel 未在本次 diff 中修改，无法确认其 Escape 关闭与 aria-label 状态；属于 pre-existing 问题。
  - 位置：`egosync-app/src/App.tsx:411`

- [x] [Review][Defer] 其他未在本次 diff 中修改的 Modal 调用点需人工检查是否都传递了 `ariaLabel`；属于 pre-existing 问题。

- [x] [Review][Defer] RoleSidebarIcon 能量状态仅靠颜色区分（色盲友好辅助已还原）；AC #2 色盲友好未实现，作为已知限制保留，待 V2 视觉优化时统一处理。

- [x] [Review][Dismiss] pa11y 仅在 Linux 运行 — spec 允许“至少 Linux”。
- [x] [Review][Dismiss] pa11y 失败不会阻断发布 — GitHub Actions 步骤失败即 job 失败，会阻断。
- [x] [Review][Dismiss] ChatStream 使用 `role="log" aria-live="polite"` — spec 明确要求。
- [x] [Review][Dismiss] ChatStream aria-live 可能过度公告 — 与 spec 一致，使用 polite 而非 assertive 已是最保守做法。
- [x] [Review][Dismiss] “index.css 删除了 .breathe/.animate-bounce-forever/.animate-loading-spin 类” — 错误，类定义仍在 index.css:100-129；本次只是从 reduced-motion media query 中移除了 infinite 重写。
- [x] [Review][Dismiss] GlobalSettingsModal.test.tsx 移除 reduced-motion 断言 — 错误，断言已改为验证 `@keyframes loading-spin` 与 `.animate-loading-spin` 定义，与当前 CSS 一致。

## Dev Agent Record

### Agent Model Used

Kimi K2.7

### Debug Log References

- 评审 diff: `_bmad-output/implementation-artifacts/CR-8-3-uncommitted.diff`

### Completion Notes List

- 完成代码评审并处理 13 个 patch 项，3 个 defer，6 个 dismiss，1 个 decision-needed 已解决。
- 创建 pa11y 扫描脚本并接入 CI 健康检查等待。
- Modal 组件增加稳定 Escape 监听、焦点初始落点、返回触发元素、ariaLabel 必填。
- Sidebar ConfirmDialog 同样增加稳定监听与焦点管理。
- ButlerSettingsContent / GlobalSettingsModal 增加 Escape 关闭。
- DashboardTab 增加能量边界校验（clampEnergy）与 progressbar aria 属性；色盲友好文字+图标辅助已按用户决策还原。
- RoleSidebarIcon 增加能量边界校验（clampEnergy）；色盲友好形状区分已按用户决策还原。
- Sidebar 角色列表容器增加 pt-2 pb-2，修复选中角色 scale-105 与焦点环被 overflow-y-auto 裁切的问题。
- TaskOverviewTab 筛选第一行间距收紧（gap-1.5→gap-1、px-2.5→px-2、px-2→px-1.5），避免"只看大石头"在常见窗口宽度下溢出到第二行。
- ChatInput 统一 aria-label 回退逻辑。
- 更新 DashboardTab 测试断言改回纯百分比文本（色盲友好已还原）。
- 删除 RoleSidebarIcon.a11y.test.tsx（色盲形状测试已失效）。
- 验证：`npm run test:frontend` 通过，`npx tsc --noEmit` 通过。

### File List

- `egosync-app/tests/e2e/scripts/pa11y-scan.mjs`
- `egosync-app/tests/e2e/helpers/a11y-helper.ts`
- `egosync-app/tests/e2e/specs/accessibility.spec.ts`
- `egosync-app/tests/e2e/package.json`
- `egosync-app/tests/e2e/wdio.conf.ts`
- `.github/workflows/ci.yml`
- `egosync-app/src/components/layout/Modal.tsx`
- `egosync-app/src/components/layout/Modal.test.tsx`
- `egosync-app/src/components/layout/Sidebar.tsx`
- `egosync-app/src/components/layout/RoleSidebarIcon.tsx`
- `egosync-app/src/components/butler/ButlerSettingsContent.tsx`
- `egosync-app/src/components/butler/DashboardTab.tsx`
- `egosync-app/src/components/butler/DashboardTab.test.tsx`
- `egosync-app/src/components/butler/DashboardTab.a11y.test.tsx`
- `egosync-app/src/components/butler/TaskOverviewTab.tsx`
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx`
- `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx`
- `egosync-app/src/components/modals/AddRoleModal.tsx`
- `egosync-app/src/components/modals/TaskModal.tsx`
- `egosync-app/src/components/modals/WeeklyReviewModal.tsx`
- `egosync-app/src/components/role/TasksTab.tsx`
- `egosync-app/src/components/chat/ChatInput.tsx`
- `egosync-app/src/components/chat/ChatInput.a11y.test.tsx`
- `egosync-app/src/components/chat/ChatStream.tsx`
- `egosync-app/src/index.css`
- `egosync-app/src/index.css.test.ts`
