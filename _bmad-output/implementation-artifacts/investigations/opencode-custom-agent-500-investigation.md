# Investigation: opencode 自定义 Agent 请求触发 HTTP 500

## Hand-off Brief

1. **What happened.** 已确认安装包中的 opencode sidecar 正常启动，但发送管家消息时因指定自定义 Agent 而返回 HTTP 500，应用因此进入基础对话降级模式。
2. **Where the case stands.** 根因边界已通过本地 A/B 请求稳定复现；`agent=build` 成功，`agent=butler` 与 `agent=管家` 均失败。
3. **What's needed next.** 保留消息体中的自定义 `agent` 选择；从生成配置中移除会覆盖内部标识的 `name` 字段，以配置键 `butler` / `role-{uuid}` 作为唯一运行时标识，然后补 Agent 启动烟测。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-15 |
| Status | Concluded |
| System | Windows x64；EgoSync 0.1.1；opencode 1.15.10；MiniMax-M3 |
| Evidence sources | EgoSync 日志、opencode 日志、运行中进程、HTTP A/B 请求、Git 基线差异 |

## Problem Statement

人工验证安装包时，管家回复“Agent 引擎暂时不可用，当前为基础对话模式”。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| `%APPDATA%/com.egosync.app/egosync.log` | Available | 记录 sidecar 成功启动和消息请求 HTTP 500 |
| opencode 运行日志 | Available | 记录 `SessionPrompt.run` 抛出 `UnknownError` |
| 本地 HTTP A/B | Available | 对照不同 `agent` 参数的确定性结果 |
| opencode 内部未压缩源码 | Missing | 日志只有 Bun 打包堆栈，未暴露更深异常消息 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | sidecar 是否打包并启动 | High | Done | 已排除 |
| 2 | 模型/API Key 是否不可用 | High | Done | `build` Agent 使用同一模型成功 |
| 3 | 自定义 Agent 选择是否触发 500 | High | Done | A/B 已确认 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-15 16:56:17 | opencode server 在 4096 成功启动 | EgoSync log 1420-1421 | Confirmed |
| 2026-07-15 16:56:50 | 管家消息返回 HTTP 500，引用 `err_8a808149` | EgoSync/opencode logs | Confirmed |
| 2026-07-15 16:57:24 | 重启后再次返回 HTTP 500，引用 `err_0cdb9985` | EgoSync/opencode logs | Confirmed |
| 2026-07-15 17:xx | 本地 A/B：`build`=200，`butler`/`管家`=500 | HTTP diagnostic | Confirmed |

## Confirmed Findings

### Finding 1: sidecar 和模型服务可用

**Evidence:** EgoSync log 1418-1421；`GET /global/health` 返回 healthy；`agent=build` 的消息返回 HTTP 200。

**Detail:** 安装包包含 `D:/Programs/EgoSync/resources/opencode.exe`，进程正常运行；同一 opencode、Provider 和 MiniMax-M3 配置能成功完成 build Agent 请求。

### Finding 2: 自定义 Agent 选择稳定触发 500

**Evidence:** A/B 请求中消息体不传 agent 或传 `build` 返回 200；传 `butler` 或 `管家` 返回 500。

**Detail:** opencode 日志在模型主调用前的 `SessionPrompt.run` 阶段抛出 `UnknownError`，所以不是模型响应后的解析错误。

### Finding 3: 当前代码每条消息都发送自定义 Agent

**Evidence:** `egosync-app/src-tauri/src/services/agent_bridge.rs:120-124`；基线提交 `e953b9e` 将 `agent` 加入消息请求体。

**Detail:** 该变更直接把已确认的失败条件带入所有管家/角色消息。

## Deduced Conclusions

### Deduction 1: 降级提示是 HTTP 500 的结果，不是 sidecar 缺失

**Based on:** Findings 1-3。

**Reasoning:** sidecar 健康且默认 Agent 请求成功；只有自定义 Agent 请求失败；应用捕获 opencode 500 后按设计回退到直接 LLM Provider，并添加统一降级文案。

**Conclusion:** 当前“不可用”的直接根因是 opencode 1.15.10 在现有生成配置下无法运行自定义 Agent，而应用强制指定了该 Agent。

## Hypothesized Paths

### Hypothesis 1: opencode 自定义 Agent 配置与 1.15.10 运行协议不兼容

**Status:** Confirmed

**Theory:** EgoSync 在以配置键建立 Agent 的同时写入中文 `name` 字段，OpenCode 用后者覆盖 Agent 内部名称，导致消息保存的名称与 Agent 查找表键不一致。

**Supporting indicators:** 自定义 Agent 均失败，内置 `build` 成功；运行时 `/agent` 返回中文 `name`，但配置键为 `butler` / `role-{uuid}`。

**Would confirm:** OpenCode 1.15.10 源码显示自定义 Agent 先存入 `agents[key]`，随后执行 `item.name = value.name ?? item.name`；`get(agent)` 则严格执行 `agents[agent]`。该链已确认。

**Would refute:** 移除 `name` 后仍以相同错误路径失败。

**Resolution:** 根因已定位到 `name` 字段与配置键的标识冲突；不应删除消息请求中的 `agent` 参数。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 修改后的安装包运行验证 | 需确认移除 `name` 后 500 消失且角色身份、权限和工具仍正确 | 重新打包后执行 Butler 与产品经理两条真实消息链 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `agent_bridge.rs:120-124` 消息体发送 `agent`；opencode `SessionPrompt.run` 返回 500 |
| Trigger | 管家或角色发送任意消息 |
| Condition | 请求选择 EgoSync 自定义 Agent |
| Related files | `agent_engine.rs` 的 `opencode_agent_key` 与 opencode 消息调用；`agent_config.rs` 的 Agent 配置生成 |

## Conclusion

**Confidence:** High

应用降级的根因已确认到字段级：EgoSync 自定义 Agent 配置同时使用对象键作为稳定 ID，又写入中文 `name`；OpenCode 1.15.10 把 `name` 覆盖进运行时 Agent 对象，但仍按对象键查找 Agent，造成首轮消息记录的 Agent 名称与后续查找键不一致并返回 HTTP 500。`agent` 参数本身是角色隔离架构所必需，不应回退；应修正配置生成，使对象键成为唯一运行时标识。

## Recommended Next Steps

### Fix direction

保留 `AgentBridge::send_message(..., agent_key)` 和 `opencode_agent_key()` 路由。仅修改 `AgentConfigService` 生成项：删除 Butler 与角色条目中的 `name` 字段；展示名称放在 `description` 或 EgoSync 自己的角色 UI 中，不再占用 OpenCode 的内部 `name`。同步更新配置结构测试，增加运行时回归测试：以 `butler` 和真实 `role-{uuid}` 分别发消息，断言 HTTP 200、返回消息中的 agent 标识保持为对应键，并验证 Butler 委派后角色仍能调用 `create_task`。

### Diagnostic

修复后执行相同 A/B 和安装包 UAT，确认应用日志不再出现 `opencode stream unavailable`，并验证委派工具仍可调用。

## Reproduction Plan

1. 启动当前安装包，确认 opencode 健康。
2. 创建 session，向 `/session/{id}/message` 发送 `agent=build`，预期 200。
3. 新 session 发送 `agent=butler`，当前稳定返回 500。
4. 在 EgoSync 管家发送任意消息，预期出现统一降级文案。

## Follow-up: 2026-07-15

用户指出既有原理变更有明确目的，不能以回退 `agent` 参数规避问题。复核 Story 2.0b/2.0c 后确认该参数承担真实的 Agent 身份、prompt 与权限隔离，旧建议作废。

OpenCode 1.15.10 `agent/agent.ts` 的确定性链路为：自定义项创建于 `agents[key]`；配置中的 `value.name` 随后覆盖 `item.name`；`Agent.get(agent)` 仍以 `agents[agent]` 严格按键读取。EgoSync 当前生成 `butler -> { name: "管家" }`、`role-{uuid} -> { name: "产品经理" }`，因此内部 ID 发生分裂。修复应删除生成配置的 `name`，保留对象键和消息请求 `agent` 参数。

## Side Findings

- 直接 LLM Provider 回退仍能继续处理，并且本次日志中已出现任务分类记录；降级不是应用完全不可用，但 opencode 的工具/会话能力不可依赖。

## Implementation Results: 2026-07-15

- 已从 `AgentConfigService` 生成的 Butler 与角色 Agent 条目中移除 `name`；`butler` / `role-{uuid}` 配置键、mode、prompt、description、permission、tools 与 disable 行为保持不变。
- 已补充 WHY 测试，覆盖直接构建、角色创建同步、`ensure_butler` 和启动 `full_sync`，明确断言所有 EgoSync 生成条目不再产生第二运行时名称，同时中文角色信息仍由 prompt/description 承载。
- `cargo test services::agent_config::tests --lib` 完成：本次新增/修改的稳定标识测试全部通过；模块共 34 项，31 通过、3 个既有 Butler prompt 断言失败。失败断言未由本补丁修改，分别仍期待旧文案 `你是EgoSync管家` 或 prompt 中不出现 `find-skills`，与当前静态 prompt 已有内容不一致，未纳入本次修复范围。
- 使用临时移除 `name` 的运行配置启动独立 OpenCode 1.15.10（端口 4097）后，`agent=butler` 与真实 `agent=role-83a90253-e088-45e3-b2a0-178a6d3c73b5` 均返回 HTTP 200，响应中的 Agent 标识分别保持为 `butler` 与对应 `role-{uuid}`。诊断进程已停止，安装应用的运行配置已恢复。
- 完整管家委派、角色调用 `create_task` 及数据库写入仍需在重新构建应用后做人工 UAT。
- `npx tauri build` 已成功完成 release 编译并生成 MSI/NSIS 完整安装包；构建包含 `resources/opencode.exe`，可用于最终人工 UAT。
