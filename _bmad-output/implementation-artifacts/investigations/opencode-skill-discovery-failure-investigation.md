# Investigation: opencode Skill 发现失败

## Hand-off Brief

1. **What happened.** UAT 阶段 D 导入的 custom Skill 被写入 opencode project 扫描根目录；阶段 E 发现时，2026-07-19 新增的跨来源同名校验将该内部托管项变成 ValidationError，并中止整个扫描。
2. **Where the case stands.** Root cause confirmed，源码调用链、目录重叠、UAT 顺序和回归提交 `2ec15d2` 相互印证；无需额外诊断日志。
3. **What's needed next.** 在扫描层把跨来源冲突降级为单项跳过，并增加“先导入 custom、再发现 opencode”的意图回归测试；随后可选过滤内部托管路径并改善前端错误展示。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-20 |
| Status | Complete |
| System | Windows；项目工作区 `D:\Windsurf-Project\EgoSync\探索` |
| Evidence sources | 用户报告；`_bmad-output/uat/UAT-Simplified-Manual.md`；待检查源代码、测试、日志与版本历史 |

## Problem Statement

用户报告：测试用例文件 `_bmad-output/uat/UAT-Simplified-Manual.md` 的用例 9、阶段 E 中，单击 opencode 生态 Skill 的“发现”按钮，出现“发现 opencode Skill 失败，请稍后重试”。要求先分析原因和方案，暂不直接执行修复。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户观察 | Partial | 有明确操作与界面错误文案；缺少时间戳、控制台/后端日志、网络或 IPC 错误详情 |
| `_bmad-output/uat/UAT-Simplified-Manual.md` | Available | 阶段 E 定义扫描列表、无效项过滤、导入与持久化预期 |
| 前端源代码 | Available | 两个设置入口均调用 `skillService.discoverOpencode`；失败经 `toFriendlyError` 映射为用户看到的兜底文案 |
| Tauri 命令/服务代码 | Available | 已定位 `skill_discover_opencode`、`opencode_workspace_dir`、`discover_opencode_skills`、`scan_opencode_root` |
| Runtime logs | Missing | 工作区无受版本控制日志；用户未提供本次失败的运行日志 |
| Automated tests | Available | 前端组件有发现/门禁测试；Rust `skill_registry.rs` 有扫描 project/global roots 等测试 |
| Version control | Available | 2026-07-20 的 `546ca14` 同时修改命令、registry 与角色设置 UI，需在因果阶段核对 |
| Static analysis/index | Available | CodeGraph 已定位符号与调用关系，未出现陈旧索引提示 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 从“发现”按钮追踪到错误文案与底层扫描调用 | High | Done | 完整因果链已确认 |
| 2 | 检查 opencode Skill 根目录发现、目录遍历、SKILL.md 解析规则 | High | Done | 根目录职责冲突及条目级错误升级已确认 |
| 3 | 查找运行日志、测试及最近相关提交 | Medium | Done | 回归引入提交已定位；现场日志仍缺失但不阻塞代码诊断 |
| 4 | 形成最小修复方向与验证矩阵 | High | Done | 推荐方案、风险及验收矩阵已完成 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-20（报告时间） | 单击 opencode Skill“发现”后显示通用失败提示 | 用户报告 | Deduced |

## Confirmed Findings

### Finding 1: UAT 明确要求 opencode Skill 扫描返回结构化候选列表

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:715-720`

**Detail:** 阶段 E 要求列表展示 name、description、来源位置与 `sourceType=opencode`，跳过无效项并提供中文摘要，导入/同步状态需准确持久化。

### Finding 2: 前端存在两个相同的 opencode 发现入口，均把异常转换为同一兜底文案

**Evidence:** `egosync-app/src/components/butler/ButlerSettingsContent.tsx:309-314`；`egosync-app/src/components/role/SettingsTab.tsx:387-394`；`egosync-app/src/services/skillService.ts:10`

**Detail:** UI 调用 Tauri 命令 `skill_discover_opencode`；任何未被 `toFriendlyError` 展开为可展示消息的异常都会呈现用户报告的通用错误。因此仅凭 UI 文案无法识别底层失败机制。

### Finding 3: 后端发现命令先执行 find-skills 权限门禁，再扫描两个 opencode 根目录

**Evidence:** `egosync-app/src-tauri/src/commands/skill.rs:65-118`；`egosync-app/src-tauri/src/services/skill_registry.rs:351-397`

**Detail:** 命令使用应用数据目录下的 `opencode-workspace` 作为 project workspace，并使用用户主目录作为 global root；实际扫描由 registry 服务完成。

### Finding 4: 存在自动化测试，但本次失败缺少运行时证据

**Evidence:** `egosync-app/src-tauri/src/services/skill_registry.rs:1336-1471`；`egosync-app/src/components/role/SettingsTab.test.tsx:815-931`；`egosync-app/src/components/butler/ButlerSettingsContent.test.tsx:344-432`

**Detail:** 测试覆盖发现入口和扫描根目录的部分契约；尚未运行测试，也没有用户现场异常栈，不能据此断言实际环境通过。

### Finding 5: 自定义 Skill 的受控存储目录与 opencode“项目级发现”扫描目录是同一个目录

**Evidence:** `egosync-app/src-tauri/src/commands/skill.rs:200-209`；`egosync-app/src-tauri/src/services/skill_registry.rs:638-648`；`egosync-app/src-tauri/src/services/skill_registry.rs:351-378`

**Detail:** 自定义导入把 Skill 写入 `app_data_dir/opencode-workspace/.opencode/skills/<name>/SKILL.md`；发现命令又把 `app_data_dir/opencode-workspace` 作为 project_dir，并扫描其 `.opencode/skills`。因此应用自身托管的 custom Skill 必然重新成为 opencode 发现候选。

### Finding 6: 扫描到 custom 来源的同名托管 Skill 时，会抛错并中止整个发现请求

**Evidence:** `egosync-app/src-tauri/src/services/skill_registry.rs:435-458`；`egosync-app/src-tauri/src/services/skill_registry.rs:743-779`

**Detail:** `scan_opencode_root` 只把读取/解析失败转为“跳过”；对 `preview_from_parsed_with_source` 使用 `?`。该函数发现数据库中同名 Skill 的 `source_type != opencode` 时返回 `ValidationError`，异常向上传播并终止整个扫描。

### Finding 7: UAT 顺序在发现之前明确执行自定义 Skill 导入

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:626-643`

**Detail:** 阶段 D 将 `uat-weekly-report` 导入受控目录，阶段 E 随后点击 opencode 发现。只要阶段 D 成功，阶段 E 扫描就会遇到数据库中 sourceType=custom、磁盘中位于扫描根目录的同名 Skill。

### Finding 8: 回归由 2026-07-19 的提交 `2ec15d2` 引入，而非 2026-07-20 的 `546ca14`

**Evidence:** `git blame`：`egosync-app/src-tauri/src/services/skill_registry.rs:748-756`；commit `2ec15d2c45b7a213195bb52ea28dbae0483013c1`。

**Detail:** project discovery root 与扫描中的 `?` 自 2026-06-10 的 `3537dc1` 已存在，但当时跨来源同名不会在 preview 阶段报错。`2ec15d2` 新增“另一来源占用”的 ValidationError 后，与既有共享目录设计组合成回归。`546ca14` 未改变发现因果链。

## Deduced Conclusions

### Deduction 1: 主要根因是“内部托管目录”与“外部生态发现目录”职责冲突

**Based on:** Finding 5、Finding 6、Finding 7。

**Reasoning:** 阶段 D 导入 custom Skill → 文件写入 project discovery root → 阶段 E 扫描该文件 → registry 已存在同名 custom 条目 → sourceType 冲突返回错误 → `?` 中止扫描 → 前端显示统一失败提示。

**Conclusion:** 该失败不是正常的“没有可发现 Skill”，而是扫描器把应用自己托管的 custom Skill 当作外部 opencode 候选后，错误地将可预期冲突升级成整个请求失败。

### Deduction 2: 前端错误映射掩盖了后端可诊断信息

**Based on:** Finding 2、`egosync-app/src/components/role/SettingsTab.tsx:1052-1059`、`egosync-app/src/components/butler/ButlerSettingsContent.tsx:1237-1243`。

**Reasoning:** 后端序列化的 ValidationError 包含“Skill 名称 … 已被另一来源占用”，但 `toFriendlyError` 只识别包含 `frontmatter` 或 `SKILL.md` 的文本，其他错误全部替换为兜底文案。

**Conclusion:** 错误可观测性不足是次要缺陷；它不是扫描失败的根因，但显著增加定位成本。

## Hypothesized Paths

### Hypothesis 1: “发现”链路中的路径定位、扫描/解析或进程调用失败，被上层统一映射为通用错误

**Status:** Confirmed

**Theory:** 发现扫描读取了应用自身托管在 project `.opencode/skills` 下的 custom Skill；来源冲突异常未经按条目降级处理，导致整个请求失败。

**Supporting indicators:** 界面显示的是泛化重试文案，而非具体路径或解析错误。

**Would confirm:** 找到该错误文案的产生代码及其捕获的底层异常；或运行日志给出具体失败栈。

**Would refute:** 代码证明该文案只对应单一、明确的失败条件，且相关条件与路径/扫描/解析无关。

**Resolution:** 源码链路确认：custom 导入目标与 project 扫描根目录相同，且来源冲突通过 `?` 直接中止扫描。反证检查发现现有 discovery 测试只覆盖纯 opencode 候选和同来源重复项，没有覆盖扫描根目录混入 custom 托管 Skill 的场景。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 浏览器/渲染进程控制台与主进程日志 | 无法确定实际异常类型 | 定位日志文件或在相同环境复现时采集 |
| 机器上的 opencode Skill 实际目录结构 | 无法验证扫描输入是否合法 | 只读盘点预期目录、环境变量和样例 Skill |
| 相关自动化测试结果 | 无法判断既有契约与回归点 | 查找并运行最小范围测试（需用户允许进入验证阶段） |

## Source Code Trace

| Element | Detail |
| --- | --- |
| UI trigger | `egosync-app/src/components/role/SettingsTab.tsx:375-397`；管家入口逻辑相同 |
| IPC boundary | `egosync-app/src/services/skillService.ts:10` 调用 Tauri `skill_discover_opencode` |
| Command gate | `egosync-app/src-tauri/src/commands/skill.rs:77-118` 校验 find-skills 并计算 workspace/home |
| Discovery roots | `egosync-app/src-tauri/src/services/skill_registry.rs:370-387` 扫描 private workspace project root 与用户 global root |
| Self-managed write | `egosync-app/src-tauri/src/commands/skill.rs:200-209`、`skill_registry.rs:638-648` 把 custom Skill 写入同一 project root |
| Error origin | `egosync-app/src-tauri/src/services/skill_registry.rs:748-756` 返回跨来源同名 ValidationError |
| Error propagation | `egosync-app/src-tauri/src/services/skill_registry.rs:444` 的 `?` 将条目冲突升级为整个扫描失败 |
| Error masking | 两处 `toFriendlyError` 仅识别 frontmatter/SKILL.md，其他错误回落为通用提示 |
| Regression commit | `2ec15d2c45b7a213195bb52ea28dbae0483013c1`，2026-07-19 19:17:56 +0800 |
| Impact radius | CodeGraph 显示 `scan_opencode_root` 直接影响 discovery 服务及 3 个现有后端测试，共 6 个符号 |

## Fix Options Analysis

### Option A: 分离内部托管目录与 opencode project discovery root

- **优点：** 从目录职责上彻底隔离。
- **问题：** OpenCode 运行时只从约定的 `.opencode/skills/<name>/SKILL.md` 加载；移动内部托管副本会同时影响运行时加载、导入校验、迁移和删除逻辑，改动半径大。
- **结论：** 不推荐作为本次缺陷的外科手术式修复，可作为后续架构清理议题。

### Option B: 发现时过滤 EgoSync 自身托管的 custom Skill

- **优点：** 符合产品语义；内部 custom Skill 不应被展示为外部 opencode 候选。
- **风险：** 需要可靠比较 canonical managed_path，处理遗留路径、文件缺失和 canonicalize 失败；实现和测试范围中等。
- **结论：** 推荐作为完善方案，但不必阻塞最小修复。

### Option C: 将跨来源名称冲突降级为单项跳过

- **优点：** 最小改动；与现有“坏条目不能阻断全局扫描”的注释和 UAT 跳过摘要契约一致；可以只捕获 `AppError::ValidationError`，继续让 DB 错误显式失败。
- **风险：** 每次扫描可能把内部 custom Skill计入跳过摘要；如果只按错误文本匹配会脆弱，因此必须按错误类型或结构化冲突处理。
- **结论：** 推荐作为第一优先修复。

### Recommended composition

1. **必须：** `scan_opencode_root` 对可预期的跨来源名称冲突执行 `reasons.push(...) + continue`，不再中止请求；数据库和不可恢复错误继续向上传播。
2. **建议：** 识别 `source_path == existing.managed_path` 且 existing.sourceType=custom 的内部托管项，静默过滤或给出专门、去重后的跳过摘要。
3. **次要：** 改善前端错误提取，至少展示后端 ValidationError 的友好消息，但不能用 UI 改善替代后端修复。

## Conclusion

**Confidence:** High

根因已确认：EgoSync 把 custom Skill 受控副本写入 `opencode-workspace/.opencode/skills`，opencode 发现又扫描该目录；commit `2ec15d2` 新增跨来源同名 ValidationError 后，扫描自身托管 custom Skill 会在 `scan_opencode_root` 的 preview `?` 处中止整个请求。前端 `toFriendlyError` 随后丢弃具体 ValidationError，显示通用重试提示。

该结论由静态调用链、相同磁盘路径、UAT 阶段 D→E 的执行顺序、测试覆盖缺口及 git 历史共同确认。未采集现场日志不再影响根因判断，但实现后仍需通过最小自动化复现和原 UAT 顺序验证修复。

## Recommended Next Steps

### Fix direction

1. **P0 / 最小必要修复：** 在 `scan_opencode_root` 中仅将可预期的跨来源名称冲突转换为跳过原因并 `continue`；数据库、I/O 基础设施等不可恢复错误继续返回失败。
2. **P1 / 语义完善：** 当扫描路径与 registry 中 `sourceType=custom` 的 `managedPath` 指向同一文件时，将其识别为 EgoSync 内部托管项并静默过滤，避免每次发现都产生误导性冲突摘要。
3. **P2 / 可观测性：** 两处 `toFriendlyError` 结构化提取 Tauri `ValidationError` 等安全消息；抽取公共实现可作为单独清理项，避免在本次修复中顺手重构。

### Required tests

- 后端新增：内部 custom Skill 位于 project discovery root 时，扫描成功且 global 合法候选仍返回。
- 后端新增：外部 opencode 候选与 custom 同名时，只跳过该候选，不中止后续扫描。
- 后端新增：真实数据库错误仍显式失败，不得被降级为 skip。
- 前端补充：成功结果中同时含 items 与 skipped reasons 时，两者都正确展示。

### Acceptance criteria

- 严格执行 UAT 阶段 D 后立即执行阶段 E，点击“发现”不再显示通用失败。
- `uat-weekly-report` 等内部 custom Skill 不出现在可导入候选中。
- project/global 目录中的合法 opencode Skill 均继续显示 name、description、来源位置和 `sourceType=opencode`。
- 单个冲突或无效项不会阻断其他候选；跳过摘要为友好中文。
- 数据库等系统级失败仍显式报错。
- 重启后导入和启用状态继续保留。

### Diagnostic

无需新增诊断日志。若实施后的最小回归测试与静态结论矛盾，再采集 Tauri command 返回对象和 `scan_opencode_root` 实际 root/path；任何临时诊断日志在定位后必须移除。

## Reproduction Plan

1. 创建临时数据库和临时 project/home 根目录。
2. 向 registry 写入 `sourceType=custom` 的 `uat-weekly-report`。
3. 在 project `.opencode/skills/uat-weekly-report/SKILL.md` 写入对应合法内容，模拟阶段 D 的受控副本。
4. 在 global `.config/opencode/skills/writer/SKILL.md` 写入合法外部候选。
5. 调用 `discover_opencode_skills`。
6. 修复前预期：返回跨来源名称占用 ValidationError，`writer` 无法返回。
7. 修复后预期：调用成功，`uat-weekly-report` 被过滤或跳过，`writer` 正常进入 items。
8. 再按 `_bmad-output/uat/UAT-Simplified-Manual.md:626-643` 执行完整 UI 回归。

## Side Findings

- UAT 数据脚本预置了 `uat-opencode-demo` registry 记录，但 UAT 文档没有明确把真实外部 fixture 复制到被扫描目录；修复后执行阶段 E 前应确认至少存在一个实际磁盘候选，否则结果可能为空而非报错。
- `546ca14` 与故障区域重合但不是引入提交；应避免错误归因。真正引入回归的是 `2ec15d2`。
