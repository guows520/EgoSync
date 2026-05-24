# Story 1.7: 用户能与管家完成首次流式对话

Status: ready-for-dev

## Story

As a 用户,
I want 输入消息后看到管家流式逐字回复,
So that 感受到 AI 正在实时思考并回应我。

## Acceptance Criteria

1. **AC-1 流式逐字显示**：LLM 已配置且连接正常，用户输入"你好"按 Enter → 流式逐字显示管家回复，首字节延迟 < 500ms。
2. **AC-2 对话持久化**：流式响应进行中，用户关闭应用并重开 → 对话历史已持久化保留（最后一条 message 的 `is_complete` 标记正确）。
3. **AC-3 错误友好显示**：LLM API 报错（网络断开、余额不足）→ 在对话气泡中以管家自然语言展示（如"我现在连不上模型，能检查一下配置吗？"），**不**使用 toast / 红框 / snackbar。
4. **AC-4 并发控制**：流式进行中用户再次发送 → 新请求等待前一个完成，或显示管家提示"我还在想上一个问题..."。
5. **AC-5 conversations.db Migration**：`migrations/002_conversations.sql` 创建 `conversations` + `messages` 表（对话日志库 `conversations.db`）。
6. **AC-6 Agent Engine**：`services/agent_engine.rs` 模块实现 System Prompt 分层（基础人格 + 当前上下文）。
7. **AC-7 Tauri Event 流式**：Tauri Event `llm:stream` payload: `{ conversationId, token, done }`。
8. **AC-8 ChatStream 组件**：新建 `components/chat/ChatStream.tsx` 替换 ButlerView 内 mock 消息列表。
9. **AC-9 useTauriEvent hook**：`useTauriEvent('llm:stream')` hook 处理流式更新。
10. **AC-10 现有测试通过**：`cargo test` + `npm run test:frontend` 全部通过。
11. **AC-11 会话历史管理**：用户可新建对话、查看历史对话列表（显示标题+相对时间）、切换对话、删除对话。
12. **AC-12 LLM 自动标题生成**：首条用户消息发送后，后端异步调用 LLM 生成对话标题（≤8字），通过 Tauri Event `llm:title-updated` 通知前端更新。
13. **AC-13 ChatOptions**：`LlmProvider::chat_stream` 接受 `ChatOptions`（如 `disable_thinking`）以支持不同场景的参数控制。
14. **AC-14 中断流式回复**：流式进行中，发送按钮变为停止按钮（同色系），点击停止按钮 → 后端通过 `CancellationToken` 中断流式 → 已生成内容保留并持久化。

## Tasks / Subtasks

### Phase 1: 对话日志数据库 — 第二个 SQLite DB (AC: #2, #5)

- [ ] **T1.1** 创建 `GUI/src-tauri/migrations/002_conversations.sql`：
  ```sql
  -- 对话日志库 conversations.db schema
  -- 注意：此 migration 运行在 conversations.db（非 egosync.db）
  
  CREATE TABLE IF NOT EXISTS conversations (
      id TEXT PRIMARY KEY NOT NULL,
      role_id TEXT,  -- NULL = 管家对话
      started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
      updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
  );

  CREATE TABLE IF NOT EXISTS messages (
      id TEXT PRIMARY KEY NOT NULL,
      conversation_id TEXT NOT NULL,
      role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
      content TEXT NOT NULL,
      is_complete INTEGER NOT NULL DEFAULT 1,
      created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
      FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
  );

  CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
  CREATE INDEX IF NOT EXISTS idx_conversations_role_id ON conversations(role_id);
  ```
- [ ] **T1.2** 修改 `src-tauri/src/db/pool.rs`：新增 `init_conversations_db(path)` 函数，创建 conversations.db 的连接池
- [ ] **T1.3** 新增类型 `pub type ConversationsPool = SqlitePool;`，与主库 `DbPool` 区分
- [ ] **T1.4** 修改 `lib.rs` `.setup()` hook：初始化 conversations.db 池并 `.manage()` 注册（与 egosync.db 分开）
- [ ] **T1.5** 创建 `src-tauri/src/db/conversations.rs`：
  - `create_conversation(pool, role_id: Option<String>) -> Result<Conversation>`
  - `get_or_create_butler_conversation(pool) -> Result<Conversation>`（查找最近管家对话或新建）
  - `insert_message(pool, conversation_id, role, content, is_complete) -> Result<Message>`
  - `update_message_content(pool, id, content) -> Result<()>`（流式累积更新）
  - `mark_message_complete(pool, id) -> Result<()>`
  - `list_messages(pool, conversation_id) -> Result<Vec<Message>>`
  - `get_recent_messages(pool, conversation_id, limit: i64) -> Result<Vec<Message>>`
- [ ] **T1.6** 在 `db/mod.rs` 添加 `pub mod conversations;`
- [ ] **T1.7** `cargo check` 通过

### Phase 2: 数据模型 (AC: #2, #7)

- [ ] **T2.1** 创建 `src-tauri/src/models/chat.rs`：
  ```rust
  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
  #[serde(rename_all = "camelCase")]
  pub struct Conversation {
      pub id: String,
      pub role_id: Option<String>,
      pub title: String,
      pub started_at: String,
      pub updated_at: String,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
  #[serde(rename_all = "camelCase")]
  pub struct Message {
      pub id: String,
      pub conversation_id: String,
      pub role: String,         // "user" | "assistant" | "system"
      pub content: String,
      pub is_complete: bool,
      pub created_at: String,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct StreamPayload {
      pub conversation_id: String,
      pub token: String,
      pub done: bool,
      pub thinking: bool,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct TitleUpdatedPayload {
      pub conversation_id: String,
      pub title: String,
  }

  #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ChatOptions {
      pub disable_thinking: Option<bool>,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ChatRequest {
      pub conversation_id: Option<String>,  // None = 新对话或复用管家对话
      pub role_id: Option<String>,          // None = 管家
      pub content: String,
  }
  ```
- [ ] **T2.2** 在 `models/mod.rs` 添加 `pub mod chat;`

### Phase 3: LlmProvider trait 扩展 — chat_stream (AC: #1, #7)

- [ ] **T3.1** 修改 `src-tauri/src/llm/traits.rs`：
  ```rust
  use crate::error::AppError;
  use tokio::sync::mpsc;

  #[derive(Debug, Clone)]
  pub enum StreamEvent {
      Token(String),
      Thinking(String),
      Done,
      Error(String),
  }

  #[async_trait::async_trait]
  pub trait LlmProvider: Send + Sync {
      async fn test_connection(&self) -> Result<(), AppError>;
      async fn chat_stream(
          &self,
          messages: Vec<ChatCompletionMessage>,
          tx: mpsc::Sender<StreamEvent>,
          options: ChatOptions,
      ) -> Result<(), AppError>;
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct ChatCompletionMessage {
      pub role: String,    // "system" | "user" | "assistant"
      pub content: String,
  }
  ```
- [ ] **T3.2** 修改 `src-tauri/src/llm/openai.rs`：实现 `chat_stream`
  - 请求体添加 `"stream": true`
  - 使用 `reqwest` 的 `.bytes_stream()` 读取 SSE
  - 解析 `data: {...}` 行，提取 `choices[0].delta.content`
  - 每个 token 通过 `tx.send(StreamEvent::Token(token))` 推送
  - 收到 `[DONE]` 时发送 `StreamEvent::Done`
  - 超时使用独立的长超时 client（流式不能 10 秒就断）
- [ ] **T3.3** 修改 `src-tauri/src/llm/anthropic.rs`：实现 `chat_stream`
  - 请求体添加 `"stream": true`
  - SSE 事件类型：`content_block_delta` → `delta.text`
  - `message_stop` 事件时发送 `StreamEvent::Done`
- [ ] **T3.4** `cargo check` 通过

### Phase 4: Agent Engine — System Prompt 分层 (AC: #6)

- [ ] **T4.1** 创建 `src-tauri/src/services/agent_engine.rs`：
  ```rust
  pub struct AgentEngine {
      conversations_pool: ConversationsPool,
      main_pool: DbPool,
  }

  impl AgentEngine {
      pub fn new(conversations_pool: ConversationsPool, main_pool: DbPool) -> Self { ... }
      
      /// 组装管家对话的完整 messages 列表
      pub async fn build_butler_messages(
          &self,
          conversation_id: &str,
          user_message: &str,
      ) -> Result<Vec<ChatCompletionMessage>, AppError> {
          // 1. 构建 system prompt（基础人格 + 管家身份）
          // 2. 加载对话历史（最近 N 条）
          // 3. 追加用户新消息
          vec![system_prompt, ...history, user_msg]
      }

      /// 执行流式对话：组装上下文 → 调 LLM → 通过 Tauri Event 推送
      pub async fn chat_stream(
          &self,
          app_handle: tauri::AppHandle,
          conversation_id: &str,
          role_id: Option<&str>,
          user_message: &str,
      ) -> Result<(), AppError> { ... }
  }
  ```
- [ ] **T4.2** System Prompt 管家基础人格定义（硬编码常量，V1 不支持自定义）：
  ```
  你是 EgoSync 的数字管家，用户的私人助理和生活协调者。
  你的语调稳重、可靠、有温度，像一位值得信赖的英式管家。
  你帮助用户管理角色、任务和日程，但决定权永远在用户手中。
  用简洁自然的中文回复，不用 emoji，不用 markdown 格式化。
  ```
- [ ] **T4.3** 在 `services/mod.rs` 添加 `pub mod agent_engine;`

### Phase 5: Chat Command 层 (AC: #1, #4, #7)

- [ ] **T5.1** 创建 `src-tauri/src/commands/chat.rs`：
  ```rust
  #[tauri::command]
  pub async fn chat_send_message(
      request: ChatRequest,
      main_pool: State<'_, DbPool>,
      conv_pool: State<'_, ConversationsPool>,
      app_handle: tauri::AppHandle,
  ) -> Result<Message, AppError>
  // 流程:
  // 1. 获取或创建 conversation
  // 2. 检查是否有进行中的流式（并发控制）
  // 3. 持久化用户消息
  // 4. 创建 assistant 消息占位（is_complete=false）
  // 5. tokio::spawn 启动流式：agent_engine.chat_stream(...)
  // 6. 返回用户消息确认

  #[tauri::command]
  pub async fn chat_get_history(
      conversation_id: String,
      conv_pool: State<'_, ConversationsPool>,
  ) -> Result<Vec<Message>, AppError>

  #[tauri::command]
  pub async fn chat_get_butler_conversation(
      conv_pool: State<'_, ConversationsPool>,
  ) -> Result<Conversation, AppError>

  #[tauri::command]
  pub async fn chat_list_conversations(
      conv_pool: State<'_, ConversationsPool>,
  ) -> Result<Vec<Conversation>, AppError>

  #[tauri::command]
  pub async fn chat_new_conversation(
      old_conversation_id: Option<String>,
      conv_pool: State<'_, ConversationsPool>,
  ) -> Result<Conversation, AppError>

  #[tauri::command]
  pub async fn chat_delete_conversation(
      conversation_id: String,
      conv_pool: State<'_, ConversationsPool>,
  ) -> Result<(), AppError>

  #[tauri::command]
  pub async fn chat_stop_streaming(
      conversation_id: String,
      app_handle: tauri::AppHandle,
  ) -> Result<(), AppError>
  ```
- [ ] **T5.2** 并发控制：使用 `Arc<Mutex<HashSet<String>>>` managed state 记录正在流式的 conversation_id
- [ ] **T5.2b** 取消控制：使用 `Arc<Mutex<HashMap<String, CancellationToken>>>` managed state 存储每个会话的取消令牌
- [ ] **T5.3** 在 `commands/mod.rs` 添加 `pub mod chat;`
- [ ] **T5.4** 在 `lib.rs` `invoke_handler` 注册新 commands
- [ ] **T5.5** `cargo check` 通过

### Phase 6: 前端 — Types + Service + Hook (AC: #8, #9)

- [ ] **T6.1** 创建 `GUI/src/types/chat.ts`：
  ```typescript
  export interface Conversation {
    id: string;
    roleId: string | null;
    startedAt: string;
    updatedAt: string;
  }

  export interface ChatMessage {
    id: string;
    conversationId: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    isComplete: boolean;
    createdAt: string;
  }

  export interface ChatRequest {
    conversationId?: string;
    roleId?: string;
    content: string;
  }

  export interface StreamPayload {
    conversationId: string;
    token: string;
    done: boolean;
  }
  ```
- [ ] **T6.2** 创建 `GUI/src/services/chatService.ts`：
  ```typescript
  import { invoke } from '@tauri-apps/api/core';
  import type { Conversation, ChatMessage, ChatRequest } from '../types/chat';

  export const chatService = {
    sendMessage: (request: ChatRequest) => invoke<ChatMessage>('chat_send_message', { request }),
    getHistory: (conversationId: string) => invoke<ChatMessage[]>('chat_get_history', { conversationId }),
    getButlerConversation: () => invoke<Conversation>('chat_get_butler_conversation'),
    listConversations: () => invoke<Conversation[]>('chat_list_conversations'),
    newConversation: (oldConversationId?: string) =>
      invoke<Conversation>('chat_new_conversation', { oldConversationId: oldConversationId ?? null }),
    stopStreaming: (conversationId: string) => invoke<void>('chat_stop_streaming', { conversationId }),
    deleteConversation: (conversationId: string) => invoke<void>('chat_delete_conversation', { conversationId }),
  };
  ```
- [ ] **T6.3** 创建 `GUI/src/hooks/useTauriEvent.ts`：
  ```typescript
  import { useEffect } from 'react';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  export function useTauriEvent<T>(
    eventName: string,
    handler: (payload: T) => void,
    deps: unknown[] = []
  ) {
    useEffect(() => {
      let unlisten: UnlistenFn | null = null;
      listen<T>(eventName, (event) => {
        handler(event.payload);
      }).then((fn) => { unlisten = fn; });

      return () => { unlisten?.(); };
    }, deps);  // eslint-disable-line react-hooks/exhaustive-deps
  }
  ```

### Phase 7: 前端 — ChatStream 组件替换 mock (AC: #1, #3, #4, #8)

- [ ] **T7.1** 创建 `GUI/src/components/chat/ChatStream.tsx`：
  - 管理 `messages: ChatMessage[]` 状态
  - 管理 `isStreaming` + `streamContent` 累积状态
  - 通过 `useTauriEvent<StreamPayload>('llm:stream', ...)` 监听流式 token
  - 收到 token → `setStreamContent(prev => prev + token)`
  - 收到 done → 将 streamContent 追加到 messages，清空 streamContent
  - 渲染消息列表（用户/管家区分样式）+ 流式中的"正在输入"气泡
- [ ] **T7.2** 创建 `GUI/src/components/chat/ChatBubble.tsx`：
  - 接收 `message: ChatMessage` + `isStreaming?: boolean`
  - 用户消息：右对齐，深色背景
  - 管家消息：左对齐，白色背景，含管家图标
  - 流式光标：`animate-pulse` 的闪烁竖线
- [ ] **T7.3** 创建 `GUI/src/components/chat/ChatInput.tsx`：
  - 受控 input 组件
  - Enter 发送，Shift+Enter 换行（V1 可选）
  - `disabled` prop 用于流式进行中
  - `isStreaming` + `onStop` props：流式进行中显示停止按钮（同色系，Square 图标），点击调用 `onStop`
  - 发送按钮 + 停止按钮状态切换
- [ ] **T7.3b** 创建 `GUI/src/components/chat/ChatHeader.tsx`：
  - 显示当前对话标题 + 新建对话按钮
  - 历史对话按钮点击展开 `ConversationList`
  - 接收 `onDeleteConversation` prop
- [ ] **T7.3c** 创建 `GUI/src/components/chat/ConversationList.tsx`：
  - 显示历史对话列表（标题 + 相对时间格式化：刚刚/X分钟前/X小时前/具体日期）
  - 点击切换对话
  - hover 显示删除按钮（Trash2 图标 + 确认）
- [ ] **T7.4** 修改 `GUI/src/components/butler/ButlerView.tsx`：
  - 移除 mock `messages` state 和 `handleSend`
  - 用 `<ChatStream roleId={null} />` 替换消息列表区域
  - 保留 header + workspace panel 不变
  - 保留初始管家摘要气泡（第一次无对话历史时展示）
- [ ] **T7.5** 安装前端依赖（如需）：`@tauri-apps/api` 的 `event` 模块

### Phase 8: 测试 (AC: #10)

- [ ] **T8.1** `db/conversations.rs` 底部 `#[cfg(test)] mod tests`：CRUD 单元测试（内存 SQLite）
- [ ] **T8.2** `services/agent_engine.rs` 底部 `#[cfg(test)] mod tests`：验证 system prompt 构建正确性
- [ ] **T8.3** `llm/openai.rs` 补充 SSE 解析单元测试（mock SSE 字符串 → 验证提取出正确 token）
- [ ] **T8.4** 前端：`ChatStream.test.tsx` 基础渲染测试（mock Tauri invoke/event）
- [ ] **T8.5** 运行 `cargo test` + `npm run test:frontend`：全部通过

## Dev Notes

### ⚠️ 致命陷阱清单

#### 陷阱 1：conversations.db 是**独立的第二个数据库**

架构规定对话日志库与主库分离：
- `egosync.db` → 主数据（roles, memories, tasks, llm_configs, app_settings）
- `conversations.db` → 对话日志（conversations, messages）

**实现方式：**
- `db/pool.rs` 导出两个初始化函数：`init_db()` (主库) + `init_conversations_db()` (对话库)
- `lib.rs` `.setup()` 中初始化两个池并分别 `.manage()`
- 两个 pool 类型需区分：可用 newtype `pub struct ConversationsPool(pub SqlitePool);` 或直接用不同类型别名 + Tauri State 键

**关键：** Tauri 2.x `app.manage()` 按类型区分 State。如果两个 pool 都是 `SqlitePool`，需要用 newtype 包装其中一个，否则 `State<'_, SqlitePool>` 会冲突。

```rust
// 推荐方案：newtype 包装
pub struct ConversationsPool(pub SqlitePool);
// 或者用 tauri::Manager 的命名 state（不推荐，API 不稳定）
```

#### 陷阱 2：OpenAI SSE 流式解析格式

OpenAI 流式响应格式：
```
data: {"id":"chatcmpl-...","choices":[{"delta":{"content":"你"},"index":0}]}

data: {"id":"chatcmpl-...","choices":[{"delta":{"content":"好"},"index":0}]}

data: [DONE]
```

**关键：**
- 每行以 `data: ` 开头，后跟 JSON 或 `[DONE]`
- 行之间可能有空行
- `choices[0].delta.content` 可能为 `null`（首个 chunk 通常只有 `role` 字段）
- 必须处理 `content` 为 null 的情况（跳过，不 unwrap）
- 使用 `reqwest::Response::bytes_stream()` + `tokio_stream` 逐块读取
- **不要**用 `.text()` 一次性读取（会等全部完成）

#### 陷阱 3：Anthropic SSE 流式格式（与 OpenAI 不同）

Anthropic 流式响应格式：
```
event: message_start
data: {"type":"message_start","message":{"id":"msg_...","model":"claude-3-sonnet-...",...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"你"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"好"}}

event: message_stop
data: {"type":"message_stop"}
```

**关键：**
- 有 `event:` 行标识事件类型（OpenAI 没有）
- token 在 `content_block_delta` 事件的 `delta.text` 字段
- 完成信号是 `message_stop` 事件（不是 `[DONE]` 字符串）
- `message_start` 和 `content_block_start` 需要跳过

#### 陷阱 4：流式超时 ≠ 连接测试超时

`test_connection` 用 10 秒超时是正确的。但 `chat_stream` 的超时完全不同：
- **连接超时（connect timeout）**：10 秒 — 建立连接的时间
- **首字节超时（TTFB）**：30 秒 — 等待第一个 token 的时间（LLM 思考可能较久）
- **读取超时（read timeout）**：不设置 — 流式传输期间不应超时
- **总超时（total timeout）**：不设置 — 长对话可能持续数分钟

```rust
// 流式请求的 client 配置
let stream_client = Client::builder()
    .connect_timeout(Duration::from_secs(10))
    // 不设置 .timeout() — 流式不能有总超时
    .build()?;
```

#### 陷阱 5：Tauri Event emit 需要 AppHandle

Tauri 2.x 中发送事件：
```rust
use tauri::Emitter;  // Tauri 2.x 的 trait

app_handle.emit("llm:stream", StreamPayload {
    conversation_id: conv_id.clone(),
    token: token_text,
    done: false,
})?;
```

- 使用 `tauri::Emitter` trait（不是 `tauri::Manager`）
- `AppHandle` 可以在 `tokio::spawn` 内安全使用（`Clone + Send + 'static`）
- **不要**用 `app.emit_all()`（那是 Tauri 1.x 的 API）

#### 陷阱 6：tokio::spawn 中的 AppHandle 生命周期

```rust
let app_handle = app_handle.clone();
let conv_id = conversation_id.to_string();
tokio::spawn(async move {
    // app_handle 和 conv_id moved in，安全使用
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);
    
    // 启动 LLM 流式
    let llm_task = tokio::spawn(async move {
        provider.chat_stream(messages, tx).await
    });
    
    // 消费 channel 并 emit
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Token(t) => {
                let _ = app_handle.emit("llm:stream", StreamPayload { ... });
                // 同时累积到 DB
            }
            StreamEvent::Done => {
                let _ = app_handle.emit("llm:stream", StreamPayload { done: true, .. });
                // mark_message_complete
            }
            StreamEvent::Error(e) => { /* 错误处理 */ }
        }
    }
});
```

#### 陷阱 7：并发控制 — 防止同一对话同时多个流式

使用 managed state 存储"正在流式中"的 conversation_id 集合：
```rust
pub struct StreamingState(pub Arc<Mutex<HashSet<String>>>);
```

在 `chat_send_message` command 中：
1. 检查 `streaming_state` 是否包含该 conversation_id
2. 如果包含 → 返回管家提示消息（不是 Error）
3. 如果不包含 → 添加到集合 → spawn 流式任务 → 任务完成后从集合移除

**注意：** 返回值应该是成功的 Message（内容为"我还在想上一个问题..."），不是 AppError。

#### 陷阱 8：前端 @tauri-apps/api/event 导入

Tauri 2.x 的事件监听：
```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<StreamPayload>('llm:stream', (event) => {
    // event.payload.token / event.payload.done
});

// cleanup
unlisten();
```

- ✅ 正确：`from '@tauri-apps/api/event'`
- ❌ 错误：`from '@tauri-apps/api'`（不直接导出 listen）
- ❌ 错误：`from '@tauri-apps/api/tauri'`（1.x 写法）

#### 陷阱 9：消息持久化时机

**用户消息**：发送时立即写入 DB（`is_complete=true`）
**Assistant 消息**：
1. 发送请求前创建占位消息（`content=""`，`is_complete=false`）
2. 流式过程中**不**逐 token 更新 DB（性能灾难）
3. 流式完成后一次性 `UPDATE content = 完整文本, is_complete = true`
4. 如果流式中断：消息保留 `is_complete=false`，下次加载时前端可标记为"生成中断"

#### 陷阱 10：ButlerView 保留初始摘要

`ButlerView.tsx` 当前有一个硬编码的"晨间摘要"气泡。在本 Story 中：
- **保留**该摘要气泡作为首次无对话历史时的展示
- 一旦有真实对话历史，用 `ChatStream` 组件的消息列表替代
- 不要删除 ActionCard 和 workspace panel 的代码

#### 陷阱 11：reqwest stream feature

`Cargo.toml` 已包含 `reqwest = { version = "0.12", features = ["json", "stream"] }`。
- `stream` feature 已启用，可直接使用 `.bytes_stream()`
- 需额外依赖处理异步流：添加 `futures = "0.3"` 用于 `StreamExt` trait
- 或使用 `tokio-stream` 的 `StreamExt`

**推荐**：添加 `futures = "0.3"` 到 Cargo.toml（轻量、广泛使用）。

### 前一 Story (1.6) 关键经验

- **SQLx 0.8** 使用运行时查询 `sqlx::query_as::<_, T>(sql)...`，不用 `sqlx::query!` 宏（避免编译时 DB 依赖）
- **Tauri 2.x async Command** 的 `State` 参数生命周期必须是 `'_`
- **AppError** 使用手动 `impl Serialize`（tagged JSON format）
- **lib.rs 结构**：`mod declarations` + Builder 链式调用 `.setup()` + `.invoke_handler()` + `.run()`
- **现有 Cargo.toml 依赖**：tauri 2, serde, serde_json, tokio (full), tracing, tracing-subscriber, async-trait, keyring 3, thiserror 1, sqlx 0.8 (runtime-tokio, sqlite), uuid 1, reqwest 0.12 (json, stream)
- **clippy 注意**：`is_char_boundary` 手动回退替代 `floor_char_boundary`（MSRV 1.77.2 不支持 1.91+ API）

### Project Structure Notes

**本 Story 新建文件：**

| 文件 | 内容 |
|---|---|
| `GUI/src-tauri/migrations/002_conversations.sql` | conversations + messages 表 |
| `GUI/src-tauri/src/db/conversations.rs` | 对话日志 DB CRUD |
| `GUI/src-tauri/src/models/chat.rs` | Conversation, Message, StreamPayload, ChatRequest |
| `GUI/src-tauri/src/services/agent_engine.rs` | Agent Engine + System Prompt 分层 |
| `GUI/src-tauri/src/commands/chat.rs` | chat_send_message, chat_get_history, chat_get_butler_conversation |
| `GUI/src/types/chat.ts` | TS 类型定义 |
| `GUI/src/services/chatService.ts` | 前端 service 封装 invoke |
| `GUI/src/hooks/useTauriEvent.ts` | Tauri Event 通用监听 hook |
| `GUI/src/components/chat/ChatStream.tsx` | 流式对话主组件 |
| `GUI/src/components/chat/ChatBubble.tsx` | 消息气泡组件 |
| `GUI/src/components/chat/ChatInput.tsx` | 输入框组件（含停止按钮） |
| `GUI/src/components/chat/ChatHeader.tsx` | 对话头部（标题+新建+历史列表） |
| `GUI/src/components/chat/ConversationList.tsx` | 历史对话列表组件 |

**本 Story 修改文件：**

| 文件 | 修改内容 |
|---|---|
| `GUI/src-tauri/Cargo.toml` | 添加 `futures = "0.3"`, `chrono = "0.4"`, `tokio-util = "0.7"` 依赖 |
| `GUI/src-tauri/src/db/pool.rs` | 新增 `ConversationsPool` 类型 + `init_conversations_db()` |
| `GUI/src-tauri/src/db/mod.rs` | 添加 `pub mod conversations;` |
| `GUI/src-tauri/src/models/mod.rs` | 添加 `pub mod chat;` |
| `GUI/src-tauri/src/services/mod.rs` | 添加 `pub mod agent_engine;` |
| `GUI/src-tauri/src/commands/mod.rs` | 添加 `pub mod chat;` |
| `GUI/src-tauri/src/lib.rs` | 初始化 conversations.db + 注册 chat commands + manage StreamingState + CancelTokens |
| `GUI/src-tauri/src/llm/traits.rs` | 添加 `chat_stream` 方法 + StreamEvent + ChatCompletionMessage |
| `GUI/src-tauri/src/llm/openai.rs` | 实现 `chat_stream`（SSE 解析） |
| `GUI/src-tauri/src/llm/anthropic.rs` | 实现 `chat_stream`（SSE 解析） |
| `GUI/src/components/butler/ButlerView.tsx` | 移除 mock messages，集成 ChatStream 组件 |

**不修改的文件（确认无需改动）：**
- `GUI/src-tauri/src/error.rs` — AppError 已含所有需要的变体（LlmError 覆盖流式错误）
- `GUI/src-tauri/src/services/secret_store.rs` — 被 agent_engine 间接调用加载 API Key，本身不改
- `GUI/src-tauri/src/services/llm_config.rs` — 提供获取默认 LLM 配置的查询，可能需要一个新的 pub fn（或者 agent_engine 直接调 db 层）
- `GUI/src-tauri/migrations/001_initial_schema.sql` — 不动，002 是新文件
- `GUI/src/components/butler/ButlerWorkspacePanel.tsx` — 不动
- `GUI/src/components/butler/ActionCard.tsx` — 不动

**与架构文档对齐：**

| 架构规范要求 | 本 Story 实现 |
|---|---|
| `LlmProvider` trait + `chat_stream` 方法 | ✅ traits.rs 扩展 |
| 对话日志库 `conversations.db` 分离 | ✅ 独立 pool + newtype |
| Tauri Event `llm:stream` 流式传输 | ✅ StreamPayload emit |
| `agent_engine` 模块 System Prompt 分层 | ✅ services/agent_engine.rs |
| 每次对话独立 `tokio::spawn` | ✅ commands/chat.rs spawn |
| 前端 service 层封装 invoke | ✅ chatService.ts |
| 组件域目录：`components/chat/` | ✅ ChatStream + ChatBubble + ChatInput |
| 事件命名 `llm:stream` | ✅ 一致 |
| 前端不直接调 LLM API | ✅ 所有 LLM 调用在 Rust 端 |
| 错误通过管家自然语言传达，不用 toast | ✅ AC-3 |

### 类型映射参考

| DB 列 (conversations.db) | Rust 类型 | TS 类型 |
|---|---|---|
| `conversations.id TEXT` | `String` | `string` |
| `conversations.role_id TEXT` | `Option<String>` | `string \| null` |
| `messages.role TEXT` | `String` | `'user' \| 'assistant' \| 'system'` |
| `messages.is_complete INTEGER` | `bool` | `boolean` |
| `messages.content TEXT` | `String` | `string` |

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` #Story 1.7] — 完整 AC 定义
- [Source: `_bmad-output/planning-artifacts/architecture.md` #API & Communication Patterns] — LlmProvider trait chat_stream 签名、Event payload
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Data Architecture] — conversations.db 分离、conversations + messages 表
- [Source: `_bmad-output/planning-artifacts/architecture.md` #LLM Streaming Standard Pattern] — 前端流式标准代码
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Internal Data Flow] — 对话核心回路数据流
- [Source: `_bmad-output/project-context.md` #框架特定规则] — Tauri IPC、Event 命名、分层规则
- [Source: `GUI/src-tauri/src/llm/traits.rs`] — 当前 LlmProvider trait（只有 test_connection）
- [Source: `GUI/src-tauri/src/llm/openai.rs`] — OpenAiProvider 现有结构
- [Source: `GUI/src-tauri/src/llm/anthropic.rs`] — AnthropicProvider 现有结构
- [Source: `GUI/src-tauri/src/lib.rs`] — 当前 app setup 和 invoke_handler 结构
- [Source: `GUI/src-tauri/src/db/pool.rs`] — 当前 DB 初始化模式（init_db）
- [Source: `GUI/src/components/butler/ButlerView.tsx`] — 当前 mock messages state（待替换）
- [Source: `_bmad-output/implementation-artifacts/1-6-llm-provider-connection-test.md` #Dev Notes] — 前一 Story 全部经验

### 验证命令清单

```bash
cd GUI/src-tauri
cargo check                    # 编译检查
cargo test                     # 单元测试
cargo clippy -- -D warnings    # lint

cd GUI
npm run test:frontend          # 前端测试无回归
npm run tauri dev              # 集成验证：发送消息 → 流式回复
```

## Dev Agent Record

### Implementation Plan

- Phase 1-2: conversations.db 独立池 + migration + 数据模型
- Phase 3: LlmProvider trait 扩展 chat_stream + OpenAI/Anthropic SSE 流式实现
- Phase 4: Agent Engine（System Prompt 分层 + 上下文组装 + 流式编排）
- Phase 5: Tauri Command 薄层 + 并发控制
- Phase 6: 前端 types/service/hook
- Phase 7: ChatStream/ChatBubble/ChatInput 组件 + ButlerView 集成
- Phase 8: 测试

### Debug Log References

### Completion Notes List

### File List

## Change Log

| 日期 | 变更 |
|------|------|
| 2026-05-21 | Story 1.7 上下文文件创建 — ready-for-dev |
| 2026-05-22 | 补充 AC-11~AC-14（会话管理、标题生成、ChatOptions、中断流式）；更新数据模型（title字段、thinking字段、TitleUpdatedPayload、ChatOptions）；更新 LlmProvider trait 签名（+options参数、+Thinking变体）；新增 commands（list/new/delete/stop）；新增前端组件（ChatHeader、ConversationList）；新增依赖（chrono、tokio-util） |
