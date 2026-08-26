---
title: 'companion-android 设计语言补齐——密度双模式 / 动效克制 / 色温偏移'
type: 'feature'
created: '2026-08-26'
status: 'done'
baseline_commit: '176289d04ffe16326f0b9432b395a429db6d7feb'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/mobile-desktop-parity-investigation.md'
  - '{project-root}/egosync-app/src/index.css'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** deferred 清单 #2（调查 A3）：色彩对齐后仍缺三件——密度双模式未显式化（chat 轻/dashboard 密是各屏自选值）；`prefers-reduced-motion` 零支持；PairingScreen 残留唯一违规装饰动画（扫掠线）且角色切换色温瞬变、缺桌面 `--duration-color:300ms` 过渡。

**Approach:** theme 层新建 Density/Motion token（含 Android 版 reduced-motion 检测）；chat/dashboard 声明式消费；移除扫掠线；呼吸/思考点/转场接入 reduced-motion 静态降级；角色卡 accent 与指示点以 300ms 过渡渐变色温。零新依赖；预期零视觉回归（有意变更仅扫掠线移除、指示点域色两处）。

## Boundaries & Constraints

**Always:**
- 动效白名单封闭：呼吸（全周期 3s）、思考弹跳点（1.4s 功能反馈）、导航转场（220/180ms）、色温 300ms 单次过渡。reduced-motion 时全部静态化：呼吸定格 alpha 0.6（桌面冻结帧）、思考点静止、转场 None、色温直切。
- reduced-motion 检测 = `Settings.Global.ANIMATOR_DURATION_SCALE == 0f`（Android「移除动画」无障碍开关，web `prefers-reduced-motion` 平台等价物），读取一次 remember 缓存。
- 密度 token 逐字段注明桌面锚点；消费点取值=各屏现状值——本次是系统化，不重调视觉。

**Ask First:** 发现第四处循环动画或需改呼吸曲线本身（easing/alpha/周期）→ HALT；token 化迫使消费点偏离现状值 → HALT 给前后对照。

**Never:** 不引入任何新依赖；不碰 ViewModel 业务逻辑/SnapshotStore 契约/导航结构；不做运行时密度设置项（PRD §4.14 是上下文自然切换，非开关）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 动效正常（默认） | ANIMATOR_DURATION_SCALE=1 | 呼吸 3s 循环、思考点 1.4s、转场 220/180ms、色温 300ms | N/A |
| 移除动画开启 | ANIMATOR_DURATION_SCALE=0 | 呼吸定格 0.6、思考点静止、转场瞬时、色温直切 | getFloat 带 1f 默认，未设置安全回落 |
| densitySpec(CONVERSATIONAL) | — | (10, 16, 12)dp | N/A |
| densitySpec(CONSOLE) | — | (10, 12, 10)dp | N/A |

</frozen-after-approval>

## Code Map

- `ui/theme/Motion.kt` / `ui/theme/Density.kt` -- 新建：动效常量与 reduced-motion 检测 / `InfoDensity`+`DensitySpec`+`densitySpec(mode)`
- `ui/chat/ChatScreen.kt`、`ui/dashboard/DashboardScreen.kt`、`pairing/PairingScreen.kt`、`ui/AppNavHost.kt` -- 消费与清理点（见 Tasks）
- `README.md`（companion-android 根）-- 对照表追加三行
- `app/src/test/java/com/egosync/companion/ui/theme/DensitySpecTest.kt` -- 新建契约单测

## Tasks & Acceptance

**Execution:**
- [x] `ui/theme/Motion.kt` -- 新建 `BreathDurationMillis=3000`（--duration-breath）、`ColorTransitionMillis=300`（--duration-color）、`rememberReducedMotion()`（读 Settings.Global 后 remember 缓存）
- [x] `ui/theme/Density.kt` -- 新建 `InfoDensity.CONVERSATIONAL/CONSOLE` + `densitySpec(mode)`，逐字段桌面锚点注释（CONVERSATIONAL←ChatStream space-y-3 / ChatBubble px-5 py-3 移动近似档=现状；CONSOLE←DashboardTab gap-2.5 / px-3 py-2.5，供 FR-38 指标网格复用）
- [x] `ui/chat/ChatScreen.kt` -- LazyColumn spacedBy 与气泡内边距改消费 CONVERSATIONAL；ThinkingDots 在 reducedMotion 下 lift 恒 0（三点静止，提取 ThinkingDot 复用渲染）。**实现注记：ThinkingBubble 容器内边距保持字面 16/14 未消费 token——现状 14dp ≠ unitPaddingY 12dp，消费会破坏 frozen「零视觉变化」约束；按 Always 层优先于任务措辞处理**
- [x] `ui/dashboard/DashboardScreen.kt` -- BreathingEnergyBar 用 BreathDurationMillis 并在 reducedMotion 时跳过 transition 定格 0.6f；RoleCardItem 图标底色与徽章 tint 用 `animateColorAsState(tween(ColorTransitionMillis))`；活动指示点颜色改为当前页域 accent 同规格过渡；OverviewEntry 垂直内边距消费 CONSOLE.unitPaddingY
- [x] `pairing/PairingScreen.kt` -- 删 scanline 无限动画与扫掠线 Box，保留静息框+四角标记+文案，清理失效 import（LinearEasing/RepeatMode/animateFloat/infiniteRepeatable/rememberInfiniteTransition/tween/offset/Brush；BrandIndigoLight 因 CornerMarks 处另有使用保留）（人审裁决 2026-08-26）
- [x] `ui/AppNavHost.kt` -- enter/popEnter = if(reduced) EnterTransition.None else fadeIn(tween(220))，exit/popExit 同理 fadeOut(180)
- [x] `README.md` -- 对照表追加三行：密度 token 及锚点、动效白名单+降级、色温过渡
- [x] `DensitySpecTest.kt` -- 断言两模式互异且 CONVERSATIONAL 各字段 ≥ CONSOLE 对应值——锁「轻≥密」空间契约：重调数值时测试必须被迫一起评审

**Acceptance Criteria:**
- Given `./gradlew :app:assembleDebug`，when 执行，then BUILD SUCCESSFUL。
- Given `grep -rn "rememberInfiniteTransition" app/src/main/java --include=*.kt | grep -cv "import"`，when 执行，then 恰 2（呼吸/思考点调用点；原写法含 import 行会得 4——评审修正）。
- Given `grep -n "scanline" pairing/PairingScreen.kt`，when 执行，then 0 命中。
- Given `./gradlew :app:testDebugUnitTest --tests '*DensitySpecTest*'`，when 执行，then 通过。
- Given 动效节奏/色温过渡/密度气质的视觉保真，then 无法自动验证——如实标注待人审。

## Spec Change Log

## Design Notes

- 不用 CompositionLocal 提供密度：仅两屏静态消费，函数取值足够；出现运行时切换需求再升级。
- 指示点域色温化是有意变更之一：PRD「角色切换时微妙色温变化」在移动 Pager IA 的落点；人审不喜可一行回退通用 primary。
- 呼吸定格 0.6 依据桌面 reduced-motion 冻结 `.breathe` 关键帧端点 opacity 0.6；半程 1500ms×Reverse=3s 全周期与 CSS 等效，保持既有实现仅提常量。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*DensitySpecTest*'` -- expected: 通过
- `cd companion-android && grep -rn "rememberInfiniteTransition" app/src/main | wc -l` -- expected: 2
- `grep -rn "scanline" app/src/main` -- expected: 空

**Manual checks (待人审):**
- 开启「移除动画」→ 呼吸静止于暗态、思考点静止、切页瞬时、**色温直切不渐变（角色卡图标底/徽章 tint/页指示点三处）**；恢复后呼吸约 3s、滑动角色卡指示点色温 300ms 渐变。
- 截图对比桌面 chat vs dashboard 的轻/密气质分野。

**Result (step-03 self-check, 2026-08-26):** ✅ `assembleDebug` BUILD SUCCESSFUL（57s）· 全量 `testDebugUnitTest` 通过（含 DensitySpecTest 3 用例）· `rememberInfiniteTransition` 使用点恰 2（呼吸/思考点）· scanline 0 命中。实现方式：直接实现（遵环境事实：子代理跑多分钟 gradle 会静默失败）。

**Review patches (step-04, 2026-08-26):** 三方评审（盲审/边界猎手/验收审计）唯一实质发现 = 三处 `animateColorAsState(tween(300))` 未接 reduced-motion 门控，违背 frozen「色温直切」→ 已修：`animationSpec = if (rememberReducedMotion()) snap() else tween(ColorTransitionMillis)`（指示点/roleAccent/roleTint 共用 spec），修复后 assembleDebug + 全量单测复验通过。其余发现分类：defer 1（reduced-motion 活跟踪）、reject 10（无上下文误报或已批准的设计裁量），无 intent_gap/bad_spec 回环。

**Pending human review:**
1. 动效降级实测：需模拟器/真机开启「移除动画」（开发者选项动画缩放置 0 或无障碍开关）人工确认呼吸定格 0.6、思考点静止、转场瞬时——无自动化手段。
2. 色温过渡与指示点域色：滑动角色卡观察 300ms 渐变是否达「微妙」预期；指示点域色为有意视觉变更，不喜可一行回退通用 primary。
3. 密度双模式气质：截图对比桌面 chat/dashboard 确认轻/密分野成立。
4. ThinkingBubble 内边距保持字面 16/14（见 Execution 实现注记），如需对齐 token 值 12dp 属视觉变更，请人审定夺。

## Suggested Review Order

**动效白名单与 reduced-motion 降级（核心契约）**

- 白名单常量单一事实源 + 平台版 prefers-reduced-motion 检测
  [`Motion.kt:15`](../../companion-android/app/src/main/java/com/egosync/companion/ui/theme/Motion.kt#L15)

- 呼吸降级分支：跳过无限过渡、定格母本关键帧端点 0.6
  [`DashboardScreen.kt:321`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt#L321)

- 思考弹跳点静态早退，ThinkingDot 提取复用渲染
  [`ChatScreen.kt:217`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L217)

- 导航转场瞬时化（EnterTransition.None）
  [`AppNavHost.kt:91`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L91)

**色温随角色域偏移（300ms 单次过渡）**

- 角色卡 accent/tint 门控渐变：reduced 时 snap 直切
  [`DashboardScreen.kt:227`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt#L227)

- 页指示点取当前页域 accent——「微妙色温变化」的移动落点
  [`DashboardScreen.kt:107`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt#L107)

**信息密度双模式 token**

- 双模式枚举与取值映射，逐字段桌面锚点注释
  [`Density.kt:10`](../../companion-android/app/src/main/java/com/egosync/companion/ui/theme/Density.kt#L10)

- 对话流轻量消费：流距 + 气泡内边距（数值=现状零视觉变化）
  [`ChatScreen.kt:70`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L70)

- 控制台密集消费：概览入口竖距
  [`DashboardScreen.kt:199`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt#L199)

**装饰动效清零**

- 扫掠线移除后的静息取景框（人审裁决项）
  [`PairingScreen.kt:147`](../../companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt#L147)

**外围**

- 对照表新增密度/动效白名单/色温三行
  [`README.md:99`](../../companion-android/README.md#L99)

- 「轻≥密」空间契约测试
  [`DensitySpecTest.kt:14`](../../companion-android/app/src/test/java/com/egosync/companion/ui/theme/DensitySpecTest.kt#L14)
