---
title: '移动端工作区页签去重——面板 tab 条小屏退场、关闭 X 上移头部'
type: 'bugfix'
created: '2026-09-26'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 手机浏览器（<768px）在管家/角色视图打开任一工作区 tab 时，头部 tab 行与工作区面板自带的 tab 行上下相邻渲染，形成两排重复页签（管家：仪表盘/任务/记忆/设置 vs 仪表盘/任务概览/管家记忆(n)/管家设置；角色：任务/记忆/设置 vs 任务清单/记忆档案(n)/设置）；桌面 65/35 双栏下两排分居头部与右栏，用户判定无影响。根因与证据链见调查案卷 `investigations/mobile-duplicate-workspace-tabs-investigation.md`。

**Approach:** 采用用户裁决的方案 A（仅移动端）：两个 WorkspacePanel 的 tab 条（含条内 X）在 <768px 隐藏（`max-md:hidden`，桌面原样保留——桌面零变化硬红线）；关闭 X 上移到视图头部且仅小屏可见（角色：头部控件簇最末端——「⋯」右侧、头部最右，同日追加人类指令；管家：头部第一行右端铃铛/连接状态簇旁——管家 tab 行 4 钮 + X 在 375px 会溢出约 10px，第一行余量约 50px），复用既有 toggle 语义（再点当前 tab = 关闭）作为 X 的 onClick，不新增 props/状态。管家面板条内 X 顺手补 `aria-label="关闭"`（与角色侧对等，不可见属性、零视觉风险）。

</frozen-after-approval>

## Implementation Notes

### 结构约束（来自调查，实施时直接采信）

- 双 tab 条控制同一 `openTab`：头部 `onToggleTab`（toggle 语义，`RoleView.tsx:68-70` / `ButlerView.tsx:76-78`）与面板 `setTab`（直切）。移除面板条的切换能力零损失——头部条全视口可见、功能完整。
- 「记忆档案(n)/管家记忆(n)」计数徽标是面板条独有信息位（MemoryTab 自身不展示）——方案 A 下面向桌面的徽标保留，仅小屏不可见（与面板条同进退），用户已接受。
- 冻结线框（spec-16-4-mobile-wireframe.html:201）明示移动端单排顶部 tab，本方案与线框一致。

### 改动点（已实施）

1. `src/components/role/RoleWorkspacePanel.tsx` — tab 条容器加 `max-md:hidden` + `data-testid="role-panel-tab-bar"`（后者供单测/e2e 钉孔）。
2. `src/components/butler/ButlerWorkspacePanel.tsx` — 同 1（`butler-panel-tab-bar`）；条内 X 补 `aria-label="关闭"`。
3. `src/components/role/RoleHeader.tsx` — `ml-auto` 控件簇最末端（`{onSwitchRole && ...}` 块之后）新增关闭钮：`openTab` 非空才渲染；类 `md:hidden w-11 h-11 flex items-center justify-center rounded-lg ...`；`aria-label="关闭"` + `data-testid="role-close-tab"`；`onClick` 先 `setShowMoreMenu(false)` 再 `onToggleTab(openTab)`（toggle 语义）；lucide 补 `X` 导入。位置演变：初版在「设置」与「⋯」之间 → 2026-09-26 人类指令「角色的关闭按钮应该放在最右边」→ 移至「⋯」之后（簇末端、头部最右），DOM 序钉孔同步改写（moreButton → closeButton FOLLOWING）。
4. `src/components/butler/ButlerView.tsx` — 头部第一行右端新增关闭钮（`butler-close-tab`；`toggleTab(openTab)`；lucide 补 `X`）。
5. 测试：`RoleHeader.test.tsx` +2 it（X 渲染/回调/DOM 序、openTab=null 不渲染）；`ButlerView.mobile.test.tsx` +3 it（X 点击关 tab、openTab=null 不渲染、位置钉孔）；两个 Panel.test 各 +1 it（`max-md:hidden` 容器钉孔，守桌面零变化）；`web-mobile.spec.ts` 头注释登记条目 10，角色任务面/管家仪表盘面两处加 existing+displayed 双断言 + 点 X 回对话。

### 盲猎评审补丁（6 条发现，全部判定为真并修复）

1. 面板侧 `max-md:hidden` 零守卫 → 两个 Panel.test 加容器类钉孔 + 容器挂 `data-testid`。
2. e2e 退场断言分不清「媒体查询隐藏 vs 整条删除」+ 文本选择器脆弱 → 改 `[data-testid=...]` existing=true + displayed=false 双断言（沿用侧栏惯例）。
3. Butler 移动单测缺位置钉孔 → 新增「X 在标题行内、不在 tab 行」DOM 归属断言。
4. Butler X 的 `ml-auto` 为死类（右对齐实际由铃铛 ml-auto 承担）→ 删该类，注释说明机制。
5. 「⋯」菜单展开时点 X 不收菜单（簇内点击绕过 ref 外部点击关闭）→ X 与 tabButton 的 onClick 均先 `setShowMoreMenu(false)`。
6. `RoleHeader.test.tsx` 缺末尾换行（存量）→ 补。

### 验证结果

- `npx vitest run` 全量 → 81 文件 / 946 用例全绿（改动前基线 940；净增 6：+2 RoleHeader、+2 ButlerView.mobile、+2 Panel 守卫；盲猎补丁后定向 22 文件 / 213 全绿，构建另跑）。
- `npm run build` → tsc 零类型错误 + vite 构建通过（改动后重跑，仅 chunk 体积提示，存量告警）。
- e2e `cd tests/e2e && npx wdio run wdio.web.conf.ts --spec web-specs/web-mobile.spec.ts` → ✅ 11/11 全绿（Chrome 148 真实宿主；先 `npm run build` 重建 dist 后跑；角色任务面/管家仪表盘面两处新断言——面板条 existing=true/displayed=false、头部 X displayed=true、点 X 回对话——全部通过）。

### 追加变更验证（2026-09-26：「角色关闭钮放最右」）

用户追加指令后：X 从「设置」与「⋯」之间移至整簇末端（「⋯」右侧），DOM 序钉孔改写为 moreButton → closeButton FOLLOWING。定向 vitest 22 文件 / 213 全绿、`npm run build` 零错、web-mobile e2e 11/11 全绿（位移后重跑）。

## Spec Change Log

（空）

## Review Triage Log（blind-hunter，1 轮，6 发现全部判定为真 → 全部 patch）

| # | 发现（摘要）                                     | 判定 | 证据/处置                                                                 |
|---|--------------------------------------------------|------|---------------------------------------------------------------------------|
| F1 | 面板侧 `max-md:hidden` 零自动化守卫              | 真   | 两个 Panel.test 加容器类钉孔；容器挂 data-testid                           |
| F2 | e2e 退场断言分不清隐藏 vs 删除、文本选择器脆弱   | 真   | 改 testid + existing/displayed 双断言（沿用侧栏 :83-84 惯例）              |
| F3 | Butler 移动单测缺位置钉孔                        | 真   | 新增「X 在标题行内、不在 tab 行」DOM 归属断言                              |
| F4 | Butler X 的 `ml-auto` 为死类且误导               | 真   | 删该类；注释说明右对齐由铃铛 ml-auto 承担（App 移动路径恒传 onToggleNotif） |
| F5 | 「⋯」菜单展开时点 X 不收菜单                     | 真   | X 与 tabButton 的 onClick 均先 `setShowMoreMenu(false)`（簇内点击绕过 ref 外部关闭） |
| F6 | `RoleHeader.test.tsx` 缺末尾换行（存量）         | 真·低 | 触及文件内顺手补换行                                                       |
