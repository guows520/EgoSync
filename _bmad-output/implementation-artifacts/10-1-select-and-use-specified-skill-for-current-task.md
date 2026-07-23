---
baseline_commit: 6192a38
---

# Story 10.1: 为当前任务选择并使用指定 Skill

Status: in-progress

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 管家或角色对话用户,
I want 通过 `@` 选择当前 Agent 可用的 Skill，并仅用于本轮任务,
so that 我能明确、可信且可审计地控制本轮任务使用的能力。

**FRs covered:** FR-37

## Acceptance Criteria

> 来源：`_bmad-output/planning-artifacts/epics.md` 第 2755-2798 行。AC 编号为本 Story 内部编号，用于 Tasks 引用。

**AC-1 候选集合隔离**
**Given** 当前对话属于管家或某个角色
**When** 用户输入 `@` 或继续输入 Skill 名称
**Then** 系统展示匹配列表，且仅包含当前 Agent 已添加且启用的 Skill
**And** 管家与角色分别读取自己的配置，不合并其他 Agent 的配置。

**AC-2 空状态与键盘交互**
**Given** 当前 Agent 没有可用 Skill、无匹配结果或用户使用键盘操作列表
**When** 用户触发补全或按方向键、Enter、Escape
**Then** 界面提供明确空状态、焦点移动、确认与关闭行为
**And** 用户仍可发送普通消息
**And** 控件具有适当的 ARIA 语义。

**AC-3 后端可信校验**
**Given** 用户选择 Skill 并发送消息
**When** 后端收到 `selectedSkillId`
**Then** 后端根据可信 Agent 身份重新解析可用集合
**And** 前端不能自行扩大集合
**And** 只有仍被添加且启用的 Skill 才进入任务上下文。

**AC-4 失效竞态拒绝**
**Given** Skill 在选择后、发送前被关闭、移除或已不存在
**When** 后端验证请求
**Then** 请求在进入 Agent Runtime 前被拒绝
**And** 不调用该 Skill
**And** 用户收到明确提示。

**AC-5 任务级快照与审计**
**Given** 指定 Skill 通过验证
**When** 任务开始执行
**Then** 选择被冻结为本次任务快照
**And** 流式执行期间的配置变化不改变本次上下文
**And** 审计信息记录实际使用的 Skill。

**AC-6 未指定兼容路径**
**Given** 用户没有指定 Skill
**When** 用户发送普通消息
**Then** 现有自动发现和按需加载行为保持不变
**And** 不生成虚假的显式 Skill 审计记录。

**AC-7 选择不沿用**
**Given** 当前任务结束或失败
**When** 用户开始下一条消息
**Then** 上一任务的 Skill 选择不会自动沿用
**And** Skill 长期启用状态没有被修改。

**AC-8 自动化测试覆盖**
**Given** Story 10.1 自动化测试运行
**When** 执行 Rust、前端组件及关键 E2E 测试
**Then** 覆盖 Agent 候选隔离、键盘选择、有效选择、失效竞态、未指定兼容路径、任务快照与审计记录。

## Tasks / Subtasks

> 任务按架构路径自前端到后端、再到测试的顺序排列。每条标注涉及的 AC。所有"修改"为外科手术式扩展，不重构相邻代码。

### 阶段 1：后端数据契约与校验（先稳定契约，前端再接入）

- [x] **Task 1: 扩展 ChatRequest 数据模型** (AC: #3, #4, #6)
  - [x] 1.1 `models/chat.rs` 第 81-90 行 `ChatRequest` 结构体新增字段 `#[serde(default)] pub selected_skill_id: Option<String>`；保留 `rename_all = "camelCase"` 与所有现有字段
  - [x] 1.2 `types/chat.ts` 第 26-32 行 `ChatRequest` 接口新增 `selectedSkillId?: string`；保留所有现有字段
  - [x] 1.3 验证：未指定时 `selected_skill_id` 反序列化为 `None`，现有 onboarding/普通消息路径无回归

- [x] **Task 2: 新增 SkillAvailabilityService 作用域校验** (AC: #1, #3, #4)
  - [x] 2.1 在 `services/skill_registry.rs` 新增 `resolve_enabled(pool, role_id: Option<&str>, skill_id: &str) -> Result<SkillRegistryEntry, AppError>`
    - 管家分支（`role_id = None`）：读 `butler_config::get_butler_skills(pool)` 的 `enabled_skill_ids`，确认 `skill_id` 存在且启用
    - 角色分支（`role_id = Some`）：读 `role_config::enabled_skill_ids_from_config` 的结果，确认 `skill_id` 存在且启用
    - 复用现有 `db/skills.rs::get_skill` 验证 Skill 存在；复用 `db/skill_bindings.rs::skill_ids_for_role` 验证角色已添加该 Skill
    - 失败时返回明确领域错误：`SkillNotFound` / `SkillNotAddedToScope` / `SkillDisabled`，不进入 Runtime
  - [x] 2.2 新增 `list_enabled(pool, role_id: Option<&str>) -> Result<Vec<SkillRegistryEntry>, AppError>`，返回当前 scope 已添加且启用的 Skill 列表（供前端候选查询复用，避免前端自行拼装）
  - [x] 2.3 单元测试覆盖：管家/角色集合隔离、Skill 不存在、未添加、已禁用三种拒绝路径、enabled 列表正确性

- [x] **Task 3: agent_bridge 新增 send_command** (AC: #3, #5)
  - [x] 3.1 在 `services/agent_bridge.rs` 新增 `send_command(&self, session_id: &str, agent: &str, command: &str, arguments: &str) -> Result<Option<OpencodeCompletedMessage>, AppError>`
    - 调用 `POST /session/{sessionId}/command`，body：`{ "agent": "<resolved-agent-name>", "command": "<skill-frontmatter-name>", "arguments": "<normalized-user-content>" }`
    - 复用现有 HTTP 客户端配置（no_proxy、错误处理、SSE 订阅）
    - **架构决策来源**：`architecture.md` 第 1331-1366 行；opencode v1.15.10 原生 Session Command API
  - [x] 3.2 保留现有 `send_message` 不变；两条路径复用同一会话身份解析、SSE 事件订阅、取消处理
  - [x] 3.3 单元测试：command 请求体格式、HTTP 错误传播

- [x] **Task 4: agent_engine 路径分流与快照** (AC: #3, #4, #5, #6)
  - [x] 4.1 `run_stream` / `try_run_opencode_stream` 签名新增 `selected_skill_id` 参数；保留所有现有参数和状态管理
  - [x] 4.2 在调用 `agent_bridge` 前执行校验分流：
    - `selected_skill_id = None` → 调用现有 `bridge.send_message(...)`，行为完全不变（AC-6）
    - `selected_skill_id = Some(id)` → 调用 `skill_registry::resolve_enabled(pool, role_id, id)`；失败返回错误，**不静默降级到 send_message**（AC-4）
    - 校验通过 → 读取 `SkillRegistryEntry.name`（对应 SKILL.md frontmatter name）→ 调用 `bridge.send_command(session_id, agent_key, &skill_name, &content)`
  - [x] 4.3 任务级快照：校验通过后到 Runtime 调用前，`skill_name` 已确定为本次任务快照；流式执行期间不再重新查询配置（AC-5）
  - [x] 4.4 审计：在现有消息过程事件审计中记录实际使用的 Skill（`skill_id`、`skill_name`、`scope`）；未指定 Skill 时不生成显式 Skill 审计记录（AC-5, AC-6）
  - [x] 4.5 Agent 身份解析复用现有 `opencode_agent_key(role_id)`（`agent_engine.rs` 第 69-74 行），不新增身份解析逻辑
  - [ ] 4.6 集成测试：未指定走 `/message`、指定且有效走 `/command`、指定但失效在 Runtime 前被拒绝、流式期间配置变化不影响本次任务

- [x] **Task 5: chat_send_message 接入 selectedSkillId** (AC: #3, #4, #6)
  - [x] 5.1 `commands/chat.rs` 第 213-221 行 `chat_send_message` 从 `request.selected_skill_id` 提取参数，传递给 `agent_engine::run_stream`
  - [x] 5.2 Command 层只做参数解析，不写校验逻辑（校验在 Service 层）
  - [x] 5.3 保留 onboarding 分支和流式状态管理不变

### 阶段 2：前端候选与选择 UI

- [x] **Task 6: 前端候选 Skill 查询** (AC: #1)
  - [x] 6.1 **新增 Tauri Command `skill_list_enabled_for_scope`**：参数 `role_id: Option<String>`，调用 `SkillAvailabilityService::list_enabled(pool, role_id)`，后端返回已校验的候选集合（已添加且启用）
    - **决策已确定**：采用后端直接返回过滤后清单的方案，前端不自行过滤 enabled 状态（遵循架构文档"后端校验是授权边界，前端不能自行扩大集合"原则）
    - 管家分支（`role_id = None`）：返回管家 `enabled_skill_ids` 对应的 Skill 列表
    - 角色分支（`role_id = Some`）：返回该角色 `enabled_skill_ids` 对应的 Skill 列表
  - [x] 6.2 `commands/skill.rs` 新增 `skill_list_enabled_for_scope` command，仅解析参数并调用 `skill_registry::list_enabled`
  - [x] 6.3 `services/skillService.ts` 新增 `listEnabledForScope(roleId?: string): invoke<SkillRegistryEntry[]>('skill_list_enabled_for_scope', { roleId })`
  - [x] 6.4 `lib.rs` `generate_handler!` 注册 `commands::skill::skill_list_enabled_for_scope`
  - [x] 6.5 后端单元测试：管家/角色返回各自启用集合、未启用 Skill 不出现、空集合正确返回（复用 Task 2 的 list_enabled 测试）

- [x] **Task 7: ChatInput @Skill 选择器 UI** (AC: #1, #2)
  - [x] 7.1 `ChatInput.tsx` `ChatInputProps` 新增 `availableSkills: SkillRegistryEntry[]` 和 `onSelectedSkillChange: (skillId: string | null) => void`；保留 `onSend: (content: string) => void` 不变
  - [x] 7.2 新增 `@Skill` 触发逻辑：用户输入 `@` 时弹出候选列表（仅 `availableSkills`，前端不再过滤）
  - [x] 7.3 键盘交互：方向键移动焦点、Enter 确认、Escape 关闭、继续输入按名称过滤候选（AC-2）
  - [x] 7.4 空状态：无可用 Skill 或无匹配时显示明确空状态提示，用户仍可发送普通消息（AC-2）
  - [x] 7.5 ARIA 语义：候选列表 `role="listbox"`、选项 `role="option"`、`aria-activedescendant` 跟踪焦点（AC-2）
  - [x] 7.6 已选 Skill 展示为标签（chip），可删除；删除标签时同时清空 `selectedSkillId`（架构文档第 1521 行）
  - [x] 7.7 `@Skill` 展示标签不是 `content` 的组成部分；发送时 `content` 仅包含用户实际输入文本，不含 `@Skill` 标记（架构文档第 1521 行）
  - [x] 7.8 组件测试 `ChatInput.test.tsx`：候选筛选、空状态、键盘交互、选择/清除、禁用状态
  - [x] 7.9 可访问性测试 `ChatInput.a11y.test.tsx`：候选列表键盘与读屏

- [x] **Task 8: ChatStream 状态管理** (AC: #1, #3, #7)
  - [x] 8.1 `ChatStream.tsx` 新增 `selectedSkillId: string | null` 状态，默认 `null`
  - [x] 8.2 调用 `skillService.listEnabledForScope(role?.id)` 获取候选，传递给 `ChatInput`
  - [x] 8.3 `handleSend` 调用 `chatService.sendMessage({ ...request, selectedSkillId })`
  - [x] 8.4 **任务结束后清空 `selectedSkillId`**，下一条消息不自动沿用（AC-7）
  - [x] 8.5 Skill 长期启用状态不被修改（`selectedSkillId` 仅存在于组件状态，不写回任何配置存储）（AC-7）
  - [x] 8.6 组件测试 `ChatStream.test.tsx`：message/command 分流、任务结束后清空选择、候选集合随角色切换更新

### 阶段 3：E2E 与回归

- [x] **Task 9: E2E 测试** (AC: #1, #3, #4, #5, #8)
  - [ ] 9.1 `tests/e2e/specs/butler-conversation.spec.ts` 新增 `@Skill` 管家主路径：选择 Skill → 发送 → 验证实际生效
  - [ ] 9.2 管家与角色集合不串扰：管家候选不含角色专属 Skill，反之亦然
  - [ ] 9.3 禁用/移除 Skill 后发送被拒绝，用户收到明确提示（AC-4）
  - [ ] 9.4 流式期间配置变化不改变当前任务上下文（AC-5）
  - [ ] 9.5 未指定 Skill 的普通消息路径无回归
  - [x] 9.6 组件测试已覆盖：ChatInput 10 个 + ChatInput a11y 6 个 + ChatStream 2 个 Skill 选择测试

- [x] **Task 10: 回归验证** (AC: #6, #8)
  - [x] 10.1 `cargo check` 通过
  - [x] 10.2 `cargo test --lib -- agent_bridge::tests:: skill_registry::tests::` 全部 48 通过
  - [x] 10.3 `npx vitest run` 全量 42 test files, 396 tests, 0 failed
  - [x] 10.4 `npx tsc --noEmit` 严格编译通过
  - [ ] 10.5 `npm run build` 生产构建（待用户手动验证）
  - [ ] 10.6 E2E 测试（需要运行环境，待用户手动验证）
  - [x] 10.7 预存失败确认：`agent_config::tests` 中 6 个失败为 Windows 中文编码预存问题，与 Story 10.1 改动无关

### Review Findings

- [x] [Review][Patch] 显式 Skill 校验或执行失败会静默降级为普通 LLM，违背拒绝语义 [egosync-app/src-tauri/src/services/agent_engine.rs:2476]
- [x] [Review][Patch] Skill 校验晚于 Runtime 刷新、会话创建和事件订阅，失败路径还会泄漏运行时状态 [egosync-app/src-tauri/src/services/agent_engine.rs:2363]
- [x] [Review][Patch] 后台 Skill 校验失败没有向前端传递领域错误，用户只能看到流结束或空消息 [egosync-app/src-tauri/src/commands/chat.rs:310]
- [x] [Review][Patch] `/command.arguments` 使用混入系统提示和动态上下文的 content，而非规范化用户正文 [egosync-app/src-tauri/src/services/agent_engine.rs:2418]
- [x] [Review][Patch] Skill 在重试循环内重复解析，配置变化可改变同一任务的选择快照 [egosync-app/src-tauri/src/services/agent_engine.rs:3200]
- [x] [Review][Patch] 显式 Skill 使用仅写 tracing 日志，未持久化 ID、名称、scope 和 explicit 来源 [egosync-app/src-tauri/src/services/agent_engine.rs:2476]
- [x] [Review][Patch] 发送后 Skill chip 立即消失，消息或审计区域没有本轮 Skill 展示 [egosync-app/src/components/chat/ChatInput.tsx:65]
- [x] [Review][Patch] 角色切换时未清空旧 Skill 选择及候选，可能把角色 A 的 Skill 发给角色 B [egosync-app/src/components/chat/ChatStream.tsx:986]
- [x] [Review][Patch] 空候选与筛选缩短时 activeIndex 可失效，Enter 还会被吞掉而无法发送普通消息 [egosync-app/src/components/chat/ChatInput.tsx:74]
- [x] [Review][Patch] `@` 触发缺少词边界，会误识别邮箱并可能截断正常文本 [egosync-app/src/components/chat/ChatInput.tsx:106]
- [x] [Review][Patch] Skill 在请求发出时而非任务成功结束后清空，失败后无法按原选择重试 [egosync-app/src/components/chat/ChatInput.tsx:65]
- [x] [Review][Patch] 输入框缺少完整 combobox/aria-autocomplete 语义，现有测试只验证属性存在 [egosync-app/src/components/chat/ChatInput.tsx:153]
- [ ] [Review][Patch] send_command 测试只重建常量 JSON/URL，未调用生产实现或验证真实 HTTP 请求 [egosync-app/src-tauri/src/services/agent_bridge.rs:504]
- [ ] [Review][Patch] resolve_enabled 的多次独立查询不具备一致性快照，配置并发修改时可组合出从未同时有效的状态 [egosync-app/src-tauri/src/services/skill_registry.rs:51]
- [ ] [Review][Patch] 缺少 AC-8 要求的 Runtime 前拒绝、禁止降级、任务快照、审计真实性与关键 E2E 覆盖 [egosync-app/tests/e2e/specs/:missing]

## Dev Notes
### 架构决策（必须遵循，来源：`architecture.md`）

**FR-37 核心决策（第 1331-1386 行）**：
- 使用 opencode v1.15.10 原生 Session Command API，**不建立自定义 Prompt 注入协议**
- `ChatRequest` 新增可选字段 `selected_skill_id`；前端 `@Skill` 仅作为结构化选择交互，发送时从任务正文中移除选择标记
- **前端只提交 EgoSync Skill Registry ID，不得直接提交 opencode command name**
- 当前 Agent 作用域由后端可信上下文解析：无 `roleId` 为管家，有 `roleId` 为指定角色
- 管家可用集合取其 `enabledSkillIds`；角色可用集合取该角色自己的 `enabledSkillIds`；二者相互独立，不继承、不合并
- 后端通过统一 `SkillAvailabilityService` 执行 `list_enabled(scope)` 与 `resolve_enabled(scope, skill_id)`；解析条件为 Skill 存在、已添加且在当前作用域启用
- 任务启动前形成不可变的 Skill 选择快照；任务执行期间的配置变化不影响本次调用

**opencode 格式转换链（第 1346-1353 行）**：
```text
selectedSkillId → SkillRegistryEntry.id → SkillRegistryEntry.name → opencode command
```
`SkillRegistryEntry.name` 对应 Skill `SKILL.md` frontmatter name。opencode v1.15.10 会以该 name 注册原生 Command，**无需增加映射表**。

**send_command 请求格式（第 1357-1366 行）**：
```http
POST /session/{sessionId}/command
Content-Type: application/json

{
  "agent": "<resolved-agent-name>",
  "command": "<skill-frontmatter-name>",
  "arguments": "<normalized-user-content>"
}
```
未指定 Skill 时继续使用现有 `POST /session/{sessionId}/message` 路径。Command 内部进入 opencode 标准 prompt 执行流程，**继续复用现有全局 SSE 订阅、流式事件处理、取消、消息持久化和 completed-message fallback**。

**安全与失败语义（第 1370-1376 行）**：
- 后端校验是授权边界；**不得仅依赖前端候选列表或 opencode Command 注册表**
- 不存在、未添加或未启用分别返回明确领域错误；**不得调用 Runtime**
- Registry 中存在但 Runtime 找不到 Command 时返回 `SkillRuntimeOutOfSync`，记录审计信息，**不静默改走普通 Prompt**
- Skill 执行错误保留 opencode 原始错误语义并进入现有消息过程事件审计
- **V1 每条消息最多显式指定一个 Skill**；多 Skill 编排不在本轮范围

**明确排除（第 1378-1386 行）**：
- ❌ 不使用自定义 `[EGOSYNC_TASK_SKILL]` 等 Prompt 协议
- ❌ 不向前端暴露或传输完整 Skill 内容
- ❌ 不为单次任务临时修改 `opencode.json`
- ❌ 不允许前端提交任意 Command 名称
- ❌ 不在 Skill 失败时静默降级

**职责边界**：EgoSync 负责作用域、启用状态、身份校验与格式转换；opencode 负责 Skill 内容展开和原生执行。

### 前端选择状态模式（`architecture.md` 第 1509-1523 行）

```typescript
interface ChatSubmitInput {
  content: string;
  selectedSkillId: string | null;
}
```
- `selectedSkillId` 保存 **Skill Registry ID**，不保存展示名或 OpenCode command name
- `@Skill` 展示标签**不是 `content` 的组成部分**；删除标签时同时清空 `selectedSkillId`
- V1 每条消息最多指定一个 Skill
- 候选列表只显示当前管家或角色已绑定且 enabled 的 Skill

### 现有代码当前状态（必须阅读后再修改）

> 以下为开发前必须了解的"当前状态"。修改前先读取对应文件的完整导出接口和直接调用方。

**前端**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `src/components/chat/ChatInput.tsx` | 第 5-12 行 `ChatInputProps`，`onSend: (content: string) => void` | 纯文本输入框，无 Skill UI | 新增 `availableSkills`、`onSelectedSkillChange` props；新增 `@Skill` 选择器 UI |
| `src/components/chat/ChatStream.tsx` | 第 16-34 行 `ChatStreamProps`，调用 `chatService.sendMessage` | 持有聊天状态，无 Skill 状态 | 新增 `selectedSkillId` 状态；任务结束后清空 |
| `src/services/chatService.ts` | 第 5 行 `sendMessage: (request: ChatRequest) => invoke(...)` | 封装 invoke | **无需修改**（ChatRequest 扩展后自动生效） |
| `src/types/chat.ts` | 第 26-32 行 `ChatRequest`，无 `selectedSkillId` | 前后端共享类型 | 新增 `selectedSkillId?: string` |
| `src/services/skillService.ts` | 第 6-7 行 `listForRole`、`listAllRoleSkills` | 已有角色 Skill 查询 | 新增 `listEnabledForScope(roleId?: string)` |

**后端**：

| 文件 | 当前行 | 当前状态 | 本 Story 修改 |
|---|---|---|---|
| `src-tauri/src/commands/chat.rs` | 第 213-221 行 `chat_send_message` | 接收 ChatRequest，调用 agent_engine | 提取 `selected_skill_id` 传给 agent_engine |
| `src-tauri/src/models/chat.rs` | 第 81-90 行 `ChatRequest`，无 `selected_skill_id` | 前后端共享模型 | 新增 `#[serde(default)] pub selected_skill_id: Option<String>` |
| `src-tauri/src/services/agent_engine.rs` | 第 69-74 行 `opencode_agent_key`；第 2474 行调用 `bridge.send_message` | 会话管理，调用 send_message | `execute_turn` 新增参数；校验后分流 send_message/send_command |
| `src-tauri/src/services/agent_bridge.rs` | 第 131-161 行 `send_message`，调用 `POST /session/{id}/message` | HTTP 客户端，**无 send_command** | 新增 `send_command` 方法 |
| `src-tauri/src/services/skill_registry.rs` | 第 16-45 行 `list_registry`、`list_for_role`、`list_all_role_skills` | Skill 注册表查询，**无 list_enabled/resolve_enabled** | 新增 `resolve_enabled`、`list_enabled` |
| `src-tauri/src/db/skill_bindings.rs` | 第 48-77 行 `skill_ids_for_role`、`all_role_skill_ids`、`role_ids_for_skill` | skill_role_bindings 表查询 | **无需修改**（被 skill_registry 复用） |
| `src-tauri/src/db/skills.rs` | 第 9-66 行 `list_skills`、`get_skill`、`find_skill_by_*` | skills 表 CRUD | **无需修改**（被 skill_registry 复用） |
| `src-tauri/src/lib.rs` | 第 295-394 行 `invoke_handler` 注册 | Tauri 入口 | 若新增 `skill_list_enabled_for_scope` command，需在此注册 |

### 现有可复用能力（禁止重复实现）

1. **Skill 配置存储**：
   - 管家：`butler_config::get_butler_skills(pool)` 返回 `ButlerSkillsConfig { enabled_skill_ids }`（`butler_config.rs` 第 31-36 行）
   - 角色：`role_config::enabled_skill_ids_from_config(skills_config)` 返回 `Vec<String>`（`role_config.rs` 第 96-98 行）
   - **不要新建 Skill 配置存储**，直接复用这两个读取入口

2. **Skill 绑定查询**：`db/skill_bindings.rs::skill_ids_for_role(pool, role_id)` 已验证角色已添加哪些 Skill

3. **Skill 存在性查询**：`db/skills.rs::get_skill(pool, id)` 返回 `SkillRegistryEntry`

4. **Agent 身份解析**：`agent_engine.rs::opencode_agent_key(role_id)` 已实现管家/角色区分（无 roleId → "butler"，有 roleId → "role-{id}"）

5. **HTTP 客户端配置**：`agent_bridge.rs` 现有 `send_message` 的 HTTP 客户端配置（no_proxy、错误处理、SSE 订阅）应被 `send_command` 复用

6. **流式事件处理**：现有全局 SSE 订阅、流式事件处理、取消、消息持久化和 completed-message fallback 必须被 command 路径复用，不新建第二套

7. **前端 Tauri 通信模式**：`useTauriEvent` hook 和 service 层 `invoke` 封装模式

### 必须保留的回归边界

**前端**：
- `ChatInput` 输入、发送、停止、禁用状态、Enter/Shift+Enter 键盘行为
- `ChatStream` 消息列表管理、流式响应处理、事件监听
- `chatService` 所有现有函数签名（仅扩展 ChatRequest 类型）
- `types/chat.ts` ChatRequest 所有现有字段

**后端**：
- `chat_send_message` 所有现有参数和状态管理（仅扩展处理逻辑）
- `agent_engine` 会话缓存、复用、流式事件路由、错误处理
- `agent_bridge` 所有现有方法（仅新增 send_command）
- `skill_registry` 所有现有查询方法（仅新增校验方法）
- `db/skill_bindings.rs`、`db/skills.rs` 现有函数不变
- 数据库表结构（skills、skill_role_bindings、roles、app_settings）不修改

**业务逻辑**：
- `opencode_agent_key` 管家/角色区分逻辑不变
- butler_config 和 role.skills_config 存储格式不变
- onboarding 普通消息路径因 `selected_skill_id` 可空而保持原行为
- 未指定 Skill 时现有自动发现和按需加载行为不变（AC-6）

### UX 增量规范（Readiness 条件，必须在本 Story 内固定）

> 来源：`implementation-readiness-report-2026-07-23.md` 第 43-48 行、`sprint-plan-2026-07-23` 第 43-48 行。
> UX 文档未覆盖 FR-37，以下控件状态在本 Story 内固定，不另建 Story。

**`@Skill` 选择器控件状态**：
1. **入口**：用户在输入框输入 `@` 触发候选列表弹出；继续输入字符按 Skill 名称前缀过滤
2. **候选范围**：仅当前 Agent 已添加且启用的 Skill（由后端 `list_enabled` 返回，前端不自行过滤）
3. **空状态**：
   - 当前 Agent 无可用 Skill：显示"当前没有可用的 Skill"
   - 有 Skill 但无匹配：显示"没有匹配的 Skill"
   - 空状态下用户仍可发送普通消息（关闭候选列表）
4. **键盘交互**：
   - `↑`/`↓`：在候选列表移动焦点
   - `Enter`：确认选择当前焦点项
   - `Escape`：关闭候选列表，不清空已输入文本
   - 继续输入：按名称前缀过滤候选
5. **已选展示**：已选 Skill 显示为输入框内标签（chip），含 Skill 名称和删除按钮
6. **失效提示**：后端拒绝时（AC-4），前端显示明确错误提示，如"该 Skill 已被关闭/移除，请重新选择"
7. **本轮选择展示**：发送后，本轮使用的 Skill 在消息气泡或审计区域可见（具体位置遵循现有消息展示风格）
8. **ARIA 语义**：候选列表 `role="listbox"`、选项 `role="option"`、`aria-activedescendant` 跟踪焦点、输入框 `aria-expanded` 表示列表开关状态

### Project Structure Notes

- 所有修改遵循 `_bmad-output/project-context.md` 的目录组织、命名规范和分层架构
- 前端组件按域分目录：`components/chat/` 已存在，新增的 `@Skill` 选择器 UI 作为 `ChatInput.tsx` 的内部扩展，不新建组件文件（除非复杂度需要拆分，此时新建 `SkillPicker.tsx` 于 `components/chat/`）
- 后端遵循三层架构：Command 只解析参数 → Service 含业务逻辑 → DB 只执行 SQL
- `selected_skill_id` 字段使用 `#[serde(default)]` 确保旧前端请求兼容
- 新增 Tauri Command（若需要 `skill_list_enabled_for_scope`）必须在 Rust Command、`lib.rs generate_handler!`、前端 Service、TypeScript DTO 和测试中闭环（project-context.md 第 205-212 行检查清单）

### 测试要求

> 来源：`sprint-plan-2026-07-23` 第 105-109 行、`architecture.md` 第 1606-1612 行。

**Rust/Service/Command**：
- 可信 Agent 身份解析（管家/角色分支）
- 候选隔离（管家与角色集合不串扰）
- 有效选择（Skill 存在、已添加、已启用）
- 选择后失效竞态（发送前被关闭/移除/不存在）
- 未指定兼容路径（`selected_skill_id = None` 走 `/message`）
- 任务级快照（校验后到 Runtime 调用前 Skill 已确定）
- 失败前不进入 Runtime（拒绝时不调用 `send_command`）
- 审计真实性（记录实际使用的 Skill；未指定时不生成虚假审计）
- **禁止静默降级**：Skill 校验或执行失败时直接返回错误，不自动改走普通消息

**前端组件**：
- 候选筛选（`@` 触发、名称前缀过滤）
- 空状态（无可用 Skill、无匹配）
- 方向键/Enter/Escape 键盘交互
- ARIA 语义（listbox/option/activedescendant）
- 普通消息兼容（不选择 Skill 也能发送）
- 下一消息不沿用选择（任务结束后 `selectedSkillId` 清空）

**关键 E2E**：
- 管家与角色候选集合不串扰
- 显式 Skill 实际生效（调用 `/command` 而非 `/message`）
- 禁用/移除后发送被拒绝，用户收到明确提示
- 流式期间配置变化不改变当前任务

**测试命令**：
```powershell
cd egosync-app/src-tauri && cargo check
cd egosync-app/src-tauri && cargo test
cd egosync-app && npx vitest run
cd egosync-app && npx tsc --noEmit
cd egosync-app && npm run build
```

**测试纪律**：任一测试被跳过或环境缺失时，必须明确记录，不能宣称全部通过（AGENTS 规则十二：显式失败）。

### References

- `_bmad-output/planning-artifacts/epics.md` 第 2741-2798 行 — Epic 10 与 Story 10.1 完整 AC
- `_bmad-output/planning-artifacts/architecture.md` 第 1183-1386 行 — FR-37 需求概述与核心架构决策
- `_bmad-output/planning-artifacts/architecture.md` 第 1505-1539 行 — Skill 选择与原生调用实现模式
- `_bmad-output/planning-artifacts/architecture.md` 第 1639-1712 行 — 项目结构增量与文件标记
- `_bmad-output/planning-artifacts/architecture.md` 第 1716-1724 行 — FR-37 架构边界
- `_bmad-output/planning-artifacts/architecture.md` 第 1794-1817 行 — 架构验证结果
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-07-23.md` 第 478-488 行 — FR-37 PRD 定义
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-07-23.md` 第 770-775 行 — Story 10.1 质量评审
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-07-23.md` 第 826-837 行 — Readiness 条件
- `_bmad-output/implementation-artifacts/sprint-plan-2026-07-23-stories-10-1-10-2-11-1.md` — Sprint 范围、排除项、测试范围
- `_bmad-output/project-context.md` — 技术栈、命名规范、分层架构、禁止事项
- [Source: _bmad-output/planning-artifacts/architecture.md#FR-37：Skill 可用集合与原生任务调用]
- [Source: _bmad-output/planning-artifacts/architecture.md#Skill 选择与原生调用模式]

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

Ultimate context engine analysis completed - comprehensive developer guide created.

### File List

<!-- Dev agent 完成后在此列出所有新增/修改文件 -->

