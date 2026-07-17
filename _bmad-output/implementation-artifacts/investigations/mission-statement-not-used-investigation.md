# 调查：管家未依据已设置的使命宣言回答价值观问题

## Hand-off Brief

1. **发生了什么。** 用户报告已设置使命宣言后，管家回答“最看重什么”时仍仅依据日程与任务行为推断。
2. **当前状态。** 根因已确认：使命可持久化，但 legacy 与 opencode 两条管家提示词链路均未读取或注入使命。
3. **下一步。** 在两条动态上下文链路统一注入格式化使命，并用明确优先级规则与意图测试锁定行为。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | 用例 8 |
| Date opened | 2026-07-17 |
| Status | Concluded |
| System | EgoSync；Windows；当前工作区源码 |
| Evidence sources | 用户提供的对话记录、源码、测试、版本控制 |

## Problem Statement

用户报告：设置使命宣言后，管家回答“你觉得我这个人最看重什么”时，没有基于明确设置的使命宣言回复，而是继续根据行为数据推断隐含价值观。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户提供的对话记录 | Available | 回复引用家长会、会议、健身等行为，没有引用使命宣言 |
| 源码 | Available | 已追踪使命 DB、管家 system prompt 与 opencode dynamic prompt |
| 自动化测试 | Partial | 已确认没有“已设置使命进入管家上下文”的测试 |
| Story/UAT 文档 | Available | Story 5.2 仅实现手动推断 API；UAT 额外要求对话中使命优先 |
| 运行时日志/数据库快照 | Missing | 尚未提供，可能用于确认实际持久化和请求载荷 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 使命宣言的存储与读取 | High | Done | 单例 upsert/get 完整，结构化内容为 JSON |
| 2 | 管家系统提示词/上下文组装 | High | Done | 两条运行链路均遗漏 mission |
| 3 | 用例 8 的测试实现与断言 | High | Done | UAT 有要求，代码测试无对应断言 |
| 4 | 相关提交与回归线索 | Medium | Done | Story 5.2 实现边界未包含对话集成，属于验收遗漏而非近期回归 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-17 | 用户报告已设置使命后仍得到行为推断回答 | 用户输入 | Confirmed |

## Confirmed Findings

### Finding 1: 实际回复采用行为证据叙述

**Evidence:** 用户提供的对话记录

**Detail:** 回复明确引用产品会议、家长会、季度汇报、健身与读书等安排，并据此推导“家”最重要；提供的文本中没有使命宣言内容。

### Finding 2: 使命持久化链路可保存并回读

**Evidence:** `egosync-app/src-tauri/src/db/mission.rs:6`, `egosync-app/src-tauri/src/db/mission.rs:13`, `egosync-app/src/components/butler/ButlerSettingsContent.tsx:443`

**Detail:** 自由文本直接保存；结构化使命序列化为包含 mission、principle、roles 的 JSON；后端写入 singleton 记录并回读。

### Finding 3: 两条管家提示词链路均未读取使命

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:1324`, `egosync-app/src-tauri/src/services/agent_engine.rs:1462`

**Detail:** legacy 的 build_butler_system_prompt 和 opencode 的 build_butler_dynamic_prompt 都只注入角色、任务、记忆、跨角色摘要及行为指南；源码中 get_mission 仅被 mission command 与数据导出调用。

### Finding 4: UAT 要求超出 Story 5.2 实现边界

**Evidence:** `_bmad-output/implementation-artifacts/5-2-behavior-inferred-values.md:82`, `_bmad-output/implementation-artifacts/5-2-behavior-inferred-values.md:92`, `_bmad-output/uat/UAT-Simplified-Manual.md:559`

**Detail:** Story 5.2 实现手动“推断使命宣言”API/弹窗，并允许已设使命时重新推断；UAT 则要求普通管家对话在已设使命后改用明确使命。后者没有对应实现任务或自动化测试。

## Deduced Conclusions

### Deduction 1: 当前回复是上下文缺失的确定性结果

**Based on:** Finding 1、Finding 2、Finding 3

**Reasoning:** 使命已经有独立持久化来源，但对话请求不读取它；任务与记忆会进入提示词，因此模型可见的唯一价值观证据是行为数据。

**Conclusion:** 根因不在使命保存，也不是模型随机忽略；是管家上下文组装遗漏了明确使命及其优先级规则。

## Hypothesized Paths

### Hypothesis 1: 已设置的使命宣言未进入本次管家提示词上下文

**Status:** Confirmed

**Theory:** 使命可能已保存，但上下文组装遗漏、读取失败或被条件分支过滤，导致模型只能使用行为记忆。

**Supporting indicators:** 回复只表现出行为数据可见。

**Would confirm:** 请求上下文构造代码未读取使命字段，或运行时请求载荷中没有使命内容。

**Would refute:** 请求载荷明确包含使命全文及强优先级指令。

**Resolution:** 两个管家 prompt builder 的源码均没有 mission DB 调用；全仓源码调用点也证明 mission 未进入对话链路。

### Hypothesis 2: 使命已经注入，但被行为数据覆盖

**Status:** Refuted

**Theory:** 模型同时看到了使命与行为，但错误选择了行为。

**Supporting indicators:** 表面症状与“提示词优先级不足”相似。

**Would confirm:** 最终 prompt 含使命全文。

**Would refute:** prompt builder 没有读取使命。

**Resolution:** 源码显示两条 prompt builder 均没有读取使命，因此本次不是“收到后忽略”。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 本次 UAT 数据库中的具体使命记录 | 仅影响单次环境是否保存成功，不改变已确认的代码缺口 | 若实施后仍失败，再检查 DB |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `agent_engine.rs:1324` 与 `agent_engine.rs:1462` |
| Trigger | 用户询问“你觉得我这个人最看重什么” |
| Condition | 已设置使命宣言，但管家上下文只加载行为资料 |
| Related files | `db/mission.rs`、`commands/mission.rs`、`ButlerSettingsContent.tsx`、Story 5.2、UAT 手册 |

## Conclusion

**Confidence:** High

根因已确认：使命存储是独立且可回读的，但普通管家对话的 legacy 与 opencode 两条上下文构造均未加载该数据，也没有“明确使命优先于行为推断”的规则。UAT 步骤 10 属于未被 Story 5.2 实现和自动化测试覆盖的集成要求。

## Recommended Next Steps

### Fix direction

推荐在 agent_engine 中增加一个统一的使命上下文构造器：读取 singleton mission；空内容不输出；free 直接输出；structured 解析并渲染 mission/principle/roles，避免把原始 JSON 交给模型。将该上下文同时注入 build_butler_system_prompt 与 build_butler_dynamic_prompt，并明确规则：身份、价值观、长期优先级问题以用户明确使命为第一依据，行为只作佐证；两者冲突时指出冲突，不得静默用行为覆盖。

### Diagnostic

无需先加诊断日志，静态链路已经足以确认根因。修复后若真实 UAT 仍失败，再检查现有 system prompt 预览日志与实际 DB 内容。

## Reproduction Plan

1. 无使命：准备足够行为数据，询问价值观，期望使用行为并明确是推断。
2. free 使命：设置与近期行为可能冲突的使命，重复询问，期望使命优先、行为仅佐证或提示冲突。
3. structured 使命：设置 mission/principle/roles，重复询问，期望自然引用结构化内容而非 JSON。
4. 分别走 legacy 降级链路与 opencode 主链路，确保两者一致。

## Side Findings

- `mission_inferrer` 的 prompt 固定声称“用户尚未设定使命”，但手动触发模式又明确允许已设使命时调用；这不会导致本次普通对话问题，但文案语义冲突，后续应单独清理，避免重新推断时给模型错误前提。
