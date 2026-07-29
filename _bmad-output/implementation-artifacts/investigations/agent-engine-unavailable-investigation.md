# Investigation: 角色对话 Agent 引擎不可用

## Hand-off Brief

1. **发生了什么。** 用户提供的角色对话记录显示，系统在 Think 文本后插入固定降级提示；主实现确认该提示只在非 onboarding 的 fallback 路径前缀中发出。
2. **当前状态。** 证据边界已清点：源码与版本历史可用，测试只有间接覆盖，运行日志、复现记录和静态分析结果缺失；根因仍未判定。
3. **下一步需要什么。** 进入因果分析，沿 fallback 之前的错误分支反向追踪 Agent/sidecar 的失败条件，并用日志或复现证据区分具体故障类型。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-29 |
| Status           | Active |
| System           | Windows；Tauri/Rust 后端与前端应用（具体运行版本待确认） |
| Evidence sources | 用户提供的对话记录；主工作区源代码 |

## Problem Statement

用户报告：角色对话中出现“Agent 引擎暂时不可用，当前为基础对话模式。”。对话在 Think 内容后仍生成了普通回复。用户要求先分析原因和方案，暂不执行修复。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户提供的角色对话记录 | Available | 确认用户可见症状与消息顺序；缺少发生时间、运行日志和复现步骤 |
| 主工作区源代码 | Available | 精确提示仅在 `agent_engine.rs` 出现；常量定义于第 26 行，使用于第 3576、3583 行 |
| 平行/恢复实现 | Partial | `_recovery_pre_cr_dirac` 与多个临时工作树存在同名实现；仅作为历史/平行线索，不能替代主工作区证据 |
| Agent/sidecar 运行日志或诊断归档 | Missing | 主应用范围未发现可用 `.log`/`.dmp`/`.trace` 证据；无法区分运行时失败类型 |
| Issue tracker | Missing | 未提供 ticket 或可查询问题记录 |
| 版本控制 | Available | 相关文件历史可查；提示由 `ef123f4d` 引入，当前非 onboarding 判定由 `96099225` 引入；7 月 23–28 日仍有 Agent/sidecar 相关修改 |
| 自动化测试源码 | Partial | `agent_engine.rs` 有大量单元测试，但精确提示只出现在生产实现，没有发现针对该降级提示/分支的直接断言 |
| 已执行测试结果 | Missing | 本调查阶段未运行测试，也未发现可归因于当前问题的结果文件 |
| 静态分析配置 | Available | 存在 `.github/workflows/ci.yml` 及前后端测试配置 |
| 静态分析结果 | Missing | 未发现可归因于当前工作树/问题的 clippy、检查或覆盖率结果 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 追踪 `OPENCODE_FALLBACK_NOTICE` 的所有使用点和调用链 | High | In Progress | 已定位两处使用；下一阶段需反向追踪进入 fallback 之前的失败分支 |
| 2 | 检查 Agent 引擎、sidecar 的启动/健康检查/请求错误路径 | High | Open | 区分引擎未启动与单次请求失败 |
| 3 | 检查角色对话请求如何选择 Agent 模式与基础对话模式 | High | Open | 已确认 onboarding 不显示提示；普通角色对话路径待追踪 |
| 4 | 获取复现日志与 Agent 进程状态 | High | Blocked | 当前没有运行日志、发生时间或稳定复现记录 |
| 5 | 检查相关测试与近期提交 | Medium | In Progress | 已完成证据清点；具体变更影响与测试意图待因果分析阶段核对 |
| 6 | 基于根因形成最小修复与验证方案 | Medium | Open | 仅输出方案，不实施 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 未提供 | 角色对话完成 Think 后显示 Agent 引擎不可用提示，随后输出基础对话回复 | 用户提供的对话记录 | Confirmed |
| 2026-07-29 | 在主工作区定位到完全一致的提示常量 | `egosync-app/src-tauri/src/services/agent_engine.rs:26` | Confirmed |

## Confirmed Findings

### Finding 1: 用户可见提示来自 Agent 引擎服务中的固定回退文案

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:26`

**Detail:** 主工作区定义了 `OPENCODE_FALLBACK_NOTICE`，内容与对话记录中的提示完全一致，并包含两个尾随换行。该证据确认症状不是前端临时生成的未知文案，而与后端 Agent 引擎回退机制直接相关。

## Deduced Conclusions

### Deduction 1: 对话没有整体失败，而是进入了显式回退路径

**Based on:** Finding 1；用户记录中提示之后仍有正常文本回复。

**Reasoning:** 固定文案明确声明“当前为基础对话模式”，且对话继续输出，因此系统更可能捕获了 Agent 路径不可用状态并降级，而非整个聊天请求崩溃。

**Conclusion:** 后续应调查“为何触发降级”，而不是把问题笼统归因于前端渲染失败。

## Hypothesized Paths

### Hypothesis 1: Agent/Opencode 执行路径发生错误后被捕获并切换到基础对话

**Status:** Open

**Theory:** Agent 引擎启动、健康检查或请求执行中的某个错误触发 `OPENCODE_FALLBACK_NOTICE`，系统随后使用基础 LLM 对话完成响应。

**Supporting indicators:** 文案名称包含 `OPENCODE_FALLBACK_NOTICE`；用户记录显示回退后仍有回复。

**Would confirm:** 找到常量使用点的分支条件，并取得对应失败日志或可重复触发该分支的测试。

**Would refute:** 证明该常量在正常 Agent 流程中也会无条件加入，或实际提示来自不同路径。

**Resolution:** 待调查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 常量使用点与调用链 | 无法确定直接触发条件 | CodeGraph 调用链与相关源代码读取 |
| 发生问题时的应用日志 | 无法区分启动、连接、超时、配置或协议错误 | 获取复现时 Tauri/sidecar 日志 |
| 稳定复现步骤与发生频率 | 无法判断必现、偶现或特定角色相关 | 在现有环境按同一角色与输入复现 |
| Agent 引擎配置与进程状态 | 无法确认依赖是否正确启动 | 检查配置加载、进程生命周期和健康状态 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | `egosync-app/src-tauri/src/services/agent_engine.rs:26`，`OPENCODE_FALLBACK_NOTICE` |
| Trigger | 待追踪 |
| Condition | 待追踪 |
| Related files | 待建立调用链后补充 |

## Conclusion

**Confidence:** Low

已确认用户看到的提示来自后端 Agent 引擎服务定义的显式回退文案，并可推导出对话进入了降级路径。尚无证据确定降级是由 Agent 进程不可用、启动失败、请求错误、超时、配置缺失还是其他条件引发。

## Recommended Next Steps

### Fix direction

待确认根因后再确定。候选机制包括：修复 Agent/sidecar 生命周期或配置；细化瞬时错误与真正不可用状态的判定；避免将可恢复错误直接降级；改善面向用户的错误信息。当前均为候选，不应直接实施。

### Diagnostic

先追踪常量使用链和错误分支，再检查对应日志点、配置来源及测试覆盖。若源码仍无法区分运行时原因，应列出最小诊断日志位置，经用户确认后再添加。

## Reproduction Plan

待源代码路径明确后设计。初始方向：在同一角色、同一输入下复现，记录 Agent 进程状态、请求结果、错误分类以及最终是否插入回退提示。

## Side Findings

- CodeGraph 当前将多个 `.claude/worktrees` 中的同名实现排在主工作区之前；本案证据锚点已通过主工作区限定搜索确认，避免将临时工作树误当成当前实现。

## Follow-up: 2026-07-29 #2

### New Evidence

- **Confirmed:** 主实现中 `OPENCODE_FALLBACK_NOTICE` 除定义外只有两处使用：`egosync-app/src-tauri/src/services/agent_engine.rs:3576` 与 `:3583`。
- **Confirmed:** `egosync-app/src-tauri/src/services/agent_engine.rs:3573-3586` 的注释与条件明确区分：onboarding 主动跳过 opencode 时不显示提示；其他 fallback 场景显示提示并将其加入累计响应。
- **Confirmed:** 精确提示没有出现在主工作区其他前端、后端测试文件中；直接分支测试覆盖目前缺失。
- **Confirmed:** Git blame 显示提示常量来自 `ef123f4d`（2026-05-31），非 onboarding 条件来自 `96099225`（2026-07-03）。
- **Confirmed:** 相关文件在 2026-07-23 至 2026-07-28 仍有多次 Agent 流、sidecar 生命周期及 thinking tool call 相关提交，属于下一阶段应核对的变更窗口。

### Additional Findings

证据边界足以进入因果分析，但不足以确认运行时根因。源码可以回答“哪些错误会进入 fallback”，运行日志或稳定复现才可回答本次事件“实际是哪一种错误”。当前必须保持二者分离。

### Updated Hypotheses

- Hypothesis 1 保持 **Open**。提示确由 fallback 路径发出，但 fallback 上游失败条件尚未追踪，不能据此断言是进程未启动、连接失败、超时或模型/工具调用错误。

### Backlog Changes

- 常量使用点定位从 Open 调整为 In Progress。
- 新增高优先级缺口：复现日志与 Agent 进程状态；在证据到位前标记 Blocked。
- 测试与版本历史完成外围清点，待下一阶段针对具体候选根因核对。

### Updated Conclusion

**Confidence:** Low

已确认提示属于“非 onboarding 的 Agent/opencode fallback”用户可见前缀，而不是通用前端错误。但同一 fallback 可能聚合多类上游错误；没有调用链与运行日志前，任何具体根因结论都属于猜测。


## Follow-up: 2026-07-29 #3

### New Evidence

- **Confirmed:** `run_stream` 仅在非 onboarding 场景调用 `try_run_opencode_stream`；成功则直接结束，失败才进入基础 `LlmProvider` 路径（`egosync-app/src-tauri/src/services/agent_engine.rs:3426-3457`）。
- **Confirmed:** fallback 入口把三类结果统一处理为降级：MCP session 重试后仍无效、Skill 加载重试后仍超时、以及 `Fatal(AppError)`（`egosync-app/src-tauri/src/services/agent_engine.rs:3458-3528`）。
- **Confirmed:** `Fatal(AppError)` 可能来自工作目录解析、MCP scope 解析/同步、创建 opencode session、构建角色 prompt，以及 AgentBridge 请求错误等多个阶段（`egosync-app/src-tauri/src/services/agent_engine.rs:2471-2514`、`egosync-app/src-tauri/src/services/agent_engine.rs:2529-2532`、`egosync-app/src-tauri/src/services/agent_bridge.rs:36-62`、`:131-160`）。
- **Confirmed:** AgentBridge 将网络请求失败、HTTP 非 2xx、响应读取失败统一包装为 `AppError::SidecarError`；因此当前用户提示无法区分 sidecar 未启动、连接失败、服务端拒绝或响应异常（`egosync-app/src-tauri/src/services/agent_bridge.rs:49-62`、`:143-160`、`:299-317`）。
- **Confirmed:** 当 opencode 失败且尚未产生普通文本时，`try_run_opencode_stream` 返回 `Fatal`；外层随后无条件给所有非 onboarding 场景发送 `OPENCODE_FALLBACK_NOTICE`（`egosync-app/src-tauri/src/services/agent_engine.rs:3100-3116`、`:3253-3263`、`:3524-3528`、`:3573-3586`）。

### Additional Findings

当前代码确认了一个独立于本次运行时具体错误的设计问题：**用户可见文案的适用范围过宽**。它把多种不同的 Agent 失败机制都描述成“Agent 引擎不可用”，而实际触发点可能只是 session、MCP、Skill、配置或单次 HTTP 请求失败。

此外，日志会记录 `opencode stream unavailable, falling back to LlmProvider: {error}`，但该错误没有随同通用提示传递给用户或持久化为可关联的诊断字段；当前没有本次事件的日志，因此无法确认 `{error}` 的具体内容。

### Updated Hypotheses

#### Hypothesis 2: 该事件由 Agent 请求在产生普通文本前失败触发

**Status:** Open

**Theory:** 角色对话的 opencode session 已进入调用流程，但在 `create_session`、`send_message` 或相关前置阶段返回 `AppError`；由于没有普通文本，代码把错误升级为 `Fatal`，再切换到基础对话。

**Supporting indicators:** 用户记录确实出现了统一 fallback 提示；代码对“无普通文本的 Agent 错误”明确走 `Fatal` 回退。

**Would confirm:** 复现日志中出现 `opencode stream unavailable, falling back to LlmProvider`，并带有 `create_session request failed`、`send_message request failed`、HTTP 错误或对应前置阶段错误。

**Would refute:** 日志证明本次先命中 `InvalidMcpSession` 或 `SkillLoadTimeout` 分支，或 opencode 已产生普通文本后由另一条路径改写了响应。

**Resolution:** 待运行日志或复现。

#### Hypothesis 3: 最近 Agent/sidecar 相关改动造成回归

**Status:** Open

**Theory:** 2026-07-23 至 2026-07-28 的 Agent 错误流、sidecar 生命周期、thinking tool call 或网络路由改动，改变了 opencode 请求的可用性或错误分类。

**Supporting indicators:** 相关提交集中在用户报告前一周，且 `43d59c5` 同时修改了 `agent_engine.rs` 与 `sidecar.rs`。

**Would confirm:** 在 `43d59c5`、`d362249`、`ecc7ea4` 等提交前后，对同一角色和请求进行复现，或提交 diff 显示本次失败路径行为发生变化。

**Would refute:** 证明当前事件对应的失败路径在这些提交之前已存在，且运行时日志指向与近期改动无关的配置/环境错误。

**Resolution:** 待提交级对比与复现。

### Backlog Changes

- 已完成 fallback 分支的代码级反向追踪。
- 新增对 `AppError` 失败类别和近期 Agent/sidecar 提交的针对性核查。
- “根因是 Agent 引擎不可用”这一用户表述已被修正为：**已确认进入 opencode fallback，但具体上游错误未确认**。

### Updated Conclusion

**Confidence:** Medium（针对“提示文案过宽”和 fallback 控制流）；Low（针对本次事件的具体运行时根因）。

源码已确认：普通角色对话中，任何未产出普通文本的 opencode 失败，都可能被统一包装为“Agent 引擎暂时不可用”，随后基础 LLM 接管。因而当前最可靠的诊断是“Agent 路径失败并触发了过宽的统一降级提示”，而不是“Agent 引擎进程必然不可用”。本次具体失败点仍需日志或复现确认。


## Follow-up: 2026-07-29 #4

### Source Code Trace

#### Error origin

- **Confirmed:** 普通角色消息使用 `role-<role_id>` 作为 OpenCode agent key；无缓存 session 时先创建 session，随后把同一 agent key 放入 `POST /session/{id}/message` 请求体（`egosync-app/src-tauri/src/services/agent_engine.rs:2512-2524`、`:2582-2631`；`egosync-app/src-tauri/src/services/agent_bridge.rs:131-151`）。
- **Confirmed:** 角色创建只把新 agent 写入工作区 `opencode.json`，没有通知运行中的 OpenCode 重新加载，也没有重启 sidecar（`egosync-app/src-tauri/src/commands/role.rs:36-65`、`egosync-app/src-tauri/src/services/mcp_server.rs:452-460`、`egosync-app/src-tauri/src/services/agent_config.rs:682-705`）。
- **Confirmed:** 本次角色 `29a96120-7282-4391-b355-8674525cfefd`（“独立开发者”）于 `2026-07-29T07:45:08Z` 创建；其首次对话消息和 fallback 响应均写入于 `2026-07-29T07:45:32Z`（只读查询 `%APPDATA%/com.egosync.desktop/egosync.db` 与 `conversations.db`）。
- **Confirmed:** 磁盘上的 `opencode-workspace/opencode.json` 已包含 `role-29a96120-7282-4391-b355-8674525cfefd`，但运行中 OpenCode 1.15.10 的 `GET /agent?directory=...` 和 `GET /config?directory=...` 均仍返回启动时的旧 agent 集合：包含已删除的产品经理 agent，不包含新建“独立开发者”agent。
- **Confirmed:** OpenCode 运行日志在同一秒记录：session `ses_0532a02ffffe5ufR3zCTSxomBY` 创建成功，随后立即发布 `session.error`，并在 `SessionPrompt.createUserMessage` 抛出 `UnknownError`（`C:/Users/Admin/AppData/Roaming/com.egosync.desktop/opencode-global/data/opencode/log/2026-07-28T145051.log:1660-1668`）。该 session 的 `GET /session/{id}/message` 返回 0 条消息，说明失败发生在创建用户消息阶段，而非模型生成阶段。
- **Confirmed:** OpenCode v1.15.10 源码的 `SessionPrompt.createUserMessage` 在请求指定的 agent 不存在时发布 `session.error` 并抛出 `NamedError.Unknown("Agent not found")`。这与本次日志栈、空 session 和运行时缺失 agent 三项证据完全吻合。

#### Trigger

新角色在 OpenCode sidecar 已运行期间创建；EgoSync 将角色配置写入磁盘后立即允许进入角色对话，但 OpenCode 进程仍持有启动时加载的 agent/config 快照。首次消息携带新 agent key，OpenCode 在 `createUserMessage` 阶段找不到该 agent 并报错；EgoSync 将错误归入 `Fatal(AppError)`，最终显示统一 fallback 文案。

#### Condition

1. sidecar 已经启动并加载过工作区配置；
2. 运行期间新增或变更角色 agent；
3. 仅写 `opencode.json`，未使 OpenCode runtime 刷新；
4. 在 sidecar 重启前向新 agent 发消息。

#### Related files

- `egosync-app/src-tauri/src/commands/role.rs:36-65`：角色创建后同步配置，但同步错误仅告警且无 runtime refresh。
- `egosync-app/src-tauri/src/services/agent_config.rs:682-705`：外部文件写入式 agent 同步。
- `egosync-app/src-tauri/src/services/agent_engine.rs:2512-2524`、`:2582-2631`：session 创建与新 agent 消息发送。
- `egosync-app/src-tauri/src/services/agent_bridge.rs:131-160`：HTTP 错误包装。
- `egosync-app/src-tauri/src/services/agent_engine.rs:3426-3586`：Fatal 回退与统一提示。
- `egosync-app/src-tauri/src/services/agent_engine.rs:3388-3403`：已有 sidecar restart + session cache clear 能力，可作为修复复用点。

### API Contract Audit

- **Confirmed / corrected:** `GET /event` 在 OpenCode 1.15.10 是有效 SSE 路由，当前 `subscribe_events` 路径及 `{type, properties}` 解析与实测一致。此前“`/event` 返回 HTML”的判断已被反驳；`/global/event` 也有效，但 payload 结构不同，不应替换当前 `/event`。
- **Confirmed:** `GET /health` 不是 OpenCode 1.15.10 健康 API；SPA fallback 会返回 HTTP 200 HTML。真实健康端点是 `GET /global/health` JSON。因此 `sidecar.rs:647-667`、`:711-716` 存在假阳性健康检查缺陷，但它不是本次直接触发点，因为本次 sidecar 确实运行并成功创建 session。
- **Confirmed:** `GET /providers`、`GET /agents`、`GET /session/{id}/messages`、`POST /session/{id}/compact` 与 1.15.10 契约不符；正确主 API 分别是 `/provider`、`/agent`、`/session/{id}/message`、`/session/{id}/summarize`。当前项目中这些四个 AgentBridge 方法没有调用方，因此属于待清理的潜在故障，不是本次角色消息失败路径。
- **Confirmed:** `create_session` 把 `agent` 放在 query，而 1.15.10 契约把 `agent` 定义在 JSON body。当前消息发送仍会再次在 body 指定 agent；此错配应修正，但本次 session 已成功创建，直接失败发生在后续 `createUserMessage`。

### Hypothesis Resolutions

#### Hypothesis 2: Agent 请求在产生普通文本前失败

**Status:** Confirmed

**Resolution:** `POST /session/{id}/message` 在 OpenCode 的 `SessionPrompt.createUserMessage` 阶段因请求 agent 不存在于运行时快照而失败；尚未写入 user message，也未开始模型生成。

#### Hypothesis 3: 最近 Agent/sidecar 改动造成回归

**Status:** Open（降级为次要）

**Resolution:** 本次直接根因已由运行时 agent 配置陈旧解释，无需依赖 Windows Job Object 或 sidecar 启动失败假设。近期提交是否引入“写配置但不刷新 runtime”的行为仍可做提交级追溯，但不阻塞诊断。

#### Hypothesis 4: OpenCode API 路径整体错配导致 EventRouter 失效

**Status:** Refuted（针对本次事件）

**Resolution:** 当前使用的 `/event` 是 1.15.10 有效 SSE 路由，事件解析契约匹配。其他未使用的复数路由确有错配，但未进入本次聊天路径。

### Diagnosis

**Confirmed root cause:** 新建角色的 agent 配置只写入磁盘，运行中的 OpenCode 1.15.10 没有重新加载该配置；EgoSync 随即用新 agent key 发送消息，OpenCode 因 `Agent not found` 在创建用户消息阶段失败。外层又把该具体错误聚合成“Agent 引擎暂时不可用”。

**Confidence:** High。

### Proposed Remediation (Not Executed)

1. **首选最小修复：角色 agent 配置发生增删改后，受控重启 OpenCode runtime，并清空 `OpencodeSessions`。** 项目已有 `refresh_opencode_runtime_for_mcp_retry` 的 restart + cache clear 能力，建议抽成通用 runtime refresh，避免复制逻辑。
2. **刷新完成后做契约级验证：** 调用 `/global/health` 并校验 JSON `healthy/version`；再调用 `/agent?directory=...` 验证目标 agent key 已出现，之后才向 UI 返回“角色创建成功”或允许首次对话。
3. **避免每次消息无条件重启：** 只在会改变 OpenCode runtime 配置的事务成功后刷新；若刷新失败，应显式返回“角色已保存但 Agent 加载失败”，而不是静默 `sync_warn`。
4. **增加一次针对 `Agent not found` 的恢复：** 若首次发送命中该明确错误，刷新 runtime、清 session cache、重建 session 后仅重试一次；不得把所有 `Fatal(AppError)` 都当作可重试。
5. **修正健康检查与未使用 API 路由：** `/health`→`/global/health` 且校验 content-type/schema；同步修正复数路由和 create-session agent body，补 OpenCode 1.15.10 contract tests。
6. **改进错误分类和可观测性：** 保留 failure stage、HTTP status、OpenCode error message/ref、agent key、session id；用户提示区分“新角色尚未加载”“sidecar 未启动”“模型请求失败”等。

### Acceptance Criteria for a Future Fix

- sidecar 运行期间新建角色后，无需重启 EgoSync 应用即可立即首次对话；OpenCode `/agent` 可见新 key。
- 角色更新、归档、删除后，runtime agent 集合同磁盘配置一致，不保留 ghost agent。
- 健康检查不会把 HTML SPA 的任意 2xx 判断为 API 健康。
- `Agent not found` 可被结构化识别，最多自动恢复一次；恢复失败时不显示笼统“Agent 引擎不可用”。
- 覆盖“运行中新增角色→首次消息”“配置刷新失败”“sidecar 重启后 session cache 清空”测试。
