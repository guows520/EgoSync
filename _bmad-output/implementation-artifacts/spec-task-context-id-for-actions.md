---
title: '修复自然语言任务操作缺少任务 ID'
type: 'bugfix'
created: '2026-07-15'
status: 'done'
baseline_commit: 'dc6bd839b0d209211e41c6cae25d742509751305'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/task-completion-missing-id-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 管家和角色的任务摘要没有携带任务 UUID，但 `complete_task` 与 `delete_task` 强制要求从该摘要取得 `task_id`，导致用户说“季度汇报已经提交了”时无法自动完成既有任务，并可能错误委派给角色。

**Approach:** 在共享任务上下文行中输出权威任务 ID，并收紧任务完成/删除的意图路由规则；用单元测试固定管家与角色上下文均可寻址任务，同时保持现有筛选、截断和角色隔离行为。

## Boundaries & Constraints

**Always:** 继续以数据库 UUID 作为完成/删除任务的唯一权威标识；管家摘要和角色摘要必须通过同一个格式化路径获得 ID；测试必须说明 ID 对自然语言任务操作的重要性；沿用现有 Rust 风格和测试结构。

**Ask First:** 若实现需要改变工具参数 schema、数据库结构、任务标题匹配规则，或引入新的任务查询接口，必须先征得用户同意。

**Never:** 不按标题模糊匹配任务；不自动处理同名任务歧义；不重构无关的 Agent prompt、任务 CRUD 或数据库代码；不向最终用户展示内部 UUID。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 完成既有任务 | 未完成任务“明天要交的季度汇报”存在 | 注入的任务行包含标题和 UUID，模型可调用 `complete_task` | 后端错误按现有权威结果返回，不改写为“没有接口” |
| 删除既有任务 | 未完成任务存在 | 同一任务行中的 UUID 可供 `delete_task` 使用 | 保持现有删除错误处理 |
| 角色隔离 | 两个角色各有任务 | 当前角色摘要只包含本角色任务及其 UUID | 不泄露其他角色任务或 ID |
| 已完成任务截断 | 已完成任务超过上下文上限 | 保持现有截断规则；所有被选中任务均包含 UUID | 被省略任务仍只显示汇总数量 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 任务上下文格式化、管家/角色摘要、动态行为指南及相关测试。
- `egosync-app/src-tauri/src/services/agent_config.rs` -- opencode 静态管家提示中的任务工具边界，需与动态提示保持一致。
- `_bmad-output/uat/UAT-Simplified-Manual.md` -- 用例 4 阶段 E/F 的人工验收依据；不修改。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 在共享任务行中加入 `id=<UUID>`，并明确完成/删除已有任务不得调用 `delegate_to_role`，修复工具参数来源和错误路由。
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 同步静态管家工具边界文案，避免新旧会话的路由规则不一致。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 保存测试创建返回值并断言管家、角色摘要包含正确 UUID且不泄露其他角色 UUID，固定可寻址性契约。

**Acceptance Criteria:**
- Given 管家上下文中存在未完成任务，when 构建 `[各角色任务]`，then 每个被注入任务同时包含标题和对应 UUID。
- Given 当前角色及其他角色都有任务，when 构建 `[当前角色任务]`，then 只包含当前角色任务的标题与 UUID。
- Given任务完成或删除语义命中已有任务，when 模型读取行为指南，then 指令要求调用对应任务工具而非委派角色。
- Given 已完成任务数量超过上限，when 构建摘要，then 现有截断与汇总数量行为保持不变。

## Spec Change Log

## Verification

**Commands:**
- `cargo test test_butler_task_summary_keeps_unfinished_and_caps_completed_tasks` -- 管家摘要可寻址且截断规则通过。
- `cargo test test_role_messages_include_role_scoped_tasks` -- 角色摘要包含自身任务 UUID且隔离其他角色。
- `cargo test test_role_task_summary_includes_all_unfinished_even_above_visible_limit` -- 超限未完成任务及已完成任务省略规则通过。
- `cargo check` -- Rust 后端编译通过。

**Manual checks:**
- 完整安装包中执行 UAT 用例 4 阶段 E/F，确认完成和删除均调用对应任务工具，UI 状态正确。

## Suggested Review Order

**权威任务寻址**

- 共享格式化入口把数据库 UUID 与任务标题绑定在同一行。
  [`agent_engine.rs:1079`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1079)

- 角色与管家摘要限制 UUID 仅供工具参数使用。
  [`agent_engine.rs:1138`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1138)

**意图路由**

- 系统提示将“已提交”等状态陈述明确识别为完成操作。
  [`agent_engine.rs:1403`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1403)

- opencode 静态提示保持相同的任务工具边界。
  [`agent_config.rs:471`](../../egosync-app/src-tauri/src/services/agent_config.rs#L471)

**回归测试**

- 角色摘要验证标题与 UUID 同行且不泄露其他角色 ID。
  [`agent_engine.rs:5655`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L5655)

- 管家摘要验证可寻址性与已完成任务截断边界。
  [`agent_engine.rs:5732`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L5732)

- 静态提示测试固定完成/删除不得委派的契约。
  [`agent_config.rs:1185`](../../egosync-app/src-tauri/src/services/agent_config.rs#L1185)
