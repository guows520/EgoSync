---
baseline_commit: d5345a6309fe0b06814d92dbd7216a92991f616e
---

# Story 12.2: 桌面配对与连接服务——QR 生成、Noise XX 握手、NSD 广播与 WS 监听

Status: done

## Story

As a 桌面用户,
I want 在桌面设置页生成配对二维码，完成与手机的绑定，并在局域网内自动广播桌面引擎供手机直连,
So that 我的手机无需手动输入任何网络配置就能安全连上这台唯一事实源。

## Acceptance Criteria（AC）

> 完整 AC 以 `_bmad-output/planning-artifacts/epics.md` Story 12.2 段为唯一事实源，以下为逐条搬运，编号供任务引用。

1. **AC1 增量模块结构**：存在 `services/companion_pairing.rs`（QR 生成、Noise XX 握手编排、paired_devices 读写）、`services/companion_connection.rs`（NSD 广播、WS 监听、连接状态机）、`commands/companion.rs`（`pairing_generate_qr / pairing_confirm / paired_device_list / paired_device_remove / companion_get_status`，薄层无业务逻辑）、`db/paired_devices.rs`、`models/companion.rs`、`migrations/031_paired_devices.sql`；`paired_devices` 表含 `id, device_name, device_pubkey, paired_at, last_seen_at`，并被纳入 data_export 导入导出与销毁清单；现有 services 文件零改动（桌面零回归；伴侣感知只存在于 companion_* 新模块内）。
2. **AC2 QR 生成与密钥管理**：用户在桌面设置页打开配对入口调用 `pairing_generate_qr`，桌面静态密钥经系统 keyring 读写（与 LLM API Key 同级管理，不落明文文件），QR payload 含 `{relay_addr, desktop_static_pubkey, relay_id(=pubkey 哈希), pairing_nonce}`；前端以 `services/` 封装 invoke、经 hook 调用，组件放域目录，遵循桌面既有前端规范。
3. **AC3 握手完成与错误路径**：手机侧发起 Noise XX 握手（Story 12.4 之前以协议 crate 测试客户端模拟），`companion_pairing` 完成握手后双向身份认证通过即写入 `paired_devices`，并发出 Tauri Event `companion:paired`；错误路径返回 `AppError` 新增变体 `PairingError / ConnectionError / ProtocolError`（序列化形状与既有 `AppError` 约定一致，用户可见文案为中文、不暴露技术细节）。
4. **AC4 免配对重连与状态查询**：已配对设备再次连接时 `companion_connection` 识别其静态公钥，免配对直接进入加密会话，状态机进入直连态，发出 `companion:connected` / 断开时 `companion:disconnected`；`companion_get_status` 可随时返回当前连接状态与最后配对信息。
5. **AC5 NSD 广播与移除拒绝**：桌面应用启动且配对过至少一台设备时，NSD/mDNS 以 `_egosync._tcp` 持续广播，WS 监听端口策略（固定端口或动态+注册约定）在 README/设置页中文说明，Windows 防火墙首次弹窗的用户引导文案已备妥；手机移除配对（`paired_device_remove`）后该设备公钥不再被接受。
6. **AC6 测试与零回归**：`cargo test`（含 `tests/test_companion.rs`）覆盖 QR payload 字段、握手成功/失败路径、paired_devices CRUD、免配对重连、事件发射与状态查询；`npm run test:all` 全绿（桌面零回归）。

## Tasks / Subtasks

- [x] Task 1：`migrations/031_paired_devices.sql` + `db/paired_devices.rs` + `models/companion.rs`（AC1）
  - [x] 迁移：`paired_devices` 表 `id TEXT PK NOT NULL`、`device_name TEXT NOT NULL`、`device_pubkey TEXT NOT NULL UNIQUE`、`paired_at TEXT NOT NULL`、`last_seen_at TEXT NOT NULL`；索引 `idx_paired_devices_device_pubkey`（命名规范 `idx_{table}_{column}`）
  - [x] `db/paired_devices.rs`：`upsert_single_device / get_all / get_by_pubkey / update_last_seen / remove / remove_all`；单对单语义在 db 层 upsert 时落地（新记录写入前清空旧记录——NFR-M6）
  - [x] `models/companion.rs`：`PairedDevice`（serde `rename_all = "camelCase"`）+ `QrPayload`（`relayAddr / desktopStaticPubkey / relayId / pairingNonce`）+ `CompanionStatus`；注册进 `models/mod.rs`
  - [x] `db/paired_devices.rs` 同文件 `#[cfg(test)]`：CRUD + 单对单替换语义测试
- [x] Task 2：`error.rs` 新增三个变体（AC3）
  - [x] `PairingError / ConnectionError / ProtocolError`，`#[error("...")]` 中文用户可见文案；`Serialize` map 追加三个 match 臂；`test_all_variants_serialize` 用例同步追加（该测试枚举全部变体，属既有 UPDATE 文件的联动修改）
- [x] Task 3：`services/companion_pairing.rs` 核心逻辑（AC2、AC3）
  - [x] 静态密钥管理：keyring key `companion_static_keypair`（复用 `secret_store::save/load_secret`；首次调用 `companion_proto::crypto::generate_static_keypair()` 生成，hex 存 keyring，内存缓存）
  - [x] `generate_qr()`：产出 `QrPayload`（pubkey hex、relay_id = pubkey SHA-256 hex 前 16 字符、pairing_nonce UUID v4、relay_addr V1 恒为 None/空——relay-server 属 12.3，本 story QR 字段位保留但值空，前端如实展示"中继未部署"）
  - [x] 握手编排：responder 侧 `HandshakeSession::responder(desktop_priv)` 三消息交换；transcript 中提取对端静态公钥（snow XX 模式下对端 s 消息由 crate `TransportSession` 握手 API 暴露——若 crate 未暴露 remote static key，在 companion_pairing.rs 内以**新增最小 pub fn** 方式经 crate `crypto` 模块透传获取，禁止在 crate 外 use snow）；握手后要求首帧 `HELLO`（protocolVersion 校验已由 `decode_frame` 内建）
  - [x] 设备名约定：握手后 app 层以 `NoticePayload.data` 携带 `{"type":"deviceInfo","deviceName":"..."}`（协议帧不 bump，app 层契约写文档注释）；缺失时 fallback `device_name = "手机伴侣"`
  - [x] 配对决策：对端公钥 == 已配对记录 → 直接进入会话（AC4 前置）；无已配对记录（首配）→ 直接写入并发 `companion:paired`（原型四步流无桌面确认环节）；已有记录且公钥不同（换绑）→ 内存单槽 pending（含对端公钥/设备名/时间戳），等待 `pairing_confirm`，超时（120s）作废
  - [x] `confirm_pending()`：pending 存在时写入 paired_devices（替换旧记录）+ 发 `companion:paired`；无 pending 返回 `AppError::ValidationError`（中文文案）
  - [x] 纯核心函数与 Tauri 胶水分离：握手验证/决策/写库均为无 `AppHandle` 的纯 async fn，事件发射经注入的 `Option<AppHandle>` 回调——保证 `tests/test_companion.rs` 可脱 UI 测试
- [x] Task 4：`services/companion_connection.rs`（AC1、AC4、AC5）
  - [x] WS 监听：`tokio-tungstenite` 直接 accept（`TcpListener::bind(("0.0.0.0", 0))` 动态端口——**注意与 delegate_bridge 的 127.0.0.1 先例刻意不同**：手机需从局域网访问，故必须对外可达，防火墙弹窗文案见 Task 7）；每连接一套 `HandshakeSession::responder` → `TransportSession`
  - [x] 帧循环：握手期 Noise 消息以 WS binary 消息为载勺（每条 WS 消息一条握手消息，无长度前缀）；transport 期经 `frames::encode_frame/decode_frame`（crate 内建长度前缀）；只处理 `HELLO / PING / Notice(deviceInfo)`，其余帧类型收到即忽略并 warn（13.x 才消费）
  - [x] NSD 广播：`mdns-sd` crate（0.17.x，实现时取最新稳定），`ServiceDaemon::new()` + `ServiceInfo`（service type `_egosync._tcp.local.`、instance 名 = `EgoSync-` + relay_id 前 8 字符、port = WS 实际端口、TXT 记录 `proto=<PROTOCOL_VERSION>`）；启动时已配对设备 ≥1 才注册，首次配对成功后动态注册；`paired_device_remove` 清空且无 pending 时注销广播
  - [x] 连接状态机：`CompanionConnectionState { Listening, Connected { device, since, peer_addr }, }`（`Arc<RwLock<...>>` managed state）；connected→disconnected 转换发 `companion:disconnected` 并回 Listening；对端来源判定 direct（LAN 网段 IP）/ relay（本 story 无中继，恒 direct）仅作状态标注
  - [x] `companion_get_status` 数据源：状态机快照 + 最后配对信息（paired_devices 最新记录）
  - [x] 移除后拒绝：连接 accept 后查 `paired_devices`（+pending），公钥既非已配对也非 pending/pairing 窗口内新公钥 → 握手后即关闭连接（错误发 `ProtocolError` 日志，不暴露细节）
- [x] Task 5：`commands/companion.rs` + 注册（AC1）
  - [x] 五个薄命令：`pairing_generate_qr / pairing_confirm / paired_device_list / paired_device_remove / companion_get_status`——参数解析 → service 调用 → 返回，零业务逻辑
  - [x] `lib.rs` `invoke_handler` 注册五命令 + `app.manage(CompanionState)` + setup 中 spawn companion_connection 监听（非阻塞、失败仅 warn 降级——沿用 sidecar/scheduler 降级先例）
- [x] Task 6：data_export 集成（AC1——"现有 services 零改动"的显式例外，见 Dev Notes 冲突裁决）
  - [x] `ExportData` 增 `#[serde(default)] pub paired_devices: Vec<serde_json::Value>`（紧随 butler_mcp_servers 先例：旧档导入兼容）
  - [x] `gather_export_data` 增 `query_paired_devices`；`MAIN_DB_TABLES` 增 `"paired_devices"`（销毁覆盖）
  - [x] `import_json_data` 增 paired_devices 恢复路径 + 旧档（无该字段）兼容测试；markdown 报告**不含**配对设备（含公钥技术数据，用户报告无消费价值——裁决记录于 Dev Notes）
- [x] Task 7：前端设置页配对入口（AC2）
  - [x] `types/companion.ts`：`PairedDevice / QrPayload / CompanionStatus`（camelCase，镜像 models）
  - [x] `services/companionService.ts`：五个 invoke 封装 + `QRCode` 渲染（npm 依赖 `qrcode`——前端生成二维码图，后端只出 payload；理由：桌面 Rust 端零图像依赖、前端 SVG 渲染即够）
  - [x] `components/settings/` 域内新增配对组件（如 `CompanionPairingSection.tsx` + `GlobalSettingsModal.tsx` 增 'companion' tab「手机伴侣」）；QR 展示、已配对设备卡（设备名/公钥前缀/最后在线）、移除按钮、换绑 pending 时显示「新设备请求替换」+ 确认按钮（调 `pairing_confirm`）、端口策略与防火墙中文说明文案（AC5）
  - [x] `useTauriEvent` 监听 `companion:paired / companion:connected / companion:disconnected` 刷新；modal 打开期间轮询 `companion_get_status`（换绑 pending 可见性）
  - [x] README.md 增「手机伴侣连接」小节：NSD 服务名、动态端口策略说明、Windows 防火墙首次弹窗引导（AC5）
- [x] Task 8：`tests/test_companion.rs`（AC6）
  - [x] QR payload 四字段断言（relay_addr 空、pubkey/relay_id/nonce 非空 + relay_id == pubkey 哈希前缀）
  - [x] 握手成功路径：随机密钥 initiator 模拟手机（`initiator_with_fixed_keys_for_testing` 钩子保留未用）→ 连本地 WS → 断言 paired_devices 落库 + 状态查询 + （事件发射以纯函数断言，不依赖 AppHandle）
  - [x] 握手失败路径：错误密钥/错误协议消息 → `ProtocolError`，不落库
  - [x] 首配直接写入；换绑 pending → confirm 后替换旧记录（单对单）；pending 超时作废
  - [x] 免配对重连：同公钥二次握手直接进入会话，不断言重复配对事件
  - [x] `paired_device_remove` 后同公钥连接被拒
  - [x] data_export：导出 JSON 含 pairedDevices 字段；旧档（无字段）导入成功
- [x] Task 9：验证收尾
  - [x] `cd egosync-app && npm run test:all` 全绿（vitest + cargo test）
  - [x] `npm run build` tsc 零类型错误
  - [x] 桌面零回归自查：既有 services 文件 diff 仅 data_export.rs（裁决允许的最小增量）+ error.rs + lib.rs 注册 + agent_config.rs/agent_engine.rs 断言修复（Change Log 2026-08-28 追加项，经 boss 裁决的预存断言漂移修复，基线 d5345a6 生产代码已含新字符串、取证见 CR）
  - [x] 日志纪律自查：companion_* 模块 tracing 只记事件元数据（帧类型/长度/对端标识/状态转换），帧明文与密钥材料零输出（NFR-M7）（评审发现 P5 兜底帧 `?other` Debug 泄露后已改为仅记帧类型判别式）

## Dev Notes

### 架构硬边界（违反即返工）

1. **加密边界**：src-tauri 经 path 依赖 `companion-proto`（`Cargo.toml` 增 `companion-proto = { path = "../../crates/companion-proto" }`）；**全仓 use snow 仅允许存在于 crate crypto.rs**——桌面代码只调 `HandshakeSession / TransportSession / encode_frame / decode_frame / generate_static_keypair`。若需对端静态公钥而 crate 未暴露，在 crate `crypto.rs` 内新增最小透传 fn（不涉帧 schema，无需 bump protocolVersion），并在 Dev Agent Record 说明。
2. **桌面边界**：`companion_pairing / companion_connection` 只读写 `db::paired_devices` 与自身 managed state，不触任何其他既有 service；commands 薄层。
3. **仓库根禁止 Cargo workspace**（12.1 已确立）；src-tauri path 依赖不构成 workspace。
4. **帧协议冻结**：8 帧类型与 payload 不动；设备名走 app 层 Notice data 约定（见 Task 3）。

### 冲突裁决（规则七，显式择一）

| # | 冲突 | 裁决 | 理由 |
|---|------|------|------|
| 1 | AC1「现有 services 文件零改动」 vs 「paired_devices 纳入 data_export 导入导出与销毁清单」 | **后者显式覆盖**：`data_export.rs` 允许最小增量（ExportData 字段 + query fn + MAIN_DB_TABLES + import 路径）；「零改动」按「零行为回归」执行——既有导出/导入/销毁/测试全部原样通过 | 同一 AC 内显式要求 > 概括性禁令；butler_mcp_servers（Story 10.2）已开同类先例（`#[serde(default)]` + 兼容测试） |
| 2 | 架构 addendum「lib.rs [M] 仅注册新 commands」 vs NSD 广播/WS 监听需在应用启动时运行 | **以 AC 功能为准**：lib.rs setup 增 companion_connection spawn（非阻塞降级启动）+ manage state + 注册命令 | 无启动钩子则 AC5「应用启动即广播」不可能成立；spawn 模式沿用 scheduler/task_deadline_watch 先例 |
| 3 | `pairing_confirm` 命令语义（AC 列出但无行为定义；原型四步流无桌面确认环节） | **语义 = 单对单换绑闸门**：首配自动接受（扫码即绑定，符合 FR-40「无需手动输入」与原型流）；已配对且新公钥到来 → pending 单槽（120s 过期）+ `companion_get_status` 可见 + `pairing_confirm` 落库替换 | 防第三台设备静默顶替绑定（安全需最小可见同意）；复用已冻结命令集与事件集，零投机设计；12.4「重装重扫恢复、新配对按产品口径处理旧记录」由此承接 |
| 4 | QR 渲染位置：后端 Rust crate vs 前端 npm | **前端 npm `qrcode`**：后端只返回 payload 字符串/对象 | 桌面 Rust 零图像依赖；前端 SVG 即可；payload 事实源在后端不受影响 |
| 5 | WS 端口策略：固定 vs 动态 | **动态端口（bind 0.0.0.0:0）+ NSD 广播携带实际端口**（ServiceInfo 自带 port 字段） | 零端口冲突风险；手机经 NSD 自动获取无需手填；README/设置页中文说明（AC5）；注意与 delegate_bridge 127.0.0.1 先例刻意不同——本监听必须局域网可达 |

### 关键技术情报

- **companion-proto 现成 API**（crates/companion-proto/src/{crypto,frames}.rs，12.1 已交付）：`generate_static_keypair() -> (priv, pub)`；`HandshakeSession::{initiator, responder}(local_static_private)`、`write_message/read_message`（返回 Vec&lt;u8&gt，握手期无长度前缀）；`into_transport() -> TransportSession`、`encrypt/decrypt`；`frames::{encode_frame, decode_frame}`（内建 u32 BE 长度前缀 + protocolVersion 校验 + `deny_unknown_fields` + 帧尺寸上限）；测试钩子 `initiator_with_fixed_keys_for_testing` 黄金向量已双向字节级互验。**本 story 无需修改 crate（除裁决 1 所述可能的 remote-pubkey 透传）**。
- **snow API 陷阱**（12.1 实测）：`Builder::local_private_key` 返回 Result；`write_message`/`read_message` 返回 usize 且消息在 out 参数——均已被 crate 封装，桌面侧不再触碰。
- **mdns-sd**：0.17.x（[docs.rs/mdns-sd](https://docs.rs/mdns-sd/0.17.0/i686-pc-windows-msvc/src/mdns_sd/service_daemon.rs.html)）；`ServiceDaemon::new()` + `register(ServiceInfo::new(...))`；Windows 依赖系统 mDNS（Win10+ 内置）。**新增依赖白名单理由**：PRD 无需人工干预局域网发现的标准解（架构 Important Decision），无替代自研路径。
- **tokio-tungstenite**：WS 服务端 accept 直收 binary 帧（版本以实现时 cargo 最新稳定为准，落 Cargo.toml 后锁定）。**新增依赖白名单理由**：relay-server（12.3）同栈，架构指定 WS 承载；不引 axum（桌面单端点无路由需求，规则二）。
- **前端 `qrcode` npm 包**：生成 SVG 字符串渲染（[crates/crates.io: qrcode](https://crates.io/keywords/qrcode?sort=downloads) 同名 npm 包 `qrcode`，kennytm 为 Rust 端作者，npm 端为独立包 davidshimjs/qrcodejs 生态——**实现时以 npm 上 `qrcode` 包为准**，如 API 不合意可换 `qrcode.react`，接口封闭在 companionService 内部）。
- **keyring 复用**：`secret_store::{save,load}_secret`（SERVICE_NAME `com.egosync.app`）；key 名 `companion_static_keypair`；测试沿用 secret_store 的"keyring 不可用跳过"纪律——**本 story 的握手测试不依赖真实 keyring**（固定密钥测试钩子注入）。
- **Tauri 事件先例**：`app_handle.emit("event-name", payload)`（agent_engine.rs 大量使用）；前端 `useTauriEvent` hook 已存在。本 story 事件 payload 冻结为：
  - `companion:paired` → `{ deviceId: string, deviceName: string, pubkeyPrefix: string }`
  - `companion:connected` → `{ deviceId: string, deviceName: string }`
  - `companion:disconnected` → `{ deviceId: string, reason: string }`（reason 为中文短句）
- **迁移编号**：031 已确认（当前最新 030_llm_provider_extended.sql，sprint plan 已核对）。
- **事件命名**：`{domain}:{verb_past}` —— `companion:paired/connected/disconnected` 完全合规；**不新增**该三个之外的 UI 事件（换绑 pending 走 status 轮询，避免私扩事件）。

### data_export 最小增量清单（裁决 1 落地口径）

`src/services/data_export.rs` 四处：① `ExportData` 增 `#[serde(default)] paired_devices` 字段；② 新 `query_paired_devices`（SELECT 全表转 serde_json::Value）；③ `gather_export_data` 调用之；④ `MAIN_DB_TABLES` 增 `"paired_devices"` + `import_json_data` 增恢复分支（含旧档无字段兼容）。**不改** markdown 生成（配对设备不进用户报告）。既有测试 `destroy_all_data_clears_all_tables` 等会自动覆盖新表（表在销毁清单内）。

### 测试策略（规则九：验证意图）

- 意图锚点：**"配对一次，之后免配对"**（信任持久化 WHY）→ 同公钥重连测试断言无第二次 paired 事件、直接 Connected；**"移除即拒绝"**（用户主权 WHY）→ remove 后同公钥握手被拒断言；**"换绑需可见同意"**（单对单安全 WHY）→ pending+confirm+替换断言；**"旧档兼容"**（数据主权 WHY）→ 无字段 JSON 导入成功。
- 服务端真 socket 测试：`tests/test_companion.rs` 起 WS listener 于随机端口（loopback 即可测握手/帧逻辑；NSD 真实广播不做自动化断言——跨平台 mDNS 环境不可控，以 `ServiceInfo` 构造参数单测替代 + 手动冒烟记录）。
- 事件发射：纯函数产出事件 payload（`Vec<(String, Payload)>` 或回调注入），测试断言内容而非 Tauri emit 调用本身。

### 前端规范锚点

- 组件域目录：`components/settings/`（GlobalSettingsModal 既有 tab 结构：llm/mcp/scheduler/data → 新增 companion，[Source: components/settings/GlobalSettingsModal.tsx:570-577]）
- service 封装 invoke（组件不直接 invoke，[Source: services/*.ts 先例]）；types 镜像 Rust camelCase
- Loading 态命名 `isLoading/isSubmitting`；不可变更新；Tailwind utility only；深浅色双态（`dark:` 变体随既有样式）
- 原型文案对照：设置入口名「手机伴侣」（[Source: companion-android PairingScreen.kt:140]「设置 · 手机伴侣」）

### Project Structure Notes

```text
egosync-app/
├── src-tauri/
│   ├── Cargo.toml                       # [M] +companion-proto(path) +tokio-tungstenite +mdns-sd
│   ├── migrations/031_paired_devices.sql # [N]
│   ├── src/
│   │   ├── commands/companion.rs        # [N] 五命令薄层
│   │   ├── commands/mod.rs              # [M] +pub mod companion
│   │   ├── db/paired_devices.rs         # [N]
│   │   ├── db/mod.rs                    # [M] +pub mod paired_devices
│   │   ├── models/companion.rs          # [N]
│   │   ├── models/mod.rs                # [M] +pub mod companion
│   │   ├── services/companion_pairing.rs # [N]
│   │   ├── services/companion_connection.rs # [N]
│   │   ├── services/mod.rs              # [M] +两个 pub mod
│   │   ├── services/data_export.rs      # [M] 四处最小增量（裁决 1）
│   │   ├── error.rs                     # [M] 三变体 + serialize 臂 + 测试
│   │   └── lib.rs                       # [M] setup spawn + manage + 注册五命令（裁决 2）
│   └── tests/test_companion.rs          # [N]
├── src/
│   ├── types/companion.ts               # [N]
│   ├── services/companionService.ts     # [N] invoke 封装 + QR 渲染
│   ├── components/settings/CompanionPairingSection.tsx # [N]（命名可按实现微调）
│   └── components/settings/GlobalSettingsModal.tsx # [M] +companion tab
└── package.json                          # [M] +qrcode 依赖
README.md                                 # [M] 手机伴侣连接说明小节
```

对架构 addendum 目录树的偏差：架构写 lib.rs「仅注册新 commands」——裁决 2 放宽为含 setup spawn（AC5 要求）；其余完全一致。

### Previous Story Intelligence（12.1 → 12.2）

- crate 的黄金向量与测试钩子（`initiator_with_fixed_keys_for_testing`）直接复用为模拟手机客户端——**不要**在 src-tauri 里重新写 Noise 驱动代码。
- 12.1 教训继承：API 字面与实测有差（`Builder` 返回值、usize 返回）——一切以 crate 现有封装签名为准，不参考网上 snow 示例。
- 日志纪律：12.1 crate 内 tracing 调用数为 0；本 story 引入日志时只记元数据（帧类型/长度/对端标识/状态转换），**pubkey 明文可记（公钥非机密），私钥与帧 payload 明文绝不入日志**。
- 变更流程：不碰帧 schema → 不 bump PROTOCOL_VERSION、不动 schema.json、不重生成向量。若实现中发现必须动帧 → 停下升级为 schema 变更流程（bump + 三端同步），并在 Dev Agent Record 记录。

### Git Intelligence

基线 d5345a6（HEAD）：最近 8 次提交全部为 companion-android UI 收尾与 Epic 12~14 规划文档——src-tauri 侧无进行中分支、无未合并的 Rust 变更；桌面侧最后一个功能 Story 为 11-1（activity 统计）。工作区干净，可直接开工。

### Latest Tech Information

- mdns-sd 0.17.x（docs.rs 已核）；实现时 `cargo add mdns-sd` 取最新稳定并跑 `cargo check`。
- tokio-tungstenite / qrcode（npm）：实现时取最新稳定；qrcode 若 API 不合意换 `qrcode.react`，封闭在 companionService 内。
- Noise 套件 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 互通性已被 12.1 黄金向量双向字节级验证——本 story 无跨语言不确定性。

### 范围外（明确不做，防 scope creep）

- relay-server 与 relay_addr 真实值（12.3）；Android 真实客户端（12.4）；SNAPSHOT/STATE_DELTA/COMMAND/STREAM_TOKEN 帧的业务处理（13.x）；直连↔中继滞回切换（12.4 手机侧）；PING 之上的心跳策略调优（保持最小实现即可）；QR 过期刷新交互打磨（nonce 单次有效已保证安全）；不做第二套 WS/加密栈；不引入 axum。
- 不改 companion-proto 帧定义；不动 opencode/agent 相关任何模块。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-12.2]（AC 唯一事实源）
- [Source: _bmad-output/planning-artifacts/epics.md#Epic-依赖图（增量）]（12.1 → {12.2, 12.3} → 12.4 强顺序）
- [Source: _bmad-output/planning-artifacts/architecture.md#Incremental-Core-Architectural-Decisions—手机伴侣基建]（决策 1/2/4、配对与信任链、Data Architecture）
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation-Patterns-Addendum—手机伴侣基建]（命名/格式/通信/流程模式、反模式）
- [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-Addendum—手机伴侣基建]（目录树、四条硬边界）
- [Source: _bmad-output/planning-artifacts/prd-egosync.md#FR-40]（扫码绑定无桌面确认环节、重装重扫恢复）
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-...md]（12.2 测试范围、依赖表、风险表）
- [Source: _bmad-output/implementation-artifacts/12-1-companion-proto-crate-frame-schema-and-interop.md]（crate API、snow 陷阱、日志纪律、Review Findings 全记录）
- [Source: _bmad-output/project-context.md]（基线 100 条规范）
- [Source: crates/companion-proto/src/{lib,crypto,frames}.rs]（实际 API 签名）
- [Source: egosync-app/src-tauri/src/{error.rs,lib.rs,services/data_export.rs,services/secret_store.rs,db/pool.rs}]（UPDATE 文件现状）
- [docs.rs/mdns-sd 0.17.0](https://docs.rs/mdns-sd/0.17.0/i686-pc-windows-msvc/src/mdns_sd/service_daemon.rs.html)

## Dev Agent Record

### Agent Model Used

glm-5.3（DeepSeek Harness / bmad-dev-story 工作流，2026-08-28）

### Debug Log References

1. **tempdir 生命周期 → readonly DB**：集成测试 `setup_listener` 解构时丢弃 `TempDir` guard，目录被删导致 `attempt to write a readonly database`。修复：`TempDir` 贯穿测试生命周期随句柄返回。
2. **Noise XX 首消息无 MAC 的测试陷阱**：`-> e` 阶段无认证标签，64 字节全零会被 snow 当合法消息+payload 接受（测试表现为死锁：responder 等第三条消息）。握手失败测试改用「短于 32 字节临时公钥」的确定性非法输入。
3. **测试轮询锁竞争（误诊）**：socket 测试超时首判为 SQLite journal 锁竞争，放宽轮询后仍失败；eprintln 诊断定位到真实根因是上述 readonly DB（诊断日志已全部移除）。
4. **预存测试失败取证**：`git stash` 在干净基线 d5345a6 上复现全部 6 个失败（agent_config 5 + agent_engine 1），证明与本 story 无关。

### Completion Notes List

- **crate 增量（Dev Notes 授权范围内）**：`companion_proto::crypto::HandshakeSession` 新增 `remote_static_pubkey() -> Option<Vec<u8>>`（snow `get_remote_static` 透传）；帧 schema/PROTOCOL_VERSION 零改动、无 bump。
- **配对窗口设计决策**：story 规定「公钥既非已配对也非 pending/pairing 窗口内新公钥 → 拒绝」但未定窗口时长；取 `PAIRING_WINDOW_TIMEOUT_SECS = 300`（QR 展示→扫码的合理人因窗口），由 `pairing_generate_qr` 打开。「移除即拒绝」依赖窗口关闭——移除后旧公钥重连被拒，重新生成 QR 才能再配对（测试 `removed_device_pubkey_is_rejected` 显式覆盖）。
- **lib.rs 模块可见性**：`db/services/models/error/commands` 声明从 `mod` 改 `pub mod`，使 `tests/test_companion.rs` 可访问内部 API（rlib 仅被测试消费，无运行时影响）。
- **NSD 版本**：story 写 mdns-sd 0.17.x「实现时取最新稳定」，实际采用 0.21.0；ServiceInfo 参数单测断言 fullname 与 `proto` TXT（真实广播按 story 要求做手动冒烟、不做自动化断言）。
- **WS 绑定地址**：`0.0.0.0` + 动态端口（与 delegate_bridge 的 127.0.0.1 先例刻意不同）；README「手机伴侣连接」章节已含端口策略与 Windows 防火墙中文引导。
- **data_export**：`paired_devices` 以 `#[serde(default)]` 进 `ExportData`（旧档导入兼容，沿用 butler_mcp_servers 先例）；SQLite 导入对旧档（无该表）双站点跳过；markdown 用户报告不含配对设备（公钥技术数据无消费价值）。
- **⚠️ 预存失败（非本 story 引入，未修）**：`cargo test --lib` 有 6 个基线即失败的用例——agent_config 5 个 + agent_engine 1 个；根因：`permission.skill` 代码已改为嵌套 map `{"*":"deny"}`（agent_config.rs L1152），测试断言仍为扁平 `"deny"`。属于早前 story 的测试漂移残留，按外科手术原则未动，待 boss 裁决。
- **环境事项**：执行中清理了磁盘（journal 634MB、apt cache、/tmp；经确认删除其他项目 ZettelWeave/backend/target 17GB 构建缓存）；本机补装了 Tauri Linux 构建系统依赖（pkg-config/glib/gtk/webkit2gtk-dev）。

### File List

新增：
- `egosync-app/src-tauri/migrations/031_paired_devices.sql`
- `egosync-app/src-tauri/src/db/paired_devices.rs`
- `egosync-app/src-tauri/src/models/companion.rs`
- `egosync-app/src-tauri/src/services/companion_pairing.rs`
- `egosync-app/src-tauri/src/services/companion_connection.rs`
- `egosync-app/src-tauri/src/commands/companion.rs`
- `egosync-app/src-tauri/tests/test_companion.rs`
- `egosync-app/src/types/companion.ts`
- `egosync-app/src/services/companionService.ts`
- `egosync-app/src/components/settings/CompanionPairingSection.tsx`

修改：
- `crates/companion-proto/src/crypto.rs`（+remote_static_pubkey 透传）
- `egosync-app/src-tauri/src/services/agent_config.rs`（boss 裁决顺手修：6 处预存断言漂移——管家身份文案「你是数字分身管家」、skill 加载措辞、permission.skill 嵌套 map 形状）
- `egosync-app/src-tauri/src/services/agent_engine.rs`（同上，身份文案断言 1 处）
- `egosync-app/src-tauri/src/error.rs`（+3 变体/serialize 臂/测试）
- `egosync-app/src-tauri/src/lib.rs`（模块 pub 化、CompanionState manage、listener spawn、5 命令注册）
- `egosync-app/src-tauri/src/db/mod.rs`、`models/mod.rs`、`services/mod.rs`、`commands/mod.rs`（注册）
- `egosync-app/src-tauri/src/services/data_export.rs`（导出/导入/销毁清单 + 旧档兼容）
- `egosync-app/src-tauri/Cargo.toml`（+companion-proto/tokio-tungstenite/mdns-sd；dev-dep +companion-proto）
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx`（+手机伴侣 tab）
- `egosync-app/package.json`（+qrcode/@types/qrcode）
- `README.md`（+「手机伴侣连接」章节）

### Change Log

- 2026-08-29（评审整改）：三路对抗评审（盲审/边界/验收）发现 3 决策项 + 19 修补项，boss 裁决后全部同日实施：① nonce 校验闭环（app 层 pairingAuth Notice 提交、命中即消费窗口，D1）；② 配对窗口期 NSD 注册/过期回收（首配发现路径打通，D2）；③ 导入不再恢复 paired_devices（配对不可跨机迁移，D3）；④ remove/confirm 终止活跃会话 + 清窗口/pending；⑤ 握手 10s/会话空闲 120s 超时；⑥ PONG 失败走统一清理；⑦ 会话单槽取代竞态（旧会话退出不得回写 Listening）；⑧ 帧日志改记类型判别式（修复 `?other` 明文泄露，NFR-M7）；⑨ data_destroy 回收 NSD/pending/窗口/keyring 静态密钥；⑩ get_status 错误传播 + DESC 首条 + Failed 状态；⑪ mDNS 主机名掺 relay_id；⑫ 前端 QR 倒计时/过期收回、keyring 故障可见、confirmedName 清理；⑬ Cargo dev-dep 去重。验证：`cargo test` 826 lib + 14 集成全绿（新增 4 用例：nonce 缺失/错误拒绝、nonce 消费后重放拒绝、移除终止会话、重连取代僵尸会话）、vitest 438/438、`npm run build` 通过。评审差异包：`CR-12-2-uncommitted.diff`。
- 2026-08-28（追加）：经 boss 裁决顺手修复 6 处预存断言漂移（agent_config 5 + agent_engine 1，均为早前 story 有意变更后测试未跟随：数字管家文案/skill 加载措辞/permission 嵌套 map）；全量 `cargo test` 恢复 834 通过 / 0 失败。
- 2026-08-28：Story 12.2 实现完成。Tasks 1–8 全部勾选；Task 9 验证：vitest 438/438 ✅、`npm run build`（tsc 零错误）✅、`cargo test` 新增 26 用例全绿（companion 单测 16 + 集成 10）✅、services 零回归自查 ✅（仅 data_export.rs 裁决允许项）。全量 `cargo test --lib` 存在 6 个**基线预存**失败（见 Completion Notes ⚠️ 项），与本 story 改动无关（干净基线 stash 复现取证）。

### Review Findings

**Code review（2026-08-29）— 三路并行对抗评审**（Blind Hunter / Edge Case Hunter / Acceptance Auditor，范围：未提交变更 ≈2,640 行，基线 d5345a6）。归并去重后：3 decision-needed / 19 patch / 0 defer / 9 dismiss。**全部 22 项已于同日实施并验证**（cargo test 826 lib + 14 companion 集成全绿、vitest 438/438、`npm run build` 通过），验证记录见 Change Log 2026-08-29。

Decision-needed（已裁决，2026-08-29，均采纳推荐项）：

- [x] [Review][Decision] D1 pairing_nonce 全链路零消费，「二维码单次有效」为未实现承诺。[companion_pairing.rs:99-109,331-350]
  **裁决：实现真 nonce 校验**——握手后手机经 app 层 Notice 提交 QR 中的 pairing_nonce，桌面比对命中即消费窗口、关闭全局放行；与 12.4 约定此契约。→ 转 Patch P0a。
- [x] [Review][Decision] D2 首配发现死锁：首配前手机无任何途径得知桌面地址。
  **裁决：配对窗口期注册 NSD**——生成 QR（开窗）即注册 `_egosync._tcp` 广播，窗口关闭且无配对成功即注销；QR schema 零改动，手机发现路径首配/重连一致。→ 转 Patch P0b。
- [x] [Review][Decision] D3 导入路径破坏单对单不变量且配对绑定跨机必然不可用。
  **裁决：导入时跳过配对绑定**——import 不恢复 paired_devices（导出保留供备份），文档明示「配对绑定不可跨机迁移，恢复后需重新配对」；与 AC1 字面的偏差按规则七记录裁决。→ 转 Patch P0c。

Patch（修法明确）：

- [x] [Review][Patch] P0a 实现 nonce 校验闭环（D1 裁决）：配对窗口持有期望 nonce；握手后经 app 层 Notice（如 `{"type":"pairingAuth","nonce":...}`）收取手机提交的 nonce，比对命中即消费窗口关闭全局放行，不匹配/缺失即拒；README/UI 无需改（承诺兑现）；补窗口内无 nonce 连接被拒测试 [companion_pairing.rs:331-350, companion_connection.rs]
- [x] [Review][Patch] P0b 配对窗口期注册 NSD（D2 裁决）：`pairing_generate_qr` 开窗即注册广播，窗口过期/被消费且无配对成功即注销；保留「已配对 ≥1」常驻注册；补窗口期 ServiceInfo 构造单测 [companion_pairing.rs generate_qr, companion_connection.rs NSD 注册条件]
- [x] [Review][Patch] P0c 导入跳过配对绑定（D3 裁决）：import_json_data/import_sqlite_data 不再恢复 paired_devices（保留 ExportData 字段与导出路径）；README/设置页数据说明补「配对绑定不随导入迁移」；既有旧档兼容测试改断言「导入后配对表为空」[data_export.rs]
- [x] [Review][Patch] P1 paired_device_remove 同步清空配对窗口与 pending 槽；tests/test_companion.rs:2487 改真实时序断言（移除后重连即拒，不再手动清窗口）[blind#3+edge#3+auditor#3]
- [x] [Review][Patch] P2 PONG 发送失败改 `break` 走统一清理，禁止 `?` 提前返回绕过状态机复位与事件发射 [companion_connection.rs:408-411]
- [x] [Review][Patch] P3 remove / confirm_pending 时终止已建立活跃会话（CancellationToken / 会话句柄）；「移除即拒绝」须覆盖存量会话 [commands/companion.rs:41-49]
- [x] [Review][Patch] P4 握手整体包 `tokio::time::timeout`（≈10s）；会话循环加空闲超时或桌面侧周期探活，半开连接不得永久挂起 [companion_pairing.rs:140-160, companion_connection.rs:394-423]
- [x] [Review][Patch] P5 帧日志泄露：`frame_type = ?other` 中 `other` 为整个 Frame 枚举，Debug 输出含解密后 payload（Snapshot/Command 明文入日志，违反 NFR-M7）；改为仅记帧类型判别式，字段名同步修正 [companion_connection.rs:418-419]
- [x] [Review][Patch] P6 并发连接状态竞态：退出清理仅当本连接仍是当前记录连接时执行；新连接进入前终止旧会话（与 P3 共用机制）[companion_connection.rs:248-268,388,424]
- [x] [Review][Patch] P7 data_destroy 后注销 NSD、清 pending/配对窗口、删除 keyring companion 静态密钥 [data_export.rs destroy_all_data]
- [x] [Review][Patch] P8 get_status：DB 错误显式传播（勿 `if let Ok` 吞掉）；`all.pop()` 取到最旧记录，改取 DESC 首条 [companion_connection.rs:184-188]
- [x] [Review][Patch] P9 pending 重绑：同公钥 pending 已存在时重连不得刷新 created_at（120s 作废语义不得被重连无限续期）[companion_pairing.rs:261-273]
- [x] [Review][Patch] P10 mDNS host_name 硬编码 `egosync.local.` 同网多桌面冲突；掺入实例标识或验证冲突改名行为 [companion_connection.rs:479]
- [x] [Review][Patch] P11 WS 监听启动失败后状态失真（listening=true、port=Some(0)）：增加失败态表达，get_status 如实反映 [companion_connection.rs:89,205, lib.rs:304-319]
- [x] [Review][Patch] P12 前端二维码按生成时间展示倒计时并在 300s 过期后收回（过期扫码当前为静默失败）[CompanionPairingSection.tsx:85-105]
- [x] [Review][Patch] P13 keyring 不可用时前端用户可见错误提示（当前仅 console.error 静默死区）[CompanionPairingSection.tsx]
- [x] [Review][Patch] P14 register_nsd try_lock 失败语义改为等效成功（并发注册误报「NSD 锁繁忙」）[companion_connection.rs:489-494]
- [x] [Review][Patch] P15 recv_frame_optional 区分「超时返回 None」与「连接错误向上传播」，已死连接不得继续走配对决策 [companion_connection.rs:454]
- [x] [Review][Patch] P16 proto_err 丢弃错误细节且注释谎称走 tracing：补 tracing 并保留细节
- [x] [Review][Patch] P17 Cargo.toml companion-proto dev-dependency 重复声明
- [x] [Review][Patch] P18 前端 confirmedName 提示在状态刷新后永不清除 [CompanionPairingSection.tsx]
- [x] [Review][Patch] P19 Task 8/9 勾选文本与实现记录校正（测试钩子实际未使用、零回归自查范围应含 agent_config/agent_engine 断言修复——后者已核实为基线预存漂移的经裁决修复，非「为绿改断言」）

Dismiss（9 条，证据驳回）：① 设备名 Notice 更新只 log 不落库——代码注释明示按设计（避免隐性写入）；② PING/PONG keepalive 未闭合——桌面已被动应答 PING，协议无桌面主动探活要求；③ 为测试 pub 化四模块——Dev Agent Record 已记载裁决，运行时零影响；④ `desktop_static_priv` pub 字段——仅存在于测试钩子；⑤ 多设备列表 UI 与单对单冗余——展示无害；⑥ remove_all 死代码——保留无害；⑦ nonce 随机性测试设计——并入 D1 处置；⑧ 盲审「断言改动疑似为绿改绿」——已取证驳回（基线 d5345a6 生产代码已含全部新字符串，旧断言基线即失败）；⑨ 两处排序方向相反——并入 P8。
