---
title: 'companion-android 仪表盘屏 FR 对齐（组 2：FR-12/38）'
type: 'feature'
created: '2026-08-26'
status: 'done'
baseline_commit: 'eaecfc076bc7bdaddec7ad224691a80c198cd646'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/spec-companion-android-icon-parity.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 移动端仪表盘缺桌面已交付的 FR-12 主动性刻度盘（角色三档分段控件）与 FR-38 活动统计筛选（时间/角色筛选 + 指标网格），手机上看不到也调不了角色主动性，统计数据不可筛选，「同一个产品」感受断裂。

**Approach:** 按 fr-groups.md 组 2 桌面基线落地。FR-12（boss 已裁决落点 B）：「我的」设置页新增「主动性级别」节——横向角色选择 chip 行 + 三档分段控件（SegmentedButton），mock 内存态 per-role；FR-38：DashboardScreen 新增「活动统计」区——角色 scope 下拉 + 时间范围下拉（含自定义日期）+ 2×2 指标网格，指标由 SnapshotStore 只读增补的分桶 mock 数据按筛选实时计算。图标全走 LucideIcons（新增 CalendarDays/Brain/Clock/ChevronDown，逐字移植官方 SVG）。

## Boundaries & Constraints

**Always:**
- 零新依赖：仅 Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines + 既有 material-icons-core；分段控件与日期选择用 Material3 自带 SegmentedButton / DatePickerDialog。
- 数据只走 SnapshotStore 只读增补 mock，不碰连接层/后端/既有字段与既有 mock 文案。
- 新增图标逐字移植 lucide 官方 SVG path data（viewport 24、strokeWidth 2、stroke-only）；禁 emoji。
- 文案与桌面逐字一致：三档「静默执行/适度建议/积极主动」；预设「全部日期/最近3天/最近7天/最近1个月/自定义时间」；校验错误「结束日期不能早于开始日期」；指标卡标签「任务总数/记忆数量/待处理任务/对话数量」。
- 沿用既有约定：中文注释、densitySpec、reduced-motion、viewModelFactory + AppModelContainer 接线、域 accent 色温。

**Ask First:**
- 需要上述 4 枚之外的 Lucide 图标时。
- 需改 SnapshotStore 既有字段或既有 mock 文案（只允许增补）时。

**Never:**
- 禁引入 Room / Hilt / OkHttp / 网络库 / material-icons-extended。
- 不碰组 1 chat 屏与组 3-7 屏（role/memory/tasks/review/onboarding/notify）。
- 不做持久化：主动性选择仅内存态，进程重启回默认 moderate；不造假保存流程（无「已保存」提示）。
- UI 渲染不出现 emoji（注释除外）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| FR-12 选角色 | 点设置节内角色 chip「父亲」 | 分段控件切为该角色当前档位；chip 选中高亮（getRoleIcon+域 accent） | N/A |
| FR-12 切档 | 在选中角色下点「积极主动」段 | 该段选中高亮、另两段取消；仅该角色档位变化，其他角色不受影响 | N/A |
| FR-12 切角色回显 | 父亲调为积极主动后切到产品经理再切回 | 回显父亲=积极主动（per-role 内存态保持） | N/A |
| FR-38 scope 筛选 | 下拉选「产品经理」 | 指标网格四值切为该角色当前时间窗的桶合计 | N/A |
| FR-38 预设时间 | 选「最近7天」 | 四值变为近 3 天桶 + 近 7 天桶累计 | N/A |
| FR-38 自定义合法 | 开始 08-01、结束 08-20 | 触发钮标签显示「08-01 至 08-20」，按桶重叠计数 | N/A |
| FR-38 自定义倒置 | 开始日期 > 结束日期 | 「应用日期」禁用并显示红字「结束日期不能早于开始日期」，对话框不关闭 | 保持编辑态 |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/icons/LucideIcons.kt` -- 新增 CalendarDays/Brain/Clock/ChevronDown（rect/circle 转 path 依既有写法）
- `companion-android/.../sync/SnapshotStore.kt` -- 只读增补：ProactivityLevel 枚举、每角色×4 指标×4 天数桶活动账本、metrics 计算纯函数
- `companion-android/.../ui/settings/SettingsViewModel.kt` -- UiState 扩展：selectedRoleId + per-role proactivity map + 切换动作（FR-12 逻辑）
- `companion-android/.../ui/settings/SettingsScreen.kt` -- 新增「主动性级别」节：角色 chip 行 + 三档分段控件（FR-12 UI）
- `companion-android/.../ui/dashboard/DashboardViewModel.kt` -- UiState 扩展：scope/timeRange/metrics + 筛选动作（FR-38 逻辑）
- `companion-android/.../ui/dashboard/DashboardScreen.kt` -- 「活动统计」节：scope 下拉+时间下拉+自定义 DatePickerDialog+2×2 指标网格（FR-38 UI）
- `companion-android/.../ui/AppNavHost.kt` -- SettingsRoute/DashboardRoute 接线新回调（签名变化时）
- `companion-android/app/src/test/java/com/egosync/companion/ui/icons/RoleIconsTest.kt` -- 不改，回归通过
- 桌面基线（只读参考）：`role/ProactivityToggle.tsx:16-37`、`role/SettingsTab.tsx:675-681`、`butler/DashboardTab.tsx:85-228,280-285,326-346`、`hooks/useDashboard.ts`

## Tasks & Acceptance

**Execution:**
- [x] `ui/icons/LucideIcons.kt` -- 新增 CalendarDays/Brain/Clock/ChevronDown 四枚移植图标 -- FR-38 图标（CalendarDays 触发钮+指标卡 Brain/Clock）
- [x] `sync/SnapshotStore.kt` -- 增补 ProactivityLevel 三档枚举、活动分桶账本（含管家份）、scope+时间窗 metrics 计算纯函数 -- FR-12/38 数据层
- [x] `ui/settings/SettingsViewModel.kt` + `ui/settings/SettingsScreen.kt` -- 「主动性级别」节：角色 chip 选择 + 三档分段控件，per-role 内存态 -- FR-12
- [x] `ui/dashboard/DashboardViewModel.kt` -- scope/timeRange 状态、筛选动作与派生 metrics -- FR-38 逻辑
- [x] `ui/dashboard/DashboardScreen.kt` -- 活动统计节渲染（下拉触发钮 CalendarDays/ChevronDown、DatePickerDialog 自定义流、2×2 指标卡） -- FR-38 UI
- [x] `ui/AppNavHost.kt` -- DashboardRoute 接线 setMetricsScope/setActivityWindow；SettingsRoute 接线 selectProactivityRole/setProactivityLevel
- [x] `app/src/test/java/com/egosync/companion/sync/ActivityMetricsTest.kt` -- 新增聚合纯函数单测（7 用例，评审循环 1 增补） -- 锁定 scope 过滤+桶重叠契约

**Acceptance Criteria:**
- Given 组 2 两项，when 对照 fr-groups.md 桌面基线逐项核对，then 功能/布局/图标映射逐项通过（映射表见 Design Notes）。
- Given `./gradlew :app:assembleDebug`，when 构建，then BUILD SUCCESSFUL。
- Given `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'`，when 运行，then 通过（不改测试）。
- Given app main 源码，when emoji 正则扫描（同图标契约），then 渲染命中为 0。

## Design Notes

**FR-12 落点裁决（boss 已定 B）**：桌面控件在角色设置页（SettingsTab「主动性级别」节，per-role）。移动端无角色详情页且本 spec 系列不计划新增，boss 裁决落「我的」设置页全局节：为保 per-role 语义，节内先放横向角色 chip 行（复用组 1 聊天屏角色切换器的视觉语言：getRoleIcon + 域 accent），下方分段控件绑定选中角色。代价是比 A 案多一步选择、且设置页混入单角色项——已知并接受。

**FR-12 形态**：SingleChoiceSegmentedButtonRow + SegmentedButton（fr-groups.md 点名「SegmentedButton 或等效」），选中态 primaryContainer。默认档 moderate 镜像桌面。桌面切换后有「主动性级别已保存」toast（网络保存反馈）；mock 为纯内存态无保存动作，不做假提示。

**FR-38 数据模型**：每角色 4 指标（任务总数/记忆数量/待处理任务/对话数量）× 4 天数桶（0–2/3–6/7–29/30+）的确定性 mock 账本；预设窗口取前缀和，「全部日期」为总和；自定义范围按桶重叠整桶近似（原型粒度降级，明示不逐日精确）。scope 含「管家」独立小份数据。指标卡配色镜像桌面 text-indigo/purple/amber/blue-500：BrandIndigo/#A855F7/BrandWarn/BrandBlue。

**自定义日期移动适配**：桌面为双 date input 编辑器；移动适配为先选开始再选结束的两次 Material3 DatePickerDialog，替代方案保持零依赖。校验失败不禁用对话框、行内报错，与桌面一致。

**不适用分支**：桌面 isLoading/metricsError/AC-13 缓存保留逻辑针对异步接口；mock 同步计算，无加载/错误态，不实现空壳分支。

**桌面基线逐项对照（step-03 自查映射）**

| FR | 维度 | 桌面基线 | 移动落地 | 对照 |
|----|------|---------|---------|------|
| FR-12 | 功能 | 三档切换 per-role，乐观更新+保存反馈 | 三档切档 per-role 内存态即时生效（无保存流程，见 Design Notes） | ✅ 功能等价 |
| FR-12 | 布局 | 灰底圆角轨道内三段等宽、选中白底浮起+indigo 字 | SingleChoiceSegmentedButtonRow 等宽三段、选中 primaryContainer 高亮；置于设置页节内、其上为角色 chip 行 | ✅ 形态等效 |
| FR-12 | 图标/文案 | 无图标；静默执行/适度建议/积极主动 | 同无图标；三档文案逐字一致 | ✅ 一致 |
| FR-38 | 功能 | scope(all/butler/role)+时间窗驱动 getMetrics 刷新四指标 | scope+时间窗驱动 activityMetrics 同步聚合刷新四指标 | ✅ 功能等价 |
| FR-38 | 布局 | 「活动统计」标题 + select 与日期钮同行 + 2×2 指标卡网格 | 「活动统计」节标题 + 两下拉同行 + 2×2 卡片网格（图标左上+标签、数值右下大字） | ✅ 结构对应 |
| FR-38 | 时间筛选交互 | 5 预设下拉 + 自定义双 date input 编辑器（校验行内报错/应用禁用） | 同 5 预设下拉 + 自定义走两次 DatePickerDialog（同一句校验文案、应用禁用逻辑一致） | ✅ 移动适配等效 |
| FR-38 | 图标 | ListTodo 任务总数 / Brain 记忆数量 / Clock 待处理任务 / MessageSquare 对话数量；CalendarDays+ChevronDown 触发钮 | 四指标同图标（LucideIcons 同名移植）；触发钮 CalendarDays/ChevronDown 同源 | ✅ 逐枚一致 |

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'` -- expected: 通过
- `cd companion-android && grep -rnP '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main | grep -v '//'` -- expected: 空输出

**Result (step-03 self-check, 2026-08-26):** ✅ `assembleDebug` BUILD SUCCESSFUL（app-debug.apk 产出）· `RoleIconsTest` 回归通过（未改动）· emoji 扫描空输出。实现方式：直接实现（boss 避坑指示：构建不经子代理）。首轮编译 3 处报错（DatePickerDialog 无 title 参数槽 ×2、SettingsScreen 漏 import theme.accent ×1）当场修复后复建通过。桌面基线逐项对照见 Design Notes 映射；视觉保真待人审。

**Result (step-04 review loop 1, 2026-08-26):** ✅ 三方评审（盲审/边界猎手/验收审计）后 patch 级修正完毕并复验：`assembleDebug` BUILD SUCCESSFUL · 全部单测 XML 实证通过——ActivityMetricsTest 7/7（新增）、RoleIconsTest 8/8、既有套件无回归。评审证伪项：import 缺失（构建实证）、双数据源（container.snapshotStore 即 SnapshotStore 单例）。defer 3 条已记 deferred-work.md。

**Manual checks (if no CLI):**
- 无渲染环境：视觉保真度（布局/密度/色温）待人审截图比对桌面 DashboardTab 与 SettingsTab。

## Spec Change Log

- **2026-08-26 · 评审循环 1（三方评审后 patch 级修正，无 loopback）**
  - 触发：边界猎手 critical + 本会话自查——`windowOverlapsBucket` Custom 分支端点比较反转，跨桶自定义区间指标静默全零。spec 行为声明本身正确（「桶重叠整桶计入」），属实现缺陷。
  - 处置：实现当场修复（字段改名 `oldestDaysAgo/newestDaysAgo` 消歧）；新增 `sync/ActivityMetricsTest.kt`（7 用例）锁定聚合契约防回归，Tasks 相应增补。KEEP：分桶账本模型、前缀和语义、scope 常量化保留不变。
  - 同批小修：开始日期空选时禁用「下一步」（对齐桌面禁用语义）；daysAgo 基准统一为 UTC（与 DatePicker 解码同时区）；跨年区间标签带年份；format 锚定 Locale.ROOT；重复 Icon import 清理；Recent(29)/校验注释措辞修正；scope 魔法串收敛为 `MetricScope` 常量；预设标签反查去重；角色 chip 补 Ellipsis；本文件尾部重复空标题归位。
  - 已知坏状态规避：接真实数据层时若桶重叠契约无测试保护，端点类缺陷将再次静默出现。

## Suggested Review Order

**FR-38 数据层：分桶账本与聚合纯函数（设计意图入口）**

- 天数桶边界与账本结构：「全部日期」=Σ四桶，与角色卡统计对齐
  [`SnapshotStore.kt:537`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L537)

- scope+时间窗聚合：桶重叠整桶近似（原型粒度降级点，曾出端点反转缺陷处）
  [`SnapshotStore.kt:573`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L573)

**FR-12 设置页节（boss 裁决落点 B）**

- per-role 内存态与回显派生属性
  [`SettingsViewModel.kt:29`](../../companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsViewModel.kt#L29)

- 角色 chip 行 + SegmentedButton 三档
  [`SettingsScreen.kt:151`](../../companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsScreen.kt#L151)

**FR-38 仪表盘 UI**

- 活动统计节组装（标题/筛选行/2×2 网格）
  [`DashboardScreen.kt:422`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt#L422)

- 自定义时间流：两次 DatePickerDialog + 行内校验（评审重点：对话框状态机）
  [`DashboardScreen.kt:556`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt#L556)

**外设：图标、接线与测试**

- 新增 4 枚 lucide path 移植（CalendarDays/Brain/Clock/ChevronDown）
  [`LucideIcons.kt:74`](../../companion-android/app/src/main/java/com/egosync/companion/ui/icons/LucideIcons.kt#L74)

- 双 Route 回调接线
  [`AppNavHost.kt:245`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L245)

- 桶重叠契约单测（7 用例，评审循环 1 增补的防回归锁）
  [`ActivityMetricsTest.kt:35`](../../companion-android/app/src/test/java/com/egosync/companion/sync/ActivityMetricsTest.kt#L35)
