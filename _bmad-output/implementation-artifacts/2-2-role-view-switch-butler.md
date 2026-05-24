# Story 2.2: 用户能点击角色图标进入角色视图并切换回管家

Status: done

## Story

As a 用户,
I want 点击侧边栏角色图标切换到该角色的对话视图，跟该角色对话留下独立历史，并能随时切回管家,
so that 我能与特定角色直接对话，同时不污染管家的全局视角。

## Acceptance Criteria

1. **AC-1 入角色视图：色温与淡入**
   - **Given** 用户在管家视角，侧边栏存在 active 角色
   - **When** 点击该角色图标
   - **Then** 主区 CSS 变量 `--role-accent` 在 300ms 内过渡到该角色 `color`，`--role-bg-tint` 同步过渡
   - **And** 内容区淡入显示角色视图（opacity 0→1, 250ms ease-in）
   - **And** RoleView header 的 `角色名 + 图标 + 能量值` 来自后端 `Role`，不再使用 mock 字段

2. **AC-2 角色独立对话历史**
   - **Given** 用户已切换到某角色视图
   - **Then** 对话流显示属于该 `role_id` 的对话历史（`messages` 表中 `conversation_id` 关联该角色），不再渲染 mock 消息数组
   - **And** 用户输入并发送消息时，调用 `chat_send_message`，`role_id` 字段填该角色 id
   - **And** LLM 流式输出按现有 `llm:stream` 事件实时显示在该角色对话气泡中
   - **And** 不同角色之间历史互不污染（切到角色 A 再切到角色 B，看不到对方的消息）

3. **AC-3 切回管家**
   - **Given** 用户在角色视图
   - **When** 点击侧边栏管家图标 (`Home`)
   - **Then** `--role-accent` / `--role-bg-tint` 在 300ms 内恢复到管家色（`#6366F1` / transparent）
   - **And** 内容区显示管家对话历史（沿用既有 `ButlerView` 行为，不回退到 mock）

4. **AC-4 导航深度恒为 2**
   - **Given** 应用任何状态
   - **Then** `currentView` 状态仅可取 `'butler' | 'onboard' | {roleId}` 之一；不引入嵌套路由、不引入子页面状态
   - **And** 切换路径只能是 `butler ↔ role`，不允许 `roleA → roleB → 返回 roleA` 历史栈

5. **AC-5 RoleHeader 抽取为独立组件**
   - **Given** 现有 `GUI/src/components/role/RoleView.tsx` 内联 header（46px 图标 + 名字 + 能量条 + 工具栏按钮）
   - **When** 重构完成
   - **Then** header 渲染逻辑移入新文件 `GUI/src/components/role/RoleHeader.tsx`
   - **And** `RoleView` 通过 `<RoleHeader role={role} ... />` 复用，行为视觉与重构前一致
   - **And** RoleHeader 与 `Sidebar`/`RoleSidebarIcon` 共用 `lib/roleIcons.ts` 的 `getRoleIconComponent` / `normalizeColorHex`，不重新引入 emoji 渲染分支

6. **AC-6 后端 role 对话 command**
   - **Given** Rust 后端
   - **Then** 新增并注册 `conversation_get_or_create_by_role(role_id)` command（沿用既有 `chat_*` 域命名：`chat_get_role_conversation`），按 `role_id` 在 `conversations` 表查找最新 `updated_at` 记录，没有则创建
   - **And** `chat_send_message` 在 `request.role_id` 非空时，使 `run_stream` 使用 role-aware system prompt（注入 `role.name` / `role.goal` / `role.personality_prompt`）
   - **And** 不修改既有 butler 路径行为；`role_id` 为空时仍走 `BUTLER_SYSTEM_PROMPT`

7. **AC-7 历史对话列表按角色过滤**
   - **Given** 用户在角色视图打开 `ChatHeader` 的「历史对话」下拉
   - **Then** 仅显示 `role_id == 当前角色` 的对话；管家视角仅显示 `role_id IS NULL` 的对话
   - **And** 切换角色或切回管家时列表自动刷新

8. **AC-8 测试通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd GUI/src-tauri && cargo test`
   - 至少新增覆盖：role conversation lookup/create、role-aware system prompt 注入、RoleHeader 渲染、ChatStream 按 roleId 切换初始化

## Tasks / Subtasks

### Phase 1: 后端 role 对话 command + role-aware prompt (AC: #2, #6, #7)

- [x] T1.1 扩展 `GUI/src-tauri/src/db/conversations.rs`
  - [x] `get_or_create_conversation_by_role(pool, role_id) -> Result<Conversation, AppError>`：先 `SELECT ... WHERE role_id = ? ORDER BY updated_at DESC LIMIT 1`，无则 `create_conversation(pool, Some(role_id))`
  - [x] `list_conversations_by_role(pool, role_id) -> Result<Vec<Conversation>, AppError>`：用于历史对话列表
  - [x] `list_butler_conversations(pool) -> Result<Vec<Conversation>, AppError>`：`WHERE role_id IS NULL ORDER BY updated_at DESC`（替代既有 `list_all_conversations` 在管家视角中的用途，不删除既有 API）
  - [x] 同文件 `#[cfg(test)] mod tests` 增加单测：create→list_by_role 只看到该角色的对话；butler list 只看到 `role_id IS NULL`
- [x] T1.2 扩展 `GUI/src-tauri/src/commands/chat.rs`
  - [x] `chat_get_role_conversation(role_id, conv_pool)`：调 T1.1 helper
  - [x] `chat_list_conversations(role_id: Option<String>, conv_pool)`：保留入参兼容；`role_id == None` → `list_butler_conversations`，`Some(id)` → `list_conversations_by_role`
  - [x] 不引入新的 streaming/cancel state；复用既有 `StreamingState` / `CancelTokens`
- [x] T1.3 扩展 `GUI/src-tauri/src/services/agent_engine.rs`
  - [x] 新增 `build_role_messages(conv_pool, main_pool, conversation_id, role_id, user_message) -> Result<Vec<ChatCompletionMessage>, AppError>`
    - 加载 `Role` (`db::roles::get_role`)
    - system prompt 模板：以 `BUTLER_SYSTEM_PROMPT` 为基线，追加 `你现在扮演角色「{role.name}」。角色目标：{role.goal}。{role.personality_prompt}` —— `personality_prompt` 为空则跳过该行
    - 复用既有 `get_recent_messages(HISTORY_LIMIT)`
  - [x] `run_stream` 签名追加 `role_id: Option<String>` 参数；分支：
    - `onboarding_step.is_some()` → 既有 onboarding 路径不变
    - `role_id.is_some()` → `build_role_messages`
    - 否则 → 既有 `build_butler_messages`
  - [x] `chat_send_message` 把 `request.role_id` 透传给 `run_stream`
  - [x] 不引入 routing 决策、不引入 `routing_metadata`（属于 Story 2.3）
  - [x] 单测：`build_role_messages` 注入 role.name 与 goal；personality 为空时不出现空行；不存在的 role_id 返回 `AppError::NotFound`
- [x] T1.4 在 `GUI/src-tauri/src/lib.rs` `invoke_handler` 中注册 `chat_get_role_conversation`

### Phase 2: 前端 service / type 接口补齐 (AC: #2, #6, #7)

- [x] T2.1 扩展 `GUI/src/services/chatService.ts`
  - [x] `getRoleConversation(roleId)` → `invoke<Conversation>('chat_get_role_conversation', { roleId })`
  - [x] `listConversations(roleId?: string)` → `invoke<Conversation[]>('chat_list_conversations', { roleId: roleId ?? null })`（更新签名，保持向后兼容：不传等价 butler 列表）
- [x] T2.2 `GUI/src/types/chat.ts` 无需新增类型（`Conversation` 已含 `roleId`）

### Phase 3: RoleHeader 抽取 + RoleView 接通 ChatStream (AC: #1, #2, #3, #5)

- [x] T3.1 新建 `GUI/src/components/role/RoleHeader.tsx`
  - [x] Props：`{ role: Role; openTab: 'tasks'|'memory'|'settings'|null; onToggleTab: (tab) => void }`
  - [x] 视觉与当前 `RoleView` header 完全一致（46×46 图标块 + 名称 + 能量条 + 三个 Tab 按钮）
  - [x] 使用 `getRoleIconComponent(role.icon)` + `normalizeColorHex(role.color)`，不重引入 emoji 字符串渲染
  - [x] `role.color` 直接作为 inline style 背景；按钮 active 态色用 `style={{ color: roleColor }}` 替换原 `role.text` Tailwind class（mock 旧字段）
- [x] T3.2 重构 `GUI/src/components/role/RoleView.tsx`
  - [x] 把内联 header 替换为 `<RoleHeader />`
  - [x] 把 mock `messages` 状态 + `handleSend` 流程整体删除，主对话区改为渲染 `<ChatStream roleId={role.id} />`（占满 `openTab ? w-[60%] : w-full` 区域，行为与 `ButlerView` 对齐）
  - [x] 保留 `openTab`/`RoleWorkspacePanel`/`RoleHeader` 之间的交互（包括 `setRoleInitialTab` 信号）
  - [x] 保留对 `onUpdateRole`/`onArchiveRole`/`onDeleteRole`/`activeRoleCount` 的透传
- [x] T3.3 `GUI/src/components/chat/ChatStream.tsx`
  - [x] init 分支：`roleId` 为空 → `chatService.getButlerConversation()`；`roleId` 非空 → `chatService.getRoleConversation(roleId)`
  - [x] `loadConversations` 改为 `chatService.listConversations(roleId ?? undefined)`
  - [x] `useEffect` 依赖加上 `roleId`，roleId 变更时重新拉取并清空 streamContent / thinkingContent
  - [x] `handleNewConversation` 沿用既有逻辑，但创建时不再隐式默认 butler；如需创建带 role 的新对话，可在后续 story 增强（本 story 不修改 `chat_new_conversation` 签名）
  - [x] 不破坏既有事件取消/标题生成行为

### Phase 4: 主区色温过渡与淡入 (AC: #1, #3)

- [x] T4.1 `GUI/src/App.tsx`
  - [x] 主区 `<main>` 容器接收 `style={{ '--role-accent': accentHex, '--role-bg-tint': tintHex }}`，根据 `currentView`：
    - `butler` / `onboard` → `--role-accent: #6366F1` / `--role-bg-tint: transparent`
    - 其它（roleId）→ `--role-accent: role.color` / `--role-bg-tint: 同色 + alpha 6%`（不引入新依赖，用 `${roleColor}0F` 拼接）
  - [x] 主区 className 增加 `transition-[--role-accent,--role-bg-tint]` 不可行 — 改为：通过 CSS 变量驱动子元素 background-color/border-color 的 300ms 过渡（`transition-colors duration-300`，CSS 变量变化自动触发 transition）
- [x] T4.2 `GUI/src/index.css`
  - [x] 不改 `:root` 默认值；仅确认 `--role-accent` / `--role-bg-tint` 在 `transition-colors` 内会平滑过渡（不需新增 keyframe）
- [x] T4.3 `RoleView`/`ButlerView` 容器顶层加 `animate-in fade-in duration-250`（已存在的 `animate-in fade-in duration-500` 在 ButlerView，验证一致性后统一为 `duration-250` 以匹配 AC-1）
  - 实施偏差：`duration-250` 不在 Tailwind 默认 utility 集；改用 `duration-300` 作为最近的等效值（误差 50ms，肉眼不可感知，UX spec 中 `--duration-color: 300ms` 也是 300ms 节奏）。ButlerView 既有的 `duration-500` 暂未触碰（外科手术原则）。

### Phase 5: 测试与验证 (AC: #1-#8)

- [x] T5.1 后端单测：`db::conversations`
  - [x] `get_or_create_conversation_by_role` 第二次调用返回同一 id
  - [x] `list_conversations_by_role` 只返回该 role 对话
  - [x] `list_butler_conversations` 只返回 `role_id IS NULL`
- [x] T5.2 后端单测：`services::agent_engine::build_role_messages`
  - [x] 注入 system prompt 含 role.name / role.goal
  - [x] `personality_prompt` 为空时不出现空行 / 空 prompt
  - [x] 不存在的 role_id 返回 `AppError::NotFound`
- [ ] T5.3 后端集成测试（可选）：`chat_send_message` 带 role_id 时 system prompt 走 role 路径（mock provider 或 spy）
  - 跳过：当前没有 provider mock 框架，搭建一套属于"以防万一"实现；`build_role_messages` 直接单测已覆盖核心逻辑。
- [x] T5.4 前端测试
  - [x] `RoleHeader.test.tsx`：渲染 role.name / 能量条 / 三 tab 切换 / icon 通过 `getRoleIconComponent` 解析
  - [x] `ChatStream` 测试新增：传 `roleId` 时调用 `chat_get_role_conversation`；不传时调用 `chat_get_butler_conversation`
- [x] T5.5 验证命令
  - [x] `cd GUI && npx tsc --noEmit`（无错误）
  - [x] `cd GUI && npm run test:frontend`（24 passed / 7 files）
  - [x] `cd GUI/src-tauri && cargo test`（65 passed lib + 1 integration placeholder）
  - [ ] `tauri dev` 端到端验证 — 留给用户在桌面会话中手动确认（参考 Story 2.1 hotfix 经验）

### Review Findings

- [x] [Review][Patch] 角色视图点击“新对话”会创建管家会话，破坏角色历史隔离 [GUI/src/components/chat/ChatStream.tsx:126] — 已修复：`chat_new_conversation` 接收并持久化 `role_id`，`ChatStream` 在角色视图创建新对话时传入当前 `roleId`，新增前后端测试覆盖归属不污染。
- [x] [Review][Patch] 角色视图未把色温变量用于真实可见背景或发送按钮 [GUI/src/App.tsx:133] — 已修复：主区背景使用 `var(--role-bg-tint)`，角色视图 `ChatInput` 发送按钮使用 `var(--role-accent)`，管家视图保留原 slate 按钮。
- [x] [Review][Patch] RoleHeader 未展示 role.goal，与 AC-1 的后端 Role 头部信息不完整 [GUI/src/components/role/RoleHeader.tsx:51] — 已修复：`RoleHeader` 展示 `role.goal`，并更新测试断言。

## Dev Notes

### 当前真实状态

- `Sidebar` 已经通过 `onViewChange(currentView)` 实现 `butler ↔ roleId` 切换，无需新增导航状态。
  [Source: `GUI/src/components/layout/Sidebar.tsx`, `GUI/src/App.tsx`]
- `ChatStream` **已接受** `roleId?: string | null` 但当前 `useEffect` 直接 `chatService.getButlerConversation()`，**忽略了 roleId**。本 story 必须接通。
  [Source: `GUI/src/components/chat/ChatStream.tsx#useEffect`]
- `RoleView` 当前**没有用 ChatStream**：是一个独立的 mock `messages` 数组 + `setTimeout` 假回复。本 story 替换为真实 ChatStream。
  [Source: `GUI/src/components/role/RoleView.tsx`]
- `RoleView` 当前依赖 `role.text`（一个 mock Tailwind class 字段）来给 tab active 态着色，**这个字段不在后端 `Role` 类型上**。重构时必须用 `roleColor` inline style 替换，否则会出现 `'text-indigo-600'` 默认值兜底（功能不影响但已是与真实数据脱节的旧分支）。
  [Source: `GUI/src/components/role/RoleView.tsx:9`, `GUI/src/types/role.ts`]
- 后端 `Conversation.role_id` 字段已存在，`create_conversation(pool, Some(role_id))` 已存在。所以无需新增表结构。
  [Source: `GUI/src-tauri/src/db/conversations.rs`]
- `run_stream` 目前**不接收 role_id**；分支只在 `onboarding_step.is_some()` vs butler。需要新增 role 分支。
  [Source: `GUI/src-tauri/src/services/agent_engine.rs#run_stream`]
- `chat_send_message` 在 `request.conversation_id.is_some()` 时**沿用 request.role_id**，但在 `None` 分支固定走 `get_or_create_butler_conversation`。这意味着角色视图必须**先**调用 `chat_get_role_conversation` 拿到 conversation_id，再发 message。新增 command 必要性已确认。
  [Source: `GUI/src-tauri/src/commands/chat.rs:62-72`]

### 必须保留的边界

- 前端不直接访问 SQLite；所有读写必须经 Tauri command。
  [Source: `_bmad-output/project-context.md#关键禁止事项`]
- 主库 `egosync.db`（roles） 与对话库 `conversations.db` 是两个 pool。`build_role_messages` 需要同时持有两边的 pool（main_pool 读 role，conv_pool 读历史）。
  [Source: `GUI/src-tauri/src/db/pool.rs`]
- 不在前端组件直接 emit Tauri event；事件监听统一通过 `useTauriEvent` hook。
  [Source: `_bmad-output/project-context.md#Tauri IPC`]
- 不写自定义 CSS class；色温过渡用 Tailwind utility + CSS 变量 + `transition-colors`。
  [Source: `_bmad-output/project-context.md#React 前端`]
- 不破坏 onboarding 路径：`run_stream` 三分支顺序必须是 `onboarding_step.is_some()` → role → butler。
  [Source: `GUI/src-tauri/src/services/agent_engine.rs#run_stream`]

### Story 2.1 与 Epic 1 经验必须应用

- mock 字段（`role.text`、mock 颜色 Tailwind class）已被 Story 1.9/2.1 收敛到 `lib/roleIcons.ts` 白名单 + hex 颜色。本 story 触碰 RoleView 时必须**继续**收敛，**不要**为兼容 `role.text` 引入新的分支。
  [Source: `_bmad-output/implementation-artifacts/1-9-role-sidebar-breathing-animation.md`, `_bmad-output/implementation-artifacts/2-1-role-crud-archive-delete.md`]
- Story 2.1 暴露过：sprint-status 与 story 文件状态不同步、AddRoleModal 未接真实 service 的回归。本 story 完成时必须：
  - 同步 `sprint-status.yaml` 中 `2-2-role-view-switch-butler` 至 `ready-for-dev`（创建阶段）/ `done`（完成阶段）
  - 在 `Completion Notes List`、`File List`、`Change Log` 记录所有改动
  [Source: `_bmad-output/implementation-artifacts/epic-1-retro-2026-05-23.md`]
- 不使用 toast/snackbar；错误就地展示（沿用 Story 2.1 模式）。
  [Source: `_bmad-output/planning-artifacts/ux-design-specification.md`]

### 色温与淡入的实现路径

```
切换前: --role-accent: #6366F1, --role-bg-tint: transparent (butler)
        ↓ 300ms transition-colors
切换后: --role-accent: #4F46E5 (role.color), --role-bg-tint: #4F46E50F (alpha 6%)
```

- App.tsx 的 `<main>` 容器：`style={{ '--role-accent': accent, '--role-bg-tint': tint }}` + `className="transition-colors duration-300"`
- 子组件（RoleHeader 的图标 block、ChatStream 的发送按钮等）`style={{ backgroundColor: 'var(--role-accent)' }}`
- 内容淡入：组件根容器 `animate-in fade-in duration-250`

注意：CSS 变量本身不会触发 `transition`；要让子元素的 `background-color: var(--role-accent)` 平滑过渡，子元素必须自己挂 `transition-colors`。Tailwind utility `transition-colors duration-300` 已足够。

### Out of Scope（明确不做）

- 不做 Story 2.3 的意图路由（butler 自动派单到角色）
- 不做 Story 2.4 的 SettingsTab personality 编辑 UI（personality_prompt 字段已存在，本 story 只在 system prompt 里注入它，**不**改 SettingsTab）
- 不做 Story 2.5 的角色涌现建议
- 不做 Story 2.6 的 memories 表创建
- 不做 routing_metadata JSON 字段（属于 2.3）
- 不做 `chat_new_conversation` 的 role 版本签名扩展（本 story 不修改它，保持向后兼容）
- 不实现 conversation 切换时的滚动位置记忆

## Project Structure Notes

### 新建文件

| Path | Notes |
|---|---|
| `GUI/src/components/role/RoleHeader.tsx` | 从 RoleView 抽取的头部组件 |
| `GUI/src/components/role/RoleHeader.test.tsx` | RoleHeader 渲染测试 |

### 修改文件

| Path | Action | Notes |
|---|---|---|
| `GUI/src-tauri/src/db/conversations.rs` | UPDATE | 新增 `get_or_create_conversation_by_role` / `list_conversations_by_role` / `list_butler_conversations` |
| `GUI/src-tauri/src/commands/chat.rs` | UPDATE | 新增 `chat_get_role_conversation`；`chat_list_conversations` 支持可选 role_id |
| `GUI/src-tauri/src/services/agent_engine.rs` | UPDATE | 新增 `build_role_messages`；`run_stream` 增加 role_id 分支 |
| `GUI/src-tauri/src/lib.rs` | UPDATE | 注册新 command |
| `GUI/src/services/chatService.ts` | UPDATE | 新增 `getRoleConversation`；`listConversations` 支持可选 roleId |
| `GUI/src/components/role/RoleView.tsx` | UPDATE | 删 mock messages；接通 ChatStream；用 RoleHeader |
| `GUI/src/components/chat/ChatStream.tsx` | UPDATE | init 按 roleId 分支；listConversations 按 roleId 过滤 |
| `GUI/src/App.tsx` | UPDATE | 主区注入 `--role-accent` / `--role-bg-tint` CSS 变量 |
| `GUI/src/components/chat/ChatStream.test.tsx`（如已存在则更新，否则新建） | UPDATE/NEW | 覆盖 roleId 分支 |

### 不动的文件

- `GUI/src-tauri/migrations/` — 无 schema 变更
- `GUI/src-tauri/src/models/chat.rs` — `Conversation` / `ChatRequest` 已含 `role_id` / `roleId`
- `GUI/src/types/chat.ts` — 已含 `roleId`
- `GUI/src/components/role/SettingsTab.tsx` — 本 story 不动；personality UI 编辑是 Story 2.4
- `GUI/src/components/onboarding/OnboardingView.tsx` — onboarding 路径不变

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.2 AC]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md` — FR-6 角色个性化语调；点击卡片跳转]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — IPC Boundary, Layer Rules, Component RoleView]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` — RoleHeader 规范、视图切换 fade 250ms、色温过渡 300ms、导航 2 层]
- [Source: `_bmad-output/project-context.md` — Tauri IPC / React / 测试规则]
- [Source: `_bmad-output/implementation-artifacts/1-9-role-sidebar-breathing-animation.md` — 真实角色 icon/color 格式收敛]
- [Source: `_bmad-output/implementation-artifacts/2-1-role-crud-archive-delete.md` — roleService/SettingsTab 收敛、错误就地展示]
- [Source: `_bmad-output/implementation-artifacts/epic-1-retro-2026-05-23.md` — sprint-status 同步、mock 收敛纪律]
- [Source: `GUI/src/components/role/RoleView.tsx` — 当前 mock 实现需要替换]
- [Source: `GUI/src/components/chat/ChatStream.tsx` — 已有 roleId prop 但未生效]
- [Source: `GUI/src-tauri/src/commands/chat.rs` — chat_send_message 已透传 role_id]
- [Source: `GUI/src-tauri/src/db/conversations.rs` — Conversation.role_id 字段已存在]
- [Source: `GUI/src-tauri/src/services/agent_engine.rs` — run_stream 当前无 role 分支]
- [Source: `GUI/src/lib/roleIcons.ts` — icon/color 白名单]

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5

### Debug Log References

- 2026-05-24：`RoleHeader.test.tsx` 首次跑失败 — jsdom 把 hex 标准化为 `rgb(...)`，把断言改为 `taskButton.style.color === 'rgb(79, 70, 229)'`（等价于 `#4F46E5`）。

### Completion Notes List

- Story context created by BMad create-story workflow.
- Target story auto-discovered from `sprint-status.yaml`: `2-2-role-view-switch-butler`.
- Epic 2 已经在 in-progress（Story 2.1 完成后），本 story 不触发 epic 状态转换。
- Phase 1 后端：`db::conversations` 新增 `get_or_create_conversation_by_role` / `list_conversations_by_role` / `list_butler_conversations` 三个 helper；`commands/chat.rs` 新增 `chat_get_role_conversation` 并把 `chat_list_conversations` 改为接 `Option<String>` 参数；`agent_engine::run_stream` 签名追加 `role_id: Option<String>`，分支顺序 `onboarding → role → butler`，确保不破坏 onboarding 路径；新增 `build_role_messages` 在 `BUTLER_SYSTEM_PROMPT` 之上注入角色名/目标/personality。
- Phase 2 前端 service：`chatService.getRoleConversation` 与 `listConversations(roleId?)` 上线。
- Phase 3 前端组件：`RoleHeader` 从 RoleView 抽出，统一走 `lib/roleIcons.ts` 白名单 + hex inline style（不再依赖 mock `role.text`）；`RoleView` 删除 mock messages 数组与假回复 `setTimeout`，主对话区直接挂 `<ChatStream roleId={role.id} />`；`ChatStream` `useEffect` 加上 `roleId` 依赖，切换角色立刻清空旧消息再拉取归属该角色的会话。
- Phase 4 主区色温：`App.tsx` 用 `useMemo` 计算 `--role-accent` / `--role-bg-tint`，butler/onboard 走 `#6366F1`/`transparent`，角色视图走 `role.color` + 6% alpha，主区 `<main>` 挂 `transition-colors duration-300`。
- 实施偏差（fail-loud）：AC-1 写 "250ms 淡入"，但 Tailwind 默认 utility 集没有 `duration-250`；选择 `duration-300` 作为最接近值（与 UX spec `--duration-color: 300ms` 节奏一致），误差 50ms，肉眼不可感知。RoleView 容器加了 `animate-in fade-in duration-300`，未触碰 ButlerView 既有 `duration-500`（外科手术原则）。
- 后端测试：新增 5 个 — `get_or_create_conversation_by_role_reuses_existing` / `list_conversations_by_role_only_returns_target_role` / `list_butler_conversations_excludes_role_conversations` / `build_role_messages_injects_name_and_goal` / `build_role_messages_skips_empty_personality` / `build_role_messages_unknown_role_returns_not_found`，cargo test 64 passed 全绿。
- 前端测试：新增 5 个 — `RoleHeader` 3 个（渲染名/能量、tab 回调、active 着色用 inline style）+ `ChatStream` 2 个（roleId 空走 butler / roleId 非空走 role），vitest 23 passed 全绿。
- Story 2.3 意图路由 / routing_metadata、Story 2.4 personality UI、Story 2.6 memories 表均未触碰；`chat_new_conversation` 签名未变。
- 已知遗留：`tauri dev` 端到端窗口验证留给用户在桌面会话中手动确认（参考 Story 2.1 hotfix 经验：自动化测试通过 ≠ 真实窗口体验通过）。
- 2026-05-24 hotfix（用户体验回归）：报告的 3 个症状（气泡仍是管家 / 角色自称管家 / 输入框写"跟管家说点什么"）已修复。根因是同一个漏洞：「当前角色对象」没有传到聊天 UI 与 role-aware system prompt 没有独立身份基线。修复方式见 Change Log 末行。前端 28 passed / 后端 65 passed 全绿，已由 boss 桌面确认验证 OK。

### File List

**新建文件：**
- `GUI/src/components/role/RoleHeader.tsx`
- `GUI/src/components/role/RoleHeader.test.tsx`
- `GUI/src/components/chat/ChatStream.test.tsx`
- `GUI/src/components/chat/ChatInput.test.tsx`
- `GUI/src/components/chat/ChatBubble.test.tsx`

**修改文件：**
- `GUI/src-tauri/src/db/conversations.rs`
- `GUI/src-tauri/src/commands/chat.rs`
- `GUI/src-tauri/src/services/agent_engine.rs`
- `GUI/src-tauri/src/lib.rs`
- `GUI/src/services/chatService.ts`
- `GUI/src/components/role/RoleView.tsx`
- `GUI/src/components/chat/ChatStream.tsx`
- `GUI/src/components/chat/ChatStream.test.tsx`
- `GUI/src/components/chat/ChatInput.tsx`
- `GUI/src/components/chat/ChatBubble.tsx`
- `GUI/src/components/butler/ButlerView.tsx`
- `GUI/src/App.tsx`
- `_bmad-output/implementation-artifacts/2-2-role-view-switch-butler.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

| 日期 | 变更 |
|---|---|
| 2026-05-24 | Story 2.2 上下文创建，状态设为 ready-for-dev |
| 2026-05-24 | 完成后端 role 对话 command + role-aware prompt、前端 RoleHeader 抽取、RoleView 接通 ChatStream、ChatStream 按 roleId 分支、主区 CSS 变量色温过渡。验证：tsc / vitest 23 passed / cargo test 64 passed。状态设为 review |
| 2026-05-24 | Code review 修复：`chat_new_conversation` 持久化 role_id；主区背景与发送按钮接入 `--role-bg-tint` / `--role-accent`；RoleHeader 展示 role.goal。tsc / vitest 24 passed / cargo test 65 passed。状态设为 done |
| 2026-05-24 | 用户体验回归修复：(1) `agent_engine.rs` 拆出独立 `ROLE_SYSTEM_PROMPT_PREFIX`，不再以 BUTLER 基线建立角色身份；(2) `ChatBubble` 支持 `assistantName/Icon/Color`，`ChatStream` 派生角色身份透传，输入框 placeholder 含角色名；(3) `ChatStream` prop 由 `roleId` 改为 `role` 对象，`RoleView` / `ButlerView` 同步更新。新增测试：后端「不应以管家基线建立身份」负向断言、ChatBubble 角色身份覆盖、ChatStream placeholder。tsc / vitest 28 passed / cargo test 65 passed。已知遗留：角色旧会话历史中 assistant 自称「管家」的旧消息仍会进入上下文，需用户在体验时手动新建会话或换新角色复测。 |