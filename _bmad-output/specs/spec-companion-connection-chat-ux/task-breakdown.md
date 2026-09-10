# 任务拆分、依赖与实施顺序（P0/P1）

> 原则：按机制拆分（不按症状打补丁）；统一状态模型（state-and-recovery-model.md）贯穿 T-S3/T-S4/T-S5；每任务外科手术式修改、可独立验收、独立提交（`<type>(scope): 中文描述`）。
> 文件均为「改动落点」，不重列只读参照。行号为调查时锚点，实施时以实际代码为准。

## 任务总表

| ID | 优先级 | 机制 | 主要落点（Android：companion-android/app/src/main/java/com/egosync/companion/） | 依赖 | 验收锚点（test-matrix.md #） |
| --- | --- | --- | --- | --- | --- |
| T-S1 | P0 | 流式 phase 契约 | `command/StreamCoordinator.kt`（共用判定函数）；`ui/chat/ChatScreen.kt`（空光标守卫）；测试：`command/StreamCoordinatorTest.kt`、`ui/chat/ChatViewModelTest.kt`、新增 `resources/streaming/*.json` 契约测试；桌面：`src-tauri` 新增 StreamPayload phase 值域锁定测试（fixture 锚定，A+B 裁决） | 无 | #9 + streaming-protocol.md §4 矩阵 |
| T-S2 | P0 | 命令就绪接线 | `command/CommandChannel.kt`（sessionActive → StateFlow）；`connection/RealConnectionClient.kt`（commandReady 转发 + bind/unbind 时序）；`ui/AppNavHost.kt`（Tasks/Memory/Onboarding（2026-09-10 裁决失效：已随手机端引导流移除——spec-companion-android-remove-onboarding）/NotificationCenter 路由 enabled 改绑——Chat 发送由 T-S10 队列化）；`AppModelContainer.kt`（flush 守门判据先行改为 commandReady && paired，载体 T-S10 换待发箱） | 无（可与 T-S1 并行） | #8、#11（flush 部分） |
| T-S3 | P0 | 统一状态模型 + 非阻断 UX | `connection/ConnectionClient.kt`/`ConnectionStateMachine.kt`/`RealConnectionClient.kt`/`FakeConnectionClient.kt`（三面模型 + grace）；`ui/AppNavHost.kt`（移除遮罩挂载、紧凑指示、Degraded 横幅）；`ui/components/DegradedOverlay.kt`（阻断形态与速记条一并退役删除）；`ui/settings/SettingsScreen.kt`（Debug 六档） | T-S2 | #1、#2 |
| T-S4 | P0 | 凭据生命周期与恢复闭环 | `connection/KeyStore.kt`（loadExisting/createForPairing）；`connection/RealConnectionClient.kt`（调用点分类、冷启动检测、恢复事件）；`AppModelContainer.kt` + `ui/AppNavHost.kt`（原子协调 + 导航 + 原因展示）；测试：`RealConnectionClientOrchestrationTest.kt` 扩展 | T-S3 | #3、#4、#5、#11 |
| T-S5 | P0 | 结构化配对失败 + 手机分类 | 桌面：`src-tauri/src/services/companion_connection.rs`（两拒绝点发 Notice）；Android：`connection/RealConnectionClient.kt`（配对 probe 识别 + 会话循环识别 + PairingProgress.Failed 结构化）；`pairing/PairingViewModel.kt`/`PairingScreen.kt`（文案映射） | T-S4（恢复事件通路） | #3（原因部分）、#6 |
| T-S6 | P0 | 桌面 QR 生命周期 | `egosync-app/src/components/settings/CompanionPairingSection.tsx`（重新生成按钮 + 确认、paired 事件清码/已使用标记） | 无（纯桌面，可并行） | #6、#7 |
| T-S7 | P0 | 手机本地解绑文案 | `ui/settings/SettingsScreen.kt`（解绑确认文案） | 无（可与 T-S5 合并提交） | #6（文案部分） |
| T-S8 | P1 | IME 诊断 → 修复 | A：`MainActivity.kt`/`ui/chat/ChatScreen.kt`/`ui/AppNavHost.kt` 加 debug 诊断日志；B：按定案改 inset owner（`ime-diagnosis.md`） | 无硬依赖（建议 T-S3 后，避免布局改动叠加）；**A 阶段先行独立交付** | #10 |
| T-S9 | P1 | 治理收尾 | 旧冻结规格/PRD FR-43 标注 superseded（supersessions.md 清单）；队列丢帧等健壮性事项记入 `_bmad-output/implementation-artifacts/deferred-work.md` | 本规格人工批准后 | 治理项（非代码） |
| T-S10 | P0 | 对话离线待发箱（用户裁决：队列化 + 落盘） | 新增 `sync/ChatOutbox.kt`（FIFO、幂等 commandId、落盘持久化——复用 `KeystoreSnapshotCipher` 加密 + 原子写，进程被杀后恢复）；`ui/chat/ChatViewModel.kt`（!commandReady 入队 + 串行 flush 管线 + 待发态）；`ui/chat/ChatScreen.kt`（输入常可用、待发态气泡、"网络不可用，恢复后自动发送"提示）；`AppModelContainer.kt`（flush 收集器改造 + 启动恢复队列，删除 `QuickNoteQueue` 接线） | T-S2，且须在 T-S3 之后 | #8（修订版）、#11 |

## 推荐实施顺序

```
T-S1 ∥ T-S2  →  T-S3  →  T-S10 ∥ T-S4  →  T-S5
（T-S6、T-S7 任意时点并行——纯桌面/纯文案，无 Android 依赖）
T-S8-A（诊断构建）可立即并行；T-S8-B 须等诊断定案
T-S9 收尾
```

排序理由：

1. **T-S1/T-S2 先行止血**：两任务小、独立、不动状态范式——流式与"不可达"是用户最高频可感知故障，可单独发布回归。
2. **T-S3 是范式地基**：非阻断 UX 依赖 commandReady（T-S2）作为写操作判据；状态模型先行，失败分类、恢复事件、离线待发箱才有挂载点。T-S10 紧随 T-S3（遮罩与速记条先退役，避免"速记条 + 待发箱"两套离线录入入口并存），与 T-S4 无相互依赖、可并行。
3. **T-S4/T-S5 串行递进**：恢复事件（T-S4）是结构化拒绝触发 PairingRevoked（T-S5）的通路。
4. **T-S8 证据门控**：A 阶段（诊断）不依赖任何前置且无风险，先落；B 阶段必须以 A 的数据定案，不得跳过（AGENTS 规则十三）。
5. **T-S9 依赖人工批准**：superseded 标注属人工裁决落笔，本规格批准后执行。

## 每任务验收条件（摘要级，细粒度见各契约文档）

- **T-S1**：`phase="answering"` 多 token 单测逐字断言通过；空光标守卫断言（无 text 为空的 streaming 消息项）；`phase=null` 兼容用例通过；既有测试辅助函数默认值改为生产形状；桌面 phase 值域锁定测试落地并通过（A+B 裁决）。
- **T-S2**：`CommandChannel.sessionActive` 为 StateFlow；bind 先于在线发布 / unbind 时 commandReady=false 先行的时序单测（时间可控）通过；Chat/Tasks/Memory 写控件 enabled 断言绑 commandReady；flush 守门单测（commandReady && paired 才触发待发箱 flush——守门逻辑先行落地，T-S10 换载体）。
- **T-S3**：冷启动初态 Connecting（单测：不 advanceTimeBy 即断言非 Degraded、无遮罩门）；20s grace 耗尽 → Degraded；网络失败用例无重配 CTA 断言；全屏遮罩挂载代码移除（grep `DegradedOverlayHost` 阻断形态零残留）；Debug 六档预览可用。
- **T-S10**：!commandReady 发送入队断言（无"桌面引擎不可达"路径触发）；flush 串行（一次一条、等本轮流式 done）；commandId 跨重发稳定断言；四路径结果处理（成功删条目/连接失败留队/业务错误删条目/并发守卫忽略）；三场景队列不丢失（断网 Degraded、unpair、恢复事件原子序列）；落盘恢复断言（进程重建/冷启动后队列与待发态还原，含本地占位会话）；ChatScreen 待发态与"网络不可用，恢复后自动发送"提示断言；`QuickNoteQueue`/速记条全仓引用零残留。
- **T-S4**：已配对 + 私钥文件缺失 → 不生成新身份 + paired=false + 配对流 + 原因事件（测试缝：SecretsProvider 假实现抛 MissingSecretsException）；运行中 SecretsInvalidatedException → 恢复事件序列调用断言（shutdown/reset/clear/wipe/navigate 各一次、待发箱队列不动）。
- **T-S5**：桌面 Rust 测试——早期准入与 nonce 校验拒绝前发送 `pairingRejected` Notice（reason 正确）；Android 单测——配对 probe 收到拒绝 Notice 直接结构化失败（不进 waitDesktopConfirm）；已配对重连收到拒绝 → PairingRevoked 恢复事件。
- **T-S6**：桌面组件级/Vitest（若 harness 覆盖）+ 人工验证——显示态有重新生成按钮、确认文案含"旧码立即失效"、`companion:paired` 即清码显示"已使用"。
- **T-S7**：文案断言（解绑确认含"当前二维码不可复用"）。
- **T-S8**：A 阶段——诊断日志仅 debug 构建存在（release 无引用）；B 阶段——真机验证表（手势/三键 × 焦点开合）间距 8–12dp 无残留；诊断日志删除（grep 零残留）。
- **T-S9**：supersessions.md 清单逐项在旧文档落标注；deferred-work 有新条目。

## 统一状态模型贯穿声明

T-S3 建立的三面模型（TransportStatus / PairingHealth / commandReady）是 T-S4（PairingHealth 恢复事件）、T-S5（拒绝分类）、T-S10（待发箱 flush 消费 commandReady）的唯一挂载点；各任务不得私建平行状态（如 PairingViewModel 内部再存一份失败分类、ChatOutbox 自行观察 binding）。验收闭环以 test-matrix.md 为准，任何任务的测试须同时跑既有回归（禁止删测试改断言）。
