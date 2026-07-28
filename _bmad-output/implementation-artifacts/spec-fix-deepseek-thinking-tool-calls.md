---
title: '修复 DeepSeek Thinking 工具调用兼容性'
type: 'bugfix'
created: '2026-07-28'
status: 'done'
baseline_commit: '6aaf479'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/second-turn-model-unavailable-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Onboarding 第 3 步同时启用 DeepSeek Thinking 与 `tool_choice: required`，导致 HTTP 400；直接 Provider 读取了 `reasoning_content`，但工具结果 follow-up 没有回传它。

**Approach:** Onboarding 强制工具阶段关闭 Thinking；OpenCode 主路径不改；直接 Provider 让 assistant 工具调用消息承载并序列化 `reasoning_content`。

## Boundaries & Constraints

**Always:** 保留 onboarding 必须调用 `create_role`；普通管家不新增 `tool_choice`；reasoning 只绑定产生该 tool call 的 assistant 消息；所有现有消息构造点显式初始化新增字段。

**Ask First:** 若需要修改 OpenCode 配置、数据库 schema、嵌套工具策略或用户流程，暂停确认。

**Never:** 不全局关闭 Thinking；不把 reasoning 混入可见 content；不修改 onboarding 状态推进和通用错误文案；不重构 Provider 架构。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Onboarding 创建角色 | step >= 3 | 关闭 Thinking，保留 `tool_choice: required` | 不再触发当前 400 |
| 普通管家工具选择 | Thinking + tools | 不传 `tool_choice` | 模型自主选择 |
| Thinking 工具回合 | SSE 含 reasoning 与 tool_calls | follow-up 回传完整 reasoning | 空值不发送字段 |
| 普通消息 | 无 reasoning | 请求保持原结构 | 不增加多余字段 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- onboarding 选项、reasoning 累积和工具 follow-up。
- `egosync-app/src-tauri/src/llm/traits.rs` -- 共享消息模型。
- `egosync-app/src-tauri/src/llm/openai.rs` -- OpenAI-compatible 请求序列化。
- `egosync-app/src-tauri/src/llm/anthropic.rs` 及其他消息构造点 -- 新字段兼容初始化。

## Tasks & Acceptance

**Execution:**
- [x] `agent_engine.rs` -- 修正 onboarding 选项并在工具 assistant 消息附加 reasoning。
- [x] `traits.rs` -- 增加可选 `reasoning_content`。
- [x] `openai.rs` -- 非空时序列化该字段并测试省略规则。
- [x] 其他 Rust 构造点 -- 初始化为 `None`，保持语义。
- [x] 增加 onboarding、普通工具选择和 reasoning 回传测试。

**Acceptance Criteria:**
- Given onboarding step 3，when 构建请求，then Thinking 关闭且 `tool_choice=required`。
- Given 普通管家工具请求，when 构建请求，then不发送 `tool_choice`。
- Given Thinking 已产生工具调用，when 发送 follow-up，then assistant 消息携带原 reasoning。
- Given无 reasoning，when序列化，then省略该字段。
- Given实现完成，when运行定向测试与 `cargo check`，then全部成功。

## Spec Change Log

## Design Notes

`reasoning_content` 是 assistant 消息协议字段，不是工具参数或 UI 文本；Anthropic 和普通 OpenAI 请求忽略空值。

## Verification

**Commands:**
- `cargo test llm::openai --lib`
- `cargo test services::agent_engine --lib`
- `cargo check`



## Verification Results

- `cargo test llm::openai --lib`: PASS — 6 passed, 0 failed.
- `cargo check`: PASS — no compilation errors; existing warnings remain.
- New `agent_engine` regression tests: PASS.
- `cargo test services::agent_engine --lib`: 113 passed, 1 failed. The failing `test_build_butler_system_prompt_omits_disabled_meta_skills` is unchanged from baseline `6aaf479` and outside this fix scope.
- `git diff --check`: PASS.

## Suggested Review Order

**请求策略与工具回合**

- Onboarding 仅在强制创建角色阶段关闭 Thinking。
  [`agent_engine.rs:2336`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2336)

- 工具 follow-up 将 reasoning 绑定到原 assistant 消息。
  [`agent_engine.rs:2297`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2297)

- 实际流式链路累积 reasoning 并构建第二轮消息。
  [`agent_engine.rs:3587`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L3587)

**协议承载与序列化**

- 共享消息类型显式承载可选 reasoning 字段。
  [`traits.rs:18`](../../egosync-app/src-tauri/src/llm/traits.rs#L18)

- OpenAI-compatible 请求仅序列化非空 reasoning 原文。
  [`openai.rs:66`](../../egosync-app/src-tauri/src/llm/openai.rs#L66)

**回归测试**

- 覆盖 onboarding step 1–5 的模式边界。
  [`agent_engine.rs:6407`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L6407)

- 捕获第二轮 Provider 消息验证完整 reasoning 生命周期。
  [`agent_engine.rs:6434`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L6434)

- 验证非空序列化与空白省略规则。
  [`openai.rs:390`](../../egosync-app/src-tauri/src/llm/openai.rs#L390)
