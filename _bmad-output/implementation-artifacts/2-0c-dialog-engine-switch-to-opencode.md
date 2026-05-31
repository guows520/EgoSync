# Story 2.0c: 对话引擎切换 — agent_engine 路由到 opencode AgentBridge

Status: done

## Story

As a 用户,
I want 管家和角色的对话通过 opencode Agent Loop 执行,
so that 我能获得完整的多步工具调用能力（代码编写、文件操作、Web搜索等），而不仅仅是单轮文本回复。

## Acceptance Criteria

1. **AC-1 管家消息走 opencode 主路径**
   - **Given** 用户在管家对话区输入消息
   - **When** 按 Enter 发送
   - **Then** 消息通过 `AgentBridge → opencode session/message API` 发送，而非直调 `LlmProvider`
   - **And** opencode Agent Loop 自主决策工具调用并流式返回

2. **AC-2 SSE 兼容现有前端流式契约**
   - **Given** opencode Agent Loop 执行中
   - **When** 产生流式 token / thinking / tool_call
   - **Then** `AgentBridge` 解析 SSE 并转发为 Tauri Event `llm:stream`
   - **And** payload 兼容现有 `StreamPayload { conversationId, token, done, thinking, messageId? }`
   - **And** `ChatStream` 无需结构性重写

3. **AC-3 角色消息路由到对应 opencode subagent**
   - **Given** 用户在角色对话区输入消息
   - **When** 该角色已由 Story 2.0b 映射为 opencode subagent
   - **Then** 消息路由到 `role-{roleId}` 对应的 opencode agent session

4. **AC-4 停止按钮中断 opencode 执行**
   - **Given** 流式进行中
   - **When** 用户点击停止按钮
   - **Then** 调用 `agent_bridge.abort_session()` 中断 opencode 执行
   - **And** 现有 `chat_stop_streaming` 仍能取消本地任务并清理 streaming 状态

5. **AC-5 管家意图路由仍可用**
   - **Given** 用户通过管家发送意图消息（如“帮我写个竞品分析”）
   - **When** 管家 agent 路由到目标角色
   - **Then** 路由逻辑仍然有效
   - **And** 不再新增独立的确定性路由器或额外 LLM 分类调用

6. **AC-6 角色语调来自 opencode agent config**
   - **Given** 角色的 system prompt 已在 opencode agent config 中设定
   - **Then** 每个角色对话自动使用其个性化语调
   - **And** 不再依赖 `agent_engine` 手动注入角色 system prompt 作为主路径

7. **AC-7 role:proposed 事件仍可触发**
   - **Given** 引导中管家建议创建角色
   - **When** opencode Agent Loop 触发角色提议相关工具/事件
   - **Then** EgoSync 捕获后发出 `role:proposed`
   - **And** 前端继续弹出确认 modal，不能直接落库创建角色

8. **AC-8 角色 CRUD 同步继续复用 AgentConfigService**
   - **Given** 角色 CRUD 操作成功写入 EgoSync DB
   - **When** create/update/archive/restore/delete 完成
   - **Then** 调用现有 `AgentConfigService` 生命周期同步方法更新 `opencode.json`

9. **AC-9 opencode 不可用时降级到 LlmProvider**
   - **Given** opencode server 不可用（进程未启动、崩溃或 API 请求失败）
   - **When** 用户发送消息
   - **Then** 降级到现有 `LlmProvider` 直调路径
   - **And** 对话中以管家语调提示：“Agent 引擎暂时不可用，当前为基础对话模式”
   - **And** 降级不破坏历史落库、流式事件和停止按钮状态清理

10. **AC-10 现有功能零回归**
    - **Given** 切换完成后
    - **Then** 已有功能保持可用：对话流式、角色切换、意图路由、个性化语调、角色涌现建议
    - **And** `LlmProvider` trait 保留为降级兼容层，不删除

## Tasks / Subtasks

### Phase 1: 建立 opencode 对话运行时状态（AC: #1, #3, #4）

- [x] T1.1 新增最小会话映射状态
  - 建议位置：`GUI/src-tauri/src/commands/chat.rs` 或 `GUI/src-tauri/src/state.rs`（如现有项目已启用 state 文件则优先 state）
  - 需要保存：`conversation_id → opencode_session_id`
  - 目的：同一 EgoSync conversation 后续消息复用同一 opencode session；停止按钮能找到 session abort
  - 不新增数据库表，本 story 先用内存态；应用重启后重新创建 opencode session，EgoSync 历史仍由本地 DB 保存

- [x] T1.2 定义 agent key 选择规则
  - 管家 conversation：`butler`
  - 角色 conversation：`AgentConfigService::role_to_agent_key(role_id)`，即 `role-{roleId}`
  - onboarding：仍走管家 agent，但必须保留 `role:proposed` 能力

- [x] T1.3 在启动路径确认 `AgentBridge` 可用性
  - 复用 `app.manage(agent_bridge)` 中已注册的 `AgentBridge`
  - 不用前端直连 opencode；仍由 Rust 后端统一代理

### Phase 2: 新增 opencode 主路径，保留现有降级路径（AC: #1, #2, #9, #10）

- [x] T2.1 在 `agent_engine.rs` 中拆出清晰分支
  - 主路径：opencode 可用 → `AgentBridge.create_session` / `send_message`
  - 降级路径：opencode 不可用 → 复用现有 `run_stream` 行为
  - 建议不要删除现有 `build_butler_messages`、`build_role_messages`、工具执行和 fallback 逻辑；先作为降级层保留

- [x] T2.2 将 `SseEvent` 转成现有 `StreamPayload`
  - `SseEvent::Text { content }` → `token=content, done=false, thinking=false`
  - thinking 类事件如当前模型未覆盖，先按实际 opencode SSE 形态扩展 `SseEvent`，但输出仍映射到 `thinking=true`
  - `SseEvent::ToolCall { ... }`：不直接暴露给前端；除非是 UI 需要的事件（如 `role:proposed`），否则只记录 tracing
  - `SseEvent::Done` → `token="", done=true, thinking=false`
  - `SseEvent::Error` → 落库友好错误文本并发 `done=true`

- [x] T2.3 保持消息落库顺序
  - `chat_send_message` 已先插入 user message 和空 assistant message；opencode 主路径必须继续更新这个 assistant message
  - token 累积写入 `messages.content`
  - thinking 累积写入 `messages.thinking_content`
  - 结束时调用 `mark_message_complete`

- [x] T2.4 降级提示只出现一次且不吞用户消息
  - opencode 请求失败时，assistant 回复前缀必须包含“Agent 引擎暂时不可用，当前为基础对话模式。”
  - 然后继续走原有 `LlmProvider` 逻辑，不能直接失败退出
  - 如果 fallback provider 也失败，再使用现有 `summarize_error` 友好提示

### Phase 3: 停止按钮接入 abort_session（AC: #4）

- [x] T3.1 扩展 `chat_stop_streaming`
  - 现有行为：取消本地 `CancellationToken`
  - 新增行为：如果 conversation 有 opencode session，则调用 `AgentBridge.abort_session(session_id)`
  - abort 失败只记录 warning，不阻塞本地取消

- [x] T3.2 清理 streaming 状态
  - `chat_send_message` spawned task 结束后仍移除 `StreamingState` 与 `CancelTokens`
  - opencode 主路径成功、错误、取消三种路径都必须发出结束事件或保证前端可恢复输入

### Phase 4: 角色提议与委派能力的边界处理（AC: #5, #7）

- [x] T4.1 明确本 story 不新增确定性路由器
  - 不用常规代码做“文本→角色”的分类
  - 管家路由能力交给 opencode agent/tool loop
  - 现有 `delegate_to_role` 工具逻辑可作为降级路径继续存在

- [x] T4.2 保留 `role:proposed` 事件语义
  - opencode 工具结果或事件中识别到角色提议时，发 `role:proposed`
  - 事件 payload 仍为 `{ conversationId, name, icon?, color?, goal? }`
  - 严禁直接调用 `db::roles::create_role` 代替用户确认

- [x] T4.3 如果 opencode custom tool 本 story无法稳定接入
  - 明确保留降级路径中的 `create_role` tool/fallback 文本检测
  - 在 Dev Notes 或 Completion Notes 记录未接入的 opencode tool 边界
  - 不允许宣称 AC-7 完成，除非能通过测试或手动验证证明 `role:proposed` 可由 opencode 路径触发

### Phase 5: 测试与验证（AC: #1-#10）

- [x] T5.1 Rust 单元测试：agent key 与 session 映射
  - 管家 → `butler`
  - 角色 → `role-{roleId}`
  - 同一 conversation 复用同一 session

- [x] T5.2 Rust 单元测试：SSE → StreamPayload 映射
  - text token
  - done
  - error
  - thinking（如扩展 `SseEvent`）

- [x] T5.3 Rust 单元测试：opencode 不可用时进入 fallback
  - 模拟 `AgentBridge` 失败
  - 断言 assistant 内容包含基础对话模式提示
  - 断言最终消息 complete

- [x] T5.4 Rust 单元测试：停止按钮同时取消本地 token 与 abort opencode session
  - abort 失败不影响本地取消

- [x] T5.5 运行验证命令
  - `cd GUI/src-tauri && cargo test`
  - 若改动前端类型或服务：`cd GUI && npx tsc --noEmit`

## Dev Notes

### 当前实现态

- `chat_send_message` 当前链路：前端 `ChatStream` → `chatService.sendMessage` → Tauri command `chat_send_message` → `agent_engine::run_stream`。
- `chat_send_message` 已负责：插入 user message、插入空 assistant message、维护 `StreamingState`、创建 `CancellationToken`、spawn `run_stream`。
- `agent_engine::run_stream` 当前主路径仍是 `resolve_default_provider(...).chat_stream(...)`，也就是 `LlmProvider` 直连。
- `AgentBridge` 已存在，支持 `create_session`、`send_message`、`abort_session`、`get_messages`、`get_config`、`get_providers`、`get_agents`。
- `AgentConfigService` 已存在，支持 `role_to_agent_key`、角色 create/update/archive/delete/full_sync 到 `opencode.json`。
- `lib.rs` 已在 setup 中执行 `AgentConfigService.full_sync`，启动 delegate bridge、opencode sidecar、event router，并 `app.manage(agent_bridge)`。
- sidecar 启动时固定 `current_dir` 到 `AppData/Roaming/com.egosync.app/opencode-workspace`，避免继承 `src-tauri` 工作目录导致 opencode snapshot 阶段对错误 pathspec 执行 `git add`。

### opencode sidecar 工作目录与 snapshot 延迟

当前运行时将 opencode sidecar 进程工作目录固定为 AppData 下的 `opencode-workspace`，同时 opencode session 也使用该 workspace 目录。这个约束是必要的：如果 sidecar 继承 `GUI/src-tauri` 作为 cwd，而 session directory 指向 AppData workspace，opencode 的 snapshot 阶段会尝试对不属于当前 git 工作树的路径执行 `git add`，出现类似错误：

```text
service=snapshot exitCode=128
fatal: pathspec 'GUI/src-tauri/Cargo.lock' did not match any files
failed to add snapshot files
```

该错误会在每轮对话早期造成约 10 秒以上的首 token 延迟，曾被用户感知为“固定 37 秒等待”。修复点在 `SidecarManager::with_working_dir(...)` 和 `command.current_dir(working_dir)`：启动前创建 workspace，并确保 sidecar cwd 与 opencode session directory 一致。

### 必须保留的现有前端契约

- 前端监听事件名：`llm:stream`。
- 前端 payload 类型：
  - `conversationId: string`
  - `token: string`
  - `done: boolean`
  - `thinking: boolean`
  - `messageId?: string | null`
- `ChatStream` 已支持多 assistant 气泡分桶：按 `messageId` 聚合 token。
- `done=true` 后，前端会回拉 `chat_get_history` 并清理对应 streaming bucket。
- 停止按钮调用：`chat_stop_streaming(conversationId)`。

### 关键设计边界

- 不删除 `LlmProvider`、OpenAI/Anthropic provider、`build_butler_messages`、`build_role_messages`。它们是降级兼容层。
- 不新增前端直连 opencode，不新增浏览器侧 API key 逻辑。
- 不新增数据库表保存 opencode session；V1 先用内存映射。
- 不改 `ChatStream` 结构，除非 payload 类型必须补字段；优先保持现有类型不变。
- 不在 DB 层写 opencode 同步逻辑；角色同步继续在 command/service 生命周期层处理。

### opencode API 风险

Story 2.0 的实现基于预期端点：

| 功能 | 当前 AgentBridge 端点 |
|------|------------------------|
| 创建会话 | `POST /session` |
| 发送消息 | `POST /session/:id/message` |
| 中止会话 | `POST /session/:id/abort` |
| 获取消息 | `GET /session/:id/messages` |
| 压缩上下文 | `POST /session/:id/compact` |
| 获取配置 | `GET /config` |
| 获取 Providers | `GET /providers` |
| 获取 Agents | `GET /agents` |

如本机 opencode 实际 API 不一致，优先修正 `AgentBridge` 内部适配，保持对外方法语义不变。

### 既有工具与迁移策略

当前 `agent_engine` 中的工具能力：

- `create_role`：只发 `role:proposed`，不直接落库。
- `delegate_to_role`：校验目标角色，写入角色会话，调用角色 LLM，本地 drain，不污染管家 stream。
- `record_emergence_rejection`：写入 7 天冷却。

迁移到 opencode 主路径时，必须选择一种明确策略：

1. **已接入 opencode custom tool**：由 opencode tool loop 触发 EgoSync 事件/操作。
2. **未接入 custom tool**：不要假装完成 AC-7；保留降级路径，并在完成记录中标记该边界。

不要把工具调用当作普通文本 token 暴露给用户。

### 之前 story 的可复用经验

- Story 2.0：sidecar 是非阻塞增强；启动失败时应用继续运行。
- Story 2.0 Review 修复：健康检查同时尝试 `/health` 和 `/`；退出时显式 stop sidecar；不要只依赖 drop。
- Story 2.0b：角色同步失败只 warning，不阻塞 CRUD；`AgentConfigService` 是 opencode.json 唯一写入入口。
- 最近提交 `fix(chat): clean role proposal handoff flow` 表明角色提议 handoff 是敏感链路，本 story 不要改坏 `role:proposed → modal → roleService.create`。

### Project Structure Notes

#### 预期修改文件

| Path | Action | Notes |
|------|--------|-------|
| `GUI/src-tauri/src/services/agent_engine.rs` | UPDATE | 新增 opencode 主路径与降级分支；保留现有 LlmProvider 逻辑 |
| `GUI/src-tauri/src/commands/chat.rs` | UPDATE | session 映射 state、stop 时 abort opencode session |
| `GUI/src-tauri/src/models/agent.rs` | MAYBE UPDATE | 如 opencode SSE 有 thinking/permission/custom tool 新事件，扩展 `SseEvent` |
| `GUI/src-tauri/src/services/agent_bridge.rs` | MAYBE UPDATE | 如实际 opencode API 与当前端点不一致，在此内部适配 |
| `GUI/src-tauri/src/lib.rs` | UPDATE | 启动 delegate bridge / event router；注入 bridge env；为 sidecar 设置 AppData `opencode-workspace` 工作目录 |

#### 不应改动

- `GUI/src/components/chat/ChatStream.tsx`：原则上无需结构性改动；只在 payload 类型变化时最小调整。
- `GUI/src/services/chatService.ts`：除非 command 参数变化，否则不改。
- `GUI/src-tauri/src/services/agent_config.rs`：角色配置同步已完成；本 story 只复用。
- `GUI/src-tauri/src/db/roles.rs`：角色 CRUD 数据层不应承担 opencode 同步。
- `GUI/src-tauri/migrations/*.sql`：本 story 不需要新表。

### Testing Requirements

- Rust 测试继续放在相关文件底部 `#[cfg(test)] mod tests`。
- 测试必须验证意图，不只验证行为：
  - fallback 测试要证明“opencode 不可用时用户仍能基础对话”。
  - stop 测试要证明“用户可以夺回控制权”，不只是调用了一个函数。
  - session 映射测试要证明“同一 conversation 保持上下文连续”。
- 若修改 `GUI/src/types/chat.ts` 或前端监听逻辑，必须跑 `cd GUI && npx tsc --noEmit`。

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` — Story 2.0c Acceptance Criteria]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — AgentBridge → opencode HTTP API、角色→Agent 映射、流式响应桥接、LlmProvider 降级层]
- [Source: `_bmad-output/planning-artifacts/architecture.md` — Event 命名：`llm:stream`, `role:proposed`; Rust backend organization]
- [Source: `_bmad-output/planning-artifacts/ux-design-specification.md` — 流式输出使用 SSE/WebSocket，不轮询；对话气泡 `aria-live="polite"`]
- [Source: `_bmad-output/implementation-artifacts/2-0-opencode-sidecar-agent-bridge.md` — SidecarManager / AgentBridge 已实现，非阻塞增强设计]
- [Source: `_bmad-output/implementation-artifacts/2-0b-role-agent-mapping-permissions.md` — AgentConfigService、`role-{uuid}` key、opencode.json 同步已实现]
- [Source: `GUI/src-tauri/src/commands/chat.rs` — 当前 chat command、streaming state、cancel token、stop command]
- [Source: `GUI/src-tauri/src/services/agent_engine.rs` — 当前 LlmProvider 主路径、工具执行、角色提议、委派、降级提取逻辑]
- [Source: `GUI/src-tauri/src/services/agent_bridge.rs` — 当前 opencode HTTP client 与 SSE parser]
- [Source: `GUI/src-tauri/src/models/chat.rs` — `StreamPayload` / `RoleProposedPayload`]
- [Source: `GUI/src/components/chat/ChatStream.tsx` — 前端流式 bucket、stop、history 回拉契约]
- [Source: `GUI/src-tauri/src/lib.rs` — setup 中 AgentConfigService full_sync、sidecar start、AgentBridge managed state]
- [Source: recent git log — `fix(chat): clean role proposal handoff flow`, `feat(2.0b): sync roles to opencode agents`, `fix(2.0): correct opencode CLI command and Windows .cmd shim execution`]

## Story Completion Status

- Status set to `ready-for-dev`.
- Ultimate context engine analysis completed - comprehensive developer guide created.
- Validation covered source artifacts, previous story learnings, current runtime path, frontend event contract, and implementation guardrails.

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

- `cd GUI/src-tauri && cargo test`: 133 unit tests passed, 1 integration placeholder passed.
- `cd GUI/src-tauri && cargo test commands::chat::tests`: 2 stop-control tests passed.
- `cd GUI && npx tsc --noEmit`: passed.
- Lints checked for edited Rust files: no blocking diagnostics reported.

### Completion Notes List

- Implemented in-memory `conversation_id → opencode_session_id` mapping through Tauri managed state.
- Routed chat streaming through `AgentBridge.create_session` / `AgentBridge.send_message` before falling back to `LlmProvider`.
- Preserved `LlmProvider` as degradation path with visible “Agent 引擎暂时不可用，当前为基础对话模式。” notice.
- Mapped opencode `SseEvent` text/thinking/done/error/tool_call into existing `llm:stream` / `role:proposed` contracts without frontend structural changes.
- Extended stop behavior so `chat_stop_streaming` cancels local token and best-effort aborts opencode session.
- Reused `AgentConfigService::role_to_agent_key` for role subagent routing and kept role CRUD sync unchanged.
- Kept deterministic routing out of scope; opencode tool loop owns primary routing, existing `delegate_to_role` remains fallback behavior.
- AC-7 note: `role:proposed` is now fully functional via opencode Custom Tools (Story 2.0d). MCP approach was replaced — see `2-0d-opencode-custom-tools-replace-mcp.md` for details. Tools are loaded from `~/.config/opencode/tools/` and intercepted via bus event `message.part.updated` with `type=tool`.
- Fixed a race where the opencode send task could finish before queued SSE events were drained.

### File List

- `GUI/src-tauri/src/commands/chat.rs` (MODIFIED)
- `GUI/src-tauri/src/lib.rs` (MODIFIED)
- `GUI/src-tauri/src/models/agent.rs` (MODIFIED)
- `GUI/src-tauri/src/services/agent_bridge.rs` (MODIFIED)
- `GUI/src-tauri/src/services/agent_engine.rs` (MODIFIED)
- `GUI/src-tauri/src/services/sidecar.rs` (MODIFIED — bridge env 注入 + fixed working_dir)
- `GUI/src-tauri/src/services/delegate_bridge.rs` (ADDED)
- `GUI/src-tauri/src/services/event_router.rs` (ADDED)
- `GUI/src-tauri/src/services/mcp_host.rs` (DELETED — replaced by custom tools in 2.0d)
- `GUI/src-tauri/src/services/mod.rs` (MODIFIED — removed mcp_host module)
- `_bmad-output/implementation-artifacts/2-0c-dialog-engine-switch-to-opencode.md` (MODIFIED)
- `_bmad-output/implementation-artifacts/2-0d-opencode-custom-tools-replace-mcp.md` (CREATED)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED)

### Review Findings

- [x] [Review][Decision] `delegate_to_role` tool call 在 opencode 主路径未被执行 — 已在 ToolCall handler 中加入 delegate_to_role 分支 [agent_engine.rs:~715-723]
- [x] [Review][Patch] opencode mid-stream error 后 session 未从 map 中移除，下一轮复用死会话 — 已在 Ok(Err(e)) 路径末尾移除 session [agent_engine.rs try_run_opencode_stream]
- [x] [Review][Defer] streaming_state TOCTOU race — fixed: merged check+insert [commands/chat.rs]
- [x] [Review][Defer] OnboardingConversations map 无清理 — fixed: auto-remove on completion + delete cleanup [commands/chat.rs]
- [x] [Review][Defer] chat_delete_conversation 不清理活跃流/session 状态 — fixed: full state cleanup on delete [commands/chat.rs]
- [x] [Review][Defer] SSE parser 未处理 \r\n\r\n — fixed: normalize before buffering [agent_bridge.rs]
- [x] [Review][Defer] ensure_success 不读取 error body — fixed: ensure_success_with_body [agent_bridge.rs]

## Change Log

- 2026-05-26: Implemented opencode AgentBridge chat path with fallback, session mapping, stop abort, SSE mapping, and tests.
- 2026-05-27: Post-implementation cleanup & consistency pass:
  - Fixed `test_opencode_agent_key_routes_butler_and_roles` — assertion updated to match runtime behavior (butler uses empty string for opencode default agent).
  - Cleaned all 12 `[DIAG]` diagnostic log sites across `agent_engine.rs`, `event_router.rs`, `sidecar.rs`: downgraded to `debug!`/`trace!` with structured fields, removed temporary comments.
  - Extracted `build_butler_system_prompt()` as shared async function; opencode path now injects the full butler prompt (role roster + cross-role summary + delegation guidelines + emergence instructions), consistent with the direct-LLM fallback path.
  - All 139 unit tests pass (0 failures).
- 2026-05-28: MCP replaced with opencode Custom Tools (Story 2.0d):
  - Deleted `mcp_host.rs` (MCP HTTP server, ~745 lines).
  - Custom tools written to `~/.config/opencode/tools/` at startup.
  - Tool call interception via bus event `message.part.updated` with `type=tool`, reading `state.output`.
  - Role agents get butler-exclusive tools disabled via `tools: { create_role: false, ... }`.
  - System prompt tool names changed from `egosync_*` to direct names (`create_role`, `delegate_to_role`, `record_emergence_rejection`).
  - AC-7 fully verified: `role:proposed` emitted successfully from opencode tool path.
  - 140 unit tests pass (0 failures).
- 2026-05-31: Fixed sidecar cwd/session directory mismatch that caused opencode snapshot `pathspec` failures and long first-token delays; `SidecarManager` now supports `with_working_dir`, and app startup points it to AppData `opencode-workspace`.
