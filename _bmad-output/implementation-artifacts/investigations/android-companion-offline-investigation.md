# Investigation: 安卓伴侣安装后「离线 · 暂无缓存」遮罩无法关闭

## Hand-off Brief

1. **What happened.** 安卓伴侣 App 启动后停留在全屏「离线 · 暂无缓存」遮罩（Confirmed：截图文案与 `DegradedOverlay.kt:96,106` 逐字一致）；遮罩无法关闭是 FR-43 设计行为，重连无手动入口、完全依赖自动发现。
2. **Where the case stands.** 代码层根因已 Confirmed（连接状态机初始态即 Offline + 直连重连环持续退避重试）；当前卡在**环境侧哪条假设成立**（桌面未运行 / 不在同一局域网 / mDNS 被拦 / 桌面重装换身份），需手机 logcat 或环境核对才能定案。
3. **What's needed next.** 用户按「重连操作清单」核对环境（桌面 App 在运行、手机与桌面同一 Wi-Fi、无 AP 隔离/VPN）；若仍不恢复，用 `adb logcat -s Companion/Connection` 取证据定案。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-08-30 |
| Status           | Active |
| System           | Android 伴侣 App（companion-android，RealConnectionClient 已换装 Story 12.4）+ 桌面 EgoSync（Tauri 2，companion_connection.rs） |
| Evidence sources | 用户截图、companion-android 源码、egosync-app src-tauri 源码 |

## Problem Statement

用户原话（视为假设）："安装安卓版后出现如图报错，界面无法关闭，我如何重新连接"。截图要点：主界面（仪表盘）之上覆盖全屏遮罩——WifiOff 图标 +「离线 · 暂无缓存」+「与桌面引擎的连接已断开，且本地没有可用快照。恢复连接后将自动补齐最新状态。」+ 底部速记条；统计卡片数值全为「—」。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户截图 | Available | 遮罩文案、速记条、统计全「—」；状态栏显示蜂窝信号，Wi-Fi 实际状态无法判读 |
| 手机端日志（CompanionLog → logcat） | Missing | tag `Companion/Connection`，可定案发现/握手/信任锚失败点（`CompanionLog.kt:13`） |
| 桌面端日志（tracing） | Missing | `lib.rs:359` 监听启动失败会 warn；companion 状态可查 |
| 代码（Android 侧） | Available | connection/ 全部 20 文件 + AppNavHost + AppModelContainer 已读 |
| 代码（桌面侧） | Available | companion_connection.rs 关键路径 + lib.rs setup 装配 |

## Confirmed Findings

### Finding 1: 遮罩是 App 自身组件，「关不掉」是 FR-43 设计行为

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt:134-139`；`companion-android/app/src/main/java/com/egosync/companion/ui/components/DegradedOverlay.kt:65-78,96,106`

**Detail:** 连接态为 Offline 时全局渲染遮罩（挂在最外层 Box，盖住底部四 Tab）；蒙层消费全部指针事件（注释明示"杜绝点穿蒙层操作底层引擎功能"），无关闭按钮，唯一可交互元素是速记条。**不存在手动"重新连接"入口——设计上唯一出路是连接状态自动离开 Offline。**

### Finding 2: 初始态即 Offline，截图文案与初始值逐字一致

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionStateMachine.kt:33-35,109-113`；`RealConnectionClient.kt:615-621`

**Detail:** 状态机初始 `Offline(false, null)`；已配对设备冷启动在会话建立前同样先呈 Offline。`snapshotAvailable=false` 时文案正是"离线 · 暂无缓存"，与截图吻合（无"数据截至"行）。

### Finding 3: 重连全自动、无限退避重试（1s→30s 封顶）

**Evidence:** `RealConnectionClient.kt:151-155,356-379,381-416`；`BackoffPolicy.kt:7-28`

**Detail:** 已配对（`PairingStateStore.paired` 持久化）则启动即进入双承载编排：直连环 NSD 发现 `EgoSync-{relayId前8}`（`_egosync._tcp.`）→ WS → Noise XX 握手 → 信任锚校验；失败后退避 1s 起步、30s 封顶、永不放弃。中继环仅当配对时 QR 带 `relayAddr`（`QrPayload.kt:10-11`，桌面未部署中继则为 null——离网即 Offline）。

### Finding 4: 直连成立的前提（桌面侧随 App 启动自动广播）

**Evidence:** `egosync-app/src-tauri/src/lib.rs:350-361`；`egosync-app/src-tauri/src/services/companion_connection.rs:36,369-376,1031`；`NsdDiscovery.kt:145-147`

**Detail:** 桌面 App 一启动即 spawn 常驻监听（`0.0.0.0` 随机端口）并注册 mDNS `_egosync._tcp.local.`；手机 NSD 依赖 UDP 5353 组播可达。

### Finding 5: 用户此前已完成配对+引导（截图处于主界面而非配对流）

**Evidence:** `AppNavHost.kt:90-96`（未配对起始页=Pairing）；`AppNavHost.kt:169-175`（配对成功才进主界面）；`RealConnectionClient.kt:260-268`（`store.save` 落盘 paired=true）

**Detail:** 能看到仪表盘说明本机 SharedPreferences 已有 paired=true，即**扫码当时**手机与桌面同网且桌面在运行。现在 Offline 说明该前提在当前时刻不成立（网络/桌面状态变化），而非"从未连上"。

## Deduced Conclusions

### Deduction 1: 界面无法关闭 ≠ 故障，是降级锁定的必然表现

**Based on:** Finding 1、2

**Reasoning:** Offline 全局遮罩 + 指针全消费 + 无任何手动出口 ⇒ 在连接恢复前任何 UI 操作都无法关闭它，包括进入「我的」查看配对状态或解除配对。

**Conclusion:** "如何重新连接"的答案不在手机 UI 里，而在恢复网络可达性；连接恢复后遮罩自动消失并补齐快照（`AppModelContainer.kt:107-120` 自动 flush 速记）。

### Deduction 2: 信任锚失配（桌面重装）会导致永久锁死——设计缺口

**Based on:** Finding 3 + `RealConnectionClient.kt:410-412,477-486`

**Reasoning:** 桌面重装生成新静态公钥 → 握手成功但信任锚校验抛 `IdentityVerificationException` → 落入泛化 `catch (_: Exception)` 分支 → 仅 `onDirectLost` 继续退避重试，**不像 Keystore 失效那样自愈清配对**（对比 `handleSecretsInvalidated`，`RealConnectionClient.kt:340-351`）。而解除配对入口被遮罩锁死（Deduction 1）。

**Conclusion:** 此场景下用户无任何应用内自救路径，只能系统级清数据/重装 App。属真实设计缺口（Hypothesis #5 给出验证方式）。

## Hypothesized Paths

### Hypothesis 1: 桌面 EgoSync 未在运行（或伴随服务启动失败）

**Status:** Open

**Theory:** 桌面 App 关闭 → mDNS 消失 → NSD 发现超时 → Offline。

**Supporting indicators:** 最常见情形；截图为初始无快照态（全「—」），暗示配对后从未真正建立过数据会话。

**Would confirm:** 桌面打开 EgoSync 后手机 ≤30s 自动恢复（退避封顶）；或桌面日志有监听启动失败 warn（`lib.rs:359`）。

**Would refute:** 桌面确认在运行且手机同网仍离线。

### Hypothesis 2: 手机与桌面不在同一局域网 / mDNS 被拦

**Status:** Open

**Theory:** 手机在蜂窝网络（截图状态栏有蜂窝流量指示）、或 Wi-Fi 开了 AP 隔离/访客网络/手机 VPN，组播过不去。

**Supporting indicators:** NSD 直连对 UDP 5353 与同网段强依赖。

**Would confirm:** 手机切到与桌面相同 Wi-Fi（关 VPN）后 ≤30s 恢复。

**Would refute:** 同网后仍离线。

### Hypothesis 3: Debug 状态模拟被设为 OFFLINE 档

**Status:** Open（低概率）

**Theory:** 「我的」→ 连点版本号 7 次开启状态模拟 → 选了"离线 · 无缓存"，覆盖真实状态输出（`RealConnectionClient.kt:127-138`）。

**Supporting indicators:** 机制存在且能精确复现截图文案。

**Would refute:** 该覆盖仅内存态，进程重启即复位（`:123` 注释）；且离线遮罩下根本无法进入「我的」开启/关闭它——与"安装后即如此"的时间线不符。

### Hypothesis 4: 桌面重装/换机 → 静态公钥变更 → 信任锚永久失配

**Status:** Open

**Theory:** 见 Deduction 2；重试环永不自愈，遮罩永不消失。

**Would confirm:** `adb logcat -s Companion/Connection` 中只见周期性"直连会话结束"重试且桌面端有握手到达记录但无会话建立；或系统级清 App 数据重扫后恢复。

**Would refute:** 清数据重扫后依旧离线（则回到 H1/H2）。

### Hypothesis 5（用户原始假设）: "连接断了，需要手动重新连接"

**Status:** Refuted（部分）

**Theory:** 用户认为存在需要手动触发的重连操作。

**Resolution:** Confirmed 证据显示重连是全自动的（Finding 3），应用内不存在手动重连入口（Finding 1）；用户唯一能做的是恢复环境前提（见 Reproduction Plan）。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 手机 logcat（tag `Companion/Connection`） | 区分 H1（发现超时）/ H4（握手到达但信任锚拒绝） | `adb logcat -s Companion/Connection`（或重试瞬间抓取） |
| 手机当前 Wi-Fi 状态与网段 | 直接验证 H2 | 手机设置截图：Wi-Fi 是否连接、与桌面是否同一路由 |
| 桌面 App 是否运行、companion 状态 | 直接验证 H1 | 桌面打开 EgoSync；设置页伴侣配对区状态 |
| 配对时 QR 是否含 relayAddr | 判断是否有中继兜底 | 桌面设置页 `CompanionPairingSection.tsx` 是否显示中继已部署 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | `RealConnectionClient.initialState()` / `ConnectionStateMachine.goOffline()`（`ConnectionStateMachine.kt:109-113`） |
| Trigger | 已配对前提下直连环 NSD 发现/WS/握手失败且（无中继配置或中继亦失败） |
| Condition | 桌面 mDNS 不可达（未运行/跨网段/组播被拦）或信任锚失配 |
| Related files | `RealConnectionClient.kt`、`ConnectionStateMachine.kt`、`NsdDiscovery.kt`、`BackoffPolicy.kt`、`DegradedOverlay.kt`、`AppNavHost.kt:134-139`；桌面 `companion_connection.rs`、`lib.rs:350-361` |

## Conclusion

**Confidence:** High（代码层机制全部 Confirmed；具体环境诱因待 H1/H2/H4 之一确认）

遮罩"无法关闭"是 FR-43 降级锁定的设计行为，不是程序卡死：离线时全屏拦截交互、无手动重连入口，重连完全由后台退避循环自动完成（1s→30s 封顶、永不放弃）。要恢复连接，必须让手机重新满足直连前提：**桌面 EgoSync 在运行 + 手机与桌面同一局域网 + mDNS 组播可达**。连接恢复后遮罩自动消失、快照自动补齐、速记自动提交。若桌面曾重装（身份换新），当前版本会陷入"信任锚失配→无限重试"且应用内无自救路径，需系统级清数据重配对——这是一处待修的设计缺口。

## Recommended Next Steps

### Fix direction（待用户确认后另行立项，本次仅诊断）

1. **出口缺口（机制：交互设计）**：Offline 遮罩应提供受控出口（如"重试""解除配对"入口放行），否则任何不可自愈的失配都变成死锁。
2. **信任锚失配自愈（机制：错误处理）**：`runDirectLoop` 对 `IdentityVerificationException` 应走类似 `handleSecretsInvalidated` 的清配对+引导重扫路径，而非泛化重试。
3. **README 过时（机制：文档）**：`companion-android/README.md` 仍称"纯前端 + mock"，实际已换装 RealConnectionClient（Story 12.4/13.x），易误导排障。

### Diagnostic（按序执行即可定案）

1. 桌面打开 EgoSync（保持前台/后台均可，确认非退出态）。
2. 手机：关闭 VPN；切换到与桌面**同一 Wi-Fi**（勿用访客网络/蜂窝）。
3. 观察手机 ≤30s 内遮罩是否消失（退避封顶 30s）。
4. 仍未恢复：手机开关一次 Wi-Fi 或强杀 App 重开（重置退避与 NSD）。
5. 依旧失败：`adb logcat -s Companion/Connection` 抓 1 分钟日志回传——若见持续重试且桌面在运行同网，即指向 H4（信任锚失配），执行：系统设置 → 应用 → EgoSync 伴侣 → 清除数据 → 重新扫码配对。

## Reproduction Plan

配对成功后：① 关闭桌面 EgoSync → 手机遮罩在会话超时后出现且无法关闭；② 重新打开桌面 → ≤30s 自动恢复。该流程可直接验证"设计行为"结论与 H1。

## Side Findings

- `companion-android/README.md:102-122`（状态模拟四档说明）与 `:137`（"后续接入真实连接层的替换点"）整体过时——替换已发生（`AppModelContainer.kt:28-33` 注释可证）。
- `DegradedOverlay.kt:96` 两态文案（离线/降级）与 `RealConnectionClient.offlineState()` 的快照信息填充链路工作正常，截图呈现的"暂无缓存"如实反映了本机无快照缓存（首次未建会话）。

## Follow-up: 2026-08-30

### New Evidence

- **用户陈述（视作高可信输入）**："从来没有连接过，界面冻结无法连接"——与 Finding 5（"已完成配对"）冲突。Finding 5 修正为：主界面可见 ⇒ 仅证明 SharedPreferences 中 `paired=true`（持久化事实），**不证明经历过真实配对成功**。
- `MainActivity.kt:23-33`：无 OnBackPressedCallback——遮罩不拦系统返回手势，但 DASHBOARD 是根路由，返回键只能退出 Activity；重进后状态不变。**应用内零出口 Confirm。**
- `PairingViewModel.kt:98-127`：真实路径的 `Success` 仅由 `finishPairing` 触发，且 `store.save` 先于 Success（`RealConnectionClient.kt:260-268`）——真实配对成功必然持久化 paired=true。⇒ "从未真配对却见主界面" ⇒ paired=true 另有来源。
- `RealConnectionClient.kt:383-384`：`val relayId = store.relayId ?: return`——paired=true 但 relayId 缺失时，直连环**首轮即静默 return，零重试零网络活动**；relayAddr 同为 null 时不启动中继环（`:372-376`）→ 永久 Offline。

### Additional Findings

- **Finding 6（Confirmed，代码级死循环路径）**：mock 原型时代的 `companion_prefs/paired=true` 与新版 `PairingStateStore` 复用同文件同键（`PairingStateStore.kt:9-11` 明示"无迁移"）。**覆盖安装**真实版后：paired=true、relayId=null → 编排瞬时退出 → 永久离线遮罩。与用户"从未连接过"完全自洽。
- **Finding 7（Confirmed）**：离线遮罩应用内零出口（Finding 1 + 返回键仅退出）＋任意配对态损坏/对端失联最终收敛为不可恢复死局。

### Updated Hypotheses

- **Hypothesis 6（Confirmed 2026-08-30）**：用户证实为**覆盖安装**（此前装过旧版/原型）。结合 Finding 6 代码证据（共键无迁移 + relayId=null 静默 return），根因成立。Resolution：用户口头确认覆盖安装史 + `RealConnectionClient.kt:383-384` 静默退出路径 + `PairingStateStore.kt:9-11` 共键设计，三证据互洽，无矛盾证据。
- H1/H2/H4 维持 Open（作为一般性离线场景仍然成立，但非本案主因）；H3（Debug 模拟）降权。

### Backlog Changes

- 新增（High）：`runDirectLoop` 对 relayId 缺失应视为配对态损坏，走自愈清配对（对齐 `handleSecretsInvalidated`），而非静默 return。
- 新增（High）：离线遮罩提供受控出口（手动重试 / 重新配对）。
- 新增（Medium）：核实桌面 SNAPSHOT 推送时点（会话建立→快照到达间隔），解释"暂无缓存"与"配对 nominally 成功"能否共存。

### Updated Conclusion

用户指控成立（Confidence: High，代码级 Confirmed）：当前版本在"配对态损坏"（H6）与"对端失联"（H1/H2/H4）两类情形下均收敛为**无出口死循环**。双机制叠加：① 全局交互锁无任何应用内出口；② 编排层对损坏配对态静默放弃、不重试不自愈。即时解锁唯一路径：系统级清除数据/卸载重装 → 重新配对。
