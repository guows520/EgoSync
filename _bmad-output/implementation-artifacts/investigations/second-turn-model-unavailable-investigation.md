# Investigation: 第二轮对话模型服务不可用

## Hand-off Brief

1. **What happened.** 用户报告首轮对话正常，后续输入“产品经理”持续收到模型服务暂时不可用。
2. **Where the case stands.** Active；正在定位错误文案来源、调用链及原始异常。
3. **What's needed next.** 对照模型调用、会话状态与角色路由代码，确认失败条件。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-28 |
| Status | Active |
| System | Windows / 本地项目 |
| Evidence sources | 用户复现记录、源代码、配置、Git 历史 |

## Problem Statement

用户报告第一次对话正常；从输入“产品经理”开始，后续请求持续返回“模型服务暂时不可用，请检查一下模型配置是否正确”。当前尚未证明失败必然由“第二轮”触发，也可能由该输入触发角色/意图分支。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户复现记录 | Available | 首两次管家回复正常；输入“产品经理”后稳定失败 |
| 源代码 | Available | 待追踪 |
| 运行日志 | Missing | 尚未提供原始模型异常 |
| 模型配置/运行环境 | Partial | 仓库配置可读，运行时有效值待核实 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 精确错误文案与异常包装 | High | In Progress | 找到原始异常被替换的位置 |
| 2 | 聊天入口与第二轮历史组装 | High | Open | 比较首轮/后续轮请求 |
| 3 | “产品经理”角色/意图路由 | High | Open | 验证是否触发不同模型或代理 |
| 4 | 模型配置加载与客户端生命周期 | High | Open | 检查配置和实例复用 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 未提供 | 首次问候与称呼流程正常 | 用户复现记录 | Confirmed |
| 未提供 | 输入“产品经理”后连续返回统一错误 | 用户复现记录 | Confirmed |

## Confirmed Findings

## Deduced Conclusions

## Hypothesized Paths

### Hypothesis 1: “产品经理”触发了不同的角色/模型调用路径

**Status:** Open

**Theory:** 失败与轮次相关只是表象，真正触发条件可能是角色创建/路由分支。

**Supporting indicators:** 用户在两轮普通管家对话后输入角色名，随后首次失败。

**Would confirm:** 代码显示该输入触发独立代理、模型配置或工具调用，且对应路径产生错误。

**Would refute:** 任意普通第三轮消息也稳定失败，或角色路径与普通聊天完全相同。

**Resolution:** 待查。

### Hypothesis 2: 多轮历史组装产生模型 API 不接受的请求

**Status:** Open

**Theory:** 第二轮后的消息历史、工具消息或 system prompt 格式非法。

**Supporting indicators:** 首轮成功而后续轮失败符合状态累积问题。

**Would confirm:** 原始异常或请求构造显示 role/content/tool_call 顺序或长度错误。

**Would refute:** 同一多轮历史经底层模型客户端调用成功。

**Resolution:** 待查。

### Hypothesis 3: 模型客户端/配置仅首次调用有效

**Status:** Open

**Theory:** 客户端生命周期、凭据或流式连接在首轮后失效。

**Supporting indicators:** 失败持续发生。

**Would confirm:** 第二次调用复用已关闭资源，或配置在状态更新后被覆盖。

**Would refute:** 客户端可独立连续调用成功。

**Resolution:** 待查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 底层异常堆栈/HTTP 响应 | 无法区分鉴权、请求格式、网络或配置问题 | 读取现有日志；若不存在，再提出诊断日志点位 |
| 精确复现条件 | 无法区分“第二轮”与“产品经理”分支 | 对照代码并设计最小复现矩阵 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | 待查 |
| Trigger | 待查 |
| Condition | 待查 |
| Related files | 待查 |

## Conclusion

**Confidence:** Low

当前只有用户侧现象，根因尚未确认。

## Recommended Next Steps

### Fix direction

待确认根因后提出，不执行。

### Diagnostic

优先查找现有异常日志和统一错误包装位置。

## Reproduction Plan

比较：首次消息、普通第二/第三条消息、“产品经理”输入、重启后直接输入“产品经理”。

## Side Findings

## Follow-up: 2026-07-28

### New Evidence

- **Confirmed:** `egosync.log:47516-47525` 显示同一会话 effective onboarding step 依次为 1、2、3；step 1/2 成功，step 3 首次返回 HTTP 400：`Thinking mode does not support this tool_choice`。
- **Confirmed:** `egosync.log:47529-47584` 显示失败重试继续推进到 effective step 4、5，并返回相同 HTTP 400。
- **Confirmed:** `egosync-app/src-tauri/src/services/agent_engine.rs:2287-2304` 在 step >= 3 时同时配置 `disable_thinking: false` 与 `tool_choice: required`。
- **Confirmed:** `egosync-app/src-tauri/src/llm/openai.rs:106-138` 仅当 `disable_thinking=true` 才发送 `enable_thinking=false`，并原样发送 `tool_choice`。
- **Confirmed:** `egosync-app/src-tauri/src/services/agent_engine.rs:3939-3953,5421-5432` 将未识别的底层错误概括为“模型服务暂时不可用”。
- **Confirmed:** `egosync-app/src-tauri/src/commands/chat.rs:236-258` 在调用模型前推进并存储 onboarding step，失败重试也会继续前进。

### Additional Findings

1. 根因不是模型服务离线或模型配置缺失。请求到达服务后因参数组合不兼容被 HTTP 400 拒绝。
2. “第二句开始失败”由 onboarding 阶段边界造成：前两个 effective step 不带工具；step 3 开始强制 `create_role` 工具，同时仍启用 thinking。
3. 输入“产品经理”不是词面触发错误；它恰好发生在 effective step 3，并进入角色创建工具阶段。
4. 统一错误摘要隐藏了原始参数冲突，误导用户检查 API Key/模型配置。
5. onboarding 状态在调用成功前推进，使每次失败从 step 3 继续到 4、5；这是次要状态一致性问题。

### Updated Hypotheses

- Hypothesis 1（角色/工具分支）：**Confirmed**。step 3 起进入强制 create_role 工具调用。
- Hypothesis 2（多轮历史格式非法）：**Refuted**。服务明确拒绝的是 thinking + tool_choice 参数组合，不是历史消息格式。
- Hypothesis 3（客户端/凭据首次调用后失效）：**Refuted**。HTTP 400 业务参数错误且前两轮同一 provider 成功。

### Updated Conclusion

**Confidence: High**

根因是 onboarding step >= 3 的请求同时启用模型 thinking 模式并发送 `tool_choice: required`。当前默认 OpenAI-compatible/DeepSeek 模型明确拒绝该组合，因而 HTTP 400。错误摘要函数未识别该错误，将其包装成“模型服务暂时不可用”。

### Recommended Fix Direction

1. **首选、最小修复：** onboarding step >= 3 且强制工具调用时设置 `disable_thinking: true`，保留 `tool_choice: required`，从而保持“必须创建角色提案”的既有产品意图。
2. **不推荐替代：** 将 `tool_choice` 改为 `None/auto` 可保留 thinking，但会失去强制调用 create_role 的确定性，与现有注释和流程意图冲突。
3. **独立可观测性改进：** 为 `Thinking mode does not support this tool_choice` 增加准确错误摘要；它只能改善提示，不能修复请求。
4. **次要一致性修复：** onboarding step 应在成功完成后提交，或失败时不推进，避免瞬时错误导致跳步。

### Verification Plan

- 单元测试：step 1/2 无工具；step 3/4/5 为 `tools=create_role + tool_choice=required + disable_thinking=true`。
- Provider 请求测试：该组合必须包含 `enable_thinking=false`。
- 回归复现：新会话依次输入称呼、角色名；第三 effective step 应产生 create_role tool call，不再出现 HTTP 400。
- 错误路径测试：模拟 provider 失败，确认 onboarding step 不被错误推进（若实施次要修复）。

### Status

Concluded；未修改业务代码，未执行修复。

## Follow-up: 2026-07-28 #2

### Additional Finding: 正常 Thinking 工具调用的兼容性边界

- **Confirmed:** 正常管家请求优先走 opencode；仅 onboarding 跳过 opencode（`agent_engine.rs:3377-3408`）。
- **Confirmed:** 本地 Provider 的普通管家 fallback 使用 `tools=Some(...)`、`tool_choice=None`（`agent_engine.rs:3499-3514`），因此不会触发 onboarding 的 `Thinking mode does not support this tool_choice`。
- **Confirmed:** OpenAI provider 能解析流式 `reasoning_content` 并转换为展示事件（`llm/openai.rs:246-253`）。
- **Confirmed:** `ChatCompletionMessage` 没有 `reasoning_content` 字段（`llm/traits.rs:13-21`）；工具结果 follow-up 只回传 assistant content、tool_calls 和 tool 结果（`agent_engine.rs:3761-3782`）。
- **Deduced:** 对 DeepSeek V4 的直接 Provider fallback，Thinking 模式成功产生工具调用后，后续请求存在因缺少 `reasoning_content` 而 HTTP 400 的风险。此风险不同于 onboarding 的 `tool_choice` 冲突。

### Revised Fix Strategy

1. Onboarding 强制工具调用：关闭 thinking，保留 `tool_choice=required`。
2. 正常 Agent：保持 thinking + tools + 不传 `tool_choice`；由 opencode 负责完整 Agent Loop。
3. 若直接 Provider fallback 也要求完整支持 Thinking 工具调用，则需端到端保存并回传 `reasoning_content`，不能只把它作为 UI 展示事件；否则应在 fallback 的工具请求中关闭 thinking，明确作为降级能力。
