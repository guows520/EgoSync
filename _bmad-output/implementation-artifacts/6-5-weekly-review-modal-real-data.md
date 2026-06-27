---
baseline_commit: da49e65a2f3c078fe0ccd355b82d0519f8cbe8ae
---

# Story 6.5: WeeklyReviewModal 接通真实复盘数据和规划功能

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 在可视化界面中回顾本周并规划下周大石头,
So that 有仪式感地完成每周的节奏闭环。

## 背景与现状（务必先读）

**本 story 是 Epic 6 的第五个 story — 在 Story 6.1（晨间简报）、Story 6.2（节奏化时间配置）、Story 6.3（大石头规划提醒）、Story 6.4（周复盘成绩单自动生成）基础上，将 `WeeklyReviewModal` 从 mock 数据接通真实后端数据，并实现"规划下周大石头"的完整交互链路（AI 建议 → 用户采纳/手动输入 → 确认创建任务）。**

**核心交付：**
1. **复盘阶段（phase='review'）**：从后端加载 `weekly_reviews` 记录，展示真实复盘摘要、能量趋势柱状图（SVG 自绘）、大石头完成列表
2. **规划阶段（phase='plan'）**：加载活跃角色列表，调用后端 AI 建议接口生成每个角色 1-2 个大石头建议，用户可采纳/手动输入/添加多个，确认后批量创建 `is_big_rock=true` 任务并触发四象限自动分类
3. **日期范围**：自动计算本周一到本周日，显示格式"YYYY年M月D日 - D日"

### 已建成的基础（本 story 的接入点）

**Story 6.4 已完成的后端设施（直接复用）：**
- `src-tauri/migrations/025_weekly_reviews.sql` — `weekly_reviews` 表已创建
- `src-tauri/src/models/weekly_review.rs` — `WeeklyReview` 结构体已定义
- `src-tauri/src/db/weekly_reviews.rs` — `create_weekly_review` / `get_weekly_review_by_week_start` / `get_latest_weekly_review` 已实现
- `src-tauri/src/services/review_generator.rs` — 周复盘自动生成服务已实现（调度器触发 → 收集数据 → LLM 生成 → 写入 DB + 管家对话 + 通知 + emit 事件）
- `src-tauri/src/commands/review.rs` — `review_get_latest` / `review_get_by_week` / `review_generate_now` 三个 command 已注册
- `src-tauri/src/lib.rs:366-368` — 三个 review command 已在 `invoke_handler` 中注册

**Story 6.4 已完成的前端设施（直接复用）：**
- `src/types/review.ts` — `ReviewGeneratedPayload` 和 `WeeklyReview` 类型已定义
- `src/services/reviewService.ts` — `getLatestReview()` / `getReviewByWeek(weekStart)` / `generateReviewNow()` 已封装
- `src/App.tsx:117-123` — `review:generated` 事件监听已实现（递增 `butlerChatRefreshTrigger`）

**Story 6.3 已完成的前端设施（直接复用）：**
- `src/App.tsx:105-114` — `bigrock:reminder` 事件监听，打开 WeeklyReviewModal 并传入 `initialPhase='plan'`
- `src/App.tsx:41-42` — `isReviewOpen` + `reviewInitialPhase` state
- `src/App.tsx:380` — `<WeeklyReviewModal roles={roles} onClose={...} initialPhase={reviewInitialPhase} />` 渲染

**任务创建设施（直接复用）：**
- `src-tauri/src/commands/task.rs:14-51` — `task_create` command，支持 `is_big_rock` 参数，创建后自动触发四象限分类
- `src-tauri/src/db/tasks.rs` — `create_task` 函数，含大石头数量限制校验（每 owner 最多 3 个）
- `src-tauri/src/services/task_classifier.rs` — `classify_and_persist` 异步自动分类
- `src/services/taskService.ts` — `taskService.create(input)` 前端封装
- `src/types/task.ts` — `CreateTaskInput` 类型，含 `isBigRock?: boolean`

**角色数据设施（直接复用）：**
- `src-tauri/src/db/roles.rs` — `list_active_roles(pool)` 函数
- `src-tauri/src/commands/role.rs` — `role_list` command
- `src/services/roleService.ts` — `roleService.list()` 前端封装
- `src/types/role.ts` — `Role` 类型，含 `id` / `name` / `icon` / `color` / `goal` / `energy`

**LLM 调用设施（参照模式）：**
- `src-tauri/src/services/review_generator.rs:468-523` — `call_llm` 函数（stream + timeout + size limit），可直接复制复用
- `src-tauri/src/services/agent_engine.rs:1928-1957` — `resolve_default_provider(pool)` 函数
- `src-tauri/src/llm/traits.rs` — `ChatCompletionMessage` / `ChatOptions` / `LlmProvider` / `StreamEvent` trait

**能量趋势数据格式（Story 6.4 已定义）：**
- `weekly_reviews.energy_trends` JSON 格式：`{ "role_id": { "energy": 85, "energyUpdatedAt": "2026-..." } }`
- `weekly_reviews.bigrock_status` JSON 格式：`[{ "id": "...", "title": "...", "isCompleted": true, "completedAt": "...", "roleName": "..." }]`

**WeeklyReviewModal 现状（本 story 需重写）：**
- `src/components/modals/WeeklyReviewModal.tsx` — 126 行，全部使用 mock 数据：
  - 复盘摘要：硬编码中文段落
  - 能量趋势：3 个硬编码柱状图（indigo 85% / amber 60% / purple 70%）
  - 大石头列表：3 个硬编码条目（2 个 ✓ + 1 个 →）
  - 规划阶段：3 个硬编码角色卡片（产品经理/家庭/学习者），每个含 mock suggestion
  - 日期范围：硬编码 "2026年5月12日 - 18日"

### 本 story 需要做的事

**核心变更：将 WeeklyReviewModal 从 mock 数据全面接通真实后端数据，并新增 AI 大石头建议 + 批量创建功能。**

1. **Rust 后端 — 新增 `review_plan_bigrocks` command**：接收 `Vec<BigRockPlanItem>` 批量创建大石头任务
2. **Rust 后端 — 新增 `review_get_bigrock_suggestions` command**：调用 LLM 为每个活跃角色生成 1-2 个大石头建议
3. **前端 — 重写 `WeeklyReviewModal.tsx`**：替换所有 mock 数据为真实数据加载
4. **前端 — 新增 `useWeeklyReview` hook**：封装复盘数据加载
5. **前端 — 新增 `useBigRockPlanning` hook**：封装 AI 建议加载和批量保存
6. **前端 — 新增 `bigrockSuggestion` 类型和服务函数**

## Acceptance Criteria

1. **AC1**: Given 用户打开 WeeklyReviewModal（phase='review'），When 复盘阶段，Then 顶部显示自然语言复盘摘要（从 `weekly_reviews.summary` 加载，替换原型 mock 文本），And 左侧显示角色能量趋势柱状图（从 `weekly_reviews.energyTrends` JSON 解析，自绘 SVG，替换原型硬编码），And 右侧显示本周大石头完成列表（从 `weekly_reviews.bigrockStatus` JSON 解析，✓ 完成 / → 继续推进）

2. **AC2**: Given 用户点击"规划下周大石头"按钮，When 切换到 phase='plan'，Then 每个活跃角色显示一个规划卡片（从 `roleService.list()` 加载），And AI 建议 1-2 个大石头（调用 `review_get_bigrock_suggestions` command，基于上周能量 + 未完成目标 + 角色目标），And 用户可点击"采纳"自动填入 / 手动输入 / 添加多个

3. **AC3**: Given 用户点击"确认规划"，When 保存，Then 每个填入的大石头创建为 `tasks`（`is_big_rock = true`，角色关联），And 触发四象限自动分类（复用 E3 Story 3.3），And Modal 关闭

4. **AC4**: Given 周复盘日期范围，Then 自动计算本周一到本周日，And 显示格式"YYYY年M月D日 - D日"

5. **AC5**: Given 前端，Then WeeklyReviewModal 接通真实数据（替换原型所有 mock 数据），And `useWeeklyReview(weekStart)` hook 封装复盘数据加载，And `useBigRockPlanning()` hook 封装 AI 建议和保存

6. **AC6**: Given Rust 后端，Then Tauri commands: `review::get_weekly { week_start }`（已存在）/ `review::plan_bigrocks { items: Vec<{role_id, title}> }`（新增）

7. **AC7**: Given 无复盘数据时（本周尚未生成复盘），When 用户打开 Modal，Then 复盘阶段显示空状态文案（如"本周复盘尚未生成"），And 仍可切换到规划阶段

8. **AC8**: Given AI 建议加载中，When 等待，Then 规划卡片显示加载状态（如"正在思考建议..."），And 加载失败时显示降级文案（如"手动填写本周大石头"），不阻塞用户手动输入

## Tasks / Subtasks

- [x] **Task 1: Rust — 新增 `review_plan_bigrocks` command** (AC: #3, #6)
  - [x] 1.0 在 `src-tauri/src/commands/review.rs` 中新增 `review_plan_bigrocks` command
  - [x] 1.1 定义 `BigRockPlanItem` 结构体（`Deserialize` + `rename_all = "camelCase"`），字段：`role_id: String`, `title: String`
  - [x] 1.2 实现 `review_plan_bigrocks(items: Vec<BigRockPlanItem>, app_handle: AppHandle, pool: State<DbPool>) -> Result<Vec<Task>, AppError>`：
    - 遍历 items，为每个 item 构造 `CreateTaskInput { owner_type: Some(Role), role_id: Some(role_id), title, is_big_rock: Some(true) }`
    - 调用 `db::tasks::create_task(&pool, &input)` 创建任务
    - 对每个创建的任务，spawn 异步调用 `task_classifier::classify_and_persist` 触发自动分类（参照 `commands/task.rs:32-48` 模式）
    - 返回创建的 `Vec<Task>`
  - [x] 1.3 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册 `commands::review::review_plan_bigrocks`

- [x] **Task 2: Rust — 新增 `review_get_bigrock_suggestions` command** (AC: #2, #8)
  - [x] 2.0 在 `src-tauri/src/services/review_generator.rs` 中新增 `generate_bigrock_suggestions(pool) -> Result<Vec<RoleBigRockSuggestions>, AppError>` 函数：
    - 加载活跃角色列表（`db::roles::list_active_roles`）
    - 加载本周未完成的大石头任务（`db::tasks::list_all_tasks(pool, None, Some(true))`，过滤 `!is_completed`）
    - 构造 LLM prompt（`build_suggestion_prompt`）：System prompt 要求为每个角色建议 1-2 个大石头，User prompt 包含角色名称/目标/当前能量值/本周未完成大石头
    - 解析 LLM provider（`agent_engine::resolve_default_provider`）
    - 调用 LLM（复用 `call_llm` 模式）
    - 解析 LLM 返回的 JSON（格式：`[{ "roleId": "...", "suggestions": ["建议1", "建议2"] }]`）
    - 错误降级：LLM 失败/超时/解析失败 → 返回空列表（前端降级为手动输入）
  - [x] 2.1 定义 `RoleBigRockSuggestions` 结构体（`Serialize` + `rename_all = "camelCase"`），字段：`role_id: String`, `role_name: String`, `suggestions: Vec<String>`
  - [x] 2.2 在 `src-tauri/src/commands/review.rs` 中新增 `review_get_bigrock_suggestions` command，调用 `review_generator::generate_bigrock_suggestions`
  - [x] 2.3 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册 `commands::review::review_get_bigrock_suggestions`

- [x] **Task 3: 前端 — 新增类型和服务函数** (AC: #5, #6)
  - [x] 3.0 在 `src/types/review.ts` 中新增 `BigRockPlanItem` 类型：`{ roleId: string; title: string }`
  - [x] 3.1 在 `src/types/review.ts` 中新增 `RoleBigRockSuggestions` 类型：`{ roleId: string; roleName: string; suggestions: string[] }`
  - [x] 3.2 在 `src/services/reviewService.ts` 中新增：
    - `planBigRocks(items: BigRockPlanItem[]): Promise<Task[]>` — 调用 `invoke('review_plan_bigrocks', { items })`
    - `getBigRockSuggestions(): Promise<RoleBigRockSuggestions[]>` — 调用 `invoke('review_get_bigrock_suggestions')`

- [x] **Task 4: 前端 — 新增 `useWeeklyReview` hook** (AC: #1, #5, #7)
  - [x] 4.0 新建 `src/hooks/useWeeklyReview.ts`
  - [x] 4.1 实现 `useWeeklyReview(weekStart: string | null)`：
    - `weekStart` 为 null 时不加载（返回 `null` + `isLoading: false`）
    - 调用 `reviewService.getReviewByWeek(weekStart)` 加载复盘数据
    - 返回 `{ review: WeeklyReview | null, isLoading: boolean, error: string | null }`
    - 解析 `review.energyTrends` JSON 字符串为 `EnergyTrendsData` 对象
    - 解析 `review.bigrockStatus` JSON 字符串为 `BigRockStatusItem[]` 数组
  - [x] 4.2 定义 `EnergyTrendsData` 类型：`Record<string, { energy: number; energyUpdatedAt?: string }>`
  - [x] 4.3 定义 `BigRockStatusItem` 类型：`{ id: string; title: string; isCompleted: boolean; completedAt: string | null; roleName: string | null }`

- [x] **Task 5: 前端 — 新增 `useBigRockPlanning` hook** (AC: #2, #3, #5, #8)
  - [x] 5.0 新建 `src/hooks/useBigRockPlanning.ts`
  - [x] 5.1 实现 `useBigRockPlanning(roles: Role[])`：
    - `loadSuggestions()`：调用 `reviewService.getBigRockSuggestions()`，为每个角色匹配建议列表
    - `savePlan(items: BigRockPlanItem[])`：调用 `reviewService.planBigRocks(items)`，返回创建的 Task 列表
    - 返回 `{ suggestions: RoleBigRockSuggestions[] | null, isLoadingSuggestions: boolean, isSaving: boolean, error: string | null, loadSuggestions, savePlan }`
    - suggestions 初始为 null（未加载），loadSuggestions 调用后更新
    - 错误降级：加载失败 → suggestions 设为空数组，error 设为友好文案

- [x] **Task 6: 前端 — 重写 `WeeklyReviewModal.tsx`** (AC: #1, #2, #3, #4, #7, #8)
  - [x] 6.0 计算本周日期范围：本周一到本周日，显示格式"YYYY年M月D日 - D日"
    - 使用 `new Date()` 获取今天，计算本周一：`today - (today.getDay() - 1) * 86400000`（getDay 返回 0=周日 ~ 6=周六，需处理周日为 0 的边界）
    - `weekStart` 格式为 "YYYY-MM-DD"（传给后端查询）
    - 显示格式：`${year}年${month}月${startDay}日 - ${endDay}日`
  - [x] 6.1 复盘阶段（phase='review'）重写：
    - 调用 `useWeeklyReview(weekStart)` 加载复盘数据
    - 顶部摘要：显示 `review.summary`（替换 mock 段落）；无数据时显示"本周复盘尚未生成，可在设置中手动触发或等待自动生成"
    - 左侧能量趋势：从 `energyTrendsData` 渲染 SVG 柱状图（每个角色一根柱子，高度 = energy%，颜色用角色 color），替换硬编码柱状图
    - 右侧大石头列表：从 `bigrockStatusList` 渲染（✓ 已完成 = emerald + CheckCircle2 / → 继续推进 = amber + ArrowRight），替换硬编码列表
    - 无复盘数据时，能量趋势和大石头区域显示空状态文案
    - "规划下周大石头"按钮始终可见
  - [x] 6.2 规划阶段（phase='plan'）重写：
    - 从 `roles` prop 获取活跃角色列表（App.tsx 已传入 `roles` state，仅含 `status === 'active'` 的角色）
    - 进入 plan 阶段时调用 `useBigRockPlanning(roles).loadSuggestions()` 加载 AI 建议
    - 每个角色一个规划卡片：角色名称（用角色 color 着色）、AI 建议区域（加载中显示"正在思考建议..."，加载失败显示"手动填写本周大石头"，有建议时显示建议 + "采纳"按钮）、大石头输入框列表（可添加/删除）
    - "采纳"按钮：将建议文本填入第一个空输入框（或新增一行）
    - "确认规划"按钮：收集所有非空 title 的输入项，构造 `BigRockPlanItem[]`，调用 `savePlan(items)`，成功后 `onClose()` 关闭 Modal
    - "返回复盘"按钮：切回 phase='review'
  - [x] 6.3 保持现有组件签名 `WeeklyReviewModal({ roles, onClose, initialPhase })`，不修改 App.tsx 中的调用方式
  - [x] 6.4 保持现有视觉风格：Tailwind utility class、圆角、间距、配色不变，仅替换数据源

- [x] **Task 7: 前端测试** (AC: #1, #2, #3, #7, #8)
  - [x] 7.1 `useWeeklyReview.test.ts` — 测试正常加载、无数据（null）、JSON 解析
  - [x] 7.2 `useBigRockPlanning.test.ts` — 测试建议加载、保存、错误降级
  - [x] 7.3 `WeeklyReviewModal.test.tsx` — 测试复盘阶段渲染真实数据、规划阶段渲染角色卡片、确认规划调用 savePlan、无复盘数据时空状态

- [x] **Task 8: Rust 测试** (AC: #2, #3, #8)
  - [x] 8.1 在 `review_generator.rs` 中测试 `build_suggestion_prompt`：包含角色名称/目标/能量值、包含未完成大石头
  - [x] 8.2 在 `review_generator.rs` 中测试 `generate_bigrock_suggestions` LLM 失败降级（返回空列表）
  - [x] 8.3 在 `commands/review.rs` 或集成测试中测试 `review_plan_bigrocks`：批量创建大石头任务、任务 `is_big_rock = true`、触发自动分类

## Dev Notes

### 关键技术决策

- **能量趋势柱状图自绘 SVG**：架构文档明确要求"自绘 SVG（周复盘能量趋势图）— 仅需简单柱状图，无需引入 Recharts/Chart.js"。`energy_trends` JSON 存储的是各角色当前能量值快照（非历史趋势），因此渲染为简单柱状图即可：每个角色一根柱子，高度 = `energy%`，颜色用角色 `color` hex 值。柱状图下方显示角色名称。

- **bigrock_status JSON 解析**：`weekly_reviews.bigrock_status` 是 JSON 字符串数组，前端需 `JSON.parse` 后渲染。每项含 `id` / `title` / `isCompleted` / `completedAt` / `roleName`。`isCompleted = true` 显示 ✓ + emerald，`isCompleted = false` 显示 → + amber。

- **AI 大石头建议的 LLM 返回格式**：LLM 返回 JSON 数组 `[{"roleId": "uuid", "suggestions": ["建议1", "建议2"]}]`。System prompt 需明确要求返回合法 JSON，并包含角色 ID 以便前端匹配。LLM 返回非法 JSON 时降级为空列表，前端显示手动输入界面。

- **`review_plan_bigrocks` 批量创建**：不直接调用 `task_create` command（跨 command 调用不规范），而是直接调用 `db::tasks::create_task` 函数。每个任务创建后 spawn 异步分类（参照 `commands/task.rs:32-48` 模式）。大石头数量限制（每 owner 最多 3 个）由 `db::tasks::create_task` 内部校验，超限时返回 `ValidationError`。

- **日期范围计算**：JavaScript `Date.getDay()` 返回 0=周日 ~ 6=周六。本周一计算：`new Date(today.getFullYear(), today.getMonth(), today.getDate() - (today.getDay() === 0 ? 6 : today.getDay() - 1))`。本周日 = 周一 + 6 天。`weekStart` 传给后端的格式为 "YYYY-MM-DD"（与 `weekly_reviews.week_start` 列格式一致）。

- **`useWeeklyReview` hook 的 weekStart 参数**：当 Modal 打开时，计算本周一日期字符串传入 hook。hook 内部调用 `reviewService.getReviewByWeek(weekStart)`。如果后端返回 `null`（本周尚未生成复盘），前端显示空状态文案但仍允许切换到规划阶段。

- **`useBigRockPlanning` hook 的调用时机**：进入 plan 阶段时调用 `loadSuggestions()`。建议加载是异步的，加载期间规划卡片显示"正在思考建议..."。加载完成后，每个角色卡片显示对应的 AI 建议。加载失败时降级为空建议列表，不阻塞手动输入。

- **WeeklyReviewModal props 不变**：保持现有签名 `({ roles, onClose, initialPhase })`，App.tsx 中的调用无需修改。`roles` 已是活跃角色列表（App.tsx 的 `roles` state 来自 `roleService.list()`，已过滤 `status === 'active'`）。

- **确认规划后的 Modal 关闭**：`savePlan` 成功后调用 `onClose()` 关闭 Modal。App.tsx 的 `onClose` 回调会重置 `reviewInitialPhase` 为 'review'。不需要额外的事件通知——任务创建后 `task:classified` 事件会自动推送，各角色任务面板会自动刷新。

### 架构合规

- **分层规则**：前端 `WeeklyReviewModal` → `useWeeklyReview` / `useBigRockPlanning` hook → `reviewService` → Tauri `invoke` → `commands/review.rs` → `db::tasks::create_task` / `services::review_generator::generate_bigrock_suggestions` / `services::task_classifier::classify_and_persist`
- **Command 层职责**：`review_plan_bigrocks` 只做参数解析 → 调 `db::tasks::create_task` → spawn 异步分类 → 返回结果；`review_get_bigrock_suggestions` 只做参数解析 → 调 `review_generator::generate_bigrock_suggestions` → 返回结果
- **Service 层职责**：`review_generator::generate_bigrock_suggestions` 负责"加载角色/任务数据 → 构造 prompt → LLM 调用 → 解析 JSON → 返回建议列表"
- **命名规范**：Rust command `review_plan_bigrocks` / `review_get_bigrock_suggestions`（snake_case），前端 service `planBigRocks` / `getBigRockSuggestions`（camelCase），前端类型 `BigRockPlanItem` / `RoleBigRockSuggestions`（PascalCase），Tauri Event 无新增
- **错误处理**：Rust 端 LLM 失败/解析失败 → 返回空列表降级；前端端 service 调用失败 → hook 设 error 状态，UI 降级显示
- **serde 桥接**：`BigRockPlanItem` 和 `RoleBigRockSuggestions` 均派生 `Serialize + Deserialize` + `#[serde(rename_all = "camelCase")]`

### 前端 UI 规范

- **不新增组件文件**：重写 `WeeklyReviewModal.tsx` 内部实现，新增 hooks 和类型/服务函数
- **不修改 App.tsx**：现有 `WeeklyReviewModal` 调用方式（props 传入 `roles` / `onClose` / `initialPhase`）不变
- **视觉风格不变**：保持现有 Tailwind utility class 风格、圆角、间距、配色，仅替换数据源
- **空状态文案**：遵循 UX-DR18 约束 — 不显示"暂无数据"，使用有温度文案（如"本周复盘尚未生成"）
- **加载状态**：遵循 UX-DR19 约束 — 不使用全屏 loading/spinner，使用内联文案（如"正在思考建议..."）
- **SVG 柱状图**：使用纯 SVG 元素（`<svg>` / `<rect>` / `<text>`），不引入图表库，GPU 加速友好

### 反模式警告

- **不要**引入 Recharts / Chart.js 等图表库 — 架构明确要求自绘 SVG
- **不要**修改 App.tsx 中的 `WeeklyReviewModal` 调用方式 — props 签名不变
- **不要**修改 `review_generator.rs` 中已有的 `generate_review_if_needed` 函数 — 只新增 `generate_bigrock_suggestions` 函数
- **不要**在 `commands/` 层添加业务逻辑 — Command 只做参数解析 → 调 Service/DB → 返回结果
- **不要**在 `review_plan_bigrocks` 中同步调用 `task_classifier::classify_and_persist` — 必须 `tokio::spawn` 异步调用（参照 `commands/task.rs:36-48`）
- **不要**在 `review_plan_bigrocks` 中跳过大石头数量限制 — `db::tasks::create_task` 内部已校验（每 owner 最多 3 个），超限返回 `ValidationError`
- **不要**让 AI 建议加载失败阻塞用户 — 降级为空列表，用户仍可手动输入
- **不要**在 WeeklyReviewModal 中直接调用 `invoke` — 通过 service 层封装（`reviewService`）
- **不要**修改 `weekly_reviews` 表结构 — Story 6.4 已定义的表结构足够（`energy_trends` 和 `bigrock_status` JSON 字段已存储所需数据）
- **不要**修改 `reviewService.ts` 中已有的三个函数 — 只新增 `planBigRocks` 和 `getBigRockSuggestions`
- **不要**在 LLM prompt 中省略角色目标 — AC2 明确要求建议"基于角色目标"
- **不要**让 LLM 返回自然语言 — AI 建议必须返回 JSON 数组格式，便于前端解析匹配

### Project Structure Notes

新增文件：
- `src/hooks/useWeeklyReview.ts` — 复盘数据加载 hook
- `src/hooks/useBigRockPlanning.ts` — 大石头规划 hook（AI 建议 + 批量保存）
- `src/hooks/useWeeklyReview.test.ts` — hook 测试
- `src/hooks/useBigRockPlanning.test.ts` — hook 测试
- `src/components/modals/WeeklyReviewModal.test.tsx` — 组件测试

修改文件：
- `src-tauri/src/commands/review.rs` — 新增 `review_plan_bigrocks` + `review_get_bigrock_suggestions` command
- `src-tauri/src/services/review_generator.rs` — 新增 `generate_bigrock_suggestions` + `build_suggestion_prompt` + `RoleBigRockSuggestions` 结构体
- `src-tauri/src/lib.rs` — 注册 `review_plan_bigrocks` + `review_get_bigrock_suggestions` command
- `src/types/review.ts` — 新增 `BigRockPlanItem` + `RoleBigRockSuggestions` 类型
- `src/services/reviewService.ts` — 新增 `planBigRocks` + `getBigRockSuggestions` 函数
- `src/components/modals/WeeklyReviewModal.tsx` — 重写内部实现，替换所有 mock 数据

不修改文件：
- `src/App.tsx` — WeeklyReviewModal 调用方式不变
- `src-tauri/migrations/025_weekly_reviews.sql` — 表结构不变
- `src-tauri/src/models/weekly_review.rs` — 结构体不变
- `src-tauri/src/db/weekly_reviews.rs` — DB 操作不变
- `src-tauri/src/services/review_generator.rs` 中已有的 `generate_review_if_needed` — 不修改

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.5] — AC 原文
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6] — Epic 6 上下文（FR-16, FR-17, FR-18, UX-DR11）
- [Source: _bmad-output/planning-artifacts/architecture.md#Chart Library] — 自绘 SVG 能量趋势图（不引入图表库）
- [Source: _bmad-output/planning-artifacts/architecture.md#weekly_reviews] — `weekly_reviews` 表架构定义
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: _bmad-output/implementation-artifacts/6-4-weekly-review-scorecard.md] — Story 6.4 实现上下文（复盘生成服务 + DB + commands + 前端事件监听）
- [Source: _bmad-output/implementation-artifacts/6-3-big-rock-planning-reminder.md] — Story 6.3 实现上下文（bigrock:reminder 事件 + WeeklyReviewModal initialPhase prop）
- [Source: src-tauri/src/commands/review.rs] — 现有 review commands（新增 plan_bigrocks + get_bigrock_suggestions）
- [Source: src-tauri/src/services/review_generator.rs] — 复盘生成服务（新增 generate_bigrock_suggestions）
- [Source: src-tauri/src/services/review_generator.rs:468-523] — `call_llm` 函数（复用模式）
- [Source: src-tauri/src/services/agent_engine.rs:1928-1957] — `resolve_default_provider` 函数
- [Source: src-tauri/src/commands/task.rs:14-51] — `task_create` command（参照批量创建 + 异步分类模式）
- [Source: src-tauri/src/db/tasks.rs] — `create_task` 函数（含大石头数量限制校验）
- [Source: src-tauri/src/services/task_classifier.rs] — `classify_and_persist` 异步自动分类
- [Source: src-tauri/src/db/roles.rs] — `list_active_roles` 函数
- [Source: src-tauri/src/lib.rs:366-368] — 现有 review command 注册位置
- [Source: src/components/modals/WeeklyReviewModal.tsx] — 组件现状（126 行 mock 数据，需重写）
- [Source: src/App.tsx:380] — WeeklyReviewModal 渲染位置（不修改）
- [Source: src/App.tsx:41-42] — `isReviewOpen` + `reviewInitialPhase` state
- [Source: src/App.tsx:105-114] — `bigrock:reminder` 事件监听（打开 Modal plan 阶段）
- [Source: src/App.tsx:117-123] — `review:generated` 事件监听
- [Source: src/types/review.ts] — 现有类型（新增 BigRockPlanItem + RoleBigRockSuggestions）
- [Source: src/services/reviewService.ts] — 现有服务（新增 planBigRocks + getBigRockSuggestions）
- [Source: src/types/task.ts] — `CreateTaskInput` 类型（参照 isBigRock 字段）
- [Source: src/services/taskService.ts] — `taskService.create` 前端封装
- [Source: src/types/role.ts] — `Role` 类型（id / name / icon / color / goal / energy）
- [Source: src/services/roleService.ts] — `roleService.list()` 前端封装
- [Source: src/hooks/useTauriEvent.ts] — Tauri 事件监听 hook
- [Source: src/components/layout/Modal.tsx] — Modal 基础组件
- [Source: src/lib/roleIcons.ts] — 角色颜色白名单 `ROLE_COLORS` + `normalizeColorHex`

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

新增文件：
- `egosync-app/src/hooks/useWeeklyReview.ts` — 复盘数据加载 hook
- `egosync-app/src/hooks/useBigRockPlanning.ts` — 大石头规划 hook（AI 建议 + 批量保存）
- `egosync-app/src/hooks/useWeeklyReview.test.ts` — hook 测试
- `egosync-app/src/hooks/useBigRockPlanning.test.ts` — hook 测试
- `egosync-app/src/components/modals/WeeklyReviewModal.test.tsx` — 组件测试（已修改，非新增）

修改文件：
- `egosync-app/src-tauri/src/commands/review.rs` — 新增 `review_plan_bigrocks` + `review_get_bigrock_suggestions` command + 预校验逻辑
- `egosync-app/src-tauri/src/services/review_generator.rs` — 新增 `generate_bigrock_suggestions` + `build_suggestion_prompt` + `RoleBigRockSuggestions` 结构体
- `egosync-app/src-tauri/src/lib.rs` — 注册 `review_plan_bigrocks` + `review_get_bigrock_suggestions` command
- `egosync-app/src/types/review.ts` — 新增 `BigRockPlanItem` + `RoleBigRockSuggestions` 类型
- `egosync-app/src/services/reviewService.ts` — 新增 `planBigRocks` + `getBigRockSuggestions` 函数
- `egosync-app/src/components/modals/WeeklyReviewModal.tsx` — 重写内部实现，替换所有 mock 数据
- `egosync-app/src/components/modals/WeeklyReviewModal.test.tsx` — 更新组件测试
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — Story 6-5 状态更新

### Review Findings

_代码审查（2026-06-27，Amelia / bmad-code-review，审查未提交改动，full 模式）_

- [x] [Review][Patch] `review_plan_bigrocks` 批量创建非原子，超限时部分写入 — 已修复（`commands/review.rs:40-66`）：保存前按 `role_id` 聚合本次新增数量，逐角色用 `count_big_rocks_by_owner` 校验「已有 + 新增 ≤ 3」，任一超限整体返回 `ValidationError`，一个都不创建，避免半成品数据。（决策 1：先校验，超限不创建）
- [x] [Review][Patch] 跨月周复盘日期标签语义缺失 — 已修复（`WeeklyReviewModal.tsx:33-41`）：`formatDateRange` 跨月时结束日补显月份（"6月29日 - 7月5日"），同月保持 AC4 字面格式。（决策 2：跨月时补显月份）
- [x] [Review][Patch] 规划保存失败对用户静默 — 已修复（`WeeklyReviewModal.tsx:54,248-250`）：解构 `useBigRockPlanning` 的 `error` 为 `planError`，在规划阶段确认区上方渲染红色错误提示。
- [x] [Review][Patch] `loadSuggestions` 缺少卸载/取消保护 — 已修复（`useBigRockPlanning.ts:22-49,58-61`）：新增 `mountedRef`，在 then/catch/finally 及 savePlan 的 catch/finally 中守卫，组件卸载后不再 setState。
- [x] [Review][Patch] `roles` 引用变化会清空规划中输入 — 已修复（`WeeklyReviewModal.tsx:64-71`）：改为按 `roleIdsKey`（role id 集合）触发重建，并保留已存在角色的输入项，仅对新增角色补默认值。
