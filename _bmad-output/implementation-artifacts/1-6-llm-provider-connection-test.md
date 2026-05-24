# Story 1.6: 用户能配置 LLM Provider 并测试连接成功

Status: done

## Story

As a 用户,
I want 在设置中添加 LLM Provider 并测试连接,
So that 确认配置正确后才开始使用。

## Acceptance Criteria

1. **AC-1 OpenAI 兼容连接测试**：用户输入 OpenAI 兼容配置（base_url + api_key + model），点击"测试连接"→ 显示"连接成功"或具体错误（401 未授权 / 网络超时 / 模型不存在等）。
2. **AC-2 Ollama 本地连接测试**：`base_url=http://localhost:11434/v1` 时，测试连接能成功。
3. **AC-3 Anthropic 格式连接测试**：Anthropic 格式配置（base_url + api_key + model=claude-*），使用 `AnthropicProvider` 路径成功或返回具体错误。
4. **AC-4 默认配置唯一性**：多个配置已保存时，用户标记其中一个为默认，`llm_configs` 表 `is_default` 列唯一为 true。
5. **AC-5 超时控制**：测试连接请求超时阈值 ≤ 10 秒，超时后显示明确错误。
6. **AC-6 持久化**：用户重启应用后，所有保存的配置仍然存在（持久化到 `egosync.db` 的 `llm_configs` 表），API Key 仅存储在 keyring 中，`llm_configs` 表只存引用标识 `api_key_ref`。
7. **AC-7 前端接通**：GlobalSettingsModal LLM tab 显示真实配置列表（替换原型 mock 数组），支持新增/编辑/删除/设为默认/测试连接。
8. **AC-8 LlmProvider trait 存在**：Rust 后端存在 `LlmProvider` trait（`async fn test_connection`）+ `OpenAiProvider` + `AnthropicProvider` 实现。
9. **AC-9 数据库 Migration**：`migrations/001_initial_schema.sql` 含 `llm_configs` 表 + `app_settings` 表。
10. **AC-10 现有测试通过**：`cargo test` + `npm run test:frontend` 全部通过。

## Tasks / Subtasks

### Phase 1: 数据库基础 — SQLx + Migration (AC: #6, #9)

- [x] **T1.1** 在 `Cargo.toml` 添加依赖：`sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }`、`uuid = { version = "1", features = ["v4"] }`、`reqwest = { version = "0.12", features = ["json", "stream"] }`
- [x] **T1.2** 创建 `GUI/src-tauri/migrations/001_initial_schema.sql`：
  ```sql
  -- 主数据库 egosync.db 初始 schema

  CREATE TABLE IF NOT EXISTS llm_configs (
      id TEXT PRIMARY KEY NOT NULL,
      name TEXT NOT NULL,
      provider TEXT NOT NULL CHECK(provider IN ('openai_compatible', 'anthropic')),
      base_url TEXT NOT NULL,
      model TEXT NOT NULL,
      api_key_ref TEXT NOT NULL,
      is_default INTEGER NOT NULL DEFAULT 0,
      created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
      updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
  );

  CREATE TABLE IF NOT EXISTS app_settings (
      key TEXT PRIMARY KEY NOT NULL,
      value TEXT,
      updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
  );
  ```
- [x] **T1.3** 创建 `src-tauri/src/db/mod.rs` + `src-tauri/src/db/pool.rs`（SQLx pool 初始化，连接 `egosync.db`，运行 migration）
- [x] **T1.4** 创建 `src-tauri/src/db/settings.rs`（`llm_configs` CRUD）
- [x] **T1.5** 在 `lib.rs` 中 `mod db;`，在 Tauri Builder 中初始化 DB pool 并注入 `tauri::State`
- [x] **T1.6** `cargo check` 通过

### Phase 2: 数据模型 (AC: #6, #8)

- [x] **T2.1** 创建 `src-tauri/src/models/mod.rs`
- [x] **T2.2** 创建 `src-tauri/src/models/settings.rs`：
  ```rust
  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
  #[serde(rename_all = "camelCase")]
  pub struct LlmConfig {
      pub id: String,
      pub name: String,
      pub provider: String,       // "openai_compatible" | "anthropic"
      pub base_url: String,
      pub model: String,
      pub api_key_ref: String,    // keyring 中的 key 名称
      pub is_default: bool,
      pub created_at: String,
      pub updated_at: String,
  }
  ```
- [x] **T2.3** 定义 `CreateLlmConfigInput` / `UpdateLlmConfigInput` DTO

### Phase 3: LLM Provider Trait + 实现 (AC: #1, #2, #3, #5, #8)

- [x] **T3.1** 创建 `src-tauri/src/llm/mod.rs`
- [x] **T3.2** 创建 `src-tauri/src/llm/traits.rs`：
  ```rust
  #[async_trait::async_trait]
  pub trait LlmProvider: Send + Sync {
      async fn test_connection(&self) -> Result<(), AppError>;
      // chat_stream 预留，Story 1.7 实现
  }
  ```
- [x] **T3.3** 创建 `src-tauri/src/llm/openai.rs`（`OpenAiProvider`）：
  - 构造：`base_url` + `api_key` + `model`
  - `test_connection()`：向 `{base_url}/chat/completions` 发送最小请求（`messages: [{"role":"user","content":"hi"}]`, `max_tokens: 1`）
  - 超时 10 秒（`reqwest::Client` 配置 `timeout(Duration::from_secs(10))`）
  - 解析 HTTP 状态码：401 → "API Key 无效或已过期"、404 → "模型不存在"、timeout → "连接超时，请检查网络"
- [x] **T3.4** 创建 `src-tauri/src/llm/anthropic.rs`（`AnthropicProvider`）：
  - `test_connection()`：向 `{base_url}/v1/messages` 发送最小请求（`model`, `max_tokens: 1`, `messages: [{"role":"user","content":"hi"}]`）
  - Header: `x-api-key` + `anthropic-version: 2023-06-01`
  - 超时 10 秒
- [x] **T3.5** 在 `lib.rs` 中 `mod llm;`

### Phase 4: LLM 配置 Service 层 (AC: #4, #6)

- [x] **T4.1** 创建 `src-tauri/src/services/llm_config.rs`：
  - `create_config(input, pool, app_handle)` — 生成 UUID，API Key 存入 keyring（`api_key_ref = "llm_{id}_api_key"`），config 存入 DB
  - `update_config(id, input, pool, app_handle)` — 更新 DB + keyring
  - `delete_config(id, pool)` — 删除 DB 记录 + 删除 keyring entry
  - `list_configs(pool)` — 返回所有配置（**不含** API Key 明文，前端不需要真实 key）
  - `set_default(id, pool)` — 事务中将所有 `is_default=0` 然后目标 `is_default=1`
  - `test_connection(id, pool)` — 从 DB 加载 config，从 keyring 加载 API Key，构建对应 Provider，调用 `test_connection()`
- [x] **T4.2** 在 `services/mod.rs` 中 `pub mod llm_config;`

### Phase 5: Tauri Command 层 (AC: #7)

- [x] **T5.1** 创建 `src-tauri/src/commands/llm_config.rs`：
  ```rust
  #[tauri::command]
  pub async fn llm_config_list(pool: State<'_, DbPool>) -> Result<Vec<LlmConfig>, AppError>

  #[tauri::command]
  pub async fn llm_config_create(input: CreateLlmConfigInput, pool: State<'_, DbPool>) -> Result<LlmConfig, AppError>

  #[tauri::command]
  pub async fn llm_config_update(id: String, input: UpdateLlmConfigInput, pool: State<'_, DbPool>) -> Result<LlmConfig, AppError>

  #[tauri::command]
  pub async fn llm_config_delete(id: String, pool: State<'_, DbPool>) -> Result<(), AppError>

  #[tauri::command]
  pub async fn llm_config_set_default(id: String, pool: State<'_, DbPool>) -> Result<(), AppError>

  #[tauri::command]
  pub async fn llm_config_test_connection(id: String, pool: State<'_, DbPool>) -> Result<(), AppError>
  ```
- [x] **T5.2** 在 `commands/mod.rs` 中 `pub mod llm_config;`
- [x] **T5.3** 在 `lib.rs` `invoke_handler` 中注册所有新 command
- [x] **T5.4** `cargo check` 通过

### Phase 6: 前端接通 (AC: #7)

- [x] **T6.1** 创建 `GUI/src/types/settings.ts`：
  ```typescript
  export interface LlmConfig {
    id: string;
    name: string;
    provider: 'openai_compatible' | 'anthropic';
    baseUrl: string;
    model: string;
    apiKeyRef: string;
    isDefault: boolean;
    createdAt: string;
    updatedAt: string;
  }

  export interface CreateLlmConfigInput {
    name: string;
    provider: 'openai_compatible' | 'anthropic';
    baseUrl: string;
    model: string;
    apiKey: string;  // 明文传给后端，后端写入 keyring
  }

  export interface UpdateLlmConfigInput {
    name?: string;
    provider?: 'openai_compatible' | 'anthropic';
    baseUrl?: string;
    model?: string;
    apiKey?: string;  // 有值则更新 keyring
  }
  ```
- [x] **T6.2** 创建 `GUI/src/services/llmConfigService.ts`：
  ```typescript
  import { invoke } from '@tauri-apps/api/core';
  import type { LlmConfig, CreateLlmConfigInput, UpdateLlmConfigInput } from '../types/settings';

  export const llmConfigService = {
    list: () => invoke<LlmConfig[]>('llm_config_list'),
    create: (input: CreateLlmConfigInput) => invoke<LlmConfig>('llm_config_create', { input }),
    update: (id: string, input: UpdateLlmConfigInput) => invoke<LlmConfig>('llm_config_update', { id, input }),
    delete: (id: string) => invoke<void>('llm_config_delete', { id }),
    setDefault: (id: string) => invoke<void>('llm_config_set_default', { id }),
    testConnection: (id: string) => invoke<void>('llm_config_test_connection', { id }),
  };
  ```
- [x] **T6.3** 修改 `GlobalSettingsModal.tsx`：移除 mock `configs` state，替换为 `useEffect` 加载真实数据 + 各操作调用 service
- [x] **T6.4** 添加“测试连接”按钮到编辑表单和配置列表
- [x] **T6.5** 添加连接测试状态 UI：loading spinner / 成功绿 ✓ / 失败红 ✗ + 错误消息

### Phase 7: 测试 (AC: #10)

- [x] **T7.1** 在 `db/settings.rs` 底部添加 `#[cfg(test)] mod tests`：CRUD 单元测试（使用内存 SQLite）
- [x] **T7.2** 在 `llm/openai.rs` 底部添加 `#[cfg(test)] mod tests`：验证 URL 构造正确性
- [x] **T7.3** 在 `llm/anthropic.rs` 底部添加 `#[cfg(test)] mod tests`
- [x] **T7.4** 前端现有测试 (App.test.tsx) 通过，组件编译无错
- [x] **T7.5** 运行 `cargo test` (25 passed) + `npm run test:frontend` (1 passed)：全部通过

### Review Findings

- [x] [Review][Patch] 全局连接测试状态 UI 混淆 (GlobalSettingsModal.tsx) — 已修复：引入 lastTestedConfigId 状态限定渲染范围
- [x] [Review][Patch] 缺少 Anthropic 默认 Base URL 占位提示 (GlobalSettingsModal.tsx) — 已修复：根据 provider 动态切换 placeholder

## Dev Notes

### ⚠️ 致命陷阱清单

#### 陷阱 1：SQLx 运行时 + 编译时检查

**SQLx 0.8** 默认启用编译时 SQL 校验（`sqlx::query!` 宏），但这需要一个已存在的数据库。
- 本 Story 使用 **运行时查询** `sqlx::query_as::<_, LlmConfig>(...)` 而非 `sqlx::query!` 宏，避免编译时 DB 依赖。
- 后续 Story 如需编译时校验，再配置 `DATABASE_URL` 环境变量 + `sqlx prepare`。

#### 陷阱 2：SQLx 0.8 SQLite 特性名称

SQLx 0.8 的 SQLite 特性是 `"sqlite"`，运行时特性是 `"runtime-tokio"`。
- ✅ 正确：`sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }`
- ❌ 错误：`features = ["runtime-tokio-native-tls", "sqlite"]`（0.7 的写法）

#### 陷阱 3：Tauri 2.x async Command + State

Tauri 2.x 中异步 Command 获取 `State` 的签名：
```rust
#[tauri::command]
pub async fn llm_config_list(pool: tauri::State<'_, DbPool>) -> Result<Vec<LlmConfig>, AppError> {
    // pool 类型为 tauri::State<'_, SqlitePool>
    // 使用 pool.inner() 或直接 &*pool 获取内部引用
}
```
- `State` 需要在 Builder 中通过 `.manage(pool)` 注册
- async Command 中 `State` 参数的生命周期必须是 `'_`

#### 陷阱 4：reqwest 在 Tauri 2.x 环境

Tauri 2.x 默认限制 HTTP 出站。需要在 `tauri.conf.json` 或 capabilities 中放行，或者直接在 Rust 后端使用 reqwest（后端不受 CSP 限制）。
- ✅ 本 Story 方案：Rust 后端直接使用 `reqwest`，无需前端发请求，不受 CSP 影响。

#### 陷阱 5：API Key 在 llm_configs 表中 **不存储明文**

`llm_configs.api_key_ref` 存储的是 keyring 中的 key 名称（如 `"llm_xxxx-uuid_api_key"`），**不是** API Key 本身。
- 前端 `CreateLlmConfigInput` 传入 `apiKey` 明文 → 后端 service 层写入 keyring → DB 只存 ref。
- 前端 `list` 返回的数据中 `apiKeyRef` 是引用名，前端显示 `"••••••••"` 而非真实 key。
- 编辑时如果 `apiKey` 字段为空字符串或 null，表示不更新 keyring。

#### 陷阱 6：OpenAI 兼容 API 的 test_connection 最小请求

```json
POST {base_url}/chat/completions
Content-Type: application/json
Authorization: Bearer {api_key}

{
  "model": "{model}",
  "messages": [{"role": "user", "content": "hi"}],
  "max_tokens": 1
}
```
- Ollama 本地模型的 base_url 是 `http://localhost:11434/v1`，使用相同的 `/chat/completions` 端点。
- **不要**用 `/models` 端点测试（某些兼容服务不实现它）。

#### 陷阱 7：Anthropic API 格式差异

```json
POST {base_url}/v1/messages
Content-Type: application/json
x-api-key: {api_key}
anthropic-version: 2023-06-01

{
  "model": "{model}",
  "max_tokens": 1,
  "messages": [{"role": "user", "content": "hi"}]
}
```
- Anthropic **不用** Bearer token，用 `x-api-key` header。
- 必须带 `anthropic-version` header。
- 端点路径：`/v1/messages`（如果 `base_url` 已含 `/v1` 则为 `{base_url}/messages`）。
- 建议：如果 `base_url` 末尾已包含 `/v1`，则拼接时不重复；否则加上 `/v1/messages`。

#### 陷阱 8：is_default 唯一约束

SQLite 无法用 UNIQUE partial index 确保只有一行 `is_default=1`。
- 在 `set_default` service 方法中用**事务**处理：`BEGIN → UPDATE all SET is_default=0 → UPDATE target SET is_default=1 → COMMIT`。
- 创建新配置时，如果是第一个配置则自动 `is_default=1`。

#### 陷阱 9：前端 @tauri-apps/api 导入路径

Tauri 2.x 的 `invoke` 导入路径：
```typescript
import { invoke } from '@tauri-apps/api/core';
```
- ❌ 错误：`from '@tauri-apps/api/tauri'`（Tauri 1.x 写法）
- ❌ 错误：`from '@tauri-apps/api'`（不直接导出 invoke）

#### 陷阱 10：DB 初始化时机 — Tauri setup hook

SQLite 连接池需要在 Tauri app 启动时创建并 `.manage()` 注册。使用 `.setup()` hook：
```rust
tauri::Builder::default()
    .setup(|app| {
        let db_path = app.path().app_data_dir()?.join("egosync.db");
        // 初始化 pool，运行 migration
        let pool = tauri::async_runtime::block_on(async {
            init_db(&db_path).await
        })?;
        app.manage(pool);
        Ok(())
    })
```
- `app.path().app_data_dir()` 获取平台标准数据目录。
- 必须确保目录存在（`std::fs::create_dir_all`）。

### 前一 Story (1.5) 关键经验

- **keyring 3.x** 需要 `Entry::new_with_target()` 确保跨调用一致性（target 格式: `{SERVICE_NAME}:{key}`）。
- **AppError** 使用手动 `impl Serialize`（tagged JSON），因为 Tauri 2.x 有 blanket `From<Serialize> for InvokeError`。
- **现有 Cargo.toml** 已含：tauri 2、serde、serde_json、tokio (full)、tracing、tracing-subscriber、async-trait、keyring 3（三平台特性）、thiserror 1。
- **lib.rs 结构**：`mod commands; mod error; mod services;` + Builder 链式调用 `.invoke_handler(...)` + `.run()`。

### Project Structure Notes

**本 Story 新建的文件：**

| 文件 | 内容 |
|---|---|
| `GUI/src-tauri/migrations/001_initial_schema.sql` | llm_configs + app_settings 表 |
| `GUI/src-tauri/src/db/mod.rs` | db 模块声明 |
| `GUI/src-tauri/src/db/pool.rs` | SQLite 连接池初始化 + migration |
| `GUI/src-tauri/src/db/settings.rs` | llm_configs CRUD 操作 |
| `GUI/src-tauri/src/models/mod.rs` | models 模块声明 |
| `GUI/src-tauri/src/models/settings.rs` | LlmConfig 数据模型 |
| `GUI/src-tauri/src/llm/mod.rs` | llm 模块声明 |
| `GUI/src-tauri/src/llm/traits.rs` | LlmProvider trait 定义 |
| `GUI/src-tauri/src/llm/openai.rs` | OpenAiProvider 实现 |
| `GUI/src-tauri/src/llm/anthropic.rs` | AnthropicProvider 实现 |
| `GUI/src-tauri/src/services/llm_config.rs` | LLM 配置 service 层 |
| `GUI/src-tauri/src/commands/llm_config.rs` | Tauri command 薄层 |
| `GUI/src/types/settings.ts` | LlmConfig TS 类型 |
| `GUI/src/services/llmConfigService.ts` | 前端 service 封装 invoke |

**本 Story 修改的文件：**

| 文件 | 修改内容 |
|---|---|
| `GUI/src-tauri/Cargo.toml` | 添加 sqlx + uuid + reqwest 依赖 |
| `GUI/src-tauri/src/lib.rs` | 添加 `mod db; mod models; mod llm;` + `.setup()` 初始化 DB + 注册新 commands |
| `GUI/src-tauri/src/services/mod.rs` | 添加 `pub mod llm_config;` |
| `GUI/src-tauri/src/commands/mod.rs` | 添加 `pub mod llm_config;` |
| `GUI/src/components/settings/GlobalSettingsModal.tsx` | 移除 mock state → 使用 service 真实数据 + 测试连接 UI |
| `GUI/package.json` | 添加 `@tauri-apps/api` 依赖（如尚未安装） |

**不修改的文件（确认无需改动）：**
- `GUI/src-tauri/src/error.rs` — `AppError` 已含所有需要的变体
- `GUI/src-tauri/src/services/secret_store.rs` — 被 llm_config service 调用，本身不改
- `GUI/src-tauri/src/commands/secret.rs` — 不变
- `GUI/src-tauri/tauri.conf.json` — 不需要修改（Rust 后端 HTTP 不受 CSP 限制）

**与架构文档对齐：**

| 架构规范要求 | 本 Story 实现 |
|---|---|
| `LlmProvider` trait + 策略模式 | ✅ traits.rs + openai.rs + anthropic.rs |
| `migrations/001_initial_schema.sql` | ✅ llm_configs + app_settings |
| commands/ 薄层 + services/ 业务逻辑 | ✅ 分层正确 |
| 前端 service 层封装 invoke | ✅ llmConfigService.ts |
| 数据模型 `#[serde(rename_all = "camelCase")]` | ✅ LlmConfig |
| API Key 不落明文，keyring 存储 | ✅ api_key_ref 引用模式 |
| FR-28 LLM 模型配置 | ✅ 完整覆盖 |

### 类型定义参考（DB 类型 ↔ Rust ↔ TS 映射）

| DB 列 | Rust 类型 | TS 类型 |
|--------|-----------|---------|
| `id TEXT` | `String` | `string` |
| `provider TEXT` | `String` | `'openai_compatible' \| 'anthropic'` |
| `is_default INTEGER` | `bool`（sqlx 自动转换 0/1） | `boolean` |
| `created_at TEXT` | `String` (ISO 8601) | `string` |

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` #Story 1.6] — 完整 AC 定义
- [Source: `_bmad-output/planning-artifacts/architecture.md` #API & Communication Patterns] — LlmProvider trait、IPC Command 模式
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Data Architecture] — llm_configs 表设计、app_settings 表
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Structure Patterns] — Rust 后端目录结构、前端 services/ 目录
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Authentication & Security] — keyring 3.x、api_key_ref 模式
- [Source: `_bmad-output/project-context.md` #关键实现规则] — serde camelCase、命名约定、分层规则
- [Source: `GUI/src-tauri/Cargo.toml`] — 当前依赖清单（无 sqlx/reqwest/uuid）
- [Source: `GUI/src-tauri/src/lib.rs`] — 当前结构：3 mod + Builder + invoke_handler
- [Source: `GUI/src-tauri/src/services/secret_store.rs`] — keyring Entry 用法：`Entry::new_with_target()`
- [Source: `GUI/src-tauri/src/error.rs`] — AppError 手动 Serialize 模式
- [Source: `GUI/src-tauri/tauri.conf.json`] — identifier: "com.egosync.app"
- [Source: `GUI/src/components/settings/GlobalSettingsModal.tsx`] — 当前 mock state 代码（待替换）

### 验证命令清单

```bash
cd GUI/src-tauri
cargo check                    # 编译检查
cargo test                     # 单元测试
cargo clippy -- -D warnings    # lint

cd GUI
npm run test:frontend          # 前端测试无回归
npm run tauri dev              # 集成验证：打开设置 → LLM tab → CRUD + 测试连接
```

## Dev Agent Record

### Implementation Plan

- Phase 1-2: SQLx 0.8 + SQLite 连接池 + migration + LlmConfig 数据模型
- Phase 3: LlmProvider trait 策略模式 + OpenAiProvider + AnthropicProvider（10s 超时）
- Phase 4: Service 层编排 DB + keyring + Provider 实例化
- Phase 5: Tauri Command 薄层代理 Service
- Phase 6: 前端 GlobalSettingsModal 移除 mock，使用真实 Tauri invoke

### Debug Log

- `cargo check` 首次失败：缺少 `use tauri::Manager;` → 已修复
- `cargo clippy` 发现 3 个警告：
  - `needless_borrows_for_generic_args` 在 `set_default_llm_config` → 提取 `now` 变量
  - `floor_char_boundary` 在 openai.rs/anthropic.rs → MSRV 1.77.2 不支持 (1.91+ API) → 用 `is_char_boundary` 手动回退

### Completion Notes

- ✅ 全量测试通过：25 Rust tests + 1 frontend test
- ✅ `cargo clippy -- -D warnings` 零警告
- ✅ `npx tsc --noEmit` 零 TS 错误
- ✅ 所有 AC 覆盖：DB CRUD、LlmProvider trait、OpenAI/Anthropic 连接测试、10s 超时、is_default 事务唯一性、前端接通
- ⚠️ 集成测试需要 `npm run tauri dev` 手动验证（需要真实 LLM API Key）

## File List

### 新建文件
- `GUI/src-tauri/migrations/001_initial_schema.sql`
- `GUI/src-tauri/src/db/mod.rs`
- `GUI/src-tauri/src/db/pool.rs`
- `GUI/src-tauri/src/db/settings.rs`
- `GUI/src-tauri/src/models/mod.rs`
- `GUI/src-tauri/src/models/settings.rs`
- `GUI/src-tauri/src/llm/mod.rs`
- `GUI/src-tauri/src/llm/traits.rs`
- `GUI/src-tauri/src/llm/openai.rs`
- `GUI/src-tauri/src/llm/anthropic.rs`
- `GUI/src-tauri/src/services/llm_config.rs`
- `GUI/src-tauri/src/commands/llm_config.rs`
- `GUI/src/types/settings.ts`
- `GUI/src/services/llmConfigService.ts`

### 修改文件
- `GUI/src-tauri/Cargo.toml` — 添加 sqlx/uuid/reqwest 依赖
- `GUI/src-tauri/src/lib.rs` — 添加 mod db/models/llm + setup hook + 注册 commands
- `GUI/src-tauri/src/services/mod.rs` — 添加 pub mod llm_config
- `GUI/src-tauri/src/commands/mod.rs` — 添加 pub mod llm_config
- `GUI/src/components/settings/GlobalSettingsModal.tsx` — 真实数据接通 + 测试连接 UI

## Change Log

| 日期 | 变更 |
|------|------|
| 2026-05-21 | Story 1.6 实现完成：SQLx DB 层 + LlmProvider trait + OpenAI/Anthropic Provider + Service + Tauri Commands + 前端接通 |
