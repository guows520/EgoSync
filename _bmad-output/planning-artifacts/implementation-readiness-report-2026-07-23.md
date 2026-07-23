---
stepsCompleted:
  - step-01-document-discovery
  - step-02-prd-analysis
  - step-03-epic-coverage-validation
  - step-04-ux-alignment
  - step-05-epic-quality-review
  - step-06-final-assessment
filesIncluded:
  - _bmad-output/planning-artifacts/prd-egosync.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/project-context.md
referenceFiles:
  - _bmad-output/planning-artifacts/review-prd-final-2026-07-22.md
---
# Implementation Readiness Assessment Report

**Date:** 2026-07-23
**Project:** 探索
## 1. Document Discovery

用于本次评估的正式文档：

- PRD：`prd-egosync.md`
- Architecture：`architecture.md`
- UX：`ux-design-specification.md`
- Epics & Stories：`epics.md`
- 项目基础事实：`_bmad-output/project-context.md`

补充参考：`review-prd-final-2026-07-22.md`。

未发现正式文档同时存在完整版和分片版的冲突；评审报告不作为第二份 PRD。

## 2. PRD Analysis

### Functional Requirements

#### FR-1: 自然语言意图解析与路由

管家可以接收用户的自然语言输入，解析意图并路由到对应角色。当意图模糊时，管家通过追问澄清而非猜测分配。

**Consequences (testable):**
- 用户输入一条任务描述后，管家在3秒内识别出目标角色并确认分配
- 当输入无法明确映射到任何角色时，管家提出澄清问题而非随机分配
- 路由决策日志可审查（透明审计）

#### FR-2: 双通道任务分配

用户可以通过管家分发任务到角色，也可以直接切换到角色视角下达任务。角色接收的直接任务自动同步给管家。

**Consequences (testable):**
- 用户通过管家下达任务，角色收到并开始处理
- 用户直接对角色下达任务，管家的全局视图中出现该任务
- 管家可查看所有角色的任务列表，无论任务来源

#### FR-3: 管家人格化语调

管家的对话语调稳重、可靠、有温度，不同于通用AI助手的生硬风格。 `[ASSUMPTION: 管家语调通过System Prompt控制，V1不支持用户自定义管家人格]`

**Consequences (testable):**
- 管家的回复包含称呼（"boss"）和情境化表达
- 管家不使用机械化列表作为唯一输出格式，优先使用自然语言段落

### 4.2 角色管理

**Description：** 角色是EgoSync的核心组织单元。用户可以创建、编辑、归档或永久删除角色。每个角色有名称、目标、职责、个性化语调，并可选配置Skill以扩展角色能力。V1支持角色从对话中自然涌现（管家识别用户描述中的角色需求并建议创建），同时提供手动创建入口满足已有明确想法的用户。实现UJ-1, UJ-5。

**Functional Requirements：**

#### FR-4: 角色CRUD

用户可以创建新角色（名称+目标+职责），编辑现有角色的属性，归档或永久删除不再需要的角色。

**Consequences (testable):**
- 创建角色后，角色卡片出现在仪表盘
- 编辑角色目标后，角色的工作循环基于新目标运行
- 归档角色后，该角色停止工作循环但记忆保留可查，可在管家设置中恢复
- 永久删除角色需二次确认，删除后角色数据不可恢复

#### FR-4b: 管家与角色Skill配置

管家和每个角色都可以独立添加 Skill，并分别维护各自的 Skill 启用状态。Skill 基于 opencode 的 SKILL.md 格式，为对应 Agent 提供额外能力。Skill 可来自三个来源：①EgoSync 内置 Skills；②用户自定义 SKILL.md 文件；③opencode 生态中的第三方 Skills。管家与角色的 Skill 配置互不继承、互不覆盖；同一个 Skill 是否启用，以当前对话所属 Agent 的配置为准。通过 MCP 协议，角色还可接入外部工具服务。实现 UJ-5。

**Consequences (testable):**
- 管家和角色均可在各自设置中添加/移除 Skill，并独立设置启用或关闭状态
- 管家未添加或未启用的 Skill，不得出现在管家对话的 Skill 可选范围内；角色同理
- 关闭某个 Agent 的 Skill 不影响其他 Agent 对同一 Skill 的配置状态
- Agent Engine 自动发现项目目录和全局目录下的 SKILL.md 文件，并以当前 Agent 的配置决定是否可用
- 角色在任务需要未配置或未启用的 Skill 时，明确提示缺少什么及如何配置
- Skill 配置后，只有处于启用状态的 Skill 才能在后续任务中自动使用
- 用户可分别为管家和角色配置相互独立的 MCP Server 绑定列表以接入外部工具

#### FR-5: 角色从对话自然涌现

管家在与用户对话过程中，可以识别出隐含的角色需求，并建议用户创建新角色。创建过程优先以引导对话完成。同时提供手动创建入口——用户可通过表单直接填写角色名称、目标、职责快速创建，适用于用户已有明确想法的场景。

**Consequences (testable):**
- 用户描述某个领域的困扰后，管家在1-2轮对话内建议创建对应角色
- 角色创建通过对话引导完成（2-3轮），自动提炼出目标和职责
- 用户可以拒绝管家的角色创建建议
- 用户也可通过侧边栏"添加角色"入口手动创建角色（表单方式）

#### FR-6: 角色个性化语调

每个角色与用户对话时使用符合其身份的语调——产品经理简洁直接，父亲角色温暖鼓励，学习者角色好奇探索。

**Consequences (testable):**
- 不同角色对同一类问题的回复风格明显不同
- 用户可识别出正在对话的角色身份（无需看UI标识）

### 4.3 结构化记忆系统

**Description：** 角色从对话中实时提炼结构化知识——用户偏好、任务清单（含状态/优先级/时间约束）、认知模型。同时保留原始对话记录作为溯源依据（仅在用户追问时引用，日常交互中不加载）。记忆是活的、可查询的、持续积累的。V1实现结构化提炼层（形态分析D3-B），为V2的三层认知和认知图谱预留接口。实现UJ-2, UJ-4。

**Functional Requirements：**

#### FR-7: 对话→结构化记忆提炼

角色在每次对话后，自动从对话内容中提炼关键信息（新偏好、新任务、状态变更、认知更新）并写入结构化记忆库。

**Consequences (testable):**
- 用户在对话中提到"我下周五要交报告"后，角色的任务列表自动出现该任务及时间约束
- 用户在对话中表达偏好后（"我喜欢数据驱动的决策"），角色记忆中记录该偏好
- 提炼后的结构化记忆覆盖对话中所有显式任务、偏好声明和状态变更（可通过审计对比对话与记忆验证）

#### FR-8: 记忆可查询与溯源

用户可以查看角色的结构化记忆，追问"你为什么这么认为？"时，系统能溯源到具体的原始对话记录。系统保留完整的原始对话日志作为溯源证据，但原始日志不参与日常推理和交互——仅在溯源时按需检索引用。

**Consequences (testable):**
- 用户输入"你为什么认为这个任务最重要？"后，角色返回基于记忆的推理链
- 推理链可追溯到原始对话记录（包含具体日期、对话原文片段）
- 原始对话日志独立存储，不影响日常交互性能
- 记忆条目包含指向原始对话的索引引用

#### FR-9: 选择性遗忘

用户可以要求角色"忘记"特定记忆。遗忘不只是删除数据条目，还清除基于该数据推导的认知。 `[ASSUMPTION: V1的遗忘实现为删除记忆条目+重新运行依赖该条目的推理，完美的认知回溯清除延期至V2]`

**Consequences (testable):**
- 用户请求遗忘后，相关记忆条目不再出现
- 遗忘后的角色建议不再基于被遗忘的信息

### 4.4 角色工作循环与主动建议

**Description：** 角色不是被动等待用户指令，而是拥有后台工作循环——定时审视目标、职责和任务，识别最重要的事并主动生成建议。V1实现"角色建议+用户确认"模式（形态分析D6-B），用户可调节每个角色的主动性级别。实现UJ-2。

**Functional Requirements：**

#### FR-10: 后台工作循环

每个角色有一个可配置频率的后台循环，定时审视目标和任务状态，生成新建议或更新现有建议。 `[ASSUMPTION: V1的工作循环在应用运行时执行，不支持系统后台服务/daemon常驻]`

**Consequences (testable):**
- 角色在配置的时间间隔后产生新的建议（如每日1次）
- 建议必须基于角色当前目标、任务状态和记忆生成，非重复性内容
- 工作循环的频率可在角色设置中调整

#### FR-11: 主动建议（需用户确认）

角色生成的建议以"待确认"状态呈现给用户。用户确认后转为正式任务，拒绝后标记为已处理。

**Consequences (testable):**
- 建议以明确的"确认/拒绝"交互呈现
- 确认后建议转为角色任务列表中的正式任务
- 拒绝后角色可学习"什么类型的建议用户不需要"

#### FR-12: 主动性三档刻度盘

用户可以为每个角色调节三级主动性：
- **静默执行**：角色仅执行用户明确下达的指令，不生成任何主动建议
- **适度建议**：角色按配置频率审视目标和任务，生成建议但需用户确认后执行
- **积极主动**：角色高频审视并生成建议，低风险操作可自动执行（如整理任务列表），高风险操作仍需确认

**Consequences (testable):**
- 设为"静默执行"的角色不生成任何主动建议
- 设为"适度建议"的角色按配置频率生成建议，所有建议需用户确认
- 设为"积极主动"的角色可自动执行低风险操作，高风险操作仍呈现为待确认建议
- 主动性设置变更立即生效
- 默认值为"适度建议"

### 4.5 使命宣言与冲突仲裁

**Description：** 用户可定义个人使命宣言作为所有角色的顶层裁决依据。当角色间产生冲突时，管家执行基于使命宣言的仲裁协议，给出带理由的建议和双赢方案，但永远不替用户做决定。实现UJ-3。

**Functional Requirements：**

#### FR-13: 使命宣言设定

用户可以通过与管家的引导对话设定个人使命宣言。支持两种格式：自由文本和结构化模板（柯维角色-价值观-目标三段式），用户自选。使命宣言是可选的——未设定时管家基于行为模式推断隐性优先级。

**Consequences (testable):**
- 使命宣言设定后，存储为管家的顶层决策依据
- 用户可选择自由文本或结构化模板格式
- 用户可随时查看和编辑使命宣言
- 未设定使命宣言时，冲突仲裁仍可基于其他维度（四象限+能量值）运行

#### FR-14: 冲突检测

管家监控所有角色的任务时间安排，当检测到两个或多个角色的任务在时间/精力上产生冲突时，主动提醒用户。

**Consequences (testable):**
- 同一时间段有两个角色的任务时，管家发出冲突提醒
- 冲突提醒包含涉及的角色和具体任务

#### FR-15: 三步仲裁协议

管家在仲裁冲突时执行三步协议：①检查使命宣言优先原则 ②评估四象限分类 ③考虑角色能量平衡——然后给出带有理由的建议和双赢方案。

**Consequences (testable):**
- 仲裁建议包含引用的使命宣言原则（如有）
- 仲裁建议包含四象限分类评估
- 仲裁建议包含至少一个双赢替代方案
- 管家明确表示"决定权在你手中"

### 4.6 晨间简报与周复盘

**Description：** 管家每日生成晨间简报（自然语言，非冰冷列表），每周引导周复盘仪式（角色成绩单+大石头规划）。这两个仪式是用户与分身团队的核心触点。实现UJ-2, UJ-4。

**Functional Requirements：**

#### FR-16: 晨间简报生成

管家每日自动生成晨间简报，从各角色视角汇总当前状态、优先任务和提醒。以自然语言段落呈现，有温度感。用户可在管家设置中配置晨间简报的推送时间。

**Consequences (testable):**
- 晨间简报在用户每日首次打开app时自动展示（或在用户配置的时间触发）
- 用户可在管家设置中调整晨间简报时间
- 简报包含每个活跃角色的关键状态和当日优先事项
- 简报格式为自然语言段落，包含角色名称和具体建议
- 简报结尾有"今天最重要的一件事"建议

#### FR-17: 大石头周规划

每周初，管家引导用户为每个角色确定1-2个大石头。管家基于上周角色能量分布和未完成目标提供智能建议，用户确认或调整。

**Consequences (testable):**
- 周规划对话在每周设定时间触发，用户可在管家设置中配置触发的星期和时间
- 管家为每个角色提供大石头建议（基于历史数据）
- 用户确认后，大石头标记为本周不可妥协的优先项
- 管家在工作日中保护大石头任务不被低优先级事项挤掉

#### FR-18: 周复盘成绩单

每周末管家生成角色成绩单——能量变化趋势、大石头完成情况、新沉淀的记忆/Skill。以反思对话形式呈现。

**Consequences (testable):**
- 周复盘展示每个角色的能量变化趋势
- 复盘包含大石头完成/未完成的统计
- 复盘对话引导用户反思和调整

### 4.7 角色仪表盘与UI

**Description：** 对话流+仪表盘融合界面（形态分析D4-C）。角色以"活的人物卡片"呈现——有能量值、最近动态、待处理事项。对话区支持管家和角色切换。角色切换带有微妙的视觉氛围变化。实现UJ-1, UJ-2。

**Functional Requirements：**

#### FR-19: 角色卡片仪表盘

管家工作面板以"仪表盘"tab展示角色状态总览。仪表盘保留角色卡片视图，每张卡片显示：角色名称/图标、当前能量值（百分比+进度条）、待处理事项数、最近活跃时间；同时增加统计区，展示任务总数、记忆条目数、对话会话数和待处理任务数。统计区支持按管家或单个角色筛选，并支持时间范围筛选。卡片有视觉优先级——有紧急事项的角色使用警告色边框，能量值<40%的角色使用红色进度条，正常角色使用标准色调。点击卡片可快速跳转到该角色的对话视图。

**Consequences (testable):**
- 管家工作面板包含"仪表盘"tab，与通用任务/管家记忆/管家设置tab平级
- 每个活跃角色有独立卡片，展示能量百分比和进度条
- 统计区至少展示：任务总数、结构化记忆条目数、对话会话数、待处理任务数
- 对话数量按独立对话会话计数，不按消息条数计数
- 用户可按"全部/管家/单个角色"筛选统计数据
- 用户可选择时间范围，统计值仅包含所选范围内创建或发生的记录；默认显示全部时间
- 角色筛选和时间筛选可以组合使用，筛选后四项统计值同步更新
- 视觉优先级：紧急事项（amber边框）> 低能量<40%（红色进度条）> 正常（绿色进度条）
- 能量值≥70%显示绿色，40%-69%显示黄色，<40%显示红色
- 归档角色不显示在角色卡片中；选择全部统计时是否包含归档角色，按数据记录的归属和时间范围计数
- 点击卡片跳转到对应角色视图

#### FR-20: 对话区角色切换

点击角色卡片切换到该角色的对话视角。对话历史、任务面板切换为该角色的专属内容。点击"管家"回到全局视图。

**Consequences (testable):**
- 切换角色后，对话区显示该角色的历史对话
- 切换角色后，任务面板显示该角色的任务
- 切换回管家后，恢复全局视图

#### FR-21: 空状态引导

新用户首次打开app时，不显示空白仪表盘，而是管家直接发起引导对话（见UJ-1）。

**Consequences (testable):**
- 无角色时不显示空卡片区域
- 管家对话自动启动引导流程
- 引导完成（创建第一个角色）后，仪表盘正常展示

### 4.8 三级通知系统

**Description：** 多角色主动提醒需要精细的过滤策略，防止从"有温度的搭档"变成"烦死人的话痨"。通知分三级，管家评估紧急度和用户当前状态后选择通知级别。

**Functional Requirements：**

#### FR-22: 三级通知

- **耳语**：角色的日常更新，静默积累到晨间简报
- **轻触**：中等优先级，出现在通知面板但不弹窗打断
- **敲门**：紧急事项，弹窗提醒——每日上限3次

**Consequences (testable):**
- 耳语级消息不产生即时通知，仅体现在晨间简报
- 轻触级消息出现在通知面板，无弹窗
- 敲门级消息弹窗提醒，每日超过3次后降级为轻触
- 用户可在设置中调整每日敲门上限

### 4.9 智能四象限

**Description：** 任务不由用户手动标P0/P1，而是系统基于角色目标、截止日期、对话上下文自动归入柯维四象限。管家特别保护Q2（重要不紧急）任务不被Q1挤掉。

**Functional Requirements：**

#### FR-23: 自动四象限分类

系统基于任务属性（截止日期、角色目标关联度、历史模式）自动为每个任务分配四象限分类。分类是动态的，随时间和上下文变化。

**Consequences (testable):**
- 每个任务有四象限标签（Q1/Q2/Q3/Q4）
- 接近截止日期的任务自动升入Q1
- 用户可手动覆盖四象限分类

#### FR-24: Q2保护机制

管家在日常规划和冲突仲裁中优先保护Q2任务。当Q1任务挤压Q2时间时，管家主动提醒。

**Consequences (testable):**
- 管家在某角色连续多日Q2任务被挤掉时发出提醒
- 冲突仲裁中，Q2任务不被Q3/Q4任务挤掉

### 4.10 数据主权与信任

**Description：** 用户的认知档案是最私密的数据资产。V1从架构层确保数据主权，不是靠隐私政策文字而是靠技术不可能性。

**Functional Requirements：**

#### FR-25: 本地优先存储

所有角色记忆、任务、使命宣言、对话历史默认存储在本地SQLite数据库，不自动上传到任何云端。

**Consequences (testable):**
- 断网状态下应用完整可用
- 本地数据库文件可在文件系统中定位
- 无网络请求发送用户内容数据（LLM API调用除外）

#### FR-26: 完整数据导出

用户可一键导出所有数据（角色定义、记忆、任务、对话历史）为标准格式（JSON/Markdown）。

**Consequences (testable):**
- 导出文件包含所有角色的完整数据
- 导出格式为人类可读的JSON或Markdown
- 导出操作在30秒内完成

#### FR-27: 数据销毁

用户可一键销毁所有数据，不留残余。

**Consequences (testable):**
- 销毁后本地数据库为空
- 销毁操作需二次确认
- 销毁后应用回到初始空状态

#### FR-28: LLM模型配置

LLM Provider配置由opencode Agent Engine统一管理。opencode原生支持30+个Provider（OpenAI、Anthropic、Google Gemini、DeepSeek、Groq、Azure、Amazon Bedrock、Ollama、LM Studio等），用户在opencode.json中配置Provider即可。EgoSync UI提供友好的配置界面，底层写入opencode配置。用户可为不同角色指定不同的模型。EgoSync不存储API调用内容。

**Consequences (testable):**
- 用户可在设置中配置LLM Provider，支持opencode原生的所有Provider
- 用户可配置多个Provider，为不同角色指定不同的模型
- 支持Ollama/LM Studio等本地模型
- 未配置任何模型时，提示用户配置
- 连接测试：配置后可一键验证连通性
- Provider配置持久化为opencode.json格式，可手动编辑

### 4.11 透明审计与不确定性表达

**Description：** AI的判断不是黑箱。用户可追问推理依据，系统在信心不足时主动表达不确定性。

**Functional Requirements：**

#### FR-29: 推理溯源

用户对角色或管家的任何建议追问"为什么？"时，系统返回推理链——引用的记忆条目、使用的规则、参考的历史模式。

**Consequences (testable):**
- "为什么"追问后返回具体依据（非泛泛回复）
- 依据包含可溯源的记忆条目引用

#### FR-30: 不确定性表达

当系统对某个判断信心不足时，主动表达不确定性："我不太确定这个判断，建议你自己评估"。

**Consequences (testable):**
- 系统不在信心不足时给出过于确定的建议
- 不确定性表达出现频率合理（非每条都加限定词）

### 4.12 Agent引擎集成（opencode）

**Description：** EgoSync集成opencode作为底层Agent执行引擎，将管家和角色从"单轮tool-use"升级为"完整Agent Loop"。opencode以Tauri sidecar binary方式打包进安装包，由Rust后端管理进程生命周期，通过HTTP API通信。此集成从根本上解决当前架构无法支撑复杂任务（代码编写、多步骤研究、文件操作等）的问题，并为后续持续演进（MCP扩展、Skill生态、Subagent协作）打下基础。

**Functional Requirements：**

#### FR-31: opencode Sidecar进程管理

Tauri后端负责opencode server的完整生命周期管理：应用启动时自动拉起opencode server进程，应用退出时优雅停止。opencode binary作为Tauri sidecar打包在安装包中，无需用户额外安装。

**Consequences (testable):**
- 应用启动后，opencode server进程自动运行并监听本地端口
- 应用退出后，opencode server进程优雅终止，无孤儿进程
- Rust后端可通过HTTP调用opencode API（session/message/agent等）
- opencode进程异常退出时，Rust后端自动重启并恢复连接
- opencode binary包含在Tauri安装包中（Windows/macOS/Linux三平台）

#### FR-32: 角色→opencode Agent动态映射

每个EgoSync角色在opencode中注册为一个独立的Agent配置（subagent模式），拥有专属system prompt、model配置、permission规则和skill绑定。管家作为primary agent注册。角色创建/编辑/归档/删除时，同步更新opencode agent配置。

**Consequences (testable):**
- 创建EgoSync角色后，opencode agent配置中新增对应agent条目
- 角色的prompt/goal/skill变更同步到opencode agent配置
- 归档角色后，对应opencode agent被disable
- 删除角色后，对应opencode agent被移除
- 管家作为primary agent始终存在，拥有最高权限级别

#### FR-33: Agent Loop对话升级

管家和角色的对话从单轮tool-use升级为完整Agent Loop。Agent可自主决策调用哪些工具、执行多少步骤、何时需要用户确认。支持复杂的多步骤任务执行（代码编写、竞品研究、文件操作等）。

**Consequences (testable):**
- 用户下达复杂任务后，Agent自主执行多步工具调用直到完成
- Agent loop过程中，流式输出中间步骤和思考过程
- 任务执行过程可被用户中断（abort）
- Agent在单次loop中可调用多个不同工具（bash、read、write、grep等）

#### FR-34: 可配置权限模型

每个角色的操作权限可独立配置。默认行为为自主执行（allow），用户可将特定操作类型设置为需确认（ask）或禁止（deny）。权限粒度覆盖：文件编辑、bash命令执行、外部目录访问、Web搜索等。管家拥有全局最高权限。

**Consequences (testable):**
- 默认状态下，角色可自主执行任务，无需逐步确认
- 用户可在角色设置中将特定操作类型设为"需确认"（ask）
- 设为ask的操作触发时，UI弹出确认请求，用户批准后继续
- 设为deny的操作类型，Agent不会尝试调用
- 权限变更即时生效，无需重启session
- 管家的权限不可被降低到deny级别（保证管理能力）

#### FR-35: opencode内置工具复用

角色可使用opencode内置的工具系统，包括但不限于：文件读写（read/write/edit）、shell命令执行（bash）、代码搜索（grep/glob）、Web搜索（websearch）、网页抓取（webfetch）。工具的可用性受角色权限控制。

**Consequences (testable):**
- 角色在对话中可自主调用文件读写工具
- 角色可执行bash命令（受权限控制）
- 角色可搜索代码库（grep/glob）
- 角色可进行Web搜索和网页抓取（如已配置）
- 未授权的工具调用被拒绝并返回明确提示

#### FR-36: Session持久化与上下文管理

每个角色的对话映射为opencode session，支持上下文持久化、自动压缩（compaction）和历史消息分页。opencode的上下文管理确保长对话不丢失重要信息。

**Consequences (testable):**
- 角色对话跨应用重启后保留历史上下文
- 长对话自动触发上下文压缩，避免token溢出
- 用户可查看角色的完整对话历史（分页加载）
- 每个角色维持独立session，互不干扰

### 4.13 Skill指定、管家统计与MCP管理

**Description：** 本组需求建立在当前代码已有的管家/角色独立 Skill 配置、Skill 启用状态和 MCP Server 实体状态之上，补充对话中的显式 Skill 选择、管家工作面板统计以及 MCP Server 生命周期管理。

#### FR-37: 对话中@Skill指定

管家和角色的对话输入均支持通过@指定 Skill，用户可在当前 Agent 可用的 Skill 范围内选择本轮任务使用的 Skill。可选列表必须基于当前 Agent 已添加且已启用的 Skill 配置；管家与角色分别使用自己的配置，不共享启用状态。用户输入不存在、未添加或已关闭的 Skill 时，系统不得调用该 Skill，并给出明确提示。 `[ASSUMPTION: @Skill指定只影响当前任务，不改变Skill的长期启用状态]`

**Consequences (testable):**
- 管家和角色输入框均支持@Skill触发的 Skill 选择/补全
- 补全列表只展示当前 Agent 已添加且启用的 Skill
- 同一 Skill 在管家启用、在角色关闭时，管家可以选择，角色不可选择
- 用户指定 Skill 后，当前任务执行上下文包含该 Skill，且可审计实际使用的 Skill
- 指定未启用或未配置的 Skill 时，不发生对应 Skill 调用，并提示用户启用或添加
- 不使用@指定时，现有 Agent 自动发现和按需加载行为保持不变

#### FR-38: 管家仪表盘统计与筛选

管家仪表盘提供跨 Agent 的活动统计，并支持按 Agent 和时间范围筛选。统计指标包括任务总数、结构化记忆条目数、对话会话数和待处理任务数；其中对话数量按对话会话计数，不按消息条数计数。时间筛选以数据记录的创建/开始时间为准：任务和记忆使用 `created_at`，对话会话使用 `started_at`；待处理任务按任务的 `created_at` 纳入范围后，再按查询时的未完成状态计数。筛选条件可组合，默认展示全部时间范围。 `[ASSUMPTION: 具体时间预设和控件形式由UX阶段确定]`

**Consequences (testable):**
- 仪表盘显示任务总数、记忆数、对话会话数和待处理任务数四项指标
- 角色筛选支持全部、管家和单个角色
- 时间筛选至少支持全部时间，并允许用户选择具体时间范围
- 角色与时间筛选同时生效，四项指标随筛选条件一致更新
- 待处理任务数是任务总数的可解释子集，不能大于任务总数
- 对话会话数在同一会话包含多条消息时仍只计为1

#### FR-39: MCP Server启停与管家独立绑定

管家设置支持查看、添加、编辑、测试、删除和启用/关闭 MCP Server，并支持管家添加或移除自己的 MCP Server 绑定。MCP Server 自身的 enabled 状态、管家绑定和各角色绑定是相互独立的配置维度：管家的有效 MCP 集合仅包含“管家已绑定且 Server 已启用”的交集；角色继续按各自现有绑定逻辑使用 MCP Server，不因管家绑定变化而改变。关闭 Server 时保留管家和角色的既有绑定，但不向运行时暴露该 Server 的工具；重新启用后，原有绑定恢复可用。

**Consequences (testable):**
- 管家可以在设置中查看所有 MCP Server 及其当前启用状态
- 管家可以独立切换单个 MCP Server 的启用/关闭状态，配置保留且无需删除重建
- 管家可以查看、添加和移除自己的 MCP Server 绑定
- 管家只能新增绑定当前已启用的 MCP Server
- 管家只能使用自己已绑定且处于启用状态的 MCP Server；仅启用但未绑定的 Server 不得成为管家能力
- 管家绑定与角色绑定相互独立：管家添加或移除绑定不改变任何角色绑定，角色绑定变化也不改变管家绑定
- 关闭 MCP Server 后，Agent Engine 不得向运行时暴露该 Server 的工具，但必须保留管家和角色的既有绑定；重新启用后原绑定恢复可用
- 角色现有的 MCP Server 绑定与使用行为保持不变
- MCP Server 配置或管家绑定变化后刷新运行时配置，后续会话使用最新状态
- 配置已保存但运行时刷新失败时，界面必须明确提示配置尚未完全生效，不得静默显示为完全成功


**FR 总数：39 个编号需求（FR-1～FR-39，另含 FR-4b）。**

### Non-Functional Requirements

PRD 未定义独立编号的 NFR。可实施性评估使用 §7 Success Metrics 与 Adapt-In Guardrails 中的隐含非功能约束，包括本地优先、隐私、安全、桌面端平台、可审计性及交互响应目标。

### Additional Requirements and Constraints

## 5. Non-Goals (Explicit)

- **不是通用AI聊天机器人**——不处理与用户角色无关的通用问答
- **不是社交产品**——V1无用户间互动，不做角色/Skill分享社区
- **不是团队协作工具**——B2B/团队版延期至V2+
- **不是心理治疗工具**——检测到深层心理问题时引导至专业资源
- **不替用户做人生决策**——所有自主行动止步于"建议"，决定权永远在用户手中
- **不追求使用时长**——产品KPI不是DAU/MAU和屏幕时间
- **不做推送上瘾机制**——通知有每日上限，管家的"沉默"跟"说话"同等重要

## 6. MVP Scope

### 6.1 In Scope

- **Agent引擎集成**（FR-31, FR-32, FR-33, FR-34, FR-35, FR-36）— 基础设施，所有其他功能的执行引擎
- 管家对话与意图路由（FR-1, FR-2, FR-3）
- 角色CRUD与对话涌现创建（FR-4, FR-5, FR-6）
- 管家与角色Skill配置、对话中@Skill指定与MCP扩展（FR-4b, FR-37, FR-39）
- 结构化记忆提炼与查询（FR-7, FR-8）
- 选择性遗忘（FR-9, 简化版）
- 角色后台工作循环与主动建议（FR-10, FR-11, FR-12）
- 使命宣言与冲突仲裁（FR-13, FR-14, FR-15）
- 晨间简报（FR-16）
- 大石头周规划（FR-17）
- 周复盘成绩单（FR-18）
- 角色卡片仪表盘与任务/记忆/对话统计（FR-19, FR-20, FR-21, FR-38）
- 三级通知（FR-22）
- 智能四象限（FR-23, FR-24）
- 本地优先存储 + 数据导出/销毁（FR-25, FR-26, FR-27）
- LLM模型配置：opencode多Provider体系（FR-28）
- 推理溯源与不确定性表达（FR-29, FR-30）
- 桌面端（Tauri + opencode sidecar）

### 6.2 Out of Scope for MVP

- **移动端** — 延期至V2，先桌面验证核心体验 `[NON-GOAL for MVP]`
- **云端同步** — V2，依赖移动端需求 `[NON-GOAL for MVP]`
- **角色自主构建Skill** — V2，需要更成熟的安全审批机制
- **Skill市场/角色模板市场** — V2+，需要用户基数
- **跨角色涌现智慧** — V2+，需要长期数据积累
- **预见性建议（时序模式）** — V2+，需要长期数据
- **分身快照/时间胶囊** — V2+
- **角色退役与档案馆** — V2，V1用归档替代
- **认知遗产传承** — V3
- **数字遗嘱** — V3
- **B2B企业版** — V2+
- **端到端加密云端** — V2
- **多语言支持** — V2 `[NOTE FOR PM: V1默认中文，英文作为快速跟进]`
- **opencode TUI/Desktop App** — 仅使用opencode server API，不暴露opencode自身UI `[NON-GOAL for MVP]`
- **opencode云端企业功能** — 仅使用本地开源核心 `[NON-GOAL for MVP]`

## 7. Success Metrics

**Primary**

- **SM-1**: 首次体验时间 — 新用户从打开app到创建第一个角色的时间 ≤ 5分钟。验证FR-5, FR-21。
- **SM-2**: 角色扩展率 — 内测用户中，创建第2个角色的比例 ≥ 60%。验证FR-4, FR-5。
- **SM-3**: 周活跃留存 — 内测用户第4周仍在使用的比例 ≥ 40%。验证FR-16, FR-17, FR-18。
- **SM-4**: 晨间简报打开率 — 每日晨间简报被查看的比例 ≥ 70%。验证FR-16。

**Secondary**

- **SM-5**: 冲突仲裁采纳率 — 管家仲裁建议被用户采纳的比例 ≥ 50%。验证FR-15。
- **SM-6**: 主动建议确认率 — 角色主动建议被用户确认执行的比例 ≥ 30%。验证FR-11。
- **SM-7**: 记忆准确性 — 用户对记忆提炼准确性的主观评分 ≥ 4/5。验证FR-7, FR-8。

**Counter-metrics (do not optimize)**

- **SM-C1**: 日均使用时长 — 不追求增长。如果用户日均使用时间持续增长而生活满意度不变，可能表明产品在制造依赖而非创造价值。反制SM-3。
- **SM-C2**: 通知点击率 — 不追求最大化。高通知点击率可能意味着通知策略过于激进。反制SM-4。

## 8. Resolved Questions

| # | 问题 | 决议 |
|---|---|---|
| 1 | V1是否支持角色间间接知识传递？ | **严格隔离到V2**。V1角色间不共享任何知识，协同需求延期。 |
| 2 | 使命宣言格式？ | **均支持**。自由文本和结构化模板（柯维角色-价值观-目标三段式）并存，用户自选。 |
| 3 | 本地模型记忆提炼质量？ | **按用户配置走**。若用户选择本地模型则用本地模型处理，准确率可能受影响，系统通过不确定性表达（FR-30）告知用户。 |
| 4 | 角色能量值计算公式？ | **加权多维公式**：任务完成率(40%) + 大石头推进度(30%) + 用户互动频率(20%) + 目标更新活跃度(10%)。V1不开放用户自定义权重。 |
| 5 | 工作循环频率？ | **默认每日2次**（早晨app启动时 + 晚间设定时间）。用户可调范围：每日1次~每日4次。应用未运行时不执行。 |
| 6 | 四象限分类准确率阈值？ | **80%**。低于80%置信度时，展示系统分类建议但明确标记"不确定"，允许用户一键修正。 |
| 7 | 跨OS LLM流式输出一致性？ | **必须保持一致体验**。通过统一的流式渲染层抽象WebView差异，QA覆盖三平台。 |
| 8 | LLM Provider最小集？ | **opencode原生多Provider**：opencode内置支持30+个Provider（OpenAI/Anthropic/Google/DeepSeek/Groq/Azure/Bedrock/Ollama/LM Studio等），用户通过opencode.json配置，EgoSync UI提供友好配置界面。 |
| 9 | Agent引擎选型？ | **opencode作为sidecar**。opencode提供完整Agent Loop、20+内置工具、Skill系统、Subagent、MCP支持和权限控制，以sidecar binary打包进Tauri安装包。 |
| 10 | 任务执行的交互模式？ | **可配置权限**。默认自主执行（allow），用户可将特定操作类型设为需确认（ask）或禁止（deny）。 |
| 11 | Agent引擎引入时机？ | **立即作为Epic 2基础设施变更**。Agent引擎是所有角色能力的基础，越早引入越早解锁后续功能。 |
| 12 | 管家与角色的Skill启用关系？ | **分别配置、分别启用**。管家和每个角色可以添加不同Skill并独立维护启用状态；以当前Agent的配置为准，不采用共享启用状态。 |
| 13 | 仪表盘的对话数量口径？ | **对话会话数**，不按消息条数统计；统计支持按管家/角色和时间范围组合筛选。 |
| 14 | MCP Server、管家绑定与角色绑定是什么关系？ | **三者相互独立**。Server自身enabled状态、管家绑定和各角色绑定分别维护；管家有效集合为“管家已绑定且Server已启用”的交集，管家与角色的绑定变化互不影响。 |

## 9. Assumptions Index

- `[ASSUMPTION]` §4.1 FR-3: 管家语调通过System Prompt控制，V1不支持用户自定义管家人格
- `[ASSUMPTION]` §4.3 FR-9: V1的遗忘实现为删除记忆条目+重新运行依赖推理，完美认知回溯清除延期至V2
- `[ASSUMPTION]` §4.4 FR-10: V1的工作循环在应用运行时执行，不支持系统后台服务/daemon常驻
- `[ASSUMPTION]` §4.3 FR-8: 原始对话日志保留用于溯源，存储空间随使用时长线性增长，V2考虑归档/压缩策略
- `[ASSUMPTION]` §4.2 FR-4b: Skill基于opencode SKILL.md格式，角色自主构建Skill延期至V2
- `[ASSUMPTION]` §4.12 FR-31: opencode binary以Tauri sidecar方式分发，安装包体积将增加约50-80MB（opencode compiled binary size）
- `[ASSUMPTION]` §4.12 FR-31: opencode server占用一个本地端口（默认4096），与其他本地开发工具可能冲突时需可配置
- `[ASSUMPTION]` §4.12 FR-32: 角色→Agent映射通过修改opencode.json配置实现，运行时热加载，不需要重启opencode进程
- `[ASSUMPTION]` §4.12 FR-36: opencode自身使用SQLite存储session/message数据，与EgoSync主数据库独立，数据一致性通过Rust后端编排层保证
- `[ASSUMPTION]` §4.7 / FR-38: 仪表盘默认显示全部时间范围；具体时间筛选控件和预设范围由UX阶段确定。统计时间字段按任务/记忆 `created_at`、对话会话 `started_at` 执行
- `[ASSUMPTION]` §4.13 / FR-37: @Skill指定优先影响当前任务，不改变Skill的长期启用状态
- `[ASSUMPTION]` §4.13 / FR-39: MCP Server的enabled状态作为Server自身运行时可用性开关；管家绑定和各角色绑定分别作为独立的使用范围配置，关闭Server时保留既有绑定

---

## Adapt-In: Constraints and Guardrails

### Privacy

- 所有用户数据本地存储，零云端依赖（V1）
- LLM API调用不存储用户对话内容（依赖Provider的隐私政策）
- 开源核心引擎，数据处理逻辑可审计
- "永不卖数据"作为品牌核心承诺

### Safety

- 分身永远不替用户做人生决策（FR-15: "决定权在你手中"）
- 检测到用户深层心理困扰时引导至专业资源，不假装心理治疗
- 不确定性表达机制（FR-30），防止AI幻觉导致信任崩塌
- 诤友机制：角色有义务告诉用户不想听的真相

### Cost

- 本地优先架构大幅降低平台运营成本（推理在用户设备/API Key）
- V1无服务器成本（纯本地应用）
- BYOK模式下LLM成本由用户自控

## Adapt-In: Aesthetic and Tone

- **整体风格**：专业但有温度，像一个值得信赖的"老管家"——不花哨、不卖萌、不说废话
- **色调方向**：默认深色主题（保护专注力），支持深色/浅色主题手动切换；角色切换时有微妙的色温变化（工作偏冷、家庭偏暖）
- **信息密度**：对话流轻量（像聊天），仪表盘信息密集（像控制台）——两种模式自然切换
- **动效原则**：角色卡片有"呼吸感"（活的实体），其他动效克制

## Adapt-In: Monetization

- **V1**：完全免费，本地运行
- **V2+付费层**：
  - 云端同步（¥15/月）
  - 专业版（¥39/月）：无限角色、高级Skill、高频工作循环
  - 家庭版（¥69/月）：独立实例+可选协同
- **BYOK**：用户自带API Key，平台不赚差价
- **生态收入（V2+）**：Skill市场/角色模板市场抽成30%

## Adapt-In: Platform

- **V1**：桌面端（Tauri + opencode sidecar），Windows/macOS/Linux
- **V2**：移动端（React Native或Flutter），iOS/Android
- **V2**：云端同步层（端到端加密）

### PRD Completeness Assessment

FR-37～FR-39 均具有明确行为描述与可测试后果；FR-39 进一步明确 Server enabled、管家绑定、角色绑定为三个独立维度。风险在于 PRD 没有编号化 NFR，相关约束只能从成功指标、假设和 Guardrails 追踪。


## 3. Epic Coverage Validation

### Coverage Matrix

### FR Coverage Map

| FR | 主 Epic | 副 Epic | 描述 |
|----|---------|---------|------|
| FR-1 | E1 | E2 | 管家意图解析与路由（基础版→E1，完整版→E2） |
| FR-2 | E2 | — | 双通道任务分配 |
| FR-3 | E1 | — | 管家人格化语调 |
| FR-4 | E1 | E2 | 角色 CRUD（Create→E1，U/D/Archive→E2） |
| FR-4b | E2 | — | 角色 Skill 配置 |
| FR-5 | E1 | E2 | 角色从对话涌现（首次→E1，持续→E2） |
| FR-6 | E2 | — | 角色个性化语调 |
| FR-7 | E2 | — | 对话→记忆提炼 |
| FR-8 | E2 | — | 记忆查询与溯源 |
| FR-9 | E2 | — | 选择性遗忘 |
| FR-10 | E4 | — | 后台工作循环 |
| FR-11 | E4 | — | 主动建议 |
| FR-12 | E4 | E2（UI） | 主动性档位（UI→E2，行为接通→E4） |
| FR-13 | E5 | — | 使命宣言 |
| FR-14 | E5 | — | 冲突检测 |
| FR-15 | E5 | — | 三步仲裁 |
| FR-16 | E6 | — | 晨间简报 |
| FR-17 | E6 | — | 大石头周规划 |
| FR-18 | E6 | — | 周复盘 |
| FR-19 | E4 | — | 角色卡片仪表盘 |
| FR-20 | E2 | — | 对话区角色切换 |
| FR-21 | E1 | — | 空状态引导 |
| FR-22 | E4 | — | 三级通知 |
| FR-23 | E3 | — | 自动四象限 |
| FR-24 | E3 | E5 | Q2 保护（属性→E3，仲裁应用→E5） |
| FR-25 | E1 | — | 本地优先存储 |
| FR-26 | E7 | — | 数据导出 |
| FR-27 | E7 | — | 数据销毁 |
| FR-28 | E1 | — | LLM 模型配置 |
| FR-29 | E2 | — | 推理溯源 |
| FR-30 | E2 | — | 不确定性表达 |
| FR-31 | E2 | — | opencode Sidecar 进程管理 |
| FR-32 | E2 | — | 角色→opencode Agent 动态映射 |
| FR-33 | E2 | — | Agent Loop 对话升级 |
| FR-34 | E2 | — | 可配置权限模型 |
| FR-35 | E2 | — | opencode 内置工具复用 |
| FR-36 | E2 | — | Session 持久化与上下文管理 |
| FR-37 | E10 | — | 对话级 @Skill 选择、校验、注入与审计 |
| FR-38 | E11 | — | 跨 Agent 四项统计及 Agent/时间组合筛选 |
| FR-39 | E10 | — | MCP Server 生命周期、管家独立绑定与运行时同步 |

✅ **39 个 FR 全部映射，无孤儿。**

### Missing Requirements

未发现未映射的 PRD FR。FR-37→Epic 10/Story 10.1，FR-38→Epic 11/Story 11.1，FR-39→Epic 10/Story 10.2。

### Coverage Statistics

- PRD 编号范围：FR-1～FR-39（另含 FR-4b）
- Epics 已映射：全部
- 覆盖率：100%


## 4. UX Alignment Assessment

### UX Document Status

已找到 `ux-design-specification.md`，但未发现 FR-37、FR-38、FR-39、`@Skill`、MCP Server 管家绑定或统计时间筛选的增量 UX 规范。

### Alignment Issues

1. **FR-37 缺少 UX 定义**：未规定 `@Skill` 选择器入口、候选范围、禁用/不存在 Skill 的错误提示、已选 Skill 展示及本轮作用域提示。
2. **FR-38 缺少 UX 定义**：未规定 Agent 筛选器、时间范围控件、空结果/loading/error 状态以及四项指标的布局和刷新反馈。
3. **FR-39 缺少 UX 定义**：未规定 Server enabled 开关与管家绑定控件如何区分、关闭时保留绑定的可见反馈、刷新失败的部分成功提示。

### Architecture Support

Architecture 已明确沿用 `React → Tauri/Rust → opencode`，并为三项需求定义 service/command/runtime 边界、可信后端校验、统计口径、MCP enabled 与绑定交集以及显式失败语义，技术上可支持所需 UX。

### Warnings

- Architecture 的 Gap Analysis 仍写有“PRD FR-39 尚未补充管家绑定”，该陈述已被 2026-07-23 更新后的 PRD 覆盖，属于待清理的过时说明；本评估采用更新后的 PRD。
- UX 文档未同步本轮增量。Story 可以实施，但 UI 细节若仅留给开发 Agent 决定，会产生验收歧义。


## 5. Epic Quality Review

### Epic 10 / Story 10.1（FR-37）

- **覆盖完整**：候选集合隔离、后端可信校验、请求级快照、失效竞态、未指定兼容路径、opencode 原生执行与审计均已覆盖。
- **架构一致**：遵循 React 选择器 → Tauri `ChatRequest.selectedSkillId` → Rust 作用域校验 → `AgentBridge` → opencode command；没有新增 Skill 执行引擎。
- **依赖**：仅依赖既有 Skill Registry、管家/角色 Skill 配置和 opencode 会话链路，均为既有或前序能力，无前向依赖。
- **尺寸**：跨前后端及测试，但属于单一垂直能力，单个开发 Agent 可完成。

### Epic 10 / Story 10.2（FR-39）

- **覆盖完整**：包含管家独立绑定、enabled 与绑定隔离、有效集合交集、关闭保留绑定、重新启用恢复、Runtime 同步、部分失败、幂等、导入导出与旧数据兼容。
- **没有误报既有能力**：当前代码已经具有 MCP Server list/create/update/delete/test、`enabled` 更新、角色绑定、`AgentConfigService`、`OpencodeMcpScopeLock` 和 Runtime refresh。Story 明确要求“复用现有链路而不重复实现”，因此 CRUD、测试和启停是回归/集成验收，不是新增后端能力。
- **架构一致**：新增范围应限于管家绑定表与接口、管家 UI、配置投影扩展、数据主权闭环，以及对现有刷新失败语义的修正；不得新建第二套 MCP 管理器或同步器。
- **依赖**：依赖均来自既有 Epic 2 MCP 能力及既有 Epic 7 导入导出，无前向依赖。
- **主要尺寸风险**：Migration、Repository、Service、Commands、UI、Runtime、部分失败重试、导入导出、兼容性和全层测试被放入一个 Story。逻辑上独立，但对单个开发 Agent 来说接近 Epic-sized，容易超过单次实现上下文并造成验证遗漏。
- **整改建议**：至少将“管家绑定垂直链路”和“导入导出/旧数据兼容”拆为两个按顺序实施的 Story；后一个只依赖前一个，不形成前向依赖。若不拆分，必须提供明确文件清单、分阶段检查点和测试命令作为执行条件。

### Epic 11 / Story 11.1（FR-38）

- **覆盖完整**：四项指标、会话口径、Agent/时间组合筛选、半开区间、失效角色回退、无效输入、跨库全失败策略和请求竞态均已覆盖。
- **架构一致**：React → Tauri Command → Dashboard Aggregation Service → 主库/Conversations Repository，前端不直接聚合数据源。
- **依赖**：只依赖已有任务、记忆、会话和 Dashboard 能力，无前向依赖。
- **尺寸**：范围较大但内聚，可由单个开发 Agent 完成；应把跨数据库失败测试和旧请求覆盖新结果测试列为强制验收。

### Quality Findings by Severity

#### 🔴 Critical Violations

无。

#### 🟠 Major Issues

1. **Story 10.2 尺寸过大**，单 Agent 执行风险高。建议拆分，或以强制分阶段检查点作为实施条件。
2. **UX 文档未覆盖 FR-37～FR-39**，关键控件和错误/部分成功状态只能从 Story AC 推断。

#### 🟡 Minor Concerns

1. Architecture 中“PRD FR-39 尚未更新”的说明已过时，应清理以避免实施 Agent 误判。
2. PRD 没有编号化 NFR，非功能约束追踪依赖 Architecture 和 Epics 的 NFR-14～NFR-20。

### Best-Practice Checklist

| 检查项 | Epic 10 / 10.1 | Epic 10 / 10.2 | Epic 11 / 11.1 |
|---|---|---|---|
| 用户价值明确 | 通过 | 通过 | 通过 |
| FR 可追踪 | 通过 | 通过 | 通过 |
| 无前向依赖 | 通过 | 通过 | 通过 |
| 符合既有架构 | 通过 | 通过 | 通过 |
| 未重复实现既有能力 | 通过 | 通过 | 通过 |
| 单 Agent 尺寸 | 通过 | **有条件** | 通过 |
| AC 可测试 | 通过 | 通过 | 通过 |


## 6. Summary and Recommendations

### Overall Readiness Status

# READY WITH CONDITIONS

FR-37～FR-39 的 PRD、Architecture 与 Epics/Stories 在需求覆盖和技术路径上已基本完整，三个 FR 均具有明确实现入口，且没有前向依赖。当前不判定为 READY，原因是 UX 增量规范缺失，以及 Story 10.2 对单个开发 Agent 的范围过大。

### Critical Issues Requiring Immediate Action

无阻断性 Critical Issue。

### Conditions Before Implementation

1. **处理 Story 10.2 尺寸风险**：推荐拆分为“管家 MCP 绑定与 Runtime 垂直链路”和“导入导出/旧数据兼容”两个顺序 Story；若保持单 Story，必须在 Story 中加入文件级任务清单、阶段检查点及分层测试命令。
2. **补充 FR-37～FR-39 UX 增量规范**：至少定义 `@Skill` 选择器、Dashboard Agent/时间筛选、MCP enabled 与管家绑定的视觉区分，以及空状态、错误、部分成功与刷新重试反馈。

### Recommended Cleanup

1. 删除或更新 Architecture 中“PRD FR-39 尚未补充管家绑定”的过时 Gap；2026-07-23 PRD 已完成同步。
2. 在 Story 10.2 中持续强调：MCP Server CRUD、测试、enabled 启停、角色绑定和 Runtime refresh 均为既有能力；新增实现不得复制这些链路。
3. 实施后执行 Rust、前端组件和关键 E2E 测试；本次为文档/架构就绪度评估，没有宣称实现测试已通过。

### Final Traceability Decision

| 需求 | PRD | Architecture | UX | Epic/Story | 结论 |
|---|---|---|---|---|---|
| FR-37 | 完整 | 完整 | 缺增量规范 | E10 / 10.1 完整 | 有条件就绪 |
| FR-38 | 完整 | 完整 | 缺增量规范 | E11 / 11.1 完整 | 有条件就绪 |
| FR-39 | 完整 | 完整；含一处过时说明 | 缺增量规范 | E10 / 10.2 完整但过大 | 有条件就绪 |

### Assessment Metadata

- 评估日期：2026-07-23
- 评估者：Codex / BMAD Implementation Readiness
- 发现：2 项 Major Issue、2 项 Minor Concern，分布于 Story sizing、UX alignment、文档一致性和 NFR traceability 四类。

