# Investigation: 仪表盘指标与筛选布局

## Hand-off Brief

五项仪表盘问题均已定位到当前未提交的 `DashboardTab.tsx` 活动统计实现：日期控件粒度、筛选容器换行、指标配置顺序/文案及卡内左聚集布局。后端已正确实现 `[startAt,endAt)` 半开区间，日期筛选只需在前端把开始日映射到当天零点、结束日映射到次日零点，无需改动 IPC、Rust 或 SQL。建议仅修改 `DashboardTab.tsx` 与其测试；卡片留白采用“左侧图标和名称、右侧数字”的明确布局，以直接利用卡片横向空间。
## Case Info

| Field | Value |
| --- | --- |
| Case ID | N/A |
| Date opened | 2026-07-24 |
| Status | Concluded |
| System | Windows；React 18 + TypeScript + TailwindCSS 前端 |
| Evidence sources | 用户描述、前端源码、测试、版本控制 |

## Problem Statement

用户报告：

- 仪表盘的时间选择，只需要选日期即可，不需要精确到具体时间点，三个筛选项放在一行即可
- “待处理任务”放在左下角
- “对话会话”更改为“对话数量”，然后放在右下角
- “记忆条目”更改为“记忆数量”
- 四个指标卡的右边的空白有点大

以上均作为待独立验证的界面问题与目标状态；本轮暂不直接执行修改。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户问题描述 | Available | 提供目标状态，尚不等同于实现根因 |
| `egosync-app/src/components/butler/DashboardTab.tsx` | Available | 精确文案命中三项指标，确认为主要源码锚点 |
| 前端测试 | Partial | 已发现 DashboardTab 测试文件，尚未读取覆盖范围 |
| UI 截图/实际渲染尺寸 | Missing | 用户未提供截图，像素级“空白过大”需由布局代码与可选渲染验证推断 |
| CodeGraph 工具 | Partial | `.codegraph` 索引存在，但当前任务未暴露 `codegraph_*` MCP 调用入口 |
| 版本控制历史 | Available | 尚未进入证据边界扫描 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 阅读 DashboardTab 导出、组件布局和日期筛选实现 | High | Open | 确认五项问题是否集中于单组件 |
| 2 | 阅读 DashboardTab 直接装配点与相关公共组件 | High | Open | 确认容器宽度及外部布局约束 |
| 3 | 阅读 DashboardTab 测试 | High | Open | 判断文案、顺序、日期粒度的现有契约 |
| 4 | 检查日期参数类型及 useDashboard 调用链 | Medium | Open | 区分纯展示问题与接口契约问题 |
| 5 | 查看相关 git 历史 | Medium | Open | 判断当前布局设计来源及最近变更 |
| 6 | 形成最小修改方案与验证计划 | High | Open | 不直接实施 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-24 | 用户提出五项仪表盘 UI 调整诉求 | 用户描述 | Confirmed |
| 2026-07-24 | 三项指标文案定位到 DashboardTab.tsx 第 84-86 行 | 源码文字检索 | Confirmed |

## Confirmed Findings

### Finding 1: 指标文案由 DashboardTab 内的同一配置区域定义

**Evidence:** `egosync-app/src/components/butler/DashboardTab.tsx:84-86`

**Detail:** “记忆条目”“对话会话”“待处理任务”均在连续的指标定义中出现，确认该组件是指标命名与排列调查的首要锚点。

## Deduced Conclusions

尚未进入源码追踪阶段。

## Hypothesized Paths

### Hypothesis 1: 五项表现主要由 DashboardTab 的局部 JSX/Tailwind 布局配置造成

**Status:** Open

**Theory:** 指标名称、数组顺序、网格布局和日期控件很可能集中在同一组件，因而可通过局部修改完成。

**Supporting indicators:** 三项指标文案均命中 `DashboardTab.tsx:84-86`。

**Would confirm:** 组件源码显示日期筛选、指标数组及网格/卡片样式均在该文件内定义，且外部容器无关键覆盖。

**Would refute:** 日期粒度受后端接口强制约束，或指标布局由共享组件/父容器决定。

**Resolution:** 待源码与调用链检查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| DashboardTab 完整源码与调用关系 | 无法确认根因和最小改动范围 | 只读检查组件、直接装配点及 hook |
| 现有测试契约 | 无法确认需更新或新增哪些测试 | 读取 DashboardTab 与 useDashboard 测试 |
| 实际界面截图/目标尺寸 | 无法量化“右边空白过大” | 先由样式代码定位；必要时再请求截图 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | 初步锚点：`egosync-app/src/components/butler/DashboardTab.tsx:84-86` |
| Trigger | 用户进入管家工作区的仪表盘标签 |
| Condition | 待源码追踪 |
| Related files | `DashboardTab.test.tsx`、装配组件、`useDashboard.ts` |

## Conclusion

**Confidence:** Low

当前仅确认指标文案的主要源码锚点；尚未读取实现，因此不能把布局原因宣称为已确认。

## Recommended Next Steps

### Fix direction

待源码追踪后确定；预计优先采用 DashboardTab 内的外科手术式修改，而非重构。

### Diagnostic

读取组件、直接装配点、hook、测试及相关版本历史，分别验证日期粒度、三项筛选横排、指标顺序/命名和卡内空白来源。

## Reproduction Plan

打开仪表盘，检查筛选控件输入粒度和横向排列；在常用窗口宽度下检查四张指标卡的阅读顺序及图标/数字区域占比。

## Side Findings

- `rg.exe` 在当前 Codex WindowsApps 路径因“拒绝访问”无法启动，已改用 PowerShell `Select-String` 完成精确文字检索；不影响已有强证据。

## Outcome 2: Evidence Perimeter（2026-07-24）

### Evidence classification

| Category | Status | Evidence |
| --- | --- | --- |
| Diagnostic archive / runtime logs | Missing | 本问题为 UI 目标差异，当前无运行时错误或日志输入 |
| Issue/story context | Partial | 工作区存在未跟踪的 Story 11.1 文档，但本阶段未将其作为用户输入读取 |
| Version control | Available | `git status` 显示 Dashboard 前后端及测试存在大批未提交修改；相关已提交历史最近为 `3b92565`、`2daf3cd`、`92a316e` |
| Test sources | Available | `DashboardTab.test.tsx`、`DashboardTab.a11y.test.tsx`、`useDashboard.test.ts` 均存在且当前也有未提交修改 |
| Test execution results | Missing | 本阶段未运行测试，不能宣称测试通过或失败 |
| Static analysis/build | Partial | `package.json` 提供 `build: tsc && vite build`，本阶段未运行 |
| Frontend source | Available | 当前工作树 `DashboardTab.tsx` 已包含活动统计、日期时间筛选和 2×2 指标网格 |
| Data contract/backend | Available | Dashboard 类型、service、Tauri command/model/service 均在当前未提交变更范围内，后续需确认日期边界语义 |
| Actual rendered screenshot | Missing | 无法对“右侧空白”做像素级量化，但源码足以确认当前 flex 内容未占满卡片宽度 |
| CodeGraph | Partial | 本地索引存在且处于运行状态，但当前工具面未暴露 `codegraph_*` 调用入口；对已明确文件采用只读检查 |

### Perimeter findings

1. **Confirmed:** 日期筛选当前明确使用 `type="datetime-local"`，开始/结束值均精确到分钟。证据：`egosync-app/src/components/butler/DashboardTab.tsx:118-131`。
2. **Confirmed:** 筛选容器使用 `flex flex-wrap`，因此实现允许控件换行，不保证三个筛选项始终同一行。证据：`egosync-app/src/components/butler/DashboardTab.tsx:104`。
3. **Confirmed:** 2×2 网格顺序由数组顺序按行填充；当前顺序为任务总数、记忆条目、对话会话、待处理任务，因此底部左/右与用户目标相反。证据：`egosync-app/src/components/butler/DashboardTab.tsx:82-87,142-162`。
4. **Confirmed:** 指标卡使用 `flex items-center gap-3`，内容区只有 `min-w-0`，没有 `flex-1`、左右分布或受控卡宽；网格列会拉伸卡片，但内部内容保持左聚集，形成右侧剩余空间。证据：`egosync-app/src/components/butler/DashboardTab.tsx:142-159`。
5. **Confirmed:** 上述活动统计区是当前工作区尚未提交的新实现，而非稳定基线；相关 Dashboard 改动跨 10 个文件、约 1062 行新增。后续方案必须保护用户现有改动，不能基于 HEAD 覆盖当前工作树。
6. **Confirmed:** `DashboardTab` 由 `ButlerWorkspacePanel` 的 `p-6` 可滚动内容区直接渲染。证据：`egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:122-124`。外层提供内边距，但四张卡的右侧内部空白来源仍在卡片自身布局。

### Backlog update

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | DashboardTab 日期与布局实现 | High | Done | 已确认控件类型、wrap、网格顺序、卡内 flex |
| 2 | 直接装配点与外层容器 | High | Done | 已确认 `ButlerWorkspacePanel` 直接装配及 `p-6` 容器 |
| 3 | 测试契约精确断言 | High | In Progress | 文件可用，下一阶段聚焦相关断言而非整文件泛读 |
| 4 | 日期参数与后端边界语义 | High | Open | 需判断日期结束值应为当天起点、次日起点还是当天末尾 |
| 5 | Git 历史及当前未提交差异 | Medium | Done | 已确认功能位于当前未提交工作树 |
| 6 | 根因、最小方案与验证计划 | High | Open | 下一阶段完成，不实施 |

## Outcome 3: Cause Reasoning（2026-07-24）

### Confirmed findings

1. **时间粒度不符合需求的直接原因是输入控件和转换函数都按“日期时间”设计。** `DashboardTab.tsx:54-79` 将控件值直接解析为具体时刻并转 ISO；`DashboardTab.tsx:118-131` 使用 `datetime-local`。
2. **结束日期不能直接转换为所选日期零点。** 后端统一采用半开区间 `[startAt, endAt)`：任务查询见 `src-tauri/src/db/tasks.rs:661-664`，记忆查询见 `src-tauri/src/db/memories.rs:418-422`，对话查询见 `src-tauri/src/db/conversations.rs:465-469`；聚合测试明确断言 end 边界外，见 `src-tauri/src/services/dashboard_service.rs:552-576`。
3. **三个筛选项换行是显式布局行为，不是浏览器偶发问题。** `flex-wrap` 允许换行；“至”还作为独立第四个 flex 子项存在，视觉上虽是三组筛选，但 DOM 实际为四项。
4. **左下/右下错误完全由数据数组顺序导致。** CSS Grid 按源顺序逐行放置；无需改变网格结构。
5. **名称错误是纯展示配置问题。** `memoryCount`、`conversationCount` 数据字段与计数语义已经正确，不需要重命名接口字段。
6. **右侧空白是拉伸网格列与左聚集卡内 flex 组合产生。** 不是数据缺失，也不是父容器异常；缺少明确设计选择：要么卡片保持满宽并利用右侧空间，要么限制指标区宽度/卡片宽度。
7. **现有前端测试只断言开始/结束控件存在。** `DashboardTab.test.tsx:267-281` 没有锁定 `type=date`、日期边界转换、指标文案或网格顺序，因此当前测试不能防止这些 UX 偏差。

### Hypothesis resolutions

#### Hypothesis 1: 五项表现主要由 DashboardTab 局部配置造成

**Status:** Confirmed

**Resolution:** 文案、数组顺序、输入类型、转换函数、筛选 flex 和卡片 flex 均位于 `DashboardTab.tsx`。父容器只负责通用滚动与 `p-6`，未覆盖这些行为。

**Refutation pass:** 检查了父容器、hook、前端 service 和后端时间查询。未发现父容器强制卡片内部留白，也未发现接口要求用户输入分钟精度；因此未能推翻该假设。

#### Hypothesis 2: 改成日期筛选需要修改后端查询机制

**Status:** Refuted

**Resolution:** 后端已稳定支持 RFC 3339 半开区间。前端只需把开始日期转换为本地当天零点，把结束日期转换为本地“次日零点”，再调用现有 `toISOString()`；无需改变 Rust/SQL 契约。

#### Hypothesis 3: 右侧空白只需给文字区加 `flex-1`

**Status:** Refuted

**Resolution:** `flex-1` 只会让一个不可见的文字容器占满剩余空间；若内部没有右对齐元素，视觉内容仍集中在左侧，不能消除用户感知的空白。必须先选择“重排卡内信息”或“缩窄卡片/指标区”中的一种机制。

### Deduced conclusions

- 最小功能改动可以限制在 `DashboardTab.tsx` 和其测试文件；后端、IPC 类型、hook 状态结构均可保持不变。
- 日期输入展示应使用 `YYYY-MM-DD`，但传输层继续保留 ISO 时间字符串，这是与现有契约冲突最少的方案。
- 结束日期应转换为次日零点，而不是 `23:59:59.999`：前者与现有半开区间及秒精度规范完全一致，避免毫秒被后端格式化丢弃。
- 指标顺序目标应为：左上任务总数、右上记忆数量、左下待处理任务、右下对话数量。
- “右侧空白”缺少截图，因此具体视觉方案仍有一个产品选择；根因已确认，但最终推荐需在源码追踪结论中明确择一并说明取舍。

## Outcome 4: Source Code Trace（2026-07-24）

### Trace summary

| Element | Detail |
| --- | --- |
| UI entry | `egosync-app/src/components/butler/ButlerWorkspacePanel.tsx:122-124` 在工作区内容区域装配 `DashboardTab` |
| Filter rendering | `egosync-app/src/components/butler/DashboardTab.tsx:104-133` |
| Date conversion | `DashboardTab.tsx:54-79` |
| Filter state / request trigger | `egosync-app/src/hooks/useDashboard.ts:17-20,55-88`；scope/timeRange 变化触发 metrics 请求 |
| IPC service | `egosync-app/src/services/dashboardService.ts:4-7` 原样传递 query |
| Backend normalization | `egosync-app/src-tauri/src/services/dashboard_service.rs:90-120` 将 RFC 3339 统一为 UTC，并验证 start < end |
| Query boundary | tasks `src-tauri/src/db/tasks.rs:661-664`；memories `src-tauri/src/db/memories.rs:418-422`；conversations `src-tauri/src/db/conversations.rs:465-469`，均为 `[start,end)` |
| Metric configuration | `DashboardTab.tsx:82-87` |
| Metric layout | `DashboardTab.tsx:142-162` |
| Existing UI test gap | `DashboardTab.test.tsx:267-281` 仅验证两个时间控件存在 |

### Causal flow

```text
用户选择日期/Agent
  → DashboardTab 更新 scope 或 timeRange
  → useDashboard effect 重新请求
  → dashboardService 通过 Tauri invoke 原样发送 ISO query
  → dashboard_service 解析并转 UTC
  → tasks/memories/conversations 使用 start <= timestamp < end 计数
  → metrics 返回 DashboardTab
  → metricCards 源顺序决定 2×2 位置
```

### Minimal change design（不执行）

#### A. 日期输入与边界转换

修改 `DashboardTab.tsx`：

1. 两个 input 从 `datetime-local` 改为 `date`。
2. 开始日期输入值按本地时区构造当天零点，再 `toISOString()`。
3. 结束日期输入值按本地时区构造所选日期的**次日零点**，再 `toISOString()`。
4. 回填结束日期时必须把保存的排他上界减一个本地日后显示。不能直接格式化 `endAt`，否则用户选择 7 月 24 日后控件会显示 7 月 25 日。
5. 日期加减使用 `setDate(getDate() ± 1)`，不要加减固定 86,400,000 毫秒，以避免夏令时地区跨日错误。
6. 继续保留 ISO 状态和现有 hook/service/backend 契约，不扩大修改范围。

#### B. 三项筛选同排

修改筛选容器：

- 移除 `flex-wrap`，保持单行；
- 日期输入从 datetime-local 变为 date 后宽度自然缩短；
- “至”仅作为分隔文本，不视为筛选项；
- 为避免窄宽度挤压，应给 Agent select 设置可收缩边界，并保持两个日期控件 `shrink-0`。

此处明确选择“单行优先”，不保留自动换行，因为自动换行与用户目标直接冲突。极窄窗口可接受容器横向溢出或由父级最小宽度承担，不能同时宣称永不溢出且永不换行。

#### C. 名称与顺序

仅调整 `metricCards`：

1. 任务总数
2. 记忆数量
3. 待处理任务
4. 对话数量

数据字段保持 `taskCount`、`memoryCount`、`pendingTaskCount`、`conversationCount` 不变。

#### D. 卡片右侧空白

推荐采用最小视觉修复：在现有卡片 flex 上增加 `justify-center`，保持“图标 + 上下两行文字”结构不变，使剩余空间左右均衡，而不是集中在右侧。

未选择以下方案：

- `flex-1`：只扩张不可见内容容器，不能改善可见内容左聚集；
- 数字强制右对齐：能利用空间，但需要重构卡内 JSX，超出“空白有点大”的最小调整；
- 限制整个指标区宽度：会在指标区外制造新的大块空白，且影响父布局。

如实际截图显示用户希望数字靠右，而非整体居中，则应显式切换到“左侧图标/名称 + 右侧数字”方案，不应把两种模式混合。

### Required test changes（不执行）

修改 `DashboardTab.test.tsx`：

- 断言开始/结束控件的 `type` 为 `date`；
- 模拟选择开始日期，断言请求中的 `startAt` 对应本地当天零点的 ISO；
- 模拟选择结束日期，断言 `endAt` 对应本地次日零点的 ISO；
- 验证结束日期回填仍显示用户选中的日期，而不是次日；
- 断言四个卡片的名称和 DOM 顺序为目标顺序。

可在 `DashboardTab.a11y.test.tsx` 保留原有可访问名称逻辑；若测试硬编码旧卡名，则同步更新为新展示名称。

后端半开区间已有明确测试，不建议为纯前端日期转换重复修改 Rust 测试。

### Verification commands（实施后才运行）

1. `npm run test:frontend -- DashboardTab.test.tsx DashboardTab.a11y.test.tsx`
2. `npm run build`
3. 手工检查常用工作区宽度下筛选项保持单行、卡片内容居中、底部顺序正确。

### Scope assessment

该修改不是单纯一行样式修正：日期结束边界和回填存在容易出错的排他上界语义。但根因局部且调用契约稳定，预计产品代码仅需修改 `DashboardTab.tsx`，测试修改集中在 `DashboardTab.test.tsx`，无后端改动。

## Final Conclusion

**Status:** Concluded

**Overall confidence:** High

四项功能性根因（日期粒度、筛选换行、卡片顺序、指标名称）均由当前源码直接确认，调用链和后端日期边界也有 SQL 与测试证据支持。卡片右侧空白的产生机制已确认；用户已明确选择“左侧图标和名称、右侧数字”，视觉方案不再存在待决分支。

### Final fix direction

1. **日期筛选：** `datetime-local` 改为 `date`；开始日转本地当天零点 ISO，结束日转本地次日零点 ISO；结束值回填时减一个本地日。
2. **筛选布局：** 移除 `flex-wrap`，Agent 选择器允许收缩，日期控件保持单行。
3. **指标配置：** 顺序改为任务总数、记忆数量、待处理任务、对话数量；只改展示配置，不改数据字段。
4. **卡片留白：** 采用“左侧图标和名称、右侧数字”的结构化布局；左侧信息组可收缩，右侧数字 `shrink-0` 并保持醒目。
5. **修改边界：** 预计仅涉及 `DashboardTab.tsx` 与 `DashboardTab.test.tsx`；不得覆盖当前工作区中尚未提交的 Dashboard 大批改动。

### Acceptance criteria

- 日期控件不再允许选择小时和分钟。
- 用户选择同一天作为开始和结束日期时，后端收到一个完整自然日的合法半开区间。
- 结束日期当天的数据不会漏计，控件回填不会多显示一天。
- Agent、开始日期、结束日期在目标工作区宽度下保持一行。
- 指标网格为：左上任务总数、右上记忆数量、左下待处理任务、右下对话数量。
- 四张卡片不再表现为内容明显左聚集、右侧单边空白过大。
- Dashboard 定向前端测试通过，完整 TypeScript/Vite 构建通过。

### Verification plan

1. 更新单元测试，覆盖 `type=date`、开始/结束 ISO 转换、结束日期回填、指标名称与 DOM 顺序。
2. 运行 `npm run test:frontend -- DashboardTab.test.tsx DashboardTab.a11y.test.tsx`。
3. 运行 `npm run build`。
4. 在应用中检查常用工作区宽度，确认左侧名称不会挤压或截断右侧数字，四张卡片的数字列视觉对齐。

### Recommended next action

这是边界清晰、局部且已有测试入口的修改，推荐使用 `bmad-quick-dev` 在当前工作树上做外科手术式实现。实施前应保留现有未提交改动，禁止 checkout、reset 或以 HEAD 版本覆盖相关文件。

## Follow-up: 2026-07-24

### User decision

用户明确选择卡片内部采用“左侧图标和名称、右侧数字”的布局，不采用整体居中方案。

### Updated fix direction

卡片内部建议拆为两个直接子区域：

- 左侧信息组：`flex min-w-0 items-center gap-2`，包含图标和名称；名称允许截断。
- 右侧数值：`shrink-0`，保留现有字号和加载态，靠右展示。
- 卡片外层：`flex items-center justify-between gap-3`。

不增加新的共享组件或抽象；仅重排现有卡片 JSX。`aria-label` 继续使用指标名称，加载期间的省略号逻辑保持不变。

### Updated acceptance criteria

- 图标和指标名称位于卡片左侧，数字位于卡片右侧。
- 四张卡片的数字视觉上形成稳定的右侧对齐区域。
- 长名称不会把数字挤出卡片；名称可截断，数字不可收缩。
- 加载态、省略号、颜色和可访问名称保持现有行为。

### Updated conclusion

视觉方案已由用户裁决，不再缺少产品选择。产品代码预计仍仅修改 `DashboardTab.tsx`，测试仍集中在 `DashboardTab.test.tsx`；当前继续保持“不执行实现”状态。

## Follow-up: 2026-07-25 #2

### New symptoms

用户反馈当前实现仍有两个视觉问题：

1. 两个日期输入框过宽，角色选择器中的角色名称显示不完整。
2. 指标卡整体高度偏高，希望进一步压缩。

### Confirmed findings

#### 1. 日期控件占用固定的剩余空间，直接压缩角色选择器

- `DashboardTab.tsx:125` 的筛选容器使用 `flex`，三个控件和间隔处于同一行。
- `DashboardTab.tsx:129` 的角色选择器使用 `min-w-0 flex-1`，只能获得日期控件和间隔之后的剩余宽度。
- `DashboardTab.tsx:143` 与 `DashboardTab.tsx:151` 的日期控件使用 `shrink-0`，但没有显式 `width`，因此会按浏览器原生 `date` 控件的内容、内边距和日历按钮占用固有宽度。
- 日期控件变宽后，角色选择器会被 `flex-1` 压缩；这解释了角色选项显示不完整。该因果关系由当前布局约束直接推出，视觉上的具体像素值尚未在浏览器中测量。

**证据等级：Confirmed（布局约束）；Deduced（角色名称被截断的直接结果）。**

#### 2. 指标卡高度由自然内容高度和内边距共同决定

- `DashboardTab.tsx:169` 的卡片使用 `p-3`，上下各增加 12px。
- `DashboardTab.tsx:172-174` 的上层图标为 20px 高，名称与其同行。
- `DashboardTab.tsx:176` 的数字使用 `mt-2` 和 `text-[20px]`，在上层内容下方额外增加间距和较大的行高。
- `DashboardTab.tsx:163` 只设置了两列网格和间距，没有固定高度；因此卡片高度由上述自然内容高度决定。

**证据等级：Confirmed。**

### Deduced conclusions

- 日期问题不需要改变日期语义或后端契约，根因仅是筛选栏中日期输入缺少宽度约束。
- 保留当前“上层图标/名称、下层数字”的两层布局，仅压缩 padding、图标尺寸、数字字号和上下间距即可降低高度，不应改成单行卡片。

### Recommended fix direction（暂不执行）

#### 日期筛选

推荐给两个日期输入增加一致的显式宽度，并略减内边距，例如：

- 日期控件使用约 `128–132px` 的固定宽度；
- 保留 `shrink-0`，避免在窄布局下被压扁；
- 可将日期控件横向内边距从 `px-2.5` 调整为 `px-2`；
- 继续让角色选择器使用 `min-w-0 flex-1`，把释放出的空间交给角色选择器。

首选先使用 `w-[132px]`，因为它应能容纳 `YYYY-MM-DD`、日历按钮和边框；若实际窗口仍显得宽，再在最小窗口宽度下验证是否可降至 `128px`。不能仅依赖 `max-width`，因为原生日期控件的固有尺寸仍可能主导 flex 布局。

#### 指标卡高度

推荐保持现有结构，只做最小尺寸收紧：

- 卡片 `p-3` → `p-2.5`；
- 图标 `20px` → `18px`；
- 数字上边距 `mt-2` → `mt-1.5` 或 `mt-1`；
- 数字字号 `20px` → `18px`，仍保留醒目的层级。

这预计可在不改变信息层次、颜色、加载态和无障碍名称的前提下减少约一档垂直空间。具体最终值需要在 900px 最小窗口和常用 1200px 窗口各检查一次。

### Verification plan

1. 在最小窗口宽度和常用窗口宽度检查角色选择器的选中角色名称是否可读。
2. 确认两个日期控件仍能完整显示日期文本和日历按钮，且筛选项保持单行。
3. 检查指标卡仍保持 2×2 网格、两层布局和数字右对齐，同时高度明显降低。
4. 运行现有 `DashboardTab` 与无障碍测试，确认只发生视觉尺寸变化。

### Scope and status

预计仍只需修改 `egosync-app/src/components/butler/DashboardTab.tsx`；若补充样式意图测试，再修改 `DashboardTab.test.tsx`。不需要修改 Hook、服务层、Rust、IPC 或 SQL。调查结论已足以进入实现阶段，但本轮按用户要求不执行实现。

## Follow-up: 2026-07-25 #3

### New user direction

用户反馈将日期控件固定为 `132px` 后问题仍存在，并提出新的交互方案：

- 日期筛选改为一个下拉控件，选项为：全部日期、最近3天、最近7天、最近1个月、自定义时间；
- 选择自定义时间时，希望将起始日期和结束日期合并为一个日期范围控件。

### Updated confirmed findings

#### 原有固定宽度修复不足

当前代码已将两个日期输入固定为 `w-[132px]`，但筛选栏仍然同时保留两个 `shrink-0` 日期控件（`DashboardTab.tsx:139-153`）。这意味着主筛选行仍需为两个日期框、连接词和间距预留约 264px 以上的空间；角色选择器继续只能使用剩余的 `flex-1` 空间（`DashboardTab.tsx:126-137`）。

**证据等级：Confirmed。**

因此问题已经不只是“日期框没有宽度约束”，而是当前交互模型在主行中固定放置两个日期控件，本身就会持续挤压角色选择器。继续单纯缩小日期框只能缓解，不能从根本上消除布局竞争。

### Recommended interaction model

推荐改为一个“日期筛选组合控件”，主筛选行只保留一个日期控件：

1. 控件触发按钮显示当前选项，例如“全部日期”“最近7天”或“2026-07-01 至 2026-07-24”；
2. 点击后打开一个紧凑面板；
3. 面板上方或左侧列出五个选项：
   - 全部日期
   - 最近3天
   - 最近7天
   - 最近1个月
   - 自定义时间
4. 选择自定义时间后，在同一个面板中显示起始日期和结束日期，并通过“应用”确认。

这样主筛选行只有 Agent 选择器和一个日期筛选控件，角色名称可获得稳定的可用宽度。

### About merging two date inputs

**可以合并为一个视觉上的日期范围控件，但不能用一个原生 HTML `input type="date"` 同时表达起始和结束日期。** 当前项目依赖中没有日期范围选择器库（`egosync-app/package.json` 的依赖列表未包含 date-range picker）。

可行方案按推荐度排序：

1. **推荐：自定义组合控件**
   - 主行显示一个 `button`/只读摘要，例如“2026-07-01 至 2026-07-24”；
   - 点击后打开 popover；
   - popover 内仍使用两个原生日期输入，但它们只在面板中出现，并属于同一个日期范围控件；
   - “应用”时一次性更新 `timeRange`。
   - 不需要新增第三方依赖，保留原生日期选择和键盘可访问性。

2. **可选：引入第三方日期范围组件**
   - 交互最完整，可提供双日历和拖拽选区；
   - 需要新增依赖、处理主题样式和打包体积；
   - 对当前只有自然日范围的需求偏重，不建议作为第一版。

3. **不推荐：单个文本框手写 `开始 - 结束`**
   - 需要自行解析、校验、格式化和处理键盘/日历交互；
   - 容易退化为不可访问的伪日期控件。

### Preset semantics to freeze before implementation

日期只按自然日计算，继续沿用本地零点和后端 `[startAt, endAt)` 契约：

| 选项 | `startAt` | `endAt` |
|---|---|---|
| 全部日期 | `null` | `null` |
| 最近3天 | 本地今天往前 2 个日历日的零点 | 本地明天零点 |
| 最近7天 | 本地今天往前 6 个日历日的零点 | 本地明天零点 |
| 最近1个月 | 本地今天往前 1 个日历月的零点 | 本地明天零点 |
| 自定义时间 | 自定义开始日当地零点 | 自定义结束日次日当地零点 |

“最近1个月”目前存在一个产品语义选择：推荐解释为“从今天往前一个日历月到今天”，而不是固定 30×24 小时；这样与自然日筛选和夏令时处理保持一致。若用户希望当前自然月，则需要另行裁决。

### State and request implications

- 现有 Hook 的 `timeRange: { startAt, endAt }` 契约可以保留，不需要修改 Rust、IPC、SQL 或服务层。
- 预设选项可以在 `DashboardTab` 内维护当前 preset，并将计算后的边界传给现有 `setTimeRange`。
- 自定义日期建议维护 draft 起止日期，只有两端都有效且开始日不晚于结束日时，点击“应用”才调用一次 `setTimeRange`。
- 不能在用户先选开始日、再选结束日时分别立即更新请求，否则会触发两次指标请求，并在中间状态展示不完整范围。
- 切换到任何预设时应清除自定义 draft 或将其保留为下次编辑值，但实际请求必须立即使用预设边界。

### Test and verification plan

1. 验证下拉选项文案和默认值为“全部日期”。
2. 验证五个选项分别生成正确的本地自然日半开区间。
3. 验证夏令时跨日仍使用日历运算，而不是固定加减 24 小时。
4. 验证自定义范围在未完成或开始日晚于结束日时不能应用。
5. 验证自定义范围应用时只触发一次指标请求，且结束边界为结束日次日零点。
6. 验证主筛选行不再同时出现两个日期输入框，角色选择器获得完整可用宽度。
7. 验证 popover 的关闭、取消、键盘焦点和暗色模式。

### Conclusion and status

**Confidence: High** for the layout diagnosis and the feasibility of a combined visual range control; **Medium** for the exact “最近1个月”产品语义，需按上述推荐解释冻结。推荐进入实现前先确认“最近1个月”是否指滚动一个日历月，并采用不新增依赖的自定义组合控件方案。本轮仍未修改产品代码。

