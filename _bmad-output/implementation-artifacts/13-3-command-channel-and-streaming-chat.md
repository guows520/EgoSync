---
baseline_commit: 19f24d99a69c40e15c88cbbcff5c0b631361f155
---

# Story 13.3: 指令通道与流式对话——手机远程操作桌面引擎

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Ultimate context engine analysis completed - comprehensive developer guide created（含桌面/Android 双路代码侦察 + 8 处关键接缝一手核实） -->

## Story

As a 手机用户,
I want 在手机上发对话、勾任务、确认/拒绝建议、查记忆，这些都由桌面引擎真实执行并实时回流结果,
So that 手机不只是看板，而是桌面引擎的完整遥控入口。

## Acceptance Criteria

1. **AC1 companion_dispatch 落地与唯一入口（桌面，硬边界 #2）**
   - Given `services/companion_dispatch.rs` 实现（0→1 新建，当前全仓不存在）
   - When 手机 `COMMAND` 帧到达（`enter_session` 的 `Frame::Command` 臂，现为忽略分支 `companion_connection.rs:871-874`）
   - Then dispatch 解析指令并调用既有命令层/服务（对话发送、会话管理、任务操作、建议确认/拒绝、记忆查询），结果以 `COMMAND_RESULT` 回流
   - And dispatch 是手机指令的唯一入口——不出现任何绕过它直调 agent_bridge/opencode/DB 的路径；**既有 services 文件零改动**（受控例外仅限 Dev Notes §9 登记 两项）
2. **AC2 幂等去重与错误回流**
   - Given COMMAND 帧携带幂等 ID（envelope `commandId`）
   - When 同 ID 指令二次到达（串行重放，14.1 速记队列的机制底座）
   - Then 桌面去重不重复执行，返回首次结果（桌面侧测试证据 = sprint-plan 检查点 S）
   - And 指令执行失败时 `COMMAND_RESULT` 携带错误分类（AppError 单键 map 序列化形状 `{"<Variant>":"<msg>"}`，`error.rs:38-62`）；未知 action / 缺 commandId / payload 超限均显式报错不悬挂
3. **AC3 流式对话换装（手机对话页）**
   - Given ChatViewModel 换装（mock 打字机 → 真实指令通道）
   - When 用户发送消息
   - Then 消息以 `COMMAND(chat.send)` 发往桌面，经 dispatch → `chat_send_message` 编排 → opencode 执行；回复以 `STREAM_TOKEN` 帧流式回流（data = 桌面 `llm:stream` payload 原样 JSON），手机逐字渲染 + 思考中态/工具执行状态行/光标等既有交互不变（UX-M1）
   - And 流式收口链路完整：`chat.send` 的 COMMAND_RESULT 即时确认（含会话/消息 id 对齐），`llm:stream` done 帧为流结束信号；中断（chat.stop）/超时（手机侧看门狗）路径有明确 UI 反馈，不悬挂
4. **AC4 任务与建议操作换装**
   - Given 手机任务页/建议卡
   - When 用户勾选任务、新建任务、确认或拒绝建议
   - Then 均作为 `COMMAND` 由桌面执行（复用命令层，写事件/写信号自然触发 STATE_DELTA），`COMMAND_RESULT` 回流后本地状态经 STATE_DELTA/快照刷新一致
   - And 建议确认后 ActionCard 呈现既有「✓ 已确认 · 已转交管家执行」态（UX-M1；文案与样式零改动）；建议确认产生的任务经写信号补充推送进快照（既有 `confirm_and_create_task` 不 emit 的缺口由本 story 收口）
5. **AC5 记忆页指令通道现查（UX-M5 防呆）**
   - Given 手机记忆页（13.2 留下的「待接指令通道」占位态）
   - When 用户打开记忆页/筛选类别/展开来源/确认遗忘
   - Then 记忆列表与来源消息以 `COMMAND(memory.list / memory.sources / memory.forget)` 现查现显，**不接入快照通道**（`SnapshotStore.memoriesOf` 恒空契约不变）
   - And 查询期间有加载态，失败有重试入口；角色域色温按快照角色数据（既有行为，勿动）
6. **AC6 帧契约与日志纪律（NFR-M7）**
   - Given 指令 envelope（app 层 JSON 塞 `data` 字符串，Dev Notes §2 冻结 schema）
   - When 编码/解码
   - Then payload 字段 camelCase、`deny_unknown_fields` 语义对齐；**协议层零改动**——8 帧类型/`schema.json`/`PROTOCOL_VERSION=1` 一律不动、不 bump（13.1 裁决 1 先例：业务结构走 app 层 JSON）
   - And 帧明文/指令 JSON/流式 token 内容（含对话原文）永不入日志——两端只记帧类型判别式/指令 action/计数（沿用 `frame_type_name` 模式）
7. **AC7 零回归与测试**
   - Given 换装前后对比（UX-M3）
   - When 逐屏截图比对（对话/任务/记忆/简报/通知中心，对照 `companion-android/README.md` 页面地图）
   - Then 视觉与交互零回归（数据源由 mock 换为真实指令通道，布局/样式/动效零变化）
   - And 桌面 `tests/test_companion.rs` 覆盖：dispatch 路由、幂等去重、STREAM_TOKEN 与 `llm:stream` 语义镜像、错误分类回流；Android `./gradlew :app:testDebugUnitTest` 全绿（ChatViewModel 流式状态机、指令发送编码、错误路径）；桌面 `npm run test:all` 全绿（桌面零回归）；无设备环境的验证边界如实声明（12.4/13.2 惯例）

## Tasks / Subtasks

- [x] T1 桌面：指令 envelope 模型与幂等表（AC:2,6）
  - [x] 新建 `models/companion_command.rs`（或并入 `models/companion.rs`）：`CommandEnvelope{schema_version, command_id, action, params}` / `CommandAck{schema_version, command_id, ok, result?, error?}`，serde camelCase；action 集合与 params 形状按 Dev Notes §2 冻结
  - [x] `companion_dispatch.rs` 内建幂等表：`Mutex<HashMap<String /*commandId*/, String /*完成的 ack JSON*/>>` + 容量上限 1000（超限淘汰最旧）；串行重放必命中缓存（契约测试）；桌面重启清空属已声明边界（Dev Notes §9）
- [x] T2 桌面：dispatch 服务与路由（AC:1,2）
  - [x] `CompanionDispatcher`（构造注入：main_pool、conv_pool、`Option<AppHandle>`、engine 写信号 `mpsc::Sender<WriteSignal>`、executor 注入缝）；执行入口 `execute(envelope) -> ack JSON`，**tokio::spawn 执行、绝不阻塞会话帧循环**（帧循环还要应答 PING/排空出站）
  - [x] 生产路由：chat/task/会话操作经 `app_handle.state::<T>()` 直调命令层 pub fn（`chat_send_message`/`chat_stop_streaming`/`chat_new_conversation`/`chat_delete_conversation`/`task_toggle_complete`/`task_create`——签名已核实可直接调用，见 Dev Notes §3）；memory 直调 `services/memory_query.rs` pub fn；suggestion 调 pub 化后的 `confirm_and_create_task`/`reject_with_reason`
  - [x] `enter_session` 的 `Frame::Command` 臂：解析 envelope → spawn dispatch → 结果 `try_enqueue_single` 回发；`CompanionState` 增 `dispatcher` 字段 + setter（镜像 `set_snapshot_request_tx` 先例，`companion_connection.rs:147`），lib.rs 装配
  - [x] 出站单帧通道：`CompanionState::try_enqueue_single(frame) -> bool`——复用 outbound mpsc，**通道满丢弃 + warn、不断连**（与 `enqueue_outbound` 的整序列语义分野，见 Dev Notes §3）
- [x] T3 桌面：STREAM_TOKEN 镜像与写信号补充（AC:3,4）
  - [x] `mirror_stream_payload(payload_json) -> Option<Frame>` 纯函数（llm:stream payload → `Frame::StreamToken(data=原样 JSON)`）；生产注册 `register_stream_mirror(app_handle, state)` 监听 llm:stream（listener 内 tokio::spawn 入队，镜像 `register_write_signal_listeners` 手法 `companion_snapshot.rs:787`）；测试直调纯函数不走事件
  - [x] suggestion.confirm/reject 执行成功后经 `engine.notify_signal()` 补发写信号（`companion_snapshot.rs:628` pub sender；堵既有"确认建任务不 emit"缺口）；task/chat 路径命令层自带 emit，勿重复补
- [x] T4 桌面：测试（AC:1,2,3,7）
  - [x] envelope 解析合法/拒绝（未知 action、缺 commandId、payload 超 65519 上限显式错误）
  - [x] dispatch 路由断言（fake executor 注入缝）；幂等串行重放；错误分类 ack 形状 `{"ValidationError":"..."}`
  - [x] STREAM_TOKEN 镜像纯函数（含 done 帧）；suggestion 确认后写信号到达
  - [x] 真链路：`phone_pair_and_connect` → 发 COMMAND(memory.list)（真 pool 种子）→ 断言 COMMAND_RESULT 载荷；chat.send 用 fake executor + 直调 mirror 模拟 token 流 → 断言 STREAM_TOKEN 顺序与 done 收口
- [x] T5 Android：指令通道基建（AC:1,3）
  - [x] 新包 `command/`：`CommandModels.kt`（envelope + action + params，org.json 手法，勿引新依赖）+ `CommandChannel.kt`（发送编码入会话出站队列 + pending 表 `CompletableDeferred` + 超时）+ `StreamCoordinator.kt`（按 conversationId 聚合 token、活跃流标志、快照延后门）
  - [x] `ConnectionClient` 六成员签名**零改动**（UX-M2 契约）；新增独立接口 `CommandSender`（send/suspend execute），`RealConnectionClient` 实现之；出站走会话循环内 pump 协程（镜像桌面 outbound 模式）；`Frame.CommandResult/StreamToken` 分发至独立注入的指令帧消费者（勿塞进每会话重建的 SnapshotFrameHandler——pending 表须跨会话存活）
  - [x] `AppModelContainer` 装配：`commandSender` 二态暴露（realConnection 实现 / fake 态 null → UI 显式失败提示，不伪造回执）
- [x] T6 Android：ChatViewModel 换装（AC:3）
  - [x] sendMessage → `COMMAND(chat.send)` + 本地回显；ack 返回的 `{conversationId, userMessageId, assistantMessageId}` 用于替换本地临时 id（快照替换 id 稳定无闪烁）
  - [x] STREAM_TOKEN 驱动流式状态机：thinking→思考中态、token+thinking→逐字渲染、phase=tool+statusText/toolName→工具执行状态行、phase=process+processEvent→溯源/提案状态（映射表对照桌面前端消费方式，事件缺席时状态保持 null 不造假）；done→收口
  - [x] 快照-流式互斥：流式活跃期间快照帧暂存（只留最新），流结束/失败立即补应用（裁决见 Dev Notes §4）；停止按钮 → `COMMAND(chat.stop)`；看门狗超时（120s 无帧）→ 错误态 + 可重试
  - [x] 会话生命周期换装：新对话/删除会话 → `COMMAND(conversation.new / conversation.delete)`；委派关键词本地路由 mock 移除（真实路由由桌面管家完成）
- [x] T7 Android：任务与建议卡换装（AC:4）
  - [x] toggleTask：乐观本地翻转 + `COMMAND(task.toggle)`；失败回滚 + 中文提示；成功经 STATE_DELTA 收敛
  - [x] createTask：乐观追加（classifyingIds 态）+ `COMMAND(task.create)`；ack 返回桌面 Task（替换临时 id）；象限归类经 task:classified→STATE_DELTA 更新（移除 4s 固定 Q2 mock）
  - [x] 建议 ActionCard：流结束后 `COMMAND(suggestion.list)` 现查 pending 建议（数据源裁决见 Dev Notes §4）；确认/拒绝 → `COMMAND(suggestion.confirm/reject)`；拒绝 reason 用固定文案（手机无 reason 输入框）；确认态 UI 沿用既有「✓ 已确认 · 已转交管家执行」
- [x] T8 Android：记忆页现查（AC:5）
  - [x] `MemoryViewModel`（已注入 AppModelContainer，顺手）改经 commandSender：memory.list 加载态/类别筛选、memory.sources 展开溯源、memory.forget 遗忘确认；失败重试入口；占位态文案移除
  - [x] `MemoryCard/MemorySourceMessage` 组件零改动复用（13.2 已预留）；`memoriesOf` 恒空契约保持（防呆断言不放松）
- [x] T9 Android：测试（AC:3,4,5,7）
  - [x] CommandChannel：envelope 编码字节断言（FakeWebSocket 先例）、pending 超时、会话断开 pending 全失败
  - [x] ChatViewModel：发送→COMMAND 帧 + 回显 id 对齐；token 注入→状态机各态；done 收口；超时错误态；快照延后/补应用；chat.stop
  - [x] TasksViewModel：乐观翻转回滚、临时 id 替换、分类收敛；ActionCard suggestion.list 渲染 + 确认拒绝；MemoryViewModel 现查/重试/遗忘
- [x] T10 验证与零回归（AC:7）
  - [x] 桌面 `npm run test:all` 全绿；Android `./gradlew :app:testDebugUnitTest` + `:app:assembleDebug` 全绿
  - [x] 逐屏截图比对；无设备环境如实声明验证边界（惯例同 12.4/13.2）
  - [x] 既有 services 零改动佐证：`git diff` 中 services/ 目录仅新增 `companion_dispatch.rs` 与 §9 登记的受控例外

## Dev Notes

### 1. 冻结契约（不得触碰）

- **协议层零改动**：`crates/companion-proto` 全部冻结。`Command/CommandResult/StreamToken` payload 均为 `{data: String}` 不透明容器（`frames.rs:65-84`）——指令业务结构走 app 层 JSON 塞 `data`，**不 bump PROTOCOL_VERSION**（13.1 裁决 1 先例；`deny_unknown_fields` 下私改 payload 字段 = 三端解码全炸）。
- **帧 wire 形态**：`[u32 BE 长度前缀][Noise 密文]`，内层 `{"type":"command","data":"<envelope JSON>"}`；单帧明文 ≤65519。**指令/结果/token 不做分帧**（envelope 远小于上限；超限显式报错）——分帧 envelope 仅属 SNAPSHOT/STATE_DELTA 既有机制。
- **出站双语义**（13.1 评审冻结的延续）：`enqueue_outbound(Vec<Frame>)` = 整序列、持锁入队、**通道满强制断连**（快照语义，不得中途丢帧）；本 story 新增 `try_enqueue_single(Frame)` = 单帧、try_send、**满则丢弃 + warn 不断连**（流式 token 丢一帧由快照兜底收敛，断连反而破坏体验）。两者共用 outbound mpsc（容量 512）与会话编码循环。
- **`ConnectionClient` 六成员签名零改动**（13.2 评审 UX-M2 契约，`FrameConsumer.kt:7`）；Android 不引入 Room/Hilt/新依赖；指令 JSON 用 org.json（SnapshotParser 先例）。
- **日志纪律（NFR-M7）**：STREAM_TOKEN/COMMAND data 含对话原文——桌面 tracing 只记 action 名/帧类型/计数；Android 沿用 `Companion/` 前缀 tag 同规则。
- **测试路径 `app_handle: None`**：`CompanionState::with_static_keypair_for_testing` 全链路无 AppHandle——凡依赖 AppHandle 的路由必须走注入缝，测试用 fake executor（chat/task/suggestion）；memory 路由纯 pool 可真跑。

### 2. 指令 envelope schema（本 story 冻结，双端共同事实源）

```jsonc
// COMMAND.data（手机 → 桌面）——camelCase
{"schemaVersion": 1, "commandId": "<uuid v4>", "action": "<action>", "params": { /* action 专属，camelCase */ }}

// COMMAND_RESULT.data（桌面 → 手机）
{"schemaVersion": 1, "commandId": "<uuid>", "ok": true, "result": { /* action 专属 */ }}
// 或
{"schemaVersion": 1, "commandId": "<uuid>", "ok": false, "error": {"<AppError变体名>": "<中文msg>"}}

// STREAM_TOKEN.data：桌面 llm:stream StreamPayload 原样 JSON（已 camelCase）：
// {conversationId, token, done, thinking, messageId?, phase?, statusText?, toolName?, processEvent?}
// ——不套 envelope；conversationId 即手机路由键。镜像全部 llm:stream（含桌面端发起的流），
// 手机侧过滤：仅当前查看会话渲染，其余忽略（快照在 done 后收敛）。
```

**action 集合（收敛为原型 UI 可触发的操作，勿扩）**：

| action | params | result | 桌面复用（零改动直调） |
|---|---|---|---|
| `chat.send` | `{conversationId?, roleId?, content}`（conversationId 空 → 桌面先建会话） | `{conversationId, userMessageId, assistantMessageId}` | `chat_send_message`（commands/chat.rs:222；含流中守卫/预插 assistant/spawn run_stream） |
| `chat.stop` | `{conversationId}` | `{}` | `chat_stop_streaming`（:558，CancelTokens+abort） |
| `conversation.new` | `{roleId?}` | Conversation 序列化 | `chat_new_conversation`（:622） |
| `conversation.delete` | `{conversationId}` | `{}` | `chat_delete_conversation`（:585） |
| `task.toggle` | `{taskId, isCompleted}` | Task 序列化 | `task_toggle_complete`（commands/task.rs:131） |
| `task.create` | `{...镜像 CreateTaskInput 字段（models/task.rs）}` | Task 序列化 | `task_create`（commands/task.rs:24，含校验+异步 LLM 分类+emit） |
| `suggestion.list` | `{conversationId}` | `{suggestions: [...]}`（镜像 `SuggestionWithRole` 序列化） | `suggestion_list_pending`（commands/suggestion.rs:13） |
| `suggestion.confirm` | `{suggestionId}` | Suggestion 序列化 | `confirm_and_create_task`（:32，pub 化受控例外） |
| `suggestion.reject` | `{suggestionId, reason}` | Suggestion 序列化 | `reject_with_reason`（:61，pub 化受控例外） |
| `memory.list` | `{roleId?, category?, limit?, offset?}`（limit 缺省 50 防大结果） | `{memories: [...]}` | `services/memory_query.rs:9/19` |
| `memory.sources` | `{memoryId}` | `{messages: [...]}`（MemorySourceMessage） | `memory_query.rs:46` |
| `memory.forget` | `{memoryId}` | `{}` | `memory_query.rs:37`（NotFound→AppError） |

未知 action → ack error `{"ValidationError":"不支持的指令类型"}`。result 序列化直接 `serde_json::to_value(&model)`（模型已 camelCase）；手机侧按同名 camelCase 解析。

### 3. 桌面侧设计要点

- **挂接点**：`enter_session`（companion_connection.rs:765）帧分发 match 新增 `Frame::Command` 臂（现为 `other =>` 忽略，:871-874）。命令执行必须 `tokio::spawn`——`agent_bridge::send_message` 类长操作 inline await 会饿死 PING 应答与出站排空（既有 120s 空闲超时依赖帧活动重置；手机 30s PING 已覆盖保活，勿重复处理）。
- **命令层直调手法**（已核实可行）：`chat_send_message(request, app_handle.state::<DbPool>(), app_handle.state::<ConversationsPool>(), app_handle.state::<StreamingState>(), app_handle.state::<OnboardingConversations>(), app_handle.state::<AgentConfigService>(), app_handle.clone())`——State 参数经 `AppHandle::state` 现场获取（commands/chat.rs:294/351 内部同款手法）；命令 fn 本体不受 `#[tauri::command]` 影响仍可直接调用。**无需提取/重构任何既有命令函数**。
- **依赖装配**：`CompanionDispatcher` 构造注入（main_pool、conv_pool、`Option<AppHandle>`、写信号 sender、executor 缝），`CompanionState` 增 `dispatcher: Mutex<Option<Arc<CompanionDispatcher>>>` + setter（镜像 `set_snapshot_request_tx` :147 先例）；lib.rs 在引擎构造后装配（dispatch 需要 `engine.notify_signal()` 的 sender clone，companion_snapshot.rs:628）。
- **STREAM_TOKEN 镜像**：`register_stream_mirror(app_handle, state)` 监听 llm:stream → `mirror_stream_payload` 纯函数转换 → `tokio::spawn(try_enqueue_single)`（listener 同步闭包内不得 await，镜像 `register_write_signal_listeners` 的 try_send 手法 :796-799）。**镜像全部流**（同用户跨设备桥接语义；手机过滤）。done 帧同时触发既有快照重建（companion_snapshot.rs:803 特判，勿动）。
- **写信号补充**：仅 suggestion.confirm/reject 需要（confirm 建任务但不 emit task:created——commands/suggestion.rs:32-58 既有缺口）；task/chat 路径命令层 emit 齐全（WRITE_SIGNAL_EVENTS，companion_snapshot.rs:739），**勿重复补发**。
- **单槽会话语义**：新连接取代旧会话时（companion_connection.rs:778-788），在途指令结果经 `try_enqueue_single` 天然发往**当前槽**（可能是新连接）——幂等 commandId 保证手机去重，此语义如实记录即可。
- **错误分类**：路由/参数错误 → AppError 直接 `serde_json::to_value`；流式失败不包装进 ack（`chat_send_message` 已把 run_stream 失败兜底成 llm:stream done 帧，commands/chat.rs:421-434）——手机靠 token 流的 done/error 与看门狗感知。

### 4. Android 侧设计要点

- **指令通道注入缝**：新包 `command/`（架构包清单的受控扩展，登记于 Project Structure Notes）。出站 = `CommandSender` 接口（RealConnectionClient 实现；会话循环内 pump 协程消费出站 Channel，`session.send(FrameCodec.encode(...))`——编码链路 12.4 已验证，RealConnectionClient.kt:529-533 同款）；入站 = 独立指令帧消费者注入（`Frame.CommandResult/StreamToken` → `withContext(ioDispatcher)` 分发，RealConnectionClient.kt:551-552 预留注释位）。**pending 表跨会话存活**（勿放每会话重建的 SnapshotFrameHandler）；会话断开 → pending 全部失败（快照兜底，简单诚实）。
- **快照-流式互斥裁决（本 story 钉死）**：流式活跃期间（StreamCoordinator 按当前查看会话追踪），SNAPSHOT/STATE_DELTA 帧暂存不应用（只留最新），流结束/失败/超时后立即补应用。理由：`chat_send_message` 写入触发 2s debounce 重建 → 流式中段 STATE_DELTA 必然到达；现有 `onSnapshotReplaced → stopStreaming()` 守卫（ChatViewModel.kt:171，13.2 评审 P3）会掐掉正在流式的气泡——全局暂存是最小正确解；任务/仪表盘秒级延迟由乐观更新掩盖（诚实代价，写入完成笔记）。
- **消息 id 对齐**：`chat.send` ack 的三个 id（conversationId/userMessageId/assistantMessageId）立即替换本地回显的临时 id——后续快照替换时 LazyColumn key 稳定，无闪烁/无重复（本地 `nextId=100` 命名空间与桌面 UUID 混排的隐患就此消除）。
- **建议卡数据源裁决**：快照无 suggestions 域（口径落死不含）——`suggestion.list` 现查（同记忆页手法，UX-M5 精神）；渲染时机 = 流结束后查询、存在 pending 才渲染。mock 触发（第 2 轮回复浮现卡片）移除。
- **演示数据处置**：`ChatDemoData` 的 butlerReplies/delegationReplies/delegationFirstSegment/toolExecutionStages/decompositionProposal/executionTrace/roleProposal/lowConfidenceReply 移出生产路径（Preview 样例可留）；onboarding demo 不动（桌面首跑流，不在本 story 范围）。真实 thinking/tool/process 状态全部来自 STREAM_TOKEN 的 phase/processEvent 字段（MessageProcessEvent 形状见 models/chat.rs:36-48；eventType→溯源块/提案的映射对照桌面前端消费方式实现，事件缺席时对应状态保持 null 不造假）。
- **任务乐观更新**：toggle 翻转 + 失败回滚；create 用 classifyingIds 过渡态（既有 UI 语义保留），ack 替换临时 id，象限归类等 task:classified → STATE_DELTA 收敛（移除 4s 固定 Q2 mock）。
- **Debug/Fake 态语义**：DebugConnectionMode 四态模拟保留（连接态可视化开发工具）；指令通道在 fake/debug 态显式失败提示（「桌面引擎不可达」），**不伪造回执**（NFR-M3）。
- **QuickNoteQueue 本 story 不动**（持久化 FIFO + 幂等提交属 14.1；13.3 交付其依赖的幂等机制底座）。

### 5. mock 替换对照表（关键约束 #6）

| 现 mock（位置） | 处置 |
|---|---|
| runNormalReply/runDelegation 打字机（ChatViewModel.kt:386-471） | STREAM_TOKEN 驱动流式状态机替换 |
| butlerReplies/delegationReplies 等（ChatDemoData.kt） | 移出生产路径（真实回复来自 token 流） |
| 委派关键词本地路由（ChatViewModel.kt:369-373） | 移除——真实委派由桌面管家路由，回复照常经 token 流回流 |
| ActionCard mock 触发（:453-465） | suggestion.list 现查驱动 |
| respondActionCard 本地翻转+本地回执（:553-579） | COMMAND 确认/拒绝 + 状态翻转保留（UX-M1 终态不变） |
| toggleTask 本地翻转（TasksViewModel.kt:155-163） | 乐观翻转 + COMMAND + 失败回滚 |
| createTask 本地追加 + 4s 固定 Q2（:189-218） | COMMAND task.create + ack id 替换 + STATE_DELTA 归类 |
| 记忆页占位态（MemoryScreen.kt:105-113, 214-223） | 现查真实数据 + 加载/错误/重试态 |
| 记忆遗忘内存态移除（MemoryViewModel.kt:98-105） | COMMAND memory.forget |

### 6. 测试要求

- **桌面**（`tests/test_companion.rs`，真 socket + `app_handle: None`，helper 全复用：`phone_pair_and_connect` :1104 / `setup_listener_with_engine` :881 / `wait_for` :120 / `seed_snapshot_domain_data` :920）：见 T4。镜像 `phone_receive_snapshot`（:1042）写 `phone_receive_frame(Frame::CommandResult/StreamToken)` 式 helper。
- **Android**（JUnit4 + kotlinx-coroutines-test，无 Turbine/mockito，Fake 手法见 RealConnectionClientOrchestrationTest.kt:39-157）：见 T5/T9。VM 测试用 StandardTestDispatcher + advanceTimeBy 推进看门狗/超时；帧注入直调消费者（比 mock delay 打字机更易测）。
- **负向防呆**：`memoriesOf` 恒空断言保持；SnapshotStore 无建议卡数据源断言（防有人把 suggestions 塞进快照通道）。

### 7. 前序 story 情报（直接适用）

1. **13.1 血泪**：分片数学上限实测（本 story 不分帧但 envelope 上限校验必须有）；超时预算防挂起；日志只记判别式；`enqueue_outbound` try_send 不阻塞。
2. **13.1 留给本 story 的收口项**：`role:deleted`/`task:deleted` payload 形状不一 → 本 story 的 ack envelope 统一一次做对（单键 error map + camelCase result）。
3. **13.2 留给本 story 的收口项**：本地写操作演示态（STATE_DELTA 覆盖本地暂存）→ 本 story 指令通道收口；`onSnapshotReplaced` 停流守卫与流式互斥 → §4 暂存裁决收口；记忆页/能量趋势占位 → 记忆页本 story 接通，能量趋势仍占位（需桌面历史 schema，不在范围）。
4. **13.2 评审教训**：VM collect 守卫（快照同一性/loaded=false 清空）换装时必须保持；unpair 清态路径勿破坏；空列表/除法守卫沿用。
5. **12.4 教训**：双承载切换单会话槽替换语义 → pending 表跨会话 + 幂等 ID 兜底；真网络路径留手动冒烟如实记录。

### 8. UX-DR 契约（epics.md:3038-3042）

- UX-M1 UI 零重做：只换数据源与执行层，样式/交互/终态文案（含「✓ 已确认 · 已转交管家执行」）零改动。
- UX-M2 装配缝换装：AppModelContainer 唯一换装点；ConnectionClient 六成员零改动。
- UX-M3 视觉零回归：对照 README 页面地图逐屏比对。
- UX-M5 记忆防呆：记忆只走指令通道，降级态入口置灰语义保留（降级 UI 本体属 14.1）。
- NFR-M4 桌面零回归 / M5 桌面唯一事实源（手机零业务库主权，指令全部桌面执行）/ M7 帧明文永不入日志。

### 9. 受控例外与已声明边界（显式登记）

1. **受控例外 A**：`commands/suggestion.rs` 的 `confirm_and_create_task`/`reject_with_reason` 私有 fn **pub 化**（纯可见性变更，零行为改动）——dispatch 复用唯一入口，避免重写确认编排造成漂移。AC「既有 services 零改动」就此一项让位（services/ 目录本身仍零改动）。
2. **受控例外 B**：`companion_connection.rs`/`companion_snapshot.rs` 属 companion 模块，本 story 的挂接/出站单帧/镜像监听均为其自身演进，不属"既有 services"。
3. **幂等边界**：去重缓存内存持有、桌面重启清空——14.1 速记重放仅在「确认丢失 + 桌面重启」双罕见叠加下可能重复入库（低危：重复一条管家消息），如实声明不掩盖。
4. **指令集边界**：task.delete/task.reorder/skill 选择/工作目录选择不在集合——手机原型 UI 无对应入口（epics AC 的「等」字空间），勿投机实现。
5. **简报行动点**：真实快照 `actionPoints` 恒空（SnapshotMapper.kt:105/115，桌面简报为自由文本无结构化行动点）——「确认行动点」无数据承载，本 story 登记为 deferred：待桌面结构化行动点落地后接线（Open Question 1）。
6. **MemoryCard 来源槽（实施中发现）**：story 原文「组件零改动复用（13.2 已预留）」经核实不准确——13.2 的 `MemoryCard` 无来源参数。本 story 补 `sources/sourcesLoading` 参数 + 新增 `SourceMessageRow`，同屏消费 `memory.sources` 懒加载结果。属「按裁决落地」性质（非偷改既有行为，仅补缺失的展示槽），批准并记录于此。
7. **MemoryScreen 占位文案落地形态**：T8「占位态文案移除」落地为加载中/错误重试/真实空三态（原「记忆通道尚未接通」占位文案移除）——三态复用既有 Material 组件样式，无新增布局范式。
8. **委派第二段角色头像退化（评审 D3-B 登记）**：STREAM_TOKEN 载荷无角色信号（schema 冻结，messageId 可选、无 roleId），管家视图的委派回复第二段渲染为管家气泡（mock 时代挂目标角色头像）。客观协议限制，待桌面结构化角色信号落地后接线；`decomposition`/`roleProposal` UiState 字段与响应函数保留（UI 能力在、生产者待桌面数据），不删。

### Project Structure Notes

**桌面新增**：`egosync-app/src-tauri/src/services/companion_dispatch.rs`（0→1）；`models/companion_command.rs`（或并入 models/companion.rs）；`tests/test_companion.rs` 增补。
**桌面修改**：`companion_connection.rs`（Command 臂 + dispatcher 字段 + try_enqueue_single）、`lib.rs`（装配 + 注册）、`commands/suggestion.rs`（pub 化受控例外 A）。既有 services/ 其他文件零改动。
**Android 新增**：`command/` 包（CommandModels.kt / CommandChannel.kt / StreamCoordinator.kt）+ 对应 test。
**Android 修改**：RealConnectionClient（CommandSender 实现 + 帧分发 + 出站 pump）、AppModelContainer（装配 + 二态暴露）、AppNavHost（VM 工厂注入 commandSender/coordinator）、ChatViewModel/TasksViewModel/MemoryViewModel 及关联 Screen（状态映射，UI 布局零改动）。
**协议 crate / relay-server**：零改动（git diff 佐证）。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-13.3（L3398-3440）]
- [Source: _bmad-output/planning-artifacts/epics.md#移动伴侣增量 NFR/UX-DR（L3008-3042）]
- [Source: _bmad-output/planning-artifacts/architecture.md#Incremental-Core-Architectural-Decisions—手机伴侣基建（L2025-2293，决策#4 指令注入点/四条硬边界/Integration Points 数据流）]
- [Source: _bmad-output/implementation-artifacts/13-1-desktop-snapshot-engine-and-state-push.md（裁决 1/2/8、出站通道语义冻结、范围外清单）]
- [Source: _bmad-output/implementation-artifacts/13-2-mobile-realtime-snapshot-view.md（§1 冻结契约/§5 mock 对照/§7 测试基建/§8 收口项）]
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-…（L95 禁区/L113-116 检查点 S/L148-150 测试范围）]
- [Source: egosync-app/src-tauri/src/services/companion_connection.rs（enter_session :765、帧分发 :845-875、enqueue_outbound :160、set_snapshot_request_tx 先例 :147）]
- [Source: egosync-app/src-tauri/src/services/companion_snapshot.rs（notify_signal :628、WRITE_SIGNAL_EVENTS :739、llm:stream 特判 :787-811、ChunkEnvelope 先例）]
- [Source: egosync-app/src-tauri/src/commands/chat.rs（chat_send_message :222、chat_stop_streaming :558、chat_new_conversation :622、chat_delete_conversation :585、State 直调手法 :294/351）]
- [Source: egosync-app/src-tauri/src/commands/task.rs（task_toggle_complete :131、task_create :24）]
- [Source: egosync-app/src-tauri/src/commands/suggestion.rs（私有 confirm_and_create_task :32 / reject_with_reason :61、缺 emit 缺口）]
- [Source: egosync-app/src-tauri/src/services/memory_query.rs（pub 注入对象 :9-94）]
- [Source: egosync-app/src-tauri/src/models/chat.rs（StreamPayload :50-69、MessageProcessEvent :36-48）、error.rs（AppError 序列化 :38-62）]
- [Source: crates/companion-proto/src/frames.rs（opaque payload :65-91、编解码 :99-144）、lib.rs（PROTOCOL_VERSION=1、变更流程）]
- [Source: egosync-app/src-tauri/tests/test_companion.rs（helper 清单：phone_pair_and_connect :1104、phone_receive_snapshot :1042、setup_listener_with_engine :881）]
- [Source: companion-android/app/src/main/java/com/egosync/companion/（ConnectionClient.kt:55-71、RealConnectionClient.kt:524-563、FrameConsumer.kt、ChatViewModel.kt、TasksViewModel.kt、MemoryViewModel.kt、AppModelContainer.kt、SnapshotMapper.kt:105）]

### Review Findings

- [x] [Review][Patch] 幂等缓存 TOCTOU 竞态：同 commandId 并发到达时缓存查/插之间存在 await 窗口，双任务均 miss 并各自执行 — **用户裁决 B：per-commandId 排队，第二条等待首次完成并共享同一 ack**（inflight 表 + 双检缓存；已实现并补并发排队测试）[companion_dispatch.rs:execute]
- [x] [Review][Patch] STREAM_TOKEN 每 token 独立 tokio::spawn 无顺序保证：多线程调度下 token 乱序、done 帧可超前于在途 token — **用户裁决 A：排队保序**（listener 同步 try_send 进 256 容量中转通道，单消费者任务串行 try_enqueue_single；listener 零 await 纪律保持）[companion_dispatch.rs:register_stream_mirror]
- [x] [Review][Patch] COMMAND 臂无并发上限：每帧无条件 spawn，配对设备可无界倾泻重操作 — **用户裁决 A：全局信号量闸门（上限 8）**，超限回 ConnectionError「指令过载」错误 ack 不断连（`with_command_limit` 测试缝注入覆盖）[companion_dispatch.rs:execute/COMMAND_CONCURRENCY_LIMIT]
- [x] [Review][Patch] 写信号 try_send 错误被 `let _ =` 静默吞掉，通道满/关闭时无日志 [companion_dispatch.rs:route → tracing::warn]
- [x] [Review][Patch] 幂等测试空转：`FakeExecutor.calls` 从未被断言，串行重放测试在无缓存时也必然通过 [test_companion.rs — 重放/淘汰用例补 call_count 断言（1 / 1002）]
- [x] [Review][Patch] COMMAND_DATA_MAX_BYTES=65519 跨 crate 硬编码副本，与 companion-proto MAX_PLAINTEXT_LEN 无编译期绑定 [models/companion_command.rs → 直接引用 companion_proto::crypto::MAX_PLAINTEXT_LEN]
- [x] [Review][Patch] memory.list offset 未校验（limit 有 clamp，offset 裸传 SQL）[companion_dispatch.rs:route_memory_list → offset.max(0)]
- [x] [Review][Patch] ack JSON 可能超 65519 帧上限（memory.list 200 条/超长结果），编码层拒绝致断连 [companion_dispatch.rs:execute 出口尺寸守卫 + 超限测试]
- [x] [Review][Patch] action 字段未限长即入日志（恶意 60KB action 串）[companion_dispatch.rs → loggable_action 截断 64 字符]
- [x] [Review][Patch] chat.send content 空串/纯空白未校验（可创建空消息并触发空 LLM 流）[companion_dispatch.rs:route 前置校验 + 测试（执行器零触达断言）]
- [x] [Review][Patch] KNOWN_ACTIONS 与执行器 match 双清单，未来新增 action 漏加执行器臂将运行期矛盾 — ProdAction 枚举化（编译器强制穷尽）+ 对齐覆盖测试 [companion_dispatch.rs]
- [x] [Review][Patch] dispatcher 未就绪分支的失败回执入队结果被忽略且无日志 [companion_connection.rs:None 分支 → debug 日志]

### Review Findings — Chunk B（Android command/ 包，2026-08-31）

- [x] [Review][Patch] ack 超时 15s 与桌面慢执行组合伪超时→重复执行 — **用户裁决 A：超时提高至 60s**（真断连由 unbind 快速失败兜底）[CommandChannel.kt:DEFAULT_ACK_TIMEOUT_MS]
- [x] [Review][Patch] execute/unbind TOCTOU：注册 pending 后复查绑定再 send（failAllPending 清不到注册前条目的窗口关闭）+ 交错测试 [CommandChannel.kt:execute]
- [x] [Review][Patch] 裸 catch TimeoutCancellationException 偷换外层取消信号 + send 不在超时覆盖内 → withTimeoutOrNull 包 send+await [CommandChannel.kt:execute]
- [x] [Review][Patch] bind() 覆盖旧绑定不 close 旧通道（pump 协程泄漏）→ 覆盖/断开时 close + warn [CommandChannel.kt:bind/unbind/shutdown]
- [x] [Review][Patch] encode 前自校验：paramsJson 合法 JSON + envelope ≤65519，本地快速失败（否则等桌面 serde 拒绝走完超时往返）[CommandModels.kt:encode]
- [x] [Review][Patch] **done 流残留后新流错误归属旧 conversationId**（快照门错开/渲染错挂会话）→ 归属键一律用事件自身 conversationId + done→跨会话测试 [StreamCoordinator.kt:onStreamToken]
- [x] [Review][Patch] streamStarting 跨会话替换活跃流前不 flush 暂存（与 onStreamToken 路径不一致）→ 覆盖前 flushStash [StreamCoordinator.kt:streamStarting]
- [x] [Review][Patch] streamEnded 无条件 flush 击穿快照门（迟到/无关收口掐掉正在流式的气泡）→ 仅无活跃流时 flush + 测试 [StreamCoordinator.kt:streamEnded]
- [x] [Review][Patch] 切换查看会话后旧暂存快照滞留回放覆盖新快照（UI 回退）→ onViewedConversation 变化即 flush + 测试；stashed 跨线程读写加 stashLock [StreamCoordinator.kt]
- [x] [Review][Patch] fromErrorJson 多键 error 任取首键（分类建立在不确定值上）→ 多键即降级 ValidationError + 测试 [CommandModels.kt]
- [x] [Review][Patch] 空 commandId 回执静默吞 → warn 留痕（区分「桌面拒绝」与「回执丢失」）[CommandChannel.kt:onCommandResult]
- [x] [Review][Patch] ack/StreamEvent 不校验 schemaVersion（未来 v2 误解析）→ 漂移显式拒收 + 测试 [CommandModels.kt]
- [x] [Review][Patch] resultOrThrow ok=true 缺 result 伪造空对象 → 显式抛 ValidationError + 测试 [CommandModels.kt]
- [x] [Review][Patch] 溯源块 id 用 identityHashCode（可碰撞/不稳定）→ AtomicLong 计数器 [StreamCoordinator.kt:toTraceBlock]
- [x] [Review][Patch] 「未知 commandId 忽略」零断言恒真测试 → 注册真 pending + ghost 回执不误完成断言 [CommandChannelTest.kt]
- [x] [Review][Defer] actionType 子串分类脆弱（contains("read") 顺序匹配）[StreamCoordinator.kt:toTraceBlock] — deferred, 对照桌面前端映射表后统一
- [x] [Review][Defer] 超时重试不复用 commandId 的重复执行组合窗口 [CommandSender doc 已声明] — deferred, 14.1 速记队列收口（已通过 60s 超时缓解）

### Review Findings — Chunk C（Android VM/UI 换装，2026-08-31）

- [x] [Review][Patch] **停止不取消发送协程——僵尸流复活**：stopStreaming 不 cancel streamJob，ack 等待窗口点停止后 ack 返回照常 streamStarting+重臂看门狗 → 先掐协程再收口 + 交错测试 [ChatViewModel.kt:stopStreaming]
- [x] [Review][Patch] **断连/收口路径依赖查看会话**：sawStream 在 viewing 门之后置位、done 帧被 viewing 门整吞、finalize 无条件 persist 查看会话（看门狗超时误伤）→ sawStream 前置 + 非查看会话 done 段落落库（appendStreamSegments）+ finalize 仅在有已渲染流时触碰消息 + 测试 [ChatViewModel.kt:onStreamState/finalizeStreamLocally]
- [x] [Review][Patch] adoptChatIds 读切换后 activeRoleId（与 newConversation 捕获值自相矛盾）→ 传发送时捕获 roleId [ChatViewModel.kt:adoptChatIds]
- [x] [Review][Patch] stream-$i 兜底 id 跨流重复（LazyColumn key 冲突）→ streamSeq 单调序号入 id [ChatViewModel.kt:renderStream]
- [x] [Review][Patch] conversation.new 失败静默吞 → onError 显式反馈（与 Tasks/Memory 方针一致）[ChatViewModel.kt:newConversation]
- [x] [Review][Patch] **new/chat.send 并发桌面双会话孤儿**：本地 id 已被 chat.send 采纳后 new 的 ack 落空 → adoptConversationId 返回采纳结果，落空即 conversation.delete 回收 + 竞速测试 [ChatViewModel.kt]
- [x] [Review][Patch] 建议回执硬编码「产品经理」与真实建议来源角色错位 → 随卡片 fromRole 动态 [ChatViewModel.kt:respondActionCard]
- [x] [Review][Patch] **闪断重连流式门永扣**：startSessionLoop finally 只 unbind 不 reset，重连后所有快照无限暂存 UI 冻结 → finally 补 streamCoordinator.reset()（顺带 flush 暂存）[RealConnectionClient.kt:startSessionLoop]
- [x] [Review][Patch] **toggle 回滚盲翻而非基线恢复** + 在途翻转被无关 STATE_DELTA 旧态覆盖 + 连点交错 → inFlightToggles 表（基线恢复/连点守卫/快照重建保持乐观态）+ 2 测试 [TasksViewModel.kt]
- [x] [Review][Patch] **task.create 双卡竞态与缺 id 滞留**：快照先含桌面 id 时替换出重复卡；ack 缺 id 乐观卡永久滞留 → 先到去重 + 缺 id 移除并显式报错 + 2 测试 [TasksViewModel.kt:replaceWithDesktopTask]
- [x] [Review][Patch] **分类收敛前提错误（D1-A）**：桌面未指定象限默认落库 Q2（db/tasks.rs:385），「象限脱离 Q3」判据 2s 内误清「分类中」而 LLM 最长 12s → 占位改 Q2 对齐桌面默认、收敛仅靠 15s 超时、未知象限不造假映射（保留占位）、classifyingIds 存在性过滤；测试按真实桌面语义重写 [TasksViewModel.kt]
- [x] [Review][Patch] Memory reload 无并发防护（旧回执覆盖新筛选）→ reloadJob 取消旧查询 + 测试 [MemoryViewModel.kt:reload]
- [x] [Review][Patch] 来源失败双重反馈且无可点重试 → 内联错误区 + 重试入口（retrySources），去掉 snackbar 叠发 + 测试 [MemoryScreen.kt/MemoryViewModel.kt]
- [x] [Review][Patch] **测试恒真**：`临时id序列唯一` advanceUntilIdle 后断言 fake 桌面 id 而非乐观临时 id → 同步读 local- 前缀临时 id 断言 [TasksViewModelTest.kt]
- [x] [Review][Patch] **StateFlow 事件吞重复错误**（连续同值去重）→ SharedFlow(extraBufferCapacity=16) 事件流，MainActivity 逐条收集 [AppModelContainer.kt/MainActivity.kt]
- [x] [Review][Patch] **13.2 守护测试净删 11 个（D2-A）**：补回 10 个快照/会话管理守护（新建设当前、切换换消息流、删当前回退、删光兜底、标题生成、同对象首发、二次快照刷新、冷启动兜底、快照删当前回落清卡、角色被删回退管家）[ChatViewModelTest.kt 19→21 用例]
- [x] [Review][Patch] 完成记录不准（记 9 用例实际 8）→ 修正（评审后 21 用例）[story Completion Notes]
- [x] [Review][Defer] conversation.delete 失败不回滚本地（下次 STATE_DELTA 会话复活，最终一致；onError 已反馈）— deferred
- [x] [Review][Defer] toggle 超时回滚 vs 已收敛快照竞态（60s 超时 + 写信号已入库的组合窗口）— deferred，与 Chunk B「commandId 非复用」同源，14.1 收口
- [x] [Review][Defer] 解析失败 ack 恒用空 commandId：超限 envelope 手机 pending 无法按 id 关单，仅恶意客户端可达（手机编码层自校验），建议边界登记 [models/companion_command.rs:parse] — deferred, 协议边界特性
- [x] [Review][Defer] route 执行无超时包装：手机 120s 看门狗覆盖用户体验，reqwest/sqlx 层有自身超时防无限挂起，桌面侧超时为额外防御层 [companion_dispatch.rs:execute] — deferred, 纵深防御


### Open Questions（待用户确认，不影响开工，均已有默认方案）

1. **简报行动点确认**：桌面无结构化行动点数据（Briefing 仅自由文本，快照 actionPoints 恒空）——epics AC 列了「确认行动点」但无数据承载。默认 = 登记 deferred 待桌面行动点 schema 落地后接线（§9.5）。备选 = 本 story 顺带在桌面侧新增行动点解析/存储（超「既有 services 零改动」边界，不推荐）。
2. **桌面发起流的手机实时呈现**：默认 = STREAM_TOKEN 镜像全部流，但手机仅渲染当前查看会话，未查看会话在 done 后经快照收敛（不做远程实时桥接渲染）。备选 = 未查看会话也实时渲染（跨设备实时桥接完整语义，状态机复杂度显著上升）。

### Agent Model Used

GLM-5.3（DeepSeek Harness / Amelia persona）

### Debug Log References

**桌面（T1-T4）**：
- `AppError` 非 `Clone` → FakeExecutor 首版 `result.clone()` 编译失败；重构为闭包 `respond: Box<dyn Fn() -> Result<Value, AppError>>` 按调用重建错误值
- `envelope_rejects_missing_command_id`：missing-field 被 serde 拦截（错误消息非「指令 ID」）；测试改为断言 ValidationError + 补 blank（"  "）字符串用例命中显式校验
- `try_enqueue_single_drops_when_full_without_disconnect` 两次失败：① PONG 由帧循环直发绕过 outbound 通道、赶超排队帧；② marker 重试死循环（phone 不读 → 通道不排空）。修法：并发 reader 任务排空至 done marker + 并行重试入队 + PING/PONG 存活断言（25s，绿）
- test_companion.rs 追加时截断于 1608 行中函数 → 重读边界补回闭合段

**Android（T5-T9）**：
- CommandChannel 首版 `bind()/unbind()` 暴露 internal Binding 报「public 函数暴露 internal 类型」→ 改 internal fun；JSONObject import 遗漏
- 会话 when 分支需补 `is Frame.Command`（防御性忽略——手机不收桌面指令）
- **StreamCoordinator 空段 bug（测试抓到）**：`streamStarting` 预置空 segments，`foldInto` 首个 token 落段时活跃段不存在 → 累加丢失。修法：无未封段时补建首段
- **`fromErrorJson` 非字符串值崩溃（测试抓到）**：`{"a":1}` 形状下 optString 取 "a" 变体名而非降级 → 值非 String 即降级 ValidationError
- **ChatViewModel ack 窗口误判断连（测试抓到）**：state=null 在 ack 前等待期即触发「连接中断」误报 → `sawStream` 门（见过流态才断连收口）
- **stop/超时后 done 回声二次收口（测试抓到）**：看门狗/停止本地收口后，流终态 done 帧回流误触发建议现查与重复收口 → `closedStreamConv` 回声记忆（新流开启即清）
- **虚拟时间陷阱（测试方法论）**：`advanceUntilIdle` 会烧掉 120s 看门狗/15s 分类兜底 → 流式窗口内一律 `runCurrent()`，仅超时用例显式 `advanceTimeBy(121_000)`
- **deleteConversation 非当前会话不发指令（测试抓到）**：早 return 跳过 `conversation.delete` → 提取 `notifyConversationDeleted` 两路径共用
- 测试 helper tokenJson 手拼 JSON 逗号错位（messageId 缺分隔）→ 5 个折叠状态机用例连锁失败，修正分隔符
- MemoryViewModelTest：task_status 记忆在 `memories` 全量列表中（仅 `visibleMemories` 过滤）——断言对象写错，改为双列表分断

### Completion Notes List

- **全部 10 任务完成**；AC1-AC7 全实现（AC6 为约束遵循型 AC，见下）
- **桌面验证**：`npm run test:all` 全绿——lib 837 passed + 集成 test_companion 33 passed（含 Story 13.3 新增 8 用例：路由+memory 真池、幂等串行重放、缓存 1000 淘汰、错误 ack 形状、suggestion 确认写信号、mirror 纯函数、真链路 memory.list 往返、真链路 chat.send+token 顺序、单帧满丢弃不断连）
- **Android 验证**：`./gradlew :app:testDebugUnitTest` 190 tests 全绿 + `:app:assembleDebug` 绿。新增 command/ 包测试 3 文件（envelope/ack/token wire 契约、通道 pending/超时/断开、折叠状态机+快照延后门）、ChatViewModelTest 重写（8 用例：指令发送+id 对齐、失败反馈、token 状态机、停止、看门狗、快照暂存收敛、多会话归属、删除通知）、TasksViewModelTest 重写（8 用例：乐观回滚、id 替换、分类收敛、同一性守卫、unpair 自愈）、MemoryViewModelTest 新建（6 用例：现查/task_status 过滤/错误重试/来源懒加载/遗忘失败保留）。评审后：command/ 包 +9 用例（199）、VM 层 +19 用例（ChatViewModelTest 21 / TasksViewModelTest 12 / MemoryViewModelTest 8，合计 218 全绿）。
- **测试暴露并修复的实现 bug（红→绿价值佐证）**：StreamCoordinator 空段丢失、fromErrorJson 形状降级、ack 窗口误断连、done 回声二次收口、非当前会话删除漏发指令——全部先红后绿
- **已批准偏差（§9 登记 + 实施中发现）**：① 简报行动点确认 deferred（桌面无结构化数据，§9.5）；② 桌面发起流仅渲染查看中会话（§9 open question 默认案）；③ **MemoryCard 来源列表**：story 原文「组件零改动复用（13.2 已预留）」经核实不准确——13.2 未预留来源槽位，本 story 为 MemoryCard 补 `sources/sourcesLoading` 参数 + `SourceMessageRow`（已列 §9 偏差表）；④ MemoryScreen 旧占位文案「记忆通道尚未接通」替换为加载/错误/空三态（story T8「占位态文案移除」的落地形态）
- **既有 services 零改动佐证**：`git diff` 中 `src-tauri/src/services/` 仅 `companion_connection.rs`（例外 B：dispatcher 字段+set_dispatcher+try_enqueue_single+Command 帧 arm）、`mod.rs`（模块注册 1 行）、新增 `companion_dispatch.rs`；`commands/` 仅 `suggestion.rs`（例外 A：两 fn pub 化）。协议层 `crates/companion-proto/` 零触碰、PROTOCOL_VERSION=1 未 bump
- **ConnectionClient 六成员签名零改动**（UX-M2 契约保持）：CommandSender 为新增独立接口，RealConnectionClient 追加实现；ChatDemoData 已移出生产路径（委派关键词本地路由、打字机、4s Q2 mock、seedTask 全部移除）
- **日志纪律（NFR-M7）**：双端只记 action/帧类型/计数（桌面 dispatch 记 command_id/action；Android `Companion/Command`、`Companion/Stream` tag 同规则），无 plaintext 落日志
- **无设备验证边界（如实声明，惯例同 12.4/13.2）**：真机/真桌面网络路径（NSD 发现→Noise 握手→指令往返的端到端时延与丢帧表现）未在本环境验证；Android 单测覆盖至 Frame 编解码与 VM 状态机层，桌面集成测试覆盖真 socket 双向流。建议真机冒烟清单：配对→发消息看流式→停止→勾任务→查记忆展开来源→遗忘
- **幂等边界重申**：缓存 in-memory，桌面重启清空（§9 已声明——仅「手机重发旧 commandId + 桌面恰在两窗口间重启」双小概率组合下重复执行）

### File List

**桌面新增**：
- `egosync-app/src-tauri/src/models/companion_command.rs`（CommandEnvelope/CommandAck + 单测）
- `egosync-app/src-tauri/src/services/companion_dispatch.rs`（CompanionDispatcher/AckCache/CommandExecutor 缝/mirror_stream_payload/register_stream_mirror）

**桌面修改（受控例外）**：
- `egosync-app/src-tauri/src/services/companion_connection.rs`（例外 B：dispatcher 字段/set_dispatcher/try_enqueue_single/Frame::Command arm）
- `egosync-app/src-tauri/src/commands/suggestion.rs`（例外 A：confirm_and_create_task/reject_with_reason pub 化）
- `egosync-app/src-tauri/src/services/mod.rs`、`models/mod.rs`（模块注册）
- `egosync-app/src-tauri/src/lib.rs`（companion 装配：dispatcher 构造 + set_dispatcher + register_stream_mirror）
- `egosync-app/src-tauri/tests/test_companion.rs`（13.3 测试节：FakeExecutor + 3 helper + 8 用例）

**Android 新增**：
- `companion-android/app/src/main/java/com/egosync/companion/command/CommandModels.kt`（envelope 编码/CommandException/CommandAck/StreamEvent 解析）
- `companion-android/app/src/main/java/com/egosync/companion/command/CommandChannel.kt`（CommandSender 接口/出站 pump Binding/pending 表/超时/断开失败）
- `companion-android/app/src/main/java/com/egosync/companion/command/StreamCoordinator.kt`（DesktopStream 折叠状态机 + 快照延后门）
- 测试：`command/CommandModelsTest.kt`、`command/CommandChannelTest.kt`、`command/StreamCoordinatorTest.kt`、`command/FakeCommandSender.kt`、`ui/memory/MemoryViewModelTest.kt`

**Android 修改**：
- `companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt`（commandChannel/streamCoordinator 注入 + 会话 pump + CommandResult/StreamToken 分发 + CommandSender 实现 + unpair shutdown/reset）
- `companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt`（装配 channel/coordinator/commandSender 二态暴露 + deliverSnapshot 包裹 + showEvent）
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt`（T6 换装：chat.send/stop + ack id 对齐 + token 状态机 + 看门狗 + 会话生命周期指令 + suggestion 现查/确认/拒绝）
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt`（T7 换装：乐观翻转回滚 + task.create id 替换 + 分类 STATE_DELTA 收敛/15s 兜底）
- `companion-android/app/src/main/java/com/egosync/companion/ui/memory/MemoryViewModel.kt`（T8 换装：memory.list/sources/forget 现查 + loading/error/重试）
- `companion-android/app/src/main/java/com/egosync/companion/ui/memory/MemoryScreen.kt`（加载/错误/空三态 + MemoryCard sources 参数〔偏差③〕+ SourceMessageRow）
- `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt`（三 VM 工厂换装装配）
- 测试重写：`ui/chat/ChatViewModelTest.kt`、`ui/tasks/TasksViewModelTest.kt`

### Change Log

| 日期 | 变更 | 作者 |
|---|---|---|
| 2026-08-27 | Story 创建（13.1/13.2 上下文 + 双路代码侦察 + 8 接缝核实） | Winston（architect）+ Amelia（dev 侦察） |
| 2026-08-27 | T1-T4 桌面实现与测试全绿（dispatch/幂等/mirror/集成） | Amelia |
| 2026-08-27 | T5-T10 Android 实现、测试全绿、story 收口（Status: done） | Amelia |
| 2026-08-31 | Chunk A（桌面 Rust）三路评审：3 decision + 9 patch + 2 defer + 11 驳回；12 项 patch 全部整改，`npm run test:all` 全绿（438 前端 + 839 lib + 37 集成 + 1 proto） | Amelia（code-review） |
| 2026-08-31 | Chunk B（Android command/ 包）三路评审：1 decision + 14 patch + 2 defer + 9 驳回；15 项全部整改，`testDebugUnitTest` 199 全绿 + `assembleDebug` 绿 | Amelia（code-review） |
| 2026-08-31 | Chunk C（Android VM/UI）三路评审：3 decision（D1-A 占位 Q2 / D2-A 补回守护 / D3-B 保留字段登记偏差）+ 17 patch + 2 defer + 7 驳回；全部整改，`testDebugUnitTest` 218 全绿 + `assembleDebug` 绿 | Amelia（code-review） |
