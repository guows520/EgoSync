# Investigation: 手机伴侣连接降级与对话体验异常

## Hand-off Brief

1. **What happened.** 已确认这是多机制问题：健康配对设备冷启动先被建模为 Offline，立即触发全屏阻断遮罩；本地解绑后旧 QR 因手机新公钥 + 已消费 nonce 必然不可复用；桌面正文 token 的 `phase="answering"` 与手机仅接收 `phase == null` 的规则冲突，导致空光标后由最终快照整段回填。
2. **Where the case stands.** 连接、重配、命令就绪、运行中配对失效路由和流式输出均有高置信代码证据；另确认元数据完整但包裹私钥文件缺失会静默生成新手机身份，现有自愈无法识别。IME 截图确认存在约数百像素空区，但各层 Insets 的精确贡献仍需真机几何取证。
3. **What's needed next.** 先按“状态/路由单一事实源、QR 与凭据生命周期、流式协议契约、IME 单一所有权”形成修复规格，再分机制实施和回归；不要把所有症状合并成一个 UI 补丁。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-09-08 |
| Status | Concluded（IME 精确机制需诊断验证） |
| System | Android 伴侣 `0.1.6-alpha.1`（Kotlin/Compose，targetSdk 37）+ EgoSync 桌面端（Tauri/Rust） |
| Evidence sources | 用户 5 张截图与复现描述；Android/桌面源码；Git 历史；既有调查与规格；测试代码。未提供手机 logcat、桌面 tracing 或 Insets 实测值。 |

## Problem Statement

用户报告：

1. 手机启动立即显示“降级模式 · 只读缓存”，而不是先尝试连接约 20 秒；全屏灰色遮罩阻断界面；网络断开时不应直接建议解绑重扫。
2. 连接成功后在手机解绑，再扫同一二维码，报“未发现桌面设备，且中继连接失败”；等二维码到期后重新生成/扫描又成功，但有效期内没有“重新生成二维码”入口。
3. 对话输入框聚焦后与键盘之间存在大空区；连接异常时界面看似正常，发送才报“桌面引擎不可达”。
4. 回复先显示思考点，随后出现空流式光标，但正文不增量显示，最后整段一次出现。

用户描述属于待验证假设；截图只直接确认全屏降级遮罩、键盘上方大空区、思考点→空光标→整段回复，以及截图中的“模型服务暂时不可用”。“桌面引擎不可达”未出现在截图中，但其代码路径已定位。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户截图 | Available | 确认全屏遮罩、IME 大空区、思考点/空光标/整段回复；截图错误为“模型服务暂时不可用”而非“桌面引擎不可达”。 |
| Android 源码 | Available | 状态机、真实连接、遮罩、配对、聊天、IME、指令通道、流聚合均已定位。 |
| 桌面源码 | Available | 配对窗口/nonce、NSD、中继、QR UI、llm:stream 发射与手机镜像已定位。 |
| Git 历史 | Available | 连接横幅曾被明确删除；全屏离线遮罩被指定为唯一离线明示。 |
| 自动化测试 | Partial | 覆盖基础状态/配对/流式，但遗漏 `phase="answering"` 跨端契约、`engineAvailable != sessionActive` 和真机 IME。 |
| 手机 logcat / 桌面 tracing | Missing | 无法给出截图当次网络承载失败和 IME Insets 数值。 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 冷启动为何立即降级且全屏阻断 | High | Done | 根因 Confirmed。 |
| 2 | 网络问题为何总出现解绑重扫入口 | High | Done | 状态模型无法表达原因 + CTA 无条件渲染。 |
| 3 | 本地解绑后旧 QR 为何失败 | High | Done | 新手机公钥 + 已消费 nonce/关闭窗口，旧码按安全设计不可复用。 |
| 4 | QR 有效期内为何无法手动刷新 | Medium | Done | 桌面 UI 仅过期后清空，显示态无重新生成按钮。 |
| 5 | 界面正常但发送“桌面引擎不可达” | High | Done | UI 用 `engineAvailable`，真实发送依赖 `sessionActive/binding`，存在状态空窗。 |
| 6 | 流式为何空光标后整段出现 | High | Done | `phase="answering"` 跨端契约冲突，根因 Confirmed。 |
| 7 | IME 大空区的精确 Insets 构成 | Medium | Done（结论止于可证边界） | 症状与高风险布局组合 Confirmed；各层像素贡献必须靠真机 Insets/坐标验证，记录为开放假设。 |
| 8 | 运行中配对失效后是否回到配对流 | High | Done | 不会自动导航；状态清理与路由/缓存清理脱节。 |
| 9 | 元数据完整但包裹私钥文件丢失 | High | Done（代码路径） | 会静默生成新身份，绕过现有元数据自愈；缺端到端自动化用例。 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 20:20 | 主界面显示全屏“降级模式 · 只读缓存” | 用户截图 1 | Confirmed |
| 20:25 | 对话页键盘弹出，输入条与键盘间约数百像素空区；显示两条“模型服务暂时不可用” | 用户截图 2 | Confirmed |
| 20:44 | 重试后管家正常回复，下一条请求出现思考点 | 用户截图 3 | Confirmed |
| 20:44 | 思考点转为空流式光标 `▍`，无正文增量 | 用户截图 4 + `ChatScreen.kt:655-661` | Confirmed |
| 20:44 | 最终正文整段出现 | 用户截图 5 | Confirmed |
| 调查期 | 确认连接横幅曾被删除，离线只由全屏遮罩明示 | commit `f13cdfe` | Confirmed |

## Confirmed Findings

### Finding 1: 健康配对态冷启动第一帧就是 Offline，不存在 Connecting/Reconnecting 宽限态

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionClient.kt:12-28`；`ConnectionStateMachine.kt:33-35`；`RealConnectionClient.kt:748-754,769-778`；`AppNavHost.kt:133-154`

**Detail:** 状态模型只有 Direct、Relay、Offline。已配对冷启动的 `initialState()` 明确返回 `offlineState()`；局部状态机也以 Offline 初始化。UI 一观察到 Offline 且 paired=true，立即挂载降级遮罩。12 秒 NSD 与后续连接尝试在后台进行，但没有 20 秒 grace period，用户第一帧已经被判定为“离线/降级”。

### Finding 2: 全屏灰色阻断和“解绑重扫”不是偶发现象，而是组件的硬编码设计

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/ui/components/DegradedOverlay.kt:46-62,90-96,121-142`；`AppNavHost.kt:133-154`；`spec-companion-offline-deadlock-recovery.md:13-33`

**Detail:** 遮罩 `.fillMaxSize()`、0.72 黑色透明层并消费全部指针事件；任意 Offline 都显示同一“连接不上？解除配对并重新扫码”CTA。既有规格甚至要求不改变遮罩样式、不新增手动重连。用户现在提出的是产品裁决变化，不是调小一个超时即可解决。

### Finding 3: 小型连接提示曾存在，后被明确删除

**Evidence:** commit `f13cdfe15605d088820c4aa17d7da272c384bcbb`；`_bmad-output/implementation-artifacts/spec-companion-android-remove-connection-banner.md:11-21`

**Detail:** 历史实现有三态连接横幅，后因 Direct 时常驻绿色块“无信息量”而整体删除，并裁决“离线明示由 DegradedOverlay 独立承担”。这与用户现在要“小图标提示、不阻断界面”直接冲突。正确方向是恢复**仅 Connecting/Degraded 时出现**的紧凑状态指示，而不是恢复原常驻横幅。

### Finding 4: 手机本地解绑后，同一 QR 按当前安全模型必然不可复用

**Evidence:** `RealConnectionClient.kt:171-186,226-237`；`egosync-app/src-tauri/src/services/companion_pairing.rs:413-455`；`companion_connection.rs:655-665,750-762`

**Detail:** 手机 `unpair()` 会清本地配对元数据并异步擦除静态私钥；下一次扫码会生成新手机公钥。首次成功配对已消费 QR nonce，并将桌面 pairing window 设为 None。桌面仍保留旧手机公钥；新公钥既不是已配对/待确认公钥，又没有打开窗口，早期准入直接关闭连接。旧 QR 不是“暂时失败”，而是安全设计上的一次性凭证。

### Finding 5: 重新生成新 QR 会打开新窗口，但桌面 UI 在旧码有效期内没有重新生成入口

**Evidence:** `companion_pairing.rs:118-133,392-455`；`companion_connection.rs:1094-1113`；`egosync-app/src/components/settings/CompanionPairingSection.tsx:104-140,285-324`

**Detail:** 每次生成 QR 都创建新 nonce；生成命令打开新的 300 秒窗口。首次配对消费 nonce 后，桌面配对事件只刷新设备列表，不会清除或标记当前 QR；组件在 `qrPayload != null` 时仍显示这个已不可用的二维码与倒计时，且没有重新生成按钮。只有本地倒计时到期把 payload 清空后，生成按钮才重新出现。因此“二维码单次有效”“重新生成会使旧码失效”的文案，与已消费状态不可见、控件不可达的实际行为冲突。

### Finding 6: “未发现桌面设备，且中继连接失败”是承载层合并文案，无法表达旧 QR/准入被拒

**Evidence:** `RealConnectionClient.kt:247-278,380-395`；`companion_connection.rs:655-665`

**Detail:** NSD 超时/IO 异常且 QR 有 relayAddr 时回退中继；中继失败后 phase 可继续保持 discovery，统一显示“双失败”文案。桌面对于未知公钥且窗口关闭仅关闭连接，不返回结构化“QR 已使用/窗口关闭”。因此手机不能区分“网络不可达”“中继配置坏”“旧 QR 的新公钥被拒”，只能误导性建议检查网络/中继。

### Finding 7: UI 的“引擎可用”和真实指令通道就绪是两套未统一的信号

**Evidence:** `ConnectionClient.kt:20-28`；`CommandChannel.kt:62-64,100-103`；`RealConnectionClient.kt:217-222,439-457,738-745`；`AppNavHost.kt:282-305`；全仓 `sessionActive` 搜索仅定义/测试使用，UI 未消费。

**Detail:** `engineAvailable` 只判断 state 不是 Offline；聊天输入是否可用也只读它。真正发送要求 `CommandChannel.binding != null`，否则抛“桌面引擎不可达”。连接建立时先置 Direct 再启动 session loop/bind；连接结束时先 unbind，再调用 `onDirectLost`，且有中继时 Direct 状态还会保持 3 秒滞回。两个空窗都可出现“界面正常、发送才失败”。

### Finding 8: 正文流式协议存在确定的跨端契约冲突

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:209-233`；`egosync-app/src-tauri/src/services/companion_dispatch.rs:668-711`；`egosync-app/src-tauri/src/services/companion_connection.rs:166-187,834-865`；`companion-android/app/src/main/java/com/egosync/companion/command/StreamCoordinator.kt:164-176,179-219`；`ChatViewModel.kt:578-608`；`ChatScreen.kt:655-661`

**Detail:** 桌面所有可见正文 token 都通过 `emit_stream_token(... thinking=false)` 发射，函数硬编码 `phase="answering"`；镜像和 WebSocket 路径保留该字段并逐帧顺序交给 Android。手机解析器正确保留 phase，却仅在 `!thinking && phase == null` 时把 token 累加到文本。于是每个正文 token 到达却被语义过滤；首 token 仍会建一个空 segment 并关闭 thinking，UI 恰好显示只有 `▍` 的空气泡。done 后最终快照携带完整已落库消息，解除流式门后整段回填，精确吻合截图顺序。Android 入站 channel 是 UNLIMITED，且 `phase=null` 测试在同一 VM/Compose 路径可逐字显示，反驳“手机入站容量或 Compose 本身吞 token”是主因。

### Finding 9: 现有测试使用了错误的“自洽契约”，因此没有发现生产协议错位

**Evidence:** `companion-android/app/src/test/java/com/egosync/companion/command/StreamCoordinatorTest.kt:22-65`；`companion-android/app/src/test/java/com/egosync/companion/ui/chat/ChatViewModelTest.kt:155-175,269-304`

**Detail:** 测试辅助函数默认 `phase=null`，文本逐字测试也不传 `answering`；仓库没有任何 Android 测试覆盖 `phase="answering"`。测试证明的是手机内部规则自洽，而不是桌面生产 payload 与手机解析器兼容。

### Finding 10: IME 大空区真实存在，代码具备重复避让条件，但尚不能仅凭静态代码断定每一像素来自哪层

**Evidence:** 用户截图 2；`MainActivity.kt:23-58`；`AndroidManifest.xml:19-23`；`AppNavHost.kt:229-278`；`ChatScreen.kt:125-129,228-276`；既有调查 `companion-android-mobile-ui-issues-investigation.md:58-65,94-100`

**Detail:** 当前可证的真实层级是 `enableEdgeToEdge` + 外层默认 Scaffold padding（仅 `Modifier.padding`，没有 `consumeWindowInsets`）+ 内层 Scaffold（底部 NavigationBar，contentWindowInsets=0）+ ChatScreen 整屏 `imePadding()` + Activity `adjustResize`；底部栏默认仍参与内层 Scaffold 的 content padding。仓库曾确认嵌套 Scaffold 导致顶部 inset 重复，并只修了内层 system-bars inset；没有 IME 仪器测试。截图中的空区约数百像素，明显不是输入条自身 10dp padding。高概率主嫌是“已为底部 app NavigationBar 让位的内容坐标系，再叠加完整 IME padding”，外层/系统导航 inset 可能继续放大。**但 `adjustResize` 与 `imePadding()` 不能仅凭同时存在就判定重复；Material3/Android/OEM 对具体消费方式需以真机数值确认。**

### Finding 11: 运行中清除 paired 不会自动导航到配对页，身份失效恢复链不闭环

**Evidence:** `AppNavHost.kt:88-97,133-153`；`RealConnectionClient.kt:398-409`；`AppModelContainer.kt:141-147`

**Detail:** `startDestination` 只在首次组合时用 `remember` 定格；运行中 `_paired` 从 true 变为 false，只会影响降级遮罩的显示守门，不会触发导航。`handleSecretsInvalidated()` 只清连接层配对元数据并发布 Offline，没有调用显式导航，也没有走 `AppModelContainer.unpair()` 的快照和通知清理。因此密钥失效会让用户留在原主界面路由，遮罩因 `paired=false` 消失，同时连接仍为 Offline，形成“未配对身份 + 旧主界面/旧缓存”的状态矛盾。

### Finding 12: 元数据完整但包裹私钥文件缺失时，现有冷启动自愈会被绕过

**Evidence:** `RealConnectionClient.kt:151-159,200-213,439-470,571-588`；`KeyStore.kt:39-55`；`PairingStateStore.kt:5-25`

**Detail:** 构造期 `healCorruptPairingIfAny()` 只检查 `paired=true` 时 `relayId` 或 `desktopPubkeyHex` 是否缺失，不检查包裹私钥文件或本机公钥是否仍与配对身份一致。元数据齐全但 `noise_static_wrapped.bin` 不存在时，`PairingSecrets.loadOrCreateStaticPrivateKey()` 不抛 `SecretsInvalidatedException`，而是按“首次使用”静默生成新私钥。随后重连以新手机公钥握手；桌面只认识旧公钥，会在准入层关闭连接，而 Android 的通用异常路径只继续退避并维持 `paired=true/Offline`。该控制流由源码确定；仓库没有覆盖“元数据完整 + 私钥文件缺失”的回归测试，实际触发来源和运行日志仍属缺失证据。

## Deduced Conclusions

### Deduction 1: “先等 20 秒再降级”不能通过延长 NSD timeout 独立实现

**Based on:** Finding 1、2

**Reasoning:** 遮罩触发由状态语义决定，当前第一帧已是 Offline；即使 NSD 从 12 秒改成 20 秒，遮罩仍会立即出现。

**Conclusion:** 必须引入 Connecting/Reconnecting（或 UI 层 grace 状态），并把“是否显示降级”与底层初始 Offline 解耦。

### Deduction 2: “小图标不阻断”与现有 FR-43/旧冻结规格冲突

**Based on:** Finding 2、3

**Reasoning:** 当前设计明确要求 Offline 时禁用依赖引擎的操作并以全屏遮罩拦截；用户现在要求保留界面可操作性。

**Conclusion:** 应明确选择新产品语义：只读浏览和本地能力继续可用，逐个禁用引擎操作；不可在全屏遮罩和非阻断界面之间折中叠加两层提示。

### Deduction 3: 本地解绑不是完整的双端“解除配对”

**Based on:** Finding 4、5、6

**Reasoning:** 手机擦密钥后变成新身份，桌面仍保存旧身份；旧 QR 已消费，新 QR 才重新表达配对/换绑意图。

**Conclusion:** 手机侧“解绑”文案应明确为本机解除并需要桌面生成新 QR；若产品要一键双端解绑，必须新增已认证 command/协议并处理断线时不可达场景。

### Deduction 4: “界面正常但不可达”不是缺少 snackbar，而是状态事实源错误

**Based on:** Finding 7

**Reasoning:** UI 的可操作性使用粗粒度承载状态；真实发送使用 binding。给 execute 失败再补 toast 只能解释失败，不能防止错误启用。

**Conclusion:** UI 可发送状态必须来自 command readiness；连接展示仍可单独显示 Direct/Relay/Reconnecting。

### Deduction 5: 流式一次性输出不是网络缓冲的首要问题

**Based on:** Finding 8、9

**Reasoning:** 即使每个 token 完整按序到达手机，`phase="answering"` 也会让手机确定性丢弃正文；空光标是该规则的直接产物。

**Conclusion:** 先修跨端 phase 契约并加契约测试；修复后若仍卡顿，再调查队列拥塞/中继抖动。当前不应先调 WebSocket 缓冲。

### Deduction 6: `paired` 不是足以驱动 UI 与连接恢复的完整事实源

**Based on:** Finding 4、7、11、12

**Reasoning:** 同一个布尔值同时承担起始路由、遮罩守门、连接编排循环和速记 flush 守门，却不表达本机凭据是否存在、桌面是否承认该手机身份、路由是否已切换。它还能在“私钥已变更但元数据齐全”时保持 true，或在运行中变 false 后不触发导航。

**Conclusion:** 修复不应继续追加零散 `paired` 条件；应建立包含凭据完整性、授权状态、会话就绪和恢复动作的单一连接/配对状态，并让导航、遮罩、缓存清理和控件 enabled 共同消费它。

## Hypothesized Paths

### Hypothesis 1: IME 大空区由多层 Insets/resize 重复避让造成

**Status:** Open（高概率）

**Theory:** 外层 Scaffold padding、`adjustResize`、底部 NavigationBar 与整屏 `imePadding()` 在该 targetSdk/Compose/系统组合下重复计入 IME 或保留导航栏空间。

**Supporting indicators:** 截图空区量级；代码层级；仓库过去已有嵌套 Scaffold 双重 system-bars inset 缺陷。

**Would confirm:** 在真机记录键盘前后根容器高度、外层 `padding.calculateBottomPadding()`、`WindowInsets.ime.getBottom()`、NavigationBar 顶/底、composer 底部坐标；临时 A/B 仅移除 ChatScreen `imePadding()` 或仅让单一层消费 IME，空区随之消失。

**Would refute:** 实测所有 Insets 仅消费一次，而空区来自系统输入法 floating/candidate 区或窗口尺寸错误。

**Resolution:** 尚未定案；实施前需一次诊断构建或 Layout Inspector。

### Hypothesis 2: 截图当次“桌面引擎不可达”发生在 Direct→Relay 3 秒滞回空窗

**Status:** Open

**Theory:** session 已 unbind，但连接状态仍保持 Direct，故 UI 正常而 execute 失败。

**Supporting indicators:** Finding 7 的确定性可达状态；用户描述与表现一致。

**Would confirm:** logcat 同时间出现 session unbind/直连会话结束、随后 send ConnectionError，state 仍为 Direct；记录 `state/sessionActive` 二元组。

**Would refute:** 报错时 `sessionActive=true`，或 commands 为 null（生产容器证据已基本排除后者）。

**Resolution:** 机制已 Confirmed 存在；尚无日志证明截图当次恰由该机制触发。

### Hypothesis 3: 本地解绑后的双失败文案包含 NSD 实际不可达

**Status:** Open

**Theory:** 当时手机不在同一局域网或 mDNS 不可达；中继又因新公钥+关闭配对窗口被拒，最终合并成双失败。

**Would confirm:** 手机日志显示 NSD 12 秒 timeout，然后 relay session 被桌面早期准入关闭；桌面日志显示“未配对公钥尝试连接，已拒绝”。

**Would refute:** NSD 成功解析，失败实际发生在 direct auth；那将暴露错误阶段分类的另一条 bug。

**Resolution:** 旧 QR 不可复用已 Confirmed；当次两承载的各自失败点缺日志。

### Hypothesis 4: 队列丢帧或上游单个大 token 放大流式卡顿

**Status:** Open（次要健壮性/放大因素，不是本次空正文的必要根因）

**Theory:** 桌面 EventRouter、流镜像和 outbound 均有有界 `try_send` 路径，拥塞可丢增量；此外 part 类型尚未知时 delta 会暂缓，某些 Provider 最终可能把累计正文作为单个大 token 发出。

**Would confirm:** 记录不含正文内容的 received/enqueued/dropped/sent 计数与时序，并观察生产轮次出现 drop 或源端仅发一个大正文 token。

**Would refute:** 低水位、零丢帧下仍复现空正文——`phase="answering"` 定向单测已经证明这一点。

**Resolution:** 修复 phase 契约后再压测。扩大队列不能修复本次根因；done 静默丢弃风险可另案治理。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 手机 `Companion/Connection` logcat | 定位当次旧 QR 的 NSD/relay 精确失败阶段、“不可达”触发窗口，以及私钥重建后的拒绝循环 | 用对应复现步骤抓 `adb logcat -s Companion/Connection Companion/Command Companion/Stream` |
| 桌面 tracing | 确认是否命中未知公钥/窗口关闭拒绝 | 对齐时间查看 companion pairing/relay 日志 |
| 凭据完整性测试 | 验证元数据完整但私钥文件缺失时的确定控制流，并防止修复回归 | 为 SecretsProvider 加“既有配对却首次生成新密钥”的可判别测试缝，断言清配对并切换恢复态，不进入无限退避 |
| 路由恢复测试 | 验证运行中配对失效会清不可信缓存并进入配对流 | 触发 `SecretsInvalidatedException`，断言状态事件、导航目标、快照/通知清理 |
| IME Insets 与布局坐标 | 将高概率 Insets 叠加假设升级为 Confirmed | 诊断构建或 Layout Inspector 记录键盘前后数值 |
| 设备/输入法型号 | 排除厂商键盘候选区/手势导航特例 | 记录系统版本、导航模式、输入法版本 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | 冷启动：`RealConnectionClient.initialState`；遮罩：`AppNavHost`→`DegradedOverlayHost`；旧 QR：桌面 `connection_pubkey_allowed`；凭据丢失：`PairingSecrets.loadOrCreateStaticPrivateKey`；路由脱节：`handleSecretsInvalidated`→冻结的 `startDestination`；不可达：`CommandChannel.execute`；流式：`emit_stream_token`→`StreamCoordinator.foldInto` |
| Trigger | paired 冷启动；任意 Offline；手机本地 unpair 后重扫旧 QR；包裹私钥文件丢失/不可解；binding 为空仍允许发送；正文 token 携带 `phase="answering"` |
| Condition | 无 Connecting 状态；CTA 不带 failure cause；新手机公钥且 pairing window 已消费；自愈只校验元数据且运行中 paired 变化不驱动导航；`engineAvailable=true && sessionActive=false`；手机把仅 `phase==null` 视作正文 |
| Related files | `ConnectionClient.kt`、`ConnectionStateMachine.kt`、`RealConnectionClient.kt`、`DegradedOverlay.kt`、`AppNavHost.kt`、`CommandChannel.kt`、`ChatViewModel.kt`、`StreamCoordinator.kt`、`ChatScreen.kt`、`companion_pairing.rs`、`companion_connection.rs`、`companion_dispatch.rs`、`agent_engine.rs`、`CompanionPairingSection.tsx` |

## Conclusion

**Confidence:** High（连接/QR/状态脱节/流式）；Medium（IME 精确机制）

本案不是单一“网络不稳”。连接 UX 的根因是状态模型把“尚在连接”直接表示为 Offline，并按旧裁决用全屏遮罩阻断；配对/凭据恢复还存在两个闭环缺口：运行中清 paired 不驱动导航与完整容器清理，元数据齐全但包裹私钥丢失会静默换身份并绕过自愈。重配问题是手机本地解绑重置身份但旧 QR 已单次消费，同时桌面有效期内缺手动刷新入口；“界面正常但不可达”是 `engineAvailable` 与真实 `sessionActive` 的事实源分裂；流式一次性出现则是已确认的 `phase="answering"` 跨端协议冲突。IME 截图与高风险代码层级已确认，但缺真机 Insets 数据，修复前应做一次最小诊断而非直接删 padding 猜测。

## Recommended Next Steps

### Fix direction

#### A. 连接状态与降级 UX（产品语义变更）

1. 引入明确状态：`Connecting`（冷启动，建议 20 秒 grace）、`Reconnecting`（断线/承载切换）、`Direct`、`Relay`、`Degraded`。
2. 冷启动/短暂重连期间只显示紧凑状态图标或顶部小条，不盖住内容。
3. grace 到期后进入 Degraded：缓存内容继续可浏览；按控件逐项禁用依赖桌面引擎的写操作；保留本地速记。
4. 只在已证实配对失效时显示“重新配对”：本地密钥损坏、桌面身份锚变化、协议明确返回 unknown/revoked/QR invalid。普通 timeout、NSD、relay 网络失败只给网络/重试信息。
5. 更新/取代旧 FR-43 与冻结规格，明确选择非阻断模式；旧“全屏拦截所有交互”应标记待清理，不能两套范式并存。

#### B. QR、凭据与解绑生命周期

1. 桌面 QR 展示态加入“重新生成二维码”按钮，二次确认“旧码立即失效”；收到配对成功事件后立即清码或标记“已使用”，不得继续显示成可扫码状态；过期后按钮转主操作。
2. 手机本地解绑确认文案明确：“仅清除此手机凭据；重新连接需桌面生成新二维码。当前二维码不可复用。”
3. 配对失败协议返回结构化原因：`pairingWindowClosed` / `nonceConsumed` / `identityRevoked` / `networkUnavailable`，手机据此展示恢复动作；不要靠连接关闭推断。
4. 区分“首次无私钥”和“已有 paired 元数据却无私钥”：后者必须作为凭据损坏处理，不得静默生成新身份后继续按已配对重连。可将 SecretsProvider API 改为显式 `loadExisting` 与 `createForPairing`，避免 `loadOrCreate` 混淆生命周期。
5. 由一个上层恢复事件原子协调：清连接层元数据、会话/流状态、快照与通知，并导航到配对流；不得由 `handleSecretsInvalidated()` 只改 `_paired`，再期待冻结的 `startDestination` 自动响应。
6. 如要一键双端解绑，单独设计认证 command + 离线兜底，不与本地 wipe 偷换概念。

#### C. 命令就绪状态

1. 对外暴露 `commandReady/sessionActive` StateFlow，并把 ChatScreen/Tasks/Memory 写操作的 enabled 与它绑定。
2. 连接图标可继续展示承载状态，但发送按钮必须以命令通道就绪为事实源。
3. 断线时先同步发布 Reconnecting/commandReady=false，再 unbind；新会话 bind 完成后再发布可发送。
4. 错误提示区分“正在重连”“桌面离线”“指令超时”，避免统一成“桌面引擎不可达”。

#### D. 流式协议

1. 推荐在手机端把 `phase in {null, "answering"}` 且 `thinking=false` 视为正文；或者桌面取消正文 token 的 `answering` phase。两者择一，推荐手机兼容 `answering`，保留桌面 phase 的可观测语义。
2. 增加桌面真实 payload → Android StreamEvent/StreamCoordinator 的黄金契约测试，必须覆盖 `phase="answering"`、thinking、tool、done、多 messageId。
3. 空 segment 不应渲染纯光标；至少等首个可见正文 token 后再建立流式气泡。
4. 修契约后再测中继丢帧/队列容量；当前证据不支持先调网络缓冲。

#### E. IME

1. 先加一次仅限 debug 的诊断：在一次焦点切换中记录 root、outer/inner content Box、NavigationBar、composer 的 top/bottom，以及 outer/inner bottom padding、IME/navigationBars inset；不得记录输入内容，定案后删除日志。
2. 决定唯一 IME 所有者：优先让 ChatScreen 输入条负责 `WindowInsets.ime`，外层 Scaffold 只处理 system bars；显式消费 Scaffold PaddingValues，或反过来，但不得让多个坐标层重复保留同一空间。
3. 若实测多余 gap 约等于底部 app NavigationBar 高度，键盘显示时隐藏 NavigationBar，或只应用“未消费的剩余 IME”；不要同时保留整个 bottomBar measured height 与完整 IME bottom。
4. 采用单变量 A/B：依次只改 `imePadding`、bottomBar 可见性、NavigationBar insets、padding consumption 中一项；在截图设备的手势/三键导航和原输入法回归。

### Diagnostic

1. 连接/重配：记录每次状态变更 `{transportState, pairingState, credentialState, sessionActive, phase, failureCategory}`，不记录密钥/QR 内容；复现后删除诊断日志。
2. 凭据损坏：补两个确定性测试——包裹文件缺失与密文不可解。两者在“已有配对”条件下都应发布统一恢复事件、清不可信缓存并进入配对流，而不是生成新身份继续重试或停在旧路由。
3. QR：同时抓手机与桌面日志，确认旧码重扫是否是 NSD timeout + desktop unknown-pubkey rejection。
4. 流式：无需额外日志即可先验证——用生产形状 `phase="answering"` 的 token 跑 Android 单测，应稳定复现空文本。
5. IME：必须获取真机 Insets 数值；这是本案唯一仍需运行期证据的布局根因。

## Reproduction Plan

1. **冷启动**：已有健康配对/缓存，强杀手机 App后重开；期望新行为在 20 秒内显示 Connecting 小提示且缓存可交互，超时后非阻断 Degraded。
2. **网络断开**：桌面关机或断网；期望 Reconnecting→Degraded，不出现解绑重扫 CTA。
3. **配对失效**：桌面删除设备/身份变更；期望结构化提示“配对已失效”并提供新 QR 路径。
4. **旧 QR**：成功配对→手机本地解绑→立即扫旧 QR；期望明确“二维码已使用，请在桌面重新生成”，而非网络/中继双失败。
5. **QR 刷新**：桌面 QR 未过期时手动重新生成；旧码立刻失败，新码成功。
6. **凭据文件缺失**：保留完整 paired 元数据但模拟包裹私钥文件不存在后冷启动；期望识别凭据损坏、清不可信缓存并进入配对流，不静默生成新身份进入无限退避。
7. **运行中凭据失效**：模拟 `SecretsInvalidatedException`；期望原子清理连接/流/快照/通知并显式导航配对页，不停留旧主界面。
8. **命令通道空窗**：主动断开直连并保持 relay 配置，让系统进入 3 秒滞回；期望发送控件立即禁用/显示重连，不抛突兀错误。
9. **流式**：注入两个 `phase="answering"` 正文 token；期望逐 token 显示，无空光标，done 后不闪变。
10. **IME**：在截图同设备/输入法聚焦 composer；期望 composer 底部与键盘顶部保持设计间距（例如 8–12dp），底部导航不夹在二者之间。

## Follow-up: 2026-09-08

### New evidence

- `AppNavHost` 的起始路由仅在首次组合时定格；`paired` 后续变化不会自动导航。运行中 `handleSecretsInvalidated()` 清 paired 后，遮罩守门反而关闭，但用户仍停在原路由。
- `PairingSecrets.loadOrCreateStaticPrivateKey()` 在包裹文件不存在时无条件生成新私钥；构造期自愈只检查非机密元数据，因此“paired 元数据完整 + 私钥文件缺失”会绕过自愈并以新手机身份连接旧桌面记录。

### Updated conclusion

配对恢复不能继续依赖一个 `paired` 布尔值和 `loadOrCreate` 隐式行为。必须把凭据完整性、桌面授权、会话就绪和恢复导航建模为显式状态，并由上层统一执行连接清理、缓存/通知清理和导航。源码控制流已定案；仍缺的是实际触发来源日志，以及两条端到端回归测试。

### Updated fix direction

优先建立统一的 pairing/credential recovery 事件，并把 SecretsProvider 生命周期拆成“加载既有密钥”和“配对时创建新密钥”。这应纳入连接状态整改的同一规格，而不是另补一个遮罩判断。

## Side Findings

- 截图中的“模型服务暂时不可用”来自桌面 Agent/LLM 错误，不等于“桌面引擎不可达”；两者必须分案看待，不能用连接修复掩盖模型服务问题。
- 旧规格明确禁止改变全屏遮罩/新增手动重连，当前用户要求与之冲突；实施前需显式更新产品裁决。
- `sessionActive` 已存在于接口却没有任何生产 UI 消费，这是明显的未接线事实源。
- 桌面流式镜像按顺序转发所有 `llm:stream`，当前主因不是“只转 done”；`companion_snapshot.rs` 的 done-only 逻辑只负责触发最终快照重建。
- Android Manifest 没有显式禁用备份/恢复，也没有仓库内 backup rules；系统恢复 SharedPreferences 与应用私有文件是否可能不同步，取决于平台和安装/恢复路径。它是“元数据完整、私钥文件缺失”的一种潜在触发来源，但本案没有安装日志，不能据此断言。
