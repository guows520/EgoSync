# Story 2.1: 用户能编辑角色名称/图标/颜色，归档和恢复角色，永久删除角色

Status: done

## Story

As a 用户,
I want 修改角色信息、归档不常用角色、恢复归档角色、永久删除不需要的角色,
so that 我的角色列表保持整洁且可控。

## Acceptance Criteria

1. **编辑角色信息**
   - Given 用户在 `RoleView` 的 Settings tab 修改角色名称、图标、颜色
   - When 点击保存
   - Then 侧边栏实时更新显示新名称、图标、颜色
   - And `roles` 表对应字段已更新

2. **归档角色**
   - Given 用户在 `RoleView` Settings tab 点击“归档角色”
   - When 确认归档
   - Then 角色从侧边栏消失
   - And 角色的对话历史和后续记忆数据保留不删除
   - And 若当前在该角色视图，则自动切回管家视角

3. **恢复归档角色**
   - Given 用户在 `GlobalSettingsModal` 的归档角色列表中点击“恢复”
   - When 恢复完成
   - Then 角色重新出现在侧边栏
   - And 历史数据完整恢复

4. **永久删除角色**
   - Given 用户点击“永久删除”
   - When 二次确认对话框要求输入角色名并确认
   - Then `roles` 表删除该角色记录
   - And 关联的 `memories`、`conversations`、`messages` 被删除
   - And 删除不可撤销

5. **至少保留一个角色**
   - Given 用户仅剩 1 个 active 角色
   - When 尝试归档或删除
   - Then 归档/删除入口禁用
   - And 显示提示“至少保留一个角色”

6. **后端命令完整**
   - Given Rust 后端
   - Then 存在并注册 Tauri commands：`role_update`、`role_archive`、`role_restore`、`role_delete`
   - And `roles` 表包含 `archived_at` 可空字段

## Tasks / Subtasks

### Phase 1: 后端角色 CRUD 扩展 (AC: #1, #2, #3, #4, #5, #6)

- [x] T1.1 扩展 `egosync-app/src-tauri/src/models/role.rs`
  - [x] 新增 `UpdateRoleInput { name?, icon?, color?, goal? }`
  - [x] 继续使用 `#[serde(rename_all = "camelCase")]`
  - [x] 不改变现有 `Role` 返回结构
- [x] T1.2 扩展 `egosync-app/src-tauri/src/db/roles.rs`
  - [x] `update_role(pool, id, input)`：只更新允许字段，刷新 `updated_at`
  - [x] `archive_role(pool, id)`：`status='archived'`，写入 `archived_at` 和 `updated_at`
  - [x] `restore_role(pool, id)`：`status='active'`，清空 `archived_at`，刷新 `updated_at`
  - [x] `delete_role(pool, conv_pool, id)`：删除主库角色，并清理对话库中该角色的 conversations/messages
  - [x] `list_archived_roles(pool)`：按 `archived_at DESC` 返回归档角色
  - [x] `count_active_roles(pool)`：用于至少保留一个 active 角色
- [x] T1.3 扩展 `egosync-app/src-tauri/src/commands/role.rs`
  - [x] `role_update(id, input, pool)`
  - [x] `role_archive(id, pool)`
  - [x] `role_restore(id, pool)`
  - [x] `role_delete(id, pool, conv_pool)`
  - [x] `role_list_archived(pool)`
  - [x] 归档/删除前校验 active 角色数量，剩 1 个时返回 `ValidationError("至少保留一个角色")`
- [x] T1.4 在 `egosync-app/src-tauri/src/lib.rs` 注册新增 commands
- [x] T1.5 如需清理对话库，扩展 `egosync-app/src-tauri/src/db/conversations.rs`
  - [x] 新增 `delete_conversations_by_role(pool, role_id)`
  - [x] 因 `conversations.db` 独立于 `egosync.db`，不要依赖跨库外键级联

### Phase 2: 前端 service/type 接口补齐 (AC: #1, #2, #3, #4, #5)

- [x] T2.1 扩展 `egosync-app/src/types/role.ts`
  - [x] 新增 `UpdateRoleInput`
  - [x] 保持 `Role.status = 'active' | 'archived'`
- [x] T2.2 扩展 `egosync-app/src/services/roleService.ts`
  - [x] `update(id, input)` → `invoke<Role>('role_update', { id, input })`
  - [x] `archive(id)` → `invoke<Role>('role_archive', { id })`
  - [x] `restore(id)` → `invoke<Role>('role_restore', { id })`
  - [x] `delete(id)` → `invoke<void>('role_delete', { id })`
  - [x] `listArchived()` → `invoke<Role[]>('role_list_archived')`

### Phase 3: RoleView SettingsTab 编辑与危险操作 (AC: #1, #2, #4, #5)

- [x] T3.1 修改 `egosync-app/src/components/role/SettingsTab.tsx`
  - [x] 名称初值来自 `role.name`
  - [x] 图标初值来自 `role.icon`
  - [x] 颜色初值来自 `role.color`
  - [x] 目标初值来自 `role.goal`
  - [x] 保存时调用 `roleService.update()`，成功后通过 `onUpdateRole(updatedRole)` 更新顶层状态
- [x] T3.2 使用 `egosync-app/src/lib/roleIcons.ts` 的 `ROLE_ICONS` / `ROLE_COLORS`
  - [x] 不再新增一套 icon/color 常量
  - [x] icon 存储稳定 id，例如 `briefcase`
  - [x] color 存储 hex，例如 `#4F46E5`
- [x] T3.3 在 Settings tab 增加危险区域
  - [x] 归档角色：二次确认
  - [x] 永久删除：必须输入角色名确认
  - [x] active 角色数 ≤ 1 时禁用归档/删除，并显示“至少保留一个角色”
- [x] T3.4 删除确认成功后通知 `App.tsx` 刷新角色列表，并在当前角色被移除时切回 `butler`

### Phase 4: 全局设置归档列表与恢复 (AC: #3)

- [x] T4.1 修改 `egosync-app/src/components/settings/GlobalSettingsModal.tsx`
  - [x] 增加“归档角色”区域，可放在现有设置侧栏内独立 tab 或数据页下方
  - [x] 打开设置时加载 `roleService.listArchived()`
  - [x] 每条显示角色图标、名称、目标、归档时间
  - [x] 点击“恢复”调用 `roleService.restore()`
- [x] T4.2 恢复成功后刷新 active roles 与 archived roles
  - [x] 恢复角色重新出现在侧边栏
  - [x] 不自动切换到恢复角色，除非用户主动点击

### Phase 5: 顶层状态收敛 (AC: #1, #2, #3, #4, #5)

- [x] T5.1 修改 `egosync-app/src/App.tsx`
  - [x] 把当前 mock-only 的 `handleArchiveRole` / `handleRestoreRole` / `handleDeleteRole` 改为真实 service 调用
  - [x] 增加 `refreshRoles()` / `refreshArchivedRoles()`，避免多处重复拉取逻辑
  - [x] 不再用本地 `archivedRoles` 模拟归档结果作为真相
- [x] T5.2 处理错误反馈
  - [x] 对 `ValidationError("至少保留一个角色")` 显示温和中文提示
  - [x] 不使用 toast/snackbar；在当前 panel/dialog 内展示错误文案

### Phase 6: 测试与验证 (AC: #1-#6)

- [x] T6.1 后端单元测试：`db::roles`
  - [x] update 修改 name/icon/color/goal
  - [x] archive 从 active 变 archived，并写 `archived_at`
  - [x] restore 从 archived 变 active，并清空 `archived_at`
  - [x] delete 清理 roles 与 conversations/messages
  - [x] active 角色仅剩 1 个时 archive/delete 返回 `ValidationError`
- [x] T6.2 前端测试
  - [x] `SettingsTab` 保存后调用 update 并刷新显示
  - [x] 删除确认要求输入角色名
  - [x] 仅剩一个角色时危险按钮禁用
  - [x] `GlobalSettingsModal` 可加载 archived roles 并 restore
- [x] T6.3 验证命令
  - [x] `cd GUI && npx tsc --noEmit`
  - [x] `cd GUI && npm run test:frontend`
  - [x] `cd egosync-app/src-tauri && cargo test`

### Phase 7: Hotfix — AddRoleModal 写入真实 DB (AC: #1, #2, #5)

> 2026-05-25 验收回归发现：侧边栏「+ 新建角色」入口仍走 mock，导致后续编辑保存 NotFound、计数失真触发"至少保留一个角色"误报。

- [x] T7.1 修改 `egosync-app/src/components/modals/AddRoleModal.tsx`
  - [x] 弃用 `constants/mockData.ts` 的 `ICON_OPTIONS` / `COLOR_OPTIONS`，改用 `lib/roleIcons.ts` 的 `ROLE_ICONS` / `ROLE_COLORS` / `DEFAULT_ICON_ID` / `DEFAULT_COLOR_HEX`
  - [x] `handleSubmit` 改为 `async`，调用 `roleService.create({ name, icon, color })`，返回后端真实 `Role` 再回调 `onAdd`
  - [x] 加入 `isCreating` / `error` 局部状态，错误就地展示（不使用 toast）
  - [x] icon 存稳定 id（如 `briefcase`），color 存 hex（如 `#4F46E5`）
- [x] T7.2 清理 `egosync-app/src/constants/mockData.ts` 中孤立的 `ICON_OPTIONS` / `COLOR_OPTIONS`，并移除随之未使用的 lucide imports（保留 `DEFAULT_ROLES` 仍在用的 `Briefcase`/`Heart`/`BookOpen`）
- [x] T7.3 验证命令
  - [x] `cd GUI && npx tsc --noEmit`
  - [x] `cd GUI && npm run test:frontend`
  - [x] `cd egosync-app/src-tauri && cargo test`
  - [x] `cd egosync-app/src-tauri && cargo check`

## Dev Notes

### 当前真实状态

- `roles` 表已存在 `status` 与 `archived_at` 字段，不需要为了 `archived_at` 单独新增 migration；本 story 重点是把 update/archive/restore/delete 行为补齐。  
  [Source: `egosync-app/src-tauri/migrations/003_roles.sql`]
- 后端当前只有 `role_create` 与 `role_list`。  
  [Source: `egosync-app/src-tauri/src/commands/role.rs`]
- 前端 `roleService` 当前只有 `create` 与 `list`。  
  [Source: `egosync-app/src/services/roleService.ts`]
- `App.tsx` 当前的 archive/restore/delete 是纯前端数组操作，不写库，不是真实实现。  
  [Source: `egosync-app/src/App.tsx`]
- `SettingsTab` 当前只保存名称到本地顶层状态，目标和职责没有从 role 初始化，也不调用后端。  
  [Source: `egosync-app/src/components/role/SettingsTab.tsx`]
- `Sidebar` 里已有右键菜单与确认弹窗，但当前只是调用 `App.tsx` 的本地 handler。  
  [Source: `egosync-app/src/components/layout/Sidebar.tsx`]
- `GlobalSettingsModal` 目前没有归档角色列表。  
  [Source: `egosync-app/src/components/settings/GlobalSettingsModal.tsx`]

### 必须保留的边界

- 前端组件不能直接访问 SQLite；全部通过 `roleService` → Tauri command。  
  [Source: `_bmad-output/project-context.md`]
- Rust command 只做参数校验、调用数据/服务层、返回结果；不要把复杂 SQL 直接堆在组件或 command 以外的错误层。  
  [Source: `_bmad-output/planning-artifacts/architecture.md#Layer Rules`]
- 主库 `egosync.db` 与对话库 `conversations.db` 是两个 pool。删除角色时，对话清理必须显式调用 `db/conversations.rs`，不能假设跨 DB 外键级联。  
  [Source: `egosync-app/src-tauri/src/db/pool.rs`, `egosync-app/src-tauri/migrations/002_conversations.sql`]

### 数据与状态机

```text
active role
  ├─ update(name/icon/color/goal) → active role with updated_at
  ├─ archive → archived role with archived_at
  └─ delete → removed role + removed role conversations/messages

archived role
  ├─ restore → active role with archived_at = null
  └─ delete → removed role + removed role conversations/messages
```

### 删除与归档边界

- 归档：只改变 `roles.status` 与 `roles.archived_at`；不删除 conversations/messages，不删除后续 memories。
- 删除：删除角色本体，并清理该角色关联的 conversations/messages。
- 当前代码库还没有 `memories` 表；实现时如果 `memories` 尚不存在，不要为了本 story 提前创建完整 memory pipeline。可以：
  - 在主库删除角色前/后预留 `DELETE FROM memories WHERE role_id = ?` 的存在性兼容逻辑；或
  - 明确等 Story 2.6 创建 memories 表后再通过外键/删除逻辑补齐。  
  推荐低风险方案：本 story 不创建 memories 表，但在 Dev Notes/测试里说明当前无表，后续 Story 2.6 补 FK/清理测试。

### 图标与颜色格式

- Epic 1 Story 1.9 已解决 mock 与真实角色格式差异：真实 `Role.icon` 可能是字符串，`Role.color` 是 hex；`RoleSidebarIcon` 已兼容。  
  [Source: `_bmad-output/implementation-artifacts/1-9-role-sidebar-breathing-animation.md`]
- 当前 `egosync-app/src/lib/roleIcons.ts` 定义的是稳定 icon id + hex color 白名单；本 story 编辑 UI 应复用它，不要继续使用 `constants/mockData.ts` 的旧 mock 结构。  
  [Source: `egosync-app/src/lib/roleIcons.ts`]
- 注意：`RoleView.tsx` 当前仍使用 `role.color` 当 Tailwind class、`role.icon` 当 React 组件渲染。真实角色数据下这会继续有风险。若本 story 触碰角色头部，必须兼容 string icon 与 hex color；不要扩大为完整 RoleHeader 重构，RoleHeader 是 Story 2.2 范围。  
  [Source: `egosync-app/src/components/role/RoleView.tsx`]

### Epic 1 经验必须应用

- LLM/tool calling 不在本 story 范围内；不要顺手改 agent_engine 或 onboarding 状态机。
- Epic 1 暴露了 story 文件与 sprint-status 不一致的问题。本 story 完成时必须同步更新 story status、Completion Notes、File List、验证记录和 sprint-status。  
  [Source: `_bmad-output/implementation-artifacts/epic-1-retro-2026-05-23.md`]
- Mock 数据与真实数据格式差异会扩散；本 story 应优先收敛角色 Display Model 或组件适配，不要继续增加只适配 mock 的分支。  
  [Source: `_bmad-output/implementation-artifacts/epic-1-retro-2026-05-23.md`]

### Testing Requirements

- 前端：Vitest + React Testing Library，测试文件同目录。  
  [Source: `_bmad-output/project-context.md#测试规则`]
- Rust：单元测试放同文件底部或集成测试放 `src-tauri/tests/test_{domain}.rs`。  
  [Source: `_bmad-output/project-context.md#测试规则`]
- 全量命令：`npm run test:all` 当前组合前端 vitest + Rust cargo test。  
  [Source: `egosync-app/package.json`]

## Project Structure Notes

### Expected new or changed files

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src-tauri/src/models/role.rs` | UPDATE | 增加 `UpdateRoleInput` |
| `egosync-app/src-tauri/src/db/roles.rs` | UPDATE | 增加 update/archive/restore/delete/list_archived/count_active |
| `egosync-app/src-tauri/src/db/conversations.rs` | UPDATE | 增加按 role_id 删除 conversations/messages 的 helper |
| `egosync-app/src-tauri/src/commands/role.rs` | UPDATE | 增加角色 CRUD commands |
| `egosync-app/src-tauri/src/lib.rs` | UPDATE | 注册新增 commands |
| `egosync-app/src/types/role.ts` | UPDATE | 增加 `UpdateRoleInput` |
| `egosync-app/src/services/roleService.ts` | UPDATE | 增加 update/archive/restore/delete/listArchived |
| `egosync-app/src/components/role/SettingsTab.tsx` | UPDATE | 编辑角色信息、危险区域、错误文案 |
| `egosync-app/src/components/settings/GlobalSettingsModal.tsx` | UPDATE | 归档角色列表与恢复入口 |
| `egosync-app/src/App.tsx` | UPDATE | 顶层真实数据刷新与删除/归档状态同步 |

### Out of scope

- 不实现 Story 2.2 的完整角色视图切换改造。
- 不创建完整 `memories` pipeline；那是 Story 2.6。
- 不实现 Skill 配置真实行为；本 story 只避免破坏现有 UI。
- 不改 LLM Provider、chat streaming、onboarding Function Calling。

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.1]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md` — FR-4 角色 CRUD]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — IPC Boundary, Layer Rules, Data Boundaries]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` — 透明可控、温和反馈、避免 toast/snackbar]
- [Source: `_bmad-output/project-context.md` — TypeScript/Rust/Tauri IPC/测试规则]
- [Source: `_bmad-output/implementation-artifacts/1-9-role-sidebar-breathing-animation.md` — 真实角色格式兼容经验]
- [Source: `_bmad-output/implementation-artifacts/epic-1-retro-2026-05-23.md` — Epic 2 前置风险与流程要求]
- [Source: `egosync-app/src-tauri/migrations/003_roles.sql` — roles schema]
- [Source: `egosync-app/src-tauri/src/commands/role.rs` — 当前 commands]
- [Source: `egosync-app/src-tauri/src/db/roles.rs` — 当前 DB 层]
- [Source: `egosync-app/src-tauri/src/db/conversations.rs` — 对话库清理入口]
- [Source: `egosync-app/src/services/roleService.ts` — 当前前端 service]
- [Source: `egosync-app/src/components/role/SettingsTab.tsx` — 当前 Settings tab]
- [Source: `egosync-app/src/components/settings/GlobalSettingsModal.tsx` — 当前全局设置]

## Dev Agent Record

### Agent Model Used

gpt-5.5

### Debug Log References

- 2026-05-24：`npm run test:frontend` 首次暴露 `SettingsTab` 名称/目标 label 未关联控件，已补 `htmlFor` / `id`。
- 2026-05-24：`cargo test` 覆盖 command 门禁、roles DB 行为和 conversations 按 role 清理。
- 2026-05-25：验收回归发现侧边栏「+ 新建角色」未写库，导致保存 NotFound、归档误报"至少保留一个角色"；定位根因为 `AddRoleModal` 沿用 mock onAdd 路径未接入 `roleService.create`，已修复并删除 `mockData.ts` 中孤立的 `ICON_OPTIONS`/`COLOR_OPTIONS`。

### Completion Notes List

- Story context created by BMad create-story workflow.
- Target story auto-discovered from `sprint-status.yaml`: `2-1-role-crud-archive-delete`.
- Epic 2 marked ready to start; this story is first backlog item in Epic 2.
- 已完成后端角色 update/archive/restore/delete/listArchived commands 注册与 DB 层实现。
- 已完成 `SettingsTab` 角色名称、图标、颜色、目标编辑和危险区域，错误反馈留在当前 panel/dialog 内。
- 已完成 `GlobalSettingsModal` 归档角色列表与恢复入口，恢复后刷新 active/archived roles。
- 当前代码库尚无 `memories` 表；本 story 未创建 memory pipeline，后续 Story 2.6 需补 memories FK/清理测试。
- 验证通过：`npx tsc --noEmit`、`npm run test:frontend`、`cargo test`、`npm run test:all`。
- 2026-05-25 Hotfix：`AddRoleModal` 走通真实 `role_create`，与 `SettingsTab`/`OnboardingView` 共用 `roleIcons` 白名单；同步清理 `mockData.ts` 中已孤立的 `ICON_OPTIONS`/`COLOR_OPTIONS`。`tsc --noEmit`、`npm run test:frontend`、`cargo test`、`cargo check` 全部通过。端到端窗口联调（`tauri dev` 中人工新建→编辑保存→归档第二个角色）已由用户验证通过，状态回到 done。

### File List

- `egosync-app/src-tauri/src/models/role.rs`
- `egosync-app/src-tauri/src/db/roles.rs`
- `egosync-app/src-tauri/src/db/conversations.rs`
- `egosync-app/src-tauri/src/commands/role.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/types/role.ts`
- `egosync-app/src/services/roleService.ts`
- `egosync-app/src/components/role/SettingsTab.tsx`
- `egosync-app/src/components/role/SettingsTab.test.tsx`
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx`
- `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx`
- `egosync-app/src/App.tsx`
- `egosync-app/src/components/modals/AddRoleModal.tsx`
- `egosync-app/src/constants/mockData.ts`
- `_bmad-output/implementation-artifacts/2-1-role-crud-archive-delete.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

| 日期 | 变更 |
|---|---|
| 2026-05-23 | Story 2.1 创建，状态设为 ready-for-dev |
| 2026-05-24 | 完成角色 CRUD、归档/恢复/永久删除、归档列表和验证，状态设为 review |
| 2026-05-24 | CR 通过 (三层审查: Blind Hunter / Edge Case Hunter / Acceptance Auditor，6/6 AC PASS)，状态设为 done |
| 2026-05-25 | 验收回归发现 `AddRoleModal` 未接 `role_create`，导致编辑保存 NotFound、归档误报；hotfix 接入真实 `roleService.create` 并清理 `mockData.ts` 孤立常量，状态回退为 review |
| 2026-05-25 | 用户在 `tauri dev` 桌面会话内完成新建→编辑保存→归档第二个角色端到端验证，状态回到 done |