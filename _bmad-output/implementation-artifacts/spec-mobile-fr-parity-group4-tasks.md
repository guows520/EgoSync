---
title: 'companion-android tasks 屏 FR 对齐（组 4：FR-23）'
type: 'feature'
created: '2026-08-27'
status: 'done'
baseline_commit: 'f6fb40c8a6368af4e1d93ec0e7e4ba84fcf71d42'
context:
  - '{project-root}/_bmad-output/specs/spec-mobile-fr-parity/SPEC.md'
  - '{project-root}/_bmad-output/specs/spec-mobile-fr-parity/fr-groups.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-companion-android-icon-parity.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 移动端 TasksScreen 当前仅展示四象限分组静态列表（勾选完成 + 大石头星标），缺桌面 useTasks 的两项核心交互：新建任务后后端异步分类期间的「智能分类中…」过渡态徽章（task:classified 事件未到前显示），以及象限筛选（全部/Q1~Q4 切换可见分组）。FR-23 处于「部分→完整」补全状态。

**Approach:** 按桌面基线 `useTasks.ts:29-49`（classifyingIds + task:classified 事件）与 `TaskOverviewTab.tsx:26-32,124-132,286-301`（Filter 行 + Loader2 旋转徽章）落地。TasksUiState 增 `classifyingIds` + `quadrantFilter`；TasksViewModel mock 一条「新建未指定象限」任务→进屏后模拟事件到达归类清标记；TasksScreen 加象限筛选 chip 行 + 分类中徽章。图标 LucideIcons 增 Filter/Loader2（lucide path 逐字移植，模式同组 3）。

## Boundaries & Constraints

**Always:**
- 零新依赖：仅 Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines + 既有 material-icons-core（本组用 viewModelScope + delay 模拟事件，kotlinx-coroutines 已在列）。
- 数据只读增补 SnapshotStore（不动既有 tasks mock 文案/字段）；分类中态与归类结果为 ViewModel 内存态，进程重启还原（同组 1/2/3 mock 不持久原则）。
- 图标一律 LucideIcons.kt（新增 Filter/Loader2，lucide 官方 SVG path 逐字移植，viewport 24、strokeWidth 2、stroke-only）；禁 emoji。
- 文案与桌面逐字一致：徽章「智能分类中…」、筛选 chip「全部/Q1/Q2/Q3/Q4」、筛选空态「当前筛选无匹配任务」、默认空态「所有角色都很轻松，可以考虑添加新目标」。
- 沿用既有约定：中文注释、`rememberReducedMotion`（Loader2 旋转在 reduced-motion 时静止，镜像桌面 motion-reduce:animate-none）、viewModelFactory + AppModelContainer 接线、indigo 色温徽章（桌面 indigo-50/600/100 → 移动 BrandIndigo）。

**Ask First:**
- 需改 TasksScreen 既有勾选完成/大石头徽章/分组行为时（只允许增补，不改既有交互）。
- 需把分类中态接到非 TasksScreen 屏（组 7 notify）时——本组不接。

**Never:**
- 禁引入 Room / Hilt / OkHttp / 网络库 / material-icons-extended。
- 不碰组 1/2/3/5-7 屏与既有行为（聊天路由/拆分/仪表盘统计/记忆/周复盘等）。
- 不实现任务创建流（FR-23 只补分类中态与象限筛选，新建入口超范围）；不接真实连接层/后端事件。
- 不加「只看大石头」/归属多选筛选（桌面 TaskOverviewTab 有，但 fr-groups 组 4 仅要求象限筛选交互，超范围）。
- UI 渲染不出现 emoji（注释除外）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 进屏 seed 分类中 | 首次进入 TasksScreen | 列表含一条「智能分类中」任务（带旋转 Loader2 徽章），其后 ~4s 归类到 Q2 并清除徽章 | N/A（纯 mock） |
| 分类中离开屏 | seed 后未到 4s 即离开 | 状态随 VM 生命周期保留，回屏若事件已到则显示归类态 | N/A |
| 全部筛选 | quadrantFilter=null | 显示全部四象限非空分组（既有行为） | N/A |
| 单象限筛选 | 选 Q4 | 仅显示 Q4 分组；Q4 无任务时显示空态「当前筛选无匹配任务」 | N/A |
| 默认无任务 | tasks 全空 | 显示「所有角色都很轻松，可以考虑添加新目标」 | N/A |
| reduced-motion | 系统「移除动画」开启 | Loader2 静止显示，不旋转 | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/icons/LucideIcons.kt` -- 新增 Filter / Loader2 两枚移植图标（FR-23 筛选行 + 分类中徽章）
- `companion-android/.../ui/tasks/TasksViewModel.kt` -- TasksUiState 增 `classifyingIds`/`quadrantFilter`/`applyClassified`/`grouped(filter)`；ViewModel init seed mock 分类中任务 + `selectQuadrantFilter`/`applyClassified` 动作
- `companion-android/.../ui/tasks/TasksScreen.kt` -- 新增 QuadrantFilterRow（Filter 图标 + 5 chip）+ ClassifyingBadge（Loader2 旋转 + 文案）；`grouped()` 接 filter；空态分支
- `companion-android/.../ui/AppNavHost.kt` -- TasksRoute 接 `onQuadrantFilterSelected` 回调上抛 VM
- 桌面基线（只读参考）：`hooks/useTasks.ts:29-49`、`components/butler/TaskOverviewTab.tsx:26-32,124-132,286-301`
- 测试：`app/src/test/java/com/egosync/companion/ui/tasks/TasksUiStateTest.kt`（新）-- 纯逻辑单测锁定筛选/分类中契约

## Tasks & Acceptance

**Execution:**
- [x] `ui/icons/LucideIcons.kt` -- 新增 Filter（`M22 3H2l8 9.46V19l4 2v-8.54L22 3`）+ Loader2（`M21 12a9 9 0 1 1-6.219-8.56`）-- lucide path 逐字移植
- [x] `ui/tasks/TasksViewModel.kt` -- TasksUiState 增 `classifyingIds`/`quadrantFilter`/`grouped(filter)`/`applyClassified`；ViewModel init seed mock 分类中任务 + `selectQuadrantFilter`/`applyClassified` 动作 -- 锁定 FR-23 状态机
- [x] `ui/tasks/TasksScreen.kt` -- QuadrantFilterRow + ClassifyingBadge（Loader2 旋转/reduced 静止 + 文案）挂到 TaskRow；grouped 接 filter；筛选/默认空态分支 -- 镜像桌面 TaskOverviewTab
- [x] `ui/AppNavHost.kt` -- TasksRoute 传 `onQuadrantFilterSelected = vm::selectQuadrantFilter` -- 接线
- [x] `app/src/test/.../tasks/TasksUiStateTest.kt` -- 单测：filter=null 全量、filter=Q4 仅 Q4、空象限全空、applyClassified 替换+清 classifyingIds、未涉及任务原样保留（规则九）

**Acceptance Criteria:**
- Given FR-23，when 对照 fr-groups.md 桌面基线逐项核对（功能/布局/图标），then 三项（分类中徽章/Loader2 旋转/象限筛选 chip）映射逐项通过（映射表见 Design Notes）。
- Given `./gradlew :app:assembleDebug`，when 构建，then BUILD SUCCESSFUL。
- Given `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'`，when 运行，then 通过（回归）。
- Given `./gradlew :app:testDebugUnitTest`，when 跑 TasksUiStateTest，then 通过。
- Given app main 源码，when emoji 正则扫描（同组 1/2/3），then 渲染命中为 0。

## Design Notes

**mock 触发裁决（推荐 A，见 CHECKPOINT）**：移动无任务创建功能，FR-23 分类中态的触发镜像桌面语义但走 mock——进屏 seed 一条「从对话/速记新建、未指定象限」任务（占位象限 Q3，模拟后端初值），标记 classifyingIds；`delay(4000)` 后模拟 task:classified 事件到达，applyClassified 替换为 Q2 任务并清标记。用户可见「占位组带徽章 → 跳到 Q2」的完整桌面过渡。不加新建按钮（超范围，规则二）。

**徽章色温**：桌面 indigo-50/600/100 → 移动 BrandIndigo(#6366F1)：文字 BrandIndigo、底 BrandIndigo.copy(alpha=0.08)、边框 BrandIndigo.copy(alpha=0.25)。与既有 BigRockBadge(BrandWarn) 同构（硬色 + 透明底 + 描边）。

**Loader2 旋转**：`rememberInfiniteTransition` + `Modifier.graphicsLayer { rotationZ = angle }`；`rememberReducedMotion()` 为真时直接画静态 Loader2（不挂动画），镜像桌面 motion-reduce:animate-none。

**筛选 chip 选中态**：桌面 indigo-100/700 → 移动 primaryContainer/onPrimaryContainer（主题语义等同 indigo 主色）；未选中 onSurfaceVariant + outline。单选互斥，默认「全部」。
（step-04 审查裁决：实现改用 primary/onPrimary + primary 描边——同组 1 RoleChip 母本配色，且主题 primaryContainer 为 25% 透明色做实心 chip 底会发灰；与库内既有约定一致性优先，记录于此。）

**桌面基线逐项对照（自查映射）**

| 维度 | 桌面基线 | 移动落地 | 对照 |
|------|---------|---------|------|
| 功能·分类中态 | useTasks classifyingIds + task:classified 事件替换+清标记 | TasksUiState classifyingIds + applyClassified；ViewModel mock 事件 | ✅ 等价 |
| 功能·象限筛选 | quadrantChips 全部/Q1~Q4，visibleQuadrants 过滤 | QuadrantFilterRow 5 chip + grouped(filter) | ✅ 等价 |
| 布局·徽章 | inline-flex Loader2(11px spin)+「智能分类中…」indigo 底 | Row Loader2(11dp spin)+「智能分类中…」BrandIndigo 底 | ✅ 结构对应 |
| 布局·筛选行 | Filter(14) + chip 行 | Filter(14dp) + chip 行 | ✅ 结构对应 |
| 图标 | Filter / Loader2(lucide) | LucideIcons.Filter / Loader2 移植 | ✅ 逐枚一致 |
| 空态 | 「当前筛选无匹配任务」/默认轻松文案 | 同款文案 | ✅ 一致 |

**不适用分支**：桌面 TaskOverviewTab 的归属多选/只看大石头/已完成折叠/删除确认弹窗不在 FR-23 范围（fr-groups 仅要求象限筛选），不实现。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*' --tests '*TasksUiStateTest*'` -- expected: 通过
- `cd companion-android && grep -rnP '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main | grep -v '//'` -- expected: 空输出

**Manual checks (if no CLI):**
- 无渲染环境：视觉保真度（徽章色温/旋转/筛选 chip 高亮）待人审截图比对桌面 TaskOverviewTab。

**Result (step-03/04 self-check, 2026-08-27):** ✅ `assembleDebug` BUILD SUCCESSFUL · 全量单测 45/45 XML 实证通过——TasksUiStateTest 6/6（新增）、RoleIconsTest 11/11（回归）、既有 6 套无回归 · emoji 扫描空输出 · 图标 path 经 lucide-static@0.292.0 官方 SVG 逐字比对一致（Filter 为同几何多边形等价 path）。step-04 三路对抗审查（盲审/边界/验收）：验收审计员全 AC 通过；3 条 patch 已修（mock 事件载荷改取当前卡片防勾选回滚、筛选行 LazyRow 化防窄屏裁剪、Design Notes 补 chip 选中态裁决），2 条 reject（seed id 去重——当前不可达防御属猜测性实现；重复 border import——既有瑕疵已登记 icon-parity spec 待清理）。首轮编译 2 处报错（BorderStroke import 归属、Map 可空接收者）当场修复。实现方式：直接实现（boss 避坑指示：构建不经子代理）。

## Spec Change Log

## Suggested Review Order

**FR-23 状态机（先看设计意图）**

- UiState 契约：classifyingIds/quadrantFilter/grouped/applyClassified（镜像 useTasks.ts 语义）
  [`TasksViewModel.kt:18`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L18)

- mock 事件流：seed 分类中任务 → 4s 后归类 Q2；载荷取当前卡片防勾选回滚（审查 patch）
  [`TasksViewModel.kt:60`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L60)

- 象限筛选动作（null=全部）
  [`TasksViewModel.kt:81`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L81)

**TasksScreen UI（镜像桌面 TaskOverviewTab）**

- 筛选行：Filter 图标 + 全部/Q1~Q4 chip，LazyRow 横向滚动防窄屏裁剪（审查 patch）
  [`TasksScreen.kt:120`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L120)

- 空态分支：默认/筛选双文案（逐字镜像桌面）
  [`TasksScreen.kt:91`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L91)

- 分类中徽章：BrandIndigo 色温 + Loader2 旋转/reduced-motion 静止
  [`TasksScreen.kt:324`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L324)

- Loader2 旋转动画（graphicsLayer draw-phase 读取）
  [`TasksScreen.kt:345`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt#L345)

**图标与接线**

- 新增 Filter/Loader2（lucide 0.292 path 逐字移植，经官方 SVG 比对）
  [`LucideIcons.kt:127`](../../companion-android/app/src/main/java/com/egosync/companion/ui/icons/LucideIcons.kt#L127)

- TasksRoute 接筛选回调上抛 VM
  [`AppNavHost.kt:245`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L245)

**测试**

- 6 用例锁定筛选/分类契约，含「事件不回滚本地勾选」回归（规则九）
  [`TasksUiStateTest.kt:15`](../../companion-android/app/src/test/java/com/egosync/companion/ui/tasks/TasksUiStateTest.kt#L15)

