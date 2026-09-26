# Investigation: 移动端工作区页签上下重复——管家/角色视图双 tab 条并存

## Hand-off Brief

1. **What happened.** 手机浏览器（<768px）打开管家/角色视图的任一工作区 tab（任务/记忆/设置/仪表盘）时，头部 tab 行与工作区面板自带的 tab 行同时在屏上相邻渲染，形成上下两排重复页签；桌面 65/35 双栏下两排也并存但用户判定无影响。
2. **Where the case stands.** 根因 **Confirmed**（代码级）：两个组件各自实现了一套控制同一 `openTab` 状态的页签条，无响应式互斥；16.4「tab 全屏化」修复（ceea15b）撤走了两排之间的对话区，使重复在小屏上直接相邻、无法忽视。修复方向已明确（保头部页签、去面板页签、X 上移），待用户在「仅移动端」与「全局移除」两个方案间裁决。
3. **What's needed next.** 用户确认方案范围后转 `bmad-quick-dev` 实施；推荐仅移动端隐藏面板页签条（桌面零变化）。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A（人类口头描述，2026-09-26）                                            |
| Date opened      | 2026-09-26                                                                 |
| Status           | Concluded（根因确认；修复方案待用户裁决）                                   |
| System           | EgoSync web 版（egosync-app），手机浏览器 <768px 视口                       |
| Evidence sources | 源码（ButlerView/RoleView/RoleHeader/ButlerWorkspacePanel/RoleWorkspacePanel）、git 历史、冻结线框 spec-16-4-mobile-wireframe.html、既有案卷 web-mobile-layout-issues-investigation.md |

## Problem Statement

用户原话（假设 #1，待验证）：「角色和管家选择任务记忆等时，现在会出现上下各有一个重复的页签，这个在桌面端不影响，但在移动端的时候会比较明显。现在把下面的那个取消掉，然后把下面的那一个关闭符号移到上面的地方。」

工作要求：先分析原因和方案，暂不直接执行。

## Evidence Inventory

| Source                                              | Status    | Notes                                                                 |
| --------------------------------------------------- | --------- | --------------------------------------------------------------------- |
| 管家/角色视图组件源码                               | Available | 直读 ButlerView.tsx / RoleView.tsx / RoleHeader.tsx / 两个 WorkspacePanel |
| git 历史（16.4 移动化、ceea15b 布局修复）            | Available | 204af71、ceea15b 及 diff                                               |
| 冻结线框 spec-16-4-mobile-wireframe.html            | Available | 屏 1/2b 均只有一排 `.tabs`；:201 明示「既有顶部 tab 保留」              |
| 既有案卷 web-mobile-layout-issues-investigation.md  | Available | 前序四症状修复的根因与钉孔记录                                          |
| 组件/e2e 测试                                       | Available | 面板条测试（RoleWorkspacePanel/ButlerWorkspacePanel）、移动钉孔 mock 面板 |
| 真机/375px 视口截图                                 | Missing   | 用户目击但未提供；不影响代码级结论（复现计划见文末）                    |

## Investigation Backlog

| # | Path to Explore                                   | Priority | Status | Notes                                  |
| - | ------------------------------------------------- | -------- | ------ | -------------------------------------- |
| 1 | 双页签条渲染路径定位                               | High     | Done   | Finding 1-4                            |
| 2 | 移动端全屏化布局对重复的放大                       | High     | Done   | Finding 5 / Deduction 1                |
| 3 | 冻结线框与桌面既有行为对方案的约束                 | High     | Done   | Finding 6-7                            |
| 4 | 受影响测试盘点                                     | High     | Done   | Finding 8                              |
| 5 | 方案裁决（仅移动端 vs 全局移除）                   | High     | Done   | 2026-09-26 用户裁决：**方案 A（仅移动端）**                |

## Timeline of Events

| Time         | Event                                                                                             | Source                          | Confidence |
| ------------ | ------------------------------------------------------------------------------------------------ | ------------------------------- | ---------- |
| 早期（1.6-2.3、3-1、4-7 故事期） | 管家/角色头部 tab 与工作区面板 tab 条分别落地，双条并存结构形成                               | git log（4e0001b 等）            | Confirmed  |
| 2026-09-25   | 204af71：16.4 移动形态（底部三 tab、铃铛搬迁、⋯菜单）                                             | git log                         | Confirmed  |
| 2026-09-25   | 人工裁决四症状修复方向（tab 全屏化等）                                                             | web-mobile-layout-issues-investigation.md | Confirmed  |
| 2026-09-26   | ceea15b：tab 打开时小屏对话区退场、工作区全屏——头部 tab 行与面板 tab 行在小屏直接相邻             | git show ceea15b                | Confirmed  |
| 2026-09-26   | 用户报告移动端上下重复页签，要求去底部条、X 上移                                                 | 本单 Problem Statement          | Confirmed  |

## Confirmed Findings

### Finding 1: 角色视图存在两套控制同一状态的页签条

**Evidence:** `egosync-app/src/components/role/RoleHeader.tsx:141-144`、`egosync-app/src/components/role/RoleWorkspacePanel.tsx:95-131`、`egosync-app/src/components/role/RoleView.tsx:49,68-70`

**Detail:** 头部 `RoleHeader` 渲染「任务/记忆/设置」三个 tab 按钮（`tabButton`，无响应式隐藏，桌面小屏同显）；工作区面板 `RoleWorkspacePanel` 又渲染一套「任务清单/记忆档案(n)/设置」tab 行 + 关闭 X（:128-130，`aria-label="关闭"`）。两套都操作 `RoleView` 的同一个 `openTab` 状态（头部 `onToggleTab`→`toggleTab` toggle 语义；面板 `setTab` 直切）。

### Finding 2: 管家视图同样是双页签条结构

**Evidence:** `egosync-app/src/components/butler/ButlerView.tsx:141-154`、`egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:103-121`

**Detail:** 头部渲染「仪表盘/任务/记忆/设置」（全视口可见）；面板渲染「仪表盘/任务概览/管家记忆(n)/管家设置」+ 关闭 X（:118-120，**无 aria-label**——见 Side Findings）。同样共享 `ButlerView` 的 `openTab`（:57,76-78）。

### Finding 3: 移动端 tab 打开 = 工作区全屏，两排页签相邻

**Evidence:** `egosync-app/src/components/role/RoleView.tsx:104-131`、`egosync-app/src/components/butler/ButlerView.tsx:162-208`

**Detail:** `openTab` 非空时，对话区 `max-md:hidden`、工作区 `max-md:flex-1` 全屏（16.4 布局修复，ceea15b，规格 spec-web-mobile-layout-fixes.md）。头部始终渲染在顶——小屏上「头部 tab 行」与「面板 tab 行」之间没有任何内容分隔，两排页签上下相邻。桌面 ≥768px 为 65/35 双栏，两排分居头部与右栏，视觉不冲突（与用户「桌面端不影响」一致）。

### Finding 4: 底部页签条的关闭 X 与计数徽标只存在于面板条

**Evidence:** `egosync-app/src/components/role/RoleWorkspacePanel.tsx:91,128-130`、`egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:99,118-120`

**Detail:** 「记忆档案 (n)」「管家记忆 (n)」计数仅出现在面板 tab 标签（`memoryService.count`，MemoryTab 自身不展示计数——grep 证实）；关闭 X 也只在面板条。移除面板条后这两项能力/信息需要安置（X 按用户指令上移头部；计数见 Deduction 2）。

### Finding 5: 冻结线框的手机屏只有一排页签

**Evidence:** `_bmad-output/implementation-artifacts/spec-16-4-mobile-wireframe.html:84,137,201`

**Detail:** 屏 1（管家）与屏 2b（角色详情）的 `.tabs` 均为单排；:201 明示「管家视图内的『仪表盘/任务/记忆/设置』既有顶部 tab 保留」。即移动端设计意图 = 单排顶部页签，面板内第二条属桌面遗留结构在小屏的泄漏。

### Finding 6: 桌面端两排并存是有意历史结构，非回归

**Evidence:** git log（头部 tab 早于 4e0001b、面板 tab 早于 3-1/4-7）；web-mobile-layout-issues-investigation.md 全篇未将「双条」列为症状

**Detail:** 双条并存自桌面原型期存在；ceea15b 未引入新条，只是撤走了小屏两排之间的对话区，使既有重复变得刺眼。属「结构性冗余被移动化放大」，不是 ceea15b 的回归缺陷。

### Finding 7: 受影响测试盘点

**Evidence:** `egosync-app/src/components/role/RoleWorkspacePanel.test.tsx:54-113`、`egosync-app/src/components/butler/ButlerWorkspacePanel.test.tsx:56-168`、`RoleView.mobile.test.tsx:11-41`、`ButlerView.mobile.test.tsx:21`

**Detail:** 两个面板测试直接断言面板条按钮（「记忆档案 (12)」「管家记忆 (18)」）并以其切换 tab；移动钉孔测试（RoleView/ButlerView.mobile.test.tsx）均 mock 掉面板，不断言面板条。e2e `briefing-review.spec.ts:28` 点的是头部「仪表盘」（`button=仪表盘`，头部条），不受影响。**仅移动端隐藏方案对现有测试零破坏；全局移除方案需重写两个面板测试的条内交互与计数断言。**

## Deduced Conclusions

### Deduction 1: 根因是「双组件双页签条 + 小屏无互斥」

**Based on:** Finding 1-3、Finding 6

**Reasoning:** 两个组件各自渲染一套页签条控制同一状态；响应式规则只管「底部三 tab vs 侧栏」「对话区 vs 工作区」，没有任何规则在页签条之间做互斥。桌面因 65/35 空间分隔而不刺眼；小屏全屏 drill-down 后两排直接相邻。

**Conclusion:** 修复 = 在页签条这一层做互斥/取舍，而非调整布局容器。

### Deduction 2: 移除面板条的三项能力损失与安置

**Based on:** Finding 4

**Reasoning:** 面板条承担三件事：①tab 切换（头部条已具备同功能，零损失）；②关闭 X（用户指令已定：上移头部）；③记忆计数徽标（唯一展示位，头部 tab 标签无计数）。

**Conclusion:** X 上移按用户指令执行；计数徽标有三种安置——接受消失 / 提升到头部「记忆」tab 标签（需把 `memoryService.count` 上移 RoleHeader/ButlerView）/ 移入 MemoryTab 内容区。默认建议：本次先接受消失或内容区展示，避免头部拉数据。

### Deduction 3: 头部条已有 toggle 关闭语义，X 是可达性补强而非唯一关闭路径

**Based on:** `RoleView.tsx:68-70`、`ButlerView.tsx:76-78`

**Reasoning:** 再点一次当前 tab 即关闭（既有 toggle 语义，移动钉孔测试已钉）。

**Conclusion:** 上移的 X 不引入新状态机，`onClick` 直接调 `onToggleTab(openTab)` 即可关闭（当前 tab 再点 = null），无需新增 props。

## Hypothesized Paths

### Hypothesis 1: 用户描述的「上下重复页签」即本双条结构

**Status:** Confirmed

**Theory:** 用户所指「下面的页签」= 工作区面板条，「上面的」= 头部条；「关闭符号」= 面板条右端 X。

**Supporting indicators:** 结构、位置、交互（选择角色/管家/任务/记忆时出现）与用户描述逐项吻合；线框单排设计与重复现象互证。

**Would confirm:** 375px 视口复现（见 Reproduction Plan）。

**Would refute:** 移动端实际渲染仅一排（与代码直读矛盾，需真机证伪）。

**Resolution:** 代码级直读 + 移动钉孔布局类名确认；缺真机截图（Missing Evidence 表）。

## Missing Evidence

| Gap                          | Impact                                     | How to Obtain                        |
| ---------------------------- | ------------------------------------------ | ------------------------------------ |
| 375px 真机/视口截图          | 佐证目击现象（不影响代码级结论）           | DevTools 设备模式或真机走一遍复现计划 |
| 桌面徽标计数消失的用户容忍度  | 决定计数徽标是否值得上移成本               | 方案裁决时一并问用户                 |

## Source Code Trace

| Element       | Detail                                                                                          |
| ------------- | ----------------------------------------------------------------------------------------------- |
| Error origin  | 无异常；结构冗余：`RoleHeader.tsx:141-144` + `RoleWorkspacePanel.tsx:95-131`；`ButlerView.tsx:141-154` + `ButlerWorkspacePanel.tsx:103-121` |
| Trigger       | 小屏点任一工作区 tab（任务/记忆/设置/仪表盘）                                                     |
| Condition     | <768px 且 `openTab` 非空 → 对话区退场、工作区全屏，头部条与面板条相邻渲染                          |
| Related files | `RoleView.tsx`、`RoleHeader.tsx`、`RoleWorkspacePanel.tsx`、`ButlerView.tsx`、`ButlerWorkspacePanel.tsx`、`lib/utils.ts`（isMobileViewport）、冻结线框 |

## Conclusion

**Confidence:** High（根因代码级 Confirmed；修复范围为产品裁决项）

用户前提全部属实：上下两排重复页签 = 头部 tab 行 + 工作区面板 tab 行控制同一 `openTab`；桌面因双栏分隔无影响，小屏因 16.4 全屏化而相邻刺眼。非回归，属历史双条结构被移动化放大。冻结线框（单排顶部 tab）支持「保头部、去面板」方向。

修复方案两选一（待用户裁决）：

- **方案 A（推荐）：仅移动端隐藏面板条。** 两个 WorkspacePanel 的 tab 条容器加 `max-md:hidden`；关闭 X 加到头部（`md:hidden`，仅小屏显）：角色放头部 tab 簇末端（3 tab + X，375px 宽余量约 77px，稳）；管家因 4 tab + X 在 375px 会溢约 10px，X 放头部第一行右端（铃铛/连接状态簇旁，该行余量约 50px）。桌面零变化，现有测试零破坏（jsdom 桌面视口 + 移动测试已 mock 面板）。
- **方案 B（字面执行）：全局移除面板条。** 两个面板的 tab 条与 X 整体删除（桌面小屏同删）；X 加到头部（全视口，`openTab` 非空时显）。代价：桌面视觉变化（右栏直起内容，无条无 X，与代码库「桌面零变化」纪律相悖）；「记忆档案(n)/管家记忆(n)」计数徽标消失（需决定是否上移头部）；两个面板测试需重写（计数断言与条内切换交互）。

## Recommended Next Steps

### Fix direction

按用户裁决执行 A 或 B。共同动作：①面板条退场（A：`max-md:hidden`；B：删除）；②X 上移头部——`onClick={() => onToggleTab(openTab)}`（复用 toggle 关闭语义，不加 props），`aria-label="关闭"`，≥44px 触控目标；③管家侧 X  Placement 按方案 A 放第一行右端 / 按方案 B 可放 tab 行末（桌面有空间）。验证：375px 视口 tab 开合、X 关闭、再点当前 tab 关闭、来源消息跳转回对话（既有钉孔）；桌面走 App.test/RoleView.test/ButlerView.test + briefing-review e2e。

### Diagnostic

无需额外诊断。真机 375px 复现一步即可佐证（见下）。

## Reproduction Plan

1. 手机浏览器（或 DevTools 375px 视口）打开 web 版 → 底部 tab「管家」→ 点「任务」（或仪表盘/记忆/设置）。
2. 预期现状（bug）：头部「仪表盘 任务 记忆 设置」下方紧跟面板「仪表盘 任务概览 管家记忆(n) 管家设置 + X」——两排重复。
3. 点底部「角色」→ 进任一角色 → 点「任务」：同样两排（「任务 记忆 设置」+「任务清单 记忆档案(n) 设置 + X」）。
4. 桌面宽窗口同路径：两排分居头部与右栏，不冲突（对应用户「桌面端不影响」）。

## Side Findings

- `ButlerWorkspacePanel.tsx:118` 的关闭 X 无 `aria-label`（角色侧 :128 有）——若采方案 A 桌面仍保留该条，建议顺手补上；采方案 B 随条删除自然消失。
- 管家头部 tab 行在 375px 下余量仅约 40px（4 个 tab ≈ 303px + 边距 32px）：方案 A 的 X 若也放该行会溢出，须放第一行右端或缩窄触控目标（不推荐缩 <44px）。
- 「记忆档案(n)」计数是面板条独有信息位（MemoryTab 内不展示）——全局移除即桌面也失去该计数，需产品知悉。

## Follow-up: 2026-09-26

### New Evidence

用户裁决：**方案 A（仅移动端隐藏面板条 + X 上移头部仅小屏可见）**。

### Additional Findings

方案 A 细化设计（待实施时核对）：

1. `RoleWorkspacePanel.tsx:95` 与 `ButlerWorkspacePanel.tsx:103` 的 tab 条容器 div 加 `max-md:hidden`（整条隐藏，含条内 X）。
2. `RoleHeader.tsx`：tab 簇「设置」之后加关闭按钮——条件 `openTab &&`、类含 `md:hidden`；`aria-label="关闭"`；`onClick={() => onToggleTab(openTab)}`（复用 toggle 关闭语义，不加 props）；触控 w-11 h-11。375px 核算：3 tab ≈216px + X 44px，余量充足。
3. `ButlerView.tsx`：X 放头部第一行右端（铃铛/连接状态簇之后，同 `md:hidden` 簇），`onClick={() => toggleTab(openTab)}`。第一行 375px 核算 ≈290px < 343px，不溢出（若放第二行 4 tab 后会溢 ~10px，故不放 tab 行）。
4. 测试：`ButlerView.mobile.test.tsx` 增「点 tab → X 现 → 点 X → tab 关」钉孔；`RoleHeader.test.tsx` 增「openTab 非空时渲染 X（md:hidden）+ 点击触发 onToggleTab(openTab)」；桌面既有测试预期零破坏（X 常驻 DOM 但 CSS 隐藏，需查 `getAllByRole` 计数类断言）。
5. 顺手项（可选，实施时确认）：`ButlerWorkspacePanel.tsx:118` 面板 X 补 `aria-label="关闭"`（方案 A 桌面仍留该条）。

### Updated Hypotheses

（无新增）

### Backlog Changes

#5 关闭；实施项移交 `bmad-quick-dev`（待用户说「实施」）。

### Follow-up 2: 2026-09-26（实施闭环）

用户说「实施」→ 走 bmad-build oneshot（spec：`spec-web-mobile-tab-dedup.md`）：面板条 `max-md:hidden`、头部 X 仅小屏显（RoleHeader tab 簇右侧 / ButlerView 第一行右端）、盲猎 6 发现全修。验证：vitest 946/946、tsc+构建零错、web-mobile e2e 11/11。根因与方案判断全部得到行为级证实。

### Updated Conclusion

根因 Confirmed 不变；修复范围已定为方案 A。详细改动点、测试与验证计划见 `spec-web-mobile-layout-fixes.md` 同族新 spec（实施时建）。状态：Concluded（诊断）+ 待实施。
