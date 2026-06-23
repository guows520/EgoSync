---
baseline_commit: 0139d7a5cea8f5df1a202b80f7ba0f5857658226
---

# Story 4.7: 仪表盘接通真实角色状态数据

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 在管家仪表盘一目了然看到所有角色的实时状态,
so that 快速了解哪些角色需要关注。

## 背景与现状（务必先读）

**本 story 是前端为主的 story — 后端新增一个聚合查询命令 `dashboard_get_status`，前端新增 `useDashboard` hook 替换 DashboardTab 中的硬编码 mock 数据（"3 待办"、"2小时前活跃"、`role.status === 'yellow'`）。后端聚合数据来自已有的 roles 表 + tasks 表 + conversations 表，无需新增 DB 迁移。前端复用现有 DashboardTab 卡片样式，仅替换数据源。**

### 已建成的基础（本 story 的接入点）

**角色数据 — 已完成：**
- `db/roles.rs:35-37` — `list_active_roles(pool)` 返回所有 `status = 'active'` 的角色（按 `created_at ASC` 排序）
- `models/role.rs:1-17` — `Role` 结构体含 `energy: i32` 字段（默认 100，实际计算引擎在 Story 4.8）
- `commands/role.rs:69-71` — `role_list` 命令已存在，前端 `roleService.list()` 已调用
- **本 story 不修改角色 CRUD — 只读取角色列表用于仪表盘聚合**

**任务数据 — 已完成：**
- `db/tasks.rs:51-53` — `list_tasks_by_role(pool, role_id)` 返回该角色的所有未删除任务
- `db/tasks.rs:86-104` — `list_tasks_by_owner(pool, 'role', Some(role_id))` 底层查询
- `tasks` 表 schema（`migrations/013_tasks.sql`）：`quadrant`（Q1-Q4）、`is_completed`（0/1）、`deleted_at`（软删除）、`owner_type`（'role'/'butler'）、`role_id`
- **本 story 需新增聚合查询：按 role_id 统计 pending 任务数 + 是否有 Q1 紧急任务**

**对话数据（最近活跃时间）— 已完成：**
- `conversations` 表（`migrations/002_conversations.sql`）：`role_id`、`updated_at`（每次消息插入时更新，见 `db/conversations.rs:71-76`）
- `ConversationsPool` — 独立于主 `SqlitePool`，需在命令层注入
- **本 story 需从 conversations DB 查询每个角色的 `MAX(updated_at)` 作为 last_active_at**

**前端 DashboardTab — 已存在但用 mock 数据：**
- `components/butler/DashboardTab.tsx:1-52` — 当前组件接收 `roles: any[]`，硬编码 "3 待办" 和 "2小时前活跃"，检查 `role.status === 'yellow'`（Role 类型中不存在此值）
- `components/butler/ButlerWorkspacePanel.tsx:123` — `<DashboardTab roles={roles} onViewChange={onViewChange} />` 传入 roles
- `components/butler/ButlerView.tsx:33-34` — ButlerView 接收 `roles: any[]` 并透传给 ButlerWorkspacePanel
- `App.tsx:43` — `roles` state 由 `roleService.list()` 加载，传入 ButlerView
- **本 story 需将 DashboardTab 从接收 `roles` 改为接收 `dashboardStatuses`（或内部调用 `useDashboard` hook）**

**前端通知/对话事件监听：**
- `hooks/useTauriEvent.ts` — Tauri Event 监听 hook（本 story 不需要新事件监听）
- `hooks/useNotifications.ts` — 通知 hook（参考其 hook 模式：loading/error/data 三态）

## Acceptance Criteria

1. **AC1**: Given 用户在 ButlerView 点击"仪表盘"Tab，When 有 3 个活跃角色，Then 显示 3 张角色卡片，每张含：角色图标+名称、能量值百分比+进度条、待办任务数、最近活跃时间

2. **AC2**: Given 角色卡片排序，Then 按视觉优先级排序：1. 有紧急事项（Q1 任务）→ 灰色边框 + 琥珀色"需关注"文字标签 2. 能量值 < 40% → red 边框 + red 进度条 3. 正常 → 标准色调

3. **AC3**: Given 能量值色谱，Then ≥ 70% = 翠绿 `bg-emerald-500` / ≥ 40% = 琥珀 `bg-amber-500` / < 40% = 红色 `bg-red-500`

4. **AC4**: Given 用户点击角色卡片，When 点击，Then 切换到该角色视图（复用 Story 2.2 导航逻辑）

5. **AC5**: Given Rust 后端，Then Tauri command: `dashboard_get_status` 返回所有活跃角色的聚合数据（energy, pending_tasks_count, last_active_at, has_urgent）

6. **AC6**: Given 前端，Then `components/butler/DashboardTab.tsx` 接通真实数据（替换原型硬编码），And `useDashboard()` hook 封装查询，And 复用原型卡片样式

## Tasks / Subtasks

- [x] **Task 1: Rust model 层 — DashboardStatus 结构体** (AC: #5)
  - [x] 1.1 新建 `models/dashboard.rs`：定义 `DashboardStatus` 结构体：
    ```rust
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DashboardStatus {
        pub role_id: String,
        pub role_name: String,
        pub role_icon: String,
        pub role_color: String,
        pub energy: i32,
        pub pending_tasks_count: i64,
        pub last_active_at: Option<String>,
        pub has_urgent: bool,
    }
    ```
  - [x] 1.2 在 `models/mod.rs` 注册 `dashboard` 模块

- [x] **Task 2: Rust DB 层 — 聚合查询函数** (AC: #5)
  - [x] 2.1 复用 `db::roles::list_active_roles(pool)` 获取所有活跃角色基本信息（无需新增 `list_active_role_ids`）
  - [x] 2.2 在 `db/tasks.rs` 新增 `get_task_stats_for_roles(pool, role_ids) -> Result<HashMap<String, TaskStats>, AppError>` — 一次 GROUP BY 批量查询所有角色的 pending 任务数和 Q1 紧急任务数（优化：合并 2.2 + 2.3 为单次批量查询）
  - [x] 2.3 （已合并到 2.2）`urgent_count` 字段统计 Q1 未完成任务数
  - [x] 2.4 在 `db/conversations.rs` 新增 `get_last_active_for_roles(pool, role_ids: &[String]) -> Result<std::collections::HashMap<String, String>, AppError>` — 批量查询每个角色的 `MAX(updated_at)`

- [x] **Task 3: Rust service 层 — 仪表盘聚合服务** (AC: #5)
  - [x] 3.1 新建 `services/dashboard_service.rs`
  - [x] 3.2 在 `services/mod.rs` 注册 `dashboard_service` 模块
  - [x] 3.3 实现 `get_dashboard_status(pool: &SqlitePool, conv_pool: &ConversationsPool) -> Result<Vec<DashboardStatus>, AppError>`：
    - 调用 `db::roles::list_active_roles(pool)` 获取所有活跃角色基本信息
    - 提取 `role_ids: Vec<String>` 用于批量查询对话最近活跃时间
    - 调用 `db::conversations::get_last_active_for_roles(conv_pool, &role_ids)` 批量获取 last_active_at
    - 调用 `db::tasks::get_task_stats_for_roles(pool, &role_ids)` 批量获取 pending 任务数和紧急任务数
    - 组装 `DashboardStatus` 列表，按视觉优先级排序：has_urgent 优先 → energy < 40 次之 → 其余按 created_at ASC
    - 批量查询失败只 `tracing::warn!`，不阻塞其他角色

- [x] **Task 4: Rust Tauri 命令 — dashboard_get_status** (AC: #5)
  - [x] 4.1 新建 `commands/dashboard.rs`，实现 `dashboard_get_status` 命令：
    ```rust
    #[tauri::command]
    pub async fn dashboard_get_status(
        pool: State<'_, DbPool>,
        conv_pool: State<'_, ConversationsPool>,
    ) -> Result<Vec<DashboardStatus>, AppError> {
        services::dashboard_service::get_dashboard_status(&pool, &conv_pool).await
    }
    ```
  - [x] 4.2 在 `commands/mod.rs` 注册 `dashboard` 模块
  - [x] 4.3 在 `lib.rs` 的 `invoke_handler` 注册 `commands::dashboard::dashboard_get_status`

- [x] **Task 5: 前端类型 + Service + Hook** (AC: #6)
  - [x] 5.1 新建 `types/dashboard.ts`，定义 `DashboardStatus` 接口：
    ```typescript
    export interface DashboardStatus {
      roleId: string;
      roleName: string;
      roleIcon: string;
      roleColor: string;
      energy: number;
      pendingTasksCount: number;
      lastActiveAt: string | null;
      hasUrgent: boolean;
    }
    ```
  - [x] 5.2 新建 `services/dashboardService.ts`：`getStatus: () => invoke<DashboardStatus[]>('dashboard_get_status')`
  - [x] 5.3 新建 `hooks/useDashboard.ts`：封装 `useState` + `useEffect` 调用 `dashboardService.getStatus()`，返回 `{ statuses, isLoading, error, clearError }`。参考 `useNotifications.ts` 的三态模式

- [x] **Task 6: 前端 — DashboardTab 接通真实数据** (AC: #1, #2, #3, #4, #6)
  - [x] 6.1 修改 `DashboardTab.tsx`：不再接收 `roles` prop，改为内部调用 `useDashboard()` hook 获取真实聚合数据
  - [x] 6.2 渲染逻辑改为遍历 `statuses` 数组，每张卡片使用 `DashboardStatus` 字段：
    - 图标+名称：`status.roleIcon` → `getRoleIconComponent()`，`status.roleName`
    - 能量值：`status.energy` 百分比 + 进度条（复用现有能量条样式）
    - 待办数：`status.pendingTasksCount`（替换硬编码 "3 待办"）
    - 最近活跃：`status.lastActiveAt` → 格式化为相对时间（如"2小时前活跃"），null 时显示"暂无活动"
    - 紧急标记：`status.hasUrgent` → 灰色边框 + 琥珀色"需关注"文字标签（替换 `role.status === 'yellow'`）
  - [x] 6.3 能量值色谱保持现有逻辑：`energy >= 70 ? emerald : energy >= 40 ? amber : red`
  - [x] 6.4 卡片点击 `onViewChange(status.roleId)` 切换角色视图
  - [x] 6.5 Loading 状态：显示"加载中…"文案
  - [x] 6.6 Error 状态：显示"仪表盘数据加载失败，请稍后再试"
  - [x] 6.7 空状态：显示"暂无角色数据"

- [x] **Task 7: 前端 — ButlerWorkspacePanel 适配** (AC: #6)
  - [x] 7.1 `ButlerWorkspacePanel.tsx:123` — DashboardTab 不再需要 `roles` prop，移除传参（保留 `onViewChange`）
  - [x] 7.2 确认 DashboardTab 在 `currentTab === 'dashboard'` 时渲染正常

- [x] **Task 8: Rust 单元测试** (AC: #1-#5)
  - [x] 8.1 `db/tasks.rs` 测试 — `get_task_stats_for_roles`：统计 pending 任务数和 urgent 任务数 / 已完成任务不计 / 空列表返回空
  - [x] 8.2 （已合并到 8.1）`urgent_count` 字段验证 Q1 未完成任务数
  - [x] 8.3 `db/conversations.rs` 测试 — `get_last_active_for_roles`：多角色批量查询 / 无对话记录的角色不出现 / 取最新 updated_at / 空列表返回空
  - [x] 8.4 `services/dashboard_service.rs` 测试 — `get_dashboard_status`：
    - 无角色时返回空列表
    - 有角色时返回正确聚合数据（pending count + last_active_at）
    - has_urgent 角色排在最前，low energy 次之

- [x] **Task 9: 前端测试** (AC: #1, #6)
  - [x] 9.1 `useDashboard.test.ts` — hook 返回 loading/data/error 三态
  - [x] 9.2 `DashboardTab.test.tsx` — 渲染真实数据、卡片点击触发 onViewChange、loading/error/empty 状态、hasUrgent 标签、lastActiveAt null

- [x] **Task 10: 更新 sprint-status.yaml**
  - [x] 10.1 将 `4-7-dashboard-real-role-status` 状态更新为 `done`

## Dev Notes

### 项目背景

本 Story 属于 Epic 4（主动循环、通知与仪表盘），是仪表盘从 mock 数据到真实数据的接通 story。Story 4.1-4.6 已完成后台调度器、主动建议、通知系统和 Q2 保护提醒。本 Story 聚焦于前端仪表盘的数据接通，后端仅需一个聚合查询命令。

### 技术栈

- **后端**: Rust + Tauri 2.x + SQLx (SQLite) + tokio
- **前端**: React 18 + TypeScript 5.2 + TailwindCSS + Vite 5
- **IPC**: Tauri Commands (invoke)

### 关键架构约束

**Rust 三层架构（严格遵守）**：
- `commands/dashboard.rs` — Tauri 命令接口，薄层，只注入 `DbPool` + `ConversationsPool` → 调用 service → 返回结果
- `services/dashboard_service.rs` — 业务逻辑（聚合角色 + 任务 + 对话数据，排序）
- `db/roles.rs` — 复用 `list_active_roles`（不新增冗余查询）
- `db/tasks.rs` — 新增 `get_task_stats_for_roles` 批量聚合查询（合并 pending count + urgent count）
- `db/conversations.rs` — 新增 `get_last_active_for_roles` 批量查询
- `models/dashboard.rs` — `DashboardStatus` 数据结构定义

**IPC 命令命名规范**: `dashboard_get_status`（下划线分隔，参考 `task_list_all` / `role_list` 模式）

**serde 序列化**: 所有 Rust struct 使用 `#[serde(rename_all = "camelCase")]`，前端 TypeScript 接口字段使用 camelCase

**ConversationsPool 获取方式**: `commands/dashboard.rs` 注入 `State<'_, ConversationsPool>`（参考 `commands/role.rs:161` 的 `conv_pool: State<'_, ConversationsPool>` 模式）

### 已有代码复用（关键 — 避免重复造轮子）

**1. `db::roles::list_active_roles` 已实现**：`db/roles.rs:35-37`。但本 story 需要更轻量的查询（只需 id/name/icon/color/energy），可新增 `list_active_role_ids` 或直接复用 `list_active_roles` 后在 service 层提取所需字段。**推荐复用 `list_active_roles`**，避免新增冗余查询函数 — 虽然多取了几个字段，但省去新增 DB 函数的维护成本。

**2. `CrossRoleTask` 查询模式参考**：`db/tasks.rs:59-84` 的 `list_all_tasks` 已展示 JOIN roles + 按 role_id 查询的模式。本 story 的聚合查询可参考此模式。

**3. `ConversationsPool` 跨库查询参考**：`db/conversations.rs:32-46` 的 `get_or_create_butler_conversation` 已展示在 conversations DB 上查询的模式。`get_last_active_for_roles` 遵循同样模式，只是改为聚合查询。

**4. `AppError` 枚举**：`error.rs` 已定义 NotFound/DbError/ValidationError 等变体，复用这些变体，不新增。

**5. 前端 hook 三态模式参考**：`hooks/useNotifications.ts` 已实现 loading/error/data 三态模式。`useDashboard` 遵循同样模式。

**6. 前端 service 层模式参考**：`services/roleService.ts` / `services/taskService.ts` 已展示 `invoke<T>()` 封装模式。`dashboardService` 遵循同样模式。

**7. `getRoleIconComponent` + `normalizeColorHex`**：`lib/roleIcons.ts:99-122` 已实现图标和颜色解析。DashboardTab 已在使用，保持不变。

**8. 能量条样式**：DashboardTab 现有能量条样式（`bg-emerald-500` / `bg-amber-500` / `bg-red-500`）已符合 AC3，只需确保数据源切换后样式逻辑不变。

### 聚合查询设计

**方案选择：批量查询 vs 逐角色查询**

推荐批量查询策略，减少 DB 往返：

1. 一次查询所有 active 角色基本信息（`list_active_roles`）
2. 一次批量查询所有角色的最近活跃时间（`get_last_active_for_roles`，IN 子句）
3. 逐角色查询 pending_tasks_count + has_urgent（这两个查询无法轻易批量化为单条 SQL，因为需要按 role_id 分组统计）

**优化方案**：可将 #3 改为一条 GROUP BY 查询：
```sql
SELECT role_id, COUNT(*) as pending_count,
       SUM(CASE WHEN quadrant = 'Q1' THEN 1 ELSE 0 END) as urgent_count
FROM tasks
WHERE owner_type = 'role' AND is_completed = 0 AND deleted_at IS NULL
  AND role_id IN (?, ?, ...)
GROUP BY role_id
```
这样只需一次查询获取所有角色的任务统计。**推荐此方案** — 新增 `get_task_stats_for_roles(pool, role_ids: &[String]) -> Result<HashMap<String, TaskStats>, AppError>`。

### 排序逻辑

在 service 层完成排序，前端直接渲染：

1. `has_urgent = true` 的角色排最前
2. `energy < 40` 的角色排次
3. 其余按 `created_at ASC`（与 `list_active_roles` 的默认排序一致）

### 最近活跃时间格式化

前端需将 ISO 8601 时间戳格式化为相对时间（如"2小时前活跃"）。**推荐方案**：在 `DashboardTab.tsx` 中实现简单的相对时间格式化函数（不引入额外依赖）：
- < 1 分钟 → "刚刚活跃"
- < 60 分钟 → "X分钟前活跃"
- < 24 小时 → "X小时前活跃"
- < 30 天 → "X天前活跃"
- < 12 个月 → "X个月前活跃"
- ≥ 12 个月 → "X年前活跃"
- null → "暂无活动"

### Previous Story Intelligence（Story 4.6 + 4.5 + 4.4）

- **4.6 ConversationsPool 传递模式**：`services/q2_protection_reminder.rs` 中 `check_and_generate_reminders` 同时接收 `pool: &SqlitePool` 和 `conv_pool: &ConversationsPool`。本 story 的 `get_dashboard_status` 遵循同样模式。
- **4.5 前端 hook 模式**：`useNotifications.ts` 的 loading/error/data 三态 + 事件监听。`useDashboard` 不需要事件监听（仪表盘数据在打开 Tab 时拉取即可），也不需要 `refresh` 函数 — DashboardTab 在 Tab 切换时随条件渲染重新挂载，useEffect 每次打开即重拉，自动重载已覆盖手动刷新需求（决策 2B）。
- **4.4 前端组件数据接通模式**：`ActionCard.tsx` 从 mock 数据接通真实 suggestion 数据。DashboardTab 遵循同样模式：替换 mock 字段为真实数据。
- **4.6 测试命令**：`cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` — 本 story 新增测试后应维持 0 failed。
- **4.6 前端测试**：`npm run test:frontend` 有 4 个既有失败（SettingsTab/TasksTab），与本 story 无关。

### Git Intelligence

- `0139d7a`（HEAD）— feat(4.6): Q2 保护管家提醒 + 通知中心已读折叠（最新提交，本 story 的直接前置）
- `1b21b4b` — feat: 三级通知系统（耳语/轻触/敲门）完整实现
- `823edbb` — feat(4.4): embed action cards in chat stream + custom reject reason
- 范本来源：`commands/role.rs`（命令 + ConversPool 注入模式）、`services/notification_service.rs`（service 层模式）、`hooks/useNotifications.ts`（hook 三态模式）

### Testing Requirements

- **Rust 单测**（`db/tasks.rs` ~2 个测试，`db/conversations.rs` ~2 个测试，`services/dashboard_service.rs` ~3 个测试）：
  - `count_pending_tasks_by_role` / `has_urgent_tasks`：正确统计 + 排除已完成/已删除/butler 任务
  - `get_last_active_for_roles`：多角色批量查询 + 无对话记录返回 null
  - `get_dashboard_status`：无角色空列表 + 有角色正确聚合 + has_urgent 排序 + 单个失败不阻塞
- **前端测试**：
  - `useDashboard.test.ts` — hook 三态（loading/data/error）
  - `DashboardTab.test.tsx` — 渲染真实数据 + 卡片点击 + loading/error/empty
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`
  - `npm --prefix "egosync-app" run build`
  - `npm --prefix "egosync-app" run test:frontend`

### Project Structure Notes

- **新增文件（7）**：
  - `egosync-app/src-tauri/src/models/dashboard.rs` — DashboardStatus 数据模型
  - `egosync-app/src-tauri/src/services/dashboard_service.rs` — 仪表盘聚合服务
  - `egosync-app/src-tauri/src/commands/dashboard.rs` — Tauri 命令
  - `egosync-app/src/types/dashboard.ts` — 前端类型定义
  - `egosync-app/src/services/dashboardService.ts` — 前端 service
  - `egosync-app/src/hooks/useDashboard.ts` — 前端 hook
  - `egosync-app/src/hooks/useDashboard.test.ts` — hook 测试
- **修改文件（5）**：
  - `egosync-app/src-tauri/src/models/mod.rs` — 注册 `dashboard` 模块
  - `egosync-app/src-tauri/src/services/mod.rs` — 注册 `dashboard_service` 模块
  - `egosync-app/src-tauri/src/commands/mod.rs` — 注册 `dashboard` 模块
  - `egosync-app/src-tauri/src/lib.rs` — 注册 `dashboard_get_status` 命令
  - `egosync-app/src-tauri/src/db/tasks.rs` — 新增 `count_pending_tasks_by_role` + `has_urgent_tasks`（或 `get_task_stats_for_roles`）
  - `egosync-app/src-tauri/src/db/conversations.rs` — 新增 `get_last_active_for_roles`
  - `egosync-app/src/components/butler/DashboardTab.tsx` — 接通真实数据
  - `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx` — 移除 DashboardTab 的 roles prop
  - `egosync-app/src/components/butler/DashboardTab.test.tsx` — 新增或更新测试
- **无新增 DB 迁移** — 所有数据来自已有表（roles, tasks, conversations）
- **无新增依赖** — 复用现有 Tauri/SQLx/React 依赖
- 符合项目规则：Rust 三层、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块 snake_case

### References

- `_bmad-output/project-context.md`（Rust 三层 / serde camelCase / 无 unwrap / tracing / 数据边界 / 前端 service 层模式）
- `_bmad-output/planning-artifacts/epics.md:1801-1833`（Story 4.7 定义）
- `_bmad-output/planning-artifacts/epics.md:199`（UX-DR16 反馈模式约束）
- `_bmad-output/planning-artifacts/epics.md:201`（UX-DR18 空状态文案约束）
- `_bmad-output/planning-artifacts/epics.md:202`（UX-DR19 加载状态约束）
- `_bmad-output/implementation-artifacts/4-6-q2-protection-butler-reminder.md`（Story 4.6 — ConversationsPool 传递模式 + 前端事件刷新模式）
- `_bmad-output/implementation-artifacts/4-5-three-tier-notification.md`（Story 4.5 — useNotifications hook 三态模式参考）
- `egosync-app/src/components/butler/DashboardTab.tsx:1-52`（当前 mock 数据实现 — 需替换）
- `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:123`（DashboardTab 调用点）
- `egosync-app/src/components/butler/ButlerView.tsx:33-34`（roles prop 传递链）
- `egosync-app/src/App.tsx:43,99-103`（roles state 加载）
- `egosync-app/src-tauri/src/db/roles.rs:35-37`（`list_active_roles` — 可复用）
- `egosync-app/src-tauri/src/db/tasks.rs:51-53`（`list_tasks_by_role` — 查询模式参考）
- `egosync-app/src-tauri/src/db/tasks.rs:59-84`（`list_all_tasks` — JOIN + 聚合查询模式参考）
- `egosync-app/src-tauri/src/db/conversations.rs:32-46`（ConversationsPool 查询模式参考）
- `egosync-app/src-tauri/src/db/conversations.rs:71-76`（`updated_at` 更新时机）
- `egosync-app/src-tauri/src/db/pool.rs:8-11`（`DbPool` = `SqlitePool`，`ConversationsPool` 定义）
- `egosync-app/src-tauri/src/commands/role.rs:157-172`（`role_delete` — ConversPool 注入模式参考）
- `egosync-app/src-tauri/src/models/role.rs:1-17`（`Role` 结构体 — energy 字段）
- `egosync-app/src-tauri/src/models/mod.rs`（模型模块注册）
- `egosync-app/src-tauri/src/services/mod.rs`（service 模块注册）
- `egosync-app/src-tauri/src/commands/mod.rs`（命令模块注册）
- `egosync-app/src-tauri/src/lib.rs:278-359`（`invoke_handler` — 注册新命令）
- `egosync-app/src-tauri/src/error.rs:1-16`（`AppError` 枚举）
- `egosync-app/src-tauri/migrations/003_roles.sql`（roles 表 schema）
- `egosync-app/src-tauri/migrations/013_tasks.sql`（tasks 表 schema）
- `egosync-app/src-tauri/migrations/002_conversations.sql`（conversations 表 schema）
- `egosync-app/src/hooks/useNotifications.ts`（hook 三态模式参考）
- `egosync-app/src/services/roleService.ts`（service 层模式参考）
- `egosync-app/src/lib/roleIcons.ts:99-122`（`getRoleIconComponent` + `normalizeColorHex`）

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Review Findings (2026-06-23)

- [x] [Review][Decision→Patch] 相对时间格式优化（决策 1B）— 保留分钟级精度，仅修掉超 30 天显示本地日期串的别扭文案。`formatRelativeTime` 超 30 天改为"X个月前活跃"、超 12 个月"X年前活跃"。已修复 [egosync-app/src/components/butler/DashboardTab.tsx:7-23]。
- [x] [Review][Decision→Note] `useDashboard` 不补 refresh（决策 2B）— 有意省略。DashboardTab 在 Tab 切换时随条件渲染重新挂载（`ButlerWorkspacePanel.tsx:123` 用 `&&`），useEffect 每次打开即重拉，自动重载已覆盖手动刷新需求。Dev Notes/Testing Requirements 中的 refresh 描述以 Task 5.3 的返回结构（不含 refresh）为准。
- [x] [Review][Patch] lastActiveAt 为 null 时显示"暂无活动活跃" [egosync-app/src/components/butler/DashboardTab.tsx:76] — 已修复：将"活跃"后缀移入 `formatRelativeTime` 仅作用于相对时间分支，null 分支返回"暂无活动"；同步更新 `DashboardTab.test.tsx:142` 断言。
