# Investigation: 移动端与桌面端一致性差距分析（改造 vs 重写裁决）

## Hand-off Brief

1. **What happened.** 移动端（companion-android）为「桌面端前端的移动适配」期望而建，但 24 个一/二档 FR 中仅 6 个完整实现、14 个完全缺失（58%），且图标层用 emoji 违反桌面明文「不使用彩色 emoji」约束——差异本质是功能覆盖缺口，非已实现屏的质量问题。
2. **Where the case stands.** 已 Confirmed（High）：在现有基础上改造、不重写——包结构/主题 token/导航壳/降级态均为可靠基础；但达成期望必然包含桌面侧 companion 模块（规格规定手机不直接复用桌面数据，须经 FR-41 桥接，桌面 companion_*.rs 全缺）。
3. **What's needed next.** 按三块局部重做（图标系统 / 三个未接线 ViewModel / 14+3 FR 补全）+ 桌面 companion 模块同步开发的分期方案推进；建议 `bmad-correct-course` 把「移动伴侣」从原型升级为正式 sprint。

## Case Info

| Field            | Value                                                                      |
| ---------------- | --------------------------------------------------------------------------- |
| Ticket           | N/A                                                                         |
| Date opened      | 2026-08-25                                                                  |
| Status           | Concluded                                                                  |
| System           | 桌面：React 18 + TS + Tailwind + Tauri 2/Rust；移动：Kotlin + Compose + Material3 |
| Evidence sources | 双端源码、git 历史、architecture.md、prd-egosync.md、用户提供的 39 FR 分档清单 |

## Problem Statement

用户原始陈述（作为假设处理）：「我本来构建移动端是为了给桌面端提供一个方便的入口，所以我期望移动端整体只是为了适配移动端的操作体验，对桌面端的前端进行适当的改造，但目前的移动端完全达不到这种期望。」

期望标准（用户给定）：
- 移动端实现第一档（18 个 FR）+ 第二档（6 个 FR）功能，以及手机端新增功能（FR-40/41/43，FR-42 已 DEFERRED）
- 一/二档功能除移动适配外，在功能、布局、图标、体验等方面与桌面端保持一致，「让用户感觉就是同一个产品」

## Evidence Inventory

| Source   | Status | Notes |
| -------- | ------ | ----- |
| companion-android/ 源码（30 个 Kotlin 文件，4,350 行 main 代码） | Available | 7 个功能屏：briefing / dashboard / review / settings / notify / chat / tasks + pairing/connection/sync 基建 |
| egosync-app/ 桌面端源码（11,847 行非测试 TSX + 17 services + 14 hooks + 完整 src-tauri Rust 后端） | Available | 组件域：butler/ role/ chat/ notifications/ settings/ onboarding/ modals/ layout/ |
| git 历史（companion-android） | Available | 仅 3 次提交：d88ef85 初版原型（纯前端+mock）、f63752a 对抗评审修复、107d906 设计语言对齐 |
| architecture.md §手机伴侣基建（L1926 起） | Available | 六个增量章节：技术栈/包结构/命名/反模式清单/FR-40~43 架构 |
| prd-egosync.md §4.14 + FR-40/41/43 | Available | 2026-08-25 更新；美学基调（专业有温度的老管家、深色默认、信息密度双模式、动效克制） |
| 桌面 Rust companion 模块（companion_pairing.rs 等，架构 L2140-2143 规定） | **Missing** | `find egosync-app -name "companion*.rs"` 为空 —— 桌面侧连接基建尚未开工 |
| 桌面端实际运行形态（构建产物/截图） | Missing | 判断「体验一致」需要 UI 证据，或以代码级比对替代 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 移动端 7 屏逐屏细读：功能完整度、数据来源（mock/snapshot）、交互 | High | Open | 与桌面对应组件比对 |
| 2 | 桌面端一/二档 FR 对应组件细读：ButlerView/DashboardTab/ChatStream/RoleView/NotificationPanel/WeeklyReviewModal/MemoryTab 等 | High | Open | 建立「桌面基准」 |
| 3 | 逐 FR 比对矩阵：24 个一/二档 FR ×（功能/布局/图标/体验）四维 | High | Open | 核心交付物 |
| 4 | PRD §4.14 + architecture 手机伴侣章节细读：规格与实现的偏差 | High | Open | 含美学基调符合度 |
| 5 | 设计语言比对：色彩 token / 字体 / 图标体系 / 深色模式 / 动效 | Medium | Open | commit 107d906 声称已对齐，需验证程度 |
| 6 | 缺失 FR 清单量化：一档 18 - 已有屏覆盖 = ？ | High | Open | 决定改造工作量 |
| 7 | 结构性判断：现有 Compose 架构（无 ViewModel 的 Screen、AppModelContainer 手工 DI、SnapshotStore mock）能否承载对等改造 | High | Open | 决定改造 vs 重写 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 2026-08-25 前 | 桌面端完整成型（Rust 后端 + 前端全功能） | 仓库结构 | Confirmed |
| d88ef85 | 移动端初版：「手机伴侣 Compose 高保真前端原型（纯前端 + mock）」 | git log | Confirmed |
| f63752a | 移动端三路对抗评审修复 | git log | Confirmed |
| 107d906 | 移动端「设计语言对齐桌面端母本——色彩 token/Shapes/呼吸节奏/气泡与徽章语义」 | git log | Confirmed |
| 2026-08-25 | PRD 更新 §4.14 手机伴侣章节；用户提出期望落差 | PRD L21、用户陈述 | Confirmed / 假设待证 |

## Confirmed Findings

### Finding 1: 移动端是「纯前端 + mock」原型，非连接桌面引擎的伴侣实现

**Evidence:** git commit d88ef85「手机伴侣 Compose 高保真前端原型（纯前端 + mock）」；`companion-android/app/build.gradle.kts` 依赖仅 Compose BOM/Material3/Navigation/Coroutines（符合原型硬性边界）；`connection/FakeConnectionClient.kt` 存在且无真实实现。

**Detail:** 移动端从未接入真实连接层——这与「纯前端原型」的硬性边界一致，但意味着用户期望中「桌面端前端的移动适配」在数据与功能纵深上天然受限。

### Finding 2: 桌面端是功能完整的全栈应用

**Evidence:** `egosync-app/src-tauri/src/` 含 commands/db/services/llm/models 完整 Rust 后端；前端 11,847 行非测试 TSX，17 个 service、14 个 hook；依赖含 react-markdown（对话渲染）、@dnd-kit（拖拽排序）等移动端没有的能力。

**Detail:** 桌面端组件域覆盖 butler/role/chat/notifications/settings/onboarding/modals/layout，远多于移动端 7 屏。

### Finding 3: 架构规定的桌面 Rust companion 模块未实现

**Evidence:** architecture.md L2140-2143 规定 `services/companion_pairing.rs / companion_connection.rs / companion_snapshot.rs / companion_dispatch.rs`；`find egosync-app -name "companion*.rs"` 返回空。

**Detail:** FR-40/41 的桌面侧（配对、连接、快照下发、指令分发）完全未开工。移动端达成「伴侣」期望需要桌面侧配合，这是移动端单独无法闭环的部分。

### Finding 4: 移动端代码量约为桌面端的 37%，且屏数远少于桌面组件域

**Evidence:** 移动 main 源码 4,350 行（30 文件）vs 桌面非测试 TSX 11,847 行（另有 17 services + 14 hooks + Rust 后端不在计数内）。

**Detail:** 数量级差距表明移动端覆盖的功能面显著小于桌面端；具体缺哪些 FR 待 backlog #3/#6 量化。

### Finding 5: 移动端用 emoji 图标，直接违反桌面端明文设计约束「不使用彩色 emoji」

**Evidence:** `egosync-app/src/lib/roleIcons.ts:8` 注释明文「全部采用 Lucide React 的【黑白线框】图标（stroke-only），不使用彩色 emoji」；白名单 24 个 lucide 线性图标（`roleIcons.ts:56-81`）+ 8 品牌色（`:83-92`），并要求与后端 `agent_engine.rs` 的 SUPPORTED_ICONS/SUPPORTED_COLORS 同步。移动端 `AppNavHost.kt:50-55` 底部导航用 emoji（💬📋📊👤），注释 `:47`「emoji 图标：与角色卡图标风格一致」——即角色卡也用 emoji。

**Detail:** 这不是「适配移动做的合理改造」，而是与桌面明文设计约束正面冲突。桌面设计意图是专业线性图标体系（含后端白名单校验），移动改用 emoji 使「同一个产品」感受断裂。图标一致性裁决成立：移动端图标层需推翻重做以对齐 lucide 白名单体系（或映射到 Compose Material Icons 等效线性图标）。

### Finding 6: 移动端多数 ViewModel 未接入连接层，承接 mock 数据自循环——「换实现零改动」说法不成立

**Evidence:** `AppNavHost.kt:197` `ChatViewModel()`、`:212` `TasksViewModel()`、`:226` `DashboardViewModel()` 均无参构造，未注入 container；仅 SettingsViewModel(:244)、NotificationCenterRoute(:276)、DegradedOverlay(:101) 读 `container.connection`/`container.notifications`。`AppModelContainer.kt:25` `connection = FakeConnectionClient(...)` 是唯一连接实现；注释 L18-19 称「接入真实连接层时只需替换 connection 实现，其余代码零改动」。

**Detail:** 实际上 chat/tasks/dashboard 三屏的 ViewModel 与连接层完全解耦、各自硬编码 mock 状态。所谓「零改动」仅对 Settings/Notifications/降级遮罩成立。改造到真实数据需为每个 ViewModel 重接数据源（SnapshotStore/指令通道），属增量改造而非零改动，但仍非推倒重写。

### Finding 7: 规格规定移动端不直接复用桌面数据/逻辑——达成期望必然包含桌面侧 companion 模块

**Evidence:** `prd-egosync.md:602`「桌面保持唯一事实源」；`architecture.md:1958`「手机无业务库主权，只有快照缓存」；`:2036`「手机永不直接触达 agent_bridge/opencode/DB」；`:1939`「一档18项+二档6项的移动呈现，架构含义全部收敛到四条通道（连接/状态/指令/推送），不逐 FR 设计」。

**Detail:** 移动获取数据唯一路径 = FR-41 状态同步桥接（SNAPSHOT/STATE_DELTA 帧）；指令执行唯一路径 = COMMAND→companion_dispatch→桌面 services。因此「改造到期望」工作量必然包含桌面侧 `companion_pairing/connection/snapshot/dispatch.rs` 四文件 + `paired_devices` 表 + `crates/companion-proto`。纯移动端改造无法独立闭环。

### Finding 8: 规格未逐 FR 细化移动交互形态——用户「严格逐项一致」是高于规格的自定标准

**Evidence:** `architecture.md:1939` 移动呈现「不逐 FR 设计」，仅规定分档归属 + 通道收敛；规格允许语义级移动适配（如列表替侧栏）。

**Detail:** 用户在本次调查中明确要求「严格逐项一致（布局/图标/交互可逐项映射到桌面实现，仅做移动适配）」，此标准高于规格。两者不冲突但需显式记录：裁决与方案以用户高标准为准，但须告知用户该标准严于规格默认，部分「语义适配」若被用户接受可显著降低工作量。

### Finding 9: 24 个一/二档 FR 的移动覆盖矩阵——仅 6 个完整、14 个完全缺失（核心差异定位）

**Evidence:** 子代理深读移动 7 屏 + SnapshotStore 后统计（详见下方矩阵，每条 path:line）。

**Detail:** 覆盖分布：完整 6 / 部分 3 / 仅样例 1 / 缺失 14。即 ~25% 完整、~58% 完全缺失。差距本质是**功能覆盖缺口**，不是已实现屏的质量问题——已实现的 6 个 FR 交互基本等效桌面（待 A 子代理桌面基准确认图标/布局细节）。

#### 24 FR 移动覆盖矩阵（移动端视角）

| 状态 | FR | 移动位置 |
| --- | --- | --- |
| ✅完整 | FR-3 管家语调 | `SnapshotStore.kt:343,348-353`（文本层，缺语调配置项） |
| ✅完整 | FR-11 主动建议确认/拒绝 | `ChatScreen.kt:232`；`BriefingScreen.kt:141`（缺第三态"稍后再提醒"） |
| ✅完整 | FR-16 晨间简报 | `BriefingScreen.kt:43`（缺生成时间/刷新语义） |
| ✅完整 | FR-18 周复盘成绩单 | `WeeklyReviewScreen.kt:49`（图表为 Canvas 静态柱） |
| ✅完整 | FR-19 角色卡片仪表盘 | `DashboardScreen.kt:196`（Pager 替网格，呼吸动效对齐） |
| ✅完整 | FR-22 三级通知 | `NotificationCenterScreen.kt:51`；`SettingsScreen.kt:121`（仅应用内，无系统推送，符合 FR-42 DEFERRED） |
| ◐部分 | FR-17 大石头规划 | `TasksScreen.kt:173` 徽章；`WeeklyReviewScreen.kt:161`（只展示，无规划/排期交互） |
| ◐部分 | FR-23 自动四象限分类 | `TasksScreen.kt:66`（展示已分类结果，无分类过程/纠正交互） |
| ◐部分 | FR-33 Agent Loop 对话 | `ChatViewModel.kt:55-109`（有流式打字机+思考态，无执行过程/工具调用可视） |
| ▽仅样例 | FR-14 冲突检测 | `SnapshotStore.kt:292`（有冲突文案 knock 样例，无检测/呈现逻辑） |
| ❌缺失 | FR-1 意图解析路由 | —（仅管家单口回复，无意图识别/路由到角色） |
| ❌缺失 | FR-2 对话分配任务 | —（无对话内创建/分配任务交互） |
| ❌缺失 | FR-12 主动性刻度盘 | —（无主动性强度设置控件） |
| ❌缺失 | FR-15 三步仲裁 | — |
| ❌缺失 | FR-20 对话区角色切换 | —（对话仅管家单一身份） |
| ❌缺失 | FR-21 空状态引导 | —（无空状态引导组件） |
| ❌缺失 | FR-24 Q2 保护提醒 | —（仅 Q2 分组着色，无提醒） |
| ❌缺失 | FR-29 推理溯源 | —（气泡无溯源/依据展示） |
| ❌缺失 | FR-30 不确定性表达 | — |
| ❌缺失 | FR-38 仪表盘统计筛选 | —（无时间/角色筛选控件） |
| ❌缺失 | FR-5 角色涌现确认 | — |
| ❌缺失 | FR-7 记忆自动提炼 | —（仅角色卡显示 memoryCount 数字 `DashboardScreen.kt:249`） |
| ❌缺失 | FR-8 记忆查询溯源 | — |
| ❌缺失 | FR-9 选择性遗忘 | — |

### Finding 10: 主题色彩 token 已与桌面 CSS 变量语义映射并固化——支持「改造」而非「重写」

**Evidence:** `Color.kt:10-26` token 与桌面 `index.css` CSS 变量一一命名映射（DarkBackground↔--bg-base、DarkSurface↔--bg-surface、DarkOutline↔--border-default 等）；强调色直接取桌面实际色值（indigo #6366F1、amber #F59E0B、功能四色）；能量三档分界 ≥70/≥40/<40（`Color.kt:54-58`）；深色 primary 用 indigo 实色而非 M3 惯例亮色，`Theme.kt:11-13` 注释明示「以桌面母本为准」。

**Detail:** 色彩 token 层的设计语言对齐（commit 107d906）是真实有效的工作，非口头声称。语义映射关系已固化。**结论：主题体系无需重写，是改造的可靠基础。** 唯一视觉硬伤是图标层（emoji 违反桌面 lucide 白名单约束，见 Finding 5）。

## Deduced Conclusions

### Deduction 1: 「体验不一致」的根因大概率是覆盖面差距 + mock 数据，而非单纯视觉走样

**Based on:** Finding 1、2、4

**Reasoning:** 移动端仅 3 次提交、7 屏、mock 数据；桌面端全栈全功能。若 24 个一/二档 FR 中有相当比例在移动端没有对应界面，则「不是同一个产品」的感受来自功能缺口本身；同时 commit 107d906 声称已做设计语言对齐，视觉层面可能已有部分基础。待 FR 矩阵验证。

**Conclusion:** 差距分析必须先量化「缺失的功能面」，再评估「已有屏的保真度」。

## Hypothesized Paths

### Hypothesis 1: 移动端与桌面端的差距主要来自功能覆盖缺口（而非已实现部分的质量）

**Status:** Confirmed

**Resolution:** B 子代理 24 FR 矩阵证实：完整 6 / 部分 3 / 仅样例 1 / 缺失 14（58% 完全缺失，>50% 阈值达成）。差距本质是覆盖缺口；已实现 6 个 FR 交互基本等效桌面。

**Theory:** 24 个一/二档 FR 中，移动端只覆盖了一小部分（估计 chat/tasks/briefing/dashboard/review/notify/settings 屏对应约 8-10 个 FR），其余完全缺失；已实现屏也存在交互/信息密度降级。

**Supporting indicators:** 屏数 vs 组件域数量差距（7 vs 8 个域、每个域多组件）；无 onboarding、无 role 详情、无 memory 查询溯源、无冲突仲裁、无主动建议确认流等桌面核心交互的对应屏。

**Would confirm:** 逐 FR 矩阵显示 >50% 一档 FR 在移动端无对应实现。

**Would refute:** 矩阵显示大部分 FR 已有对应实现且保真度高，差距集中在视觉/交互细节。

### Hypothesis 2: 现有 Compose 原型的架构基础可以承载改造（无需重写）

**Status:** Confirmed

**Resolution:** 包结构符合规范、主题 token 已对齐桌面 CSS 变量并固化（Finding 10）、导航壳/降级态/配对基建成型；仅需局部重做三块（图标系统、三个未接线的 ViewModel、14+3 缺失/部分 FR 补全）。裁决：在现有基础上改造，不重写。refute 条件（mock 耦合进 UI/无法替换 ConnectionClient）未发生——Container 已隔离 Fake，仅 3 个 VM 需重接线。

**Theory:** 包结构、主题体系、导航结构符合架构规范，且已做过设计语言对齐；缺失的是功能面与数据接入，属于「增量开发」而非「推倒重来」。

**Supporting indicators:** 包结构完全遵循 architecture.md 规定（pairing/connection/sync/notify/ui/<feature>/theme）；依赖边界干净（无违规运行依赖）。

**Would confirm:** 深读后确认各 Screen/ViewModel 结构清晰、主题 token 与桌面 Tailwind token 有可延续的映射关系。

**Would refute:** 深读发现结构性问题（如 mock 数据耦合进 UI、无法替换为真实 ConnectionClient、主题体系与桌面 token 语义冲突需推翻）。

### Hypothesis 3: 达成期望的路径上，桌面侧 companion 模块缺失是移动端无法独立解决的前置依赖

**Status:** Confirmed

**Resolution:** C 子代理证实：规格规定桌面是唯一事实源、手机不直接复用桌面数据/逻辑（`prd-egosync.md:602`、`architecture.md:1958/2036`）；移动获取数据唯一路径 = FR-41 状态同步桥接（SNAPSHOT/STATE_DELTA 帧），指令执行唯一路径 = COMMAND→companion_dispatch→桌面 services。达成期望必然包含桌面 companion 模块四文件 + proto + paired_devices 表。

**Theory:** 用户期望「同一个产品」的体验依赖 FR-41 实时同步；移动端即使 UI 全部对齐，没有桌面 companion 模块就无法脱离 mock。裁决「改造 vs 重写」时必须把这部分工作量计入方案。

**Supporting indicators:** Finding 3。

**Would confirm:** PRD/architecture 细读确认 FR-41 是一/二档功能在移动端「可用」的前提。

**Would refute:** 规格显示一/二档功能可在纯快照模式下完整体验（无实时指令回流需求）。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 逐 FR 的移动端实现状态矩阵 | 直接回答「差异在哪里」 | 深读移动端 7 屏 + 桌面端对应组件（backlog #1-#3） |
| 已有屏的保真度细节（布局/图标/交互 vs 桌面） | 回答「体验一致性」程度 | 同上，四维比对 |
| 桌面端 UI 实际视觉证据 | 图标/布局比对精度 | 代码级比对（lucide-react 图标名 vs Compose Material Icons 映射）已可执行；截图非必需 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | 不适用（差距分析型调查，非缺陷） |
| Trigger | 用户期望（桌面端移动适配 + 一致体验）vs 移动端现状（纯前端原型） |
| Condition | 移动 7 屏 + 30 文件覆盖 6/24 FR；桌面 11,847 行 TSX + Rust 后端覆盖全量；桌面 companion_*.rs 全缺 |
| Related files | 桌面：`egosync-app/src/{components,services,hooks,lib/roleIcons.ts}`；移动：`companion-android/app/src/main/java/com/egosync/companion/{ui,pairing,connection,sync,notify}` |

## Conclusion

**Confidence:** High（诊断）/ Medium（方案落地，因桌面 companion 模块工作量未被本仓库既有代码佐证，依赖架构规格推断）

**诊断（Confirmed）**：移动端与桌面端的差距，根因是**功能覆盖缺口**（24 个一/二档 FR 中 14 个完全缺失、3 个仅部分、1 个仅样例），而非已实现屏的质量问题。已实现的 6 个 FR（管家语调/主动建议/晨间简报/周复盘/角色仪表盘/三级通知）交互基本等效桌面。

**结构性事实（Confirmed）**：
- 移动端主题色彩 token 已与桌面 CSS 变量语义映射并固化（`Color.kt:10-26`），是改造的可靠基础。
- 图标层用 emoji 直接违反桌面 `roleIcons.ts:8` 明文「不使用彩色 emoji」约束——显式冲突。
- chat/tasks/dashboard 三个 ViewModel 硬编码 mock，未接连接层，「换实现零改动」不成立。
- 规格规定手机不直接复用桌面数据/逻辑，须经 FR-41 桥接——**桌面 companion 模块是达成期望的硬前置**，当前全缺。

**裁决**：**在现有基础上改造，不重新开发。** 现有 Compose 原型的包结构、主题体系、导航壳、降级态基建是可延续的可靠骨架；缺失的是「功能面 + 数据接入 + 图标对齐 + 桌面侧连接基建」，属增量改造而非推倒重来。

## Recommended Next Steps

### 裁决：在现有基础上改造，不重新开发

依据：包结构符合架构规范、主题 token 已对齐桌面母本（Finding 10）、导航壳/降级态/配对基建成型、依赖边界干净；缺口是「没做」而非「架构做错」。

### A. 三块局部重做（移动端，不触及整体架构）

1. **图标系统重做**——emoji 全量替换为对齐桌面 lucide 白名单的线性图标体系。
   - 桌面白名单：`roleIcons.ts:56-81`（24 角色图标，stroke-only）+ 8 品牌色（`:83-92`），须与后端 `agent_engine.rs` SUPPORTED_ICONS 同步。
   - 移动落地方案二选一：(a) Compose Material Icons Extended 取等效线性图标 + 保留 `iconId`(kebab) 语义映射；或 (b) 自绘 lucide SVG（保证与桌面像素级一致，工作量更大）。建议 (a)。
   - 影响范围：`AppNavHost.kt:50-54`（导航 emoji）、角色卡、ActionCard、通知分组等全量 emoji 处。

2. **三个未接线 ViewModel 重接数据源**——chat/tasks/dashboard 从硬编码 SnapshotStore mock → 经 AppModelContainer 接 SnapshotStore（先只读快照）→ 后接 COMMAND 指令通道。
   - 证据：`AppNavHost.kt:197/212/226` 三 VM 无参构造；`ChatViewModel.kt:18,71` 等硬编码 mock。
   - 改造后 `AppModelContainer.kt:18-19` 的「换 connection 实现零改动」才真正成立（届时再加真实 ConnectionClient）。

3. **设计语言补齐**——信息密度双模式（对话流轻量 / 仪表盘密集）、动效克制（仅角色卡呼吸 `--breath-duration:3s`）、色温随角色偏移（工作冷/家庭暖）、`prefers-reduced-motion` 降为 0ms。色彩已对齐，补的是密度/动效/色温三件。

### B. 14 缺失 + 3 部分 FR 补全（逐 FR：桌面基线 → 移动需新增）

| FR | 桌面基线(path:line) | 移动需新增/补全 |
| --- | --- | --- |
| FR-1 意图解析路由 | `chatService.ts:5` routingMetadata；`ChatStream.tsx:551` 委派两段气泡 | ChatViewModel 接指令通道，渲染「委派中→角色回复」两段气泡 |
| FR-2 对话分配任务 | `TaskDecompositionCard.tsx:12-85`（ListTree）对话流内嵌拆分提案卡 | ChatScreen 内嵌 TaskDecompositionCard + 接受/不要拆分 |
| FR-12 主动性刻度盘 | `ProactivityToggle.tsx:16-37` 三档分段 | 角色/设置页加三档分段控件 |
| FR-20 对话区角色切换 | `Sidebar.tsx:6-144` 左 64px 图标栏（Home/Plus/Moon…） | 顶部或抽屉式角色切换器（移动无侧栏，**语义适配点**） |
| FR-21 空状态引导 | `OnboardingView.tsx:24-386` + `RoleConfirmModal.tsx:47` | OnboardingScreen（未配对/无角色时引导） |
| FR-24 Q2 保护提醒 | `App.tsx:87` q2:reminder 事件；`TaskOverviewTab.tsx:115` amber 徽章+左边框（AlertTriangle） | TasksScreen 任务卡加 Q2 徽章 + 通知中心承载提醒 |
| FR-29 推理溯源 | `ChatBubble.tsx:236` ExecutionTrace 可折叠（ChevronRight） | ChatBubble 加可折叠执行过程区（Think/Narration/Action） |
| FR-30 不确定性表达 | `ButlerSettingsContent.tsx:1457` confidence<0.7「仅供参考」 | 对应内联文本（桌面徽章已移除，对齐即可） |
| FR-38 仪表盘统计筛选 | `DashboardTab.tsx:85-228` DateRangeFilter + scope 下拉 + 4 指标卡（CalendarDays） | DashboardScreen 加筛选行 + 指标网格 |
| FR-5 角色涌现确认 | `RoleConfirmModal.tsx:47-269` 图标 24 选+色板 8 选（X） | 角色涌现确认屏（引导阶段） |
| FR-8 记忆查询溯源 | `MemoryTab.tsx:129-158` toggleSource + 来源消息列表（Clock） | 角色详情/记忆屏加来源展开区 |
| FR-9 选择性遗忘 | `MemoryTab.tsx:160-215` 遗忘确认流（确认/再想想） | 记忆屏加遗忘确认流 |
| FR-17 大石头规划（部分→完整） | `WeeklyReviewModal.tsx:181-262` plan 阶段（Check/Plus/X） | WeeklyReviewScreen 加规划阶段交互（当前只读） |
| FR-23 自动四象限分类（部分→完整） | `useTasks.ts:29` task:classified + 「智能分类中…」徽章（Filter/Loader2） | TasksScreen 加分类中态 + 象限筛选 |
| FR-33 Agent Loop 对话（部分→完整） | `ChatStream.tsx:529` 流式+thinking+工具执行+停止（Play/Square） | ChatScreen 加工具执行过程可视（@Skill 工作目录属三档桌面优先，不进移动） |

### C. 桌面侧 companion 模块同步开发（硬前置，规格强制）

规格规定手机不直接复用桌面数据/逻辑（`prd-egosync.md:602`、`architecture.md:1958/2036`），须经 FR-41 桥接。以下桌面侧文件全缺（`find companion*.rs` 为空），是移动端脱离 mock 的必要前置：

- `services/companion_pairing.rs` — QR 生成、Noise XX 握手、paired_devices 读写
- `services/companion_connection.rs` — NSD 广播、WS 监听、连接状态机、直连↔中继切换
- `services/companion_snapshot.rs` — 快照节流重建、SNAPSHOT 帧下发
- `services/companion_dispatch.rs` — COMMAND 解析→现有 services 调用→RESULT 回流 + NotificationDispatch 抽象
- `commands/companion.rs` — `pairing_*`/`paired_device_*`/`companion_get_status`
- `crates/companion-proto/` — 帧协议（SNAPSHOT/STATE_DELTA/COMMAND/COMMAND_RESULT/STREAM_TOKEN）
- `paired_devices` 表 + migration
- 桌面 Tauri Events `companion:` 命名空间（`companion:paired/connected/disconnected`、`quicknote:submitted`）

> 边界遵守：纯前端原型阶段移动侧仍只建接口 + Fake（不实现网络/Noise/NSD/FCM/Room/Hilt/OkHttp，见 architecture 反模式清单 L2190-2195）。真实连接能力随桌面 companion 模块落地而开关。

### D. 「严格逐项一致」的边界冲突（需用户裁决，规则七）

用户要求「严格逐项一致」，但证据显示三个 FR 在桌面端也无独立 UI，无法逐项镜像：
- **FR-14 冲突检测**：桌面「未找到独立 UI」（`ButlerSettingsContent.tsx:713` 仅使命宣言 placeholder 提及），靠后端使命宣言驱动；移动仅样例数据。
- **FR-15 三步仲裁**：桌面「未找到」UI，纯后端逻辑。
- **FR-7 记忆自动提炼**：桌面「未找到前端独立组件」（后端自动提炼，结果在 MemoryTab 展示）。

对这三项，「逐项一致」无对象可对——需桌面+移动联合设计后端事件呈现（如冲突/仲裁经 knock 通知承载，移动 NotificationCenter 已具备承载位）。另：FR-20（侧栏→移动）与 FR-21（全屏引导→移动）因移动无侧栏/尺寸限制，**强制语义适配**而非逐项一致。

**建议**：对 D 类采用「语义一致」（信息架构/语义对齐即可），其余 18 项维持「严格逐项一致」。若用户接受，可显著降低工作量。

### 分期落地（建议）

1. **Phase 0｜对齐设计语言**：A1 图标系统重做 + A3 密度/动效/色温补齐。纯移动、不依赖桌面侧，可立即开工。
2. **Phase 1｜移动端功能补全**：A2 ViewModel 重接线（接 SnapshotStore 只读）+ B 的 14+3 FR 屏补全。仍跑在 mock/快照上，但 UI 与桌面逐项对齐。
3. **Phase 2｜桌面 companion 模块**：C 全套落地（pairing/connection/snapshot/dispatch/proto/migration）。移动侧 ConnectionClient 由 Fake 切真实。
4. **Phase 3｜指令闭环**：移动 COMMAND 通道接通，速记排队/降级态实战验证（FR-43）。

### Diagnostic（验证改造完成度）

- 图标一致性：`grep emoji` 全移动源应为 0；角色图标 id 覆盖桌面 24 白名单。
- FR 覆盖率：重跑 24 FR 矩阵，完整数应 ≥ 21（除 FR-14/15/7 三项后端驱动）。
- 数据接线：`AppNavHost.kt` 三 VM 构造处应注入 container；`ChatViewModel` 不再硬编码 mock。
- 桌面 companion：`find companion*.rs` 应返回 4 文件 + proto crate。

## Reproduction Plan

不适用（非缺陷复现）。验证计划：以逐 FR 矩阵为交付物，用户可按矩阵抽查任意 FR 的双端实现对照。

## Side Findings

- 移动端依赖完全符合原型硬性边界（无 Room/Hilt/OkHttp/网络实现）——架构纪律执行良好（证据：app/build.gradle.kts）。
- 桌面端对 FR-14/FR-15/FR-7 无独立前端 UI（`ButlerSettingsContent.tsx:713` 仅 placeholder 提及仲裁），这三项靠后端逻辑驱动、前端隐式呈现——移动「缺失」无桌面 UI 可逐项镜像。
- 桌面深色模式已实现（`App.tsx:49-53/324-331`，`dark:` variant 656 处），移动深色亦默认（`AppModelContainer.kt:30`）——双端深色一致性基础良好。
- 移动 `DashboardScreen` 用 HorizontalPager 横滑角色卡替代桌面 `DashboardTab.tsx:253` 垂直滚动卡片流，属合理移动语义适配，呼吸动效已对齐。
- 桌面最常用 15 个 lucide 图标：X/Check/ChevronDown/ChevronRight/Trash2/Home/Plus/Bell/AlertTriangle/Clock/ListTodo/BrainCircuit/Sliders/Circle/CheckCircle2——移动图标重做时需逐个找 Compose 等效映射。

## Follow-up: 2026-08-25

### New Evidence

用户裁决（D 边界冲突）：
- **18 项维持「严格逐项一致」**：除 D 类外的 FR-1/2/3/5/8/9/11/12/16/17/18/19/20/21/22/23/24/29/30/33/38（功能/布局/图标/交互可逐项映射桌面，仅做移动适配）。
- **3 项采用「语义一致」**：FR-14 冲突检测、FR-15 三步仲裁、FR-7 记忆自动提炼——桌面无独立 UI，经后端事件 → knock 通知/对话承载。
- **2 项强制语义适配**：FR-20（侧栏→抽屉/顶部切换器）、FR-21（全屏引导→移动引导屏）因移动尺寸限制。

### Updated Conclusion

裁决与方案据此定稿，无未决项。Case 状态：**Concluded**。最高价值后续动作：`bmad-correct-course`（移动伴侣从原型升级为正式对等产品的 sprint 修订）或 `bmad-create-story`（先拆 Phase 0 图标系统重做 + 设计语言补齐，无桌面依赖可立即开工）。
