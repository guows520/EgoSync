# Investigation: 第一轮对话就建议创建角色

## Hand-off Brief

1. **What happened.** 用户在管家对话第 1 轮提到健身话题时，管家立即建议创建角色，违反了 UAT 用例1 阶段B 预期"前 1-2 轮不急于建议创建角色"。
2. **Where the case stands.** 根因已 Confirmed：管家 system prompt 中 `[行为指南]` 和 `[角色涌现行为]` 两个段落存在指令冲突，`[行为指南]` 的"追问用户由谁处理"指令导致 LLM 在第一轮就提及创建角色。
3. **What's needed next.** 修改 prompt 文案消除冲突，需要用户确认修改方案。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A                                                                        |
| Date opened      | 2026-07-07                                                                 |
| Status           | Active                                                                     |
| System           | Windows, EgoSync Tauri 桌面应用                                             |
| Evidence sources | 源码 agent_engine.rs, UAT 测试文档, 对话记录                                |

## Problem Statement

用户在管家对话中发送第 1 条消息"我想开始健身，但不知道怎么制定训练计划"后，管家立即回复建议创建健身管家角色。根据 UAT 用例1 阶段B 预期，前 1-2 轮管家应正常回答健身问题，不急于建议创建角色，第 3 轮后才自然提议。

## Evidence Inventory

| Source           | Status    | Notes                                                              |
| ---------------- | --------- | ------------------------------------------------------------------ |
| UAT 测试文档     | Available | `_bmad-output/uat/UAT-Simplified-Manual.md` 用例1 阶段B 预期结果    |
| 对话记录         | Available | 用户提供的第 1 轮对话内容                                          |
| agent_engine.rs  | Available | 管家 system prompt 构建逻辑，含 `[行为指南]` 和 `[角色涌现行为]`   |
| Story 2.5 文档   | Available | `_bmad-output/implementation-artifacts/2-5-role-emergence-suggestion.md` |

## Investigation Backlog

| # | Path to Explore                         | Priority | Status | Notes                        |
| - | --------------------------------------- | -------- | ------ | ---------------------------- |
| 1 | 分析 prompt 中两段指令的冲突             | High     | Done   | 根因已定位                   |
| 2 | 确认 opencode 路径是否使用相同 prompt    | Medium   | Done   | 两条路径共用 build_butler_system_prompt |

## Timeline of Events

| Time        | Event                                         | Source           | Confidence |
| ----------- | --------------------------------------------- | ---------------- | ---------- |
| 2026-07-07  | 用户发送"我想开始健身"                         | 对话记录         | Confirmed  |
| 2026-07-07  | 管家第 1 轮即建议创建角色                      | 对话记录         | Confirmed  |

## Confirmed Findings

### Finding 1: 管家 system prompt 包含两段存在冲突的指令

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:1430-1441`（`[行为指南]`）和 `egosync-app/src-tauri/src/services/agent_engine.rs:1460-1467`（`[角色涌现行为]`）

**Detail:**

`[行为指南]` 第 1439 行：
> "意图模糊或没有合适角色时：不要调用工具，用一句话主动追问用户希望由谁来处理。"

`[角色涌现行为]` 第 1463 行：
> "不要在第一轮就建议，至少观察到用户 2-3 次提及同一领域后再提议。"

当用户说"我想开始健身"且没有健身相关角色时：
- `[行为指南]` 指示管家"追问用户希望由谁来处理"
- 管家遵循此指令，在追问中提及了"创建一个健身管家角色"
- 这实际上构成了第一轮就建议创建角色，违反了 `[角色涌现行为]` 的约束

### Finding 2: 两条执行路径共用同一 prompt

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:1349-1352`

**Detail:** `build_butler_system_prompt` 函数的注释明确说明"Used by both the direct LLM path and the opencode path"。直接 LLM 路径（line 1489）和 opencode 路径（line 2117）都调用此函数。因此无论走哪条路径，prompt 冲突都存在。

### Finding 3: Story 2.5 设计文档已预见此模糊性

**Evidence:** `_bmad-output/implementation-artifacts/2-5-role-emergence-suggestion.md:125`

**Detail:** 设计决策第 1 点写道："缺点：LLM 可能不严格遵守'3+ 次'阈值，但这是可接受的模糊性"。但当前问题不是简单的"不严格"，而是 `[行为指南]` 的指令直接与 `[角色涌现行为]` 冲突，导致 LLM 有合理理由在第一轮就提及角色创建。

## Deduced Conclusions

### Deduction 1: 根因是 prompt 指令冲突而非 LLM 随机行为

**Based on:** Finding 1, Finding 3

**Reasoning:** `[行为指南]` 明确指示"没有合适角色时追问用户由谁处理"，LLM 在追问中自然提及"创建角色"选项。这不是 LLM 无视 `[角色涌现行为]` 的约束，而是两个指令给出了矛盾的方向——一个说"问用户怎么办"，另一个说"别急着建议创建角色"。

**Conclusion:** 修改 prompt 消除冲突即可解决问题，无需引入代码层面的轮次计数机制。

## Hypothesized Paths

### Hypothesis 1: 仅修改 `[行为指南]` 的追问措辞

**Status:** Open

**Theory:** 在 `[行为指南]` 的"意图模糊或没有合适角色时"指令中，明确排除"建议创建角色"的表述，改为"直接用文字帮助用户或简短追问需求细节，不要在此提及创建角色"。

**Supporting indicators:** 这是最小改动，直接消除冲突源头。

**Would confirm:** 修改后 LLM 在第 1 轮不再提及创建角色。

**Would refute:** 修改后 LLM 仍在第 1 轮提及创建角色。

**Resolution:** 待用户确认后验证。

## Missing Evidence

| Gap               | Impact                               | How to Obtain           |
| ----------------- | ------------------------------------ | ----------------------- |
| 修改后的实际效果  | 确认 prompt 修改是否有效             | 修改后运行 UAT 用例1 验证 |

## Source Code Trace

| Element       | Detail                                                                                   |
| ------------- | ---------------------------------------------------------------------------------------- |
| Error origin  | `agent_engine.rs:1439` — `[行为指南]` 中的追问指令                                        |
| Trigger       | 用户发送的消息没有匹配到任何 active 角色                                                  |
| Condition     | `[行为指南]` 指令"追问用户由谁处理" + `[角色涌现行为]` 指令"等 2-3 轮再建议" 同时生效     |
| Related files | `agent_engine.rs`（prompt 构建）、`UAT-Simplified-Manual.md`（预期行为定义）               |

## Conclusion

**Confidence:** High

根因已 Confirmed：管家 system prompt 中 `[行为指南]` 的"没有合适角色时追问用户由谁处理"指令与 `[角色涌现行为]` 的"不要在第一轮就建议，至少等 2-3 轮"指令存在冲突。LLM 遵循 `[行为指南]` 追问时，自然提及了"创建角色"选项，导致第一轮就出现了角色涌现建议。

## Recommended Next Steps

### Fix direction

修改 `agent_engine.rs` 中 `[行为指南]` 的 prompt 文案（约 1439 行），在"意图模糊或没有合适角色时"的指令中明确：不要在追问中提及创建角色，仅简短追问需求细节或直接用文字帮助用户。将角色创建建议的时机控制完全交给 `[角色涌现行为]` 段落。

### Diagnostic

修改后运行 UAT 用例1 阶段B 验证：前 1-2 轮管家不提及创建角色，第 3 轮自然提议。

## Reproduction Plan

1. 启动应用，确保已有至少 1 个 active 角色（如产品经理）
2. 在管家对话中发送"我想开始健身，但不知道怎么制定训练计划"
3. 观察：管家是否在第 1 轮回复中提及创建角色
