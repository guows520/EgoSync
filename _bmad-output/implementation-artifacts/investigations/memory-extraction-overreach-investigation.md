# Investigation: 记忆提取过度推断

## Hand-off Brief

1. **What happened.** 当前记忆管线把“是否具有长期价值、是否为显式偏好/事实”完全交给 LLM 判断，程序侧只做结构校验，因此单次任务指令可被模型过度概括为学习状态、格式偏好或使用习惯。
2. **Where the case stands.** 当前机制与测试缺口已确认；历史三条记录因原始会话和提取响应已删除，无法确认当时的精确输入、分类及模型版本，历史事件根因置信度为 Medium。
3. **What's needed next.** 优先采用“保守提示词 + 可执行的候选验收规则 + 三个反例回归用例”的最小修复；若要治理短期任务和长期认知混存，再单独设计记忆生命周期。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-28 |
| Status           | Concluded（待用户决定是否实施） |
| System           | Windows / EgoSync workspace |
| Evidence sources | 用户报告、当前源代码、Story/UAT、测试结果、Git 历史 |

## Problem Statement

用户询问当前记忆提取规则，并报告历史上存在不适合写入长期记忆的内容：把文档处理任务推断为“正在学习”；把一次 PDF 处理要求推断为格式偏好；把正常使用 Skill 的操作推断为稳定习惯。用户要求先分析原因和方案，暂不执行修复。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户提供的三条误记忆文本 | Available | 可确认产出文本；无法确认当时完整上下文、category、模型与版本 |
| 原始历史会话/提取日志 | Missing | 用户说明已删除，不能精确重放历史决策 |
| 当前提取源码 | Available | 已追踪触发、prompt、解析、冲突审查、写入链路 |
| Story / UAT | Available | Story 明确“无持久价值不写入”，UAT 仅覆盖纯寒暄 |
| 自动化测试 | Available | `cargo test memory_pipeline::tests:: -- --nocapture`：22 passed / 0 failed / 0 ignored |
| Git 历史 | Available | prompt 初始于 `ef123f4`，`d01ef68`（2026-06-01）调整措辞并移除“一次性确认”显式排除 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 定位记忆提取入口、提示词、解析和持久化调用链 | High | Done | 已完成 |
| 2 | 将三类误判映射到规则缺陷 | High | Done | 已完成 |
| 3 | 盘点相关测试及缺失边界 | High | Done | 已完成 |
| 4 | 形成最小修复与验证方案 | Medium | Done | 未执行修改 |
| 5 | 取得历史原始会话/提取响应 | Medium | Blocked | 数据已删除；仅未来复现可补齐 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 2026-05-31 | 初版记忆提炼 prompt 引入 | commit `ef123f4` / git blame | Confirmed |
| 2026-06-01 | prompt 改为“只记录用户自己说出的”，移除“不记录……一次性确认”措辞 | commit `d01ef68` / git diff | Confirmed |
| 历史时间未知 | 系统生成三条被用户认为不合适的记忆 | 用户报告 | Confirmed（文本）；触发上下文未知 |
| 2026-07-28 | 完成当前代码、Story、测试与历史调查 | 当前调查 | Confirmed |

## Confirmed Findings

### Finding 1: 提取由 LLM 做语义判断，程序不是规则引擎

**Evidence:** `egosync-app/src-tauri/src/services/memory_pipeline.rs:1058-1097`

**Detail:** prompt 只用自然语言要求“未来有持续价值”，并要求模型在 `preference/task_status/cognition_update/fact` 中分类；没有给出偏好、习惯、一次性指令、显式事实之间的操作性定义。

### Finding 2: 三条消息阈值是“启动条件”，不是“单条记忆证据门槛”

**Evidence:** `egosync-app/src-tauri/src/commands/chat.rs:85-90`; `egosync-app/src-tauri/src/services/memory_pipeline.rs:56-85`

**Detail:** conversation 有至少 3 条完整 user messages 后，全部合格 user messages 进入提取；模型仍可只引用其中 1 条生成一条偏好或事实。

### Finding 3: 后处理只校验结构，不校验语义支持关系

**Evidence:** `egosync-app/src-tauri/src/services/memory_pipeline.rs:1113-1151`; `egosync-app/src-tauri/src/db/memories.rs:533-548`

**Detail:** 验证项仅包括 category 白名单、content 非空、sourceMessageIds 非空且属于允许集合。程序不判断来源文字是否真的表达了“学习”“偏好”或“习惯”。

### Finding 4: 冲突审查只处理重复/更新，不承担候选质量审查

**Evidence:** `egosync-app/src-tauri/src/services/memory_pipeline.rs:389-467`; `egosync-app/src-tauri/src/services/memory_pipeline.rs:563-614`

**Detail:** 若没有已有记忆，候选直接全部 insert；有已有记忆时，审查目标仍只是 insert/skip/update，不会因“这是一次性指令”而拒绝。

### Finding 5: Story 的业务意图比实现的可执行规则更强

**Evidence:** `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md:31-35`; `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md:50-57`; `_bmad-output/implementation-artifacts/2-6-conversation-memory-extraction.md:121-127`

**Detail:** Story 要求无持久价值不写入、全局只记总体偏好/长期事实，并写明“不记录一次性寒暄”；实现没有把“持久价值”细化为可验证判据。

### Finding 6: 自动化测试没有覆盖用户报告的三类语义误判

**Evidence:** `egosync-app/src-tauri/src/services/memory_pipeline.rs:1300-2637`; 2026-07-28 测试执行结果

**Detail:** 22 个测试验证 JSON、过滤非法字段、消息范围、路由、去重、错误与超时。不存在“文档处理≠正在学习”“单次 PDF 指令≠偏好”“单次 Skill 操作≠习惯”的负向测试。现有测试全部通过，因此它们不会暴露本问题。

### Finding 7: schema 没有长期性或证据强度字段

**Evidence:** `egosync-app/src-tauri/migrations/004_memories.sql:1-10`

**Detail:** 记录只有 category/content/source/created_at；没有 confidence、evidence_type、evidence_count、valid_until、confirmed_at 或 lifecycle。早期三层记忆仅是构想：`_bmad-output/brainstorming/brainstorming-session-2026-05-18-1000.md:139-141`，不是当前已实现契约。

### Finding 8: `task_status` 被保存，但默认查询与上下文注入排除

**Evidence:** `egosync-app/src-tauri/src/db/memories.rs:316-355`; `egosync-app/src-tauri/src/db/memories.rs:361-387`; `egosync-app/src-tauri/src/services/agent_engine.rs:1473-1520`

**Detail:** 默认列表和用于管家摘要的 `list_all_memories` 排除 `task_status`。因此第一条若当时分类为 task_status，通常不会进入默认已知记忆；但历史 category 已丢失，不能确认。若被误分为 fact/preference，则会持续可见并被注入。

## Deduced Conclusions

### Deduction 1: 主根因是“抽象提示 + 无语义验收”的组合

**Based on:** Findings 1-4

**Reasoning:** LLM 被要求自行解释“持续价值”，候选一旦 JSON 合法就可写入；不存在第二道独立门槛纠正模型的概括或归因。

**Conclusion:** 当前实现天然允许从一次性命令上升为稳定属性，这不是数据库故障，而是提取契约与验收机制不足。

### Deduction 2: “≥3 条用户消息”制造了稳定性错觉

**Based on:** Finding 2

**Reasoning:** 阈值只控制何时运行，不要求同一 claim 被多条消息支持；`sourceMessageIds` 明确允许只有 1 个。

**Conclusion:** 不能把当前阈值解释为“偏好已经出现三次”。

### Deduction 3: 历史 prompt 改动可能降低保守性，但不是已确认单因

**Based on:** Git history and missing original trace

**Reasoning:** 旧 prompt 显式排除“一次性确认”，当前版本不再包含；但它原本也未明确排除一次性文档处理、格式要求或 Skill 操作。

**Conclusion:** 该改动是可能的促成因素，不足以确认是三条历史记录的直接根因。

### Deduction 4: 三个实例属于两种不同错误

**Based on:** User examples and Findings 1-3

**Reasoning:** “正在学习”是把任务对象错误推断成用户状态（unsupported entailment）；“偏好 PDF”和“Skill 使用习惯”是把单次行为错误推广为稳定特质（unsupported generalization）。

**Conclusion:** 修复不能只增加“一次性任务”一句话，还必须分别约束谓词推断和跨时间泛化。

## Hypothesized Paths

### Hypothesis 1: 三条历史记录由当前同类 prompt 直接产生

**Status:** Open

**Theory:** 当时模型读取单次任务指令后，按宽泛的“持续价值”要求生成了稳定化表述。

**Supporting indicators:** 当前 prompt 与 parser 确实允许这些输出通过。

**Would confirm:** 取得当时 source messages、模型响应、category 和版本，或用同版本/provider 稳定复现。

**Would refute:** 历史记录来自不同导入/迁移/手工写入链路，或当时 source messages 含明确的学习/偏好/习惯陈述。

**Resolution:** 历史证据已删除，保持 Open。

### Hypothesis 2: 第一条被分类为 `task_status`

**Status:** Open

**Theory:** “正在学习……”语言形态接近 task_status。

**Supporting indicators:** category 白名单包含 task_status。

**Would confirm:** 历史 DB 行或提取日志包含 category。

**Would refute:** 历史行显示 fact/preference/cognition_update。

**Resolution:** 历史记录已删除，保持 Open。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 历史完整 source messages | 无法判断三条输出是否与原文矛盾到何种程度 | 未来复现时保留来源消息快照 |
| 历史 LLM 原始 JSON 与模型/provider 版本 | 无法定位是 prompt、模型差异还是其他旧链路 | 增加临时、可脱敏诊断日志；复现完成后完全移除 |
| 历史 category | 无法判断第一条是否仅为 task_status | 未来复现时记录候选 category 和最终写入 category |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Trigger | `egosync-app/src-tauri/src/commands/chat.rs:107-163`：≥3 条完整 user messages 或存在 delegation 后，idle 300 秒/切换会话触发 |
| Input filtering | `egosync-app/src-tauri/src/services/memory_pipeline.rs:948-983`：排除部分委派链，最终只保留完整、非空 user messages |
| Semantic decision | `egosync-app/src-tauri/src/services/memory_pipeline.rs:1058-1097`：默认 LLM 按自然语言 prompt 生成候选 |
| Structural acceptance | `egosync-app/src-tauri/src/services/memory_pipeline.rs:1113-1151`；`egosync-app/src-tauri/src/db/memories.rs:533-548` |
| Reconciliation | `egosync-app/src-tauri/src/services/memory_pipeline.rs:389-467`：重复/冲突处理，不做持久价值复核 |
| Persistence | `egosync-app/src-tauri/migrations/004_memories.sql:1-10`：单表持久化，无生命周期/置信度 |

## Conclusion

**Confidence:** Medium（对当前机制为 High；对历史三条记录的精确因果为 Medium）

当前机制的根本缺陷已确认：把语义判断完全委托给 LLM，却只用结构校验守门；“三条消息”只是运行阈值，并非稳定偏好的证据阈值。三条示例分别暴露了“任务对象→用户状态”的无依据推断，以及“单次操作→长期偏好/习惯”的无依据泛化。历史原始数据已删除，所以不能确认当时 category、模型版本和精确触发文本，也不能把 2026-06-01 的 prompt 改动认定为唯一根因。

## Recommended Next Steps

### Fix direction

#### 方案 A：最小、优先推荐——收紧提取契约并补负向回归

1. 在 prompt 中给 category 明确定义和拒绝规则：
   - 命令/任务要求不等于用户偏好、事实或习惯。
   - “处理/总结/截取某文档”不等于“正在学习该材料”。
   - 一次指定 PDF/Markdown/Word 不等于稳定格式偏好，除非显式说“我偏好/以后默认/请记住”。
   - 一次搜索、安装、创建或调用 Skill 不等于使用习惯。
   - 不确定时必须输出空数组；宁可漏记，不可推断。
2. 约束内容忠实度：只允许保留原消息明确表达的主体、谓词、时态和模态；不得把“请处理 X”改写为“我正在学习 X”。
3. 增加三个用户示例对应的负向测试，并增加显式正例，防止规则过严。

**优点：** 修改小，符合外科手术原则。  
**局限：** 仍依赖模型遵循 prompt，测试最多能断言 prompt 契约，无法完全保证所有 provider 的语义输出。

#### 方案 B：推荐与 A 同期做的可执行验收门——高精度优先

让候选增加 `evidenceType`（如 `explicit_statement | repeated_observation | inferred_from_request`）及可回查的 `evidenceText`：

- 程序确定性拒绝 `inferred_from_request`。
- `preference` 仅接受显式偏好/默认要求，或跨不同来源重复出现；单次输出格式指令拒绝。
- “习惯”类内容必须是用户明确自述，或有多个独立会话证据；当前单 conversation 的 3 条消息不算跨时间重复。
- `task_status` 只允许复述用户明确说出的进行中/计划状态，不允许从任务请求推导状态。
- `evidenceText` 必须能在对应 source message 中找到，避免无来源扩写。

**优点：** 把部分保守策略从 prompt 变成可测试的验收规则。  
**局限：** `evidenceType` 仍由模型判断；若再加入中文关键词硬门槛，会提高精度但可能降低召回，需要明确产品取舍。对于长期记忆，建议选择高精度、低召回。

#### 方案 C：长期治理——短期任务与长期记忆分层

依据早期三层构想，拆分或增加 lifecycle：

- 一次性任务/当前上下文 → working/short-term，带 TTL 或完成状态，不进入长期人格认知。
- 显式偏好/长期事实 → durable。
- 单次行为推断出的候选 → pending，需用户确认或跨会话重复后晋升。

**优点：** 从数据模型上解决“任务状态与长期认知混存”。  
**局限：** 涉及 schema、查询、注入、UI 和迁移，不属于最小修复，不建议与本次缺陷一起直接展开。

### Recommended choice

选择 **A + B 的最小子集**：先把三类反例写进提取契约，并增加一个程序可执行的“推断自请求则拒绝”门；暂不引入完整三层架构。原因是本问题首要目标应是降低错误记忆污染，允许少记，不能继续让合法 JSON 等同于合法记忆。

### Diagnostic

若要把历史根因从 Medium 提升到 High，未来复现时建议临时记录以下脱敏信息，取得用户确认后再实施：

1. `build_extraction_prompt` 输入的 message id、role、时间和脱敏内容摘要。
2. provider/model 标识与原始 extraction JSON。
3. parser 丢弃/接受原因、最终 category/content/source ids。
4. reconciliation action 与最终写入 id。

诊断日志不得记录密钥；涉及个人内容应默认脱敏。根因确认后必须完全移除临时诊断日志。

## Reproduction Plan

1. 在隔离测试 DB 中，分别建立三段会话，每段满足 3 条完整 user messages。
2. 反例 A：只要求处理/总结某课程文档，不说“我正在学习”。期望不生成“正在学习”类 fact/task_status。
3. 反例 B：一次要求输出 PDF，不说“偏好/以后/默认”。期望不生成 preference。
4. 反例 C：一次要求搜索或创建 Skill，不说“习惯/经常”。期望不生成习惯类 fact/preference。
5. 正例对照：明确说“以后文档默认给我 PDF”“我正在系统学习该课程”“我经常使用 Skill 扩展能力，请记住”。期望生成相应记忆。
6. 对每种受支持 provider 重复运行，记录候选 JSON 和最终写入结果；通过标准优先看误记忆为 0，再观察正例召回。

## Side Findings

- 默认记忆查询会排除 `task_status`，但这只是展示/注入过滤，不是提取质量控制。
- `sanitize_memory_content` 会移除“用户偏好/用户喜欢/用户正在”等前缀（`egosync-app/src-tauri/src/services/memory_pipeline.rs:1154-1191`），属于表述清洗，不能修复错误归因；反而会让模型的推断看起来更像第一人称事实。
- Story 的测试要求包含“无价值 JSON 输出 0 条”，当前测试实质验证 parser 能接收空数组，而非真实模型能正确判断某段具体对话无价值。
