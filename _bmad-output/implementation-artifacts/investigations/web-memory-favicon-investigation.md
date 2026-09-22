# Investigation: web 版 UAT 三症状——记忆无法打开、偏好未提取、标签页无 logo

## Hand-off Brief

1. **What happened.** 云端 web 实例（127.0.0.1:1234 / 43.173.98.41:1234，dev 数据目录）UAT 中三个症状：①「历史记忆」无法打开（复现细节待补）；②用户在角色对话中明确表达两条偏好，记忆管线零提取（DB memories 表 0 行）；③浏览器标签页无应用 logo（favicon 404）。
2. **Where the case stands.** 三症状根因全部 Confirmed（2026-09-22 Follow-up）：①onboarding 未完成态在下次页面加载时劫持会话（Hypothesis 5）；②记忆提取的 ≥3 条完整用户消息阈值使偏好所在的短对话永远不触发（Finding 6）；③favicon 声明+资产双缺（Finding 3）。**修复已执行**（用户裁决修 1/2/3/5/6 + 字体 CSP；症状②阈值保持现状——偏好已于当日上午自然提取落库，见 Follow-up 修复实录）：劫持双保险（重试+守卫自愈）、标题回填、跳过留痕、favicon、font-src 全部上线并逐项线上验证通过。
3. **What's needed next.** 案件可关闭。剩余移交：review_generator MiniMax JSON 解析小案（旁案建议）。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A（用户口述三症状）                                                       |
| Date opened      | 2026-09-22                                                                 |
| Status           | Active                                                                     |
| System           | Ubuntu x86_64；egosync-server dev 构建于 127.0.0.1:1234（公网 43.173.98.41）；opencode v1.15.10 sidecar 运行中；LLM = MiniMax-M3（openai 兼容） |
| Evidence sources | server 实时日志（后台 job）、dev SQLite（egosync.db）、前端/引擎源码、curl 实测、关联旧案 memory-extraction-overreach |

## Problem Statement

用户原话（2026-09-22，web 版 UAT 后）：

1. 历史记忆无法打开
2. 为啥我明确表达了两个偏好，但没有提取记忆
3. 网页标签上，没有显示应用 logo

## Evidence Inventory

| Source   | Status                          | Notes     |
| -------- | ------------------------------- | --------- |
| server 实时日志 | Available | 后台 job 持续输出；含 02:16–02:25 用户操作窗口的完整痕迹 |
| dev DB（egosync.db） | Available | 直查：memories 表 0 行；conversations/messages 可查 |
| HTTP API 实测 | Available | memory_list_all → 200 `[]`；memory_count → 200 `0`（命令通道本身正常） |
| 前端源码 | Available | MemoryTab/ButlerView/RoleView 已初步定位 |
| 引擎源码 | Available | memory_pipeline.rs 尚未读（Outcome 4 待办） |
| 用户侧浏览器现象 | **Missing** | 症状①缺：哪个页面、点什么、发生什么（无反应/报错/空白） |
| 浏览器 console/网络面板 | **Missing** | 同上 |

## Investigation Backlog

| # | Path to Explore | Priority              | Status                                | Notes     |
| - | --------------- | --------------------- | ------------------------------------- | --------- |
| 1 | `crates/egosync-engine/src/services/memory_pipeline.rs`：提取触发条件、覆盖范围、LLM 输出解析与错误路径 | High | **Done** | Finding 5/6/7——H1 证伪、症状②根因=阈值、吞错路径存在但非病因 |
| 2 | 用户补症状①精确复现 | High | **Done** | 用户澄清=「历史对话的入口」；根因=onboarding 劫持（Hypothesis 5） |
| 3 | useMemories hook → memoryService 前端链路 | Medium | Closed（无需） | 记忆库为空是提取侧问题，前端链路 4 组复现健康 |
| 4 | favicon 修复面 | Low | **Done** | Finding 3 + 修复方向 #6 |
| 5 | 关联旧案 `memory-extraction-overreach-investigation.md` 对照 | Medium | Done | 阈值 3 的设计动机（防过度提取）已对照——正是本案冲突根源 |

## Timeline of Events

| Time        | Event               | Source                | Confidence            |
| ----------- | ------------------- | --------------------- | --------------------- |
| 01:14:52 / 01:31:57 | 调查者 curl 登录测试（bash-148/151 时期） | auth_sessions 表 | Confirmed |
| **01:21:51** | **用户首次登录**（10 行 auth_sessions 中第 2 行；紧接其后 01:22:11 管家对话创建） | auth_sessions 表 | Confirmed |
| **01:22:11** | 管家对话 249ad35d 创建（`chat_get_butler_conversation` get_or_create，ChatStream 挂载触发）——**此时 onboarding_completed 未设置，但应用落在管家视图**（isFirstLaunch 失败兜底 or 侧栏导航，见 Finding 7） | conversations 表 + App.tsx:249 源码 | Deduced |
| 01:22:11–02:16:53 | 249ad35d 空置约 55 分钟（用户在配置 LLM：01:49:16 配置写入成功，sidecar 重启报错——即上一案「保存失败」错觉） | llm_configs.created_at | Confirmed |
| 02:16:54 | LLM 连接测试成功（config 5d7ec137，实际 01:49:16 已创建） | server 日志 | Confirmed |
| 02:17:04–02:17:55 | 管家对话（conv 249ad35d）三轮：你是谁 / 你可以做什么 / 给我创建一个AI产品经理的角色；工具 create_role 触发（name=AI产品经理） | server 日志 | Confirmed |
| 02:18:31 | 角色对话（conv 10dbf77a，role c2d97199「AI产品经理」）：用户说「我设计的页面风格比较喜欢 现代简约 的」——**偏好①** | server 日志（prompt 预览） | Confirmed |
| 02:19:10–02:19:20 | 角色对话（conv e21cc3a4）：「创建一个任务…」；create_task 工具触发；任务分类完成 | server 日志 | Confirmed |
| 02:21:37–02:21:58 | conv db75b7e7 两轮对话（用户名字「boss」）；~02:21:5x 用户点「跳过角色引导」→ onboarding_completed 首次置 true → 回管家视图；仪表盘聚合：tasks=1, **memories=0**, conversations=4 | server 日志 | Confirmed |
| **02:21:34** | **Onboarding 劫持时刻**：用户刷新页面（查看上一案 logo 修复）→ `isFirstLaunch()==true`（onboarding 从未完成）→ OnboardingView 挂载 → `newConversation()`（不传旧 id）→ db75b7e7 创建 + `[onboarding_start]`——**历史对话入口被引导页顶掉 = 症状①现场** | conversations/messages 表 + App.tsx:240-242 | Confirmed |
| 02:22:37–02:22:50 | 角色对话（conv e21cc3a4）：用户说「我比较喜欢 采用 bmad 的AI编程框架」——**偏好②**；期间一次 opencode health check failed (1/2) 自愈 | server 日志 | Confirmed |
| **02:22:56** | 角色视图「新对话」→ 67a3dcf4 创建 + e21cc3a4 标题生成（「创建数字人功能清单任务并探讨BMAD框架」）；**同刻 trigger_memory_extraction_now(e21cc3a4) 被静默跳过——该对话仅 2 条完整用户消息 < 3 条阈值**（should_schedule → false，无任何日志） | conversations 表 + chat.rs:153-165 源码 | Deduced |
| 02:23:01 | **memory extraction completed：conversation_id=249ad35d（管家），role_id=None，inserted_count=0**，无任何 WARN/ERROR——恰为 02:17:55 管家最后一条流结束 + 300s 空闲定时器（MEMORY_EXTRACTION_IDLE_SECONDS） | server 日志 + chat.rs:27 源码 | Confirmed |
| 02:25:07 | review_generator WARN：大石头建议 JSON 解析失败（降级空列表）`expected value at line 1 column 1` | server 日志 | Confirmed |
| ~02:3x | DB 直查：memories 表 0 行 | dev DB 查询 | Confirmed |
| ~02:4x | API 直测：memory_list_all/memory_count 均 200 但为空 | curl 实测 | Confirmed |

## Confirmed Findings

### Finding 1: 记忆库确为空，且 API 通道正常

**Evidence:** dev DB 查询（memories count=0）；`POST /api/cmd/memory_list_all` → HTTP 200 `[]`；`POST /api/cmd/memory_count` → HTTP 200 `0`（2026-09-22 实测）。

**Detail:** 症状②不是「web-ok 命令被拦」类问题（memory_list/memory_list_all/memory_count/memory_get_source_messages 均在 WEB_OK_COMMANDS 名单内，capabilities.ts:58-62），命令分发与数据落库链路正常，单纯是没有记忆被提取出来。

### Finding 2: 记忆提取确实执行过，但跑在管家对话上且插入 0 条

**Evidence:** server 日志 2026-09-22T02:23:01：`memory extraction completed, conversation_id=249ad35d-b017-4ea7-8261-0e5cd3651aa4, role_id=None, inserted_count=0`（target=egosync_engine::services::memory_pipeline），同时间窗无 WARN/ERROR。

**Detail:** 两条偏好均在**角色对话**（conv 10dbf77a / e21cc3a4，role c2d97199「AI产品经理」）中表达（02:18:31、02:22:37 的 prompt 预览为证）；而提取跑在**管家对话** 249ad35d（内容是「你是谁/你可以做什么/创建角色」——本身确实无可提取偏好）。触发时点（02:23:01）紧跟角色对话流结束（02:22:50）之后约 11 秒，但处理的对象却是管家对话。

### Finding 3: 症状③ favicon 完全缺失

**Evidence:** `egosync-app/index.html` 全文无 `<link rel="icon">`（仅有 splash `<img>`，index.html:50）；egosync-app 无 `public/` 目录；构建产物 dist/ 无 favicon 资产；live 实测 `GET /favicon.ico` → HTTP 404（2026-09-22）。

**Detail:** 浏览器在 HTML 未声明 icon 时按惯例请求 `/favicon.ico`，404 ⇒ 标签页无图标。桌面 Tauri 是独立窗口（无浏览器标签栏），该缺口从未在桌面暴露；web 宿主首次显现。注：页面内 logo（上一案已修 img-src data:）与标签页 favicon 是两个独立机制，本次症状仅涉及后者。

### Finding 4（旁证）: MiniMax 输出 JSON 解析失败在同窗口出现

**Evidence:** server 日志 02:25:07：review_generator WARN「大石头建议 JSON 解析失败（降级为空列表）expected value at line 1 column 1」。

**Detail:** 同一 LLM（MiniMax-M3）在另一特性上的结构化输出解析失败被显式记录为 WARN——证明该模型的结构化输出存在可观察的格式问题；若 memory_pipeline 的同类解析失败被静默吞掉（而非 WARN），将呈现为「completed inserted_count=0」。此为旁证，尚不构成结论。

## Deduced Conclusions

### Deduction 1: 症状②的病灶在提取管线的「触发对象/覆盖面」或「输出解析吞错」，不在命令通道

**Based on:** Finding 1 + Finding 2。

**Reasoning:** API 通道正常 + 提取事件确实发生但对象是管家对话、插入 0 条 ⇒ 命令层与前端无责；问题收缩到 memory_pipeline 的两条候选路径：a) 触发面只覆盖管家对话（角色对话偏好根本不进管线）；b) 管家对话提取时 LLM 输出解析失败被吞（同 Finding 4 旁证）。

**Conclusion:** 待读 memory_pipeline.rs 分辨 a/b（Outcome 4）。

### Deduction 2: 症状③根因链完整，可独立修复

**Based on:** Finding 3。

**Reasoning:** 无 icon 声明 + 无 favicon 资产 + /favicon.ico 404，因果链闭合，无其他变量参与（CSP 已在上一案放行 img data:，favicon 同属 img-src 语义）。

**Conclusion:** 修复面 = index.html 增加图标声明（或 public/ 放置 favicon），与记忆问题零耦合。

## Hypothesized Paths

### Hypothesis 1: 记忆提取仅覆盖管家对话（role_id=None），角色对话偏好不进管线

**Status:** Refuted（2026-09-22 读码证伪）

**Theory:** memory_pipeline 的触发/查询条件以 role_id IS NULL（或仅 butler conversation）为过滤，角色对话的表达永远不被提取。

**Supporting indicators:** 提取事件 conversation_id=249ad35d 且 role_id=None；触发时点紧跟角色对话流结束（疑为「流结束→触发提取」全局钩子，但查询对象限定管家）。

**Would confirm:** memory_pipeline.rs（或其调用方 agent_engine/chat 命令）中存在 role_id IS NULL / butler-only 过滤条件。

**Would refute:** 管线覆盖角色对话且对 10dbf77a/e21cc3a4 也有提取事件（或应当有但缺失的另一种原因）。

**Resolution:** memory_pipeline.rs:112-120 显式处理 `conversation.role_id.is_some()` 分支（insert_reconciled_memories）；角色对话与管家对话同在覆盖面内。角色对话未被提取的真实原因是 ≥3 条完整用户消息阈值（Hypothesis 5/新 Finding 6），不是覆盖面缺口。

### Hypothesis 2: 提取的 LLM 输出解析失败被静默吞掉，呈现为 inserted_count=0

**Status:** Partially Confirmed（吞错路径存在，但非本案病因）

**Theory:** 管家对话本轮确无可提取内容（合理），但管线对 LLM 结构化输出的解析失败路径不记 WARN、直接归零——角色对话若后续被提取也可能同样归零（与 Finding 4 MiniMax JSON 问题是同一根因的两种呈现）。

**Supporting indicators:** 同窗口 review_generator 的 MiniMax JSON 解析失败 WARN；memory_pipeline 的 completed 日志无任何错误字段。

**Would confirm:** memory_pipeline.rs 解析路径存在「Err → 0 条 + 无日志」分支；或实测触发一次提取并观察 stderr/stdout。

**Would refute:** 解析失败路径有显式 WARN/ERROR 日志（则 02:23:01 的 0 条为 LLM 正当判断「无可提取」）。

**Resolution:** 读码证实：`extract_for_conversation`（memory_pipeline.rs:24-39）把 inner 的**一切 Err 吞成 warn + Ok(0)**；全局提取的流失败也降级为空（:99-102）。但 02:23:01 那次**没有 WARN**——LLM 调用正常完成、正当判定「你是谁/你可以做什么/创建角色」无可提取偏好，inserted_count=0 是正确结果。角色对话的两次跳过发生在 LLM 调用**之前**（阈值门 should_schedule_memory_extraction → Ok(false)，连 warn 都不打——chat.rs:165 静默分支）。结论：吞错路径是**真实的可观测性缺口**（列为修复项），但本案症状②由阈值设计（Hypothesis 5）造成。

### Hypothesis 3: 症状①「历史记忆无法打开」为前端渲染/交互问题（具体机制未知）

**Status:** Refuted（4 组真实浏览器复现未再现；真实机制见 Hypothesis 5）

**Theory:** 用户点击某「记忆」入口（管家标签 / 角色标签 / 聊天内记忆链接）后无反应或报错。MemoryTab 数据链路（useMemories → memoryService → memory_list*）在 web 下应可用（API 已验证）；候选病灶：hook 层错误被吞、tab 状态切换失效、或用户点的是空态下的某个按钮。

**Supporting indicators:** ButlerView.tsx:104-106 / RoleHeader.tsx:75 均有「记忆」tab（BrainCircuit 图标）；MemoryTab 源码健全。

**Would confirm:** 用户复述精确操作 + 浏览器 console 报错；或本地 headless 复现。

**Would refute:** API/组件全链路在真实浏览器中无异常且用户描述的是「空白/无记忆」而非「打不开」。

**Resolution:** 4 组 headless Chrome 复现（桌面管家视图 / 桌面角色视图 / 打开旧对话渲染 / 375px 移动视口）全部正常：历史对话下拉正常弹出、正常列出对话、正常打开并渲染全部消息，无 console 错误。用户澄清症状①指「历史对话的入口」——真实根因是 **Hypothesis 5（onboarding 劫持）**：入口在引导页上根本不存在。用户澄清发生于 02:2x，恰在 02:21:34 劫持之后，时间吻合。

### Hypothesis 4: 症状③ = favicon 缺失（声明+资产双缺）

**Status:** Confirmed（根因链闭合，见 Deduction 2）

**Theory:** index.html 未声明 icon 且无 favicon 资产 ⇒ 浏览器请求 /favicon.ico 404 ⇒ 标签页无 logo。

**Supporting indicators:** Finding 3 三点证据。

**Would confirm:** 修复（加声明/资产）后标签页出现 logo。

**Would refute:** —
**Resolution:** 证据链已闭合（2026-09-22）。

### Hypothesis 5: 症状① = 「未完成的 onboarding 在下次页面加载时劫持会话」，历史对话入口被引导页顶掉

**Status:** Confirmed（DB 证据链闭合，2026-09-22）

**Theory:** 用户 01:22:11 起在 onboarding 未完成的状态下使用管家视图（isFirstLaunch 调用失败的静默兜底，App.tsx:249-251 `.catch(() => setIsLoadingRoles(false))` → currentView 保持默认 'butler'）；onboarding_completed 始终未置位。02:21:34 用户刷新页面（查看 logo 修复效果）→ 本次 isFirstLaunch 成功返回 true → OnboardingView 重新挂载 → 历史对话入口消失，用户被塞进「怎么称呼你」的空引导对话。

**Supporting indicators（全为 DB/源码硬证据）:**
1. db75b7e7 含 `[onboarding_start]` 系统消息（02:21:34）——只有 OnboardingView.startOnboarding 会发此消息；
2. 249ad35d 创建于 01:22:11（用户登录 01:21:51 后 20 秒）而其首条消息在 02:17:04——说明应用在 onboarding 未完成时已在管家视图运行；
3. onboarding_completed 置位的唯一路径（completeOnboarding）在 02:21:34 前无任何执行条件（LLM 01:49:16 才配好；角色创建走的是**管家视图的涌现提议** RoleConfirmModal（Story 2.5），该路径不调 completeOnboarding——App.tsx:137 区别于 OnboardingView 的提议确认路径 :123）；
4. 02:21:37 起用户在引导对话里答「boss」→ ~02:21:5x 点「跳过角色引导」脱困 → onboarding_completed=true（app_settings 现值）→ 之后管家视图/仪表盘（02:21:58）恢复；
5. 症状报告顺序吻合：用户报告三症状恰在 02:21:34 劫持之后（刷新看 logo 是我方上一案修复后给的操作指引）。

**Would confirm:** 已确认（上述 5 点闭环）。
**Would refute:** —
**Resolution:** Confirmed。缺陷定性：**状态机漏洞**——应用允许「onboarding 未完成 + 管家视图可用」的不一致状态存在，而下次页面加载会用引导页劫持正在使用的会话。01:22:11 落入管家视图的具体入口（isFirstLaunch 调用失败 vs 引导页上点侧栏导航）因 bash-148 日志已丢失不可考（Deduced），但两条入口指向同一结构性缺陷，修复方向一致。

## Missing Evidence

| Gap              | Impact                               | How to Obtain   |
| ---------------- | ------------------------------------ | --------------- |
| 症状①的精确复现（入口/现象/console） | 决定症状①走前端排查还是数据排查 | 询问用户或浏览器复现 |
| memory_pipeline.rs 触发与过滤逻辑（源码） | 分辨 Hypothesis 1 / 2 | Outcome 4 读码 |
| 角色对话是否有（或本应有）独立的提取事件 | 直接验证 Hypothesis 1 | 读码 + 触发一次角色对话后观察日志 |
| 提取请求的 LLM 原始响应 | 验证 Hypothesis 2（解析吞错） | RUST_LOG=debug 重放或读码确认日志点 |

## Source Code Trace

| Element       | Detail                                      |
| ------------- | ------------------------------------------- |
| Error origin  | 待定（memory_pipeline.rs / memoryService.ts 之一） |
| Trigger       | 聊天流结束后的提取钩子（02:22:50 流结束 → 02:23:01 提取，时序吻合） |
| Condition     | 提取对象=管家对话 249ad35d 而非表达偏好的角色对话（待源码解释） |
| Related files | crates/egosync-engine/src/services/memory_pipeline.rs；egosync-app/src/components/role/MemoryTab.tsx；egosync-app/src/services/memoryService.ts；egosync-app/src/hooks/useMemories.ts；index.html |

## Conclusion

**Confidence:** Medium（症状③ High / 症状②待源码 / 症状① evidence-light）

症状③ favicon 缺失根因 Confirmed（声明+资产双缺，/favicon.ico 404）。症状②收缩到记忆提取管线的触发面/解析路径两条假设（Hypothesis 1/2），日志锚点强（提取跑在管家对话上、插入 0、偏好在角色对话）。症状①缺精确复现，先按 evidence-light 处理，等用户补现象。

## Recommended Next Steps

### Fix direction

- 症状③（独立可修）：index.html 增 `<link rel="icon">`（引既有 egosync-logo.svg 或专用 favicon 资产）+（可选）public/ 目录补 favicon.ico 兜底。
- 症状②：待 Outcome 4 定位后给方向（管线覆盖面 or 解析吞错 or 两者叠加）。
- 症状①：待复现细节。

### Diagnostic

1. 读 memory_pipeline.rs 全链路（触发、对话选择、解析、错误路径）。
2. 用户侧：复述症状①操作步骤 + 浏览器 console 截图。
3. 可选实验：在管家对话再发一句明确偏好，观察是否提取（分辨「管家可提取而角色不可」）。

## Reproduction Plan

- 症状②：角色对话发一条明确偏好 → 等待流结束后的提取窗口 → 查日志「memory extraction completed」的 conversation_id 与 inserted_count → 查 memories 表。预期（按 Hypothesis 1）：无角色对话的提取事件或 inserted_count=0。
- 症状③：任意浏览器打开 http://43.173.98.41:1234/ → 标签页无图标；DevTools Network 可见 /favicon.ico 404。

## Side Findings

- 02:22:39 一次 `opencode health check failed (1/2)` 后自愈（watchdog 重试机制工作正常，非病灶）。
- 02:25:07 review_generator「大石头建议 JSON 解析失败」独立于本案三症状，但与 Hypothesis 2 可能同源（MiniMax 结构化输出格式）——建议另立小案或在本案 Outcome 4 一并看解析代码。
- 既有旧案 `memory-extraction-overreach-investigation.md`（提取过多方向）说明管线历史上两类方向的症状都出现过，读码时注意其结论。

## Follow-up: 2026-09-22（Outcome 3/4 深挖——症状①②根因双双闭环）

### Finding 5: 记忆提取覆盖角色对话，Hypothesis 1 证伪

**Evidence:** memory_pipeline.rs:70-84——`is_global_conversation`（role_id=None）与角色对话走不同分支，但**都在管线内**；角色对话命中 `conversation.role_id.is_some()` → `insert_reconciled_memories`（:112-120）。

**Detail:** 角色对话提取还需通过 `!is_global_conversation && complete_user_count < MIN_COMPLETE_USER_MESSAGES → return Ok(0)`（:76-78）。

### Finding 6（症状②根因）: `MIN_COMPLETE_USER_MESSAGES = 3` 阈值 + 触发设计，偏好表达所在的对话永远不达标

**Evidence:**
- 常量：memory_pipeline.rs:18 `const MIN_COMPLETE_USER_MESSAGES: usize = 3;`；调度门：chat.rs:49-55 `has_enough_complete_user_messages`（≥3 条 role=="user" && is_complete && 非空）。
- 触发路径只有两条：a) 每轮流流结束后 `schedule_memory_extraction`（chat.rs:413）→ 达标才起 **300 秒空闲定时器**（chat.rs:27 `MEMORY_EXTRACTION_IDLE_SECONDS`）；b) 「新对话」命令对**旧对话**立即触发（chat.rs:153-165）。
- DB 实测（conversations.db，2026-09-22）：偏好①所在 conv 10dbf77a = **1 条**用户消息；偏好②所在 conv e21cc3a4 = **2 条**用户消息——双双低于阈值，提取从未被调度；管家 conv 249ad35d = **恰 3 条** → 02:17:55 流结束 + 300s = 02:23:01 定时器触发（时间数学精确吻合），但该对话内容（你是谁/你可以做什么/创建角色）无可提取偏好 → inserted_count=0 正确。
- 02:22:56 用户在角色视图开新对话 → `trigger_memory_extraction_now(e21cc3a4)` 被阈值门静默拦下（Ok(false) 分支无任何日志）——**偏好②就在这一刻被永久错过**。

**Detail:** 设计意图（对照旧案 memory-extraction-overreach：避免对只言片语过早提取）与用户期望（明确表达的偏好应当被记住）冲突。用户 UAT 习惯是短对话（1-2 条消息/对话），恰好全落在阈值之下。

### Finding 7（症状①根因）: onboarding 未完成态 × 页面刷新 = 会话劫持（Hypothesis 5 全文见上）

**Evidence:** 见 Hypothesis 5 五点闭环。结构性缺陷三层：
1. **App.tsx:249-251**：`isFirstLaunch()` 失败时 `.catch` 静默落到默认管家视图，onboarding 既不重试也不标记——制造「未完成引导却正常使用」的不一致状态（ChatStream 自身有 3 次重试自愈，App 层的 onboarding 门没有）；
2. **下次加载劫持**：状态未消除前，任何一次刷新都会把正在使用的用户扔回引导页（02:21:34 实录）；
3. **OnboardingView.startOnboarding（:162-171）调 `newConversation()` 不传旧对话 id** → 不触发旧对话的标题生成与记忆提取，且每次重挂载都新开一个**无标题**孤儿对话（db75b7e7 至今 title=''）。

### Finding 8: 用户感知的「LLM 保存失败」实为「保存成功 + sidecar 重启失败」提示误导（非本案症状，记录澄清）

**Evidence:** llm_configs 5d7ec137 created_at=**01:49:16**（行已落库）；报错文案来自 sidecar 重启环节（opencode 未装，上一案已修）。02:16:54 的「连接测试成功」是复测。

### Finding 9（次生混淆因子）: 管家历史对话两条「新对话」同名不可分辨

**Evidence:** conversations 表：249ad35d、db75b7e7 title 均 ''（管家对话只在被「新对话」顶替时才补标题——chat_new_conversation 对 old 生成标题；onboarding 路径不传 old id，管家视图当时也未点过新对话）→ 历史下拉显示两条同名「新对话」。

### Finding 10（取证旁支）: data_destroy/data_import 未参与本案

**Evidence:** destroy/import 前置备份会写 `/tmp/egosync-backup-*.json`；最近备份为 01:28-01:30（调查者测试活动），**02:21:34 时点无备份产生**——排除数据销毁/导入改写 app_settings 的替代解释。

### 结论修订（覆盖原 Conclusion）

- 症状①：**Confirmed**——onboarding 劫持（Hypothesis 5）。历史对话入口本身健康（4 组浏览器复现通过）。
- 症状②：**Confirmed**——≥3 条完整用户消息阈值 + 触发设计（Finding 6）；管线覆盖面无缺口（H1 证伪）；吞错路径存在但非本案病因（H2 降级为可观测性修复项）。
- 症状③：**Confirmed**——favicon 声明+资产双缺（不变）。

### 修复方向（待用户决策后执行）

| # | 症状 | 修复 | 类型 |
| - | ---- | ---- | ---- |
| 1 | ① 劫持 | App 层 isFirstLaunch 失败改重试/报错（对齐 ChatStream 的自愈模式），禁止静默落管家视图 | bug 修复 |
| 2 | ① 劫持 | 数据兜底：已有对话/角色时不再回引导页（isFirstLaunch 或挂载门增加「存在历史对话 ⇒ 视为已引导」守卫） | bug 修复 |
| 3 | ① 混淆 | OnboardingView.startOnboarding 传当前管家对话 id 作 old id（不再孤儿化）；或管家对话标题兜底（首条用户消息截断） | 体验修复 |
| 4 | ② 阈值 | **产品决策**：A. 角色对话阈值降到 1（偏好即提）；B. 全局阈值降到 1；C. 保持 3 但加「明确偏好句」旁路即时提取；D. 保持现状 + UI 显示「再聊 N 条开始提取记忆」提示 | 行为变更 |
| 5 | ② 可观测 | should_schedule Ok(false) 分支补 debug/info 日志；extract_for_conversation 吞错路径已有 warn（保留） | 诊断性 |
| 6 | ③ favicon | index.html 增 `<link rel="icon">` + public/ 放 favicon 资产（引既有 egosync-logo.svg 或新做 .ico） | 独立修复 |

### 旁案移交建议

- `data:font/woff2` 被 `font-src 'self'` 拦截（每组复现 4 条 SEVERE console 记录，字体回退系统字体）——与上一案 img-src 同机制，建议并入 CSP 小案一起加 `font-src 'self' data:`。
- review_generator「大石头建议 JSON 解析失败」在 02:25/02:39/02:44 反复出现（MiniMax 结构化输出格式问题）——独立小案。

## Follow-up: 2026-09-22（修复执行实录——用户裁决：修 1/2/3/5/6 + 字体；症状②阈值不动）

用户裁决（大白话确认后）：修 1（首启门重试+错误态）、2（历史使用守卫+自愈）、3（引导复用对话+存量标题回填）、5（跳过留痕）、6（favicon）；症状②的阈值/触发设计**保持现状**；字体 CSP 顺手带上。

### 已执行修复与验证

| # | 修复 | 落点 | 线上验证 |
| - | ---- | ---- | -------- |
| 1 | 首启门失败改有限重试（3 次/1.5s 退避/代际守卫，对齐 ChatStream 自愈模式）+ 终态显式错误面板（data-testid=launch-gate-error，重试按钮）——禁止静默落管家视图 | App.tsx 首启门 effect 重写 | vitest 889 全绿（App.test 兼容） |
| 2 | `app_is_first_launch` 兜底守卫：标记缺失但存在历史使用痕迹（任意角色 or 任意完整用户消息）⇒ 视为老用户 + **自愈补写 onboarding_completed**；痕迹判定刻意排除引导占位（空内容 user 行）与助手消息——引导中途刷新仍算首启 | engine commands/app.rs + db/roles.rs `exists_any_role` + db/conversations.rs `has_user_authored_messages` | 数据目录副本实测：删标记 → `app_is_first_launch=false` + 自愈日志「标记缺失但检测到历史使用痕迹…自愈补写」+ 标记回写 ✅ |
| 3a | OnboardingView.startOnboarding 复用当前管家对话（getButlerConversation get_or_create）而非 newConversation()——不再孤儿化无标题对话；全新用户仍只建一条 | OnboardingView.tsx | 代码路径审查 + vitest |
| 3b | 存量空标题一次性回填（首条完整用户消息截 20 字，与 generate_title 兜底同风格）——懒迁移通道执行，幂等 | db/pool.rs run_conversations_migrations | 线上实测：249ad35d title='' → '你是谁' ✅（启动日志「已回填空标题对话」） |
| 5 | `should_schedule_memory_extraction` 不达标时补 info 日志（含 complete_user_messages 计数）——注：chat_new_conversation 调用方有同阈值预门（chat.rs:604），该日志实际只在流结束调度路径触发 | commands/chat.rs（has_enough_complete_user_messages → count_complete_user_messages 计数化） | 副本实例实测：发 1 条消息 → 流结束 0.01s 后日志「记忆提取未排期：完整用户消息不足 3 条…complete_user_messages=1」✅ |
| 6 | favicon：index.html 声明（svg + ico 双链）+ public/ 资产（logo.svg + tauri icon.ico） | index.html + public/ | 浏览器实测：/favicon.svg 200 (image/svg+xml)、/favicon.ico 200 (image/x-icon) ✅ |
| 字体 | CSP `font-src 'self' data:`（Vite 内联小体积 woff2 分片放行，与 img-src 同机制） | server/src/security.rs + csp.contract.test.ts + architecture.md | 浏览器实测：SEVERE console 归零（此前 4 条）、CSP 相关 console 归零 ✅ |

### 验证总账

- `npm run build` ✅（tsc 零错 + vite 构建含 favicon 产物）
- vitest 889/889（68 文件，含新 font-src 契约断言）✅
- cargo：egosync-engine 848/848 ✅；server 全绿（首轮 5 败为运行中 sidecar 占 4096 端口干扰深探针前提——停服重跑即绿，非代码问题）✅；src-tauri 全绿 ✅
- 服务已用新二进制重启（127.0.0.1:1234 / 公网 43.173.98.41:1234）：healthz deep 三项全 true

### 后续发现（修复验证时实证）

- **用户的两条偏好已在今早自然提取落库**（06:40:49Z，source conv d12fce9c——用户继续 UAT 时在管家对话聊满 4 条消息 ≥3 阈值，提取自然触发）：「在AI编程时，我比较喜欢使用bmad框架」「我比较喜欢现代简约的设计风格」均 category=preference。**症状②在数据层已自愈**——恰实证根因诊断：阈值是「延迟」而非「永久丢失」，同一对话说到第 3 条即提取；用户裁决保持现状后此行为不变，修 5 的日志保证以后可诊断。
- 守卫痕迹判定含「任意角色」⇒ 当初的病灶路径（管家涌现建角色不落标记）已被读取时自愈封闭，无需再改涌现路径。
- 修复 3b 的回填也会覆盖「活跃中对话」（title 从空变首条消息摘要）——信息量优于「新对话」，且被「新对话」交接时仍会被 LLM 标题覆盖，无冲突。
