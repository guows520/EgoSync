---
baseline_commit: 76a65ac
---

# Story 3.7: 管家视图通用任务 Tab 汇总所有角色的任务

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 在管家视角一站式查看所有活跃角色 + 管家自己的任务（按四象限分区、每条标注归属、可按角色/象限/大石头筛选、可新建与就地编辑）,
so that 不用逐个切换角色就能掌握全局，并能就地快速处理任意一条任务。

## 背景与现状（务必先读）

**本 story 是全栈 story（Rust 后端新增 1 个命令 + 前端新增「任务概览」视图，含就地编辑/完成/删除），不是纯前端。**

管家面板现有「通用任务」Tab（`ButlerWorkspacePanel.tsx:102-104`）当前渲染**管家自己的任务**——`useTasks({ ownerType: 'butler' })`（`:45`）+ 复用角色版 `<TasksTab>`（`:118-130`）。

Story 3.7 将其**改造为「任务概览」**：标签改名「通用任务」→「任务概览」；内容改为**所有活跃角色 + 管家自己的任务**，按**四象限分区**展示（Q1→Q4 小标题；**不**按角色分组、组内跨 owner 混排；已完成默认折叠），每条任务**就近标注归属**（角色色点 + 名称；管家则标「管家」），并提供**角色多选 / 象限 / 仅大石头**筛选；管家可在概览里**新建任务（选归属、默认管家）并就地编辑任意任务**（保存自动同步到原角色、**不跳转**）：

| AC | 需求 | 实际状态 |
|----|------|----------|
| AC1 四象限分区（不按角色分组） | 保留 Q1→Q4 小标题；组内跨 owner 混排，不按角色分组 | ✅ 已实现：`TaskOverviewTab` 按 Q1→Q4 分区，组内跨 owner 展示 |
| AC2 每条标注归属 | 卡片就近显示「色点 + 角色名」；管家任务标「管家」 | ✅ 已实现：角色任务显示 `roleName`/`roleColor`，管家固定「管家」 |
| AC3 组内排序 + 完成折叠 | 各象限内：未完成在上（大石头→sortOrder）；已完成默认折叠 | ✅ 已实现：未完成优先、已完成默认折叠；概览内额外按 owner 名称稳定排序 |
| AC4 角色多选筛选 | 可选 1 个或多个角色（含「管家」）过滤 | ✅ 已实现：归属 chips 前端过滤，支持全选/全不选/多选 |
| AC5 象限 + 大石头筛选 | 象限（全部/Q1/Q2/Q3/Q4）+ 仅大石头 | ✅ 已实现：象限/大石头状态上提到 `ButlerWorkspacePanel`，经 `useAllTasks` 服务端过滤 |
| AC6 新建 + 就地编辑 | 顶部「+新建」（选归属、默认管家）；点任意卡就地编辑，保存即同步、**不跳转** | ✅ 已实现：App 级 `TaskModal` 复用，Butler 注册 `useAllTasks` actions，保存后刷新概览 |
| AC7 空态 | 「所有角色都很轻松，可以考虑添加新目标」 | ✅ 已实现：全空与筛选空分别显示不同文案 |
| AC8 Rust `task::list_all` | 返回**全 owner**任务（角色含 roleName/roleColor，管家为 null），支持 quadrant?/isBigRock? | ✅ 已实现：`task_list_all` + `db::tasks::list_all_tasks` |
| AC9 `useAllTasks(filters)` hook | 封装跨 owner 查询 | ✅ 已实现：含稳定 `filterKey`、分类事件处理、CRUD actions 与错误兜底 |

> **关键设计决策（已采纳，见文末「待确认问题」可回退）**：
> - **D1（按 boss 指示更新）**：「通用任务」改名「任务概览」，内容为**所有活跃角色 + 管家自己**的任务，**按四象限分区**（Q1→Q4 小标题）、组内**跨 owner 混排**（不按角色分组），每条**就近标注归属**。管家任务**并入**概览。
> - **D2（按 boss 指示更新）**：`task::list_all` 返回 **未删除** 的任务（**含已完成**，已完成在概览各象限内**默认折叠**、仿角色 `TasksTab`）：角色任务限 `status='active'` 的角色，管家任务**全部并入**。**排除归档角色任务**。
> - **D3（按 boss 指示更新）**：概览**可新建 + 可编辑**——点任意任务卡弹出 `TaskModal` 就地编辑（保存经 `taskService.update` 按 id 落库、自动归属原 owner、`refetch`、**不跳转**），并支持完成切换、删除；顶部有「+ 新建任务」，新建时**可选归属角色、默认「管家」**（经 `taskService.create`）。**不**支持拖拽排序（跨 owner 无意义）。
> - **D4**：象限 + 仅大石头 = **服务端过滤**（`task::list_all` 参数，贴合 epic 契约）；**角色多选 = 前端过滤**（数据集小、交互即时、避免动态 IN 子句）。
> - **D5**：按 AC 由**后端富化** roleName/roleColor（`CrossRoleTask`，管家任务该两字段为 null），而非前端用 `roles` 数组本地 join。

## Acceptance Criteria

> 既有行为（角色 TasksTab 全部交互、管家记忆/仪表盘/设置 Tab、TaskModal 角色场景）必须零回归。

1. **Rust 命令 `task_list_all`（AC8）**
   - **Then** 新增 `#[tauri::command] task_list_all(quadrant: Option<String>, is_big_rock: Option<bool>, pool) -> Result<Vec<CrossRoleTask>, AppError>`，注册进 `lib.rs` 的 `generate_handler!`
   - **And** 委托 `db::tasks::list_all_tasks(&pool, quadrant, is_big_rock)`
   - **And** `quadrant` 非空时校验 ∈ {Q1,Q2,Q3,Q4}（复用 `validate_quadrant`），非法返回 `ValidationError`

2. **DB 查询 `list_all_tasks`（AC8/D2）**
   - **Given** tasks **LEFT JOIN** roles（角色任务取 name/color；管家任务 `role_id=null` 仍需返回）
   - **Then** 返回 `deleted_at IS NULL`（**含已完成**）且满足「`owner_type='butler'` **或**（`owner_type='role'` AND `roles.status='active'`）」的任务，附 `roles.name AS role_name`、`roles.color AS role_color`（管家任务两者为 NULL）
   - **And** 可选过滤：`quadrant` 提供则 `quadrant = ?`；`is_big_rock` 提供则 `is_big_rock = ?`（`None` 不过滤）
   - **And** 排序：`quadrant ASC`（Q1→Q4 字典序成立）→ `is_completed ASC`（未完成在前）→ `is_big_rock DESC` → `owner_type ASC` → `sort_order ASC`
   - **And** **包含已完成**（前端按象限折叠展示）、**排除归档角色任务**；**包含**管家自有任务

3. **`CrossRoleTask` 类型（前后端对齐）**
   - **Then** Rust `models/task.rs` 新增 `CrossRoleTask`（`#[derive(FromRow, Serialize)]` + `#[serde(rename_all="camelCase")]`）：含 `Task` 全字段 + `role_name: Option<String>` + `role_color: Option<String>`
   - **And** 前端 `types/task.ts` 新增 `CrossRoleTask extends Task { roleName: string | null; roleColor: string | null }`

4. **`useAllTasks(filters)` hook（AC9）**
   - **Given** `filters: { quadrant?: TaskQuadrant; isBigRock?: boolean }`（仅服务端过滤项；**角色多选不进此 hook**，由组件本地过滤）
   - **Then** 新增 `hooks/useAllTasks.ts`：调用 `taskService.listAll(filters)`，返回 `{ tasks: CrossRoleTask[], isLoading, error, refetch }`
   - **And** `filters` 变化时重查（依赖归约成稳定 `filterKey`，仿 `useTasks` scopeKey，避免对象引用导致无限重载）
   - **And** 失败 `setError('任务暂时加载失败，请稍后再试')` 且保持空列表（不抛到 UI）

5. **四象限分区 + 组内跨 owner + 完成折叠 + 归属标注（AC1/AC2/AC3）**
   - **Given** 过滤后有任务
   - **When** 打开管家「任务概览」Tab
   - **Then** 按 **Q1→Q4** 渲染分区小标题（沿用 `quadrantLabels`）；**每个象限内不按角色分组**，跨 owner 混排
   - **And** 各象限内：**未完成**任务在上（按 大石头→`sortOrder`）；**已完成**任务收进该象限的「已完成」子区、**默认折叠**（仿角色 `TasksTab` 的 `expandedCompleted`/`CollapsibleSection`）
   - **And** 每张卡就近显示归属标签：角色任务 = 角色色点（`roleColor`）+ 角色名（`roleName`）；管家任务 = 固定「管家」+ 管家色 `#6366F1`；卡含 `deadline`/`大石头`/`at_risk` 既有 badge（复用 `TasksTab` 视觉，**不**复用可拖拽 `SortableTaskCard`）
   - **And** 某象限无任何任务（未完成+已完成均空）时可隐藏该象限分区（概览经筛选，避免空标题堆叠）

6. **筛选器（AC4/AC5）**
   - **Then** 顶部渲染：象限 chips `全部/Q1/Q2/Q3/Q4`（单选，默认全部）+「仅大石头」开关（默认关）→ 经 `useAllTasks` **服务端**过滤
   - **And** 渲染**角色多选**筛选：所有活跃角色 + 「管家」（多选，默认全选/不限）→ **前端**按所选 owner 过滤返回列表
   - **And** 控件可访问：`<button aria-pressed>`，键盘可达，`motion-reduce` 友好
   - **And** 角色多选与象限/大石头**叠加生效**（先服务端 quadrant/bigRock，再前端 owner 过滤）

7. **新建 + 就地编辑 + 完成 + 删除（AC6）**
   - **Given** 概览中一张任务卡（角色任务或管家任务）
   - **When** 用户点击编辑按钮
   - **Then** 弹出 `TaskModal`（编辑模式，预填该任务），**不切换视图、不跳转**
   - **And** 保存调用 `taskService.update(task.id, input)`（后端按 id 更新，`role_id`/`owner_type` 不变 → 自动同步到原角色/管家）；**成功后**才 `useAllTasks.refetch()` 刷新概览
   - **And（大石头上限校验，重点）** 若将某任务改为大石头、而**该任务所属角色（或管家）已满 3 个大石头**，`task_update` 返回 `ValidationError`「每个任务清单每周最多 3 个大石头，请先取消一个再标记」；概览的 `onSave` **不得 try/catch 吞错**，让其**冒泡回 `TaskModal`** 由弹窗在管家界面直接提示，弹窗保持打开、**不** refetch、不写库。上限按**被编辑任务的 owner** 计（改产品角色的任务就校验产品角色的大石头数，与「当前在管家视角」无关）——`update_task` 已实现此校验（`db/tasks.rs:86-91` 用 `existing.owner_type`/`existing.role_id`），**本 story 无需改后端**（如需提示里带上具体角色名，是可选小增强）
   - **And** 卡片提供完成切换（`taskService.toggleComplete` → refetch；完成后该卡**移入所在象限的「已完成」折叠子区**（不再消失）；**完成路径不触发大石头上限校验**，方案 D 会自动清大石头）
   - **And** 卡片提供删除（`taskService.delete`，带二次确认 → refetch）
   - **And** 其它保存/删除失败同样以友好中文错误呈现（复用 `TaskModal`/既有错误文案），不破坏列表
   - **And（新建任务）** 概览顶部「+ 新建任务」→ 打开 `TaskModal`（新建模式）并提供**归属选择**：「管家」**默认** + 各活跃角色；选中决定 `CreateTaskInput` 的 `ownerType`/`roleId`，保存经 `taskService.create` → 成功后 refetch；新建同样受**大石头上限**与 `ValidationError` 冒泡约束；未选象限走「智能判断」（`task_create` 后台异步分类，`useAllTasks` 监听 `task:classified` 刷新）

8. **空态 / 加载 / 错误（AC7）**
   - **Given** 过滤后结果为空
   - **Then** 无任何任务时显示「所有角色都很轻松，可以考虑添加新目标」；仅因角色多选过滤掉全部时显示「当前筛选无匹配任务」
   - **And** 加载中显示占位、失败显示错误文案（复用既有模式）

9. **零回归（既有功能）**
   - **Then** 角色侧 `TasksTab`/`RoleView`/`RoleWorkspacePanel` 原有任务能力保持不回归（拖拽、完成、分类中、at_risk、四象限折叠、大石头排序仍可用）
   - **And** 后续 UX 修复已触碰 `TasksTab`/`RoleView`：角色任务顶部改为上下布局；角色右侧面板动画改为单一宽度/透明度过渡，并补充测试
   - **And** 管家「仪表盘 / 管家记忆 / 管家设置」三个 Tab 行为不变；App 级 `TaskModal` 的角色创建/编辑路径保持兼容
   - **And** 现有全部前端测试 + Rust 测试保持通过（涉及 mock 调整的测试同步更新，但断言语义不削弱）

## Tasks / Subtasks

- [x] Rust：DB 层全 owner 查询（AC: 2, 3）
  - [x] `src-tauri/src/models/task.rs` 新增 `CrossRoleTask`（FromRow + camelCase serde，含 Task 全字段 + `role_name: Option<String>` + `role_color: Option<String>`）
  - [x] `src-tauri/src/db/tasks.rs` 新增 `pub async fn list_all_tasks(pool, quadrant: Option<&str>, is_big_rock: Option<bool>) -> Result<Vec<CrossRoleTask>, AppError>`：`SELECT t.<cols>, r.name AS role_name, r.color AS role_color FROM tasks t LEFT JOIN roles r ON t.role_id = r.id WHERE t.deleted_at IS NULL AND (t.owner_type='butler' OR r.status='active') AND (?quadrant IS NULL OR t.quadrant=?quadrant) AND (?is_big_rock IS NULL OR t.is_big_rock=?is_big_rock) ORDER BY t.quadrant ASC, t.is_completed ASC, t.is_big_rock DESC, t.owner_type ASC, t.sort_order ASC`（**含已完成**；绑定写法见 Dev Notes，避免布尔 NULL 坑）

- [x] Rust：命令层 + 注册（AC: 1）
  - [x] `src-tauri/src/commands/task.rs` 新增 `task_list_all`（quadrant 非空时 `validate_quadrant`）
  - [x] `src-tauri/src/lib.rs` 在 `generate_handler!` 加入 `commands::task::task_list_all`

- [x] Rust：单元测试（AC: 2, 3）
  - [x] `db/tasks.rs` 既有 `#[cfg(test)] mod tests`（已有 `setup_test_db` 建 role-a/role-b）补用例：包含 butler 任务、角色任务带 role_name/role_color、管家任务 role_name/color 为 None、quadrant 过滤、is_big_rock 过滤、**包含已完成**、排除归档角色任务、排序（象限→未完成在前→大石头→sortOrder）正确

- [x] 前端：类型 + service（AC: 3, 4）
  - [x] `src/types/task.ts` 新增 `CrossRoleTask`（roleName/roleColor 可空）+ `AllTasksFilter`（`{ quadrant?: TaskQuadrant; isBigRock?: boolean }`）
  - [x] `src/services/taskService.ts` 新增 `listAll: (filter) => invoke<CrossRoleTask[]>('task_list_all', { quadrant: filter.quadrant ?? null, isBigRock: filter.isBigRock ?? null })`

- [x] 前端：`useAllTasks` hook（AC: 4）
  - [x] 新建 `src/hooks/useAllTasks.ts`：依赖 `filterKey = `${quadrant ?? 'all'}:${isBigRock ? '1' : '0'}`` 防无限重载；返回 `{ tasks, isLoading, error, refetch }`
  - [x] 监听 `task:classified` 事件（仿 `useTasks`）→ `refetch()`，使新建任务的异步分类结果在概览中更新（否则新建后象限/排序短暂滞后）

- [x] 前端：任务概览组件（AC: 5, 6, 7, 8）
  - [x] 新建 `src/components/butler/TaskOverviewTab.tsx`：接收 `roles`、`tasks`、加载/错误状态、分类中集合和任务操作回调，负责概览 UI 展示与本地筛选
  - [x] 顶部：象限筛选、归属筛选（全部/管家/角色，横向细滚动条）+「只看大石头」开关 +「新增任务」按钮
  - [x] 派生 owner 标识：角色任务→`roleName`/`roleColor`；管家任务→`管家`/`#6366F1`；按所选归属前端过滤
  - [x] 按 **Q1→Q4 分区**渲染；各象限内未完成在上、**已完成默认折叠**；卡含归属标签 + deadline/大石头/at_risk badge；**不**用 dnd/`SortableTaskCard`
  - [x] 每卡交互：完成圈、编辑按钮、删除按钮（二次确认）
  - [x] 编辑任意任务时从概览打开 App 级 `TaskModal`，使用 Butler 全量任务 actions 更新，避免依赖角色工作区是否已挂载
  - [x] 空态 / loading / error 文案；过滤后为空显示「当前筛选无匹配任务」

- [x] 前端：`TaskModal` 加归属选择（仅新建，AC: 6）
  - [x] `src/components/modals/TaskModal.tsx`：新增可选 `roles?: Role[]`；当处于 Butler scope 且是新建模式时渲染「归属」`<select>`：默认「管家」+ 各活跃角色；选中决定 create 分支 `CreateTaskInput` 的 `ownerType`/`roleId`
  - [x] **向后兼容**：角色视图创建/编辑与编辑模式不受影响

- [x] 前端：管家面板接线改造（AC: 5, 9）
  - [x] `src/components/butler/ButlerWorkspacePanel.tsx`：tasks Tab 改渲染 `<TaskOverviewTab />`，并使用 `useAllTasks` 注册 Butler 全量任务 actions
  - [x] `src/App.tsx`：向 App 级 `TaskModal` 传入 `roles`，支持 Butler 新建任务时选择归属
  - [x] 保留角色视图的 `taskModalContext`/`handleSaveTask`/`role:` 路径不变

> 注：Story 3.7 主实现不需要角色侧任务逻辑；后续 UX 修复已额外触碰 `src/components/role/TasksTab.tsx` 与 `src/components/role/RoleView.tsx`，仅调整角色任务顶部布局与右侧面板动画，原有任务交互保持并由测试覆盖。

- [x] 前端：测试（AC: 5, 6, 7, 8, 9）
  - [x] 新建 `src/components/butler/TaskOverviewTab.test.tsx`：覆盖四象限展示、归属标签、已完成折叠、归属筛选、只看大石头、打开新建/编辑、完成切换、删除确认
  - [x] `src/components/modals/TaskModal.test.tsx` 新增：Butler scope 新建模式渲染归属下拉，选角色后 `onSave` 收到对应 `ownerType`/`roleId`
  - [x] 更新 `src/components/butler/ButlerWorkspacePanel.test.tsx`：mock `TaskOverviewTab` 与 `useAllTasks`；保持记忆 badge 用例通过
  - [x]（可选）`src/hooks/useAllTasks.test.tsx`：filters 变化重查、错误兜底

- [x] 验证（AC: 1-9）
  - [x] `npm --prefix "egosync-app" run test:frontend` 全通过
  - [x] `npm --prefix "egosync-app" run build`（`tsc && vite build`）通过
  - [x] `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` 通过

## Dev Notes

### Current State（基于当前代码 @ 76a65ac）

- **后端任务命令**（`commands/task.rs`）：`task_create / task_list_by_role / task_list_butler / task_update / task_delete / task_reorder / task_toggle_complete / task_check_protection_status`。**无跨角色查询**。`validate_quadrant` 已存在可复用。
- **DB 层**（`db/tasks.rs`）：`TASK_SELECT_COLUMNS`（`:6`）含全字段；`list_tasks_by_owner`（`:59`）是单 owner 查询范本；`get_active_task`（`:264`）；`#[cfg(test)] mod tests` 的 `setup_test_db`（`:517`）已建 `roles`（role-a「产品」/role-b「学习」，color 默认 `#4F46E5`）与 `tasks` 表，并有 `create_and_list_tasks_stays_scoped_to_role`（`:585`）可作新测试范本。
- **roles 表**：`id, name, color, status('active'/'archived'), created_at ...`（`db/roles.rs:9`，`list_active_roles` 按 `status='active'`）。
- **模型**（`models/task.rs`）：`Task`（FromRow, camelCase）。新增 `CrossRoleTask` 时**列顺序必须与 SELECT 顺序一致**（sqlx `FromRow` 按列名匹配，安全，但保持可读）。
- **命令注册**：`lib.rs:265` `generate_handler!`，task 命令在 `:300-307`。
- **前端 hook**（`hooks/useTasks.ts`）：`scopeKey` 把对象 scope 归约成稳定字符串依赖（`:18-22`）——`useAllTasks` 须照此用 `filterKey`，否则对象 filters 每次新引用会无限重载。
- **管家面板**（`ButlerWorkspacePanel.tsx`）：tasks Tab（`:118-130`）现用 `useTasks({ownerType:'butler'})` + `<TasksTab role={{color:'#6366F1'}}>`；`onTasksApiReady('butler')`（`:47-55`）注册 butler 任务 actions 给 App 的 `TaskModal` 保存路径。**本 story 移除这两处 butler 接线**（butler 任务改由 `task_list_all` 并入「任务概览」展示，编辑直接走 `taskService`）。
- **App 编排**（`App.tsx`）：`taskModalContext`+`handleOpenTask`+`handleSaveTask`+`taskActionsRef`（`:36,181-202`）是**角色视图**任务弹窗的保存编排（本 story **保留不动**）；`<ButlerView>`（`:257-270`）当前向其传 `onOpenTask`/`onTasksApiReady`（butler 任务接线）——本 story **仅移除这两个传参**。本 story **不**新增任何跨视图导航状态。
- **`TaskModal` 范本**（`components/modals/TaskModal.tsx`）：`props {scope, task, onClose, onSave}`；`task` 非空即编辑模式，`onSave(input: UpdateTaskInput)`；编辑时仅在 quadrant 变化才发 `quadrant`（避免冻结自动分类）。**概览自渲一个 `<TaskModal>` 复用此组件**，`onSave` 内调 `taskService.update`。
- **角色色标范本**（`DashboardTab.tsx:26`）：`style={{ backgroundColor: normalizeColorHex(role.color) }}`；可用 `lib/roleIcons` 的 `normalizeColorHex` 规范化十六进制色（但后端返回的 `roleColor` 已是规范 hex，前端展示可直接用，必要时再 `normalizeColorHex`）。

### What This Story Changes

**Rust（4 文件）：**
1. `models/task.rs`：+`CrossRoleTask`（roleName/roleColor 可空）
2. `db/tasks.rs`：+`list_all_tasks`（LEFT JOIN，含 butler）+ 单测
3. `commands/task.rs`：+`task_list_all`
4. `lib.rs`：注册 `task_list_all`

**前端（新增 4 + 修改 10，含后续 UX polish）：**
5. `types/task.ts`：+`CrossRoleTask` +`AllTasksFilter`
6. `services/taskService.ts`：+`listAll`
7. `hooks/useAllTasks.ts`：**新建**（稳定 `filterKey`、监听 `task:classified`、提供全量任务 CRUD actions）
8. `hooks/useAllTasks.test.tsx`：**新建**（filters 重查、错误兜底、失败冒泡等）
9. `components/butler/TaskOverviewTab.tsx`：**新建**（四象限分区 + 组内跨 owner + 已完成默认折叠 + 归属标签 + 角色多选/象限/大石头筛选 + 横向细滚动条 + 完成/删除）
10. `components/butler/TaskOverviewTab.test.tsx`：**新建**（概览展示/筛选/交互覆盖）
11. `components/modals/TaskModal.tsx`：新增可选 `roles`；当 `scope.ownerType === 'butler'` 且新建模式时渲染归属下拉（默认管家）；向后兼容
12. `components/butler/ButlerWorkspacePanel.tsx`：标签改名「任务概览」+ tasks Tab 改渲染 `TaskOverviewTab` + 使用 `useAllTasks` 注册 `butler` actions 给 App 级 `TaskModal`
13. `components/butler/ButlerView.tsx`：保留 `onOpenTask`/`onTasksApiReady` 透传给 `ButlerWorkspacePanel`
14. `App.tsx`：继续向 `<ButlerView>` 传任务弹窗编排；App 级 `<TaskModal>` 传入 `roles`，支持 Butler 新建任务选择归属
15. `components/role/TasksTab.tsx`：后续 UX 修复，角色任务顶部改为标题/新增任务 + 筛选卡片上下结构
16. `components/role/RoleView.tsx`：后续 UX 修复，角色右侧面板改为单一宽度/透明度过渡，去掉叠加 slide-in
17. `index.css`：新增局部 `.thin-horizontal-scrollbar`，只作用于任务概览归属筛选横向滚动条

> **角色侧说明**：Story 3.7 主功能不依赖角色侧改动；当前代码包含后续 UX polish，未改变角色任务数据流和交互语义。

**测试：** 新建 `TaskOverviewTab.test.tsx`（+可选 `useAllTasks.test.tsx`）；改 `ButlerWorkspacePanel.test.tsx`、`TaskModal.test.tsx`；`db/tasks.rs` 测试模块。

### What Must Be Preserved（防回归）

- **角色侧任务能力不回归**：`TasksTab.tsx`/`RoleView.tsx` 后续仅做 UX polish（顶部布局、右侧面板动画），角色任务拖拽/完成/分类中/at_risk/四象限折叠/大石头排序保持。`RoleWorkspacePanel.tsx` 任务数据流不变。
- **TaskModal 角色路径**：`App.handleSaveTask` 的 `role:` 分支、`onTasksApiReady('role:<id>')`、角色 `onOpenTask`、`taskModalContext` 不变；Butler 侧注册 `butler` 全量任务 actions 给同一 App 级弹窗编排使用。
- **`TaskModal` 改动向后兼容**：仅新增可选 `roles`；不传时行为与现状完全一致。只有 `scope.ownerType === 'butler'` 且新建模式且 `roles.length > 0` 时渲染归属下拉。
- **管家其余三 Tab**：仪表盘 / 管家记忆 / 管家设置 不变；`ButlerWorkspacePanel` 的 memory badge 逻辑（既有测试覆盖）不动。
- **`task_list_butler` 命令 / `owner_type='butler'` 数据**：后端保留；butler 任务改由 `task_list_all` 并入概览展示（`task_list_butler` 暂无前端消费方，保留不动）。
- **既有测试**：`ButlerWorkspacePanel.test.tsx` 当前以 `currentTab='memory'` 测记忆 badge，不渲染 tasks Tab；改动 tasks 渲染不影响其断言，但其顶层 mock 了 `useTasks` 与 `TasksTab`——本 story 改为 mock `TaskOverviewTab`、按需移除 `useTasks` mock，**保持记忆用例语义不变**。

### 关键正确性要点（极易踩坑）

- **sqlx 可选过滤的 NULL 绑定**：`(?2 IS NULL OR t.quadrant = ?2)` 配合 `.bind(quadrant)`（`Option<&str>`）安全；布尔过滤建议绑 `Option<bool>`，SQL 写 `(?3 IS NULL OR t.is_big_rock = ?3)`。**勿**把 `Option<bool>` 当作 0/1 直接拼接。参考 `list_tasks_by_owner` 的 `(?2 IS NULL ...)` 写法。
- **quadrant 字典序 = 业务序**：`'Q1'<'Q2'<'Q3'<'Q4'`，故 `ORDER BY quadrant ASC` 即 Q1→Q4，无需 CASE 映射。
- **`useAllTasks` 依赖归约**：必须用稳定 `filterKey` 字符串做 effect 依赖（仿 `useTasks.scopeKey`），否则 `filters` 对象每渲染新引用 → 无限请求。
- **编辑走 App 级 `TaskModal` + Butler 全量任务 actions**：管家视图下角色面板未挂载，`taskActionsRef` 取不到 `role:<其它角色>` actions。实现中 `ButlerWorkspacePanel` 使用 `useAllTasks` 注册 `butler` actions；概览点编辑通过 `onOpenTask({ ownerType:'butler' }, task)` 打开 App 级 `TaskModal`，保存时 `handleSaveTask` 调 `actions.updateTask(task.id, input)`，最终按任务 id 落库并刷新全量概览。
- **概览不自带 `TaskModal`**：当前实现复用 App 级 `<TaskModal>`，与角色侧弹窗共用同一保存编排；`CrossRoleTask extends Task`，可直接作为 `task` 传入。
- **概览卡不要塞进 dnd**：概览**不**使用 `SortableContext`/`useSortable`/`GripVertical`（跨 owner 拖拽排序无意义）；新建轻量卡（可在 `TaskOverviewTab.tsx` 内联），含完成圈/编辑/删除但无拖拽手柄。
- **TS strict / noUnusedLocals**：新增 props/常量若未使用会编译失败；Butler 任务概览保留 `onOpenTask`/`onTasksApiReady` 透传时，需确保 `ButlerView`、`ButlerWorkspacePanel`、`App` 三层类型同步。
- **`CrossRoleTask` 可直接喂 `TaskModal`**：`CrossRoleTask extends Task`，结构上可赋给 `TaskModal` 的 `task: Task`（多出的 roleName/roleColor 不影响）。编辑保存只发 `UpdateTaskInput`，`task_update` 按 id 更新且不动 `role_id`/`owner_type`，天然「同步到原角色/管家」。`task_update` 仅在 quadrant 变化时置 `manual_override`（`TaskModal` 已处理）。
- **大石头上限按「被编辑任务的角色」校验，错误必须冒泡**：`update_task`（`db/tasks.rs:86-91`）在 `is_big_rock` 由 false→true 时对 `existing.owner_type`/`existing.role_id` 调 `count_big_rocks_by_owner`，≥3 返回 `ValidationError`。管家在概览改某角色任务为大石头时，校验的是**那个角色**的额度（非管家），后端已正确处理。前端务必：概览 `onSave` 写成 `async input => { await taskService.update(id, input); await refetch(); }`（**不要** `.catch` 吞错），让 rejection 冒泡到 `TaskModal`（其 `handleSubmit` 已 try/catch 并显示 `errorObj.ValidationError`）；update 失败时 `refetch` 不执行、弹窗不关。
- **新建归属选择，默认管家**：概览「+新建」通过 App 级 `TaskModal` 新建分支 + `roles` prop 渲染归属 `<select>`（默认「管家」=`ownerType:'butler'`；选角色=`ownerType:'role'`+`roleId`）。新建经 `taskService.create`，同样受**大石头上限**（`create_task` 对所选 owner 计数）+ 错误冒泡约束。
- **新建后异步分类需刷新**：未显式选象限的新建任务，`task_create` 立即返回默认 Q2 并后台分类，完成后发 `task:classified`；`useAllTasks` 须监听并 `refetch`（仿 `useTasks`），否则概览中该任务象限/排序滞后。
- **Tailwind utility 优先**：象限 chips / 色点 / 选中态 / 按钮全部 Tailwind，动效加 `motion-reduce:` 变体（项目无障碍约定）。例外：归属筛选横向滚动条使用局部 `.thin-horizontal-scrollbar` CSS，仅作用于该横向筛选条。

### Previous Story Intelligence

- **3.6**（`3-6-quadrant-grouped-display.md`）：角色 TasksTab 的四象限分组/配色/计数/折叠刚完成；其卡片视觉（`CARD_BASE_CLASS`、deadline/大石头/at_risk badge）与 `quadrantLabels`/`orderBySort` 是聚合视图卡片与排序的参照系。本概览**复用视觉语言**但**不复用**可拖拽实现。3.6 的「已完成」默认折叠子区（`expandedCompleted` + `CollapsibleSection` 含 `inert`）是本概览**各象限已完成折叠**的直接参照；折叠状态用组件 `useState`、不持久化。
- **3.5**（at_risk）：`protectionStatus==='at_risk'` 的「被挤压」badge 渲染在卡片层；聚合卡也应展示该 badge（数据已在 `CrossRoleTask` 内）。
- **3.4**（大石头）：`orderBySort` = 大石头优先 + sortOrder；后端 `list_all_tasks` 的 `is_big_rock DESC, sort_order ASC` 与之同语义，保证跨视图一致。
- **3.2**（owner scope）：`owner_type='butler'` vs `'role'` 的分野来自此；本 story 概览**混排 role + butler 两类**，编辑/完成/删除按任务 id 落库（owner 无关、自动归属）。
- **通用**：前端 Vitest + Testing Library，co-located 测试；Rust 单测放同文件 `#[cfg(test)] mod tests`；图标 `lucide-react`（已依赖）。

### Git Intelligence

- `76a65ac`（HEAD，本 story baseline）— docs: 记录 story 3.6 实现与代码评审结果
- `6dd4af1` — feat(tasks): 四象限分组显示（story 3.6）
- `068c84b` — feat(tasks): Q2 at_risk 预警（story 3.5）
- `b248df2` — task owner scope + 大石头完成即释放名额（方案 D，owner_type 分野来源）
- `c4248c6` — async classification + event（3.3，TasksTab 分组骨架来源）
- 范本提交：3.5/3.6 展示了「DB 查询 + 命令 + 前端 service/hook/组件 + 双端测试」的全栈 story 落地节奏，可直接借鉴本 story 的拆分顺序。

### Testing Requirements

- **Rust 单测**（`db/tasks.rs` 测试模块）：用 `setup_test_db` 造 role-a/role-b + 跨 owner 任务（含 1 个 butler 任务、1 个已完成任务、不同象限、大石头），断言：
  - **包含** butler 任务；角色任务带正确 `role_name`/`role_color`；管家任务 `role_name`/`role_color` 为 `None`；
  - **包含已完成**；`quadrant=Some("Q1")` 仅返回 Q1；`is_big_rock=Some(true)` 仅返回大石头；
  - 排序：象限 Q1→Q4、未完成在前、大石头优先、owner role 先于 butler；
  - 补插一个 `status='archived'` 角色 + 任务，断言被排除（roles 表含 status 列）。
- **前端测试**：
  - `TaskOverviewTab.test.tsx`：通过 props 注入 `roles` 和 `CrossRoleTask[]`，断言按象限展示、归属标签、已完成默认折叠、归属筛选、象限/大石头回调、打开新建/编辑、完成切换、删除确认、归属筛选使用细横向滚动条类。
  - `TaskModal.test.tsx`：传 `roles=[...]` + Butler 新建模式 → 渲染归属下拉（默认管家），选角色后 `onSave` 收到 `ownerType:'role'`+`roleId`；角色 scope 不渲染归属下拉（回归）。
  - `useAllTasks.test.tsx`：filters 变化重查、错误兜底、更新失败不刷新并向上抛出错误，覆盖大石头超限冒泡所需路径。
  - `ButlerWorkspacePanel.test.tsx`：mock `TaskOverviewTab` 与 `useAllTasks`；保留并通过记忆 badge 用例。
  - `TasksTab.test.tsx` / `RoleView.test.tsx`：覆盖后续 UX polish（角色任务顶部上下结构、角色右侧面板不再叠加 slide-in）。
- **必跑命令**：
  - `npm --prefix "egosync-app" run test:frontend`
  - `npm --prefix "egosync-app" run build`
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`（或 `cd egosync-app/src-tauri && cargo test`）

### Project Structure Notes

- **新增文件（4）**：`hooks/useAllTasks.ts`、`hooks/useAllTasks.test.tsx`、`components/butler/TaskOverviewTab.tsx`、`components/butler/TaskOverviewTab.test.tsx`。
- **无新增依赖、无新增迁移、无 DB schema 改动**（roles/tasks 表已含所需列）。
- 符合项目规则：组件按域分目录（butler/）、hook `useXxx.ts`、service 封装 invoke、仅 Tailwind、图标 lucide-react、不在 `App.tsx` 内定义组件（仅加状态/handler/props）、Rust 三层（command→db）、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`。

### References

- `_bmad-output/project-context.md`（前后端规则：Tailwind-only、strict TS、组件分域、Rust 三层/serde camelCase、无障碍 prefers-reduced-motion）
- `_bmad-output/planning-artifacts/epics.md:1547-1577`（Story 3.7 定义）
- `_bmad-output/implementation-artifacts/3-6-quadrant-grouped-display.md`（角色 TasksTab 分组/卡片视觉/排序基线）
- `egosync-app/src-tauri/src/commands/task.rs:53-64`（`task_list_by_role`/`task_list_butler` 命令范本、`validate_quadrant`）
- `egosync-app/src-tauri/src/db/tasks.rs:6`（`TASK_SELECT_COLUMNS`）、`:59-77`（`list_tasks_by_owner` 查询范本）、`:517-583`（`setup_test_db`）、`:585-627`（list 测试范本）
- `egosync-app/src-tauri/src/db/roles.rs:9,35-71`（roles 列 / `list_active_roles` status 过滤）
- `egosync-app/src-tauri/src/models/task.rs:17-37`（`Task` 模型，`CrossRoleTask` 参照）
- `egosync-app/src-tauri/src/lib.rs:265,300-307`（命令注册位置）
- `egosync-app/src/hooks/useTasks.ts:15-97`（`scopeKey` 稳定依赖范式 → `useAllTasks.filterKey`）
- `egosync-app/src/services/taskService.ts`、`egosync-app/src/types/task.ts:10-58`（Task/TaskActions 类型）
- `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:45,102-130`（待改造的「通用任务」Tab）
- `egosync-app/src/components/butler/ButlerView.tsx:62-63,91-104`（管家 Tab 切换 + 面板 props 链）
- `egosync-app/src/App.tsx:36,181-202,257-270`（角色任务弹窗编排 `taskModalContext`/`handleSaveTask`/`taskActionsRef`（保留）+ `<ButlerView>` 的 butler 任务 props 传参（本 story 移除））
- `egosync-app/src/components/modals/TaskModal.tsx`（编辑弹窗复用：`scope`/`task`/`onSave` 契约；概览自渲此组件，`onSave` 调 `taskService.update`）
- `egosync-app/src/components/butler/DashboardTab.tsx:26`（角色色点 `backgroundColor` 范式）
- `egosync-app/src/components/butler/ButlerWorkspacePanel.test.tsx`（需同步的测试 mock）

## Dev Agent Record

### Agent Model Used

Cascade

### Debug Log References

- 前端定向测试：`npm run test:frontend -- TaskOverviewTab TaskModal ButlerWorkspacePanel useAllTasks` 通过。
- 前端全量测试：`npm run test:frontend` 通过，20 个测试文件 / 213 个测试用例通过。
- 前端构建：`npm run build` 通过。
- Rust 测试：`cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` 通过，lib 400 个测试通过，integration 1 个测试通过。

### Completion Notes List

- 新增 Rust `CrossRoleTask`、`list_all_tasks` DB 查询、`task_list_all` Tauri 命令并注册。
- 新增前端 `CrossRoleTask` / `AllTasksFilter`、`taskService.listAll`、`useAllTasks`。
- Butler 任务 Tab 改为 `TaskOverviewTab`，展示管家 + 所有 active 角色任务，支持归属筛选、大石头筛选、四象限分区、已完成折叠、编辑、删除、完成切换。
- 归属筛选横向滚动条改为局部细滚动条，并微调厚度与筛选项间距。
- `TaskModal` 支持 Butler 新建任务时选择归属，编辑路径保持兼容。
- 角色任务页顶部布局与角色右侧面板动画完成 UX polish，原有交互保持。
- 更新并新增 Rust / 前端测试，验证全量测试和构建通过。

### File List

- `egosync-app/src-tauri/src/models/task.rs`
- `egosync-app/src-tauri/src/db/tasks.rs`
- `egosync-app/src-tauri/src/commands/task.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/types/task.ts`
- `egosync-app/src/services/taskService.ts`
- `egosync-app/src/hooks/useAllTasks.ts`
- `egosync-app/src/hooks/useAllTasks.test.tsx`
- `egosync-app/src/components/butler/TaskOverviewTab.tsx`
- `egosync-app/src/components/butler/TaskOverviewTab.test.tsx`
- `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx`
- `egosync-app/src/components/butler/ButlerWorkspacePanel.test.tsx`
- `egosync-app/src/components/modals/TaskModal.tsx`
- `egosync-app/src/components/modals/TaskModal.test.tsx`
- `egosync-app/src/App.tsx`
- `egosync-app/src/components/role/TasksTab.tsx`
- `egosync-app/src/components/role/TasksTab.test.tsx`
- `egosync-app/src/components/role/RoleView.tsx`
- `egosync-app/src/components/role/RoleView.test.tsx`
- `egosync-app/src/index.css`
- `_bmad-output/implementation-artifacts/3-7-butler-all-roles-task-tab.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-20: 完成 Story 3.7 管家全角色任务概览实现与验证。
- 2026-06-20: 完成后续 UX polish：角色任务顶部上下布局、角色右侧面板单一过渡动画、任务概览归属筛选细横向滚动条，并刷新本文档使其与实际代码一致。

## 待确认问题（boss 已给主方向，剩余请开发前确认）

> boss 已确认全部要点：标签改「任务概览」、**按四象限分区（不按角色分组、组内跨 owner）**、每条**标注归属**、**角色多选**筛选、**管家任务并入**、**可新建（选归属、默认管家）+ 就地编辑/完成/删除**、**含已完成（默认折叠）**、**排除归档角色**。下列条目均已定（保留以记录决策）：

1. **概览「新建任务」入口**（已确认）
   - 已采纳（boss 指示）：概览顶部有「+ 新建任务」，新建时可**选择归属角色**、**默认「管家」**；经 `taskService.create`，同样受大石头上限与错误提示约束。

2. **概览里除了编辑，是否也要「完成」和「删除」？**
   - 已采纳：都给（完成切换 + 删除带二次确认）。若只要编辑，请告知。

3. **四象限分区小标题**（已确认）
   - 已采纳（boss 指示）：**保留 Q1→Q4 分区小标题**（仍不按角色分组，组内跨 owner 混排）。

4. **已完成任务 + 归档角色**（已确认）
   - 已采纳（boss 指示）：概览**含已完成任务**（各象限内默认折叠、仿角色 TasksTab）；**排除已归档角色**任务。

## Review Findings

> 代码评审（2026-06-20，基线 `76a65ac`）。三层评审：Blind Hunter（仅 diff）+ Edge Case Hunter（diff+代码）+ Acceptance Auditor（对照 AC）。

- [x] [Review][Patch] 「通用任务」Tab 改名「任务概览」（boss 确认为漏改）— 已修复 `ButlerWorkspacePanel.tsx:109`。
- [x] [Review][Patch] 筛选器按原 AC4/AC5/AC6/D4 回补（boss 选择回补）— 已实现：角色归属多选 chips（`TaskOverviewTab.tsx`）、象限 chips 单选（全部/Q1-Q4）、象限+大石头改服务端过滤（状态上提至 `ButlerWorkspacePanel.tsx:45-51`、经 `useAllTasks(filter)`），角色多选仍前端过滤；同步更新 `TaskOverviewTab.test.tsx`。
- [x] [Review][Patch] 空态文案与 AC7 对齐 — 已实现两态：全空显示「所有角色都很轻松，可以考虑添加新目标」，筛选空显示「当前筛选无匹配任务」（`TaskOverviewTab.tsx`）。
- [x] [Review][Patch] 补大石头上限拒绝路径测试 — `TaskModal.test.tsx` 已覆盖弹窗保持打开 + 新建归属 payload；新增 `useAllTasks.test.tsx`「更新失败时不刷新列表并向上抛出错误」用例，验证超限拒绝时 `refetch` 未触发且错误冒泡。
- [x] [Review][Defer] `groupedTasks` 对非法 quadrant 无防御 — deferred，后端 `validate_quadrant` 已约束取值，实际不可达，仅防御性提示。
- [x] [Review][Docs Refresh] `3-7-butler-all-roles-task-tab.md` 已按实际代码刷新：保留 `onOpenTask`/`onTasksApiReady` 透传，Butler 通过 `useAllTasks` 注册全量任务 actions；App 级 `TaskModal` 负责新建/编辑；后续角色侧 UX polish 与局部滚动条样式已纳入 File List / Change Log。
- [x] [Review][Dismiss] `orderByOverview` 在大石头与 sortOrder 之间插入 owner 名称排序，与 AC3「大石头→sortOrder」略有出入 — 视觉影响轻微，按 owner 聚合可读性更好。
