# Investigation: UAT 用例 5 委派任务未经确认被拆分创建

## Hand-off Brief

1. **发生了什么。** 管家把“安排家长会”扩展为多个具体行动交给家庭角色，角色依照现有多行动任务规则一次生成四个工具调用，后端未经确认逐项写库。
2. **当前状态。** 根因与源码边界已确认：角色首轮产生多个工具调用后，后端立即逐项写库；系统没有拆分提案、确认状态或确认后的恢复协议。
3. **下一步。** 用户已选择方案 C；应创建持久化拆分提案规格，明确整批确认、保持单任务、编辑范围和过期语义后实施。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-16 |
| Status           | Concluded |
| System           | Windows；EgoSync Tauri 开发版；OpenCode sidecar |
| Evidence sources | UAT 用例、用户转录、`egosync.db`、`conversations.db`、Git 历史、需求/测试/源码清单；关联运行日志缺失 |

## Problem Statement

用户报告：输入“明天下午3点要参加儿子的家长会，帮我安排一下”后，管家将事项委派给家庭角色。家庭角色把该事项拆成四项任务并直接创建，没有先让用户确认拆分是否合理。用户期望：单任务可直接创建；一旦拆成多个子任务，应先展示拆分结果，让用户选择接受拆分或保持为一个任务。用户要求先分析原因和方案，暂不执行修复。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| `_bmad-output/uat/UAT-Simplified-Manual.md` | Available | 用例 5 位于第 340 行，阶段 C 位于第 362 行 |
| 用户提供的执行转录 | Available | 包含家庭角色建议、四项权威创建结果和管家最终转述 |
| `egosync.db` | Available | 四个 taskId 均存在，归属同一 role_id，created_at 完全相同 |
| `conversations.db` | Available | 已定位家庭角色会话 `79b21264-...`、消息 `9821a028-...` 的四次 create_task，以及父会话 `4d095199-...` 的 delegate_to_role completed |
| 应用日志 | Partial | 文件存在但最后写入早于本次 09:03Z 事件，taskId 无命中；无法用于本次时间线 |
| 产品规格 | Available | PRD、架构、任务 Story、委派规格、UAT 均可检索；需在下一阶段读取相关条款 |
| 源代码 | Available | `agent_config.rs`、`agent_engine.rs`、`delegate_bridge.rs`、任务 DB/Command 等入口可用 |
| 版本历史 | Available | `9b16c88` 引入被委派角色创建跟踪任务；随后有过程持久化与任务 ID 上下文修复 |
| 测试资产 | Partial | 有委派、任务 CRUD、E2E 测试，但清单中未发现“拆分前确认”的专用测试 |
| 静态/测试结果 | Partial | 仅有 2026-06-10 的旧 `rust-test-full.log`，不能代表当前版本；未执行新测试 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 定位管家与家庭角色会话及四次 create_task 事件 | High | Done | 四次调用来自同一家庭角色消息，父会话随后委派完成 |
| 2 | 检查管家/角色 prompt 的任务创建与用户确认规则 | High | Done | prompt 明确鼓励多行动分别调用，无拆分确认 |
| 3 | 检查 create_task 工具与委派桥接是否支持批量前确认 | High | Done | 首轮调用在 `agent_engine.rs:4050` 立即执行，`3924-3960` 直接写库 |
| 4 | 检查 PRD、Story、UAT 对任务拆分确认的明确要求 | High | Done | 现有规格允许多任务，UAT 无确认要求 |
| 5 | 检查相关测试和近期提交 | Medium | Done | 测试明确锁定多任务即时创建；无确认相关测试 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 未知 | 用户请求安排次日下午 3 点家长会 | 用户转录 | Confirmed |
| 未知 | 管家委派家庭角色处理 | 用户转录 | Confirmed |
| 2026-07-16T09:03:41Z | 四项家庭任务同批写入数据库 | `egosync.db` | Confirmed |
| 2026-07-16T09:03:46Z | 家庭角色同一 assistant 消息记录四次 create_task completed | `conversations.db` | Confirmed |
| 2026-07-16T09:03:46Z | 父会话记录 delegate_to_role completed | `conversations.db` | Confirmed |
| 2026-07-16T09:03:46Z | 家庭角色回复“四个任务已经建好” | `conversations.db` | Confirmed |
| 未知 | 管家向用户告知家庭角色已建好四项任务 | 用户转录 | Confirmed |

## Confirmed Findings

### Finding 1: 一个用户事项被真实拆成四项任务

**Evidence:** `egosync.db` 中 taskId `355a9b07-...`、`2f2559f3-...`、`8f156e78-...`、`c83228c3-...`

**Detail:** 四项任务归属同一家庭角色 `be8c3d91-1049-453f-b45a-d88f643078d5`，created_at 均为 `2026-07-16T09:03:41Z`，证明它们来自同一轮委派执行。

### Finding 2: UAT 场景位置明确

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:340`、`_bmad-output/uat/UAT-Simplified-Manual.md:362`

**Detail:** 报告属于用例 5 阶段 C“事实记忆与任务分流”。

### Finding 3: 管家把一个概括性请求扩展为多个行动维度

**Evidence:** `conversations.db` 父会话 message `6d1b0b0d-...` 的 delegate_to_role input；子会话 user message `0117c8c5-...`

**Detail:** 委派上下文不是只传“参加家长会”，而是明确列出“提前下班时间、提醒事项、是否需要准备材料或与老师沟通等”。家庭角色收到的输入因此已经呈现为多个可执行事项。

### Finding 4: 现有规格明确允许多行动分别创建

**Evidence:** `_bmad-output/implementation-artifacts/spec-delegated-role-task-tracking.md:19`、`:35`、`:69`；commit `9b16c88`

**Detail:** 规格 Always 写明“一句话中的多个独立行动项允许分别创建”，边界矩阵要求多个合法调用分别创建；设计说明规定首轮允许多个工具调用，后端逐项执行。

### Finding 5: UAT 阶段 C 没有拆分确认要求

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:362-383`

**Detail:** 现有预期只要求事实不委派、任务安排委派给家庭并转述结果；没有限制创建数量，也没有要求拆分前确认。

### Finding 6: 被委派角色使用专用 Provider 工具协议，而非依赖 opencode Agent 配置

**Evidence:** `_bmad-output/implementation-artifacts/spec-delegated-role-task-tracking.md:69`；commit `9b16c88`

**Detail:** 委派路径向角色 system message追加 `DELEGATED_TASK_PROMPT`，提供 `create_delegated_task`；首轮工具调用会被后端立即执行，第二轮仅负责生成建议和权威结果转述。

### Finding 7: 后端即时写入边界没有确认钩子

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:4034-4055`、`:3924-3960`

**Detail:** `run_delegated_role_provider` 收到首轮 tool calls 后直接调用 `execute_delegated_task_calls`；后者逐项调用 `tasks::create_task`。调用前没有 pending proposal、用户确认标识或数量守卫。

### Finding 8: 后端忠实转发管家生成的摘要和上下文

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:4137-4146`；`egosync-app/src-tauri/src/services/agent_config.rs:77-110`

**Detail:** `execute_delegate_to_role` 只 trim 并格式化 task_summary/context，不负责扩写。扩写发生在管家模型生成 delegate_to_role 参数时；当前 context 描述允许“用户原话或必要背景”，后端无法验证内容是否源于用户原话。

### Finding 9: 项目已有两种确认范式，但语义不同

**Evidence:** `egosync-app/src-tauri/src/services/agent_engine.rs:4303-4377`；`egosync-app/src-tauri/src/commands/suggestion.rs:18-63`

**Detail:** `role:proposed` 是瞬时事件+Modal，确认后创建角色；suggestion 是持久化 pending→confirm/reject→创建任务。后者更接近任务拆分确认，但缺少 deadline、委派来源、批次语义，且确认写入不是完整事务。

## Deduced Conclusions

### Deduction 1: 问题发生在任务写入之前的决策边界

**Based on:** Finding 1

**Reasoning:** 四项任务都成功写入，说明数据库和 create_task 执行链完成；争议是“是否应拆分及何时确认”，而非创建失败。

**Conclusion:** 调查应优先检查 Agent 行为契约和确认策略，而不是数据库写入实现。

### Deduction 2: 四项拆分是当前 prompt 与工具协议共同产生的预期行为

**Based on:** Finding 3、Finding 4、Finding 6

**Reasoning:** 管家把请求展开为四类行动；角色被明确要求对多个独立行动分别调用工具；后端设计为逐调用立即写库。

**Conclusion:** 该结果不是随机失控或重复调用，而是现有契约下可预期的任务分解。

### Deduction 3: 用户提出的是产品契约变更，而非现有验收失败

**Based on:** Finding 4、Finding 5

**Reasoning:** 实现符合已批准的多任务创建规格，UAT 又未要求拆分确认；“拆分前必须确认”此前没有被定义。

**Conclusion:** 修复方向不能只当作一行 prompt bug，应先明确新的确认语义和边界。

### Deduction 4: 仅修改 prompt 可以改善行为，但不能形成强保证

**Based on:** Finding 7、Finding 8

**Reasoning:** prompt 可要求多任务时先返回建议，但后端依然会无条件执行模型误发的多个调用；管家 context 也仍可能扩写。

**Conclusion:** 若验收要求“拆分后必须确认”，仅 prompt 方案存在不可消除的概率性绕过。

### Deduction 5: 完整用户选择语义需要持久化两阶段协议

**Based on:** Finding 7、Finding 9

**Reasoning:** 用户要接受拆分或保持单任务，需要保存提案批次、等待选择，并在确认后才产生真实任务。现有 suggestion 模式已提供大部分持久化和 UI 交互基础。

**Conclusion:** 语义完整方案应复用 pending suggestion/ActionCard，而不是只发瞬时事件。

## Hypothesized Paths

### Hypothesis 1: 现有 prompt 允许角色自主把委派事项拆成多个任务并直接创建

**Status:** Confirmed

**Theory:** 家庭角色被授权调用 create_task，但没有“多任务拆分前必须确认”的约束，因此模型自主规划并连续调用四次工具。

**Supporting indicators:** 四项任务在同一秒创建，转录中没有确认轮次。

**Would confirm:** 角色 prompt 仅规定何时创建任务，不限制单次委派的创建数量，也没有拆分确认要求。

**Would refute:** prompt 已明确要求多任务拆分前确认，但运行时没有遵守或没有收到该 prompt。

**Resolution:** `spec-delegated-role-task-tracking.md` 与 commit `9b16c88` 均明确允许多个独立行动分别调用工具，后端逐项立即执行。

### Hypothesis 2: “拆分必须确认”尚未进入现有产品规格

**Status:** Confirmed

**Theory:** 当前实现可能符合既有“角色可创建跟踪任务”规范，而用户此次提出了新的交互规则。

**Supporting indicators:** 当前 UAT 阶段名称强调任务分流，但用户提供内容未显示用例原文是否规定拆分确认。

**Would confirm:** PRD、Story、UAT 和测试均没有多任务拆分确认条款。

**Would refute:** 已有验收标准明确规定拆分前必须让用户接受或拒绝。

**Resolution:** UAT 阶段 C、委派跟踪任务规格和现有测试均未规定拆分确认；相反，规格明确允许多调用。

### Hypothesis 3: 管家委派上下文放大了拆分倾向

**Status:** Confirmed

**Theory:** 管家将用户原始的一项安排主动扩写为提前下班、提醒、材料、老师沟通四类动作，使角色把它们判断为独立任务。

**Supporting indicators:** 子会话输入逐字包含这些行动维度，四个最终任务与之直接对应。

**Would confirm:** 委派工具 input 与四项任务一一对应。

**Would refute:** 子角色只收到原始概括请求，行动维度完全由其自行生成。

**Resolution:** 委派 input 已确认包含四类行动，反驳条件不成立。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 对应管家与家庭角色会话事件 | 无法确认由哪一层决定拆分 | 按 taskId、消息文本和时间查询 `conversations.db` |
| 本次运行时应用日志 | 无法用日志交叉验证工具调用细节 | 当前日志未覆盖该时段；优先使用已持久化过程事件 |
| 产品对确认粒度的最终选择 | 决定是整批接受/拒绝，还是允许逐项编辑 | 用户确认产品交互规则后再实施 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | `agent_engine.rs:3813` 鼓励多调用；`:4050` 立即执行；`:3952` 写库 |
| Trigger | 管家将概括请求扩写为多个行动并委派给家庭角色 |
| Condition | 角色首轮返回两个及以上 create_delegated_task，系统没有确认状态或数量守卫 |
| Related files | `agent_config.rs`、`agent_engine.rs`、`commands/suggestion.rs`、suggestion 模型/DB、`ActionCard.tsx`、`ButlerView.tsx` |

## Conclusion

**Confidence:** High

根因已确认：当前系统按既有契约鼓励多行动分别调用工具，并在模型首轮结束后立即逐项写库；既没有拆分确认产品规则，也没有 pending/confirm/reject 技术状态。用户期望属于新产品契约。若只要求低成本改善，可改 prompt；若要求可靠保证并允许接受拆分或保持单任务，则需持久化两阶段确认。

**Status:** Concluded

## Recommended Next Steps

### Fix direction

**方案 A：prompt 级两轮确认（最小改动，弱保证）。**

- 多个行动时禁止首轮调用工具，先用普通消息列出拆分建议。
- 用户下一轮明确接受拆分后再重新委派并创建；拒绝拆分时只创建一个总任务。
- 同时收紧管家 delegate_to_role 的 task_summary/context，禁止添加用户未表达的子任务。
- 优点是改动小；风险是模型仍可能误发多个调用，后端会照常写库。

**方案 B：prompt + 后端多调用守卫（中等改动，但需确认令牌设计）。**

- 后端发现 `calls.len()>1` 时零写入并返回 `confirmation_required`。
- 用户确认后必须携带可验证的批次/确认标识，后端才允许批量创建。
- 比纯 prompt 可靠，但会引入跨轮状态、批次身份和协议变化；若没有确认令牌，系统会永远阻止合法批量创建。

**方案 C：持久化任务提案 → 接受拆分/不拆分（推荐，语义完整）。**

- 首轮工具调用只生成一个提案批次，不直接写 tasks。
- 在管家会话显示拆分卡片：接受拆分、保持单任务，也可扩展逐项编辑/拒绝。
- 接受拆分后批量创建；保持单任务则创建一个总任务；所有结果可恢复、可审计。
- 优先复用 suggestion pending/confirm/reject 与 ActionCard，但需增加 deadline、委派来源 conversation_id、批次和原始总任务字段，并修复确认过程的事务一致性。

### Diagnostic

根因不需要新增诊断日志。实施前需补充产品决策：确认是整批级还是逐项级；是否允许编辑标题/deadline；拒绝拆分时总任务标题如何生成；提案多久过期。

## Reproduction Plan

使用同一输入、家庭角色和产品会议冲突数据复现。方案验收必须先观察拆分提案且 tasks 表零新增；选择“接受拆分”后才新增四项，选择“不要拆分”时只新增一个总任务；刷新或重启后 pending 提案仍可继续处理。

## Side Findings

- 四项任务 deadline 均为 `2025-07-16/17`，而创建时间为 `2026-07-16`；日期年份异常是独立风险，暂不纳入本案主线。
- 当前工作区仅新增本调查案卷，业务代码保持在提交 `a67eef9` 状态。
