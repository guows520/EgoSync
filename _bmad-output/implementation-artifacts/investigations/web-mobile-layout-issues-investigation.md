# Investigation: web 版手机浏览器四个布局症状——tab 不独显、图标位置、⋯菜单位置、设置模态冗余切换

## Hand-off Brief

1. **What happened.** 手机浏览器（<768px）访问 web 版时四个布局症状：①管家/角色视图点「仪表盘/任务」等 tab 后对话界面仍占上半屏（58/42 堆叠）；②通知铃铛+连接状态图标跟在中标题右侧而非头部右端；③角色详情头部「⋯」菜单在「设置」按钮左边；④设置 tab 点任一配置项弹出的 GlobalSettingsModal 顶部出现 4 个 tab 切换按钮。
2. **Where the case stands.** 四症状根因全部 **Confirmed**（代码级定位）：①16.2 响应式基线刻意把桌面 65/35 双栏在小屏改为 58/42 堆叠（ButlerView.tsx:159-162、RoleView.tsx:106-131）；②铃铛/连接状态渲染在头部左组内（ButlerView.tsx:98-135），偏离冻结线框「右上角」裁决（spec-16-4-mobile-wireframe.html:190）；③「⋯」按钮渲染在 tab 组之前（RoleHeader.tsx:145-155 vs 262-264）；④移动设置入口逐项带 initialTab 打开桌面同款模态，其左侧 tab 列在小屏转为横向包裹行（MobileSettingsView.tsx:53-60 → App.tsx:449-452 → GlobalSettingsModal.tsx:647-672）。均为 16.2/16.4 的有意设计与本次用户预期冲突——属**设计变更**而非单纯回归。
3. **What's needed next.** 用户已明示「先分析原因和方案，暂不直接执行」。待用户裁决四项修复方向（本卷「Recommended Next Steps」已给出逐项机制、涉及文件与测试影响面），批准后走 bmad-quick-dev 或建 story。

## Case Info

| Field            | Value                                                                  |
| ---------------- | ---------------------------------------------------------------------- |
| Ticket           | N/A（用户口述 2026-09-25，web 版手机浏览器自适应四症状）                 |
| Date opened      | 2026-09-25                                                             |
| Status           | Active（分析完成，待用户裁决修复方向）                                   |
| System           | 手机浏览器访问 EgoSync web 版（项目移动基线口径 375×812，<768px 断点；宿主=浏览器，非 Tauri 桌面） |
| Evidence sources | 前端源码（ButlerView/RoleView/RoleHeader/GlobalSettingsModal/MobileSettingsView/App.tsx）、16.4 规格与冻结线框、既有 vitest/e2e 钉孔、git 提交史（204af71/1b19c7f） |

## Problem Statement

用户原话（2026-09-25）：

- 数字管家和角色，单击「仪表盘」、「任务」等，应该只显示对应页面，不应该再显示对话界面，目前是上部分还是对话界面
- 通知和连接状态图标应该靠右边显示
- 角色：更多选项的 3 个点，应该放在设置按钮的右边
- 设置：单击任何一个配置选项，目前界面上的顶部都会出现 4 个切换选项，不需要这4个切换选项

## Evidence Inventory

| Source                | Status      | Notes                                                                                              |
| --------------------- | ----------- | -------------------------------------------------------------------------------------------------- |
| 前端源码              | Available   | 五处布局代码全部直读定位（见 Confirmed Findings）                                                     |
| 16.4 规格+冻结线框    | Available   | spec-16-4-mobile-web-form-and-pwa.md、spec-16-4-mobile-wireframe.html:190（铃铛=头部右上角）         |
| git 提交史            | Available   | 204af71（16.4 移动形态）、1b19c7f（D2 ⋯菜单扩展），确认各症状为有意设计的副产品                     |
| 既有测试钉孔          | Available   | RoleView.test.tsx:168-170、RoleView.mobile.test.tsx:58-84 钉死 58/42 堆叠；web-mobile.spec.ts 流程断言 |
| 用户手机型号/宽度     | Missing     | 具体机型与视口宽度未知；项目既有移动基线统一 375px 口径，代码级断点 max-md:<768px 不受单机型影响       |
| 复现截图/录屏         | Missing     | 无；代码证据已闭合，截图仅能佐证无法改变结论                                                         |

## Investigation Backlog

| # | Path to Explore                                                | Priority | Status | Notes                                            |
| - | -------------------------------------------------------------- | -------- | ------ | ------------------------------------------------ |
| 1 | ButlerView 小屏布局类（chat/workspace 比例）                     | High     | Done   | Finding 1/2——58/42 堆叠根因                      |
| 2 | RoleView 小屏布局类 + inert h-[42%] 垫片来历                     | High     | Done   | Finding 3——垫片为保 16.2 旧钉孔的遗留             |
| 3 | ButlerView 头部铃铛/连接状态渲染位置 vs 冻结线框                 | High     | Done   | Finding 4/5——实现偏离线框「右上角」               |
| 4 | RoleHeader ⋯按钮与 tab 组渲染顺序                               | High     | Done   | Finding 6                                        |
| 5 | 设置链：MobileSettingsView → App handleOpenSettingsTab → GlobalSettingsModal nav | High | Done | Finding 7——tab 列在小屏转横向包裹行             |
| 6 | 受修复影响的既有测试/e2e 钉孔清点                                | High     | Done   | 见「Recommended Next Steps」测试影响面             |
| 7 | 记忆来源跳转在「tab 全屏化」后的行为（cli 点击来源消息）           | Medium   | Done   | Finding 8——全屏化后需同步关闭 tab，否则移动端无可见变化 |

## Timeline of Events

| Time           | Event                                                                 | Source                                          | Confidence |
| -------------- | --------------------------------------------------------------------- | ----------------------------------------------- | ---------- |
| Story 16.2     | 小屏基线：≥md 双栏 65/35，<768px 堆叠对话 58%+工作区 42%（h-[58%]/h-[42%]） | ButlerView.tsx:152-156、RoleView.tsx:93-104 注释 | Confirmed  |
| 16.4 评审轮 G11 | h-[58%]/h-[42%] 百分比对 flex 列子项退化 → 改 max-md:flex-[58]/[42]，RoleView 留 inert h-[42%] 垫片保旧钉孔 | RoleView.tsx:98-104、RoleView.mobile.test.tsx:6-9 | Confirmed  |
| 2026-09-25     | 人工裁决：铃铛+连接状态搬管家视图头部；主题/登出搬设置 tab            | spec-16-4-mobile-web-form-and-pwa.md:30、wireframe:190 | Confirmed  |
| 2026-09-25     | 204af71：16.4 移动形态落地（底部三 tab、铃铛搬迁、⋯菜单）              | git log                                         | Confirmed  |
| 2026-09-25     | 1b19c7f：D2 收口——⋯菜单追加归档/删除                                 | git log                                         | Confirmed  |
| 2026-09-25     | 用户口述四症状；本卷开立                                             | 用户消息                                        | Confirmed  |

## Confirmed Findings

### Finding 1: 管家视图小屏「tab 打开 = 对话区仍占 58%」

**Evidence:** `egosync-app/src/components/butler/ButlerView.tsx:157-219`

**Detail:** `openTab` 非空时，对话区容器带 `max-md:flex-[58]`、工作区带 `max-md:flex-[42]`，父容器 `flex-col md:flex-row`——<768px 时上下堆叠，对话区在上、工作区在下。用户点「仪表盘/任务/记忆/设置」任一 tab，上半屏仍是 ChatStream（含敲门 ActionCard）。代码注释明示这是 Story 16.2 刻意的小屏基线（152-156 行）而非回归。

### Finding 2: 角色视图小屏同一机制 + inert 垫片

**Evidence:** `egosync-app/src/components/role/RoleView.tsx:93-149`

**Detail:** 与 Finding 1 同构：chat pane `max-md:flex-[58]`（110 行）、workspace pane `max-md:flex-[42] h-[42%]`（129 行）。`h-[42%]` 是 G11 评审为保 16.2 既有测试钉孔（RoleView.test.tsx:168-170）保留的 inert 遗留垫片（98-104 行注释自述）。

### Finding 3: 铃铛+连接状态渲染在头部左组（标题右侧），非右端

**Evidence:** `egosync-app/src/components/butler/ButlerView.tsx:98-135`

**Detail:** header 为 `flex flex-wrap justify-between`：左组（`min-w-0`）= 管家图标+标题+**铃铛+ConnectionStatus**（106-134 行，`md:hidden`），右组 = 四个 tab 按钮（136-149 行）。375px 下左组总宽 ≈46+标题+44+徽标 ≈270px+，tab 组被 wrap 到第二行；铃铛/连接状态停留在第一行中段、标题文字右侧——不在头部右端。

### Finding 4: 冻结线框要求铃铛在「管家视图头部右上角」

**Evidence:** `_bmad-output/implementation-artifacts/spec-16-4-mobile-wireframe.html:190`

**Detail:** 线框原文「🔔 通知铃铛 → 管家视图头部**右上角**（今天有什么新建议一眼可见）」「🟢 连接状态 → 管家视图头部」。实现落点在左组内，与人工定稿线框存在偏差——用户本次报告与线框一致，是实现偏了。

### Finding 5: 角色详情「⋯」按钮渲染在 tab 组左侧

**Evidence:** `egosync-app/src/components/role/RoleHeader.tsx:141-155, 262-264`

**Detail:** `ml-auto` 簇内渲染顺序：`onSwitchRole && <⋯按钮>`（145-155 行）→ `tabButton('tasks')` → `tabButton('memory')` → `tabButton('settings')`（262-264 行）。即小屏头部右侧控件顺序为 [⋯][任务][记忆][设置]——「⋯」在「设置」左边，与用户要求的「设置按钮右边」相反。

### Finding 6: 设置模态顶部 4 个切换按钮的来源

**Evidence:** `egosync-app/src/components/settings/MobileSettingsView.tsx:53-60` → `egosync-app/src/App.tsx:449-452` → `egosync-app/src/components/settings/GlobalSettingsModal.tsx:647-672`

**Detail:** 移动设置列表每一行 `onOpenSettings(tab)` 带 initialTab 打开桌面同款 GlobalSettingsModal（16.4 裁决「同一批组件不重写」）。模态左侧 tab 列（llm/mcp/scheduler/data 四钮；remote/companion 在浏览器宿主被 capabilities 门控不渲染）在 <768px 以 `flex flex-row flex-wrap` 横向包裹行呈现（644-646 行注释自述）——即用户所见「顶部 4 个切换选项」。用户已通过列表选定分区，此切换行为冗余。

### Finding 7: 记忆来源跳转依赖对话区可见

**Evidence:** `egosync-app/src/components/butler/ButlerView.tsx:80-91, 204-218`；`egosync-app/src/components/role/RoleView.tsx:72-87, 134-147`

**Detail:** 工作区（MemoryTab 等）的「查看来源消息」经 `onSourceMessageClick` → `setSourceNavigationTarget` 驱动 ChatStream 滚动定位，但**不关闭 openTab**。桌面双栏下对话区常驻可见故链路成立；若按症状①把移动端改为「tab 全屏、对话区隐藏」，来源跳转将无可见效果——修复①必须同步「跳转来源时关闭 tab（回对话）」，否则引入新断裂。

## Deduced Conclusions

### Deduction 1: 症状①是 16.2 设计决策与移动端使用预期的冲突，非代码缺陷

**Based on:** Finding 1/2

**Reasoning:** 58/42 堆叠有明确注释 lineage（16.2 基线 → G11 flex 修复），是「tab = 侧边工作区」桌面隐喻向小屏的移植；用户预期「tab = 独立页面」是移动端主导航范式。两者是有意设计 vs 使用预期的冲突。

**Conclusion:** 修复①属产品设计变更（小屏工作区全屏化），需要显式裁决而非按 bug 修。

### Deduction 2: 症状②用户报告与冻结线框一致，是实现偏差

**Based on:** Finding 3/4

**Reasoning:** 线框 2026-09-25 人工定稿「右上角」，实现落在左组中段。

**Conclusion:** 修复②是回归线框意图，风险最低。

### Deduction 3: 症状④的 4 按钮在移动端无信息增量

**Based on:** Finding 6

**Reasoning:** 模态总由 MobileSettingsView 带 initialTab 打开（onboarding 首启路径同样带 tab）；h3 标题（675 行）已标示当前分区，X 关闭（676-694 行）可返回列表。

**Conclusion:** 移动端隐藏 tab 列不损失任何可达性；桌面（≥768px，自侧栏进入）必须原样保留。

### Deduction 4: 症状③修复为纯位置调整，零行为变化

**Based on:** Finding 5 + 测试清点（RoleHeader*.test.tsx、web-mobile.spec.ts 均无顺序钉孔，均按 testid/名称点击）

**Reasoning:** 「⋯」及其菜单、确认弹窗全部自包含（state/ref 在组件内，下拉定位锚父容器 141 行的 `relative` + `right-2`，不依赖按钮在簇内顺序）。

**Conclusion:** 移动渲染块到 tab 组之后即可，桌面本就 `md:hidden` 不渲染——桌面零变化。

## Hypothesized Paths

### Hypothesis 1: 用户在 ≥768px 窄桌面窗口或折叠屏访问，导致症状与断点口径不符

**Status:** Refuted

**Theory:** 若视口 ≥768px，四症状的表现会不同（双栏并列、铃铛不渲染、模态纵列 nav）。

**Supporting indicators:** 用户未提供视口宽度。

**Would confirm:** 用户提供机型/宽度。

**Would refute:** 用户描述与 <768px 代码路径逐条吻合——「上部分还是对话界面」= 58/42 堆叠独有现象；「4 个切换选项」= 小屏横向包裹行独有现象；「⋯ 与设置按钮同排」仅在移动簇出现（桌面无 ⋯）。

**Resolution:** 用户原话三个特征均只可能在 <768px 分支产生，与项目 375px 移动基线口径一致， host 分支无需再议。（机型具体宽度仍 Missing，见 Missing Evidence）

## Missing Evidence

| Gap                    | Impact                                              | How to Obtain                          |
| ---------------------- | --------------------------------------------------- | -------------------------------------- |
| 用户机型/视口宽度       | 仅影响断点微调（如 <480px 二级适配），不影响根因       | 用户补充或 UAT 时记录                   |
| 修复后的实机走查         | 四项修复的视觉确认（尤其①工作区全屏后的滚动/安全区）   | 修复后 web e2e + 真机验证               |

## Source Code Trace

| Element       | Detail                                                                                              |
| ------------- | --------------------------------------------------------------------------------------------------- |
| Error origin  | ButlerView.tsx:159-162/202-203（①）；ButlerView.tsx:98-135（②）；RoleHeader.tsx:145-155 vs 262-264（③）；GlobalSettingsModal.tsx:647-672（④） |
| Trigger       | ①小屏点 tab；②小屏进管家视图；③小屏进角色详情；④小屏设置列表点配置行                                    |
| Condition     | 视口 <768px（max-md/md:hidden 分支），宿主=浏览器                                                        |
| Related files | RoleView.tsx:105-149、App.tsx:449-459/575-581、MobileSettingsView.tsx:53-60、spec-16-4-mobile-wireframe.html:190、RoleView.mobile.test.tsx、RoleView.test.tsx:168-170、web-mobile.spec.ts:166-262 |

## Conclusion

**Confidence:** High

四症状根因全部 Confirmed（代码级直读）。核心判断：**四个症状全部是 Story 16.2/16.4 移动化改造的有意设计产物**，与用户（及症状②对应的冻结线框）预期冲突——除症状②是「实现偏离已定稿线框」外，①③④都需要产品侧显式裁决「移动端主导航范式」：tab 全屏化 vs 堆叠双栏、⋯ 归位、设置模态去切换行。无未知代码路径，无环境证据缺口影响结论。

## Recommended Next Steps

### Fix direction

**① tab 打开时工作区全屏、对话区让位（管家+角色同改）**
- `ButlerView.tsx:159-162`：`openTab` 时 chat pane 加 `max-md:hidden`；`202-203`：workspace 去 `max-md:flex-[42]`，改 `max-md:flex-1`（全高）。桌面 `md:w-[65%]/md:w-[35%]` 与 toggle 关闭行为不动。
- `RoleView.tsx:106-131`：同改；删除 inert `h-[42%]` 垫片（已无存在理由）。
- 联动（Finding 7）：两视图 `handleSourceMessageClick`/`handleMemoryReferenceClick` 已在 Bess 上的来源跳转需 `setOpenTab(null)` 回对话，否则移动端点了没反应。
- 测试影响（须显式解冻）：`RoleView.test.tsx:168-170`、`RoleView.mobile.test.tsx:58-84` 两处钉孔按新布局改写（项目冻结规则「禁改既有断言」——本次为产品行为变更，需用户/owner 授权后改）；`web-mobile.spec.ts:206-262` 仪表盘用例（现行注释「小屏工作区 h-[42%] 底栏面板」即旧行为描述）与 166-204 任务面用例需按全屏化重走。

**② 铃铛+连接状态右对齐（回归线框）**
- `ButlerView.tsx:97-150`：header 改 `flex-col md:flex-row md:items-center`；第一行 = 图标+标题 + `ml-auto md:hidden` 的铃铛/连接状态簇；第二行 = tab 组。桌面 `md:` 下铃铛 `md:hidden` 不渲染、行内排布与今天逐像素一致。
- 风险最低；`web-mobile.spec.ts` 铃铛断言（`butler-notif-bell` isDisplayed/点击）不受位置影响。

**③ 「⋯」移到「设置」右边**
- `RoleHeader.tsx`：把 145-261 行 `onSwitchRole && (<>⋯+菜单+确认弹窗</>)` 整块移到 264 行 `tabButton('settings', …)` 之后（同 `ml-auto` 簇内末端）。下拉菜单定位锚在父容器（141 行 `relative`+`right-2`），顺序无关。
- 既有测试零顺序钉孔；e2e 按 testid 点击，零影响。

**④ 设置模态去顶部 4 切换（仅移动端）**
- `GlobalSettingsModal.tsx:647`：nav 容器类加 `hidden md:flex`（CSS-only）。移动端模态 = 单分区全屏（h3 标题+X 关闭），换分区回列表重选；桌面（≥768px 自侧栏进入）nav 原样。
- jsdom 不应用 Tailwind 样式 → `GlobalSettingsModal.test.tsx/browser.test.tsx` 全部按名称点击 nav 的既有断言原样通过，零测试改动。备选方案（props 传 hideNav）会破坏这些断言，不推荐。

### Diagnostic

- 修复①后必须真机/375px 视口验证：tab 开关往返、来源消息跳转回对话、工作区内滚动（任务四象限排序）不溢出、安全区（输入区/横幅）不变。
- 修复②后验证 375px 下第一行不换行、tab 组第二行完整可达。
- 四项均建议复用 `tests/e2e/web-specs/web-mobile.spec.ts` 单跑（`npm run test:web` 或指定 spec）做回归，全量套件已知有 setWindowSize 负载抖动面（16.4 规格验证表已备案），按既有结论逐 spec 单跑兜底。

## Reproduction Plan

1. 手机浏览器（或 DevTools 375×812 设备模拟）登录 web 版。
2. 管家视图点「仪表盘」→ 预期旧行为：上半屏仍是对话（现象①）；点头部铃铛 → 预期旧行为：铃铛在标题右侧而非右端（现象②）。
3. 底部 tab 进角色 → 预期旧行为：「⋯」在「设置」左侧（现象③）；点「任务」→ 上半屏仍是对话（现象①）。
4. 底部 tab 进设置 → 点「模型服务」→ 预期旧行为：全屏模态顶部横向 4 个切换按钮（现象④）。
5. 四项修复后按相同路径重走，预期：tab 全屏无对话区、铃铛/连接状态头部右端、「⋯」在设置右侧、设置模态无顶部切换行。

## Side Findings

- 角色视图移动端无通知铃铛（仅管家视图有，符合线框裁决）——角色页内无通知入口，需回管家页，属既有定稿行为，不在本卷范围。
- `MobileSettingsView` 的「通知」行打开的是 scheduler tab，模态标题显示「调度时间配置」——预制定稿语义（通知=调度 tab 内敲门声音），但移动端隐藏 tab 列后标题与入口名不一致感会更强；如用户介意可在修复④时改成直达/改名，需产品裁决。
- GlobalSettingsModal 的 X 关闭按钮带编辑态返回语义（677-691 行：mcp/llm 编辑中先退编辑再关闭）——隐藏 nav 不影响该语义。

## Follow-up: 2026-09-25（裁决与实施授权）

### New Evidence

无新证据（用户为产品 owner 裁决，非事实补充）。

### Additional Findings

用户对「Recommended Next Steps」菜单裁决：**选 A——全项修**。含两项隐含授权：
1. 症状①按「tab 全屏化」设计变更执行（移动端主导航范式 =  drill-down 换页，取代 16.2 堆叠双栏）；
2. 解冻测试授权——`RoleView.test.tsx:168-170`、`RoleView.mobile.test.tsx:58-84` 两处 58/42 钉孔可按新行为改写（原冻结规则「禁改既有断言」的例外由 owner 显式授予）。

### Updated Hypotheses

无（四症状根因均 Confirmed，不涉及假设）。

### Backlog Changes

| # | 原状态 | 新状态 | 说明 |
| - | ------ | ------ | ---- |
| 1-7 | Done | Done | 调查项全部关闭 |
| 8 | — | In Progress | 实施：①tab 全屏化（Butler/Role + 来源跳转联动）②铃铛右对齐 ③⋯归位 ④设置模态去切换行 |
| 9 | — | Open | 验证：受影响组件 vitest + 375px web e2e 走查 |

### Updated Conclusion

调查 Closed（Status 改 Concluded 于实施完成后）。实施按 bmad-build 流程执行，桌面路径零变化红线逐项保持（md: 类不动）。

## Follow-up: 2026-09-26（实施完成与验证回填）

### New Evidence

无新事实（实施为既定方案落地）。验证证据：vitest 全量 940/940（81 文件）；`tsc --noEmit` 退出 0；web-mobile.spec.ts e2e 11/11（Chrome 148 真实宿主，须先 `npm run build` 重建 dist 再跑——首跑 4 失败皆为旧 dist 伪失败，非代码缺陷）。

### Additional Findings

- 盲审（blind-hunter 1 轮）6 findings：F1/F2 矛盾注释、F3 注释措辞、F4 RoleView 移动跳转分支零覆盖、F5 案卷行号漂移、F6 non-defect。处置：F1-F5 全部随提交修复/回填，F6 拒稿（详见 spec 档 Review Triage Log）。
- GlobalSettingsModal nav 隐藏后类表 `max-md:flex-wrap max-md:[&>button]:*` 成小屏死类——按硬红线保留不动并注释注明，未清理（清理属行为外改动）。

### Updated Hypotheses

无（实施未推翻任何 Confirmed 根因）。

### Backlog Changes

| # | 原状态 | 新状态 | 说明 |
| - | ------ | ------ | ---- |
| 8 | In Progress | Done | 四症状修复全部落地（spec-web-mobile-layout-fixes.md status: done） |
| 9 | Open | Done | 三层验证全绿（单测/tsc/375px e2e 行为级断言） |
| 10 | — | Open（Side Finding） | 「通知」入口开 scheduler tab 的标题不一致（Side Findings 既有项，本次隐藏 nav 后更明显）——需产品裁决，未在本次范围 |

### Updated Conclusion

调查与实施均 Closed。遗留仅 Side Finding 10（产品裁决项），与修复④无阻塞关系。
