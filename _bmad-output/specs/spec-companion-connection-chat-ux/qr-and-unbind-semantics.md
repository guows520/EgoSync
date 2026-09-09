# QR 生命周期与解绑语义（CAP-4 契约）

> 现状证据（Confirmed，Finding 4/5/6）：
> - 手机 `unpair()` 清本地元数据并异步擦私钥（`RealConnectionClient.kt:171-187`）→ 重扫旧 QR 必然生成新公钥；首配已消费 nonce 且窗口置 None（`companion_pairing.rs:413-455`）→ 桌面早期准入拒绝（`companion_connection.rs:657-665` 直连 / `:568` 中继，同构）。
> - 桌面 UI 有效期内无重新生成入口：`CompanionPairingSection.tsx:285-326` 仅 `qrPayload == null` 时显示生成按钮；`companion:paired` 事件只 refresh 设备列表（`:92-94`），不清码。
> - 手机失败文案按承载层合并（`RealConnectionClient.kt:380-396`），无法表达"旧 QR/准入被拒"。

## 1. 桌面：重新生成二维码

- 显示态（`qrPayload != null`）新增「重新生成二维码」按钮（次强调样式，紧邻倒计时说明）。
- 点击 → 二次确认对话框：「重新生成后当前二维码立即失效，正在进行的扫码将无法完成配对。继续？」。
- 确认后调用既有 `handleGenerateQr`：后端 `generate_qr` 产出新 nonce、`open_pairing_window_and_sync` 开新窗口（`companion_connection.rs:1094-1113`，覆盖旧窗口语义已存在）→ 旧码即刻失效，新码可扫。
- 过期后（倒计时清空）生成按钮转为主操作——现有行为保留。

## 2. 桌面：QR 被消费后立即失效标记

- `companion:paired` 事件（首次配对/换绑成功，payload 含 deviceId）→ 立即清除 `qrPayload/qrSvg/qrExpiresAt`，展示「二维码已被使用」状态 + 「重新生成二维码」为主操作。
- 不得继续把已消费码显示为可扫（文案"二维码单次有效"与显示行为矛盾是 Finding 5 的核心）。
- 实现注意：`companion:paired` 也会因换绑确认（`pairing_confirm`）触发——同一清码语义适用（该窗口 nonce 已消费）。

## 3. 结构化配对失败原因（协议机制）

### 3.1 桌面侧（唯一协议改动）

在两个拒绝点、经已建立的 transport 发送 NOTICE 帧后再关闭连接（复用既有 `Frame.Notice`，不新增帧类型、不动协议版本）：

```json
{ "type": "pairingRejected", "reason": "pairingWindowClosed" | "nonceConsumed" }
```

| 拒绝点 | 位置 | reason 判定 |
| --- | --- | --- |
| 早期准入（直连 `companion_connection.rs:657-665`、中继 `:568` 同构路径） | 未知公钥 + 窗口关闭/已消费 | `pairingWindowClosed` |
| nonce 校验失败（`run_authorized_session`，`companion_connection.rs:750-762`） | 窗口已过期/关闭 → `pairingWindowClosed`；窗口开但 nonce 不匹配 → `nonceConsumed` |

- 仅发送 reason 等判别信息，不含密钥/QR 内容（NFR-M7 对齐）。
- 兼容性：旧手机在会话循环 `Frame.Notice -> Unit` 忽略（`RealConnectionClient.kt:714`）→ 回退现状；旧桌面不发 → 手机回退现状。双端可独立发布。

### 3.2 手机侧识别与分类

- **配对路径**（`runPairing` → probe 阶段）：`probeSessionAlive`（`:370-378`）当前"任一有效帧即存活"——改为先识别 `pairingRejected` Notice：命中 → 直接返回结构化失败（`PairingProgress.Failed` 按 §4 文案），**不得**落入 `waitDesktopConfirmAndRetry`（旧 QR 场景现状会误等 120s 桌面确认）或网络双失败文案。
- **已配对重连路径**（`runDirectLoop`/`runRelayLoop` → `startSessionLoop`）：会话循环收到 `pairingRejected` Notice → 本地 paired=true 且本机公钥未变 → 判定 `PairingRevoked`，发布恢复事件（见 state-and-recovery-model.md §5.2），终止退避重试；不再无限重连。
- **trustMismatch**：手机侧 `verifyTrustAnchor` 失败（`IdentityVerificationException`）即本地判定，无需桌面信号。
- **networkUnavailable**：仅承载层失败（NSD/中继连接失败）且无结构化拒绝时归此类——手机不再通过"连接被关闭"猜测所有失败都是网络问题。

## 4. 原因码 → 用户文案映射

| 原因码 | 判定来源 | 用户文案（示例，实施可微调但语义不得缩水） | 恢复动作 |
| --- | --- | --- | --- |
| pairingWindowClosed | 桌面 Notice（配对流程中） | 「二维码已使用或已过期，请在桌面重新生成后再扫」 | 引导回桌面重新生成 |
| nonceConsumed | 桌面 Notice（配对流程中） | 「二维码已失效（配对码不匹配），请使用桌面当前显示的二维码」 | 同上 |
| identityRevoked | 桌面 Notice + 本地已配对且公钥未变 | 「桌面已解除与此手机的配对，请重新扫码配对」 | 重配入口（PairingRevoked 恢复事件） |
| trustMismatch | 手机信任锚校验失败（已配对上下文） | 「桌面身份已变化（可能重装），需重新配对」 | 重配入口 |
| networkUnavailable | 承载层失败、无结构化拒绝 | 保留既有如实网络文案（「未发现桌面设备…」/「中继连接失败…」），但不得附带"重新扫码"暗示 | 检查网络/稍后自动重试 |

配对失败原因须以结构化枚举进入 `PairingProgress.Failed`（携带 reason 而非仅字符串），文案层再映射——禁止在连接层拼用户文案。

## 5. 手机本地解绑语义

- `SettingsScreen` 解绑确认文案明确：「仅清除此手机的凭据；重新连接需要桌面端生成新二维码，当前二维码不可复用。」（现文案「需重新扫码才能再次连接」未表达"旧码不可复用"，必须替换）。
- 本地解绑保持现有清理面（`unpair()`：会话/指令/流式/元数据/密钥 + 容器层快照与通知清理 + 待发箱保留，恢复连接后续发）。
- **语义边界声明（规格内必须体现，不实施协议）**：本地解绑 ≠ 双端解绑。桌面仍保留旧手机公钥记录，须在桌面「已配对设备」列表中手动移除，或忽略其存在（旧公钥无窗口不可重连，无安全影响）。双端一键解绑（在线认证命令 + 断线离线兜底）为独立立项，本规格 Non-goal。
