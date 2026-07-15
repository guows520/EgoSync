---
title: '持久化委派角色的任务执行过程'
type: 'bugfix'
created: '2026-07-15'
status: 'done'
baseline_commit: 'd75a9f11bfd07644ac49c99f47c907165f6c3a50'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/delegated-role-tool-process-visibility-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 管家委派产品经理后，角色可以真实创建任务并保存最终回复，但委派专用执行链没有为角色 assistant 消息持久化工具过程事件，导致产品经理界面无法展示“执行过程”。

**Approach:** 保持角色内部 Provider 流不发送到管家界面的既有隔离，只把每个委派任务调用的权威结果持久化为角色会话下的工具过程事件，让现有 `ChatStream` 历史回拉直接展示。

## Boundaries & Constraints

**Always:** 每个任务工具调用对应一条独立过程事件；事件绑定当前 `role_conv.id` 和 `role_assistant_msg.id`；展示状态来源于 `DelegatedTaskOutcome`，成功、失败和重复跳过均不得伪造；多任务保持原顺序；纯咨询不产生任务过程事件；过程事件写入失败不得回滚已经成功创建的任务，但必须显式记录后端警告。

**Ask First:** 若实现需要修改数据库 schema、前端过程事件协议、`delegate_to_role` 公开参数，或向管家会话发送角色内部过程流，必须暂停确认。

**Never:** 不回退已验证的委派与任务创建逻辑；不把事件绑定到管家 conversation/message；不恢复委派角色 token/tool 的 `llm:stream` 广播；不修改任务分类、截止时间降级、多任务和去重语义；不为该问题新增前端专用组件。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 单任务成功 | 产品经理调用一次任务工具且数据库写入成功 | 产品经理 assistant 消息下保存一条 `tool/create_task/completed` 过程事件，原始数据包含调用参数和 task id | N/A |
| 多任务成功 | 同一委派产生两个不同任务调用 | 按调用顺序保存两条独立 completed 事件，界面展示两次创建过程 | N/A |
| 非法或创建失败 | 标题为空、角色无效或数据库创建失败 | 保存 error 事件，摘要与原始结果包含可理解错误，不声称创建成功 | 其他调用继续处理并各自落事件 |
| 完全重复调用 | 后续调用被判定为 `duplicate_skipped` | 保存一条可区分的 skipped 结果，不产生第二个任务 | 保留首次结果关联信息 |
| 纯咨询 | Provider 未产生任务工具调用 | 不写任务过程事件，最终建议照常保存 | N/A |
| 过程事件落库失败 | 任务已成功创建但 conversations DB 写入失败 | 任务和最终回复仍保留，后端输出包含会话/消息上下文的 warning | 不把任务改判为失败 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 委派角色 Provider、任务权威结果、标准过程事件持久化帮助函数及相关单元测试。
- `egosync-app/src-tauri/src/db/conversations.rs` -- `message_process_events` 的既有写入与按消息读取接口；复用而不改 schema。
- `egosync-app/src/components/chat/ChatStream.tsx` -- 已按 assistant message id 回拉并渲染过程事件；仅作为兼容契约，不修改。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 将 `DelegatedTaskOutcome` 确定性转换为用户可理解的任务工具过程候选，保留调用参数、task id、warning/error 和重复状态。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 在角色 assistant 消息创建后、委派结果产生时，将每项结果按顺序持久化到角色 conversation/message；保持 persistence-only，不调用 emit。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 添加业务意图测试，覆盖多任务绑定、成功/失败/重复状态、纯咨询零事件以及不依赖管家流。

**Acceptance Criteria:**
- Given 管家委派产品经理创建两个不同任务, when 两个任务均成功, then 产品经理最终 assistant 消息下存在两条按序 completed 工具事件，且每条关联正确标题与 task id。
- Given 同一批调用含失败或完全重复项, when 委派执行结束, then 每项结果均以真实状态保存且其他合法任务不受阻断。
- Given 委派是纯咨询, when 产品经理没有调用任务工具, then 不产生任务过程事件且最终回复正常完成。
- Given 委派执行完成, when 检查事件归属, then 事件只绑定产品经理会话与消息，不写入管家消息，也不向管家实时流广播。

## Spec Change Log

## Design Notes

委派 Provider 当前在角色 assistant placeholder 创建之后运行，因此最小改法是在 `execute_delegate_to_role` 获得 `run.outcomes` 后调用 persistence-only 帮助函数。使用现有 `persist_process_event`，并为本地委派生成稳定、仅用于审计的 session 标识；工具显示名采用前端可识别的 `create_task`，而 `raw_json` 同时保留原始调用名 `create_delegated_task` 和权威 outcome。单次调用保存一个终态事件即可，避免后台执行已完成后补写虚假的实时 running 动画。

## Verification

**Commands:**
- `cargo test services::agent_engine --lib`（在 `egosync-app/src-tauri`）-- 委派任务与过程事件测试全部通过。
- `cargo check`（在 `egosync-app/src-tauri`）-- Rust 编译检查通过。

**Manual checks:**
- 完整应用中从管家委派一句话里的两个产品经理任务；产品经理界面应展示两项“创建任务”过程，管家界面仍只展示委派结果。

## Suggested Review Order

**角色事件持久化入口**

- 委派完成后只向角色消息落事件，并保持管家流隔离。
  [`agent_engine.rs:4067`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4067)

- 按调用顺序持久化每个权威任务结果。
  [`agent_engine.rs:3904`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3904)

**状态与展示身份**

- 将成功、失败、重复结果转换为可展示终态。
  [`agent_engine.rs:3867`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3867)

- 调用序号与 call ID 防止相邻任务被前端合并。
  [`agent_engine.rs:3867`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3867)

**业务回归测试**

- 验证多任务顺序、角色归属及管家消息零事件。
  [`agent_engine.rs:4721`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4721)

- 验证失败后的重复调用不会掩盖错误。
  [`agent_engine.rs:4761`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4761)

- 验证纯咨询不会制造任务过程。
  [`agent_engine.rs:4781`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4781)
