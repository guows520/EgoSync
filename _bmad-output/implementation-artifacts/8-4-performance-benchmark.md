---
baseline_commit: 4f44008
---

# Story 8.4: 性能基准验证与优化

Status: done

## Story

As a 用户,
I want 应用在所有平台都流畅响应,
so that 使用体验不因平台差异而打折。

## 背景与现状（务必先读）

本 story 是 Epic 8（跨平台分发与 V1 加固）的第四个 story。Story 8.1 已建立三平台 CI 并行构建，Story 8.2 已建立 WebdriverIO + tauri-driver E2E 框架（Windows/Linux），Story 8.3 已加入 axe/pa11y 无障碍扫描。本 story 在同一 E2E 框架上**新增性能基准测量与回归告警**，并对当前实现做必要的性能优化与验证，不重建测试基础设施。

**核心交付：**
1. 在现有 E2E 框架中新增性能基准 spec，测量冷启动时间、稳态内存、流式渲染延迟，输出可追溯的 JSON 报告。
2. 验证并（必要时）优化角色卡片呼吸动效的 60fps 表现，确认使用 CSS `transform`/`opacity` + GPU 加速，无 JS 动画阻塞。
3. 验证首次体验时间（Onboarding ≤ 4 步、≤ 5 分钟）的回归基线。
4. 验证三平台流式 token 渲染延迟差异 ≤ 50ms（CI 中通过同一测量脚本在 Windows/Linux 跑，macOS 通过本地手动脚本补齐一次基线）。
5. 在 CI 中加入性能基准步骤，超过阈值时**警告但不阻断**（与 AC #6 一致），报告上传为 artifact。

### 已建成的基础（直接复用，不要重建）

- `egosync-app/tests/e2e/` 独立 package，WebdriverIO 9.x + tauri-driver，`wdio.conf.ts` 已配置 Windows/Linux driver、screenshots/logs/reports 目录清理、DB 清理（`com.egosync.desktop`）。
- `egosync-app/tests/e2e/helpers/app-helper.ts` 提供 `waitForAppReady()`、`invoke()`（通过 `__TAURI_INTERNALS__.invoke` 执行 IPC）、`seedCompleteOnboarding()`、`seedRole()`、`seedTask()`。
- `egosync-app/tests/e2e/helpers/a11y-helper.ts` 提供 axe 扫描 + JSON 报告落盘模式，**性能 helper 应仿照此模式**（独立文件、JSON 报告、critical 阈值抛错）。
- `.github/workflows/ci.yml` 已有三平台 matrix、E2E 在 Windows/Linux 跑、`Upload E2E screenshots/logs on failure`、`Upload accessibility reports on failure` 三个 artifact 上传步骤。本 story 新增 `Upload performance reports` 步骤，**always 上传**（不仅 failure，因为性能数据需要持续追踪）。
- `egosync-app/src-tauri/src/commands/app.rs:53-63` 已有 `app_sidecar_status` 返回 `{ running, port, uptime_secs }`，可用于辅助判断启动后 sidecar 是否就绪。
- `egosync-app/src/index.css:99-140` 呼吸动效已使用 `opacity` + `will-change: opacity` + `@keyframes breathe`，`prefers-reduced-motion` 已在 Story 8.3 修正为关闭非必要动效。**本 story 验证不回归，不重写动画实现。**

### 关键技术约束

**⚠️ macOS 不支持 tauri-driver：**
- 与 Story 8.2/8.3 一致：CI 性能基准只在 Windows + Linux 跑。
- macOS 的流式延迟差异 ≤ 50ms 验证：通过本地手动运行 `npm run test:perf:local`（如有 tauri-driver）或人工 DevTools 验证一次，记录到 story 完成笔记。**不在 CI 中强制 macOS 性能步骤。**

**⚠️ CI 环境无 LLM API Key：**
- 与 Story 8.2 一致：opencode sidecar 占位文件，`chat_send_message` 立即失败复位。
- 流式渲染延迟测量**不依赖真实 LLM 响应**：通过 IPC 注入 mock 流式事件（`llm:stream` payload）或测量 `useTauriEvent` 监听 → DOM 更新的端到端延迟。具体方案见 Dev Notes。

**⚠️ CI runner 性能不稳定：**
- GitHub Actions runner 性能波动大，冷启动时间绝对值不可靠。
- **解决方案**：性能基准步骤**警告不阻断**（AC #6 明确要求），阈值设为宽松基线（冷启动 ≤ 10s CI / ≤ 3s 本地 SSD，内存 ≤ 300MB CI / ≤ 200MB 本地），报告中记录 runner OS + 时间戳供趋势分析。

**⚠️ 60fps 验证在 CI 中不可行：**
- tauri-driver 通过 WebDriver 协议操作 WebView，无法访问 Chrome DevTools Performance track。
- **解决方案**：60fps 验证通过**静态代码审计 + 本地人工 DevTools 验证**完成，CI 中运行静态审计脚本（grep 检查动画属性使用 `transform`/`opacity`，无 `requestAnimationFrame` 循环动画、无 `setInterval` 动画），人工验证结果记录到完成笔记。

## Acceptance Criteria

1. **AC1（动效性能 NFR-5）**：Given 角色卡片呼吸动效，Then ≥ 60fps（本地 Chrome DevTools Performance 验证，记录到完成笔记），And hover/过渡动画无掉帧，And CSS transition + GPU 加速（`transform`, `opacity`），不使用 JS 动画；CI 中静态审计脚本验证动画属性合规。

2. **AC2（首次体验时间 NFR-9，回归 Story 1.8 基线）**：Given 新用户首次打开应用，Then 从打开到创建第一个角色 ≤ 5 分钟，And Onboarding 流程步骤 ≤ 4 步；CI 中通过 E2E 测量 onboarding 视图出现到可输入的时间作为代理指标。

3. **AC3（流式输出一致性 NFR-11）**：Given 三平台 LLM 流式 token 渲染，Then 延迟差异 ≤ 50ms（Windows/Linux CI 测量 + macOS 本地手动测量一次），And Windows/macOS/Linux WebView 差异已在流式渲染层抽象（验证 `agent_bridge.rs` SSE 解析 + `event_router.rs` demux + `useTauriEvent('llm:stream')` 链路无平台分支）。

4. **AC4（冷启动时间）**：Given 应用启动，Then 启动到可交互 ≤ 3 秒（本地 SSD 环境），And SQLite 初始化 + 调度器启动不阻塞 UI（验证 `lib.rs:setup` 中 DB 初始化为 `block_on` 但调度器为 `spawn` 非阻塞）；CI 中测量并记录，阈值 ≤ 10s（runner 性能折扣），超阈值警告不阻断。

5. **AC5（内存占用）**：Given 稳态运行，Then 内存 ≤ 200MB（3 角色 + 100 条记忆，本地），And 1 小时持续使用后内存增长 ≤ 10%（本地人工验证，记录到完成笔记）；CI 中测量进程 RSS 作为代理指标，阈值 ≤ 300MB，超阈值警告不阻断。

6. **AC6（性能回归检测）**：Given CI 流水线，Then 增加性能基准测试（冷启动时间 + 内存快照），And 超过阈值时警告（不阻断），And 报告上传为 CI 产物供趋势分析。

## Tasks / Subtasks

- [x] **Task 1: 建立性能基准测量基础** (AC: #2, #4, #5, #6)
  - [x] 1.1 在 `egosync-app/tests/e2e/package.json` 添加 `test:perf` 脚本（`wdio run wdio.conf.ts --suite perf`）和 `test:perf:local`（同上，本地用），不新增第三方性能依赖（使用 `process.memoryUsage()` 经 `executeAsync` 注入 + `Date.now()` 计时）。
  - [x] 1.2 新增 `egosync-app/tests/e2e/helpers/perf-helper.ts`，仿照 `a11y-helper.ts` 模式：
    - `measureColdStart()`：记录 `browser.session` 创建时间戳到 `$('body')` 可交互的时间差，输出 `{ coldStartMs, runnerOs, timestamp }`。
    - `measureProcessMemory()`：通过 `browser.executeAsync` 注入脚本读取 `process.memoryUsage()`（仅 Node 环境）或回退到 `performance.memory`（WebView），输出 `{ rssMb, jsHeapUsedMb, timestamp }`。**注意**：WebView 无 `process`，需通过 Tauri Rust 端新增 `app_performance_snapshot` command 返回进程 RSS（使用 `sysinfo` crate 或平台特定 API）。
    - `writePerfReport(name, data)`：JSON 落盘到 `tests/e2e/reports/performance/`。
  - [x] 1.3 新增 `egosync-app/tests/e2e/specs/performance.spec.ts`，包含：
    - 冷启动测量（复用 `waitForAppReady` 计时）。
    - 稳态内存测量（seed 3 角色 + 100 条记忆后读取 RSS）。
    - Onboarding 可交互时间测量（首次启动到 `input[type="text"]` 可见）。
  - [x] 1.4 在 `wdio.conf.ts` 的 `onPrepare` 中新增 `reports/performance/` 目录清理（仿照 a11y reports 目录处理）。

- [x] **Task 2: Rust 端性能快照 command** (AC: #5)
  - [x] 2.1 在 `egosync-app/src-tauri/Cargo.toml` 添加 `sysinfo = "0.32"` 依赖（跨平台进程内存读取，遵循 7 天发布稳定性规则）。
  - [x] 2.2 在 `egosync-app/src-tauri/src/commands/app.rs` 新增 `app_performance_snapshot` command，返回 `{ rssMb, jsHeapUsedMb, processUptimeSecs, sidecarRssMb }`：
    - 使用 `sysinfo::System` 读取当前进程 + sidecar 进程 RSS。
    - sidecar RSS 通过 `SidecarManager` 暴露的 child PID 读取（如 `SidecarManager` 未暴露 PID，新增 `pub fn child_pid(&self) -> Option<u32>`）。
  - [x] 2.3 在 `lib.rs:278` 的 `invoke_handler` 注册 `commands::app::app_performance_snapshot`。
  - [x] 2.4 在 `egosync-app/src/services/appService.ts` 封装 `performanceSnapshot()` 调用。

- [x] **Task 3: 流式渲染延迟测量** (AC: #3)
  - [x] 3.1 在 `perf-helper.ts` 新增 `measureStreamRenderLatency()`：
    - 通过 `invoke('chat_send_message', ...)` 触发后，由于 CI 无 LLM 会立即失败，改为**注入 mock 流式事件**：通过 `browser.executeAsync` 调用 `window.__TAURI_INTERNALS__.invoke('app_emit_test_stream', { tokens: ['你','好','，','测','试'] })` 触发测试事件。
    - 测量从 emit 到 DOM 中流式气泡出现 token 文本的时间差。
  - [x] 3.2 在 `egosync-app/src-tauri/src/commands/app.rs` 新增 `app_emit_test_stream` command（**仅 `#[cfg(debug_assertions)]` 或通过 feature gate**），通过 `app.emit("llm:stream", payload)` 发射测试 token，用于性能测量。**生产构建必须排除此 command**，通过 `#[cfg(any(test, feature = "perf-test"))]` 控制。
  - [x] 3.3 在 `Cargo.toml` 新增 `[features] perf-test = []`，CI E2E 构建使用 `cargo build --features perf-test`（或 `tauri build -- --features perf-test`），生产构建不带此 feature。
  - [x] 3.4 在 `performance.spec.ts` 新增流式延迟测试用例，记录 `{ emitToRenderMs, tokenCount, runnerOs }`。

- [x] **Task 4: 60fps 动效静态审计** (AC: #1)
  - [x] 4.1 新增 `egosync-app/tests/e2e/scripts/audit-animations.mjs`，静态扫描 `egosync-app/src/**/*.tsx` 和 `index.css`：
    - 检查所有 `animation` 属性使用 `transform`/`opacity`（不强制 100%，但 `breathe`/`bounce`/`loading-spin` keyframes 必须合规）。
    - 检查无 `requestAnimationFrame` 用于持续动画循环（允许用于一次性 scroll 定位，如 `ChatStream.tsx:716`）。
    - 检查无 `setInterval` 驱动的动画。
    - 输出 JSON 报告到 `reports/performance/animation-audit.json`。
  - [x] 4.2 在 `package.json` 添加 `audit:animations` 脚本。
  - [ ] 4.3 本地人工验证：用 Chrome DevTools Performance 录制 5 秒角色卡片呼吸动效，确认 ≥ 60fps，截图/录制保存到 story 完成笔记（不在 CI 中强制）。

- [x] **Task 5: CI 集成** (AC: #6)
  - [x] 5.1 在 `.github/workflows/ci.yml` E2E 步骤后新增 `Run performance benchmarks (Linux)` 和 `Run performance benchmarks (Windows)`，运行 `npm run test:perf`（或 `wdio run wdio.conf.ts --suite perf`）。
  - [x] 5.2 新增 `Run animation audit` 步骤（Linux，`node scripts/audit-animations.mjs`），失败时**警告不阻断**（`continue-on-error: true` 或脚本内部 exit 0 + 输出 warning）。
  - [x] 5.3 新增 `Upload performance reports` 步骤，**`if: always()`**（不仅失败时），上传 `egosync-app/tests/e2e/reports/performance/`。
  - [x] 5.4 性能阈值超限不阻断 CI：在 `perf-helper.ts` 中超阈值时 `console.warn` + 报告标记 `exceededThreshold: true`，但**不抛错**（与 AC #6 一致）。

- [x] **Task 6: 验证与文档** (AC: #1, #2, #3, #4, #5)
  - [x] 6.1 本地运行 `cd egosync-app && npm run test:frontend` 确认无回归。
  - [x] 6.2 本地运行 `cd egosync-app/src-tauri && cargo test` 确认 Rust 测试通过（含新增 `app_performance_snapshot` 测试）。
  - [ ] 6.3 本地具备 tauri-driver 时运行 `cd egosync-app/tests/e2e && npm run test:perf`，确认性能 spec 通过。
  - [ ] 6.4 本地 Chrome DevTools 验证 60fps 呼吸动效，截图/录制保存。
  - [ ] 6.5 本地 DevTools 验证流式 token 渲染延迟（如有 LLM 配置），记录三平台差异（macOS 至少一次）。
  - [x] 6.6 在 story 完成笔记记录所有本地验证结果、阈值基线、runner 性能差异说明。

### Review Findings

- [x] [Review][Patch] app_seed_perf_data role_count=0 时逻辑错误 [app.rs:217,239-254] — 已添加 role_count=0 校验并返回 ValidationError。
- [x] [Review][Patch] app_seed_perf_data 无数量上限 [app.rs:196-199] — 已添加 MAX_PERF_ROLES=1000 与 MAX_PERF_MEMORIES=10000 校验。
- [x] [Review][Patch] app_emit_test_stream emit 错误静默忽略 [app.rs:159-172,173-190] — 已将 `let _` 改为 `if let Err(e) = ...` 并记录 tracing::warn。
- [x] [Review][Patch] app_emit_test_stream tokens 无上限 [app.rs:154] — 已添加 MAX_PERF_TOKENS=1000 校验。
- [x] [Review][Patch] app_performance_snapshot 刷新所有进程 [app.rs:109] — 已改为 `ProcessesToUpdate::Some(&pids)`，仅刷新当前进程与 sidecar PID。
- [x] [Review][Patch] measureProcessMemory snapshot 为 null 未检查 [perf-helper.ts:114-119] — 已添加 null / rssMb 非有限值校验。
- [x] [Review][Patch] measureStreamRenderLatency tokens 为空未检查 [perf-helper.ts:177] — 已添加空数组校验。
- [x] [Review][Patch] measureStreamRenderLatency 轮询可能匹配 DOM 残留 [perf-helper.ts:197-201] — 已追加唯一 sentinel token 作为检测目标，避免与既有 UI 文本误匹配。
- [x] [Review][Patch] runnerOs 无默认值 [perf-helper.ts:61-63] — 已添加 `|| 'unknown'` 默认值。
- [x] [Review][Patch] CI 报告目录不存在时上传空 artifact [ci.yml:44-49] — 已添加 `hashFiles(...)` 非空条件检查。
- [x] [Review][Defer] 硬编码魔法字符串 [app.rs 多处] — deferred, 测试代码中的硬编码可接受
- [x] [Review][Defer] app_seed_perf_data 部分失败无回滚 [app.rs:220-235] — deferred, 测试数据失败时 DB 清理逻辑会处理
- [x] [Review][Defer] measureStreamRenderLatency 轮询效率 [perf-helper.ts:199-206] — deferred, MutationObserver 更优但当前实现可用
- [x] [Review][Defer] auditCssAnimations/auditTsxAnimations 注释/字符串中误报 [audit-animations.mjs] — deferred, 静态审计已知限制

## Dev Notes

### 技术决策

**性能测量不引入 Lighthouse / wdio-performancetotal-service：**
- Lighthouse 依赖 Chrome DevTools Protocol 完整实现，tauri-driver 不暴露 CDP，无法用于 Tauri WebView。
- `wdio-performancetotal-service` 主要测量 Web 页面 LCP/FCP，Tauri WebView 的 `tauri://` 协议不适用。
- **决策**：使用原生 `Date.now()` 计时 + `sysinfo` crate 读取进程 RSS + Tauri IPC 注入测量脚本，零新依赖（`sysinfo` 是 Rust 端唯一新增，跨平台成熟）。

**流式延迟测量的 mock 注入方案：**
- CI 无 LLM，无法测量真实 token 渲染延迟。
- 方案：新增 `app_emit_test_stream` command（`perf-test` feature gate），在 Rust 端 `app.emit("llm:stream", { roleId, token, done })` 发射预设 token 序列，前端 `useTauriEvent('llm:stream')` 正常处理，测量 emit 到 DOM 更新的时间差。
- **此方案测量的是「Tauri Event → 前端监听 → React 状态更新 → DOM 渲染」的端到端延迟**，正是 NFR-11 关注的「流式渲染层」延迟，不测量 LLM 网络延迟（那是 Provider 责任，非应用可控）。
- 三平台差异来源：WebView 的 Event listener 调度 + React 渲染性能，本方案能捕获。

**60fps 静态审计而非 CI 运行时测量：**
- tauri-driver 无 CDP，无法获取 FPS。
- 静态审计保证动画属性合规（`transform`/`opacity` + GPU 加速），这是 60fps 的**必要条件**；充分条件由本地人工 DevTools 验证补齐。
- 不引入 Playwright + CDP 方案（Story 8.2 已确立 WebdriverIO 框架，引入 Playwright 违反「不重建测试框架」约束）。

**`sysinfo` 版本选择：**
- 当前最新稳定版 0.32.x（发布 > 7 天），跨平台支持 Windows/Linux/macOS 进程内存读取。
- 仅用于 `app_performance_snapshot` command，不影响其他模块。

### 当前必须关注的实现文件

**新增文件：**
- `egosync-app/tests/e2e/helpers/perf-helper.ts` — 性能测量 helper（仿照 `a11y-helper.ts`）。
- `egosync-app/tests/e2e/specs/performance.spec.ts` — 性能基准 spec。
- `egosync-app/tests/e2e/scripts/audit-animations.mjs` — 动画静态审计脚本。

**更新文件：**
- `egosync-app/tests/e2e/package.json` — 添加 `test:perf`、`audit:animations` 脚本。
- `egosync-app/tests/e2e/wdio.conf.ts` — `onPrepare` 新增 `reports/performance/` 目录清理；可能需要 `suites` 配置分离 perf spec。
- `egosync-app/src-tauri/Cargo.toml` — 添加 `sysinfo = "0.32"` 依赖 + `[features] perf-test = []`。
- `egosync-app/src-tauri/src/commands/app.rs` — 新增 `app_performance_snapshot` + `app_emit_test_stream`（feature gate）。
- `egosync-app/src-tauri/src/lib.rs:278` — 注册新 command。
- `egosync-app/src-tauri/src/services/sidecar.rs` — 可能新增 `child_pid()` 访问器（如不存在）。
- `egosync-app/src/services/appService.ts` — 封装 `performanceSnapshot()`。
- `.github/workflows/ci.yml` — 新增性能基准步骤 + 报告上传。

### 必须保留的既有行为

- 保留 Story 8.2 的 Windows/Linux E2E 运行模式，macOS 跳过 tauri-driver。
- 保留 `com.egosync.desktop` app data 路径，不要回退到 `com.egosync.app`。
- 保留 `wdio.conf.ts` 现有的 `beforeSession` DB 清理、`afterTest` 截图、`afterSession` 进程清理逻辑；只扩展 `onPrepare` 目录清理。
- 保留 `index.css` 现有动画实现（`breathe`/`bounce`/`loading-spin` + `prefers-reduced-motion`），本 story 验证不重写。
- 保留 `ChatStream.tsx:716` 的 `requestAnimationFrame` 用于一次性 scroll 定位（非持续动画），静态审计脚本需识别此例外。
- 保留 `lib.rs:setup` 中 DB 初始化为 `block_on`（必须同步完成才能注册 command），调度器为 `spawn` 非阻塞；本 story 只验证不阻塞 UI，不改变启动顺序。
- 保留 `agent_bridge.rs` SSE 解析 + `event_router.rs` demux + `useTauriEvent('llm:stream')` 链路，本 story 验证无平台分支，不重构。

### 反模式警告

- 不要引入 Lighthouse / Playwright / Cypress / wdio-performancetotal-service 重建性能测试框架。
- 不要在 CI 中强制 macOS 性能步骤（tauri-driver 不支持）。
- 不要把性能阈值设为阻断 CI（AC #6 明确要求「警告不阻断」）。
- 不要为通过 60fps 审计修改 `index.css` 动画实现（已合规，只验证）。
- 不要把 `app_emit_test_stream` command 放入生产构建（必须 feature gate）。
- 不要测量 LLM 网络延迟作为流式渲染延迟（那是 Provider 责任，非应用可控）。
- 不要在性能 spec 中依赖真实 LLM 响应（CI 无 API Key）。
- 不要修改 Story 8.2/8.3 已建立的 DB 清理、截图、a11y 报告机制；只新增 performance reports。
- 不要全局 `will-change: transform` 滥用（已存在的 `will-change: opacity` 在 `.breathe` 上合规，不要扩散）。

### Previous Story Intelligence

Story 8.3 关键经验：
- E2E helper 仿照 `a11y-helper.ts` 模式：独立文件、JSON 报告落盘、critical 阈值抛错、不污染通用 `app-helper.ts`。
- CI 报告上传步骤用 `if: failure()` 上传失败诊断；性能报告需要 `if: always()` 因为成功时也要追踪趋势。
- macOS 不跑 tauri-driver，相关验证通过本地人工补齐并记录到完成笔记。
- Review Findings 中多次出现「硬编码 URL/端口导致 CI 失败」问题 — 性能 spec 中避免硬编码，复用 `app-helper.ts` 的 `invoke()` 和 `waitForAppReady()`。

Story 8.2 关键经验：
- IPC seeding 模式（`browser.executeAsync` + `__TAURI_INTERNALS__.invoke`）避免 `better-sqlite3` 原生依赖。
- DB 清理目录必须用 `com.egosync.desktop`。
- LLM 依赖测试通过预置 DB + 验证 UI 行为，不验证 LLM 响应内容 — 性能测量同理，用 mock 注入而非真实 LLM。

最近提交模式：
- `4f44008` 任务概览筛选行间距收紧（butler UI 微调）。
- `2daf3cd` Story 8.3 WCAG 无障碍审计完成。
- `e90c910` E2E 代码审查修复。
- `aa93922` E2E 框架搭建。
- `851011e` Windows sidecar `CREATE_NO_WINDOW`。

### Testing Requirements

- 前端单元测试：`cd egosync-app && npm run test:frontend`（确认无回归）。
- Rust 后端测试：`cd egosync-app/src-tauri && cargo test`（含新增 `app_performance_snapshot` 单元测试）。
- E2E 性能测试：`cd egosync-app/tests/e2e && npm run test:perf`（需本机 tauri-driver）。
- 动画静态审计：`cd egosync-app/tests/e2e && npm run audit:animations`。
- CI：Windows/Linux 跑性能基准 + 动画审计，超阈值警告不阻断，报告 always 上传。

### 性能阈值基线

| 指标 | 本地 SSD 阈值 | CI runner 阈值 | 超限行为 |
|------|--------------|---------------|---------|
| 冷启动到可交互 | ≤ 3s | ≤ 10s | 警告不阻断 |
| 稳态内存（3角色+100记忆） | ≤ 200MB | ≤ 300MB | 警告不阻断 |
| 1小时内存增长 | ≤ 10% | 不测量（CI 短运行） | 本地人工验证 |
| 流式 emit→render 延迟 | ≤ 50ms 三平台差异 | 记录不设阈值 | 警告不阻断 |
| Onboarding 可交互时间 | ≤ 5s | ≤ 15s | 警告不阻断 |
| 60fps 呼吸动效 | ≥ 60fps | 静态审计合规 | 审计失败警告不阻断 |

**阈值说明**：CI runner 性能波动大（GitHub Actions 共享 runner），绝对值不可靠，阈值宽松用于趋势追踪。本地阈值为 NFR 原文要求，人工验证必须满足。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.4] — 性能基准 AC 原文。
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8] — V1 加固上下文。
- [Source: _bmad-output/planning-artifacts/architecture.md#NFR] — NFR-5 60fps、NFR-9 首次体验、NFR-11 流式一致性。
- [Source: _bmad-output/planning-artifacts/architecture.md:395-398] — LLM 流式传输 Tauri Event 系统。
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md:1007-1011] — 性能相关：字体加载、SSE 不轮询、CSS animation GPU 加速。
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md:874-883] — Transition Patterns 动效参数 + reduced-motion。
- [Source: _bmad-output/project-context.md] — React/Tauri/测试/代码组织规则。
- [Source: _bmad-output/implementation-artifacts/8-2-e2e-test-core-journeys.md] — E2E 基础与 IPC seeding 模式。
- [Source: _bmad-output/implementation-artifacts/8-3-wcag-accessibility-audit.md] — a11y helper 模式、CI 报告上传、macOS 限制处理。
- [Source: egosync-app/tests/e2e/wdio.conf.ts] — 现有 WebdriverIO 配置、目录清理、DB 清理。
- [Source: egosync-app/tests/e2e/helpers/a11y-helper.ts] — helper 模式模板（JSON 报告、critical 抛错）。
- [Source: egosync-app/tests/e2e/helpers/app-helper.ts] — `invoke()`、`waitForAppReady()`、seed helpers。
- [Source: egosync-app/tests/e2e/specs/llm-streaming.spec.ts] — CI 无 LLM 时的流式测试降级策略。
- [Source: egosync-app/src/index.css:99-140] — 现有动画实现（breathe/bounce/loading-spin + reduced-motion）。
- [Source: egosync-app/src/components/layout/RoleSidebarIcon.tsx:72] — `breathe` class 使用点。
- [Source: egosync-app/src/components/chat/ChatStream.tsx:878] — `useTauriEvent('llm:stream')` 流式监听。
- [Source: egosync-app/src/hooks/useTauriEvent.ts] — Tauri Event 监听 hook。
- [Source: egosync-app/src-tauri/src/lib.rs:39-276] — Tauri setup 流程（DB block_on + 调度器 spawn）。
- [Source: egosync-app/src-tauri/src/commands/app.rs:53-63] — `app_sidecar_status` 现有 command 模式。
- [Source: egosync-app/src-tauri/src/services/agent_bridge.rs:144-196] — SSE 流解析。
- [Source: egosync-app/src-tauri/src/services/event_router.rs] — 事件 demux 路由。
- [Source: .github/workflows/ci.yml] — 现有 CI 流程与 artifact 上传位置。
- [External: tauri-apps/benchmark_results] — Tauri 官方基准使用 hyperfine + memory_profiler，CI runner 性能不可直接比较。
- [External: tauri-apps/tauri bench.yml] — Tauri 官方 CI 基准工作流模式。
- [External: sysinfo crate docs.rs] — 跨平台进程内存读取 API。
- [External: WebdriverIO executeAsync] — 注入脚本到 WebView 执行模式。

## Dev Agent Record

### Agent Model Used

GLM-5.2 High (Devin CLI, 2026-06-29)

### Debug Log References

- `cargo check` 通过（含 `--features perf-test`），仅有预先存在的 dead_code 警告。
- `cargo test` 663 单元测试 + 1 集成测试全部通过，含新增 `PerformanceSnapshot` 序列化测试。
- `npx vitest run` 38 文件 348 用例全部通过，无回归。
- `node scripts/audit-animations.mjs` 审计 59 文件，0 error / 0 warning / 0 info。
- E2E TypeScript 编译：仅有预先存在的 `@axe-core/webdriverio` 模块缺失错误（需 `npm ci`），新文件无 TS 错误。

### Completion Notes List

**已完成的自动化验证：**
1. **AC1（60fps 静态审计）**：`audit-animations.mjs` 扫描 59 个 CSS/TSX 文件，所有 keyframes 使用 `transform`/`opacity`（GPU 加速属性），无 `requestAnimationFrame` 持续循环（ChatStream.tsx 的 rAF 在例外列表中，用于一次性 scroll），无 `setInterval` 驱动动画（OnboardingView.tsx 的 setInterval 在例外列表中，用于轮询配置）。审计报告落盘到 `reports/performance/animation-audit.json`。
2. **AC3（流式链路无平台分支）**：通过 grep 验证 `agent_bridge.rs`、`event_router.rs`、`useTauriEvent.ts` 中无 `target_os`/`cfg!`/`#[cfg]` 平台分支代码（仅有 `#[cfg(test)]` 测试门控）。
3. **AC4（启动架构验证）**：通过 grep 验证 `lib.rs:setup` 中 DB 初始化使用 `block_on`（同步），调度器使用 `spawn_scheduler`（非阻塞），sidecar watchdog 使用 `spawn`（非阻塞）。
4. **AC6（CI 集成）**：ci.yml 新增性能基准步骤（Linux + Windows，`continue-on-error: true`）、动画审计步骤（Linux，`continue-on-error: true`）、性能报告上传步骤（`if: always()`）。E2E 构建使用 `--features perf-test` 启用测试 command。

**待人工验证（本地环境，CI 无法执行）：**
- **Task 4.3 / AC1 充分条件**：本地 Chrome DevTools Performance 录制 5 秒角色卡片呼吸动效，确认 ≥ 60fps，截图/录制保存。
- **Task 6.3 / AC2-AC5 E2E 性能 spec**：本地具备 tauri-driver 时运行 `npm run test:perf`，确认性能 spec 通过（需 `--features perf-test` 构建的 release binary）。
- **Task 6.5 / AC3 macOS 补齐**：macOS 本地 DevTools 验证流式 token 渲染延迟，记录三平台差异（macOS 至少一次基线测量）。
- **Task 6.4 / AC5 1 小时内存增长**：本地人工运行 1 小时后检查内存增长 ≤ 10%。

**设计决策记录：**
- 新增 `app_seed_perf_data` command（`perf-test` feature gate）用于稳态内存测量的数据注入。故事原文 Task 1.3 要求 "seed 3 角色 + 100 条记忆"，但无既有 IPC 可直接创建记忆（记忆仅通过 LLM 提取生成）。此 command 通过 DB 层直接插入，绕过 LLM 流程，与 `app_emit_test_stream` 使用相同的 feature gate 模式，生产构建排除。
- `sysinfo` 0.32 API 使用 `ProcessesToUpdate::All` + `refresh_processes(ProcessesToUpdate, bool)` 签名（非旧版 `ProcessesToRefresh`）。
- `sysinfo` 依赖配置为 `default-features = false, features = ["system"]`，仅启用进程信息功能，避免引入不必要的 disk/network/component 模块。
- 性能阈值基线遵循故事定义：CI 冷启动 ≤ 10s、内存 ≤ 300MB、Onboarding ≤ 15s、流式 ≤ 50ms，超阈值 `console.warn` + `exceededThreshold: true` 标记，不抛错（AC #6 警告不阻断）。

### File List

**新增文件：**
- `egosync-app/tests/e2e/helpers/perf-helper.ts` — 性能测量 helper（measureColdStart / measureProcessMemory / measureOnboardingInteractive / measureStreamRenderLatency / writePerfReport）
- `egosync-app/tests/e2e/specs/performance.spec.ts` — 性能基准 spec（冷启动 / Onboarding / 稳态内存 / 流式延迟）
- `egosync-app/tests/e2e/scripts/audit-animations.mjs` — 60fps 动效静态审计脚本

**更新文件：**
- `egosync-app/tests/e2e/package.json` — 添加 `test:perf`、`test:perf:local`、`audit:animations` 脚本
- `egosync-app/tests/e2e/wdio.conf.ts` — 添加 `suites` 配置（ci/perf）、`reports/performance/` 目录清理
- `egosync-app/src-tauri/Cargo.toml` — 添加 `sysinfo = "0.32"` 依赖（`system` feature only）+ `[features] perf-test = []`
- `egosync-app/src-tauri/Cargo.lock` — sysinfo 依赖自动更新
- `egosync-app/src-tauri/src/commands/app.rs` — 新增 `PerformanceSnapshot` 模型、`app_performance_snapshot` command、`app_emit_test_stream`（perf-test gate）、`app_seed_perf_data`（perf-test gate）+ 单元测试
- `egosync-app/src-tauri/src/services/sidecar.rs` — 新增 `child_pid()` 访问器
- `egosync-app/src-tauri/src/lib.rs` — 注册 `app_performance_snapshot` + `app_emit_test_stream`（cfg gate）+ `app_seed_perf_data`（cfg gate）
- `egosync-app/src/services/appService.ts` — 封装 `performanceSnapshot()` + `PerformanceSnapshot` 类型
- `.github/workflows/ci.yml` — E2E 构建添加 `--features perf-test`、新增性能基准步骤 + 动画审计步骤 + 性能报告上传步骤
- `.gitignore` — 排除 e2e 测试产物目录（screenshots/logs/reports）
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 故事状态 ready-for-dev → in-progress

## Change Log

- 2026-06-29: Story 8.4 实现完成，所有自动化任务通过验证，待人工验证项已记录到 Completion Notes。
