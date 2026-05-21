# PRD Quality Review — EgoSync

## Overall verdict

这份PRD在产品愿景、功能覆盖和用户旅程方面表现扎实——它有一个清晰的论点（"助手用完消失，分身持续存在"），功能需求按Feature分组且全局编号连续，UJ与FR之间有明确追溯。主要风险集中在两个方面：(1) FR-12"主动性刻度盘"的描述仍然是两档而非原型中实现的三档，PRD与实际实现出现了偏离；(2) 部分FR的可测试后果（Consequences）使用了模糊形容词而非可量化阈值，会给下游的story创建和QA验收带来困难。整体而言，这是一份结构良好的PRD，经过针对性修补后可以直接驱动架构和开发。

## Decision-readiness — adequate

PRD在大多数核心决策上给出了明确的表态：角色间知识严格隔离到V2（RQ-1）、使命宣言格式均支持（RQ-2）、能量值计算公式确定（RQ-4）、工作循环频率有默认值和可调范围（RQ-5）、四象限置信度阈值80%（RQ-6）、LLM Provider走两种标准格式（RQ-8）。§8 Resolved Questions覆盖了8个关键决策点，每个都有明确决议。

然而，FR-19"角色卡片仪表盘"说"卡片有呼吸感"但没有定义什么触发高亮/暗淡的阈值。FR-3"管家人格化语调"的测试后果只说"包含称呼和情境化表达"，这不足以让QA验收。

### Findings
- **[medium]** 仪表盘视觉优先级阈值未定义 (§4.7 FR-19) — "紧急>低能量>正常"的排序规则没有定义"低能量"阈值是40%还是30%。*Fix:* 在FR-19 Consequences中添加能量阈值定义，如"能量值<40%时卡片使用警告色调"。
- **[low]** 管家人格化语调的验收标准模糊 (§4.1 FR-3) — "包含称呼和情境化表达"难以量化验收。*Fix:* 补充2-3个具体示例或anti-pattern。

## Substance over theater — strong

这份PRD的内容是earned的，不是furniture。单一persona（知识工作者boss）做了所有的重活——5个UJ全围绕同一persona展开，没有为了看起来完整而造出多余的persona。Vision清楚地说明了与现有AI助手的区别（"助手用完消失，分身持续存在"），这不是套话。Non-Goals写了7项且每项都做了实质工作（"不追求使用时长"、"不是心理治疗工具"）。Counter-metrics（SM-C1, SM-C2）是真正的反制指标，不是装饰。Glossary定义了14个领域术语，每个都在FR中被引用。

### Findings
- 无

## Strategic coherence — strong

PRD有清晰的thesis：将内在多个自我外化为可协作AI Agent，以"角色"为核心组织单元。所有功能都服务于这个核心——从管家路由（FR-1/2）到角色CRUD（FR-4/5）到记忆系统（FR-7/8/9）到冲突仲裁（FR-13/14/15）到仪表盘（FR-19/20）。MVP scope的选择逻辑一致：V1做核心角色体验，社交/团队/移动端全部明确推迟。

Success metrics与thesis对齐：SM-1验证onboarding速度、SM-2验证角色扩展（核心价值传递）、SM-3验证持续使用。Counter-metrics（SM-C1不追求使用时长）直接呼应了"帮用户活得更好而非花更多时间在app里"的thesis。

### Findings
- 无

## Done-ness clarity — adequate

大多数FR有可测试的Consequences，但部分使用了模糊措辞：

### Findings
- **[high]** FR-12主动性级别与原型实现不一致 (§4.4 FR-12) — PRD描述为两档（"纯被动"和"主动建议"），但原型已实现三档（静默执行/适度建议/积极主动）。*Fix:* 更新FR-12为三档：静默执行（不生成建议）、适度建议（低频建议，用户确认后执行）、积极主动（高频建议+自动执行低风险操作）。
- **[medium]** FR-7记忆提炼的"不丢失关键信息"无法验收 (§4.3 FR-7) — "不丢失关键信息"是主观判断。*Fix:* 改为"提炼后的结构化记忆覆盖对话中所有显式任务、偏好声明和状态变更"。
- **[medium]** FR-10工作循环"新建议"的质量标准未定义 (§4.4 FR-10) — "产生新的建议"只要求有输出，不约束质量。*Fix:* 补充"建议必须基于角色目标和当前任务状态，非重复性内容"。
- **[low]** FR-6角色语调差异的验收方式 (§4.2 FR-6) — "用户可识别出正在对话的角色身份"是用户体验测试，不是自动化验收。*Fix:* 可保留，但标注为需要UX测试。

## Scope honesty — strong

PRD在scope方面表现出色。§5 Non-Goals列了7项且每项都有实质意义。§6.2 Out of Scope for MVP明确列出了12项延期内容，每项标注了目标版本。`[ASSUMPTION]`标签在正文中内联使用（5处），并在§9 Assumptions Index中汇总索引——roundtrip完整。`[NON-GOAL for MVP]`标签在scope外用到了2处。`[NOTE FOR PM]`有1处（多语言支持）。

### Findings
- **[medium]** 缺少FR-4b Skill配置的scope边界 (§4.2 FR-4b) — FR-4b说V1为"手动配置"但没有列出V1支持的Skill类型白名单（Web搜索、文件读写、代码执行这些都是V1吗？）。*Fix:* 在FR-4b或§6.1中明确V1支持的Skill类型范围。

## Downstream usability — adequate

Glossary定义完整（14个术语），FR编号全局连续（FR-1到FR-30），UJ编号连续（UJ-1到UJ-5）。每个Feature section标注了实现的UJ。SM与FR有交叉引用。这些为下游架构和story creation提供了良好的基础。

### Findings
- **[medium]** FR-19/FR-20混合了仪表盘与对话区切换两个不同关注点 (§4.7) — FR-19说"主界面展示所有活跃角色的卡片"，但原型中仪表盘是管家工作面板的一个tab，不是"主界面"的独立区域。PRD与实际UI实现有偏差。*Fix:* 更新FR-19描述以反映仪表盘作为管家工作面板tab的实际位置。
- **[low]** FR-22三级通知的"每日上限3次"出现在两处 (§4.8 FR-22 和 §5 Non-Goals) — 冗余但不矛盾。

## Shape fit — strong

这是一个consumer product / single-operator，PRD选择了合适的形状：单一persona、5个UJ覆盖关键场景、FR按Feature分组、Counter-metrics反映产品哲学。没有过度formalize（没有无意义的多persona矩阵），也没有under-formalize（UJ足够支撑consumer体验设计）。Glossary和FR编号足够支撑下游story creation。

### Findings
- 无

## Mechanical notes
- Glossary术语使用一致，未发现drift。
- FR编号连续FR-1到FR-30，无gap。但FR-4b的编号打破了纯整数序列，虽然合理（FR-4的子功能）但应在Convention中注明。
- §9 Assumptions Index与正文中的5处`[ASSUMPTION]`标签roundtrip完整。
- 所有UJ均引用了"boss" persona。
- PRD frontmatter缺少`status`字段。
