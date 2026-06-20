---
baseline_commit: 068c84b
---

# Story 3.6: 任务按四象限分组显示在 TasksTab 中

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 任务按四象限分组展示，每组有清晰的标题、配色与计数，空象限给出鼓励文案，并能折叠/展开,
so that 我能一眼看清哪些紧急、哪些重要，并按需聚焦某个象限。

## 背景与现状（务必先读）

**本 story 是对 `TasksTab.tsx` 的增量重构，不是从零搭建。** 四象限「分组渲染 + 组内大石头优先排序 + 拖拽排序 + 已完成折叠」的骨架在 Story 3.3 引入分类时**已经建立**（`TasksTab.tsx:225-357`）。本 story 仅补齐 Story 3.6 特有的、当前**缺失**的 4 个 AC：

| AC | 需求 | 当前状态 |
|----|------|----------|
| AC1 分组标题**按象限配色** | Q1 红 / Q2 蓝 / Q3 灰 / Q4 淡灰 | ❌ 当前所有标题统一 `text-slate-400`（`TasksTab.tsx:326`） |
| AC2 标题右侧**任务数 badge** | 如 `重要且紧急 (3)` | ❌ 无计数 |
| AC3 空象限**折叠 + 鼓励文案** | 如 Q1 空："没有紧急任务，太棒了！" | ❌ 当前空象限被 `.filter(length>0)` 直接隐藏（`TasksTab.tsx:242`） |
| AC4 组内大石头优先 + `sortOrder` | — | ✅ 已实现（`orderBySort` `TasksTab.tsx:233`），本 story **保持不变** |
| AC5 点击标题**折叠整组 + 300ms 动画** | 折叠态不持久化 | ❌ 当前仅「已完成」子区可折叠，象限组本身不可折叠 |
| AC6 卡片样式复用 + 替换原型单组列表 | GripVertical + Circle + deadline + 大石头 | ✅ 卡片样式已就绪，分组已替换原型，本 story **保持不变** |

## Acceptance Criteria

> 以下 AC 已结合**当前实现**重写为可验证条款。未列为「变更」的既有行为（拖拽、完成、分类中、at_risk 预警）必须原样保留。

1. **四象限分组渲染（替换原 filter 隐藏逻辑）— AC1/AC6**
   - **Given** 角色/管家有任务（`tasks.length > 0`）
   - **When** 打开 TasksTab
   - **Then** 固定渲染 4 个象限分组，顺序恒为 Q1 → Q2 → Q3 → Q4（即使某象限为空也渲染，**移除**当前 `quadrants = allQuadrants.filter(... length > 0)` 的隐藏行为）
   - **And** 每组标题文案沿用 `quadrantLabels`：`Q1 · 重要且紧急` / `Q2 · 重要不紧急` / `Q3 · 紧急不重要` / `Q4 · 不重要不紧急`
   - **And** 标题文字颜色按象限区分：Q1 `text-red-600`、Q2 `text-blue-600`、Q3 `text-slate-600`、Q4 `text-slate-400`
   - **And** 卡片样式（`CARD_BASE_CLASS` + GripVertical 手柄 + Circle 完成圈 + deadline/大石头/at_risk/分类中 badge）保持原样

2. **整体空态优先于象限鼓励文案 — AC3 边界**
   - **Given** 完全没有任务（`tasks.length === 0`）
   - **Then** 仍只显示既有全局空态文案「还没有任务，先添加一个小目标吧」（保留 `TasksTab.tsx:308-311` 分支与对应测试）
   - **And** 此时**不**渲染 4 个象限分组、不渲染各象限鼓励文案

3. **每组标题右侧任务数 badge — AC2**
   - **Given** 任一被渲染的象限组
   - **Then** 标题行右侧显示该象限任务数 badge，数值 = 该象限**全部未删除任务数**（未完成 + 已完成，即 `groupedTasks[quadrant].length`）
   - **And** badge 用中性药丸样式（如 `inline-flex items-center justify-center min-w-[20px] px-1.5 rounded-full bg-slate-100 text-slate-500 text-[11px] font-semibold`）
   - **And** 空象限同样显示 badge，值为 `0`

4. **空象限折叠并显示鼓励文案 — AC3**
   - **Given** 某象限无任务（`groupedTasks[quadrant].length === 0`）
   - **Then** 该组只渲染标题（含配色 + `(0)` badge）+ 一段鼓励文案，**不**渲染任务列表区，也**不**渲染折叠箭头（无内容可折叠）
   - **And** 鼓励文案按象限取自常量映射：
     - Q1：`没有紧急任务，太棒了！`
     - Q2：`暂无重要规划，别忘了为长远目标留出时间`
     - Q3：`没有需要应付的杂事，很清爽`
     - Q4：`没有可有可无的任务，注意力很集中`
   - **And** 文案为弱化样式（如 `text-[12.5px] text-slate-400`）

5. **组内排序：大石头优先，其后按 sortOrder — AC4（保持现状）**
   - **Given** 某象限内有多条未完成任务
   - **Then** 排序规则保持 `orderBySort`：先按 `isBigRock` 降序，再按 `sortOrder` 升序
   - **And** 不得改动该比较函数（Story 3.4 行为，已被测试覆盖）

6. **点击标题折叠/展开整组 + 300ms 动画 — AC5**
   - **Given** 一个**非空**象限组
   - **Then** 标题行整体可点击（`<button>`），左侧显示折叠箭头（`ChevronDown` 展开 / `ChevronRight` 折叠，均已 import）
   - **And** 默认全部展开（非空组初始 `expanded`）
   - **When** 用户点击标题
   - **Then** 该象限的任务区（未完成列表 + 已完成子区）以 ~300ms 动画收起/展开
   - **And** 动画用 Tailwind 实现（推荐 grid-rows 过渡：外层 `grid transition-[grid-template-rows] duration-300 motion-reduce:transition-none`，展开 `grid-rows-[1fr]` / 折叠 `grid-rows-[0fr]`，内层 `overflow-hidden`），**禁止**新增自定义 CSS class
   - **And** 标题 `<button>` 设置 `aria-expanded`，可访问名包含象限标题
   - **And** 折叠状态**不持久化**：仅用组件内 `useState`，组件卸载（切换 tab / 关闭面板）后重新打开默认全展开

7. **既有交互零回归 — AC6**
   - **Given** 分组重构后
   - **Then** 拖拽排序（3.2）、完成/撤销（3.2）、自动分类「分类中」徽章（3.3）、at_risk「被挤压」预警 + 琥珀左竖线（3.5）、已完成子区折叠，全部行为不变
   - **And** `handleDragEnd` 仍按 `allQuadrants` 固定顺序重建 `fullOrder` 全量 `sortOrder`（不受分组折叠影响）
   - **And** 现有全部前端测试保持通过

## Tasks / Subtasks

- [x] 前端 UI：象限配色与计数 badge（AC: 1, 3）
  - [x] 在 `egosync-app/src/components/role/TasksTab.tsx` 新增象限配色常量：`const quadrantTitleColor: Record<TaskQuadrant, string> = { Q1: 'text-red-600', Q2: 'text-blue-600', Q3: 'text-slate-600', Q4: 'text-slate-400' };`
  - [x] 标题由固定 `text-slate-400` 改为 `quadrantTitleColor[quadrant]`，保留 `text-[12px] font-bold tracking-widest`（标题节点由 `<h3>` 改为 `<span>`，因放入 `<button>` 内 `<h3>` 非 phrasing content 会产生非法嵌套；可访问名仍含象限标题）
  - [x] 在标题行右侧渲染计数 badge，值为 `groupedTasks[quadrant].length`，中性药丸样式

- [x] 前端 UI：渲染全部 4 象限 + 空象限鼓励文案（AC: 1, 2, 4）
  - [x] **移除** `const quadrants = allQuadrants.filter(...)`，改为遍历 `allQuadrants`（恒 4 组，顺序 Q1→Q4）
  - [x] 保留 `tasks.length === 0` 的全局空态分支在 4 组渲染**之外、之前**判断；命中时不渲染分组
  - [x] 新增鼓励文案常量 `quadrantEmptyHint`
  - [x] 组渲染分支：`count === 0` → 仅渲染标题（配色 + `(0)` badge）+ `quadrantEmptyHint[quadrant]`（弱化样式 `text-[12.5px] text-slate-400`），不渲染列表、不渲染折叠箭头/按钮

- [x] 前端 UI：象限组折叠/展开 + 300ms 动画（AC: 6）
  - [x] 新增组件内状态：`const [collapsedQuadrants, setCollapsedQuadrants] = useState<Set<TaskQuadrant>>(() => new Set());`（默认空 = 全展开；不持久化）
  - [x] `toggleQuadrant` 切换 Set 成员
  - [x] **非空**象限：标题改为可点击 `<button type="button" onClick aria-expanded={!isCollapsed}>`，左侧加 `ChevronDown`/`ChevronRight`
  - [x] 任务区（未完成 `SortableContext` + 已完成子区）包入 grid-rows 折叠包裹层（外层 `grid transition-[grid-template-rows] duration-300 ease-in-out motion-reduce:transition-none` + `grid-rows-[0fr]/[1fr]`，内层 `overflow-hidden`）
  - [x] 折叠态下任务卡片仍挂载（被裁剪），dnd `SortableContext` 稳定

- [x] 前端测试（AC: 1, 2, 4, 6 + 回归）
  - [x] 在 `egosync-app/src/components/role/TasksTab.test.tsx` 新增 AC1/AC2/AC3-空/AC3-边界/AC6 五个用例
  - [x] 确认既有测试全部仍通过（空态、加载/错误、拖拽手柄、完成、编辑、删除、大石头排序、分类中、at_risk 预警/左竖线、已完成折叠）

- [x] 测试与验证（AC: 1-7）
  - [x] `npm --prefix "egosync-app" run test:frontend` → 18 文件 199 测试全通过
  - [x] `npm --prefix "egosync-app" run build`（`tsc && vite build`）→ 通过，无 strict / noUnusedLocals 报错

## Dev Notes

### Current State（基于当前代码 @ 068c84b）

- **本 story 纯前端**：后端 `quadrant` 列、分类、`sort_order`、`protection_status` 均已就绪，**无 Rust / DB / 迁移 / 类型改动**。
- **`TasksTab.tsx` 关键现状**：
  - `quadrantLabels`（`:173-178`）：4 象限标题文案，已含 `Q1 · 重要且紧急` 等，**复用**，仅补颜色与计数。
  - 分组（`:225-231`）：`groupedTasks = tasks.reduce(... { Q1:[],Q2:[],Q3:[],Q4:[] })`，按 `task.quadrant` 分桶。
  - `orderBySort`（`:233`）：`(Number(b.isBigRock)-Number(a.isBigRock)) || (a.sortOrder-b.sortOrder)`，**AC4 已满足，禁止改动**。
  - `allQuadrants`（`:234`）= `Object.keys(quadrantLabels)`，顺序 Q1→Q4。
  - `incompleteByQuadrant` / `completedByQuadrant`（`:235-241`）：各象限按完成态拆分并 `orderBySort` 排序。
  - **隐藏空象限**（`:242`）：`const quadrants = allQuadrants.filter(q => groupedTasks[q].length > 0)` —— **本 story 移除**，改遍历 `allQuadrants`。
  - 全局空态（`:308-311`）：`tasks.length === 0` → 「还没有任务，先添加一个小目标吧」，**保留**。
  - 组渲染（`:320-357`）：`quadrants.map(...)`，标题 `<h3 className="text-[12px] font-bold tracking-widest text-slate-400 uppercase">{quadrantLabels[quadrant]}</h3>`（`:326`）—— 改配色 + 加 badge + 外包折叠层 + 标题改 `<button>`。
  - 已完成子区折叠（`expandedCompleted` 状态 `:187-195`，渲染 `:337-353`）：**与本 story 的「象限组折叠」是两层不同折叠，互不替代**。象限折叠包住整段（含已完成子区）；已完成子区折叠保持原样。
  - 拖拽（`:261-289`）：`handleDragEnd` 用 `incompleteByQuadrant` 定位象限、`arrayMove`，再按 `allQuadrants` 顺序重建 `fullOrder`（`:278-283`，已遍历全部象限，不依赖被过滤的 `quadrants`）—— **无需改动**。
  - 图标：`ChevronDown` / `ChevronRight` 已 import（`:2`），折叠直接复用，**不新增依赖**。
- **类型**：`TaskQuadrant = 'Q1'|'Q2'|'Q3'|'Q4'`（`types/task.ts:1`），`Record<TaskQuadrant, string>` 常量天然类型安全。
- **挂载方式（决定 AC6「不持久化」）**：`RoleWorkspacePanel.tsx:140`（`{currentTab === 'tasks' && <TasksTab .../>}`）与 `ButlerWorkspacePanel.tsx:118` 同款条件渲染；`RoleView.tsx:106` 整个面板也 `{openTab && ...}` 条件渲染。⇒ 切走 tab / 关面板即**卸载** TasksTab，组件级 `useState(collapsedQuadrants)` 自动复位为全展开，满足「每次打开默认全展开」。无需 localStorage / 后端持久化。

### What This Story Changes

仅 2 个文件：

1. **`egosync-app/src/components/role/TasksTab.tsx`**：
   - 新增 `quadrantTitleColor` / `quadrantEmptyHint` 两个 `Record<TaskQuadrant,string>` 常量。
   - 新增 `collapsedQuadrants` 状态 + `toggleQuadrant`。
   - 移除空象限过滤（`:242`），改遍历 `allQuadrants`（恒 4 组）。
   - 标题：配色化 + 右侧计数 badge + 非空组改可点击 `<button>` + Chevron 箭头。
   - 任务区外包 grid-rows 300ms 折叠层。
   - 空象限分支：标题 + `(0)` badge + 鼓励文案，无列表无箭头。
2. **`egosync-app/src/components/role/TasksTab.test.tsx`**：新增 AC1/AC2/AC3/AC6 测试，并保留全部既有用例。

### What Must Be Preserved（防回归）

- **拖拽排序（3.2）**：`handleDragEnd` / `fullOrder` / `SortableContext` / `pointerWithinFallbackToClosestCenter` 全部不动。`fullOrder` 已遍历 `allQuadrants`，分组恒显与折叠均不影响其正确性。折叠组内的卡片仍挂载（grid-rows-0fr 裁剪），dnd 上下文稳定。
- **大石头优先排序（3.4）**：`orderBySort` 一字不改（已被「大石头任务在同象限内排在非大石头任务之前」测试锁定）。
- **完成 / 撤销（3.2）**：完成圈、灰显沉底、已完成子区折叠（`expandedCompleted`）保持。
- **自动分类（3.3）**：`classifyingIds` → 「智能分类中…」徽章保持；`task:classified` 事件链路不在本 story。
- **at_risk 预警（3.5）**：`protectionStatus === 'at_risk'` 的「被挤压」badge + `border-l-4 border-l-amber-400` 左竖线 + DragOverlay 镜像左竖线，全部保持。
- **全局空态**：`tasks.length === 0` → 「还没有任务，先添加一个小目标吧」保留（对应现有测试 `TasksTab.test.tsx:264-275`）。
- **两个消费方零改动**：`RoleWorkspacePanel`（role 维度）与 `ButlerWorkspacePanel`（butler 维度，`role={{color:'#6366F1'}}`）都不传新 prop；`TasksTabProps` 接口不变。
- **现有全部前端测试保持通过**（`TasksTab.test.tsx` 13 例 + 相关面板测试）。

### 关键正确性要点（极易踩坑）

- **两层折叠不要混淆**：本 story 的「象限组折叠」（`collapsedQuadrants`）包住整组（未完成列表 + 已完成子区）；既有「已完成子区折叠」（`expandedCompleted`）是组内的二级折叠。两者并存，象限折叠在外层。测试断言折叠用各自的 `aria-expanded`，**不要**用同一按钮名混测。
- **折叠动画用 grid-rows 而非 `max-height`**：内容高度可变（任务数不定），`max-height` 需猜测峰值且回弹生硬；`grid-rows-[0fr]↔[1fr]` + 内层 `overflow-hidden` 是 Tailwind 原生、对任意高度平滑的方案。务必加 `motion-reduce:transition-none`（项目无障碍要求）。
- **折叠时卡片仍在 DOM**：grid-rows-0fr 是裁剪不是卸载，故 RTL 的 `queryByText` 仍能找到折叠内容。**AC6 测试以 `aria-expanded` 断言折叠状态**，不要用「文本消失」断言（会误判）。这也是有意为之：保持 `SortableContext` 挂载，避免 dnd 重建抖动。
- **空象限不要渲染折叠箭头/按钮**：空组无可折叠内容，标题不做成 toggle 按钮（否则 `getAllByRole('button')` 计数变化可能影响断言；也无 UX 意义）。
- **计数语义已定**：badge = `groupedTasks[quadrant].length`（含已完成）。空象限判定同样用 `=== 0`。若产品希望「仅未完成计数」，见文末澄清问题——当前实现按含已完成。
- **顺序恒定**：象限渲染顺序必须 Q1→Q2→Q3→Q4（用 `allQuadrants`，它来自 `Object.keys(quadrantLabels)` 的声明顺序），不要因 `Set`/对象遍历产生不确定顺序。
- **TS strict / noUnusedLocals**：新增常量/状态若未使用会编译失败；`build` 必须过 `tsc`。

### Previous Story Intelligence

- **3.3**：四象限分组 + `groupedTasks` + `orderBySort` 的骨架就是在这里随自动分类一起加入 TasksTab 的——本 story 是在此之上「补配色/计数/空态/折叠」，而非重写。Task fixture 必须含全字段（`quadrant`、`confidence`、`manualOverride`、`classificationReason` 等）。
- **3.4**：`orderBySort` 大石头优先逻辑 + 其测试（「大石头任务在同象限内排在非大石头任务之前」）已存在，改动分组渲染时**不得破坏该断言**（它依赖 `getAllByRole('button',{name:/编辑/})` 的 DOM 顺序）。
- **3.5**：at_risk 预警的 badge/左竖线/DragOverlay 镜像三处渲染都在 `TaskCardBody` / `SortableTaskCard` / `DragOverlay`，本 story 不碰卡片层，只碰「组」层。3.5 也示范了 TasksTab 测试中「用 `aria-label`/className 断言 + `.closest('[class*="rounded-xl"]')`」的写法，可复用。
- **通用**：前端 Vitest + Testing Library；仅 Tailwind utility，禁自定义 CSS；图标用 `lucide-react`（已依赖）；验证命令见下。

### Git Intelligence

- `068c84b`（当前 HEAD，本 story baseline）— `feat(tasks): Q2 任务保护属性 at_risk 预警 (story 3.5)`
- `b248df2` — task owner scope + 大石头完成即释放名额（方案 D）
- `c4248c6` — async classification + event + Q2-only escalation（3.3，引入了 TasksTab 分组骨架）
- `0871096` — fix story 3.2 drag-sort
- 相关范本：`TasksTab.tsx` 自身既有的 `expandedCompleted` 折叠（条件渲染版）可作为「折叠状态用组件 useState」的范例，但本 story 升级为带 300ms 动画的 grid-rows 版。

### Testing Requirements

- 前端测试 co-located：`egosync-app/src/components/role/TasksTab.test.tsx`（已存在，追加用例）。
- Task fixture 必须含**所有字段**（参考文件顶部 `tasks` / `completedTask` 字面量），新建对象用 `{ ...tasks[0], id, title, quadrant }` 派生最稳。
- 断言要点：
  - 配色：取标题节点（按文案 `getByText('Q1 · 重要且紧急')` 或其容器）断言 `className` 含目标颜色类。
  - 计数：断言象限标题行内出现对应数字（注意与任务标题/deadline 中的数字区分，建议用 badge 容器作用域或精确文本）。
  - 折叠：`getByRole('button', { name: /Q1 · 重要且紧急/ })` → 断言 `aria-expanded` 初始 `'true'`，点击后 `'false'`。
  - 空象限：`queryByText('没有紧急任务，太棒了！')` 等按需断言出现/不出现。
- 必跑验证命令：
  - `npm --prefix "egosync-app" run test:frontend`
  - `npm --prefix "egosync-app" run build`
- 说明：本 story 无 Rust 改动，无需 `cargo test`（如顺手回归可跑，但非本 story 范围）。

### Project Structure Notes

- **无新增文件、无新增依赖、无新增迁移、无类型改动**。
- 修改文件清单（仅 2 个）：
  - `egosync-app/src/components/role/TasksTab.tsx`
  - `egosync-app/src/components/role/TasksTab.test.tsx`
- 符合「前端组件按域分目录」「仅 Tailwind」「图标用 lucide-react」「不在 App.tsx 新增组件」等项目规则。

### References

- `_bmad-output/project-context.md`（前端规则：Tailwind-only、strict TS、组件分域、无障碍 prefers-reduced-motion）
- `_bmad-output/planning-artifacts/epics.md:1510-1545`（Story 3.6 定义）
- `_bmad-output/planning-artifacts/ux-design-specification.md:763-770`（RoleWorkspacePanel 任务面板：四象限 + 大石头）、`:777-778`（动效统一 CSS transition + prefers-reduced-motion）
- `_bmad-output/implementation-artifacts/3-3-auto-quadrant-classification.md`（分组骨架来源）
- `_bmad-output/implementation-artifacts/3-4-big-rock-marking.md`（`orderBySort` 与其测试）
- `_bmad-output/implementation-artifacts/3-5-q2-protection-at-risk.md`（at_risk 渲染与 TasksTab 测试写法）
- `egosync-app/src/components/role/TasksTab.tsx`（`quadrantLabels:173-178`、`orderBySort:233`、`allQuadrants:234`、`incomplete/completedByQuadrant:235-241`、空象限过滤 `:242`、全局空态 `:308-311`、组渲染/标题 `:320-357,326`、已完成折叠 `:337-353`、拖拽 `:261-289`）
- `egosync-app/src/components/role/TasksTab.test.tsx`（既有 13 例，含空态 `:264-275`、大石头排序 `:328-386`、at_risk `:388-426`）
- `egosync-app/src/types/task.ts:1`（`TaskQuadrant`）
- `egosync-app/src/components/role/RoleWorkspacePanel.tsx:140`、`egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:118`、`egosync-app/src/components/role/RoleView.tsx:106`（条件挂载 → 折叠态不持久化）

## Dev Agent Record

### Agent Model Used

Amelia (Senior Software Engineer) — Cascade

### Debug Log References

无（一次性通过，无需调试循环）。

### Completion Notes List

- AC1：新增 `quadrantTitleColor`，标题按象限配色（Q1 红 / Q2 蓝 / Q3 灰 / Q4 淡灰）。标题节点由 `<h3>` 改为 `<span>`，避免在 `<button>` 内嵌入非 phrasing 内容（HTML 合法性）；可访问名仍含象限标题文案。
- AC2：标题行右侧新增中性药丸计数 badge，值为 `groupedTasks[quadrant].length`（含已完成），空象限同样显示 `0`。
- AC3：移除空象限过滤，恒渲染 4 组 Q1→Q4；空象限仅渲染标题 + `(0)` badge + 弱化鼓励文案（`quadrantEmptyHint`），无列表、无折叠按钮。全局空态（`tasks.length===0`）仍优先于象限鼓励文案。
- AC4：`orderBySort` 一字未改，大石头优先排序保持。
- AC6：新增组件内 `collapsedQuadrants` (`Set<TaskQuadrant>`) + `toggleQuadrant`，默认全展开、不持久化（依赖 TasksTab 条件挂载，卸载即复位）。非空组标题为可点击 `<button aria-expanded>`，任务区包入 grid-rows 300ms 折叠层（含 `motion-reduce:transition-none`），折叠态卡片仍挂载以稳定 dnd。
- AC7：拖拽（`handleDragEnd`/`fullOrder`）、完成/撤销、分类中徽章、at_risk 预警+左竖线、已完成子区折叠均零改动；既有 13 例 TasksTab 测试 + 其它面板测试全部仍通过。
- 仅 Tailwind utility，无新增依赖、无 Rust/DB/类型改动。

### File List

- `egosync-app/src/components/role/TasksTab.tsx`（修改）
- `egosync-app/src/components/role/TasksTab.test.tsx`（修改：新增 5 个 AC 用例；code-review 再补 3 个 → 共 8 个新增）

### Change Log

- 2026-06-20：实现 Story 3.6 —— 四象限分组配色 + 计数 badge + 空象限鼓励文案 + 象限组 grid-rows 300ms 折叠/展开；移除空象限隐藏过滤，恒渲染 Q1→Q4。新增前端测试 AC1/AC2/AC3/AC6，`test:frontend`（199 通过）与 `build` 均通过。状态 → review。
- 2026-06-20（code-review 修复）：落实评审 4 项 patch —— ① 折叠区抽 `CollapsibleSection` 并在折叠态加 `inert`（键盘/读屏不再进入收起内容）；② 空象限标题补 `w-3.5` 占位对齐；③ 折叠 toggle 加 `aria-controls` + `useId` 派生 region id；④ 新增 3 个测试（AC3 `(0)` badge、AC6 动画类、inert/aria-controls）。`test:frontend` 202/202、`build` 均通过。状态 → done。

### Review Findings

**评审日期**：2026-06-20 ｜ **评审模式**：full（对照故事规格）｜ **Diff**：未提交改动（baseline 068c84b）
**测试取证（修复后）**：`TasksTab.test.tsx` 21/21 通过（18 旧 + 3 新）；全量套件 202/202 通过（`build` 亦通过）。评审中观察到的 `ButlerSettingsContent.test.tsx` flaky 失败本次复跑已通过，确认与本 diff 无关。
**结论**：AC1–AC6 全部满足。1 项需决策（已采纳 inert 方案）、3 项可修补 → 均已修复；2 项遗留（既有/越界）；7 项噪声已丢弃。

#### 需决策（Decision Needed）

- [x] [Review][Decision] 折叠态象限内容仍可被键盘聚焦 / 暴露给辅助技术 — 折叠用 `grid-rows-[0fr]` + 内层 `overflow-hidden`（`TasksTab.tsx:388-394`）裁剪到 0 高度，但内部交互控件并非 `display:none`，仍保留在 Tab 焦点顺序与无障碍树中。**已决策（2026-06-20，boss）：选「现在顺手修」→ `inert` 方案，转为下方 Patch P0。**

#### 可修补（Patch）

- [x] [Review][Patch] P0：折叠态内层包裹用 `inert` 退出焦点顺序与无障碍树（决策 #1 落地） — ✅ 已修复：抽出 `CollapsibleSection`（`TasksTab.tsx`），`useEffect` 按 `collapsed` 经 ref `setAttribute('inert')`/`removeAttribute`
- [x] [Review][Patch] 空象限标题缺少箭头宽度缩进，与非空组标题左缘错位 — ✅ 已修复：空象限标题前加 `w-3.5 shrink-0` 占位 span 对齐 chevron 宽度
- [x] [Review][Patch] 折叠 toggle 缺 `aria-controls` 关联被控折叠区（a11y 增强） — ✅ 已修复：toggle 加 `aria-controls={regionId}`，`CollapsibleSection` 持同名 `id`（`useId` 派生，防多实例冲突）
- [x] [Review][Patch] 测试缺口：AC6 折叠层动画类 + AC3 空象限 `(0)` badge — ✅ 已修复：新增 3 个用例（AC3 `(0)` badge / AC6 `grid-rows`+`duration-300`+`motion-reduce` 切换 / inert+aria-controls）

#### 遗留（Defer，既有/越界）

- [x] [Review][Defer] `task.quadrant` 取值非 Q1–Q4 时 `groupedTasks[task.quadrant].push` 抛错（既有 reduce，未被本 diff 改动，仅被其依赖） [TasksTab.tsx:252-258] — deferred, pre-existing
- [x] [Review][Defer] 全量套件 flaky 失败 `ButlerSettingsContent.test.tsx`（孤立运行通过），与故事「199 全通过」声明不符，但与 3-6 diff 无因果关系 [ButlerSettingsContent.test.tsx:95] — deferred, pre-existing
