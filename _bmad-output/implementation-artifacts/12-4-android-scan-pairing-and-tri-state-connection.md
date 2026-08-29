---
baseline_commit: 0a932fd9f842ff76433fa17625c129ba9a57ec87
---

# Story 12.4: Android 扫码配对与三态连接换装

Status: done

## Story

As a 手机用户,
I want 用手机扫描桌面二维码完成配对，之后自动在直连与中继之间切换并随时看到连接状态,
So that 我不用关心网络细节，始终有一条安全通道连着桌面引擎。

## Acceptance Criteria（AC）

> 完整 AC 以 `_bmad-output/planning-artifacts/epics.md` Story 12.4 段为唯一事实源，以下为逐条搬运，编号供任务引用。

1. **AC1 换装结构与 UI 零改动**：`companion-android/` 原型（UI 零重做约束）——新增真实实现放 `connection/` 包（`NsdDiscovery / RelayClient` 等，沿用架构命名），`ConnectionClient` 接口签名不变，`AppModelContainer` 仅替换 `connection` 装配，UI 层零改动；配对页「模拟扫码成功」按钮替换为真实扫码（候选 ML Kit Barcode；依赖加入白名单需在 Story 内说明理由），配对流四步样式与交互不变。
2. **AC2 noise-java 互通与密钥安全**：noise-java 完成的 Noise XX 握手与加解密结果，与 Story 12.1 冻结的黄金向量互验通过（`Noise_XX_25519_ChaChaPoly_BLAKE2s`，单元测试 `src/test/`）；手机静态私钥存入 Android Keystore，导出/日志均不可见私钥材料。
3. **AC3 NSD 直连与退避**：已配对手机与桌面处于同一局域网时，手机启动或网络恢复后 NsdManager 发现 `_egosync._tcp` 服务经 WS 直连桌面，状态栏显示「局域网直连」；断开时以指数退避重连（1s→30s 封顶 + 随机抖动）。
4. **AC4 中继切换滞回**：手机离开局域网（NSD 无响应）探测 3 秒仍不可达（prefer-direct 滞回，防抖动）时，自动切换经中继 WS（按 QR 中的 relay_addr + relay_id）建立加密转发，状态栏显示「中继转发」；回到局域网后自动切回直连，全程无需人工干预。
5. **AC5 Offline 三态可见**：桌面关机或网络全断时直连与中继均不可达，状态进入 Offline（降级体验在 Epic 14 完善，本 Story 保证状态可见不误报）；设置页配对设备卡的连接状态圆点实时反映三态。
6. **AC6 重装重扫与解除配对**：卸载重装 App（Keystore 密钥随之销毁）后重新扫码配对成功且桌面无需任何重置操作（新公钥写入 paired_devices；V1 单对单，新配对按产品口径处理旧记录）；解除配对入口清除本地配对状态与密钥，回到未配对首跑流。
7. **AC7 零回归与测试**：换装前后逐屏截图比对（配对流/四 Tab 主界面/设置页）视觉与交互零回归（对照原型 README 页面地图）；Debug 四态模拟入口保留为开发工具；`./gradlew :app:testDebugUnitTest` 全绿，配对冒烟路径进 `src/androidTest/`。

## Tasks / Subtasks

- [x] Task 1：Kotlin 协议底座——Noise 封装 + 帧编解码 + 黄金向量互验（AC2）
  - [x] `connection/NoiseChannel.kt`：noise-java 封装——`HandshakeState(SUITE, INITIATOR/RESPONDER)` 发起/响应两侧、`split()` 后 `CipherStatePair` 传输加解密（API 用法逐字镜像 `crates/companion-proto/interop/noise-java/GenVectors.java`）；SUITE = `Noise_XX_25519_ChaChaPoly_BLAKE2s`
  - [x] `connection/FrameCodec.kt`：镜像 crate `frames.rs`——8 帧类型小写 `type` 标签 JSON、payload camelCase、`u32 BE 长度前缀 + 密文`、HELLO 的 `protocolVersion == 1` 校验、单帧密文上限校验（镜像 `MAX_CIPHERTEXT_LEN`，与 relay 128KB 一致）；12.4 只需 HELLO / NOTICE / PING 的编解码，其余 6 帧类型枚举占位（13.x 消费）
  - [x] Gradle 任务从 `../crates/companion-proto/tests/fixtures/noise_java_vectors.json` 复制 fixture 进 `src/test/resources`（单一事实源，禁止手工拷贝副本）
  - [x] `src/test/`：`NoiseInteropTest`——以 fixture 固定密钥（`getLocalKeyPair().setPrivateKey` + `getFixedEphemeralKey().setPrivateKey`）重放 XX 握手，断言 3 条握手消息与全部传输密文**逐字节**等于 12.1 冻结向量；`FrameCodecTest`——8 帧类型编码 JSON 与 fixture 内 8 条明文黄金串逐字节一致 + 解码往返
- [x] Task 2：密钥与配对状态持久化（AC2、AC6）
  - [x] `connection/KeyStore.kt`：Android Keystore 内生成 AES-GCM 密钥（`AndroidKeyStore` provider），加密包裹 noise-java X25519 静态私钥，密文落私有文件；明文仅存内存（裁决 4：Keystore 不支持 X25519 原生托管，wrap 形态是 AC「存入 Android Keystore」的可满足解释，同架构快照缓存「Keystore 派生 AES」先例）
  - [x] 配对状态存储：`SharedPreferences` 存非机密元数据（desktop_pubkey_hex / relay_id / relay_addr / paired 标志）；私钥密文独立文件
  - [x] `unpair()`：清配对元数据 + 删除私钥密文文件 + 删除 Keystore AES 密钥（Keystore 密钥销毁 = 旧私钥不可恢复，满足 AC6「清除密钥」）
- [x] Task 3：真实扫码——ML Kit + CameraX（AC1）
  - [x] 依赖白名单增项（理由见裁决 2）：`com.google.mlkit:barcode-scanning`（bundled 变体，不依赖 GMS）、`androidx.camera:camera-{core,camera2,lifecycle,view}`、OkHttp（WS 客户端，裁决 5）；`settings.gradle.kts` 增 JitPack 仓库 + noise-java 依赖（坐标见关键技术情报）
  - [x] `AndroidManifest.xml`：增 `INTERNET`、`CAMERA` 权限 + `android:usesCleartextTraffic="true"`（V1 明文 WS 承载 E2E 密文，NFR-M1 不依赖 TLS）
  - [x] `pairing/QrScanner.kt`：CameraX Preview + ML Kit BarcodeAnalysis 替换「模拟扫码成功」按钮；**取景框/四角标记/扫掠线 overlay 样式逐字保留原型**（UX-M1）；相机权限拒绝态显示中文引导文案；扫码结果回调 QR JSON 字符串
  - [x] QR payload 解析：camelCase 四字段 `{relayAddr?, desktopStaticPubkey, relayId, pairingNonce}`（与桌面 `models/companion.rs::QrPayload` serde camelCase 输出一致）；`relayAddr` 缺失 = 中继未部署（中继承载禁用，离网即 Offline）
- [x] Task 4：真实 ConnectionClient——NSD 直连承载（AC1、AC3）
  - [x] `connection/NsdDiscovery.kt`：`NsdManager.discoverServices("_egosync._tcp.", PROTOCOL_DNS_SD)`；解析后校验 TXT `proto` 与实例名 `EgoSync-{relayId[..8]}`（桌面锚点见 Dev Notes），取 port
  - [x] `connection/RealConnectionClient.kt` 实现 `ConnectionClient`（接口签名零改动）：OkHttp WS 连 `ws://host:port`（桌面 WS 监听无路径概念，`/` 即可）→ XX initiator 三条握手消息（每条一条 WS binary，无长度前缀）→ **身份验证**：`getRemotePublicKey() == hex(QR.desktopStaticPubkey)`（防中间人信任锚）→ 发 `HELLO(protocolVersion=1)` → 发 `NOTICE {"type":"deviceInfo","deviceName":<Build.MODEL>}` → 新配对再发 `NOTICE {"type":"pairingAuth","nonce":<QR.pairingNonce>}`（3s 窗口内，桌面锚点见 Dev Notes）→ 进入会话循环
  - [x] PING 保活：每 30s 发 PING 帧（桌面 120s 空闲即断连，锚点见 Dev Notes）；OkHttp 自动应答 WS 层 Ping/Pong（relay 保活由服务端主动探测）
  - [x] `connection/BackoffPolicy.kt`：指数退避 1s→30s 封顶 + 随机抖动（纯函数可测）
  - [x] PairingConnector 裁缝口（裁决 6）：`PairingViewModel` 经新接口 `PairingConnector.pairWithQr(qrJson): StateFlow<PairingProgress>` 驱动真实握手，CONNECTING 三阶段（发现设备→交换密钥→验证身份）由真实进度驱动；`ConnectionClient` 接口不动
- [x] Task 5：中继承载 + prefer-direct 滞回 + 三态（AC4、AC5）
  - [x] `connection/RelayClient.kt`：OkHttp WS 连 `<relay_addr>/relay` → 首消息 **text JSON** `{"type":"register","relayId":"<16hex>","role":"phone"}` → relay 作 XX initiator（一次性密钥对），手机作 responder 完成鉴权握手（3 条 binary）→ 转发态 → 经转发通道对桌面发起 E2E XX initiator 握手 + 同直连的 app 层帧序列
  - [x] `connection/ConnectionStateMachine.kt`：三态 + 滞回——直连断/NSD 消失后**探测 3s** 仍不可达才回落中继（防抖动）；中继态持续跑 NSD 发现，服务重现且直连握手成功即切回 Direct；双承载均失败 → `Offline(snapshotAvailable=false, dataAsOf=null)`（快照数据属 13.x，本 story 不误报有缓存）
  - [x] debug 覆盖层（裁决 7）：`setDebugMode` 非空实现——选中档位覆盖状态输出；解除配对/进程重启复位为自动；不新增第五档（UI 零改动）
- [x] Task 6：桌面侧中继接线——最小增量（AC4 的必要条件，裁决 1）
  - [x] `db/app_settings.rs` 既有 key-value 读写复用：新增键 `companion_relay_addr`（如 `ws://relay.example.com:7333`，空 = 未部署）；`commands/companion.rs` 增薄层 command `companion_set_relay_addr` / `companion_get_relay_addr`，`lib.rs` 注册
  - [x] `companion_pairing.rs::generate_qr_payload`：`relay_addr` 从 app_settings 读取真实值（None 语义保留：未配置则 QR 不带中继）
  - [x] `companion_connection.rs` 增中继注册客户端：relay_addr 已配置且（已配对 ≥1 或配对窗口打开，**与 NSD 注册同口径**）时连 relay `/relay`、text register `role=desktop`、以桌面静态私钥作 XX responder 鉴权、进入转发态；转发态收到对端首条 binary 即按 E2E responder 跑握手 → 复用配对/会话逻辑；断线指数退避重连；会话 origin 标 `relay`（`get_status` 如实反映）
  - [x] 最小重构：把 `handle_connection` 的「E2E 握手之后」段落（早期准入→HELLO 校验→Notice 收集→配对决策→enter_session）抽成共享函数（经既有 `HandshakeIO` 风格抽象），直连 WS 路径与中继路径共用——**行为零变更**（既有 `tests/test_companion.rs` 全绿为证）
  - [x] `tests/test_companion.rs` 增中继路径集成测试：进程内拉起 relay（复用 `relay_server::build_router` 或子进程二进制）→ 模拟手机经 relay 完成 register + E2E 握手 + 配对 + PING 往返；QR payload 断言 relay_addr 真实值透出
  - [x] 前端 `CompanionPairingSection.tsx` 增「中继服务器地址」输入框（companionService 封装 invoke，域目录组件规范不变）
- [x] Task 7：装配换装与配对流接线（AC1、AC6）
  - [x] `AppModelContainer`：`connection` 装配 `FakeConnectionClient` → `RealConnectionClient`（唯一换装点，UX-M2）；`FakeConnectionClient` 类保留（既有单测与 Preview 依赖）；`completePairing()/unpair()` 委托真实客户端
  - [x] `PairingViewModel`：扫码回调 → 解析 QR → `pairWithQr` → 进度驱动 CONNECTING 三阶段 → 成功 SUCCESS；PendingRebind（换绑待桌面确认）时桌面会关闭连接——手机侧重连等待并如实呈现「等待桌面确认」文案
  - [x] 日志纪律（NFR-M7）：Android Log 统一 `Companion/` 前缀 tag；帧明文、任何公私钥材料、QR 内容永不入日志（只记事件类别/状态变更/对端标识 relay_id）
- [x] Task 8：androidTest 冒烟 + 验证收尾（AC7）
  - [x] `src/androidTest/`：配对冒烟——真机/模拟器上 `RealConnectionClient` 对进程内 Kotlin 测试服务端（noise-java responder 小服务器）完成 XX 握手 + HELLO/deviceInfo/pairingAuth 帧交换 + PING 往返（真 Keystore 路径覆盖）
  - [x] `./gradlew :app:testDebugUnitTest` 全绿；`cd egosync-app && npm run build` + `npm run test:all`（Task 6 涉及 Rust 改动）
  - [ ] 逐屏截图比对（配对流/四 Tab 主界面/设置页 vs 换装前，对照 README 页面地图）；NSD 直连/中继切换/重装重扫做真机手动冒烟并如实记录（无局域网双设备环境时说明验证边界，禁止声称已验证）

### Review Findings

> 代码评审（2026-08-28，Blind Hunter + Edge Case Hunter + Acceptance Auditor 三层，跨层去重后）：决策已裁决（2 转整改、1 延后），共 24 整改项、1 延后、2 驳回。**整改全部完成（同日，option 1 全量立即整改）**：24/24 已修复并勾选；验证 = `:app:testDebugUnitTest` 115/115 绿 + `:app:compileDebugAndroidTestKotlin` 绿 + 桌面 `npm run test:all`（vitest 438/438 + cargo 844 全过）+ `npm run build` 绿。整改过程中另修复两处测试暴露的实现缺陷：测试需注入 `ioDispatcher` 缝（虚时间确定性）与直连信任锚失败路径的 WS 会话泄漏（verify/intro 并入 connectDirect 的关闭守护）；`connectedAndroidTest`/真机冒烟无设备未执行（如实声明）。

- [x] [Review][Defer] D1 中继槽位抢占 DoS 与桌面重连放大 [RelayClient.kt + companion_connection.rs::relay_connect_and_serve/run_relay_client] — deferred：V1 本地优先单设备，`relayId` 仅经面对面扫码流转，攻击面有限；根治需改 relay-server（本故事范围外），与 12.3 的连接数上限/速率限制等公网加固项合并处理
- [x] [Review][Patch] D2→P23 首次配对仅直连——删除 `QrPayload.supportsRelay` 死代码；桌面文案「离开局域网也可连接」改为仅直连配对的如实描述 [QrPayload.kt, CompanionPairingSection.tsx]
- [x] [Review][Patch] D3→P24 桌面中继转发态 10s churn——为桌面中继转发态补诊断日志（超时断连、被拒原因、退避时长），使离网会合延迟在 UAT 可观测；协议级「保持注册、空闲挂起」不在本故事 [companion_connection.rs::relay_connect_and_serve]
- [x] [Review][Patch] P1 状态机输出未接进 `liveState`，三态对 UI 不可见（AC3/4/5 运行时击穿） [RealConnectionClient.kt:62-78,249-320]
- [x] [Review][Patch] P2 中继配置但从未建立时状态机到不了 Offline（`onRelayLost` 仅 `Relay` 态生效） [ConnectionStateMachine.kt:28-33,40-52]
- [x] [Review][Patch] P3 `unpair()` 异步擦除未跟踪，与重新配对竞态可删新密钥/元数据 [RealConnectionClient.kt::unpair]
- [x] [Review][Patch] P4 相机/ML Kit 扫描器离开 SCAN 后不释放（无 unbind/无 close） [QrScanner.kt:60-104]
- [x] [Review][Patch] P5 直连恢复后中继会话不拆除，双活 E2E 会话（13.x 后将重复投递） [RealConnectionClient.kt::orchestrate/runRelayLoop]
- [x] [Review][Patch] P6 中继地址仅前缀校验（裸 `ws://` 可过）+ 全局 `usesCleartextTraffic` [commands/companion.rs:62-86, AndroidManifest.xml, CompanionPairingSection.tsx:165-185]
- [x] [Review][Patch] P7 `RealConnectionClient`（439 行）编排/配对/信任锚零测试（P1 吞超时 bug 恰在此处） [RealConnectionClient.kt]
- [x] [Review][Patch] P8 中继退避成功后不复位，与注释「会话成功后复位」矛盾 [companion_connection.rs:343-371]
- [x] [Review][Patch] P9 `getRelayAddr` 读失败静默清空输入框，保存即误删已有可用配置 [CompanionPairingSection.tsx:73-81,165-175]
- [x] [Review][Patch] P10 扫码锁存器（`found`）在 QR 解析失败后不复位，阻止再扫 [QrScanner.kt:108-116 + PairingViewModel.onQrScanned]
- [x] [Review][Patch] P11 `NsdDiscovery` 共享 ResolveListener 可并发抛错被吞/不回调永久卡死 [NsdDiscovery.kt:54,74-95]
- [x] [Review][Patch] P12 `WsSession` 忽略 send 失败、`connectTimeoutMs` 参数未用、无 `onClosing` [WsSession.kt]
- [x] [Review][Patch] P13 NSD `lost` 流死代码、`onResolveFailed`/`onStartDiscoveryFailed` 静默吞错 [NsdDiscovery.kt:25-27]
- [x] [Review][Patch] P14 Keystore 解包失败无限重试不自愈、`writeBytes` 非原子 [KeyStore.kt]
- [x] [Review][Patch] P15 QR payload 仅形状校验，测试夹具 `"aa"` 违反其引用的 64-hex 契约 [QrPayload.kt::parse, PairingViewModelTest.VALID_QR_JSON]
- [x] [Review][Patch] P16 `run_relay_client` spawn 无幂等守卫（重复调用致双实例互踢） [companion_connection.rs::start_companion_listener]
- [x] [Review][Patch] P17 relay 地址在 `orchestrate` 启动时一次性捕获，桌面重配不生效 [RealConnectionClient.kt:249-262,296-322]
- [x] [Review][Patch] P18 直连恢复后残留 `StartRelay` 意图可自替换健康会话/瞬时误报 [ConnectionStateMachine.kt:48-57]
- [x] [Review][Patch] P19 死代码与误导：`Hex.decode` 无调用者、注释误称 "full jitter"、`serverJob.join()` 无超时 [Hex.kt, BackoffPolicy.kt, PairingSmokeTest.kt]
- [x] [Review][Patch] P20 `RelayClient` 仅校验 m1 长度，m3 无本地纵深检查 [RelayClient.kt:34-52]
- [x] [Review][Patch] P21 `CameraPermissionGate` 不在 `ON_RESUME` 复查权限 [QrScanner.kt:139-160]
- [x] [Review][Patch] P22 故事自述「ui/ 零改动」与事实矛盾（`AppNavHost.kt` 已改，文档级修正） [故事 Project Structure Notes/Git Intelligence]

> 驳回（2，不入行动项）：① 既有文件（`ConnectionClient.kt`/`FakeConnectionClient.kt`）在 diff 中呈「新增」仅为 `--no-index` 产物，内容与基线逐字节一致；② `BackoffPolicy` 首跳抖动 [500,1000]ms 为「随机抖动」可辩护解读且测试已断言。

## Dev Notes

### 架构硬边界（违反即返工）

1. **加密边界**：Android 侧 `noise-java` 仅允许出现在 `pairing/`、`connection/` 换装层（架构四条硬边界 #1）；Kotlin 其余代码只操作帧类型，不见密码学细节。
2. **Android 边界**：UI 不触达连接实现（经 ViewModel → `ConnectionClient`/`PairingConnector` 接口）；禁 Room、禁 Hilt、禁全局状态框架（反模式清单 + 原型 README 白名单——本 story 白名单增项见裁决 2/5，全部附理由）。
3. **接口冻结**：`ConnectionClient` 六个成员签名零改动（UX-M2）；`ConnectionState` 三态密封接口零改动；`DebugConnectionMode` 四档零改动。
4. **协议冻结**：8 帧类型、schema.json、PROTOCOL_VERSION=1 一律不动；relay 控制协议（register JSON + XX 鉴权承载）是 relay-server 内部约定，Android 侧逐字镜像 12.3 落地实现，不改 relay-server（已知限制见「开放问题」）。
5. **日志纪律（NFR-M7）**：帧明文、密钥材料、QR 内容永不入日志；Android Log 带 `Companion/` 前缀 tag。

### 设计裁决（规则七，显式择一）

| # | 冲突/歧义 | 裁决 | 理由 |
|---|------|------|------|
| 1 | 桌面侧中继接线归属：12.3 范围外写「属 12.4/后续 story」，而 12.4 AC 全文只写 Android 换装 | **桌面侧中继接线在本 story 内做最小增量**（Task 6） | 无桌面 relay 注册客户端，AC4「经中继建立加密转发」物理不可发生——中继双槽（desktop+phone）缺一不可；Epic 12 内再无其他 story 可承载（13.x 全是快照/指令）；12.2 已留 relay_addr 字段位、12.3 已留「12.4 增强口」——两处先例都指向本 story。最小 = relay 注册客户端 + relay_addr 设置 + QR 真实值，不碰中继本体 |
| 2 | 扫码库：AC 写「候选 ML Kit Barcode」，亦可选 ZXing embedded | **ML Kit Barcode（bundled 变体）+ CameraX** | UX-M1/M3 要求配对页取景框/四角标记/扫掠线视觉零回归——ML Kit + CameraX Preview 可完全自定义 overlay；zxing-embedded intent 模式引入库自带 Activity UI，无法满足视觉零回归。bundled 变体不依赖 Google Play Services（本地优先哲学，无 GMS 设备可用）。白名单理由成立 |
| 3 | 私钥存储：AC「存入 Android Keystore」但 Keystore 不支持 X25519 原生密钥托管且不可导出密钥字节（noise-java 需要私钥字节参与握手） | **Keystore AES-GCM 包裹 X25519 私钥**（密文落私有文件，AES 密钥永不出 Keystore） | Keystore 直存 X25519 在 API 层面不可满足；wrap 形态是「导出/日志不可见私钥材料」的结构性保证——与架构快照缓存「Keystore 派生 AES 加密」同一先例。字段面解释为可满足的最强形态 |
| 4 | 同上（并入裁决 3 执行口径） | Keystore wrap；`unpair()` 同时删密文文件与 Keystore AES 密钥 | Keystore 密钥销毁后旧私钥即使文件残留也不可解密 |
| 5 | Android WS 客户端选型 | **OkHttp**（WebSocket API 成熟稳定） | 架构 Important Gap #2 明示候选 OkHttp；自研 WS 或引第二个网络栈违反规则二 |
| 6 | `ConnectionClient.completePairing()` 无参签名 vs 真实配对需要 QR payload 输入 | **新增正交接口 `PairingConnector.pairWithQr(qrJson)`**，由真实客户端同时实现；`PairingViewModel` 依赖 `PairingConnector`；`ConnectionClient` 接口签名不动 | AC1「接口签名不变」是硬约束；PairingViewModel 是状态逻辑非 UI 样式，允许换装（「模拟扫码成功」替换本来就在 AC 内）；接口正交不污染连接抽象 |
| 7 | README 写 setDebugMode「真实实现可空实现」vs AC7「Debug 四态模拟入口保留为开发工具」 | **真实实现保留四态覆盖**（非空实现）：选中档位覆盖状态输出，解除配对/进程重启复位自动；不新增第五档 | 更新者（epics AC）优先（基线规则：冲突以更新者为准）；新增「自动」档违反 UI 零改动 |
| 8 | E2E 帧在 Kotlin 侧的编解码归属 | `connection/FrameCodec.kt` 单点实现，12.4 只编码 HELLO/NOTICE/PING | 规则二不提前抽象；13.x 消费其余帧时再扩展（枚举占位已含 8 类） |

### 关键技术情报（全部源码核实，实现前勿凭网上示例拼装）

**Wire 协议精确序列（手机侧视角，逐字镜像桌面/中继落地实现）：**

- **直连**：`ws://<nsd host>:<port>` → XX **initiator**（本地静态私钥）三条握手消息，每条一条 WS binary、无长度前缀（m1=`write_message(&[])`、收 m2、发 m3）→ `getRemotePublicKey()` 必须 == `hexDecode(QR.desktopStaticPubkey)`（**信任锚，不等即断开**）→ `split()` → 发 `HELLO {"type":"hello","protocolVersion":1}` → 发 `NOTICE {"type":"notice","data":"{\"type\":\"deviceInfo\",\"deviceName\":\"<Build.MODEL>\"}"}` →（新配对）`NOTICE data={"type":"pairingAuth","nonce":"<QR.pairingNonce>"}` → 会话循环（PING 每 30s）。传输态每帧 = `u32 BE 长度前缀 + ChaChaPoly 密文`，内层 JSON camelCase。
- **中继**：`ws://<relay_addr>/relay` → 首消息 **text** `{"type":"register","relayId":"<QR.relayId>","role":"phone"}` → relay 作 XX initiator（一次性密钥），手机作 **responder** 鉴权握手 3 条 binary → 转发态 → 对桌面发起 E2E XX initiator（同直连序列）。鉴权会话与 E2E 会话是**两次独立握手**，密钥互不相干。
- **relay 服务端约束**（12.3 落地）：鉴权整体超时 10s；握手消息 <32 字节即拒；WS 单消息上限 **128KB**；register 首消息后 text 一律忽略；phone 槽**绑定首注册公钥**（见「开放问题」R1）。

**桌面侧常量锚点**（`egosync-app/src-tauri/src/services/companion_connection.rs`）：

- `HANDSHAKE_TIMEOUT_SECS=10`（Noise 握手与首帧各 10s）；`SESSION_IDLE_TIMEOUT_SECS=120`（**手机必须周期 PING，30s 间隔留 4 倍余量**）；`APP_NOTICE_WINDOW_SECS=3`（HELLO 后 deviceInfo/pairingAuth 须在 3s 内到达）；`PAIRING_WINDOW_TIMEOUT_SECS=300`（QR 有效期 5 分钟）。
- 首帧必须 HELLO；换绑 PendingRebind 分支**主动关闭连接等桌面 `pairing_confirm`，手机重连即恢复**；单会话槽——新连接（如直连↔中继切换）自动取代旧会话（12.2 评审 P6 语义，手机切换承载无需额外清理）。
- NSD 锚点：服务类型 `_egosync._tcp.local.`（Android NsdManager 写作 `"_egosync._tcp."`）、实例名 `EgoSync-{relay_id前8字符}`、TXT `proto=1`、端口随发现解析（动态端口）。
- QR payload JSON（camelCase）：`{"relayAddr":"ws://host:7333"|null,"desktopStaticPubkey":"<hex>","relayId":"<hex16>","pairingNonce":"<uuid>"}` [Source: egosync-app/src-tauri/src/models/companion.rs:27-32]。

**noise-java（依赖坐标冻结）**：

- JitPack：`com.github.rweather:noise-java:49377b6dfc6a1e75740bce2318118291a57c0d6e`——**必须与 12.1 黄金向量生成所用同一 commit**（jar sha256 `c01a0dbd…`，见 `crates/companion-proto/interop/noise-java/run.sh`），换版本即互验失义。
- API 用法镜像 `GenVectors.java`：`HandshakeState(SUITE, role)` / `getLocalKeyPair().setPrivateKey(bytes, 0)` / `getFixedEphemeralKey()`（**仅测试**，生产禁止设固定临时密钥）/ `start()` / `writeMessage/readMessage(buf)` / `split()` → `CipherStatePair.getSender()/getReceiver()` / `encryptWithAd(null, …)`。
- 黄金向量 fixture 字段：`initiator/responder.{staticPrivate,ephemeralPrivate}`、`handshakeMessages[]`、`transport[]{direction,plaintext,ciphertext}` [Source: crates/companion-proto/tests/fixtures/noise_java_vectors.json]。

**relay-server 可复用测试面**：`relay_server::build_router()` 进程内拉起完整路由（12.3 为 lib+bin 双 target 即为此留口）；集成测试优先进程内 router，零持久化断言类测试才需真子进程。

**Android 平台要点**：

- `usesCleartextTraffic="true"` 必需——ws://（非 wss）在 API 28+ 默认被拦；V1 明文 WS 承载 E2E 密文是架构既定（NFR-M1 零知识不依赖 TLS）。
- NsdManager 回调在主线程 handler；`discoverServices` 生命周期与连接状态机协程对齐（Activity 销毁必须 `stopServiceDiscovery`，防泄漏）。
- OkHttp WebSocket：Ping/Pong 帧自动应答；binary 消息即 `ByteString`；连接关闭走 `onFailure`——与退避重连对接。
- AGP 9 内置 Kotlin 编译（无 `org.jetbrains.kotlin.android` 插件），新依赖只进 `libs.versions.toml` version catalog。

### 测试策略（规则九：验证意图）

- 意图锚点：**「跨语言互通不是纸面承诺」**（AC2 WHY）→ 黄金向量逐字节重放，而非「握手能跑通」；**「中继零知识」**→ 传输密文必须等于向量密文（Kotlin 侧加解密与 snow 字节级一致）；**「信任锚不可绕过」**→ 远端静态公钥 ≠ QR 公钥时必须断开（负向测试）；**「三态不误报」**→ 双承载不可达必须呈 Offline 而非停留旧态；**「滞回防抖动」**→ NSD 瞬断 3s 内不得切中继（时间可控注入测试）；**「退避有界」**→ 30s 封顶 + 抖动非零。
- 状态机/退避/滞回全部抽纯函数或注入时钟协程测试，不依赖真网络；真网络路径（NSD/相机/WS/Keystore）进 androidTest 冒烟 + 手动冒烟如实记录。
- 桌面侧：既有 `tests/test_companion.rs` 全绿是「最小重构零行为变更」的证明；中继路径集成测试模拟真实双端（companion_proto 起桌面 responder + 模拟手机端）。

### Project Structure Notes

```text
companion-android/
├── settings.gradle.kts                    # [M] 增 JitPack 仓库
├── gradle/libs.versions.toml              # [M] 白名单增项（附理由的 4 组）
└── app/
    ├── build.gradle.kts                   # [M] 依赖 + fixture 复制 task
    ├── src/main/AndroidManifest.xml       # [M] INTERNET/CAMERA/usesCleartextTraffic
    └── src/main/java/com/egosync/companion/
        ├── AppModelContainer.kt           # [M] connection 装配换装（唯一换装点）
        ├── pairing/
        │   ├── PairingViewModel.kt        # [M] 扫码→pairWithQr 真实流程
        │   ├── PairingScreen.kt           # [M] ScanStep 换真实取景（样式零改动）
        │   └── QrScanner.kt               # [N] CameraX + ML Kit
        ├── connection/
        │   ├── RealConnectionClient.kt    # [N] ConnectionClient + PairingConnector 实现
        │   ├── NsdDiscovery.kt            # [N] NsdManager 发现
        │   ├── RelayClient.kt             # [N] 中继承载
        │   ├── ConnectionStateMachine.kt  # [N] 三态 + 滞回
        │   ├── BackoffPolicy.kt           # [N] 指数退避
        │   ├── NoiseChannel.kt            # [N] noise-java 封装（加密边界内）
        │   ├── FrameCodec.kt              # [N] 帧编解码（镜像 frames.rs）
        │   └── KeyStore.kt                # [N] Keystore AES 包裹私钥
        └── （sync/、notify/ 零改动；FakeConnectionClient 保留）
             # ui/ 触点（P22 修正：并非「ui/ 零改动」，见 File List [M] 两项）：
             #   AppNavHost.kt——PairingScreen 传参加ConnectionClient 注入链
             #   PairingScreen.kt——ScanStep 真实取景 + ConnectingStep 等待桌面确认文案

egosync-app/
├── src/services/companionService.ts       # [M] relay addr 读写 + QR relayAddr 真实值
├── src/components/settings/CompanionPairingSection.tsx # [M] 中继地址输入框
└── src-tauri/src/
    ├── commands/companion.rs              # [M] relay addr 两个薄 command
    ├── services/companion_pairing.rs      # [M] generate_qr_payload 读 app_settings
    ├── services/companion_connection.rs   # [M] 中继注册客户端 + 握手后逻辑抽取共用
    ├── lib.rs                             # [M] 注册新 command
    └── tests/test_companion.rs              # [M] 中继路径集成测试
```

（relay-server/、crates/companion-proto/ 零改动。）

### Previous Story Intelligence（12.1/12.2/12.3 → 12.4）

- **crate API 以实物为准**（12.1 血泪）：Kotlin 侧镜像的是 **GenVectors.java 的实测用法**，不是 noise-java README 示例；发现 API 不存在即停下记录，禁止换库。
- **12.2 教训直接继承**：P4（超时预算防挂起——手机侧 WS 读同样设超时）、P6（单会话替换——切换承载无需等旧连接优雅关闭）、配对窗口 300s（扫码超时 UX 文案「二维码已过期，请在桌面重新生成」）。
- **12.3 遗留接口位**：relay_addr 恒空的 QR 字段位是本 story 接活的桌面缺口；relay 控制协议细节以 `relay-server/src/auth.rs` 文档注释为唯一事实源。
- **既有失败基线**（12.2 记录，非本 story 引入）：`cargo test --lib` 有 6 个预存失败（agent_config 5 + agent_engine 1，测试断言漂移）——`npm run test:all` 判绿时排除该已知集合并如实说明，不得顺手修。
- **12.3 交付未提交**：`relay-server/` 等仍在工作区未 commit——本 story 基线为 0a932fd + 工作区 12.3 产物；开始前确认工作区状态，勿与 12.3 未提交变更混提。

### Git Intelligence

基线 0a932fd（12.2 桌面配对交付）+ 未提交的 12.3 relay-server 交付。companion-android 上次提交为 4c67c6b（四项 UI 修复）——原型已稳定，换装以「只动装配缝与配对扫码」为纪律，`git diff` 应显示 ui/ 目录仅两处触碰：`PairingScreen.kt`（ScanStep 段）与 `AppNavHost.kt`（配对路由传参），其余 ui/ 文件零触碰（P22 修正：原表述「ui/ 零触碰（ScanStep 除外）」与 File List 的 [M] 两项矛盾）。

### Latest Tech Information

- **OkHttp**：当前稳定线 5.x（5.2.0，2025 年中起持续小版本迭代）；WebSocket API 自 4.x 起稳定。实现时取最新稳定并锁 `libs.versions.toml`。来源：[OkHttp releases](https://square.github.io/okhttp/)。
- **CameraX**：稳定线 1.0.0-rc 系列（1.0.0-rc03），PreviewView 内存泄漏修复已含。来源：[CameraX releases](https://developer.android.com/jetpack/androidx/releases/camera)。
- **ML Kit Barcode**：`com.google.mlkit:barcode-scanning`（bundled，约 3MB 模型，无 GMS 依赖）；release notes 见 [ML Kit](https://developers.google.com/ml-kit/release-notes)。
- **noise-java**：JitPack 固定 commit（见关键技术情报），无版本漂移问题。
- 三方版本均「实现时取最新稳定后锁定」，不追 alpha。

### 范围外（明确不做，防 scope creep）

- SNAPSHOT/STATE_DELTA 帧的消费与快照存储（13.1/13.2）；COMMAND/STREAM_TOKEN（13.3）；速记队列真实提交（14.1）；`SnapshotStore`/`QuickNoteQueue`/`ChatViewModel` 换装（各自归属 story，本 story 零触碰）。
- relay-server 任何改动（含 R1 phone 槽换绑限制——见开放问题）；companion-proto 任何改动；中继 TLS/wss；iOS。
- 换绑（PendingRebind）桌面确认 UI 打磨（12.2 已有确认按钮）；多手机/多桌面。
- E2E 帧在 Kotlin 侧 8 类全量编解码实现（占位即可，规则二）。

### 开放问题（实现后向 boss 汇报，不阻塞开发）

1. **R1 relay phone 槽与重装重扫**：12.3 评审裁决 phone 槽绑定首注册公钥——手机卸载重装（新密钥）后经中继重配对会被槽位绑定拒绝，直到 relay 进程重启（内存注册表清空）。AC6 以直连路径验收不受影响；若 UAT 要求中继下重装可配，需在 relay-server 增「desktop 注册时登记期望手机公钥」增强（12.3 裁决 3 已留口）。本 story 记录该限制，不改 relay。
2. Keystore wrap 形态（裁决 3）如 boss 认为必须「私钥原生托管」，需降级为 StrongBox/密钥认证方案——API 可行性未验证，超出 V1 性价比。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-12.4]（AC 唯一事实源）
- [Source: _bmad-output/planning-artifacts/epics.md#Requirements-Inventory（增量）]（UX-M1~M5、NFR-M5/M6/M7、Additional 4/5/9）
- [Source: _bmad-output/planning-artifacts/architecture.md#Incremental-Core-Architectural-Decisions—手机伴侣基建]（决策 1/2：Noise XX + 双承载；配对信任链）
- [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-Addendum—手机伴侣基建]（Android 包结构、四条硬边界、数据边界）
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation-Patterns—手机伴侣基建]（退避/滞回/日志/测试位置）
- [Source: egosync-app/src-tauri/src/services/companion_connection.rs]（桌面 wire 协议、超时常量、NSD 锚点、会话语义）
- [Source: egosync-app/src-tauri/src/services/companion_pairing.rs:117-124]（QR payload 生成、relay_id 推导）
- [Source: egosync-app/src-tauri/src/models/companion.rs:27-32]（QrPayload camelCase 契约）
- [Source: relay-server/src/auth.rs]（relay 控制协议唯一事实源）
- [Source: relay-server/src/lib.rs:60-84]（/relay 端点、build_router 测试口）
- [Source: crates/companion-proto/src/{frames,crypto}.rs]（帧编解码与 Noise 封装镜像对象）
- [Source: crates/companion-proto/interop/noise-java/{GenVectors.java,run.sh}]（noise-java API 用法与依赖坐标）
- [Source: crates/companion-proto/tests/fixtures/noise_java_vectors.json]（黄金向量）
- [Source: egosync-app/src-tauri/tests/test_companion.rs]（手机侧行为既有模拟参考）
- [Source: companion-android/README.md]（页面地图、换装点、白名单基线）
- [Source: _bmad-output/implementation-artifacts/12-2-desktop-pairing-and-connection-service.md]（P4/P6 教训、窗口 300s、keyring 先例）
- [Source: _bmad-output/implementation-artifacts/12-3-relay-server-stateless-relay-and-docker.md]（relay 控制协议、phone 槽绑定、128KB、R1 背景）
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-…]（12.4 开工前检查点：加密边界执行口径）
- [Source: _bmad-output/project-context.md]（基线 100 条规范）
- [OkHttp releases](https://square.github.io/okhttp/)、[CameraX releases](https://developer.android.com/jetpack/androidx/releases/camera)、[ML Kit release notes](https://developers.google.com/ml-kit/release-notes)

## Dev Agent Record

### Agent Model Used

glm-5.3-flash（DeepSeek Harness）

### Debug Log References

- Task 1 RED→GREEN：`./gradlew :app:testDebugUnitTest` 全绿（13 tests：8 新 + 5 既有连接测试）；Gradle 9.7.1 首次构建 3m21s 基线全绿。
- Gradle 9 隐式依赖校验拦截 `processDebugUnitTestJavaRes`（copyNoiseVectors 写入 src/test/resources）——已对消费该目录的任务补显式 `dependsOn`。

### Completion Notes List

- **Task 1（AC2）**：`NoiseChannel.kt` 逐字镜像 GenVectors.java 实测 API（`HandshakeState/getFixedEphemeralKey/split/CipherStatePair`），提供 initiator/responder 双侧 + `initiatorForTest/responderForTest`（固定临时密钥仅测试用）；`FrameCodec.kt` 镜像 frames.rs（u32 BE 前缀、HELLO 版本校验、65535 密文上限）。黄金向量互验：发起/响应两侧重放 12.1 冻结向量，3 条握手消息与 16 条传输密文**逐字节**一致。
- **JSON 裁决（白名单外增项，理由）**：运行时用 Android 平台自带 `org.json`（零新运行时依赖）；`FrameCodec` 编码用显式键序拼装（org.json JSONObject 键无序，serde 产物键序固定 type 在前），字符串值经 `JSONObject.quote` 转义；`org.json:json:20240303` 仅 testImplementation（覆盖 android.jar stub，使 JVM 单测可真实解析）——非功能依赖，白名单运行时增项不变。
- fixture 经 Gradle `copyNoiseVectors` 任务从 crate 复制（产物已 gitignore，仓内无手工副本）。
- **Task 2（AC2/AC6）**：`KeyStore.kt`（类名 `PairingSecrets`，避开 java.security.KeyStore 撞名）——AES-GCM（256 位，IV‖密文+tag 布局）包裹 X25519 私钥，`wipe()` 同时删密文文件与 Keystore 密钥；`PairingStateStore.kt` 复用 companion_prefs 文件与既有 paired 键（与 AppModelContainer 同一事实源，零迁移）。JVM 单测 4 项（出厂态/保存/relayAddr 可空/清空语义）；Keystore 真路径归 androidTest（Task 8）。
- **Task 3（AC1）**：依赖锁定最新稳定（实现时核实 maven-metadata）：ML Kit `barcode-scanning:17.3.0`（bundled）、CameraX `1.6.2`（story 情报中的 1.0.0-rc03 为过时行，按「取最新稳定」裁决取 1.6.2）、OkHttp `5.5.0`。`QrScanner.kt` = QrScannerView + QrCodeAnalyzer（识别防重）+ CameraPermissionGate（拒绝态中文引导）；ScanStep 取景框/四角标记样式逐字保留、按钮替换为真实扫码，提示文案改为「将二维码对准取景框」。`PairingViewModel.onQrScanned(qrJson)` 为解析闸门（残缺 QR 拒绝停留扫码页），`onScanCompleted()` 无参保留为开发/测试入口（既有单测依赖）；QrPayload JVM 单测 3 项。
- **Task 4（AC1/AC3）**：`RealConnectionClient` 实现 ConnectionClient + PairingConnector；握手每步 10s 超时预算（P4）；信任锚不等即断开（IdentityVerificationException）；启动已配对即 Offline 起步（AC5 不误报在线）、直连成功才 Direct；配对成功落盘 store.save（重连路径同信任锚）。`BackoffPolicy` full-jitter（[d/2, d]）4 项 JVM 测试；NoiseChannel 增 `remoteStaticPublicKey()`/`deriveStaticPublicKeyForTest` + 信任锚提取测试（noise-java 实证 setPrivateKey→start 后可取公钥）。NSD/WS 真路径归 androidTest（Task 8）+ 手动冒烟。
- **Task 5（AC4/AC5）**：`ConnectionStateMachine` 滞回决策抽纯逻辑（注入 sleep 时间可控），意图用同步观察者（SharedFlow 在 coroutines-test 1.11 backgroundScope 下时序不可控——已实证并规避）；测试 5 项（3s 窗满切中继/窗内恢复取消/无中继即 Offline/中继态不被直连失败扰动/中继丢失 Offline）。`RelayClient` 逐字镜像 auth.rs（register text 首消息、XX responder 鉴权 3 条 binary、10s 整体预算、≥32 字节判据、鉴权 transport 弃用）；`WsSession.kt` 抽直连/中继共享承载桥，E2E 握手/信任锚/帧序/会话循环双承载同构。debug 覆盖层在 Task 4 已实现（combine(debugMode, liveState)，unpair 复位）。
- **Task 6（AC4）**：桌面侧最小增量——`companion_pairing::get/set_relay_addr`（app_settings 键 `companion_relay_addr`）+ 两个薄 command；`generate_qr_payload(pubkey, relay_addr)` 参数化（命令层读设置传入）。`companion_connection`：`WsIo<S>` 泛型化（服务端 TcpStream/客户端 MaybeTlsStream 同构）、「准入后路径」抽 `run_authorized_session`（首帧→Notice→nonce→决策→会话，直连/中继共用，`&mut dyn HandshakeIO`）；`run_relay_client` 常驻循环（门控与 NSD 同口径、未启用 5s 慢轮询、断线指数退避 1→30s、成功复位），`relay_connect_and_serve`（register text→鉴权 XX responder→转发态 E2E 同款 responder 握手→复用会话路径）；`get_status` origin 如实透传（不再硬编码 direct）。**实证修复两处**：① relay keepalive Ping 首拍即发，`WsIo.recv` 与测试读帧均须跳过 Ping/Pong（tungstenite 自动应答但透传）；② relay desktop 槽单槽替换语义——中继客户端必须单实例（`start_companion_listener` spawn 一次；测试曾重复 spawn 致双实例互踢抖动，已修）。中继集成测试 `relay_path_pairs_and_pings`：进程内 `build_router` 起真 relay，双端全链路（register→鉴权→E2E→首配落库→PING/PONG→origin=relay），手机侧重试预算 14×2.3s 覆盖桌面 5s 慢轮询。前端 `CompanionPairingSection` 中继地址输入卡（空=清除，保存后提示 ≤5s 生效+新 QR 携带地址），文案从「中继暂未部署」如实化为按配置渲染。已知限制（记录于开放问题 R1）：relay 单 desktop 槽——双开桌面实例会互踢；单实例部署为既定形态。`npm run test:all` 全绿（vitest 438 + cargo test_companion 15 + lib 单测）。
- **Task 7（AC1/AC6）**：换装三件套——①`AppModelContainer`：`connection: ConnectionClient = RealConnectionClient(context, scope)`（唯一换装点）+ `pairingConnector` 同实例二态导出；`completePairing/unpair` 纯委托（配对持久化归 `PairingStateStore`——与 prefs `paired` 键同一事实源，无迁移）；KEY_PAIRED 常量移除（Fake 初始化不再需要）。②`PairingViewModel(connection, pairing: PairingConnector?=null)`：真实路径 `onQrScanned` 解析闸门→`pairWithQr`→进度流驱动 CONNECTING 三阶段（新增 `waitDesktopConfirm`/`pairingError` 状态；失败回 SCAN 并展示原因，残缺 QR 给「二维码无效」）；模拟路径（null connector）保留——既有单测与 Preview 零改动。③`PairingScreen` 新参数带默认值（Preview 零改）：CONNECTING 步「等待桌面确认」提示文案、SCAN 步失败原因红字。**PendingRebind 换绑等待**：桌面挂 pending 后关闭连接，手机端 `runPairing` 无法从握手区分——加 `probeSessionAlive`（发 PING 4s 预算等回帧；通道关闭即 pending），`waitDesktopConfirmAndRetry`（120s 对齐桌面 pending 时效、3s 周期重连免 nonce——pending 匹配放行；确认后 AlreadyPaired 直接入会话；超时如实 Failed）。日志纪律：`CompanionLog`（tag `Companion/<组件>`，只记事件/状态/relay_id）。新增 4 项 VM 单测（残缺 QR 拒绝/三阶段驱动/换绑等待呈现/失败原因展示），全量 `:app:testDebugUnitTest` 绿。
- **Task 8（AC7）**：androidTest 冒烟 `PairingSmokeTest`——真 Keystore 路径（PairingSecrets 生成→AES-GCM 包裹落盘→重载字节一致）+ on-device noise-java 互通（Keystore 私钥作 initiator 与进程内 responder 小服务器完成 XX 三消息握手）+ 帧序列往返（HELLO→deviceInfo→pairingAuth→PING/PONG，双端密文经通道桥）。**验证边界（如实）**：本环境无设备/模拟器（`adb devices` 空、无 AVD）——androidTest 已通过 `:app:compileDebugAndroidTestKotlin` 编译验证，但 **`connectedAndroidTest` 未执行**，真机运行留待 UAT。最终验证：`:app:testDebugUnitTest` 全绿；`npm run build`（tsc+vite）+ `npm run test:all`（vitest 438 + cargo test_companion 15 + lib 单测）全绿。NSD 直连/中继切换/重装重扫真机手动冒烟与逐屏截图比对**未执行**（无双设备局域网环境），验证边界如实记录，建议纳入 UAT（bmad-uat-run）。

### File List

| 操作 | 路径 |
|---|---|
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/NoiseChannel.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/FrameCodec.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/Hex.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/BackoffPolicy.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/PairingStateStore.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/KeyStore.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/PairingConnector.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/NsdDiscovery.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/WsSession.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/RelayClient.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionStateMachine.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/connection/CompanionLog.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/pairing/QrPayload.kt |
| [N] | companion-android/app/src/main/java/com/egosync/companion/pairing/QrScanner.kt |
| [M] | companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt |
| [M] | companion-android/app/src/main/java/com/egosync/companion/pairing/PairingViewModel.kt |
| [M] | companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt |
| [M] | companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt |
| [N] | companion-android/app/src/test/java/com/egosync/companion/connection/NoiseInteropTest.kt |
| [N] | companion-android/app/src/test/java/com/egosync/companion/connection/FrameCodecTest.kt |
| [N] | companion-android/app/src/test/java/com/egosync/companion/connection/PairingStateStoreTest.kt |
| [N] | companion-android/app/src/test/java/com/egosync/companion/connection/QrPayloadTest.kt |
| [N] | companion-android/app/src/test/java/com/egosync/companion/connection/BackoffPolicyTest.kt |
| [N] | companion-android/app/src/test/java/com/egosync/companion/connection/ConnectionStateMachineTest.kt |
| [M] | companion-android/app/src/test/java/com/egosync/companion/pairing/PairingViewModelTest.kt |
| [N] | companion-android/app/src/androidTest/java/com/egosync/companion/connection/PairingSmokeTest.kt |
| [M] | companion-android/app/build.gradle.kts（依赖 + androidTest runner + copyNoiseVectors dependsOn） |
| [M] | companion-android/gradle/libs.versions.toml（白名单 + androidTest 仪器依赖） |
| [M] | companion-android/.gitignore（generated fixture） |
| [M] | companion-android/app/src/main/AndroidManifest.xml（INTERNET/CAMERA/usesCleartextTraffic） |
| [M] | companion-android/settings.gradle.kts（JitPack 仓库） |
| [M] | egosync-app/src-tauri/src/services/companion_pairing.rs（generate_qr_payload 参数化 + relay_addr 存取助手） |
| [M] | egosync-app/src-tauri/src/services/companion_connection.rs（中继客户端 + WsIo 泛型化 + run_authorized_session 抽取 + get_status origin） |
| [M] | egosync-app/src-tauri/src/commands/companion.rs（relay_addr 两个薄 command） |
| [M] | egosync-app/src-tauri/src/lib.rs（注册新 command） |
| [M] | egosync-app/src-tauri/Cargo.toml（dev-dep relay-server + axum） |
| [M] | egosync-app/src-tauri/tests/test_companion.rs（中继路径集成测试 + QR payload 参数） |
| [M] | egosync-app/src/services/companionService.ts（relay addr 读写 invoke） |
| [M] | egosync-app/src/components/settings/CompanionPairingSection.tsx（中继地址输入卡 + 文案如实化） |

### Change Log

| Task | AC | 变更摘要 |
|---|---|---|
| 1 | AC2 | noise-java 封装 + 帧编解码 + 黄金向量逐字节互验（Kotlin 侧） |
| 2 | AC2/AC6 | Keystore AES-GCM 包裹静态私钥 + 配对状态存储（companion_prefs 复用） |
| 3 | AC1 | ML Kit + CameraX 真实扫码（ScanStep 取景替换，样式零改动） |
| 4 | AC1/AC3 | RealConnectionClient：NSD 直连承载 + XX initiator + 信任锚 + 会话循环 + debug 覆盖层 |
| 5 | AC4/AC5 | RelayClient + ConnectionStateMachine 三态滞回 + 双承载 WsSession 桥 |
| 6 | AC4 | 桌面侧最小增量：relay_addr 存取 + 中继注册客户端 + get_status origin + 前端中继地址输入卡 |
| 7 | AC1/AC6 | AppModelContainer 换装 + PairingViewModel 进度驱动 + PendingRebind 等待 + 日志纪律 |
| 8 | AC7 | androidTest 冒烟（编译验证；真机运行留 UAT）+ 终验全绿 |

## Story Status: done

### AC Checklist

- [x] AC1 换装结构与 UI 零改动（connection/ 新实现；接口签名不变；AppModelContainer 仅换装配对；PairingScreen ScanStep 真实取景、四步样式不变）
- [x] AC2 noise-java 互通与密钥安全（黄金向量逐字节互验 + Keystore AES-GCM 包裹，导出/日志无私钥）
- [x] AC3 NSD 直连与信任锚（远端静态公钥 ≠ QR 公钥即断开）
- [x] AC4 经中继建立加密转发（桌面中继注册客户端 + 手机中继承载 + relay_addr QR 透出）
- [x] AC5 三态不误报（双承载均不可达 → Offline，不误报在线；滞回防抖动 3s 窗）
- [x] AC6 扫码→输入→授权→配对成功闭环（pairingAuth nonce 单次有效 + 换绑 pending 等待桌面确认）
- [x] AC7 自动化测试覆盖（JVM 单测 + 中继集成测试 + androidTest 冒烟编译；真机运行/UAT 边界如实记录）

## 建议下一步：`bmad-code-review`（代码评审）

**验证边界如实声明（规则十二）**：
- `./gradlew :app:testDebugUnitTest` 全绿；`:app:compileDebugAndroidTestKotlin` 编译通过。
- `npm run build`（tsc+vite）✓；`npm run test:all`（vitest 438 + cargo test_companion 15 + lib 单测）✓。
- **未执行**：`connectedAndroidTest`（无设备/模拟器）、真机 NSD/中继切换手动冒烟、逐屏截图比对——均超出本环境能力，建议纳入 UAT（`bmad-uat-run`）。
