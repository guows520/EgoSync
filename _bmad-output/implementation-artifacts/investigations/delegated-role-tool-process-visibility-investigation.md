# Investigation: 委派角色任务工具执行过程未展示

## Hand-off Brief

1. **What happened.** 用户确认管家可正常委派、产品经理可实际创建任务，但产品经理界面只显示委派文本与最终回复，没有展示 `create_task` 执行过程。
2. **Where the case stands.** 根因已确认：委派角色走本地收集与直接工具执行分支，只落库角色最终回复，没有为角色 assistant message 持久化任何 process event。
3. **What's needed next.** 在不恢复/污染管家实时流的前提下，为产品经理角色会话持久化任务工具的 running/completed/error 事件。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-15 |
| Status | Fix implemented; pending manual UAT |
| System | Windows x64；EgoSync 0.1.1；OpenCode 1.15.10 |
| Evidence sources | 用户 UAT 对话、运行日志、后端委派/事件代码、前端 ChatStream |

## Problem Statement

管家委派产品经理后，产品经理实际创建了两项任务，产品经理界面却只展示委派输入和最终回复，没有展示任务工具的执行过程。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户 UAT 对话 | Available | 已确认委派文本、最终角色回复可见，任务实际创建 |
| EgoSync 最新日志 | Available | 09:53:55 委派开始，09:54:09 完成；没有过程事件持久化记录 |
| 对话数据库 | Available | 产品经理最新委派消息、assistant 消息的 process event 数均为 0 |
| 后端过程事件链 | Available | 委派分支直接创建任务并只更新最终消息正文 |
| 前端 ChatStream | Available | 已具备按 message id 回拉并展示历史过程事件的能力 |
| CodeGraph | Missing | 当前 MCP transport closed，改用目标文件读取 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 最新日志中是否产生角色 `create_task` process event | High | Closed | 未发现持久化；数据库进一步确认事件数为 0 |
| 2 | 委派角色消息和过程事件写入哪个 conversation/message | High | Closed | 消息正确写入角色会话，但委派分支根本没有写过程事件 |
| 3 | ChatStream 是否仅监听当前管家 stream bucket | High | Closed | 前端支持历史回拉；返回空数组是因为数据库无事件 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-15 | 管家成功委派产品经理 | 用户 UAT | Confirmed |
| 2026-07-15 | 产品经理成功创建两项任务并返回最终文本 | 用户 UAT / 任务列表 | Confirmed |
| 2026-07-15 | 产品经理界面未展示工具执行过程 | 用户 UAT | Confirmed |
| 2026-07-15 09:53:55 | `delegate_to_role` 开始执行，目标角色为产品经理 | `egosync.log` | Confirmed |
| 2026-07-15 09:54:09 | 委派执行完成，返回长度 1220 | `egosync.log` | Confirmed |

## Confirmed Findings

### Finding 1: 缺失的是过程可视化，不是工具执行

**Evidence:** 用户提供的产品经理最终回复明确列出两项已创建任务，且用户确认任务已实际创建。

**Detail:** 委派输入与角色最终回复均进入产品经理界面，因此角色会话本身存在；问题范围缩小到中间工具过程事件。

### Finding 2: 产品经理消息在数据库中没有任何过程事件

**Evidence:** 产品经理角色 `83a90253-e088-45e3-b2a0-178a6d3c73b5` 的最新会话 `efe80ea2-36da-4ffc-9961-6ebe4279074c` 中，委派 user message `673c19c7-4f16-4612-9c2c-d674e9f6c197` 与最终 assistant message `715722b7-cb53-4cbf-ac1e-7a3335b09eef` 的 process event 数均为 0；前一次委派结果同样为 0。

**Detail:** 这排除了“事件已正确落库但产品经理 UI 没渲染”的路径。

### Finding 3: 前端具备展示能力，空展示来自空数据

**Evidence:** `egosync-app/src/components/chat/ChatStream.tsx` 会调用 `chatService.getMessageProcessEvents(message.id)`，并把结果渲染为 `ExecutionTrace`；`chatService.ts` 对应调用 `chat_get_message_process_events`。

**Detail:** 产品经理界面复用同一消息历史组件，不需要另造一套执行过程 UI。

### Finding 4: 委派分支绕过标准过程事件持久化链

**Evidence:** `execute_delegated_task_calls` 直接调用 `tasks::create_task` 并返回 `DelegatedTaskOutcome`；`run_delegated_role_provider` 使用 `collect_local_stream` 收集工具调用；`execute_delegate_to_role` 只插入委派 user message、创建 assistant placeholder，并在结束时更新 assistant content。

**Detail:** 该链路没有调用 `persist_process_event` 或 `persist_and_emit_process_event`。源码注释明确说明本地 drain 不 emit `llm:stream`，目的是让后端结果独立于角色建议并作为权威状态返回管家；但当前实现把“不向管家发实时流”和“不为角色保存过程事件”绑定成了一件事。

## Deduced Conclusions

### Deduction 1: 数据库任务写入链已成功

**Based on:** Finding 1。

**Reasoning:** 两项任务真实存在，说明 `create_task` 工具至少执行到数据库写入；最终角色文本可见，说明委派角色回复也已落库。

**Conclusion:** 优先调查过程事件是否只发给父管家流、没有写入角色消息，或历史回拉时被丢弃。

### Deduction 2: 根因在后端委派执行分支，不在前端

**Based on:** Findings 2–4。

**Reasoning:** 前端按消息回拉过程事件的能力存在，但目标 assistant message 的事件数为 0；同时委派后端链中没有过程事件写入调用。

**Conclusion:** 产品经理界面没有展示执行过程，是因为后端从未生成/持久化可供它展示的记录。

## Hypothesized Paths

### Hypothesis 1: 委派子会话的工具过程没有绑定到产品经理 conversation/message

**Status:** Confirmed

**Theory:** 委派执行在后台子会话完成，工具事件被父会话消费或只形成结构化 tool result，没有转换成产品经理界面的过程事件记录。

**Supporting indicators:** 工具和最终文本成功，唯独中间过程缺失。

**Would confirm:** 日志/源码显示 `create_task` 事件缺少角色 conversation ID 或 assistant message ID。

**Would refute:** 数据库已有正确绑定的角色过程事件，但 ChatStream 未渲染。

**Resolution:** 源码与数据库共同确认：没有错绑到父消息，而是委派分支完全未写 process event。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 无阻断性证据缺口 | — | 根因已由运行数据和源码双重确认 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `agent_engine.rs` 的委派角色本地执行链 |
| Trigger | 管家通过 `delegate_to_role` 委派角色，角色调用 `create_task` |
| Condition | 工具执行成功但角色 UI 无过程事件 |
| Related files | `agent_engine.rs`、`delegate_bridge.rs`、`ChatStream.tsx`、对话消息模型/DB |

## Conclusion

**Confidence:** High

问题不是任务未创建，也不是产品经理 UI 不支持执行过程。根因是委派角色采用 `collect_local_stream → execute_delegated_task_calls → tasks::create_task` 的专用执行链；它有意不向管家发送角色的实时流，但同时也没有把任务调用及结果持久化为产品经理 assistant message 的 process event。产品经理界面随后回拉到空事件列表，因此只显示委派消息和最终回复。

## Recommended Next Steps

### Fix direction

保持现有“角色内部流不发送到管家界面”的原则，不回退已解决的问题；只补齐角色侧事件持久化：

1. 每个 `create_delegated_task` 调用生成角色作用域的 running 事件。
2. 任务数据库写入后，用权威 `DelegatedTaskOutcome` 生成 completed/error 事件。
3. 事件必须绑定 `role_conv.id` 与 `role_assistant_msg.id`，不能绑定管家会话或管家消息。
4. 后台委派默认只需持久化；若支持产品经理界面同时打开时实时展示，也只能向产品经理 conversation id 发事件。
5. 展示名称可映射为用户可理解的“创建任务”，原始参数和结果中保留标题、截止时间、task id、错误/警告。

### Diagnostic

实现时增加针对数据库事件绑定的自动化验证，不需要再加临时诊断日志。

## Reproduction Plan

1. 管家一句话委派两个产品经理任务。
2. 验证任务表新增两项。
3. 验证产品经理 assistant message 下有两组创建过程记录，状态为 completed。
4. 打开产品经理界面，验证“执行过程”展示两次创建及结果。
5. 验证管家界面没有出现产品经理内部 token/tool 流。
6. 补充失败调用、无工具调用及多任务部分成功场景，确认状态与绑定正确。

## Side Findings

- Agent 身份 500 修复已通过本次真实委派与任务创建 UAT。
- 2026-07-15：已在委派角色路径补齐 persistence-only 任务过程事件，绑定角色 conversation/assistant message；自动化测试与编译检查通过，待完整应用人工验证界面展示。
