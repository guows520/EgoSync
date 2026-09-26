---
title: 'web 版手机浏览器布局四症状修复'
type: 'bugfix'
created: '2026-09-26'
status: 'done'
route: 'oneshot'
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 手机浏览器（<768px）使用 web 版时四个布局问题：①管家/角色视图点「仪表盘/任务」等 tab 后，对话界面仍占上半屏，未做到只显示对应页面；②通知铃铛与连接状态图标不在头部右端；③角色详情头部「⋯」更多菜单在「设置」按钮左侧；④设置 tab 点任一配置项弹出的 GlobalSettingsModal 顶部出现 4 个 tab 切换按钮，冗余。

**Approach:** 四处定位均在 Story 16.2/16.4 移动化代码中（根因与证据链见调查案卷 `investigations/web-mobile-layout-issues-investigation.md`，用户已选「全项修」并授权解冻 58/42 相关既有测试钉孔）。修复方向：①小屏 tab 打开时工作区全屏、对话区让位（桌面 65/35 双栏零变化），并联动「记忆来源消息跳转」在移动端关闭 tab 回对话；②管家头部改两行布局，铃铛+连接状态右对齐（回归 16.4 冻结线框「右上角」）；③「⋯」按钮移到 tab 组末端（设置按钮右边）；④GlobalSettingsModal 的 tab 切换列在 <768px 隐藏（桌面保留）。桌面路径逐像素零变化是硬红线。

</frozen-after-approval>

## Implementation Notes

### ① tab 全屏化（ButlerView / RoleView）

- `egosync-app/src/components/butler/ButlerView.tsx:166`：对话区 `openTab` 分支 `max-md:flex-[58]` → `max-md:hidden`；`:207-208` 工作区容器去 `max-md:flex-[42]`，改 `max-md:flex-1`（全高），保留 `md:h-auto md:w-[35%]` 桌面双栏。
- `egosync-app/src/components/role/RoleView.tsx:109`：对话区 `openTab` 分支 `max-md:flex-[58]` → `max-md:hidden`；`:128` 工作区 `w-full h-[42%] max-md:flex-[42]` → `w-full max-md:flex-1`（h-[42%] inert 垫片删除，owner 已授权），保留 `md:h-auto md:w-[35%]`；关闭态 `w-0 h-0 md:h-auto opacity-0` 不动。
- 联动（调查 Finding 7）：两视图 `handleSourceMessageClick` 在 `window.matchMedia('(max-width: 767px)').matches` 时先 `setOpenTab(null)` 再设 sourceNavigationTarget——移动端跳来源消息必须回到对话才可见；桌面不关 tab（保持既有双栏零变化）。新增 `isMobileViewport()` 助手函数放 `src/lib/utils.ts`（与 `cn` 同文件），两视图共用。
- 触控返回路径不变：再点一次当前 tab 按钮即关闭回对话（既有 toggle 语义，桌面/移动通用）。

### ② 铃铛+连接状态右对齐（ButlerView 头部）

- `egosync-app/src/components/butler/ButlerView.tsx:102`：header 类 `flex flex-wrap items-center ... justify-between` → `flex flex-col md:flex-row md:flex-wrap md:items-center ... md:justify-between`（去掉 flex-wrap；移动两行、桌面单行）。
- 左簇（图标+标题）内铃铛按钮（`:120`）加 `ml-auto`，连接状态包裹 div（`:135`）随之被推到第一行右端；两者本已 `md:hidden`，桌面不渲染、单行结构与今天一致（gap-2 保持）。
- 无需改 RoleHeader（角色视图无铃铛，16.4 线框定稿如此）。

### ③ 「⋯」移到「设置」右边（RoleHeader）

- `egosync-app/src/components/role/RoleHeader.tsx`：把原 `:145-261` 的 `{onSwitchRole && (<>⋯按钮+下拉菜单+归档删除确认弹窗</>)}` 整块（120 行）移到 `tabButton('settings', ...)` 之后（现 `:145-264`，同一 `ml-auto relative` 簇内末端）。下拉菜单位置锚父容器（`moreMenuRef` + `right-2`）不依赖按钮顺序；桌面本就 `md:hidden`，零变化。

### ④ 设置模态去顶部切换行（GlobalSettingsModal）

- `egosync-app/src/components/settings/GlobalSettingsModal.tsx:651`：nav 容器类表插 `hidden md:flex`（base `flex` 之前插 `hidden`——移动端整列不渲染，≥768px 恢复）。内容区 h3 标题 + X 关闭语义不动；jsdom 不应用 Tailwind 样式，既有 vitest 断言（按名称点击 nav 按钮）不受影响。类表中 `max-md:flex-wrap`/`max-md:[&>button]:*` 变为小屏死类，按硬红线保留不动（注释已注明）。
- 移动端换分区路径 = X 关闭回 MobileSettingsView 列表重选（用户明确要求的交互）。

### 测试改造（owner 已授权解冻 58/42 钉孔）

- `egosync-app/src/components/role/RoleView.mobile.test.tsx`：钉孔按新布局改写——对话区 `max-md:hidden`、工作区 `max-md:flex-1` 且断言 `not.toHaveClass('h-[42%]')`；`md:w-[65%]/md:w-[35%]` 桌面钉孔保留；新增「再点 tab 关闭回对话」「移动端来源跳转关 tab（matchMedia 打桩）+ 桌面对照分支」两 it。
- `egosync-app/src/components/role/RoleView.test.tsx:168-173`：删 `h-[42%]` 断言（inert 垫片删除），`md:w-[35%]` 保留，补 `max-md:flex-1` 与 `not.toHaveClass('h-[42%]')`。
- 新测（新建文件，守冻结规则「新增不改旧」）：`ButlerView.mobile.test.tsx`——①tab 打开时对话区 `max-md:hidden`、工作区 `max-md:flex-1`；②移动端来源跳转关 tab + 桌面对照分支。`RoleHeader.test.tsx` 新增 it——「⋯」按钮 DOM 序在「设置」tabButton 之后（compareDocumentPosition，纯增量）。
- `egosync-app/tests/e2e/web-specs/web-mobile.spec.ts`：仪表盘用例补「chat-pane 不可见、workspace-pane 可见且占高 >60%、连接状态右缘间距 ≤24px」；任务面补「tab 打开后输入框不可见」；设置用例补「nav 切换钮（button*=模型服务配置）isExisting=true/isDisplayed=false」；为 ButlerView 两 pane 加 `data-testid="butler-chat-pane|butler-workspace-pane"` 供断言钩子；文件头注释登记 9 号条目。

### 执行期追加（2026-09-26 回填）

实际触碰文件（9 改 + 1 新建）：`src/lib/utils.ts`（+`isMobileViewport`）、`src/components/butler/ButlerView.tsx`（①+②+pane testid）、`src/components/role/RoleView.tsx`（①+联动）、`src/components/role/RoleHeader.tsx`（③）、`src/components/settings/GlobalSettingsModal.tsx`（④）、`tests/e2e/web-specs/web-mobile.spec.ts`、4 个测试文件（2 改钉孔 + 1 新建 + 1 增量）。

偏差与 surprises：
1. `edit` 工具对含全角标点的注释块多次匹配失败（不可见字节差异）→ 改述区域均改由 Python 按行号/唯一锚点完成；RoleHeader 首轮移动脚本锚点撞重用注释（同一文案两处）曾写坏文件，`git checkout` 恢复后用唯一锚点重做成功。教训：多行中文注释块编辑优先用行号/唯一锚点，勿整段照抄。
2. e2e 首跑 4 失败均为「静态服旧 dist」伪失败——web e2e 服 `egosync-app/dist`（9-25 旧包），`npm run build` 重建后 11/11 全绿。凡 web e2e 环境须先构建再跑。
3. RoleHeader 移动「⋯」块 120 行用脚本搬运后，D2/任务面/⋯ 回根 e2e 与全部单测原样通过（下拉锚父容器与顺序无关的判断得到行为级验证）。
4. GlobalSettingsModal nav 隐藏后，类表 `max-md:flex-wrap max-md:[&>button]:*` 成小屏死类；按「桌面逐像素零变化、只加 max-md:/md: 类对」红线保留不动并加注释，未做清理。
5. 新增 `isMobileViewport()` 采用「点击时点查、不订阅」——jsdom matchMedia 默认 matches=false，天然保证桌面行为与既有测试不回归；单测经 stubGlobal 覆盖移动分支。

## Review Triage Log（blind-hunter，1 轮）

| # | 严重度 | 位置 | 判定 | 处置 |
|---|---|---|---|---|
| F1 | minor | ButlerView.tsx:157-161 注释残留 58/42 描述 | 接受 | 已按 RoleView:97-103 措辞改写 |
| F2 | minor | GlobalSettingsModal.tsx:641/648-650 旧注释与 hidden nav 对冲、max-md 死类 | 接受 | 两处注释重写，死类保留并注明原因 |
| F3 | minor | web-mobile.spec.ts 文件头声称四需求均有断言，③实际在 vitest | 接受 | 措辞改为「①②④见本 spec、③见 RoleHeader.test.tsx」 |
| F4 | minor | RoleView 移动端来源跳转分支零覆盖 | 接受 | RoleView.mobile.test.tsx 补 stubMobileViewport + 关 tab 断言 + 桌面对照分支（5 it 全绿） |
| F5 | nit | spec 行号漂移、执行期追加空置 | 接受 | 本档回填（行号已校正至最终值） |
| F6 | nit | e2e `as number` 强转 | 拒稿 | 评审自陈非缺陷：null 会抛 matcher error 不静默通过，且前置 isDisplayed 已兜底 |

## Verification

**Commands:**
- `cd egosync-app && npx vitest run src/components/butler src/components/role src/components/settings` -- ✅ 28 文件/276 用例全绿（含新测与改写钉孔）；追加全量 `npx vitest run` ✅ 81 文件/940 用例全绿
- `cd egosync-app && npm run build` -- ✅ tsc 零类型错误 + 构建通过（tsc --noEmit 单独复核退出 0）
- `cd egosync-app && npm run build && cd tests/e2e && npx wdio run wdio.web.conf.ts --spec web-specs/web-mobile.spec.ts` -- ✅ 11/11 全绿（Chrome 148 真实宿主；**须先 build：web e2e 静态服 dist**，旧 dist 下 4 条新断言伪失败）

**Manual checks (if no CLI):**
- 375×812 视口四路径复走：管家 tab 全屏/铃铛右端/角色 ⋯ 在设置右/设置模态无切换行；≥1280px 桌面双栏、侧栏铃铛、模态 nav 与今天逐像素一致
- ✅ e2e 已含等价行为断言（chat-pane isDisplayed=false、workspace-pane 占高>60%、连接状态右缘≤24px、nav 切换钮 isDisplayed=false、任务 tab 后输入框不可见）
