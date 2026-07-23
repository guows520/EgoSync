# Investigation: 设置页面、任务弹窗、Markdown、深色模式与默认语言问题

## Hand-off Brief

1. **What happened.** 用户报告了 8 组前端体验与打包默认行为问题，涉及管家/角色设置、任务时间选择、角色创建、Markdown 图表渲染、术语、深色模式和默认语言；当前尚未完成源代码归因。
2. **Where the case stands.** Active；调查已完成激活与范围定义，源码、版本控制及既有实现文档可作为证据，尚未开始逐项追踪。
3. **What's needed next.** 先建立受影响页面和共享组件的证据边界，再沿事件与渲染链路确认根因，避免把视觉症状误判为单一组件问题。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A                                                                        |
| Date opened      | 2026-07-22                                                                 |
| Status           | Active                                                                     |
| System           | Windows；React 18 + TypeScript + Vite + TailwindCSS；Tauri 2 计划/桌面应用上下文 |
| Evidence sources | 源代码、配置、既有实现文档、版本控制；尚未收集运行时截图/日志/复现录屏 |

## Problem Statement

用户要求先分析原因和方案，暂不直接执行，提出以下问题：管家设置和角色社会设置分组折叠/全部折叠展开及若干文案/保存位置调整；创建任务选择日期时间后时间弹窗未自动关闭；新角色缺少目标输入位置；Markdown 图表未正确渲染；“分身管家”需改为“数字分身管家”；建议对话框及右上角选项标签的深色模式不完整；打包默认语言需为中文。

## Evidence Inventory

| Source   | Status                          | Notes     |
| -------- | ------------------------------- | --------- |
| 前端源码 | Available                       | 项目上下文确认前端位于 `egosync-app/`，待按功能入口扫描 |
| Tauri/Rust 源码 | Available                  | 可能涉及打包语言、持久化或窗口行为，待确认边界 |
| 项目上下文 | Available                     | `_bmad-output/project-context.md`，记录技术栈与约束 |
| 既有实现文档 | Available                   | `_bmad-output/implementation-artifacts/` 下已有角色、任务、主题、Skill/MCP 等故事文档 |
| 版本控制 | Available                       | 待检查相关页面/组件近期变更 |
| 运行时日志/截图/录屏 | Missing                | 当前仅有用户描述，无法直接确认具体复现状态及浏览器/打包环境差异 |
| 自动化测试结果 | Partial                    | 项目存在测试/UAT资产，但尚未确认覆盖这些具体行为 |

## Investigation Backlog

| # | Path to Explore | Priority              | Status                                | Notes     |
| - | --------------- | --------------------- | ------------------------------------- | --------- |
| 1 | 定位管家设置、角色社会设置及共享分组组件 | High | Open | 确认折叠状态、按钮复用和保存动作边界 |
| 2 | 定位创建任务弹窗的日期/时间选择状态链 | High | Open | 确认关闭事件是否由分钟选择器、日期选择器或父弹窗控制 |
| 3 | 定位角色创建表单及目标字段 | High | Open | 对照 Role 类型、表单、持久化/编辑页面 |
| 4 | 定位 Markdown 渲染器及图表语法支持 | High | Open | 区分 Markdown 解析、代码块识别、图表渲染库/样式问题 |
| 5 | 全局检索“分身管家”、默认语言及打包配置 | Medium | Open | 区分 UI 文案、国际化资源、Tauri/构建默认值 |
| 6 | 审查深色模式 token/class 与弹窗、导航选中态 | High | Open | 检查 darkMode 配置、共享 Dialog/Tabs 组件和硬编码颜色 |
| 7 | 检查现有测试/UAT覆盖与可验证方案 | Medium | Open | 形成后续实施验收条件，不执行修改 |

## Timeline of Events

| Time        | Event               | Source                | Confidence            |
| ----------- | ------------------- | --------------------- | --------------------- |
| 2026-07-22 | 用户提交 8 组问题并明确“先分析原因和方案，暂不直接执行” | 用户输入 | Confirmed |
| 2026-07-22 | `bmad-investigate` 激活脚本改用 `python` 后成功解析 workflow | `.agents/skills/bmad-investigate/SKILL.md` 及命令输出 | Confirmed |

## Confirmed Findings

### Finding 1: 当前任务范围是调查而非实现

**Evidence:** 用户明确要求“先分析原因和方案，暂不直接执行”。

**Detail:** 本次调查不得修改业务源码、配置或测试；调查产物仅记录在 case file 中。

### Finding 2: 项目已具备与问题直接相关的前端技术约束

**Evidence:** `_bmad-output/project-context.md`：React 18、TypeScript、TailwindCSS `darkMode: 'class'`，以及前端组件/样式规范。

**Detail:** 深色模式、折叠分组、Markdown 渲染和弹窗交互应优先沿既有组件与状态管理模式追踪，而不是预设新增架构。

## Deduced Conclusions

尚无；需完成源码扫描后建立可审计的因果链。

## Hypothesized Paths

### Hypothesis 1: 多个页面问题可能共享设置分组/主题基础组件

**Status:** Open

**Theory:** 管家与角色社会的折叠、全部折叠/展开以及深色模式缺失，可能部分来自共享 Section/Dialog/Tabs 组件或同一套颜色类，而非完全独立页面缺陷。

**Supporting indicators:** 用户同时报告两个设置页的分组交互和多个深色模式缺口；项目上下文要求使用 Tailwind dark class。

**Would confirm:** 两个页面调用同一分组/弹窗/导航组件，且该组件缺少受控折叠状态或 dark variant。

**Would refute:** 两个页面完全独立实现，问题由各自局部状态/类名造成。

**Resolution:** 待源码追踪。

## Missing Evidence

| Gap              | Impact                               | How to Obtain   |
| ---------------- | ------------------------------------ | --------------- |
| 运行时截图/录屏与具体版本 | 无法区分视觉样式缺失、组件覆盖和构建产物差异 | 用户提供，或在后续获授权后本地复现 |
| 具体 Markdown 图表样例 | 无法确认是 Mermaid、代码围栏、HTML/SVG 还是其他图表语法 | 用户提供原始文本及期望渲染结果 |
| 目标字段的业务定义 | 无法确认“目标”应保存到角色哪一字段/是否已有隐含字段 | 对照现有 Role schema、故事文档与表单实现 |

## Source Code Trace

| Element       | Detail                                      |
| ----------- | ------------------------------------------- |
| Error origin  | 待源码扫描                                  |
| Trigger       | 待源码扫描                                  |
| Condition     | 待源码扫描                                  |
| Related files | 预计涉及 `egosync-app/src/`、`egosync-app/src-tauri/` 及实现文档 |

## Conclusion

**Confidence:** Low

当前仅确认用户报告的问题范围及项目技术约束，尚未确认根因。下一阶段应先绘制证据边界，再分别追踪 UI 状态、渲染链路、文案/国际化和打包配置。

## Recommended Next Steps

### Fix direction

暂不提出代码级修复；完成源码追踪后按机制分组给出方案，预计分为：共享设置分组交互、保存动作与字段模型、任务时间弹窗状态、Markdown 图表渲染、术语/语言资源、主题 token、打包 locale。

### Diagnostic

优先收集源码与 git 证据；若源码仍无法区分运行时差异，再请求针对性截图、Markdown 样例或运行时日志，不增加临时诊断日志。

## Reproduction Plan

待源码追踪后形成针对每一项的最小复现步骤与预期结果。

## Side Findings

- 当前证据表明这是一个跨多个功能域的联合调查，不应默认存在单一根因。

## Follow-up: 2026-07-22

### New Evidence

### Additional Findings

### Updated Hypotheses

### Backlog Changes

### Updated Conclusion

## Follow-up: 2026-07-22 #2

### New Evidence

- `egosync-app/src/components` contains separate butler, role, modals, chat, layout, settings and common domains; relevant concrete files include `butler/ButlerSettingsContent.tsx`, `butler/ButlerSettingsGroupedDemo.tsx`, `role/SettingsTab.tsx`, `modals/TaskModal.tsx`, `modals/AddRoleModal.tsx`, and `chat/ChatStream.tsx`.
- Dedicated tests exist beside several affected components: `ButlerSettingsContent.test.tsx`, `ButlerSettingsGroupedDemo.test.tsx`, `SettingsTab.test.tsx`, `TaskModal.test.tsx`, and `ChatStream.test.tsx`. This is Partial evidence of intended behavior; test contents still need to be mapped to each reported requirement.
- `egosync-app/package.json` declares `react-markdown` 10.1.0; no chart-specific renderer was confirmed in the initial package/config scan.
- `egosync-app/tailwind.config.js:3` confirms `darkMode: 'class'`.
- `egosync-app/index.html:2` sets `lang="zh-CN"`; `index.html:10-11` initializes theme from `egosync-theme`/system preference. This confirms the document shell defaults to Chinese and theme initialization exists, but does not prove the packaged application locale or every component's dark styles.
- `egosync-app/src/App.tsx:50-53` reads the theme preference and `src/App.tsx:325-330` persists/toggles the `dark` class. The remaining dark-mode reports therefore require component-level class tracing, not a global-theme diagnosis alone.
- Git history contains feature/fix commits for task CRUD, role settings, theme tokens and current skill/MCP work, but the initial history scan did not isolate a single commit that explains all eight reports.
- The working tree already contained unrelated modifications before this investigation; only the investigation case file was added by this task. No product source was changed.

### Additional Findings

1. **Evidence perimeter is broad but bounded.** The primary path is frontend source under `egosync-app/src`; Tauri/build files are only needed for default packaged language and possibly locale initialization. This is an Available source-code perimeter, not yet a causal trace.
2. **Markdown symptom is under-specified.** The current dependency evidence establishes a Markdown renderer but not the requested chart syntax or expected output. The original Markdown sample remains a high-value missing input.
3. **Default language has a premise conflict candidate.** The HTML shell already declares `zh-CN`, so “打包默认语言应该选中文” may refer to an application/runtime locale, seeded data, installer metadata, or packaged UI rather than the HTML document language. This must be verified rather than fixed at `index.html`.
4. **Test coverage is present but intent coverage is unknown.** Existing tests may encode the current behavior or may only cover adjacent interactions; they cannot be treated as acceptance evidence until read.

### Updated Hypotheses

- **H1 shared settings/theme layer:** remains Open; concrete component files are identified, but shared implementation and dark variants have not been traced.
- **H2 Markdown chart support is absent or incomplete:** Open; `react-markdown` is present, while chart renderer support was not confirmed. Need inspect `ChatStream.tsx` and package imports/config.
- **H3 “default language” is not controlled by `index.html`:** Open; `lang="zh-CN"` is confirmed, but the package/runtime locale path is unknown.

### Backlog Changes

- Marked source inventory and dependency/config scan Done.
- Kept component source tracing, test-intent mapping, Tauri packaging/locale tracing and runtime evidence collection Open.

### Updated Conclusion

**Confidence: Low-to-Medium for the evidence perimeter, Low for root causes.** The affected code areas and some global constraints are now confirmed. No item is yet sufficiently traced to claim a root cause or propose a code-level fix with high confidence.


## Follow-up: 2026-07-22 #3

### Confirmed Findings

1. **新增角色的目标字段在数据模型和编辑页存在，但新增弹窗没有接入。**
   - `egosync-app/src/types/role.ts:19-24` 的 `CreateRoleInput` 已定义可选 `goal`。
   - `egosync-app/src/components/modals/AddRoleModal.tsx:14-19` 只维护 `name`、`iconId`、`colorHex`，没有目标 state。
   - `egosync-app/src/components/modals/AddRoleModal.tsx:30-34` 创建参数也只传名称、图标、颜色。
   - 对照实现：`egosync-app/src/components/role/SettingsTab.tsx:607-610` 已有目标输入，且 `SettingsTab.tsx:209-225` 保存时传 `goal`；引导创建路径 `egosync-app/src/components/onboarding/RoleConfirmModal.tsx:39-44,48-60,80-87` 也已支持目标。
   - **根因等级：Confirmed，属于新增表单字段遗漏，不是后端数据模型缺失。**

2. **Markdown 图表渲染链路没有图表分支。**
   - `egosync-app/package.json:14-26` 只有 `react-markdown`，未发现 Mermaid、PlantUML、Graphviz 或 ECharts 等图表渲染依赖。
   - `egosync-app/src/components/chat/ChatBubble.tsx:273-288` 直接将消息交给 `ReactMarkdown`；自定义组件仅覆盖 `a`、`p`、`li`、`strong`、`em`（`ChatBubble.tsx:275-285`），没有 `code`/fenced-code 处理，也没有图表渲染入口。
   - **根因等级：Confirmed（缺少图表渲染能力）；具体语法和期望图表类型仍 Missing Evidence。** 普通 Markdown 解析本身存在，不能把问题笼统归因于 Markdown 全部失效。

3. **任务截止时间使用原生 `datetime-local`，当前代码没有选择完成后的主动关闭逻辑。**
   - `egosync-app/src/components/modals/TaskModal.tsx:41` 以 React state 保存截止时间。
   - `TaskModal.tsx:158-166` 使用 `<input type="datetime-local">`，`onChange` 只有 `setDeadline(event.target.value)`。
   - 没有 picker ref、关闭状态或日期/小时/分钟完成回调。
   - **根因等级：Confirmed（代码没有关闭动作）；“自动关闭”是否可由当前 Windows WebView2 原生控件自行实现，运行时未验证。**

4. **生产设置页没有接入分组折叠状态；折叠 Demo 与生产页是两套路由/组件。**
   - 管家生产页 `egosync-app/src/components/butler/ButlerSettingsContent.tsx:501-610`、`610` 之后以连续区块和分隔线呈现使命、Skill、节奏等内容，未发现统一的 open/expanded state。
   - 角色生产页 `egosync-app/src/components/role/SettingsTab.tsx:551-1015` 同样是连续内容区块，没有分组折叠状态。
   - 已有 Demo 提供可复用的交互证据：角色 Demo `egosync-app/src/components/role/GroupedSettingsDemo.tsx:113-133` 有 `expandedSections`、单组切换和全量设置；管家 Demo `egosync-app/src/components/butler/ButlerSettingsGroupedDemo.tsx:174-207` 还有 localStorage 持久化、全部展开/折叠。
   - `egosync-app/src/App.tsx:381-383` 将 Demo 作为 `role-settings-demo` 单独视图展示，因此 Demo 不是生产设置页的当前实现。
   - **根因等级：Confirmed，属于交互实现未从 Demo 接入生产页。**

5. **角色社会页面的文案、分组和保存按钮位置与需求存在直接源码对应。**
   - `SettingsTab.tsx:643` 为“Skill 配置”；`SettingsTab.tsx:867-871` 为独立的“自定义 Skill”区块；因此“自定义 Skill”当前确实不在 Skill 配置区块内。
   - `SettingsTab.tsx:786-790` 当前标题为“外部 MCP 工具”，内部说明和操作已经使用 “MCP server” 术语（`SettingsTab.tsx:790-843`）。
   - `SettingsTab.tsx:551-610` 是角色信息和目标字段；`SettingsTab.tsx:1001-1014` 是页面底部全局“保存更改”按钮。
   - 管家已有保存使命宣言按钮，`egosync-app/src/components/butler/ButlerSettingsContent.tsx:591-601` 可作为角色信息按钮的样式/反馈参考。
   - `SettingsTab.tsx:209-225` 的 `handleSave` 一次保存名称、图标、颜色、目标、个性描述，故移动按钮时应保持这个保存边界，不把 Skill/MCP 的即时保存混入角色信息保存。
   - **根因等级：Confirmed，均为现有生产组件的布局/文案设计问题，而不是后端协议问题。**

6. **深色模式全局开关存在，但目标组件存在硬编码浅色样式。**
   - `egosync-app/tailwind.config.js:3` 为 `darkMode: 'class'`；`egosync-app/src/App.tsx:50-53,325-330` 负责读取、持久化并切换 `dark` class。
   - 管家右上角导航 active/inactive class 在 `egosync-app/src/components/butler/ButlerView.tsx:97-107` 使用 `bg-white`、`text-indigo-600`、`text-slate-500`，没有对应 dark variant。
   - 角色右上角导航在 `egosync-app/src/components/role/RoleHeader.tsx:27-39` 的 active class 使用 `bg-white`，active 文字由 `roleColor` inline style 注入；也没有 dark variant。
   - 建议卡片主要实现为 `egosync-app/src/components/butler/ActionCard.tsx:136-146`，固定白色背景/浅边框；标题和内容为 `ActionCard.tsx:161-166` 的 `text-slate-800/500`；拒绝区域和输入按钮在 `ActionCard.tsx:200-245` 仍有多个浅色 class。`ButlerView.tsx:115-130` 复用该卡片显示建议/敲门通知。
   - **根因等级：Confirmed，属于组件级 dark token 缺失；不是主题开关失效。**

7. **“分身管家”不止一处 UI 文案，后端 prompt 与测试也包含该词。**
   - UI 可见位置：`egosync-app/src/components/butler/ButlerView.tsx:94`、`egosync-app/src/components/onboarding/OnboardingView.tsx:306`，引导消息还在 `OnboardingView.tsx:178`。
   - 后端身份 prompt/测试在 `egosync-app/src-tauri/src/services/agent_config.rs:509`、`agent_engine.rs:1132,1979,1991,6702,6847,7643` 仍出现“分身管家”。
   - E2E 文案断言在 `egosync-app/tests/e2e/specs/briefing-review.spec.ts:22-24`、`butler-conversation.spec.ts:12-16`、`cold-start-onboarding.spec.ts:16`、`conflict-arbitration.spec.ts:29-31`。
   - **根因等级：Confirmed，属于术语未统一；不能只修改 ButlerView 一个标题。** 但是否连后端角色 prompt、测试描述/断言一起改，需产品确认“全局术语替换”边界。

8. **“打包默认中文”目前没有可定位的应用 locale 配置；HTML 语言已是中文。**
   - `egosync-app/index.html:2` 已为 `<html lang="zh-CN">`。
   - `egosync-app/src-tauri/tauri.conf.json:1-45` 只配置产品、窗口、构建和 bundle 资源，没有 locale/language/installer language 字段。
   - 前端/后端源码扫描未发现应用语言状态、i18n 配置或 `navigator.language` 初始化；仅有日期展示 `toLocaleDateString()`（`ActionCard.tsx:49`）和中文排序 `localeCompare(..., 'zh-CN')`（`TaskOverviewTab.tsx:221`）。
   - **根因等级：Open with a clear data gap。** 当前证据只能证明网页文档语言是中文，无法证明用户所说的“打包默认语言”指应用 UI、原生日期控件、Tauri 安装器，还是宿主系统语言。

### Deduced Conclusions

- 设置折叠和角色保存按钮可在前端完成，且已有 Demo/同类按钮样式可复用；不需要数据库迁移或新的后端接口。折叠状态是否持久化是产品决策：Demo 的管家版本持久化到 localStorage，角色 Demo 的状态实现为组件 state；生产方案应先统一策略，建议持久化用户最后一次展开状态。
- 角色设置中的 Skill 开关、Skill 导入/删除、MCP 添加/移除已有独立即时保存反馈（例如 `SettingsTab.tsx:304,500-521`），因此删除底部“保存更改”时，不应改变这些操作的事务边界。
- 对原生 `datetime-local` 直接调用 blur/click/DOM hack 不能从源码证明跨 WebView 稳定；若验收标准是“选完分钟立即关闭”，可靠方案是换成项目可控的日期/小时/分钟选择器，或先接受平台原生控件行为并把“关闭”降级为失焦。
- “自定义 Skill 放入 Skill 配置”更像 DOM 结构调整，而非服务层合并：Skill 数据、事件刷新和保存逻辑已在 SettingsTab 内独立存在。

### Hypothesized Paths and Status

| Hypothesis | Status | Confirm/refute evidence |
| --- | --- | --- |
| 生产设置页可以直接复用 Demo，不需重新设计交互 | Open/partially supported | Demo 提供状态模式，但生产内容、加载态、错误态、即时保存区块不同；需实现阶段验证键盘可访问性和布局。 |
| 时间弹窗是项目自定义 picker，缺少 close callback | Refuted by source | 当前源码是原生 `input[type=datetime-local]`；仍需运行时确认用户所见面板。 |
| Markdown 问题是普通 Markdown 解析失败 | Refuted/unsupported | ReactMarkdown 已接入且测试验证普通 Markdown/记忆链接；缺失的是图表语法分支。 |
| “打包默认语言”可通过把 `index.html` 改为中文解决 | Refuted by source | `index.html` 已是 `zh-CN`；实际边界仍需安装包/运行环境证据。 |
| “建议对话框”指 ActionCard | Open but strongest candidate | `ButlerView.tsx:115-130` 明确将建议/敲门通知渲染为 ActionCard；若用户指其他弹窗，需要截图或具体入口确认。 |

### Source Code Trace

| Issue | Error origin | Trigger | Condition | Related files |
| --- | --- | --- | --- | --- |
| 管家/角色设置不折叠 | 生产组件直接渲染连续区块 | 打开设置页 | 没有 expanded/open state 或 section toggle | `ButlerSettingsContent.tsx`; `SettingsTab.tsx`; grouped Demo components |
| 角色新增没有目标 | AddRoleModal state → `roleService.create` payload | 点击创建 | form/payload 忽略 `goal`，虽类型和后端输入支持 | `AddRoleModal.tsx`; `role.ts`; `roleService.ts`; `SettingsTab.tsx` |
| 图表不渲染 | ChatBubble → ReactMarkdown default code rendering | assistant message 含图表 fenced block | 无 `code` handler、插件或 chart renderer | `ChatBubble.tsx`; `package.json`; `ChatBubble.test.tsx` |
| 选择时间后不主动关闭 | native datetime input `onChange` | 日期/小时/分钟改变 | handler 只更新 `deadline` state | `TaskModal.tsx`; runtime WebView2 remains evidence gap |
| 深色建议/导航缺失 | target components use fixed light classes | toggle dark theme / open suggestion or tab | global dark class exists but local variants missing | `App.tsx`; `ButlerView.tsx`; `RoleHeader.tsx`; `ActionCard.tsx` |
| 术语不一致 | UI、prompt、E2E literals independently contain old label | render/onboarding/prompt/test | no central display-name constant or replacement policy | Butler/Onboarding views; Rust prompt; E2E specs |
| 打包语言未确认 | no locale setting found in shell/Tauri config | build/install/run package | available evidence cannot identify which locale boundary user means | `index.html`; `tauri.conf.json`; source scan |

### Backlog Changes

- Completed source-level causal tracing for all eight issue groups.
- Completed test-intent mapping for the affected areas where tests exist. Existing tests cover current role save, task deadline payload, Markdown memory links, and Demo folding; no test currently proves the requested new behavior for AddRole goal, production folding, chart rendering, or dark variants.
- Remaining evidence gaps are limited to: (a) the concrete Markdown chart syntax/sample, (b) runtime identity of the time picker and target WebView behavior, (c) exact meaning of packaged default language, and (d) whether “建议的对话框” means `ActionCard` only.
- No product source, test, configuration, or build artifact was modified by this investigation.

### Updated Conclusion

**Confidence: Medium overall; High for the four code-local root causes (new-role goal omission, production folding absent, Markdown chart renderer absent, component-level dark classes absent); Medium for time-picker implementation boundary and terminology scope; Low for packaged default language until runtime/package evidence is provided.**

## Follow-up: 2026-07-22 #4 — Final hand-off

### Final Conclusion

The eight reports are not one shared defect. They split into four direct frontend implementation gaps, two direct UI design/token gaps, one terminology consistency task, and one unresolved packaging/runtime-locale question. The strongest confirmed fixes are surgical: add the missing goal field to `AddRoleModal`, introduce a small shared/parallel section-toggle pattern in the two production settings pages, add a chart-aware fenced-code renderer after the chart syntax is confirmed, and add dark variants to `ActionCard` plus Butler/Role header tabs. The native time picker and packaged-language item must not be “fixed” by guessing: they require runtime/package evidence before implementation.

### Recommended Fix Direction (implementation not executed)

1. **Settings folding:** define the production section IDs from the existing visible groups; add per-section controlled open state and “全部展开/全部折叠” actions. Reuse the Demo’s minimal state pattern rather than copying its mock content. Keep interactive forms mounted only as required by existing loading/save behavior, and add tests for single toggle, all-open, all-closed, and keyboard/ARIA state.
2. **Role settings structure:** keep `角色信息`, `Skill 配置`, `MCP Server配置`, and the remaining behavior/proactivity blocks as separate sections; move the existing custom-Skill block under `Skill 配置`; replace the bottom global button with a role-information-local button invoking the existing `handleSave` boundary. Do not merge the already-immediate Skill/MCP mutations into this button.
3. **Task time:** first reproduce in the packaged Windows WebView2 build. If “close after minute” is mandatory, replace the native control with a controlled picker; avoid platform-specific `blur()`/synthetic click hacks as the primary solution.
4. **New role goal:** add a multiline goal input to `AddRoleModal`, pass trimmed goal as optional `CreateRoleInput.goal`, and add a test asserting the exact create payload and visible field.
5. **Markdown charts:** obtain one failing raw assistant message. If it is Mermaid, add a fenced `mermaid` renderer and a controlled Mermaid component with loading/error handling; otherwise select a renderer matching the actual syntax. Add tests for the intended syntax, fallback code block behavior, and unsafe/invalid chart input.
6. **Terminology:** confirm whether the requested rename is UI-only or global. If global, update visible UI, onboarding copy, prompt wording, and E2E assertions together; preserve any backend identity contract only if tests or product requirements require the old phrase.
7. **Dark mode:** add dark variants to the exact classes identified above, then verify contrast for active tabs using both Butler indigo and arbitrary role colors. Do not alter global theme initialization.
8. **Packaged default language:** identify the observed English surface (app UI, native picker, installer, or OS-level UI) and capture package/runtime version. Then change only that layer; the current `index.html` is already correct for document language.

### Verification Plan

- Frontend unit tests for `AddRoleModal`, production Butler/Role section toggles, role-local save, TaskModal deadline behavior, ChatBubble chart syntax, ActionCard dark-state class contracts, and visible terminology.
- Existing regression tests: `ButlerSettingsContent.test.tsx`, `SettingsTab.test.tsx`, `TaskModal.test.tsx`, `ChatBubble.test.tsx`, and grouped-settings Demo tests.
- Windows packaged smoke checks: native datetime picker close behavior, installed UI language, dark active tabs, suggestion card contrast, onboarding and role-creation goal persistence.
- No tests were run in this investigation, so this report does not claim test passage.

### Status

**Concluded — source diagnosis complete; implementation intentionally not executed.**

### Next-step Menu

- Highest-value next action: collect the three missing runtime inputs (failing Markdown sample, screenshot/video of the time picker and English packaged surface, and clarification of the target suggestion dialog), then use `bmad-create-story` or `bmad-quick-dev` for implementation.
- If the user accepts the inferred boundaries without additional evidence, implementation can proceed as a set of small stories rather than one broad refactor.


## Follow-up: 2026-07-22 #5 — packaging evidence refinement

### New Evidence

- The workspace contains an actual release MSI artifact: `egosync-app/src-tauri/target/release/bundle/msi/EgoSync_0.1.1_x64_en-US.msi`.
- The same release bundle contains an NSIS installer: `egosync-app/src-tauri/target/release/bundle/nsis/EgoSync_0.1.1_x64-setup.exe`.
- The tracked implementation artifact already records the expected historical MSI naming pattern with `_en-US`: `_bmad-output/implementation-artifacts/1-1-tauri-desktop-app-existing-ui.md:60` and `_bmad-output/uat/UAT-Manual-Checklist.md:17`.
- The installed Tauri CLI schema at `egosync-app/node_modules/@tauri-apps/cli/config.schema.json:1` defines WiX language configuration under `bundle.windows.wix.language`, whose default is `en-US`; it also defines NSIS `bundle.windows.nsis.languages`, whose default is English when not configured and whose fallback behavior uses the OS language/first configured language.
- The checked-in `egosync-app/src-tauri/tauri.conf.json:32-45` does not declare `bundle.windows.wix` or `bundle.windows.nsis` language configuration.

### Updated Finding: packaged default language

**MSI installer language cause is now Confirmed at configuration/artifact level.** The current configuration leaves WiX at its default `en-US`, and the generated MSI filename independently records `en-US`. This explains an English-default MSI installer without implicating `index.html`.

The NSIS installer is a separate path: its artifact filename has no locale suffix, and the schema indicates OS-language selection with English fallback when no languages are configured. Whether the observed English screen came from MSI, NSIS, or the application itself remains unresolved.

### Updated Fix Boundary

- If the requirement means **WiX/MSI installer UI**, configure the Tauri Windows WiX language to Simplified Chinese (the exact WiX locale identifier should be validated by a build smoke test) instead of changing `index.html`.
- If the requirement includes **NSIS installer UI**, configure its supported language list to the valid NSIS Simplified Chinese identifier and disable the selector only if Chinese must be forced. If the product should respect the OS but prefer Chinese, keep OS selection and put Chinese first as the fallback.
- If the requirement means **application UI**, the installer setting will not solve it; the actual English surface must still be identified.

### Hypotheses Update

- H3 “default language is not controlled by `index.html`”: **Confirmed for the MSI installer path; Open for NSIS/application runtime path.**

### Backlog Changes

- Removed “no packaging evidence” as a blocker for the MSI path.
- Retained a narrower runtime gap: identify which installer/app surface the user observed, and validate the correct WiX/NSIS locale identifiers by building a package.


## Follow-up: 2026-07-22 #6 — Markdown 表格证据修正

### 用户补充证据

用户明确说明问题是 **Markdown 表格**，并提供了以制表符分隔的内容，而不是 Mermaid/PlantUML 等图表：

- 表头单元格之间使用制表符（TAB）分隔；
- 表头使用 `**粗体**`；
- 没有 `|` 管道符；
- 没有 GFM 必需的表头分隔行（例如 `| --- | --- |`）。

### 运行时验证

在当前项目依赖和当前 `ReactMarkdown` 用法下，对用户样例以及标准 GFM 管道表格进行静态渲染验证：

- 用户样例被渲染为单个 `<p>`，TAB 保留为文本间距，未生成 `<table>`。
- 标准管道表格在未启用 GFM 插件时也被渲染为单个 `<p>`，未生成 `<table>`。

该验证对应当前实现 `egosync-app/src/components/chat/ChatBubble.tsx:273-288`，以及依赖清单 `egosync-app/package.json:14-26`。

### Updated Finding: Markdown 表格

**根因现在可以拆成两个 Confirmed 条件：**

1. 当前 ReactMarkdown 没有配置 `remark-gfm`，因此标准 Markdown/GFM 管道表格也不会被解析成 HTML table。
2. 用户提供的 TAB 格式本身不是标准 GFM 表格；即使只添加 `remark-gfm`，也不能保证该原始格式自动变成表格。

因此此前“图表渲染器缺失”的描述已被修正为“Markdown 表格解析链路缺失/输入格式不符合 GFM”。不需要引入 Mermaid 等图表依赖。

### Recommended Fix Direction

优先采用最小、确定性更高的方案：

1. 为 `ReactMarkdown` 接入 `remark-gfm`，支持标准 GFM 表格、删除线等语法。
2. 约束生成端输出标准 GFM 表格格式，例如：

   ```markdown
   | 城市 | 国家 | 人口（万） |
   | --- | --- | --- |
   | 成都 | 🇨🇳 中国 | 2120 |
   ```

3. 如果产品必须兼容用户提供的 TAB 格式，再额外增加“仅针对表格候选行”的归一化步骤：只在 fenced code 之外、连续多行具有一致 TAB 列数且首行符合表头特征时，将 TAB 表格转换为 GFM；不能对所有 TAB 文本做全局替换，以免破坏普通段落和代码块。
4. 对表格组件补充横向滚动、边框和深色样式；现有 `prose`/`dark:prose-invert` 可作为基础，但不应假设它能弥补解析缺失。

### Verification Requirements

- 标准 GFM 表格能够渲染为 `<table>`、`<thead>`、`<tbody>`、`<th>`、`<td>`。
- 用户提供的 TAB 格式若被纳入兼容范围，也必须渲染为表格。
- 普通含 TAB 的段落不应被误判为表格。
- fenced code 中的 TAB 内容必须保持代码块，不得被表格归一化。
- 表格中的链接、记忆引用、emoji 和中文内容保持正常渲染。

### Hypothesis Update

- “需要图表渲染库”的假设：**Refuted**。
- “缺少 GFM 表格插件”的假设：**Confirmed**。
- “添加 `remark-gfm` 即可兼容用户样例”的假设：**Refuted**，因为用户样例不是标准 GFM 表格；还需要生成格式约束或受控归一化。

### Backlog Changes

- Markdown 缺口从“需要确认图表语法”改为“已确认是 TAB 表格，需决定是否兼容非标准格式”。
- 删除 Mermaid/PlantUML/Graphviz 作为本问题的实现前提。
- 仍未修改产品源码或依赖配置。


## Follow-up: 2026-07-22 #7 — implementation boundaries confirmed by user

### Confirmed Product Decisions

1. **TAB 表格格式不需要兼容。** Markdown 需求只支持标准 GFM 表格；不增加 TAB 表格自动归一化逻辑。
2. **Rust Prompt 不修改。** “数字分身管家”只处理用户可见 UI、引导文案及相关测试断言；保留后端 Prompt 中的“分身管家”身份文本。
3. **两种 Windows 安装包都纳入中文默认语言范围。** 同时处理 WiX/MSI 和 NSIS，不只修复 MSI 文件名对应的语言配置。
4. **必要时允许替换原生 `datetime-local`。** 先验证原生控件；若无法稳定满足“选完分钟自动关闭”，可替换为 React 可控的自定义日期/小时/分钟选择器。

### Updated Implementation Boundary

- Markdown：`remark-gfm` + 标准 GFM 输出约束；不实现 TAB 兼容层。
- 术语：修改前端可见文案和前端测试；不修改 `src-tauri` Rust Prompt。
- Packaging：配置 `bundle.windows.wix` 的简体中文语言，以及 `bundle.windows.nsis.languages` 的简体中文语言；具体语言标识符在构建验证中确认。
- Task time：优先保留原生控件进行 Windows WebView2 验证，失败后改为受控 picker。

### Recommended Execution Order

1. Markdown GFM 表格支持和新角色目标字段；
2. 管家/角色设置分组及角色信息保存按钮；
3. 深色模式局部样式和 UI 术语；
4. MSI/NSIS 中文安装语言配置与打包验证；
5. 原生时间控件验证，必要时替换自定义 picker。

### Status

调查边界已由用户确认，剩余工作从“需求澄清”转为“实施故事/代码变更准备”。本轮仍未修改产品源码、测试、依赖或打包配置。
