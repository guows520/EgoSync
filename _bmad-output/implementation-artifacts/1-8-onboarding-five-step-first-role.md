# Story 1.8: 新用户首次打开应用，被管家引导完成 5 步对话创建第一个角色

Status: done

## Story

As a 新用户,
I want 首次打开应用时被管家友好引导,
So that 不到 5 分钟就能创建第一个角色并理解 EgoSync 的核心概念。

## Acceptance Criteria

1. **AC-1 自动进入引导**：新用户首次打开应用（`app_settings.onboarding_completed` 不存在或为 `false`），LLM 已配置 → 自动进入 `OnboardingView`，5 步引导对话流：欢迎 → 自我介绍 → 痛点 → 角色提议 → 创建确认。
2. **AC-2 LLM 未配置前置**：引导开始时 LLM 未配置 → 引导第 0 步直接跳转到 GlobalSettingsModal LLM tab；配置完成后自动返回继续引导。
3. **AC-3 角色创建持久化**：引导第 4 步用户确认创建角色 → `roles` 表写入角色记录（id, name, icon, color, goal, status='active', energy=100）；`app_settings.onboarding_completed = 'true'`；跳转管家主视图。
4. **AC-4 不重复引导**：已完成引导的用户重启应用 → 直接进主视图，不重复引导。
5. **AC-5 时间约束**：端到端从启动到创建第一个角色 ≤ 5 分钟（含 LLM 响应时间）。
6. **AC-6 roles 表 Migration**：`migrations/003_roles.sql` 创建 `roles` 表（在主库 `egosync.db`）。
7. **AC-7 Tauri Commands**：`role::create` 写入角色数据；`app::is_first_launch` 检测 `app_settings.onboarding_completed`。
8. **AC-8 OnboardingView 接通真实 LLM**：`OnboardingView` 替换硬编码 mock 步骤流程，使用真实 LLM 流式对话（通过 `agent_engine` + `llm:stream` Event）。
9. **AC-9 引导状态机**：引导流程由前端 `onboardingStep`（1-5）驱动，后端 `OnboardingConversations` HashMap 记录 conv_id → step 映射；`ChatRequest.onboarding_step: u8`（`#[serde(default)]`）传递当前步骤；后端根据 step 选择 `build_onboarding_messages` + `get_onboarding_chat_options(step)`（step < 3: 无 tools；step ≥ 3: `create_role` Function Calling + `tool_choice="required"`）。
10. **AC-10 现有测试通过**：`cargo test` + `npm run test:frontend` 全部通过。

## Tasks / Subtasks

### Phase 1: Roles 表迁移 + 数据模型 (AC: #3, #6)

- [ ] **T1.1** 创建 `egosync-app/src-tauri/migrations/003_roles.sql`：
  ```sql
  -- 角色表（主库 egosync.db）
  CREATE TABLE IF NOT EXISTS roles (
      id TEXT PRIMARY KEY NOT NULL,
      name TEXT NOT NULL,
      icon TEXT NOT NULL DEFAULT '🎯',
      color TEXT NOT NULL DEFAULT '#6366F1',
      goal TEXT NOT NULL DEFAULT '',
      personality_prompt TEXT NOT NULL DEFAULT '',
      status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
      energy INTEGER NOT NULL DEFAULT 100,
      skills_config TEXT NOT NULL DEFAULT '{}',
      proactivity_level TEXT NOT NULL DEFAULT 'moderate' CHECK(proactivity_level IN ('passive', 'moderate', 'proactive')),
      archived_at TEXT,
      created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
      updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
  );

  CREATE INDEX IF NOT EXISTS idx_roles_status ON roles(status);
  ```
- [ ] **T1.2** 创建 `src-tauri/src/models/role.rs`：
  ```rust
  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
  #[serde(rename_all = "camelCase")]
  pub struct Role {
      pub id: String,
      pub name: String,
      pub icon: String,
      pub color: String,
      pub goal: String,
      pub personality_prompt: String,
      pub status: String,
      pub energy: i32,
      pub skills_config: String,
      pub proactivity_level: String,
      pub archived_at: Option<String>,
      pub created_at: String,
      pub updated_at: String,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct CreateRoleInput {
      pub name: String,
      pub icon: Option<String>,
      pub color: Option<String>,
      pub goal: Option<String>,
  }
  ```
- [ ] **T1.3** 在 `models/mod.rs` 添加 `pub mod role;`

### Phase 2: Roles DB 层 (AC: #3, #7)

- [ ] **T2.1** 创建 `src-tauri/src/db/roles.rs`：
  - `create_role(pool, input: &CreateRoleInput) -> Result<Role>`（生成 UUID、写入默认值）
  - `list_active_roles(pool) -> Result<Vec<Role>>`（`WHERE status = 'active'`）
  - `get_role(pool, id: &str) -> Result<Role>`
- [ ] **T2.2** 在 `db/mod.rs` 添加 `pub mod roles;`
- [ ] **T2.3** `cargo check` 通过

### Phase 3: App Settings 读写工具 (AC: #1, #4, #7)

- [ ] **T3.1** 创建 `src-tauri/src/db/app_settings.rs`：
  - `get_setting(pool, key: &str) -> Result<Option<String>>`
  - `set_setting(pool, key: &str, value: &str) -> Result<()>`
- [ ] **T3.2** 在 `db/mod.rs` 添加 `pub mod app_settings;`

### Phase 4: Tauri Commands — role + app (AC: #7)

- [ ] **T4.1** 创建 `src-tauri/src/commands/role.rs`：
  ```rust
  #[tauri::command]
  pub async fn role_create(
      input: CreateRoleInput,
      pool: State<'_, DbPool>,
  ) -> Result<Role, AppError>

  #[tauri::command]
  pub async fn role_list(
      pool: State<'_, DbPool>,
  ) -> Result<Vec<Role>, AppError>
  ```
- [ ] **T4.2** 创建 `src-tauri/src/commands/app.rs`：
  ```rust
  #[tauri::command]
  pub async fn app_is_first_launch(
      pool: State<'_, DbPool>,
  ) -> Result<bool, AppError>
  // 读取 app_settings.onboarding_completed，不存在或非 "true" → 返回 true

  #[tauri::command]
  pub async fn app_complete_onboarding(
      pool: State<'_, DbPool>,
  ) -> Result<(), AppError>
  // 写入 app_settings: key="onboarding_completed", value="true"

  #[tauri::command]
  pub async fn app_is_llm_configured(
      pool: State<'_, DbPool>,
  ) -> Result<bool, AppError>
  // 查 llm_configs 表是否有 is_default=1 的记录
  ```
- [ ] **T4.3** 在 `commands/mod.rs` 添加 `pub mod role;` + `pub mod app;`
- [ ] **T4.4** 在 `lib.rs` `invoke_handler` 注册新 commands：`role_create`, `role_list`, `app_is_first_launch`, `app_complete_onboarding`, `app_is_llm_configured`
- [ ] **T4.5** `cargo check` 通过

### Phase 5: 引导 System Prompt 分层 (AC: #8, #9)

- [ ] **T5.1** 在 `services/agent_engine.rs` 新增引导对话构建函数：
  ```rust
  const ONBOARDING_SYSTEM_PROMPT: &str = "...";
  // 含 5 步引导指令：
  // Step 0 (welcome): 友好开场，问用户称呼
  // Step 1 (self_intro): 追问用户最近在忙什么
  // Step 2 (pain_points): 理解痛点/关注领域
  // Step 3 (role_proposal): 基于对话提出角色建议（名称+图标+颜色+目标）
  // Step 4 (role_confirm): 确认创建，输出特定 JSON 格式供前端解析

  pub async fn build_onboarding_messages(
      conv_pool: &ConversationsPool,
      conversation_id: &str,
      user_message: &str,
      step: u8,
  ) -> Result<Vec<ChatCompletionMessage>, AppError>
  ```
- [ ] **T5.2** 引导 step ≥ 3 使用 Function Calling：
  - `create_role_tool_definition()` → OpenAI function schema（name/icon/color/goal 参数）
  - `get_onboarding_chat_options(step)` → step < 3: `ChatOptions::default()`; step ≥ 3: `tools=[create_role]`, `tool_choice="required"`
  - LLM 发出 `tool_calls` → 后端 `StreamEvent::ToolCall` → `execute_create_role` → emit `role:proposed` 事件（前端弹出 `RoleConfirmModal` 供用户确认/编辑）
  - **不再使用** `<!--ROLE_CREATE:{...}-->` 正则标记

### Phase 6: 前端 Types + Service (AC: #8)

- [ ] **T6.1** 创建 `egosync-app/src/types/role.ts`：
  ```typescript
  export interface Role {
    id: string;
    name: string;
    icon: string;
    color: string;
    goal: string;
    personalityPrompt: string;
    status: 'active' | 'archived';
    energy: number;
    skillsConfig: string;
    proactivityLevel: 'passive' | 'moderate' | 'proactive';
    archivedAt: string | null;
    createdAt: string;
    updatedAt: string;
  }

  export interface CreateRoleInput {
    name: string;
    icon?: string;
    color?: string;
    goal?: string;
  }
  ```
- [ ] **T6.2** 创建 `egosync-app/src/services/roleService.ts`：
  ```typescript
  import { invoke } from '@tauri-apps/api/core';
  import type { Role, CreateRoleInput } from '../types/role';

  export const roleService = {
    create: (input: CreateRoleInput) => invoke<Role>('role_create', { input }),
    list: () => invoke<Role[]>('role_list'),
  };
  ```
- [ ] **T6.3** 创建 `egosync-app/src/services/appService.ts`：
  ```typescript
  import { invoke } from '@tauri-apps/api/core';

  export const appService = {
    isFirstLaunch: () => invoke<boolean>('app_is_first_launch'),
    completeOnboarding: () => invoke<void>('app_complete_onboarding'),
    isLlmConfigured: () => invoke<boolean>('app_is_llm_configured'),
  };
  ```

### Phase 7: OnboardingView 重写 — 接通真实 LLM (AC: #1, #2, #3, #8, #9)

- [ ] **T7.1** 重写 `egosync-app/src/components/onboarding/OnboardingView.tsx`：
  - 状态：`step`(1-5), `conversationId`, `messages`, `isStreaming`, `streamContent`, `thinkingContent`, `showRoleModal`, `proposedRole`
  - 启动时调用 `appService.isLlmConfigured()`：
    - 未配置 → 打开 GlobalSettingsModal，配置完成后继续
    - 已配置 → 发送空消息 + `onboardingStep: 1` 触发管家开场
  - 使用 `chatService.sendMessage()` 发送用户消息（携带 `onboardingStep`）
  - 监听 `useTauriEvent<StreamPayload>('llm:stream', ...)` 渲染流式回复
  - 监听 `useTauriEvent('role:proposed', ...)` → 设置 `proposedRole` + 显示 `RoleConfirmModal`
  - `RoleConfirmModal` 确认 → `roleService.create(editedInput)` → `appService.completeOnboarding()` → `onComplete()`
  - `RoleConfirmModal` 取消 → 关闭 modal，继续对话
  - 使用 `roleCreatedRef = useRef(false)` 防止重复创建
  - 管家消息使用 `ChatBubble` 组件 + `BounceDots` 加载动画
- [ ] **T7.2** 引导对话使用独立 conversation（`role_id = null`，但通过前端传递 onboarding step metadata 让后端选择正确的 System Prompt）
- [ ] **T7.3** 视觉保持现有 OnboardingView 样式（居中布局、管家头部、圆角气泡、底部输入框）
- [ ] **T7.4** 管家开场由后端生成（调用一次 chat_send_message，content 为空或特殊标记 `__onboarding_start__`），确保流式体验一致

### Phase 8: App.tsx 集成引导路由 (AC: #1, #4)

- [ ] **T8.1** 修改 `App.tsx`：
  - 新增 `useEffect` 启动时调用 `appService.isFirstLaunch()`
  - 返回 `true` → `setCurrentView('onboard')`
  - 返回 `false` → `setCurrentView('butler')`（当前默认）
  - OnboardingView `onComplete` 回调中 `setCurrentView('butler')` 并刷新角色列表
- [ ] **T8.2** 确保 `currentView === 'onboard'` 时侧边栏隐藏或显示最小化状态（仅管家图标）

### Phase 9: chat_send_message 扩展支持引导模式 (AC: #8, #9)

- [ ] **T9.1** 修改 `ChatRequest` 模型，新增字段：
  ```rust
  pub struct ChatRequest {
      pub conversation_id: Option<String>,
      pub role_id: Option<String>,
      pub content: String,
      #[serde(default)]
      pub onboarding_step: u8,  // 新增：0=butler, 1-5=onboarding step
  }
  ```
  > **注意：** 使用 `u8` + `#[serde(default)]`（默认 0），而非 `Option<u8>`，简化后续判断逻辑。
- [ ] **T9.2** 修改 `commands/chat.rs` `chat_send_message`：
  - 读取 `request.onboarding_step`（u8, default 0）
  - 若 > 0：注册到 `OnboardingConversations` HashMap（conv_id → step），使用该 step
  - 若 = 0 但 conv_id 在 `OnboardingConversations` 中存在 → 使用 stored step+1（cap 5）
  - 否则 → butler 路径（step=0）
  - step > 0 → `build_onboarding_messages` + `get_onboarding_chat_options(step)`
  - step = 0 → `build_butler_messages` + `ChatOptions::default()`
- [ ] **T9.3** 前端 `chatService.sendMessage` 的 `ChatRequest` TS 类型同步更新：
  ```typescript
  export interface ChatRequest {
    conversationId?: string;
    roleId?: string;
    content: string;
    onboardingStep?: number;  // 新增
  }
  ```

### Phase 10: 测试 (AC: #10)

- [ ] **T10.1** `db/roles.rs` 底部 `#[cfg(test)] mod tests`：create + list 单元测试（内存 SQLite）
- [ ] **T10.2** `db/app_settings.rs` 底部 `#[cfg(test)] mod tests`：get/set 单元测试
- [ ] **T10.3** `commands/app.rs` 测试 `is_first_launch` 逻辑
- [ ] **T10.4** 运行 `cargo test` + `npm run test:frontend`：全部通过

## Dev Notes

### ⚠️ 致命陷阱清单

#### 陷阱 1：Migration 003 在主库 `egosync.db` 运行，不是 `conversations.db`

`roles` 表属于主数据，写入 `egosync.db`。迁移文件 `003_roles.sql` 通过 `sqlx::migrate!("./migrations")` 自动在主库池上运行。

**关键：** `pool.rs` 中 `run_migrations()` 调用 `sqlx::migrate!`，它会自动发现 `migrations/` 下所有 `.sql` 文件并按序号执行。003 号文件放进去即可，无需手动调用。

#### 陷阱 2：`app_settings` 表已存在于 001 migration

`app_settings` 表已在 `001_initial_schema.sql` 中创建：`(key TEXT PRIMARY KEY, value TEXT, updated_at TEXT)`。本 Story 只需要 **读写** 该表，**不需要** 创建新 migration。

读写方式：
```rust
// 写入
sqlx::query("INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)")
    .bind("onboarding_completed").bind("true").bind(&now)
    .execute(pool).await?;

// 读取
sqlx::query_as::<_, (Option<String>,)>("SELECT value FROM app_settings WHERE key = ?1")
    .bind("onboarding_completed")
    .fetch_optional(pool).await?;
```

#### 陷阱 3：现有 `OnboardingView` 是纯 mock，必须完全替换逻辑

当前 `OnboardingView.tsx`（84 行）使用硬编码 `setTimeout` 模拟对话：
- step 0: 输入名字 → mock 回复
- step 1: 描述工作 → mock "产品经理" 建议
- step 2: 确认 → mock 完成

本 Story 需要：
1. 保留 **视觉样式**（header、chat area、input area 的 CSS 完全保留）
2. **替换所有逻辑**：mock chat 数组 → 真实 `useTauriEvent` 流式消息
3. `handleSend` 改为调用 `chatService.sendMessage({ content, onboardingStep: step })`
4. 管家回复通过 `llm:stream` event 实时渲染

#### 陷阱 4：引导对话的 conversation + step 管理

引导对话需要一个 conversation record（因为 `chat_send_message` 需要 `conversation_id`）：
- 前端 step 从 1 开始（避开 falsy 0 值），范围 1-5
- 引导开始时调用 `chatService.sendMessage({ content: '', onboardingStep: 1 })`
- 后端 `OnboardingConversations`（`DashMap<String, u8>` 或 `Mutex<HashMap>`）记录 conv_id → step
- 后续消息如不传 `onboardingStep`（默认 0），后端自动查 map 并 step+1（cap 5）
- 前端每次发送消息时也主动递增并传递 `onboardingStep`

**推荐方案：** 前端启动引导时先调用 `chat_new_conversation` 获取 conversationId，然后第一次 `sendMessage` 用空 content + `onboardingStep: 1` 触发管家开场。

#### 陷阱 5：引导 step ≥ 3 使用 Function Calling 而非正则标记

实际实现使用 OpenAI Function Calling 机制：
- `create_role_tool_definition()` 返回 JSON Schema 描述 `create_role` 函数（参数：name, icon, color, goal）
- `get_onboarding_chat_options(step)`:
  - step < 3: `tools=None, tool_choice=None`（纯对话）
  - step ≥ 3: `tools=[create_role], tool_choice="required"`（强制调用）
- LLM 返回 `tool_calls` → 后端 `StreamEvent::ToolCall("create_role", args)` → `execute_create_role` → emit `role:proposed` 事件
- 前端监听 `role:proposed` → 弹出 `RoleConfirmModal`（可编辑 name/icon/color/goal）
- 用户确认 → `roleService.create()` 实际写库；取消 → 关闭 modal

**不再使用** `<!--ROLE_CREATE:{...}-->` 正则匹配方案。

#### 陷阱 6：`invoke_handler` 注册顺序

在 `lib.rs` 的 `invoke_handler` 中追加新 commands 时，只需在现有列表末尾添加：
```rust
commands::role::role_create,
commands::role::role_list,
commands::app::app_is_first_launch,
commands::app::app_complete_onboarding,
commands::app::app_is_llm_configured,
```

#### 陷阱 7：前端 `ChatRequest` 类型需要同步更新

`egosync-app/src/types/chat.ts` 中的 `ChatRequest` 接口需要添加 `onboardingStep?: number`（对应 Rust `u8` + `serde(default)`，前端不传时后端默认 0 = butler 路径）。同时 `chatService.sendMessage` 的参数签名不变（接受 `ChatRequest` 对象）。

#### 陷阱 8：App.tsx 启动时的异步检测

`App.tsx` 中检测首次启动需要异步调用 Tauri invoke。初始 `currentView` 应设为 `'loading'` 或保持 `'butler'`，`useEffect` 内调用 `appService.isFirstLaunch()` 后再决定是否切换到 `'onboard'`。

```typescript
useEffect(() => {
  appService.isFirstLaunch().then(isFirst => {
    if (isFirst) setCurrentView('onboard');
  }).catch(() => {
    // invoke 失败（可能是开发模式无后端），保持 butler 视图
  });
}, []);
```

**注意：** 不要用 `useState('onboard')` 作为初始值，否则已完成引导的用户会闪烁看到引导页面。

#### 陷阱 9：`DbPool` vs `ConversationsPool` 类型区分

- `role_create` / `role_list` / `app_*` commands 使用 `State<'_, DbPool>`（主库）
- `chat_send_message` 使用 `State<'_, ConversationsPool>`（对话库）+ `State<'_, DbPool>`（读 LLM 配置）

两者类型不同（`DbPool = SqlitePool` vs `ConversationsPool` newtype），Tauri State 能正确区分。

#### 陷阱 10：`DEFAULT_ROLES` mock 数据在引导后应被替换

`App.tsx` 当前使用 `DEFAULT_ROLES` 常量初始化 roles state。引导完成后需要从后端加载真实角色列表：

```typescript
// OnboardingView onComplete 后
roleService.list().then(setRoles);
```

同时在 App 启动时也应加载真实角色列表（如果已完成引导）：
```typescript
useEffect(() => {
  appService.isFirstLaunch().then(isFirst => {
    if (isFirst) {
      setCurrentView('onboard');
    } else {
      setCurrentView('butler');
      roleService.list().then(realRoles => {
        if (realRoles.length > 0) setRoles(realRoles);
      }).catch(() => {});
    }
  }).catch(() => {});
}, []);
```

### 前一 Story (1.7) 关键经验

- **SQLx 0.8** 使用运行时查询 `sqlx::query_as::<_, T>(sql)...`，不用 `sqlx::query!` 宏
- **Tauri 2.x async Command** 的 `State` 参数生命周期必须是 `'_`
- **AppError** 使用手动 `impl Serialize`（tagged JSON format）
- **lib.rs 结构**：`mod declarations` + Builder 链式调用 `.setup()` + `.invoke_handler()` + `.run()`
- **现有 Cargo.toml 依赖**：tauri 2, serde, serde_json, tokio (full), tracing, tracing-subscriber, async-trait, keyring 3, thiserror 1, sqlx 0.8 (runtime-tokio, sqlite), uuid 1, reqwest 0.12 (json, stream), futures 0.3, chrono 0.4, tokio-util 0.7
- **Tauri Event 使用 `tauri::Emitter` trait**（不是 `tauri::Manager`）
- **流式对话已实现**：`chat_send_message` 已能流式响应、持久化、并发控制、取消，本 Story 复用该基础设施

### Project Structure Notes

**本 Story 新建文件：**

| 文件 | 内容 |
|---|---|
| `egosync-app/src-tauri/migrations/003_roles.sql` | roles 表 |
| `egosync-app/src-tauri/src/db/roles.rs` | 角色 DB CRUD |
| `egosync-app/src-tauri/src/db/app_settings.rs` | app_settings 读写工具 |
| `egosync-app/src-tauri/src/models/role.rs` | Role, CreateRoleInput |
| `egosync-app/src-tauri/src/commands/role.rs` | role_create, role_list |
| `egosync-app/src-tauri/src/commands/app.rs` | app_is_first_launch, app_complete_onboarding, app_is_llm_configured |
| `egosync-app/src/types/role.ts` | TS 类型定义 |
| `egosync-app/src/services/roleService.ts` | 前端 service 封装 invoke |
| `egosync-app/src/services/appService.ts` | 前端 app service |
| `egosync-app/src/lib/roleIcons.ts` | 24 个 Lucide 图标白名单 + 12 色白名单 + 映射工具 |
| `egosync-app/src/components/onboarding/RoleConfirmModal.tsx` | 角色确认弹窗（可编辑 name/icon/color/goal） |

**本 Story 修改文件：**

| 文件 | 修改内容 |
|---|---|
| `egosync-app/src-tauri/src/db/mod.rs` | 添加 `pub mod roles;` + `pub mod app_settings;` |
| `egosync-app/src-tauri/src/models/mod.rs` | 添加 `pub mod role;` |
| `egosync-app/src-tauri/src/commands/mod.rs` | 添加 `pub mod role;` + `pub mod app;` |
| `egosync-app/src-tauri/src/lib.rs` | 注册 5 个新 commands |
| `egosync-app/src-tauri/src/services/agent_engine.rs` | 新增 `build_onboarding_messages` + `ONBOARDING_SYSTEM_PROMPT` + `create_role_tool_definition()` + `get_onboarding_chat_options(step)` + `execute_create_role` + `SUPPORTED_ICONS` 枚举 |
| `egosync-app/src-tauri/src/commands/chat.rs` | `chat_send_message` 支持 `onboarding_step` 分支 + `OnboardingConversations` 服务端状态管理 |
| `egosync-app/src-tauri/src/models/chat.rs` | `ChatRequest` 新增 `onboarding_step: u8`（`#[serde(default)]`） + 新增 `RoleProposedPayload` struct |
| `egosync-app/src-tauri/src/llm/traits.rs` | `ChatOptions` 新增 `tool_choice: Option<String>` 字段 |
| `egosync-app/src-tauri/src/llm/openai.rs` | `chat_stream` 序列化 `tool_choice` + 解析 `StreamEvent::ToolCall` |
| `egosync-app/src/types/chat.ts` | `ChatRequest` 新增 `onboardingStep?: number` |
| `egosync-app/src/components/onboarding/OnboardingView.tsx` | 完全重写逻辑，保留视觉 |
| `egosync-app/src/App.tsx` | 启动时检测首次启动 + 引导完成后加载真实角色 |

**不修改的文件（确认无需改动）：**
- `egosync-app/src-tauri/src/error.rs` — AppError 已含所有需要的变体
- `egosync-app/src-tauri/src/db/pool.rs` — migrations 自动发现 003 文件，无需改动
- `egosync-app/src-tauri/src/services/secret_store.rs` — 不涉及
- `egosync-app/src-tauri/src/llm/` — Provider 层不变，agent_engine 层处理引导差异
- `egosync-app/src/services/chatService.ts` — sendMessage 签名不变，ChatRequest 类型更新即可
- `egosync-app/src/hooks/useTauriEvent.ts` — 已有，直接复用

**与架构文档对齐：**

| 架构规范要求 | 本 Story 实现 |
|---|---|
| `roles` 表在主库 `egosync.db` | ✅ migration 003 在主库 |
| 角色 CRUD 通过 Tauri Command | ✅ `role_create` / `role_list` |
| 前端 service 层封装 invoke | ✅ `roleService.ts` / `appService.ts` |
| Command 层只做参数解析 → 调 Service → 返回结果 | ✅ 薄层 command |
| 组件域目录 `components/onboarding/` | ✅ 已有位置 |
| 错误通过管家自然语言传达 | ✅ 引导中错误在对话中展示 |
| serde `rename_all = "camelCase"` | ✅ Role struct |
| UUID v4 作为 ID | ✅ `uuid::Uuid::new_v4()` |

### 类型映射参考

| DB 列 (egosync.db) | Rust 类型 | TS 类型 |
|---|---|---|
| `roles.id TEXT` | `String` | `string` |
| `roles.name TEXT` | `String` | `string` |
| `roles.icon TEXT` | `String` | `string` |
| `roles.color TEXT` | `String` | `string` |
| `roles.goal TEXT` | `String` | `string` |
| `roles.status TEXT` | `String` | `'active' \| 'archived'` |
| `roles.energy INTEGER` | `i32` | `number` |
| `roles.skills_config TEXT` | `String` (JSON) | `string` |
| `roles.proactivity_level TEXT` | `String` | `'passive' \| 'moderate' \| 'proactive'` |
| `roles.archived_at TEXT` | `Option<String>` | `string \| null` |

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` #Story 1.8] — 完整 AC 定义
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Data Architecture] — roles 表设计、主库 schema
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Implementation Patterns] — 命名规范、serde 标注
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Frontend Architecture] — service 层封装 invoke
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Journey 1: 冷启动] — 5 步引导对话流设计
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` #Emotional Journey] — 首次打开情感：好奇、被关注
- [Source: `_bmad-output/project-context.md` #框架特定规则] — Tauri IPC、Command 薄层规则
- [Source: `egosync-app/src-tauri/src/lib.rs`] — 当前 app setup 和 invoke_handler 结构
- [Source: `egosync-app/src-tauri/src/db/pool.rs`] — 当前 DB 初始化模式（migration 自动发现）
- [Source: `egosync-app/src-tauri/src/services/agent_engine.rs`] — 当前 build_butler_messages 模式
- [Source: `egosync-app/src-tauri/migrations/001_initial_schema.sql`] — app_settings 表已存在
- [Source: `egosync-app/src/components/onboarding/OnboardingView.tsx`] — 当前 mock 实现（待替换）
- [Source: `egosync-app/src/App.tsx`] — 当前 currentView 状态管理
- [Source: `_bmad-output/implementation-artifacts/1-7-butler-first-streaming-conversation.md`] — 前一 Story 全部经验

### 验证命令清单

```bash
cd egosync-app/src-tauri
cargo check                    # 编译检查
cargo test                     # 单元测试
cargo clippy -- -D warnings    # lint

cd GUI
npm run test:frontend          # 前端测试无回归
npm run tauri dev              # 集成验证：首次启动 → 引导对话 → 创建角色 → 进入主视图
```

## Dev Agent Record

### Implementation Plan

- Phase 1-2: roles 表 migration + 数据模型 + DB CRUD
- Phase 3: app_settings 读写工具
- Phase 4: Tauri Commands（role + app）
- Phase 5: 引导 System Prompt 分层 + Function Calling（`create_role_tool_definition` + `get_onboarding_chat_options`）
- Phase 6: 前端 types/service
- Phase 7: OnboardingView 重写 + `RoleConfirmModal` + `roleIcons.ts`（核心工作量）
- Phase 8: App.tsx 集成
- Phase 9: chat_send_message 扩展 + `OnboardingConversations` 状态管理
- Phase 10: 测试

### Debug Log References

- Issue 7（step 3 对话中断）通过临时文件日志 `debug_chat.log` 诊断，根因为 `tool_choice: "required"` 格式问题，已修复并移除所有诊断日志

### Completion Notes List

1. **角色创建机制**：最终采用 OpenAI Function Calling（`create_role` tool）+ `role:proposed` 事件 + `RoleConfirmModal` 确认流程，替代原设计的 `<!--ROLE_CREATE:{...}-->` 正则标记方案
2. **OnboardingConversations**：后端使用 `Mutex<HashMap<String, u8>>` 跟踪引导对话的 conv_id → step 映射，支持 step 自动递增
3. **step-gated tools**：step < 3 不提供 tools（纯自由对话），step ≥ 3 提供 `create_role` + `tool_choice="required"`（强制结构化输出）
4. **ChatRequest.onboarding_step**：使用 `u8` + `#[serde(default)]`（默认 0 = butler），而非 `Option<u8>`
5. **RoleConfirmModal**：支持编辑 name/icon/color/goal；icon 使用 Lucide React 黑白线框图标（24 个白名单）；color 使用 12 色白名单
6. **Onboarding bug fixes（Issues 1-7）**：修复了 ChatBubble/BounceDots 缺失、重复角色创建、LLM 不调用 tools、system prompt 统一、自动创建无确认、onboarding 使用 butler prompt、step 3 对话中断等问题

### File List

**新建文件：**
- `egosync-app/src-tauri/migrations/003_roles.sql`
- `egosync-app/src-tauri/src/db/roles.rs`
- `egosync-app/src-tauri/src/db/app_settings.rs`
- `egosync-app/src-tauri/src/models/role.rs`
- `egosync-app/src-tauri/src/commands/role.rs`
- `egosync-app/src-tauri/src/commands/app.rs`
- `egosync-app/src/types/role.ts`
- `egosync-app/src/services/roleService.ts`
- `egosync-app/src/services/appService.ts`
- `egosync-app/src/lib/roleIcons.ts`
- `egosync-app/src/components/onboarding/RoleConfirmModal.tsx`

**修改文件：**
- `egosync-app/src-tauri/src/services/agent_engine.rs` — ONBOARDING_SYSTEM_PROMPT, create_role_tool_definition, get_onboarding_chat_options, execute_create_role, SUPPORTED_ICONS
- `egosync-app/src-tauri/src/commands/chat.rs` — OnboardingConversations, onboarding_step 分支逻辑
- `egosync-app/src-tauri/src/models/chat.rs` — ChatRequest.onboarding_step: u8, RoleProposedPayload
- `egosync-app/src-tauri/src/llm/traits.rs` — ChatOptions.tool_choice
- `egosync-app/src-tauri/src/llm/openai.rs` — tool_choice 序列化, StreamEvent::ToolCall 解析
- `egosync-app/src-tauri/src/lib.rs` — invoke_handler 注册新 commands
- `egosync-app/src-tauri/src/db/mod.rs` — pub mod roles + app_settings
- `egosync-app/src-tauri/src/models/mod.rs` — pub mod role
- `egosync-app/src-tauri/src/commands/mod.rs` — pub mod role + app
- `egosync-app/src/types/chat.ts` — ChatRequest.onboardingStep
- `egosync-app/src/components/onboarding/OnboardingView.tsx` — 完全重写：role:proposed 监听, RoleConfirmModal 集成, ChatBubble/BounceDots
- `egosync-app/src/App.tsx` — isFirstLaunch 检测, onComplete 后加载真实角色

## Change Log

| 日期 | 变更 |
|------|------|
| 2026-05-22 | Story 1.8 上下文文件创建 — ready-for-dev |
| 2026-05-23 | Story 1.8 实现完成 — done；含 Function Calling 角色创建、RoleConfirmModal、OnboardingConversations、Issues 1-7 修复 |
| 2026-05-24 | 文档同步更新：Status→done, AC-9/T5.2/T7.1/T9.1/T9.2 更新为实际实现, 陷阱 4/5/7 重写, Dev Agent Record 填充, 文件列表完善 |