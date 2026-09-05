# Investigation: 手机伴侣未发现桌面设备——局域网发现与公网中继路径的关系

## Hand-off Brief

1. **What happened.** 用户在首次配对（扫码页）跨网扫码时手机报「未发现桌面设备」——已 Confirmed 为设计内行为：首次配对按 story 12-4 评审裁决 D2→P23 仅支持局域网直连（NSD），无公网路径。
2. **Where the case stands.** Concluded。根因 = 设计决定而非实现缺失；公网中继链路三端齐备（relay-server + RelayClient + 桌面注册），配对成功后离网即走中继。附带发现手机端 relayAddr 配对固化缺口与两处过时注释。
3. **What's needed next.** 无需修复即恢复使用：把手机连入桌面同一局域网完成配对，此后离网自动经中继（前提：桌面已配置中继地址）。可选后续：relayAddr 固化缺口修补（bmad-quick-dev / bmad-create-story）。

## Case Info

| Field            | Value                                                                   |
| ---------------- | ----------------------------------------------------------------------- |
| Ticket           | N/A（自由文本描述开案）                                                   |
| Date opened      | 2026-09-04                                                              |
| Status           | Concluded（2026-09-04 用户确认首次配对/扫码页报错，Hypothesis 1 → Confirmed） |
| System           | EgoSync 桌面端（Tauri/Rust）+ companion-android 手机伴侣                  |
| Evidence sources | 源码三端（companion-android / egosync-app/src-tauri / relay-server）、story 文档、sprint-status.yaml、README.md |

## Problem Statement

用户原述（作为假设登记，非事实）：

> 采用手机伴侣，通过中继服务器与桌面版本连接，为什么只做了局域网发现，没有公网的发现。导致未发现桌面设备。

## Evidence Inventory

| Source                                          | Status    | Notes                                             |
| ----------------------------------------------- | --------- | ------------------------------------------------- |
| 手机端连接层源码（connection/ 包）               | Available | RealConnectionClient / RelayClient / NsdDiscovery / PairingStateStore |
| 桌面端伴侣源码（companion_connection/pairing）   | Available | QR 生成、relay_addr 配置门控、中继注册               |
| relay-server 源码                                | Available | 中继服务已实现（auth/forward/registry）             |
| Story 文档 12-2/12-3/12-4                        | Available | 设计裁决与验证边界记录                              |
| 用户实际触发场景（首次配对 or 已配对离网）         | Missing   | 决定根因落点，见 Missing Evidence                  |
| 手机 logcat / 桌面 egosync.log                    | Missing   | 未提供；当前不需要即可定位设计层根因                 |

## Investigation Backlog

| # | Path to Explore                                        | Priority | Status | Notes                                        |
| - | ----------------------------------------------------- | -------- | ------ | -------------------------------------------- |
| 1 | 用户触发场景确认（首次配对 vs 已配对离网）              | High     | Done   | 2026-09-04 确认为首次配对/扫码页 → Hypothesis 1 Confirmed |
| 2 | 手机端 relayAddr 配对时固化的补救路径评估               | Medium   | Open   | 桌面后配中继需重扫码，可能违背用户直觉          |
| 3 | 过时注释清理（models/companion.rs:23、types/companion.ts:12） | Low  | Open   | 文档债，非行为缺陷                             |

## Timeline of Events

| Time       | Event                                                              | Source                        | Confidence |
| ---------- | ------------------------------------------------------------------ | ----------------------------- | ---------- |
| Story 12-2 | 桌面配对 + NSD 广播；当时中继未实现，relay_addr 恒 None             | 12-2-desktop-pairing…md       | Confirmed  |
| Story 12-3 | 无状态中继服务器实现（relay-server/，含 Docker）                    | sprint-status.yaml:188        | Confirmed  |
| Story 12-4 | 手机扫码配对 + 三态连接；评审裁决 D2→P23 **首次配对仅直连**          | 12-4-…md:74；sprint-status:189 | Confirmed  |
| 2026-09-04 | 用户报告「未发现桌面设备」                                           | 本案 Problem Statement        | Confirmed（症状） |

## Confirmed Findings

### Finding 1: 报错文案「未发现桌面设备」只存在于首次配对路径

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt:357-364`

**Detail:** `onPairingFailed` 中 `phase == "discovery"`（NSD 发现/解析失败或超时）时输出「未发现桌面设备，请确认与桌面端在同一网络」。已配对会话的断连走 `ConnectionStateMachine` 三态（Direct/Relay/Offline），不产生此文案。

### Finding 2: 首次配对按设计仅支持局域网直连（NSD），无任何公网路径

**Evidence:** `RealConnectionClient.kt:247-252`（`runPairing` 仅 `nsd.discover` + `awaitResolved`）；story 12-4 评审补丁「D2→P23 首次配对仅直连——删除 `QrPayload.supportsRelay` 死代码」（`_bmad-output/implementation-artifacts/12-4-android-scan-pairing-and-tri-state-connection.md:74`）；桌面文案 `egosync-app/src/components/settings/CompanionPairingSection.tsx:292`「首次配对需与电脑处于同一局域网（扫码配对仅支持直连）」

**Detail:** 配对三阶段（发现设备→交换密钥→验证身份）全部经 NSD 发现的直连 WS 完成。这是评审后的显式裁决，不是实现遗漏。

### Finding 3: 公网中继链路已完整实现——但它是"按址寻径"而非"发现"

**Evidence:**
- 中继服务器：`relay-server/src/{auth,forward,registry,main}.rs`，story 12-3 done（`sprint-status.yaml:188`）
- 手机中继承载：`companion-android/.../connection/RelayClient.kt:25-57`（register + XX 鉴权 + 转发态）
- 手机双承载编排：`RealConnectionClient.kt:386-409`（直连环 + 中继环并行，prefer-direct 滞回）
- 桌面中继注册：`egosync-app/src-tauri/src/services/companion_connection.rs:414-456`（配置了 relay_addr 且门控开放时注册 desktop 槽位）

**Detail:** 手机从中继找到桌面的方式是 QR 携带的 `relayAddr + relayId`（`RelayClient.kt:29-34` register 消息按 relayId 寻址），不存在也不需要公网侧的"发现/目录"服务。mDNS/NSD 是 UDP 组播协议，本身无法跨公网。

### Finding 4: 中继是否启用取决于桌面端用户配置，且手机端在配对时固化 relayAddr

**Evidence:**
- 桌面：`egosync-app/src-tauri/src/commands/companion.rs:68-84`（`companion_get/set_relay_addr`，存 `app_settings.companion_relay_addr`，可留空）；`companion_pairing.rs:131-132`（QR 生成时读取该配置）
- 未配置时 QR `relayAddr = null`：`companion-android/.../pairing/QrPayload.kt:10-12`「relayAddr 为 null = 中继继承载禁用，离网即 Offline」
- 手机端固化：`RealConnectionClient.kt:292`（`store.save(payload.desktopStaticPubkey, payload.relayId, payload.relayAddr)`——唯一写入口）；`RealConnectionClient.kt:402-406`（编排启动时 `store.relayAddr != null` 才起中继环）；`PairingStateStore.kt:19-26`（除 `save` 外无任何更新 relay_addr 的方法）

**Detail:** 桌面端 relay_addr 是现读配置（`companion_connection.rs:447`，后补配置 ≤5s 生效注册中继）；但手机端 relayAddr 只在扫码配对那一刻从 QR 固化进 SharedPreferences，之后没有任何同步/更新通道。若配对时桌面尚未配置中继，之后在桌面补配中继，手机仍不会启用中继环——必须重新扫码。

## Deduced Conclusions

### Deduction 1: "没有公网的发现"是把架构设计误读成了实现缺失

**Based on:** Finding 3、Finding 2

**Reasoning:** 公网路径的桌面可达性由 QR 携带的中继地址 + relayId 寻址解决；跨公网做"发现"（类似 mDNS 的广播）技术上不可行，且 PRD FR-40 的设计就是"局域网 NSD 发现直连 + 离网经中继加密转发"双轨，中继被刻意设计为无状态零知识转发（不提供设备目录服务）。

**Conclusion:** 不存在"漏做公网发现"这一缺陷；中继链路已实现且按设计以寻址替代发现。

### Deduction 2: 用户报错的最可能落点是首次配对且手机不在桌面局域网

**Based on:** Finding 1、Finding 2

**Reasoning:** 报错文案只在配对路径出现；配对仅支持 NSD 直连。手机在蜂窝网络/异地扫码 → NSD 12s 超时 → 「未发现桌面设备，请确认与桌面端在同一网络」。

**Conclusion:** 若用户处于首次配对，这是设计内行为（FR-40：配对需同一局域网，配对完成后才能离网经中继）。

## Hypothesized Paths

### Hypothesis 1: 用户在首次配对阶段（未配对）跨网扫码

**Status:** Confirmed（2026-09-04 用户确认报错出现在配对扫码页）

**Theory:** 手机与桌面不在同一局域网时扫码配对，NSD 发现失败报「未发现桌面设备」。

**Supporting indicators:** 报错文案证据（Finding 1）；设计裁决（Finding 2）。

**Would confirm:** 用户确认当时处于配对扫码页、且手机使用移动数据或异地 Wi-Fi。

**Would refute:** 用户确认已配对成功后离网时仍见此文案（不可能路径——已配对断连走 Offline 遮罩）。

**Resolution:** 2026-09-04 用户在场景确认中选择「首次配对/扫码页报错」。链路闭合：手机不在桌面局域网 → NSD 发现 12s 超时 → `onPairingFailed(phase="discovery")` 输出该文案。属 FR-40 设计内行为，非缺陷。

### Hypothesis 2: 用户已配对，但配对时桌面未配置中继（或中继未部署），离网即 Offline

**Status:** Open

**Theory:** 桌面 `companion_relay_addr` 未配置或配对晚于配置，手机 `store.relayAddr = null`，中继环不启动，离网永远 Offline；用户把离线降级体验描述为"未发现桌面设备"。

**Supporting indicators:** Finding 4 的固化缺口；README.md:158「当前限制：中继服务暂未部署」。

**Would confirm:** 用户桌面「全局设置 → 手机伴侣」中继地址为空，或配对早于中继配置；手机离网后呈降级遮罩（只读缓存+速记排队）而非配对报错。

**Would refute:** 桌面已配置中继、且配对时 QR 明确含中继地址（UI 会显示「本二维码含中继地址」）。

### Hypothesis 3: 桌面已配置中继并正确配对，但中继服务器实例本身未部署/不可达

**Status:** Open

**Theory:** relayAddr 链路完好，但自托管中继（ws://…）实际没有运行，手机中继环连接失败回到 Offline。

**Supporting indicators:** README「当前限制」措辞；中继部署是用户自建动作（story 12-3 交付的是 Docker 化服务，不是运营实例）。

**Would confirm:** 中继地址 ping/WS 握手不可达；手机 logcat 中 `RelayClient.connect` 报 IOException。

**Would refute:** 中继实例在线且 desktop 槽位注册成功（桌面日志「中继已连接，注册 desktop 槽位」，companion_connection.rs:508）。

## Missing Evidence

| Gap                          | Impact                                  | How to Obtain                              |
| ---------------------------- | --------------------------------------- | ------------------------------------------ |
| 用户触发场景（配对中/已配对） | 区分 Hypothesis 1 与 2/3 的根因落点      | 询问用户（见 Recommended Next Steps）        |
| 桌面中继配置状态              | 验证 Hypothesis 2                        | 桌面「全局设置 → 手机伴侣」查看中继地址输入框 |
| 中继服务器运行状态            | 验证 Hypothesis 3                        | 核对中继实例部署与桌面日志                   |

## Source Code Trace

| Element       | Detail                                                                 |
| ------------- | ---------------------------------------------------------------------- |
| Error origin  | `companion-android/.../connection/RealConnectionClient.kt:362`（`onPairingFailed`，phase="discovery"） |
| Trigger       | `runPairing`/`waitDesktopConfirmAndRetry` 中 `nsd.awaitResolved` 超时（NSD_WAIT_MS=12s）或发现层 IOException |
| Condition     | 手机与桌面不在同一局域网（或 mDNS 组播被拦/桌面未运行），且处于首次配对/重配对路径 |
| Related files | `NsdDiscovery.kt`、`QrPayload.kt`、`PairingStateStore.kt`、`RelayClient.kt`、`ConnectionStateMachine.kt`；桌面 `companion_pairing.rs`、`companion_connection.rs`、`commands/companion.rs`、`CompanionPairingSection.tsx` |

## Conclusion

**Confidence:** High

1. **"只做局域网发现、没做公网发现"的前提不成立（已被证据修正）。** 公网中继链路三端齐备（relay-server 实现、手机 RelayClient、桌面注册），story 12-3/12-4 均 done。公网路径按 FR-40 架构设计以「QR 携带 relayAddr + relayId 按址寻径」替代"发现"——mDNS 组播本身无法跨公网，中继被刻意设计为无状态零知识转发、不提供设备目录。
2. **「未发现桌面设备」根因 Confirmed（Hypothesis 1）：首次配对跨网扫码。** 首次配对按设计仅支持局域网直连（story 12-4 评审裁决 D2→P23），手机不在桌面局域网 → NSD 超时 → 该报错。配对成功后离网才走中继。**这不是 bug，是设计内行为**；恢复路径：手机连入桌面同一局域网完成配对（前提：桌面在运行、mDNS 组播可达）。
3. **附带发现的真实设计缺口**：手机端 relayAddr 在配对时固化且无更新通道——桌面后补中继配置对已配对手机不生效，需重新扫码；README「中继暂未部署」措辞与代码现状不一致。
4. Hypothesis 2/3（已配对离网场景）因用户确认场景为首次配对而不适用于本案，但固化缺口对后续使用有实际影响，建议关注。

## Recommended Next Steps

### 诊断（先确认场景，无需改码）

1. 确认触发页面：报错出现在**配对扫码页**（Hypothesis 1 成立，设计内行为——把手机连入桌面同一局域网完成配对，之后离网即可经中继）；还是已进入主界面后呈**离线降级遮罩**（Hypothesis 2/3）。
2. 桌面「全局设置 → 手机伴侣」：中继服务器地址是否已填（ws://relay.example.com:7333 形态）；生成二维码时 UI 是否显示「本二维码含中继地址」。
3. 若已填中继：核对中继实例已按 story 12-3 的 Docker 方式部署且可达，桌面日志应有「中继已连接，注册 desktop 槽位」。

### Fix direction（如确认 Hypothesis 2 的固化缺口需修）

- **机制 A（配置固化）**：手机端 relayAddr 仅配对时落盘——修复方向是会话建立后由桌面经任何在用承载下发当前 relay 配置（或 QR 重新展示时更新），避免"后配中继必须重扫码"。
- **机制 B（文档债）**：README.md:158「中继暂未部署」与 `models/companion.rs:23`、`types/companion.ts:12` 的过时注释应更新为"中继为可选自建组件，未配置时仅局域网直连"。

## Reproduction Plan

- 首次配对跨网（Hypothesis 1）：手机关 Wi-Fi 用蜂窝 → 扫桌面 QR → ~12s 后配对页报「未发现桌面设备，请确认与桌面端在同一网络」。
- 固化缺口（Hypothesis 2）：桌面不配中继完成配对 → 手 机离网确认 Offline → 桌面补配中继 → 手机仍 Offline（不重新扫码不恢复中继）。

## Side Findings

- 过时注释：`egosync-app/src-tauri/src/models/companion.rs:23`「relay_addr 在 V1 恒为 None（Story 12.3 之前）」——12-3 已 done，该注释不再成立（Confirmed，文档债）。
- `egosync-app/src/types/companion.ts:12`「V1 中继未部署（Story 12.3），恒为 null」同样过时（Confirmed，文档债）。
- `companion-android/.../PairingScreen.kt:281` 配对页 UI 文案「发现桌面设备（NSD 局域网发现）」如实标注了发现机制仅局域网。

## Follow-up: 2026-09-04

### New Evidence

- `architecture.md:2057`：配对流程原文「手机：扫码 → 经**直连或中继**发起 Noise XX 握手」——架构基线允许配对走中继，**「仅局域网配对」并非原始产品/架构需求**。
- `12-4-…md:74`：限制的真实出处是评审补丁 D2→P23「删除 `QrPayload.supportsRelay` 死代码」——手机端 `runPairing` 只实现了 NSD 直连路径，`supportsRelay` 字段从未被读取；评审面对「补实现中继配对 vs 删死代码讲真话」选择了后者。
- `12-4-…md:73`（D1 deferred）：「中继槽位抢占 DoS 与桌面重连放大……relayId 仅经面对面扫码流转，攻击面有限；根治需改 relay-server，与 12.3 的连接数上限/速率限制等公网加固项合并处理」——V1 中继无鉴权、无加固，relayId 在 QR 内可流转。
- `12-4-…md:231`（R1 开放问题）：「12.3 评审裁决 phone 槽绑定首注册公钥——手机卸载重装（新密钥）后经中继重配对会被槽位绑定拒绝，直到 relay 进程重启。AC6 以直连路径验收不受影响」。
- `12-4-…md:277`（Task 6）：桌面侧「准入后路径」直连/中继共用（`run_authorized_session`），集成测试 `relay_path_pairs_and_pings` 走「register→鉴权→E2E→**首配落库**」——**桌面端本就支持经中继完成首次配对**，限制只存在于手机端 `runPairing` 的 NSD 单路径。

### Additional Findings

**「首次配对必须局域网」的四层理由（按证据强度排序）：**

1. **评审选择的最小改动（直接原因）**：手机端从未实现中继配对路径，`supportsRelay` 是死代码。评审按「外科手术式修改 + 显式失败」原则删除死代码、把文案改真，而不是当场补一条新路径——中继配对从未被显式否决，只是没被实现。
2. **V1 中继无加固（安全理由，D1 deferred）**：中继零知识、无账号、无鉴权目录，槽位先到先得。relayId 就在 QR 里（且可由桌面公钥哈希导出）——若配对走公网中继，任何拿到 QR 内容的互联网攻击者都可抢占 phone 槽发起配对请求；而局域网路径要求攻击者先进入同一局域网，物理临近本身是隐式的带外认证因子（QR 展示在桌面屏幕上，扫码即同处一室）。
3. **重装恢复独立性（R1）**：中继 phone 槽绑定首注册公钥，手机换密钥经中继重配对会被拒；直连配对让「重装重扫即恢复」（AC6）不依赖中继状态。
4. **零服务器首跑（成本理由）**：V1 基线是零服务器成本；首次配对若依赖中继，冷启动体验就依赖用户先自部署 VPS——局域网配对保证开箱即用。

### Backlog Changes

- 新增 #4（Medium）：评估放开「中继首配」——桌面侧已支持，手机侧需在 `runPairing` NSD 失败后回退 `RelayClient.connect` + E2E + pairingAuth；前置条件是 D1 中继加固（槽位抢占防护/速率限制）与 R1（重装重配对被槽位绑定拒绝）有解。

### Updated Conclusion

「首次配对仅局域网」不是产品级硬性需求，而是 12-4 评审在「手机端未实现中继配对」事实上的最小诚实化处理，叠加 V1 中继未加固的安全边界（D1/R1）与零服务器首跑偏好。桌面侧协议上支持中继首配（集成测试已覆盖），放开限制主要是手机端改动 + 中继加固前置。
