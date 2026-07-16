---
title: '委派任务拆分确认'
type: 'feature'
created: '2026-07-16'
status: 'done'
baseline_commit: 'a67eef97f55e62a0f85283741ad97069b5a183ef'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/uat-case-5-unconfirmed-task-decomposition-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 被委派角色把一个事项拆成多个任务时会立即创建，用户无法确认拆分或选择保持单任务。

**Approach:** 单个任务继续直接创建；两个及以上任务改为持久化拆分提案，在管家会话展示整批选择：“接受拆分”后创建全部子任务，“不要拆分”后只创建一个原始总任务。

## Boundaries & Constraints

**Always:** 多任务时 tasks 表零新增；提案持久化并绑定来源管家会话和目标角色；每批只能处理一次；接受拆分使用提案子项，不拆分使用原始 `task_summary`；创建成功后才更新终态；重启后 pending 仍可处理；单任务保持即时创建。

**Ask First:** 若必须改变 `delegate_to_role` 公开参数或通用 suggestion/ActionCard 语义，暂停确认。

**Never:** 不只靠 prompt；不支持逐项编辑/接受或全部取消；不自动过期；模型不能提供 role_id；提案不能误报为已创建；不修 deadline 年份问题。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 单任务 | 角色返回 1 个合法调用 | 立即创建并沿用现有权威结果 | 沿用现有失败语义 |
| 多任务 | 2 个以上合法调用 | 保存 pending 批次，零任务写入，显示两项选择 | 不足 2 项退化为单任务 |
| 接受拆分 | pending 点击“接受拆分” | 原子创建全部子任务并标记 accepted | 失败整批回滚 |
| 不要拆分 | pending 批次点击“不要拆分” | 创建 1 个原始总任务并标记 kept_single | 创建失败不改变 pending |
| 重复/恢复 | 重复提交或重启返回 | 不重复创建；来源会话恢复卡片 | 其他会话不显示 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 单/多调用分流与提案生成。
- `egosync-app/src-tauri/{migrations,src/models,src/db,src/commands}` -- 持久化、事务和 IPC。
- `egosync-app/src/{types,services,hooks,components}` -- 会话卡片与操作状态。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/{migrations,src/models,src/db}` -- 新增提案批次和事务化处理。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 多调用生成提案，单调用保持原路径，传递来源会话。
- [x] `egosync-app/src-tauri/src/{commands,lib.rs}` -- 查询、接受拆分、保持单任务 IPC。
- [x] `egosync-app/src/{types,services,hooks}` -- 封装加载与幂等操作。
- [x] `egosync-app/src/components/` -- 展示批次明细与两个选择，处理 loading/error。
- [x] 后端与前端测试 -- 覆盖矩阵全部场景及原有单任务回归。

**Acceptance Criteria:**
- Given 委派角色返回多个任务调用，when 首轮执行结束，then 只产生一个 pending 提案且 tasks 表不新增。
- Given pending 提案，when 用户接受拆分，then 全部子任务一次性创建、归属目标角色且不会重复。
- Given pending 提案，when 用户选择不要拆分，then 只创建一个以原始 task_summary 命名的任务。
- Given 应用重启或切换会话，when 返回来源管家会话，then pending 提案卡片仍可继续处理。
- Given 委派角色只返回一个任务，when 执行完成，then 行为与当前即时创建路径一致。

## Spec Change Log

## Design Notes

使用独立 `task_decomposition_proposals`，避免扭曲 suggestion 的单建议语义。批次保存 role、来源会话、原始标题和规范化子项；确认命令在单事务内创建任务并更新状态，分类在提交后触发。前端仅提供整批两选项。

## Verification

**Commands:**
- `cd egosync-app/src-tauri && cargo test task_decomposition` -- expected: 后端提案与事务测试通过。
- `cd egosync-app/src-tauri && cargo test delegated_provider` -- expected: 单任务与多任务委派回归通过。
- `cd egosync-app && npx vitest run src/components/butler/TaskDecompositionCard.test.tsx` -- expected: 卡片交互测试通过。
- `cd egosync-app/src-tauri && cargo check` -- expected: 编译通过，无新增 error。
- `git diff --check` -- expected: 无空白错误。

## Suggested Review Order

**委派分流与来源绑定**

- 多调用在写任务前转为持久化提案。
  [`agent_engine.rs:4093`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L4093)

- 本地桥把真实管家会话传入提案链路。
  [`delegate_bridge.rs:341`](../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L341)

**持久化与原子确认**

- 独立表保存批次、子项和终态。
  [`027_task_decomposition_proposals.sql:1`](../../egosync-app/src-tauri/migrations/027_task_decomposition_proposals.sql#L1)

- 接受拆分在单事务创建全部任务。
  [`task_decomposition.rs:156`](../../egosync-app/src-tauri/src/db/task_decomposition.rs#L156)

- 保持单任务使用原始委派摘要。
  [`task_decomposition.rs:226`](../../egosync-app/src-tauri/src/db/task_decomposition.rs#L226)

**用户确认界面**

- 卡片仅提供整批接受或保持单任务。
  [`TaskDecompositionCard.tsx:12`](../../egosync-app/src/components/butler/TaskDecompositionCard.tsx#L12)

- 会话切换、恢复和操作状态由专用 Hook 管理。
  [`useTaskDecompositions.ts:8`](../../egosync-app/src/hooks/useTaskDecompositions.ts#L8)

- 流结束后刷新当前会话的待确认提案。
  [`ButlerView.tsx:55`](../../egosync-app/src/components/butler/ButlerView.tsx#L55)
