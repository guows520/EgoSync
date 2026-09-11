---
title: '手机端对话流式回复对齐桌面端（执行过程显示 + 气泡消失修复）'
type: 'bugfix'
created: '2026-09-11'
status: 'done'
route: 'dispatch'
baseline_commit: 'dff41c46aff561530080b9f139afb34845df2cb4'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 手机端对话回复与已确认无问题的桌面端不一致：①流式过程中执行过程（溯源块）不显示，done 后又因渲染用合成 id 而快照用真实 id 被孤儿化；②回复气泡先显示、done 时刻突然消失、约 2s 后再次出现——快照暂存门在流结束时刻回放了流中段陈旧快照（含空 assistant 占位行）。
**Approach:** 手机侧三处协同修复（桌面零改动）：暂存门 flush 前做「已含本轮完成回复」新鲜度校验（陈旧即丢弃，done 必触发桌面重建、新快照约 2s 后直通）；renderStream 用 chat.send ack 的 assistantMessageId 作普通流渲染 id（快照落地 key 稳定、trace 不孤儿化）并按 FR-29 在流式中暴露/展开溯源；委派分段条件放宽恢复 FR-1 两段气泡。

## Boundaries & Constraints

**Always:** 桌面端是唯一基线（用户已确认无问题）——行为对齐已批准规格 FR-29/FR-1（`_bmad-output/implementation-artifacts/spec-mobile-fr-parity-group1-chat.md`），本修复是规格恢复而非新设计；不破坏跨端既有契约（phase 值域、空光标守卫、快照暂存门本身）；新行为用契约测试锁定防再漂移（13.3 T6 mock→真实流漂移正因缺锁定）。
**Never:** 不改桌面端任何代码；不做快照 schema 扩展（backlog 另立）；不引入超时强制回放（连接死亡时保持本地已定文本优于回放陈旧快照）；不为变绿删测试断言——受影响测试按新契约改写并保留 WHY 注释。

## I/O & Edge-Case Matrix

| 场景 | 输入/状态 | 期望行为 | 错误处理 |
|------|-----------|----------|----------|
| 普通流式（无委派） | ack 含 assistantMessageId；全程 token messageId=null | 流式中溯源实时显示（有正文挂气泡上方、展开；无正文独立条目）；渲染 id=锚点；done 后快照落地 trace 保留、气泡不换 key | N/A |
| done 时刻陈旧暂存 | 流中段快照（占位 isComplete=false、content 空）暂存中，done 到达 | flush 校验失败→丢弃暂存（不回放空气泡）；done 写信号触发的新快照直通应用 | 断连致新快照不达：保持本地已定文本，重连后收敛 |
| 暂存已含本轮完成回复 | 暂存快照含该 assistant 完成行（isComplete=true、content 非空） | 校验通过→照常应用 | N/A |
| 委派两段（FR-1） | 首段 messageId=null；唤醒帧/二段 token messageId=Some(占位id)、phase=answering | null→Some 切换即分段：两气泡；trace 挂末段（真实 id） | N/A |
| process/tool 帧不误分段 | phase=process/tool 帧（messageId=Some）到达 null 活跃段 | 不分段（仅 answering 帧允许 null→Some 切换） | N/A |
| done 帧不误分段 | done 帧（messageId=Some）到达 null 活跃段 | 不分段（收口帧归入活跃段） | N/A |
| 停止/看门狗超时 | 流被本地收口、暂存为流中段快照 | 已浮现文本保留；陈旧暂存被丢弃 | N/A |
| ack 无 assistantMessageId | 旧桌面/兜底路径 | 渲染回退合成 id；桌面发起流由 done 帧 Some 兜底捕获锚点（仅 done 落库受益） | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/app/src/main/java/com/egosync/companion/command/StreamCoordinator.kt` — 快照暂存门 `flushStash`(:155)、分段条件 `foldInto`(:193)、`streamStarting`(:72)、`foldNew`(:164)、`reset`(:128)；M1/M2'/锚点全部落点
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt` — `ChatUiState`(:38)、`renderStream`(:783，done 分支 :817-832)、非查看 done 落库(:761)、`sendMessage` ack(:517-520)、`resendEntry` ack(:583-589)、`finalizeStreamLocally`(:839)
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt` — LazyColumn 流式区(:201-218)、`ExecutionTrace`(:532)
- `companion-android/app/src/test/java/com/egosync/companion/command/StreamCoordinatorTest.kt` — 契约测试；`SnapshotFixture`(:275) 无会话数据，3 个旧 flush 断言需按新契约补会话夹具改写
- `companion-android/app/src/test/java/com/egosync/companion/ui/chat/ChatViewModelTest.kt` — `chatSendAck`(:184，已含 assistantMessageId="m-a-d")、`tokenJson`(:160)、快照暂存收敛用例(:559，convergedSnapshot 含完成行→新守卫下仍通过)
- 桌面契约锚（只读不改）：`egosync-app/src-tauri/src/services/companion_dispatch.rs:521-530`（ack 三 id）、`agent_engine.rs:2817-2832`（slot→messageId）、`:3034/3047`（唤醒帧）、`:940-959`（done 帧 phase="done"）、`:3033/3046`（final_message_id=占位 id）

## Tasks & Acceptance

**Execution:**
- [x] `StreamCoordinator.kt` — 新增 `roundAssistantId`（`streamStarting` 传参 + 任意 Some messageId 帧捕获）；`flushStash` 新鲜度守卫（锚点消息 isComplete 且 content 非空；无锚回退会话末条 assistant；无会话视为陈旧）；分段条件新增「null 活跃段 + answering + 非 done」分支；`reset` 先按本回合上下文 flush 再清态
- [x] `ChatViewModel.kt` — 发送/重发两处 ack 捕获 assistantMessageId 传入 `streamStarting`；`renderStream`：全 null 段用锚点 id（混合段保留各自 id）、流式中挂载 trace 到末段（宿主切换清旧条目）、无宿主时置 `streamingTrace`、done/本地收口清理；非查看 done 落库同 id 规则
- [x] `ChatScreen.kt` — `ChatUiState.streamingTrace` 独立条目（置于 thinking 前）；`ExecutionTrace` 增 `initiallyExpanded` 参数（宿主=message.streaming，独立=true）
- [x] `StreamCoordinatorTest.kt` — 新契约用例：陈旧暂存丢弃/完成暂存放行、唤醒帧分段、process 帧不分段、done 帧不分段、done 帧锚点捕获；旧 flush 用例补含会话数据的夹具
- [x] `ChatViewModelTest.kt` — 流式中 trace 暴露（有/无宿主两形）、锚点渲染 id、快照落地后 trace 存活且 id 不变、委派两气泡

**Acceptance Criteria:**
- Given 流式进行中（部分文本 + processEvent 已到），when 渲染，then 流式消息 id==ack assistantMessageId 且 traceByMessageId 含该 id 溯源块（FR-29 流式中可见、默认展开）
- Given 流中段快照（占位未完成）暂存且 done 帧到达，when 收口，then 该暂存未被应用且本地文本保留
- Given 暂存快照含本轮完成 assistant，when flush，then 照常应用
- Given 首段 null token 后唤醒帧（Some id、answering、空 token、非 done）到达，when 折叠，then 产生 2 段且首段 sealed
- Given process/tool 帧或 done 帧携带 Some id 到达 null 活跃段，when 折叠，then 不分段
- Given done 后含完成行的快照落地，when onSnapshotReplaced，then trace 锚点条目保留、消息 id 不变（无气泡闪烁）

## Design Notes

锚点可靠性链：ack（companion_dispatch.rs:529）与 done 帧恒携带 assistantMessageId；final_message_id==占位 id（agent_engine.rs:3033/3046），本轮所有 Some messageId 同指一落库行——ack 最早可得。仅「全 null 段」替换 id：混合段（委派）替换会与二段真实 id 撞 LazyColumn key。
分段窄化理由：process/tool 帧也带 Some(占位id)（agent_engine.rs:844），仅按 messageId!=null 放宽会把普通流工具期误切两气泡；唤醒帧（answering+空 token+非 done）可由 isAnswerText 与 !done 区分。

## Verification

**Commands:**
- `./gradlew :app:testDebugUnitTest` -- expected: 全绿（含新契约用例）
- `./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `git status --short -- egosync-app/` -- expected: 空（桌面零改动）

## Implementation Notes

- 实现经派发子代理完成，orchestrator 以 diff 复核（893 行）后自跑验证：`testDebugUnitTest` 268 tests / 0 failures / 0 errors（含新增 13 个契约用例：StreamCoordinatorTest 19 例、ChatViewModelTest 34 例），`assembleDebug` BUILD SUCCESSFUL，`git status -- egosync-app/` 空（桌面零改动）。
- 矩阵审计补遗：第 8 行「ack 缺锚点→渲染回退合成 id」原无直接断言，补 `ack缺assistantMessageId时渲染回退合成id`（ChatViewModelTest）后 8/8 行全覆盖。
- 实现细节偏差（可接受）：`streamingTrace` 用 `List<ExecutionTraceBlock>?`（null=无）而非规格的 `emptyList()` 常态——与相邻 `streamingToolTitle: String?` 的可空约定一致。
- 评审补丁轮（三层并行评审 → 分诊见 Review Triage Log，12 项 patch 全部应用）：①ExecutionTrace remember 键改 `initiallyExpanded`（折叠语义对齐桌面）；②溯源挂载并入 renderStream 单次 update（消除每帧双发射）；③锚点生命周期修正（foldInto 仅 done 帧捕获 + foldNew 同会话重启保留 ack 锚点）；④done 残留分支补 flush；⑤streamingTrace 入两处复位清单；⑥新增 7 个评审用例（reset/切走陈旧丢弃、停止清理、重发锚点、切走落库锚点、委派收敛塌缩断言）+ 4 处 process 帧夹具转生产形状。
- 补丁轮验证：`testDebugUnitTest` **273 tests / 0 failures / 0 errors**（StreamCoordinatorTest 21 例、ChatViewModelTest 37 例），`assembleDebug` BUILD SUCCESSFUL，桌面仍零改动。
- 已知残留（真机人审项）：①FR-29 流式中展开等 UI 形态无 Compose UI 测试，需真机/预览比对桌面（deferred-work.md）；②非查看会话轮次溯源整轮丢失——先于本变更存在，属快照无 trace 域的数据通道缺口（deferred-work.md）；③无锚回退路径下，流中段快照若在占位行落库前生成，「末条 assistant」校验可能误判新鲜——规格已将该路径限定为仅 done 落库受益（矩阵行 8）；④委派唤醒帧瞬间首段气泡 key 从锚点翻转为合成兜底 id（「仅全 null 段替换」与「null 首段不得与二段真实 id 撞 key」两约束的必然推论，每轮一次、随后由收敛塌缩吸收；替代设计会在普通流 done 时引入更差翻转，接受）。
## Spec Change Log

## Review Triage Log

评审：三层并行（盲扫 / 边缘用例 / 验证缺口），diff 49kB；子代理不可续用，补丁由 orchestrator 自行应用。行合并规则：同根因归一行；verdict 引用证据。

| # | 发现（层） | verdict | 处置/证据 |
|---|-----------|---------|----------|
| 1 | ExecutionTrace `remember(blocks)` 语义：新块重置展开（覆盖用户手动折叠）且 done 不重置（「历史折叠」不生效）〔盲扫+边缘〕 | medium | **已修**：remember 键改 `initiallyExpanded`（块增长不重置、streaming→false 翻转重置——镜像桌面 useState 挂载初始化 + 完成身份切换） |
| 2 | renderStream + 挂载溯源每帧两次 `_uiState.update`（双发射双重组）〔盲扫〕 | medium | **已修**：溯源增量先算后并入单次 update；StateFlow 等值不发射核对（空 trace 分支无发射） |
| 3 | 委派两气泡收敛无测试（快照落地塌缩形态未锁定）〔盲扫〕 | medium | **已修**：`委派两段流式渲染两气泡溯源挂末段` 补收敛快照投递断言（两气泡塌缩一段、溯源存活） |
| 4 | foldNew 无条件覆写 roundAssistantId——停止后同会话重启流丢 ack 锚点〔补丁期派生，自验〕 | low | **已修**：foldNew 增 prevRound 参数，同会话保留旧锚点、跨会话不沿用 |
| 5 | foldInto 流中段捕获 Some 锚点致 null 段渲染 id 中途翻转（规格行 8「仅 done 落库受益」偏差）〔盲扫〕 | low | **已修**：仅 done 帧捕获；ack 锚点与段自身 id 不受影响 |
| 6 | done 残留分支 foldNew 替换前不 flush（与跨会话分支不变量不一致）〔边缘〕 | low | **已修**：补 flushStash（守卫按旧回合上下文判定） |
| 7 | `streamingTrace` 未入 endStreamVisuals 与解配 `!state.loaded` 复位清单（隐式顺序耦合）〔盲扫+验证缺口〕 | low | **已修**：两处显式清理 |
| 8 | reset/切走路径的陈旧暂存丢弃无直接断言〔盲扫〕 | low | **已修**：`断连reset时陈旧暂存被丢弃`、`流式中切走查看会话时陈旧暂存被丢弃` |
| 9 | 停止路径 streamingTrace 清理无测试（mutation 删除后套件仍绿）〔验证缺口〕 | medium | **已修**：`停止后流式溯源独立条目被清理` |
| 10 | 待发箱重发路径锚点传递无测试（mutation 回退后仍绿）〔验证缺口〕 | medium | **已修**：`待发箱重发ack锚点传入渲染id用锚点` |
| 11 | 非查看 done 落库锚点无 null-id 形状测试〔验证缺口〕 | medium | **已修**：`流式中切走后done落库段落id用ack锚点` |
| 12 | VM 测试 process 帧 messageId=null 偏离生产形状（生产带 Some 占位 id）〔盲扫〕 | low | **已修**：4 处夹具转生产形状 |
| 13 | 无锚回退不识别「本轮行缺席」快照（占位落库前快照误判新鲜）〔盲扫+边缘×2〕 | low | **拒绝**：规格矩阵行 8 已接受残留（无直接修正可用——手机在该路径无回合身份信息；ack 路径不受影响）；Implementation Notes 记录 |
| 14 | flushStash 锁外读回合上下文竞窗（reset 并发时新鲜暂存误丢）〔盲扫〕 | low | **拒绝**：需断连恰与 done-flush 新鲜暂存竞速；后果自愈（done 后新快照直通收敛），修正引入锁序复杂度不值 |
| 15 | 历史 SSE 中途帧（phase=null+Some+空 token+非 done+null 活跃段）误分段〔边缘〕 | false | **拒绝**：生产无该形状发射者——空 token answering 帧恰为唤醒帧发射器（agent_engine.rs:3034/3047/3803），!done 判据排除收口帧；分段触发集精确匹配 |
| 16 | ChatScreen 渲染行为无执行级测试〔验证缺口〕 | low | **defer**：deferred-work.md（无 Compose 测试基建，先于本变更存在） |
| 17 | 非查看会话轮次整轮丢失溯源〔盲扫〕 | medium | **defer**：deferred-work.md（先于本变更；桌面走 DB 回放而手机快照无 trace 域，属数据通道缺口非本变更引入） |
