---
title: 'companion-android onboarding 屏 FR 对齐（组 6：FR-21）'
type: 'feature'
created: '2026-08-27'
status: 'done'
baseline_commit: '84a75ef3fe5fa5c863834c177b48e3647b897d3a'
context:
  - '{project-root}/_bmad-output/specs/spec-mobile-fr-parity/SPEC.md'
  - '{project-root}/_bmad-output/specs/spec-mobile-fr-parity/fr-groups.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-companion-android-icon-parity.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 移动端无空状态引导：已配对但未完成初始设置的用户直接落入仪表盘，缺桌面 `OnboardingView.tsx:24-386` 的管家五步访谈 + `RoleConfirmModal.tsx:47` 角色提议确认流（FR-21 缺失→完整，强制语义适配：桌面全屏引导→移动全屏引导，尺寸/布局适配移动）。

**Approach:** 新建 `ui/onboarding/` 包：OnboardingScreen（头部 Home 磁贴+标题 / 气泡访谈流 / 打字机流式 / 步进 placeholder / 「跳过角色引导」链接）+ OnboardingViewModel（步进状态机：发送→思考→流式回复，第 2 轮浮现角色提案）。触发门槛=已配对且未完成引导（未配对由既有 PairingScreen 承载）；完成标记落 SharedPreferences。提案确认弹窗（24 图标+8 色板）与提案卡从 ChatScreen 抽至 `ui/components/` 共享。

## Boundaries & Constraints

**Always:**
- 零新依赖：仅 Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines + 既有 material-icons-core；图标全用既有 LucideIcons（Home/ConciergeBell/Play/X/Check），零新增移植。
- 桌面语义逐项镜像：五步 placeholder 逐字、step=min(step+1,5) 封顶、step≥5 即标记完成（中途杀进程不再重复引导）、step≥2 才显示跳过链接、确认创建后 800ms 进主界面、流式中忽略新发送/跳过。
- 完成路径三条（镜像桌面）：提案确认创建 / 「跳过角色引导」/ 第 5 步标记（留在屏上继续对话）。
- 共享抽取保持行为零变更：RoleConfirmDialog/RoleIconGrid/RoleColorPalette/parseHexColor/RoleProposalCard/ThinkingDots 从 ChatScreen 平移 `ui/components/`，ChatScreen 改 import，不改逻辑。
- 沿用既有约定：中文注释、viewModelFactory + AppModelContainer 接线、densitySpec(InfoDensity.CONVERSATIONAL)、imePadding。

**Ask First:**
- 需改 PairingScreen 成功页文案/结构时——本组仅改 onEnterApp 跳转目标（DASHBOARD→ONBOARDING），不动文案。
- 需把创建的角色写入 SnapshotStore.roles 时——不做（mock 只读约束），创建仅内存态+完成标记。

**Never:**
- 禁引入 Room / Hilt / OkHttp / 网络库 / material-icons-extended；UI 渲染零 emoji。
- 不碰组 1-5/7 屏既有行为（ChatScreen 仅做平移抽取）；不接真实连接层/后端。
- 桌面「llmReady==null 加载态」「llmReady==false 配置大模型提示」两分支不实现：移动无 LLM 配置（桌面为唯一引擎，配置在桌面端），未配对门槛由 PairingScreen 承载，mock 无异步初始化——属不可达 UI（组 4/5 同款裁决）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 首启·未配对 | paired=false | PAIRING（既有），不进引导 | N/A |
| 首启·已配对未引导 | paired=true 且 !onboarded | 起始页=ONBOARDING 全屏引导 | N/A |
| 配对成功「进入主界面」 | PairingScreen SUCCESS 点击 | 跳 ONBOARDING（清 PAIRING 栈） | N/A |
| 开场 | 进入引导屏 | 管家问候气泡（桌面 fallback 逐字） | N/A |
| 发送消息 | 第 k 条（k=1..4） | 用户气泡→思考点→打字机回复，step=k+1，placeholder 换下一条 | N/A |
| 第 5 条消息 | step=4 发送 | step 封顶 5，标记 onboarded 但留在屏上 | N/A |
| 流式中发送/跳过 | thinking/responding | 忽略（桌面 disabled 语义） | N/A |
| 提案浮现 | 第 2 轮完整回复后 | 提案卡入流（一次性守卫）；「创建角色」开弹窗 | N/A |
| 确认创建 | 弹窗「创建」（名称非空） | 弹窗关+回执气泡，800ms 后 onboarded+进 DASHBOARD | N/A |
| 空名创建 | 名称空白 | 「创建」禁用（镜像桌面 canConfirm） | N/A |
| 提案「不需要」 | 弹窗/卡片拒绝 | 卡置已跳过+回执，引导继续（桌面允许再次提议语义→mock 单次） | N/A |
| 跳过链接 | step≥2 点击 | onboarded+立即进 DASHBOARD | N/A |
| 跳过链接可见性 | step=1 | 不显示（桌面 L362） | N/A |
| 引擎离线 | Offline 态 | 输入禁用+离线提示（同 ChatScreen）；全局降级遮罩仍生效 | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/onboarding/OnboardingScreen.kt` -- 新建：全屏引导 UI（头部/气泡流/输入条+Play 发送钮/跳过链接/弹窗挂载）
- `companion-android/.../ui/onboarding/OnboardingViewModel.kt` -- 新建：`OnboardingUiState` 纯函数步进契约 + VM mock 流（思考→打字机；第 2 轮提案；三条完成路径）
- `companion-android/.../ui/components/RoleConfirmDialog.kt` -- 新建（自 ChatScreen 平移）：RoleConfirmDialog + RoleIconGrid + RoleColorPalette + parseHexColor
- `companion-android/.../ui/components/RoleProposalCard.kt` -- 新建（自 ChatScreen 平移）：提案卡
- `companion-android/.../ui/components/ThinkingDots.kt` -- 新建（自 ChatScreen 平移）：三点弹跳（含 reduced-motion 静止降级）
- `companion-android/.../ui/chat/ChatScreen.kt` -- 删除平移出的私有组件，改 import 共享件（逻辑零变更）
- `companion-android/.../AppModelContainer.kt` -- 增 KEY_ONBOARDED / isOnboarded() / completeOnboarding()
- `companion-android/.../ui/AppNavHost.kt` -- Routes.ONBOARDING + 起始页三态判定 + PairingRoute 跳转目标 + OnboardingRoute
- `companion-android/.../sync/SnapshotStore.kt` -- 增 onboarding mock（问候/5 placeholder/5 回复）；提案种子复用 roleProposal
- 桌面基线（只读参考）：`egosync-app/src/components/onboarding/OnboardingView.tsx`、`RoleConfirmModal.tsx`
- 测试：`app/src/test/java/com/egosync/companion/ui/onboarding/OnboardingUiStateTest.kt`（新）-- 步进契约单测

## Tasks & Acceptance

**Execution:**
- [x] `ui/components/` 三个共享件 -- 自 ChatScreen 平移（RoleConfirmDialog 全家/RoleProposalCard/ThinkingDots），ChatScreen 改 import -- 单一事实源，防弹窗契约分叉
- [x] `sync/SnapshotStore.kt` -- 增 onboardingGreeting（桌面 fallback 逐字）/onboardingPlaceholders(5)/onboardingReplies(5) -- mock 数据归位
- [x] `ui/onboarding/OnboardingViewModel.kt` -- UiState 纯函数（nextStep 封顶/marksCompletion/skipVisible/placeholderFor/PROPOSAL_ROUND）+ VM 三条完成路径 -- 锁定 FR-21 状态机
- [x] `ui/onboarding/OnboardingScreen.kt` -- 全屏引导 UI（镜像桌面 OnboardingView 结构）-- FR-21 主体
- [x] `AppModelContainer.kt` + `AppNavHost.kt` -- onboarded 门槛 + 路由接线（起始页三态/配对成功转引导/完成后清栈进 DASHBOARD）
- [x] `OnboardingUiStateTest.kt` -- 步进封顶/完成标记/跳过可见性/placeholder 越界防护（规则九）

**Acceptance Criteria:**
- Given FR-21，when 对照 fr-groups.md 桌面基线逐项核对（功能/布局/图标），then 逐项映射通过（映射表见 Design Notes）。
- Given `./gradlew :app:assembleDebug`，when 构建，then BUILD SUCCESSFUL。
- Given `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*' --tests '*OnboardingUiStateTest*'`，when 运行，then 通过。
- Given app main 源码，when emoji 正则扫描（同组 1-5），then 渲染命中为 0。

## Spec Change Log

## Design Notes

**触发门槛裁决**：FR-21「未配对/无角色」在移动拆为两段——未配对=既有 PairingScreen（不动）；无角色/未完成引导=新 ONBOARDING（paired && !onboarded，prefs 持久）。移动无桌面 isFirstLaunch 的 DB 语义，prefs 布尔等价（原型粒度）。

**提案交互形态**：桌面 role:proposed 事件直开弹窗；移动沿组 3 已确立的 FR-5 惯用法（提案卡入流+按钮开弹窗），产品内自洽优先。提案种子复用 `SnapshotStore.roleProposal`（策划师），不另造数据（规则二）。

**桌面基线逐项对照（自查映射）**

| 维度 | 桌面基线（OnboardingView.tsx） | 移动落地 | 对照 |
|------|-------------------------------|---------|------|
| 功能·空态触发 | isFirstLaunch → onboard 视图 | paired && !onboarded → ONBOARDING 路由 | ✅ 语义等价 |
| 功能·五步访谈 | step+onboardingStep 发送，placeholder 步进 | 同款步进状态机+placeholder 逐字 | ✅ 等价 |
| 功能·流式回复 | llm:stream 打字机+thinking | 思考点→打字机（mock） | ✅ 等价 |
| 功能·角色提议 | role:proposed→RoleConfirmModal | 第 2 轮提案卡→确认弹窗（移动惯用法） | ✅ 形态适配 |
| 功能·完成路径 | 创建/跳过/第 5 步标记 | 三条同款（800ms 延迟镜像） | ✅ 等价 |
| 布局·头部 | h-76px header：Home 磁贴+「数字分身管家」 | 同构头部（移动密度 token） | ✅ 结构对应 |
| 布局·输入条 | 底部输入+Play 发送钮+右下跳过链接 | 同构（imePadding 适配软键盘） | ✅ 结构对应 |
| 布局·配置提示分支 | llmReady==false 配置大模型 | 不实现（不可达：移动无 LLM 配置，未配对由 PairingScreen 承载） | ✅ 语义合并 |
| 图标 | Home/Play/(X/Check 于弹窗) | LucideIcons 同名既有，零新增 | ✅ 逐枚一致 |

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*' --tests '*OnboardingUiStateTest*'` -- expected: 通过
- `cd companion-android && grep -rnP '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main | grep -v '//'` -- expected: 空输出

**Manual checks (if no CLI):**
- 无渲染环境：引导屏视觉保真度（头部磁贴/气泡/输入条/弹窗网格）待人审截图比对桌面 OnboardingView。

**Result (step-03/04 self-check, 2026-08-27):** ✅ `assembleDebug` BUILD SUCCESSFUL · 全量单测 57/57 XML 实证通过——OnboardingUiStateTest 5/5（新增）、RoleIconsTest 11/11（回归）、既有无回归 · emoji 扫描空输出。step-04 三路对抗审查（盲审/边界/验收）：验收审计员全 AC 通过裁决 ACCEPT（diff 与磁盘逐字节一致、共享抽取经基线 `git show` 逐字比对零行为变更、桌面映射表逐行打开桌面代码验证属实）；三路 findings 去重后 3 patch（confirmRoleProposal 完成标记反序——镜像桌面 L123-124 先落标记再 delay(800)，防 800ms 窗口杀进程丢标记；RoleProposalCard 平移多出的 modifier 形参移除恢复纯平移；spec frontmatter baseline_commit 哈希笔误修正）已修，11 reject（busy 守卫/unpair 不重置/placeholder 索引/onDismiss 合流等——均为桌面忠实镜像、既有模式或不可达路径，两路审查交叉确认）。实现方式：直接实现（boss 避坑指示：构建不经子代理）。

## Suggested Review Order

**FR-21 状态机（先看设计意图）**

- UiState 纯函数契约：步进封顶/完成标记/跳过可见性/placeholder 索引（镜像桌面 L205/L213-217/L351/L362）
  [`OnboardingViewModel.kt:37`](../../companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingViewModel.kt#L37)

- 三条完成路径之一：发送步进到第 5 步即落标记但留在屏上（桌面 L213-217 防杀进程重复引导）
  [`OnboardingViewModel.kt:81`](../../companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingViewModel.kt#L81)

- 审查修正点：确认创建路径先同步落完成标记、再 delay(800) 导航（镜像桌面 L123-124 顺序）
  [`OnboardingViewModel.kt:162`](../../companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingViewModel.kt#L162)

**OnboardingScreen UI（镜像桌面 OnboardingView 结构）**

- 头部：Home 磁贴 + 「数字分身管家」标题 + 分隔线（桌面 h-76 header 同构）
  [`OnboardingScreen.kt:115`](../../companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingScreen.kt#L115)

- 输入条：步进 placeholder + Play 发送钮 + 右下「跳过角色引导」链接（桌面 L342-374）
  [`OnboardingScreen.kt:163`](../../companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingScreen.kt#L163)

- FR-5 提案卡与确认弹窗挂载（复用组 3 共享组件，移动惯用法承载桌面 role:proposed）
  [`OnboardingScreen.kt:212`](../../companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingScreen.kt#L212)

**路由与门槛接线**

- 起始页三态：未配对→PAIRING / 已配对未引导→ONBOARDING / 否则 DASHBOARD
  [`AppNavHost.kt:90`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L90)

- 配对成功「进入主界面」按引导状态分流（Ask-First：不改 PairingScreen 本体）
  [`AppNavHost.kt:164`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L164)

- OnboardingRoute：完成路径统一出口清栈进 DASHBOARD（viewModelFactory 接线）
  [`AppNavHost.kt:176`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L176)

- 完成标记持久化（镜像桌面 isFirstLaunch 语义，prefs 布尔等价）
  [`AppModelContainer.kt:70`](../../companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt#L70)

**共享组件抽取（对既有文件的唯一触碰）**

- 角色确认弹窗（24 图标网格 + 8 色板，自 ChatScreen 逐字平移仅放宽可见性）
  [`RoleConfirmDialog.kt:50`](../../companion-android/app/src/main/java/com/egosync/companion/ui/components/RoleConfirmDialog.kt#L50)

- 提案卡与思考点（平移共享；ChatScreen 改 import，逻辑零变更）
  [`RoleProposalCard.kt:39`](../../companion-android/app/src/main/java/com/egosync/companion/ui/components/RoleProposalCard.kt#L39)
  [`ThinkingDots.kt:29`](../../companion-android/app/src/main/java/com/egosync/companion/ui/components/ThinkingDots.kt#L29)

- ChatScreen 调用点原样（抽取后行为回归由全量单测与逐字 diff 共同背书）
  [`ChatScreen.kt:175`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L175)

**数据与测试**

- onboarding mock：问候（桌面 fallback 逐字）/五步 placeholder/五条回复
  [`SnapshotStore.kt:738`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L738)

- 步进契约单测：封顶/完成标记/跳过可见性/越界防护（规则九 WHY 注释）
  [`OnboardingUiStateTest.kt:14`](../../companion-android/app/src/test/java/com/egosync/companion/ui/onboarding/OnboardingUiStateTest.kt#L14)
