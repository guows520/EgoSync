---
id: SPEC-companion-connection-chat-ux
companions:
  - state-and-recovery-model.md
  - qr-and-unbind-semantics.md
  - streaming-protocol.md
  - ime-diagnosis.md
  - task-breakdown.md
  - test-matrix.md
  - supersessions.md
  - ../../project-context.md
sources:
  - ../../implementation-artifacts/investigations/companion-connection-chat-ux-investigation.md
---

> **Canonical contract.** 本 SPEC 与 `companions:` 所列文件共同构成待实施、待测试、待验收的完整契约。`sources:` 仅为溯源审计；证据等级（Confirmed / Open）以本契约各处标注为准，实施时不得重新猜测已定案根因。

# 手机伴侣连接状态与对话体验系统性修复

## Why

已配对用户在 Android 伴侣端遭遇六类相互纠缠的体验断裂：冷启动第一帧即全屏阻断遮罩、网络失败被误导为"解除配对重扫"、本地解绑后旧二维码复用必然失败且无重新生成入口、界面正常但发送才报"桌面引擎不可达"、正文流式空光标后整段出现、键盘与输入条之间数百像素空区。调查（2026-09-08，`companion-connection-chat-ux-investigation.md`）已按证据等级定案多机制根因：连接/命令/流式为 High 置信（代码级 Confirmed），IME 精确像素机制为 Medium（症状 Confirmed、根因待真机诊断）。本规格以产品新裁决（非阻断连接体验）为纲，把六个机制收敛为统一状态模型 + 各机制外科手术式修复，供开发代理直接实施。

## Capabilities

- id: CAP-1
  intent: 非阻断连接体验——已配对用户在连接未就绪期间仍可浏览缓存快照、使用本地能力并直接在对话输入框录入消息（网络不可用时仅提示，恢复后自动续发），系统以紧凑状态指示（小图标/小状态条）呈现 Connecting/Reconnecting，取代旧 FR-43 的全屏遮罩阻断模式。
  success: 健康配对设备冷启动后 20 秒 grace 内不出现任何全屏遮罩或指针拦截层；grace 耗尽进入 Degraded 后缓存可浏览、对话输入可离线排队发送（明确"网络不可用，恢复后自动发送"提示，含数据截止时间标注）、其余依赖桌面引擎的写操作逐项禁用并说明原因；`DegradedOverlayHost` 全屏阻断形态与其速记条、`QuickNoteQueue` 一并退役。

- id: CAP-2
  intent: 统一连接/配对/凭据状态模型——系统以单一可观察状态表达连接承载（Connecting/Reconnecting/Direct/Relay/Degraded）、配对/凭据健康（含 CredentialMissing/CredentialInvalid/PairingRevoked/TrustMismatch）与命令就绪（commandReady），取代 `paired: Boolean` 与 `state !is Offline` 分裂事实源。
  success: 导航、降级提示、重配入口、控件 enabled、恢复协调全部由该模型派生；普通网络/NSD/中继失败不出现重配入口；仅凭据/信任锚/桌面授权失效显示重新配对入口并附带准确原因。

- id: CAP-3
  intent: 凭据损坏检测与恢复闭环——系统区分"首次配对创建密钥"与"已配对加载既有密钥"，凭据缺失/损坏时发布统一恢复事件并由上层原子协调清理与导航，绝不静默生成新手机身份。
  success: `loadOrCreateStaticPrivateKey` 拆分为 `loadExisting` 与 `createForPairing`（或等价显式方案）；"paired 元数据完整但私钥文件缺失"被识别为凭据损坏并进入配对流（准确原因 + 待发箱保留 + 不无限退避重试）；运行中 `SecretsInvalidatedException` 使连接、命令、流式、快照、通知、导航完整收口并显式落配对流。

- id: CAP-4
  intent: QR 生命周期与解绑语义——桌面二维码在有效期内可重新生成（明确旧码立即失效）、被消费后立即清除或标记"已使用"；手机端区分本地解绑与配对失效，配对失败返回结构化原因而非以连接关闭猜测网络问题。
  success: 本地解绑后再扫旧 QR 显示"二维码已使用/需在桌面重新生成"而非网络双失败；有效期内手动重新生成后旧码立即失效、新码可用；配对成功事件即清码/标记已使用；本地解绑确认文案明确"仅清除此手机凭据，需桌面生成新二维码，当前二维码不可复用"；手机可按 `pairingWindowClosed`/`nonceConsumed`/`identityRevoked`/`trustMismatch`/`networkUnavailable` 分类展示恢复指引。

- id: CAP-5
  intent: 命令通道就绪——界面写操作以 `commandReady`（`CommandChannel.binding`）为唯一事实源，连接状态只负责展示 Direct/Relay/Connecting/Degraded；对话发送在网络不可用时进入离线待发箱（幂等 commandId 续发），其余写操作逐项禁用。
  success: `commandReady`/`sessionActive` 以可观察 StateFlow 暴露并被 UI 消费；`engineAvailable=true` 但 binding 为空的竞速窗口内发送不产生突兀失败（落入待发箱，bind 后自动发出）；连接断开时先发布 commandReady=false 再 unbind 展示，bind 完成后才发布可发送；非对话写操作在 !commandReady 时逐项禁用；不存在"界面显示正常、点击发送才报桌面引擎不可达"。

- id: CAP-6
  intent: 流式正文增量显示——Android 按 `thinking=false && phase ∈ {null, "answering"}` 契约将桌面正文 token 逐 token 累积展示，修复空光标后整段出现的跨端契约冲突。
  success: `phase="answering"` 的多个正文 token 逐 token 增量显示、无纯 `▍` 空光标气泡；`foldNew` 与 `foldInto` 共用同一正文判定函数；tool/process/thinking/done 不误归正文；`phase=null` 向后兼容；新增桌面真实 payload → Android 聚合器的跨端黄金契约测试覆盖 answering/thinking/tool/process/done/多 messageId。

- id: CAP-7
  intent: IME 布局正确——键盘弹出时 composer 紧贴键盘顶部（仅设计间距），无底部导航栏高度残留的空区；按"先诊断、后机制性修复"闭环落地。
  success: 诊断阶段仅 debug 构建记录几何数据（不含用户输入内容）并以单变量 A/B 定案，定案后诊断日志完全删除；修复阶段确立唯一 IME inset owner 并显式处理 Scaffold PaddingValues 消费；复现设备原输入法 + 手势导航 + 三键导航下验证 composer 与键盘间距为 8–12dp 且无 bottomBar 高度残留。

## Constraints

- **范式唯一**：全屏阻断遮罩与紧凑指示两套模式不得并存；本规格批准后旧遮罩范式标记为被取代（见 `supersessions.md`），实施须移除阻断挂载，不是叠加一层新提示。
- **网络失败 ≠ 重配推荐**：普通网络、NSD、中继失败不得直接显示解除配对/重新配对入口；只有确认凭据缺失/损坏、信任锚失配或桌面授权失效（结构化拒绝 + 本地已配对推断）才出现重配入口。
- **写操作事实源唯一**：写操作就绪判据只能来自 `commandReady`——网络不可用时对话发送进离线待发箱、其余写操作逐项禁用；连接状态流不得替代命令就绪判据。
- **离线待发消息不可丢失**：待发箱消息在断网、重配、凭据恢复与进程被杀（落盘恢复）全程不得丢失或被虚假标记为已发送；flush 守门须为真实可发送会话（commandReady && paired）；重发必须复用同一 commandId（桌面幂等缓存去重，禁止重复执行）。
- **流式根因不得掩盖**：禁止以伪打字机动画、扩大队列或调整 WebSocket 缓冲掩盖 `phase` 契约根因；队列丢帧、done 丢失、上游单个大 token 作为独立健壮性事项另行记录（不与本根因混修）。
- **IME 证据边界**：未经真机诊断数据，禁止把 `adjustResize + imePadding()` 宣称为已确认的"双重计算"根因；诊断日志仅限 debug 构建、不记录用户输入内容、定案后必须删除。
- **协议兼容**：结构化配对失败原因复用既有 `Frame.Notice` 帧类型（不新增帧类型、不破坏协议版本）；旧版手机忽略该 Notice、旧版桌面不发该 Notice 时，双端回退既有行为。
- **外科手术式修改**：仅改动本规格任务清单所列文件与症状相关代码；禁止顺手重构无关代码；保持既有中文注释与文案风格。
- **测试验证意图**：每项测试须体现行为为何重要（如"answering 兼容为何防整段回填"），业务逻辑变更时测试必须报错。

## Non-goals

- 双端一键解绑（在线认证命令 + 离线兜底）不在本规格范围；本规格仅区分语义并在本地解绑文案中说明边界，双端撤销另行立项。
- 队列丢帧、done 帧静默丢失、上游单个大 token 的健壮性治理不在本规格范围（记录为 deferred 事项）。
- 桌面 Agent/LLM 层"模型服务暂时不可用"错误治理不在范围（与连接问题分案）。
- 不修改桌面正文 token 的 `phase="answering"` 发射语义（保留桌面 phase 可观测性，由手机端兼容）。
- FR-42 系统推送、NotificationDispatch 适配器不在范围。
- 不重构 ChatViewModel 多会话、快照分帧、通知中心等无关模块；不做像素级视觉重设计；不引入 DI 框架/Room/新网络库。
- 不恢复旧三态常驻连接横幅（Direct 常驻绿色块已被 `spec-companion-android-remove-connection-banner.md` 正确删除；新紧凑指示仅出现在 Connecting/Reconnecting/Degraded）。

## Success signal

`test-matrix.md` 的 11 条验收场景全部通过（含真机 IME 与 QR 端到端项），`./gradlew :app:assembleDebug` 与 `:app:testDebugUnitTest` 全绿、桌面端改动 `npm run test:all` 全绿；已配对用户冷启动 20 秒内可正常浏览与操作（无阻断遮罩），断网时界面诚实降级（离线输入的消息在网络恢复后自动送达）而非推荐解绑，配对失效时一次性给出准确原因并落入配对流；对话回复逐 token 浮现；键盘弹出时输入条紧贴键盘。

## Assumptions

- 冷启动 grace 取 20 秒常量（产品裁决"约 20 秒"），Reconnecting 复用同一常量；不做成用户可配置项。
- 新状态模型的承载/健康/就绪三面均以 Android 端为事实源改造；桌面侧唯一协议改动为配对拒绝结构化 Notice 与 QR UI 行为。
- 结构化原因码集合采用用户裁决的五值（pairingWindowClosed / nonceConsumed / identityRevoked / trustMismatch / networkUnavailable）；其中 identityRevoked 由"手机本地已配对 + 公钥未变 + 桌面结构化拒绝"推断，不在桌面侧新增撤销名单存储。
- 流式修复采用"手机兼容 answering"方向（调查 D.1 推荐），桌面发射端不动。
- IME 修复的最终机制（隐藏 bottomBar 或应用剩余 inset）由诊断数据决定，规格不预设结论。
- 离线待发箱落盘持久化（用户裁决 B）：队列写入应用私有目录（复用既有 `KeystoreSnapshotCipher` 加密，与快照缓存同级保护；原子写），进程被杀/冷启动后恢复队列与待发态气泡，恢复网络后续发。
- 黄金契约 fixture 采用双轨维护（用户裁决 A+B 都做）：手工快照进 Android 测试资源（P0）+ 桌面 Rust 测试锁定 phase 值域锚定 fixture（必做）。
- 验收 #8 的字面"发送控件必须不可用"经用户确认（裁决 ①A）正式修订为"发送不产生突兀失败、落入待发箱"；其意图（不允许界面显示正常、点击后突兀报错）不变。非对话写操作（Tasks/Memory/ActionCard 等）仍按原产品裁决逐项禁用。
- Onboarding 引导流的 sendMessage 不接入待发箱（一次性引导在线场景），仍按 commandReady 禁用。

