# Investigation: @ Skill 菜单缺项与发送按钮错位

## Hand-off Brief

1. **发生了什么。** 普通已导入 skill 的 `@` 缺项由“作用域启用/绑定”与 ChatStream 不刷新共同造成；`find-skills`、`skill-creator` 是 meta 布尔能力而非 registry 条目，当前数据与发送链路从结构上无法把它们列入 `@`；按钮错位仍是 `b9b75b1` 的独立布局回归。
2. **案件状态。** Concluded after clarification；结论已依据 2026-07-23 的只读生产数据库、源码、测试和 Git 历史修正，前一版关于 meta skill discovery roots 的解释已明确推翻。
3. **下一步需要什么。** 实施 scope-aware picker refresh，并将 picker/resolve 数据模型扩展为 registry + meta 的可选联合类型，同时保持各自授权校验。
## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-23 |
| Status | Concluded |
| System | Windows；React 18 + TypeScript 5.2 + Tauri 2.x + OpenCode sidecar |
| Evidence sources | 用户问题描述；`_bmad-output/project-context.md`；源码；Git 历史；测试；本机目录清单 |

## Problem Statement

用户报告三个现象：用 `@` 选中 skill 后发送按钮错位；`@` 候选项没有 OpenCode 生态 skill；`@` 候选项没有元 skill。目标是先分析原因和修复方案，暂不修改业务代码。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户问题描述 | Partial | 提供现象，缺少截图、复现步骤和运行日志 |
| `_bmad-output/project-context.md` | Available | 确认技术栈、OpenCode sidecar 与项目规则 |
| 前端源码 | Available | 尚未扫描 |
| Rust/Tauri skill 服务源码 | Available | 尚未扫描 |
| Git 历史 | Available | 尚未扫描 |
| 运行截图/DOM 尺寸 | Missing | 用于确认按钮错位的具体布局条件 |
| skill 实际目录清单与接口响应 | Missing | 用于运行态验证菜单缺项 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | `@` 菜单入口、skill 数据源、聚合与过滤 | High | Open | 查明 OpenCode skill 与元 skill 是否被发现或被过滤 |
| 2 | skill 选中后输入附件/mention chip 与发送按钮布局 | High | Open | 查明容器尺寸、定位和换行条件 |
| 3 | Rust/Tauri skill 枚举接口及目录范围 | High | Open | 查明后端是否只返回部分生态目录 |
| 4 | 相关 Git 变更与测试覆盖 | Medium | Open | 确认是否为近期回归及缺失验收条件 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-23 | 用户报告三个 `@` skill 选择相关问题 | 当前任务 | Confirmed |
| 2026-07-23 | 调查工作流使用 `python` 成功激活 | 命令输出 | Confirmed |
| 2026-07-23 | 加载项目上下文，确认项目集成 OpenCode sidecar 和 SKILL.md 技能系统 | `_bmad-output/project-context.md` | Confirmed |

## Confirmed Findings

### Finding 1: 项目设计包含 OpenCode SKILL.md 技能系统

**Evidence:** `_bmad-output/project-context.md`

**Detail:** 项目上下文明确说明 Agent 引擎采用 OpenCode sidecar，并依赖其 SKILL.md 技能系统；因此“OpenCode 生态 skill 未显示”属于与设计目标相关的缺口，而非明显超出系统边界的请求。

## Deduced Conclusions

暂无；需源码取证后建立。

## Hypothesized Paths

### Hypothesis 1: skill 候选的数据源范围不完整

**Status:** Open

**Theory:** `@` 菜单可能只消费 EgoSync 自有 skill 列表，未聚合 OpenCode 兼容目录或元 skill 目录。

**Supporting indicators:** 两类缺项同时发生，可能共享同一发现/过滤链路。

**Would confirm:** 数据源或后端枚举函数明确限制目录、来源类型或名称。

**Would refute:** 接口已返回两类 skill，但前端渲染层丢弃。

**Resolution:** 待查。

### Hypothesis 2: 前端过滤条件排除了 OpenCode skill 与元 skill

**Status:** Open

**Theory:** 数据已经返回，但类型白名单、可见性、来源或路径过滤将其排除。

**Would confirm:** 接口响应模型包含目标项，而 selector 的 filter/map 不保留。

**Would refute:** 后端从未返回目标项。

**Resolution:** 待查。

### Hypothesis 3: mention chip 改变输入区高度但发送按钮仍按旧坐标定位

**Status:** Open

**Theory:** 选中 skill 后新增 chip/附件行，父容器换行或增高，而按钮使用 absolute、固定高度或不匹配的 flex 对齐。

**Would confirm:** 相关 JSX/Tailwind 布局存在固定定位、固定高度或 chip 与 textarea 分属不协调容器。

**Would refute:** 按钮完全处于自适应 flex 流且运行态尺寸无异常。

**Resolution:** 待查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 精确复现截图与窗口尺寸 | 区分水平偏移、垂直偏移、遮挡或换行 | 运行应用并截图，或由用户提供 |
| 候选接口响应 | 区分后端发现缺失与前端过滤缺失 | 检查调用链；必要时仅提出诊断日志点 |
| 本机 skill 目录样本 | 判断 OpenCode/元 skill 的实际路径和元数据差异 | 检查配置与目录枚举实现 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | 待定位 |
| Trigger | 输入 `@` 并选择 skill |
| Condition | 选中 skill 后；或候选数据源中存在特定生态/元 skill 时 |
| Related files | 待 CodeGraph 扫描 |

## Conclusion

**Confidence:** Low

当前仅确认问题与项目设计相关，尚无源码证据支持具体根因。最优先调查共享的 skill 发现/过滤链路，再独立调查选中后的布局变化。

## Recommended Next Steps

### Fix direction

待源码取证后给出；不在本阶段实施。

### Diagnostic

优先进行 CodeGraph 调用链分析、相关文件源码阅读及 Git 变更检查；若静态证据仍不足，再列出最小诊断日志或 DOM 测量点供用户确认。

## Reproduction Plan

1. 打开聊天输入区并输入 `@`。
2. 检查候选列表是否包含 OpenCode 生态 skill 与元 skill。
3. 选中任一 skill，观察发送按钮相对输入框、mention chip 的位置变化。
4. 分别在常规宽度和窄窗口下重复。

## Side Findings

- CodeGraph 的 `**/project-context.md` 查询未命中该输出文件，后由用户提供明确路径直接读取；这不代表文件不存在。

## Follow-up: 2026-07-23

### New Evidence — Outcome 2 Evidence Perimeter

| Category | Status | Evidence / Notes |
| --- | --- | --- |
| Source code | Available | 前端入口已定位到 `egosync-app/src/components/chat/ChatInput.tsx` 与 `ChatStream.tsx`；skill IPC 位于 `src/services/skillService.ts`；后端入口位于 `src-tauri/src/commands/skill.rs`，模型位于 `src-tauri/src/models/skill.rs`。 |
| Tests | Partial | 存在 `ChatInput.test.tsx` 与 `ChatInput.a11y.test.tsx`；尚未确认是否覆盖 skill 缺项和选中后布局。 |
| Version control | Available | 相关路径近期提交包括 `546ca14`（2026-07-20，skill imports 后刷新 runtime）与 `2ec15d2`（2026-07-19，执行和热加载 role skills）；需在因果阶段检查 diff。 |
| Static analysis | Available, not run | CodeGraph 索引可用；尚未运行编译/测试，因为当前阶段只做证据映射。 |
| Runtime diagnostics | Missing | 未提供截图、DOM 尺寸、接口响应或应用日志。 |
| Diagnostic archives | Missing | 无诊断包。 |
| Issue tracker | Missing | 未提供 ticket 或外部问题记录。 |

### Additional Findings

1. **Confirmed:** `ChatInput` 接收 `availableSkills`，本地过滤只按名称前缀匹配；候选完整性首先取决于上游传入的数据。证据：`egosync-app/src/components/chat/ChatInput.tsx:13-15,25-44`。
2. **Confirmed:** 选中 skill 会设置 selection、关闭 picker、清除 query 并删除输入中的 `@` 触发文本。证据：`egosync-app/src/components/chat/ChatInput.tsx:50-64`。
3. **Confirmed:** `ChatStream` 是 `skillService` 与 `ChatInput` 之间的关联入口（CodeGraph relationship: `ChatStream → skillService`）。
4. **Confirmed:** 当前源码树包含两类直接相关测试：`ChatInput.test.tsx`、`ChatInput.a11y.test.tsx`。
5. **Confirmed:** 工作区除本调查案件文件外无其他未提交改动；调查未污染业务源码。

### Updated Hypotheses

- **H1 数据源范围不完整：Open。** 已确认 `ChatInput` 不负责生态目录发现，调查重点上移到 `ChatStream → skillService → Rust skill command`。
- **H2 前端过滤排除来源类型：弱化但仍 Open。** `ChatInput` 自身只做名称前缀过滤，没有看到来源类型过滤；仍需检查 `ChatStream` 在传参前是否过滤。
- **H3 选中后父容器高度变化与绝对定位冲突：Open。** 已定位 `ChatInput`，但 Outcome 2 尚未完整读取 JSX 布局区，需在源码追踪阶段确认。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| --- | --- | --- | --- | --- |
| 1 | `ChatStream → skillService → commands/skill.rs` 数据流 | High | In Progress | 已定位入口，下一阶段读取实现并追踪来源 |
| 2 | `ChatInput` JSX/Tailwind 布局与 selection UI | High | In Progress | 已定位组件，下一阶段检查布局条件 |
| 3 | `546ca14`、`2ec15d2` diff | High | Open | 判断两项 skill 相关提交是否引入/暴露问题 |
| 4 | `ChatInput` 相关测试意图 | Medium | Open | 判断三个现象是否缺少回归测试 |
| 5 | 运行态截图/响应样本 | Medium | Blocked | 静态证据不足时再请求或提出诊断点 |

### Updated Conclusion

证据范围已经足以进入因果推理和源码追踪。两个“候选缺项”问题很可能共享上游 registry/枚举链路；发送按钮错位属于同一组件内的独立布局机制。此时不应把三者强行归为一个根因。

## Follow-up: 2026-07-23 #2

### New Evidence — Outcome 3 Cause Reasoning

1. `ChatStream` loads `skillService.listEnabledForScope(roleId)` and passes the result unchanged to `ChatInput` (`ChatStream.tsx:987-1007,1243-1245`).
2. Rust `list_enabled` deliberately returns only IDs enabled for the Butler scope, or the intersection of role-bound and role-enabled IDs (`skill_registry.rs:96-137`). It is an authorization boundary, not ecosystem discovery.
3. OpenCode discovery is a separate command and currently scans only project `.opencode/skills` and global `~/.config/opencode/skills` (`skill_registry.rs:430-466`).
4. Current machine evidence: project `.opencode/skills` is absent; project `.claude/skills` and `.agents/skills` each contain 50 entries; global `~/.config/opencode/skills` has 1, global `~/.claude/skills` has 163, and global `~/.agents/skills` has 1.
5. Current machine contains relevant skill collections under project/global `.claude/skills` and `.agents/skills`, while EgoSync discovery scans neither category; this is sufficient local evidence for the reported omission.
6. `ChatInput` inserts the selected-skill chip before the input inside the same `relative` container (`ChatInput.tsx:134-169`), while send/stop buttons are positioned `absolute ... top-1/2` relative to that whole container (`ChatInput.tsx:215-236`). The chip therefore changes the reference height only after selection.

### Hypothesis Resolution

#### H1: `@` 菜单直接展示 OpenCode 生态中的全部 skill

**Status:** Refuted as current implementation behavior; Confirmed as a requirement gap.

**Resolution:** `@` only consumes the enabled EgoSync registry. OpenCode discovery candidates are neither queried nor merged in the chat path. Even the one skill under `~/.config/opencode/skills` will not appear until represented in the registry and enabled for the current scope.

#### H2: 元 skill 被前端名称/来源过滤

**Status:** Refuted at `ChatInput`; Confirmed root cause upstream if “元 skill” means `.agents/skills`/`.claude/skills` entries.

**Resolution:** `ChatInput` only applies a name-prefix filter. The backend discovery omits four compatible roots, including both project and global `.agents/skills` and `.claude/skills`; this deterministically excludes the observed 50/163-entry collections before frontend rendering.

#### H3: 选中 skill 后 chip 使发送按钮错位

**Status:** Confirmed by deterministic layout mechanism.

**Resolution:** Without selection, the relative container height is essentially the input height, so `top-1/2` centers the button on the input. With selection, the chip and bottom margin increase container height above the input, but the button remains centered on the combined chip+input container, shifting it upward relative to the input. No runtime log is required to establish this CSS geometry; a screenshot is only needed to quantify pixels.

### Refutation Pass

- Checked whether `ChatInput` filters by source/type: it does not; only `name.startsWith(query)` is present (`ChatInput.tsx:40-44`).
- Checked whether chat calls ecosystem discovery: it does not; it calls only `listEnabledForScope` (`ChatStream.tsx:992-996`).
- Checked whether button positioning is relative to a nested input-only wrapper: it is not; chip, input, picker, and button share the outer `relative` container (`ChatInput.tsx:134-237`).
- Remaining ambiguity: “元 skill” is not a code-level type in the examined model. The conclusion assumes it refers to skills stored under `.agents/skills`/`.claude/skills`, which matches the machine inventory. If it means a different category, that taxonomy must be supplied.
- External documentation search returned no usable citable result and is not relied upon for this conclusion.

### Deduced Conclusions

1. The two missing-list symptoms share an upstream mechanism but represent two policy layers: discovery coverage and scope authorization.
2. Simply merging every discovered skill into `@` would bypass the explicit backend authorization boundary documented in `list_enabled`; this conflicts with current architecture.
3. The safer fix direction is to expand discovery to all OpenCode-compatible roots, then retain an explicit import/register/enable step before a skill becomes executable from `@`. If product intent is “discover and select in one interaction,” the picker must visually distinguish unavailable candidates and perform import/enable with confirmation rather than silently bypass authorization.
4. The layout fix is independent: establish an input-only relative wrapper for the input and button; render the selected chip outside that wrapper. Avoid compensating pixel offsets because chip height may vary.

### Updated Conclusion

**Confidence:** High for the button root cause and omitted compatible roots; Medium for the intended `@` product behavior because current architecture intentionally shows only enabled registry entries.



## Follow-up: 2026-07-23 #3

### New Evidence — Outcome 4 Source Trace

#### Timeline / regression classification

| Date | Commit | Finding | Grade |
| --- | --- | --- | --- |
| 2026-06-10 | `3537dc1 feat(skill): import opencode skills across scopes` | Introduced discovery limited to project `.opencode/skills` and global `~/.config/opencode/skills`; test explicitly names and locks “only project and global skill roots.” | Confirmed |
| 2026-07-19 | `2ec15d2` | Changed role-skill execution/hot-load but did not introduce discovery root limitation. | Confirmed |
| 2026-07-20 | `546ca14` | Added runtime refresh after imports but did not introduce discovery root limitation. | Confirmed |
| 2026-07-21 | `dcd6ff7` | Improved discovery resilience while retaining the same two roots. | Confirmed |
| 2026-07-23 | `b9b75b1` | Added selected-skill chip and `@` picker while retaining the pre-existing send-button absolute positioning relative to the outer container. This introduced the deterministic layout regression. | Confirmed |

#### Source Code Trace

| Symptom | Error origin | Trigger | Producing condition | Related files |
| --- | --- | --- | --- | --- |
| 选中 skill 后发送按钮错位 | `ChatInput.tsx:134-169,215-236` | 在 `@` 菜单确认 skill | chip 被插入 outer `relative` container；按钮仍按 outer `top-1/2` 定位 | `ChatInput.tsx`, `ChatInput.test.tsx` |
| `@` 不显示 OpenCode 生态候选 | `ChatStream.tsx:987-1007` → `skillService.ts:8` → `skill_registry.rs:96-137` | 进入/切换聊天角色 | chat 只请求当前 scope 已启用 registry，未调用 discovery | `ChatStream.tsx`, `skillService.ts`, `commands/skill.rs`, `skill_registry.rs` |
| `@` 不显示元 skill | `skill_registry.rs:430-466` | 执行 OpenCode discovery/import 流程 | discovery 只遍历两个 `.opencode` roots，不遍历本机存在的 `.agents/skills`、`.claude/skills` | `skill_registry.rs` 及其内联 Rust tests |

### Test Intent Audit

1. `ChatInput.test.tsx:23-31` 明确将 AC-1 定义为“仅包含当前 Agent 已添加且启用的 Skill”。因此当前 `@` 数据策略不是偶然实现，而是被测试固化的旧/现行产品契约。
2. `ChatInput.test.tsx:81-94` 只验证 chip 出现和可删除，没有验证发送按钮仍与 input 对齐。测试验证了 WHAT，但遗漏了 chip 不应破坏输入区布局这一 WHY。
3. `ChatStream.test.tsx:2090-2130` 验证调用 `listEnabledForScope` 以及发送时携带 `selectedSkillId`，进一步确认 chat path 不负责 discovery。
4. Rust 测试 `skill_registry.rs:1439-1473` 的名称是 `discover_opencode_skills_scans_only_project_and_global_skill_roots`，明确把两-root 范围当作预期行为；若现在要求支持 `.agents/.claude`，必须先更新产品契约和该测试，而不是在不改测试意图的情况下偷偷扩展。

### Final Root-Cause Classification

- **发送按钮错位：近期回归。** 由 `b9b75b1` 在 2026-07-23 添加 chip 时未重建按钮定位上下文导致。
- **OpenCode 生态 skill 不显示：产品契约/入口错配。** `@` 被设计和测试为 enabled-registry selector，而用户把它理解为 ecosystem discovery selector。
- **元 skill 不显示：旧 discovery 范围不足。** 该限制自 `3537dc1`（2026-06-10）存在且被测试固化；属于需求演进后的设计缺口，不是近期代码回归。

### Fix Direction — Diagnosis Only

1. **Layout mechanism:** chip 保持在外层；新增 input-only `relative` wrapper，input、picker、send/stop button 放入 wrapper。禁止像素补偿。
2. **Discovery mechanism:** 把 discovery roots 抽成确定性路径列表，加入所需 `.agents/skills`/`.claude/skills` 项目与用户目录；逐 root 标记 source location，并保持当前无效项/冲突项报告机制。
3. **Authorization/product mechanism:** 保持 `listEnabledForScope` 为执行授权边界。若 `@` 需要展示未启用生态候选，UI 必须区分“可执行”和“可添加”，后者通过显式导入/绑定/启用动作进入 registry。
4. **Tests:** 添加布局结构/定位上下文测试；更新 discovery root 测试；增加 `@` 中 enabled 与 discoverable 两类候选的产品契约测试。不要用 jsdom 像素值作为唯一布局断言，可优先断言按钮属于 input-only relative wrapper，并补一条浏览器级视觉回归。

### Missing Evidence Remaining

- “元 skill”的正式产品定义仍缺失；当前按 `.agents/.claude` 下的上层技能解释。
- 若产品坚持 `@` 直接展示全部生态候选，需要明确点击未启用项时的授权、冲突和失败交互。
- 实际错位像素未测量，但不影响根因确认，仅影响视觉验收基线。

### Updated Conclusion

**Confidence:** High for all three implementation mechanisms. Product修复方案的唯一剩余决策是：`@` 是否只展示可执行 skill，还是同时承担生态发现与导入入口。

## Follow-up: 2026-07-23 #4 — Finalization

### Final Conclusion

**Status:** Concluded
**Confidence:** High（实现根因）；Medium（`@` 最终产品交互，取决于产品选择）

1. **发送按钮错位是近期回归。** 2026-07-23 的 `b9b75b1` 把 chip 放进按钮的 outer positioning context，却保留旧的 `top-1/2`；布局机制可确定复现。
2. **OpenCode 生态 skill 缺项是入口契约错配。** chat path 只调用 `listEnabledForScope`，且测试明确要求仅列已添加并启用的 skill；它从未被设计为生态发现入口。
3. **元 skill 缺项是 discovery 范围不足。** 自 2026-06-10 的 `3537dc1` 起，只扫描项目 `.opencode/skills` 与用户 `~/.config/opencode/skills`，遗漏本机存在的项目/用户 `.agents/skills`、`.claude/skills`，并由旧测试固化。

### Recommended Implementation Boundary

#### Required

- `ChatInput.tsx`：建立 input-only `relative` wrapper，按钮与 picker 相对该 wrapper 定位，chip 留在 wrapper 外。
- `ChatInput.test.tsx`：增加结构回归测试，表达“chip 不得改变按钮相对 input 的定位上下文”。
- `skill_registry.rs`：把 discovery roots 建模为确定性列表，扩展到已确认需要支持的 `.agents/.claude` 项目与用户目录。
- Rust tests：将旧的 two-root 契约更新为明确的多-root 覆盖、去重、无效项和同名冲突行为。

#### Product-dependent

- 若 `@` 只用于执行：保持 `listEnabledForScope`，在 skill 管理入口改善 discovery/import/enable 流程。
- 若 `@` 同时用于发现：增加“当前可用/可添加”分组；未启用候选必须显式导入、绑定、启用后才能执行。

#### Explicitly out of scope

- 不在 `ChatInput` 直接扫描文件系统。
- 不把未授权候选直接合并成可执行 skill。
- 不用固定 margin/top 像素补偿按钮。
- 不顺手重构 skill registry、角色配置或 OpenCode runtime refresh 的无关逻辑。

### Verification Plan

1. **Frontend unit:** 无 skill、picker 打开、选择 chip、删除 chip、发送后清空 selection 均保持现有行为。
2. **Layout intent:** 断言 send/stop buttons 与 input 处于同一 input-only relative wrapper；chip 在其外。
3. **Browser visual:** 常规宽度、窄窗口、长 skill 名称、深浅色、streaming/非-streaming 下，按钮中心均与 input 中心对齐。
4. **Rust discovery:** 临时目录分别构造 project/global `.opencode`、`.agents`、`.claude` roots，验证全部发现、来源标记、非递归规则、无效 SKILL.md、同名冲突与不可读目录。
5. **Authorization:** 未启用 skill 不可直接执行；导入并启用后才进入 `listEnabledForScope`。
6. **Integration:** 角色切换重新加载正确 scope；发送请求携带正确 `selectedSkillId`；导入后的 runtime refresh 行为不回归。
7. **Project verification order:** `cargo test`（skill registry 相关）→ `npm test`/目标 Vitest → `cargo check` → `npx tauri dev` 手工验证。完整 Agent UAT 需要存在 `resources/opencode.exe`。

### Recommended Next Action

使用 `bmad-quick-dev` 执行最小修复；在开发前先确认 `@` 采用以下产品策略：推荐“可发现但不可越权执行”，即 UI 采用两组候选，后端继续以 enabled registry 作为执行授权边界。


## Follow-up: 2026-07-23 #5 — User Clarification and Corrected Diagnosis

### Clarification

用户明确：

1. “OpenCode 生态 skill”指**已经发现、导入并启用**的普通 skill，而非未导入生态候选。
2. “元 skill”专指 `find-skills` 与 `skill-creator`，两者已在某个作用域启用。

这推翻了前一版“主要因为 discovery roots 未覆盖 `.agents/.claude`”对当前两个缺项现象的解释。该目录覆盖仍是旁支设计缺口，但不是本次澄清后的直接根因。

### Read-only Runtime Database Evidence

数据库：`C:\Users\Admin\AppData\Roaming\com.egosync.desktop\egosync.db`，以 SQLite `mode=ro` 打开，未写入。

- Registry 中存在普通 skill：`pdf`（opencode）、`uat-greeting`（custom）。
- `pdf` 仅绑定并启用于角色 `UAT-全关角色`。
- `uat-greeting` 仅绑定并启用于角色 `UAT-全开角色`。
- Butler 的 `butler.skills_config` 为：`findSkills=true`、`skillCreator=false`、`enabledSkillIds=[]`。
- `UAT-全开角色` 为：`findSkills=true`、`skillCreator=true`、`enabledSkillIds=[uat-greeting id]`。

**Confirmed:** Skill 启用是作用域级而非全局。当前数据库没有任何普通 registry skill 对 Butler 启用；普通 skill 只会在与其绑定且启用的对应角色中通过 `list_enabled` 返回。

### Corrected Root Cause 1 — Imported/Enabled Registry Skill Missing

**Status:** Confirmed combined mechanism.

1. `ChatStream` 只在 `[roleId]` 变化时调用 `listEnabledForScope`（`ChatStream.tsx:987-1007`）。
2. 角色设置保存后会更新同一 role 的 `skillsConfig`，但 role ID 不变（`SettingsTab.tsx:301-315`）。`RoleView` 同时保持 ChatStream 挂载（`RoleView.tsx:81-110`）。因此当前候选数组不会重新加载。
3. Butler 设置是 ChatStream 的 sibling panel；保存 `butler.skills_config` 后 ChatStream 同样保持挂载且 `roleId` 始终为 `undefined`（`ButlerView.tsx:112-145`、`ButlerSettingsContent.tsx:277-305`）。
4. 已有 `skill-registry-updated` 事件只被 App/SettingsTab 监听，ChatStream 不监听；且源码中事件仅由 delegate bridge 的创建流程发出，设置页普通 toggle 不会形成 ChatStream refresh 信号。

**Conclusion:** 即使数据库中的绑定与 enabled IDs 正确，用户在同一个已挂载聊天作用域中启用/import 后，`availableSkills` 仍保留旧快照，直到切换角色、重新挂载或重启。若在其他作用域测试，则还会叠加“启用不是全局”的作用域差异。

### Corrected Root Cause 2 — `find-skills` / `skill-creator` Missing

**Status:** Confirmed structural exclusion.

1. 两个 meta skill 存储在 `roles.skills_config` / `butler.skills_config` 的布尔字段中：`meta.findSkills`、`meta.skillCreator`，并兼容 legacy `find-skills`、`skill-creator` keys（`role_config.rs`）。
2. 它们不是 `skills` 表记录，也没有 registry UUID。
3. `skill_list_enabled_for_scope` 返回类型是 `Vec<SkillRegistryEntry>`，实现只从 `skills` 表按 enabled IDs 过滤（`skill_registry.rs:96-137`）。因此 meta skill 无法进入 `availableSkills`。
4. 发送链路收到 `selectedSkillId` 后调用 `resolve_enabled`，要求该 ID 是 registry entry；随后 Agent Engine 取 registry entry name 调用 OpenCode `send_command`（`chat.rs:222-234`、`agent_engine.rs:2472-2515`）。所以只在前端伪造两个 option 也会在发送前校验失败。

**Conclusion:** “启用了但 `@` 不显示”不是刷新问题，而是 picker DTO、查询 API 和发送 resolver 都只支持 registry skill，完全未建模 meta skill。

### Scope Contradiction Exposed

用户陈述“两个 meta skill 已启用”与当前 Butler 数据并不完全一致：2026-07-23 只读数据库显示 Butler 为 `findSkills=true`、`skillCreator=false`；`UAT-全开角色`才是两者都为 true。因此必须按正在聊天的具体作用域判断，不能把某角色启用理解为全局启用。

### Corrected Fix Direction

#### A. Refresh already-imported registry skills

- 新增语义明确的 `skill-scope-updated` 通知，payload 至少包含 scope kind/owner ID/revision。
- 角色与 Butler 的 skill 配置保存、导入、删除、绑定变化成功后统一触发。
- `ChatStream` 抽取稳定的 `reloadAvailableSkills`，在 role ID 初始/切换和匹配 scope 的事件到达时调用。
- 角色侧可把 `role.skillsConfig` 加入依赖作为即时补强，但不能只靠它，因为 Butler 没有对应 role prop，且导入/删除也需要统一通知。
- refresh 后若 selected ID 已失效，清空 selection。

#### B. Model meta skills end-to-end

不要把 `find-skills`/`skill-creator` 伪装写入 `skills` registry 表。推荐新增选择 DTO：

```text
SelectableSkill
- key: registry:<uuid> | meta:find-skills | meta:skill-creator
- name
- description
- kind: registry | meta
- enabled
- sourceType
```

- 新 API `skill_list_selectable_for_scope`：组合 `list_enabled` 的 registry entries 与当前 scope 已启用的 meta booleans。
- 新 resolver 按 kind 校验：registry 继续验证绑定 + enabled ID；meta 验证对应配置布尔值。
- resolver 输出统一的 command name，Agent Engine 继续通过 OpenCode `send_command(name, arguments)` 显式执行。
- 前端 picker 使用 `key` 而不是假设所有项都有 registry UUID。

#### C. Required Tests

1. 同 role ID 但 `skillsConfig` 变化时，ChatStream 重新获取候选。
2. Butler 配置变化事件触发候选刷新。
3. 非当前 scope 的更新不触发 reload。
4. refresh 后已失效 selection 被清空。
5. 当前 scope 启用 `find-skills`/`skill-creator` 时列表包含对应 meta option；关闭时不包含。
6. 伪造/跨 scope meta key 在 command gate 被拒绝。
7. registry skill 与 meta skill 都能走显式 OpenCode command；普通消息不受影响。

### Superseded Findings

- **Refuted:** “`find-skills`/`skill-creator` 没显示的直接原因是 discovery 没扫描 `.agents/.claude`。”它们已有独立 meta 配置与运行时权限链路，本次缺项发生在 picker 数据模型。
- **Refined:** “OpenCode skill 没显示是 enabled-registry 产品契约。”用户指的正是已启用 registry skill；直接问题是 scope 配置与前端缓存未刷新，而非是否应该展示未导入候选。

### Revised Final Conclusion

**Confidence:** High.

- 普通已导入 skill：先确认当前聊天 scope；匹配 scope 时仍不显示的确定性缺陷是 ChatStream 候选快照不随 skill 配置变化刷新。
- `find-skills`/`skill-creator`：当前 picker/query/send 三层均只支持 registry entry，meta skill 被结构性排除。
- 按钮错位结论不变，仍为独立布局回归。
