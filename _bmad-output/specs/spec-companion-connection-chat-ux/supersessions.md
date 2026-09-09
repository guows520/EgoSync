# 与旧 FR-43 / 旧冻结规格的冲突与取代关系

> 规则七（显式暴露冲突，拒绝折中调和）：本规格明确选择**非阻断连接体验**新范式；旧"全屏遮罩阻断所有操作"范式整体被取代，不得两套并存。下表逐项列出冲突与处置；标注动作（T-S9）在本规格人工批准后执行——旧冻结规格受 `<frozen-after-approval>` 保护，须以人工批准为前提才可改状态行。

## 1. PRD FR-43（`_bmad-output/planning-artifacts/prd-egosync.md` §4.14，:647-655）

| FR-43 条款 | 处置 | 说明 |
| --- | --- | --- |
| 离线时界面明确标注离线与数据截止时间，不以陈旧数据冒充实时的 | **保留** | Degraded 横幅携带 dataAsOf（OfflineSnapshotInfo 既有供给） |
| 降级态下可进行文字速记，重连后自动入队交管家处理且无丢失 | **语义保留、形态取代**（2026-09-08 用户裁决） | 速记条与 `QuickNoteQueue` 退役；离线文字录入经对话 composer 进入离线待发箱（幂等 commandId、落盘持久化——进程被杀不丢），恢复后自动续发；flush 守门收紧为 commandReady && paired |
| 依赖引擎的功能入口在降级态不可交互并说明原因 | **保留（语义）** | 从"全屏遮罩统一拦截"改为"逐项禁用 + 说明原因"——PRD 正文本身未规定遮罩形态，阻断遮罩是实现层裁决（见 §2） |
| 局域网桌面关机与广域网中断呈现统一降级体验 | **保留** | Connecting/Reconnecting/Degraded 统一覆盖；新增冷启动 20s grace 属产品裁决扩展 |
| `[ASSUMPTION] 离线缓存限于最后已知快照+文字速记` | **保留** | 不扩大缓存范围 |

**PRD 修订动作（T-S9）**：FR-43 正文追加一行裁决记录：「2026-09-08 产品裁决：降级呈现采用非阻断紧凑状态指示（SPEC-companion-connection-chat-ux），全屏阻断遮罩范式废止；连接期引入 Connecting/Reconnecting 宽限态；离线文字录入由速记条改为对话输入离线待发（网络不可用仅提示，恢复后自动续发）。」不删除任何保留条款。

## 2. `spec-companion-offline-deadlock-recovery.md`（frozen，2026-08-30，status: done）

| 旧条目 | 冲突 | 新裁决 | 处置 |
| --- | --- | --- | --- |
| 「FR-43 语义不变——速记条仍是遮罩唯一持续开放入口」（Always 条） | 遮罩退役后"遮罩唯一入口"失去载体 | 速记条与 `QuickNoteQueue` 退役；离线录入经对话 composer 离线待发箱承接（2026-09-08 用户裁决：网络不可用仅提示，恢复后自动续发） | **取代** |
| 「不重设遮罩既有样式与速记交互（UX-M 约束），仅追加受控出口」（Never 条） | 遮罩整体退役 | 阻断形态（fillMaxSize + 0.72 蒙层 + pointerInput 消费）与速记条一并移除、不复用速记输入 UI——离线录入由对话 composer 离线待发箱承接（2026-09-08 裁决） | **取代** |
| 「不新增手动重连按钮（自动退避已覆盖）」（Never 条） | 无直接冲突 | 自动重连仍为主，不新增手动重连按钮 | **保留** |
| 「不改帧协议/握手/信任锚逻辑；不动桌面端」（Never 条） | T-S5/T-S6 需桌面改动与 Notice 发送 | 复用既有 `Frame.Notice`（无 schema/帧类型变更）、握手与信任锚逻辑不动；桌面仅两拒绝点 + QR UI | **部分取代**（范围扩大经本规格人工批准授权） |
| 遮罩受控出口「连接不上？解除配对并重新扫码」无条件渲染于任意 Offline | 网络失败误导为解绑建议 | 重配入口仅由 PairingHealth ≠ Ok 驱动并附原因 | **取代** |
| `healCorruptPairingIfAny()` 元数据判空自愈 | 未覆盖私钥缺失 | 保留元数据自愈，扩展凭据完整性检测（loadExisting 拆分） | **扩展保留** |
| 评审 patch「速记 flush 守门 = engineAvailable && paired」 | engineAvailable 仍会假在线；且 `QuickNoteQueue.flush()` 为纯 mock（从未真实发送，`QuickNoteQueue.kt:38-44`） | 守门收紧为 commandReady && paired，载体换为离线待发箱并真实发送（意图不变：非真实恢复不得 flush / 虚假标记已发送） | **判据取代，意图保留** |
| 「自愈须在 startDestination 冻结前同步完成」（Always 条） | 无冲突 | 冷启动 CredentialMissing 检测沿用同一时序约束 | **保留** |

**标注动作（T-S9）**：该规格 frontmatter `status` 追加 `superseded-by: SPEC-companion-connection-chat-ux (2026-09-08 裁决：非阻断降级范式)`；冻结块不改写正文（历史记录），以状态行 + 本表为准。

## 3. `spec-companion-android-remove-connection-banner.md`（done，2026-05-27）

| 旧条目 | 冲突 | 新裁决 | 处置 |
| --- | --- | --- | --- |
| 删除三态常驻横幅；「离线明示由 DegradedOverlay 独自承担」 | 新裁决要求 Connecting/Reconnecting 有紧凑明示 | 常驻横幅不复活（Direct 不再显示横幅）；仅 Connecting/Reconnecting/Degraded 出现紧凑状态指示（小图标/小条） | **部分取代**：删除裁决保留，"遮罩唯一明示"被取代 |

**标注动作（T-S9）**：无需改该文档正文；本表为其裁决边界的现行解释源。

## 4. 其他相关既有规格

| 文档 | 关系 |
| --- | --- |
| `spec-companion-android-mobile-ui-fixes.md`（内层 Scaffold contentWindowInsets=0 修顶部双重 inset） | 无冲突；其"内层置零"手法是 IME 诊断 A/B 变量 3 的既有参照，B 阶段若需调整须以诊断数据为准并保持顶部修复不回归 |
| `spec-mobile-fr-parity`（移动端 FR 屏补全） | 无冲突（其 Non-goals "不接入真实连接层"系该规格 story 范围声明，已完结） |
| `architecture.md` 手机伴侣增量章节（FR-40~43 三态状态机描述） | **需同步（T-S9 一并处理）**：状态机描述补 Connecting/Reconnecting/Degraded 与 PairingHealth；帧协议章节补 pairingRejected Notice（复用 Notice 类型） |

## 5. 并存禁令

实施验收包含显式检查：主界面不得残留任何全屏遮罩挂载路径（`DegradedOverlayHost` 阻断形态零引用）；不得同时存在"遮罩阻断"与"紧凑指示"两条降级呈现代码路径；`engineAvailable`/`ConnectionState.Offline` 不得再被任何 UI 消费（grep 断言）。
