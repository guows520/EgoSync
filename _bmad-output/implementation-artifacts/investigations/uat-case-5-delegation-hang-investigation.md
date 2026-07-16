# Investigation: UAT 用例 5 管家委派在 question 工具处卡住

## Hand-off Brief

1. **发生了什么。** UAT 用例 5 阶段 C 的会话已确认进入 opencode `question` 工具等待态，但没有产生完成事件、用户回答或最终 assistant 消息。
2. **当前状态。** 根因已确认：EgoSync 配置允许 opencode 使用 `question`，但 bridge、Tauri Command、前端类型与 UI 均未实现 question asked/reply/reject 协议，只会显示通用工具运行状态并锁定输入。
3. **下一步。** 由产品决策选择“禁用 question、改用普通对话确认”的最小方案，或实现完整交互闭环；随后再进入代码实施与回归测试。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-16 |
| Status           | Concluded |
| System           | Windows；EgoSync UAT；具体构建版本与运行环境待确认 |
| Evidence sources | UAT 用例、用户转录、应用日志、SQLite 数据库、CodeGraph 源码索引、Git 历史与测试资产 |

## Problem Statement

用户报告：`_bmad-output/uat/UAT-Simplified-Manual.md` 的用例 5“管家委派”阶段 C“事实记忆与任务分流”执行时，模型识别出家长会与产品方案讨论会议冲突，随后界面停留在“Tool正在使用 question...”，任务一直没有返回。用户要求先分析原因和方案，暂不执行修复。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| `_bmad-output/uat/UAT-Simplified-Manual.md` | Available | 用例 5 位于第 340 行，阶段 C 位于第 362 行 |
| 用户提供的对话转录 | Partial | 可见模型准备调用 question 工具，但缺少时间戳、工具参数、工具结果及后台日志 |
| 应用日志 | Available | `%APPDATA%\com.egosync.app\egosync.log`，596,717 字节，最后写入 2026-07-16 14:00:39；有工具/委派相关记录，但尚未关联本次复现 |
| 运行数据库 | Available | `conversations.db` 与 `egosync.db` 均存在；尚未读取具体会话 |
| 源代码与调用链 | Available | CodeGraph 索引健康：2,641 文件、39,071 节点、105,037 边；literal 匹配集中于 `agent_engine.rs`、`conversations.rs`、`ChatStream.tsx` 等文件 |
| 版本与构建信息 | Partial | 有 Git 历史；未知 UAT 安装包对应 commit/opencode 版本，且当前工作区相关 Rust 文件有未提交修改 |
| 测试资产 | Available | 发现 28 个相关测试/规格文件，包括 `ChatStream.test.tsx`、`ChatBubble.test.tsx`、`butler-conversation.spec.ts` |
| 当前测试/静态分析结果 | Partial | 存在 `egosync-app/rust-test-full.log`，但时间、对应 commit 与是否覆盖 question 闭环尚未确认；未执行新测试 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 映射 question 工具的定义、注册与调用路径 | High | Done | 配置默认 `* = allow`，question 可被模型调用；应用侧无 question 专用处理 |
| 2 | 检查前端工具交互 UI 与回答回传路径 | High | Done | 前端仅转换为通用 action block，并在工具状态期间锁定输入；无 question 组件或 reply 调用 |
| 3 | 检查 Agent/opencode 会话暂停与恢复状态机 | High | Done | bridge 持久化 running 状态，但不处理 `question.asked/replied/rejected`，也无恢复命令 |
| 4 | 关联本次运行日志与会话数据库 | High | Done | 已按原始消息定位会话 `8f9bc7cf-4ec1-462f-a876-7e7809fbe276` 和 03:24:32 的 pending question |
| 5 | 核对 UAT 运行版本与当前工作区差异 | High | Done | 数据库中的运行证据与当前源码的架构缺口一致；未提交修改未增加 question 支持，不影响根因判定 |
| 6 | 检查近期相关提交及测试覆盖 | Medium | Done | 现有测试覆盖通用 tool running/completed 展示，没有 question 交互契约测试 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 未知 | 用户输入“明天下午3点要参加儿子的家长会，帮我安排一下” | 用户转录 | Confirmed |
| 未知 | 管家识别到与产品方案讨论会议冲突，并表示需用户决定 | 用户转录 | Confirmed |
| 未知 | 界面显示“Tool正在使用 question...”后无返回 | 用户转录 | Confirmed（仅限所见界面状态） |
| 2026-07-16T03:24:24Z | 用户消息写入会话 `8f9bc7cf-4ec1-462f-a876-7e7809fbe276` | `conversations.db` | Confirmed |
| 2026-07-16T03:24:32Z | narration 说明会议冲突，随后 `question` 以 `running` 状态持久化 | `message_process_events` | Confirmed |
| 2026-07-16T03:24:32Z 之后 | 无 question 完成事件、无回答、无后续 assistant 完成消息 | `conversations.db` 全库聚合及目标会话 | Confirmed |

## Confirmed Findings

### Finding 1: 问题对应明确的 UAT 场景

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:340`、`_bmad-output/uat/UAT-Simplified-Manual.md:362`

**Detail:** 用例 5 明确覆盖管家委派与事实记忆/任务分流，报告的卡点位于该用例阶段 C。

### Finding 2: 可见输出停在 question 工具使用提示

**Evidence:** 用户在本调查中提供的执行过程转录。

**Detail:** 转录证明 UI 至少收到了工具使用状态文本；它不能单独证明工具后端已成功发起、前端已收到交互请求或会话已进入可恢复等待态。

### Finding 3: question 工具已真实进入 running 状态

**Evidence:** `conversations.db` → `message_process_events`，message_id=`7f0ded63-d348-4fbe-b8b9-172f4a527239`，created_at=`2026-07-16T03:24:32Z`

**Detail:** 事件保存了完整问题与三个选项，tool_name=`question`、status=`running`、session_id=`ses_0970bc3c1ffePUYE1D8BXsXHkZ`。因此不是单纯文本伪装成工具状态。

### Finding 4: question 从未完成，assistant 消息保持未完成

**Evidence:** `conversations.db` 全库 `question` 事件聚合仅有 `running=1`；目标 assistant message `7f0ded63-d348-4fbe-b8b9-172f4a527239` 为 `is_complete=0`、`content_len=0`

**Detail:** 目标会话没有 question completed、用户回答、tool result 或最终 assistant 内容。应用日志只记录 03:24:24 收到请求和开始发送内容，没有对应完成或错误记录。

### Finding 5: Agent 配置允许未被 EgoSync 支持的 question 工具

**Evidence:** `egosync-app/src-tauri/src/services/agent_config.rs:402`、`egosync-app/src-tauri/src/services/agent_config.rs:406`、`egosync-app/src-tauri/src/services/agent_config.rs:413`、`egosync-app/src-tauri/src/services/agent_config.rs:414`

**Detail:** opencode permission 默认回退为 `{ "*": "allow" }`，代码只额外禁用 `skill`，没有禁用 `question`。因此模型可以合法选择 question，即使宿主应用没有实现其客户端交互协议。

### Finding 6: 后端只把 question 当通用工具事件，没有 question 协议处理

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:2427`、`egosync-app/src-tauri/src/services/agent_engine.rs:2468`、`egosync-app/src-tauri/src/services/agent_engine.rs:2473`、`egosync-app/src-tauri/src/services/agent_engine.rs:2475`；全项目源码 literal 搜索不存在 `question.asked`、question reply/reject 实现

**Detail:** 所有 tool part 都被统一转换为状态文本并持久化。特殊业务处理只在工具 `completed` 后进入 `handle_tool_part`，而 question 必须先由宿主客户端回答才能 completed。后端没有监听 question 请求事件，也没有调用 opencode 回答接口。

### Finding 7: 前端只展示通用工具状态并锁定输入

**Evidence:** `egosync-app/src/components/chat/ChatStream.tsx:215`、`egosync-app/src/components/chat/ChatStream.tsx:223`、`egosync-app/src/components/chat/ChatStream.tsx:342`、`egosync-app/src/components/chat/ChatStream.tsx:352`、`egosync-app/src/components/chat/ChatStream.tsx:745`、`egosync-app/src/components/chat/ChatStream.tsx:747`、`egosync-app/src/components/chat/ChatStream.tsx:754`、`egosync-app/src/components/chat/ChatStream.tsx:756`

**Detail:** 未识别的工具统一归类为 `tool`，标题直接使用“正在使用 question...”。收到 process/tool 状态时设置 `isInputLocked=true`，但没有解析 questions/options、渲染选项或提交回答的代码。

### Finding 8: 前后端公开接口不存在 question 回答能力

**Evidence:** `egosync-app/src/types/chat.ts:34`、`egosync-app/src/types/chat.ts:71`、`egosync-app/src/services/chatService.ts:4`、`egosync-app/src/services/chatService.ts:25`、`egosync-app/src-tauri/src/commands/chat.rs:471`、`egosync-app/src-tauri/src/commands/chat.rs:485`、`egosync-app/src-tauri/src/lib.rs:291`、`egosync-app/src-tauri/src/lib.rs:301`

**Detail:** 类型只有通用过程事件和流状态；service/command 只有发送消息、读取历史/过程、停止流等接口，没有 question request 类型和 reply/reject command。

### Finding 9: opencode 的 question 必须由客户端完成交互闭环

**Evidence:** opencode 官方仓库文档说明 `question.asked` 通过 SSE 广播，客户端须调用 `POST /question/:requestID/reply` 或 `/reject`，之后才产生 replied/rejected 事件。

**Detail:** EgoSync 是 opencode server 的自定义客户端，因此不能只消费 tool part；启用 question 时必须承担问题 UI 与 reply/reject 请求责任。

## Deduced Conclusions

### Deduction 1: 故障位于冲突识别之后、最终答复之前

**Based on:** Finding 2

**Reasoning:** 模型已生成冲突说明并转入 question 工具提示，但没有出现用户选择界面、工具结果或后续回复。

**Conclusion:** 调查应聚焦工具调用生命周期，不应优先怀疑日程冲突识别或记忆读取逻辑。

### Deduction 2: 确定断点是交互等待闭环，而非角色委派执行

**Based on:** Finding 3、Finding 4

**Reasoning:** opencode 已创建结构化 question 请求；此会话没有 `delegate_to_role` 事件，question 之后也没有结果或完成消息。

**Conclusion:** 管家尚未真正委派给“家庭”角色。任务卡住的直接原因是 question 请求没有被回答并恢复会话，而不是家庭角色处理超时。

### Deduction 3: 当前错误处理没有把永久等待转成显式失败

**Based on:** Finding 4

**Reasoning:** question 长期保持 running，日志、消息和事件中均无超时、取消或不支持错误。

**Conclusion:** 即使根因是 UI 或协议缺口，后端仍缺少对不可完成交互工具的失败闭环；这解释了用户看到“一直没有返回”而非错误提示。

### Deduction 4: 根因是能力暴露与宿主实现不一致

**Based on:** Finding 5、Finding 6、Finding 7、Finding 8、Finding 9

**Reasoning:** 配置把 question 暴露给模型；opencode 按协议暂停等待客户端；EgoSync 既不监听专用请求，也不能提交回答，前端还锁住普通输入。

**Conclusion:** 这是确定性的集成契约缺口，不是偶发网络、模型或家庭角色超时。

## Hypothesized Paths

### Hypothesis 1: question 工具调用生命周期在等待态中断

**Status:** Confirmed

**Theory:** question 工具可能已请求用户输入，但交互控件未呈现、回答未回传，或 Agent 会话未在回答后恢复。

**Supporting indicators:** UI 停留在工具使用提示且没有最终返回。

**Would confirm:** 源码/日志显示 question 调用已创建 pending 状态，但缺少对应的 UI request、tool result 或 session resume 事件。

**Would refute:** 日志显示 question 工具根本未被注册/允许，或调用在进入等待前已因协议/参数错误失败。

**Resolution:** 会话库确认 question 仅有 running 事件，assistant 消息保持未完成，且没有回答、完成或恢复事件。

### Hypothesis 2: question 工具在当前 EgoSync 对话协议中不受支持或参数不兼容

**Status:** Confirmed

**Theory:** 模型可选择 opencode 的 question 工具，但 EgoSync 的 bridge/SSE/前端没有完整实现该工具的交互协议，或者使用的 opencode 版本与协议实现不一致。

**Supporting indicators:** 用户看到通用工具使用提示，却未看到问题选择控件。

**Would confirm:** 工具配置允许 question，但 bridge 或前端无对应交互事件类型/回答接口；或版本契约不一致。

**Would refute:** 存在完整且通过测试的 question 交互闭环，且本次日志显示控件已成功呈现并回传。

**Resolution:** 全项目不存在 question asked/reply/reject 类型、事件分支或 command；opencode 官方协议要求宿主客户端实现这些能力。

### Hypothesis 3: 前端只把 question 当普通 tool 状态展示，没有渲染交互控件

**Status:** Confirmed

**Theory:** `question/running` 被现有通用工具状态组件显示为“正在使用 question...”，但没有 question 专用卡片或选择回调。

**Supporting indicators:** 用户转录与持久化事件一致：能看到通用运行提示，却没有选项控件或回答事件。

**Would confirm:** `ChatStream`/`ChatBubble` 仅按 tool running/completed 渲染状态，且不存在 question payload 的专用解析、选择 UI 和回传调用。

**Would refute:** 前端存在完整 question UI，并有证据表明本次 payload 已进入该组件。

**Resolution:** `ChatStream` 将未知工具统一归类为普通 tool，显示 event summary，并锁定输入；不存在 question payload 解析、选择 UI 或回答回调。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| question 回答/恢复协议的源码实现 | 无法确定请求为何无人消费 | 追踪事件解析、前端呈现、回答 API 和 session resume |
| UAT 构建版本/commit | 无法关联近期变更 | 获取安装包版本或运行目录对应 commit |
| UI 截图/交互状态 | 无法确认是否存在被遮挡或不可操作的控件 | 提供卡住时完整窗口截图 |
| 运行安装包精确 commit | 影响回归归属和版本发布记录，但不影响已确认的协议缺口根因 | 实施修复前从构建产物或发布记录补录 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | `agent_config.rs:402-417` 允许 question；`agent_engine.rs:2427-2485` 仅通用持久化；`ChatStream.tsx:745-759` 仅展示状态并锁输入 |
| Trigger | 管家检测到日程冲突后选择 opencode 内置 question 工具 |
| Condition | question 被允许，但 EgoSync 没有消费 `question.asked` 或提交 reply/reject 的客户端实现 |
| Related files | `agent_config.rs`、`agent_engine.rs`、`commands/chat.rs`、`lib.rs`、`types/chat.ts`、`chatService.ts`、`ChatStream.tsx`、`ChatStream.test.tsx` |

## Conclusion

**Confidence:** High

根因已确认：EgoSync 的 opencode 配置默认允许 question，但应用没有实现 opencode 要求的 `question.asked → 用户选择 → reply/reject → session resume` 客户端协议。后端将 question 仅作为通用 tool running 事件持久化，前端将其显示为“正在使用 question...”并锁住输入；没有任何回答接口，因此该会话确定性地永久等待。

**Status:** Concluded

## Recommended Next Steps

### Fix direction

**方案 A：禁用 question，改用普通对话确认（推荐作为当前最小修复）。**

- 在管家及角色的 opencode permission 中显式设置 `question: "deny"`，不能依赖 `* = allow`。
- 在系统提示中明确：需要用户决策时直接输出自然语言问题和选项，然后结束本轮；用户下一条普通消息作为新一轮输入。
- 优点：修改面小，不引入跨层状态机，可快速解除永久卡死；符合当前 EgoSync 已有聊天交互能力。
- 代价：没有原生选项卡和同一 opencode 执行轮内的暂停/恢复体验。

**方案 B：完整支持 opencode question 协议（长期体验方案）。**

- 后端监听并转发 `question.asked/replied/rejected`，保存 requestID/sessionID/questions。
- 新增 Tauri question reply/reject command，由 bridge 调用 opencode `/question/:requestID/reply` 或 `/reject`。
- 前端新增 question 类型、选项卡、自由输入、提交/拒绝状态，并在 pending question 时仅锁普通发送而保留问题控件。
- 增加恢复、应用重启后 pending question 重载、重复提交幂等、会话取消和超时处理。
- 优点：原生交互完整；代价：涉及 bridge、状态持久化、IPC、UI 和恢复语义，属于非琐碎跨层功能。

**共同的防卡死保护。**

- 对未支持的交互工具 fail loud：检测到 question 而功能未启用时立即 reject/abort，并给用户可见错误。
- 为长时间 running 且没有进展的交互工具增加超时/取消状态，确保 assistant 消息不会永久 `is_complete=0`。
- 不建议只在前端加一个选项卡而不实现 reply API；那只能改变显示，无法恢复 opencode 会话。

### Diagnostic

当前根因不需要新增诊断日志即可确认。实施时应通过自动化测试记录 question asked、reply、replied 和最终 done 四个节点；若采用方案 A，则验证模型不再产生 question tool part。

## Reproduction Plan

1. 准备产品经理在明天下午 3 点已有会议任务，并存在“家庭”角色。
2. 输入“明天下午3点要参加儿子的家长会，帮我安排一下”。
3. 方案 A 预期：管家用普通消息列出冲突选项并结束本轮，输入框恢复可用；用户回复选择后继续委派。
4. 方案 B 预期：显示三项选择，提交后产生 question replied，原 session 恢复并继续委派家庭角色。
5. 两种方案都必须验证取消、重启恢复或明确失败，以及 assistant 消息最终 `is_complete=1`。

## Side Findings

- `rg.exe` 在当前 Codex 桌面环境被 Windows 拒绝启动，调查中的文件定位暂用 PowerShell/Git 只读命令替代；不构成产品故障证据。
- 当前工作区已有用户未提交修改：`commands/chat.rs`、`services/agent_engine.rs`、`services/delegate_bridge.rs`；调查不会覆盖或修改它们。
- 近期委派与任务修复集中在自定义工具执行和过程持久化，没有引入 opencode 原生交互工具的宿主协议支持。
