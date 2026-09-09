# 流式正文协议契约（CAP-6 契约）

> 根因（Confirmed，High 置信，Finding 8/9——不得重新猜测，也不得以缓冲/动画掩盖）：
> 桌面所有可见正文 token 经 `emit_stream_token(thinking=false)` 发射，硬编码 `phase=Some("answering")`（`agent_engine.rs:209-233`）；镜像路径保留字段逐帧转发（`companion_dispatch.rs:668-711`）；手机解析器保留 phase（`CommandModels.kt:127-183`）却仅在 `!thinking && phase == null` 时累加文本（`StreamCoordinator.kt:166,198`）。token 到达被语义过滤 → 空段建立 + thinking 关闭（`StreamCoordinator.kt:208` token 非空即关 thinking，即使文本未累加）→ UI 只剩 `▍` 空光标（`ChatScreen.kt:655-661`）→ done 后最终快照整段回填。
> 既有测试用 `phase=null` 辅助函数自洽（`StreamCoordinatorTest.kt:22-65`、`ChatViewModelTest.kt:155-175,269-304`），证明的是手机内部规则自洽而非跨端兼容——测试改造是本契约的一部分。

## 1. 正文判定契约（单一函数）

Android 端新增唯一判定函数，`foldNew` 与 `foldInto` 共用（消除两处 `phase == null` 判定）：

```kotlin
/** 桌面正文 token：thinking=false 且 phase 为 null（历史/测试兼容）或 "answering"（生产）。 */
fun isAnswerText(event: StreamEvent): Boolean =
    !event.thinking && (event.phase == null || event.phase == "answering")
```

落点：

- `foldNew`（`StreamCoordinator.kt:164-177`）：`isText` 判定改用该函数。
- `foldInto`（`StreamCoordinator.kt:179-219`）：文本累加条件（`:198`）与 thinking 关闭条件（`:208` 的正文分支）改用该函数；tool/process 分支维持独立（`phase == "tool" || phase == "process"` 出现同样关闭 thinking 态，现状语义保留）。

桌面发射端 **不动**：`phase="answering"` 保留（可观测语义），修复方向为手机兼容（调查 D.1 推荐 + 用户裁决固定）。

## 2. 误归类防护（必须逐条满足）

| 事件 | 不得归为正文 |
| --- | --- |
| `thinking=true`（phase="thinking" 或 statusText="思考中..."） | thinking 态由 ThinkingBubble 承载 |
| `phase="tool"`（含 toolName/statusText） | 工具标题/状态行承载 |
| `phase="process"`（processEvent 三型） | ExecutionTrace 溯源块承载 |
| `done=true` 且 token 为空 | 收口事件，不创建/追加文本 |
| 未知 phase 值（未来桌面扩展） | 保守忽略文本累加（fail-safe：不崩溃不误显示），不改变 thinking 态 |

## 3. 空光标气泡禁止

- 无可见文本时不得创建纯 `▍` 空气泡：`ChatScreen` 流式段消息渲染（`:655-661`）在 `message.text` 为空且无任何已完成内容时不产生消息项；思考期由既有 `ThinkingBubble`（`uiState.thinking`）承载，不在消息列表里放空占位。
- `streamStarting` 预置的空段列表（thinking=true 初始态）维持——它对应"等待首 token 的思考展示"，不是文本气泡；首个 answering token 经 §1 判定落段后即非空。
- 防回归断言：契约测试中 thinking→answering 首帧后 UI 态不得出现"text 为空的 streaming 消息项"。

## 4. 黄金契约测试（跨端）

目标：锁定「桌面真实 payload 形状 → Android `StreamEvent.parse` → `StreamCoordinator` 聚合」全链，防止再次出现自洽但错位的契约（Finding 9 的失败模式）。

- **Fixture 来源**：桌面生产发射路径的真实 payload JSON，置于 Android 测试资源（如 `app/src/test/resources/streaming/*.json`）；每个文件头部注释锚定来源（桌面 file:line、导出方式与日期）。禁止手写"想当然"形状。
- **覆盖矩阵**（每个用例一个 fixture，断言聚合结果而非只断言不崩溃）：

| 用例 | 输入形状 | 关键断言 |
| --- | --- | --- |
| answering 多 token | N 个 `phase="answering"`、thinking=false、同 messageId | 文本逐 token 累积（中间态逐字可观测）；thinking 关闭 |
| thinking token | `thinking=true`、`phase="thinking"`、statusText | 无文本段；thinking=true |
| thinking→answering 切换 | 先 thinking 后 answering | 首个正文 token 落段、非空气泡 |
| tool | `phase="tool"`、toolName/statusText | 无文本累加；toolTitle 出现；thinking 关闭 |
| process 三型 | processEvent（thinking/narration/其他工具型） | traceBlocks 累积；无文本累加 |
| done | `done=true`、token 空 | done 收口；不新增空文本段 |
| 多 messageId | 两段不同 messageId 的 answering 序列 | 段切换：前段 sealed、后段新段 |
| 多 conversationId | 流切换到另一会话 | foldNew 重建归属（既有整改语义保持） |
| phase=null 兼容 | 旧形状（无 phase 字段） | 仍按正文累加 |

- **既有测试改造**：`StreamCoordinatorTest`/`ChatViewModelTest` 的 token 辅助函数默认值改为生产形状（`phase="answering"`）；`phase=null` 只在显式兼容用例中出现——使测试编码的契约与生产一致。
- **桌面侧锚定（必做——用户裁决 A+B 双轨）**：桌面新增小型 Rust 测试锁定 `llm:stream` StreamPayload 的 phase 值域（answering/thinking/tool/process/null 语义），fixture 注明由该测试锚定——桌面漂移时双端同时红灯。

## 5. 明确不在本根因修复内（独立健壮性事项，防混修）

以下事项记录于 deferred-work（另案），不得借本修复夹带：

- 镜像/出站有界通道 `try_send` 拥塞丢帧（`companion_dispatch.rs` mirror 通道）；
- done 帧丢失的静默风险（手机看门狗 120s 兜底已存在，`ChatViewModel` watchdog）；
- 上游 Provider 以单个大 token 发出累计正文。
- 修复本根因后若流式仍有卡顿/断流，再依调查 Diagnostic 4（received/enqueued/dropped/sent 计数，不含正文内容）另案取证。
