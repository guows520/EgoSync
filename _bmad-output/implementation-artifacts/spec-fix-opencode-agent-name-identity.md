---
title: '修复 OpenCode 自定义 Agent 标识冲突'
type: 'bugfix'
created: '2026-07-15'
status: 'in-review'
baseline_commit: '9b16c8800830e518cd040917d5bfa78ac3bac5b2'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/opencode-custom-agent-500-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** EgoSync 以配置键 `butler` / `role-{uuid}` 选择 OpenCode Agent，同时又生成中文 `name` 字段。OpenCode 1.15.10 用该字段覆盖运行时名称，但仍按配置键查找 Agent，导致管家和角色消息稳定返回 HTTP 500，应用降级为基础对话模式。

**Approach:** 保留 `AgentBridge` 的 `agent` 请求参数、稳定键路由、角色 prompt/权限/工具隔离；仅从 EgoSync 生成的管家和角色 Agent 配置中删除 `name` 字段，让配置键成为唯一运行时标识。中文名称继续由 EgoSync UI、prompt 与 description 承载。

## Boundaries & Constraints

**Always:** 保留 `butler` 和 `role-{uuid}` 键；保留管家 `primary`、角色 `subagent` 模式；保留现有 prompt、permission、tools、disable 和角色生命周期同步行为；测试必须说明稳定标识为何重要。

**Ask First:** 若移除 `name` 后真实 OpenCode 请求仍返回 500，或必须升级/替换 OpenCode 二进制，停止并报告新证据，不扩大修改范围。

**Never:** 不删除消息体的 `agent` 参数；不回退到内置 `build`；不恢复“只靠消息文本模拟角色”的旧路径；不修改 Logo 等现有未提交文件；不将中文展示名重新写入 OpenCode `name`。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 管家配置生成 | Butler skills 任意组合 | `agent.butler` 不含 `name`，mode/prompt/permission 保持正确 | 单测失败即阻止交付 |
| 角色配置生成 | active 产品经理角色 | `agent.role-{id}` 不含 `name`，description/prompt 保留中文角色信息 | 单测失败即阻止交付 |
| 启动全量同步 | active 与 archived 角色并存 | 所有生成条目均无 `name`，archived 仍为 `disable=true` | 保持现有显式错误返回 |
| 既有人工配置 | `ensure_butler` 遇到已存在条目 | 保持既有“不覆盖”语义；本修复不承担迁移任意手写配置 | 若旧手写配置含 `name`，由后续全量同步或独立迁移决策处理 |
| 运行时请求 | `agent=butler` 或真实 `role-{uuid}` | OpenCode 返回 HTTP 200，消息 Agent 标识保持稳定键 | 仍为 500 时停止并保留日志证据 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_config.rs` -- 生成并同步 Butler/角色的 OpenCode Agent 配置，测试也位于本文件。
- `egosync-app/src-tauri/src/services/agent_bridge.rs` -- 发送稳定 Agent 键；作为必须保留的架构边界，不计划修改。
- `_bmad-output/implementation-artifacts/investigations/opencode-custom-agent-500-investigation.md` -- 字段级根因、A/B 证据与修复方向。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 删除角色和管家生成项的 `name` 字段；不改变其它配置属性。
- [x] `egosync-app/src-tauri/src/services/agent_config.rs` -- 调整/新增 WHY 测试，覆盖直接构建、角色同步、Butler 确保与全量同步后的稳定标识契约。
- [x] `_bmad-output/implementation-artifacts/investigations/opencode-custom-agent-500-investigation.md` -- 记录实施结果与真实运行验证结论。

**Acceptance Criteria:**
- Given EgoSync 生成 Butler 和角色 Agent 配置，when 读取 `agent` 段，then 条目不含 `name`，且 mode、prompt、description、permission、tools 与 disable 语义未回归。
- Given 应用通过 `AgentBridge` 分别发送 `butler` 和真实 `role-{uuid}`，when OpenCode 1.15.10 处理消息，then 请求返回 HTTP 200 且不触发基础对话降级提示。
- Given 管家收到“明天下午3点要开会讨论产品方案”，when 它识别为产品经理任务并完成委派，then 产品经理仍能调用 `create_task` 创建可追踪任务。

## Spec Change Log

## Design Notes

OpenCode 1.15.10 将配置保存为 `agents[key]`，随后允许 `value.name` 覆盖运行时对象名称，而 `Agent.get()` 仍严格使用 `agents[agent]`。因此把展示名与稳定 ID 分离是最小且保留架构意图的修复；把 `name` 设置为键虽然也能规避错误，但属于重复状态，未来仍有漂移风险。

## Verification

**Commands:**
- `cargo test services::agent_config::tests`（在 `egosync-app/src-tauri`）-- 预期所有 Agent 配置测试通过。
- `cargo test`（在 `egosync-app/src-tauri`）-- 预期完整 Rust 测试零失败、零跳过。
- 使用本机 OpenCode 1.15.10 对 `butler` 与真实 `role-{uuid}` 执行隔离 HTTP 请求 -- 预期均为 HTTP 200。

**Manual checks (if no CLI):**
- 管家界面执行目标自然语言任务，确认无降级提示，并在产品经理任务列表和数据库中看到新任务。
