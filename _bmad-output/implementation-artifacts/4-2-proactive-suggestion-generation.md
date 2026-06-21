---
baseline_commit: c83d6dd
---

# Story 4.2: 角色工作循环生成主动建议并存储

Status: in-progress

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 角色在工作循环中基于当前状态生成有价值的建议,
so that 我能收到角色的主动帮助而不只是被动响应。

## 背景与现状（务必先读）

**本 story 是纯 Rust 后端 story — 把 Story 4.1 调度器中的占位工作循环替换为真实的建议生成逻辑，并新增 `suggestions` 表持久化。不涉及前端改动**（建议的展示/确认/拒绝 UI 是 Story 4.4 的职责，本 story 只负责后台生成并写入 `pending` 状态的建议）。

### Story 4.1 已建成的基础（本 story 的接入点）

- `services/scheduler.rs` 已实现后台调度器：60 秒基础 tick，按角色 `proactivity_level` 在用户配置的 HH:MM 时间点触发，每个角色独立 `tokio::spawn`，动态读取 DB，错误只 `tracing::warn!` 不 panic。
- **核心接入点 = `services/scheduler.rs:145` 的 `run_work_loop_for_role(pool: &SqlitePool, role: &Role) -> Result<(), AppError>`**。当前是占位实现（只 `tracing::info!` + 返回 `Ok(())`）。**本 story 在此函数内填充建议生成业务逻辑**，函数签名保持兼容（调度器 spawn 调用方式不变）。
- 调度器已处理：触发频率、`passive` 跳过、每角色独立 spawn、失败隔离、`duration_ms`/`result` 日志。**这些本 story 不要重做**。

### 后台 LLM 调用的既有范本（务必复用，不要新发明）

`services/memory_pipeline.rs` 已建立"后台结构化 LLM 任务"的完整范本，本 story 的建议生成应**照此模式**，而非走 opencode session/agent_bridge（那是交互式对话路径）：

1. `agent_engine::resolve_default_provider(main_pool).await?` → 拿到 `Arc<dyn LlmProvider>`（`services/agent_engine.rs:1928`）。
2. 构造 `Vec<ChatCompletionMessage>`（system + user）作为 prompt。
3. 在 `tokio::spawn` 内调用 `provider.chat_stream(prompt, tx, ChatOptions { disable_thinking: true, tools: None, tool_choice: None })`，通过 `mpsc::channel::<StreamEvent>` 累积 token（参考 `memory_pipeline.rs:419-467` 的 `reconcile_memories`）。
4. 用 `timeout(Duration::from_secs(...), ...)` 包裹，超时则 `abort()` 并降级返回空结果。
5. 累积响应大小上限保护（参考 `MAX_EXTRACTION_RESPONSE_BYTES`）。
6. `strip_json_code_fence(response.trim())` 去除 markdown code fence，再 `serde_json::from_str` 解析为结构化建议（参考 `parse_role_memory_assignment_response`，`memory_pipeline.rs:830`）。
7. system prompt 强制"只输出严格 JSON，不要 Markdown / code fence / 解释文本"（参考 `memory_pipeline.rs:800`）。

### 角色上下文构建的既有能力（直接复用）

`services/agent_engine.rs` 已有为角色聚合上下文的函数，构造 prompt 时复用：
- `build_role_memory_summary(main_pool, role_id)`（`:1534`）— 角色记忆摘要。
- `build_role_task_summary(main_pool, &role.id)`（在 `build_role_system_prompt` 内被调用，`:1608`）— 角色任务摘要。
- 角色目标 `role.goal`、个性 `role.personality_prompt` 已在 `Role` 模型上。
- 任务列表：`db::tasks::list_tasks_by_role(pool, role_id)`（`db/tasks.rs:51`）。
- 记忆列表：`db::memories::list_memories(pool, Some(role_id), None, limit, None)`。

## Acceptance Criteria

> 既有调度器（Story 4.1）、后台任务、记忆管线和所有前端功能必须零回归。

1. **建议生成与存储（AC1）**
   - **Given** 角色工作循环触发（`run_work_loop_for_role` 被调度器调用）
   - **When** Agent 审视角色目标 + 任务状态 + 记忆
   - **Then** 调用 LLM 生成 0-3 条建议
   - **And** 每条建议写入 `suggestions` 表，`status = 'pending'`，`priority ∈ {high, medium, low}`
   - **And** 写入字段含 `role_id`、`title`、`content`、`priority`、`created_at`

2. **建议必须基于真实上下文（AC2）**
   - **Given** 生成的建议
   - **Then** prompt 必须包含角色当前 `goal`、任务状态（quadrant / is_completed / deadline 等）和最近记忆
   - **And** 建议内容应可追溯到这些上下文，不得凭空捏造

3. **去重 — 由 LLM 判重（AC3）**
   - **Given** 角色最近 7 天已生成的建议
   - **When** 本次生成新建议
   - **Then** 把近 7 天已有建议（title + content）一并注入生成 prompt，明确指示 LLM「不要生成与以下已有建议重复或高度相似的内容」
   - **And** 由 LLM 在生成阶段直接规避重复（建议数量少，模型判重足够可靠）
   - **And** 另加一道确定性兜底：归一化后 title 与近 7 天某条完全相同的候选不写入
   - **And** 不使用 embedding / 余弦相似度（原 epic 写的"余弦相似度 > 0.85"在当前代码库无向量基础设施，已改为 LLM 判重）

4. **空输出允许（AC4）**
   - **Given** 角色无目标、无任务、无近期记忆
   - **When** 工作循环触发
   - **Then** 不生成建议（允许 0 条输出，正常返回 `Ok(())`，不写入任何行）
   - **And** `tracing::info!` 记录"本次无建议生成"

5. **LLM 失败降级（AC5）**
   - **Given** LLM 生成失败（超时 / 格式错误 / provider 错误）
   - **Then** `tracing::warn!` 记录失败原因（含 `role_id`、`role_name`、`error`）
   - **And** 不向用户报错，不 panic，`run_work_loop_for_role` 返回 `Ok(())`（让调度器认为本轮完成，等待下次循环）
   - **And** 不写入任何半成品建议

6. **数据库迁移（AC6）**
   - **Given** 应用启动执行迁移
   - **Then** 新增迁移文件 `migrations/016_suggestions.sql`（**注意：原 epic 写 006 已过时，当前迁移已到 015**）
   - **And** 创建 `suggestions` 表，字段：`id`(TEXT PK)、`role_id`(TEXT NOT NULL, FK→roles ON DELETE CASCADE)、`title`(TEXT NOT NULL)、`content`(TEXT NOT NULL)、`priority`(TEXT NOT NULL CHECK in high/medium/low)、`status`(TEXT NOT NULL DEFAULT 'pending' CHECK in pending/confirmed/rejected)、`rejection_reason`(TEXT)、`converted_task_id`(TEXT)、`created_at`(TEXT NOT NULL DEFAULT)
   - **And** 建索引：`idx_suggestions_role_id`、`idx_suggestions_status`、`idx_suggestions_created_at`

7. **建议生成服务封装（AC7）**
   - **Given** Rust 后端
   - **Then** 新增 `services/suggestion_generator.rs`：构造 prompt（角色定义 + 目标 + 任务列表 + 最近记忆）→ 调 LLM → 解析结构化建议 JSON → 去重 → 返回待写入建议列表
   - **And** `run_work_loop_for_role` 调用该服务并把结果写入 DB
   - **And** 服务对外暴露可单测的纯函数（prompt 构造、JSON 解析、去重判定）

8. **零回归（AC8）**
   - **Given** 本 story 完成
   - **Then** `cargo test` 全量通过，既有调度器/记忆管线/任务测试不回归
   - **And** 前端 `build` 和 `test:frontend` 通过（确认无前端影响）

## Tasks / Subtasks

- [ ] Rust：新增迁移 `migrations/016_suggestions.sql`（AC: 6）
  - [ ] `CREATE TABLE IF NOT EXISTS suggestions (...)`，字段与约束见 AC6
  - [ ] 三个索引：`idx_suggestions_role_id` / `idx_suggestions_status` / `idx_suggestions_created_at`
  - [ ] 参考 `migrations/013_tasks.sql` 的写法（`created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))`、FK ON DELETE CASCADE）

- [ ] Rust：新增 `models/suggestion.rs`（AC: 1, 7）
  - [ ] `Suggestion` 结构体（`Serialize, Deserialize, sqlx::FromRow` + `#[serde(rename_all = "camelCase")]`），字段与表对齐
  - [ ] `models/mod.rs` 追加 `pub mod suggestion;`
  - [ ] 参考 `models/task.rs` 的结构体风格

- [ ] Rust：新增 `db/suggestions.rs`（AC: 1, 3）
  - [ ] `pub async fn create_suggestion(pool, &CreateSuggestionInput) -> Result<Suggestion, AppError>`（生成 uuid + `chrono_now_pub()` 时间戳，参考 `db/roles.rs:11`）
  - [ ] `pub async fn list_recent_suggestions(pool, role_id, since_iso) -> Result<Vec<Suggestion>, AppError>`（查近 7 天，供去重）
  - [ ] `db/mod.rs` 追加 `pub mod suggestions;`
  - [ ] 所有查询用 `?` 处理错误，禁止 `.unwrap()`；错误用 `AppError::DbError`

- [ ] Rust：新增 `services/suggestion_generator.rs`（AC: 1, 2, 3, 4, 5, 7）
  - [ ] `build_suggestion_prompt(role, goal, task_summary, memory_summary) -> Vec<ChatCompletionMessage>`（纯函数，system 强制严格 JSON 输出，user 注入上下文）
  - [ ] LLM 调用：复用 `resolve_default_provider` + `chat_stream` + `mpsc` 累积 + `timeout` + 大小上限（照 `memory_pipeline.rs:419-467`）
  - [ ] `parse_suggestions_response(&str) -> Result<Vec<RawSuggestion>, AppError>`（纯函数，`strip_json_code_fence` + `serde_json::from_str`，校验 priority 合法、裁剪到最多 3 条）
  - [ ] `build_suggestion_prompt` 入参含近 7 天已有建议，在 user 段注入「已有建议清单」+ 指示 LLM 规避重复（LLM 判重，AC3）
  - [ ] `is_exact_title_duplicate(candidate, recent_suggestions) -> bool`（纯函数，归一化 title 精确匹配的确定性兜底）
  - [ ] `pub async fn generate_suggestions(main_pool, role) -> Result<Vec<CreateSuggestionInput>, AppError>`（编排：聚合上下文 + 近 7 天建议→prompt→LLM→解析→title 兜底过滤→返回；失败时 `Ok(Vec::new())` 降级）
  - [ ] `services/mod.rs` 追加 `pub mod suggestion_generator;`

- [ ] Rust：把 `run_work_loop_for_role` 占位实现替换为真实逻辑（AC: 1, 4, 5）
  - [ ] 调用 `suggestion_generator::generate_suggestions(pool, role)`
  - [ ] 对返回的每条建议调 `db::suggestions::create_suggestion` 写入 `pending`
  - [ ] 0 条时 `tracing::info!` 记录"无建议"；写入成功记录条数；任何错误 `tracing::warn!` 并返回 `Ok(())`（不向上抛，保持调度器稳定）
  - [ ] 保持函数签名 `async fn run_work_loop_for_role(pool: &SqlitePool, role: &Role) -> Result<(), AppError>` 不变

- [ ] Rust：单元测试（AC: 2, 3, 4, 7）
  - [ ] `suggestion_generator.rs` 底部 `#[cfg(test)] mod tests`
  - [ ] `parse_suggestions_response`：合法 JSON（含/不含 code fence）→ 正确解析；超过 3 条 → 裁剪；非法 priority → 过滤或报错；空数组 → `Ok(vec![])`；非 JSON → `Err`
  - [ ] `is_exact_title_duplicate`：归一化 title 相同 → true；不同 → false；空近期列表 → false
  - [ ] `build_suggestion_prompt`：验证 system 含"严格 JSON"约束、user 含 goal/task/memory 占位、且含近 7 天已有建议清单与「规避重复」指示
  - [ ] （可选）`db/suggestions.rs` 集成测试：用内存/临时 DB 验证 create + list_recent

- [ ] 验证（AC: 1-8）
  - [ ] `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` 通过
  - [ ] `npm --prefix "egosync-app" run build` 通过
  - [ ] `npm --prefix "egosync-app" run test:frontend` 通过

## Dev Notes

### Current State（基于当前代码 @ c83d6dd）

- **调度器接入点**（`services/scheduler.rs:142-155`）：`run_work_loop_for_role` 当前占位，注释已写明"Story 4.2 将在此函数内添加 LLM 调用、建议生成逻辑"。调度器 spawn 该函数时已记录 `triggered_at`/`duration_ms`/`result`，本 story 不要重复这些日志，只在函数内部记录建议生成相关日志。
- **后台 LLM 范本**（`services/memory_pipeline.rs`）：
  - `extract_for_conversation` → 顶层 `match` 把内部错误降级为 `Ok(0)` + `tracing::warn!`（本 story 的 `run_work_loop_for_role` 照此"永不向上抛错"原则）。
  - `reconcile_memories`（`:389-467`）：完整的 `chat_stream` + `mpsc` + `timeout` + 大小上限 + JSON 解析模板，**直接照抄结构**。
  - `parse_role_memory_assignment_response`（`:830`）：`strip_json_code_fence` + `serde_json::from_str` + 校验过滤模板。
  - system prompt 严格 JSON 约束示例（`:800`）。
- **LLM provider 解析**（`services/agent_engine.rs:1928` `resolve_default_provider`）：返回 `Arc<dyn LlmProvider>`，anthropic/openai 自动选择。
- **角色上下文聚合**（`services/agent_engine.rs`）：`build_role_memory_summary`（`:1534`）、`build_role_task_summary`（在 `:1608` 被调用，确认其为 `pub`，若非 pub 需提升可见性或在 suggestion_generator 内自行聚合 `db::tasks::list_tasks_by_role`）。
- **LLM traits**（`src/llm/traits.rs`）：`ChatCompletionMessage { role, content, tool_calls, tool_call_id }`、`ChatOptions { disable_thinking, tools, tool_choice }`、`StreamEvent::{Token, Done, Error, Thinking, ToolCall}`、`LlmProvider::chat_stream`。
- **DB 写入范本**（`db/roles.rs:11` `create_role`）：`uuid::Uuid::new_v4().to_string()` + `crate::db::settings::chrono_now_pub()` ISO 8601 时间戳。
- **迁移机制**：SQLx migrate 自动扫描 `migrations/` 目录，按文件名序号执行；只需新增文件，无需手动注册。当前最高序号 **015**，本 story 用 **016**。

### What This Story Changes

**Rust（新增 4 + 修改 4 文件）：**
1. `migrations/016_suggestions.sql` — **新建**：`suggestions` 表 + 索引
2. `models/suggestion.rs` — **新建**：`Suggestion` + `CreateSuggestionInput`
3. `db/suggestions.rs` — **新建**：`create_suggestion` + `list_recent_suggestions`
4. `services/suggestion_generator.rs` — **新建**：prompt 构造 + LLM 调用 + 解析 + 去重 + 编排
5. `services/scheduler.rs` — **修改**：`run_work_loop_for_role` 填充真实逻辑（占位→真实）
6. `models/mod.rs` / `db/mod.rs` / `services/mod.rs` — **修改**：各追加一行模块声明

**无前端改动、无新增 crate 依赖（uuid/sqlx/tokio/serde/tracing/chrono 均已在用）。**

### What Must Be Preserved（防回归）

- **调度器 Story 4.1 行为不回归**：`run_work_loop_for_role` 签名、调度频率、`passive` 跳过、每角色独立 spawn、失败隔离、`triggered_at`/`duration_ms`/`result` 日志全部不变。
- **`run_work_loop_for_role` 必须永不向上抛错**：内部所有失败（LLM/DB/解析）都降级为 `tracing::warn!` + 继续，最终返回 `Ok(())`。一条建议写入失败不应阻断其他建议写入（逐条 `match`，参考 memory_pipeline 风格）。
- **记忆管线/任务/角色既有测试通过**：本 story 新增模块，不改既有模块对外行为。
- **前端零回归**：本 story 不触碰任何前端文件。

### 去重策略（AC3 — 已决策：LLM 判重）

原 epic 写"余弦相似度 > 0.85"，但**当前代码库没有任何 embedding / 向量化基础设施**（记忆去重用的是 `deterministic_reconciliation_actions` 的归一化内容精确匹配 + LLM 复核，见 `memory_pipeline.rs:470`，并无 cosine）。引入 embedding 会显著扩大本 story 范围。

**已确定方案：由 LLM 在生成阶段判重（建议数量少，模型判重可靠且零额外依赖）**：
1. `generate_suggestions` 先查该角色近 7 天 `suggestions`（`list_recent_suggestions`）。
2. `build_suggestion_prompt` 把这批已有建议（title + content）注入 user 段，并明确指示：「以下是该角色近 7 天已生成的建议，请不要生成与之重复或高度相似的新建议；若没有新的有价值建议，返回空数组」。
3. LLM 在生成时即规避重复 —— 这是主路径，不再单独算相似度分数。
4. **确定性兜底**：返回前用 `is_exact_title_duplicate` 过滤掉归一化 title 与近 7 天某条完全相同的候选（防 LLM 偶发无视指示）。

此方案确定性兜底 + LLM 智能判重结合，纯本地、零额外依赖、可单测。无需 embedding 基础设施，无需追加前置 story。

### LLM 输出契约（建议 JSON 结构）

system prompt 强制 LLM **只输出严格 JSON**，建议顶层格式：
```json
{ "suggestions": [ { "title": "...", "content": "...", "priority": "high|medium|low" } ] }
```
- 最多 3 条；无建议时输出 `{ "suggestions": [] }`。
- `priority` 非法值的条目在 `parse_suggestions_response` 中过滤掉（不报错整体失败）。
- 解析前 `strip_json_code_fence`，容忍 LLM 偶尔包 ```json fence。

### Previous Story Intelligence（Story 4.1）

- 4.1 实现演进为 **DB 驱动的 HH:MM 时间点调度**（非原计划的固定 interval），新增了 `scheduler_get_times`/`scheduler_set_times` 命令和前端 SettingsTab 编辑 UI。本 story **不动这套时间配置**，只填充工作循环内部逻辑。
- 4.1 代码评审遗留一条 **deferred 项**直接指向本 story：「同一 tick 多角色到期并发 spawn，4.2 接入 LLM 后可能瞬时高并发」（`scheduler.rs:111`）。**本 story 需注意**：多角色同一时间点触发时会并发调用 LLM。V1 可接受（用户角色数量有限），但应在 `generate_suggestions` 内用 `timeout` 防止单次调用挂死。若需限流，记为后续优化，不在本 story 强制实现（除非用户要求）。
- 4.1 使用 `crate::db::settings::chrono_now_pub()` 做 ISO 8601 墙钟时间戳 — 本 story 写库时间戳沿用同一函数，保持一致。

### Git Intelligence

- `c83d6dd`（HEAD，本 story baseline）— docs: refresh task overview story
- 范本来源：`memory_pipeline.rs`（后台结构化 LLM 任务，记忆提炼，故事 2.6）— **建议生成的直接范本**。
- `068c84b` / `c4248c6`（3.5 / 3.3）— 后台任务 + DB 写入模式参考。
- 注：4.1 的 scheduler 改动在工作区中（HEAD 仍为 c83d6dd），dev 实现前确认 `scheduler.rs` 已是 4.1 完成态（含 `run_work_loop_for_role` 占位）。

### Testing Requirements

- **Rust 单测**（`suggestion_generator.rs` 测试模块）：聚焦纯函数 —
  - `parse_suggestions_response`：含 code fence / 不含 / 超 3 条裁剪 / 非法 priority 过滤 / 空数组 / 非 JSON 报错。
  - `is_duplicate`：相同、高相似（≥阈值）、不同、空历史。
  - `build_suggestion_prompt`：system 含严格 JSON 约束、user 含 goal/task/memory。
- **LLM 调用本身不做联网单测**（与 memory_pipeline 一致，provider 通过参数注入便于测试；若 `chat_stream` 难以 mock，则只测纯函数）。
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`
  - `npm --prefix "egosync-app" run build`
  - `npm --prefix "egosync-app" run test:frontend`

### Project Structure Notes

- **新增文件（4）**：`migrations/016_suggestions.sql`、`models/suggestion.rs`、`db/suggestions.rs`、`services/suggestion_generator.rs`
- **修改文件（4）**：`services/scheduler.rs`、`models/mod.rs`、`db/mod.rs`、`services/mod.rs`
- **无新增依赖、无 DB schema 破坏性改动**（纯新增表）
- 符合项目规则：Rust 三层（command→service→db，本 story 无 command 层，建议读写命令是 Story 4.4 范畴）、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块/文件 snake_case、迁移 `{seq}_{description}.sql`

### References

- `_bmad-output/project-context.md`（Rust 三层 / serde camelCase / 无 unwrap / tracing / 迁移命名 / 数据边界）
- `_bmad-output/planning-artifacts/epics.md:1620-1651`（Story 4.2 定义）
- `_bmad-output/planning-artifacts/architecture.md:478`（实现序列「工作循环调度器 + 建议系统」）、`:498`（`suggestions` 表命名约定）
- `_bmad-output/implementation-artifacts/4-1-background-scheduler-work-loop.md`（前序故事，调度器接入点）
- `egosync-app/src-tauri/src/services/scheduler.rs:142-155`（`run_work_loop_for_role` 占位 — 本 story 填充处）
- `egosync-app/src-tauri/src/services/memory_pipeline.rs:389-467`（后台 LLM 调用范本：chat_stream + mpsc + timeout + 大小上限）
- `egosync-app/src-tauri/src/services/memory_pipeline.rs:816-871`（结构化 JSON 解析 + 校验过滤范本）
- `egosync-app/src-tauri/src/services/agent_engine.rs:1928-1952`（`resolve_default_provider`）
- `egosync-app/src-tauri/src/services/agent_engine.rs:1534-1554`（`build_role_memory_summary`）、`:1608`（`build_role_task_summary` 调用点）
- `egosync-app/src-tauri/src/llm/traits.rs`（`ChatCompletionMessage` / `ChatOptions` / `StreamEvent` / `LlmProvider`）
- `egosync-app/src-tauri/src/db/roles.rs:11-33`（DB 写入 + uuid + chrono_now_pub 范本）
- `egosync-app/src-tauri/src/db/tasks.rs:51`（`list_tasks_by_role`）
- `egosync-app/src-tauri/migrations/013_tasks.sql`（迁移文件写法范本）
- `egosync-app/src-tauri/src/models/task.rs`（模型结构体风格范本）

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Change Log

### Review Findings
