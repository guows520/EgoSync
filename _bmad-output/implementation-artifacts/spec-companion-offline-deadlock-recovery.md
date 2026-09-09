---
title: '伴侣端离线死锁恢复：损坏配对态自愈 + 降级遮罩受控出口'
type: 'bugfix'
created: '2026-08-30'
status: 'done' # 2026-08-30 人工批准 [A]；baseline_commit: 499625c。superseded-by: SPEC-companion-connection-chat-ux (2026-09-08 裁决：非阻断降级范式)
context:
  - '_bmad-output/implementation-artifacts/investigations/android-companion-offline-investigation.md'
  - '_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 覆盖安装后旧版遗留的 `paired=true`（缺 `relayId`/`desktopPubkeyHex`）使连接层跳过配对流、又在编排首轮静默退出（`RealConnectionClient.kt:383-384`）；叠加离线遮罩全屏锁死且无应用内出口，用户永久卡在「离线 · 暂无缓存」——违背 FR-40「重装重扫即恢复，无需桌面端重置」验收。

**Approach:** 两条恢复路径：① 连接层启动时检测损坏配对态（`paired=true` 但 `relayId` 或 `desktopPubkeyHex` 缺失）自动清配对回未配对态；② 离线遮罩新增显式受控出口「解除配对并重新扫码」（二次确认，复用设置页既有对话框模式），导航回配对流。

## Boundaries & Constraints

**Always:**
- 自愈判定：`store.paired == true && (store.relayId == null || store.desktopPubkeyHex == null)`；命中即走 `unpair()` 同等清理语义（store.clear、密钥擦除、Debug 复位、快照清除），日志走 `CompanionLog`（warn，不打印密钥材料，NFR-M7）。
- 自愈必须在 UI 起始路由冻结（`AppNavHost.startDestination` 的 `remember{}`）之前同步完成——放在 `RealConnectionClient.init` 内，保证冷启动直接落配对流、不闪现遮罩。
- 遮罩出口是破坏性动作：必须 AlertDialog 二次确认（样式与 error 色确认钮沿用 `SettingsScreen.kt:83-89` 既有模式）；FR-43 语义不变——速记条仍是遮罩唯一持续开放入口，出口按钮为低强调样式。
- 出口确认后镜像 `SettingsRoute.onUnpair`（`AppNavHost.kt:369-374`）的导航：`navigate(PAIRING) { popUpTo(0) { inclusive = true } }`。

**Ask First:**
- 若发现需要改动 `ConnectionStateMachine` 三态语义、帧协议或桌面端代码才能完成 → HALT 询问。

**Never:**
- 不改帧协议/握手/信任锚逻辑；不动桌面端。
- 不新增"手动重连"按钮（自动退避已覆盖，避免 UI 膨胀）。
- 不重设遮罩既有样式与速记交互（UX-M 约束），仅追加受控出口。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 覆盖安装遗留态 | paired=true，relayId=null 且/或 desktopPubkeyHex=null | 冷启动即自愈：清配对、paired=false、直落配对流，logcat 有 warn；不发起任何网络尝试 | 不崩溃、不显示离线遮罩 |
| 健康配对态冷启动 | paired=true，relayId 与 pubkey 齐全 | 现状不变：进入双承载编排退避重连 | N/A |
| 遮罩出口确认 | Offline 遮罩 → 出口按钮 → AlertDialog 确认 | unpair + 导航 PAIRING（清空返回栈）；速记队列保留（FR-43 无丢失） | 取消则停留遮罩原状 |

</frozen-after-approval>

## Code Map

- `companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt` -- 自愈落点：init 同步检查 + `unpair()` 复用参照（167-183 行）
- `companion-android/app/src/main/java/com/egosync/companion/connection/PairingStateStore.kt` -- 判定字段来源（paired/relayId/desktopPubkeyHex getter）
- `companion-android/app/src/main/java/com/egosync/companion/ui/components/DegradedOverlay.kt` -- 出口按钮 + 确认对话框追加处
- `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt` -- DegradedOverlayHost 调用点（134-139）接线 onRePair
- `companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsScreen.kt` -- AlertDialog 既有模式参照（只读）
- `companion-android/app/src/test/java/com/egosync/companion/connection/RealConnectionClientOrchestrationTest.kt` -- 自愈单测落点（P7 测试缝）

## Tasks & Acceptance

**Execution:**
- [x] `RealConnectionClient.kt` -- 新增私有 `healCorruptPairingIfAny()`：命中判定则 CompanionLog.warn + 清 store/`_paired`/debugMode/指令通道/流式态（与 `unpair()` 清理面一致，但不清速记）；`init` 块最前同步调用，编排启动决策在其后 -- 根治静默死锁
- [x] `DegradedOverlay.kt` -- `DegradedOverlayHost` 新增 `onRePair: () -> Unit` 参数；文案下方加低强调文字按钮「连接不上？解除配对并重新扫码」+ AlertDialog 二次确认（确认=error 色调） -- 应用内唯一受控出口
- [x] `AppNavHost.kt` -- 134-139 行调用点传入 `onRePair`：`container.unpair()` + 导航 PAIRING 清空返回栈 -- 与设置页解除配对行为一致
- [x] `RealConnectionClientOrchestrationTest.kt` -- 新增用例：遗留态构造后 paired=false（自愈生效、无网络尝试）；健康态构造后行为不变 -- 防回归

**Acceptance Criteria:**
- Given 覆盖安装遗留 `paired=true` 且 `relayId=null`，when App 冷启动，then 直接落在配对扫码流，全程不出现离线遮罩。
- Given 任一 Offline 遮罩，when 用户点出口并在对话框确认，then 配对态清空并回到配对流；when 取消，then 停留原状。
- Given 健康配对态且桌面关机，when 冷启动，then 行为与现状一致（退避重连 + 遮罩），出口按钮可见可用。

## Spec Change Log

### 2026-08-30 评审轮 1（盲审/边界/验收三路并行；无 intent_gap/bad_spec，未触发 loopback）

- **patch（验收·高危，FAIL 主因）**：出口路径误触发速记 flush——`unpair()` 置 Direct 后，`AppModelContainer` 恢复收集器仅判 `engineAvailable` 即清队并提示「已提交管家」，速记静默丢失，击穿冻结 I/O 矩阵场景三。修法：收集器加 `connection.paired.value` 守门（真实恢复必然已配对；`unpair()` 先置 paired=false 再置 Direct，时序安全）。避免的坏状态：遮罩出口确认 → 速记丢失 + 虚假成功提示。
- **patch（验收·中）**：自愈补异步 `secrets.wipe()`（镜像 unpair 的 wipeJob 模式），对齐 Always 条「密钥擦除」。**KEEP**：init 同步段只做内存/元数据清理、IO 全部异步的时序结构。
- **patch（边界）**：遮罩显示加 `paired` 门控——配对流内首次尝试失败也呈 Offline，但 PairingScreen 自有错误态承载它，降级遮罩（含解除配对出口）只属已配对设备。避免的坏状态：从未配对成功的设备在配对屏上看到「解除配对并重新扫码」的语义错位。
- **patch（边界）**：`onRePair` 内 Offline 复核——对话框开着时自动重连翻 Direct 的迟到确认不得拆掉刚恢复的健康会话。
- **patch（边界）**：`showRePairDialog` 改 `rememberSaveable`（旋转/进程重建不丢确认流程）。
- **patch（盲审 #3/#5/#7、边界 #5、验收·低）**：自愈判定 `== null` → `isNullOrBlank()`（空串遗留元数据不得绕过）；测试补持久层断言（store.paired/relayId/desktopPubkeyHex 清除）、init 同步时序护栏（构造后不 advanceTimeBy 立即断言）、OR 单缺分支用例（防误写 AND）。
- **defer**：自愈后无用户可见提示；容器层守门与导航接线无 JVM/Compose 测试缝；heal 后旧快照/通知残留理论场景 → `deferred-work.md`。
- **reject（证据裁定）**：盲审 #11「因果叙事矛盾」被 `ConnectionStateMachine.kt:33-35`（状态机初态即 Offline）+ `RealConnectionClient.kt`（machine.state 收集进 liveState）驳回——静默 return 后状态停留 Offline，遮罩必然显示；#8 对话框文案属实（container.unpair 确清快照缓存与通知）；#2/#10 清理调用为规格枚举的对称防御面；#1 KDoc 已述差异；#9 过早抽象；#12 沿用文件既有硬编码色约定。
- **冻结条张力（呈请人工知悉，未擅改冻结块）**：Always 条「快照清除」架构上属容器层——遮罩出口路径经 `container.unpair()` 已覆盖；自愈路径客户端层不可达，且目标场景从未建立会话、无缓存可清。裁定口径：以 Code Map/Tasks 的客户端层语义为准。

## Design Notes

自愈后不设 `liveState = Offline`：未配对态沿用既有 `initialState()` 语义（`RealConnectionClient.kt:615-621`，paired=false → Direct，遮罩自然不显示）。同步自愈放在 `init` 而非 `orchestrate()` 协程内，是因为 `Main.immediate` 下协程首段执行时序与首组合存在竞态，`startDestination` 冻结后无法再改起始路由。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:testDebugUnitTest` -- expected: 全绿（含新增自愈用例）
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL

## Suggested Review Order

**损坏配对态自愈（连接层根因修复）**

- init 同步段调用点——晚一拍即死锁复现（startDestination 首组合定格）
  [`RealConnectionClient.kt:155`](../../companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt#L155)

- 自愈本体：isNullOrBlank 判定 + 元数据同步清 + 密钥异步擦（wipeJob 镜像 unpair）
  [`RealConnectionClient.kt:200`](../../companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt#L200)

**速记无丢失守门（FR-43，评审高危修复）**

- paired 守门：遮罩出口/自愈的 Direct 非真实恢复，不得 flush
  [`AppModelContainer.kt:113`](../../companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt#L113)

**降级遮罩受控出口（UI 层）**

- paired 门控（配对失败错误态归 PairingScreen）+ onRePair 的 Offline 迟到确认复核
  [`AppNavHost.kt:136`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L136)

- 出口按钮 + AlertDialog 二次确认（对话框状态 rememberSaveable）
  [`DegradedOverlay.kt:66`](../../companion-android/app/src/main/java/com/egosync/companion/ui/components/DegradedOverlay.kt#L66)

**测试（锁定修复意图，防回归）**

- 时序护栏（不 advanceTimeBy 立即断言）+ 持久层清除断言
  [`RealConnectionClientOrchestrationTest.kt:314`](../../companion-android/app/src/test/java/com/egosync/companion/connection/RealConnectionClientOrchestrationTest.kt#L314)

- OR 单缺分支用例（含空串边界）
  [`RealConnectionClientOrchestrationTest.kt:361`](../../companion-android/app/src/test/java/com/egosync/companion/connection/RealConnectionClientOrchestrationTest.kt#L361)
