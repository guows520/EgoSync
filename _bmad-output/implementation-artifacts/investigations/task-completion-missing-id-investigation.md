# 调查：自然语言完成任务时提示缺少任务 ID

## Hand-off Brief

1. **发生了什么。** UAT 用例 4 阶段 E 中，用户说“季度汇报已经提交了”后，系统没有自动完成既有任务，而是提示缺少任务 ID。
2. **当前状态。** 根因已确认：任务操作工具强制要求使用任务摘要中的 `task_id`，但摘要格式化函数没有输出 ID；后端接口本身存在且可用。
3. **下一步。** 在任务上下文行中加入稳定 ID，并补充验证“上下文可寻址”的测试，再执行 UAT 用例 4 阶段 E/F。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-15 |
| Status | Concluded |
| System | Windows；EgoSync；UAT Simplified Manual 用例 4 阶段 E |
| Evidence sources | UAT 用例、用户提供的对话记录、源代码、版本控制 |

## Problem Statement

用户报告：自然语言“季度汇报已经提交了”应触发对既有任务的自动完成，但实际委派角色称没有更新任务状态接口，管家又称缺少任务 ID，最终未完成任务。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| `_bmad-output/uat/UAT-Simplified-Manual.md` | Available | 用例 4 阶段 E 的验收预期 |
| 用户提供的对话记录 | Available | 观察到角色与管家的实际输出 |
| 源代码 | Available | 待用 CodeGraph 追踪 |
| 运行日志与数据库快照 | Missing | 当前尚未提供，若静态证据不足则需要 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 核对 UAT 阶段 E 前置步骤和预期 | High | Done | UAT 明确要求自动标记完成 |
| 2 | 追踪任务查询/完成工具的注册和暴露 | High | Done | 三个任务工具均在当前代码中注册；完成接口存在 |
| 3 | 追踪管家汇总时任务 ID 的来源 | High | Done | `format_task_context_line` 查询到 Task 后丢弃了 `task.id` |
| 4 | 检查相关提交与测试覆盖 | Medium | Done | 摘要先于操作工具存在；新增工具未同步更新摘要契约，测试只覆盖标题/数量 |

## Hypothesized Paths

### Hypothesis 1: 任务 ID 未被提供给自然语言任务完成链路

**Status:** Confirmed

**Theory:** 任务存在，但委派角色缺少查询/更新能力，管家也没有从权威任务列表解析出 ID，导致系统只能要求用户提供内部 ID。

**Supporting indicators:** 对话中角色明确称仅有创建工具，管家明确称缺少 ID。

**Would confirm:** 工具注册或提示词显示委派角色无查询/更新工具，且管家完成路径要求显式 task_id 而未先查询。

**Would refute:** 运行记录显示查询及更新工具均已暴露并被成功调用，只是运行时数据异常。

**Resolution:** `agent_config.rs:196` 明确要求 ID 来自 `[各角色任务]`；`agent_engine.rs:1079-1099` 的格式化输出没有 ID。两者直接构成确定性矛盾。

## Confirmed Findings

### Finding 1: UAT 明确要求自然语言完成既有任务

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:274-277,317-319`

**Detail:** 输入“季度汇报已经提交了”后，管家应确认标记完成，产品经理任务列表应显示已完成。

### Finding 2: 完成工具和后端完成接口均已实现

**Evidence:** `egosync-app/src-tauri/src/services/agent_config.rs:191-228`；`egosync-app/src-tauri/src/services/delegate_bridge.rs:190-217,481-497`

**Detail:** `complete_task` 工具调用 `/complete-task`；后端以 `task_id` 调用 `set_task_completion(..., true)`。

### Finding 3: 工具契约要求 ID 来自任务摘要

**Evidence:** `egosync-app/src-tauri/src/services/agent_config.rs:194-200`；`egosync-app/src-tauri/src/services/agent_engine.rs:1400-1405`

**Detail:** 工具参数说明与行为指南都声明 `task_id` 必须取自 `[各角色任务]`。

### Finding 4: 任务摘要没有输出 ID

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:1079-1100`

**Detail:** `format_task_context_line` 只输出状态、象限、大石头/保护标记、截止时间和标题，未使用 `task.id`。管家摘要与角色摘要都复用该函数。

### Finding 5: 当前测试遗漏“可寻址性”意图

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:5721-5879`

**Detail:** 测试验证未完成任务保留、已完成任务截断、角色隔离和标题存在，但不验证任务 ID 被注入。

## Deduced Conclusions

### Deduction 1: 失败发生在工具调用之前

**Based on:** Finding 2、3、4。

**Reasoning:** 后端只接受 ID；模型被要求从摘要取 ID；摘要没有 ID。因此模型无法构造合法调用，不能到达后端完成逻辑。

**Conclusion:** “缺少任务 ID”不是数据库没有 ID，也不是完成接口不存在，而是上下文序列化层丢失了已查询到的 ID。

### Deduction 2: 错误委派是主缺陷的次生表现

**Based on:** Finding 2、4 与用户对话记录。

**Reasoning:** 行为指南要求任务完成直接调用 `complete_task`，实际却调用 `delegate_to_role`；在缺少必需 ID 时，模型选择了仍可调用的错误路径。角色关于“只有创建工具”的自然语言陈述不能证明运行时工具清单。

**Conclusion:** 应先修复上下文契约；同时用提示词/测试收紧“完成、删除既有任务不得委派”的路由边界。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 本次会话的原始 opencode 工具清单 | 可确认角色所称“只有创建工具”是否为幻觉或旧构建 | 检查 opencode 会话与启动日志；不影响已确认主根因 |
| 运行安装包版本/commit | 可判断是否还叠加了旧工具目录或未重启问题 | 记录 UAT 构建 commit 与启动日志 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `agent_engine.rs:1079-1099`，任务上下文格式化遗漏 `task.id` |
| Trigger | 用户输入“季度汇报已经提交了” |
| Condition | 已存在语义匹配任务，但注入给模型的任务行不含 ID，而工具参数强制要求 ID |
| Related files | `agent_config.rs`、`agent_engine.rs`、`delegate_bridge.rs`、UAT 手册 |

## Conclusion

**Confidence:** High

根因已确认：新加入的 `complete_task`/`delete_task` 工具依赖任务摘要提供 ID，但既有 `format_task_context_line` 未包含 ID，形成确定性的跨模块契约断裂。后端完成接口和数据库更新能力均存在；对话中的“没有更新接口”是模型在缺少可用参数后的错误解释，不是代码事实。尚未确认是否还叠加旧构建/工具同步问题，但它不影响主根因成立。

## Recommended Next Steps

### Fix direction

1. **推荐的最小修复：** 在 `format_task_context_line` 输出每条任务的 `id`，让管家和角色共享的任务摘要满足现有工具契约。无需新增查询接口或语义匹配服务。
2. **路由加固：** 明确“已提交/做完/完成”等命中现有未完成任务时调用 `complete_task`，不得委派；工具失败时只转述权威错误，不编造接口能力。
3. **不推荐作为首选：** 让工具接受标题并在后端模糊匹配。它会引入同名任务、近似匹配和误完成风险，复杂度明显高于传递已有 UUID。

### Diagnostic

新增测试应保存创建返回的任务对象，并断言管家摘要和角色摘要都包含该任务 UUID；测试注释说明 WHY：自然语言完成/删除操作必须能从上下文取得权威 ID。另增加工具契约测试，防止 schema 声称 ID 可用但摘要再次移除。

## Reproduction Plan

1. 单元测试：任务摘要包含标题、状态和对应 UUID；角色隔离及截断规则保持不变。
2. 集成验证：给定已知 `task_id` 调用 `/complete-task`，确认状态和 `completed_at` 更新。
3. UAT：按用例 4 完成 A-D，输入“季度汇报已经提交了”，确认调用 `complete_task` 而非 `delegate_to_role`，随后 UI 显示已完成。
4. 回归阶段 F：删除任务使用同一 ID 注入机制，确认 `delete_task` 正常。

## Side Findings

- `git blame` 显示任务摘要格式化来自提交 `c39c9681`（2026-06-17），任务完成工具来自 `e953b9e`（2026-07-15）；集成时没有同步更新旧摘要格式。
