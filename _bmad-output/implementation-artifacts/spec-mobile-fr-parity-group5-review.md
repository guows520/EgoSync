---
title: 'companion-android review 屏 FR 对齐（组 5：FR-17）'
type: 'feature'
created: '2026-08-27'
status: 'done'
baseline_commit: 'eca358fae648861f7fa4ea5161242d08efe7a28c'
context:
  - '{project-root}/_bmad-output/specs/spec-mobile-fr-parity/SPEC.md'
  - '{project-root}/_bmad-output/specs/spec-mobile-fr-parity/fr-groups.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-companion-android-icon-parity.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 移动端 WeeklyReviewScreen 是纯只读成绩单（叙事/能量趋势/大石头推进结果），缺桌面 WeeklyReviewModal plan 阶段的完整规划交互：建议采纳、逐行输入、添加/移除、确认保存。FR-17 处于「部分→完整」补全状态（勾选=确认规划的 Check、添加=Plus、移除=X，按桌面基线整体镜像）。

**Approach:** 按桌面基线 `WeeklyReviewModal.tsx:181-262`（plan 阶段）+ `useBigRockPlanning.ts`（加载建议/保存状态机）落地。新建 WeeklyReviewViewModel 持 phase/planStates/suggestions/isLoading/isSaving；进 plan 阶段 mock 延迟出建议（产品经理/父亲有、学习者无→触发「手动填写」分支）；WeeklyReviewScreen 增两阶段切换（review 底部「规划下周大石头」入口 + plan 阶段角色卡/输入行/底部双按钮）；SnapshotStore 增 BigRockPlanItem 类型与建议 mock；LucideIcons 增 Plus。

## Boundaries & Constraints

**Always:**
- 零新依赖：仅 Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines + 既有 material-icons-core（mock 延迟用 viewModelScope + delay）。
- 数据只读增补 SnapshotStore（不动既有 weeklyReview mock 文案/字段）；planStates/建议加载/保存态为 ViewModel 内存态，进程重启还原（同组 1-4 mock 不持久原则）。
- 图标一律 LucideIcons.kt：新增 Plus（lucide path 逐字移植，viewport 24、strokeWidth 2、stroke-only）；建议行桌面 💡 emoji → LucideIcons.Lightbulb 替代（禁 emoji 硬约束的强制适配）；Check/X/ArrowRight/Lightbulb 已有。
- 文案与桌面逐字一致：info 条「为每个角色设定下周最重要的大石头，系统会在日程中优先为它们保留时间。」、「正在思考建议...」、「采纳」、「手动填写本周大石头」、输入 placeholder「{角色名} 下周重要的事...」、「添加」、「返回复盘」、「确认规划」、「保存中...」、入口按钮「规划下周大石头」、标题「规划下周大石头」。
- adoptSuggestion 语义逐字镜像桌面 L84-97：填首个空行，无空行则追加。
- 沿用既有约定：中文注释、viewModelFactory + AppModelContainer 接线、indigo 色温（BrandIndigo 硬色 + 透明底 + 描边，同组 4 徽章母本）。

**Ask First:**
- 需改 review 阶段既有展示区块（叙事/成绩单/能量图/大石头推进）时——只允许追加底部入口按钮，不动既有内容。
- 需把确认结果写入 SnapshotStore/tasks（跨屏联动）时——本组不做，确认仅内存态 + pop 返回。

**Never:**
- 禁引入 Room / Hilt / OkHttp / 网络库 / material-icons-extended。
- 不碰组 1-4/6-7 屏与既有行为；不实现真实保存（桌面 review_plan_bigrocks 后端调用不在移动范围）。
- 桌面 planError 红色错误条不实现：mock 无失败路径，属不可达 UI（组 4 同款裁决）。
- UI 渲染不出现 emoji（注释除外）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 进 plan 阶段 | 点 review 底部「规划下周大石头」 | phase=PLAN，先显「正在思考建议...」占位，~1.5s 后出建议 | N/A（纯 mock） |
| 有建议角色 | role-pm / role-father | 建议行：Lightbulb + 文案 + 「采纳」按钮 | N/A |
| 无建议角色 | role-learner（mock 无建议） | 「手动填写本周大石头」占位框 | N/A |
| 采纳·有空行 | items=["",""] 时采纳 X | 首个空行变为 X，不新增行 | N/A |
| 采纳·无空行 | items=["a"] 时采纳 X | 行尾追加 X | N/A |
| 移除按钮可见性 | 当前角色仅 1 行 | 不显示 X 按钮（镜像桌面 items.length>1 才渲染） | N/A |
| 添加 | 点「添加」 | 该角色行尾追加空行 | N/A |
| 确认·全空白 | 所有角色均为空行 | 点击无效果（镜像桌面 items.length===0 return） | N/A |
| 确认·有内容 | ≥1 条非空 | 按钮转「保存中...」并禁用，~1s 后 pop 回上一屏（镜像桌面 savePlan→onClose） | N/A |
| 阶段往返 | 输入后返回复盘再进规划 | planStates 保留；建议重新加载（镜像桌面 phase useEffect 每次 load） | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/icons/LucideIcons.kt` -- 新增 Plus 一枚移植图标（添加行按钮）
- `companion-android/.../sync/SnapshotStore.kt` -- 新增 `BigRockPlanItem(roleId, title)` 类型 + `bigRockSuggestions: Map<String, List<String>>` mock（PM/父亲各 2 条，学习者缺席）
- `companion-android/.../ui/review/WeeklyReviewViewModel.kt` -- 新建：`ReviewPhase`/`RolePlanState`/`WeeklyReviewUiState`（纯函数 updateItem/removeItem/addItem/adoptSuggestion/plannedItems）+ VM（enterPlan mock 延迟加载建议、confirmPlan mock 保存后回调 pop）
- `companion-android/.../ui/review/WeeklyReviewScreen.kt` -- 增 plan 阶段 UI（info 条/角色卡/建议行/输入行+X/添加/底部双按钮）+ review 阶段底部入口按钮 + TopAppBar 标题随 phase 切换
- `companion-android/.../ui/AppNavHost.kt` -- WeeklyReviewRoute 接 container + viewModelFactory，回调下传
- 桌面基线（只读参考）：`egosync-app/src/components/modals/WeeklyReviewModal.tsx:181-262`、`hooks/useBigRockPlanning.ts`
- 测试：`app/src/test/java/com/egosync/companion/ui/review/WeeklyReviewUiStateTest.kt`（新）-- 纯逻辑单测锁定规划状态机

## Tasks & Acceptance

**Execution:**
- [x] `ui/icons/LucideIcons.kt` -- 新增 Plus（`M5 12h14` + `M12 5v14`）-- lucide path 逐字移植
- [x] `sync/SnapshotStore.kt` -- 增 `BigRockPlanItem` + `bigRockSuggestions` mock -- 建议 join 锚点 + 「手动填写」分支种子
- [x] `ui/review/WeeklyReviewViewModel.kt` -- 新建 UiState 契约（镜像 useBigRockPlanning 状态机 + Modal L80-115 规划操作）+ VM mock 流 -- 锁定 FR-17 状态机
- [x] `ui/review/WeeklyReviewScreen.kt` -- plan 阶段全量 UI + review 底部入口 + 标题切换 -- 镜像桌面 Modal L177-179/L181-262
- [x] `ui/AppNavHost.kt` -- WeeklyReviewRoute 接 VM -- 接线
- [x] `app/src/test/.../review/WeeklyReviewUiStateTest.kt` -- 单测：采纳填空/采纳追加/移除/添加/plannedItems 过滤空白/全空确认无动作（规则九）

**Acceptance Criteria:**
- Given FR-17，when 对照 fr-groups.md 桌面基线逐项核对（功能/布局/图标），then 逐项映射通过（映射表见 Design Notes）。
- Given `./gradlew :app:assembleDebug`，when 构建，then BUILD SUCCESSFUL。
- Given `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'`，when 运行，then 通过（回归）。
- Given `./gradlew :app:testDebugUnitTest`，when 跑 WeeklyReviewUiStateTest，then 通过。
- Given app main 源码，when emoji 正则扫描（同组 1-4），then 渲染命中为 0。

## Spec Change Log

## Design Notes

**角色名色温裁决**：桌面 plan 阶段角色名用 `normalizeColorHex(role.color)`（每角色自选色）；移动 RoleCard 无 color 字段，用既有 `RoleDomain.accent().accent`（PM=靛蓝/父亲=琥珀/学习者=紫罗兰）——Color.kt:65 已注明「接真实数据后改为角色自带 color 字段」，不为此加字段（规则三），色温语义等价。

**建议数据形态**：桌面 `RoleBigRockSuggestions[]`（含 roleName 冗余）→ 移动 `Map<roleId, List<String>>`：roles 与建议同源 join，无需冗余 roleName（规则二）。

**确认后行为**：桌面 `savePlan` 成功 → `onClose()` 关 Modal → 移动等价 pop 回上一屏；VM `confirmPlan(onSaved)` 回调式（Main dispatcher 内调用）。

**Plus 缺失**：fr-groups 称「Check/Plus/X 已有」不实（Plus 不在 51 枚内），照组 4 Filter/Loader2 增补模式处理。

**桌面基线逐项对照（自查映射）**

| 维度 | 桌面基线（Modal L181-262） | 移动落地 | 对照 |
|------|---------------------------|---------|------|
| 功能·阶段切换 | review 底部「规划下周大石头」→ plan；plan「返回复盘」 | 同款入口/返回按钮 + phase 状态 | ✅ 等价 |
| 功能·建议采纳 | loadSuggestions + adoptSuggestion 填空/追加 | VM mock 延迟加载 + 同语义纯函数 | ✅ 等价 |
| 功能·添加/移除 | Plus 添加；X 移除（>1 行才显示） | 同款（Plus 新增图标） | ✅ 等价 |
| 功能·确认 | 过滤空白→savePlan→onClose；isSaving 禁用 | plannedItems 过滤空白→mock 保存→pop；同禁用 | ✅ 等价 |
| 布局·info 条 | indigo-50 底/indigo-100 边/indigo-800 字 | BrandIndigo alpha 0.08/0.25/硬色 | ✅ 结构对应 |
| 布局·角色卡 | 白底卡 + 角色名着色 + 建议区 + 输入区 | Surface + 域色温角色名 + 同构分区 | ✅ 结构对应 |
| 图标 | Check/Plus/X（+建议行 💡） | LucideIcons 同名移植（💡→Lightbulb，禁 emoji 适配） | ✅ 逐枚一致 |

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*' --tests '*WeeklyReviewUiStateTest*'` -- expected: 通过
- `cd companion-android && grep -rnP '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main | grep -v '//'` -- expected: 空输出

**Manual checks (if no CLI):**
- 无渲染环境：plan 阶段视觉保真度（info 条色温/角色卡布局/按钮态）待人审截图比对桌面 WeeklyReviewModal。

**Result (step-03/04 self-check, 2026-08-27):** ✅ `assembleDebug` BUILD SUCCESSFUL（首轮零编译错误）· 全量单测 52/52 XML 实证通过——WeeklyReviewUiStateTest 7/7（新增）、RoleIconsTest 11/11（回归）、既有无回归 · emoji 扫描空输出 · Plus path 经 lucide@0.292.0 官方源码（unpkg）逐字比对一致。step-04 三路对抗审查（盲审/边界/验收）：验收审计员全 AC 通过裁决 ACCEPT；13+1 条 findings 去重后 3 patch（enterPlanPhase 陈旧协程竞态——loadJob 取消，同 ChatViewModel.streamJob 模式；plan 阶段 imePadding 防键盘遮挡，同 ChatScreen 模式；updateItem 补测）已修，10 reject（含桌面本就「手动填写本周大石头」的逐字镜像、桌面 Modal Esc=关整模态故不加 BackHandler、全空白零反馈/重复采纳无去重为桌面同款忠实镜像、rest 为库内既有模式或不可达防御）。实现方式：直接实现（boss 避坑指示：构建不经子代理）。

## Spec Change Log

## Suggested Review Order

**FR-17 状态机（先看设计意图）**

- UiState 契约：phase/planStates/suggestions + 5 个纯函数（镜像 useBigRockPlanning + Modal L80-115）
  [`WeeklyReviewViewModel.kt:26`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewViewModel.kt#L26)

- mock 建议流：重进 plan 先取消在途 loadJob，防陈旧协程打破占位时序（审查 patch）
  [`WeeklyReviewViewModel.kt:103`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewViewModel.kt#L103)

- 确认规划守卫：全空白/保存中双守卫 + mock 保存后回调 pop（镜像 handleConfirmPlan L99-115）
  [`WeeklyReviewViewModel.kt:130`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewViewModel.kt#L130)

**WeeklyReviewScreen UI（镜像桌面 Modal L181-262）**

- 阶段分发 + 标题切换：review 只读成绩单 / plan 规划交互
  [`WeeklyReviewScreen.kt:63`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt#L63)

- review 底部「规划下周大石头」入口（镜像桌面 L177-179）
  [`WeeklyReviewScreen.kt:250`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt#L250)

- plan 阶段骨架：info 条 + 角色卡列表 + imePadding + 底部双按钮（镜像 L183-262）
  [`WeeklyReviewScreen.kt:266`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt#L266)

- 角色规划卡：建议三分支（加载中/建议行+采纳/手动填写占位，镜像 L194-212）
  [`WeeklyReviewScreen.kt:341`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt#L341)

- 建议行外壳：BrandIndigo 色温（桌面 indigo-50/100）+ Lightbulb 替代桌面 emoji
  [`WeeklyReviewScreen.kt:468`](../../companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt#L468)

**数据与图标**

- 建议 mock：PM/父亲各 2 条，学习者缺席触发「手动填写」分支
  [`SnapshotStore.kt:724`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L724)

- 新增 Plus（lucide 0.292 path 逐字移植，经官方源码比对）
  [`LucideIcons.kt:130`](../../companion-android/app/src/main/java/com/egosync/companion/ui/icons/LucideIcons.kt#L130)

- WeeklyReviewRoute 接 VM（viewModelFactory + 确认后 pop 返回）
  [`AppNavHost.kt:302`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L302)

**测试**

- 7 用例锁定规划契约（采纳填空/追加、定向更新/移除、过滤空白、全空无动作）（规则九）
  [`WeeklyReviewUiStateTest.kt:21`](../../companion-android/app/src/test/java/com/egosync/companion/ui/review/WeeklyReviewUiStateTest.kt#L21)
