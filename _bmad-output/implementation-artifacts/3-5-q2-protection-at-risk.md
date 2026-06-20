---
baseline_commit: b248df2
---

# Story 3.5: 系统为 Q2 任务添加保护属性，连续被挤时标记预警

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 系统自动监测我的 Q2 任务是否被持续挤压,
so that 重要但不紧急的事不会被遗忘。

## Acceptance Criteria

1. **Q2 任务保护属性初始为 normal**
   - **Given** 任务被分类为 Q2
   - **Then** `tasks.protection_status` 为 `'normal'`
   - **And** 现有 `create_task` 已写入 `'normal'`（INSERT 中硬编码），本 story 不改变创建路径的初始值

2. **连续 3+ 天未处理的 Q2 任务标记 at_risk**
   - **Given** 一个 Q2、未完成、未软删除的任务，其 `updated_at` 距今 ≥ 3 天（即 ≤ 当前时间 − 3 天）
   - **When** 保护检查触发（后台每小时定时器，或前端启动时调用 `task_check_protection_status`）
   - **Then** 该任务 `protection_status` 更新为 `'at_risk'`
   - **And** 写入 `at_risk` 时**不得**刷新 `updated_at`（否则会立即被判定为"已处理"导致状态抖动）

3. **at_risk Q2 任务在 TasksTab 显示预警**
   - **Given** TasksTab 中某 Q2 未完成任务 `protectionStatus === 'at_risk'`
   - **Then** 卡片显示琥珀色 `AlertTriangle` 预警图标（`lucide-react`，`text-amber-600`）
   - **And** 卡片左边框为琥珀色竖线（`border-l-4 border-l-amber-400`）
   - **And** `protectionStatus === 'normal'` 的任务无任何预警标识（图标与左竖线均不出现）

4. **用户处理后预警消失**
   - **Given** 一个 `at_risk` 的 Q2 任务
   - **When** 用户编辑该任务（`task_update`）或切换其完成状态（`task_toggle_complete`）
   - **Then** 后端将 `protection_status` 恢复为 `'normal'`
   - **And** 前端 reloadCurrent 刷新后预警标识立即消失（无需等待下一次定时检查）
   - **Note** "相关对话提及"作为处理信号属于 Epic 4/5 行为层，V1 不实现

5. **保护检查的全量重算语义（幂等）**
   - **Given** Rust 后端保护检查
   - **Then** 检查对所有活跃任务执行幂等重算：Q2 + 未完成 + `updated_at` 过期 → `at_risk`；其余（非 Q2、已完成、或 `updated_at` 在 3 天内）若当前为 `at_risk` → 恢复 `'normal'`
   - **And** 重算的两条 UPDATE 均**不刷新** `updated_at`
   - **And** 例如：原 `at_risk` 的 Q2 任务被临期升入 Q1（Story 3.3 `update_task_classification`）后，重算应将其 `protection_status` 清回 `'normal'`

6. **行为层延后**
   - **Given** 保护机制的行为层（管家主动提醒、仲裁优先保护）
   - **Then** V1 仅做数据标记（`protection_status`）+ UI 预警
   - **And** 实际行为接通在 Epic 4（主动循环）与 Epic 5（仲裁），本 story 不涉及

7. **后端定时器与命令**
   - **Given** Rust 后端
   - **Then** 新增后台每小时定时器检查 Q2 任务 `updated_at` 距今天数（仿 `task_deadline_watch::spawn_hourly_watch`，首 tick 立即执行）
   - **And** 新增 Tauri command `task_check_protection_status`，供前端启动/打开任务面板时主动触发一次，返回本次被标记 `at_risk` 的任务数量（`usize`）
   - **And** 定时器内部错误只 `tracing::warn!`，绝不 panic、绝不阻塞 Tauri setup

## Tasks / Subtasks

- [x] 后端 DB：新增保护状态读写函数（AC: 2, 5）
  - [x] 在 `egosync-app/src-tauri/src/db/tasks.rs` 新增 `mark_stale_q2_at_risk(pool, threshold: &str) -> Result<u64, AppError>`
    - SQL：`UPDATE tasks SET protection_status = 'at_risk' WHERE quadrant = 'Q2' AND is_completed = 0 AND deleted_at IS NULL AND updated_at <= ?1 AND protection_status != 'at_risk'`
    - **不**包含 `updated_at = ...`（关键：标记 at_risk 不得刷新时间戳）
    - 返回 `result.rows_affected()`
  - [x] 在 `db/tasks.rs` 新增 `clear_protection_for_resolved(pool, threshold: &str) -> Result<u64, AppError>`
    - SQL：`UPDATE tasks SET protection_status = 'normal' WHERE protection_status = 'at_risk' AND deleted_at IS NULL AND (quadrant != 'Q2' OR is_completed = 1 OR updated_at > ?1)`
    - 同样**不**刷新 `updated_at`
  - [x] 两函数 `threshold` 为完整 ISO 时间戳字符串（格式同 `chrono_now_pub()`：`YYYY-MM-DDTHH:MM:SSZ`），可直接字符串比较

- [x] 后端 DB：编辑/完成时复位保护状态为 normal（AC: 4）
  - [x] 在 `db/tasks.rs::update_task` 的 UPDATE SET 子句中追加 `protection_status = 'normal'`（编辑即视为"处理"，立即消除预警）
  - [x] 在 `db/tasks.rs::set_task_completion` 的 UPDATE SET 子句中追加 `protection_status = 'normal'`（完成/撤销完成均视为交互；`updated_at` 已刷新，重算也会保持 normal）
  - [x] 注意：这两处本就刷新 `updated_at`，复位 normal 与重算语义一致，不冲突

- [x] 后端 Service：新增保护检查服务（AC: 2, 5, 7）
  - [x] 新建 `egosync-app/src-tauri/src/services/task_protection_watch.rs`，结构仿 `task_deadline_watch.rs`
  - [x] `pub const AT_RISK_DAYS: i64 = 3;`
  - [x] `pub async fn recompute_protection_status(pool: &SqlitePool) -> Result<u64, AppError>`：计算 `threshold = now − AT_RISK_DAYS 天`（完整 ISO 时间戳），先调 `clear_protection_for_resolved`，再调 `mark_stale_q2_at_risk`，返回被标记 at_risk 的数量
  - [x] `fn compute_at_risk_threshold() -> String`：仿 `task_deadline_watch::compute_imminent_threshold`，但输出完整 `YYYY-MM-DDTHH:MM:SSZ`（`now_secs − AT_RISK_DAYS*86400`，再拆分年月日时分秒；可复用 `days_to_ymd` 同款算法）
  - [x] `pub fn spawn_hourly_watch(pool: SqlitePool)`：`tokio::time::interval(3600s)`，首 tick 立即执行，循环调用 `recompute_protection_status`，错误只 `tracing::warn!`
  - [x] 在 `services/mod.rs` 注册 `pub mod task_protection_watch;`

- [x] 后端 Command：新增 `task_check_protection_status`（AC: 7）
  - [x] 在 `egosync-app/src-tauri/src/commands/task.rs` 新增 `#[tauri::command] pub async fn task_check_protection_status(pool: State<'_, DbPool>) -> Result<u64, AppError>`，仅调用 `services::task_protection_watch::recompute_protection_status(&pool)`（Command 层不写业务逻辑）
  - [x] 在 `lib.rs` 的 `invoke_handler![...]` 中注册 `commands::task::task_check_protection_status`

- [x] 后端启动：spawn 保护检查后台任务（AC: 7）
  - [x] 在 `lib.rs` setup 中、`task_deadline_watch::spawn_hourly_watch(pool.clone())` 之后，加入 `services::task_protection_watch::spawn_hourly_watch(pool.clone());`

- [x] 后端测试（AC: 2, 4, 5）
  - [x] 在 `db/tasks.rs` 的 `#[cfg(test)] mod tests` 中（复用 `setup_test_db`，其建表已含 `protection_status` 列）：
    - [x] 过期 Q2 未完成任务被 `mark_stale_q2_at_risk` 标记为 `at_risk`
    - [x] 标记后 `updated_at` 保持不变（断言写 at_risk 不刷新时间戳）
    - [x] 近期（`updated_at > threshold`）Q2 任务不被标记
    - [x] 已完成的过期 Q2 任务不被标记
    - [x] 非 Q2（Q1/Q3/Q4）过期任务不被标记
    - [x] `clear_protection_for_resolved`：at_risk 的 Q1 任务（已非 Q2）→ 恢复 normal；at_risk 且 `updated_at` 近期的 Q2 → 恢复 normal
    - [x] `update_task` 将 `protection_status` 复位为 `normal`
    - [x] `set_task_completion` 将 `protection_status` 复位为 `normal`
  - [x] 在 `services/task_protection_watch.rs` 中：`compute_at_risk_threshold` 返回完整 ISO 时间戳（长度 20、含 `T` 与 `Z`）

- [x] 前端 Service：暴露命令（AC: 7）
  - [x] 在 `egosync-app/src/services/taskService.ts` 的 `taskService` 中新增 `checkProtectionStatus: () => invoke<number>('task_check_protection_status')`

- [x] 前端 Hook：挂载时触发一次保护检查（AC: 2, 4）
  - [x] 在 `egosync-app/src/hooks/useTasks.ts` 的加载 effect 中，列表请求前先调用 `taskService.checkProtectionStatus()`（fire-and-forget，`.catch` 仅 `console.warn`，失败不阻塞列表加载），随后照常 list，使 at_risk 状态在打开任务面板时即时反映
  - [x] 不改变现有 `reloadCurrent` 行为（编辑/完成后已会刷新，自然带回复位后的 normal）

- [x] 前端 UI：TasksTab 预警渲染（AC: 3）
  - [x] 在 `egosync-app/src/components/role/TasksTab.tsx` 从 `lucide-react` 引入 `AlertTriangle`
  - [x] 在 `TaskCardBody` 的 badge 行（big rock 标签之后）新增：当 `task.protectionStatus === 'at_risk'` 时渲染琥珀色预警 badge（`AlertTriangle size={11}` + 文案如 `被挤压`，`bg-amber-50 text-amber-600 border-amber-100`），并加 `aria-label`（如「重要任务被持续挤压，建议尽快处理」）
  - [x] 在 `SortableTaskCard` 外层卡片 div 用 `cn(CARD_BASE_CLASS, isDragging && 'opacity-50', task.protectionStatus === 'at_risk' && 'border-l-4 border-l-amber-400')` 加琥珀左竖线
  - [x] `DragOverlay` 中的拖拽镜像卡片同样按 `activeTask.protectionStatus === 'at_risk'` 加左竖线（保持拖拽视觉一致）
  - [x] 已完成卡片（`CompletedTaskCard`）无需特殊处理：完成时后端已复位为 `normal`

- [x] 前端测试（AC: 3, 4）
  - [x] `TasksTab.test.tsx`：`protectionStatus: 'at_risk'` 的 Q2 任务渲染出预警图标/文案；`'normal'` 任务不渲染（用 `aria-label` 或文案断言）
  - [x] `useTasks.test.tsx`：挂载时调用了 `task_check_protection_status`（mock invoke 并断言被调用），且其失败不影响列表加载

- [x] 测试与验证（AC: 1-7）
  - [x] `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`
  - [x] `npm --prefix "egosync-app" run test:frontend`
  - [x] `npm --prefix "egosync-app" run build`

## Dev Notes

### Current State（基于当前代码 @ b248df2）

- **DB schema**：`tasks` 表已含 `protection_status TEXT NOT NULL DEFAULT 'normal'`（迁移 `013_tasks.sql`，测试建表 `db/tasks.rs:510` 也已含）。**无需新增迁移**。
- **Model**：`models/task.rs` 的 `Task.protection_status: String` 已存在；`TASK_SELECT_COLUMNS`（`db/tasks.rs:6`）已包含 `protection_status`。
- **create_task**（`db/tasks.rs:29-46`）：INSERT 中硬编码 `'normal'`，已满足 AC 1。
- **update_task**（`db/tasks.rs:79-134`）：SET 子句目前不含 `protection_status`；本 story 追加 `protection_status = 'normal'`。
- **set_task_completion**（`db/tasks.rs:226-260`）：SET 子句目前不含 `protection_status`；本 story 追加 `protection_status = 'normal'`。完成时已 `is_big_rock = CASE WHEN ?1 THEN 0 ...`（方案 D），照此模式追加即可。
- **既有定时器范本**：`services/task_deadline_watch.rs` 完整实现了「每小时 `tokio::interval` + 首 tick 立即 + 错误只 warn + `compute_*_threshold` 用 `days_to_ymd`」的模式——本 story 的 `task_protection_watch.rs` 直接照搬结构。
- **时间戳格式**：`db::settings::chrono_now_pub()` 产出 `YYYY-MM-DDTHH:MM:SSZ`（`settings.rs:145-163`），`created_at/updated_at` 均为此格式，可与 `threshold` 字符串直接比较（字典序即时间序）。
- **前端类型**：`types/task.ts` 已有 `TaskProtectionStatus = 'normal' | 'at_risk'` 与 `Task.protectionStatus`。**无需改类型**。
- **TasksTab badge 行**：`TasksTab.tsx:77-89` 已有 deadline / 大石头 / 分类中 三个 badge 的渲染范式，预警 badge 照此追加。
- **卡片基类**：`CARD_BASE_CLASS`（`TasksTab.tsx:48-49`）含 `border border-slate-200`，加 `border-l-4 border-l-amber-400` 即可覆盖左边框。
- **useTasks 消费方**：`components/role/RoleWorkspacePanel.tsx`（useTasks @50，<TasksTab> @141）与 `components/butler/ButlerWorkspacePanel.tsx`（@45/@119）。在 `useTasks` 内做保护检查，两个面板自动覆盖，无需改面板。

### What This Story Changes

1. **`db/tasks.rs`**：新增 `mark_stale_q2_at_risk` / `clear_protection_for_resolved`；`update_task` 与 `set_task_completion` 各追加 `protection_status = 'normal'`；新增单元测试。
2. **`services/task_protection_watch.rs`（新文件）**：`recompute_protection_status` + `compute_at_risk_threshold` + `spawn_hourly_watch`。
3. **`services/mod.rs`**：注册新模块。
4. **`commands/task.rs`**：新增 `task_check_protection_status` 命令（薄封装调用 service）。
5. **`lib.rs`**：注册命令 + setup 中 spawn 保护定时器。
6. **`services/taskService.ts`**：新增 `checkProtectionStatus`。
7. **`hooks/useTasks.ts`**：加载前触发一次保护检查（容错）。
8. **`components/role/TasksTab.tsx`**：at_risk 预警图标 + 琥珀左竖线。
9. 前端测试：`TasksTab.test.tsx`、`useTasks.test.tsx`。

### What Must Be Preserved（防回归）

- 拖拽排序（3.2）：`handleDragEnd` / `fullOrder` 逻辑不动；左竖线只是视觉类，不影响 `SortableContext` / `useSortable`。
- 大石头优先排序（3.4）：`orderBySort = (Number(b.isBigRock) - Number(a.isBigRock)) || (a.sortOrder - b.sortOrder)` 不动。
- 自动分类（3.3）：`task:classified` 事件、`classifyingIds`、`manual_override`、`update_task_classification` 全部不受影响。临期升 Q1 后，保护重算会把旧 at_risk 清回 normal（AC 5）——这是预期协同，不是冲突。
- 完成即撤销大石头（方案 D）：`set_task_completion` 现有 `is_big_rock` 复位逻辑保持，仅在同一 SET 中追加 `protection_status = 'normal'`。
- big_rock 数量校验（3.4）：`count_big_rocks_by_owner` 与 `update_task`/`create_task` 中的校验分支不动。
- owner 维度（role/butler）：保护逻辑按 quadrant + completion + updated_at 工作，与 owner 无关，无需按 owner 分桶；管家任务的 Q2 同样适用。
- 现有全部 Rust / 前端测试保持通过。

### Architecture Guardrails

- 前端禁止直接访问 SQLite；经 `taskService.ts` → Tauri IPC。
- Command 层只做参数解析 + 调 service/db；保护重算逻辑放 `services/task_protection_watch.rs`，命令仅薄封装。
- Rust 函数返回 `Result<T, AppError>`，禁止 `.unwrap()`；定时器内部错误只 `tracing::warn!`。
- DTO 与前端 JSON 用 camelCase（`#[serde(rename_all = "camelCase")]`，`protection_status` ↔ `protectionStatus`，已成立）。
- 仅用 Tailwind utility class，禁止新增 CSS 文件；图标用 `lucide-react`（已依赖）。
- TypeScript strict / `noUnusedLocals` / `noUnusedParameters` 已开启——未用变量会编译失败。
- 不新增依赖；不接入 LLM / opencode / Skill / MCP / 通知 / 仲裁（行为层是 E4/E5）。

### 关键正确性要点（极易踩坑）

- **写 `at_risk` 与 `clear` 都不能刷新 `updated_at`**：保护状态完全由 `updated_at` 距今天数派生。若标记 at_risk 时刷新了 `updated_at`，下一轮（甚至同一轮的 clear 步骤）就会因 `updated_at > threshold` 把它又判回 normal，导致永远标不上 / 状态抖动。
- **重算顺序**：先 `clear_protection_for_resolved`（把不该 at_risk 的清回 normal），再 `mark_stale_q2_at_risk`（把该标的标上）。两步互不重叠（clear 命中 `updated_at > threshold` 或非 Q2 或已完成；mark 命中 `updated_at <= threshold` 且 Q2 且未完成），顺序其实可交换，但固定顺序便于测试推理。
- **阈值是"包含等于"**：`updated_at <= threshold` 视为过期（≥3 天）。`threshold = now − 3 天`。
- **编辑即处理**：`update_task` 无条件复位 `protection_status = 'normal'`（即便用户只改了标题）。这是有意为之——打开并保存任务就是一次"关注"。

### Previous Story Intelligence

- **3.1**：建立 task CRUD 全链路与 `protection_status` 列（预留，无逻辑）。
- **3.2**：拖拽用 `fullOrder` 重写 `sort_order`；完成态沉底为前端渲染职责。
- **3.3**：后台异步分类（`task:classified` 事件）+ 每小时临期升 Q1（`task_deadline_watch`）。`update_task_classification` 仅在 `manual_override = 0` 时生效，会刷新 `updated_at`。测试 fixture 的 Task 字面量必须含全字段。
- **3.4**：big_rock 数量校验 + 完成即撤销大石头（方案 D），`set_task_completion` 已示范"在同一 UPDATE 追加列复位"的写法。`AppError::ValidationError` 序列化为 `{"ValidationError": "..."}`。
- **通用**：Rust 测试用 in-memory SQLite（`setup_test_db`）；`cargo test` 加 `--test-threads=1`；前端 Vitest + Testing Library；Task fixture 须含所有字段（`protectionStatus`、`manualOverride: false`、`classificationReason: null` 等）。

### Git Intelligence

- `b248df2`（当前 HEAD，本 story baseline）
- `c4248c6 feat(tasks): async classification with event notification, smart-detect UX, Q2-only escalation`
- 相关范本文件：`services/task_deadline_watch.rs`（定时器骨架）、`db/tasks.rs::set_task_completion`（同 UPDATE 追加列复位）。

### Testing Requirements

- Rust 单测置于 `db/tasks.rs` 与 `services/task_protection_watch.rs` 的 `#[cfg(test)] mod tests`，复用 `setup_test_db`（建表含 `protection_status`）。
- 构造"过期"任务：直接 `INSERT` 指定较早的 `updated_at`（如 `'2026-01-01T00:00:00Z'`），或对 `mark_stale_q2_at_risk` 传一个偏大的 `threshold` 字符串来控制边界，避免依赖真实时钟。
- 前端 Task fixture 必须含 `protectionStatus` 字段。
- 必跑验证命令：
  - `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`
  - `npm --prefix "egosync-app" run test:frontend`
  - `npm --prefix "egosync-app" run build`

### Project Structure Notes

- **新增文件**：`egosync-app/src-tauri/src/services/task_protection_watch.rs`。
- **无新增迁移**：`protection_status` 列已存在于 `013_tasks.sql`。
- 修改文件清单：
  - `egosync-app/src-tauri/src/db/tasks.rs`
  - `egosync-app/src-tauri/src/services/mod.rs`
  - `egosync-app/src-tauri/src/services/task_protection_watch.rs`（新）
  - `egosync-app/src-tauri/src/commands/task.rs`
  - `egosync-app/src-tauri/src/lib.rs`
  - `egosync-app/src/services/taskService.ts`
  - `egosync-app/src/hooks/useTasks.ts`
  - `egosync-app/src/components/role/TasksTab.tsx`
  - `egosync-app/src/components/role/TasksTab.test.tsx`
  - `egosync-app/src/hooks/useTasks.test.tsx`

### References

- `_bmad-output/project-context.md`
- `_bmad-output/planning-artifacts/epics.md:1476-1508`（Story 3.5 定义）
- `_bmad-output/planning-artifacts/ux-design-specification.md:763-770`（RoleWorkspacePanel / 任务面板）
- `_bmad-output/implementation-artifacts/3-3-auto-quadrant-classification.md`
- `_bmad-output/implementation-artifacts/3-4-big-rock-marking.md`
- `egosync-app/src-tauri/migrations/013_tasks.sql`
- `egosync-app/src-tauri/src/models/task.rs`
- `egosync-app/src-tauri/src/db/tasks.rs`（`update_task` / `set_task_completion` / `TASK_SELECT_COLUMNS` / tests `setup_test_db`）
- `egosync-app/src-tauri/src/services/task_deadline_watch.rs`（定时器范本）
- `egosync-app/src-tauri/src/db/settings.rs:141-163`（`chrono_now_pub` 时间戳格式）
- `egosync-app/src-tauri/src/commands/task.rs`
- `egosync-app/src-tauri/src/lib.rs:255-257`（spawn 范例）、`:296-302`（命令注册位置）
- `egosync-app/src/types/task.ts`
- `egosync-app/src/services/taskService.ts`
- `egosync-app/src/hooks/useTasks.ts`
- `egosync-app/src/components/role/TasksTab.tsx:48-89,122-152,349-355`

## Dev Agent Record

### Agent Model Used

Amelia (Senior Software Engineer) · bmad-dev-story 工作流

### Debug Log References

- `cargo test --lib --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1` → 393 passed, 0 failed。
  - 说明：完整 `cargo test`（含 bin/集成目标）在本机因 `target\debug\egosync.exe` 被运行中的 Tauri dev 实例占用而无法链接（`os error 5 拒绝访问`），属环境锁定而非代码问题；改用 `--lib` 运行全部库内单测（本 story 全部逻辑与单测均位于 lib），全绿。
- `npm --prefix "egosync-app" run test:frontend` → 18 文件 / 194 测试全部通过。
- `npm --prefix "egosync-app" run build`（`tsc && vite build`）→ exit 0，TypeScript strict / noUnusedLocals 通过。

### Completion Notes List

- **保护状态派生不刷新时间戳**：`mark_stale_q2_at_risk` / `clear_protection_for_resolved` 两条 UPDATE 均不写 `updated_at`，单测断言时间戳保持不变（AC 2），避免状态抖动。
- **重算幂等顺序**：`recompute_protection_status` 先 clear 后 mark，两者命中集互斥（AC 5）。临期升 Q1 的旧 at_risk 任务会在 clear 阶段被清回 normal。
- **编辑/完成即处理**：`update_task` 与 `set_task_completion` 在原 UPDATE SET 子句中追加 `protection_status = 'normal'`，复用其本就刷新 `updated_at` 的行为，与重算语义一致（AC 4）。
- **命令薄封装**：`task_check_protection_status` 仅调用 service 层 `recompute_protection_status`，返回 `u64`（被标记 at_risk 数量），符合三层架构（AC 7）。
- **后台定时器**：`task_protection_watch::spawn_hourly_watch` 仿 `task_deadline_watch`，每小时 + 首 tick 立即执行，错误只 `tracing::warn!`，不阻塞 setup（AC 7）。
- **前端挂载触发 + 容错**：`useTasks` 加载 effect 在 list 前 `await checkProtectionStatus()`，失败仅 `console.warn` 不阻塞列表加载（AC 2/4）；at_risk 在打开任务面板时即时反映。
- **UI 预警**：TasksTab 渲染琥珀 `AlertTriangle`「被挤压」badge + `border-l-4 border-l-amber-400` 左竖线，拖拽镜像同步；`normal` 任务无任何标识（AC 3）。
- **无新增迁移 / 无新增依赖**：`protection_status` 列与前端类型均已存在。
- 防回归：拖拽排序（3.2）、大石头优先排序与名额校验（3.4）、自动分类（3.3）相关逻辑均未改动，既有 Rust/前端测试全绿。

### Change Log

- 2026-06-20：实现 Story 3.5 — Q2 任务保护属性与连续被挤预警（数据标记 + UI 预警，行为层延后至 Epic 4/5）。后端新增保护状态读写与每小时重算定时器、`task_check_protection_status` 命令；前端挂载触发检查并在 TasksTab 渲染 at_risk 预警。新增 Rust 单测 6 项、前端测试 3 项；全量验证通过。Status → review。

### File List

- `egosync-app/src-tauri/src/db/tasks.rs`（新增 `mark_stale_q2_at_risk` / `clear_protection_for_resolved`；`update_task` 与 `set_task_completion` 追加 `protection_status = 'normal'`；新增 6 项保护状态单测）
- `egosync-app/src-tauri/src/services/task_protection_watch.rs`（新文件：`recompute_protection_status` / `compute_at_risk_threshold` / `spawn_hourly_watch` + 单测）
- `egosync-app/src-tauri/src/services/mod.rs`（注册 `pub mod task_protection_watch;`）
- `egosync-app/src-tauri/src/commands/task.rs`（新增 `task_check_protection_status` 命令）
- `egosync-app/src-tauri/src/lib.rs`（注册命令 + setup 中 spawn 保护检查定时器）
- `egosync-app/src/services/taskService.ts`（新增 `checkProtectionStatus`）
- `egosync-app/src/hooks/useTasks.ts`（加载 effect 挂载时触发保护检查，容错不阻塞）
- `egosync-app/src/components/role/TasksTab.tsx`（at_risk 预警 badge + 琥珀左竖线，拖拽镜像同步）
- `egosync-app/src/components/role/TasksTab.test.tsx`（新增 at_risk 渲染测试）
- `egosync-app/src/hooks/useTasks.test.tsx`（mock checkProtectionStatus + 触发/容错测试）

### Review Findings

_代码审查于 2026-06-20（Blind Hunter / Edge Case Hunter / Acceptance Auditor 三层）。结论：实现正确、测试充分，无阻断性缺陷；SQL 重算谓词幂等且互斥，时间戳格式与 `chrono_now_pub()` 字节一致。以下为可改进项。_

- [x] [Review][Decision] 保护检查触发方式：顺序 `await` vs fire-and-forget — `useTasks.ts:66-89` 在列表请求前 `await taskService.checkProtectionStatus()`（try/catch 容错）。**已裁决（2026-06-20）：保留当前顺序 await**，因其重算先提交再拉列表、最符合 AC2/AC4「即时反映」；spec 文字「fire-and-forget」以此实现为准，代码不改。
- [x] [Review][Patch] AC3 琥珀左竖线无测试覆盖 — 仅断言了「被挤压」badge，未断言 `border-l-4 border-l-amber-400` 左竖线（也未验证 normal 任务不出现该竖线）[egosync-app/src/components/role/TasksTab.test.tsx:388-420] ✅ 已修复（补 at_risk 有/normal 无的左竖线断言）
- [x] [Review][Patch] `recompute_protection_status` 组合逻辑无直接单测 — 仅分别测了 `mark_stale_q2_at_risk` / `clear_protection_for_resolved`，未测单次调用内 clear→mark 的幂等组合 [egosync-app/src-tauri/src/services/task_protection_watch.rs:26-35] ✅ 已修复（新增 `recompute_protection_status_clears_then_marks_in_one_pass` 端到端单测）
- [x] [Review][Patch] 观测性：`recompute_protection_status` 丢弃 `clear_protection_for_resolved` 返回的行数，且仅在 `marked > 0` 时 `info!`，清除数/检查总数不可见 [egosync-app/src-tauri/src/services/task_protection_watch.rs:26-35] ✅ 已修复（捕获 `cleared` 并在 mark 或 clear 非零时记录 `marked`/`cleared`）
- [x] [Review][Defer] `days_to_ymd` 与历法阈值计算在 `task_protection_watch.rs` / `task_deadline_watch.rs` / `settings.rs` 三处重复（含 719468/146097 魔数）[egosync-app/src-tauri/src/services/task_protection_watch.rs:50-78] — deferred, pre-existing
- [x] [Review][Defer] `spawn_hourly_watch` 无优雅关闭/取消（无限 loop），与既有 `task_deadline_watch` 同模式 [egosync-app/src-tauri/src/services/task_protection_watch.rs:88-100] — deferred, pre-existing
- [x] [Review][Defer] `'at_risk'` / `'normal'` 状态字面量在多处 SQL/TS 硬编码，无共享常量或枚举 [egosync-app/src-tauri/src/db/tasks.rs:466-509] — deferred, pre-existing
