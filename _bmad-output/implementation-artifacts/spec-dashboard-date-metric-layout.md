---
title: '优化仪表盘日期筛选与指标卡布局'
type: 'bugfix'
created: '2026-07-24'
baseline_commit: '81885e9ceabc95a3a7ef117f78220ef0b776d6f1'
status: 'done'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/11-1-view-and-filter-cross-agent-activity-statistics.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/dashboard-metric-layout-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 仪表盘时间筛选暴露了不必要的小时/分钟输入且可能换行；四项指标的文案、位置和卡内空间利用也不符合目标，当前内容集中在左侧形成明显空白。

**Approach:** 将时间范围收敛为自然日选择并在前端映射到现有半开 ISO 区间；筛选保持单行；调整指标配置顺序与文案，并采用“左上图标和名称、右下数字”的两层卡片布局。

## Boundaries & Constraints

**Always:** 在当前未提交工作树上做最小增量修改；保留现有 Dashboard 加载态、错误态、暗色模式、颜色与无障碍语义；结束日期使用排他上界的次日当地零点；日期跨日使用日历运算以兼容夏令时。

**Ask First:** 若实现必须修改 `DashboardTab.tsx`、`DashboardTab.test.tsx` 之外的产品文件，或必须改变 Rust/IPC/SQL 时间契约，先暂停确认。

**Never:** 不 checkout/reset/覆盖用户未提交修改；不重构 Dashboard hook/service；不新增共享组件；不把结束日表示为 `23:59:59.999`；不恢复自动换行；不把卡片压成单行细长布局。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 开始日期 | 选择 `2026-07-24` | `startAt` 为本地 7 月 24 日零点对应 ISO | 清空时发送 `null` |
| 结束日期 | 选择 `2026-07-24` | `endAt` 为本地 7 月 25 日零点对应 ISO，控件仍显示 7 月 24 日 | 清空时发送 `null` |
| 同日范围 | 开始、结束均为同一天 | 后端收到合法的一整天半开区间 | 沿用现有请求错误展示 |
| 长指标名称 | 卡片可用宽度受限 | 左侧名称可截断，右下数字不可收缩或溢出 | 保持可访问名称完整 |

</frozen-after-approval>

## Code Map

- `egosync-app/src/components/butler/DashboardTab.tsx` -- 日期输入、筛选布局、指标配置及卡片 JSX 的唯一产品修改点。
- `egosync-app/src/components/butler/DashboardTab.test.tsx` -- 日期转换、回填、指标文案与 DOM 顺序的行为测试。
- `egosync-app/src/components/butler/DashboardTab.a11y.test.tsx` -- 现有可访问性回归验证，不预期修改。
- `egosync-app/src-tauri/src/services/dashboard_service.rs` -- 已有 `[startAt,endAt)` 契约证据，不修改。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src/components/butler/DashboardTab.tsx` -- 将控件改为日期输入，正确转换开始/排他结束日期并回填；保持筛选单行；更新指标文案、顺序和两层卡片布局。
- [x] `egosync-app/src/components/butler/DashboardTab.test.tsx` -- 补充日期类型、ISO 边界、结束日回填、指标名称和顺序测试，保护用户意图。

**Acceptance Criteria:**
- Given 用户打开仪表盘, when 查看筛选区, then Agent、开始日期、结束日期保持同一行且不显示小时或分钟。
- Given 用户选择自然日范围, when 指标请求发出, then 开始边界为开始日当地零点、结束边界为结束日次日当地零点。
- Given 指标区渲染成功, when 用户从左到右、从上到下查看, then 顺序为任务总数、记忆数量、待处理任务、对话数量。
- Given 任一指标卡, when 渲染标签和数值, then 图标与名称位于左上区域、数字位于右下区域，数字不会被名称挤压。
- Given 原有加载、错误或暗色状态, when 本次修改生效, then 既有行为保持不变。

## Spec Change Log

- 2026-07-24：按批准方案完成日期筛选、指标文案/顺序和两层卡片布局。
- 2026-07-24：审查后补充日期清空测试，并加固低年份与无效自然日转换。
- 2026-07-24：其余审查项属于既存 Story 11.1 或已被最新用户裁决覆盖，未扩大本次修改范围。

## Design Notes

卡片保持 2×2 网格。内部使用上层图标/名称、下层右对齐数值，不设置固定高度；若自然内容高度不足，仅在实际视觉验证后考虑最小高度。筛选区优先保证单行，Agent 选择器可收缩，日期控件不可收缩。

## Verification

**Commands:**
- `npm run test:frontend -- DashboardTab.test.tsx DashboardTab.a11y.test.tsx` -- 2 个测试文件、18 项测试全部通过。
- `npm run build` -- TypeScript 检查和 Vite 构建成功。

**Manual checks:**
- 在常用工作区宽度检查筛选单行、2×2 顺序、卡片高度与右下数字对齐。

## Suggested Review Order

**日期边界与筛选绑定**

- 自然日转换保留本地零点，并用日历运算生成排他结束边界。
  [`DashboardTab.tsx:54`](../../egosync-app/src/components/butler/DashboardTab.tsx#L54)

- 三个筛选项保持单行，日期输入不再暴露小时和分钟。
  [`DashboardTab.tsx:125`](../../egosync-app/src/components/butler/DashboardTab.tsx#L125)

**指标语义与视觉布局**

- 配置顺序直接对应 2×2 网格的目标阅读顺序。
  [`DashboardTab.tsx:103`](../../egosync-app/src/components/butler/DashboardTab.tsx#L103)

- 两层结构平衡卡片留白，同时保持名称可截断、数字右对齐。
  [`DashboardTab.tsx:163`](../../egosync-app/src/components/butler/DashboardTab.tsx#L163)

**意图回归测试**

- 验证当地零点、次日排他边界与结束日期回填。
  [`DashboardTab.test.tsx:283`](../../egosync-app/src/components/butler/DashboardTab.test.tsx#L283)

- 验证清空任一日期发送 null 且保留另一侧边界。
  [`DashboardTab.test.tsx:307`](../../egosync-app/src/components/butler/DashboardTab.test.tsx#L307)

- 验证四项指标的最终文案和 DOM 顺序。
  [`DashboardTab.test.tsx:343`](../../egosync-app/src/components/butler/DashboardTab.test.tsx#L343)
