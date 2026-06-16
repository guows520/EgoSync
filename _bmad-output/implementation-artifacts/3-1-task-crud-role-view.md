---
baseline_commit: dccfdc609cfa7410f35b9721304d6e311c0307a9
---

# Story 3.1: 用户能在角色视图创建、编辑、删除任务

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 在角色的任务面板中创建、编辑和删除任务,
so that 我能为每个角色管理独立的待办事项。

## Acceptance Criteria

1. **任务创建持久化**
   - Given 用户在 `RoleView` → `TasksTab` 点击 “+” 按钮
   - When `TaskModal` 弹出并填写任务内容、截止时间、四象限分类和大石头标记
   - Then 点击“保存任务”后 `tasks` 表写入新记录
   - And 新任务归属当前 `role.id`
   - And Tauri command 返回完整 task 对象
   - And `TasksTab` 无需刷新页面即可显示新任务

2. **任务编辑持久化**
   - Given `TasksTab` 中已有任务
   - When 用户点击任务卡片打开编辑态，修改任务内容、截止时间、四象限分类或大石头标记并保存
   - Then `tasks` 表对应记录更新
   - And `updated_at` 更新为当前时间
   - And `TasksTab` 实时显示修改后的任务

3. **任务删除为软删除**
   - Given 用户在任务卡片上点击删除操作
   - When 用户确认删除
   - Then `tasks.deleted_at` 写入当前时间，不物理删除记录
   - And `TasksTab` 列表实时移除该任务
   - And 后续 `task_list_by_role` 默认不返回已软删除任务

4. **数据库 schema**
   - Given 当前迁移目录已到 `012_mcp_server_standard_types.sql`
   - Then 新增迁移必须命名为 `013_tasks.sql`
   - And 创建 `tasks` 表，字段固定为：`id`, `role_id`, `title`, `deadline`, `quadrant`, `is_big_rock`, `is_completed`, `completed_at`, `sort_order`, `protection_status`, `confidence`, `created_at`, `updated_at`, `deleted_at`
   - And 以本 story 字段契约为准；不要额外添加架构概览旧表述中的 `content` 或 `status` 列
   - And `role_id` 外键关联 `roles(id)`，角色删除时级联删除任务
   - And `quadrant` 仅允许 `Q1` / `Q2` / `Q3` / `Q4`
   - And `protection_status` 默认 `normal`，为后续 Story 3.5 预留
   - And 建立常用索引：`role_id`、`quadrant`、`deleted_at`、`sort_order`

5. **Rust 后端 API**
   - Given Rust 后端
   - Then 新增 Tauri commands：`task_create` / `task_list_by_role` / `task_update` / `task_delete`
   - And command 层只做参数校验和调用 `db::tasks` helper；本 story 沿用 `role` 模块的轻量 CRUD 模式，不新增中间 service，除非实现时发现必须复用跨模块业务规则
   - And 新增/更新操作返回完整 `Task` 对象供前端同步状态
   - And `task_delete` 软删除成功后返回 `()` / void；前端按 id 从本地列表移除或 refetch
   - And 对不存在或已软删除任务执行 update/delete 时返回 `AppError::NotFound`
   - And 创建/更新时任务标题不能为空，非法 `quadrant` 返回 `AppError::ValidationError`

6. **前端接通真实数据**
   - Given 前端
   - Then 新增 `egosync-app/src/types/task.ts`、`egosync-app/src/services/taskService.ts`、`egosync-app/src/hooks/useTasks.ts`
   - And `TasksTab` 使用 `useTasks(role.id)` 读取真实任务，替换 `ROLE_TASKS` mock
   - And `TaskModal` 改为受控表单，支持 create/edit 两种模式
   - And 组件不得直接调用 `invoke()`，所有 IPC 必须走 `taskService`

7. **现有角色工作台交互不回归**
   - `RoleWorkspacePanel` 的 `任务清单 / 记忆档案 / 设置` tab 切换保持可用
   - `MemoryTab` 的记忆计数、source navigation、删除回调和目标记忆跳转保持可用
   - 角色视图的色温、右侧 40% 工作台布局、`TaskModal` 全局弹窗入口保持一致
   - 不实现拖拽排序、勾选完成、自动四象限分类、大石头数量限制、Q2 保护预警或管家全局任务汇总；这些属于 Story 3.2-3.7

8. **测试与验证**
   - Rust 单元/集成测试覆盖：任务创建、按角色列出、编辑、软删除、非法 quadrant、空 title、已删除任务不可更新/删除、角色隔离
   - 前端测试覆盖：加载真实任务、创建后插入列表、编辑后更新列表、删除后移除、加载失败中文内联提示、空状态文案
   - 至少运行：`npm --prefix "egosync-app" run test:frontend`、`cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`、`npm --prefix "egosync-app" run build`
   - 可见 UI 改动需启动应用并人工验证任务创建/编辑/删除 golden path；若无法启动，必须在 Dev Agent Record 写明原因

## Tasks / Subtasks

- [x] 后端：新增任务数据模型与迁移（AC: 4, 5）
  - [x] 新增 `egosync-app/src-tauri/migrations/013_tasks.sql`，不要使用规划文档中的旧编号 `005_tasks.sql`
  - [x] 新增 `egosync-app/src-tauri/src/models/task.rs`，`Task` / `CreateTaskInput` / `UpdateTaskInput` 使用 `#[serde(rename_all = "camelCase")]`
  - [x] 在 `egosync-app/src-tauri/src/models/mod.rs` 导出 `task`
  - [x] `deadline` 使用 `Option<String>`，存 ISO 8601 或 `YYYY-MM-DD` 字符串；不要新增日期库
  - [x] `is_big_rock` / `is_completed` 在 SQLite 中用 INTEGER 0/1，Rust model 可用 bool 并让 SQLx 正确映射

- [x] 后端：实现任务 DB CRUD（AC: 1-5）
  - [x] 新增 `egosync-app/src-tauri/src/db/tasks.rs`
  - [x] 在 `egosync-app/src-tauri/src/db/mod.rs` 导出 `tasks`
  - [x] `create_task(pool, input)` 生成 UUID v4、设置 `created_at` / `updated_at`、计算当前 role 下末尾 `sort_order`
  - [x] DB helper 负责 SQL 与数据完整性；title/quadrant 的用户输入校验在 command 层先做一遍，DB 层仍应防御非法数据
  - [x] `list_tasks_by_role(pool, role_id)` 只返回 `deleted_at IS NULL`，按 `sort_order ASC, created_at ASC` 排序
  - [x] `update_task(pool, id, input)` 仅更新传入字段，保持未传字段不变，返回完整 `Task`
  - [x] `soft_delete_task(pool, id)` 写 `deleted_at` 和 `updated_at`，不物理删除
  - [x] 所有 SQL 错误映射为中文 `AppError::DbError`，不存在映射为 `AppError::NotFound`

- [x] 后端：实现并注册 Tauri task commands（AC: 5）
  - [x] 新增 `egosync-app/src-tauri/src/commands/task.rs`
  - [x] 在 `egosync-app/src-tauri/src/commands/mod.rs` 导出 `task`
  - [x] 在 `egosync-app/src-tauri/src/lib.rs` 的 `tauri::generate_handler!` 注册 `task_create`、`task_list_by_role`、`task_update`、`task_delete`
  - [x] command 层校验 title trim 非空、quadrant 合法；CRUD 调用 `db::tasks`，不新增 `services/task.rs`，除非后续故事引入跨模块业务规则
  - [x] `task_delete` 返回 `Result<(), AppError>`；前端成功后按 task id 移除或 refetch
  - [x] 不接入 opencode、agent_config、scheduler 或 notification，本 story 只做任务 CRUD

- [x] 前端：新增任务类型、service、hook（AC: 1, 2, 3, 6）
  - [x] 新增 `egosync-app/src/types/task.ts`：`Task`、`TaskQuadrant`、`CreateTaskInput`、`UpdateTaskInput`
  - [x] 新增 `egosync-app/src/services/taskService.ts`：`create`、`listByRole`、`update`、`delete`
  - [x] 新增 `egosync-app/src/hooks/useTasks.ts`，对齐 `useMemories` 的 `isLoading/error/refetch` 模式
  - [x] `useTasks` 在 roleId 变化时清空过期数据，避免切换角色时短暂显示上一个角色任务

- [x] 前端：改造 `TasksTab` 读取真实任务（AC: 1, 2, 3, 6, 7）
  - [x] 删除 `ROLE_TASKS` 依赖，不再从 `constants/mockData` 读取任务
  - [x] 保留现有卡片视觉：`GripVertical`、`Circle`、deadline badge、大石头标签、hover 边框/阴影
  - [x] 显示真实任务列表；空状态使用温和中文文案，不显示“暂无数据”
  - [x] 任务卡片点击进入编辑；删除操作需可达且不影响卡片点击
  - [x] 创建、更新、删除成功后通过 hook 本地状态更新或 `refetch` 立即反映，不要求用户刷新
  - [x] 加载/失败状态使用面板内联反馈，不新增 toast/snackbar

- [x] 前端：改造 `TaskModal` 为 create/edit 表单（AC: 1, 2, 6, 7）
  - [x] `TaskModal` props 至少支持：`roleId`、`task?`、`mode`、`onSave`、`onClose`
  - [x] 标题随模式显示“新建任务”/“编辑任务”
  - [x] 表单字段：任务内容、四象限分类、截止时间、大石头标记
  - [x] 删除 `TaskModal` 里“在周规划中优先受到系统时间保护”等 Q2/保护行为暗示；本 story 只保存 `isBigRock` 标记，不实现保护逻辑
  - [x] 保存按钮在 title 为空或提交中禁用，并显示内联中文错误
  - [x] 保存成功后关闭弹窗；失败时保持弹窗打开并显示错误
  - [x] 取消按钮只关闭弹窗，不写入数据

- [x] 前端：保持 App/RoleWorkspacePanel 入口一致（AC: 1, 2, 6, 7）
  - [x] 更新 `egosync-app/src/App.tsx` 的 `TaskModal` 打开状态，使其知道当前 active role 与可选编辑 task
  - [x] 如需在 `RoleWorkspacePanel` 与 `TasksTab` 间传递 `onEditTask` / `onTaskSaved`，保持 props 精简且不影响 MemoryTab
  - [x] 不把任务 CRUD 状态提升到全局 roles state，除非为保持现有弹窗入口不可避免

- [x] 测试与验证（AC: 1-8）
  - [x] Rust：为 `db::tasks` 添加单元测试或集成测试，覆盖 CRUD、软删除、排序、角色隔离和校验
  - [x] 前端：新增/更新 `TasksTab.test.tsx`、`TaskModal.test.tsx`、`useTasks` 或 service 测试
  - [x] 回归 MemoryTab / RoleWorkspacePanel 相关测试，确保 tab 与记忆计数未被破坏
  - [x] 运行 `npm --prefix "egosync-app" run test:frontend`
  - [ ] 运行 `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`（环境 registry mirror 阻塞，详见 Dev Agent Record）
  - [x] 运行 `npm --prefix "egosync-app" run build`
  - [x] 启动 `npm --prefix "egosync-app" run tauri dev` 或等价流程；应用已以复用现有 Vite 的 Tauri dev 流程启动，当前 `egosync.exe` 窗口可供 boss 人工验证创建、编辑、删除任务 golden path

### Review Findings

- [x] [Review][Patch] 删除任务缺少应用内确认弹窗，当前会直接软删除 [`egosync-app/src/components/role/TasksTab.tsx:81`]
- [x] [Review][Patch] 截止时间无法在编辑时清空 [`egosync-app/src-tauri/src/db/tasks.rs:67`]
- [x] [Review][Patch] 保存按钮未在标题为空时禁用 [`egosync-app/src/components/modals/TaskModal.tsx:129`]
- [x] [Review][Patch] 点击任务卡片本身不会进入编辑态 [`egosync-app/src/components/role/TasksTab.tsx:63`]
- [x] [Review][Patch] `protectionStatus` 前端类型过宽，未对齐 story 数据契约 [`egosync-app/src/types/task.ts:13`]

## Dev Notes

### Current State

- Sprint 已进入 Epic 3；`epic-3` 将随本 story 创建从 `backlog` 更新为 `in-progress`。
- 任务前端 UI 已存在但仍是 mock：`egosync-app/src/components/role/TasksTab.tsx:1-32` 从 `ROLE_TASKS` 读取，并用数组 index 作为 key。
- `egosync-app/src/components/modals/TaskModal.tsx:4-52` 只有静态表单 UI；保存按钮仅调用 `onClose`，没有 state、校验、create/update 调用。
- `egosync-app/src/components/role/RoleWorkspacePanel.tsx:64-105` 已负责 tab 布局和 `TasksTab` / `MemoryTab` / `SettingsTab` 切换；本 story 应复用该入口，不重建角色工作台。
- `egosync-app/src/App.tsx` 当前用 `isTaskModalOpen` 全局控制 `TaskModal`：打开入口在 `RoleView` props，渲染在 `egosync-app/src/App.tsx:267`；需补齐 active role/edit task 上下文。
- 后端还没有 task 模块：当前 `commands/mod.rs` 仅导出 app/chat/llm_config/memory/role/secret，`db/mod.rs` 仅导出 app_settings/conversations/memories/pool/roles/settings，`models/mod.rs` 仅导出 agent/chat/memory/role/settings。
- 迁移目录当前到 `012_mcp_server_standard_types.sql`；必须新增 `013_tasks.sql`，不要与现有迁移编号冲突。

### Architecture Guardrails

- 前端不得直接访问 SQLite、opencode server 或 LLM API；Tauri IPC 必须经 `egosync-app/src/services/*Service.ts`。
- Rust `commands/` 只做 IPC 参数校验与调用 `db/` 或 service；业务规则不要塞进 command 大函数。
- Rust DB SQL 放在 `egosync-app/src-tauri/src/db/`；新增表/列必须通过 SQLx migration 文件。
- Rust command 返回 `Result<T, AppError>`；不要在 command path 使用 `.unwrap()` 或 `panic!`。
- 所有 Rust DTO/Model 与前端 JSON 交互使用 camelCase，Rust/DB 内部保持 snake_case。
- TypeScript strict mode、`noUnusedLocals`、`noUnusedParameters` 均开启；不要留下未用类型、变量或 mock。
- Tailwind utility classes only；不要新增 CSS 文件。
- 不新增依赖；React、Tauri、SQLx、serde、uuid、chrono 已足够完成本 story。
- 不接入 LLM、opencode、Skill、MCP、通知、调度器；本 story 是本地任务 CRUD 基础设施。
- UX 文档中“工作台手动操作后左侧角色对话流感知反馈”不在本 story 范围；本 story 只更新任务列表 UI，不生成对话消息或 agent 事件。

### Existing Files to Touch

- `egosync-app/src/components/role/TasksTab.tsx`
  - Current state: 展示 Q1 单组 mock 任务，使用 `ROLE_TASKS[role.id]`。
  - Change: 替换为 `useTasks(role.id)` 真实列表；保留卡片视觉，加入空/加载/错误状态、编辑/删除入口。
  - Preserve: `+` 按钮、卡片 hover、deadline badge、大石头标签、`GripVertical`/`Circle` 视觉占位。

- `egosync-app/src/components/modals/TaskModal.tsx`
  - Current state: 静态“新建任务”表单，保存不写数据。
  - Change: 改为 create/edit 受控表单，调用父级传入 save handler 或 `taskService` 包装链路。
  - Preserve: Modal 宽度、标题区域、字段布局、按钮层级和 Tailwind 视觉风格。

- `egosync-app/src/components/role/RoleWorkspacePanel.tsx`
  - Current state: tab 容器和 MemoryTab 计数/跳转逻辑稳定。
  - Change: 仅传递任务相关回调/上下文；不要重写记忆逻辑。
  - Preserve: `任务清单 / 记忆档案 / 设置` 三 tab、close 按钮和 `MemoryTab` props。

- `egosync-app/src/App.tsx`
  - Current state: `TaskModal` 打开状态没有携带 active role 或 edit task。
  - Change: 维护当前 task modal 上下文，使创建/编辑知道 roleId/task；保存后能通知对应 `TasksTab` 更新。
  - Preserve: 现有 `RoleView` 渲染、`WeeklyReviewModal`、`AddRoleModal`、`RoleConfirmModal`、notification/modal 入口。

- New frontend files:
  - `egosync-app/src/types/task.ts`
  - `egosync-app/src/services/taskService.ts`
  - `egosync-app/src/hooks/useTasks.ts`

- New/update Rust files:
  - `egosync-app/src-tauri/migrations/013_tasks.sql`
  - `egosync-app/src-tauri/src/models/task.rs`
  - `egosync-app/src-tauri/src/models/mod.rs`
  - `egosync-app/src-tauri/src/db/tasks.rs`
  - `egosync-app/src-tauri/src/db/mod.rs`
  - `egosync-app/src-tauri/src/commands/task.rs`
  - `egosync-app/src-tauri/src/commands/mod.rs`
  - `egosync-app/src-tauri/src/lib.rs`

### Previous Story Intelligence

- Recent commits show active work in memory/skill areas: `dccfdc6 feat(skill): enforce meta skill configuration`, `a4fb7c7 feat(memory): add traceable memory source navigation`, `55582e2 fix(memory): stabilize selective forgetting`.
- Story 2.10 completion notes show validation pattern already expected by this repo: frontend Vitest, Rust cargo test with `--manifest-path`, Vite build, then visible UI verification.
- Story 2.10 notes also mention current Windows environment may lack `python3`; Rust tests are more stable with `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`.
- Do not reuse Story 2.x Skill/agent_config patterns for tasks; task CRUD is ordinary domain data and should not sync opencode agent config.

### Data Contract

Suggested Rust / TypeScript contract:

```ts
export type TaskQuadrant = 'Q1' | 'Q2' | 'Q3' | 'Q4';
export type TaskProtectionStatus = 'normal' | 'at_risk';

export interface Task {
  id: string;
  roleId: string;
  title: string;
  deadline: string | null;
  quadrant: TaskQuadrant;
  isBigRock: boolean;
  isCompleted: boolean;
  completedAt: string | null;
  sortOrder: number;
  protectionStatus: TaskProtectionStatus;
  confidence: number | null;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
}
```

- `CreateTaskInput`: `roleId`, `title`, optional `deadline`, optional `quadrant` default `Q2`, optional `isBigRock` default false.
- `UpdateTaskInput`: optional `title`, `deadline`, `quadrant`, `isBigRock`; do not include completion/reorder behavior yet.
- `task_delete` 返回 `void`；前端成功后按 id 从本地列表移除或调用 `refetch`。不要返回软删除后的 `Task`，避免前后端测试契约分叉。

### Regression Risks

- **Mock leakage**: leaving `ROLE_TASKS` in `TasksTab` means AC 6 is not met.
- **Wrong migration number**: creating `005_tasks.sql` conflicts with existing migrations; use `013_tasks.sql`.
- **Hard delete**: `DELETE FROM tasks` violates AC 3; deletion must set `deleted_at`.
- **Cross-role bleed**: `TasksTab` must only show tasks for current role; role switch must not briefly show stale tasks.
- **Scope creep**: drag/drop, completion toggles, auto classification, big-rock max count, Q2 protection, all-role task tab are later stories.
- **UI feedback regression**: errors should be inline in the EgoSync UI, not toast/snackbar/window alert.
- **MemoryTab regression**: `RoleWorkspacePanel` changes must not break target memory routing introduced in Story 2.7/2.9.

### Testing Standards

- Frontend: Vitest + React Testing Library under `egosync-app/src/**/*.test.tsx`; service/hook behavior may mock `taskService`.
- Rust: `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1` on Windows for stability.
- Build: `npm --prefix "egosync-app" run build`.
- UI: visible task create/edit/delete flow must be exercised in Tauri app before reporting implementation complete.

### Project Structure Notes

- Existing role UI is under `egosync-app/src/components/role/`; keep `TasksTab` there.
- Existing modal UI is under `egosync-app/src/components/modals/`; keep `TaskModal` there.
- Existing IPC services are under `egosync-app/src/services/`; create `taskService.ts` there rather than calling invoke in components.
- Existing hooks are under `egosync-app/src/hooks/`; create `useTasks.ts` there.
- Existing Rust domain pattern is `models/{domain}.rs` + `db/{domain}.rs` + `commands/{domain}.rs` + `lib.rs` generate_handler registration.

### References

- Story source: `_bmad-output/planning-artifacts/epics.md` → Story 3.1, lines 1337-1371.
- Epic context: `_bmad-output/planning-artifacts/epics.md` → Epic 3 objectives, lines 1333-1336.
- PRD requirements: `_bmad-output/planning-artifacts/prd-egosync.md` → FR-23/FR-24 and task/four-quadrant domain, lines 381-403.
- Architecture data model: `_bmad-output/planning-artifacts/architecture.md` → core DB tables include `tasks`, lines 213-233.
- Architecture IPC/layering rules: `_bmad-output/planning-artifacts/architecture.md` → commands/services/db boundaries, lines 932-939.
- Architecture naming/testing rules: `_bmad-output/planning-artifacts/architecture.md` → naming and tests, lines 495-620.
- UX RoleWorkspacePanel requirements: `_bmad-output/planning-artifacts/ux-design-specification.md` → RoleWorkspacePanel Tasks tab, lines 763-770.
- UX feedback/no toast rule: `_bmad-output/planning-artifacts/ux-design-specification.md` → Feedback Patterns, lines 816-827.
- Existing mock task UI: `egosync-app/src/components/role/TasksTab.tsx:1-32`.
- Existing task modal mock: `egosync-app/src/components/modals/TaskModal.tsx:4-52`.
- Existing workspace tab container: `egosync-app/src/components/role/RoleWorkspacePanel.tsx:64-105`.
- Existing service pattern: `egosync-app/src/services/roleService.ts:4-14`.
- Existing hook loading/error pattern: `egosync-app/src/hooks/useMemories.ts:11-64`.
- Existing Rust command pattern: `egosync-app/src-tauri/src/commands/role.rs:20-120`.
- Existing Rust DB pattern: `egosync-app/src-tauri/src/db/roles.rs:11-253`.
- Current command registration: `egosync-app/src-tauri/src/lib.rs:220-259`.
- Current migrations: `egosync-app/src-tauri/migrations/001_initial_schema.sql` through `012_mcp_server_standard_types.sql`.

## Dev Agent Record

### Agent Model Used

SWE-1.6 / GPT-5.1

### Debug Log References

- `npm run test:frontend -- --run`：通过，18 个 test files / 173 个 tests 全部通过。
- `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" -- --test-threads=1`：未进入编译；Cargo registry 被本机配置替换到 `https://mirrors.ustc.edu.cn/crates.io-index/`，该仓库返回 not found，属于环境依赖解析阻塞。
- `cargo test --offline --manifest-path "egosync-app/src-tauri/Cargo.toml" --lib agent_engine -- --nocapture`：未进入编译；离线 registry index 缺少 lockfile 中的 `tauri 2.11.2`。
- `cargo test --lib agent_engine -- --nocapture`（工作目录 `egosync-app/src-tauri`）：通过，88 个 `agent_engine` 相关测试全部通过；覆盖角色任务 prompt 注入、管家全角色任务摘要、50 条可见限制以及所有未完成任务必须注入。
- `npm run build`：通过，`tsc && vite build` 成功生成 `dist`；此前首次失败于既有测试文件 `GlobalSettingsModal.test.tsx` 使用 `node:fs` / `node:path` 但 `tsconfig.json` 未声明 Node types，补充 `node` type 后通过。
- UI 人工验证：Tauri dev 已启动，当前复用已有 `http://localhost:5173` Vite，并以临时配置跳过重复 `beforeDevCommand` 启动桌面壳；`egosync.exe` 窗口已打开，可供人工验证 create/edit/delete golden path。

### Completion Notes List

- 新增 `tasks` migration、Rust model、DB helper 与 Tauri commands，支持按角色创建、列表、编辑与软删除任务。
- 新增前端 task 类型、IPC service 与 `useTasks` hook，统一由 service 调用 Tauri command，组件不直接 `invoke()`。
- `TaskModal` 改为 create/edit 受控表单，支持 title/quadrant/deadline/isBigRock，保存失败保持弹窗并显示中文错误。
- `TasksTab` 移除 `ROLE_TASKS` mock，改为真实任务 props、按四象限分组、加载/错误/空态和创建/编辑/删除入口。
- `App` / `RoleView` / `RoleWorkspacePanel` 接通任务弹窗上下文与当前角色 task actions，同时保持 MemoryTab tab/计数/跳转回归测试通过。
- 为构建补充 `tsconfig.json` 的 Node types，修复既有测试文件被 `tsc` 编译时无法解析 `node:fs` / `process` 的问题。
- 补充角色/管家聊天 prompt 的任务上下文注入：角色 prompt 注入当前角色任务；管家 prompt 注入所有角色任务；每个角色默认最多展示 50 条，但所有未完成任务必须全部注入，已完成任务只填补剩余名额。

### File List

- `_bmad-output/implementation-artifacts/3-1-task-crud-role-view.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `egosync-app/src-tauri/migrations/013_tasks.sql`
- `egosync-app/src-tauri/src/models/task.rs`
- `egosync-app/src-tauri/src/models/mod.rs`
- `egosync-app/src-tauri/src/db/tasks.rs`
- `egosync-app/src-tauri/src/db/mod.rs`
- `egosync-app/src-tauri/src/commands/task.rs`
- `egosync-app/src-tauri/src/commands/mod.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/types/task.ts`
- `egosync-app/src/services/taskService.ts`
- `egosync-app/src/hooks/useTasks.ts`
- `egosync-app/src/hooks/useTasks.test.tsx`
- `egosync-app/src/components/modals/TaskModal.tsx`
- `egosync-app/src/components/modals/TaskModal.test.tsx`
- `egosync-app/src/components/role/TasksTab.tsx`
- `egosync-app/src/components/role/TasksTab.test.tsx`
- `egosync-app/src/components/role/RoleWorkspacePanel.tsx`
- `egosync-app/src/components/role/RoleView.tsx`
- `egosync-app/src/App.tsx`
- `egosync-app/src-tauri/src/services/agent_engine.rs`
- `egosync-app/tsconfig.json`

### Change Log

- 2026-06-15: Implemented role-scoped task CRUD backend, frontend data chain, modal/task tab wiring, tests, and validation records.
- 2026-06-16: Added role/butler chat task-context prompt injection so configured EgoSync tasks are visible to agents, with 50 visible task limit while preserving all unfinished tasks.
