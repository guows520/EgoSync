---
title: '禁用 OpenCode question 工具并改用普通对话确认'
type: 'bugfix'
created: '2026-07-16'
status: 'done'
baseline_commit: '99e2105723483a6e17be1a3553318999837d03fc'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/uat-case-5-delegation-hang-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** EgoSync 允许 OpenCode 调用 `question`，但没有实现回答与会话恢复协议，导致界面永久停在“正在使用 question...”。

**Approach:** 对管家和所有角色强制设置 `question: "deny"`；需要决策时改用普通消息列出问题与选项，结束本轮并等待用户回复。

## Boundaries & Constraints

**Always:** 保留其他权限值；最终强制 `skill`、`question` 为 `deny`；管家和角色 prompt 使用相同确认规则；修改限定在 `agent_config.rs`。

**Ask First:** 如需修改 `agent_engine.rs`、前端、数据库或新增运行时协议，立即暂停。

**Never:** 不实现 question UI、reply/reject、恢复或超时状态机；不改委派与任务工具；用户配置不能重新开启 `question`。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 默认配置 | 配置为空或无效 | `*:"allow"`，`skill/question:"deny"` | 沿用现有回退 |
| 自定义权限 | `question:"allow"` | 强制改为 `deny`，保留其他权限 | 覆盖不支持能力 |
| 需要确认 | 管家或角色遇到歧义/冲突 | 普通消息给出问题与选项，结束本轮 | 等待下一条用户消息 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_config.rs` -- Agent 权限、prompt 生成及同文件测试。
- `_bmad-output/implementation-artifacts/investigations/uat-case-5-delegation-hang-investigation.md` -- 根因与方案证据。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 共享权限解析强制禁用 `question`。
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 管家和角色 prompt 注入普通确认规则。
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 更新断言并测试默认、自定义 allow、两类 prompt。

**Acceptance Criteria:**
- Given 任意有效或无效权限配置，when 生成 Agent entry，then `question="deny"` 且其他非保留权限不变。
- Given 管家或角色需要确认，when 读取其 prompt，then 要求普通消息提问、结束本轮并等待下一条回复。
- Given 现有 agent_config 测试，when 执行针对性测试，then 全部通过。

## Spec Change Log

## Verification

**Commands:**
- `cd egosync-app/src-tauri && cargo test agent_config` -- expected: agent_config 相关单元测试全部通过。
- `cd egosync-app/src-tauri && cargo fmt --check` -- expected: Rust 格式检查通过。
- `git diff --check` -- expected: 无空白错误。

## Suggested Review Order

**能力边界**

- 共享规则定义普通确认行为，避免进入不可恢复等待。
  [`agent_config.rs:278`](../../egosync-app/src-tauri/src/services/agent_config.rs#L278)

- 权限末端强制禁用宿主不支持的交互工具。
  [`agent_config.rs:420`](../../egosync-app/src-tauri/src/services/agent_config.rs#L420)

**Prompt 覆盖**

- 角色在所有 Skill/MCP 指令后获得最终确认规则。
  [`agent_config.rs:392`](../../egosync-app/src-tauri/src/services/agent_config.rs#L392)

- 管家同样将确认规则置于自定义 Skill 之后。
  [`agent_config.rs:489`](../../egosync-app/src-tauri/src/services/agent_config.rs#L489)

**回归保护**

- 自定义 allow 无法重新开启 question，其他权限保持。
  [`agent_config.rs:1040`](../../egosync-app/src-tauri/src/services/agent_config.rs#L1040)

- 独立验证管家规则最终优先于自定义 Skill。
  [`agent_config.rs:1241`](../../egosync-app/src-tauri/src/services/agent_config.rs#L1241)
