---
baseline_commit: 34aab28f79924cb6aa5c30944b2498871fca0186
---

# Story 2.6: 系统从每次对话中自动提炼结构化记忆

Status: done

## Story

As a 用户,
I want 系统自动从管家和角色对话中提取关键信息保存为记忆,
so that 管家和角色都能记住我说过的话，下次对话时更懂我。

## Acceptance Criteria

1. **AC-1 管家/角色对话结束后后台提炼记忆**
   - **Given** 用户与管家或某个角色完成一段对话（该 conversation 中已有 ≥ 3 条完整 user messages）
   - **When** 用户 5 分钟内无新消息，或用户从该 conversation 手动新建/切换到新 conversation
   - **Then** 后台 LLM 静默提炼结构化记忆
   - **And** `memories` 表写入 ≥ 1 条有价值记忆
   - **And** 管家 conversation 写入全局记忆（`role_id = NULL`），角色 conversation 写入角色专属记忆（`role_id = <角色 id>`）
   - **And** 提炼过程不阻塞流式回复完成、不保持 `StreamingState` 锁定、不阻塞用户继续发送下一条消息

2. **AC-2 记忆字段完整且可溯源**
   - **Given** 提炼出的每条记忆
   - **Then** 每条包含：`role_id`（管家全局记忆为 `NULL`，角色专属记忆为角色 id）、`category`、`content`、`source_conversation_id`、`source_message_ids`、`created_at`
   - **And** `category` 只能是 `preference` / `task_status` / `cognition_update` / `fact`
   - **And** `source_message_ids` 是 JSON 数组，且每个 id 必须属于该 `source_conversation_id` 的原始 messages

3. **AC-3 无价值对话不写入记忆**
   - **Given** 对话内容只有寒暄、确认收到、空泛闲聊或无持久价值内容
   - **When** 提炼完成
   - **Then** 不写入任何记忆，允许输出 0 条
   - **And** tracing 记录 debug/info 级摘要即可，不向用户展示错误或“无记忆”提示

4. **AC-4 数据库 schema 正确落地**
   - **Given** 主数据库 `egosync.db`
   - **Then** `egosync-app/src-tauri/migrations/004_memories.sql` 创建 `memories` 表
   - **And** `role_id` 允许 `NULL`，`NULL` 表示管家全局记忆；非空时通过数据库外键关联 `roles.id`，删除角色时级联删除该角色专属记忆
   - **And** `source_conversation_id` 存储 `conversations.db.conversations.id` 的文本引用并建立索引
   - **And** 由于对话日志在独立 `conversations.db`，不得尝试对 `conversations.id` 建 SQLite 跨库外键；必须由 `memory_pipeline` 在写入前通过 `ConversationsPool` 做应用层校验

5. **AC-5 后端 memory pipeline 行为稳定**
   - **Given** Rust 后端
   - **Then** 新增 `memory_pipeline` 服务：加载 conversation messages → 构造提炼 prompt → 调用默认 LLM Provider → 解析结构化 JSON → 写入 `memories` 表
   - **And** LLM 超时、Keyring 缺失、格式错误、JSON 不合法、无默认 Provider 等失败只写 tracing 日志，不向前端 emit 错误，不影响对话完成状态
   - **And** 重复触发同一 conversation 提炼时，不产生重复记忆

6. **AC-6 管家记忆与角色记忆分域保存**
   - **Given** 管家 conversation（`role_id = None`）
   - **When** 对话完成或 idle timer 到期
   - **Then** 写入管家全局记忆，`memories.role_id = NULL`
   - **And** 仅提炼适合全局使用的信息，如用户总体偏好、跨角色约束、长期事实、价值观/工作方式偏好
   - **Given** 含 onboarding system marker 的管家 conversation 或内部 system-only conversation
   - **When** 对话完成或 idle timer 到期
   - **Then** 提炼 prompt 排除 system 脚手架消息；只有满足阈值的完整 user messages 可进入记忆提炼，纯内部 system-only conversation 不写入 `memories`

7. **AC-7 测试通过**
   - `cd GUI && npx tsc --noEmit`
   - `cd GUI && npm run test:frontend`
   - `cd egosync-app/src-tauri && cargo test`
   - 至少覆盖：memories migration/DB 写入、管家 `role_id = NULL` 记忆、角色专属记忆、category 白名单、source_message_ids 校验、无价值 JSON 输出 0 条、提炼失败不传播到 chat flow、5 分钟 debounce 取消旧任务

## Tasks / Subtasks

### Phase 1: 数据模型与 migration（AC: #2, #4, #5）

- [x] T1.1 新增 `egosync-app/src-tauri/migrations/004_memories.sql`
  - 表名：`memories`
  - 字段：`id TEXT PRIMARY KEY NOT NULL`
  - `role_id TEXT`
  - `category TEXT NOT NULL CHECK(category IN ('preference','task_status','cognition_update','fact'))`
  - `content TEXT NOT NULL`
  - `source_conversation_id TEXT NOT NULL`
  - `source_message_ids TEXT NOT NULL DEFAULT '[]'`
  - `created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))`
  - `FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE`（SQLite 对 `NULL` 外键不做校验，用于管家全局记忆）
- [x] T1.2 在 migration 中新增索引
  - `idx_memories_role_id` on `role_id`
  - `idx_memories_category` on `category`
  - `idx_memories_source_conversation_id` on `source_conversation_id`
  - `idx_memories_created_at` on `created_at`
  - `004_memories.sql` 保留初始唯一去重索引：`idx_memories_source_dedupe` on `(source_conversation_id, category, content)`，避免已执行迁移的 checksum 风险
  - `005_memory_role_scoped_dedupe.sql` 删除并重建同名唯一索引，最终运行态为 `COALESCE(role_id, '')`, `source_conversation_id`, `category`, `source_message_ids`，允许同一来源记忆同时存在于管家全局和匹配角色范围
- [x] T1.2b 新增 `egosync-app/src-tauri/migrations/005_memory_role_scoped_dedupe.sql`
  - `DROP INDEX IF EXISTS idx_memories_source_dedupe`
  - `CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_source_dedupe ON memories(COALESCE(role_id, ''), source_conversation_id, category, source_message_ids)`
  - 该迁移只变更运行态索引，不回改 `004_memories.sql`，避免真实库已执行 migration 后出现校验不一致
- [x] T1.3 `egosync-app/src-tauri/src/db/pool.rs`：确保主库连接启用外键
  - 优先使用 `SqliteConnectOptions::foreign_keys(true)`；若不可用，则连接后执行 `PRAGMA foreign_keys = ON`
  - 不改变 `conversations.db` 独立迁移机制
- [x] T1.4 新增 `egosync-app/src-tauri/src/models/memory.rs`
  - `Memory` 使用 `#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]`
  - `#[serde(rename_all = "camelCase")]`
  - 字段与 AC-2 完全一致；Rust 字段 snake_case，JSON 输出 camelCase
  - 新增内部输入结构 `NewMemory` 或 `ExtractedMemory`，供 pipeline/db 层使用
- [x] T1.5 更新 `egosync-app/src-tauri/src/models/mod.rs`：加入 `pub mod memory;`
- [x] T1.6 新增 `egosync-app/src-tauri/src/db/memories.rs`
  - `insert_memories(pool, role_id: Option<&str>, source_conversation_id, extracted) -> Result<usize, AppError>`
  - 使用 `INSERT OR IGNORE` 或等价方式配合唯一索引防重复
  - 写入前校验 category 白名单、content 非空、source_message_ids JSON 数组非空且由 pipeline 传入已验证 ids
- [x] T1.7 更新 `egosync-app/src-tauri/src/db/mod.rs`：加入 `pub mod memories;`

### Phase 2: memory_pipeline 服务（AC: #1, #2, #3, #5, #6）

- [x] T2.1 新增 `egosync-app/src-tauri/src/services/memory_pipeline.rs`
  - 入口函数建议：`extract_for_conversation(main_pool: DbPool, conv_pool: ConversationsPool, conversation_id: String) -> Result<usize, AppError>`
  - 函数内部通过 `conversations` 查询 conversation 与 messages，不依赖前端传 role_id
- [x] T2.2 `db/conversations.rs` 新增只读 helper
  - `get_conversation(pool, conversation_id) -> Result<Option<Conversation>, AppError>`
  - 仅查询 `SELECT id, role_id, title, started_at, updated_at FROM conversations WHERE id = ?`
  - 不创建新 conversation，避免提炼路径产生副作用
- [x] T2.3 pipeline 前置过滤与归属判定
  - conversation 不存在 → 返回 `Ok(0)` 并 warn
  - `role_id = Some(id)` → 提炼为角色专属记忆
  - `role_id = None` 且不是 onboarding/system-only → 提炼为管家全局记忆（写入 `memories.role_id = NULL`）
  - onboarding/internal conversation → 返回 `Ok(0)`；可通过 `system` 消息 `[onboarding_start]` 或调用方传入的 onboarding 标记避免写入
  - 完整 user messages 数量 `< 3` → 返回 `Ok(0)`
  - messages 中 `system` 角色和空 content 不进入 prompt
- [x] T2.4 构造 LLM 提炼 prompt
  - system prompt 强制输出严格 JSON，不允许 Markdown/code fence/解释文本
  - system prompt 根据归属区分：管家全局记忆只记录跨角色/全局偏好，角色专属记忆只记录该角色领域内的偏好、任务状态、认知更新和事实
  - JSON 形状：`{"memories":[{"category":"preference|task_status|cognition_update|fact","content":"...","sourceMessageIds":["..."]}]}`
  - 明确“无有价值信息时输出 `{"memories":[]}`”
  - 输入消息必须带 message id、role、created_at、content，方便 LLM 引用 sourceMessageIds
  - Prompt 必须要求只记录对未来有持续价值的信息，不记录一次性寒暄
- [x] T2.5 调用默认 LLM Provider
  - 复用 `agent_engine::resolve_default_provider(&main_pool)`
  - 使用 `ChatOptions { disable_thinking: true, tools: None, tool_choice: None }`
  - 聚合 `StreamEvent::Token` 到字符串，`Done` 后解析 JSON
  - `StreamEvent::Error` 只记录 tracing warn，返回 `Ok(0)`
- [x] T2.6 解析与清洗 LLM JSON
  - 去掉常见 code fence 包裹，但不要接受自然语言解释作为成功
  - category 不在白名单 → 丢弃该条并 warn
  - content trim 后为空 → 丢弃
  - `sourceMessageIds` 为空或包含不属于该 conversation 的 id → 丢弃该条
  - 全部无效 → 返回 `Ok(0)`
- [x] T2.7 写入 `memories` 表
  - 调用 `db::memories::insert_memories`
  - 写入前标准化 `source_message_ids`：trim、去空、排序去重、序列化为 JSON 数组
  - 同一 `role_id` 范围内按 `source_conversation_id + category + source_message_ids` 跳过重复写入，避免 LLM 改写 content 后重复提取
  - 返回实际插入数量
  - tracing info 包含 `conversation_id`、`role_id`（管家全局记忆为 None）、`inserted_count`，不记录完整用户隐私内容
- [x] T2.7b 管家全局事实同步到匹配角色记忆
  - 管家 conversation 先写入 `role_id = NULL` 的全局记忆
  - 对 `preference` / `cognition_update` / `fact` 记忆，调用角色归属判定 prompt，在 active roles 中选择明确匹配的角色并额外写入该角色范围
  - `task_status` 不走事实同步；角色相关任务、安排、日程、待办应由管家委派路径处理，而不是作为事实复制到角色
  - 角色归属失败只记录 warning 并跳过，不影响管家回复和全局记忆写入
- [x] T2.8 更新 `egosync-app/src-tauri/src/services/mod.rs`：加入 `pub mod memory_pipeline;`

### Phase 3: 对话完成触发与 5 分钟 debounce（AC: #1, #5, #6）

- [x] T3.1 在 `egosync-app/src-tauri/src/commands/chat.rs` 新增 `MemoryExtractionState`
  - 类型：`Arc<Mutex<HashMap<String, CancellationToken>>>`
  - 用于每个 conversation 的 idle debounce；新消息完成后取消旧 token，重新计时 5 分钟
- [x] T3.2 在 `egosync-app/src-tauri/src/lib.rs` setup 中 `app.manage(commands::chat::MemoryExtractionState::default())`
- [x] T3.3 在 `chat_send_message` 的 spawned task 中接入调度
  - 必须在 `agent_engine::run_stream(...).await` 返回后执行
  - 必须先从 `StreamingState` 移除 `conv_id`、从 `CancelTokens` 移除 cancel token，再调度 memory extraction
  - 调度条件以 conversation 实际归属为准：角色 conversation 和管家 conversation 都可调度；onboarding 的 system 脚手架消息由 pipeline 过滤，纯内部 system-only conversation 因完整 user messages 不足而不写入记忆
  - 调度本身再 `tokio::spawn`，不得 await 5 分钟 sleep 阻塞 chat task
- [x] T3.4 debounce 行为
  - 新一轮同 conversation 回复完成时，取消旧 extraction token
  - 新 token sleep `Duration::from_secs(300)`
  - sleep 结束且 token 未取消 → 调 `memory_pipeline::extract_for_conversation(...)`
  - 提炼完成或失败后，从 `MemoryExtractionState` 移除该 conversation token
- [x] T3.5 手动新建 conversation 时立即收尾旧 conversation
  - 在 `chat_new_conversation(old_conversation_id, role_id, ...)` 中，若 `old_conversation_id` 存在且旧 conversation 是管家或角色 conversation，则取消该 conversation 的 debounce token 并立即后台触发一次 `extract_for_conversation`
  - 不要在 `chat_delete_conversation` 删除前提炼；用户删除 conversation 时源消息即将被删除，不应生成无法溯源的新记忆
- [x] T3.6 不改变现有流式 UI contract
  - `llm:stream` payload 不新增字段
  - 不改变 `message_id` 两气泡分桶逻辑
  - 不改变 `chat_stop_streaming` 的取消语义

### Phase 4: 测试与验证（AC: #7）

- [x] T4.1 `egosync-app/src-tauri/src/db/memories.rs` 单测
  - migration 创建表成功
  - 写入 1 条管家全局记忆成功（`role_id = NULL`）
  - 写入 1 条角色专属 preference 记忆成功
  - invalid category 被拒绝或不写入
  - 同一角色范围内重复 `source_conversation_id + category + normalized source_message_ids` 不重复插入；`004` 的 content 精确去重只保留为初始迁移历史
- [x] T4.2 `egosync-app/src-tauri/src/services/memory_pipeline.rs` 单测纯函数
  - `parse_extraction_response` 接受严格 JSON
  - code fence 包裹 JSON 可清洗
  - 自然语言非 JSON 返回空/错误但不 panic
  - `sourceMessageIds` 不属于 conversation 时被过滤
  - `{"memories":[]}` 返回 0 条
- [x] T4.3 `egosync-app/src-tauri/src/commands/chat.rs` 单测
  - 同 conversation 新 token 会取消旧 memory extraction token
  - 管家 conversation 会调度提炼为全局记忆
  - 角色 conversation 会调度提炼为角色专属记忆
  - onboarding 不调度提炼
- [x] T4.4 运行命令
  - `cd GUI && npx tsc --noEmit`
  - `cd GUI && npm run test:frontend`
  - `cd egosync-app/src-tauri && cargo test`
- [x] T4.5 若本 story 改动了 UI 或前端行为，启动 `cd GUI && npm run tauri dev` 做人工验证；本 story 默认后端后台提炼，无 UI 改动时可记录“无新增 UI，未做浏览器交互验证”

## Dev Notes

### 当前实现态

- `chat_send_message` 在 `egosync-app/src-tauri/src/commands/chat.rs` 中先写入 user message 和空 assistant message，再 `tokio::spawn` 执行 `agent_engine::run_stream`；函数本身立即返回 user message。流式状态在后台 task 末尾移除。此处是 2.6 的正确触发点，但必须在移除 `StreamingState` 后再启动记忆提炼，避免 AC-1 的“用户下次对话不阻塞”被破坏。[Source: `egosync-app/src-tauri/src/commands/chat.rs`:47-209]
- `agent_engine::try_run_opencode_stream` 已有 opencode 优先路径，普通 completion 在函数末尾落库 assistant 内容并 emit done；delegate 两气泡路径会创建 follow-up assistant message。memory pipeline 不应侵入这些分桶和 emit 逻辑。[Source: `egosync-app/src-tauri/src/services/agent_engine.rs`:660-1129]
- `agent_engine::run_stream` 在 opencode 不可用时 fallback 到 `LlmProvider`，并在 fallback 路径中处理 tool calls、follow-up、onboarding 兜底。memory pipeline 应挂在 `run_stream` 外层完成后，而不是分别改 opencode/fallback 两套内部逻辑。[Source: `egosync-app/src-tauri/src/services/agent_engine.rs`:1131-1201,1420-1627]
- 当前 `db/conversations.rs` 已有 `list_messages`、`get_recent_messages`、`list_conversations_by_role`，但没有 `get_conversation(id)`；2.6 需要新增只读 helper。[Source: `egosync-app/src-tauri/src/db/conversations.rs`:255-286]
- 当前 `models/chat.rs` 的 `Message` 已包含 `routing_metadata`，用于 Story 2.3 的委派审计；memory source ids 应直接引用 `messages.id`，不要复用 `routing_metadata` 字段。[Source: `egosync-app/src-tauri/src/models/chat.rs`:18-32]
- 当前主库 migration 目录只有 `001_initial_schema.sql` 与 `003_roles.sql`，对话库 schema 在 `002_conversations.sql` 但由 `init_conversations_db` 通过 `include_str!` 单独运行；2.6 的 `004_memories.sql` 属于主库 `egosync.db`，会由 `sqlx::migrate!("./migrations")` 执行。[Source: `egosync-app/src-tauri/src/db/pool.rs`:43-50,75-100]
- 当前已新增 `db/memories.rs`、`models/memory.rs`、`services/memory_pipeline.rs`，并在后续增量中接入 `commands/memory.rs`、`memoryService.ts`、`types/memory.ts` 与 `MemoryTab.tsx`。MemoryTab 已从 mock 静态卡片改为读取真实记忆数据；Story 2.7 仍可继续补充溯源、删除和更完整的查询交互。[Source: `egosync-app/src-tauri/src/db/mod.rs`; `egosync-app/src-tauri/src/models/mod.rs`; `egosync-app/src-tauri/src/services/mod.rs`; `egosync-app/src-tauri/src/commands/memory.rs`; `egosync-app/src/components/role/MemoryTab.tsx`]

### 架构冲突与裁决

| 冲突 | Epic 原文 | 当前架构/代码事实 | 2.6 裁决 |
|---|---|---|---|
| 记忆归属 | Epic 2 主要强调“角色记忆” | 产品心智中管家也会沉淀通用偏好与跨角色上下文；管家 conversation 当前以 `role_id = NULL` 存储 | `memories.role_id` 允许 NULL：NULL = 管家全局记忆，非 NULL = 角色专属记忆。Story 2.7 查询 UI 可分别展示管家记忆和角色记忆 |
| `source_conversation_id` 外键 | “外键关联 `roles.id` 和 `conversations.id`” | `conversations` 表在独立 `conversations.db`，主库 `egosync.db` 通过 `sqlx::migrate!` 管理；SQLite 不支持直接跨独立 DB 文件声明 FK | `role_id` 做可空 FK；`source_conversation_id` 做 indexed TEXT + pipeline 写入前校验 conversation 存在。不得合并 DB 或 attach DB 只为满足 FK 字面要求 |

### 必须复用/遵守的既有资产

| 资产 | 用法 |
|---|---|
| `agent_engine::resolve_default_provider(&main_pool)` | memory_pipeline 调默认 LLM Provider，不新增 Provider 解析逻辑 |
| `llm::traits::{ChatCompletionMessage, ChatOptions, StreamEvent}` | 记忆提炼 LLM 调用沿用现有 streaming provider trait |
| `db::conversations::list_messages` | 构造提炼上下文和校验 source_message_ids |
| `db::roles` + `roles.id` | 非空 `memories.role_id` 的外键来源；管家全局记忆使用 NULL，不查 roles |
| `SqliteConnectOptions` / `sqlx::migrate!("./migrations")` | 主库 migration 继续走现有模式 |
| `tracing` | 提炼失败只写日志，不通过 UI 报错 |

### Prompt 与 JSON 约束

- 输出必须是严格 JSON 对象，顶层只有 `memories` 数组。
- 不允许 Markdown code fence；解析层可容忍并清洗 code fence，但 prompt 不应鼓励。
- 管家全局 conversation 现在采用双层归属：先保存 `role_id = NULL` 的全局记忆，再将非 `task_status` 的角色事实/偏好/认知更新通过角色归属判定同步到匹配 active 角色。
- `task_status` 明确不走事实同步；与角色相关的任务、安排、日程、待办由 Story 2.3 的委派路径处理，避免把“需要行动的任务”误当作普通事实复制。
- 最终去重口径不再依赖 content 精确相等，而是同一 `role_id` 范围内基于来源 conversation、category 与标准化后的 source message id 集合去重；这能过滤同一来源的 LLM 改写重复记忆。
- 由于 `source_message_ids` 会排序去重，同一组来源消息顺序不同也会被视为重复来源。

### 实现边界

- MemoryTab 已接入真实记忆查询：角色页默认查询当前角色记忆，管家/全局视图可通过 `includeRoleMemories` 查询全部记忆。
- 选择性遗忘仍不做；Story 2.8 负责 delete command。
- 对话时上下文注入记忆已在管家 prompt 中部分落地：管家会看到全局记忆和角色记忆摘要，用于已知事实直接回答；更完整的引用解释和“为什么”推理链仍可留给 Story 2.9。
- 不做 embedding、相似度、长期归档或压缩；V1 用来源消息集合和角色范围唯一索引去重。
- 不新增外部 crate；现有 `serde_json`、`chrono`、`uuid`、`tokio` 足够。
- 不在 prompt 或日志中输出完整 API Key、Provider 配置或用户大段隐私原文。

### 前序 Story 经验

- Story 2.5 扩展了 `agent_engine` 的 tool 分支与管家 prompt，说明该文件已高度复杂；2.6 不应把提炼逻辑塞进 `agent_engine.rs`，应新建 `services/memory_pipeline.rs` 并只在 chat 完成处调度。
- Story 2.5 使用 `app_settings` 存冷却，适合轻量 key-value；但 2.6 是核心领域数据，必须用正式 `memories` 表和 migration，不能塞进 `app_settings`。
- Story 2.5 的 Dev Record 显示 `cargo test`、`npx tsc --noEmit`、`npm run test:frontend` 可作为基础验证组合；继续沿用。
- Story 2.3/2.5 已建立“tool 后续回复不能嵌套工具”的约束；memory extraction 的 LLM 调用必须 `tools: None`，避免后台提炼意外触发 role proposal/delegate。
- Story 2.5 未运行桌面端 `tauri dev` 人工验证；2.6 若不改 UI，可明确记录无 UI 验证范围，不能声称人工 UI 已验证。

### Git Intelligence Summary

- 最近提交 `f814983 fix(chat): unlock input after stream completion` 大量修改 `ChatStream.tsx` 与测试，说明 chat completion/done 状态非常敏感；2.6 必须避免延迟 `done` 或保持输入锁定。
- 最近提交 `179a27c fix(chat): clean role proposal handoff flow` 修改 `agent_engine.rs`、`db/conversations.rs`、`lib.rs` 与 `ChatStream.tsx`，说明 opencode/tool handoff 与前端 bucket 分配刚修过；2.6 不应改 `llm:stream` payload 或 `message_id` 语义。
- 最近提交 `717be87 feat(2.0b): sync roles to opencode agents` 新增 `agent_config.rs` 并在 role CRUD 同步 opencode；2.6 的 memories 写入不需要同步 opencode config。
- 最近提交 `34aab28 docs(2.3): refresh delegate bridge implementation notes` 只更新 story 文档；实现时以当前代码为准，不以旧 story 假设为准。

### Latest Technical Information

- `sqlx::migrate!("./migrations")` 在编译期嵌入 migrations，路径相对 `Cargo.toml` 所在目录；运行时通过 `.run(pool).await` 执行。新增 migration 文件后如构建未感知，应确保 `build.rs` 或现有构建流程能触发 rerun，但本项目已有 Tauri build 流程通常会重新编译 Rust。[Source: SQLx docs `https://docs.rs/sqlx/latest/sqlx/macro.migrate.html`]
- Tauri v2 新增 command 时需要在 Rust 模块中 `#[tauri::command]` 并统一注册到 `tauri::generate_handler![...]`；本 story 默认不新增 command。若实现者为测试/调试临时加 command，完成前必须删除，不要暴露未规划 IPC。[Source: Tauri v2 docs `https://v2.tauri.app/develop/calling-rust/`]

### Project Structure Notes

#### 新增文件

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src-tauri/migrations/004_memories.sql` | NEW | 主库 memories schema、索引、去重 |
| `egosync-app/src-tauri/migrations/005_memory_role_scoped_dedupe.sql` | NEW | 将 memories 去重索引迁移为角色范围 + 来源消息集合去重 |
| `egosync-app/src-tauri/src/commands/memory.rs` | NEW | 暴露 `memory_list` / `memory_list_all` 查询命令 |
| `egosync-app/src-tauri/src/models/memory.rs` | NEW | Memory / ExtractedMemory 模型 |
| `egosync-app/src-tauri/src/db/memories.rs` | NEW | memories 写入与基础查询 helper（仅后端使用） |
| `egosync-app/src-tauri/src/services/memory_pipeline.rs` | NEW | 后台提炼主逻辑、prompt、JSON 解析 |

#### 修改文件

| Path | Action | Notes |
|---|---|---|
| `egosync-app/src-tauri/src/db/mod.rs` | UPDATE | 导出 `memories` 模块 |
| `egosync-app/src-tauri/src/models/mod.rs` | UPDATE | 导出 `memory` 模块 |
| `egosync-app/src-tauri/src/services/mod.rs` | UPDATE | 导出 `memory_pipeline` 模块 |
| `egosync-app/src-tauri/src/db/pool.rs` | UPDATE | 主库启用 FK；保持 conversations DB 独立迁移 |
| `egosync-app/src-tauri/src/db/conversations.rs` | UPDATE | 新增 `get_conversation` helper 与测试 |
| `egosync-app/src-tauri/src/commands/chat.rs` | UPDATE | MemoryExtractionState、5 分钟 debounce、manual new conversation immediate extraction |
| `egosync-app/src-tauri/src/lib.rs` | UPDATE | manage MemoryExtractionState；注册 memory 查询 command |
| `egosync-app/src-tauri/src/services/memory_pipeline.rs` | UPDATE | 管家全局记忆同步到角色记忆；`task_status` 排除角色事实同步 |
| `egosync-app/src/components/role/MemoryTab.tsx` | UPDATE | 从 mock 静态卡片改为调用 `memoryService` 展示真实记忆 |
| `egosync-app/src/services/memoryService.ts` | NEW | 前端 memory IPC service |
| `egosync-app/src/types/memory.ts` | NEW | 前端 Memory 类型 |

#### 不应改动

- `egosync-app/src/components/role/MemoryTab.tsx` 已接入真实数据，不再属于“不应改动”；后续 Story 2.7 可继续补溯源与删除交互。
- `egosync-app/src/services/memoryService.ts`、`egosync-app/src/types/memory.ts` 已作为最小查询 UI 配套新增。
- `egosync-app/src-tauri/src/services/agent_bridge.rs`、`event_router.rs`、`delegate_bridge.rs` 仍只承担 stream/委派基础设施，不直接写记忆。
- `egosync-app/src-tauri/src/services/agent_config.rs`（记忆不影响 opencode agent 配置）
- `egosync-app/src-tauri/src/llm/openai.rs` / `anthropic.rs`（复用 provider trait，不改 provider）

### Testing Requirements

- Rust 单测必须覆盖新 DB 和 JSON parser，避免依赖真实 LLM/API Key。
- `extract_for_conversation` 的集成式测试可通过拆纯函数来避免真实 provider；不要把 CI 绑定到外部模型。
- 如果需要测试 LLM 调用路径，新增 trait 注入或内部 helper，但不要为了测试引入 mock framework 依赖。
- 运行 `cargo test` 前注意 `sqlx::migrate!` 会在编译期读取 migrations；新增 migration 文件后必须让 Rust 重新编译。
- 前端未改动仍需跑 `npx tsc --noEmit` 和 `npm run test:frontend`，防止共享类型/构建被 Rust/Tauri 生成侧影响。

## References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.6 AC and Epic 2 memory requirements]
- [Source: `_bmad-output/planning-artifacts/prd-egosync.md` — FR-7/FR-8 structured memory and traceability]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — Data Architecture, separate `egosync.db` / `conversations.db`, memory pipeline placement]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` — Memory tab appears in RoleWorkspacePanel but query UI belongs to later story]
- [Source: `_bmad-output/project-context.md` — Rust/Tauri layering, migration rules, testing commands]
- [Source: `_bmad-output/implementation-artifacts/2-5-role-emergence-suggestion.md` — previous story implementation lessons]
- [Source: `egosync-app/src-tauri/src/commands/chat.rs`:47-209 — chat send flow and background stream task]
- [Source: `egosync-app/src-tauri/src/services/agent_engine.rs`:660-1129 — opencode stream completion and message persistence]
- [Source: `egosync-app/src-tauri/src/services/agent_engine.rs`:1131-1201,1420-1627 — fallback stream and tool follow-up behavior]
- [Source: `egosync-app/src-tauri/src/db/conversations.rs`:255-286 — message listing helpers]
- [Source: `egosync-app/src-tauri/src/db/pool.rs`:43-50,75-100 — main vs conversations migration paths]
- [Source: `egosync-app/src-tauri/src/lib.rs`:184-213 — current invoke handler registration]
- [Source: `egosync-app/src/components/role/MemoryTab.tsx`:1-29 — current mock UI, out of scope]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.7 (claude-opus-4-7[1m])

### Debug Log References

- 2026-05-30: `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" init_db_runs_memories_migration_with_indexes` passed.
- 2026-05-30: `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" services::memory_pipeline::tests` passed.
- 2026-05-30: `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" commands::chat::tests` passed.
- 2026-05-30: `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml"` passed: 178 lib tests + 1 integration test.
- 2026-05-30: `npx tsc --noEmit -p "egosync-app/tsconfig.json"` passed.
- 2026-05-30: `npm --prefix "GUI" run test:frontend` passed: 8 files / 47 tests.
- 2026-05-30: `npm --prefix "GUI" run build` passed.
- 2026-05-30: `cargo check --manifest-path "egosync-app/src-tauri/Cargo.toml"` passed.
- 2026-05-30: `rustfmt --edition 2021 --check "egosync-app/src-tauri/src/services/memory_pipeline.rs" "egosync-app/src-tauri/src/commands/chat.rs"` passed.

### Completion Notes List

- Added main-db `memories` schema with nullable `role_id`, category whitelist, source trace fields, FK cascade for role memories, source conversation index, and initial `(source_conversation_id, category, content)` dedupe in `004_memories.sql`.
- Added `005_memory_role_scoped_dedupe.sql` to migrate the runtime dedupe index to `COALESCE(role_id, '') + source_conversation_id + category + source_message_ids`, allowing the same source fact to exist in both global and role scopes while still suppressing reworded duplicates.
- Added `Memory` / `ExtractedMemory`, `db::memories::insert_memories`, and `db::conversations::get_conversation` with validation for category, non-empty content, and non-empty source ids.
- Added `services::memory_pipeline` to load conversations/messages, filter system-only short conversations, build scoped extraction prompts without system messages, call the default provider without tools, parse strict JSON/code-fence/surrounded JSON, validate source message ids, swallow extraction failures, insert deduped memories, and synchronize global butler facts/preferences/cognition updates into matching role memory scopes.
- Explicitly excluded `task_status` from global-to-role fact synchronization; role tasks, schedules, todos, and follow-ups are handled by the butler delegation path rather than copied as facts.
- Added chat completion scheduling: existing debounce token cancellation on new messages, 5-minute idle extraction after stream completion, immediate extraction on manual new conversation, deletion cleanup, and no `llm:stream` payload changes.
- MemoryTab now reads real memories through `memoryService` and backend commands `memory_list` / `memory_list_all`; source-message drilldown and deletion remain later-story scope.

### File List

- `egosync-app/src-tauri/migrations/004_memories.sql`
- `egosync-app/src-tauri/migrations/005_memory_role_scoped_dedupe.sql`
- `egosync-app/src-tauri/src/commands/memory.rs`
- `egosync-app/src-tauri/src/models/memory.rs`
- `egosync-app/src-tauri/src/models/mod.rs`
- `egosync-app/src-tauri/src/db/memories.rs`
- `egosync-app/src-tauri/src/db/mod.rs`
- `egosync-app/src-tauri/src/db/pool.rs`
- `egosync-app/src-tauri/src/db/conversations.rs`
- `egosync-app/src-tauri/src/services/memory_pipeline.rs`
- `egosync-app/src-tauri/src/services/mod.rs`
- `egosync-app/src-tauri/src/commands/chat.rs`
- `egosync-app/src-tauri/src/lib.rs`
- `egosync-app/src/components/role/MemoryTab.tsx`
- `egosync-app/src/services/memoryService.ts`
- `egosync-app/src/types/memory.ts`
- `egosync-app/src/components/role/MemoryTab.test.tsx`
- `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-05-30: Implemented Story 2.6 conversation memory extraction pipeline and backend scheduling; status moved to review.
- 2026-05-30: Code review patches applied; onboarding memory extraction decision resolved with message-level system filtering; status moved to done.
- 2026-05-31: Post-implementation alignment: added role-scoped dedupe migration, synchronized butler facts/preferences/cognition updates into matching role memories, excluded `task_status` from fact sync, connected MemoryTab to real memory queries, and documented butler prompt memory/delegation split.

### Review Findings

#### Decision Needed

- [x] [Review][Decision→Patch] onboarding 标记永久禁用被复用的管家会话 — 已按选项 3 解决：移除 pipeline/chat 调度中的会话级 onboarding 跳过，保留 prompt 层 system message 过滤；onboarding 后续真实用户消息可进入管家全局记忆提炼。

#### Patch

- [x] [Review][Patch] chat_new_conversation 用 `?` 让记忆资格检查阻断新建对话 [egosync-app/src-tauri/src/commands/chat.rs:~354] — 已改为旧会话 messages 单次 best-effort 查询；查询失败只 warn 并继续创建新 conversation，标题生成与记忆提炼均跳过。[blind+auditor]
- [x] [Review][Patch] remove_memory_extraction_token_if_active 在锁外检查 is_cancelled 的 TOCTOU [egosync-app/src-tauri/src/commands/chat.rs:~57] — 已改为持锁后判 `is_cancelled()`，避免旧 task 误删新 token。[blind]
- [x] [Review][Patch] LLM JSON/code-fence 清洗过窄导致合法记忆被静默丢弃 [egosync-app/src-tauri/src/services/memory_pipeline.rs:~203] — 已改为提取首个 `{` 到末个 `}`，并补充夹带说明文字/`~~~Json` 回归测试。[edge]
- [x] [Review][Patch] 提炼超时后 spawn 的 chat_stream 任务未取消 [egosync-app/src-tauri/src/services/memory_pipeline.rs:~57] — 已持有 `JoinHandle` 并在 timeout 分支 `abort()`；provider 直接错误也会写真实 warn 日志。[blind+edge]
- [x] [Review][Patch] AC-7 缺 pipeline 角色/全局记忆路由的端到端测试 [egosync-app/src-tauri/src/services/memory_pipeline.rs] — 已补 fake provider pipeline 测试，断言管家写 `role_id=NULL`、角色会话写目标 role id。[auditor]

#### Deferred (pre-existing / known V1 boundary)

- [x] [Review][Defer] streaming 标志在消息插入失败时永久卡死会话 [egosync-app/src-tauri/src/commands/chat.rs:~242] — `streaming.insert` 被提前到 busy 检查后、assistant 消息插入前；DB 失败 `?` 提前返回时清理任务尚未 spawn，会话本进程内永久 busy。**经 git 取证：baseline 34aab28 顺序安全，此错误顺序来自工作树未提交的前序 opencode 重构，非 Story 2.6 引入。** [blind+edge HIGH]
- [x] [Review][Patch] 去重仅覆盖精确文本，LLM 重述产生近似重复记忆 [migrations/004_memories.sql:15] — 已新增 `005_memory_role_scoped_dedupe.sql`，运行态唯一索引改为角色范围 + source_message_ids；DB 写入前也按标准化来源消息集合查询重复，LLM 改写同一来源 content 不会重复写入。[edge+auditor]
- [x] [Review][Defer] 提炼无取消钩子 + 跨库悬挂记忆 TOCTOU [egosync-app/src-tauri/src/services/memory_pipeline.rs] — 5 分钟到期进入提炼后无法打断；查存在性(起点)与写入(终点)间删除会话会写入悬挂 source_conversation_id。V1 无消费方，记忆查询/遗忘留待 Story 2.7/2.8。[edge]
- [x] [Review][Defer] insert_memories DB 层不校验 source_message_ids 归属 [egosync-app/src-tauri/src/db/memories.rs:39] — 归属白名单仅在 pipeline 层；当前唯一调用链一致，属防御性提示，未来新增绕过 pipeline 的写入路径时再加固。[edge]
