---
stepsCompleted:
  - step-01-document-discovery
  - step-02-prd-analysis
  - step-03-epic-coverage-validation
  - step-04-ux-alignment
  - step-05-epic-quality-review
  - step-06-final-assessment
filesIncluded:
  - _bmad-output/planning-artifacts/prd-egosync.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/epics.md
referenceFiles:
  - _bmad-output/planning-artifacts/addendum.md
  - _bmad-output/planning-artifacts/.decision-log.md
  - _bmad-output/project-context.md
  - companion-android/README.md
  - _bmad-output/planning-artifacts/implementation-readiness-report-2026-07-23.md
---
# Implementation Readiness Assessment Report — 手机伴侣（Android）基建增量

**Date:** 2026-08-27
**Project:** 探索

**范围声明**：本报告为**增量校验**，仅覆盖手机伴侣（Android）基建——PRD §4.14（FR-40～FR-43）↔ `architecture.md` 文末六个增量章节 ↔ `epics.md` Epic 12～14（3 Epic / 9 Story）三方对齐。**桌面基线（Epic 1～11、架构文档桌面章节）沿用 `implementation-readiness-report-2026-07-23.md` 结论（READY WITH CONDITIONS），本报告不重复校验。**

校验方式：只读核对，未修改任何规划文档；对架构结构清单与实际仓库做了事实核验（迁移编号、command 注册位置、原型文件存在性、workspace 禁令）。

## 1. Document Discovery（增量）

### 校验对象

| 文档 | 增量段落 | 状态 |
|------|----------|------|
| `prd-egosync.md` | §4.14（FR-40～43 + 分档表）、§6.1 In Scope、§9 假设索引、Privacy/Cost/Platform 守则 | 已提交（3a23ca4，2026-08-25） |
| `architecture.md` | 行 1926～2388 共六个增量章节：Incremental Project Context Analysis → Starter Template Evaluation → Core Architectural Decisions → Implementation Patterns Addendum → Project Structure Addendum → Architecture Validation Results | 已提交（3a23ca4，2026-08-25） |
| `epics.md` | 行 2982～3536「增量拆分：手机伴侣（Android）基建（Epic 12 起）」，557 行纯追加 | **未提交**（工作区变更，git diff 557 insertions） |

### 补充参照

- `addendum.md`：四通道模型、已否决备选（B 独立 App / C PWA）——与 epics 增量段引用一致
- `.decision-log.md`：决策 #17（立项形态）、#18（基建定案与 FR-42 降级）——内容与 PRD/架构三方一致
- `_bmad-output/project-context.md`：基线 100 条规范（命名/IPC/错误/keyring），增量显式声明全量继承
- `companion-android/`：高保真原型，移动端 UX 事实源（README 页面地图 + 换装点文档）

### 发现

- 无整档/分片重复冲突；无第二份 PRD。
- epics.md frontmatter（stepsCompleted / inputDocuments / date=2026-07-23）未随增量更新——这是「严禁修改既有内容」约束下的**已知显式取舍**：增量输入文档完整记录于追加段开头的「追加说明」块中。经核验该说明块内容齐备（范围声明、六项权威输入、FR-42 处理口径），**不判为缺失**。
- UX 对齐来源说明：`ux-design-specification.md` 为桌面规范，不覆盖移动端；移动端 UX 事实源为 companion-android 高保真原型，且 epics 增量段已将其编码为 UX-M1～M5 验收约束。此为刻意设计而非缺口。

## 2. PRD Analysis（增量）

### FR 条款清单与可测性

| FR | 状态 | 可测条款 | 假设 |
|----|------|----------|------|
| FR-40 配对与连接 | 在范围 | 6 条：扫码绑定 / 局域网自动发现直连 / 中继切换与切回 / 三态可见 / 配对持久化重装恢复 / 中继不可读明文 | V1 单对单；中继仅加密流量转发 |
| FR-41 实时状态同步 | 在范围 | 5 条中 4 条在范围（第 4 条「锁屏快捷操作等效」随 FR-42 一并延后，见 N-1） | 重连仅补最新快照 |
| FR-42 移动推送 | **DEFERRED（决策 #18）** | 原条款保留、恢复条件已记录 | 假设索引已划线更新 |
| FR-43 离线降级 | 在范围 | 4 条：离线标注截止时间 / 速记无丢失 / 入口禁用说明原因 / 两种断网统一降级 | 缓存限于最后快照+速记 |

### 一致性核验

- **分档表自洽**：核心 18 项（FR-1,2,3,11,12,14,15,16,18,19,20,21,22,24,29,30,33,38）+ 适配 6 项（FR-5,7,8,9,17,23）+ 桌面优先 9 项 + 引擎持有 6 项 = 39 项 FR 全量分档，无遗漏无重叠；epics 增量段分档清单与 PRD §4.14 逐项一致。
- **FR-42 降级三方一致**：PRD §4.14 DEFERRED 块（含理由：FCM service account 凭证无法安全分发至用户桌面端；中继 FCM 代理因需放宽零知识假设被否决）↔ 决策 #18 ↔ 架构「通知分发（FR-42 降级记录）」节，口径完全一致；PRD §6.1 / §9 / Non-Goals 均已同步修订（架构随附待办已履行）。
- **§6.1 / 守则一致**：In Scope「手机伴侣App……FR-40至FR-43（FR-42 推送已降级出 V1，仅应用内通知）」与 §4.14 一致；Privacy「中继仅做端到端加密转发」、Cost「中继是首个服务端组件」、Platform「仅 Android」均与架构增量章节对应。
- **假设索引**：§9 中 §4.14 四条假设与各 FR 内联 ASSUMPTION 一一对应，FR-42 条目已用删除线+取代说明更新。

## 3. Architecture Alignment（增量章节）

### 3.1 定稿决策继承核对（未被重新发明）

| 定稿决策 | 架构增量 | epics 增量 | 一致 |
|----------|----------|------------|------|
| Kotlin + Compose 原生客户端 | ✅ | ✅（12.4 沿用原型栈） | ✓ |
| 自建 Rust 无状态中继（axum 0.8） | ✅ | ✅（12.3） | ✓ |
| Noise XX + 扫码互信 | ✅ | ✅（12.1/12.2/12.4） | ✓ |
| NSD/mDNS `_egosync._tcp` 直连 | ✅ | ✅ | ✓ |
| 同一加密帧协议双承载 | ✅ | ✅ | ✓ |
| 快照口径落死（200 条 / 10MB / 记忆不上机） | ✅ | ✅（13.1 逐字继承） | ✓ |
| companion_dispatch 唯一指令入口 | ✅ | ✅（13.3 硬边界 #2 AC） | ✓ |
| FR-42 降级 + NotificationDispatch 预留 | ✅ | ✅（14.2 薄故事） | ✓ |

### 3.2 Implementation Sequence（8 步）↔ Story 顺序（9 个）映射

| 架构步骤 | Story | 备注 |
|----------|-------|------|
| 1 加密协议 crate + schema | 12.1 | 先行冻结，三方共同依赖 |
| 2 桌面 pairing/connection 服务 | 12.2 | |
| 3 relay-server + Docker | 12.3 | 可与 12.2 并行（见下） |
| 4 Android 骨架：扫码配对+状态机 | 12.4 | 依赖 12.2+12.3 |
| 5 快照引擎 + 状态通道 | 13.1 + 13.2 | 桌面侧与手机侧各一 Story |
| 6 指令通道 + 流式回流 | 13.3 | |
| 7 降级态 + 速记队列 | 14.1 | |
| 8 NotificationDispatch 接口（预留位） | 14.2 | |

**结论：一致。** 9 Story 对 8 步的唯一非一一映射是步骤 5 拆为 13.1（桌面生成/下发）+ 13.2（手机消费/渲染），顺序不变。epics 允许 12.2/12.3 并行，是对架构唯一硬前置约束（「加密协议 crate → 三方共同依赖，最高优先级，先行冻结」）的合法松弛——12.2 与 12.3 均仅依赖 12.1，互不依赖；12.4 依赖 12.2+12.3 完整保留。强顺序链 `12.1 → {12.2, 12.3, 12.4} → 13.1 → 13.2 → 13.3 → 14.1 → 14.2` 与架构无矛盾。

### 3.3 四条硬边界 → Story 验收落位

| 硬边界 | 落位 Story / AC | 判定 |
|--------|----------------|------|
| #1 加密边界（Noise 依赖收敛） | 12.1「crypto.rs 是全仓唯一依赖 `snow` 的位置」；12.4 Android 侧 noise-java 握手 | ✓（架构原文措辞有歧义，见 I-2；Story AC 口径已自洽） |
| #2 桌面边界（dispatch 唯一入口、现有 services 零感知） | 12.2「现有 services 文件零改动」；13.1「不改现有 services 业务逻辑」；13.3「不出现任何绕过 dispatch 直调 agent_bridge/opencode/DB 的路径」 | ✓ |
| #3 中继边界（零持久化） | 12.3「注册表只存在于内存，零数据库、零磁盘写、断线即丢」+ 集成测试「重启后注册表为空」断言 | ✓ |
| #4 Android 边界（UI 不触连接实现、禁 Room、速记只进队列） | 12.4「ConnectionClient 接口签名不变……UI 层零改动」；13.2「不引入 Room」；14.1「速记写入本地持久化 FIFO 队列」 | ✓ |

### 3.4 结构清单 vs 仓库事实核验

| 架构声明 | 仓库事实 | 判定 |
|----------|----------|------|
| `migrations/029_paired_devices.sql` | **029/030 已被占用**（`029_llm_config_network_location.sql`、`030_llm_provider_extended.sql`，2026-07-27 提交，早于架构增量定稿 2026-08-25） | ✗ 见 I-1（应用 031） |
| `lib.rs [M] 仅注册新 commands` | `lib.rs:306` invoke_handler 实存 | ✓ |
| 桌面 services/ 扁平文件 + 前缀命名 | `services/` 30 个扁平 .rs（agent_bridge.rs 等） | ✓ |
| commands/{domain}.rs 薄层 | `commands/` 21 个域文件 | ✓ |
| 禁止仓库根 Cargo workspace | 仓库根无 Cargo.toml | ✓（禁令可执行） |
| `crates/companion-proto`、`relay-server` 全新子项目 | 均不存在，为待新建 | ✓ |
| Android 包结构 pairing/connection/sync/notify/ui | 原型文件逐一实存（AppModelContainer / ConnectionClient / FakeConnectionClient / SnapshotStore / QuickNoteQueue / NotificationDispatch / InAppNotificationAdapter / PairingScreen / ui/memory 等 38 个 .kt） | ✓ |
| 帧类型 8 种冻结 | 12.1 AC 八种逐一列出，与架构通信模式节一致 | ✓ |
| Tauri 事件 `companion:` 命名空间 | 基线事件规范 `{domain}:{verb_past}`（role:created 等） | ✓ 延续惯例 |
| `AppError` 新增三变体、序列化形状不变 | 基线 AppError 约定（`{ "Variant": "msg" }`） | ✓ 延续惯例 |

### 3.5 Story 12.2 桌面配对入口（最小必要推断）与桌面规范一致性

架构结构清单未枚举桌面 React 变更；Story 12.2 AC 以「前端以 `services/` 封装 invoke、经 `useTauriCommand`/hook 调用，组件放域目录，遵循桌面既有前端规范」落位。与基线规范逐项核对：settings 域目录实存（`src/components/settings/`）；service 封装 invoke 惯例实存（20 个 camelCase service 文件）；`useTauriCommand` 惯例符合 project-context.md IPC 规则；新增 5 个 Tauri Command（`pairing_generate_qr` 等）为域前缀 snake_case，与 `chat_send_message`/`role_create` 同构。**判定：一致性成立，属 pairing_generate_qr 必须有前端调用方的最小必要推断，非越界。**

## 4. Epic Coverage Validation（增量）

### 4.1 FR 逐条落位矩阵

**FR-40（6 条，全部落位 E12）：**

| PRD 可测条款 | Story / AC |
|--------------|------------|
| 扫码完成绑定，无需手动网络配置 | 12.2（pairing_generate_qr + QR payload 四字段）+ 12.4（ML Kit 真实扫码；「或配对码」为可选替代，扫码即满足条款） |
| 同一局域网自动发现直连 | 12.2（NSD `_egosync._tcp` 持续广播）+ 12.4（NsdManager 发现 → WS 直连） |
| 离网经中继、回网切回直连 | 12.4（探测 3s 滞回 → 中继；回网自动切回，全程无人工干预） |
| 任意时刻可查连接状态 | 12.2（companion_get_status + 事件）+ 12.4（状态栏三态 + 设置页圆点） |
| 配对持久化、重装重扫恢复 | 12.2（paired_devices 表 + 免配对重连）+ 12.4（重装重扫、桌面零重置 AC） |
| 中继无法读取明文 | 12.1（Noise XX 会话密钥仅两端）+ 12.3（纯二进制密文转发、不解析） |

**FR-41（在范围 4 条，全部落位 E13；第 4 条锁屏条款随 FR-42 延后，见 N-1）：**

| PRD 可测条款 | Story / AC |
|--------------|------------|
| 角色/管家活动即时更新 | 13.1（STATE_DELTA 主动推送）+ 13.2（即时刷新、无需下拉） |
| 四象限/大石头/仪表盘实时反映 | 13.1（快照口径含三者）+ 13.2（各屏真实数据） |
| 手机对话桌面执行、流式回流 | 13.3（COMMAND → dispatch → 既有 services → STREAM_TOKEN 镜像 `llm:stream`） |
| 断线变化重连后最新快照补齐 | 13.1（重连全量 SNAPSHOT 替换，不做历史回放）+ 13.2（各屏以最新快照替换） |

**FR-43（4 条，全部落位 14.1）：**

| PRD 可测条款 | Story / AC |
|--------------|------------|
| 离线明确标注 + 数据截止时间 | 14.1（Offline 态标注「离线」+ 快照元数据截止时间） |
| 速记重连自动提交无丢失 | 14.1（持久化 FIFO + 幂等 ID + 桌面确认删队 + 中途断线重试） |
| 依赖引擎入口不可交互并说明原因 | 14.1（灰化/拦截 + 原因说明 + 记忆入口置灰） |
| 关机与广域网中断统一降级 | 14.1（同一 Offline 状态机路径、无差异化处理，AC 显式断言） |

**FR-42（DEFERRED，接口预留边界守住）：** 全增量**仅 Story 14.2 一个 NotificationDispatch 接口预留薄故事**；其 AC 显式声明「本 Story 不实现任何系统推送、锁屏操作、FCM 集成（FR-42 DEFERRED 边界，违反即超出范围）」，并将接口冻结为 InAppAdapter 唯一实装 + FCM/UnifiedPush 适配器位。逐 Story 扫描确认：12.1 的 NOTICE 帧仅为传输层帧类型定义，其余故事无任何推送实现内容。**判定：刻意裁剪边界完整守住，无缺口亦无越界。**

### 4.2 快照口径与 UX-M5 防呆的负向断言核验

| 防呆点 | 落位（含负向断言） |
|--------|--------------------|
| 记忆库不上机 | 13.1 AC「**记忆库内容不上机**（记忆条数统计数字属仪表盘指标，允许）」+ 测试「含『记忆内容不在快照中』的**负向断言**」；13.2「真实 SnapshotStore 中**不存在**记忆条目内容（mock 的 memories/memorySources 被移除出快照通道）」；13.3 记忆「**不接入快照通道**」走 COMMAND 现查；14.1「记忆入口同步置灰并说明原因」 |
| 10MB 截断 | 13.1（按最旧截断 + 截断信息进元数据 + 核心状态不缺失 + 测试覆盖）+ 13.2（截断域按截止时间明示，不以缺失冒充完整） |
| 200 条会话 | 13.1（口径「活跃及近期会话各最近 200 条消息」逐字落死） |
| UX-M5 记忆走 COMMAND 现查 | 13.2（记忆页置「待接指令通道」状态、不显示假数据）→ 13.3（现查现显 + 加载态 + 重试入口）→ 14.1（降级置灰），三段闭环 |

### 4.3 帧协议冻结核验（Story 12.1）

- 8 种帧类型全部定义（枚举小写字符串、payload camelCase、encode→decode 逐帧往返测试）✓
- `HELLO` 必含 `protocolVersion: u16`，**缺版本字段的解码被拒绝** ✓
- 黄金向量互通：snow 侧握手转录 + 加密帧 fixtures 进 cargo test；noise-java 侧等价向量开发期手动产出、随测试提交、CI 自动执行——snow↔noise-java 互通风险在首故事内出清 ✓
- 参数套件 `Noise_XX_25519_ChaChaPoly_BLAKE2s` 验证后冻结写入 schema.json ✓（架构 Important Gap #1「Noise 参数套件未冻结」已被本 Story 吸收解决）

### 4.4 依赖入口核验（companion_dispatch 注入对象 ↔ Epic 1～11）

| dispatch 调用的既有能力（13.3 AC 列举） | 桌面既有 Epic | 判定 |
|------------------------------------------|----------------|------|
| 对话发送（管家/角色，流式） | E1（管家 Agent + llm:stream）/ E2（Agent Loop 回路） | 在范围 |
| 任务操作（四象限勾选等） | E3（任务 CRUD + 四象限） | 在范围 |
| 建议确认/拒绝 | E4（建议确认/拒绝学习反馈） | 在范围 |
| 记忆查询（含溯源/遗忘轻操作） | E2（记忆查询/溯源 UI、选择性遗忘） | 在范围 |
| 简报行动点确认 | E6（晨间简报生成器） | 在范围 |

快照消费侧数据源同样在范围内：仪表盘指标（E4/E11）、四象限任务（E3）、会话消息（E1/E2，conversations.db）、晨间简报/周复盘（E6）、未读通知（E4 三级通知）、角色卡/能量（E4）。13.1 的 debounce 触发依赖「既有事件/通知路径」——基线「写操作返回确认 + Event 推送变更通知」惯例已提供。

**判定：Epic 12～14 的全部依赖入口均落在桌面 Epic 1～11 已完成范围内；其就绪度沿用 2026-07-23 报告结论（READY WITH CONDITIONS），本报告不重复校验。**

### 4.5 NFR / Additional / UX-DR 覆盖抽查

- NFR-M1（零知识）：12.1（密钥仅两端）+ 12.3（零持久化断言 + 帧明文不入日志）✓
- NFR-M2（成本红线）：12.3（单 VPS Docker、$5/月级、数百并发压测文档化）✓
- NFR-M3（诚实代价）：14.1（降级明示）+ 14.2（桌面未运行=无新通知）✓
- NFR-M4（桌面零回归）：12.2/13.1/13.3 均有「现有 services 零改动」AC ✓
- NFR-M5（唯一事实源）：13.1/13.2（快照口径+记忆不上机）+ 14.1（队列本地、提交回桌面）✓
- NFR-M6（单对单）：12.2（paired_devices 模型）+ 12.4（新配对按产品口径处理旧记录）✓
- NFR-M7（基线规范继承）：全部桌面 Story + 12.1/12.3 日志纪律 AC ✓
- Additional 1～9 与 UX-M1～M5 的映射表（epics 增量段自带的覆盖核查表）逐项核对无空档 ✓
- 换装约束：UX-M1（零重做）/ UX-M2（AppModelContainer 唯一换装点，原型 README「只改容器一处，UI 零改动」+ Story AC 三处落位）/ UX-M3（截图零回归，所有换装 Story 均有逐屏比对 AC）/ UX-M4（mock 替换清单：模拟扫码→ML Kit、QuickNoteQueue mock flush→真实 COMMAND、Debug 四态保留）✓

## 5. Epic Quality Review（增量）

### 5.1 逐 Story 质量评估

| Story | 尺寸 | 无前向依赖 | AC 可测 | 架构一致 | 备注 |
|-------|------|-----------|---------|----------|------|
| 12.1 协议 crate | 合适 | ✓ | ✓ | ✓ | 风险出清点设计合理；noise-java 黄金向量「开发期手动产出」为 AC 内显式声明的手动步骤，不构成工具链耦合 |
| 12.2 桌面配对连接 | **偏大（有条件通过）** | ✓（12.4 前以协议 crate 测试客户端模拟握手，AC 显式） | ✓ | ✓ | 迁移编号须按 I-1 调整；建议 create-story 时列文件级任务清单（Rust 六文件 + 前端 pairingService/types/settings 组件） |
| 12.3 relay-server | 合适 | ✓ | ✓ | ✓ | 零持久化断言进 `tests/`，独立子项目可独立验收 |
| 12.4 Android 配对换装 | **偏大（有条件通过）** | ✓ | ✓ | ✓ | 扫码+握手+Keystore+NSD+双承载+滞回+退避+三态+重装+截图；受 UX-M2 换装约束（UI 零改动）显著减负；建议 create-story 时设分阶段检查点 |
| 13.1 快照引擎 | 合适 | ✓ | ✓（含负向断言） | ✓ | 口径逐字落死，防漂移 |
| 13.2 快照视图换装 | 合适 | ✓ | ✓ | ✓ | UX-M5 防呆过渡态（记忆页「待接指令通道」）处理干净 |
| 13.3 指令通道 | **偏大（有条件通过）** | ✓ | ✓ | ✓ | dispatch + 流式镜像 + 多操作面换装；换装约束减负；幂等去重 AC 为 14.1 提供机制底座（跨 Story 协同显式） |
| 14.1 降级与速记 | 合适 | ✓ | ✓ | ✓ | 两条降级路径一致性有显式 AC；断线重试 + 幂等联用闭环 |
| 14.2 通知薄故事 | 合适 | ✓ | ✓ | ✓ | 接口冻结 + 边界声明清晰；见 N-3 建议 |

### 5.2 质量总评

- 三个「偏大」Story（12.2 / 12.4 / 13.3）与 2026-07-23 报告对 Story 10.2 的关切同类，但风险更低：换装模型（UI 零改动、装配点唯一）把工作量压缩到协议/连接层；AC 均已分组且可测。处理方式沿用基线报告口径：**拆分非必需，但 create-story 阶段须给出文件级清单与分阶段检查点。**
- Story 间协同机制（12.1 黄金向量 → 12.4 互验；13.3 幂等 ID → 14.1 无丢失）在 AC 中显式声明，无隐式耦合。
- 全部 9 个 Story 的用户价值陈述、FR 追溯、NFR 追溯齐备。

## 6. Summary and Recommendations

### Overall Readiness Status

# **GO**（可进入实施，附 2 项 Important 消解口径）

手机伴侣基建增量的 PRD ↔ 架构 ↔ Epic/Story 三方对齐度高：FR-40 六条、FR-41 在范围四条、FR-43 四条可测条款逐条落位；FR-42 DEFERRED 边界严格守住（仅接口预留薄故事，无任何推送实现内容）；架构 8 步序列与 Story 顺序一致且无前向依赖；四条硬边界、快照口径防呆、帧协议冻结、原型换装约束全部转化为可测验收条件；依赖入口全部落在桌面 Epic 1～11 已完成范围。**无 Critical 问题。** Story 12.1（风险出清点）可立即开工，两项 Important 分别在 12.2 / 12.4 开工前按下述口径消解即可，不构成 NO-GO 因素。

### 问题分级清单

#### 🔴 Critical（阻断实施）

无。

#### 🟠 Important（实施前/中须消解，均不改需求覆盖与故事逻辑）

1. **迁移编号冲突**：架构结构清单与 Story 12.2 AC 均指定 `migrations/029_paired_devices.sql`，但仓库已存在 `029_llm_config_network_location.sql` 与 `030_llm_provider_extended.sql`（2026-07-27 提交，早于架构增量定稿的 2026-08-25——即定稿时编号已陈旧）。**消解口径：实施时使用 `031_paired_devices.sql`（或开工当日下一个可用编号），把 AC 中的「029」读作陈旧编号而非契约**；涉及 architecture.md 结构清单、epics.md Additional #2 与 Story 12.2 AC 三处字面。SQLx 迁移版本号重复会硬失败，此项必须在 12.2 开工前对齐。
   > ✅ **已修复（2026-08-27，同日经 boss 确认）**：architecture.md 结构清单、epics.md Additional #2 与 Story 12.2 AC 三处字面已统一改为 `031_paired_devices.sql`（031 经核实未被占用）。
2. **加密硬边界 #1 措辞歧义**：架构原文「`crates/companion-proto` 是全仓唯一允许依赖 snow / noise-java 的位置」与其自身决策 #1（Rust: snow；**Android: noise-java**）、Android 包结构（pairing/ 含握手）及 Story 12.4（Android 端 noise-java 握手）字面冲突——Android 无法依赖 Rust crate，noise-java 必然出现在 Android 侧。**消解口径：以 Story 12.1 AC 的精确表述为准执行——Rust 侧 `snow` 仅存在于 `companion-proto/crypto.rs`；Android 侧 `noise-java` 限于 pairing/connection 换装层；两端 UI 与业务代码只见帧类型、不见密码学细节。** 架构意图（密码学收敛）在双端均保持，仅「全仓唯一」的字面表述不适用于跨语言场景。
   > ✅ **已修复（2026-08-27，同日经 boss 确认）**：architecture.md 硬边界 #1、目录树注释与 epics.md Additional #1 已改为双端分别收敛表述（Rust 侧 snow 仅 companion-proto；Android 侧 noise-java 限 pairing/connection 换装层）；Story 12.1 AC 原本即正确，未动。

#### 🟡 Nice-to-have（不阻塞，择机处理）

1. **PRD FR-41 锁屏条款未标注延后**：第 4 条「锁屏通知上的确认/拒绝操作等效」随 FR-42 降级一并延后，架构（「锁屏快捷确认/拒绝随 FR-42 一并延后」）与 epics（需求清单注记）均已显式记录，但 PRD 该条款本身无 DEFERRED 标记（FR-42 有完整降级块而此条没有）。建议 PRD 下次修订时补一行划线注记，消除 PRD↔epics 的轻微不对齐。
2. **`.decision-log.md` 头部元数据陈旧**：「条目数量：16」（实际 18 条，#17/#18 为 2026-08-25 追加）、「记录日期：2026-07-23」。内容本身完整准确，仅头部计数与日期未更新。
3. **Story 14.2 通知中心「敲门级确认/拒绝」指令接线建议显式化**：原型通知中心有敲门级确认/拒绝交互（NotificationDispatch.respond），14.2 AC 覆盖了 NOTICE 呈现与「样式与交互零回归」，但未显式声明该响应按钮经指令通道真实执行（13.3 已建立「建议确认/拒绝」COMMAND 能力，可直接复用）。建议 create-story 时补一句 AC，防止实现为仅本地状态翻转的「死按钮」。
4. **epics.md 增量尚未提交**：557 行纯追加仍为工作区未提交变更（其余三方文档已随 3a23ca4 提交）。建议尽快 commit 固化，避免与后续桌面实施工作相互漂移。

### 条件与建议执行顺序

1. **立即可做**：Story 12.1 开工（不受任何 Important 项影响）。
2. **12.2 开工前**：确认迁移编号按 I-1 口径执行（031 或下一可用号）；create-story 输出文件级任务清单。
3. **12.4 开工前**：确认 I-2 加密边界执行口径（Story 12.1 AC 表述）；create-story 设分阶段检查点。
4. **文档侧（无需阻塞实施）**：N-1/N-2 待下次修订 PRD 与决策日志时顺手清理；N-3 在 14.2 create-story 时补 AC；N-4 尽快提交 epics.md。
5. 桌面基线的两项前置条件（Story 10.2 拆分/检查点、FR-37～39 UX 增量规范）沿用 2026-07-23 报告结论，属桌面范围，不在本报告重复。

### Final Traceability Decision

| 需求 | PRD | Architecture | UX 事实源 | Epic/Story | 结论 |
|------|-----|--------------|-----------|------------|------|
| FR-40 | 完整（6 条款 + 2 假设） | 完整 | companion-android 原型 | E12 / 12.1–12.4，六条逐条落位 | 就绪 |
| FR-41 | 完整（第 4 条延后未标注，见 N-1） | 完整 | 原型 | E13 / 13.1–13.3，在范围四条逐条落位 | 就绪 |
| FR-42 | DEFERRED 块，与决策 #18 一致 | 降级记录 + 接口预留 | 原型（仅应用内语义） | E14 / 14.2 仅接口薄故事，边界守住 | 就绪（DEFERRED 口径三方一致） |
| FR-43 | 完整（4 条款 + 1 假设） | 完整 | 原型（降级遮罩 + 速记条） | E14 / 14.1，四条逐条落位 | 就绪 |
| 18+6 项移动呈现 | 分档表 | 四通道收敛声明 | 原型四 Tab + 二级页 | E13 承载（不逐 FR 拆，刻意设计） | 就绪 |
| NFR-M1～M7 / Additional 1～9 / UX-M1～M5 | — | 增量章节定义 | 原型 README | 覆盖核查表逐项落位 | 就绪 |

### Assessment Metadata

- 评估日期：2026-08-27
- 评估者：BMAD Implementation Readiness（增量模式）
- 桌面基线：沿用 implementation-readiness-report-2026-07-23.md（READY WITH CONDITIONS）
- 发现：Critical 0 项、Important 2 项、Nice-to-have 4 项；另核验通过项含 FR 逐条落位、序列对齐、硬边界、快照防呆、帧协议冻结、无前向依赖、换装约束、依赖入口八组重点核对
- 校验期间未修改 prd-egosync.md / architecture.md / epics.md 任何内容
