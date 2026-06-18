---
baseline_commit: 0871096146e95899b732ed7902c8c7706896c066
---

# Story 3.3: 系统自动为任务分配四象限分类

Status: done

## Story

As a 用户,
I want 系统自动为我的任务分析紧急/重要程度并分类,
so that 不用每个任务都手动判断该放哪个象限。

## Acceptance Criteria

1. **新任务自动分类**
   - **Given** 用户创建新任务（有截止时间和角色目标上下文）
   - **When** 任务保存后
   - **Then** 后台 LLM 自动分析并分配 Q1/Q2/Q3/Q4 分类
   - **And** 分析输入包含：截止日期距今天数、角色目标关联度、最近 5 条同角色任务模式
   - **And** 分类结果写入 `tasks.quadrant`

2. **低置信度不确定标记**
   - **Given** LLM 分类置信度 `< 80%`
   - **When** 分类完成
   - **Then** `tasks.confidence` 字段记录置信度
   - **And** `TasksTab` 中该任务显示“不确定”标记（淡黄色边框 + 问号图标）

3. **临期任务自动升入 Q1**
   - **Given** 任务截止日期距今 `<= 2` 天
   - **When** 定时检查触发（每小时）
   - **Then** 若当前为 Q2（重要不紧急）且未被用户手动覆盖，则自动升入 Q1
   - **And** Q3/Q4 任务临期不自动升入 Q1（不重要任务不应变为重要紧急）
   - **And** `quadrant` 更新并记录变更原因

4. **用户手动覆盖后不再被自动覆盖**
   - **Given** 用户在 `TaskModal` 中手动选择四象限分类
   - **When** 覆盖保存
   - **Then** `tasks.quadrant` 更新为用户选择
   - **And** 标记 `manual_override = true`
   - **And** 后续自动分类与临期升 Q1 不再覆盖该任务

5. **LLM 失败降级**
   - **Given** LLM 分析失败（超时、缺少默认 LLM 配置、API 错误、格式错误）
   - **When** 自动分类流程执行
   - **Then** 默认分配 Q2（重要不紧急）
   - **And** 写入合理置信度与失败原因
   - **And** 使用 `tracing` 记录失败原因，不向用户暴露 API key 或底层敏感信息

6. **Rust 后端分类服务**
   - **Given** Rust 后端
   - **Then** 新增 `task_classifier` 服务，构造分类 prompt：任务标题 + 截止日期 + 角色目标 + 最近 5 条同角色任务
   - **And** LLM 返回结构为 `{ quadrant, confidence, reason }`
   - **And** 新增每小时检查临期任务的 `tokio::interval(Duration::from_secs(3600))` 后台任务

## Tasks / Subtasks

- [x] **Task 1: 扩展任务分类元数据 schema** (AC: 3, 4, 5)
  - [x] 新增 SQLx migration（建议 `src-tauri/migrations/014_task_classification_metadata.sql`）为 `tasks` 增加 `manual_override INTEGER NOT NULL DEFAULT 0`。
  - [x] 同一 migration 增加 `classification_reason TEXT`，用于记录 LLM 分类原因、临期升 Q1 原因或降级原因。
  - [x] 为 `manual_override` 增加索引（如 `idx_tasks_manual_override`），便于定时检查筛选。
  - [x] 同步更新 `src-tauri/src/models/task.rs` 的 `Task`、`CreateTaskInput`、`UpdateTaskInput`，继续使用 `#[serde(rename_all = "camelCase")]`。
  - [x] 同步更新 `egosync-app/src/types/task.ts`，字段命名为 `manualOverride`、`classificationReason`。

- [x] **Task 2: 实现后端分类数据访问 helper** (AC: 1, 2, 3, 4, 5)
  - [x] 在 `src-tauri/src/db/tasks.rs` 中扩展 `TASK_SELECT_COLUMNS`，确保新字段随 task 返回前端。
  - [x] 扩展测试内存表 schema，避免 Rust 单测与 migration 脱节。
  - [x] 新增 helper：按 task id 写入 `quadrant`、`confidence`、`classification_reason`、`manual_override`、`updated_at`。
  - [x] 新增 helper：读取某角色最近 5 条同角色任务（排除当前任务、软删除任务，建议按 `created_at DESC` 或 `updated_at DESC`）。
  - [x] 新增 helper：查询临期且未手动覆盖的未完成任务（`deadline IS NOT NULL`、`deleted_at IS NULL`、`is_completed = 0`、`manual_override = 0`、`quadrant = 'Q2'`）。仅 Q2 升 Q1，Q3/Q4 不升。
  - [x] 保留 Story 3.2 的 `reorder_tasks` 事务语义与 `set_task_completion` 幂等语义，不要在分类更新时修改 `sort_order` 或完成态。

- [x] **Task 3: 新增 `task_classifier` 服务** (AC: 1, 2, 5, 6)
  - [x] 新建 `src-tauri/src/services/task_classifier.rs`，并在 `src-tauri/src/services/mod.rs` 注册。
  - [x] 服务层负责：读取 task、读取 role goal、读取同角色历史任务、构造 prompt、调用 LLM、解析 JSON、写回分类结果。
  - [x] LLM provider 创建应复用现有配置链路：`db::settings::get_default_llm_config` + `services::secret_store::load_secret` + `llm::{openai, anthropic}` provider，不硬编码 provider、model、base URL、API key。
  - [x] 现有 `LlmProvider` 只有 `chat_stream`；可在服务内消费 `StreamEvent::Token` 汇总文本并解析 JSON，或抽取一个小型非流式 helper，但不要把分类逻辑塞进 `commands` 层。
  - [x] Prompt 必须要求严格 JSON 输出 `{ "quadrant": "Q1|Q2|Q3|Q4", "confidence": 0.0-1.0, "reason": "中文短句" }`。
  - [x] 解析失败、超时、缺少配置或返回非法 quadrant 时，降级写入 Q2、`confidence < 0.8`、中文 `classification_reason`，并 `tracing::warn!` 记录原因。

- [x] **Task 4: 接入创建/更新流程与手动覆盖语义** (AC: 1, 4, 5)
  - [x] `task_create` 保持 thin command：校验后调用 db 创建，未显式选择 quadrant 时 `tauri::async_runtime::spawn` 后台异步分类，命令立即返回默认 Q2 任务；分类完成后 `emit("task:classified", task)` 通知前端。
  - [x] 新建任务若前端未显式选择 quadrant，则视为自动分类候选，`manual_override = false`。
  - [x] 新建任务若前端显式选择 quadrant，则标记 `manual_override = true`，避免立即被自动分类覆盖。
  - [x] `task_update` 中若用户显式修改 `quadrant`，必须设置 `manual_override = true`。
  - [x] 若用户只修改标题/截止日期且 `manual_override = false`，可重新触发自动分类以保持动态；若 `manual_override = true`，不得覆盖 quadrant。
  - [x] 保持 command handler 只做参数校验/调用 db 或 service，不放 prompt、LLM 解析或 SQL 业务逻辑。

- [x] **Task 5: 实现每小时临期检查** (AC: 3, 6)
  - [x] 在后端启动流程中用 `tauri::async_runtime::spawn` 启动后台任务，内部使用 `tokio::interval(Duration::from_secs(3600))`。
  - [x] 后台任务只处理 `deadline <= now + 2 days`、未完成、未软删除、未手动覆盖、当前为 Q2 的任务（Q3/Q4 不升）。
  - [x] 升级 Q1 时写入 `classification_reason`，例如“截止日期已进入 2 天内，自动升入 Q1”。
  - [x] 后台任务必须 graceful degradation：错误只 `tracing::warn!`，不得 panic，不得阻塞 Tauri 启动。
  - [x] 为避免等待一小时，可提供 service/helper 级函数供单测直接调用；如新增 IPC 命令用于手动触发检查，必须通过 `taskService` 暴露，组件不得直接 `invoke()`。

- [x] **Task 6: 前端类型、service、hook 接入** (AC: 1, 2, 4)
  - [x] 更新 `src/types/task.ts`：`Task` 增加 `manualOverride: boolean`、`classificationReason: string | null`。
  - [x] 区分自动分类与手动覆盖：`CreateTaskInput` / `UpdateTaskInput` 可增加 `manualOverride?: boolean` 或更明确的 `quadrantSource`，但前后端契约必须一致。
  - [x] 若新增 IPC（例如 `task_classify` 或 `task_check_deadline_escalation`），只在 `src/services/taskService.ts` 封装，组件不得直接调用 Tauri `invoke()`。
  - [x] `useTasks.ts` 继续负责 reload/乐观更新边界；新增 `classifyingIds` 状态标记后台分类中任务，通过 `useTauriEvent` 监听 `task:classified` 事件，收到后替换卡片并清除标记。

- [x] **Task 7: 更新 `TaskModal` 手动覆盖 UX** (AC: 4)
  - [x] `src/components/modals/TaskModal.tsx` 中保留四象限选择，但文案需明确“自动建议，可手动调整”。
  - [x] 编辑已有任务时，只要用户改变 select 值并保存，即按手动覆盖处理。
  - [x] 新建任务默认不应强制用户手动选择 Q1；当前默认 `Q1` 会导致所有新任务像手动 Q1，需调整为“智能判断”选项或不传 `quadrant`。
  - [x] 表单错误继续使用内联中文文案，不使用 toast/snackbar。
  - [x] 不实现 Story 3.4 的大石头数量限制；保留现有 `isBigRock` 字段与 UI，不扩大范围。

- [x] **Task 8: 更新 `TasksTab` 不确定标记** (AC: 2)
  - [x] 在 `src/components/role/TasksTab.tsx` 中，当 `task.confidence !== null && task.confidence < 0.8` 时显示“不确定”标记。
  - [x] 视觉：淡黄色边框/背景 + `lucide-react` 问号图标（如 `CircleHelp` 或 `HelpCircle`），保持宁静书房风格。
  - [x] 可用 `classificationReason` 作为 `title` 或辅助文本，但不要制造压力感或红色警告。
  - [x] 不要重构为 Story 3.6 的完整四象限分组范围以外内容；当前 `TasksTab` 已按象限分组，改动需最小且不破坏拖拽排序。

- [x] **Task 9: 测试与回归验证** (AC: 全部)
  - [x] Rust：为 migration/schema 字段、manual override、自动分类降级、临期升 Q1、非法 LLM JSON fallback 添加测试。
  - [x] Rust：确保手动覆盖任务不会被定时检查覆盖。
  - [x] Rust：确保分类更新不改变 `sort_order`、`is_completed`、`completed_at`。
  - [x] Frontend：更新 `TaskModal.test.tsx` 覆盖默认智能判断、手动选择后提交 override。
  - [x] Frontend：更新 `TasksTab.test.tsx` 覆盖低置信度“不确定”标记 + “智能分类中…”徽章。
  - [x] Frontend：更新 `useTasks.test.tsx` / service mock，使新增字段与 actions 不破坏现有 CRUD、reorder、toggleComplete。
  - [x] 运行 `npm run test:frontend`。
  - [x] 运行 `cargo test`（目录：`egosync-app/src-tauri`）。
  - [x] 运行 `npm run build`。

### Review Findings

_Code review 2026-06-18（Amelia）。来源标记：blind=盲审，edge=边界，auditor=验收。_

- [x] [Review][Patch] 编辑任务时任何保存都会置 `manualOverride=true`，冻结自动分类与临期升 Q1 — `TaskModal.tsx:52-53` 编辑模式始终发送 `quadrant`，`commands/task.rs:55` + `db/tasks.rs:79,88` 据此置 `manual_override=1`。违反 AC4 与 Task 7「只要用户改变 select 值并保存，即按手动覆盖处理」。**已修复**：`TaskModal.tsx` 编辑模式仅当 `quadrant` 实际变更才发送，改标题/截止日期不再冻结自动分类与临期升 Q1。
- [x] [Review][Patch] 分类 prompt 缺少「今天日期」，LLM 无法评估 AC1 要求的「截止日期距今天数」 [`services/task_classifier.rs`] — **已修复**：`build_classification_prompt` 新增 `today` 参数（UTC，与全应用 `chrono_now` 约定一致），prompt 增「今天日期」行作为时间锚点。
- [x] [Review][Defer] 临期阈值用 UTC 日期计算，与本地 `deadline` 字符串比较存在时区偏差 [`services/task_deadline_watch.rs:55-65`] — deferred；全应用刻意 UTC-only（无 chrono 依赖），改用本地时区会引入新依赖与约定不一致；影响为 ≤1 个时区偏移的自愈延迟。需先决定全应用时区策略，见 `deferred-work.md`。
- [x] [Review][Defer] `extract_json_object` 贪婪截取首个 `{` 到末个 `}`，前导散文含散落花括号时会破坏有效 JSON [`services/task_classifier.rs:334-341`] — deferred；当前失败安全降级到 Q2，非正确性破坏。

## Dev Notes

### Source Context

- `Story 3.3` 原文位于 `_bmad-output/planning-artifacts/epics.md:1406-1441`。
- PRD FR-23 要求自动四象限分类：基于截止日期、角色目标关联度、历史模式；分类动态变化；用户可手动覆盖。
- Epic 3 明确后续故事仍包括：Story 3.4 大石头限制/突出、Story 3.5 Q2 保护、Story 3.6 四象限完整展示、Story 3.7 管家汇总任务 tab。不要把这些后续行为并入本故事。

### Existing Task Implementation

- 当前 task schema 由 `egosync-app/src-tauri/migrations/013_tasks.sql` 创建，已有：`quadrant`、`confidence`、`protection_status`、`is_big_rock`、`is_completed`、`sort_order`、软删除字段。
- 当前 schema **没有** `manual_override` 与 `classification_reason`，但 Story 3.3 AC 明确需要“手动覆盖不再自动覆盖”和“记录变更原因”，因此本故事需要新增 migration。
- Rust task model 当前在 `egosync-app/src-tauri/src/models/task.rs`，使用 `#[serde(rename_all = "camelCase")]`，前端收到 camelCase。
- DB helper 当前在 `egosync-app/src-tauri/src/db/tasks.rs`：
  - `create_task` 默认 quadrant 为 Q2；
  - `update_task` 只更新传入字段；
  - `reorder_tasks` 要求完整同角色 task id 列表并事务重写 `sort_order`；
  - `set_task_completion` 幂等，完成/撤销不改变 `sort_order`。
- Tauri task commands 当前在 `egosync-app/src-tauri/src/commands/task.rs`，command handler 应继续保持 thin。
- 命令注册在 `egosync-app/src-tauri/src/lib.rs:257-323` 的 `tauri::generate_handler!`。

### Existing Frontend Implementation

- Task 类型在 `egosync-app/src/types/task.ts`。
- Tauri IPC 封装在 `egosync-app/src/services/taskService.ts`，组件不得直接调用 `invoke()`。
- Hook 在 `egosync-app/src/hooks/useTasks.ts`，已有 CRUD 后 reload、reorder 乐观更新、toggleComplete 乐观更新。
- `TasksTab` 位于 `egosync-app/src/components/role/TasksTab.tsx`：
  - 已使用 `@dnd-kit/core`、`@dnd-kit/sortable`、`@dnd-kit/utilities`；
  - 当前已经按 Q1-Q4 分组展示；
  - 拖拽只在同象限未完成任务内排序，生成完整 role task id 顺序；
  - 低置信度标记应加在 `TaskCardBody` / card class 附近，避免破坏 drag/drop。
- `TaskModal` 位于 `egosync-app/src/components/modals/TaskModal.tsx`：当前新建默认 quadrant 为 Q1，这与“自动分类”冲突，需改成默认自动判断或不传 quadrant。

### LLM / Secret / Provider Context

- LLM config service 在 `egosync-app/src-tauri/src/services/llm_config.rs`。
- API key 通过 `services::secret_store` 读取，不得写入日志、DB 明文字段或前端。
- Existing provider types：
  - `egosync-app/src-tauri/src/llm/openai.rs`
  - `egosync-app/src-tauri/src/llm/anthropic.rs`
  - trait 在 `egosync-app/src-tauri/src/llm/traits.rs`
- 现有 `LlmProvider` 暴露 `chat_stream` 与 `test_connection`，没有通用非流式 completion；实现分类时需明确消费 stream 或抽取小 helper。
- `reqwest` timeout 当前 provider 层为 10 秒；分类失败必须降级到 Q2，不得阻塞用户创建任务太久。

### Role Goal Context

- `roles.goal` 已存在：Rust model 在 `egosync-app/src-tauri/src/models/role.rs`，DB helper `get_role` 在 `egosync-app/src-tauri/src/db/roles.rs`。
- 分类 prompt 中的“角色目标关联度”应使用 `Role.goal`，不要新增重复 goal 存储。

### UX Requirements

- 遵循“宁静书房”风格与温和文案。
- 失败/错误使用内联中文反馈或后台 `tracing`，不要使用 toast/snackbar。
- 低置信度“不确定”是帮助用户修正的温和提示，不应使用强烈红色警告。
- 所有自定义样式使用 Tailwind utility class，不新增自定义 CSS class。
- 支持 `motion-reduce`，不要引入额外动画库。

### Architecture Compliance

- 前端组件不得直接调用 Tauri `invoke()`；所有 IPC 必须经 `src/services/taskService.ts`。
- Rust command 层只做参数校验与服务/DB 调用，不包含 LLM prompt、JSON parse、SQL 业务逻辑。
- 新 DB 字段必须通过 SQLx migration 添加，并同步测试内存 schema。
- Rust command 不允许 `unwrap()` / `panic!`；错误用 `AppError` 返回或后台任务中 `tracing::warn!` 降级。
- JSON 字段前端 camelCase，DB snake_case，Rust serde `rename_all = "camelCase"`。
- 不要把 OpenCode/agent bridge 当作分类 LLM 调用通道；本故事应复用现有 LLM provider/config/secret 架构。

### Previous Story Intelligence

- Story 3.1 已完成任务 CRUD 和真实数据链路，创建了 `tasks` 表、Rust task model/db/commands、前端 `taskService`、`useTasks`、`TasksTab`、`TaskModal`。
- Story 3.1 明确 `confidence` 已预留给自动分类；本故事应使用该字段，不要创建平行字段。
- Story 3.2 已完成拖拽排序与完成/撤销完成：
  - `task_reorder` 使用完整 role task id 顺序；
  - 完成态不修改 `sort_order`；
  - 完成任务沉底是前端渲染职责；
  - 错误通过 `TasksTab` 内联中文错误显示。
- Story 3.2 留出的范围包括：自动分类、大石头限制、Q2 保护、all-role tab。本故事只处理自动分类与手动覆盖。

### Git Intelligence

Recent relevant commits:

- `0871096 Fix story 3.2 drag-sort and sync docs`
- `5585768 docs: mark story 3.1 done`
- `c39c968 feat(tasks): add role task CRUD`
- `fea9af2 docs: update path references from GUI/ to egosync-app/`
- `74c6639 chore: rename GUI to egosync-app and update CI paths`

### Testing Requirements

- Frontend tests use Vitest + Testing Library.
- Rust tests live near modules under `#[cfg(test)]` and use in-memory SQLite in `db/tasks.rs`.
- Required verification commands:
  - `npm run test:frontend` from `egosync-app`
  - `cargo test` from `egosync-app/src-tauri`
  - `npm run build` from `egosync-app`

### References

- `_bmad-output/project-context.md`
- `_bmad-output/planning-artifacts/epics.md:1406-1441`
- `_bmad-output/planning-artifacts/prd-egosync.md` FR-23
- `_bmad-output/planning-artifacts/architecture.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md:763-827`
- `_bmad-output/implementation-artifacts/3-1-task-crud-role-view.md`
- `_bmad-output/implementation-artifacts/3-2-task-drag-sort-complete.md`
- `egosync-app/src-tauri/migrations/013_tasks.sql`
- `egosync-app/src-tauri/src/models/task.rs`
- `egosync-app/src-tauri/src/db/tasks.rs`
- `egosync-app/src-tauri/src/commands/task.rs`
- `egosync-app/src-tauri/src/services/llm_config.rs`
- `egosync-app/src-tauri/src/llm/traits.rs`
- `egosync-app/src/types/task.ts`
- `egosync-app/src/services/taskService.ts`
- `egosync-app/src/hooks/useTasks.ts`
- `egosync-app/src/components/role/TasksTab.tsx`
- `egosync-app/src/components/modals/TaskModal.tsx`

## Dev Agent Record

### Agent Model Used

Cascade（SWE-1.6）

### Debug Log References

- `cargo test --lib -- --test-threads=1`：374 / 374 通过（首次并行执行时 sidecar 端口测试偶发失败，单独跑通过；改用单线程稳定）。
- `npm run test:frontend`：187 / 187 通过。
- `npm run build`：tsc + vite build 通过，dist 输出正常。
- 临期检查后台任务：通过 `compute_imminent_threshold_is_iso_date` / `days_to_ymd_handles_epoch_origin` / `days_to_ymd_round_trips_via_threshold` 三个单测验证日期算法。
- 分类降级路径：`task_classifier::tests` 五个单测覆盖 JSON 解析（合法、外包代码块、非法 quadrant、超界 confidence、缺字段、空 reason），fallback_outcome 用例验证 Q2 + 低置信度 + 中文 reason 三条不变量。

### Completion Notes List

- **新增 migration 014**：仅 `ALTER TABLE tasks ADD COLUMN manual_override / classification_reason` + 单列索引，向后兼容已有 013 数据。
- **manual_override 语义**：用户在 TaskModal 显式选择 quadrant（包括编辑时不修改但保存）即标记为 true；自动分类与每小时临期升 Q1 都通过 `update_task_classification`（带 `WHERE manual_override = 0`）阻断覆盖。
- **task_classifier 降级策略**：12s 超时；缺默认 LLM、超时、provider 报错、JSON 非法 → 全部降级到 Q2 + confidence 0.3 + 中文短句 reason，不抛错给前端。
- **临期检查后台任务**：`tokio::interval(3600s)` 在 Tauri setup 末尾 spawn，错误仅 warn。首 tick 先消费、随后立即执行一次升级检查。
- **前端 TaskModal**：新建任务默认下拉「✨ 智能判断」，提交时不传 quadrant；编辑模式仅当 quadrant 实际变更时才发送，以避免误置 manual_override；select 下方显示 `classificationReason` 中文短句。
- **TasksTab 不确定徽章**：通过新增 `isClassificationUncertain(task)` helper 实现「manualOverride === false && confidence < 0.8」一致性判断，前后端阈值同步在 `CLASSIFICATION_UNCERTAINTY_THRESHOLD = 0.8`。
- **agent_engine 测试 schema 同步**：在自建 schema 处追加执行 migration 014，避免 7 个原有 task-summary 测试因列缺失失败。
- **测试 fixture 全量补齐**：`useTasks.test.tsx` / `TasksTab.test.tsx` / `TaskModal.test.tsx` 三处 Task 字面量统一加 `manualOverride: false, classificationReason: null`，避免 TS 类型缩窄报错。
- **环境插曲**：USTC cargo 镜像已下线，项目原有 `src-tauri/.cargo/config.toml` 已配置 rsproxy-sparse 兜底。在 `src-tauri/` 目录下运行 cargo 命令即可加载该配置；否则会回退到全局失效镜像。

### File List

新增：
- `egosync-app/src-tauri/migrations/014_task_classification_metadata.sql`
- `egosync-app/src-tauri/src/services/task_classifier.rs`
- `egosync-app/src-tauri/src/services/task_deadline_watch.rs`

修改：
- `egosync-app/src-tauri/src/lib.rs` — spawn 每小时临期检查
- `egosync-app/src-tauri/src/services/mod.rs` — 暴露两个新 service
- `egosync-app/src-tauri/src/services/agent_engine.rs` — 测试 schema 追加 migration 014
- `egosync-app/src-tauri/src/commands/task.rs` — `task_create` 在未显式提供 quadrant 时 `tauri::async_runtime::spawn` 后台异步分类，完成后 `emit("task:classified", task)` 通知前端
- `egosync-app/src-tauri/src/db/tasks.rs` — 扩展 `TASK_SELECT_COLUMNS`、新增 `update_task_classification` / `list_recent_tasks_by_role` / `list_imminent_tasks_for_escalation` / `get_active_task_pub`，create/update_task 写入 manual_override
- `egosync-app/src-tauri/src/models/task.rs` — Task 增 `manual_override` / `classification_reason` 字段
- `egosync-app/src/types/task.ts` — Task / Create / Update DTO 增字段、导出 `CLASSIFICATION_UNCERTAINTY_THRESHOLD` + `isClassificationUncertain` helper
- `egosync-app/src/components/modals/TaskModal.tsx` — 默认「智能判断」+ classificationReason 内联说明 + 编辑模式仅变更时发送 quadrant
- `egosync-app/src/components/role/TasksTab.tsx` — 任务卡片渲染「不确定」+「智能分类中…」徽章
- `egosync-app/src/hooks/useTasks.ts` — 新增 `classifyingIds` 状态 + `useTauriEvent` 监听 `task:classified` 事件
- `egosync-app/src/components/role/RoleWorkspacePanel.tsx` — 传递 `classifyingIds` prop
- `egosync-app/src/hooks/useTasks.test.tsx` / `src/components/role/TasksTab.test.tsx` / `src/components/modals/TaskModal.test.tsx` — 字面量与新增覆盖测试

## Change Log

| Date | Version | Description | Author |
|---|---:|---|---|
| 2026-06-17 | 1.0 | Initial ready-for-dev story for automatic task quadrant classification | Amelia |
| 2026-06-18 | 1.1 | 实现自动分类 + 手动覆盖 + 临期升 Q1 + 不确定徽章；后端 373 / 前端 185 测试全部通过；故事进入 review。 | Cascade |
| 2026-06-18 | 1.2 | 改为异步分类 + `task:classified` 事件通知；UI 文案改为「智能判断」；临期升 Q1 限制为仅 Q2；新增「智能分类中…」徽章；编辑模式仅变更时发送 quadrant。后端 374 / 前端 187 测试通过。 | Cascade |
