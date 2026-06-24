---
baseline_commit: 2b77a1b
---

# Story 5.2: 未设定使命时管家基于行为推断隐含价值观

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 即使没有设定使命宣言系统也能理解我的优先级,
so that 不需要显式声明也能得到合理的仲裁建议。

## 背景与现状（务必先读）

**本 story 是 Epic 5 的第二个 story — 在 Story 5.1 已建成的 `mission` 表和 CRUD 基础上，新增 `mission_inferrer` Rust 服务，当用户未设定使命宣言时，通过 LLM 分析最近 30 天的对话/任务/记忆模式，推断 2-3 条隐含价值观摘要。推断结果在前端 ButlerSettingsContent 中展示，用户可"确认采纳"或"不准确"。本 story 不涉及冲突仲裁逻辑（Story 5.4）和冲突检测（Story 5.3），只负责"推断 + 展示 + 采纳/拒绝"。**

### 已建成的基础（本 story 的接入点）

**Story 5.1 已完成的设施：**
- `src-tauri/migrations/020_mission.sql` — `mission` 表已创建（`id TEXT PK, content TEXT, format TEXT, updated_at TEXT`，单行 `id='singleton'`）
- `src-tauri/src/models/mission.rs` — `Mission` struct（`id, content: Option<String>, format: String, updated_at: String`）
- `src-tauri/src/db/mission.rs` — `get_mission(pool) -> Result<Option<Mission>>` 和 `upsert_mission(pool, content, format) -> Result<Mission>`
- `src-tauri/src/commands/mission.rs` — `mission_get` 和 `mission_update` 两个 Tauri command 已注册
- `src/services/missionService.ts` — `missionService.get()` 和 `missionService.update(content, format)`
- `src/types/mission.ts` — `Mission` interface
- `src/components/butler/ButlerSettingsContent.tsx` — 使命宣言区域已接通真实数据，支持自由文本/结构化模板切换

**本 story 需要新增的核心服务 — `mission_inferrer`：**
- 新建 `src-tauri/src/services/mission_inferrer.rs` — LLM 推断服务
- 需在 `src-tauri/src/services/mod.rs` 添加 `pub mod mission_inferrer;`
- 需在 `src-tauri/src/commands/mission.rs` 新增 `mission_infer` command
- 需在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册 `commands::mission::mission_infer`

**LLM 调用模式参考 — `task_classifier.rs`：**
- `src-tauri/src/services/task_classifier.rs:1-316` — 完整的 LLM 调用模式：`build_default_provider` → `call_llm_with_timeout` → `parse_response` → 降级处理
- `src-tauri/src/services/task_classifier.rs:28` — `LLM_TIMEOUT_SECS = 12`，本 story 复用相同超时
- `src-tauri/src/services/task_classifier.rs:306-316` — `build_default_provider(pool)` 函数从 DB 读取默认 LLM 配置，创建 provider 实例
- `src-tauri/src/services/task_classifier.rs:257-303` — `call_llm_with_timeout(provider, prompt)` 函数，使用 mpsc channel 收集流式 token + timeout
- `src-tauri/src/services/task_classifier.rs:436-472` — `parse_classification_response(raw)` 和 `extract_json_object(raw)` 函数，容忍 LLM 在 JSON 前后输出额外文字
- **本 story 的 `mission_inferrer` 应复用 `build_default_provider` 和 `call_llm_with_timeout` 的模式，但 prompt 构造和 response 解析不同**

**历史数据查询 — 推断依据：**
- `src-tauri/src/db/tasks.rs:60-85` — `list_all_tasks(pool, quadrant, is_big_rock)` 查询全量任务（含角色名 JOIN）
- `src-tauri/src/db/tasks.rs:429-454` — `list_recent_tasks_by_owner(pool, owner_type, role_id, exclude_task_id, limit)` 查询最近 N 条任务
- `src-tauri/src/db/memories.rs:357-387` — `list_all_memories_with_options(pool, category, limit, offset)` 查询全部记忆（排除 task_status 类别）
- `src-tauri/src/db/memories.rs:389-401` — `count_memories(pool, role_id, include_role_memories, category)` 统计记忆数量
- `src-tauri/src/db/conversations.rs:253-263` — `list_all_conversations(pool)` 查询全部对话
- `src-tauri/src/db/conversations.rs:288-300` — `list_messages(pool, conversation_id)` 查询对话消息
- `src-tauri/src/db/conversations.rs:376-393` — `get_recent_messages(pool, conversation_id, limit)` 查询最近 N 条已完成消息
- `src-tauri/src/db/roles.rs:35-37` — `list_active_roles(pool)` 查询活跃角色列表
- **注意**：对话数据存储在独立的 `conversations.db`（`ConversationsPool`），任务和记忆存储在主 `egosync.db`（`DbPool`）。推断服务需同时访问两个 pool

**前端展示接入点 — ButlerSettingsContent.tsx：**
- `src/components/butler/ButlerSettingsContent.tsx:41-46` — 现有 mission 相关 state（`mission`, `missionFormat`, `structuredMission` 等）
- `src/components/butler/ButlerSettingsContent.tsx:100-128` — `useEffect` 加载 mission 数据的模式
- `src/components/butler/ButlerSettingsContent.tsx:343-372` — `handleSaveMission` 保存使命宣言的模式
- `src/components/butler/ButlerSettingsContent.tsx:400-489` — 使命宣言区域 JSX（自由文本/结构化模板切换 + 保存按钮）
- **本 story 需在使命宣言区域上方或下方新增"系统推断的优先级"灰色提示框区域，仅在用户未设定使命宣言且有足够数据时显示**

**前端 service 层接入点：**
- `src/services/missionService.ts:4-8` — 现有 `missionService` 对象，需新增 `inferValues()` 方法
- `src/types/mission.ts:1-6` — 现有 `Mission` interface，需新增 `InferredValues` interface

**Tauri command 注册：**
- `src-tauri/src/lib.rs:360-361` — 现有 `commands::mission::mission_get` 和 `commands::mission::mission_update` 注册位置
- **本 story 需在此处添加 `commands::mission::mission_infer`**

**Error 处理：**
- `src-tauri/src/error.rs:1-66` — `AppError` 枚举，含 `LlmError(String)` / `DbError(String)` / `ValidationError(String)` 等变体
- **本 story 沿用现有 AppError，无需新增变体**

**DB 迁移：**
- 现有迁移文件编号到 `020_mission.sql`
- **本 story 不需要新建迁移文件** — 推断结果不持久化到 DB，每次按需调用 LLM 推断（推断结果仅在用户点击"确认采纳"时通过现有 `mission_update` 写入 `mission` 表）

## Acceptance Criteria

1. **AC1**: Given 用户未设定使命宣言（`mission` 表无记录或 `content` 为空），When 冲突仲裁触发（本 story 中为前端手动触发推断请求），Then `mission_inferrer` 服务收集最近 30 天的对话/任务/记忆模式数据，And LLM 推断隐含优先级（如"用户频繁优先处理家庭相关任务"）

2. **AC2**: Given 推断结果，Then 格式为 2-3 条价值观摘要（如"家庭陪伴 > 工作效率 > 个人学习"），And 推断结果以结构化 JSON 返回（`{ "values": ["价值观1 > 价值观2 > 价值观3"], "summary": "中文摘要", "confidence": 0.0-1.0 }`），And 在仲裁时可替代使命宣言作为第一步依据（本 story 只提供推断 API，仲裁集成在 Story 5.4）

3. **AC3**: Given 用户在 ButlerView SettingsTab 点击"推断使命宣言"按钮，Then 弹窗显示"基于行为推断的使命宣言"（含推断的优先级列表 + 可编辑的使命宣言文本 + 置信度提示），And 用户可编辑推断的使命宣言文本后点击"确认采纳"将编辑后内容转为正式使命宣言（调用现有 `mission_update`，`format='free'`，`content` = 用户编辑后的文本），And 用户可点击"不准确"关闭弹窗

4. **AC4**: Given 推断依据不足（新用户，对话 < 5 轮 或 任务总数 < 5），Then 不显示推断区域，And `mission_infer` command 返回 `None`（而非错误），And 仲裁仅基于四象限 + 能量值（仲裁集成在后续 Story 5.4）

5. **AC5**: Given Rust 后端，Then `mission_inferrer` 服务：收集历史数据（最近 30 天对话消息 + 全量任务 + 全量记忆 + 活跃角色）→ 构造推断 prompt → 调用默认 LLM provider（复用 `build_default_provider` 模式）→ 返回结构化优先级 JSON，And LLM 超时/错误/解析失败时返回 `None`（降级，不阻塞用户）

6. **AC6**: Given 用户已设定使命宣言（`content` 非空），Then "推断使命宣言"按钮仍可点击（手动触发模式下，用户可随时用最新数据重新推断），And 推断结果在弹窗中展示，用户可选择采纳或忽略

7. **AC7**: Given 前端，Then `missionService` 新增 `inferValues()` 方法调用 `mission_infer` command，And `InferredValues` 类型定义新增到 `src/types/mission.ts`，And ButlerSettingsContent 在使命宣言区域上方显示推断结果（灰色提示框 + "确认采纳" / "不准确" 两个按钮）

## Tasks / Subtasks

- [ ] **Task 1: Rust service 层 — `mission_inferrer` 服务** (AC: #1, #2, #5)
  - [ ] 1.1 新建 `src-tauri/src/services/mission_inferrer.rs`：
    - 定义 `InferredValues` struct：`{ values: Vec<String>, summary: String, confidence: f64 }`
    - 定义 `InferenceSource` enum：`Llm` / `SkippedHasMission` / `SkippedInsufficientData` / `FallbackLlmError` / `FallbackTimeout` / `FallbackParseError`
    - 定义 `InferenceOutcome` struct：`{ values: Option<InferredValues>, source: InferenceSource }`
    - 常量：`LLM_TIMEOUT_SECS = 12`（与 task_classifier 一致）、`MIN_CONVERSATIONS = 5`（最小对话轮数阈值）、`MIN_TASKS = 5`（最小任务数阈值）、`HISTORY_DAYS = 30`（历史数据天数）
    - `pub async fn infer_values(pool: &SqlitePool, conv_pool: &ConversationsPool) -> Result<InferenceOutcome, AppError>` — 主入口函数
  - [ ] 1.2 在 `src-tauri/src/services/mod.rs` 添加 `pub mod mission_inferrer;`
  - [ ] 1.3 实现 `infer_values` 函数逻辑：
    1. 调用 `db::conversations::list_all_conversations(conv_pool)` 统计对话数 — 若 < `MIN_CONVERSATIONS`，返回 `SkippedInsufficientData`
    2. 调用 `db::tasks::list_all_tasks(pool, None, None)` 统计任务数 — 若 < `MIN_TASKS`，返回 `SkippedInsufficientData`
    3. 收集历史数据：
       - 最近 30 天对话消息：遍历全部对话，取每个对话最近 N 条 user 消息（截断到 30 天内），汇总为对话摘要文本
       - 全量任务（含角色名）：`list_all_tasks(pool, None, None)`，格式化为任务列表文本
       - 全量记忆（排除 task_status）：`list_all_memories_with_options(pool, None, Some(50), None)`，格式化为记忆列表文本
       - 活跃角色列表：`list_active_roles(pool)`，格式化为角色+目标文本
    4. 构造推断 prompt（见 1.4）
    5. 调用 `build_default_provider(pool)` → `call_llm_with_timeout(provider, prompt)` → `parse_inference_response(raw)`
    6. LLM 超时/错误/解析失败 → 返回对应 Fallback source（`None` values）
  - [ ] 1.3b 实现 `check_eligibility` 函数（预检，不调用 LLM）：
    - 检查对话数 >= `MIN_CONVERSATIONS` 且任务数 >= `MIN_TASKS`
    - 返回 `InferenceEligibility { eligible: bool, reason: String }`
    - 注意：使命宣言是否已设定不影响 eligibility — 用户可随时重新推断
  - [ ] 1.4 实现 `build_inference_prompt` 函数：
    - 输入：对话摘要 + 任务列表 + 记忆列表 + 角色列表
    - 输出要求：严格 JSON `{ "values": ["价值观1 > 价值观2 > 价值观3"], "summary": "中文摘要<=60字", "confidence": 0.0-1.0 }`
    - prompt 内容：说明 EgoSync 管家角色，要求基于用户行为模式推断 2-3 条隐含价值观优先级，给出中文摘要
    - 参照 `task_classifier.rs:332-373` 的 `build_butler_classification_prompt` 格式风格
  - [ ] 1.5 实现 `parse_inference_response(raw: &str) -> Option<InferredValues>` 函数：
    - 复用 `extract_json_object` 模式（从第一个 `{` 到最后一个 `}`）
    - 解析 JSON：`values` 为字符串数组（1-3 条），`summary` 为中文字符串，`confidence` 为 0.0-1.0 浮点数
    - 任何字段缺失/非法 → 返回 `None`
    - 参照 `task_classifier.rs:436-463` 的 `parse_classification_response` 模式
  - [ ] 1.6 实现 `build_default_provider` 和 `call_llm_with_timeout`：
    - **重要**：这两个函数在 `task_classifier.rs` 中是 private 的。本 story 有两个选择：
      - **方案 A（推荐）**：在 `mission_inferrer.rs` 中复制 `build_default_provider` 和 `call_llm_with_timeout` 的实现（约 60 行），保持模块独立性
      - **方案 B**：将 `task_classifier.rs` 中的这两个函数提取为 pub，在 `mission_inferrer.rs` 中直接调用
    - 无论哪种方案，函数签名和行为必须与 `task_classifier.rs` 完全一致
  - [ ] 1.7 实现 `truncate` 辅助函数（与 `task_classifier.rs:474-483` 一致），用于截断过长的历史数据文本

- [ ] **Task 2: Rust command 层 — `mission_infer` + `mission_infer_eligibility` Tauri command** (AC: #1, #4, #5, #6)
  - [ ] 2.1 在 `src-tauri/src/commands/mission.rs` 新增 `mission_infer` command：
    ```rust
    #[tauri::command]
    pub async fn mission_infer(
        pool: State<'_, DbPool>,
        conv_pool: State<'_, ConversationsPool>,
    ) -> Result<Option<InferredValues>, AppError> {
        let outcome = services::mission_inferrer::infer_values(&pool, &conv_pool).await?;
        Ok(outcome.values)
    }
    ```
  - [ ] 2.2 新增 `mission_infer_eligibility` command（预检，不调用 LLM）：
    ```rust
    #[tauri::command]
    pub async fn mission_infer_eligibility(
        pool: State<'_, DbPool>,
        conv_pool: State<'_, ConversationsPool>,
    ) -> Result<InferenceEligibility, AppError> {
        mission_inferrer::check_eligibility(&pool, &conv_pool).await
    }
    ```
  - [ ] 2.3 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册 `commands::mission::mission_infer` 和 `commands::mission::mission_infer_eligibility`
  - [ ] 2.4 `InferredValues` 和 `InferenceEligibility` 需要派生 `serde::Serialize` 以便通过 Tauri IPC 传递到前端

- [ ] **Task 3: 前端类型定义** (AC: #7)
  - [ ] 3.1 在 `src/types/mission.ts` 新增 `InferredValues` 和 `InferenceEligibility` interface：
    ```typescript
    export interface InferredValues {
      values: string[];
      summary: string;
      confidence: number;
    }
    export interface InferenceEligibility {
      eligible: boolean;
      reason: string;
    }
    ```

- [ ] **Task 4: 前端 service 层** (AC: #7)
  - [ ] 4.1 在 `src/services/missionService.ts` 新增 `inferValues` 和 `checkInferenceEligibility` 方法：
    ```typescript
    inferValues: () => invoke<InferredValues | null>('mission_infer'),
    checkInferenceEligibility: () => invoke<InferenceEligibility>('mission_infer_eligibility'),
    ```
  - [ ] 4.2 导入 `InferredValues` 和 `InferenceEligibility` 类型

- [ ] **Task 5: 前端 UI — ButlerSettingsContent 手动触发推断 + 弹窗确认** (AC: #3, #4, #7)
  - [ ] 5.1 新增 state：`inferredValues`、`isInferring`、`inferenceDismissed`、`inferenceEligibility`、`isInferenceModalOpen`、`editableSummary`
  - [ ] 5.2 在 `useEffect`（依赖 `[]`，仅挂载时执行）中加载 mission 数据后调用 `refreshEligibility()` 检查推断资格
  - [ ] 5.3 在使命宣言区域的格式切换按钮旁新增"推断使命宣言"按钮：
    - `disabled={!inferenceEligibility?.eligible}`（数据不足时禁用，悬浮提示 reason）
    - 推断中时用 `pointer-events-none` 防止重复点击，显示 `animate-loading-spin` spinner + "推断中..."
    - 点击调用 `handleTriggerInference`
  - [ ] 5.4 `handleTriggerInference` 函数：
    - 设置 `isInferring = true` → 调用 `missionService.inferValues()` → 成功后设置 `inferredValues`、`editableSummary`、打开弹窗
    - 返回 null → 显示"暂无足够数据进行推断"错误提示
    - 失败 → 显示"推断失败，请稍后重试"错误提示
  - [ ] 5.5 推断结果弹窗（`isInferenceModalOpen && inferredValues`）：
    - 标题："基于行为推断的使命宣言"
    - 推断的优先级列表（`inferredValues.values` 每条一行）
    - 可编辑的使命宣言 textarea（绑定 `editableSummary`）
    - 置信度提示：若 `inferredValues.confidence < 0.7`，显示"（置信度较低，仅供参考）"
    - "确认采纳"按钮：调用 `missionService.update(editableSummary, 'free')` → 刷新 mission state → 关闭弹窗 → `refreshEligibility()`
    - "不准确"按钮：关闭弹窗，设置 `inferenceDismissed = true`
  - [ ] 5.6 `handleAdoptInferred` 函数：使用 `editableSummary`（用户编辑后的文本）而非 `inferredValues.summary`
  - [ ] 5.7 `handleDismissInferred` 函数：关闭弹窗，设置 `inferenceDismissed = true`
  - [ ] 5.8 `refreshEligibility` 函数：调用 `missionService.checkInferenceEligibility()` 更新 `inferenceEligibility` state，在 `handleSaveMission` 和 `handleAdoptInferred` 后调用

- [ ] **Task 6: 单元测试** (AC: #1, #2, #4, #5, #6)
  - [ ] 6.1 Rust service 测试 — 在 `mission_inferrer.rs` 添加 `#[cfg(test)] mod tests`：
    - `parse_inference_response_accepts_clean_json` — 正常 JSON 解析
    - `parse_inference_response_strips_surrounding_prose` — 容忍 JSON 前后额外文字
    - `parse_inference_response_rejects_empty_values` — values 数组为空时返回 None
    - `parse_inference_response_rejects_out_of_range_confidence` — confidence > 1.0 或 < 0.0 返回 None
    - `parse_inference_response_rejects_missing_field` — 缺少 values/summary/confidence 任一字段返回 None
    - `build_inference_prompt_contains_required_context` — prompt 包含对话/任务/记忆/角色上下文
    - `build_inference_prompt_contains_json_format_instruction` — prompt 包含严格 JSON 输出要求
  - [ ] 6.2 Rust command 测试 — 验证 `mission_infer` 在 mission 已设定时返回 `None`（mock 或集成测试）
  - [ ] 6.3 前端 service 测试 — 验证 `missionService.inferValues()` 的 invoke 调用参数正确

## Dev Notes

### 关键技术决策

- **推断结果不持久化**：每次 `mission_infer` 调用都实时执行 LLM 推断，不缓存到 DB。理由：(1) 用户行为持续变化，缓存会过期；(2) 推断结果仅在用户"确认采纳"时才通过现有 `mission_update` 写入 `mission` 表；(3) 避免新增 DB 表/迁移
- **双 Pool 访问**：推断服务需同时访问主 DB（任务/记忆/角色）和对话 DB（对话/消息）。`mission_infer` command 需注入 `DbPool` 和 `ConversationsPool` 两个 State
- **数据不足阈值**：对话 < 5 轮 或 任务 < 5 个，不执行推断。这是 AC4 的硬性要求。对话数按 `list_all_conversations` 返回的对话数量计算（非消息数），任务数按 `list_all_tasks` 返回的任务数量计算
- **30 天时间窗口**：对话消息按 `created_at` 过滤最近 30 天。任务和记忆不按时间过滤（全量取，因为任务/记忆数量相对较少且已有自然淘汰机制）
- **LLM 降级策略**：LLM 超时/错误/解析失败时返回 `None`（而非错误），前端不显示推断区域。这与 `task_classifier` 的降级策略一致（不阻塞用户操作）
- **`build_default_provider` 复用**：`task_classifier.rs` 中的 `build_default_provider` 和 `call_llm_with_timeout` 是 private 函数。推荐方案 A（复制实现），因为：(1) 两个服务的 LLM 调用需求略有不同（prompt 构造、response 解析不同）；(2) 避免修改已测试通过的 `task_classifier.rs`；(3) 代码量小（约 60 行）
- **推断 prompt 设计**：prompt 需包含足够的上下文让 LLM 推断价值观，但不能过长（token 限制）。对话消息取最近 30 天每个对话最近 5 条 user 消息（截断），任务取全部（含角色名），记忆取最近 50 条（排除 task_status），角色取活跃角色列表
- **"确认采纳"的存储格式**：采纳时 `format='free'`，`content = inferredValues.summary`（中文摘要文本）。不使用 `structured` 格式，因为推断结果是自然语言摘要

### 架构合规

- **分层规则**：前端 → `missionService.inferValues()` → `invoke('mission_infer')` → `commands/mission.rs`（薄层解析）→ `services/mission_inferrer.rs`（业务逻辑 + LLM 调用）→ `db/` 层（SQL 执行）→ SQLite
- **services/ 层职责**：`mission_inferrer` 拥有所有推断业务逻辑（数据收集、prompt 构造、LLM 调用、response 解析、降级处理）。command 层只做参数校验 + 调 service + 返回结果
- **命名规范**：Rust command 使用 `snake_case`（`mission_infer`），前端 service 使用 `camelCase`（`missionService.inferValues()`）
- **错误处理**：command 返回 `Result<Option<InferredValues>, AppError>`，前端 try-catch。推断失败返回 `Ok(None)` 而非 `Err(...)`，因为"无法推断"是正常业务状态而非错误
- **serde 桥接**：Rust `InferredValues` 需派生 `serde::Serialize` + `#[serde(rename_all = "camelCase")]`，前端 `InferredValues` interface 使用 camelCase

### 前端 UI 规范

- **设计系统**：Tailwind CSS + shadcn/ui，参照现有 ButlerSettingsContent 的使命宣言区域样式
- **推断区域样式**：灰色提示框 `bg-slate-50 border border-slate-200 rounded-lg p-4`，与使命宣言区域用 `pt-6 border-t border-slate-200/80` 分隔
- **按钮样式**：
  - "确认采纳"：`bg-indigo-600 text-white` 主按钮样式（参照 `ButlerSettingsContent.tsx:477`）
  - "不准确"：`bg-white border border-slate-200 text-slate-600` 次按钮样式（参照 `ButlerSettingsContent.tsx:465`）
- **加载中状态**：显示"正在分析您的行为模式..."文本，参照 `ButlerSettingsContent.tsx:421` 的加载提示样式
- **置信度提示**：`text-[12px] text-slate-400`，低置信度时显示
- **推断区域位置**：使命宣言区域**上方**，因为推断是"建议"，用户先看到建议再决定是否手动设定

### 反模式警告

- **不要**新建 DB 迁移文件 — 推断结果不持久化到 DB，仅在用户"确认采纳"时通过现有 `mission_update` 写入 `mission` 表
- **不要**在 `commands/` 层添加业务逻辑 — `mission_infer` command 只做调 service + 返回结果
- **不要**在推断服务中修改任何 DB 数据 — 推断是只读操作，不写入任何表
- **不要**缓存推断结果到全局 state 或 DB — 每次按需调用，避免过期数据
- **不要**在 `task_classifier.rs` 中提取 `build_default_provider` 为 pub — 推荐在 `mission_inferrer.rs` 中复制实现，保持模块独立性
- **不要**将推断结果用于仲裁 — 仲裁集成是 Story 5.4 的职责，本 story 只提供推断 API 和前端展示
- **不要**在推断 prompt 中包含完整的对话内容 — 对话可能很长，需截断每个对话到最近 5 条 user 消息
- **不要**忽略 `ConversationsPool` — 对话数据在独立的 `conversations.db`，需要同时注入 `DbPool` 和 `ConversationsPool`
- **不要**在用户已设定使命宣言时阻止推断 — 手动触发模式下，用户可随时用最新数据重新推断，使命宣言是否已设定不影响 eligibility 或推断执行

### Project Structure Notes

新增文件：
- `src-tauri/src/services/mission_inferrer.rs` — Rust LLM 推断服务

修改文件：
- `src-tauri/src/services/mod.rs` — 添加 `pub mod mission_inferrer;`
- `src-tauri/src/commands/mission.rs` — 新增 `mission_infer` command + 导入 `InferredValues` 和 `ConversationsPool`
- `src-tauri/src/lib.rs` — 注册 `commands::mission::mission_infer`
- `src/types/mission.ts` — 新增 `InferredValues` interface
- `src/services/missionService.ts` — 新增 `inferValues()` 方法
- `src/components/butler/ButlerSettingsContent.tsx` — 新增推断区域 UI + state + 事件处理

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.2] — AC 原文
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping] — FR-13~15 → `arbitration.rs` + `mission` DB 表
- [Source: _bmad-output/planning-artifacts/architecture.md#Layer Rules] — 分层规则
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Boundaries] — 主 DB vs 对话 DB 的访问规则
- [Source: _bmad-output/implementation-artifacts/5-1-mission-statement-setting.md] — 前一 story（mission 表 + CRUD + 前端 UI）
- [Source: src-tauri/src/services/task_classifier.rs] — LLM 调用模式参考（build_default_provider, call_llm_with_timeout, parse_response, 降级策略）
- [Source: src-tauri/src/db/conversations.rs] — 对话/消息查询函数
- [Source: src-tauri/src/db/memories.rs] — 记忆查询函数
- [Source: src-tauri/src/db/tasks.rs] — 任务查询函数
- [Source: src-tauri/src/db/roles.rs] — 角色查询函数

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Review Findings

_代码审查 2026-06-24（审查者：Amelia）。范围：Story 5.2 未提交改动（9 文件，约 +752/-2）。审查模式：full（含 AC 验收审计）。_

- [x] [Review][Patch] 推断区域渲染条件 `!mission` 守卫已移除 [egosync-app/src/components/butler/ButlerSettingsContent.tsx] — 变更：手动触发模式下，使命宣言是否已设定不影响推断，`!mission` 守卫已移除。推断结果改为弹窗展示，显示条件为 `isInferenceModalOpen && inferredValues`。
- [x] [Review][Patch] `get_recent_messages` 硬编码取数 10，未与 `MSG_PER_CONVERSATION` 关联 [egosync-app/src-tauri/src/services/mission_inferrer.rs:186] — 已修复：新增常量 `RECENT_MESSAGES_FETCH = 10` 替换魔法数字。 — `collect_conversation_summary` 取每个对话最近 10 条消息，但常量 `MSG_PER_CONVERSATION = 5` 仅用于 `.take(5)`。魔法数字 10 应抽为常量或基于 `MSG_PER_CONVERSATION` 推导，避免后续两值不一致。
- [x] [Review][Patch] Task 6.2 测试已更新为 `infer_values_ignores_mission_and_checks_data` [egosync-app/src-tauri/src/services/mission_inferrer.rs] — 变更：`infer_values` 不再因 mission 已设定而跳过，测试已更新为验证 mission 已设定时仍继续检查数据充足性。

_已核实安全/符合设计（不计入修复项）：_
- 消息 `created_at` 与 30 天截止字符串均为 `%Y-%m-%dT%H:%M:%SZ` 固定宽度 ISO 8601 UTC，字典序比较 `>=` 正确，无误判。
- 数据充足性门槛按"对话记录总数 < 5 或任务总数 < 5"判定（不限 30 天），与 AC4 + Dev Notes 一致；30 天窗口仅作用于对话消息收集，属设计内行为。
- `build_default_provider` / `call_llm_with_timeout` 为 `task_classifier.rs` 的精确复制，符合 spec 方案 A（保持模块独立、不改动已测试模块）。

_流程提醒（非代码缺陷）：_
- 故事文件 Tasks/Subtasks 复选框仍全部未勾选，Dev Agent Record（File List / Completion Notes）为空，建议补齐。
- `mission_inferrer.rs` 仍为 Git 未跟踪状态（`??`），合并前需 `git add`。
- `services/mod.rs` 中 `pub mod mission_inferrer;` 插在 `memory_pipeline` 与 `memory_query` 之间，破坏字母序（轻微风格）。
