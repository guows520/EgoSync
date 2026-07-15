---
title: '委派角色自动创建跟踪任务'
type: 'bugfix'
created: '2026-07-15'
status: 'done'
baseline_commit: 'e953b9e8e7a83edaa623fe46c7c9577b09ba5028'
context:
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 管家能把明确属于某角色的事项委派给该角色，但当前委派执行只生成文本回复，角色没有工具能力把明确日程、承诺或交付物创建为自身任务，导致用户看到“已跟进”却没有任务记录。

**Approach:** 保持“管家识别归属并委派、目标角色决定如何处理”的边界，在委派角色的本地 LLM 执行中开放一个绑定当前角色的任务创建工具，执行工具调用并将真实结果交回角色和管家后再生成最终回复。

## Boundaries & Constraints

**Always:** 任务必须归属当前被委派角色，模型不得提供或覆盖 `role_id`；只有包含明确行动、未来日程、承诺、待办或交付物的委派才创建任务；纯咨询、分析和建议请求不得创建；一句话中的多个独立行动项允许分别创建；截止时间始终可选，格式异常时先尝试确定性纠正，仍无法恢复则去掉截止时间但继续创建任务；只有数据库写入成功后才能向用户声称对应任务已创建；沿用现有任务创建与后台自动分类逻辑。

**Ask First:** 若实现需要改变 `delegate_to_role` 的公开参数、数据库 schema，或把委派角色执行迁移到 opencode 子会话，必须暂停确认。

**Never:** 不让管家代替角色创建本次任务；不让所有委派无条件创建任务；不新增猜测性的 NLP 路由器；不重构无关的管家工具链、任务 UI 或分类器。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 明确日程 | `[管家委派] 明天下午3点开会讨论产品方案`，目标为产品经理 | 产品经理先创建自身任务，再返回会议处理建议；管家可确认真实创建结果 | 创建失败时明确返回失败，不声称已创建 |
| 纯咨询 | `[管家委派] 帮我分析产品方案应该怎么讲` | 产品经理只返回分析建议，不创建任务 | N/A |
| 标题非法 | 一个创建调用的标题为空或仅含空白 | 该调用不写入数据库，其他合法调用继续执行 | 返回可理解错误，角色不得声称该任务已创建 |
| 截止时间异常 | 标题合法，但截止时间格式无法直接使用 | 先规范化可识别格式；仍无法恢复则以无截止时间创建任务 | 返回创建成功及“截止时间未保存”的警告 |
| 多个独立任务 | 一句话包含两个或更多不同的行动项，模型产生多个创建调用 | 每个规范化后不同的合法任务分别创建 | 单项失败不阻断其他任务，并逐项返回结果 |
| 重复工具调用 | 同一角色回合产生规范化标题与截止时间均相同的调用 | 只执行第一条，跳过完全重复项 | 返回 `duplicate_skipped` 及首个任务 ID |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 构造角色提示词、执行委派角色 LLM、接收工具调用、写入任务并完成工具结果回合；单元测试也位于此文件。
- `egosync-app/src-tauri/src/db/tasks.rs` -- 现有任务创建入口与所有权/标题校验，必须复用。
- `egosync-app/src-tauri/src/models/task.rs` -- 构造绑定当前角色的 `CreateTaskInput`。
- `egosync-app/src-tauri/src/services/task_classifier.rs` -- 创建成功后的既有异步分类机制；仅复用，不改变分类策略。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 为委派角色增加“创建自身任务”的工具定义和委派专用行为规则，让模型能区分需要跟踪的事项与纯咨询。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 将委派执行从纯文本 drain 扩展为单次工具执行循环：捕获全部任务调用、强制绑定当前角色、规范化或降级截止时间、逐项调用现有 DB 创建入口、跳过完全重复项、回传逐项结果，再以禁用工具的收尾回合生成回复。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 添加以业务意图为中心的测试，覆盖明确日程建任务、纯咨询不被规则要求建任务、角色 ID 不可由模型指定、无截止时间创建、异常时间纠正/降级、单句多任务、完全重复调用去重，以及创建失败不伪报成功。

**Acceptance Criteria:**
- Given 管家把含明确时间的产品事项委派给产品经理, when 产品经理处理委派, then 产品经理任务列表新增归属正确的任务，返回链包含真实创建成功信息。
- Given 管家委派的是产品方案分析咨询, when 产品经理处理委派, then 不产生新任务且正常返回建议。
- Given 一次委派包含两个不同的明确行动项, when 产品经理处理委派, then 两个任务均归属产品经理并分别返回创建结果。
- Given 任务标题合法但截止时间无法恢复, when 产品经理创建任务, then 任务以无截止时间成功保存，并在工具结果中明确时间降级。
- Given 任务创建失败, when 产品经理生成最终回复, then 回复不得包含任务已创建的确定表述，并保留可诊断错误。
- Given 角色模型尝试指定其他角色, when 工具执行, then 后端仍只使用当前委派角色 ID。

## Spec Change Log

- 2026-07-15：完成委派角色任务工具、角色绑定、多任务/去重、截止时间纠正与降级、工具结果收尾及业务测试。
- 2026-07-15：根据代码审查补强事件队列 drain、逐调用权威结果、失败调用去重、不可解析时间去重身份及 provider 双回合测试。

## Design Notes

委派路径当前直接调用默认 Provider，而不是 opencode 角色 Agent，因此不能依赖全局 `create_task.ts`。最小方案是在既有 Provider 工具协议内提供角色限定工具。工具参数只包含 `title` 与可选 `deadline`，角色归属由 `execute_delegate_to_role` 的已校验 `role.id` 注入。首轮允许返回多个工具调用；后端逐项执行，并以规范化后的标题与截止时间作为回合内去重键。截止时间先按受支持格式确定性解析并规范化，无法恢复时设置为 `None`，不影响标题合法任务的创建。执行后追加 assistant tool-call 与逐项 tool result 消息，第二轮 `tools=None`，避免嵌套调用。任务创建成功后继续触发既有异步分类；分类失败保留默认 Q2，不把任务创建判为失败。

## Verification

**Commands:**
- `cargo test services::agent_engine --lib`（在 `egosync-app/src-tauri`）-- 委派与任务工具相关单元测试全部通过。
- `cargo check`（在 `egosync-app/src-tauri`）-- Rust 编译检查通过，无新增 warning/error。

**Manual checks:**
- 完整 Tauri 构建中从管家输入“明天下午3点要开会讨论产品方案”，确认发生委派、产品经理任务列表出现新任务，且回复只在真实写入成功后确认创建。
- 新对话输入“帮我分析产品方案应该怎么讲”，确认发生委派但任务数量不变。

## Suggested Review Order

**委派入口与责任边界**

- 管家委派后由目标角色运行工具，并把权威结果返回管家。
  [`agent_engine.rs:4011`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4011)

**任务工具与数据真实性**

- 工具隐藏角色 ID，后端固定绑定当前被委派角色。
  [`agent_engine.rs:3813`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3813)

- 截止时间可纠正或降级，原始异常时间仍参与精确去重。
  [`agent_engine.rs:3829`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3829)

- 多任务逐项写入，重复及失败结果保持可诊断。
  [`agent_engine.rs:3867`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3867)

**Provider 双回合协议**

- 完成后继续排空事件，避免丢失工具调用和尾部文本。
  [`agent_engine.rs:3928`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3928)

- 角色建议与后端权威任务状态分离后统一返回。
  [`agent_engine.rs:3976`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3976)

**业务测试**

- 验证多任务、角色绑定、异常时间降级与完全重复去重。
  [`agent_engine.rs:4563`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4563)

- 验证 Provider 完成后仍排空工具调用及收尾 Token。
  [`agent_engine.rs:4601`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4601)
