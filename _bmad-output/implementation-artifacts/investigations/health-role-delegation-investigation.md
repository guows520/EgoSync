# Investigation: 健康管理角色委派失败

## Hand-off Brief

1. **发生了什么。** UAT 用例 5 阶段 B 中，管家调用 `delegate_to_role` 后收到“未找到当前会话上下文”，健康管理角色委派未完成（用户提供的执行记录，Confirmed）。
2. **当前状态。** 调查已建立强证据锚点，但尚未检查错误产生位置、调用链、运行日志和近期变更，因此根因未定。
3. **下一步。** 定位精确错误字符串及 `delegate_to_role` 的实现与调用链，并对照运行日志确认会话标识在哪一层丢失。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-16 |
| Status           | Active |
| System           | Windows；EgoSync UAT；本地时区 Asia/Shanghai |
| Evidence sources | 用户提供的对话/执行记录；`_bmad-output/uat/UAT-Simplified-Manual.md`（尚未展开）；源代码、运行日志、版本控制（待调查） |

## Problem Statement

用户报告：测试用例 `_bmad-output/uat/UAT-Simplified-Manual.md` 的用例 5“管家委派”阶段 B“多角色并行委派”中，健康管理角色委派失败。执行记录显示工具调用完成提示后返回：“委派失败：未找到当前会话上下文，请稍后重试。”用户要求先分析原因和方案，暂不执行修复。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户提供的对话与执行记录 | Available | 含精确错误字符串与触发语句，是当前强证据锚点 |
| `_bmad-output/uat/UAT-Simplified-Manual.md` | Available | 已给出路径，尚未在本阶段展开读取 |
| 源代码 | Partial | 项目可访问，尚未定位错误源与调用链 |
| EgoSync 运行日志 | Partial | 项目上下文给出默认位置 `%APPDATA%\com.egosync.app\egosync.log`，尚未读取 |
| 版本控制历史 | Available | 尚未检查相关近期变更 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 定位精确错误字符串与 `delegate_to_role` 定义 | High | Open | 建立错误产生位置 |
| 2 | 追踪会话上下文的创建、绑定、查找和清理链路 | High | Open | 确认 session/conversation 标识在哪一层缺失 |
| 3 | 检查对应时段 `delegate_bridge.rs` 诊断日志 | High | Open | 区分未绑定、过早清理、并发覆盖或跨进程不一致 |
| 4 | 核对 UAT 用例前置条件与实际操作步骤 | Medium | Open | 排除测试步骤遗漏或环境不满足 |
| 5 | 检查相关代码近期提交 | Medium | Open | 判断是否为回归 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 2026-07-16（具体时刻未知） | 用户请求同时安排产品工作计划和健身计划 | 用户提供的对话记录 | Confirmed |
| 同一轮对话 | 管家先生成产品计划，并声明把健身计划委派给健康管理角色 | 用户提供的执行记录 | Confirmed |
| 同一轮对话 | `delegate_to_role` 显示已完成，随后业务结果返回“未找到当前会话上下文” | 用户提供的执行记录 | Confirmed |

## Confirmed Findings

### Finding 1: 失败发生在委派工具已被调用之后

**Evidence:** 用户提供的执行记录：“Tooldelegate_to_role 已完成，正在整理结果...”之后紧接“委派失败：未找到当前会话上下文”。

**Detail:** 这排除了“模型完全没有发起委派”的解释；故障位于工具执行路径或工具结果处理路径，而不是委派意图识别阶段。

### Finding 2: 可见失败原因被归类为当前会话上下文缺失

**Evidence:** 用户提供的精确错误：“委派失败：未找到当前会话上下文，请稍后重试。”

**Detail:** 尚未确认该提示对应底层哪一项缺失（session id、conversation id、窗口上下文映射或运行时注册表条目）。

## Deduced Conclusions

### Deduction 1: 角色是否存在不是当前首要根因方向

**Based on:** Finding 2

**Reasoning:** 可见错误明确指向当前会话上下文查找失败，而不是“角色不存在/角色配置缺失”。

**Conclusion:** 首轮调查应优先追踪会话上下文桥接，不应先改健康管理角色配置。

## Hypothesized Paths

### Hypothesis 1: 委派桥接层未能用本次工具调用关联到当前会话

**Status:** Open

**Theory:** 管家对话可正常生成回复，但 `delegate_to_role` 的独立执行路径依赖额外的 session/conversation 映射；该映射未建立、键不一致、已被清理或在并发调用时丢失。

**Supporting indicators:** 精确错误指向“当前会话上下文”；项目上下文说明 `delegate_bridge.rs` 在“请求到达、session 查找、执行、完成/超时”节点有诊断日志。

**Would confirm:** 错误字符串源代码显示其由 session/context 查找失败分支产生，且日志显示本次调用查找键为空或未命中。

**Would refute:** 源代码显示该提示只是对其他错误的通用包装，或日志证明上下文查找成功、失败发生在角色解析/下游执行。

**Resolution:** 待调查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 错误字符串的源代码位置 | 无法确定真正失败分支 | 结构化代码搜索精确错误及相关符号 |
| 失败时的 `delegate_bridge` 日志 | 无法区分映射未建立、键不一致、清理竞态或通用包装错误 | 读取 `%APPDATA%\com.egosync.app\egosync.log` 对应时段 |
| 本次对话的 session/conversation 标识 | 无法端到端关联日志事件 | 从日志或数据库只读查询提取 |
| 相关近期变更 | 无法判断回归来源 | 检查错误源及会话映射文件的 git 历史 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | 待定位 |
| Trigger | 管家调用 `delegate_to_role` |
| Condition | 可见条件为“当前会话上下文未找到”；底层状态待确认 |
| Related files | 初步线索：`delegate_bridge.rs`；其余待结构化追踪 |

## Conclusion

**Confidence:** Low

已确认委派意图被识别且工具路径被调用，失败提示明确指向会话上下文查找。根因目前只能假设位于委派桥接的会话关联链路；没有源代码分支和对应运行日志前，不能判断是上下文未注册、键不一致、生命周期清理竞态还是错误包装。

## Recommended Next Steps

### Fix direction

暂不定修复方向。应先证明上下文在哪个节点丢失，再在最小责任层修复；当前不建议通过提示用户“切换角色”规避，因为这改变了用例要求的管家自动委派语义。

### Diagnostic

1. 定位错误字符串与 `delegate_to_role` 调用链。
2. 对照 `delegate_bridge.rs` 既有诊断日志，关联本次会话标识。
3. 检查上下文映射的注册、读取、清理及并发行为。

## Reproduction Plan

沿用 UAT 用例 5 阶段 B：在管家会话输入“帮我安排今天的产品工作计划和健身计划”，观察是否同时触发产品与健康管理角色委派，并用日志确认每个委派请求是否携带且命中相同的当前会话上下文。

## Side Findings

- 工具宿主中的 `rg.exe` 因 `StandardOutputEncoding` 兼容问题未能启动；持久事实文件改用只读 PowerShell 枚举定位。该限制尚不影响代码图谱调查。

## Follow-up: 2026-07-16

### New Evidence

- UAT 用例明确要求“当前处于管家视角”，阶段 B 也要求“在管家对话中输入”：`_bmad-output/uat/UAT-Simplified-Manual.md:344-360`。
- 实际失败轮次由产品经理角色 agent 执行。日志中的系统指示为“你是用户的『产品经理』分身”：`%APPDATA%/com.egosync.app/egosync.log:4602-4608`。
- 失败请求来自 session `ses_097966355ffepU9mQz4iiVYvyQ`，目标是健康管理角色；后端立即记录 `session_not_found`：`%APPDATA%/com.egosync.app/egosync.log:4641-4643`。
- `delegate_to_role` 只在 `sessions.get(request.session_id)` 未命中时返回该错误：`egosync-app/src-tauri/src/services/delegate_bridge.rs:261-283`。
- 委派 session 仅在 `role_id.is_none()`（管家）且 user message id 非空时注册：`egosync-app/src-tauri/src/services/agent_engine.rs:2167-2172`。角色 agent 不注册是设计行为。
- 角色配置生成逻辑明确禁用 `delegate_to_role`：`egosync-app/src-tauri/src/services/agent_config.rs:344-349`；对应单测也断言角色条目禁用该工具：`egosync-app/src-tauri/src/services/agent_config.rs:1406-1412`。
- 运行时 `opencode.json` 在 2026-07-16 08:52:59（失败前约 8 秒）已对产品经理设置 `delegate_to_role: false`，但该角色 session 仍调用了该工具。配置文件路径：`%APPDATA%/com.egosync.desktop/opencode-workspace/opencode.json`。
- 同一日志中存在多次管家委派成功记录，例如 `egosync.log:4597-4601`；因此 bridge 整体不可用、健康角色不存在、网络不可达均被反证。

### Additional Findings

#### Finding 3: 本次记录没有满足 UAT 用例 5 的视角前置条件

**Evidence:** 用例要求管家视角与管家对话（UAT 文档 348、358 行），实际 prompt 是产品经理角色（日志 4602-4608 行）。

**Grade:** Confirmed。

**Implication:** 该次执行不能直接证明“管家 session 的多角色委派逻辑失败”；它证明阶段 B 实际执行时上下文已经变成产品经理角色。现有证据无法判定该切换由用户操作造成，还是阶段 A 后出现了前端视图/会话路由异常，因此不再归因为单纯测试操作偏差。

#### Finding 4: 直接失败机制是角色 session 没有注册委派上下文

**Evidence:** 注册条件排除 `role_id.is_some()` 的角色流（agent_engine 2167-2172 行），错误分支由 session map 未命中触发（delegate_bridge 261-283 行），日志确认该 session 未命中（4641-4643 行）。

**Grade:** Confirmed。

#### Finding 5: 工具可见性/执行权限与配置声明不一致

**Evidence:** 源码、测试和刚写入的运行时配置都将角色的 `delegate_to_role` 设为 false，但产品经理 agent 实际调用成功到达 bridge。

**Grade:** Confirmed（不一致现象）；其底层原因尚未确定。

### Updated Hypotheses

#### Hypothesis 1: 管家 session 映射丢失

**Status:** Refuted for this incident。

**Resolution:** 失败 session 属于产品经理角色，不是管家；角色 session 按设计不注册委派上下文。

#### Hypothesis 2: opencode 复用旧 session，导致新的工具禁用配置未应用

**Status:** Refuted。

**Would confirm:** 该 session 创建于配置同步之前，且 opencode 只在 session 创建时快照 agent 工具配置。

**Would refute:** 新建产品经理 session 在相同配置下仍能调用禁用工具。

**Resolution:** opencode 数据库显示失败 session `ses_097966355ffepU9mQz4iiVYvyQ` 创建于 2026-07-16 00:52:59Z，agent 为 `role-83a90253-e088-45e3-b2a0-178a6d3c73b5`；这是阶段 B 当时新建的产品经理 session，不是旧 session 复用。

#### Hypothesis 3: opencode 的 agent `tools.{name}=false` 对全局 custom tool 不生效或匹配名不一致

**Status:** Confirmed at EgoSync runtime boundary。

**Would confirm:** 新建角色 session 的可用工具列表仍包含 `delegate_to_role`，或最小化运行时验证证明 false 不阻断该 custom tool。

**Would refute:** 新建 session 正确隐藏该工具，且只有旧 session 可见。

**Resolution:** 新建产品经理 session 在同步后的 `delegate_to_role=false` 配置下仍实际调用该工具并到达 bridge。底层究竟是 opencode 1.15.10 对 custom tool 的配置语义，还是 EgoSync 生成的工具键匹配方式不正确，尚需最小化 opencode 验证；但 EgoSync 当前隔离机制无效已确认。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 定位错误字符串与失败分支 | High | Done | 已确认 session map 未命中分支 |
| 2 | 追踪上下文注册条件 | High | Done | 仅管家流注册，角色流不注册 |
| 3 | 对照失败时日志 | High | Done | 已关联精确 session、角色与时间线 |
| 4 | 核对 UAT 前置条件 | High | Done | 实际在产品经理视角，违反用例前置条件 |
| 5 | 解释禁用工具仍被调用 | High | Open | 区分旧 session 缓存与权限语义/工具名匹配问题 |
| 6 | 检查相关版本历史 | Medium | Partial | 当前禁用逻辑自 2026-06-13 已存在；尚未追踪 opencode session 生命周期变更 |

### Updated Conclusion

当前可把问题拆成两层。第一层已确认：这次并不是管家多角色并行委派，而是产品经理角色会话发起了健康角色委派；角色 session 按设计没有注册 delegate context，所以后端确定性返回 `session_not_found`。第二层仍待解释：产品经理角色本应被 `tools.delegate_to_role=false` 阻止调用，但运行时没有阻止；下一阶段应聚焦 opencode session/配置生命周期与 custom tool 权限生效机制，而不是修改 delegate bridge 的 session 查找。

### Correction after acceptance-criteria clarification

- 预期行为没有争议：阶段 B 必须由管家在同一轮分别委派产品经理和健康角色，再统一转述；`_bmad-output/uat/UAT-Simplified-Manual.md:357-378` 明确规定这一点。
- 实际回复的产品计划是由产品经理 agent 自己生成的，并非管家向产品经理发起的一次委派；随后产品经理试图再委派健康角色，因没有管家 session context 而失败。这解释了为何表面上“产品部分成功、健康部分失败”，但实际上两个目标角色并没有被管家并行委派。
- `ButlerView` 固定传入 `role={null}`：`egosync-app/src/components/butler/ButlerView.tsx:130-143`；`ChatStream` 由该 prop 计算请求 `roleId`：`egosync-app/src/components/chat/ChatStream.tsx:512,998-1003`。后端又直接信任请求携带的 `role_id` 并传入 `run_stream`：`egosync-app/src-tauri/src/commands/chat.rs:256-265,361-382`。
- 因而新增根因候选：阶段 A 完成后，UI 可能进入/保留了产品经理 `ChatStream`，或请求状态携带了产品经理 role；后端缺乏“conversation_id 与 role_id 必须一致且管家界面必须 role_id=None”的数据库校验，允许错误上下文继续执行。

### Source trace completion

- 数据库确认阶段 A 位于管家 conversation `0856e67d-05a2-4728-9e97-eeaa12e71584`（`role_id=NULL`），其 routing metadata 指向产品经理 conversation `c24779ac-363a-480a-9320-92f502273408`。
- 阶段 B 用户输入实际写入后者；该 conversation 的 `role_id=83a90253-e088-45e3-b2a0-178a6d3c73b5`，因此产品经理直接回答产品计划，并尝试把健康任务再次委派出去。
- 源码未发现委派完成后自动调用 `setCurrentView(roleId)` 的路径。`App.tsx:370-402` 只根据 `currentView` 在 ButlerView 与 RoleView 之间择一渲染；现有持久证据无法还原是谁或什么事件改变了 `currentView`。
- 失败 opencode session 是 00:52:59Z 创建的新 session，agent 明确为产品经理，排除了旧 session 沿用旧工具配置。

## Final Conclusion

**Confidence:** High（直接失败机制与工具隔离失效）；Medium（视角切换来源）。

阶段 B 的验收目标没有被执行：阶段 A 在管家 conversation 完成后，阶段 B 输入落入了产品经理 conversation。产品经理因而自行生成产品计划，再调用本应仅管家可用的 `delegate_to_role` 处理健康计划；角色 session 按设计未注册委派上下文，最终确定性返回 `session_not_found`。与此同时，新建角色 session 无视 `delegate_to_role=false` 仍调用全局 custom tool，证明当前配置级工具隔离并不可靠。现有证据不能区分阶段间视角变化是用户点击造成还是未记录的 UI 状态变化；需要一次受控复现或前端诊断事件才能裁决。

### Fix direction

1. **会话归属防线：** 后端根据 `conversation_id` 从数据库解析真实 `role_id`，不直接信任前端组合；发现请求 role 与 conversation 归属不一致时显式失败。
2. **工具授权防线：** `delegate_to_role` 请求增加管家身份/能力凭证校验，不能仅以“session 是否碰巧注册”作为授权；非管家调用返回明确禁止。
3. **工具暴露修复：** 针对 opencode 1.15.10 做最小化验证，确定 custom tool 的实际禁用键/权限配置，并增加运行时集成测试，而不只测试生成 JSON。
4. **UAT 流程验证：** 从明确的管家视角连续执行阶段 A、B，确认 currentView 不变且阶段 B 同一管家 session 产生两次委派。

### Diagnostic positions requiring approval before changes

- `egosync-app/src/components/chat/ChatStream.tsx:970-1003`：发送前记录视图类型、conversation id、role id。
- `egosync-app/src-tauri/src/commands/chat.rs:247-265`：记录 request role id，并对照数据库 conversation role id。
- `egosync-app/src-tauri/src/services/agent_engine.rs:2167-2182`：记录 resolved agent/session、是否注册委派上下文。
- `egosync-app/src-tauri/src/services/delegate_bridge.rs:108-121`：记录 register/unregister 的 session id、context 数量与原因。

### Navigation refutation pass

- 穷举前端 `setCurrentView` / `onViewChange` 后，没有发现 delegate completion 自动切换到目标角色的代码。角色视图切换入口只有侧边栏角色点击、来源导航以及编辑角色等显式交互：`egosync-app/src/components/layout/Sidebar.tsx:65-82`、`egosync-app/src/App.tsx:271-285,353`。
- 因此“管家收到阶段 B query 但只委派健康角色”的叙述被日志和数据库反证：管家没有收到阶段 B query。该 query 被产品经理 conversation 接收，产品部分由产品经理自行回答，只有健康部分触发了一次跨角色工具调用。
- 阶段间为何发生角色视图切换仍缺少 UI 事件证据；当前最可能是显式导航，但必须通过受控复现或临时 currentView 诊断记录确认，不能把可能性写成已确认事实。

### Stage B diagnostic instrumentation

**Scope correction:** 阶段 A 已由用户确认通过，与当前问题无因果关系。后续只调查阶段 B 双角色 query。

经用户授权，已添加统一前缀 `[stage-b-diag]` 的临时 Rust 日志：

- `commands/chat.rs`：记录请求 conversation id、role id。
- `services/agent_engine.rs`：记录最终 session id、role id、conversation id、是否注册委派上下文。
- `services/delegate_bridge.rs`：记录委派上下文 register/unregister 及当前数量。

这些改动不改变业务逻辑。`cargo check` 通过；存在 27 个原有 warning，本次未新增编译错误。复现并得出结论后必须删除所有 `[stage-b-diag]` 日志。
