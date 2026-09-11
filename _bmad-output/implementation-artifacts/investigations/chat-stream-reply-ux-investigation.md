# Investigation: 对话回复流式 UX——执行过程不显示 + 回复气泡消失再出现

## Hand-off Brief

1. **What happened.** 桌面端对话回复期间，执行过程（工具/思考轨迹）不可见；回复气泡先显示、突然整体消失、随后再次出现。
2. **Where the case stands.** 两个症状的机制均已在代码层确认：症状②主因是前端在首个思考/工具帧到达后将流式气泡正文设计性清空（`ChatStream.tsx:1207`），并在 done 时刻经历两次 React key 更换；症状①存在多个确认机制（完成态轨迹默认折叠、委派执行期间零事件、事件通道丢帧、LLM fallback 路径无过程帧），用户具体命中哪条需一次复现确认。
3. **What's needed next.** 若接受代码层根因，直接实施方案 A + D（最小修复）；若需先确证症状①命中路径，按 Diagnostic 步骤收集一次复现日志。

## Case Info

| Field            | Value |
| --------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-30 |
| Status           | Concluded（诊断完成，修复未执行——用户指示暂不执行） |
| System           | EgoSync 桌面端（Tauri 2 + React 18）；OpenCode 1.15.10 sidecar |
| Evidence sources | 前端 ChatStream/ChatBubble/useTauriEvent 源码与测试、后端 agent_engine/commands-chat/models 源码、既有前案（delegated-role-tool-process-visibility、model-thinking-display） |

## Problem Statement

用户报告（原文，视为假设）：

- 对话回复的问题
  - 执行过程没有显示
  - 整个回复对话框会先显示，然后整个回复对话框会突然消失，然后再次出现

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| ChatStream.tsx（1405 行，全文已读） | Available | 流式状态机、done 处理、消息合并、trace 渲染 |
| ChatBubble.tsx（382 行，全文已读） | Available | 气泡/ExecutionTrace 渲染与展开逻辑 |
| agent_engine.rs（8707 行，关键段直读 + 子代理结构化追踪） | Available | llm:stream 全部发射点、去重、split 状态机、落库顺序 |
| commands/chat.rs（chat_send_message 全路径） | Available | 消息生命周期、spawn/返回值、Err 兜底 done |
| models/chat.rs + 测试 | Available | phase 值域契约（Android 黄金契约锚点） |
| useTauriEvent.ts + main.tsx | Available | 监听器注册机制 + StrictMode |
| ChatStream.test.tsx / ChatBubble.test.tsx | Available | 委派两段式契约、历史滞后防御的测试锚点 |
| 运行日志（egosync.log） | Missing | 无法确认实际走 opencode 还是 fallback、是否丢帧 |
| 用户复现场景细节（管家委派 / 角色直聊 / 纯文本） | Missing | 决定症状①命中哪条机制 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 症状②机制：流式正文清空 + key 更换 | High | Done | 见 Finding 1/2 |
| 2 | 症状②委派形态：第一段文本不落库 | High | Done | 见 Finding 4 |
| 3 | 症状①候选机制枚举与分级 | High | Done | 见 Finding 5/6 |
| 4 | 用户实际命中路径确认（日志/复现） | High | Open | 需用户提供日志或授权诊断日志（方案 F） |

## Timeline of Events

以一次带工具调用的普通管家回复为例（时间轴为逻辑顺序）：

| 时刻 | 事件 | Source | Confidence |
| ---- | ---- | ------ | ---------- |
| T0 | 前端插入本地 user 消息，isStreaming=true，空流式气泡（BounceDots） | ChatStream.tsx:1137-1155 | Confirmed |
| T1 | thinking token 到达 → 前端合成 thinking trace（summary 非空）→ `hasStreamingExecution=true` → 流式气泡 content 置 `''` | ChatStream.tsx:857-866, 626-646, 1196-1207 | Confirmed |
| T2 | 叙述 token 到达：内容累积进 streamBubbles，但渲染层 content 仍为 `''`（正文不可见） | ChatStream.tsx:999-1008, 1207 | Confirmed |
| T3 | 工具 part 到达：后端先发 phase=tool 状态帧、再发 phase=process 帧（带 processEvent），同时落库；running/completed 各一帧 | agent_engine.rs:2971-3023, 806-854 | Confirmed |
| T4 | 工具执行期间：前端只显示 trace 块 + BounceDots，正文持续隐藏 | ChatStream.tsx:1196-1213, ChatBubble.tsx:341-343 | Confirmed |
| T5 | done=true（每轮恰一帧，严格晚于落库）：同步批处理——追加本地 `__completed__` 消息 + 清空流气泡 + isStreaming=false → 正文"再次出现" | agent_engine.rs:3397-3412; ChatStream.tsx:885-929 | Confirmed |
| T6 | getHistory 异步返回：本地 `__completed__` 消息被替换为持久化消息（key 更换，第二次 remount）；trace map 迁移（要求内容完全匹配） | ChatStream.tsx:944-990, 177-215 | Confirmed |

## Confirmed Findings

### Finding 1: 流式期间正文被设计性清空（症状②主链路）

**Evidence:** `egosync-app/src/components/chat/ChatStream.tsx:1196-1213`、`egosync-app/src/components/chat/ChatBubble.tsx:341-343`、`egosync-app/src/components/chat/ChatStream.tsx:626-646`

**Detail:** `hasStreamingExecution` 在出现任一非空 thinking trace 事件或 tool 状态/process 事件后为 true；此时 `streamingMessages[0].content` 被置为 `''`。thinking token 到达即触发（合成的 thinking trace 事件 summary 非空）。效果：叙述文本先显示（若模型不先思考），首个思考/工具帧到达瞬间正文消失（BounceDots 取代），此后整个工具执行期间正文持续不可见，直到 done 由本地 `__completed__` 消息重新渲染全文。这正是"先显示→突然消失→再次出现"的三段形态。

### Finding 2: done 时刻同一回复经历两次 React key 更换

**Evidence:** `egosync-app/src/components/chat/ChatStream.tsx:1204`（`__streaming__first__`/`__streaming__{messageId}`）、`:72`（`__completed__{id}`）、`:177-215`（mergeHistoryWithLocalMessages 按内容/相邻 user 匹配后替换为持久化 id）

**Detail:** done 同步批处理（914-928）完成第一次替换（流式 key → 本地完成 key），getHistory 异步返回（983-985）完成第二次（本地完成 key → 持久化 id）。每次 key 更换都是 unmount/remount。过程轨迹 map 需随之两次迁移（917-923 → 977-981），第二次迁移要求 completed 内容与持久化内容完全一致（969-975），不一致则轨迹短暂丢失，直到 1091-1102 的异步按消息回拉返回。两次 remount + 轨迹迁移失败窗口，加重视觉跳变。

### Finding 3: 后端每轮只发一帧 done，前端"中间 done"防御分支不可达

**Evidence:** 后端 done 发射点全集 `egosync-app/src-tauri/src/services/agent_engine.rs:2467/2740/2753/3239/3381/3412/3551/3789/3908/3989/4016/4055` + `egosync-app/src-tauri/src/commands/chat.rs:421`（Err 兜底），每轮恰一帧且严格晚于落库；前端 `egosync-app/src/components/chat/ChatStream.tsx:886-908` 的 `isDelegationSegmentDone` 分支基于"done 发两次"的历史契约

**Detail:** 单 done 且携带最终 messageId 时 `doneMatchesActiveBucket=true` → `isDelegationSegmentDone=false`，该分支成为死代码。测试 `ChatStream.test.tsx:1341-1346、1392-1403` 仍按旧契约 mock（两次 done）。这是契约漂移，非当前症状的直接根因，但说明前后端时序契约已失去单一事实源。

### Finding 4: 委派/角色工具 split 路径第一段文本不落库

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:3366-3381`（只把 final_text 写回 assistant_message_id）、`:3032/3045`（first_bubble_text_at_split 仅用于 followup tail 剥离 `:1098-1103`）；前端 `egosync-app/src/components/chat/ChatStream.tsx:69-80`（completedAssistantMessagesFromBubbles 只保留最后一个有内容气泡）

**Detail:** 委派场景完整形态：叙述"稍等，我让 X 看一下"显示 → 工具帧到达正文消失 → 角色执行期间零事件（见 Finding 5）→ done 时本地完成消息只含 followup 文本，第一段文本从屏幕永久消失（DB 无此行，历史合并拿不回）。前端测试 mock 的历史却包含第一段 assistant 消息（`ChatStream.test.tsx:1322-1331`），与后端实际行为不一致。

### Finding 5: 委派期间角色执行零事件；完成态轨迹默认折叠（症状①两个确认机制）

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:4955、5154`（角色 LLM 本地化 drain 不发 llm:stream，注释明确"避免污染管家流"）、`:5727` 附近测试（角色过程不写管家消息）；`egosync-app/src/components/chat/ChatBubble.tsx:307`（`executionTraceDefaultExpanded = Boolean(isStreaming && hasExecutionTrace)`）

**Detail:**
- 机制 a：管家委派后，角色执行期间（可达数十秒）管家界面完全静止——只有 BounceDots 与已到达的 trace 行，无任何新事件。用户感知"执行过程没有显示/卡住"。
- 机制 b：流式中 trace 默认展开（`ChatStream.tsx:1254/1342` 传 defaultExpanded），但 done 后常规渲染路径不传 `isStreaming` → 折叠为一行 "▸ 执行过程" 小按钮。用户在完成后查看，极易感知为"执行过程没有显示"。前案 `delegated-role-tool-process-visibility-investigation.md` 已修复角色会话的轨迹持久化（角色自己的会话页可见），不覆盖管家实时流。

### Finding 6: 三个可导致过程帧丢失/缺失的通道（机制确认，触发未证）

**Evidence:**
- `egosync-app/src-tauri/src/services/event_router.rs:46-62`：EventRouter 通道容量 64，`try_send` 满即静默丢帧。
- `egosync-app/src/hooks/useTauriEvent.ts:13-23` + `egosync-app/src/main.tsx:11`：`listen()` 为异步 promise，effect 重订阅期间存在无监听窗口；dev 下 StrictMode 双挂载加剧。
- `egosync-app/src-tauri/src/services/agent_engine.rs:3554-3558`：run_stream Err → LlmProvider fallback，纯文本流、无任何 tool/process 帧。

**Detail:** 三者任一命中都会让前端收不到过程帧。是否实际触发需要运行日志确认（fallback 有显式 warn 日志 "falling back to LlmProvider"）。

## Deduced Conclusions

### Deduction 1: 症状② = Finding 1 主导 + Finding 2 加重 + Finding 4（委派场景）

**Based on:** Finding 1、2、4

**Reasoning:** "先显示"= 叙述文本渲染（hasStreamingExecution 尚为 false 的窗口）；"突然消失"= 首个 thinking/tool/process 帧将 content 置空；"再次出现"= done 时 `__completed__` 全文渲染。两次 key remount 与轨迹迁移失败窗口使跳变更剧烈。委派场景额外叠加第一段文本在 done 后永久消失。

**Conclusion:** 症状②根因是前端渲染策略（执行期间隐藏正文）+ key 生命周期设计，非后端数据错误。

### Deduction 2: 症状①需按查看时机分诊

**Based on:** Finding 5、6

**Reasoning:** 完成后查看 → 折叠按钮（Finding 5b，最可能，纯前端展示策略）；流式中查看且为委派 → 角色执行零事件（Finding 5a，设计现状）；流式中查看且非委派 → Finding 6 三通道之一（丢帧/重订阅窗口/fallback）。

**Conclusion:** 症状①存在多个确认机制；用户具体命中哪条需一次复现或日志。最高概率是 Finding 5b（完成态折叠）。

## Hypothesized Paths

### Hypothesis 1: 用户症状②由 Finding 1+2 组合命中（普通工具回合即可复现）

**Status:** Open

**Theory:** 任何带工具调用的管家/角色回复都呈现三段形态。

**Supporting indicators:** 代码路径确定性强，无环境依赖。

**Would confirm:** 复现时观察正文是否恰在工具状态行出现瞬间清空、done 瞬间恢复。

**Would refute:** 复现时正文全程可见（则需查 Fallback/丢帧导致流式断流）。

**Resolution:** 待用户复现确认。

### Hypothesis 2: 症状①主因是完成态折叠（Finding 5b）

**Status:** Open

**Theory:** 流式中 trace 展开可见，done 后折叠成一行按钮，用户误以为消失。

**Would confirm:** 复现完成后界面上存在可点开的"执行过程"按钮。

**Would refute:** 完成后连按钮都没有（则查 processEventsByMessageId 为空的原因：丢帧/fallback/回拉失败）。

### Hypothesis 3: 用户环境实际走 LLM fallback（Finding 6c）

**Status:** Open

**Theory:** opencode sidecar 不可用时静默降级为纯文本 LLM 流，无任何执行过程。

**Would confirm:** `egosync.log` 出现 "opencode stream unavailable, falling back to LlmProvider" warn。

**Would refute:** 日志显示 opencode session 正常建立。

### Hypothesis 4: EventRouter 丢帧 / StrictMode 重订阅窗口丢帧（Finding 6a/6b）

**Status:** Open

**Theory:** 工具帧被静默丢弃。

**Would confirm:** `RUST_LOG=debug` 复现并对比后端发射日志与前端收到的事件序列（需临时前端日志）。

**Would refute:** 前端收到全部帧。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | -------------- |
| egosync.log（问题发生时段） | 直接判定 Hypothesis 3；辅助判定 4 | 用户提供 `%APPDATA%\com.egosync.app\egosync.log` |
| 复现场景类型（管家委派 / 角色直聊 / 纯文本 / 是否 dev 模式） | 决定症状①命中机制 | 用户描述或一次定向复现 |
| 前端 console 输出 | 排查 getHistory/回拉失败 | 复现时打开 devtools |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| 症状② origin | `egosync-app/src/components/chat/ChatStream.tsx:1207`（`content: hasStreamingExecution ? '' : ...`） |
| 症状② trigger | 任一 thinking（非空 summary）/tool/process 帧到达（`:857-883`） |
| 症状② condition | hasStreamingExecution 为 true 的整个流式期间；done + 异步历史合并完成前 |
| 症状① origin（折叠） | `egosync-app/src/components/chat/ChatBubble.tsx:307` |
| 症状① origin（委派静止） | `egosync-app/src-tauri/src/services/agent_engine.rs:4955/5154`（设计决策） |
| 症状① origin（丢帧通道） | `event_router.rs:46-62`、`useTauriEvent.ts:13-23` + `main.tsx:11`、`agent_engine.rs:3554` |
| Related files | `commands/chat.rs`、`models/chat.rs`（phase 契约）、`ChatStream.test.tsx`、`ChatBubble.test.tsx` |

## Conclusion

**Confidence:** 症状②机制 **High**（根因代码确认，确定性复现路径存在：任何带工具的回复）；症状① **Medium**（多个机制均确认存在，用户具体命中路径需一次复现/日志定位；最可能为完成态折叠）。

症状②根因不是数据丢失而是渲染策略：执行期间隐藏正文（避免叙述与 trace 重复的设计）+ 三段式 key 生命周期（`__streaming__` → `__completed__` → 持久化 id）造成两次 remount。症状①在"完成后折叠"与"委派期间零事件"两点上是当前设计的直接结果，其余为通道可靠性问题。

## Recommended Next Steps

### Fix direction（暂不执行，待用户裁决）

按机制分类，相互独立可组合：

- **方案 A（症状②-正文消失，P0，最小改动）**：`ChatStream.tsx:1207` 停止清空正文——执行期间正文照常渲染，与 trace 并存。代价：叙述文本与 trace 中 narration 行短期重复（后端已做前缀剥离，done 后不重复）；需同步更新受影响渲染测试。
- **方案 B（症状②-key 抖动，P1）**：统一 key——后端首帧即携带 messageId（或前端流式 key 直接采用后端 messageId），done 后沿用同一 key，历史合并按 id 原位替换；remount 从 2 次降为 0，同时消除轨迹二次迁移。
- **方案 C（症状②-委派第一段，P1，需产品决策）**：二选一——后端 split 时把 first_bubble_text 落库为独立 assistant 消息（前端测试 mock 已如此期望）；或明确放弃第一段持久化，删除前端死分支（Finding 3）并同步修正测试。
- **方案 D（症状①-完成态折叠，P0）**：新完成消息的轨迹默认展开一次（给 ChatBubble 传"刚完成"语义），或把 ExecutionTrace 展开状态提升/持久化以跨 remount 保留。
- **方案 E（症状①-委派静止，P2）**：后端在角色执行期间定期发心跳帧（phase=tool 更新 statusText，如"X 正在执行… 已 Ns"），复用 emit_tool_status。
- **方案 F（诊断，执行前需确认）**：若想先确证症状①命中路径——handleStreamEvent 入口临时日志 + EventRouter 丢帧 warn 日志，复现一次对话后收集，定位后完全移除。

推荐组合：**A + D**（直接消解两个症状的主体，改动最小）；若需先确证再动手，先 **F**。B/C/E 属结构优化，可后续排期。

### Diagnostic

1. 取问题发生时段 `egosync.log`，检索 "falling back to LlmProvider"（判 Hypothesis 3）与 tool_call/process event 行。
2. 定向复现一次：管家发起带工具请求（如"查看我今天的任务"），记录正文清空/恢复时刻；完成后检查是否存在"执行过程"折叠按钮（判 Hypothesis 1/2）。
3. 若仍有疑点，加方案 F 临时日志复现。

## Reproduction Plan

1. 启动 `npm run tauri dev`（注意 dev 有 StrictMode 双挂载变量）。
2. 管家对话发送需要工具的请求（任务查询/天气）。
3. 预期（当前代码行为）：正文出现后于工具状态行出现瞬间清空；done 时全文恢复；完成后 trace 折叠为一行按钮。
4. 委派场景：请求"让产品经理…"，观察第一段叙述在 done 后是否永久消失、角色执行期间界面是否静止数十秒。

## Side Findings

- `commands/chat.rs:421-435` 的 Err 兜底 done 与收尾 `.ok()` 吞错（`agent_engine.rs:3378-3411`）会发出 done 但 assistant 行可能停留在空/isComplete=false；切换页面重载历史会出现空消息（`agent_engine.rs:3775` 注释提到类似形态）。
- `ChatStream.tsx:937` 在 setState updater 内调用 `setIsStreaming`（副作用写在 updater 里），React 严格模式下 updater 可能被调用两次，属潜在隐患。
- 前端测试 mock 的委派历史含第一段 assistant 消息（`ChatStream.test.tsx:1322-1331`），与后端"只落一条"的实际行为不一致——Finding 3/4 契约漂移的直接体现。
- `useTauriEvent` 的异步 listen 窗口 + StrictMode 双挂载是 dev-only 丢帧源，生产包无 StrictMode 双挂载。

## Follow-up: 2026-07-30

### New Evidence

用户追问：按该方案执行后，手机伴侣端（companion-android）能否与桌面端表现一致。补充调查手机侧 `llm:stream` 镜像渲染链路：`StreamCoordinator.kt`（282 行全文）、`ChatViewModel.kt`（关键段）、`ChatScreen.kt`（关键段）、`companion_dispatch.rs` 镜像（已知：全部 llm:stream 原样镜像为 STREAM_TOKEN，含 process 帧）。

### Additional Findings

**Finding 7: 手机端症状①机制与桌面不同——快照替换孤儿化 trace。**
- 流式期间：trace 块累积在 `DesktopStream.traceBlocks` 但**不渲染**（`ChatViewModel.kt:783-816` 只暴露 messages/thinking/responding/streamingToolTitle）；期间仅 ThinkingBubble + ToolStatusRow 单行（`ChatScreen.kt:212-217`）。
- done 时：trace 附加到最后一条流式消息（`ChatViewModel.kt:817-826`），key 为 `seg.messageId ?: "stream-$seq-$i"`——普通单段流无 messageId，key 为合成 id。
- 快照落地（done 触发 write signal → STATE_DELTA → 暂存快照 flush）：`onSnapshotReplaced` 用 DB 消息（真实 id）整表替换消息列表（`ChatViewModel.kt:252`）；`traceByMessageId` 同会话保留（`:261`）但 key 是合成 id → **孤儿化，执行过程消失**。委派 split 流最后一段带真实 messageId → trace 存活，但默认折叠（`ChatScreen.kt:533` `mutableStateOf(false)`）。
- 手机与桌面共享的后端缺口同样生效：委派角色执行零事件（Finding 5a）、fallback 无过程帧（Finding 6c）、EventRouter 丢帧（Finding 6a）。

**Finding 8: 手机端症状②机制与桌面不同——手机无"清空正文"设计。**
- 手机文本始终渲染（`StreamCoordinator.kt:198-204` 累积；`ChatViewModel.kt:799-811` 仅过滤空白段）——桌面 Finding 1 的机制在手机**不存在**。
- 历史 bug（快照掐流式气泡）已由快照暂存门修复（`StreamCoordinator.kt:12-19`）。
- 残余跳变：done→快照落地时 LazyColumn key 从合成 id 换为真实 id（全列表重组）+ 流式文本（含 narration 前缀）被 DB 文本（narration 已剥离）替换 → 内容收缩。
- 委派两段在手机**合并为一个气泡**：分段条件要求 active.messageId 非空（`StreamCoordinator.kt:193`），而桌面首段发 messageId=None → followup 文本并入首段；快照落地后文本变化幅度更大。

**Finding 9: 各方案对手机端的影响（契约层）。**
- 方案 A、D：纯桌面前端渲染改动，不触碰事件流/快照 → **手机端零变化**。
- 方案 B（首帧带 messageId）：手机分段逻辑已兼容非空首段 messageId（`StreamCoordinator.kt:164-195`），委派两段将正确切分；但属 payload 形状变化，按 `models/chat.rs:50-53` 契约须同步黄金契约 fixture 与两侧测试。
- 方案 C（第一段落库）：快照自动下发两条 assistant 消息 → 手机历史视图与桌面一致，无需改手机代码。
- 方案 E（心跳帧 phase=tool）：手机 ToolStatusRow 自动显示（`ChatViewModel.kt:213` toolTitle 路径已存在），无需改手机代码。

### Updated Hypotheses

Hypothesis 5（新增）：用户最初报告的两个症状若实际观察于手机端，则症状① 由 Finding 7（快照孤儿化）+ 共享后端缺口解释，症状② 由 Finding 8 残余跳变解释；桌面方案 A+D 完全不覆盖。Would confirm：用户确认观察端。Would refute：用户确认症状在桌面端。

### Updated Conclusion

**方案执行后手机端能否与桌面一致：不能自动一致。** A+D 是桌面渲染修复，手机端保持现状（流式期间仅工具状态单行、完成后普通流 trace 孤儿化消失、委派两段合并）；B+C+E 经共享事件流/快照让手机顺带受益且手机代码已兼容（仅需同步契约 fixture）；手机要真正对齐还需手机侧独立修复两处：① 流式期间渲染 trace 块（renderStream 暴露 + ChatScreen 流式 trace 区）；② 快照替换时把 trace 从合成 id 迁移到真实 id（或快照下发 per-message trace）。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 5 | 用户症状观察端确认（桌面 / 手机） | High | Done | 用户确认：症状在手机端，桌面端是基准（无问题）。桌面方案 A+D 不再适用 |

### Architecture Note: 为何手机端独立实现、漂移如何发生

**代码层独立是硬约束**：① 运行时不同（React/TS vs Kotlin/Compose）；② 数据访问模型不同——桌面进程内可任意拉取（`chat_get_history`、按消息 `chat_get_message_process_events`），手机经中继网络仅有快照推送 + `llm:stream` 镜像 + 有限命令集，**没有拉取执行过程的命令，快照 schema 亦无 trace 字段**（`companion_snapshot.rs` 无 trace 序列化）；③ 可靠性模型不同（网络断连/丢帧 → 手机需要看门狗/outbox/快照暂存门）。

**但行为层本应对齐，且已有既定规格**：`spec-mobile-fr-parity-group1-chat.md`（FR-29）明确规定"历史默认折叠/**流式中展开**"、FR-1 规定两段委派气泡，mock 阶段均已实现并验收 ✅。

**漂移落点**：Story 13.3 T6 用真实 `STREAM_TOKEN` 替换 mock 打字机（`ChatViewModel.kt:157`）时，流式状态机重写丢失了部分 parity 语义——流式中不渲染 trace（`ChatViewModel.kt:817-826` 仅 done 后挂载）、委派两段在 `messageId=None` 契约下合并（`StreamCoordinator.kt:193`）——且无测试锁定，静默漂移至今。

**工程结论**：手机侧修复（M1/M4/M5）是**恢复已批准规格**而非新设计；防再漂移应把渲染行为提炼为跨端契约（复用 phase 值域契约模式：桌面常量 + 两侧锁定测试 + 手机黄金 fixture），如"流式期间 trace 必须渲染且展开 / done 后文本保留至含完成消息的快照落地 / messageId 切换必须分段"。

## Follow-up: 2026-07-30 #2

### New Evidence

用户确认症状在手机端、桌面端为基准后，完成最后一环验证：写信号触发链、快照消息序列化、手机消息映射、`applySnapshot` 接线（`AppModelContainer.kt:56-58`：flushStash → `snapshotStore.applySnapshot` → store.state 发射 → `onSnapshotReplaced`）。

### Additional Findings

**Finding 10: 手机症状②根因（Confirmed，全链路闭合）——done 时刻回放的是流中段陈旧快照。**

时序链（每步均有证据）：
1. T0：`chat_send_message` 插入 user 消息并发 `message:saved`（`chat.rs:341-344`）+ 插入**空 assistant 占位**（content=""、is_complete=false，`chat.rs:346-347`）。
2. T0+2s：`message:saved` 触发写信号（`companion_snapshot.rs:763`）→ 2s 防抖（`:34`）→ 快照重建。快照消息**原样序列化、无过滤**（`:266-276`）：含 [user, 空 assistant 占位]。
3. 手机：流活跃 → 暂存不应用（`StreamCoordinator.kt:140-153`）→ 文本仍可见。
4. done 帧 → renderStream 渲染最终文本、responding=false（`ChatViewModel.kt:809-816`）→ `stream.streamEnded`（`:830`）→ state.done → **flushStash 应用暂存的陈旧快照**（`StreamCoordinator.kt:113-125` → `AppModelContainer.kt:56-58` → `SnapshotStore.applySnapshot:372`）。
5. store.state 发射 → `onSnapshotReplaced`（`ChatViewModel.kt:228`）→ messages = 快照消息 = [user, **空 assistant**]（`SnapshotMapper.kt:217-226` 映射全部消息、无过滤）→ **流式文本消失**（空气泡）。
6. done=true 同时触发写信号（`companion_snapshot.rs:803-810`）→ 2s 防抖 → 新快照（assistant 已完成）→ STATE_DELTA → 手机流已 done → 直通应用 → **文本恢复**。

间隔 = ~2s 防抖 + 快照构建/推送。**先显示→突然消失→再次出现**与用户描述逐字吻合。触发条件：流时长 > 2s（恰好是带工具/委派的回复——与症状①共现）。桌面基准行为对照：桌面在 done 后本地消息始终保留、历史温和合并，无此窗口。

**根因定性**：快照暂存门（为修复"快照掐流式气泡"而设计）在 flush 时不校验暂存内容的新鲜度——done 时刻暂存里必然还是流中段快照（assistant 完成消息的快照重建最早在 done 后 2s），必然回放陈旧数据。这不是竞态偶发，是**确定性必然**。

### Updated Conclusion

两个症状的手机端根因均已 Confirmed（High）：
- 症状② = Finding 10（陈旧快照回放）+ 次因（快照落地时合成 key → 真实 id 全列表重组、narration 前缀剥离致文本收缩——后者与桌面 done 后行为一致，属可接受基线）。
- 症状① = Finding 7（流式中不渲染 trace + 快照替换孤儿化合成 id trace）+ 共享后端缺口（委派零事件/Fallback 无过程帧）。

### 手机侧修复方案（定稿，暂不执行）

| 项 | 针对 | 改动 | 依据/风险 |
| - | ---- | ---- | --------- |
| **M1（P0）** flushStash 新鲜度守卫 | 症状②根因 | flush 前校验暂存快照已含本轮完成 assistant（按 conversationId 找最后一条 is_complete=true 且非空的 assistant）；不含则丢弃暂存 | done 必触发新快照（Finding 10 链第 6 步），2s 后到达；快照为全量重建，丢弃不丢任何数据。风险：新快照因断连丢失 → 需超时兜底（如 10s 强制应用） |
| **M4（P0）** 流式中渲染 trace | 症状①流式中 | renderStream 暴露 `s.traceBlocks` 至 uiState；ChatScreen 流式区渲染 ExecutionTrace（默认展开） | 恢复 FR-29 已批准规格（spec-mobile-fr-parity-group1-chat："流式中展开"）；桌面参照 `ChatStream.tsx:1341-1343` |
| **M5（P0）** trace 迁移 | 症状①完成后 | `onSnapshotReplaced` 时若快照含本轮完成 assistant（真实 id），把 traceByMessageId 从合成 id 迁移到真实 id（位置匹配：该会话最后一条 assistant） | 委派流最后段已带真实 messageId 天然对上；普通流靠位置匹配 |
| **M2'（P1）** 委派分段放宽 | 委派两段气泡对齐 | `StreamCoordinator.foldInto` 分段条件放宽：active.messageId 为 null 时也允许切到 Some 段（当前要求两者都非空） | 恢复 FR-1 规格。安全性已验证：thinking 帧恒 None 不受影响；普通流全程 None 不受影响；恰好命中委派唤醒帧/首 followup token。不依赖桌面任何改动 |
| **M6（P2）** 契约测试 | 防再漂移 | StreamCoordinatorTest 锁定"流式中 trace 暴露/委派分段/flush 新鲜度守卫"；ChatViewModelTest 锁定"快照替换不孤儿化 trace、不回放陈旧快照" | 复用 phase 值域契约模式；mock→真实流漂移正是缺测试锁定所致 |

执行顺序建议：M1+M4+M5 为一组（消解两症状主体）→ M2' → M6 随各修复同 story 落地。验证：`./gradlew :app:testDebugUnitTest` + `assembleDebug`；桌面端零改动。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 6 | 快照 schema 增加 per-message trace（根治 M5 的位置匹配脆弱性） | Low | Open | 需桌面 schema 扩展（参照 metrics 域先例，另立 story）；M5 已足够对齐桌面当前行为 |
