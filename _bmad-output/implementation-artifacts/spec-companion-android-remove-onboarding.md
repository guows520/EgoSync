---
title: 'companion-android 移除引导流程（FR-21 移动侧退役）'
type: 'chore'
created: '2026-09-10'
status: 'in-review'
baseline_commit: '294a8a0c782716e7e0d7b0e28d86911eb8bfdf0b'
route: 'dispatch'
review_loop_iteration: 0
context:
  - '{project-root}/companion-android/README.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 手机端不应有引导流程：桌面是唯一事实源，初始设置（五步访谈、首角色创建）在桌面完成即可；现状却是已配对未引导的用户被拦入 OnboardingScreen（FR-21 移动侧、spec-mobile-fr-parity 组 6 落地物）。

**Approach:** 整体移除 `ui/onboarding/` 包与全部接线（起始页三态归并两态、配对成功直达 DASHBOARD、删 onboarded 标记方法）；共享组件保留（ChatScreen 仍用）。按治理惯例对旧规格、契约文档与 PRD 做「移除裁决」标注，冻结正文不改写。

## Boundaries & Constraints

**Always:**
- 路由两态：`!paired → PAIRING`，否则 `DASHBOARD`；配对完成清配对栈直达 `DASHBOARD`。
- 三共享件（RoleConfirmDialog/RoleProposalCard/ThinkingDots）代码零变更，仅 KDoc 摘「（组 6 onboarding 复用）」。
- 治理标注仿既有先例（见 Design Notes）：状态行后缀、PRD 日期块引、行内日期批注，只增不删原文。
- 注释文档中文；提交 `refactor(companion): …`，提交范围仅限本任务触及文件。

**Never:**
- 不动 `egosync-app/`（桌面 FR-21 引导保留）；不动 ChatScreen/ChatViewModel/待发箱（引导的待发箱排除是结构性的，无代码牵连）。
- 不清理遗留 `onboarded` prefs 键（删读写方后即无害，不为一次性清理加代码）。
- 不改 README 与 architecture.md（均未把引导屏列为移动行为；README 对照表 L97 行是配对流步进节奏母本来源，历史事实仍成立）。
- 不重写冻结正文；不顺手优化相邻代码。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 冷启动·未配对 | paired=false | 配对流（不变项） | N/A |
| 冷启动·已配对 | paired=true（onboarded 任意值） | 直达 DASHBOARD 主壳 | N/A |
| 配对完成 | onEnterApp | 清配对栈直达 DASHBOARD | N/A |
| 解除配对→再配对 | unpair 后重扫 | PAIRING→…→DASHBOARD（现状即如此，行为归一） | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/onboarding/`（OnboardingScreen/OnboardingViewModel/OnboardingDemoData.kt）与 `app/src/test/.../ui/onboarding/OnboardingUiStateTest.kt` -- 整包删除
- `companion-android/.../ui/AppNavHost.kt` -- 摘接线：imports L60-61、`Routes.ONBOARDING` L88、起始页分支 L104-110、composable 注册 L140、onEnterApp 目标 L259-267、`OnboardingRoute` L277-302；注释 L85/L103/L261 同步
- `companion-android/.../AppModelContainer.kt` -- 删 `isOnboarded()`/`completeOnboarding()`（L147-152）与 `KEY_ONBOARDED`（L188）；调用方全在待删代码内
- `companion-android/.../ui/components/{RoleConfirmDialog,RoleProposalCard,ThinkingDots}.kt` -- 三处同款 KDoc「自 ChatScreen 平移共享（组 6 onboarding 复用），逻辑零变更。」摘括号句；ChatScreen 用例不动
- 治理标注：`_bmad-output/implementation-artifacts/spec-mobile-fr-parity-group6-onboarding.md` 状态行加 superseded-by；`_bmad-output/specs/spec-mobile-fr-parity/SPEC.md` L41-43/L54/L69 与 `fr-groups.md` L133-142 加退役批注；`_bmad-output/specs/spec-companion-connection-chat-ux/`（SPEC.md L90、task-breakdown.md L11、state-and-recovery-model.md L108）加失效批注；`_bmad-output/planning-artifacts/prd-egosync.md` §4.14 层级表（L606 附近）加日期块引（FR-21 移出移动核心体验）
- `egosync-app/` -- 只读参考，零改动

## Tasks & Acceptance

**Execution:**
- [x] 删 `ui/onboarding/` 3 文件 + `OnboardingUiStateTest.kt` -- 主体退役
- [x] `AppNavHost.kt` 摘接线、归并两态、onEnterApp 直达 DASHBOARD、删 OnboardingRoute -- 引导不可达
- [x] `AppModelContainer.kt` 删三成员 -- 死代码清零
- [x] 三共享件 KDoc 摘句 -- 注释不失指
- [x] 治理标注五处（见 Code Map） -- 裁决留痕
- [x] 构建与全量单测全绿后提交 -- 门槛
- [x] `AppNavHost.kt` 起始页判定抽为 `coldStartDestination()` 纯函数 + `ui/AppNavHostTest.kt` 契约测试 2 例 -- 矩阵测试审计补覆盖（行 1/2；实现期追加）

**Acceptance Criteria:**
- Given 未配对设备，when 冷启动，then 落配对流
- Given 已配对设备（onboarded 任意值/缺失），when 冷启动，then 直达仪表盘主壳
- Given 新配对完成，when 点「进入应用」，then 清栈直达仪表盘
- Given `grep -ri onboarding companion-android/app/src`，then 零命中
- Given 提交后 `git status`，then `egosync-app/` 与框架目录（`.agents/`、`_bmad/`、`.claude/`）无改动混入
- Given `./gradlew :app:assembleDebug` 与 `:app:testDebugUnitTest`，then BUILD SUCCESSFUL、测试全绿

## Implementation Notes

- 2026-09-10 实现提交（本地未 push）：范围 17 个任务文件，`egosync-app/` 与框架目录（`.agents/`/`_bmad/`/`.claude/`）零混入。首次构建遇 Gradle transforms 缓存损坏（环境问题非代码问题），清缓存并停 daemon 后全绿。
- 2026-09-10 矩阵测试审计：行 1/2 由新增 `ui/AppNavHostTest.kt`（`coldStartDestination` 契约测试 2 例，随全量套件运行通过）覆盖；行 3/4（onEnterApp 导航事件流）无 JVM 可测缝隙——Navigation-Compose 测试依赖不在仓库白名单、androidTest 仅 PairingSmokeTest——按仓库既有惯例以「构建+全量单测+diff 审读」覆盖，特此披露。为此将起始页判定抽为纯函数（矩阵审计驱动的实现期追加，见任务列表末行）。
- 2026-09-10 环境披露：`spec-mobile-fr-parity/SPEC.md` 文件尾部的 `</parameter></function>` 两行损坏残留系 baseline 已有问题，本次未触碰。
- 2026-09-10 勘误：上文「17 个任务文件」漏计规格文件自身，实际提交 18 个文件（`git show --stat` 核验：+145/−713）。
- 2026-09-10 覆盖口径勘误：矩阵行 1/2 的覆盖为 `coldStartDestination` 纯函数契约级；AppNavHost 组合层装配行（`remember` 单行接线）与行 3/4 导航事件流同样无执行级测试——评审确认系仓库既有测试边界（无 compose-ui-test/Robolectric/导航测试依赖；offline-deadlock 评审 2026-08-30 已 defer 同类基建缺口），见 Review Triage Log #10/#11 与 deferred-work 新条目。
- 2026-09-10 评审补丁（三路评审 11 条发现：盲审 9 + 边界猎手 2，验证缺口层零发现；2 条拒绝，其余 patch/defer）：PRD §6.1 行内批注（18→17 口径）；connection-chat-ux 目录 decision-log 追加 2026-09-10 日期段、supersessions.md 追加 §6 登记；fr-parity SPEC CAP-6 success 行补批注、Success signal 批注精确化（15→14、失效范围限定）；fr-groups 交付顺序组 6 项批注；task-breakdown T-S2 批注移位至 Onboarding 成员；deferred-work #4 条目补完成+退役标记；deferred-work 新增导航接线测试缝隙 defer 条目。实现子代理为前台一次性启动无持久 id 可回访，按补丁路由由主会话自行修补。

## Spec Change Log

## Review Triage Log

- 2026-09-10 三路评审（盲审猎手 9 条 / 边界猎手 2 条 / 验证缺口 0 条）逐条裁决（发现均经主会话亲验，非采信报告）：
  1. [medium→patch] PRD §6.1 L693 活断言「一档18项核心FR」未随裁决更新（应为 17）——亲验原文属实；L21 为 2026-08-25 历史修订记录，按只增惯例留档不改。已补行内批注。
  2. [medium→patch] connection-chat-ux 目录 `.decision-log.md` L24/L33 仍以生效口吻记录 Onboarding 待发箱裁决、`supersessions.md` 无本次裁决留痕——亲验属实。已补日期段 + §6 登记表。
  3. [medium→patch] fr-parity `SPEC.md` L69 批注未纠正「15 项 FR 全部实现」（应为 14）且缀尾式批注把仍有效断言一并判失效——亲验属实。批注已精确化（只改我方批注文本，原文未动）。
  4. [low→patch] task-breakdown T-S2 批注位置使「整段路由改绑失效」误读（实际仅 Onboarding 成员失效）——亲验 diff 属实。批注已移位。
  5. [medium→patch] CAP-6 success 行（Code Map 标注范围 L41-43 内，实现漏批 L43）与 fr-groups L165 交付顺序组 6 项无批注——亲验属实。两处已补批注。
  6. [low→patch] Implementation Notes「17 个任务文件」与提交实际 18 文件不符——git show --stat 核验属实。已追加勘误行。
  7. [low→拒绝] README L97 成为 companion-android 仅存 onboarding 命中且无标记——指控属实，但冻结 Never 明确排除 README 改动（意图排除，out of scope by intent）。
  8. [medium→patch] deferred-work #4 仍列 onboarding FR-21 为建议工作且无 ✅ 标记（#2/#3 均有），诱导复活已退役功能——亲验 L271 属实。已补完成+退役标记。
  9. [low→拒绝] 新规格 `context:` 未含手术目标与治理文档——修复即编辑本规格 frontmatter（实现已完成、无消费者），按「拒绝修复=改本规格」规则拒绝。
  10. [medium→defer+patch] 冷启动契约仅纯函数级锁定，组合层装配行无测试，接线回归时套件仍绿；「行 1/2 已覆盖」措辞夸大——亲验属实（测试类无 AppNavHost 引用、无 compose-ui-test/Robolectric 依赖）。基建缺口 defer（deferred-work 新条目）；措辞勘误已 patch。
  11. [medium→defer] 配对完成「清栈直达 DASHBOARD」无执行级测试——亲验属实（onEnterApp/navigate 无测试引用；PairingSmokeTest 不测导航且不在常规验证路径）。与 #10 同根因（导航接线无测试缝隙）合并 defer。
  - 路由汇总：G1 治理标注补全（#1/#2/#3/#4/#5/#8，最高 medium）→ patch；G2 规格注记勘误（#6 + #10 措辞部）→ patch；G3 导航测试缝隙（#10/#11）→ defer。无 intent_gap / bad_spec，不触发回环。

## Design Notes

**治理标注模式（仿 2026-09-08 FR-43 裁决先例，日期 2026-09-10）：**
- 旧规格状态行后缀：仿 `spec-companion-offline-deadlock-recovery.md:5`，在 status 行注释追加 `superseded-by: spec-companion-android-remove-onboarding (2026-09-10 裁决：手机端引导流程移除)`
- PRD 日期块引：仿 `prd-egosync.md:657`（只增不删），置于 §4.14 层级表后：`> 2026-09-10 产品裁决：手机端不提供空状态引导流程（FR-21 移动侧退役，桌面引导保留），初始设置统一在桌面完成；spec-mobile-fr-parity 组 6 相应失效。`
- 契约行内批注：仿 architecture.md「（2026-09-08 增补，SPEC-…）」风格，受影响条款处加 `（2026-09-10 裁决失效：手机端引导流已移除，spec-companion-android-remove-onboarding）`；fr-parity 侧批注顺带说明 L69「完整」数口径下调

**关键裁决：** `remember` 块定格首帧——删 `!container.isOnboarded()` 分支后已配对用户不可能落入引导；onEnterApp 目标表达式塌缩为常量 `Routes.DASHBOARD`。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest` -- expected: 全部通过（OnboardingUiStateTest 已删，其余零回归）
- `grep -ri onboarding companion-android/app/src` -- expected: 零命中
