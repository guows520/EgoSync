---
baseline_commit: e0484dcbb042eb56087da14243ae8ab21dadf7ca
---

# Story 12.5: 中继首配——手机跨网扫码配对与桌面远程确认门

Status: review

## Story

As a 手机伴侣用户,
I want 首次配对时手机不在桌面局域网也能经已配置的中继服务器完成扫码配对,
so that 出差/异地场景无需先回到家连入局域网才能绑定手机（当前报「未发现桌面设备」即此限制，见调查档案 `investigations/companion-public-discovery-investigation.md`）。

## Acceptance Criteria

1. **AC1 手机中继首配回退**：QR 携带 `relayAddr`（桌面已配置中继）且 NSD 局域网发现失败/超时后，配对流程自动回退经中继完成：`RelayClient.connect`（register phone 槽 → XX 鉴权）→ E2E XX initiator 握手 → 信任锚校验 → HELLO/deviceInfo/pairingAuth(nonce) 帧序 → 会话建立。配对页三阶段进度（发现设备→交换密钥→验证身份）如实反映中继路径。NSD 成功时行为零变化（直连优先）。QR 未携带 relayAddr 时失败文案如实提示需同局域网（现状保留）。
2. **AC2 桌面确认门（安全裁决，核心）**：`origin=relay` 的**首次配对**不再「扫码即绑定」，改走 pending 待确认流（复用既有换绑 pending 槽 + `pairing_confirm` 命令 + 前端确认 UI）；`origin=direct` 的首配保持扫码即绑定（物理临近为隐式认证因子）。用户点确认 → `confirm_pending` 落库 → 手机重连即 `AlreadyPaired` 进入会话；拒绝或 120s 超时 → 手机侧如实 Failed。**判定必须挂在 `decide_pairing` 的首配分支本身**（按 origin 参数），不能只拦首次连接——手机在 pending 等待期的重试会再次进入首配分支，漏拦即自动落库（见 Dev Notes 陷阱 T1）。
3. **AC3 等待确认期间的中继重试**：`waitDesktopConfirmAndRetry` 的周期重连在 `payload.relayAddr != null` 时改走中继路径（原仅 NSD）；`needsPairingAuth=false` 语义不变（pending 匹配免 nonce 放行）。
4. **AC4 文案与文档如实化**：配对页阶段行「发现桌面设备（NSD 局域网发现）」、失败文案「未发现桌面设备，请确认与桌面端在同一网络」、桌面 `CompanionPairingSection.tsx:292`「首次配对需与电脑处于同一局域网（扫码配对仅支持直连）」、README.md:154/158 三处不再断言「仅支持局域网/中继暂未部署」，改为按中继配置状态如实渲染。顺带修正三处过时注释：`models/companion.rs:23`、`types/companion.ts:12`、README「当前限制」句（文档债，调查档案 Side Findings）。
5. **AC5 零回归与测试**：relay-server **零改动**（协议冻结）；直连配对、已配对离网中继会话、指令通道等既有行为零回归。新增测试：① 桌面集成测试——中继首配 → pending → `pairing_confirm` → 会话建立（改造既有 `relay_path_pairs_and_pings`，其当前断言「首配落库」需改为「首配入 pending」）；② 直连首配仍扫码即绑（守护测试）；③ 手机 JVM 单测——NSD 超时 → relay 回退编排（注入 fake `RelayClient`/`wsOpener` 测试缝）。`npm run test:all` + `:app:testDebugUnitTest` + `npm run build` 全绿。

## Tasks / Subtasks

- [x] Task 1（AC2，先桌面——手机回退依赖确认门语义）: 桌面确认门
  - [x] `companion_pairing.rs::decide_pairing` 增 `origin: PairingOrigin`（enum direct/relay，或 `&'static str` 对齐 `run_authorized_session` 既有 origin 形态）参数；首配分支（`paired_devices` 为空）在 `origin=relay` 时改走 pending 建槽并返回 `PendingRebind`，`origin=direct` 维持 `FirstPairing`（实现取 `&'static str` 形态，与既有 origin 一致——设计裁决 5 倾向前者但明言「实现时择一，不给硬约束」）
  - [x] 调用方 `companion_connection.rs::run_authorized_session`（约 :765）把已有 `origin` 变量传入 `decide_pairing`；两处调用点（直连 `handle_connection` / 中继 `relay_connect_and_serve`）确认传值正确（origin 变量直传，两调用点无需改动）
  - [x] 确认事件链：pending 建立后 `companion_get_status` 的 `pending_pairing` 字段与前端确认 UI 对「无既有设备」场景文案适配（现状文案「新设备请求替换配对」在首配时语义不准，改为按有无已配对设备渲染）
  - [x] 守护测试：直连首配即绑不变；中继首配入 pending；`pairing_confirm` 后重连 `AlreadyPaired`（`confirm_pending` 已用 `upsert_single_device`，首配/换绑无特例——勿加分支）
- [x] Task 2（AC1/AC3）: 手机中继回退编排
  - [x] `RealConnectionClient.runPairing`：捕获 NSD 发现失败（`TimeoutCancellationException`/IOException）且 `payload.relayAddr != null` 时，走 `connectViaRelay(payload)` = `relay.connect` → `e2eHandshake` → `verifyTrustAnchor` → `sendIntroFrames(needsPairingAuth=true, nonce)`；后续 `probeSessionAlive`/`finishPairing` 复用（`store.save` 已含 relayAddr）
  - [x] 抽共享 `connectOverSession`（直连 `connectDirect` 与中继路径仅承载建立方式不同，握手/信任锚/帧序同构——镜像桌面 `WsIo`/`run_authorized_session` 先例；注意 12-4 P5 会话单槽语义不受影响，配对期无并行会话）
  - [x] `waitDesktopConfirmAndRetry`：`payload.relayAddr != null` 时重连走中继（`needsPairingAuth=false`），NSD 路径保留（relayAddr 为空时）；`finally` 不再无脑 `nsd.stopDiscovery()`（中继轮无 NSD 活动时无害，保留即可）
  - [x] 进度阶段映射：中继回退期间 `PairingProgress` 仍按 DiscoveringDevice → ExchangingKeys → VerifyingIdentity 驱动（UI 零改动约束）
- [x] Task 3（AC1）: 失败文案与日志
  - [x] `onPairingFailed`：`phase=="discovery"` 且 QR 含 relayAddr 时文案改「未发现桌面设备，且中继连接失败，请检查网络或中继配置」；无 relayAddr 维持现状
  - [x] 中继回退触发/成功各一条 `CompanionLog.info`（NFR-M7：不打印 relayId 全量/密钥/QR 内容——沿用现有前缀截断先例 `relayId.take(8)`）
- [x] Task 4（AC4）: 文案与文档
  - [x] `PairingScreen.kt:281` 阶段行文案；`CompanionPairingSection.tsx:190/291-294` 按配置渲染；README.md:151-159 手机伴侣连接节改写；`models/companion.rs:23`、`types/companion.ts:12` 注释修正
- [x] Task 5（AC5）: 测试与验证
  - [x] 桌面：改造 `tests/test_companion.rs::relay_path_pairs_and_pings`（:771）为中继首配 pending→confirm→会话；新增直连首配即绑守护（既有单测 `first_pairing_writes_device_directly` 明确 origin=direct 守护语义 + 集成测试直连路径零改动全绿）
  - [x] 手机：`RealConnectionClientOrchestrationTest` 增中继回退用例（fake NSD 超时 + fake RelayClient 成功/中继也失败两分支）
  - [x] 全量验证（见 Completion Notes）；真机跨网冒烟无环境，验证边界已声明

## Dev Notes

- **协议与边界冻结**：8 帧类型、`PROTOCOL_VERSION`、relay 控制协议（register text JSON + XX 鉴权）一律不动——本 story relay-server 零改动。Android `NoiseChannel`/`RelayClient` 逐字镜像的纪律不变。
- **架构依据**：`architecture.md:2057` 配对流程原文即「经直连**或中继**发起 Noise XX 握手」——本 story 是把架构基线落地，不是新增需求。
- **安全模型（裁决核心）**：中继首配的新风险 = QR 内容泄露（拍照/转发）后攻击者可在 5 分钟窗口内从任意网络完成首配并自动绑定 → 连接即获快照数据（FR-41 全量推送）。确认门把信任决策交回桌面用户；直连首配维持即绑（同处一室的物理事实即认证因子）。nonce 单次消费照旧（`companion_connection.rs:760-761` 校验成功清窗重建 pending）。
- **R1 评估结论（不在本 story）**：relay phone 槽公钥绑定是存活连接级（`registry.rs:118-139` 断线即清槽），12-4 R1「直到 relay 重启」的表述与现行代码不符——旧手机断线后新公钥即可注册，换绑场景由既有 pending+confirm 覆盖。无 relay 改动必要。
- **桌面测试基建**：进程内起真 relay 的模式见 `relay_path_pairs_and_pings`（`build_router` + 双端全链路）；手机侧重试预算 14×2.3s 覆盖桌面 5s 慢轮询的既有手法照用。
- **手机测试缝**：`RealConnectionClient` 主构造（internal）注入 `nsd`/`relayClient`/`wsOpener`/`ioDispatcher`（runTest 虚时间确定性必须传 `EmptyCoroutineContext`）；P3 教训——`pairWithQr` 先 join `wipeJob`；P12——`WsSession.send` 失败要即时失败。
- **陷阱 T1（AC2 关键）**：`decide_pairing` 首配分支（`companion_pairing.rs:300-311`）不看 origin 时，手机 pending 等待期重试（pending 匹配放行、免 nonce）会再次命中首配分支自动落库——确认门必须在 `decide_pairing` 内按 origin 分流，而非只在首次连接外层拦。
- **陷阱 T2**：中继首配成功后手机 `store.save` 落 `relayAddr`；后续 `orchestrate` 的中继环启动条件 `store.relayAddr != null` 自然满足——无需额外接线。
- **陷阱 T3**：`relay_gate_open`（`companion_connection.rs:482`）在配对窗口打开时放行桌面中继注册——QR 生成即开窗，中继首配期间桌面侧已注册，无需新增门控逻辑。

### 设计裁决（规则七，显式择一）

| # | 冲突/歧义 | 裁决 | 理由 |
|---|------|------|------|
| 1 | 回退时机：NSD 失败后顺序回退 vs 双路并发竞速 | **顺序回退（NSD 先，12s 预算后转中继）** | 镜像连接期 prefer-direct 哲学；并发竞速引入会话双活管理复杂度（12-4 P5/P18 已为此付出 2 个 patch） |
| 2 | 中继首配是否需要桌面确认 | **需要（pending+confirm）**；直连即绑不变 | QR 泄露 + 全网可达 = 自动绑定会直接泄露快照数据；确认 UI/命令/事件链全部现成，成本最低 |
| 3 | relay-server 是否按 12-3 裁决 3 增强口登记期望手机公钥 | **不启用，relay 零改动** | phone 槽绑定是连接级（registry.rs:118-139），抢占 DoS 有界且断线自愈；确认门已挡落库，公钥 ACL 收益不抵协议变更成本 |
| 4 | R1（中继换绑槽位冲突）是否纳入 | **不纳入，结论记录在案** | 代码证据显示槽位断线即清，换绑已有 pending+confirm 流；仅存旧连接存活期的短暂竞争，V1 单设备可容忍 |
| 5 | `decide_pairing` 的 origin 形态 | enum 或对齐既有 `&'static str` origin（实现时择一，倾向前者） | 类型安全 vs 既有形态一致性——小项，不给硬约束 |

### Project Structure Notes

- 手机改动全部在 `companion-android/app/src/main/java/com/egosync/companion/connection/` + `pairing/PairingScreen.kt`（UI 仅文案，布局不动）；测试同包 `app/src/test/.../connection/`。
- 桌面改动：`egosync-app/src-tauri/src/services/companion_pairing.rs`、`companion_connection.rs`、`tests/test_companion.rs`、前端 `CompanionPairingSection.tsx`。不新增文件、不新增依赖、无 DB 迁移。
- 文档：README.md「手机伴侣连接」节。

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/companion-public-discovery-investigation.md`]（根因与 Follow-up：四层理由、D1/R1 证据链）
- [Source: `_bmad-output/implementation-artifacts/12-4-android-scan-pairing-and-tri-state-connection.md`]（D2→P23 裁决出处、Task 6 桌面中继接线、测试基建先例）
- [Source: `_bmad-output/implementation-artifacts/12-3-relay-server-stateless-relay-and-docker.md`]（裁决 3 增强口、D1-a phone 槽绑定）
- [Source: `egosync-app/src-tauri/src/services/companion_pairing.rs:284-368`]（decide_pairing/confirm_pending 现状）
- [Source: `egosync-app/src-tauri/src/services/companion_connection.rs:444-585,657-792`]（relay 客户端/门控/准入与会话路径）
- [Source: `relay-server/src/registry.rs:96-139`]（槽位绑定与断线清理语义）
- [Source: `companion-android/.../RealConnectionClient.kt:247-368`]（runPairing/waitDesktopConfirmAndRetry/onPairingFailed 现状）

## Dev Agent Record

### Agent Model Used

glm-5.3（DeepSeek Harness，bmad-agent-dev / Amelia）

### Debug Log References

- 集成测试 `relay_path_pairs_and_pings` 首轮失败（14 次重试全 miss）→ 以进程内 relay + tracing 取证：手机 E2E m1 撞上桌面转发态空窗被 relay 丢弃（无离线投递），手机 2s 超时断开又触发 relay 对端 close 通知杀死刚重建的桌面槽位，退避翻倍后窗口更难命中——活锁。修复：测试 helper 与手机端中继握手均在 m2 等待期周期重发 m1（`e2eHandshakeRelay`，重发 1s/步预算 10s；直连不重发）。收敛时间 34.9s → ~2.3s。
- 诊断手段（规则十三）：临时 eprintln 段标记 + 测试二进制 tracing 初始化；定位后全部移除（grep 验证零残留）。

### Completion Notes List

- **AC2 确认门**：`decide_pairing` 增 `origin: &'static str` 参数（设计裁决 5 明言形态可任选——对齐 `run_authorized_session` 既有 origin 形态取字符串）；首配分支 `origin=relay` 落入 pending（与换绑共用 confirm 链，`confirm_pending` 无特例），`origin=direct` 维持即绑。判定挂在首配分支本身（陷阱 T1 由单测 `relay_pending_wait_retry_does_not_auto_bind` 锁定）。
- **AC1 手机回退**：`runPairing` NSD 发现失败（超时/IOException，非真实取消）且 QR 携带 relayAddr 时回退 `connectViaRelay`；顺序回退（NSD 12s 预算先行），NSD 成功零变化。共享序列抽 `connectOverSession`（信任锚+帧序），直连/中继仅承载建立方式不同。
- **AC3 等待期重试**：`waitDesktopConfirmAndRetry` 在 `relayAddr != null` 时重连走中继（`needsPairingAuth=false`）；`finishPairing` 增 `viaRelay` 如实标记承载态（ConnectionState.Relay）。
- **AC4 文案**：`PairingScreen` 发现阶段行按 `qrHasRelay` 如实渲染（「局域网优先，中继兜底」/「NSD 局域网发现」）；等待确认文案不再断言「桌面已有配对设备」（中继首配也会触发）；`CompanionPairingSection` 顶部说明/QR 区文案、pending 卡片按有无已配对设备分「请求配对/请求替换配对」；README「手机伴侣连接」节改写 + 3 处过时注释修正（models/companion.rs、types/companion.ts、README「当前限制」句）。
- **AC5 验证结果（全绿）**：cargo test 842 unit + 37 integration + 1 relay-server；vitest 438/438（42 文件）；`npm run build`（tsc 零类型错误 + Vite）✓；Android `:app:testDebugUnitTest` 226/226（新增中继回退 3 例：回退成功含 relayAddr 落盘、双失败合并文案、pending 等待期走中继确认后成功）；`:app:assembleDebug` ✓。relay-server 零改动（`git status` 未见本 story 触碰 relay-server）。
- **验证边界（如实声明）**：真机跨网扫码冒烟无物理环境（本机仅 loopback 仿真全链路），未执行；建议 UAT 阶段以真中继 + 双网络实测。
- **基线外修复（显式暴露，非本 story 范围但阻断验证）**：`companion-android/gradle/libs.versions.toml` 存在上一提交 b1f8258 遗留的 4 条引用已删 `camerax` 版本号的悬空 CameraX 条目，任何 Gradle 调用即失败——删除该 4 条死条目恢复构建（该提交已从 build.gradle.kts 移除 CameraX 依赖，条目为纯残留）。另：`Cargo.lock` 0.1.4→0.1.5 为上一 release bump（c4e0a6b）遗漏的 lock 同步，本 story 顺带落盘。

### File List

- `egosync-app/src-tauri/src/services/companion_pairing.rs`（decide_pairing origin 分流 + 3 新单测）
- `egosync-app/src-tauri/src/services/companion_connection.rs`（origin 传参 + 注释更新）
- `egosync-app/src-tauri/src/models/companion.rs`（QrPayload 注释修正）
- `egosync-app/src-tauri/src/types/companion.ts`（relayAddr 注释修正）
- `egosync-app/src-tauri/tests/test_companion.rs`（relay_path_pairs_and_pings 改造为 pending→confirm→重连 + relay_e2e_connect 抽取 + m1 重发）
- `egosync-app/src/components/settings/CompanionPairingSection.tsx`（pending 卡片首配/换绑文案 + QR 区/顶部说明如实化）
- `companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt`（中继回退 + connectViaRelay + e2eHandshakeRelay m1 重发 + connectOverSession 抽取 + waitDesktopConfirmAndRetry 走中继 + onPairingFailed 双失败文案 + viaRelay 承载态）
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt`（qrHasRelay 阶段行文案 + 等待确认文案）
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingViewModel.kt`（qrHasRelay 流）
- `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt`（qrHasRelay 接线）
- `companion-android/app/src/test/java/com/egosync/companion/connection/RealConnectionClientOrchestrationTest.kt`（泵替身泛化 + FakeRelay + 中继回退 3 用例）
- `companion-android/gradle/libs.versions.toml`（删除 4 条悬空 CameraX 条目——基线外阻断性修复）
- `README.md`（手机伴侣连接节如实化）
- `egosync-app/src-tauri/Cargo.lock`（0.1.5 lock 同步——上一 release bump 遗漏，非本 story 改动）

### Change Log

- 2026-09-04：Story 12.5 实现完成——中继首配回退（AC1/AC3）、桌面确认门（AC2）、文案与文档如实化（AC4）、全量测试（AC5）。relay-server 零改动（协议冻结）。状态 → review。
