---
title: '修复角色直聊任务归属'
type: 'bugfix'
created: '2026-07-17'
status: 'done'
baseline_commit: 'b1a6b67341d5d2c8ea209b8f85ef577aceccac80'
context:
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 角色直聊的 `create_task` 要求模型填写当前角色 UUID，但角色 Agent 不知道自己的 UUID，因而把“家庭”等猜测值作为 `role_id`，触发数据库外键错误。

**Approach:** 在 Rust 桥接层绑定 opencode session 与当前角色 UUID；角色直聊由服务端决定归属，模型只提供标题和截止时间。管家仍可显式指定目标角色。

## Boundaries & Constraints

**Always:** 角色归属来自服务端会话；请求携带 `context.sessionID`；写库前验证角色存在；返回中文业务错误；保留自动分类及 `task:classified`；测试覆盖防猜测和防串角色。

**Ask First:** 若 `context.sessionID` 不可用，或必须改变前端 IPC、数据库 schema、管家交互协议，则暂停确认。

**Never:** 不以 Prompt 注入 UUID 为主修复；不信任角色 Agent 的 `role_id`；不改外键、不重构无关功能、不新增依赖。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 角色直聊创建 | session 已绑定家庭角色；工具只传 title/deadline | 任务写入绑定角色，模型无需 UUID | N/A |
| 角色伪造归属 | 家庭 session 传入其他 `role_id` | 忽略模型值，任务仍归家庭 | 记录绑定 ID |
| 管家跨角色创建 | 管家 session 传入真实目标 `role_id` | 保持现有目标角色任务创建行为 | 角色不存在时返回业务错误 |
| 未注册会话 | create-task 使用未知 session | 不写数据库 | 返回“未找到当前会话上下文” |
| 角色已删除 | session 绑定的角色在写入前已删除 | 不写数据库 | 返回“目标角色不存在或已不可用” |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_config.rs` -- `create_task.ts` 工具协议。
- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 持有可信角色 ID 与 opencode session。
- `egosync-app/src-tauri/src/services/delegate_bridge.rs` -- session 上下文与任务归属裁决。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 让 `role_id` 可选、提交 session ID，并测试生成协议。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 注册角色 session 绑定，在现有退出路径清理。
- [x] `egosync-app/src-tauri/src/services/delegate_bridge.rs` -- 裁决可信归属、验证会话与角色、保持管家兼容并测试边界。

**Acceptance Criteria:**
- Given 用户位于角色界面，when 要求创建任务，then 任务归属当前角色且模型无需 UUID。
- Given 角色 session 与工具参数中的角色值冲突，when 桥接层处理请求，then 只能使用 session 绑定角色。
- Given 管家使用现有 create_task 工具指定有效角色，when 请求到达桥接层，then 原有跨角色创建行为保持可用。
- Given session 未注册或目标角色已不存在，when 请求创建任务，then 返回明确业务错误且 tasks 表不新增记录。
- Given 修改完成，when 运行定向测试和 `cargo check`，then 新旧路径通过且无编译错误。

## Spec Change Log

- 2026-07-17 Review patch：将角色 session 注册移动到可失败的 Prompt 构建之后，避免早退遗留可信绑定；新增 HTTP camelCase 契约及解码测试。KEEP：服务端绑定角色、忽略角色 Agent 伪造 ID、保留管家显式目标。

## Design Notes

`DelegateSessionContext` 承载管家触发消息 ID 或当前角色 UUID。`CreateTaskRequest` 增加 `session_id`、将 `role_id` 改为可选；角色会话采用绑定 UUID，只有管家会话读取显式 UUID，保持单一 HTTP 端点。

## Verification

**Commands:**
- `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" delegate_bridge -- --nocapture` -- expected: 会话归属、伪造防护和请求校验测试通过。
- `cargo test --manifest-path "egosync-app/src-tauri/Cargo.toml" agent_config -- --nocapture` -- expected: 生成的 create_task 工具协议符合新契约。
- `cargo check --manifest-path "egosync-app/src-tauri/Cargo.toml"` -- expected: Rust 后端编译通过。

**Results:**
- `git diff --check` -- passed.
- `cargo check` -- passed using isolated D-drive Cargo home/temp; project emitted existing warnings only.
- `cargo test ... delegate_bridge` -- test target compiled, but the Windows test executable could not start (`STATUS_ENTRYPOINT_NOT_FOUND`, `0xc0000139`), so cases did not execute.
- `cargo test ... agent_config` -- not executed separately because it uses the same blocked test binary.

## Suggested Review Order

**可信角色归属**

- 从当前角色与 opencode session 建立服务端可信绑定。
  [`agent_engine.rs:2194`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2194)

- 角色 session 覆盖模型参数，管家 session 保留显式目标。
  [`delegate_bridge.rs:157`](../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L157)

- 所有既有退出路径对称清理桥接 session。
  [`agent_engine.rs:2279`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2279)

**工具协议边界**

- 工具提交 sessionId，角色直聊不再被迫猜 UUID。
  [`agent_config.rs:144`](../../egosync-app/src-tauri/src/services/agent_config.rs#L144)

- Rust 请求采用 camelCase，并在写库前验证角色有效。
  [`delegate_bridge.rs:58`](../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L58)

**回归测试**

- 覆盖伪造归属、跨角色隔离、管家兼容及失效角色。
  [`delegate_bridge.rs:831`](../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L831)

- 固化生成工具的 session 与可选角色参数契约。
  [`agent_config.rs:1271`](../../egosync-app/src-tauri/src/services/agent_config.rs#L1271)
