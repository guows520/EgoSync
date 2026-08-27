# Investigation: companion-android 四项移动端问题（横幅/图标/任务/记忆）

## Hand-off Brief

1. **What happened.** 用户报告移动端 4 项问题；经查 ①②③ 证实（横幅为 FR-40 设计特性需删除决策、图标从未与桌面同步、任务缺归属字段与新建能力），④ 与前提矛盾——记忆查看/遗忘已完整实现且两层可达，疑为旧 APK 观察或入口未发现。
2. **Where the case stands.** Concluded（High 置信）：根因全部定性，唯一未决项是用户 APK 构建版本（决定④走"重建验证"还是"改信息架构"）与横幅删除范围（整组件 vs 仅 direct 隐藏）。
3. **What's needed next.** 重建最新 APK 验证④；确认横幅删除范围与图标背景色后，①②③可交 bmad-quick-dev 按案件 Fix direction 执行。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A（用户自由文本输入）                                                     |
| Date opened      | 2026-05-27                                                                 |
| Status           | Concluded（待执行）                                                        |
| System           | companion-android（Compose 原型，纯前端 + mock，AGP 9.3.2 / Kotlin 2.4.10） |
| Evidence sources | 源码、git log、README、res 资源清单、桌面端 icons 目录                     |

## Problem Statement

用户原话（视为假设）：

> - 页面顶部的"局域网直连 ……"的那个绿色块全部去掉
> - 应用的图标要替换成桌面版对应的图标
> - 任务
>   - 任务应支持按管家或角色筛选
>   - 应支持新建任务
> - 记忆
>   - 应该支持记忆的查看和遗忘操作

用户明确：先分析原因和方案，暂不直接执行。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| companion-android 源码 | Available | 48 个 .kt 文件已编目 |
| git log（companion-android） | Available | 13 个提交，d88ef85 初始 → e6dba8a 最新 |
| companion-android/README.md | Available | 页面地图未含 memory/onboarding 屏（文档滞后于代码） |
| AndroidManifest.xml | Available | `android:icon="@mipmap/ic_launcher"` |
| res/ 资源清单 | Available | 仅模板 ic_launcher（无 PNG mipmap-*dpi） |
| 桌面端 icons 目录 | Available | src-tauri/icons/ 全套 + android/mipmap-* 现成 |
| 桌面端任务/记忆母本代码 | Partial | subagent 调查中 |
| 用户实际安装的 APK 构建版本 | Missing | 无法确认用户观察基于哪个 commit 的构建 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 记忆功能全貌（Screen/VM/Store/入口可达性） | High | Done | subagent 已返回，见 Finding 6 |
| 2 | 任务功能全貌 + 桌面端筛选/新建母本对照 | High | Done | subagent 已返回，见 Finding 8 |
| 3 | 横幅删除方案评估（FR-40 语义、DegradedOverlay 独立性） | High | Done（初步） | 挂载点已确认 |
| 4 | 图标替换方案（桌面 mipmap → companion res） | Medium | Done（初步） | 资源已就位 |
| 5 | 用户 APK 构建版本确认 | High | Open | 需用户提供或对比构建时间 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| d88ef85 | 手机伴侣 Compose 高保真原型初版 | git log | Confirmed |
| f6fb40c | role/memory 屏落地组 3：记忆二级屏 + 选择性遗忘确认流 | git log | Confirmed |
| eca358f | tasks 屏落地组 4 FR-23：分类中态 + 象限筛选 | git log | Confirmed |
| e6dba8a | 最新提交：onboarding 屏 FR-21 | git log | Confirmed |
| 2026-05-27 | 用户报告 4 项问题（含"记忆应支持查看和遗忘"） | 用户输入 | Confirmed |

## Confirmed Findings

### Finding 1: 三态连接横幅的实现与挂载

**Evidence:** companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionStatusBanner.kt:36-42；AppNavHost.kt:213

**Detail:** `ConnectionStatusBanner` 是 PRD FR-40 的三态横幅（direct 绿"局域网直连"/relay 蓝/offline 红），在 `MainShellRoute` 的 `Scaffold.topBar` 挂载（AppNavHost.kt:213）。原型默认 mock 状态为 Direct，故用户顶部常驻看到绿色块。

### Finding 2: 移动端图标为 AGP 模板默认图标

**Evidence:** companion-android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml（仅 adaptive XML）+ res/drawable/ic_launcher_foreground.xml（模板 vector）；AndroidManifest.xml `android:icon="@mipmap/ic_launcher"`

**Detail:** res/ 下无任何 mipmap-*dpi PNG 目录；ic_launcher_foreground.xml 为脚手架模板矢量图。桌面端 `egosync-app/src-tauri/icons/` 拥有完整图标集（icon.png/ico/icns），且 `icons/android/mipmap-{mdpi..xxxhdpi}/` 已含 ic_launcher.png、ic_launcher_round.png、ic_launcher_foreground.png 与 values/ic_launcher_background.xml——替换资源已存在，只差搬运。

### Finding 3: 任务屏现有筛选仅为象限维度

**Evidence:** AppNavHost.kt:281-286（TasksRoute 仅传 onToggleTask + onQuadrantFilterSelected）；commit eca358f（FR-23 象限筛选）

**Detail:** 任务 Tab 现有四象限分组 + 象限筛选行；无按管家/角色筛选维度，无新建任务入口（待 subagent 确认细节与桌面母本对照）。

### Finding 4: 记忆屏（查看 + 遗忘确认流）已在代码中落地并接入导航

**Evidence:** AppNavHost.kt:121-130（MEMORY 路由，注释"FR-8/9 记忆屏：按角色进入（仪表盘角色卡「查看记忆」入口）"）、AppNavHost.kt:374-388（MemoryRoute 含 onOpenForgetConfirm/onCancelForget/onConfirmForget 回调）；commit f6fb40c（"记忆来源溯源展开/选择性遗忘确认流"）

**Detail:** 与用户前提④"应该支持记忆的查看和遗忘操作"存在矛盾——功能代码已存在。用户观察可能基于旧 APK（f6fb40c 之前），或入口太深未被发现（仪表盘→角色卡→查看记忆）。README 页面地图未记载记忆屏，佐证文档滞后。

### Finding 5: 降级遮罩独立于横幅存在

**Evidence:** AppNavHost.kt:133-139（DegradedOverlayHost 在 Offline 时全局覆盖）

**Detail:** 离线明示由 DegradedOverlay 遮罩独立承担，不依赖横幅。删除横幅不会使离线状态失去提示；主要损失是 direct/relay 常驻状态显示与 offline 时 topBar 的红色提示。

### Finding 6: 记忆功能已完整实现（查看 + 遗忘确认流），唯一入口为仪表盘角色卡

**Evidence:** MemoryViewModel.kt:47-52（按角色加载）、MemoryScreen.kt:92-103（类别筛选）、MemoryScreen.kt:229-288（遗忘三段式确认流）、DashboardScreen.kt:335-366（"查看记忆"入口）、SnapshotStore.kt:224-268/675-724（数据模型与 mock）

**Detail:** 功能全貌（subagent 确认）：
- **查看**：按 roleId 加载、四类别筛选 chips（task_status 镜像桌面被过滤不显示）、来源对话溯源展开（isSource 高亮 + 空来源"已不可用"分支）、空态文案、ISO 时间格式化——全部实现。
- **遗忘**：完整三段式确认流（红色"遗忘"钮→卡内嵌确认面板"确定要忘记这条吗？…原始对话还会留在历史里。"→确认/取消），文案逐字镜像桌面。
- **入口**：唯一入口在仪表盘 Tab → 角色卡 → "查看记忆"行，两层可达；聊天/任务/通知等其余界面均无 memory 入口。
- **数据层限制**：遗忘仅从 ViewModel 内存列表 filterNot 移除（MemoryViewModel.kt:83-91），不触碰 SnapshotStore 静态数据——重进页面/重启进程即还原（原型"只读渲染、桌面为唯一事实源"的既定语义，注释自认接入真实连接层后改 COMMAND 帧驱动）。

**与用户前提的矛盾结论**：④"应该支持记忆的查看和遗忘操作"在 UI 层面已 100% 实现。用户观察缺失的最可能解释：旧 APK 构建（f6fb40c 之前）或未发现仪表盘角色卡内入口。

### Finding 7: 桌面图标为品牌化 indigo 渐变图形，移动端为自制极简同心圆环，两者视觉不一致

**Evidence:** companion-android/app/src/main/res/drawable/ic_launcher_foreground.xml:9-23（同心圆 vector，#A5B4FC/#6366F1）+ values/colors.xml:4（背景 #0F1117 深色）；egosync-app/src-tauri/icons/icon.png（512×512，主色 #4E45E5/#4F46E5 indigo 实色 + 透明背景）；桌面 android 侧 values/ic_launcher_background.xml:3（#fff 白底）；git log 3b92565（"替换应用 logo"）

**Detail:** 桌面图标是 indigo 色品牌图形（icon.ico/icns/png 全套），且 `src-tauri/icons/android/mipmap-*` 六档密度 ic_launcher/ic_launcher_round/ic_launcher_foreground PNG + 白色 background 全部现成。移动端自制了一个"中心圆点+双环"极简矢量图（深色背景），与桌面视觉语言不同。用户诉求"替换成桌面版对应的图标"资源上零障碍：直接复制 `src-tauri/icons/android/` 的 mipmap 树与背景色即可。

### Finding 8: 任务屏缺失"归属筛选"与"新建"两项能力，桌面母本均存在；根因是数据模型缺归属字段

**Evidence:** TasksViewModel.kt:16-26（UiState 仅 quadrantFilter）、TasksScreen.kt:62-69（无新建入口/回调）、SnapshotStore.kt:39-50（TaskItem 仅 roleName 字符串，无 ownerType/roleId）；桌面母本 TaskOverviewTab.tsx:184-208/315-348（归属多选筛选）、TaskOverviewTab.tsx:276-282 + TaskModal.tsx:101-148（新建任务流）、egosync-app/src/types/task.ts:8-13（ownerType + roleId 结构化归属）

**Detail:** 用户陈述③与现状完全一致：
- **归属筛选缺失的根因在数据模型层**：移动端 TaskItem 只有 `roleName: String` 展示字符串，无桌面端的结构化归属（ownerType: 'role'|'butler' + roleId），"管家"靠字符串约定（SnapshotStore.kt:370）。无稳定标识 → 筛选无从做起。
- **新建缺失**：无入口、无回调、无 VM 方法。但移动端已有"智能分类中"过渡态机制（classifyingIds + applyClassified，TasksViewModel.kt:50-69，有单测锁定契约），新建任务选"智能判断"象限可无缝复用。
- 桌面母本细节：归属筛选为"多选排除"语义（deselectedOwners 取反，前端过滤）；新建表单字段为归属下拉/内容/四象限（含✨智能判断）/截止时间（日期+时+分）/大石头标记。

### Finding 9: 移动端相对桌面的附带任务功能差异（用户未声称）

**Evidence:** TaskOverviewTab.tsx:303-313（只看大石头开关）、TaskOverviewTab.tsx:141-162（删除/编辑）、TaskOverviewTab.tsx:395-421（已完成折叠）vs TasksScreen.kt/TasksViewModel.kt 对应缺失

**Detail:** 只看大石头开关、删除（含确认）、编辑、已完成折叠均缺失。列附带差异供用户决策是否一并补齐，不主动扩大范围。

## Deduced Conclusions

### Deduction 1: 用户报告的④记忆缺口大概率是"观察基线过旧"或"入口可达性"问题

**Based on:** Finding 4、Timeline（f6fb40c 早于用户报告）

**Reasoning:** 功能代码与导航入口均已存在且带单测；用户仍报告缺失 → 要么用户安装的 APK 构建于 f6fb40c 之前，要么入口（仪表盘→角色卡→查看记忆）层级太深导致未被发现。

**Conclusion:** 需向用户确认 APK 构建版本；若为最新构建，则问题定性为"入口可达性/信息架构"而非"功能缺失"，方案方向完全不同。

## Hypothesized Paths

### Hypothesis 1: 用户在旧构建 APK 上观察④（及可能的部分③）

**Status:** Open

**Theory:** 用户安装的 APK 构建于 f6fb40c/eca358f 之前，记忆屏与象限筛选均未包含。

**Supporting indicators:** 记忆功能代码完整存在；用户报告与代码状态矛盾。

**Would confirm:** 用户提供 APK 安装时间/构建版本，或重新构建后问题消失。

**Would refute:** 用户在最新构建上仍看不到记忆入口（→ 转向可达性调查）。

**Resolution:** 待定。

### Hypothesis 2: "绿色块全部去掉"指删除整个三态横幅组件

**Status:** Open

**Theory:** 用户嫌横幅常驻占空间（原型中 direct 为默认态，绿块永远在顶部）。

**Supporting indicators:** 用户措辞"那个绿色块全部去掉"；原型无法真实切换连接态，绿块是纯噪音。

**Would refute:** 用户只想在 direct 态隐藏、offline 时保留红色提示。

**Resolution:** 待用户澄清（方案影响：删组件 vs. 仅 direct 态隐藏）。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 用户 APK 构建版本 | 决定④是"升级构建"还是"改信息架构" | 问用户安装时间/来源，或重新构建安装验证 |
| 任务/记忆功能全貌细节 | 决定③④的精确方案 | subagent 返回中 |
| 桌面端新建任务/筛选母本 | 决定③方案是否以桌面为事实源对齐 | subagent 返回中 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | N/A（非 bug 案件：2 项特性移除/替换 + 2 项功能缺口核实） |
| Trigger | 用户对移动端原型的使用观察 |
| Condition | 见各 Finding |
| Related files | AppNavHost.kt、ConnectionStatusBanner.kt、TasksScreen/ViewModel、MemoryScreen/ViewModel、SnapshotStore.kt、res/、桌面 icons/ |

## Conclusion

**Confidence:** High（四项问题全部有直接代码证据定性）

| # | 用户诉求 | 定性 | 根因 |
| - | ------- | ---- | ---- |
| ① 去掉绿色横幅 | 证实（设计取舍问题，非 bug） | 横幅为 PRD FR-40 三态设计，原型 mock 恒为 Direct → 绿块常驻成噪音 |
| ② 替换应用图标 | 证实（资源从未搬运） | companion res/ 用自制同心圆环矢量图，桌面 mipmap 全套在 src-tauri/icons/android/ 现成未用 |
| ③ 任务归属筛选 + 新建 | 证实（功能缺失） | TaskItem 数据模型缺 ownerType/roleId 结构化归属字段；新建无入口/回调/VM 方法 |
| ④ 记忆查看与遗忘 | **与前提矛盾**：UI 已 100% 实现且可达（f6fb40c 落地） | 最可能：用户 APK 为旧构建，或未发现仪表盘→角色卡→查看记忆入口 |

**Confirmed 根因**：①③为真实缺口；②为资源搬运遗漏；④大概率是观察基线问题（Hypothesis 1 保持 Open，需用户确认 APK 版本）。

## Recommended Next Steps

### Fix direction

**① 横幅删除**（需用户澄清范围，见 Hypothesis 2）：
- 方案 A（推荐，若"全部去掉"=整个组件）：删除 `MainShellRoute` topBar 挂载（AppNavHost.kt:213）+ 删除 ConnectionStatusBanner.kt + 删除其 Preview 测试引用。离线明示已由 DegradedOverlay 独立承担（Finding 5），无功能损失。Debug 状态模拟档位说明（README:113-118）需同步修订。
- 方案 B（若仅 direct 态隐藏）：横幅加 `state is Direct` 时返回零高度/隐藏分支。保留 offline/relay 提示能力。
- 语义注意：`SettingsScreen` 配对设备卡仍有连接状态圆点（SettingsScreen.kt:435 一带），不受横幅删除影响——连接状态的"我的"页观察点保留。

**② 图标替换**（零障碍）：
- 将 `egosync-app/src-tauri/icons/android/mipmap-{mdpi..xxxhdpi}/`（ic_launcher.png / ic_launcher_round.png / ic_launcher_foreground.png）复制到 `companion-android/app/src/main/res/mipmap-*dpi/`；
- 用桌面 `values/ic_launcher_background.xml`（#fff）替换移动端 colors.xml 中 `ic_launcher_background`（#0F1117 深色）——或保留深色背景仅换前景（需用户选，白底更贴桌面观感）；
- 删除移动端自制 `drawable/ic_launcher_foreground.xml` 矢量图；`mipmap-anydpi-v26/ic_launcher.xml` 的 foreground 引用改为 `@mipmap/ic_launcher_foreground`。
- 注意：桌面 PNG 前景可能不带内边距安全区（108dp 画布中心 66dp 安全区），需目检 adaptive icon 裁切效果。

**③ 任务功能补齐**（两缺口均需三层改动，镜像桌面母本）：
- 数据层：TaskItem 增加 `ownerType: TaskOwner`（BUTLER/ROLE）+ `roleId: String?`，mock 数据同步（管家任务改结构化表达）；SnapshotStore.roles 可作筛选选项源。
- 归属筛选：TasksUiState 增加 ownerFilter（对齐桌面"多选排除"语义或简化为单选——建议简化为多选排除，语义一致）；grouped() 扩展过滤；UI 在象限筛选行下加归属 chip 行（管家 + 各角色，Users 图标）。
- 新建任务：TasksScreen 加"新增任务"入口（右下 FAB 或顶栏按钮）+ 底部弹层表单（归属下拉/标题/四象限含"智能判断"/截止/大石头，镜像 TaskModal.tsx 字段）；VM 加 create 方法，"智能判断"复用既有 classifyingIds 过渡态机制（TasksUiStateTest.kt:61-105 契约已锁定）；mock 本地追加。
- 附带决策点：是否一并补"只看大石头"开关（改动小：UI+VM，数据层不动）。

**④ 记忆功能**：
- 先验证：`./gradlew :app:assembleDebug` 构建最新 APK 安装，走 仪表盘→角色卡→查看记忆 验证查看/遗忘流。
- 若用户要的是"遗忘真实生效"（跨页面持久）：原型语义为只读 mock，需将遗忘改为 SnapshotStore 可变状态或引入内存持久层——但这与"桌面为唯一事实源"的原型定位冲突，真实语义应留给 COMMAND 帧接入（README 替换点 3/5 已预留）。建议维持现状 + 验证构建版本。

### Diagnostic

- 确认用户 APK 构建版本：对比 `app/build/outputs/apk/debug/app-debug.apk` 构建时间与 git log e6dba8a 时间；或直接重新构建安装验证④。

## Reproduction Plan

（探索型案件的验证计划）
1. `cd companion-android && ./gradlew :app:assembleDebug`，安装最新构建。
2. 逐项核对：①顶部绿色横幅存在（待删除）；②桌面图标未应用（launcher 显示同心圆环）；③任务 Tab 无归属筛选/新建入口；④仪表盘→任一角色卡→查看记忆→类别筛选/来源展开/遗忘确认流全部可用。
3. 若④在最新构建不可复现 → 证实 Hypothesis 1（旧 APK）。

## Side Findings

- companion-android/README.md 页面地图滞后：未记载 memory 屏（FR-8/9）、onboarding 屏（FR-21）、chat 屏六项 FR、设置页主动性控件等后续增量（git log 显示 README 写于 d88ef85 前后，之后 7 个功能提交未同步文档）。证据：README.md:24-60 vs git log f6fb40c/e6dba8a。

## Follow-up: 2026-05-27

### New Evidence

用户反馈：④记忆功能已找到（Hypothesis 1 就此结案——功能存在且入口可达，此前观察缺失系未发现入口或旧构建）。但发现新问题：**"指标名称的文本显示不全"**。

### Additional Findings

### Finding 10: MetricCard 标签 Text 缺 weight(1f)，与同文件 FilterChipSurface 处理方式不一致（潜在截断缺陷）

**Evidence:** DashboardScreen.kt:701-715（MetricCard 标签行：Icon 16dp + Spacer 6dp + Text[maxLines=1, Ellipsis, **无 weight**]）；对比 DashboardScreen.kt:666-673（FilterChipSurface 标签 Text **有** `Modifier.weight(1f)`）；标签文案 SnapshotStore.kt:198-203（任务总数/记忆数量/待处理任务/对话数量）

**Detail:** Compose `Row` 中无 weight 的子项按传入约束（整行宽）测量，Text 的省略号按整行宽计算而非"扣除图标后的剩余宽"——标签可用宽度被图标+间距（22dp）挤占，在窄屏或大字体（fontScale）下标签会在卡片右缘被裁切/截断。同文件 FilterChipSurface:672 的正确写法（weight(1f)）证明这是遗漏而非设计。

**宽度验算（360dp 屏，默认字体）：** 卡片内容宽 ≈135dp，"待处理任务"labelSmall≈55dp，理论上放得下——默认条件下不应截断。**触发条件疑为窄屏或系统字体放大**，无法从静态代码完全确认用户设备上的实际触发点。

### Finding 11: MemoryFilterChip 行为窄屏溢出风险（次要）

**Evidence:** MemoryScreen.kt:92-103（chips 行无 weight、取固有宽，"认知模式" 4 字 chip 最宽）

**Detail:** 默认 360dp 屏下总宽约 240dp < 可用 328dp，不溢出；窄屏/大字体下最后一枚 chip 可能被裁。

### Updated Hypotheses

### Hypothesis 1（旧 APK/未发现入口）→ **Resolved**

**Resolution:** 用户已找到记忆功能，证实功能存在且可达。此前观察缺失系入口未发现（或旧构建），非代码缺陷。

### Hypothesis 3: "指标名称显示不全"由 MetricCard 标签缺 weight 触发（窄屏/大字体）

**Status:** Open

**Theory:** 见 Finding 10。但默认 360dp + 默认字体下宽度验算显示不应截断，静态分析无法确证。

**Would confirm:** 用户提供截图或指明具体截断元素（活动统计四指标卡 / 角色卡四宫格 / 记忆屏类别 chips / 其他）+ 设备宽度或字体缩放设置。

**Would refute:** 截断元素为其他位置（如 StatCell 或 chips）。

### Backlog Changes

| # | 新增 | Priority | Status |
| - | --- | -------- | ------ |
| 6 | 定位"指标名称显示不全"的确切元素（需用户输入：截图/位置+设备信息） | High | Open |

### Updated Conclusion

④结案（Hypothesis 1 Resolved）。新问题"指标名称显示不全"已锁定一个 Confirmed 的结构性缺陷（MetricCard 标签缺 weight，Finding 10），但用户设备上的实际触发点存在多种候选（指标卡/四宫格/chips），按规则十三不猜测，待用户指明确切位置后给出针对性修复方案。

### 用户指认"角色卡四宫格"后的证据矛盾与再推断（同日续）

用户在选项中指认为"角色卡四宫格"（任务/记忆/会话/待办）。静态验算产生**矛盾**：

- **StatCell 行不可能横向截断**：标签每格 2 字（≈22dp），格子宽 ≈(屏宽-84)/4（360dp 屏→69dp；320dp 屏→59dp；fontScale 2.0 时标签 ≈40dp 仍放得下）；布局无裁切约束（DashboardScreen.kt:326-331 weight 均分、:372-385 Column 居中无 maxLines）。
- **纵向裁切排除**：labelSmall 10sp/行高 14sp（Type.kt:23）比例健康，无高度约束。
- **再推断（Hypothesis 4）**：用户所说"角色卡四宫格"实为**活动统计 2×2 指标网格**——它同为四宫格、自带角色 scope 下拉（全部/管家/各角色，DashboardScreen.kt:509-525）、紧邻角色卡列表；其标签（任务总数/记忆数量/待处理任务/对话数量，4-5 字）正对应 Finding 10 的 Confirmed 缺陷（缺 weight → 窄屏/大字体下右缘截断）。"指标名称"措辞与代码注释"指标卡/指标网格"（DashboardScreen.kt:485, 685）吻合。
### 用户澄清症状为"纵向遮盖"后的再定位（同日续 #2）

用户澄清：受影响元素确认为仪表盘底部角色卡的四宫格统计（任务/记忆/会话/待办），症状是**标签文本的下半部分被遮盖**（纵向，非横向截断）。

**排除项（静态验证）：**
- 角色卡内部（RoleCardItem，DashboardScreen.kt:285-367）为纯 Column 顺序布局：头部行→能量条→统计行→查看记忆钮，无绝对定位/负偏移/重叠组件——卡内无元素可遮盖统计标签。
- BreathingEnergyBar（:393-440）为顺序布局 6dp 轨道条，位于统计行上方隔 Spacer(20)，无重叠可能。
- Scaffold content padding 已应用（AppNavHost.kt:242-243），底部导航栏不与内容重叠。

**纵向空间验算（Finding 12，Confirmed 算术证据）：**
- 角色卡固有高度 ≈ 259-267dp（头部 46 + 能量条 39 + 统计行 45 + 查看记忆 41 + 内边距 36 + 间距 52 + pager contentPadding 8）。
- 仪表盘 Column 固定内容（顶部 Spacer 12 + 管家概览卡 ≈132 + Spacer 18 + 活动统计区 ≈226 + Spacer 14 + 底部指示点区 ≈32）合计 ≈434dp。
- HorizontalPager 以 weight(1f) 取剩余高度：360×800dp 屏（扣横幅 ~48 + 底栏 ~80）→ pager ≈ 238dp < 卡高 ≈267dp，**角色卡溢出页面 ~29dp**；屏幕越矮溢出越多；pager 高度 ≈190dp 时裁切边界恰好落在统计标签中部。
- 结论：**角色卡高度超过 pager 页可用高度**是 Confirmed 的结构性问题；具体遮盖表现（标签下半被切）取决于设备实际屏高与字体缩放，静态验算无法唯一确定裁切边界位置。

**待确证证据**：用户设备屏幕规格/字体缩放；查看记忆按钮是否也被裁切（判别裁切边界位置）；或直接提供截图。

## Follow-up 收口：用户接受诊断（2026-05-27 同日 #3）

用户以"OK"接受纵向溢出诊断，未再提供设备参数——该证据缺口不阻塞修复方向（无论裁切边界精确落点，修复均为"角色卡页内垂直自适应"）。最终执行清单（5 项，均暂不执行、待用户发起）：

| # | 问题 | 已裁决方案 | 关键改动点 |
| - | ---- | ---------- | ---------- |
| ① | 绿色横幅 | 删整个三态横幅 | AppNavHost.kt:213 去 topBar 挂载；删 ConnectionStatusBanner.kt；README:34/113-118 同步修订 |
| ② | 应用图标 | 桌面版白底图标 | 复制 src-tauri/icons/android/mipmap-* → res/；ic_launcher_background 改 #fff；删自制 drawable 矢量 |
| ③a | 任务归属筛选 | 补齐（多选排除语义） | TaskItem 补 ownerType/roleId；UiState 加 ownerFilter；UI 加归属 chip 行 |
| ③b | 新建任务 | 补齐 | "新增任务"入口 + 底部弹层表单（归属/标题/象限含智能判断/截止/大石头）；复用 classifyingIds 过渡态 |
| ③c | 只看大石头开关 | 一并补齐 | UI chip + VM showBigRocksOnly 过滤；数据层不动 |
| ④ | 记忆功能 | 无需改（已实现，用户已找到） | — |
| ⑤ | 角色卡纵向溢出 | 页内垂直自适应 | 角色 HorizontalPager 页加垂直滚动或压缩卡内间距（267→≤pager 可用高） |

**次级顺手项**（可并入 ① 或单独小改，非必须）：MetricCard 标签补 weight(1f)（Finding 10，一行，防大字体截断）；README 页面地图补 memory/onboarding 屏（Side Finding）。

## 执行日志（2026-05-27，用户裁决全部走 bmad-quick-dev）

| # | 提交 | 结果 | 备注 |
| - | ---- | ---- | ---- |
| 前置 | `46a28a5` | FR-24 脏改动（上一会话遗留）单独提交 | 基线单测绿后提交，保持特性隔离 |
| ① 横幅删除 | `f13cdfe` | 完成 | 对抗评审 6 处过时文案残留全部 patch；状态栏 padding 经字节码级验证无回归 |
| ② 图标替换 | `3e49fcf` | 完成 | 评审抓出 major：Tauri foreground 无安全区留白（94×88dp 超 66dp）→ PIL 后处理 5 密度缩放居中；hdpi 49×49 异常修复为 72×72；round 死资源删除；monochrome 层/桌面端同源问题记入 deferred-work |
| ⑤ 角色卡溢出 | `bad7634` | 完成 | 评审抓出 major：纯滚动方案默认视图不改变可见像素 → 叠加压缩卡内间距（省约 30dp ≥ 29dp 溢出），滚动保留为极矮屏兜底 |
| ③ 任务三件套 | `cad428b` | 完成 | 三路评审（盲猎手/边界猎手/验收审计员）：I/O 矩阵 7 场景全过、Boundaries 零违反；patch 7 项（major：分类中任务豁免象限过滤保「立即上屏」契约；minor：表单离线门禁、rememberSaveable、id 从种子推导、no-op 回退、补 toggleOwner 中间路径与空标题校验测试）；defer 4 项记入 deferred-work.md。66 单测全绿 |

评审拦截价值记录：②⑤ 两个 major 均为初版方案的真实缺陷，若直接提交将分别产生"图标被裁切 1.3 倍"与"首屏仍被裁"的用户可见回归。

**移交建议**：①②⑤改动小且已裁决到位 → bmad-quick-dev 直接执行；③ 涉及三层（Store/VM/UI）+ 既有单测契约 → 建议 bmad-create-story 立故事跟踪后执行。
