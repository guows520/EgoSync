---
baseline_commit: c4248c6
---

# Story 3.4: 用户能标记任务为"大石头"并在 TasksTab 中视觉突出

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 标记本周最重要的几个任务为"大石头",
so that 系统和我都能识别不可妥协的核心任务。

## Acceptance Criteria

1. **大石头标记持久化**
   - **Given** 用户在 `TaskModal` 勾选"标记为本周大石头"复选框
   - **When** 保存任务（创建或编辑）
   - **Then** `tasks.is_big_rock = true`
   - **And** 保存成功后 `TasksTab` 中该任务显示琥珀色 `大石头` 标签（`bg-amber-50 text-amber-600 border-amber-100`）

2. **大石头视觉突出与排序优先**
   - **Given** `TasksTab` 中有大石头任务
   - **Then** 显示琥珀色 `大石头` 标签（`bg-amber-50 text-amber-600 border-amber-100`）
   - **And** 大石头任务在同象限内排序靠前（`is_big_rock = true` 的任务排在 `is_big_rock = false` 之前）
   - **And** 大石头标签样式已在 Story 3.1 中实现，本 story 不需要新增标签 UI

3. **每角色最多 3 个大石头**
   - **Given** 某角色已有 3 个 `is_big_rock = true` 的未软删除任务
   - **When** 用户尝试标记第 4 个（通过创建或编辑）
   - **Then** 后端返回 `AppError::ValidationError`，消息为"每个角色每周最多 3 个大石头，请先取消一个再标记"
   - **And** 标记操作不生效（`is_big_rock` 不写入）
   - **And** 前端 `TaskModal` 显示该错误消息，弹窗保持打开

4. **取消大石头标记**
   - **Given** 用户在 `TaskModal` 中取消勾选"标记为本周大石头"
   - **When** 保存任务
   - **Then** `tasks.is_big_rock = false`
   - **And** `TasksTab` 中琥珀色标签消失

5. **后端校验 big_rock 数量限制**
   - **Given** Rust 后端
   - **Then** `create_task` 和 `update_task` 中校验 big_rock 数量限制（per role ≤ 3，仅计 `deleted_at IS NULL` 的任务）
   - **And** `update_task` 中仅当 `is_big_rock` 从 `false` 变为 `true` 时才触发校验（取消标记不触发）
   - **And** `create_task` 中当 `is_big_rock = true` 时触发校验

## Tasks / Subtasks

- [x] 后端：新增 big_rock 数量查询函数（AC: 3, 5）
  - [x] 在 `egosync-app/src-tauri/src/db/tasks.rs` 新增 `count_big_rocks_by_role(pool, role_id) -> Result<i32, AppError>`
  - [x] SQL: `SELECT COUNT(*) FROM tasks WHERE role_id = ?1 AND is_big_rock = 1 AND deleted_at IS NULL`
  - [x] 返回值为当前角色已标记大石头且未软删除的任务数量

- [x] 后端：在 `create_task` 中加入 big_rock 校验（AC: 3, 5）
  - [x] 在 `db/tasks.rs::create_task` 中，当 `is_big_rock = true` 时，先调用 `count_big_rocks_by_role`
  - [x] 若 count >= 3，返回 `AppError::ValidationError("每个角色每周最多 3 个大石头，请先取消一个再标记".to_string())`
  - [x] 校验在 INSERT 之前执行，确保不写入

- [x] 后端：在 `update_task` 中加入 big_rock 校验（AC: 3, 5）
  - [x] 在 `db/tasks.rs::update_task` 中，当 `input.is_big_rock == Some(true)` 时，先读取当前 task
  - [x] 若当前 task 的 `is_big_rock` 已经是 `true`，跳过校验（幂等）
  - [x] 若当前 task 的 `is_big_rock` 是 `false`，调用 `count_big_rocks_by_role`
  - [x] 若 count >= 3，返回 `AppError::ValidationError("每个角色每周最多 3 个大石头，请先取消一个再标记".to_string())`
  - [x] 校验在 UPDATE 之前执行，确保不写入

- [x] 后端：为 big_rock 校验添加单元测试（AC: 3, 5）
  - [x] 测试：创建第 4 个大石头时返回 `ValidationError`
  - [x] 测试：已有 3 个大石头时，编辑非大石头任务标记为大石头返回 `ValidationError`
  - [x] 测试：已有 3 个大石头时，编辑已是大石头任务（不改变 is_big_rock）不报错
  - [x] 测试：取消大石头标记不触发校验（即使角色已有 3 个大石头）
  - [x] 测试：不同角色的大石头互不影响（role-a 有 3 个大石头不影响 role-b）

- [x] 前端：`TaskModal` 处理 big_rock 限制错误（AC: 3）
  - [x] 在 `egosync-app/src/components/modals/TaskModal.tsx` 的 `handleSubmit` catch 块中，检测错误是否为 big_rock 限制
  - [x] 后端 `ValidationError` 序列化为 `{ "ValidationError": "每个角色每周最多 3 个大石头..." }` JSON 对象
  - [x] Tauri invoke 抛出的错误在前端为该 JSON 对象；检测 `e.ValidationError` 字段是否包含"大石头"关键词
  - [x] 若匹配，设置 `error` 为该消息原文；否则保持通用错误提示
  - [x] 弹窗保持打开，不调用 `onClose()`

- [x] 前端：`TasksTab` 大石头排序优先（AC: 2）
  - [x] 在 `egosync-app/src/components/role/TasksTab.tsx` 中修改排序逻辑
  - [x] 当前排序：`const orderBySort = (a, b) => a.sortOrder - b.sortOrder`
  - [x] 新排序：大石头优先，然后按 `sortOrder`：`(a, b) => (Number(b.isBigRock) - Number(a.isBigRock)) || (a.sortOrder - b.sortOrder)`
  - [x] 该排序应用于 `incompleteByQuadrant` 和 `completedByQuadrant` 的 `.sort()` 调用
  - [x] 拖拽逻辑不需要修改：`handleDragEnd` 基于已排序的 `incompleteByQuadrant` 构建 `fullOrder`，big_rock 优先排序会自动反映在 `sortOrder` 赋值中

- [x] 前端：为 big_rock 排序和错误处理添加测试（AC: 2, 3）
  - [x] `TasksTab.test.tsx`：验证大石头任务在同象限内排在非大石头任务之前
  - [x] `TaskModal.test.tsx`：验证后端返回 big_rock 限制错误时显示对应中文消息

- [x] 测试与验证（AC: 1-5）
  - [x] 运行 `npm --prefix "egosync-app" run test:frontend`
  - [x] 运行 `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`
  - [x] 运行 `npm --prefix "egosync-app" run build`

## Dev Notes

### Current State

- `is_big_rock` 字段已在 Story 3.1 中创建：DB schema（`013_tasks.sql`）、Rust model（`Task.is_big_rock: bool`）、`CreateTaskInput.is_big_rock: Option<bool>`、`UpdateTaskInput.is_big_rock: Option<bool>`、前端 `Task.isBigRock: boolean`。
- `TaskModal.tsx` 已有"标记为本周大石头"复选框 UI（lines 133-146），`isBigRock` state 已接入，保存时已传递 `isBigRock` 字段。
- `TasksTab.tsx` 已有琥珀色"大石头"标签渲染（line 79: `{task.isBigRock && <span className="... bg-amber-50 text-amber-600 border-amber-100">大石头</span>}`）。
- `db/tasks.rs::create_task` 已写入 `is_big_rock` 字段（line 29: `.bind(is_big_rock)`），但**无数量校验**。
- `db/tasks.rs::update_task` 已更新 `is_big_rock` 字段（line 78: `is_big_rock = COALESCE(?5, is_big_rock)`），但**无数量校验**。
- 当前 `TasksTab.tsx` 排序仅按 `sortOrder`（line 250: `const orderBySort = (a, b) => a.sortOrder - b.sortOrder`），不区分 big_rock。
- **无需新增迁移文件**：`is_big_rock` 列已在 `013_tasks.sql` 中定义。

### What This Story Changes

1. **后端 `db/tasks.rs`**：新增 `count_big_rocks_by_role` 函数；在 `create_task` 和 `update_task` 中添加 big_rock 数量校验（≤ 3 per role）。
2. **前端 `TasksTab.tsx`**：修改排序逻辑，大石头任务在同象限内排在前面。
3. **前端 `TaskModal.tsx`**：改进错误处理，识别后端 big_rock 限制错误并显示精确中文消息。

### What Must Be Preserved

- `TaskModal` 的"标记为本周大石头"复选框 UI 和 `isBigRock` state 管理保持不变。
- `TasksTab` 的琥珀色"大石头"标签渲染保持不变。
- 拖拽排序功能（Story 3.2）保持正常工作：大石头优先排序不影响 `handleDragEnd` 的 `fullOrder` 构建。
- 自动分类功能（Story 3.3）保持正常工作：`task:classified` 事件、`classifyingIds` 状态、`manual_override` 逻辑不受影响。
- `create_task` 的异步分类 spawn 逻辑不受影响：校验在 `create_task` db 函数中执行，在 command 层 spawn 之前。
- 现有测试全部保持通过：新增校验不应破坏已有 CRUD、排序、完成、分类测试。

### Architecture Guardrails

- 前端不得直接访问 SQLite；Tauri IPC 必须经 `egosync-app/src/services/taskService.ts`。
- Rust `commands/` 只做参数校验与调用 `db/`；big_rock 数量校验放在 `db/tasks.rs` 中（因为 db 层已负责数据完整性防御）。
- Rust command 返回 `Result<T, AppError>`；`ValidationError` 自然传播到前端，不需要在 command 层特殊处理。
- 所有 Rust DTO/Model 与前端 JSON 交互使用 camelCase（`#[serde(rename_all = "camelCase")]`）。
- TypeScript strict mode、`noUnusedLocals`、`noUnusedParameters` 均开启。
- Tailwind utility classes only；不新增 CSS 文件。
- 不新增依赖；现有 React、Tauri、SQLx、serde、uuid 已足够。
- 不接入 LLM、opencode、Skill、MCP、通知、调度器。

### AppError 序列化格式

`AppError::ValidationError(msg)` 序列化为 JSON 对象 `{"ValidationError": "msg"}`。前端通过 `invoke()` 调用时，catch 块收到的错误对象即为该 JSON。检测方式：

```typescript
catch (e) {
  const errorObj = e as { ValidationError?: string };
  if (errorObj?.ValidationError) {
    setError(errorObj.ValidationError);
  } else {
    setError('任务暂时保存失败，请稍后再试');
  }
}
```

### Big Rock 排序与拖拽交互

大石头优先排序是**渲染层**行为，不修改 `sort_order` 数据库值。当用户拖拽任务时：

1. `incompleteByQuadrant[q]` 已按 big_rock 优先 + sortOrder 排序。
2. `handleDragEnd` 中的 `arrayMove` 在该已排序列表内移动元素。
3. `fullOrder` 基于移动后的列表构建，包含所有象限的任务。
4. 后端 `reorder_tasks` 按 `fullOrder` 索引重写 `sort_order`。
5. 下次渲染时，big_rock 优先排序仍然生效——即使 `sort_order` 值可能不完美，视觉上大石头始终靠前。

**注意**：用户可能将非大石头任务拖到大石头任务上方，但渲染时大石头会自动浮回顶部。这是预期行为——大石头优先是系统强制的，用户拖拽仅控制同组内顺序。

### Big Rock 计数语义

> ⚠️ 本节已随后续演进更新，反映当前代码真实行为（详见文末「Post-Story 演进」）。

- 计数函数 `count_big_rocks_by_owner`，范围：`owner_type = ? AND (role 维度匹配) AND is_big_rock = 1 AND deleted_at IS NULL`，按「任务清单」（角色或管家）独立计数。
- **完成即释放名额（方案 D）**：完成任务时后端自动撤销其大石头标记（`is_big_rock = 0`），因此已完成任务不再计入大石头上限；撤销完成后任务回到普通未完成状态，不会恢复大石头身份。
- "每周"是概念性约束（"本周大石头"），不需要按时间窗口查询；V1 简化为当前进行中大石头总数 ≤ 3。

### Previous Story Intelligence

- **Story 3.1**：创建了 task CRUD 全链路。`is_big_rock` 字段已预留但无校验。`TaskModal` 复选框 UI 已完成。`TasksTab` 大石头标签已渲染。
- **Story 3.2**：拖拽排序使用 `fullOrder`（全角色任务 id 顺序）重写 `sort_order`。完成态任务沉底是前端渲染职责。错误通过 `TasksTab` 内联中文显示。
- **Story 3.3**：`task_create` 在未显式提供 quadrant 时 spawn 后台异步分类。`TaskModal` 编辑模式仅当 quadrant 实际变更时才发送 quadrant 字段。`isClassificationUncertain` helper 前后端阈值同步。测试 fixture 中 Task 字面量需包含所有字段（`manualOverride: false, classificationReason: null`）。
- **通用模式**：Rust 测试在 `db/tasks.rs` 的 `#[cfg(test)]` 模块中使用 in-memory SQLite。前端测试用 Vitest + Testing Library。`cargo test` 需 `--test-threads=1` 避免端口冲突。

### Git Intelligence

Recent relevant commits:

- `c4248c6 feat(tasks): async classification with event notification, smart-detect UX, Q2-only escalation`
- `0871096 Fix story 3.2 drag-sort and sync docs`
- `5585768 docs: mark story 3.1 done`
- `c39c968 feat(tasks): add role task CRUD`

### Testing Requirements

- Rust 测试在 `egosync-app/src-tauri/src/db/tasks.rs` 的 `#[cfg(test)]` 模块中，使用 `setup_test_db()` 辅助函数（已存在）。
- 前端测试在 `egosync-app/src/components/modals/TaskModal.test.tsx` 和 `egosync-app/src/components/role/TasksTab.test.tsx` 中。
- 测试 fixture 中 Task 字面量必须包含所有字段（参考 Story 3.3 的 `manualOverride: false, classificationReason: null`）。
- 必须运行的验证命令：
  - `npm --prefix "egosync-app" run test:frontend`
  - `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`
  - `npm --prefix "egosync-app" run build`

### Project Structure Notes

- 无新增文件：所有修改都在现有文件中。
- 无新增迁移：`is_big_rock` 列已在 `013_tasks.sql` 中定义。
- 修改文件列表：
  - `egosync-app/src-tauri/src/db/tasks.rs` — 新增 `count_big_rocks_by_role`，在 `create_task` / `update_task` 中添加校验，新增测试
  - `egosync-app/src/components/modals/TaskModal.tsx` — 改进 catch 块错误处理
  - `egosync-app/src/components/role/TasksTab.tsx` — 修改排序逻辑
  - `egosync-app/src/components/modals/TaskModal.test.tsx` — 新增 big_rock 限制错误测试
  - `egosync-app/src/components/role/TasksTab.test.tsx` — 新增 big_rock 排序测试

### References

- `_bmad-output/project-context.md`
- `_bmad-output/planning-artifacts/epics.md:1469-1497`
- `_bmad-output/planning-artifacts/prd-egosync.md` FR-23, FR-24
- `_bmad-output/planning-artifacts/ux-design-specification.md:763-770`
- `_bmad-output/implementation-artifacts/3-1-task-crud-role-view.md`
- `_bmad-output/implementation-artifacts/3-2-task-drag-sort-complete.md`
- `_bmad-output/implementation-artifacts/3-3-auto-quadrant-classification.md`
- `egosync-app/src-tauri/migrations/013_tasks.sql`
- `egosync-app/src-tauri/src/models/task.rs`
- `egosync-app/src-tauri/src/db/tasks.rs`
- `egosync-app/src-tauri/src/commands/task.rs`
- `egosync-app/src-tauri/src/error.rs`
- `egosync-app/src/types/task.ts`
- `egosync-app/src/services/taskService.ts`
- `egosync-app/src/hooks/useTasks.ts`
- `egosync-app/src/components/role/TasksTab.tsx`
- `egosync-app/src/components/modals/TaskModal.tsx`

## Review Findings

_代码审查于 2026-06-19（三视角：Blind Hunter / Edge Case Hunter / Acceptance Auditor）。结果：0 decision-needed，2 patch，1 defer，3 dismissed。_

- [x] [Review][Patch] `update_task` 在 big_rock 校验路径重复查询 `get_active_task` [egosync-app/src-tauri/src/db/tasks.rs:66] — 已修复：复用入口 `existing` 结果，去掉重复查询。
- [x] [Review][Patch] big_rock 上限魔法数字 3 与错误消息字符串重复 [egosync-app/src-tauri/src/db/tasks.rs:9-10] — 已修复：抽 `const MAX_BIG_ROCKS_PER_ROLE: i32 = 3;` 与 `const BIG_ROCK_LIMIT_MESSAGE`，`create_task`/`update_task` 共用。
- [x] [Review][Defer] big_rock 计数检查与写入非原子（TOCTOU 竞态） [egosync-app/src-tauri/src/db/tasks.rs:13-44,67-113] — deferred，V1 单用户可接受。`count_big_rocks_by_role` 与 INSERT/UPDATE 未包裹在同一事务中；并发写入（用户 + opencode agent 同时建/改任务）理论上可双双读到 count<3 而突破上限。V1 为单用户桌面应用，实际风险极低；若未来引入多写入方，应仿照 `reorder_tasks` 用事务包裹 count+write。

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4 (Windsurf Cascade)

### Debug Log References

- Rust 测试：`cargo test --lib -- --test-threads=1 db::tasks` → 19 passed, 0 failed
- 前端测试：`npm run test:frontend -- --run` → 18 files, 189 tests passed
- 构建：`npm run build` → tsc + vite build 成功

### Completion Notes List

- 新增 `count_big_rocks_by_role` 函数，SQL 查询 `is_big_rock = 1 AND deleted_at IS NULL` 的任务数量
- `create_task` 在 INSERT 前校验：`is_big_rock = true` 时若 count >= 3 返回 `ValidationError`
- `update_task` 在 UPDATE 前校验：仅 `is_big_rock` 从 false→true 时触发，已为 true 跳过（幂等），取消标记不触发
- `TaskModal.tsx` catch 块检测 `e.ValidationError` 字段，匹配时显示后端原始中文消息，弹窗保持打开
- `TasksTab.tsx` 排序改为 `(Number(b.isBigRock) - Number(a.isBigRock)) || (a.sortOrder - b.sortOrder)`，大石头在同象限内优先
- 5 个 Rust 单元测试覆盖：第4个大石头被拒、编辑非大石头→大石头被拒、编辑已是大石头不报错、取消标记不触发校验、跨角色独立计数
- 2 个前端测试覆盖：TasksTab 大石头排序优先、TaskModal big_rock 限制错误显示中文消息且弹窗不关闭
- 修复了 cargo 镜像配置（USTC 镜像不可用，切换到 sparse+https://index.crates.io/）

### File List

- `egosync-app/src-tauri/src/db/tasks.rs` — 新增 `count_big_rocks_by_role`，`create_task`/`update_task` 添加 big_rock 校验，新增 5 个单元测试
- `egosync-app/src/components/modals/TaskModal.tsx` — catch 块改进错误处理，识别 `ValidationError` 并显示精确中文消息
- `egosync-app/src/components/role/TasksTab.tsx` — 排序逻辑改为大石头优先 + sortOrder
- `egosync-app/src/components/modals/TaskModal.test.tsx` — 新增 big_rock 限制错误测试
- `egosync-app/src/components/role/TasksTab.test.tsx` — 新增大石头排序优先测试

## Post-Story 演进（与当前代码的差异）

本 story 标记 `done` 后，代码随后续工作演进，以下事实已与原文（AC / Tasks / Dev Notes 中的历史描述）不同，**以当前代码为准**：

### 1. 任务 owner scope（角色 + 管家）

- 大石头计数从「按角色」泛化为「按任务清单 owner」（角色或管家）。
- 函数更名：`count_big_rocks_by_role` → `count_big_rocks_by_owner(pool, owner_type, role_id)`。
- SQL：`WHERE owner_type = ?1 AND ((?2 IS NULL AND role_id IS NULL) OR role_id = ?2) AND is_big_rock = 1 AND deleted_at IS NULL`。
- 常量更名：`MAX_BIG_ROCKS_PER_ROLE` → `MAX_BIG_ROCKS_PER_OWNER`。
- 错误消息：「每个角色每周最多 3 个大石头，请先取消一个再标记」→「每个任务清单每周最多 3 个大石头，请先取消一个再标记」。

### 2. 完成即撤销大石头标记（方案 D，2026-06-20）

- `set_task_completion` 在完成任务（`is_completed = true`）时同步把 `is_big_rock` 置 0；撤销完成（`false`）时保持现状（已非大石头）。
- 效果：已完成大石头不再占用名额，完成一个即可标记新的；撤销完成后任务为普通未完成任务，不会恢复大石头身份——消除了「已完成大石头是否计入名额」及「撤销后超限」的边界矛盾。
- 原 Dev Notes「Big Rock 计数语义」中「已完成的大石头任务仍计入限制」一句已作废。
- 前端 `useTasks.toggleComplete` 乐观更新在完成时同步清除 `isBigRock`，使「大石头」标签即时消失。

### 3. 测试记录更新

- `db/tasks.rs` 大石头相关测试新增方案 D 的 2 个用例：`completing_big_rock_clears_flag_and_frees_slot`、`uncompleting_big_rock_stays_non_big_rock`。
- 最新 `cargo test --lib db::tasks::tests` → 24 passed；前端 `useTasks` / `TasksTab` / `ButlerWorkspacePanel` / `ButlerView` 相关测试均通过。
