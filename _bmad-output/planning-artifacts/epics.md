---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - prd-egosync.md
  - architecture.md
  - ux-design-specification.md
  - egosync-app/src/App.tsx
project_name: EgoSync
date: 2026-05-25
user_name: boss
---

# EgoSync - Epic Breakdown

## Overview

本文档基于 PRD（36 个 FR）、Architecture（Tauri 2.x + React 18 + Rust 后端 + opencode sidecar 架构）、UX Design Specification（"宁静书房"布局 + shadcn/ui 设计系统）和**已实现的高保真前端原型 `egosync-app/src/App.tsx`（1458 行，21 个组件）**，将需求分解为可由开发者执行的 Epic 和 Story 列表。

## ⚠️ 关键实现约束（影响所有 Story 设计）

**前端原型已完整实现：** `egosync-app/src/App.tsx` 已包含 21 个 React 组件，覆盖管家视角、角色视角、所有弹窗、引导流程、通知面板、LLM 配置 UI 等。样式系统（Tailwind + 角色色温 + 圆角 + 动效）和交互（视图切换、Modal 开关、Tab 切换、Mock 对话流）均已就绪。

**因此本 Epic 拆分遵循以下约束：**

1. **UI 实现零重做** — 不允许"重新构建 RoleCard 组件"类故事；只允许"从 `App.tsx` 抽取 `RoleCard` 到 `components/role/RoleCard.tsx`"。
2. **前端工作 = 拆分 + 接 API** — Story 1.x 完成 `App.tsx` 单文件 → 域目录组件的拆分；后续 Story 在拆分好的组件内将 mock state 替换为 Tauri `invoke()` 调用。
3. **视觉零回归是验收硬条件** — 任何前端 Story 必须通过"拆分前/拆分后视觉一致"截图对比。
4. **后端 Story 是主战场** — Rust 端的 commands/services/db/llm 模块需要 0→1 实现。
5. **`App.tsx` 顶部 Pitch Mode Bar 不进 V1** — 那是演示用的场景切换器（onboard/daily/conflict/review），生产构建移除。
6. **mock 数据需绘出对照表** — 拆分时把 `DEFAULT_ROLES`、`ROLE_TASKS`、`NotificationPanel.notifications` 等硬编码常量列为待替换清单。

## Requirements Inventory

### Functional Requirements

来源：`prd-egosync.md` §4 Features

**4.1 管家对话与路由**
- **FR-1**: 自然语言意图解析与路由 — 管家接收用户输入，解析意图并路由到对应角色；意图模糊时主动追问澄清；路由决策可审计
- **FR-2**: 双通道任务分配 — 用户可通过管家分发任务到角色，也可直接切换角色下达任务；角色接收的直接任务自动同步给管家
- **FR-3**: 管家人格化语调 — 管家对话语调稳重、可靠、有温度，通过 System Prompt 控制；V1 不支持用户自定义管家人格

**4.2 角色管理**
- **FR-4**: 角色 CRUD — 创建/编辑/归档/永久删除角色；归档保留记忆可恢复；永久删除需二次确认
- **FR-4b**: 角色 Skill 配置 — 基于 opencode SKILL.md 格式；三来源（内置/用户自定义/opencode 生态）；Agent 自动发现；MCP 外部工具接入；角色缺 Skill 时主动提示
- **FR-5**: 角色从对话自然涌现 — 管家从对话中识别角色需求并建议创建（2-3 轮引导）；同时提供手动表单创建入口
- **FR-6**: 角色个性化语调 — 不同角色用符合身份的语调（产品经理简洁、父亲温暖、学习者好奇）

**4.3 结构化记忆系统**
- **FR-7**: 对话 → 结构化记忆提炼 — 每次对话后自动提炼偏好/任务/状态变更/认知更新写入记忆库
- **FR-8**: 记忆可查询与溯源 — 用户可查看记忆并追问"为什么"；保留原始对话日志独立存储；推理链可溯源到对话原文
- **FR-9**: 选择性遗忘 — 用户可要求"忘记"特定记忆，删除可见条目并记录同源屏蔽，防止旧对话再次提炼回流（V1 简化版，不重跑依赖推理）

**4.4 角色工作循环与主动建议**
- **FR-10**: 后台工作循环 — 角色按配置频率（默认每日 2 次，1-4 次可调）审视目标和任务生成建议；仅应用运行时执行
- **FR-11**: 主动建议（需用户确认）— 建议以"待确认"状态呈现；确认转任务，拒绝标记已处理供学习
- **FR-12**: 主动性三档刻度盘 — 静默执行/适度建议/积极主动；默认"适度建议"；变更立即生效

**4.5 使命宣言与冲突仲裁**
- **FR-13**: 使命宣言设定 — 自由文本或柯维三段式结构化模板二选一；可选；未设定时基于行为推断
- **FR-14**: 冲突检测 — 监控所有角色任务的时间冲突，主动提醒涉及角色和任务
- **FR-15**: 三步仲裁协议 — ①使命宣言 ②四象限 ③能量平衡 → 带理由的建议 + 双赢方案；明确"决定权在你手中"

**4.6 晨间简报与周复盘**
- **FR-16**: 晨间简报生成 — 每日自动生成自然语言段落简报；推送时间可配置；含"今天最重要的一件事"
- **FR-17**: 大石头周规划 — 每周触发引导对话，为每个角色定 1-2 个大石头；触发时间可配置；本周不可妥协
- **FR-18**: 周复盘成绩单 — 周末生成能量趋势 + 大石头完成情况 + 新记忆/Skill；反思对话形式

**4.7 角色仪表盘与 UI**
- **FR-19**: 角色卡片仪表盘 — "仪表盘"tab 展示活跃角色卡片（名称/图标/能量条/待办数/最近活跃）；视觉优先级（紧急 amber > 低能量 < 40% 红 > 正常绿）
- **FR-20**: 对话区角色切换 — 点击角色卡片切换对话视角，对话历史和任务面板同步切换；点击管家回到全局
- **FR-21**: 空状态引导 — 新用户首次打开自动启动管家引导对话，不显示空白仪表盘

**4.8 三级通知系统**
- **FR-22**: 三级通知 — 耳语（积累到晨间简报）/ 轻触（通知面板，无弹窗）/ 敲门（弹窗，每日上限 3 次，超限降级）；上限可调

**4.9 智能四象限**
- **FR-23**: 自动四象限分类 — 系统基于截止日期/角色目标关联/历史模式自动分类 Q1-Q4；动态调整；用户可手动覆盖；置信度 < 80% 标记"不确定"
- **FR-24**: Q2 保护机制 — 仲裁中保护 Q2 不被 Q3/Q4 挤掉；连续多日 Q2 被挤主动提醒

**4.10 数据主权与信任**
- **FR-25**: 本地优先存储 — 所有数据本地 SQLite；断网完整可用；无内容数据上传（LLM API 除外）
- **FR-26**: 完整数据导出 — 一键导出全部数据为 JSON/Markdown；30 秒内完成
- **FR-27**: 数据销毁 — 一键销毁，二次确认；销毁后应用回到初始状态
- **FR-28**: LLM 模型配置 — 由 opencode Agent Engine 统一管理，原生支持 30+ Provider；EgoSync UI 提供友好配置界面写入 opencode.json；支持 Ollama/LM Studio 本地模型；多 Provider 配置；连接测试

**4.11 透明审计与不确定性表达**
- **FR-29**: 推理溯源 — "为什么"追问返回引用的记忆条目和规则推理链
- **FR-30**: 不确定性表达 — 信心不足时主动表达"我不太确定，建议你自己评估"

**4.12 Agent 引擎集成（opencode）**
- **FR-31**: opencode Sidecar 进程管理 — Tauri 后端管理 opencode server 生命周期（启动/停止/健康检查/异常重启）；opencode binary 作为 Tauri sidecar 打包，无需用户安装
- **FR-32**: 角色→opencode Agent 动态映射 — 每个角色注册为 opencode subagent（专属 prompt/model/permission/skill）；管家作为 primary agent；角色 CRUD 时同步更新 opencode agent 配置
- **FR-33**: Agent Loop 对话升级 — 从单轮 tool-use 升级为完整 Agent Loop；Agent 自主决策工具调用和步骤数；支持复杂多步任务（代码编写/研究/文件操作）；可中断
- **FR-34**: 可配置权限模型 — 默认自主执行(allow)，用户可设特定操作为需确认(ask)或禁止(deny)；权限粒度覆盖文件编辑/bash/外部目录/Web 搜索
- **FR-35**: opencode 内置工具复用 — 角色可用 read/write/edit/bash/grep/glob/websearch/webfetch/skill/task 等 20+ 内置工具；工具可用性受权限控制
- **FR-36**: Session 持久化与上下文管理 — 角色对话映射为 opencode session；支持上下文自动压缩(compaction)和历史消息分页；跨应用重启保留

### NonFunctional Requirements

来源：`prd-egosync.md` §Adapt-In 和 `architecture.md` §NFR

- **NFR-1 本地优先**：零云端依赖，断网完整可用（LLM API 调用除外）
- **NFR-2 桌面端 Tauri**：Tauri 2.x + Rust 后端 + WebView 前端，三平台（Windows/macOS/Linux）一致体验
- **NFR-3 BYOK**：用户自带 API Key，平台不做中间商、不抽差价
- **NFR-4 性能 - 流式渲染**：LLM 流式 token 实时渲染，跨平台流式输出统一抽象 WebView 差异
- **NFR-5 性能 - 动效**：角色卡片 60fps 呼吸动效，CSS transition + GPU 加速，不阻塞渲染
- **NFR-6 数据隐私**：所有数据本地 SQLite，LLM 调用不存储用户对话内容
- **NFR-7 安全 - API Key 存储**：使用系统钥匙串（keyring 3.x，Windows Credential Manager / macOS Keychain / Linux Secret Service），不落明文文件
- **NFR-8 安全 - V1 不加密 DB**：依赖 OS 文件权限；V2 可选 SQLCipher
- **NFR-9 首次体验时间**：新用户从打开到创建第一个角色 ≤ 5 分钟（SM-1）
- **NFR-10 V1 商业模式**：完全免费，纯本地运行
- **NFR-11 跨平台流式一致性**：统一的流式渲染层抽象 WebView 差异，QA 覆盖三平台
- **NFR-12 不替用户做决策**：所有自主行动止步于"建议"
- **NFR-13 不追求使用时长**：KPI 不是 DAU/MAU/屏幕时间（含反指标 SM-C1/SM-C2）

### Additional Requirements

来源：`architecture.md` §Starter Template、§Implementation Sequence、§Validation Gaps

**Starter Template & 项目初始化（影响 Epic 1 Story 1）：**
- 保留现有 `egosync-app/` 前端原型，通过 `npx tauri init` 附加 Rust 后端层（**非** greenfield 模板）
- 配置 `devUrl=http://localhost:5173`, `devCommand="npm run dev"`, `buildCommand="npm run build"`
- Rust 依赖：tauri 2.x、serde、sqlx (sqlite + runtime-tokio)、tokio、uuid、keyring 3.x、tracing、async-trait

**基础设施 & 数据：**
- SQLite schema 设计与迁移：主数据库 `egosync.db`（roles/memories/tasks/suggestions/notifications/mission/llm_configs/app_settings）+ 对话日志库 `conversations.db`（conversations/messages）
- SQLx migration 文件版本化（`{seq}_{description}.sql`）
- 主 DB 与对话日志 DB 分离

**LLM 集成层（由 opencode 统一管理）：**
- opencode 原生支持 30+ Provider（OpenAI/Anthropic/Google/DeepSeek/Groq/Azure/Bedrock/Ollama/LM Studio 等）
- EgoSync 通过写入 opencode.json 配置 Provider，opencode 热加载
- API Key 由 EgoSync keyring 管理，启动时注入 opencode 环境变量
- LLM 流式输出：opencode SSE stream → Rust AgentBridge 解析 → Tauri Event (`llm:stream`)
- 保留 LlmProvider trait 作为降级兼容层（连接测试、opencode 不可用时的基础对话）

**Agent 引擎（opencode sidecar）：**
- opencode binary 作为 Tauri sidecar 打包（`resources/opencode`），三平台
- Rust 后端 `sidecar.rs` 管理进程生命周期（spawn/health check/restart/kill）
- Rust 后端 `agent_bridge.rs` 封装 opencode HTTP API（session/message/agent/config）
- 管家映射为 opencode primary agent，角色映射为 subagent
- `agent_config.rs` 动态管理 opencode.json（角色 CRUD 时同步）
- 权限模型：EgoSync UI 配置 → opencode permission 规则（allow/ask/deny）
- opencode 内置工具（read/write/edit/bash/grep/glob/websearch 等）直接可用
- Skill 体系：opencode SKILL.md 格式，EgoSync 内置 skills + 用户自定义 skills

**调度器：**
- `tokio::interval` 实现工作循环
- 应用运行时执行（不是系统 daemon）
- 每日 1-4 次频率可调

**前端组件拆分（Epic 0 - 一次性基础工作）：**
- `App.tsx` 1458 行单文件 → 按域拆分为 `components/{layout,butler,role,chat,tasks,modals,notifications,onboarding,settings}/`
- 抽取 hooks/services/types 到对应目录
- 顶部 Pitch Mode bar 移除（演示用，不进 V1）
- 视觉零回归验收

**IPC 协议：**
- 按域分模块的 Tauri Command：`chat::send_message`、`role::create` 等
- 类型化错误枚举 `AppError`，序列化为 JSON
- Event 命名 `{domain}:{verb_past}`
- 前端 service 层封装所有 invoke 调用，组件不直接调用

**测试要求（V1 必需）：**
- 单元测试：Vitest（前端） + cargo test（后端）
- 集成测试：`src-tauri/tests/test_{domain}.rs`
- E2E 测试：Tauri driver（WebDriver 协议），覆盖冷启动引导/管家对话/角色 CRUD/LLM 流式响应/任务管理

**CI/CD：**
- GitHub Actions 三平台并行构建（Windows/macOS/Linux）
- 自动化测试链路（Rust + Vitest + E2E）
- Release artifact 自动发布

**分发：**
- Tauri bundler 生成平台安装包：MSI（Windows）、DMG（macOS）、AppImage（Linux）

### UX Design Requirements

来源：`ux-design-specification.md`

**注意：因 `App.tsx` 原型已实现视觉层，以下 UX-DR 在故事粒度上整合为"组件拆分 Story"或"细节微调 Story"，不再单独为每个组件创建从零构建的故事。**

- **UX-DR1**: 设计 Token 体系实现 — Tailwind 配置 CSS 变量（色彩/间距/圆角/动效时长/角色色温），支持浅色+深色双主题切换（原型已有 `theme` state，需迁移到 ThemeProvider）
- **UX-DR2**: 角色色温系统 — 进入角色后 `--role-accent` 和 `--role-bg-tint` 动态更新，过渡 300ms（原型在 `COLOR_OPTIONS` 数组已硬编码，需提取到 token）
- **UX-DR3**: 能量值色谱 — 高（80-100%）翠绿 / 中（40-79%）琥珀 / 低（0-39%）暗淡灰（非红色）
- **UX-DR4**: 字体系统加载 — Inter + Noto Sans SC + JetBrains Mono，配置 `font-display: swap`
- **UX-DR5**: ButlerSummary 组件抽取 — 从 `App.tsx::ButlerView` 抽取居中摘要部分到 `components/butler/ButlerSummary.tsx`
- **UX-DR6**: ActionCard 组件抽取 — 从 `App.tsx::ActionCard` 抽取到 `components/butler/ActionCard.tsx`（独立于 ButlerView）
- **UX-DR7**: RoleSidebarIcon 组件抽取 — 从 `App.tsx::Sidebar` 抽取角色图标渲染逻辑到 `components/layout/RoleSidebarIcon.tsx`，含呼吸动画（CSS keyframe）
- **UX-DR8**: ChatStream 组件实现 — 流式对话组件，`isStreaming` + `streamContent` 状态，监听 `llm:stream` 事件（原型未实现流式，需新增 `components/chat/ChatStream.tsx` 替换 ButlerView 内 mock 消息）
- **UX-DR9**: ChatInput 组件抽取与增强 — 从 `App.tsx::ButlerView`/`RoleView` 内联输入框抽取到 `components/chat/ChatInput.tsx`，高度自适应（48-120px）
- **UX-DR10**: ArbitrationDialog 组件迁移 — `App.tsx::ArbitrationModal` 迁移到 `components/modals/ArbitrationModal.tsx`（已实现三步可视化）
- **UX-DR11**: WeeklyReview 组件迁移 — `App.tsx::WeeklyReviewModal` 迁移到 `components/modals/WeeklyReviewModal.tsx`
- **UX-DR12**: EnergyIndicator 组件抽取 — 从 Sidebar 和 ButlerView 中重复出现的能量条渲染抽取到 `components/role/EnergyIndicator.tsx`，支持 small/standard 变体
- **UX-DR13**: RoleHeader 组件抽取 — 从 `RoleView` 顶部抽取到 `components/role/RoleHeader.tsx`，含色温 tint 背景
- **UX-DR14**: RoleWorkspacePanel 组件迁移 — `App.tsx::RoleWorkspacePanel` 迁移到 `components/role/RoleWorkspacePanel.tsx`，含 Tasks/Memory/Settings 三 Tab
- **UX-DR15**: 按钮层级系统统一 — Primary/Ghost/Text/Destructive 四级，圆角 6px（原型已有但未抽象，需提取为 shadcn `Button` variants）
- **UX-DR16**: 反馈模式约束 — 所有反馈通过管家自然语言传达，不使用传统 toast/snackbar（影响所有 Story 的错误提示设计）
- **UX-DR17**: 导航深度约束 — 永远不超过 2 层（管家视角 ↔ 角色视图），无角色内子页面
- **UX-DR18**: 空状态文案约束 — 不显示"暂无数据"，使用有温度文案（如"今天一切平稳"）
- **UX-DR19**: 加载状态 — 流式输出光标闪烁，不使用全屏 loading/spinner
- **UX-DR20**: 三级通知视觉 — 耳语（侧边栏绿点）/轻触（管家摘要提及）/敲门（ActionCard），不使用系统弹窗或红色 badge
- **UX-DR21**: 过渡动效体系 — 卡片 hover translateY(-1px) / 视图淡入淡出 250ms / 色温 300ms / 呼吸 3s / 气泡滑入 200ms（CSS transition + keyframe，不引入额外动画库）
- **UX-DR22**: WCAG 2.1 AA 无障碍 — 对比度 4.5:1+ / 色盲友好（不仅靠颜色，加图标形状）/ Tab 键盘导航 / aria-label / `role="log" aria-live="polite"` 用于流式输出
- **UX-DR23**: `prefers-reduced-motion` 支持 — CSS media query 关闭所有动效和过渡
- **UX-DR24**: shadcn/ui 基础组件接入 — Button/Card/Dialog/Input/Tooltip/Sheet/ScrollArea/Separator/Toggle（替换原型中的纯 div+Tailwind 实现，规范交互行为）
- **UX-DR25**: Pitch Mode bar 移除 — 生产构建移除 `App.tsx` 顶部 12 行 PITCH SCENARIO BAR

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

✅ **36 个 FR 全部映射，无孤儿。**

## Epic List

### Epic 1: 基础平台与首次对话（Foundation & First Conversation）

用户能下载安装应用，配置 LLM，完成 5 分钟冷启动引导对话，创建第一个角色，看到流式响应。完成后是一个最小可用的 AI 助手。

**FRs covered:** FR-1（基础版）, FR-3, FR-4（Create）, FR-5（首次涌现）, FR-21, FR-25, FR-28

**NFRs covered:** NFR-1, NFR-2, NFR-3, NFR-4, NFR-6, NFR-7, NFR-9

**UX-DRs covered:** UX-DR1, UX-DR2, UX-DR4, UX-DR7, UX-DR8（新建）, UX-DR15, UX-DR21, UX-DR23, UX-DR24, UX-DR25

**核心交付：**
- Tauri 2.x 集成现有 `egosync-app/` 前端 + Rust 后端骨架（`npx tauri init` 附加，非 greenfield）
- `App.tsx` 拆分为域目录（`components/{layout,butler,role,chat,modals,onboarding,settings}/`）
- SQLite 双库 schema + migrations（主库 `egosync.db` + 对话日志库 `conversations.db`）
- LLM Provider trait + OpenAi/Anthropic 策略实现 + keyring 安全存储
- LLM 配置 UI 接通（GlobalSettingsModal LLM tab + 连接测试）
- 管家 Agent 引擎 + System Prompt 分层 + Tauri Event 流式输出（`llm:stream`）+ 会话管理（新建/切换/删除）+ LLM 自动标题生成 + 流式中断（CancellationToken）
- 冷启动引导对话（5 步：欢迎 → 自我介绍 → 痛点 → 第一个角色提议 → 创建确认）
- 角色 Create 路径
- 移除 Pitch Mode bar

---

### Epic 2: Agent 引擎集成、角色对话与记忆（Agent Engine, Role Engagement, Memory & Trust）

用户能在多个角色间切换对话，每个角色通过 opencode Agent Loop 执行复杂多步任务，用符合身份的语调回应，系统从对话中提炼记忆，用户可查询、追溯、删除记忆，并随时追问"为什么"获得透明推理。底层由 opencode sidecar 提供完整 Agent 能力（工具调用、Skill、权限控制）。完成后是一个完整的多角色个性化 AI Agent 系统。

**FRs covered:** FR-1（完整路由）, FR-2, FR-4（U/D/Archive）, FR-4b, FR-5（持续涌现）, FR-6, FR-7, FR-8, FR-9, FR-12（UI 层）, FR-20, FR-29, FR-30, **FR-31, FR-32, FR-33, FR-34, FR-35, FR-36**

**UX-DRs covered:** UX-DR9, UX-DR13, UX-DR14（含 Memory tab）

**核心交付：**
- **opencode sidecar 进程管理**（`sidecar.rs`：spawn/kill/health check/restart）
- **AgentBridge HTTP 客户端**（`agent_bridge.rs`：session/message/agent API 封装 + SSE 流解析）
- **角色→opencode Agent 动态映射**（`agent_config.rs`：角色 CRUD 同步 opencode.json agent 配置）
- **Agent Loop 对话核心回路**（替代原 tool-use：用户消息→opencode session→Agent 自主多步执行→SSE→Tauri Event→前端）
- **可配置权限模型**（UI 配置 allow/ask/deny → opencode permission 规则）
- 角色 CRUD 完整版（编辑、归档可恢复、永久删除二次确认、Skill 配置 UI）
- 完整路由引擎（管家分发 ↔ 角色直接对话双通道）
- 角色个性化 System Prompt 模板（注入 opencode agent prompt）
- 记忆提炼 pipeline（对话后台 LLM 提炼 → memories 表，标签化偏好/任务/状态/认知更新）
- 记忆查询/溯源 UI（MemoryTab 接通真实数据，可追溯到原始对话日志）
- 选择性遗忘（V1 简化版：删除可见条目 + 同源屏蔽，不重跑推理）
- 推理溯源（"为什么"追问返回引用记忆条目链）
- 不确定性表达（信心不足时主动声明）
- 对话区角色切换 + 历史同步
- 主动性档位 UI（行为接通延后到 E4）

---

### Epic 3: 任务管理与智能四象限（Task Management & Eisenhower Matrix）

用户可在每个角色下管理任务，系统自动四象限分类，标记大石头，Q2 任务受保护属性标记。完成后是一个 AI 任务管理器。

**FRs covered:** FR-23, FR-24（属性层）

**UX-DRs covered:** UX-DR14（TasksTab 数据接通）

**核心交付：**
- 任务 CRUD（手动创建、拖拽排序、勾选完成 - 原型 UI 接通真实数据）
- 截止日期 + 角色目标关联输入
- 自动四象限分类引擎（基于截止日期 + 角色目标 + 历史模式 + LLM 辅助）
- 置信度 < 80% 标记"不确定"
- 用户手动覆盖
- 大石头标识（isBigRock）
- Q2 保护属性存储与判定（"连续多日被挤"检测属性）

---

### Epic 4: 主动循环、通知与仪表盘（Proactive Loop, Notifications & Dashboard）

角色在后台按用户配置频率自主工作，生成主动建议，用户在管家视角的仪表盘能一目了然看到所有角色状态、待处理事项和能量值。完成后是一个主动协作伴侣。

**FRs covered:** FR-10, FR-11, FR-12（行为接通）, FR-19, FR-22

**UX-DRs covered:** UX-DR3, UX-DR12, UX-DR16, UX-DR18, UX-DR19, UX-DR20

**核心交付：**
- 后台调度器（tokio::interval，应用运行时执行，1-4 次/日 可调）
- 主动建议生成 service（角色 Agent 自审目标 → 建议）
- 建议确认/拒绝学习反馈（拒绝标记 reason 供 LLM 学习）
- 主动性三档刻度盘行为接通（passive/moderate/proactive 影响调度行为）
- 三级通知系统（耳语/轻触/敲门，敲门每日 ≤3 次降级）
- 角色卡片仪表盘（DashboardTab 接通真实数据：能量条/待办数/最近活跃）
- 视觉优先级排序（紧急 amber > 低能量 < 40% > 正常）
- 能量值色谱（高翠绿 / 中琥珀 / 低暗淡灰）
- Q2 保护"连续多日被挤"主动提醒（建立在 E3 属性之上）

---

### Epic 5: 使命宣言与冲突仲裁（Mission & Arbitration）

用户设定个人使命，多角色任务冲突时系统主动检测并提供基于使命+四象限+能量平衡的三步仲裁建议。完成后是一个价值观协调器。

**FRs covered:** FR-13, FR-14, FR-15, FR-24（仲裁应用层）

**UX-DRs covered:** UX-DR10

**核心交付：**
- 使命宣言设定（自由文本 OR 柯维三段式结构化模板二选一）
- 行为推断（未设定时基于历史推断隐含价值观）
- 冲突检测引擎（监控所有角色任务的时间冲突）
- 三步仲裁协议：使命对齐 → 四象限定位 → 能量平衡 → 双赢方案
- ArbitrationModal 三步可视化迁移与数据接通
- "决定权在你手中"明确表达（最终选择由用户做）
- Q2 保护参与仲裁决策（来自 E3 的 Q2 属性）

---

### Epic 6: 节奏化简报与复盘（Daily Briefing & Weekly Review Rhythm）

用户每日打开应用收到晨间简报，每周末收到正向叙事的复盘成绩单，每周开始有大石头规划引导。完成后是一个节奏化生活伴侣。

**FRs covered:** FR-16, FR-17, FR-18

**UX-DRs covered:** UX-DR11

**核心交付：**
- 晨间简报生成器（基于昨日记忆 + 今日任务 + 角色状态生成自然语言段落）
- 推送时间可配置
- "今天最重要的一件事"识别
- 大石头周规划引导对话（每周触发，为每个角色设 1-2 大石头）
- 周复盘成绩单生成（能量趋势 + 大石头完成 + 新记忆/Skill）
- WeeklyReviewModal 迁移与数据接通
- 正向叙事框架（"完成的 ✓ / 继续推进的 →"，禁用"未完成"）

---

### Epic 7: 数据主权（Data Sovereignty）

用户随时可一键导出全部数据为 JSON/Markdown，或一键销毁全部数据回到初始状态。完成后用户对自己的数据拥有完全控制。

**FRs covered:** FR-26, FR-27

**核心交付：**
- 全量数据导出（JSON 完整 schema + Markdown 可读版，30 秒内完成）
- 二次确认销毁流程
- 销毁后应用回到初始引导状态
- GlobalSettingsModal 数据 tab 接通

**实现备注：** 可在 Epic 1 完成后任何时机插入；建议放在 Epic 6 之后作为 V1 信任基石。

---

### Epic 8: 跨平台分发与 V1 加固（Cross-Platform Distribution & V1 Hardening）

用户在 Windows/macOS/Linux 三个平台都能下载安装稳定的 V1 安装包，应用已通过端到端核心旅程验证。完成后是一个三平台 V1 产品。

**FRs covered:** （无新 FR；交付 NFR 验证）

**NFRs covered:** NFR-2, NFR-5, NFR-9, NFR-11

**UX-DRs covered:** UX-DR17, UX-DR22

**核心交付：**
- GitHub Actions 三平台并行 CI（Windows/macOS/Linux）
- Tauri bundler MSI/DMG/AppImage 自动产物（含 opencode sidecar binary）
- E2E 测试套件（Tauri driver / WebDriver）覆盖：冷启动引导、管家对话、角色 CRUD、LLM 流式响应、任务管理、仲裁、简报复盘
- WCAG 2.1 AA 无障碍审计 & 修复（对比度、键盘导航、aria-label、屏幕阅读器）
- 性能基准验证（60fps 动效、首次体验 ≤ 5 分钟、流式输出延迟）
- Release 自动发布

---

## Epic 依赖图

```
E1 (基础) ─┬─→ E2 (Agent引擎+角色+记忆) ─┬─→ E5 (使命+仲裁)
           │                               ├─→ E6 (简报+复盘)
           ├─→ E3 (任务+四象限) ────────────┴─→ E4 (主动+仪表盘)
           │
           └─→ E7 (数据主权) [E1 之后任意时机]

E8 (V1 加固) ← 所有 Epic 完成后
```

每个 Epic 都自包含、可独立验收。E2 包含 Agent Engine 基础设施（opencode sidecar）作为首要 story，后续角色/记忆 story 依赖于此。E2 与 E3 可并行，E4 需要 E3，E5/E6 需要 E2+E3+E4。

---

## Epic 1: 基础平台与首次对话（Foundation & First Conversation）

用户能下载安装应用，配置 LLM，完成 5 分钟冷启动引导对话，创建第一个角色，看到流式响应。

### Story 1.1: 用户能打开 Tauri 桌面应用看到现有前端 UI

As a 用户,
I want 下载并打开 EgoSync 桌面应用,
So that 我能在独立窗口中使用它而不是浏览器。

**Acceptance Criteria:**

**Given** 开发者在三平台之一上 clone 仓库
**When** 在 `egosync-app/` 执行 `npm install && npm run tauri dev`
**Then** 桌面窗口打开并显示原型 UI（与 `npm run dev` 浏览器版本视觉一致）

**Given** 三平台环境
**When** 执行 `npm run tauri build`
**Then** 分别产出 `.msi` / `.dmg` / `.AppImage` 文件，能双击启动看到 UI

**Given** 冷启动
**When** 用户双击应用图标
**Then** 窗口出现时间 < 3 秒（M1 Mac / 中端 Windows）

**Given** `Cargo.toml`
**Then** 包含 tauri 2.x、serde、tokio、tracing、async-trait 依赖且 `cargo check` 通过

---

### Story 1.2: 建立 Rust + 前端最小测试基础设施

As a 开发者,
I want 项目从第一天就有可运行的测试链路（Rust 单元/集成 + 前端 Vitest + CI workflow 骨架）,
So that 后续每个 Story 都能在提交时验证不破坏已有功能。

**Acceptance Criteria:**

**Given** 开发者在 `egosync-app/` 目录下
**When** 执行 `npm run test:frontend`
**Then** Vitest 运行并报告 ≥ 1 个测试通过，退出码 0

**Given** 开发者在 `egosync-app/src-tauri/` 目录下
**When** 执行 `cargo test`
**Then** Rust 测试运行并报告 ≥ 1 个测试通过，退出码 0

**Given** 开发者在 `egosync-app/` 目录下
**When** 执行 `npm run test:all`
**Then** 按顺序运行前端测试 + Rust 测试，全部通过

**Given** 推送到 GitHub 任意分支
**When** CI workflow 触发
**Then** 三平台 matrix（ubuntu/macos/windows）均执行 `cargo test` + `npm run test:frontend` + `npm run tauri build`
**And** 任一步骤失败则整个 workflow 失败

**Given** 后续 Story 添加新 Rust 测试文件（如 `src-tauri/tests/test_llm.rs`）
**When** 执行 `cargo test`
**Then** 自动发现并运行，无需修改配置

---

### Story 1.3: 用户看到组件按域拆分后的稳定代码结构（视觉零回归 + 移除 Pitch Mode）

As a 用户,
I want 应用界面保持完全一致,
So that 代码重构不影响我的体验。

**Acceptance Criteria:**

**Given** 拆分前的截图（管家视角 + 角色视图 + 各 Modal）
**When** 拆分后启动应用
**Then** 像素级视觉零回归（允许 ≤ 2px 容差）

**Given** 拆分后代码
**When** 运行 `tsc --noEmit`
**Then** 零 TypeScript 错误

**Given** 生产构建
**Then** 顶部不再有黑色 Pitch Mode bar

**Given** 拆分后的 `App.tsx`
**Then** ≤ 100 行（仅做路由和 Provider 组合）

**Given** 拆分后目录结构
**Then** 组件按域分布在 `components/{layout,butler,role,chat,modals,onboarding,settings}/`
**And** 工具函数在 `lib/`、类型在 `types/`、hooks 在 `hooks/`

**Given** `DEFAULT_ROLES`、`ROLE_TASKS`、mock notifications 等硬编码常量
**Then** 集中到 `__mocks__/` 或 `constants/` 目录，标记为待替换

---

### Story 1.4: 用户能切换浅色/深色主题，所有动效遵循设计 token

As a 用户,
I want 切换浅色/深色主题且切换流畅,
So that 在不同光线环境下都能舒适使用。

**Acceptance Criteria:**

**Given** 当前是浅色主题
**When** 用户点击侧边栏底部主题切换按钮
**Then** 切换到深色 < 100ms 立即生效
**And** 主题偏好在重启后保留（写入 localStorage 或 app_settings）

**Given** 用户系统设置了 `prefers-reduced-motion: reduce`
**When** 切换主题或视图
**Then** 所有动效降为 ≤ 10ms

**Given** 任意主题
**Then** 所有文本/背景对比度 ≥ 4.5:1（axe DevTools 验证）

**Given** `tailwind.config.js`
**Then** 包含 CSS 变量 token：色彩（slate/indigo/amber 等）、间距（4px 基准）、圆角（6px 按钮 / 10px 卡片）、动效时长（200ms/250ms/300ms/3s）、角色色温表

**Given** 字体加载
**Then** Inter + Noto Sans SC + JetBrains Mono 均通过 `font-display: swap` 加载，无 FOIT

---

### Story 1.5: 用户的 LLM API Key 安全存储到系统钥匙串

As a 用户,
I want 我的 API Key 存储在操作系统安全区域,
So that 不会泄露到任何文件或日志中。

**Acceptance Criteria:**

**Given** 用户通过 UI 保存 API Key
**When** 检查 `egosync.db` 和所有日志文件
**Then** **不出现** Key 明文（grep `sk-` / `sk-ant-` 无匹配）

**Given** Windows 平台
**Then** Key 写入 Windows Credential Manager
**Given** macOS 平台
**Then** Key 写入 Keychain
**Given** Linux 平台
**Then** Key 写入 Secret Service

**Given** 卸载并重装应用
**Then** keyring 中的 Key 仍在（OS 级持久）

**Given** keyring 不可用（如无桌面环境的 Linux CI）
**When** 应用启动
**Then** 显示明确错误对话框说明原因，**不**降级到明文存储

**Given** Rust 后端代码
**Then** 存在 `services/secret_store.rs` 抽象层（save/load/delete），所有 Key 操作通过该抽象
**And** `AppError` 枚举包含 `KeyringError` 变体

---

### Story 1.6: 用户能配置 LLM Provider 并测试连接成功

As a 用户,
I want 在设置中添加 LLM Provider 并测试连接,
So that 确认配置正确后才开始使用。

**Acceptance Criteria:**

**Given** 用户输入 OpenAI 兼容配置（base_url + api_key + model）
**When** 点击"测试连接"
**Then** 显示"连接成功"或具体错误（401 未授权 / 网络超时 / 模型不存在等）

**Given** Ollama 本地运行（base_url=`http://localhost:11434/v1`）
**When** 测试连接
**Then** 同样的流程能成功

**Given** Anthropic 格式配置（base_url + api_key + model=claude-*）
**When** 测试连接
**Then** 使用 `AnthropicProvider` 路径成功或返回具体错误

**Given** 多个配置已保存
**When** 用户标记其中一个为默认
**Then** `llm_configs` 表 `is_default` 列唯一为 true

**Given** 测试连接请求
**Then** 超时阈值 ≤ 10 秒，超时后显示明确错误

**Given** 用户重启应用
**Then** 所有保存的配置仍然存在（持久化到 `egosync.db` 的 `llm_configs` 表）
**And** API Key 仅存储在 keyring 中，`llm_configs` 表只存引用标识

**Given** GlobalSettingsModal LLM tab
**Then** 显示真实配置列表（替换原型 mock 数组），支持新增/编辑/删除/设为默认

**Given** Rust 后端
**Then** 存在 `LlmProvider` trait（`async fn chat_completion`、`async fn test_connection`）+ `OpenAiProvider` + `AnthropicProvider` 实现
**And** `migrations/001_initial_schema.sql` 含 `llm_configs` 表 + `app_settings(key TEXT PRIMARY KEY, value TEXT, updated_at TEXT)` key-value 模式（后续 Story 通过 INSERT/UPDATE 写入配置项，无需 ALTER TABLE）

---

### Story 1.7: 用户能与管家完成首次流式对话

As a 用户,
I want 输入消息后看到管家流式逐字回复,
So that 感受到 AI 正在实时思考并回应我。

**Acceptance Criteria:**

**Given** LLM 已配置且连接正常
**When** 用户在管家对话区输入"你好"按 Enter
**Then** 流式逐字显示管家回复，首字节延迟 < 500ms

**Given** 流式响应进行中
**When** 用户关闭应用并重开
**Then** 对话历史已持久化保留（最后一条 message 的 `is_complete` 标记正确）

**Given** LLM API 报错（如网络断开、余额不足）
**Then** 在对话气泡中以管家自然语言展示（如"我现在连不上模型，能检查一下配置吗？"）
**And** **不**使用 toast / 红框 / snackbar

**Given** 流式进行中
**When** 用户再次发送消息
**Then** 新请求等待前一个完成或显示管家提示"我还在想上一个问题..."

**Given** 流式进行中
**When** 用户点击停止按钮
**Then** 后端通过 CancellationToken 中断流式，已生成内容保留并持久化

**Given** 用户发送首条消息
**Then** 后端异步调用 LLM 生成对话标题（≤8字），通过 Event 通知前端更新

**Given** 用户在对话界面
**Then** 可新建对话、查看历史对话列表、切换对话、删除对话

**Given** Rust 后端
**Then** `migrations/002_conversations.sql` 创建 `conversations` + `messages` 表（对话日志库 `conversations.db`）
**And** `agent_engine` 模块实现 System Prompt 分层（基础人格 + 当前上下文）
**And** Tauri Event `llm:stream` payload: `{ conversationId, token, done, thinking }`
**And** Tauri Event `llm:title-updated` payload: `{ conversationId, title }`

**Given** 前端
**Then** 新建 `components/chat/ChatStream.tsx` 替换 ButlerView 内 mock 消息列表
**And** 新建 `components/chat/ChatHeader.tsx` + `ConversationList.tsx` 管理会话历史
**And** `ChatInput.tsx` 支持流式中显示停止按钮
**And** `useTauriEvent('llm:stream')` hook 处理流式更新

---

### Story 1.8: 新用户首次打开应用，被管家引导完成 5 步对话创建第一个角色

As a 新用户,
I want 首次打开应用时被管家友好引导,
So that 不到 5 分钟就能创建第一个角色并理解 EgoSync 的核心概念。

**Acceptance Criteria:**

**Given** 新用户首次打开应用
**When** LLM 已配置
**Then** 自动进入 `OnboardingView`，5 步引导对话流：欢迎 → 自我介绍 → 痛点 → 角色提议 → 创建确认

**Given** 引导开始时 LLM 未配置
**Then** 引导第 0 步直接跳转到 GlobalSettingsModal LLM tab
**And** 配置完成后自动返回继续引导

**Given** 引导第 4 步用户确认创建角色
**When** 角色创建成功
**Then** `roles` 表写入角色记录
**And** `app_settings.onboarding_completed = true`
**And** 跳转管家主视图

**Given** 已完成引导的用户
**When** 重启应用
**Then** 直接进主视图，不重复引导

**Given** 端到端从启动到创建第一个角色
**Then** 总耗时 ≤ 5 分钟（含 LLM 响应时间）

**Given** Rust 后端
**Then** `migrations/003_roles.sql` 创建 `roles` 表
**And** Tauri command `role::create` 写入角色数据
**And** `app::is_first_launch` 检测 `app_settings.onboarding_completed`

**Given** 前端
**Then** `OnboardingView` 接通真实 LLM 对话（替换原型 mock 步骤流程）
**And** 监听 `role:proposed` 事件，弹出 `RoleConfirmModal` 供用户确认/编辑角色（name/icon/color/goal）
**And** 用户确认后调用 `roleService.create()` 实际写库

**Given** Rust 后端（引导 step ≥ 3）
**Then** 使用 Function Calling（`create_role` tool + `tool_choice="required"`）替代文本正则提取角色信息
**And** `OnboardingConversations` HashMap 跟踪引导 conv_id → step 映射
**And** `ChatRequest.onboarding_step: u8`（`#[serde(default)]`）驱动步骤切换

---

### Story 1.9: 用户在管家视角看到呼吸动画的角色侧边栏图标

As a 用户,
I want 侧边栏的角色图标有呼吸动画并显示真实状态,
So that 能感受到角色是"活的"实体。

**Acceptance Criteria:**

**Given** 用户已创建 1+ 角色
**When** 打开应用
**Then** 侧边栏显示真实角色图标（来自 `role::list` command，不是 `DEFAULT_ROLES` 硬编码）

**Given** 任意角色图标
**Then** CSS keyframe 呼吸动画（opacity 0.65→1, 3s ease-in-out 循环），60fps 无掉帧（Chrome DevTools Performance 验证）
**And** 动画使用 GPU 加速（`will-change: opacity`）

**Given** `prefers-reduced-motion: reduce`
**Then** 呼吸动画停止，图标保持静态 opacity 1

**Given** 屏幕阅读器
**When** 焦点到角色图标
**Then** 朗读 `aria-label`（如"产品经理 - 能量值 85%"）

**Given** 键盘导航
**When** 焦点在侧边栏
**Then** 上下键切换角色图标，Enter 进入角色视图

**Given** 前端
**Then** `components/layout/RoleSidebarIcon.tsx` 从 Sidebar 抽取独立组件
**And** 使用 `useRoles()` hook 调用 `role::list` 获取角色列表

---

## Epic 2: Agent 引擎集成、角色对话与记忆（Agent Engine, Role Conversation, Memory & Trustworthiness）

用户能与多个角色分别对话，每个角色通过 opencode Agent Loop 执行复杂任务，有独特语调；系统自动从对话中提炼记忆，用户能查看、溯源和遗忘记忆；AI 做出建议时提供推理透明度，不确定时主动声明。底层由 opencode sidecar 提供完整 Agent 能力。

### Story 2.0: opencode Sidecar 进程管理与 AgentBridge HTTP 客户端

As a 开发者,
I want Tauri 应用启动时自动拉起 opencode server 并通过 HTTP API 通信,
So that 后续所有 Agent 对话和工具调用有执行引擎支撑。

**Acceptance Criteria:**

**Given** 开发者执行 `npm run tauri dev`
**When** 应用启动完成
**Then** opencode server 进程已在后台运行，监听 `127.0.0.1:4096`（或可配置端口）

**Given** opencode server 正在运行
**When** Rust 后端调用 `agent_bridge.get_providers()`
**Then** 返回当前已配置的 Provider 列表（JSON）

**Given** 应用退出（关闭窗口或 Cmd+Q）
**When** 检查进程列表
**Then** opencode server 进程已终止，无孤儿进程

**Given** opencode server 意外崩溃
**When** 健康检查失败
**Then** Rust 后端在 3 秒内自动重启 opencode 进程

**Given** `src-tauri/resources/` 目录
**Then** 包含当前平台的 opencode binary（Windows: `opencode.exe`, macOS/Linux: `opencode`）

**Given** Rust 后端代码
**Then** 存在 `services/sidecar.rs`（进程管理：spawn/kill/health_check/restart）
**And** 存在 `services/agent_bridge.rs`（HTTP 客户端：create_session/send_message/abort/get_messages/get_config/get_providers）

---

### Story 2.0b: 角色→opencode Agent 动态映射与权限配置

As a 开发者,
I want EgoSync 角色 CRUD 时自动同步为 opencode agent 配置,
So that 每个角色都有独立的 Agent 身份（prompt/model/permission）在 opencode 中运行。

**Acceptance Criteria:**

**Given** 用户创建一个新角色（name="产品经理", goal="..."）
**When** 角色写入 EgoSync DB
**Then** opencode.json 的 `agent` 段新增对应条目（mode: "subagent", prompt 含角色 goal）

**Given** 用户编辑角色 prompt/goal
**When** 保存成功
**Then** opencode.json 对应 agent 的 prompt 字段同步更新

**Given** 用户归档角色
**When** 归档成功
**Then** opencode.json 对应 agent 设为 `disable: true`

**Given** 用户永久删除角色
**When** 删除成功
**Then** opencode.json 中对应 agent 条目被移除

**Given** 用户在角色设置中将 "bash" 权限从 "allow" 改为 "ask"
**When** 保存成功
**Then** opencode.json 对应 agent 的 permission 段更新为 `{ "bash": "ask" }`

**Given** 管家（Butler）
**Then** 始终作为 primary agent 存在于 opencode.json，permission 为 `{ "*": "allow" }`

**Given** Rust 后端代码
**Then** 存在 `services/agent_config.rs`（read/write opencode.json agent 段、同步逻辑）

---

### Story 2.0c: 对话引擎切换 — agent_engine 路由到 opencode AgentBridge

As a 用户,
I want 管家和角色的对话通过 opencode Agent Loop 执行,
So that 我能获得完整的多步工具调用能力（代码编写、文件操作、Web搜索等），而不仅仅是单轮文本回复。

**Acceptance Criteria:**

**Given** 用户在管家对话区输入消息
**When** 按 Enter 发送
**Then** 消息通过 AgentBridge → opencode session/message API 发送，而非直调 LlmProvider
**And** opencode Agent Loop 自主决策工具调用，流式返回

**Given** opencode Agent Loop 执行中
**When** 产生流式 token / thinking / tool_call
**Then** AgentBridge 解析 SSE，转发为 Tauri Event（`llm:stream`），前端实时渲染
**And** 前端 ChatStream 组件无需修改（Event payload 格式兼容）

**Given** 用户在角色对话区输入消息
**When** 该角色已映射为 opencode subagent（由 2-0b 完成）
**Then** 消息路由到该角色对应的 opencode agent session

**Given** 流式进行中
**When** 用户点击停止按钮
**Then** 调用 `agent_bridge.abort_session()` 中断 opencode 执行

**Given** 用户通过管家发送意图消息（如"帮我写个竞品分析"）
**When** 管家 agent 路由到目标角色
**Then** 路由逻辑仍然有效（意图解析通过 opencode Agent 完成，无需独立 LLM 调用）

**Given** 角色的 system prompt 已在 opencode agent config 中设定（由 2-0b 完成）
**Then** 每个角色对话自动使用其个性化语调，无需 agent_engine 手动注入 prompt

**Given** 引导中管家建议创建角色（Function Calling `create_role`）
**When** opencode Agent Loop 触发 tool use
**Then** EgoSync 通过自定义 tool 或 Event 机制捕获 `role:proposed`，弹出确认 modal

**Given** 角色 CRUD 操作（创建/编辑/归档/删除）
**When** 操作成功写入 EgoSync DB
**Then** 调用现有 AgentConfigService 生命周期同步方法更新 opencode.json（create/update/archive/restore/delete 对应同步）

**Given** opencode server 不可用（进程未启动或崩溃）
**When** 用户发送消息
**Then** 降级到 LlmProvider 直调路径，并在对话中以管家语调提示"Agent 引擎暂时不可用，当前为基础对话模式"

**Given** 切换完成后
**Then** 所有已有功能（对话流式、角色切换、意图路由、个性化语调、角色涌现建议）保持可用
**And** LlmProvider trait 保留为降级兼容层，不删除

---

### Story 2.1: 用户能编辑角色名称/图标/颜色，归档和恢复角色，永久删除角色

As a 用户,
I want 修改角色信息、归档不常用角色、永久删除不需要的角色,
So that 我的角色列表保持整洁且可控。

**Acceptance Criteria:**

**Given** 用户在 RoleView SettingsTab 修改角色名称/图标/颜色
**When** 点击保存
**Then** 侧边栏实时更新显示新名称/图标/颜色
**And** `roles` 表对应字段已更新

**Given** 用户在 RoleView SettingsTab 点击"归档角色"
**When** 确认归档
**Then** 角色从侧边栏消失
**And** 角色的对话历史和记忆数据保留不删除
**And** 若当前在该角色视图则自动切回管家视角

**Given** 用户在 GlobalSettingsModal 归档列表中点击"恢复"
**When** 恢复完成
**Then** 角色重新出现在侧边栏
**And** 所有历史数据完整恢复

**Given** 用户点击"永久删除"
**When** 二次确认对话框（输入角色名确认）
**Then** `roles` 表删除记录
**And** 关联的 `memories`、`conversations`、`messages` 级联删除
**And** 删除不可撤销

**Given** 用户仅剩 1 个角色
**When** 尝试归档或删除
**Then** 按钮禁用并显示提示"至少保留一个角色"

**Given** Rust 后端
**Then** Tauri commands: `role::update` / `role::archive` / `role::restore` / `role::delete`
**And** `roles` 表增加 `archived_at` 可空字段

---

### Story 2.2: 用户能点击角色图标进入角色视图并切换回管家

As a 用户,
I want 点击侧边栏角色图标切换到该角色的对话视图,
So that 我能与特定角色直接对话。

**Acceptance Criteria:**

**Given** 用户在管家视角
**When** 点击侧边栏某角色图标
**Then** 页面色温 300ms CSS 变量过渡到该角色色系
**And** 内容区 250ms 淡入显示角色视图
**And** 对话区显示该角色的独立对话历史（不是管家的）

**Given** 用户在角色视图
**When** 点击侧边栏管家图标（Home）
**Then** 色温 300ms 还原到管家色系
**And** 内容区显示管家对话历史

**Given** 导航层级
**Then** 永远不超过 2 层（管家 ↔ 角色），无嵌套路由

**Given** 角色视图头部
**Then** 显示角色名称、图标、能量值、状态指示器
**And** 抽取为独立 `components/role/RoleHeader.tsx` 组件

**Given** 前端路由
**Then** `currentView` 状态驱动视图切换
**And** 每个角色的 `conversation_id` 独立管理

**Given** Rust 后端
**Then** `conversation::get_or_create_by_role` command 按 role_id 返回活跃对话

---

### Story 2.3: 用户能在管家对话中输入意图，管家自动路由到正确角色

As a 用户,
I want 直接跟管家说需求，管家帮我找到最合适的角色处理,
So that 不需要手动切换角色就能得到专业回应。

**Acceptance Criteria:**

**Given** 用户在管家对话中输入"帮我跟进本周 OKR"
**When** 系统存在"产品经理"角色
**Then** 管家回复中自然引导"这个交给产品经理角色来跟进"
**And** 用户确认后自动切换到产品经理角色视图继续对话

**Given** 用户输入意图模糊（如"帮我想想"）
**When** 无法明确匹配单一角色
**Then** 管家主动追问"你想让哪个角色帮忙？这几个角色可能相关：..."

**Given** 用户通过点击角色图标直接进入角色视图
**When** 在角色视图中发送消息
**Then** 不走意图路由，直接以该角色身份对话

**Given** 路由决策
**Then** 每次路由在 `messages` 表记录 `routing_metadata` JSON 字段（source_role, target_role, confidence, reason）
**And** 路由日志可审计

**Given** Rust 后端 `agent_engine`
**Then** 意图分类模块：LLM 分析用户输入 → 返回 `{ target_role_id, confidence, reason }`
**And** confidence < 0.6 时触发追问而不是强制路由

---

### Story 2.4: 每个角色用符合身份的个性化语调回应

As a 用户,
I want 每个角色的回复风格与其身份匹配,
So that 不同角色之间有明显区分感，对话更自然。

**Acceptance Criteria:**

**Given** 用户向"产品经理"角色提问
**When** 角色回复
**Then** 语调简洁专业，偏结构化表达

**Given** 用户向"家庭"角色提问相同问题
**When** 角色回复
**Then** 语调温暖关怀，偏情感化表达
**And** 与产品经理回复风格明显不同

**Given** 用户在 RoleView SettingsTab 编辑角色个性描述字段
**When** 保存后发起对话
**Then** 下次回复立即反映新的个性语调

**Given** System Prompt 组装
**Then** 三层分离：`base_persona`（管家基础人格）+ `role_definition`（角色身份+个性描述）+ `context_injection`（当前上下文+记忆）
**And** 每层可独立调试和修改

**Given** Rust 后端
**Then** `roles` 表增加 `personality_prompt` TEXT 字段（migration）
**And** `agent_engine` 组装 System Prompt 时按三层顺序拼接
**And** 预设语调模板：产品经理=简洁专业、家庭=温暖关怀、学习者=好奇探索

**Given** 前端 RoleView SettingsTab
**Then** 新增"角色个性描述"多行文本编辑区域
**And** 显示预设模板作为参考但允许自由编辑

---

### Story 2.5: 管家从对话中持续识别角色需求并建议创建新角色

As a 用户,
I want 管家在对话中发现我有新领域的需求时主动建议创建角色,
So that 角色体系随着我的使用自然生长。

**Acceptance Criteria:**

**Given** 用户连续 3+ 次与管家聊健身相关话题
**When** 系统中不存在健身类角色
**Then** 管家以自然对话形式建议"我注意到你最近经常聊健身，要不要创建一个健身教练角色？"

**Given** 管家建议创建角色
**When** 用户接受
**Then** 进入 2-3 轮引导对话（角色名 → 职责 → 个性描述）
**And** 创建完成后角色出现在侧边栏

**Given** 管家建议创建角色
**When** 用户拒绝
**Then** 管家回复"好的，以后有需要再说"
**And** 同一领域短期内不再重复建议（冷却期 ≥ 7 天）

**Given** 手动创建入口
**Then** 侧边栏 `+` 按钮（AddRoleModal）保持不变，与涌现建议并存

**Given** Rust 后端
**Then** `agent_engine` 涌现检测模块：分析最近 N 条管家对话 → 识别频繁出现的未覆盖领域
**And** 涌现建议不阻塞正常对话流

---

### Story 2.6: 系统从每次对话中自动提炼结构化记忆

As a 用户,
I want 系统自动从对话中提取关键信息保存为记忆,
So that 角色能记住我说过的话，下次对话时更懂我。

**Acceptance Criteria:**

**Given** 用户与产品经理角色完成一段对话（≥ 3 条 user messages）
**When** 对话结束（用户 5 分钟无新消息或手动关闭对话窗口）
**Then** 后台 LLM 静默提炼 → `memories` 表写入 ≥ 1 条记忆
**And** 提炼过程不阻塞用户下次对话

**Given** 提炼的记忆
**Then** 每条包含：`role_id`、`category`（preference / task_status / cognition_update / fact）、`content`（结构化摘要）、`source_conversation_id`、`source_message_ids`（JSON 数组）、`created_at`

**Given** 对话内容无有价值信息（如纯寒暄）
**When** 提炼完成
**Then** 不写入任何记忆（允许 0 条输出）

**Given** 数据库
**Then** `migrations/004_memories.sql` 创建 `memories` 表
**And** 外键关联 `roles.id` 和 `conversations.id`

**Given** Rust 后端
**Then** `memory_pipeline` 服务：对话完成 → 构造提炼 prompt → LLM 返回结构化 JSON → 写入 memories 表
**And** 提炼失败（LLM 超时/格式错误）记录到 tracing 日志但不向用户报错

---

### Story 2.7: 用户能在记忆面板查看、追溯记忆来源

As a 用户,
I want 查看角色积累的所有记忆并追溯每条记忆的来源对话,
So that 我能理解 AI 为什么记住了这些信息。

**Acceptance Criteria:**

**Given** 用户在 RoleView 点击 MemoryTab
**When** 该角色有 5 条记忆
**Then** 按时间倒序显示 5 条记忆卡片
**And** 每张卡片显示：类别标签、内容摘要、创建时间

**Given** 用户点击某条记忆卡片
**When** 展开详情
**Then** 显示原始对话片段（从 `conversations.db` 读取 `source_message_ids` 对应的 messages）
**And** 提炼来源的 messages 高亮显示

**Given** 用户在类别筛选器选择"偏好"
**When** 筛选生效
**Then** 仅显示 `category = 'preference'` 的记忆

**Given** MemoryTab 标签
**Then** 显示该角色记忆总数 badge（如 `记忆 (12)`）

**Given** 角色无记忆
**Then** 显示空态提示"还没有记忆，多和这个角色聊聊吧"

**Given** Rust 后端
**Then** Tauri commands: `memory::list_by_role { role_id, category?, limit, offset }` / `memory::get_source_messages { memory_id }`

**Given** 前端
**Then** `components/role/MemoryTab.tsx` 接通真实数据（替换原型 mock 列表）
**And** `useMemories(roleId)` hook 封装查询逻辑

---

### Story 2.8: 用户能删除特定记忆（选择性遗忘）

As a 用户,
I want 删除不准确或不想被记住的记忆,
So that 我对 AI 记住的内容有控制权。

**Acceptance Criteria:**

**Given** 用户在 MemoryTab 某条记忆上点击"遗忘"按钮
**When** 触发确认
**Then** 显示管家风格确认文案（如"确定要忘记这条吗？忘了就真忘了哦"）
**And** 不是系统级 `window.confirm` 对话框

**Given** 用户确认遗忘
**When** 删除完成
**Then** `memories` 表该条可见记录被删除
**And** 记录同源屏蔽，旧会话再次触发记忆提炼时，同一 `source_conversation_id + category + source_message_ids` 不会重新写回
**And** MemoryTab 列表实时移除该卡片
**And** badge 数字 -1

**Given** 删除的记忆曾被对话引用
**When** 用户查看历史对话
**Then** 历史对话内容不受影响（历史不篡改）

**Given** 用户取消遗忘
**Then** 无任何操作

**Given** Rust 后端
**Then** Tauri command: `memory::delete { memory_id }`
**And** 删除 `memories` 可见记录并写入同源屏蔽
**And** 不级联删除 conversations/messages

---

### Story 2.9: 用户追问"为什么"获得透明推理链，AI 不确定时主动声明

As a 用户,
I want AI 做出建议时能解释推理依据，不确定时主动告诉我,
So that 我能判断建议是否可信。

**Acceptance Criteria:**

**Given** 管家/角色基于记忆做出建议后
**When** 用户追问"为什么"或"你怎么知道的"
**Then** 回复包含引用的记忆条目列表（格式：`[记忆#ID] 内容摘要`）
**And** 引用的记忆 ID 可点击跳转到 MemoryTab 对应条目

**Given** 复合推理
**When** 建议基于多条记忆
**Then** 溯源链格式：`[记忆#12] 你之前说过周五有家庭聚餐 → [记忆#8] 产品评审也在周五 → 建议调整`

**Given** AI 对建议的信心不足
**When** LLM 输出包含 hedging 语言（"可能"、"不太确定"、"也许"）
**Then** 主动声明不确定性（如"我不太确定这个判断，建议你自己评估一下"）
**And** 不隐藏不确定性

**Given** `agent_engine` 上下文注入
**Then** 注入记忆时附带记忆 ID 标记
**And** System Prompt 包含指令：引用记忆时标注来源、不确定时主动声明

**Given** 前端
**Then** 对话气泡中的 `[记忆#ID]` 渲染为可点击链接
**And** 点击后打开 MemoryTab 并滚动到对应记忆

---

### Story 2.10: 用户能为角色配置默认元 Skill 并看到主动性刻度盘 UI

As a 用户,
I want 为角色启用默认元 Skill 并调节主动性档位,
So that 控制角色发现/创建 Skill 的能力边界以及多主动地帮助我。

**Acceptance Criteria:**

**Given** 用户在 RoleView SettingsTab 或管家设置页的 Skill 配置区域
**When** 查看可用 Skill 列表
**Then** V1 显示两个默认元 Skill 开关：`find-skills`、`skill-creator`
**And** 每个开关显示 Skill 名称、来源和简短描述

**Given** 用户切换角色 `find-skills` 或 `skill-creator`
**When** 保存成功
**Then** `roles.skills_config` JSON 字段更新并在重启后保留
**And** 角色 opencode agent 配置或 prompt 能力约束同步反映启用/禁用状态

**Given** 用户切换管家 `find-skills` 或 `skill-creator`
**When** 保存成功
**Then** `app_settings` 中的 `butler.skills_config` 更新并在重启后保留
**And** 管家 opencode agent 配置或 prompt 能力约束同步反映启用/禁用状态

**Given** 角色或管家收到需要关闭元 Skill 的请求
**When** `find-skills=false` 的会话请求发现/搜索/推荐 Skill，或 `skill-creator=false` 的会话请求创建/扩展 Skill
**Then** 后端在发送给 opencode 前返回边界提示，不调用 opencode `skill` 工具
**And** 关闭的元 Skill 不出现在对应 system prompt 中
**And** 两个元 Skill 都关闭时，对应 opencode agent permission 写入 `skill: deny`

**Given** ProactivityToggle 三档切换（静默执行 / 适度建议 / 积极主动）
**When** 用户切换档位
**Then** 写入 `roles.proactivity_level`（枚举：passive / moderate / proactive）
**And** 重启后保留

**Given** 主动性档位的行为差异
**Then** V1 仅存储档位值和显示 UI
**And** 实际行为差异在 Epic 4 接通

**Given** 前端
**Then** RoleView SettingsTab 接通真实 Skill 配置（替换原型 mock/API key UI）
**And** ProactivityToggle 组件接通真实数据（替换原型 mock 滑块）

---

### Story 2.11: 用户能导入自定义 SKILL.md 并按角色启用

As a 用户,
I want 把本地自定义 SKILL.md 加入 EgoSync 的 Skill 库并绑定到角色,
So that 我的角色能复用我自己沉淀的能力模块，而不需要每次手动复制提示词。

**Acceptance Criteria:**

**Given** 用户在角色 SettingsTab 的 Skill 区域点击“导入自定义 Skill”
**When** 选择一个包含合法 frontmatter（name/description）的 `SKILL.md` 文件或目录
**Then** 系统解析并展示名称、描述、来源路径和校验结果
**And** 不把文件内容或路径密钥写入日志

**Given** 用户确认导入自定义 Skill
**When** 保存成功
**Then** Skill 被复制到 EgoSync 管理的 opencode skills 目录或登记为受控路径
**And** 全局 Skill registry 记录 Skill id、name、description、sourceType=`custom`、managedPath/contentHash
**And** 角色的 `skills_config` 仅记录启用的 Skill id，不覆盖已有 `find-skills` / `skill-creator` 配置

**Given** 用户在某角色启用或禁用自定义 Skill
**When** 保存成功
**Then** 对应角色的 opencode agent 配置或 prompt 能力约束同步更新
**And** 应用重启后 Skill 库、角色绑定和启用状态仍保持一致

**Given** 用户导入重复 Skill（同 name 或同 content hash）
**When** 确认导入
**Then** 系统提示已存在并允许取消或覆盖元数据
**And** 不创建不可区分的重复条目

**Given** 当前 Story 2.10 已实现两个元 Skill 开关
**Then** 新的 Skill 配置 schema 必须向后兼容旧 JSON
**And** 普通元 Skill toggle 不得丢弃自定义 Skill、permissions 或未来扩展字段

---

### Story 2.12: 用户能发现并导入 opencode 生态 Skill

As a 用户,
I want 从 opencode 可发现的 Skill 目录中扫描并导入第三方 Skill,
So that 我能复用 opencode 生态能力，同时仍由 EgoSync 管理每个角色启用什么。

**Acceptance Criteria:**

**Given** 用户已启用 `find-skills`
**When** 在 Skill 配置中点击“发现 Skill”
**Then** 系统扫描 opencode 项目级与全局 Skill 目录（如 `.opencode/skills/`、`~/.config/opencode/skills/`）
**And** 以列表展示可导入 Skill 的 name、description、来源位置和是否已导入

**Given** 扫描到无效 Skill（缺少 `SKILL.md`、frontmatter 缺失、文件不可读）
**When** 结果展示
**Then** 无效项不进入可导入列表
**And** 设置页以友好中文说明跳过原因，不暴露底层堆栈

**Given** 用户选择一个第三方 opencode Skill 导入
**When** 确认导入
**Then** Skill 被登记到全局 Skill registry，sourceType=`opencode`
**And** 用户可立即在当前角色启用该 Skill

**Given** 第三方 Skill 已导入并启用到角色
**When** 角色下一轮对话开始
**Then** 角色 opencode agent 能自动发现/加载该 Skill
**And** 禁用后角色不得再声明自己拥有该 Skill

**Given** 当前 V1 范围
**Then** 不实现远程市场、付费 Skill、账号登录或自动下载未知 URL
**And** 如用户需要导入远程获得的 Skill，必须先保存为本地 SKILL.md 再走 Story 2.11 路径

---

### Story 2.13: 用户能配置 MCP server 列表并按角色接入外部工具

As a 用户,
I want 配置外部 MCP server 并选择哪些角色可用,
So that 角色能安全接入日历、邮件、代码仓库等外部工具服务。

**Acceptance Criteria:**

**Given** 用户在设置中打开 MCP server 管理区域
**When** 新增一个 MCP server
**Then** 可填写 server 名称、类型（HTTP/SSE 或 command，本地能力优先）、连接参数、说明和启用状态
**And** secrets/API key 不写入 `roles.skills_config`、app_settings 明文字段或日志；只能使用 keyring 引用或环境变量引用

**Given** 用户保存 MCP server
**When** 配置校验通过
**Then** server 列表持久化到 EgoSync 本地配置
**And** `AgentConfigService` 同步 opencode.json 的外部 `mcp` 配置
**And** `full_sync()` 不得删除用户配置的外部 MCP server

**Given** Story 2.0d 已将 EgoSync 内部工具从 MCP 改为 opencode Custom Tools
**Then** 本 story 不恢复旧的 `egosync` MCP host
**And** 继续保留 `create_role`、`delegate_to_role`、`record_emergence_rejection` 的 custom tools 路径

**Given** 用户为某个角色启用一个 MCP server
**When** 保存成功
**Then** 角色配置记录该 MCP server 可用
**And** opencode agent 配置或 prompt 能力约束反映该角色可使用的外部工具
**And** 其他未启用该 server 的角色不得声明或调用该外部工具

**Given** 用户测试 MCP server 连接
**When** server 不可达或配置错误
**Then** 设置页显示友好错误并允许修改
**And** 不影响应用启动、角色 CRUD、普通对话和已有 custom tools

---

## Epic 3: 任务管理与智能四象限（Task Management & Eisenhower Matrix）

用户可在每个角色下管理任务，系统自动四象限分类，标记大石头，Q2 任务受保护属性标记。完成后是一个 AI 任务管理器。

### Story 3.1: 用户能在角色视图创建、编辑、删除任务

As a 用户,
I want 在角色的任务面板中创建、编辑和删除任务,
So that 我能为每个角色管理独立的待办事项。

**Acceptance Criteria:**

**Given** 用户在 RoleView TasksTab 点击"+"按钮
**When** TaskModal 弹出并填写任务内容 + 截止时间
**Then** 点击"保存任务"后 `tasks` 表写入新记录
**And** TasksTab 实时显示新任务（无需刷新）

**Given** 用户点击已有任务卡片
**When** 编辑任务内容/截止时间
**Then** 修改后保存到 `tasks` 表
**And** TasksTab 实时更新显示

**Given** 用户在任务卡片上点击删除
**When** 确认删除
**Then** `tasks` 表标记 `deleted_at`（软删除）
**And** TasksTab 列表实时移除该任务

**Given** 数据库
**Then** `migrations/005_tasks.sql` 创建 `tasks` 表：`id`, `role_id`, `title`, `deadline`, `quadrant`(Q1-Q4), `is_big_rock`, `is_completed`, `completed_at`, `sort_order`, `protection_status`, `confidence`, `created_at`, `updated_at`, `deleted_at`
**And** 外键关联 `roles.id`

**Given** Rust 后端
**Then** Tauri commands: `task::create` / `task::update` / `task::delete`
**And** 返回完整 task 对象供前端更新

**Given** 前端
**Then** TaskModal 接通真实数据（替换原型 mock 表单）
**And** `useTasks(roleId)` hook 封装 CRUD 操作

---

### Story 3.2: 用户能拖拽排序任务并勾选完成

As a 用户,
I want 拖拽调整任务顺序并一键标记完成,
So that 我能按自己的优先级排列任务并追踪进度。

**Acceptance Criteria:**

**Given** TasksTab 中有 3+ 个任务
**When** 用户拖拽任务卡片的 GripVertical 手柄
**Then** 任务实时重新排列
**And** 松手后 `sort_order` 批量更新到 `tasks` 表

**Given** 用户点击任务左侧圆圈
**When** 完成动画播放（圆圈变绿 ✓，卡片 200ms 淡出缩小）
**Then** `tasks.is_completed = true` + `completed_at` 写入时间戳
**And** 该任务灰显并沉到列表底部

**Given** 用户点击已完成任务的绿色 ✓
**When** 撤销完成
**Then** `is_completed = false` + `completed_at = null`
**And** 任务恢复正常显示并回到原 sort_order 位置

**Given** 拖拽排序操作
**Then** 使用 `@dnd-kit/core` 或类似库实现
**And** 拖拽时显示半透明占位符

**Given** Rust 后端
**Then** Tauri commands: `task::reorder { task_ids: Vec<String> }` / `task::toggle_complete { task_id, is_completed }`

---

### Story 3.3: 系统自动为任务分配四象限分类

As a 用户,
I want 系统自动为我的任务分析紧急/重要程度并分类,
So that 不用每个任务都手动判断该放哪个象限。

**Acceptance Criteria:**

**Given** 用户创建新任务（有截止时间和角色目标上下文）
**When** 任务保存后
**Then** 后台 LLM 自动分析并分配 Q1-Q4 分类
**And** 分析输入：截止日期距今天数 + 角色目标关联度 + 历史同类任务模式
**And** 分类结果写入 `tasks.quadrant`

**Given** LLM 分类置信度 < 80%
**When** 分类完成
**Then** `tasks.confidence` 字段记录置信度
**And** TasksTab 中该任务显示"不确定"标记（淡黄色边框 + 问号图标）

**Given** 任务截止日期距今 ≤ 2 天
**When** 定时检查触发（每小时）
**Then** 自动升入 Q1（若当前不是 Q1）
**And** `quadrant` 更新 + 记录变更原因

**Given** 用户手动覆盖四象限分类（在 TaskModal 中选择）
**When** 覆盖保存
**Then** `tasks.quadrant` 更新为用户选择
**And** 标记 `manual_override = true`，后续自动分类不再覆盖该任务

**Given** LLM 分析失败（超时/格式错误）
**Then** 默认分配 Q2（重要不紧急）
**And** tracing 日志记录失败原因

**Given** Rust 后端
**Then** `task_classifier` 服务：构造分类 prompt（任务标题 + 截止日期 + 角色目标 + 最近 5 条同角色任务）→ LLM 返回 `{ quadrant, confidence, reason }`
**And** 定时器 `tokio::interval(Duration::from_secs(3600))` 检查截止日期临近任务

---

### Story 3.4: 用户能标记任务为"大石头"并在 TasksTab 中视觉突出

As a 用户,
I want 标记本周最重要的几个任务为"大石头",
So that 系统和我都能识别不可妥协的核心任务。

**Acceptance Criteria:**

**Given** 用户在 TaskModal 勾选"标记为本周大石头"复选框
**When** 保存任务
**Then** `tasks.is_big_rock = true`

**Given** TasksTab 中有大石头任务
**Then** 显示琥珀色 `大石头` 标签（`bg-amber-50 text-amber-600 border-amber-100`）
**And** 大石头任务在同象限内排序靠前

**Given** 某角色已有 3 个大石头任务
**When** 用户尝试标记第 4 个
**Then** 提示"每个角色每周最多 3 个大石头，请先取消一个再标记"
**And** 标记操作不生效

**Given** 用户取消大石头标记
**When** 在任务编辑中取消勾选
**Then** `tasks.is_big_rock = false`
**And** 琥珀色标签消失

**Given** Rust 后端
**Then** `task::update` 中校验 big_rock 数量限制（per role per week ≤ 3）

---

### Story 3.5: 系统为 Q2 任务添加保护属性，连续被挤时标记预警

As a 用户,
I want 系统自动监测我的 Q2 任务是否被持续挤压,
So that 重要但不紧急的事不会被遗忘。

**Acceptance Criteria:**

**Given** 任务分类为 Q2
**Then** `tasks.protection_status` 初始为 `normal`

**Given** Q2 任务连续 3+ 天未被处理（无完成、无编辑、无对话提及）
**When** 定时检查触发
**Then** `protection_status` 更新为 `at_risk`

**Given** TasksTab 中 `at_risk` 状态的 Q2 任务
**Then** 显示 ⚠️ 预警图标（`AlertTriangle` 琥珀色）
**And** 卡片左边框添加琥珀色竖线

**Given** 用户处理了 `at_risk` 任务（完成、编辑或相关对话）
**When** 操作完成
**Then** `protection_status` 恢复为 `normal`
**And** 预警标识消失

**Given** 保护机制的行为层（管家主动提醒、仲裁优先保护）
**Then** V1 仅做数据标记和 UI 预警
**And** 实际行为接通在 Epic 4（主动循环）和 Epic 5（仲裁）

**Given** Rust 后端
**Then** 定时器检查 Q2 任务 `updated_at` 距今天数
**And** Tauri command: `task::check_protection_status`（可由前端启动时调用）

---

### Story 3.6: 任务按四象限分组显示在 TasksTab 中

As a 用户,
I want 任务按四象限分组展示,
So that 一眼看到哪些紧急哪些重要。

**Acceptance Criteria:**

**Given** 角色有分布在不同象限的任务
**When** 打开 TasksTab
**Then** 任务分 4 组显示，每组标题：
- Q1: 重要且紧急（红色强调 `text-red-600`）
- Q2: 重要不紧急（蓝色 `text-blue-600`）
- Q3: 紧急不重要（灰色 `text-slate-600`）
- Q4: 不重要不紧急（淡灰 `text-slate-400`）

**Given** 每组标题
**Then** 右侧显示任务数 badge（如 `Q1 · 重要且紧急 (3)`）

**Given** 某象限无任务
**Then** 该组折叠且显示鼓励文案（如 Q1 空:"没有紧急任务，太棒了！"）

**Given** 每组内任务
**Then** 按 `sort_order` 排序
**And** 大石头任务排在同组最前

**Given** 用户点击组标题
**When** 折叠/展开
**Then** 300ms 动画收起/展开该象限任务列表
**And** 折叠状态不持久化（每次打开默认全展开）

**Given** 前端
**Then** `components/role/TasksTab.tsx` 重构为四象限分组视图（替换原型单组 Q1 列表）
**And** 复用原型卡片样式（GripVertical + Circle + deadline badge + 大石头标签）

---

### Story 3.7: 管家视图通用任务 Tab 汇总所有角色的任务

As a 用户,
I want 在管家视角一站式查看所有角色的任务,
So that 不用逐个切换角色就能掌握全局。

**Acceptance Criteria:**

**Given** 用户在 ButlerView 点击"任务"Tab
**When** 多个角色各有任务
**Then** 按角色分组显示（每组显示角色名 + 角色色标圆点）
**And** 每组内按四象限排序（Q1 在前）

**Given** 任务卡片
**Then** 显示所属角色色标 + 角色名缩写
**And** 点击任务 → 切换到对应角色视图的 TasksTab 并高亮该任务

**Given** 顶部筛选器
**Then** 支持按四象限筛选（全部 / Q1 / Q2 / Q3 / Q4）
**And** 支持按"仅大石头"过滤

**Given** 所有角色均无任务
**Then** 显示空态"所有角色都很轻松，可以考虑添加新目标"

**Given** Rust 后端
**Then** Tauri command: `task::list_all { quadrant?, is_big_rock? }` 返回跨角色任务列表（含 role_id + role_name + role_color）

**Given** 前端
**Then** ButlerView 通用任务 Tab 接通真实数据（替换原型单条 mock 任务）
**And** `useAllTasks(filters)` hook 封装跨角色查询

---

## Epic 4: 主动循环、通知与仪表盘（Proactive Loop, Notifications & Dashboard）

角色在后台按用户配置频率自主工作，生成主动建议，用户在管家视角的仪表盘能一目了然看到所有角色状态、待处理事项和能量值。完成后是一个主动协作伴侣。

### Story 4.1: 角色后台调度器按配置频率运行工作循环

As a 用户,
I want 角色在应用运行期间按我设定的频率自动审视目标和任务,
So that 不需要我主动发起对话角色也能持续工作。

**Acceptance Criteria:**

**Given** 应用启动且用户有 2+ 个角色
**When** 到达调度时间点
**Then** 调度器逐一触发每个角色的工作循环
**And** 每个角色独立 `tokio::spawn` 执行，互不阻塞

**Given** 角色主动性设为 `moderate`
**When** 调度器运行
**Then** 每日触发 1-2 次工作循环

**Given** 角色主动性设为 `proactive`
**When** 调度器运行
**Then** 每日触发 3-4 次工作循环

**Given** 角色主动性设为 `passive`
**When** 调度器运行
**Then** 跳过该角色（不触发工作循环）

**Given** 应用关闭
**Then** 调度器停止，不消耗系统资源
**And** 下次启动时重新初始化

**Given** Rust 后端
**Then** `scheduler` 模块：`tokio::interval` 实现
**And** 每次循环记录 tracing 日志：`role_id, triggered_at, duration_ms, result`
**And** 循环频率从 `roles.proactivity_level` 动态读取

---

### Story 4.2: 角色工作循环生成主动建议并存储

As a 用户,
I want 角色在工作循环中基于当前状态生成有价值的建议,
So that 我能收到角色的主动帮助而不只是被动响应。

**Acceptance Criteria:**

**Given** 角色工作循环触发
**When** Agent 审视角色目标 + 任务状态 + 记忆
**Then** LLM 生成 0-3 条建议
**And** 每条建议写入 `suggestions` 表（status: `pending`, priority: high/medium/low）

**Given** 生成的建议
**Then** 必须基于角色当前目标、任务状态和记忆
**And** 不能与最近 7 天已生成的建议内容重复（余弦相似度 > 0.85 视为重复）

**Given** 角色无目标、无任务、无近期对话
**When** 工作循环触发
**Then** 不生成建议（允许 0 条输出）

**Given** LLM 生成失败（超时/格式错误）
**Then** tracing 日志记录失败原因
**And** 不向用户报错，等待下次循环

**Given** 数据库
**Then** `migrations/006_suggestions.sql` 创建 `suggestions` 表：`id`, `role_id`, `title`, `content`, `priority`, `status`(pending/confirmed/rejected), `rejection_reason`, `converted_task_id`, `created_at`

**Given** Rust 后端
**Then** `suggestion_generator` 服务：构造 prompt（角色定义 + 目标 + 任务列表 + 最近记忆）→ LLM 返回结构化建议 JSON

---

### Story 4.3: 主动性三档刻度盘行为接通

As a 用户,
I want 不同主动性档位带来明显不同的角色行为体验,
So that 我能精细控制角色的打扰程度。

**Acceptance Criteria:**

**Given** 角色设为 `passive`（静默执行）
**When** 后台调度触发
**Then** 跳过该角色，不生成建议，不发送通知
**And** 仅在用户主动对话时响应

**Given** 角色设为 `moderate`（适度建议）
**When** 后台生成建议
**Then** 仅保留 priority = high/medium 的建议
**And** 通知级别最高为"轻触"（不使用"敲门"）

**Given** 角色设为 `proactive`（积极主动）
**When** 后台生成建议
**Then** 保留所有优先级的建议
**And** high priority 建议可触发"敲门"通知

**Given** 用户在 RoleView SettingsTab 切换主动性档位
**When** 保存后
**Then** 下次调度循环立即生效（无需重启）

**Given** Rust 后端
**Then** `scheduler` 读取 `proactivity_level` 决定：是否跳过、频率、建议过滤规则、通知级别上限

---

### Story 4.4: 用户在管家对话中看到主动建议卡片并确认/拒绝

As a 用户,
I want 在管家视角看到角色生成的待确认建议并快速处理,
So that 我能决定哪些建议值得执行。

**Acceptance Criteria:**

**Given** 有 pending 状态的建议
**When** 用户打开管家视角
**Then** 管家对话区嵌入 ActionCard 组件展示建议（图标 + 标题 + 来源角色 + 时间）

**Given** 用户点击 ActionCard 的"确认"按钮
**When** 确认完成
**Then** `suggestions.status` 更新为 `confirmed`
**And** 自动在对应角色的 `tasks` 表创建新任务（`converted_task_id` 关联）
**And** ActionCard 显示 ✓ 动画后消失

**Given** 用户点击"拒绝"按钮
**When** 弹出拒绝原因选择（不相关/时机不对/已完成/其他）
**Then** `suggestions.status` 更新为 `rejected` + `rejection_reason` 写入
**And** ActionCard 消失

**Given** 拒绝记录
**Then** 角色下次生成建议时 System Prompt 注入"用户曾拒绝以下类型建议：..."
**And** 减少类似建议的生成频率

**Given** 已处理（confirmed/rejected）的建议
**Then** 不再重复显示在管家对话区

**Given** 前端
**Then** `components/butler/ActionCard.tsx` 接通真实建议数据（替换原型 mock 卡片）
**And** `useSuggestions()` hook 封装查询和操作

**Given** Rust 后端
**Then** Tauri commands: `suggestion::list_pending` / `suggestion::confirm { id }` / `suggestion::reject { id, reason }`

---

### Story 4.5: 三级通知系统（耳语/轻触/敲门）

As a 用户,
I want 收到不同紧急程度的通知且不被过度打扰,
So that 重要信息不遗漏同时保持专注。

**Acceptance Criteria:**

**Given** 角色生成"耳语"级通知
**Then** 静默积累到通知列表
**And** 不显示任何前台提示
**And** 在下次晨间简报中汇总

**Given** 角色生成"轻触"级通知
**Then** 侧边栏通知铃铛显示红点 badge
**And** 不弹窗打断用户当前操作

**Given** 角色生成“敲门”级通知
**Then** 管家对话区嵌入 ActionCard 展示通知（图标 + 标题 + 来源角色 + 时间 + “立即处理 / 稍后”按钮）
**And** 侧边栏铃铛显示红点 badge
**And** 可选声音提示（用户在设置中开启，默认关闭）
**And** 遵守 UX-DR16（不使用传统 toast/snackbar）+ UX-DR20（三级通知视觉规则）

**Given** 同一天已有 3 次"敲门"通知
**When** 第 4 次"敲门"触发
**Then** 自动降级为"轻触"
**And** tracing 日志记录降级

**Given** 用户点击通知铃铛
**When** NotificationPanel 打开
**Then** 按时间倒序显示所有通知
**And** 每条显示：角色色标 + 角色名 + 级别标签（耳语/轻触/敲门）+ 内容 + 时间
**And** 敲门级通知标签带 `animate-pulse`

**Given** 数据库
**Then** `migrations/007_notifications.sql` 创建 `notifications` 表：`id`, `role_id`, `level`(whisper/tap/knock), `content`, `is_read`, `created_at`

**Given** Rust 后端
**Then** Tauri commands: `notification::create` / `notification::list` / `notification::mark_read`
**And** Tauri Event: `notification:new { level, content, role_name }` 供前端实时响应

**Given** 前端
**Then** NotificationPanel 接通真实数据（替换原型 mock 列表）
**And** `useNotifications()` hook + `useTauriEvent('notification:new')` 实时更新

---

### Story 4.6: Q2 保护任务被挤时管家主动提醒

As a 用户,
I want 重要但不紧急的任务被忽略时收到管家温和提醒,
So that Q2 任务不会在忙碌中被遗忘。

**Acceptance Criteria:**

**Given** 调度器检测到 `at_risk` 状态的 Q2 任务（来自 E3 Story 3.5）
**When** 生成提醒
**Then** 管家在对话中以自然语言提醒"你的'XX'任务已经 3 天没动了，要不要今天安排一下？"
**And** 不使用 toast / 红框，遵循 UX-DR16 管家对话反馈模式

**Given** 提醒通知
**Then** 级别为"轻触"（不打断用户）

**Given** 同一任务
**Then** 提醒频率 ≤ 每日 1 次
**And** 连续提醒 3 天无响应后停止提醒（避免骚扰）

**Given** 用户处理了该任务（完成/编辑）
**Then** 提醒立即停止
**And** `protection_status` 恢复为 `normal`（由 E3 Story 3.5 逻辑处理）

**Given** Rust 后端
**Then** 调度器中增加 Q2 保护检查步骤
**And** 生成通知 + 管家对话消息

---

### Story 4.7: 仪表盘接通真实角色状态数据

As a 用户,
I want 在管家仪表盘一目了然看到所有角色的实时状态,
So that 快速了解哪些角色需要关注。

**Acceptance Criteria:**

**Given** 用户在 ButlerView 点击"仪表盘"Tab
**When** 有 3 个活跃角色
**Then** 显示 3 张角色卡片，每张含：角色图标+名称、能量值百分比+进度条、待办任务数、最近活跃时间

**Given** 角色卡片排序
**Then** 按视觉优先级排序：
1. 有紧急事项（Q1 任务）→ amber 边框 + `ring-1 ring-amber-200`
2. 能量值 < 40% → red 进度条
3. 正常 → 标准色调

**Given** 能量值色谱
**Then** ≥ 70% = 翠绿 `bg-emerald-500` / ≥ 40% = 琥珀 `bg-amber-500` / < 40% = 红色 `bg-red-500`

**Given** 用户点击角色卡片
**When** 点击
**Then** 切换到该角色视图（复用 Story 2.2 导航逻辑）

**Given** Rust 后端
**Then** Tauri command: `dashboard::get_status` 返回所有活跃角色的聚合数据（energy, pending_tasks_count, last_active_at, has_urgent）

**Given** 前端
**Then** `components/butler/DashboardTab.tsx` 接通真实数据（替换原型 `ROLES` 硬编码）
**And** `useDashboard()` hook 封装查询
**And** 复用原型卡片样式（EnergyIndicator 组件）

---

### Story 4.8: 能量值计算引擎

As a 用户,
I want 每个角色有一个动态变化的"能量值"反映角色健康度,
So that 我能直觉地感知哪个角色需要更多关注。

**Acceptance Criteria:**

**Given** 调度器每次工作循环完成后
**When** 重新计算角色能量值
**Then** 新值写入 `roles.energy_value`（0-100 整数）

**Given** 能量值计算公式（V1 线性加权，对应 PRD §8 Resolved Question 4）
**Then** `energy = 0.4 * task_completion_rate + 0.3 * recent_activity_score + 0.2 * goal_progress + 0.1 * (100 - at_risk_penalty)`
> PRD 追溯：任务完成率(40%)=task_completion_rate / 大石头推进度(30%)≈goal_progress / 用户互动频率(20%)=recent_activity_score / 目标更新活跃度(10%)=(100-at_risk_penalty)
**And** `task_completion_rate` = 最近 7 天完成任务数 / 总任务数 * 100
**And** `recent_activity_score` = 基于最近对话时间衰减（今天=100, 昨天=80, 3天前=50, 7天+=10）
**And** `goal_progress` = 角色目标相关大石头完成率 * 100
**And** `at_risk_penalty` = at_risk 的 Q2 任务数 * 20（最高扣 60）

**Given** 能量值 < 40%
**When** 下降到阈值
**Then** 生成"轻触"通知"你的XX角色能量值较低，可能需要关注"

**Given** 计算过程
**Then** 每次计算结果存入 `roles.energy_value` + `roles.energy_updated_at`
**And** tracing 日志记录计算明细（便于调试和调参）

**Given** 用户手动操作后（完成任务、对话等）
**Then** 不实时重算（等待下次调度循环）
**And** V1 不追求实时性，允许延迟

**Given** Rust 后端
**Then** `energy_calculator` 服务：读取角色任务/对话/记忆数据 → 公式计算 → 更新 roles 表

---

## Epic 5: 使命宣言与冲突仲裁（Mission & Arbitration）

用户设定个人使命，多角色任务冲突时系统主动检测并提供基于使命+四象限+能量平衡的三步仲裁建议。完成后是一个价值观协调器。

### Story 5.1: 用户能设定和编辑个人使命宣言

As a 用户,
I want 设定我的人生使命宣言作为角色间优先级指南,
So that 冲突发生时系统能基于我的价值观给出建议。

**Acceptance Criteria:**

**Given** 数据库
**Then** `migrations/012_mission.sql` 创建 `mission` 表：`id` TEXT PRIMARY KEY, `content` TEXT, `format` TEXT DEFAULT 'free' CHECK(format IN ('free','structured')), `updated_at` TEXT
**And** 仅允许一行数据（INSERT OR REPLACE）

**Given** 用户在 ButlerView SettingsTab 使命宣言区域
**When** 输入自由文本并保存
**Then** 写入 `mission` 表（`content` = 用户输入, `format` = 'free'）
**And** 重启后保留

**Given** 用户点击参考模板（如"家庭优先"）
**When** 点击
**Then** 文本自动填入编辑框（可继续修改）
**And** 模板包含：家庭优先 / 事业与家庭平衡 / 终身学习（来自原型）

**Given** 柯维三段式结构化模板
**Then** 提供可选结构：「角色 → 价值观 → 目标」三段引导
**And** 用户可在自由文本和结构化模板间切换

**Given** 用户选择不设定使命宣言
**Then** `mission` 表无记录或 `content` 为空
**And** 冲突仲裁仍可基于其他维度运行（四象限 + 能量值）

**Given** 用户编辑已有使命宣言
**When** 修改后保存
**Then** 立即生效，下次仲裁使用新版本

**Given** 前端
**Then** ButlerView SettingsTab 使命宣言接通真实数据（替换原型 mock `useState('')`）
**And** 结构化模板 UI 新增（三段输入框 OR 自由文本切换）

**Given** Rust 后端
**Then** Tauri command: `mission::update { content, format }` / `mission::get`
**And** `mission::update` 执行 INSERT OR REPLACE 到 `mission` 表

---

### Story 5.2: 未设定使命时管家基于行为推断隐含价值观

As a 用户,
I want 即使没有设定使命宣言系统也能理解我的优先级,
So that 不需要显式声明也能得到合理的仲裁建议。

**Acceptance Criteria:**

**Given** 用户未设定使命宣言
**When** 冲突仲裁触发
**Then** `agent_engine` 分析最近 30 天的对话/任务/记忆模式
**And** LLM 推断隐含优先级（如"用户频繁优先处理家庭相关任务"）

**Given** 推断结果
**Then** 格式为 2-3 条价值观摘要（如"家庭陪伴 > 工作效率 > 个人学习"）
**And** 在仲裁时替代使命宣言作为第一步依据

**Given** 用户在 ButlerView SettingsTab 查看推断结果
**Then** 显示"系统推断的优先级"区域（灰色提示框）
**And** 用户可点击"确认采纳"将推断转为正式使命宣言
**And** 用户可点击"不准确"并手动设定

**Given** 推断依据不足（新用户，对话 < 5 轮）
**Then** 不显示推断
**And** 仲裁仅基于四象限 + 能量值

**Given** Rust 后端
**Then** `mission_inferrer` 服务：收集历史数据 → 构造推断 prompt → 返回结构化优先级 JSON

---

### Story 5.3: 系统自动检测多角色任务的时间冲突

As a 用户,
I want 系统在发现日程冲突时主动告诉我,
So that 我不会忘记处理时间重叠的任务。

**Acceptance Criteria:**

**Given** 调度器定时扫描所有角色的 tasks
**When** 发现两个或多个角色的任务 deadline 在同一时段（±1 小时内）
**Then** 生成冲突记录写入 `conflicts` 表

**Given** 冲突检测结果
**When** 新冲突产生
**Then** 生成"敲门"级通知"发现时间冲突：产品经理 vs 家庭，周五 15:00"
**And** 通知中包含"处理仲裁"按钮

**Given** 冲突已被用户处理（采纳/延后/拒绝）
**Then** 不重复检测同一组冲突

**Given** 用户手动调整了冲突任务的时间
**Then** 冲突自动解除，不再提醒

**Given** 数据库
**Then** `migrations/008_conflicts.sql` 创建 `conflicts` 表：`id`, `task_id_a`, `task_id_b`, `role_id_a`, `role_id_b`, `conflict_time`, `status`(detected/resolved/dismissed), `resolution`, `created_at`

**Given** Rust 后端
**Then** `conflict_detector` 服务：扫描 deadline 重叠 → 写入 conflicts 表 → 发送通知
**And** 调度器每次循环包含冲突检测步骤

---

### Story 5.4: 管家执行三步仲裁协议并生成双赢方案

As a 用户,
I want 管家基于使命+四象限+能量给出有理有据的仲裁建议,
So that 我能在冲突中做出更好的决策。

**Acceptance Criteria:**

**Given** 冲突被检测到
**When** 用户点击"处理仲裁"或系统自动触发
**Then** `arbitration_engine` 执行三步协议：
1. **使命对齐**：检查使命宣言（或推断优先级）中哪个角色优先
2. **四象限定位**：对比冲突任务的 quadrant（Q1 > Q2 > Q3 > Q4）
3. **能量平衡**：对比两个角色的 energy_value

**Given** 三步协议完成
**When** LLM 综合三个维度
**Then** 生成 2 个方案（方案 A 标记为"推荐"）
**And** 每个方案包含：标题、具体行动描述、受益分析
**And** 方案尽可能双赢（调整时间而非取消）

**Given** Q2 保护任务参与冲突
**Then** 仲裁中 Q2 保护权重提高（避免 Q2 被 Q1 完全压制）
**And** 方案中明确说明 Q2 保护考量

**Given** 数据库
**Then** `migrations/009_arbitrations.sql` 创建 `arbitrations` 表：`id`, `conflict_id`, `mission_alignment`, `quadrant_analysis`, `energy_analysis`, `proposals`(JSON), `user_choice`, `created_at`
**And** 外键关联 `conflicts.id`

**Given** 仲裁结果
**Then** 写入 `arbitrations` 表

**Given** Rust 后端
**Then** `arbitration_engine` 服务：读取冲突 + 使命 + 任务属性 + 能量值 → 构造仲裁 prompt → LLM 返回结构化方案 JSON

---

### Story 5.5: ArbitrationModal 接通真实仲裁数据并可视化三步过程

As a 用户,
I want 在清晰的界面中看到冲突分析和方案选择,
So that 我能直观理解仲裁逻辑并做出决定。

**Acceptance Criteria:**

**Given** 仲裁触发
**When** ArbitrationModal 打开
**Then** 顶部显示冲突双方对比卡片：
- 左侧：角色 A 名称+图标+任务描述+四象限标签
- 右侧：角色 B 名称+图标+任务描述+四象限标签
- 中间："VS" 分隔

**Given** 使命宣言依据区
**Then** 显示当前使命宣言（或推断优先级）引用文本
**And** 灰色斜体样式 + "使命宣言依据" 标签

**Given** 方案展示
**Then** 方案 A（推荐）= 翠绿卡片 + 💡 图标
**And** 方案 B = 蓝色卡片 + 🔄 图标
**And** 每个方案显示标题 + 具体行动描述

**Given** 底部操作区
**Then** "最终决定权在你手中。" 文案居中
**And** 按钮：延后处理 / 采纳建议（选择方案 A 或 B）

**Given** 前端
**Then** ArbitrationModal 接通真实仲裁数据（替换原型 mock 内容）
**And** `useArbitration(conflictId)` hook 封装数据加载

**Given** Rust 后端
**Then** Tauri command: `arbitration::get { conflict_id }` 返回完整仲裁结果

---

### Story 5.6: 用户采纳仲裁建议后系统自动执行方案

As a 用户,
I want 选择方案后系统自动帮我调整任务安排,
So that 不需要手动逐个修改任务。

**Acceptance Criteria:**

**Given** 用户在 ArbitrationModal 点击"采纳建议"选择方案 A
**When** 执行方案
**Then** 自动调整涉及任务（如：修改 deadline / 标记委托 / 调整 quadrant）
**And** `conflicts.status` 更新为 `resolved`
**And** `arbitrations.user_choice` 记录 "proposal_a"
**And** ArbitrationModal 关闭

**Given** 用户点击"延后处理"
**When** 操作完成
**Then** `conflicts.status` 保持 `detected`
**And** ArbitrationModal 关闭
**And** 24 小时后再次提醒（降级为"轻触"）

**Given** 用户拒绝所有方案（两次延后后出现"自行处理"选项）
**When** 选择自行处理
**Then** `conflicts.status` 更新为 `dismissed`
**And** 不再提醒该冲突

**Given** 仲裁采纳记录
**Then** 每次采纳/拒绝记录用于统计采纳率（SM-5 目标 ≥ 50%）
**And** 拒绝原因供后续仲裁学习

**Given** Rust 后端
**Then** Tauri commands: `arbitration::adopt { conflict_id, proposal }` / `arbitration::defer { conflict_id }` / `arbitration::dismiss { conflict_id }`
**And** `adopt` 内部调用 `task::update` 自动调整任务属性

---

## Epic 6: 节奏化简报与复盘（Daily Briefing & Weekly Review Rhythm）

用户每日打开应用收到晨间简报，每周末收到正向叙事的复盘成绩单，每周开始有大石头规划引导。完成后是一个节奏化生活伴侣。

### Story 6.1: 管家每日生成晨间简报并以自然语言呈现

As a 用户,
I want 每天打开应用就能看到管家为我汇总的今日概览,
So that 快速了解各角色状态和今天最重要的事。

**Acceptance Criteria:**

**Given** 到达用户配置的晨间简报时间
**When** 调度器触发简报生成
**Then** LLM 基于以下输入生成自然语言段落：
- 各角色当前状态（能量值 + 待处理任务数）
- 今日截止的任务列表
- 昨日新增的记忆/建议
- 耳语级通知的汇总

**Given** 简报内容
**Then** 以管家对话消息形式呈现在 ButlerView 对话区
**And** 段落中嵌入角色名称链接（点击跳转对应角色视图）
**And** 结尾含"今天最重要的一件事"建议

**Given** 简报中有待确认建议
**Then** 嵌入 ActionCard 组件（复用 E4 Story 4.4）

**Given** 用户当日已查看简报
**Then** 不重复生成

**Given** 用户在简报时间前打开应用
**Then** 显示上次简报内容（若有）
**And** 到达简报时间后自动追加新简报

**Given** 数据库
**Then** `migrations/010_briefings.sql` 创建 `briefings` 表：`id`, `content`, `date`, `created_at`

**Given** Rust 后端
**Then** `briefing_generator` 服务：收集各角色数据 → 构造简报 prompt → LLM 返回自然语言段落
**And** 简报写入 `briefings` 表

---

### Story 6.2: 用户能配置晨间简报、周复盘和大石头规划的触发时间

As a 用户,
I want 自定义各节奏化功能的触发时间,
So that 适配我的作息和生活习惯。

**Acceptance Criteria:**

**Given** 用户在 ButlerView SettingsTab 配置晨间简报时间
**When** 修改时间（如 07:30）并保存
**Then** `app_settings.briefing_time` 更新
**And** 调度器下次按新时间触发

**Given** 用户配置周复盘触发时间
**When** 修改星期和时间（如周日 20:00）并保存
**Then** `app_settings.review_day` + `app_settings.review_time` 更新

**Given** 用户配置大石头规划补触发时间
**When** 修改星期和时间（如周一 09:00）并保存
**Then** `app_settings.bigrock_reminder_day` + `app_settings.bigrock_reminder_time` 更新
**And** 可选星期：周一 / 周二

**Given** 默认值
**Then** 晨间简报：08:00 / 周复盘：周日 20:00 / 大石头补触发：周一 09:00

**Given** 前端
**Then** ButlerView SettingsTab 时间配置接通真实数据（替换原型 mock `defaultValue`）
**And** 大石头补触发时间配置新增 UI（星期选择 + 时间选择）

**Given** Rust 后端
**Then** Tauri commands: `settings::update_schedule { briefing_time, review_day, review_time, bigrock_reminder_day, bigrock_reminder_time }`
**And** 调度器启动时从 `app_settings` 读取所有时间配置

---

### Story 6.3: 每周初管家检测并补充引导大石头规划

As a 用户,
I want 如果我在周复盘中没有规划大石头系统会提醒我补上,
So that 每周都有明确的优先级锚点。

**Acceptance Criteria:**

**Given** 大石头规划的主入口是 WeeklyReviewModal 的规划阶段（Story 6.5 phase='plan'）
**Then** 用户在周复盘中完成规划后，本周大石头已设定 → 无需额外触发

**Given** 用户在周复盘中跳过了规划阶段（关闭 Modal 未进入 plan 阶段）
**When** 到达 `app_settings.bigrock_reminder_day` + `bigrock_reminder_time`（默认周一 09:00）
**Then** 调度器检测本周是否有 `is_big_rock=true` 的任务

**Given** 检测结果为无大石头
**When** 补触发
**Then** 管家发送"轻触"通知"还没规划本周大石头，要安排一下吗？"
**And** 通知中包含"开始规划"按钮

**Given** 用户点击"开始规划"
**When** 操作
**Then** 直接打开 WeeklyReviewModal 的规划阶段（phase='plan'）

**Given** 用户完全未打开过周复盘
**Then** 同样在补触发时间检测并提醒

**Given** 本周已有大石头（无论来源：周复盘规划 / 手动标记 / 上周延续）
**Then** 不触发补提醒

**Given** Rust 后端
**Then** 调度器增加大石头检测步骤：查询本周 `tasks` where `is_big_rock = true`
**And** 检测时间从 `app_settings.bigrock_reminder_day/time` 读取

---

### Story 6.4: 每周末管家生成周复盘成绩单（正向叙事）

As a 用户,
I want 每周收到一份温暖的成绩单回顾本周进展,
So that 感受到进步的满足感并为下周做准备。

**Acceptance Criteria:**

**Given** 到达周复盘触发时间
**When** 调度器触发复盘生成
**Then** LLM 基于以下输入生成复盘内容：
- 各角色能量值变化趋势（本周 vs 上周）
- 大石头完成情况
- 新沉淀的记忆条数
- 新启用/使用的 Skill
- 任务完成统计

**Given** 复盘叙事风格
**Then** 使用正向框架：
- 已完成 = ✓ 图标 + 翠绿色
- 继续推进 = → 图标 + 琥珀色
- **禁止使用"未完成""失败"等负面措辞**

**Given** 复盘以反思对话形式呈现
**Then** 管家对话区展示复盘摘要段落
**And** 末尾显示"查看详细复盘"按钮 → 打开 WeeklyReviewModal

**Given** 复盘生成后
**Then** 发送"轻触"通知"本周复盘已准备好"

**Given** 数据库
**Then** `migrations/011_weekly_reviews.sql` 创建 `weekly_reviews` 表：`id`, `week_start`, `week_end`, `summary`, `energy_trends`(JSON), `bigrock_status`(JSON), `new_memories_count`, `created_at`

**Given** Rust 后端
**Then** `review_generator` 服务：收集本周数据 → 构造复盘 prompt → LLM 返回结构化复盘 JSON
**And** 复盘写入 `weekly_reviews` 表

---

### Story 6.5: WeeklyReviewModal 接通真实复盘数据和规划功能

As a 用户,
I want 在可视化界面中回顾本周并规划下周大石头,
So that 有仪式感地完成每周的节奏闭环。

**Acceptance Criteria:**

**Given** 用户打开 WeeklyReviewModal（phase='review'）
**When** 复盘阶段
**Then** 顶部显示自然语言复盘摘要（LLM 生成，替换原型 mock 文本）
**And** 左侧：角色能量趋势柱状图（真实 energy_value 数据，替换原型 `ROLES` 硬编码）
**And** 右侧：本周大石头完成列表（✓ 完成 / → 继续推进）

**Given** 用户点击"规划下周大石头"按钮
**When** 切换到 phase='plan'
**Then** 每个活跃角色显示一个规划卡片
**And** AI 建议 1-2 个大石头（基于上周能量 + 未完成目标 + 角色目标）
**And** 用户可点击"采纳"自动填入 / 手动输入 / 添加多个

**Given** 用户点击"确认规划"
**When** 保存
**Then** 每个填入的大石头创建为 `tasks`（`is_big_rock = true`, 角色关联）
**And** 触发四象限自动分类（复用 E3 Story 3.3）
**And** Modal 关闭

**Given** 周复盘日期范围
**Then** 自动计算本周一到本周日
**And** 显示格式"YYYY年M月D日 - D日"

**Given** 前端
**Then** WeeklyReviewModal 接通真实数据（替换原型所有 mock 数据）
**And** `useWeeklyReview(weekStart)` hook 封装复盘数据加载
**And** `useBigRockPlanning()` hook 封装 AI 建议和保存

**Given** Rust 后端
**Then** Tauri commands: `review::get_weekly { week_start }` / `review::plan_bigrocks { items: Vec<{role_id, title}> }`

---

### Story 6.6: 管家在工作日保护大石头任务不被低优先级挤掉

As a 用户,
I want 管家帮我守住本周最重要的事,
So that 大石头不会被琐事淹没。

**Acceptance Criteria:**

**Given** 工作日调度循环中检测大石头进展
**When** 大石头任务本周未有任何进展（无编辑/无对话提及/未完成子步骤）
**Then** 管家在对话中温和提醒"你的大石头'XX'这周还没动，要不要今天安排一点时间？"

**Given** 低优先级任务（Q3/Q4）与大石头时间冲突
**When** 冲突检测触发（复用 E5 Story 5.3）
**Then** 仲裁中大石头权重自动加成
**And** 方案倾向保护大石头时间

**Given** 大石头保护提醒
**Then** 级别为"轻触"
**And** 每个大石头每日最多提醒 1 次
**And** 大石头已完成或已有当日进展 → 不提醒

**Given** 周五仍有未完成大石头
**When** 调度触发
**Then** 管家提醒"本周大石头还有 N 个未完成，周末要安排时间吗？"

**Given** Rust 后端
**Then** 调度器增加大石头保护检查步骤
**And** 与 E5 仲裁引擎联动（大石头 `is_big_rock` 在仲裁 prompt 中加权）

---

## Epic 7: 数据主权（Data Sovereignty）

用户随时可一键导出全部数据为 JSON/Markdown，或一键销毁全部数据回到初始状态。完成后用户对自己的数据拥有完全控制。

### Story 7.1: 用户能一键导出全部数据为 JSON 和 Markdown

As a 用户,
I want 随时将我的所有数据导出为标准格式,
So that 我拥有数据的完全控制权且可迁移。

**Acceptance Criteria:**

**Given** 用户在 GlobalSettingsModal 数据 Tab 点击"导出存档"
**When** Tauri 文件保存对话框弹出，用户选择保存目录
**Then** 生成两个文件：
- `egosync-export-{date}.json`：完整 schema 的 JSON（含所有表数据）
- `egosync-export-{date}.md`：人类可读的 Markdown 版本

**Given** JSON 导出内容
**Then** 包含：角色定义、任务、记忆、对话历史、使命宣言、建议、通知、冲突仲裁记录、周复盘、app_settings
**And** 每个实体保留完整字段和关联关系

**Given** Markdown 导出内容
**Then** 按角色分章节，每章包含：角色信息 → 任务列表 → 记忆条目 → 对话摘要
**And** 可读性优先（无 UUID，使用角色名/任务标题）

**Given** 导出性能
**Then** 30 秒内完成（含 1000+ 条记忆和 500+ 条对话）
**And** 导出期间显示进度条

**Given** 导出成功
**Then** 显示成功提示 + 文件路径链接

**Given** Rust 后端
**Then** Tauri command: `data::export { dir_path }` → 查询所有表 → 序列化 JSON + 生成 Markdown → 写入文件
**And** 使用 Tauri `dialog::save_file` API 选择保存位置

---

### Story 7.2: 用户能一键销毁全部数据并回到初始状态

As a 用户,
I want 在不想继续使用时彻底删除所有数据,
So that 确保我的隐私不留残余。

**Acceptance Criteria:**

**Given** 用户在 GlobalSettingsModal 数据 Tab 点击"销毁所有数据"
**When** 第一次确认弹窗
**Then** 显示警告"此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。"
**And** 要求用户输入"确认销毁"四个字才能继续

**Given** 用户输入确认文字并点击"确认"
**When** 销毁执行
**Then** 清空所有数据表（DELETE FROM 每张表，保留 `_sqlx_migrations` schema 元数据，不重新跑 migration）
**And** 删除本地缓存文件
**And** 销毁完成后应用自动跳转到 Onboarding 引导页面

**Given** 销毁后状态
**Then** 数据库为空（仅保留 schema）
**And** `app_settings` 恢复为默认值
**And** Sidebar 无角色
**And** 等同全新安装

**Given** 用户取消操作
**Then** 无任何副作用

**Given** Rust 后端
**Then** Tauri command: `data::destroy` → 事务内 DELETE FROM 所有数据表（保留 `_sqlx_migrations`）→ 重置 app_settings 为默认 key-value → 返回成功
**And** 操作前自动创建隐藏备份（`egosync-backup-{timestamp}.json`，写入临时目录，7 天后自动清理）

---

### Story 7.3: GlobalSettingsModal 数据 Tab 接通真实导出和销毁功能

As a 用户,
I want 在全局设置中方便地找到数据管理功能,
So that 导出和销毁操作简单直观。

**Acceptance Criteria:**

**Given** 用户在 GlobalSettingsModal 点击"数据与主权" Tab
**When** Tab 打开
**Then** 上方显示导出区域：标题 + 说明 + "导出存档"按钮
**And** 下方显示危险区域：红色标题 + 警告说明 + "销毁所有数据"按钮

**Given** 导出按钮点击
**When** 导出执行中
**Then** 按钮变为加载状态（spinner + "导出中..."）
**And** 完成后显示 ✓ "导出完成" + 文件路径

**Given** 销毁按钮点击
**When** 二次确认弹窗显示
**Then** 确认输入框 + 取消/确认按钮
**And** 确认按钮在用户输入正确文字前禁用

**Given** 前端
**Then** GlobalSettingsModal 数据 Tab 接通真实后端（替换原型 mock 按钮）
**And** 导出使用 `@tauri-apps/api/dialog` 的 `save` API
**And** 销毁确认 Modal 新增（内联在数据 Tab 中）

**Given** Rust 后端
**Then** 复用 Story 7.1 / 7.2 的 Tauri commands

---

## Epic 8: 跨平台分发与 V1 加固（Cross-Platform Distribution & V1 Hardening）

用户在 Windows/macOS/Linux 三个平台都能下载安装稳定的 V1 安装包，应用已通过端到端核心旅程验证。完成后是一个三平台 V1 产品。

### Story 8.1: GitHub Actions 三平台并行 CI 与自动构建产物

As a 开发者,
I want 每次代码推送自动在三平台构建并生成安装包,
So that 确保跨平台兼容性且随时可发布。

**Acceptance Criteria:**

**Given** 代码推送到 main 分支或 PR 创建
**When** GitHub Actions 触发
**Then** 三平台并行构建矩阵：
- Windows: `windows-latest` → MSI 安装包
- macOS: `macos-latest` → DMG 安装包
- Linux: `ubuntu-latest` → AppImage 安装包

**Given** 构建成功
**Then** 三平台产物上传为 GitHub Actions Artifacts
**And** 产物命名：`egosync-{version}-{platform}.{ext}`

**Given** 构建失败
**Then** PR 合并被阻断
**And** 失败日志清晰标注平台和错误位置

**Given** CI 流水线
**Then** 步骤：checkout → setup Rust → setup Node → install deps → `cargo test` → `npm run build` → `tauri build`
**And** 使用缓存加速（Rust target + node_modules）

**Given** Tauri 配置
**Then** `tauri.conf.json` 中 bundle 配置完整（identifier、icons、版本号）

---

### Story 8.2: E2E 测试套件覆盖核心用户旅程

As a 开发者,
I want 自动化测试覆盖所有核心用户旅程,
So that 每次发布前确保功能完整无回归。

**Acceptance Criteria:**

**Given** E2E 测试框架
**Then** 使用 Tauri WebDriver（`tauri-driver` + WebDriverIO）
**And** 测试在 CI 中自动运行

**Given** 7 条核心旅程测试用例
**Then** 覆盖：
1. **冷启动引导**：首次启动 → Onboarding → 创建角色 → 进入管家视图
2. **管家对话**：发送消息 → LLM 流式响应 → 消息显示
3. **角色 CRUD**：创建角色 → 编辑名称 → 归档 → 恢复 → 删除
4. **LLM 流式响应**：发送请求 → token 逐字显示 → 完成标记
5. **任务管理**：创建任务 → 四象限分类 → 标记完成 → 删除
6. **冲突仲裁**：检测冲突 → 打开仲裁 Modal → 采纳方案
7. **简报复盘**：查看晨间简报 → 打开周复盘 → 规划大石头

**Given** 测试失败
**Then** 阻断 Release 发布
**And** 截图 + 日志附加到 CI 产物

**Given** 测试性能
**Then** 全部 7 条旅程总耗时 ≤ 5 分钟

---

### Story 8.3: WCAG 2.1 AA 无障碍审计与修复

As a 用户,
I want 应用符合无障碍标准,
So that 视觉障碍或行动不便的用户也能使用。

**Acceptance Criteria:**

**Given** 颜色对比度
**Then** 所有文本与背景对比度 ≥ 4.5:1（使用 axe-core 自动检测）
**And** 大号文本（≥ 18px）对比度 ≥ 3:1

**Given** 色盲友好
**Then** 所有状态指示不仅靠颜色区分
**And** 能量值色谱增加图标形状辅助（如低能量 = 红色 + ⚠️ 图标）
**And** 四象限标签增加前缀文字（不仅靠颜色）

**Given** 键盘导航
**Then** 完整 Tab 键序覆盖：Sidebar → 管家对话 → 工作面板 → Modal
**And** Enter 激活按钮 / Escape 关闭 Modal
**And** 可见焦点环（`focus-visible` 样式）

**Given** 屏幕阅读器
**Then** 所有交互元素有 `aria-label`
**And** 流式输出区域 `role="log" aria-live="polite"`
**And** Modal 有 `role="dialog" aria-modal="true"`

**Given** 动效偏好
**Then** `prefers-reduced-motion: reduce` 时关闭所有动效和过渡（呼吸动效、滑入、淡出）
**And** 功能不受影响

**Given** 审计工具
**Then** 使用 `axe-core` + `pa11y` 自动扫描
**And** 0 个 critical/serious 级别问题

---

### Story 8.4: 性能基准验证与优化

As a 用户,
I want 应用在所有平台都流畅响应,
So that 使用体验不因平台差异而打折。

**Acceptance Criteria:**

**Given** 动效性能（NFR-5）
**Then** 角色卡片呼吸动效 ≥ 60fps（Chrome DevTools Performance 验证）
**And** hover/过渡动画无掉帧
**And** CSS transition + GPU 加速（`transform`, `opacity`），不使用 JS 动画

**Given** 首次体验时间（NFR-9，回归验证 Story 1.8 基线）
**Then** 新用户从打开到创建第一个角色 ≤ 5 分钟
**And** Onboarding 流程步骤 ≤ 4 步

**Given** 流式输出一致性（NFR-11）
**Then** 三平台 LLM 流式 token 渲染延迟差异 ≤ 50ms
**And** Windows/macOS/Linux WebView 差异已在流式渲染层抽象

**Given** 冷启动时间
**Then** 应用启动到可交互 ≤ 3 秒（SSD 环境）
**And** SQLite 初始化 + 调度器启动不阻塞 UI

**Given** 内存占用
**Then** 稳态运行内存 ≤ 200MB（3 角色 + 100 条记忆）
**And** 无内存泄漏（1 小时持续使用后内存增长 ≤ 10%）

**Given** 性能回归检测
**Then** CI 中增加性能基准测试（冷启动时间 + 内存快照）
**And** 超过阈值时警告（不阻断）

---

### Story 8.5: Release 自动发布与版本管理

As a 开发者,
I want 通过 Git tag 自动触发正式发布,
So that 发布流程标准化且可重复。

**Acceptance Criteria:**

**Given** 推送 `v*` 格式的 Git tag（如 `v1.0.0`）
**When** GitHub Actions 触发 Release 工作流
**Then** 三平台构建 → E2E 测试通过 → 自动创建 GitHub Release
**And** 三平台安装包上传为 Release Assets

**Given** Release 信息
**Then** 自动生成 CHANGELOG（基于 conventional commits）
**And** Release Notes 包含：新功能、修复、破坏性变更

**Given** 版本号
**Then** 语义化版本（semver）：`MAJOR.MINOR.PATCH`
**And** `tauri.conf.json` 版本号与 Git tag 一致

**Given** macOS 签名（可选）
**Then** 支持 Apple notarize（需配置 Apple Developer 证书）
**And** 未签名时 DMG 可运行但显示安全提示

**Given** Windows 签名（可选）
**Then** 支持代码签名证书
**And** 未签名时 MSI 显示 SmartScreen 警告

**Given** 发布后验证
**Then** 自动下载各平台安装包并验证文件完整性（SHA256 校验）
