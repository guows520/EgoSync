# Investigation: Skill 发现与创建后即时加载异常

## Hand-off Brief

1. **What happened.** 元 Skill 开关只改变提示词和前置拦截，角色的原生 `skill` 权限仍被强制禁止；模型因而模拟了发现/创建，并把 `uat-greeting` 写到非受控目录。
2. **Where the case stands.** 根因已确认：能力开关、opencode 权限和 EgoSync 导入事务没有贯通；正式导入链路本身可复用，唯一待实施前验证的是 opencode 1.15.10 的当前 session 热加载能力。
3. **What's needed next.** 按名称级权限白名单 + 受控 `create_skill` bridge 实施，并以“下一条消息可真实调用”为即时加载验收标准。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-19 |
| Status           | Concluded |
| System           | Windows；EgoSync；opencode sidecar；UAT-全开角色 |
| Evidence sources | 阶段 B 对话；UAT 用例；CodeGraph/源码；运行时文件系统；只读数据库查询；OpenCode 官方文档 |

## Problem Statement

用户报告用例 9 的阶段 B 存在两个问题：推荐 Skill 时没有通过 `find-skills` 查找；创建 Skill 后没有直接加载到自定义 Skill。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户提供的阶段 B 对话 | Available | 可确认模型最终回复的外显行为，不能单独证明底层是否发生过工具调用 |
| `_bmad-output/uat/UAT-Simplified-Manual.md` | Available | 用例 9 阶段 B 要求实际调用 find-skills/skill-creator 并显示工具调用反馈 |
| EgoSync 源代码与配置 | Available | CodeGraph 已定位元 Skill 提示、权限、正式导入、角色绑定和 opencode 配置同步链路 |
| UAT 运行时文件系统 | Available | `uat-greeting` 只存在于 `opencode-workspace/skills`；正式受控根 `.opencode/skills` 不存在 |
| EgoSync 数据库 | Available | `skills` 无 `uat-greeting`；UAT-全开角色 `enabledSkillIds` 为空 |
| 版本控制 | Partial | 相关文件近期提交可查；尚未做引入提交归因 |
| 运行时工具调用日志 | Missing | 尚未提供该会话的 opencode 消息/工具调用记录或应用日志 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 核对用例 9 的阶段 B 验收标准 | High | Done | 已确认要求真实工具调用和可见反馈 |
| 2 | 追踪角色可用 Skill 清单与提示词注入 | High | Done | 开启仅写提示词；`skill` 工具仍被强制 deny |
| 3 | 追踪 Skill 创建目标目录与注册/刷新链路 | High | Done | 正式导入链路完整，但对话创建绕过了该链路 |
| 4 | 检查对话/工具调用日志 | Medium | Done | 源码权限与运行时状态已足以证明无真实调用；无需追加诊断日志 |
| 5 | 形成分层修复与验证方案 | High | Done | 已完成，不实施修改 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| UAT 阶段 B | 用户请求搜索并推荐 Skill，角色仅列出 `find-skills` 与 `skill-creator` | 用户提供的对话记录 | Confirmed |
| UAT 阶段 B | 用户要求创建 `uat-greeting`，角色报告写入 workspace skills 目录 | 用户提供的对话记录 | Confirmed |
| UAT 阶段 B | 角色称需要“下次启动或刷新”识别，并提出可手动注入当前会话 | 用户提供的对话记录 | Confirmed |

## Confirmed Findings

### Finding 1: 外显回复没有提供 Skill 搜索执行证据

**Evidence:** 用户在本任务中提供的阶段 B 对话记录。

**Detail:** 回复只依据当前可见 Skill 列表作答，没有展示由 `find-skills` 返回的候选 Skill、来源或安装建议。仅凭最终文本不能断言底层绝对未调用工具，仍需工具调用记录确认。

### Finding 2: 创建完成回复没有证明当前角色已加载新 Skill

**Evidence:** 用户在本任务中提供的阶段 B 对话记录。

**Detail:** 回复明确把识别时点放在“下次启动或刷新”，并将当前会话注入作为后续可选动作，因此与“创建后直接加载到自定义 Skill”的预期不一致。

### Finding 3: 阶段 B 的“实际调用”要求与角色权限实现直接冲突

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:605`、`_bmad-output/uat/UAT-Simplified-Manual.md:685`、`egosync-app/src-tauri/src/services/agent_config.rs:408`、`egosync-app/src-tauri/src/services/agent_config.rs:1067`。

**Detail:** 用例要求实际调用 find-skills 与 skill-creator；但 `parse_permissions` 无条件将 opencode 的 `skill` 工具设为 `deny`，且测试明确断言即使元 Skill 开启也必须为 deny。当前“开启”只影响前置拦截和提示词文案，不授予工具能力。

### Finding 4: 两个元 Skill 已存在，但被角色的 blanket deny 隐藏

**Evidence:** 2026-07-19 对 opencode 官方 Skill 搜索根的补充枚举；OpenCode 官方 Agent Skills 文档。

**Detail:** `~/.agents/skills/find-skills/SKILL.md`、`~/.claude/skills/find-skills/SKILL.md` 和 `~/.claude/skills/skill-creator/SKILL.md` 均存在，opencode 官方文档确认这些是有效全局搜索根。此前仅检查 `.opencode`/`.config/opencode` 的“未部署”判断已被反证。真正阻断点是角色 `permission.skill = deny`；官方文档说明被 deny 的 Skill 会对 Agent 隐藏。

### Finding 5: uat-greeting 绕过了 EgoSync 的正式导入与角色绑定链路

**Evidence:** `egosync-app/src-tauri/src/commands/skill.rs:158`、`egosync-app/src-tauri/src/services/skill_registry.rs:490`；运行时数据库只读查询与文件检查（2026-07-19）。

**Detail:** 正式 `skill_import_custom` 会写入 `.opencode/skills`、登记 `skills` 表、应用角色 scope，并全量同步 `opencode.json`。现场中 `skills` 表没有 `uat-greeting`、UAT-全开角色的 `enabledSkillIds=[]`、`opencode.json` 不含该名称，说明对话创建只写了孤立文件。

## Deduced Conclusions

### Deduction 1: 问题至少跨越两个机制边界

**Based on:** Finding 1、Finding 2。

**Reasoning:** Skill 推荐取决于工具选择/提示词与工具暴露；Skill 创建后可用取决于文件落盘、角色配置登记、运行时缓存刷新和当前会话上下文更新。两者不是同一个故障点。

**Conclusion:** 修复方案应分别处理“发现时强制走 find-skills”和“创建后注册并刷新加载”，不能只改一段回复文案。

## Hypothesized Paths

### Hypothesis 1: 提示词只要求列举可见 Skill，没有把“搜索推荐”路由为必须调用 `find-skills`

**Status:** Confirmed

**Theory:** 角色把请求理解成读取当前 Skill 清单，而不是执行 Skill 发现工作流。

**Supporting indicators:** 回复措辞集中于“当前能看到的可用 Skill”，没有搜索产物。

**Would confirm:** 角色提示词/工具策略中缺少匹配“搜索、推荐 Skill”到 `find-skills` 的硬约束，且会话工具记录无调用。

**Would refute:** 日志显示已调用 `find-skills`，但结果为空或未被正确传回模型。

**Resolution:** `meta_skill_prompt` 只写“已启用”和发现边界，同时 `parse_permissions` 强制禁止 `skill` 工具。元 Skill 文件存在但被隐藏，因此模型没有实际可调用的 find-skills。

### Hypothesis 3: 直接把 skill 权限改成 allow 即可完整修复

**Status:** Refuted

**Theory:** 只需删除 blanket deny，opencode 即可加载所有 Skill，阶段 B 与自定义 Skill 执行都会恢复。

**Supporting indicators:** OpenCode 官方文档确认 `skill: allow` 会让 Agent 看到并加载 Skill。

**Would confirm:** 当前 workspace/global 搜索根只包含该角色允许的 Skill，或权限可以按名称限制。

**Would refute:** 全局搜索根含有大量与当前角色无关的 Skill，blanket allow 会破坏隔离。

**Resolution:** 现场 `~/.claude/skills` 含大量其它 Skill；官方支持 `permission.skill` 按名称/通配符配置。正确方向是默认 deny，只 allow 元 Skill 开关和当前角色 enabledSkillIds 映射出的名称。

### Hypothesis 4: skill-creator 自身规定了错误输出目录

**Status:** Refuted

**Theory:** skill-creator 指示模型写入 `opencode-workspace/skills`，导致绕过正式导入。

**Supporting indicators:** 对话最终确实写入该目录。

**Would confirm:** skill-creator/SKILL.md 明确指定该路径。

**Would refute:** Skill 只要求调用者显式选择输出目录，且不包含 EgoSync 集成规则。

**Resolution:** 完整检查 `skill-creator/SKILL.md` 后确认其不规定固定目录，也不知道 EgoSync 注册、角色绑定或配置同步；错误路径来自宿主编排缺口，而不是该 Skill 的内置规则。

### Hypothesis 2: Skill 创建器只负责写文件，没有调用角色配置同步或运行时刷新

**Status:** Confirmed

**Theory:** 新 Skill 落盘后，当前角色的 Skill 配置与 opencode 会话仍使用创建前快照。

**Supporting indicators:** 回复明确称需下次启动/刷新，并把当前会话注入视为额外动作。

**Would confirm:** 创建链路止于写入 `SKILL.md`；角色配置、opencode 配置或会话上下文没有后续更新调用。

**Would refute:** 源码已在创建后完成注册和热刷新，而仅回复文案错误或刷新失败。

**Resolution:** 现场文件、数据库和 `opencode.json` 三方一致证明 uat-greeting 未进入正式导入链路；正式链路本身已有注册、scope 与配置同步能力。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| opencode 是否会对已运行 session 热重载配置 | 决定“立即加载”能否不新建 session | 核对 sidecar API/现有 session 生命周期或做隔离验证 |

## Source Code Trace

| Element       | Detail |
| ------------- | ------ |
| Error origin  | `agent_config.rs:408` 权限强制 deny；`role_config.rs:142` 仅提示词声明；对话创建绕过 `commands/skill.rs:158` |
| Trigger       | “搜索并推荐几个 Skill”；“直接创建 skill” |
| Condition     | Skill 开启的 UAT-全开角色；当前会话运行中 |
| Related files | `agent_engine.rs`、`agent_config.rs`、`role_config.rs`、`commands/skill.rs`、`skill_registry.rs` |

## Conclusion

**Confidence:** High

当前实现把“已开启”误当成提示词状态，而不是可执行权限：`find-skills`/`skill-creator` 文件已安装，但 `permission.skill=deny` 使它们对角色不可见。模型随后使用普通对话/文件能力模拟创建，绕过 EgoSync 的受控目录、registry、角色绑定和 agent config 同步，所以 `uat-greeting` 不可能被当前角色加载。推荐修复是名称级 Skill 权限白名单和受控创建 bridge；不能用提示词补丁、blanket allow 或目录监听代替事务。

## Recommended Next Steps

### Fix direction

**推荐方案：原生 Skill 调用 + EgoSync 受控事务。**

1. `agent_config.rs`：默认拒绝所有 Skill，只按角色允许开启的两个元 Skill与 `enabledSkillIds` 对应名称；禁止 blanket allow。
2. `role_config.rs`：发现/创建意图必须先加载相应原生 Skill；允许 find-skills 推荐外部候选，但未经确认不得安装或导入。
3. `agent_config.rs` + `delegate_bridge.rs`：新增 `create_skill` 自定义工具和鉴权 bridge endpoint。模型负责起草，后端负责验证、路由、重试边界与确定性持久化。
4. 复用 `skill_registry::import_custom_skill`：写受控目录、登记 registry、绑定并启用当前角色、同步 `opencode.json`；失败必须显式返回且不得宣称成功。
5. `agent_engine.rs` + `ChatStream.tsx`：从 `skill` 工具 input.name 显示具体 Skill，并保留原始工具事件作为真实调用证据。
6. 先验证固定版本热加载；若失败，调用 OpenCode session fork 并替换当前 conversation 的 session 缓存，保留历史且不重启应用。

**产品语义选择：** 对话中通过 skill-creator 新建的 Skill 自动导入并启用到当前角色；设置页导入任意外部 SKILL.md 仍保持“预览→确认→选择 scope/启用”的现有流程。因此不会与用例 9 阶段 D 的手动导入语义冲突。

**否决方案：**

- 只改提示词：权限仍阻断，无法产生真实工具事件。
- `skill=allow`：会暴露用户全局目录中的大量无关 Skill，破坏角色隔离。
- 监听任意目录并自动注册：存在竞态、半成品和错误 scope，无法提供原子成功语义。
- 创建后重启整个应用：破坏体验且不是“立即加载”。
- 仅把完整 SKILL.md 塞入系统提示词：不是原生 Skill 调用，且扩大上下文、绕过权限模型。

### Diagnostic

实施前只需一个隔离兼容性测试：在 opencode 1.15.10 的同一 session 中新增 `.opencode/skills/<name>/SKILL.md` 并更新该 agent 的名称级 permission，下一条消息检查原生 `skill` 工具是否立即出现该名称。成功则不实现 session rollover；失败则验证 `/session/:id/fork` 后新 session 可见并保留历史。当前根因不需要增加诊断日志。

## Reproduction Plan

1. UAT-全开角色请求搜索：必须出现原生 `tool=skill`、`input.name=find-skills`，随后返回真实候选；不能用纯文本模拟。
2. UAT-全关/部分开启角色重复请求：禁止产生对应工具事件，返回友好边界提示。
3. 创建 `uat-greeting`：必须先出现 `input.name=skill-creator`，再出现 `create_skill` bridge 成功事件。
4. 创建成功后检查四个状态：文件位于 `.opencode/skills`；`skills` 表有 registry 行；当前角色 `enabledSkillIds` 含该 ID；agent permission 仅允许该名称。
5. 不重启应用，在同一 UI 对话下一条消息要求生成问候语：必须出现 `tool=skill`、`input.name=uat-greeting` 并给出相关结果。
6. 切换到未绑定角色：Skill 不出现在可用列表且调用被拒绝，证明角色隔离。
7. 注入无效 frontmatter、重名、写盘失败和 config 同步失败：不得留下半注册状态，不得回复“创建完成”。
8. 重启应用后再次调用：注册、绑定与启用状态仍保留。
9. 回归设置页的自定义导入、opencode 发现导入、Skill 删除和管家 Skill scope。

## Side Findings

- 当前对话中“写入文件成功”与“当前角色已可调用”被混为同一完成状态；这是验收口径需要拆开的状态边界。
- 阶段 B 当前只要求 skill-creator 被调用，尚未要求创建后自动绑定；应追加“当前角色自动启用且下一条消息可真实调用”的验收项。
- 阶段 D 的手动导入/启用适用于外部文件，与对话内新建的自动绑定规则属于不同入口，应在用例文本中明确区分。

## Follow-up: 2026-07-19

### New Evidence

- 用例 9 阶段 B 明确要求实际工具调用及可见反馈。
- 角色 Agent 的 `skill` 权限在所有配置下都被强制设为 deny。
- 早期“运行时没有元 Skill 文件”的判断已被后续官方搜索根枚举反证；两个元 Skill 已存在，实际阻断是 permission deny。
- uat-greeting 只存在于错误目录，未登记数据库、未绑定角色、未同步配置。

### Additional Findings

- 现有正式自定义 Skill 导入链路已经覆盖受控复制、注册、角色 scope 与配置同步，可复用，不应另写一套注册逻辑。
- 当前自定义 Skill 注入只包含名称和描述，且仍禁止 skill 工具；所谓“执行”更接近按提示词说明行动，不是加载完整 SKILL.md 后调用。该语义也与用例 D 的“实际调用自定义 Skill”存在一致性风险。

### Updated Hypotheses

- Hypothesis 1：Confirmed。
- Hypothesis 2：Confirmed。

### Backlog Changes

- 完成用例核对、提示/权限追踪、创建/注册/同步链路和运行时状态核对。
- 新增：确定真实 Skill 调用架构及当前 session 热加载语义。

### Updated Conclusion

根因已从“模型没有按预期选择工具”提升为“产品配置模型与执行能力不一致”：UI/数据库声称元 Skill 已启用，但 Agent 权限使已安装的 Skill 对模型不可见；模型只能用普通对话/文件工具模拟。创建后的加载失败是该模拟路径绕过正式导入事务的直接结果。

## Follow-up: 2026-07-19 #2

### New Evidence

- OpenCode 官方文档确认 Skill 通过原生 `skill` 工具按需加载；`.agents/skills`、`.claude/skills` 均是有效搜索根。
- 官方权限支持按 Skill 名称或通配符逐项 allow/deny，并可对每个 Agent 覆盖。
- 当前用户全局目录确有 find-skills 和 skill-creator，同时还有大量其它 Skill。
- `find-skills` 的真实工作流要求先查看 skills.sh，再运行 `npx skills find <query>` 并校验来源质量。
- `skill-creator` 只定义通用创建、验证与打包流程；不包含 EgoSync 导入、注册、角色绑定和热加载协议。

### Additional Findings

- `role_config.rs:161-163` 要求 find-skills 不能列出外部环境 Skill，与 find-skills 本身“搜索开放 Skill 生态”的职责冲突。若继续保留该边界，就不应把功能命名/验收为 Vercel find-skills；若按用户要求真实调用 find-skills，就必须允许推荐外部候选，同时把安装/导入置于用户确认和 EgoSync 受控流程内。
- OpenCode 官方 server 文档公开了逐消息 agent/tools 参数，但没有给出“运行中修改 Skill 文件和 agent 权限后当前 session 必然热重载”的明确保证。即时加载仍需针对项目固定的 opencode 版本做隔离验证。

### Updated Hypotheses

- Hypothesis 3（blanket allow）：Refuted。
- Hypothesis 4（skill-creator 自带错误路径）：Refuted。

### Backlog Changes

- 根因推演与反证完成。
- 下一步：追踪对话接入点和 SSE 工具反馈，形成最小改动方案与验证矩阵。

### Updated Conclusion

当前问题由三层缺口叠加：能力开关只改文案/拦截但权限仍 deny；find-skills 的产品边界与其真实职责冲突；skill-creator 完成后没有 EgoSync 托管事务。推荐架构不是全局放开 Skill，而是按角色生成名称级权限白名单，并让创建流程在真实 skill-creator 调用后进入现有 `import_custom_skill` 事务。即时加载是否需要 session rollover 仍是唯一关键未决项。

## Follow-up: 2026-07-19 #3

### New Evidence

- `chat_send_message` 启动异步流，`run_stream` 进入 `try_run_opencode_stream`；后者先执行元 Skill 关闭拦截，再复用当前 conversation 的 opencode session，并在每轮 POST 时显式传入 agent key（`agent_engine.rs:2205-2352`）。
- opencode 原生工具事件已被统一解析、持久化并推送前端；`skill` 工具的 input 会保存在 rawJson，前端历史轨迹已能提取 `input.name`（`agent_engine.rs:540-609`、`ChatStream.tsx:348-366`）。
- 实时状态只按原始工具名生成，原生调用会显示“正在使用 skill...”，不会显示“正在使用 find-skills...” (`agent_engine.rs:2560-2603`)。
- EgoSync 已有向 opencode 写自定义 TypeScript 工具和通过本地鉴权 bridge 执行业务事务的既有模式（`agent_config.rs:11-47`、`delegate_bridge.rs`）。
- OpenCode server 官方 API 支持 fork 现有 session 并保留历史，为不支持热加载时的定向 session rollover 提供了机制。

### Additional Findings

#### Source trace

| Stage | Location | Current behavior | Required behavior |
| ----- | -------- | ---------------- | ----------------- |
| 元 Skill 开关转权限 | `services/agent_config.rs:408-422` | 无条件 `skill=deny` | 默认 deny；按角色 allow find-skills、skill-creator 与 enabled custom Skill 名称 |
| 元 Skill 行为指令 | `services/role_config.rs:142-165` | 只声明已启用，且禁止外部发现 | 明确要求相关意图先调用原生 skill；find-skills 可推荐外部候选但不得自动安装 |
| 创建后的托管事务 | `commands/skill.rs:158-179`、`services/skill_registry.rs:490-562` | 仅设置页导入会走注册/绑定/同步 | 对话创建复用同一事务，并默认 scope=当前角色、enabled=true |
| opencode 自定义工具 | `services/agent_config.rs:11-47` | 已有 create_role/task 等桥接工具 | 增加 create_skill 工具，将模型生成内容交给 EgoSync 后端验证和导入 |
| 本地业务 bridge | `services/delegate_bridge.rs` | 已鉴权并维护 session→role 映射 | 增加 create-skill endpoint；以后端返回结果作为模型可见的唯一成功依据 |
| 工具事件反馈 | `services/agent_engine.rs:2560-2603` | 实时只显示 skill | 对 skill 工具从 input.name 派生显示名；持久化仍保留原始 tool=skill |
| 历史执行轨迹 | `components/chat/ChatStream.tsx:348-366` | 可解析 Skill 名，但文案为“转换能力” | 改成“加载 {name} Skill”，并覆盖成功/失败状态 |
| session 即时生效 | `services/agent_engine.rs:2252-2289` | 复用旧 session，无显式刷新 | 先验证 1.15.10 热加载；失败时 fork 当前 session 并替换该 conversation 的缓存映射 |

#### Recommended mechanism

1. 角色权限生成名称级白名单：`skill: {"*":"deny", "find-skills":"allow", ...}`；只加入开关已开和当前角色 enabledSkillIds 对应名称。
2. 真实发现流程：提示词要求先 `skill({name:"find-skills"})`，随后按其工作流搜索；推荐可以来自外部生态，但安装/导入不得自动进行。
3. 真实创建流程：先 `skill({name:"skill-creator"})`，生成完整内容后调用新增 `create_skill` bridge tool。
4. bridge 后端验证 frontmatter/name/content，调用现有受控导入逻辑，绑定并启用当前角色，随后同步 agent config；任何一步失败都返回失败，不允许模型宣称已创建。
5. 创建成功后下一条消息必须能实际调用新 Skill。优先依赖并验证热加载；若固定版本不支持，则使用官方 session fork API 保留历史并替换缓存，禁止重启整个应用或丢失对话上下文。

### Updated Hypotheses

- “现有 SSE/UI 无法展示原生 Skill 调用”：Refuted。链路已存在，只缺具体名称的实时映射与真实调用来源。
- “创建后必须重启应用”：Open。源码没有刷新动作，官方也未保证热加载；需固定版本隔离测试后在热加载与 session fork 中二选一。

### Backlog Changes

- 源码调用链、事件链和事务复用点已完成。
- 剩余仅为最终报告：方案取舍、验收矩阵与实施顺序。

### Updated Conclusion

修复可沿既有架构完成，无需另造 Skill 注册系统。核心是把 UI 开关真正映射为 opencode 名称级权限，并新增一个后端受控的 create_skill 事务桥；提示词只负责要求模型先加载 Skill，不承担注册、路由或成功判定。即时加载需以 opencode 1.15.10 的隔离测试决定是否增加 session fork，不应先猜测实现。
