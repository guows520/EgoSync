# Investigation: 对话执行期间展示模型思考状态

## Hand-off Brief

1. **What happened.** 用户反馈对话执行时看不到模型思考过程，容易产生焦虑；希望执行期间实时展示“Think 思考了xx秒”，其中秒数持续增加。
2. **Where the case stands.** 已完成调查技能激活，尚未确认是后端没有提供思考事件，还是前端已有事件但未渲染；下一步追踪对话流式链路。
3. **What's needed next.** 盘点对话相关源码、事件类型和现有测试，以确认可展示信息的真实边界，避免把不可获得的原始思维过程误当成可直接展示的数据。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-25 |
| Status           | Active |
| System           | Windows; EgoSync 探索 workspace |
| Evidence sources | 源码、测试、项目上下文、Git history（待调查） |

## Problem Statement

用户报告：对话时模型的思考过程不展示，希望在模型执行过程中实时展示思考状态，标题为“Think 思考了xx秒”，xx实时增加。

## Evidence Inventory

| Source   | Status                          | Notes     |
| -------- | ------------------------------- | --------- |
| User report | Available | 现象和期望行为 |
| Project context | Available | 技术栈和架构约束 |
| Source code | Available | 已定位后端事件、Tauri payload、前端订阅和 ChatBubble 渲染链路 |
| Tests | Available | ChatBubble/ChatStream 相关测试已盘点并执行：2 个文件、71 项通过 |
| Logs / runtime trace | Missing | 当前未提供具体会话日志 |
| Version control | Available | 待检查相关实现历史 |

## Investigation Backlog

| # | Path to Explore | Priority              | Status                                | Notes     |
| - | --------------- | --------------------- | ------------------------------------- | --------- |
| 1 | 定位对话页面、消息状态和流式事件订阅 | High | Done | 已确认前端接收 thinking 并保存状态 |
| 2 | 追踪 Rust agent_bridge 到 Tauri Event 再到前端的链路 | High | Done | 已确认 `llm:stream` payload 链路 |
| 3 | 检查 opencode 是否提供 reasoning/thinking 事件及当前适配 | High | Done | 后端支持 `reasoning`/`thinking` part 并转发 |
| 4 | 检查相关测试和历史变更 | Medium | Done | 测试明确断言 thinking 原文不可见 |

## Timeline of Events

| Time        | Event                | Source       | Confidence |
| ----------- | -------------------- | ------------ | ---------- |
| 2026-07-25 | 用户提出缺少模型思考展示的问题及期望 UI 文案 | User report | Confirmed |

## Confirmed Findings

## Deduced Conclusions

## Hypothesized Paths

### Hypothesis 1: 当前链路只传输回答文本/工具状态，没有可供前端展示的 reasoning 事件

**Status:** Open

**Theory:** 后端或上游事件适配没有将模型思考相关事件传递给前端，因此前端无法展示原始思考内容，只能新增执行计时状态。

**Supporting indicators:** 项目上下文描述的是流式传输，但未证明包含 reasoning 事件。

**Would confirm:** agent_bridge 的事件解析、Tauri emit 类型和前端订阅类型均不包含 reasoning/thinking 内容。

**Would refute:** 已存在 reasoning 事件或思考字段，只是 UI 没有消费。

**Resolution:**

### Hypothesis 2: 用户实际需要的是“思考中”可见状态，而不是原始隐藏思维链

**Status:** Open

**Theory:** 需求文案中的“思考过程”可落地为安全的执行状态指示器和耗时计时器，而不直接展示模型内部隐藏推理文本。

**Supporting indicators:** 明确指定标题为“Think 思考了xx秒”和实时增加的秒数，没有要求展示具体推理文本。

**Would confirm:** 上游没有稳定、可公开展示的 reasoning 文本，或产品设计明确只允许状态/摘要。

**Would refute:** 产品协议明确要求展示并持久化可公开 reasoning 摘要。

**Resolution:**

## Missing Evidence

| Gap              | Impact                               | How to Obtain   |
 | ---------------- | ------------------------------------ | --------------- |
 | 对话实际流式事件样例 | 无法确认思考状态起止点及事件频率 | 查看 agent_bridge 日志或抓取一次事件序列 |
 | opencode 适配层事件结构 | 无法判断是否已有 reasoning 字段 | 读取相关 Rust/TS 类型和解析代码 |
 | UI 当前执行态组件 | 无法确定最小修改位置 | 追踪 chat 页面和消息组件 |

## Source Code Trace

| Element       | Detail                                      |
 | ------------- | ------------------------------------------- |
| Error origin  | 待定位 |
| Trigger       | 用户发送对话消息后进入流式执行 |
| Condition     | 执行期间没有用户可见的思考状态/计时器 |
| Related files | 待定位 |

## Conclusion

**Confidence:** Low

源码追踪已完成：后端确实产生并转发 thinking/reasoning 内容，前端也实时接收并保存；当前缺口是 ChatBubble 对这些 props 不渲染，相关测试还明确锁定了“不展示原文”的旧行为。

## Recommended Next Steps

### Fix direction

待源码调查后确定；原则上优先考虑新增安全的执行计时状态，不预设直接展示隐藏思维链文本。

### Diagnostic

读取并追踪对话流式链路；必要时补充一次非持久化事件序列诊断，调查结束后移除临时诊断。

## Reproduction Plan

1. 启动应用并进入对话页面。
2. 发送一条需要模型执行的消息。
3. 记录从发送到首个可见回答/工具事件期间，前端收到的事件类型和 UI 状态。
4. 对照期望：执行期间显示“Think 思考了N秒”，N 持续增长，收到首个可见输出或执行结束后停止/转换状态。

## Side Findings

- 当前未发现。

## Follow-up: 2026-07-25 #2

### New Evidence

1. **后端已有 thinking 事件模型和转发能力（Confirmed）。** `egosync-app/src-tauri/src/models/agent.rs:27-36` 定义 `SseEvent::Thinking`；`egosync-app/src-tauri/src/services/agent_engine.rs:98-125` 将其映射为 `StreamPayload`，设置 `thinking: true`、`phase: "thinking"` 和 `statusText: "思考中..."`。
2. **全局 opencode 事件链路也识别 reasoning/thinking（Confirmed）。** `egosync-app/src-tauri/src/services/agent_engine.rs:166-207` 将 `reasoning`/`thinking` part 分类为 Thinking；`egosync-app/src-tauri/src/services/agent_engine.rs:2718-2742` 继续把该类 part 作为 thinking 流式输出。
3. **前端已实时接收和累计思考内容（Confirmed）。** `egosync-app/src/components/chat/ChatStream.tsx:873-878` 收到 `payload.thinking` 时更新 `thinkingContent`；`egosync-app/src/components/chat/ChatStream.tsx:1227-1233` 将 `streamingThinking` 和 `isThinkingPhase` 传给 `ChatBubble`。
4. **后端的思考状态文案当前也没有进入前端执行状态（Confirmed）。** `egosync-app/src/components/chat/ChatStream.tsx:763-777` 只对 `process` 和 `tool` 设置 `streamStatus`，`thinking` 分支没有消费 `payload.statusText`；因此后端已有的 `"思考中..."` 不能直接形成执行过程标题。
5. **当前 UI 丢弃了上述 props（Confirmed）。** `egosync-app/src/components/chat/ChatBubble.tsx:206-216` 接收参数时没有解构 `streamingThinking`；`egosync-app/src/components/chat/ChatBubble.tsx:266-303` 在无正文时只渲染 `BounceDots`，没有渲染思考标题、耗时或内容。
6. **旧测试把“不展示思考原文”固化为通过条件（Confirmed）。** `egosync-app/src/components/chat/ChatBubble.test.tsx:130-150` 明确断言历史 `thinkingContent` 和实时 `streamingThinking` 均不应出现在页面；`egosync-app/src/components/chat/ChatStream.test.tsx:183-196` 也断言 thinking token 不可见。针对两个文件执行结果为 2 个测试文件、71 项测试全部通过。
7. **当前没有耗时字段或计时状态（Confirmed）。** `egosync-app/src/types/chat.ts:73-84` 的 `StreamPayload` 只有 `thinking`、`phase`、`statusText` 等字段，没有开始时间/持续秒数；`egosync-app/src-tauri/src/models/chat.rs:50-69` 同样没有 duration 字段。
8. **思考内容会被保存到消息记录（Confirmed）。** `egosync-app/src-tauri/src/models/chat.rs:18-31` 的 `Message` 包含 `thinking_content`；`egosync-app/src-tauri/src/services/agent_engine.rs:3026-3029` 在结束时更新并完成消息。

### Additional Findings

#### Finding 1: 根因不是模型没有思考输出，而是前端有意隐藏

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:115-125`、`egosync-app/src/components/chat/ChatStream.tsx:873-878`、`egosync-app/src/components/chat/ChatBubble.tsx:266-303`。

**Detail:** 数据路径已经打通：上游 reasoning → Rust `StreamPayload` → Tauri `llm:stream` → React state/props。最终展示层没有消费 `streamingThinking`，所以用户看到的只是等待动画或其他执行过程。

#### Finding 2: “实时展示思考内容”会与现有产品约定和测试发生直接冲突

**Evidence:** `egosync-app/src/components/chat/ChatBubble.test.tsx:130-150`、`egosync-app/src/components/chat/ChatStream.test.tsx:183-196`。

**Detail:** 这不是缺少测试，而是现有测试明确保护了“不展示 reasoning 原文”的行为。实现前必须明确选择：以当前新需求覆盖旧约定，还是只新增“Think + 秒数”状态而继续隐藏原文。

#### Finding 3: 现有“执行过程”组件适合承载标题，但不能直接复用为计时器

**Evidence:** `egosync-app/src/components/chat/ChatBubble.tsx:179-203`、`egosync-app/src/components/chat/ChatStream.tsx:1081-1100`。

**Detail:** `ExecutionTrace` 当前是静态 block 列表，流式期间只由 `processEvent` 或工具状态驱动；thinking 状态没有进入 `streamingTraceBlocks`。如果要显示 `Think 思考了N秒`，需要在 ChatStream 增加独立计时状态，或把 thinking 设计成一个可更新的 trace block，而不是仅修改 `statusText`。

### Updated Hypotheses

#### Hypothesis 1: 当前链路只传输回答文本/工具状态，没有可供前端展示的 reasoning 事件

**Status:** Refuted

**Resolution:** 后端 `SseEvent::Thinking`、`reasoning`/`thinking` part 分类以及前端 `payload.thinking` 处理均已直接观察到。

#### Hypothesis 2: 用户实际需要的是“思考中”可见状态，而不是原始隐藏思维链

**Status:** Open

**Resolution:** 用户明确要求“实时展示思考内容”，因此不能把需求自动缩减为仅计时器；但是否允许向终端用户公开原始 reasoning 仍是产品/安全决策缺口。

#### Hypothesis 3: 最小改动是在现有 ChatBubble 增加 thinking 展示和前端计时

**Status:** Confirmed as implementation direction, not yet executed

**Supporting indicators:** 数据链路和状态已经存在，变更集中在 `ChatStream.tsx`、`ChatBubble.tsx` 与对应测试；后端不需要新增事件字段即可实现前端本地计时。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 5 | 决定是否允许展示原始 reasoning 文本及其持久化策略 | High | Blocked | 当前代码和测试默认隐藏，需求明确要求显示，存在产品/安全冲突 |
| 6 | 定义计时起点、结束点和多段委派对话的计时语义 | Medium | Open | 需避免 done 中间段错误停止计时 |
| 7 | 设计前端回归测试并更新旧的反向断言 | High | Open | 旧测试会在采用新需求后失败，属于预期变更而非回归 |

### Updated Conclusion

**Confidence:** High

已确认根因：thinking/reasoning 数据已经由后端实时发出，前端也已实时累计并传递给 `ChatBubble`，但 `ChatBubble` 当前明确不渲染这些内容；现有测试还把隐藏思考原文作为预期行为。因此，本问题是展示层行为与新需求不一致，不是模型或流式链路缺失。最小技术方案是前端新增 thinking 展示区和本地计时，不必先改 Rust payload；但“是否公开原始 reasoning”与计时语义必须在实施前定案。

### Recommended Next Steps

#### Fix direction

**方案 A（严格满足当前需求，需产品/安全确认）：** 在现有 `ChatBubble` 的执行过程区域增加一个实时更新的 thinking block：标题固定为 `Think 思考了N秒`，正文增量展示 `thinkingContent`；首次收到 thinking 事件时启动计时，收到最终 done 时停止；完成后沿用消息中的 `thinkingContent` 展示历史内容。同步修改两组旧测试，把“不可见”改成“可见且不进入正文”。

**方案 B（更安全的默认推荐）：** 只显示 `Think 思考了N秒` 和非敏感的状态/摘要，不把原始 reasoning 文本直接展示或继续持久化；保留现有隐藏原文测试，仅新增计时和生命周期测试。该方案能解决等待焦虑，但不完全满足“实时展示思考内容”的字面要求。

#### Diagnostic

实施前建议先确认三项产品决策：
1. “思考内容”是否指原始 reasoning token，还是安全摘要/状态文案；
2. 计时从发送消息、收到首个 thinking 事件，还是进入执行态开始；
3. 多段委派路径中，计时是一轮总计时，还是每个 assistant 气泡独立计时。

无需新增诊断日志即可完成技术定位；当前静态代码和测试证据已足够确认根因。

### Verification Plan

若采用方案 A，验收至少包括：
- 首个 thinking 事件到达后，页面显示 `Think 思考了0秒` 或约定起始值，并每秒更新；
- thinking token 增量只出现在思考区，不进入正文 Markdown、记忆链接或工具正文；
- thinking → answering 后标题停止计时，正文继续正常流式输出；
- done 后历史消息仍能显示与设计一致的思考区；
- 中间 done/委派 follow-up 不会提前停止总计时；
- 无 thinking 事件时仍显示合理的等待状态，不显示虚假的思考原文。

本轮未执行任何业务代码修改。

## Follow-up: 2026-07-25 #3

### New Evidence

用户已确认三个实施前提：

1. 展示原始 reasoning token。
2. 计时起点为首个 `thinking` 事件到达。
3. 计时只覆盖单次 Think 阶段；Think 结束后进入委托/工具执行，委托不会继续计入 Think 时长。

### Additional Findings

#### Finding 4: 计时可以完全在前端完成，无需修改后端事件协议

**Based on:** `egosync-app/src/components/chat/ChatStream.tsx:873-878` 已能识别首个 thinking token，`egosync-app/src/types/chat.ts:73-84` 已有 `thinking` 和 `phase` 字段。

**Detail:** 前端可在首个 `payload.thinking === true` 时记录 `Date.now()`，通过 React effect 定时刷新整数秒数；收到第一个非-thinking 阶段（回答、工具、process 或 done）时停止。无需新增 duration 字段，也无需让后端每秒发计时事件。

#### Finding 5: 委托路径不需要特殊跨段累计，但必须在非-thinking事件到达时立即封存 Think 状态

**Based on:** `egosync-app/src/components/chat/ChatStream.tsx:763-777` 对 process/tool 事件已有分支，`egosync-app/src/components/chat/ChatStream.tsx:779-894` 对 done、thinking、answering 有明确分支。

**Detail:** 计时只属于本次连续 thinking 段。第一个非-thinking事件到达时停止计时并保留最终秒数；之后的委托工具阶段不再更新该秒数。若后续 follow-up assistant 再次产生 thinking，应视为新的 Think 段并重新计时，而不是把两段相加。

### Updated Hypotheses

#### Hypothesis 2: 用户实际需要的是“思考中”可见状态，而不是原始隐藏思维链

**Status:** Refuted by product clarification

**Resolution:** 用户明确要求展示原始 reasoning token，因此实现目标是实时显示思考正文和单段耗时；隐私/安全风险转为已知产品决策，而非当前需求歧义。

#### Hypothesis 3: 最小改动是在现有 ChatBubble 增加 thinking 展示和前端计时

**Status:** Confirmed

**Resolution:** 后端数据链路已完整，计时语义也可以在 `ChatStream` 内闭环。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 5 | 决定是否允许展示原始 reasoning 文本及其持久化策略 | High | Done | 用户已明确选择原始 reasoning token |
| 6 | 定义计时起点、结束点和多段委派对话的计时语义 | Medium | Done | 首个 thinking 事件开始；第一个非-thinking 事件结束；后续新 thinking 重新计时 |
| 7 | 设计前端回归测试并更新旧的反向断言 | High | Open | 实施阶段处理 |

### Updated Conclusion

**Confidence:** High

根因和需求边界均已确认：后端已发送、前端已接收 reasoning token，但 ChatBubble 只显示等待动画而不渲染思考内容。实施时无需修改 Rust 事件协议；在 `ChatStream` 增加单段 Think 计时状态，在 `ChatBubble`/执行过程区域显示 `Think 思考了N秒` 和原始 reasoning 内容即可。

### Recommended Next Steps

#### Fix direction

建议采用以下最小实现：

1. `ChatStream` 新增：
   - `thinkingStartedAt: number | null`
   - `thinkingElapsedSeconds: number`
   - 一个 interval effect：仅在 `thinkingStartedAt !== null` 时每秒刷新；清理 interval，避免组件卸载后更新状态。
2. `handleStreamEvent` 调整：
   - 首个 `payload.thinking` 到达时记录开始时间并初始化 `0` 秒；
   - 后续 thinking token 只追加 `thinkingContent`；
   - 第一个非-thinking事件到达时计算最终秒数并停止计时；
   - `done`、错误、取消、新会话切换时清理当前 Think 段状态；
   - 如果 follow-up 之后再次出现 thinking，重新开始新的单段计时。
3. `ChatBubble` 增加思考执行块：
   - 标题：`Think 思考了${thinkingElapsedSeconds}秒`；
   - 正文：实时渲染 `streamingThinking`，完成消息渲染 `message.thinkingContent`；
   - 思考内容使用纯文本 `white-space: pre-wrap`，不要走回答 Markdown/记忆链接渲染；
   - thinking 阶段默认展开；结束后可保留展开状态或按现有执行过程规则折叠。
4. `ExecutionTrace` 的位置：
   - 推荐把 Think 区放在助手气泡正文之前、工具执行过程之前，保持用户看到的顺序为 Think → Tool/Process → Answer；
   - 不建议把 thinking 伪装成 `actionType: tool`，否则语义和样式都会错误。
5. 测试调整：
   - 将 `ChatBubble.test.tsx:130-150` 的“不可见”断言改为标题和 reasoning 正文可见；
   - 新增首个 thinking 事件启动计时、非-thinking 事件停止计时、后续 thinking 重新开始计时的测试；
   - 保留 thinking token 不进入回答正文、Markdown 和记忆链接的断言；
   - 覆盖 done/error/切换会话时 interval 清理。

#### Diagnostic

不需要新增诊断日志。现有静态证据已经足够；实施后使用前端针对性测试和一次真实流式会话验证即可。

### Verification Plan

- 首个 thinking 事件到达后显示 `Think 思考了0秒`，随后每秒递增。
- 原始 reasoning token 在 Think 正文中实时出现。
- 第一个 tool/process/answering/done 事件到达后，计时停止，委托耗时不计入 Think。
- 后续 follow-up 再产生 thinking 时，显示新的单段计时，不与上一段相加。
- Think 正文不会进入回答正文、Markdown 链接或记忆引用。
- 完成后历史消息可继续查看已保存的 thinkingContent。

本次 follow-up 仍未执行业务代码修改。

## Follow-up: 2026-07-25 #4

### New Evidence

用户补充了 Think 区的展示约束：

- reasoning 内容可能有很多行；
- 思考进行中只显示约 2～3 行；
- Think 结束后自动折叠。

### Additional Findings

#### Finding 6: Think 区应与普通回答内容分离，并采用“流式摘要视窗 + 完整内容折叠”

**Based on:** 用户确认的交互要求，以及当前 `ChatBubble` 已将助手执行过程放置在正文气泡之前的结构。

**Detail:** Think 内容不能直接作为完整长文本流式撑开页面。应保持一个固定/受限高度的预览视窗，思考结束后将完整内容收纳到可展开区域。这样既能提供实时反馈，也不会让聊天列表被 reasoning 内容推离。

### Updated Conclusion

**Confidence:** High

Think UI 的最终交互定义为：思考中显示 `Think 思考了N秒`，正文区域限制约 2～3 行并实时更新；进入非-thinking阶段后停止计时、自动折叠正文，仅保留标题和最终耗时；用户可主动展开查看完整 reasoning 内容。

### Recommended Next Steps

#### Fix direction

建议新增独立 `ThinkingTrace` 展示组件，避免把 reasoning 当成普通工具执行项：

1. **标题行**
   - 思考中：`Think 思考了N秒`；
   - 思考结束：保留最终耗时，例如 `Think 思考了8秒`；
   - 右侧显示展开/折叠箭头。
2. **思考中预览**
   - 默认展开；
   - 内容容器限制为约 2～3 行；
   - 使用 `max-height` + `overflow: hidden`，避免整段内容撑开；
   - 为了让用户看到最新内容，预览区域应跟随 reasoning 底部滚动，或直接显示最后 2～3 行，而不是始终显示开头。
3. **思考结束**
   - 自动设置为折叠；
   - 保留完整 `thinkingContent` 在组件状态/消息数据中；
   - 用户点击标题后展开全部内容；
   - 再次点击恢复折叠。
4. **计时与折叠的先后顺序**
   - 先封存最终耗时；
   - 停止 interval；
   - 再将 Think 区设置为折叠；
   - 避免最后一个 token 到达后标题显示 `0秒` 或内容瞬间丢失。
5. **多段 Think**
   - 每个连续 thinking 段独立生成一个 Think 区；
   - 当前段结束后自动折叠；
   - 后续新的 thinking 段创建新的展开 Think 区。

#### Diagnostic

不需要增加诊断日志。需要通过组件测试验证折叠状态、限制行数和最新内容可见性。

### Verification Plan

- reasoning 超过 3 行时，思考过程中消息气泡高度保持受限。
- 新 token 到达后，预览仍能看到最新的 2～3 行，而不是停留在开头。
- Think 结束后自动折叠，页面只显示标题和最终耗时。
- 点击 Think 标题可展开完整 reasoning，再次点击可折叠。
- Think 内容不会进入回答正文。
- 连续多个 Think 段分别计时、分别折叠，互不累计。

本轮仍未执行业务代码修改。

## Follow-up: 2026-07-26 — Think 内容消失与应用外框再次不可见

### Stronghold Evidence

1. **最新运行轮次没有产生 reasoning 数据。** 只读检查 `C:\Users\Admin\AppData\Roaming\com.egosync.desktop\opencode-global\data\opencode\opencode.db`：2026-07-26 21:16:49 至 22:36:33（UTC+8）的最近 6 个 assistant message 均为 `openai/MiniMax-M3`，`tokens.reasoning = 0`，对应 part 只有 `step-start`、`text`、`step-finish`，没有 `type: reasoning`。同日 17:43:43、18:56:47、20:15:40 的同一模型响应分别存在 70、74、45 reasoning tokens，并有 `type: reasoning` 的 part。
2. **EgoSync 数据库与上游数据一致。** 最近 6 轮的 `messages.thinking_content` 长度为 0，且没有 `message_process_events`；较早有 reasoning 的轮次同时存在 thinking_content 和 thinking event。因此“完成后看不到执行过程”不是前端 reload 后单独丢失，而是本轮没有可持久化的 Think 数据。
3. **前端当前只在收到真实 thinking payload 后创建 Think。** `egosync-app/src/components/chat/ChatStream.tsx` 的当前逻辑仅在 `payload.thinking === true` 时累积内容和创建/更新 thinking trace；空 thinking event 会被过滤。这与此前“没有实际 Think 内容时不要展示 Think 框”的产品要求一致。
4. **外框 DOM 仍存在。** `egosync-app/src/App.tsx:438-441` 有顶层绝对定位 overlay：`border border-slate-200`。生产 Tailwind CSS 已包含该规则。窗口仍为 `decorations:false`、`transparent:true`（`egosync-app/src-tauri/tauri.conf.json`），因此没有系统原生边框作为视觉兜底。
5. **当前外框改动与录入框无关。** 工作树中的相关 diff 只在 `App.tsx`：从根容器 border 改为最上层 overlay；`ChatInput` 外框修改没有触及 App 根窗口。overlay 解决了被子背景覆盖的问题，但当前浅色使用 `slate-200`（约 `#E2E8F0`），在白色桌面上的对比度仅约 1.24:1，1px 线很容易肉眼不可辨。

### Findings

#### Finding 1 — Think 消失的当前直接原因（Confirmed）

当前样本中，Think 不是“前端收到后隐藏/丢失”，而是 opencode/provider 返回的完成消息本身 `reasoning=0` 且没有 reasoning part。EgoSync 因此既没有实时 thinking token可展示，也没有内容可持久化。较早同一模型能够产生 reasoning，说明解析链并非永久失效；MiniMax-M3 是否输出 reasoning 在现有数据中具有轮次差异。

#### Finding 2 — “结束后整个执行过程消失”是同一数据链的结果（Confirmed）

`flush_thinking_process_event` 只有在 pending thinking 非空时才写过程事件；`thinking_content` 也只有 accumulated thinking 非空时才写入。最新轮次上游为空，所以完成后历史消息没有 Think 过程可恢复。当前前端隐藏空 Think 框符合既定要求。

#### Finding 3 — 外框再次不可见是可见度不足，不是 DOM 或 Tailwind 丢失（Deduced, High）

顶层 overlay 已避免旧的“根节点 ring 被内部背景覆盖”问题，但 `slate-200` + 1px + 透明无边框窗口在白色桌面上缺少足够对比。证据不支持“录入框外框修改连带删除应用外框”。

### Recommended Fix Plan（analysis only）

1. **Think：先区分“无 reasoning”与“reasoning 传输故障”。**
   - 保持“无实际内容不显示 Think 框”，不要重新制造空占位框。
   - 在验收用例中使用已确认会产生 reasoning 的复杂请求，并同时核对 opencode `part.data`、`tokens.reasoning`、EgoSync `thinking_content/process_events` 与 UI。
   - 若上游 `reasoning > 0`、UI 仍为空，再修 bus 适配；重点覆盖 delta 早于 part type 时不能 `continue` 丢弃，以及 reasoning part 的真实字段结构。
   - 若产品要求每轮都必须有 Think，则问题在模型/provider 请求策略，需要明确配置 reasoning/thinking 参数；不能由前端伪造。
2. **Think 测试补强。** 新增真实生产 bus 路径测试，而不是只测未被生产调用的 `stream_payload_from_sse`：覆盖有 reasoning、无 reasoning、delta 先到、完成后持久化与历史恢复。
3. **外框：保留 overlay 结构，只调整可识别度。** 推荐使用 `ring-1 ring-inset ring-black/10~12` 或固定中间灰 `#D5DAE0`；不要恢复偏深的 `slate-300`，也不要恢复系统 decorations。优先 inset ring，避免透明 WebView 最外像素裁切。
4. **外框验收。** 在白色桌面、浅灰桌面、深色桌面，以及普通/最大化窗口状态下确认四边连续可见；同时检查圆角处无双线。

### Diagnostic Decision

当前数据已足以解释这次样本，不建议立即增加长期日志。若使用“确定会产生 reasoning”的复杂请求仍复现，则增加一次性诊断：记录 bus event type、partID、partType、delta 长度、reasoning_parts 命中状态及最终 reasoning token 数；定位后全部移除。

### Status

**Analysis complete; no business code or package changes performed. Awaiting user confirmation before implementation.**

## Follow-up: 2026-07-26 #2 — Think 三行固定高度与外框 inset ring

### Stronghold Evidence

1. `egosync-app/src/components/chat/ChatBubble.tsx:147-150` 将流式预览定义为最后 3 个换行分段；`ThinkingTraceBlock` 在 `ChatBubble.tsx:183-186` 同一个元素上同时使用 `max-h-[4.5rem]`、`px-3 py-2`、`leading-6` 和 `border-t`。
2. `leading-6` 每行高度为 1.5rem，三行文本本身正好需要 4.5rem；但 Tailwind 的 border-box 模型使 `max-height:4.5rem` 同时包含上下 padding（1rem）和顶部 border（1px）。按 16px 根字号计算，实际文本高度约为 `72-16-1=55px`，只够约 2.29 行。这与用户看到“2行多”一致。
3. 当前只设置 `max-height`，没有固定高度或最小高度。流式内容从 1 行增长到 2 行再到截断上限时，容器自然高度连续变化，外层执行过程和回答区域随之移动，形成高度抖动。
4. `thinkingPreview()` 按显式换行切最后 3 行，而不是按浏览器视觉换行计算。单个长逻辑行可能折成多行，因此“3 个逻辑行”不等于“3 个可见行”。
5. 当前 `ChatBubble.test.tsx:195-198` 只断言最后三条文本存在，不断言容器的固定高度、可视行数或流式更新时几何尺寸稳定，因此该缺陷未被现有测试覆盖。
6. 当前应用外框仍为 `egosync-app/src/App.tsx:438-441` 的 `border border-slate-200` 顶层 overlay，尚未采用 inset ring。

### Findings

#### Finding 1 — 只显示 2 行多（Confirmed）

`max-h-[4.5rem]` 只为三行文字预留了高度，却把 padding 和 border 也计算进同一个 4.5rem 盒子，导致真正的文本区不足三行。

#### Finding 2 — 流式更新时高度抖动（Confirmed）

`max-height` 仅限制上限，不固定实际高度。内容不足三行时元素按内容高度伸缩；每次 token/newline 更新都可能触发布局重新计算和下方内容位移。

#### Finding 3 — 外框方案可直接收敛（Confirmed）

当前 overlay 的层级结构正确，问题只剩绘制方式和颜色。无需恢复根容器 border，也无需修改 Tauri 窗口配置；将 overlay 从 border 改为 1px inset ring，并采用 `#D5D9DE`，是最小且与目标一致的修改。

### Recommended Fix Plan（analysis only）

1. **Think 使用“外层负责边框和 padding，内层负责固定三行视窗”的结构。**
   - 外层保留 `border-t px-3 py-2`；
   - 内层设置 `h-[4.5rem] leading-6 overflow-hidden whitespace-pre-wrap`；
   - 固定三行文本视窗后，无论当前只有一行还是持续追加 token，整体高度保持不变。
2. **保持最新内容可见。** 推荐内层使用 ref，在 `block.content` 更新后将 `scrollTop` 设置到 `scrollHeight`，但隐藏滚动条；比按 `\n` 截取最后三段更准确，因为它同时处理浏览器自动换行。完成态仍按当前逻辑自动折叠，用户展开后显示完整内容。
3. **移除或降级 `thinkingPreview()`。** 固定视窗并滚动到底部后，不再需要按逻辑换行裁剪；否则长逻辑行仍可能看不到最新 token。
4. **外框 overlay 改为：** `ring-1 ring-inset ring-[#D5D9DE] dark:ring-white/12`，删除原 `border border-slate-200 dark:border-slate-700/70`，避免双线。
5. **测试补强。** 增加三类断言：活动 Think 内层具有固定 `4.5rem` 文本视窗；从 1 行更新到 4 行时外框高度不变；长行自动换行/多行更新后视窗保持在最新内容。外框测试断言 overlay 只有 inset ring、没有 border 双线。

### Verification Plan

- Think 内容为 1 行、2 行、3 行和超过 3 行时，活动 Think 卡片高度完全一致。
- 三行均完整显示，不裁掉第三行底部。
- token 连续更新和新增换行时，下方回答区域不发生上下跳动。
- 长文本自动换行后，仍能看到最新内容。
- Think 完成后自动折叠；手动展开显示完整内容。
- 白色/浅灰桌面上能识别应用四边，但不形成明显深框；圆角无双线。

### Status

**Analysis complete; no business code or package changes performed. Awaiting user confirmation before implementation.**

## Follow-up: 2026-07-27 — Think 仅在结束前短暂出现且历史只有一行

### Scope

仅调查原因与修复方向；未修改业务代码，未执行构建或打包。

### Stronghold Evidence

1. 最新问题样本（2026-07-27 09:41:07，OpenCode message `msg_fa13ba1ca0017GW7DoTR4ca1o5`）的 reasoning part 直到 09:41:24.220 才创建，09:41:24.224 即结束，可见窗口约 4ms；随后 09:41:24.226 开始 text part。OpenCode 最终只保存 69 字符、0 个换行的 reasoning 文本，但记录了 432 reasoning tokens。
2. EgoSync 日志 `C:\Users\Admin\AppData\Roaming\com.egosync.app\egosync.log` 在 2026-07-27T01:41:24.237159Z 记录 `[stream-stage] first thinking emitted elapsed_ms=16686`，证明请求后约 16.7 秒才收到首个可公开 thinking 文本。
3. OpenCode 最新五轮 reasoning part 都只有 34～69 字符且 0 个换行；EgoSync `conversations.db` 的 `thinking_content` 和 completed thinking event 与上游文本逐字一致，没有持久化截断。
4. 对照样本（2026-07-26 22:51:23）同为 OpenCode 1.15.10、MiniMax-M3，曾保存 37,572 字符、3,204 个换行、持续约 170 秒的 reasoning，EgoSync 也完整保存。说明解析和持久化链并非永久只能保留一行。
5. Rust 对 `message.part.delta` 的 reasoning delta 会立即调用 `emit_stream_token(..., true)`，见 `egosync-app/src-tauri/src/services/agent_engine.rs:2679-2727`；对完整 `message.part.updated` 也只计算新增 delta 后立即转发，见 `agent_engine.rs:2824-2859,2985-2997`。没有定时聚合、按行过滤或长度裁剪。
6. 前端收到 `payload.thinking` 后立即追加内容并更新 running trace，见 `egosync-app/src/components/chat/ChatStream.tsx:851-866`；但空 summary 会被 `processEventsToTraceBlocks` 跳过，见 `ChatStream.tsx:475-498`。所以第一段真实文本到达前不会显示空 Think 框，符合此前“没有实际 Think 内容不展示”的要求。
7. 任意后续非-thinking payload 都先调用 `finishThinkingTimer()`，见 `ChatStream.tsx:869`；它把 trace 改为 completed，见 `ChatStream.tsx:648-656`。`ThinkingTraceBlock` 在 `block.isActive` 变为 false 后同步折叠，见 `egosync-app/src/components/chat/ChatBubble.tsx:147-154`。当 summary 与回答紧邻到达时，用户只能看到一次极短闪现。
8. 完成态展开不使用活动态 `h-[4.5rem]` 视窗；完成态为 `max-h-64 overflow-y-auto whitespace-pre-wrap` 并直接渲染完整 `block.content`，见 `ChatBubble.tsx:181-198`。因此“展开后只有一行”不是三行高度 CSS 裁剪，而是源数据本来只有一行。
9. MiniMax 配置路径会显式写入 `reasoning_split: true`，见 `egosync-app/src-tauri/src/services/llm_config.rs:156-190`；代码和已检查配置中没有 reasoning effort、thinking budget 或 summary-only 开关。当前证据不能把变化归因于 EgoSync 主动选择 summary-only。

### Findings

#### Finding 1 — 过程不可见（Confirmed）

这批 MiniMax-M3 样本在请求后的大部分时间没有向 OpenCode/EgoSync提供可公开 reasoning 文本。EgoSync 只在真实 thinking 文本到达时创建可见块，因此约 16.7 秒内无内容可显示。

#### Finding 2 — 结束前闪现后消失（Confirmed）

首个且唯一的短 reasoning summary 到达后，几毫秒内回答 text 随即到达；非-thinking payload 立即完成 trace，React 又在 `isActive=false` 时自动折叠。上游极短窗口与当前自动折叠交互共同产生“短暂显示一下后消失”。

#### Finding 3 — 完成后只有一行（Confirmed）

OpenCode 原始 part、EgoSync 持久化字段和完成事件均只有一行。当前 UI 完成态没有三行裁剪；增加高度不能恢复不存在的内容。

#### Finding 4 — 为什么同一模型从长 reasoning 变成短 summary（Open）

同一 OpenCode 1.15.10 与 MiniMax-M3 既出现过完整长 reasoning，也出现过末尾单行 summary；本地没有找到 summary-only 配置。尚不能区分 MiniMax 服务端动态行为、兼容 API 流格式差异或会话级模型行为。需要原始 provider SSE 请求/响应证据才能继续归因。

### Recommended Fix Direction（analysis only）

1. **交互层首选：Think 首次出现后，不在第一个回答 token 到达时立刻折叠；保留展开到本轮 `done`，再自动折叠。** 这能消除“4ms 闪现”，同时不违反“无实际内容不显示 Think 框”。
2. **备选：设置最短可见时长 800～1500ms。** 若回答已开始但 Think 展示不足最短时长，延迟折叠。此方案改动更复杂，且可能让回答与已结束的 Think 状态短暂并存；优先级低于“到 done 再折叠”。
3. **数据层：若产品目标是实时展示完整推理，必须先确认/更换为稳定流出公开 reasoning delta 的模型或 provider。** 前端不能根据 `reasoning_tokens` 反推、补写或伪造推理内容。
4. **完成态保持真实内容。** 上游只有一句时就显示一句，不人为凑成三行；可按产品语义将短内容标为“思考摘要”，但这是文案决策，不是数据修复。
5. **一次性诊断建议（需用户确认后再实施）：** 在 provider/OpenCode 边界记录脱敏后的 SSE event 类型、partID、partType、delta 长度与时间戳；在 EgoSync bus 边界记录同一字段和最终 part.text 长度。诊断后必须全部移除。

### Verification Plan

- reasoning 长时间无文本、结束前一次性返回 summary：Think 不提前显示空框；一旦出现，至少保留到本轮 done。
- summary 与首个 answer token 相隔数毫秒：不得闪现后立即折叠。
- 完成态单行 summary：展开后完整显示该行，不截断、不伪造额外内容。
- 完整多行 reasoning：活动态保持固定三行视窗并自动跟随最新内容；完成后展开显示完整多行内容。
- 无 reasoning 的模型：全过程不生成 Think 框。

### Status

**Cause established at the display-chain level with High confidence; provider-side reason for summary-only remains Open and requires raw SSE evidence. No business code or package changes performed.**
