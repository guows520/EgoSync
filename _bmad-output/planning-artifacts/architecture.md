---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
baselineStepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments: ['prd-egosync.md', 'ux-design-specification.md', 'brainstorming-session-2026-05-18-1000.md', 'egosync-app/src/App.tsx', 'addendum.md', '.decision-log.md']
workflowType: 'architecture'
lastStep: 8
status: 'complete'
baselineCompletedAt: '2026-05-25'
updated: '2026-08-25'
completedAt: '2026-07-23'
project_name: '探索'
user_name: 'boss'
date: '2026-05-25'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements (36 FRs across 12 domains):**

| 功能域 | FR范围 | 架构影响 |
|--------|--------|----------|
| 管家对话与路由 | FR-1~3 | 意图解析引擎、任务路由、System Prompt 管理 |
| 角色管理 | FR-4~6 | 多Agent实例化、CRUD、Skill注册表 |
| 结构化记忆 | FR-7~9 | 记忆提炼管线、溯源索引、选择性遗忘 |
| 工作循环与建议 | FR-10~12 | 后台调度器、建议状态机、主动性配置 |
| 使命宣言与仲裁 | FR-13~15 | 仲裁协议引擎、冲突检测、优先级层级 |
| 晨间简报与周复盘 | FR-16~18 | 定时生成器、自然语言综合 |
| 角色仪表盘 | FR-19~21 | 前端状态聚合、实时更新 |
| 三级通知 | FR-22 | 通知分级引擎、频率限制 |
| 智能四象限 | FR-23~24 | 自动分类模型、Q2保护规则 |
| 数据主权 | FR-25~27 | 本地SQLite、导出/销毁机制 |
| LLM配置 | FR-28 | opencode多Provider体系（30+原生支持） |
| **Agent引擎集成** | **FR-31~36** | **opencode sidecar进程、Agent Loop、权限模型、工具复用、Session持久化** |

**Non-Functional Requirements (Architecture Drivers):**

- **本地优先**：零云端依赖，断网完整可用
- **Tauri 桌面应用**：Rust后端 + WebView前端 + opencode sidecar
- **BYOK模式**：用户自带 API Key，opencode原生支持30+ Provider
- **Agent引擎**：opencode sidecar提供完整Agent Loop、内置工具、Skill系统、Subagent、MCP
- **性能**：WebView渲染60fps动效、LLM流式输出实时渲染
- **数据隐私**：所有数据本地SQLite，LLM调用不存储内容
- **跨平台**：Windows/macOS/Linux 一致体验

**Scale & Complexity:**

- Primary domain: Full-stack desktop application (Tauri = Rust backend + React frontend)
- Complexity level: High — Multi-Agent autonomous system + local LLM integration + background scheduling + structured memory
- Estimated architectural components: ~15 core modules

### Technical Constraints & Dependencies

| 约束 | 来源 | 架构影响 |
|------|------|----------|
| 前端已固定 | egosync-app/App.tsx 1458行原型 | React 18 + TS + Vite + Tailwind，只接API |
| Tauri框架 | PRD §Platform | Rust后端处理业务逻辑，通过IPC与前端通信 |
| opencode sidecar | PRD §4.12 FR-31 | opencode binary打包进安装包，Rust管理其生命周期，通过HTTP API通信 |
| 本地SQLite | FR-25 | 所有持久化走SQLite，需设计schema |
| opencode SQLite | FR-36 | opencode自身session/message存储独立于EgoSync主DB |
| LLM流式 | UX Spec | 必须支持SSE/流式token渲染（通过opencode Event stream） |
| 无后台daemon | FR-10 Assumption | 工作循环仅在应用运行时执行 |
| 应用运行时执行 | PRD §Assumption | 后台任务 = 应用内定时器，非系统服务 |
| opencode端口 | FR-31 Assumption | opencode server占用本地端口（默认4096），需可配置 |

### Cross-Cutting Concerns Identified

1. **opencode Sidecar管理** — 进程生命周期（启动/停止/重启/健康检查）、端口分配、配置同步
2. **Agent引擎通信层** — Rust后端通过HTTP API调用opencode server，封装为统一的AgentBridge服务
3. **记忆管线** — 对话→结构化提炼→存储→检索→溯源，贯穿管家和所有角色
4. **Tauri IPC通道** — 前端每个操作都需要通过IPC调用Rust后端，需设计统一的命令协议
5. **角色生命周期管理** — 创建/归档/删除/恢复跨越UI、EgoSync DB、opencode agent config
6. **通知与建议状态** — 从工作循环产生→分级→呈现→用户响应→反馈学习
7. **权限同步** — EgoSync角色权限配置 ↔ opencode permission规则的双向映射

## Starter Template Evaluation

### Primary Technology Domain

Full-stack desktop application (Tauri 2.x = Rust backend + React/Vite WebView frontend)

### Setup Strategy

**Manual Setup** — 保留现有 egosync-app/ 前端原型代码，通过 `tauri init` 附加 Rust 后端层。

**理由：** 前端已有1458行高保真原型（React 18 + TS + Vite + TailwindCSS），无需从模板重建，只需桥接 Tauri IPC 层。

### Initialization Commands

```bash
# 1. 在 GUI 目录，为现有前端添加 Tauri 后端
cd GUI
npm install -D @tauri-apps/cli@latest @tauri-apps/api@latest
npx tauri init
# 配置: devUrl=http://localhost:5173, devCommand="npm run dev", buildCommand="npm run build"

# 2. Rust 依赖（在 src-tauri/Cargo.toml 中配置）
# tauri 2.x, serde, sqlx(sqlite+runtime-tokio), tokio
```

### Architectural Decisions Provided by Starter

**Language & Runtime:**
- Frontend: TypeScript (strict mode, React 18)
- Backend: Rust (edition 2021, tokio async runtime)
- IPC: Tauri Command system (invoke from JS → Rust handler)

**Styling Solution:**
- TailwindCSS 3 + tailwindcss-animate (已配置于 egosync-app/)
- Lucide React icons
- 自定义色温系统（角色切换）

**Chart Library:**
- 自绘 SVG（周复盘能量趋势图）— 仅需简单柱状图/折线图，无需引入 Recharts/Chart.js 等重依赖
- 理由：图表场景单一（仅 WeeklyReviewModal 能量趋势），自绘 SVG 零依赖且 GPU 加速友好
- V2 图表场景增多时可考虑引入 Recharts（轻量、React 原生、基于 SVG）

**Build Tooling:**
- Frontend: Vite 5 (HMR dev, optimized production build)
- Backend: Cargo (Rust compiler, release optimizations)
- Desktop packaging: Tauri bundler (MSI/DMG/AppImage)

**Database Layer:**
- SQLx 0.8+ with SQLite backend
- Async runtime: tokio
- Compile-time SQL verification via `sqlx::query!`
- Migration management: `sqlx migrate`

**State Management:**
- Frontend: React useState/useEffect (当前原型模式，后续可引入 Zustand 如需)
- Backend: Tauri managed state (Arc<Mutex<T>> pattern)

**Testing Framework:**
- Frontend: Vitest + React Testing Library
- Backend: Rust built-in #[cfg(test)] + sqlx test fixtures
- E2E: Tauri driver (WebDriver protocol)

**Code Organization:**

```
EgoSync/
├── egosync-app/                    # 前端（现有原型）
│   ├── src/
│   │   ├── components/     # 拆分后的 React 组件
│   │   ├── hooks/          # 自定义 hooks (useTauriCommand 等)
│   │   ├── services/       # API 层（封装 Tauri invoke）
│   │   ├── stores/         # 状态管理
│   │   └── types/          # TypeScript 类型定义
│   └── src-tauri/          # Rust 后端
│       ├── src/
│       │   ├── commands/   # Tauri IPC 命令处理
│       │   ├── agents/     # Agent 引擎（管家+角色）
│       │   ├── agent_bridge/ # opencode HTTP API 客户端
│       │   ├── sidecar/    # opencode 进程管理（启动/停止/健康检查）
│       │   ├── llm/        # LLM Provider 兼容层（降级方案）
│       │   ├── memory/     # 结构化记忆管线
│       │   ├── scheduler/  # 工作循环调度器
│       │   ├── db/         # SQLx 数据访问层
│       │   └── models/     # 数据模型
│       ├── migrations/     # SQLx 数据库迁移
│       ├── resources/      # Tauri sidecar资源
│       │   └── opencode    # opencode binary (平台特定)
│       └── Cargo.toml
├── skills/                 # EgoSync内置Skills (SKILL.md格式)
│   ├── task-management/SKILL.md
│   ├── daily-briefing/SKILL.md
│   └── weekly-review/SKILL.md
└── docs/
```

**Note:** 项目初始化（tauri init + Rust依赖配置）应作为第一个实现故事。

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Agent引擎集成：opencode sidecar + HTTP API bridge
- 数据架构：EgoSync DB（角色元数据/记忆/任务）+ opencode DB（session/message）
- IPC协议：前端 ↔ Rust ↔ opencode 三层通信

**Important Decisions (Shape Architecture):**
- 角色→Agent映射：EgoSync角色配置同步为opencode agent config
- 权限模型：opencode permission系统 + EgoSync UI配置
- Skill体系：opencode SKILL.md格式 + EgoSync内置Skills
- 状态同步：事件驱动增量更新

**Deferred Decisions (Post-MVP):**
- DB加密（V2可选SQLCipher）
- 前端状态管理升级（V2视复杂度引入Zustand）
- Actor模型（V2角色数量增长后评估）
- opencode多workspace支持

### Data Architecture

**Schema组织：单一DB文件 + 表级隔离**
- 理由：管家需跨角色只读索引所有记忆(PRD §Glossary)；单DB简化 JOIN；表通过 role_id 外键隔离
- 影响：所有后端数据访问模块

**记忆存储：JSON字段 + 关系索引混合**
- 理由：结构化记忆内容半结构化（偏好/认知模型），`content JSON` 存灵活数据，`type/role_id/created_at` 做索引列
- 影响：记忆管线、Agent引擎上下文注入

**对话历史：分离存储（主DB + 对话日志DB）**
- 理由：PRD FR-8 明确要求原始日志独立存储，不参与日常推理
- 影响：对话模块、溯源查询

**迁移策略：SQLx migrate**
- 理由：与SQLx工具链一致，SQL文件版本化，编译时校验

**核心表设计：**

```
主数据库 (egosync.db):
├── roles          — 角色定义（name, icon, color, goal, status, energy, settings_json）
├── memories       — 结构化记忆（role_id, type, content_json, source_ref, created_at）
├── tasks          — 任务（role_id, content, quadrant, status, deadline, is_big_rock）
├── suggestions    — 主动建议（role_id, content, status[pending/accepted/rejected], created_at）
├── notifications  — 通知（role_id, level[whisper/tap/knock], content, read, created_at）
├── mission        — 使命宣言（content, format[free/structured], updated_at）
├── conflicts      — 冲突检测（task_id_a, task_id_b, role_id_a, role_id_b, conflict_time, status, resolution）
├── arbitrations   — 仲裁记录（conflict_id, mission_alignment, quadrant_analysis, energy_analysis, proposals, user_choice）
├── briefings      — 晨间简报（content, date, created_at）
├── weekly_reviews — 周复盘（week_start, week_end, summary, energy_trends, bigrock_status）
├── llm_configs    — LLM配置（name, provider, base_url, api_key_ref, model）
├── app_settings   — 全局设置（key TEXT PRIMARY KEY, value TEXT, updated_at TEXT）

对话日志库 (conversations.db):
├── conversations  — 对话会话（role_id, title, started_at, updated_at）
├── messages       — 消息（conversation_id, role[user/assistant], content, thinking, is_complete, created_at）
```

### Agent Engine Integration (opencode Sidecar)

**架构决策：opencode作为Sidecar进程**

```
┌────────────────────────────────────────────────────────────────────┐
│                    EgoSync Desktop (Tauri)                          │
├────────────────────────────────────────────────────────────────────┤
│  React UI ←── Tauri IPC ──→ Rust Backend (编排层)                  │
│                                    │                               │
│                          ┌─────────▼──────────┐                    │
│                          │  AgentBridge       │                    │
│                          │  (HTTP Client)     │                    │
│                          └─────────┬──────────┘                    │
│                                    │ HTTP API (localhost:4096)      │
│                          ┌─────────▼──────────┐                    │
│                          │  opencode server   │                    │
│                          │  (sidecar process) │                    │
│                          │  ├─ Butler Agent   │                    │
│                          │  ├─ Role Agents    │                    │
│                          │  ├─ Tools (20+)    │                    │
│                          │  ├─ Skills         │                    │
│                          │  └─ MCP Servers    │                    │
│                          └────────────────────┘                    │
└────────────────────────────────────────────────────────────────────┘
```

**进程生命周期管理：**
- Tauri应用启动 → Rust后端spawn opencode server子进程
- 健康检查：定时HTTP ping（`GET /`），失败时自动重启
- 应用退出 → 发送SIGTERM → 等待优雅退出 → 超时SIGKILL
- opencode binary位置：Tauri sidecar目录（`resources/opencode` 或 `resources/opencode.exe`）

**通信协议（AgentBridge → opencode HTTP API）：**
```rust
// services/agent_bridge.rs — 核心接口
pub struct AgentBridge {
    base_url: String,        // http://127.0.0.1:4096
    project_id: String,      // opencode project ID
    http_client: reqwest::Client,
}

impl AgentBridge {
    // Session管理
    pub async fn create_session(&self, agent: &str, directory: &str) -> Result<SessionInfo>;
    pub async fn send_message(&self, session_id: &str, content: &str) -> Result<MessageStream>;
    pub async fn abort_session(&self, session_id: &str) -> Result<()>;
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<Message>>;
    pub async fn compact_session(&self, session_id: &str) -> Result<()>;

    // 配置管理
    pub async fn get_config(&self) -> Result<OpencodeConfig>;
    pub async fn get_providers(&self) -> Result<Vec<Provider>>;
    pub async fn get_agents(&self) -> Result<Vec<AgentInfo>>;
}
```

**角色→Agent映射策略：**
- EgoSync角色创建 → 写入 `opencode.json` 的 `agent` 配置段
- 每个角色映射为一个opencode agent（mode: "subagent"）
- 管家映射为primary agent（mode: "primary"）
- 配置变更通过修改 `opencode.json` 实现，opencode热加载

**opencode.json 配置示例（由Rust后端动态管理）：**
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "provider": {
    // 用户通过EgoSync UI配置的Provider
  },
  "agent": {
    "butler": {
      "name": "管家",
      "mode": "primary",
      "prompt": "你是EgoSync管家...",
      "permission": { "*": "allow" }
    },
    "role-product-manager": {
      "name": "产品经理",
      "mode": "subagent",
      "description": "产品经理角色Agent",
      "prompt": "你是用户的产品经理分身...",
      "permission": {
        "*": "allow",
        "bash": "ask"
      }
    }
  }
}
```

**权限模型映射（EgoSync UI → opencode permission）：**
| EgoSync UI设置 | opencode permission值 | 行为 |
|---------------|---------------------|------|
| 自主执行 | `"allow"` | Agent直接执行，无需确认 |
| 需确认 | `"ask"` | 触发时暂停，通过Rust→前端弹确认 |
| 禁止 | `"deny"` | Agent不会尝试调用 |

**opencode内置工具复用（Agent可用工具集）：**
- `read` — 读取文件
- `write` — 写入文件
- `edit` — 编辑文件（搜索替换）
- `bash` — 执行shell命令
- `grep` — 搜索代码
- `glob` — 文件模式匹配
- `websearch` — Web搜索
- `webfetch` — 网页抓取
- `skill` — 加载SKILL.md
- `task` — 子Agent委派
- `todo` — 待办管理
- 自定义EgoSync工具（通过opencode custom tool机制注册）

**Skill体系（SKILL.md格式）：**
- 位置：`.opencode/skills/<name>/SKILL.md`（项目级）或 `~/.config/opencode/skills/`（全局）
- EgoSync内置Skills存放于sidecar资源目录，启动时复制到opencode skills路径
- 角色可通过配置绑定特定skills

**流式响应桥接：**
```
opencode SSE stream → AgentBridge (Rust) → Tauri Event → 前端渲染
```
- opencode的消息流通过HTTP SSE返回
- Rust AgentBridge解析SSE，转发为Tauri Event（`llm:stream`）
- 前端监听逻辑不变（useTauriEvent）

### Authentication & Security

**API Key存储：系统钥匙串 (keyring crate)**
- 理由：跨平台支持（Windows Credential Manager / macOS Keychain / Linux Secret Service）；API Key不落明文文件
- 版本：keyring 3.x
- 影响：LLM配置模块、设置UI

**DB加密：V1不加密，依赖OS权限**
- 理由：本地应用，用户设备用户自管；加密增加复杂度和性能开销
- V2可选：SQLCipher

### API & Communication Patterns

**IPC命令组织：按域分模块**
- 模式：`chat::send_message`, `role::create`, `role::update`, `memory::list`, `task::create`
- 理由：直觉清晰，与前端 service 层一一对应
- 影响：前端 services/ 目录结构、Rust commands/ 目录结构

**错误处理：类型化错误枚举**
```rust
pub enum AppError {
    NotFound(String),
    LlmError(String),
    DbError(String),
    ValidationError(String),
    KeyringError(String),
}
```
- 序列化为 JSON，前端可针对性处理

**状态同步：事件驱动增量更新**
- 写操作返回确认 + Tauri Event 推送变更通知
- 前端监听事件刷新受影响的组件
- 事件命名规范：`role:updated`, `task:created`, `memory:changed`, `llm:stream`

**LLM流式传输：Tauri Event系统**
- Rust端 `app.emit("llm-stream", payload)` 推送每个token
- 前端 `listen("llm-stream")` 实时渲染
- 零额外依赖，原生支持

**LLM Provider适配：由opencode统一管理**
- opencode原生支持30+ Provider（OpenAI/Anthropic/Google/DeepSeek/Groq/Azure/Bedrock/Ollama等）
- 用户通过EgoSync UI配置 → Rust后端写入opencode.json → opencode热加载
- EgoSync不再自行实现LLM HTTP调用，全部委托opencode
- API Key存储仍由EgoSync管理（keyring），启动时注入环境变量给opencode进程

**原有LlmProvider Trait保留为兼容层（降级方案）：**
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        on_token: tokio::sync::mpsc::Sender<StreamEvent>,
        options: ChatOptions,
    ) -> Result<(), LlmError>;
    async fn test_connection(&self) -> Result<(), LlmError>;
}
// 保留用于：连接测试、opencode不可用时的降级对话
```

**并发模型：每次对话独立 tokio task**
- `tokio::spawn` 处理对话，V1角色少(<10)，简单直观

**上下文管理：后端全权管理**
- 前端只发用户消息，Rust端负责注入 system prompt + 记忆 + 上下文裁剪 + token计数

### Frontend Architecture

**State Management：React useState/useEffect + Tauri Event listeners**
- 当前原型模式保持，通过自定义 hooks 封装 Tauri invoke/listen
- V2视复杂度决定是否引入 Zustand

**Component Architecture：从单文件拆分为域模块**
- 现有 App.tsx 1458行拆分为 components/, hooks/, services/, types/
- 拆分原则：每个Modal/View独立文件，共享组件提取

**前端 Service 层（封装 Tauri invoke）：**
```typescript
// services/chat.ts
export const chatService = {
  sendMessage: (roleId: string, content: string) => invoke('chat_send_message', { roleId, content }),
  getHistory: (roleId: string) => invoke('chat_get_history', { roleId }),
};
// services/role.ts
export const roleService = {
  list: () => invoke('role_list'),
  create: (data: CreateRoleInput) => invoke('role_create', { data }),
  update: (id: string, data: UpdateRoleInput) => invoke('role_update', { id, data }),
  archive: (id: string) => invoke('role_archive', { id }),
  delete: (id: string) => invoke('role_delete', { id }),
};
```

### Infrastructure & Deployment

**分发方式：Tauri bundler 生成平台安装包**
- Windows: MSI/NSIS installer
- macOS: DMG
- Linux: AppImage/deb

**CI/CD：GitHub Actions**
- 三平台并行构建
- 自动化测试（Rust tests + Vitest）
- Release artifact 自动发布

**自动更新：Tauri updater plugin（V2考虑）**
- V1手动下载更新

### Decision Impact Analysis

**Implementation Sequence:**
1. Tauri 项目初始化 + SQLite schema 迁移
2. **opencode sidecar集成（进程管理 + AgentBridge HTTP客户端）**
3. **角色→opencode Agent映射（opencode.json动态管理）**
4. Agent Loop对话核心回路（通过AgentBridge → opencode session/message API）
5. 前端 IPC service 层 + App.tsx 组件拆分 + mock→real 替换
6. 记忆提炼管线
7. 工作循环调度器 + 建议系统
8. 通知系统 + 仲裁引擎
9. 晨间简报 + 周复盘生成器

**Cross-Component Dependencies:**
- **opencode sidecar → 被 AgentBridge、所有Agent对话、Skill加载 依赖（最高优先级）**
- AgentBridge → 被 Agent引擎、记忆提炼（对话触发）共用
- SQLite schema → 被所有后端模块依赖（EgoSync自身数据）
- Tauri Event → 被 LLM流式（桥接自opencode SSE）、状态同步、通知 三者共用
- 前端 service 层 → 所有 UI 组件的数据来源

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:** 6 个领域（命名、结构、格式、通信、流程、强制规则）需要统一规范以防止 AI Agent 实现冲突

### Naming Patterns

**Database Naming Conventions:**
- 表名：snake_case 复数 — `roles`, `memories`, `tasks`, `suggestions`
- 列名：snake_case — `role_id`, `created_at`, `content_json`
- 外键：`{被引用表单数}_id` — `role_id`, `conversation_id`
- 索引：`idx_{table}_{column}` — `idx_memories_role_id`
- 迁移文件：`{seq}_{description}.sql` — `001_initial_schema.sql`

**Rust Backend Naming Conventions:**
- 模块/文件：snake_case — `llm_provider.rs`, `chat_commands.rs`
- 结构体/枚举：PascalCase — `RoleModel`, `AppError`, `StreamEvent`
- 函数/方法：snake_case — `send_message()`, `get_by_role_id()`
- Tauri Command：snake_case 带域前缀 — `chat_send_message`, `role_create`
- 常量：SCREAMING_SNAKE_CASE — `DEFAULT_WORK_LOOP_INTERVAL`
- Trait：PascalCase — `LlmProvider`, `MemoryExtractor`

**TypeScript Frontend Naming Conventions:**
- 组件文件：PascalCase.tsx — `ButlerView.tsx`, `RoleCard.tsx`
- Hook文件：camelCase.ts + use前缀 — `useTauriEvent.ts`, `useRoleList.ts`
- Service文件：camelCase.ts — `chatService.ts`, `roleService.ts`
- Type文件：camelCase.ts — `role.ts`, `chat.ts`
- 接口/类型：PascalCase — `Role`, `ChatMessage`, `CreateRoleInput`
- 函数/变量：camelCase — `sendMessage`, `roleId`
- 常量：SCREAMING_SNAKE_CASE — `ROLES`, `COLOR_OPTIONS`
- CSS类：Tailwind utility（不写自定义class）

**Tauri Event Naming Conventions:**
- 格式：`{domain}:{verb_past}` — `role:created`, `role:updated`, `role:deleted`
- 角色提议：`role:proposed` — payload: `{ name, icon, color, goal }`（引导中 LLM 提议角色，前端弹出确认 modal）
- LLM流：`llm:stream` — payload: `{ roleId, token, done }`
- LLM完成：`llm:complete` — payload: `{ roleId, fullContent }`
- 通知：`notification:new` — payload: `{ id, roleId, level, content }`
- 建议：`suggestion:new` — payload: `{ id, roleId, content }`

### Structure Patterns

**Frontend File Organization (按域 + 按类型混合):**

```
src/
├── components/
│   ├── layout/          # Sidebar, Modal, Header
│   ├── butler/          # ButlerView, ButlerWorkspacePanel, MorningBriefing
│   ├── role/            # RoleView, RoleCard, RoleSettings
│   ├── chat/            # ChatFlow, ChatBubble, ChatInput
│   ├── modals/          # ArbitrationModal, WeeklyReviewModal, TaskModal
│   ├── notifications/   # NotificationPanel
│   └── settings/        # GlobalSettingsModal
├── hooks/
│   ├── useTauriCommand.ts   # 通用 invoke 封装（含 loading/error）
│   ├── useTauriEvent.ts     # 通用 event listener 封装
│   ├── useRoles.ts
│   ├── useChat.ts
│   └── useNotifications.ts
├── services/                # 纯函数，封装 Tauri invoke
│   ├── chatService.ts
│   ├── roleService.ts
│   ├── memoryService.ts
│   ├── taskService.ts
│   └── settingsService.ts
├── types/
│   ├── role.ts
│   ├── chat.ts
│   ├── memory.ts
│   ├── task.ts
│   └── settings.ts
├── utils/
│   └── cn.ts
├── App.tsx
├── main.tsx
└── index.css
```

**Rust Backend Organization:**

```
src-tauri/src/
├── main.rs                  # Tauri setup, 注册 commands
├── commands/                # IPC command handlers（薄层）
│   ├── mod.rs
│   ├── chat.rs
│   ├── role.rs
│   ├── memory.rs
│   ├── task.rs
│   └── settings.rs
├── services/                # 业务逻辑层
│   ├── mod.rs
│   ├── agent_engine.rs
│   ├── agent_bridge.rs      # opencode HTTP API 客户端
│   ├── sidecar.rs           # opencode 进程管理
│   ├── agent_config.rs      # 角色→opencode Agent映射
│   ├── memory_pipeline.rs
│   ├── arbitration.rs
│   ├── scheduler.rs
│   └── briefing.rs
├── llm/                     # LLM Provider 兼容层（降级方案）
│   ├── mod.rs
│   ├── traits.rs
│   ├── openai.rs
│   └── anthropic.rs
├── db/                      # 数据访问层
│   ├── mod.rs
│   ├── pool.rs
│   ├── roles.rs
│   ├── memories.rs
│   ├── tasks.rs
│   ├── conversations.rs
│   └── settings.rs
├── models/                  # 数据模型
│   ├── mod.rs
│   ├── role.rs
│   ├── memory.rs
│   ├── task.rs
│   ├── chat.rs
│   └── settings.rs
├── error.rs                 # AppError 定义
└── state.rs                 # Tauri managed state
```

**Test Location:**
- Rust 单元测试：同文件底部 `#[cfg(test)] mod tests`
- Rust 集成测试：`src-tauri/tests/test_{domain}.rs`
- 前端组件测试：同目录 co-located `{Component}.test.tsx`
- 前端 service 测试：同目录 co-located `{service}.test.ts`

### Format Patterns

**IPC Response Format:**
- 成功：直接返回数据（Tauri `Result<T, E>` 自动处理）
- 错误：类型化枚举序列化为 JSON，如 `{ "NotFound": "Role with id xxx not found" }`

**Data Exchange Formats:**
- JSON字段命名：前端 camelCase ↔ Rust/DB snake_case，serde `#[serde(rename_all = "camelCase")]` 自动转换
- 日期格式：ISO 8601 字符串 `2026-05-20T10:00:00Z`
- ID格式：UUID v4 字符串
- 布尔值：`true/false`
- 空值：`Option<T>` → JSON `null`，前端 `T | null`
- 枚举值：小写字符串 — `"pending"`, `"accepted"`, `"whisper"`

**Serde Standard Annotation:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleModel {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub energy: i32,
    pub status: RoleStatus,
    pub settings_json: Option<serde_json::Value>,
    pub created_at: String,
}
```

### Communication Patterns

**Event System Patterns:**
- 命名：`{domain}:{verb_past}` — 如 `role:created`, `role:proposed`, `task:updated`
- Payload结构：始终包含 `id` 字段 + 变更相关数据（`role:proposed` 含 `name/icon/color/goal`）
- 流式事件：`llm:stream` payload 含 `{ roleId, token, done }` 三字段
- 角色提议事件：`role:proposed` — 引导中 LLM Function Calling 触发，前端弹 `RoleConfirmModal`
- 前端监听：统一通过 `useTauriEvent` hook 注册/清理

**State Update Patterns:**
- 不可变更新：`setState(prev => [...prev, newItem])` 或 `setState(prev => prev.map(...))`
- 事件驱动：写操作成功后，监听 Tauri Event 触发局部状态刷新
- 禁止直接 mutation

### Process Patterns

**Error Handling:**
- Rust：所有函数返回 `Result<T, AppError>`，Command 层禁止 `.unwrap()`
- 前端：每个 service 调用用 try-catch，Hook 层统一管理 loading/error
- 用户可见错误：友好中文提示，不暴露技术细节
- 日志错误：`tracing` crate，包含完整上下文

**Loading State Patterns:**
- 命名：`isLoading`, `isSubmitting`, `isSending`, `isStreaming`
- 粒度：组件级（非全局）
- 流式对话：`isStreaming` 布尔 + `streamContent` 累积字符串
- 骨架屏：仅首次加载使用；后续增量更新无骨架

**LLM Streaming Standard Pattern (Frontend):**
```typescript
const [isStreaming, setIsStreaming] = useState(false);
const [streamContent, setStreamContent] = useState('');

useEffect(() => {
  const unlisten = listen<StreamPayload>('llm:stream', (event) => {
    if (event.payload.done) {
      setIsStreaming(false);
    } else {
      setStreamContent(prev => prev + event.payload.token);
    }
  });
  return () => { unlisten.then(fn => fn()); };
}, []);
```

### Enforcement Guidelines

**All AI Agents MUST:**
1. 使用上述命名规则，无例外
2. 新增 Tauri Command 必须同时创建前端 service 函数 + TypeScript 类型
3. 新增数据库表/列必须通过 SQLx migration 文件，禁止手动改DB
4. Rust struct 必须 derive `Serialize, Deserialize` 并使用 `#[serde(rename_all = "camelCase")]`
5. 前端组件必须放在对应域文件夹，禁止在 App.tsx 中新增组件
6. 错误永远用 `Result`/try-catch 处理，禁止 unwrap/panic/ignore

**Anti-Patterns (禁止):**
- ❌ 在前端直接操作 SQLite（必须走 Tauri IPC）
- ❌ 硬编码 API Key 到源码
- ❌ 在 Rust Command 层写业务逻辑（Command 只做参数解析 → 调用 Service → 返回结果）
- ❌ 使用 `.unwrap()` 处理可能失败的操作
- ❌ 混用 camelCase 和 snake_case（遵守各层约定）
- ❌ 在前端写自定义 CSS class（使用 Tailwind utility）

## Project Structure & Boundaries

### Complete Project Directory Structure

```
EgoSync/探索/
├── README.md
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml                    # Test + Tauri Build + Artifacts (三平台)
│       └── release.yml               # 构建安装包 + 发布
│
├── egosync-app/                              # 前端 + Tauri 桌面壳
│   ├── package.json
│   ├── package-lock.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── tsconfig.node.json
│   ├── tailwind.config.js
│   ├── postcss.config.js
│   ├── index.html
│   │
│   ├── src/                          # React 前端源码
│   │   ├── main.tsx
│   │   ├── App.tsx                   # 顶层路由/场景管理
│   │   ├── index.css
│   │   │
│   │   ├── components/
│   │   │   ├── layout/
│   │   │   │   ├── Sidebar.tsx
│   │   │   │   ├── MainLayout.tsx
│   │   │   │   └── Header.tsx
│   │   │   ├── butler/
│   │   │   │   ├── ButlerView.tsx
│   │   │   │   ├── ButlerWorkspacePanel.tsx
│   │   │   │   ├── MorningBriefing.tsx
│   │   │   │   └── ButlerChat.tsx
│   │   │   ├── role/
│   │   │   │   ├── RoleView.tsx
│   │   │   │   ├── RoleCard.tsx
│   │   │   │   ├── RoleSettings.tsx
│   │   │   │   └── RoleEnergyBar.tsx
│   │   │   ├── chat/
│   │   │   │   ├── ChatFlow.tsx
│   │   │   │   ├── ChatBubble.tsx
│   │   │   │   ├── ChatInput.tsx
│   │   │   │   └── StreamingText.tsx
│   │   │   ├── tasks/
│   │   │   │   ├── TaskList.tsx
│   │   │   │   ├── TaskCard.tsx
│   │   │   │   └── QuadrantView.tsx
│   │   │   ├── modals/
│   │   │   │   ├── ArbitrationModal.tsx
│   │   │   │   ├── WeeklyReviewModal.tsx
│   │   │   │   ├── TaskModal.tsx
│   │   │   │   ├── AddRoleModal.tsx
│   │   │   │   └── MissionModal.tsx
│   │   │   ├── notifications/
│   │   │   │   ├── NotificationPanel.tsx
│   │   │   │   └── NotificationItem.tsx
│   │   │   ├── onboarding/
│   │   │   │   ├── OnboardingView.tsx
│   │   │   │   ├── RoleConfirmModal.tsx
│   │   │   │   └── OnboardingStep.tsx
│   │   │   └── settings/
│   │   │       ├── SettingsModal.tsx
│   │   │       └── LlmConfigPanel.tsx
│   │   │
│   │   ├── hooks/
│   │   │   ├── useTauriCommand.ts
│   │   │   ├── useTauriEvent.ts
│   │   │   ├── useRoles.ts
│   │   │   ├── useChat.ts
│   │   │   ├── useTasks.ts
│   │   │   ├── useNotifications.ts
│   │   │   └── useSettings.ts
│   │   │
│   │   ├── services/
│   │   │   ├── chatService.ts
│   │   │   ├── roleService.ts
│   │   │   ├── appService.ts
│   │   │   ├── memoryService.ts
│   │   │   ├── taskService.ts
│   │   │   ├── notificationService.ts
│   │   │   └── settingsService.ts
│   │   │
│   │   ├── types/
│   │   │   ├── role.ts
│   │   │   ├── chat.ts
│   │   │   ├── memory.ts
│   │   │   ├── task.ts
│   │   │   ├── notification.ts
│   │   │   ├── settings.ts
│   │   │   └── common.ts
│   │   │
│   │   └── utils/
│   │       ├── cn.ts
│   │       └── format.ts
│   │
│   │   ├── lib/
│   │   │   └── roleIcons.ts            # Lucide icon whitelist + color whitelist
│   │
│   └── src-tauri/                      # Rust 后端
│       ├── Cargo.toml
│       ├── Cargo.lock
│       ├── tauri.conf.json
│       ├── build.rs
│       ├── icons/
│       │
│       ├── migrations/
│       │   ├── 001_initial_schema.sql
│       │   ├── 002_conversations.sql
│       │   └── ...
│       │
│       ├── src/
│       │   ├── main.rs
│       │   ├── state.rs
│       │   ├── error.rs
│       │   │
│       │   ├── commands/
│       │   │   ├── mod.rs
│       │   │   ├── chat.rs
│       │   │   ├── role.rs
│       │   │   ├── app.rs
│       │   │   ├── memory.rs
│       │   │   ├── task.rs
│       │   │   ├── notification.rs
│       │   │   └── settings.rs
│       │   │
│       │   ├── services/
│       │   │   ├── mod.rs
│       │   │   ├── agent_engine.rs
│       │   │   ├── agent_bridge.rs      # opencode HTTP API 客户端
│       │   │   ├── sidecar.rs           # opencode 进程生命周期
│       │   │   ├── agent_config.rs      # 角色→opencode Agent映射
│       │   │   ├── memory_pipeline.rs
│       │   │   ├── arbitration.rs
│       │   │   ├── scheduler.rs
│       │   │   ├── briefing.rs
│       │   │   └── suggestion.rs
│       │   │
│       │   ├── llm/                         # LLM Provider 兼容层
│       │   │   ├── mod.rs
│       │   │   ├── traits.rs
│       │   │   ├── openai.rs
│       │   │   ├── anthropic.rs
│       │   │   └── context.rs
│       │   │
│       │   ├── db/
│       │   │   ├── mod.rs
│       │   │   ├── pool.rs
│       │   │   ├── roles.rs
│       │   │   ├── memories.rs
│       │   │   ├── tasks.rs
│       │   │   ├── conversations.rs
│       │   │   ├── app_settings.rs
│       │   │   ├── suggestions.rs
│       │   │   ├── notifications.rs
│       │   │   └── settings.rs
│       │   │
│       │   └── models/
│       │       ├── mod.rs
│       │       ├── role.rs
│       │       ├── memory.rs
│       │       ├── task.rs
│       │       ├── chat.rs
│       │       ├── notification.rs
│       │       └── settings.rs
│       │
│       └── tests/
│           ├── test_chat.rs
│           ├── test_roles.rs
│           ├── test_llm.rs
│           └── common/
│               └── mod.rs
│
├── docs/
│   ├── api-reference.md
│   └── database-schema.md
│
└── _bmad-output/
    └── planning-artifacts/
        ├── architecture.md
        ├── prd-egosync.md
        └── ux-design-specification.md
```

### Architectural Boundaries

**IPC Boundary (前端 ↔ Rust ↔ opencode):**
```
┌─────────────────┐     Tauri invoke()      ┌─────────────────┐
│   React 前端    │ ─────────────────────→  │   commands/     │
│   services/     │                          │   (薄层解析)    │
│                 │ ←─────────────────────   │                 │
│   hooks/        │     Result<T, E>         └───────┬─────────┘
│   (listen)      │                                  │
│                 │ ←── Tauri Event ──────  services/ │ (编排层)
└─────────────────┘     (llm:stream,                 │
                         role:updated...)    ┌───────┴─────────┐
                                             │  agent_bridge   │
                                             │  + sidecar      │
                                             └───────┬─────────┘
                                                     │ HTTP API
                                             ┌───────▼─────────┐
                                             │  opencode       │
                                             │  server         │
                                             │  (Agent Loop +  │
                                             │   Tools + LLM)  │
                                             └───────┬─────────┘
                                                     │
                                             ┌───────▼─────────┐
                                             │  db/ (EgoSync)  │
                                             │  + opencode DB  │
                                             └─────────────────┘
```

**Layer Rules:**
- 前端 **永远不** 直接访问 DB、LLM API 或 opencode server
- commands/ **只做** 参数校验 + 调 services + 返回结果
- services/ **拥有** 所有业务逻辑，通过 agent_bridge 调 opencode，通过 db/ 读写EgoSync数据
- agent_bridge/ **只做** HTTP请求封装和SSE流解析，不含业务判断
- sidecar/ **只管** opencode进程生命周期，不含业务逻辑
- db/ **只做** SQL 执行（EgoSync自身数据），不含业务判断
- opencode server **全权管理** LLM调用、工具执行、session/message持久化

**Data Boundaries:**
| 数据库 | 所属 | 访问规则 |
|--------|------|----------|
| `egosync.db` | EgoSync主数据（角色/记忆/任务） | 所有 services 可读写；前端通过 IPC 间接访问 |
| `conversations.db` | EgoSync对话日志 | 仅 `db/conversations.rs` 访问；其他模块通过 service 层接口读取 |
| opencode SQLite | opencode session/message | 由opencode server独立管理；EgoSync通过AgentBridge HTTP API访问 |

**Component Communication:**
| 通信方向 | 机制 | 示例 |
|----------|------|------|
| 前端 → Rust | `invoke()` | 发送消息、创建角色 |
| Rust → 前端 | `app.emit()` | 流式 token、状态变更通知 |
| Services 间 | 直接函数调用 | agent_engine 调 llm/traits |
| 定时触发 | `tokio::interval` → service | scheduler 触发 work loop |

### Requirements to Structure Mapping

| FR域 | 前端组件 | Rust服务 | DB表 |
|------|----------|----------|------|
| FR-1~3 管家对话路由 | `butler/`, `chat/` | `agent_engine.rs` | `conversations`, `messages` |
| FR-4~6 角色管理 | `role/`, `modals/AddRoleModal` | `commands/role.rs` | `roles` |
| FR-7~9 结构化记忆 | — (后端自动) | `memory_pipeline.rs` | `memories` |
| FR-10~12 工作循环 | `notifications/` | `scheduler.rs`, `suggestion.rs` | `suggestions` |
| FR-13~15 仲裁 | `modals/ArbitrationModal` | `arbitration.rs` | `mission`, `tasks` |
| FR-16~18 简报复盘 | `butler/MorningBriefing`, `modals/WeeklyReviewModal` | `briefing.rs` | `tasks`, `memories` |
| FR-19~21 仪表盘 | `role/RoleCard`, `layout/Sidebar` | `commands/role.rs` | `roles`, `tasks` |
| FR-22 通知 | `notifications/` | `commands/notification.rs` | `notifications` |
| FR-23~24 四象限 | `tasks/QuadrantView` | `commands/task.rs` | `tasks` |
| FR-25~27 数据主权 | `settings/` | `commands/settings.rs` | 全部（导出/销毁） |
| FR-28 LLM配置 | `settings/LlmConfigPanel` | `commands/settings.rs`, `agent_config.rs` | `llm_configs` + opencode.json |
| FR-31~36 Agent引擎 | — (透明) | `sidecar.rs`, `agent_bridge.rs`, `agent_config.rs` | opencode DB (session/message) |

### Integration Points

**Internal Data Flow (对话核心回路 — 通过opencode Agent Loop):**
```
用户输入 → ChatInput → invoke('chat_send_message')
    → commands/chat.rs → services/agent_engine.rs
    → agent_bridge.rs::send_message(session_id, content)
    → [HTTP POST] opencode /project/:id/session/:sid/message
    → opencode Agent Loop（自主决策工具调用、多步执行）
    → [SSE stream] → agent_bridge 解析 → app.emit("llm:stream", {token, thinking, tool_call, done})
    → 前端 useTauriEvent → StreamingText 实时渲染
    → [用户点停止] → invoke('chat_stop_streaming') → agent_bridge::abort_session()
    → [完成后] → memory_pipeline.rs 提炼记忆（从opencode messages提取）
    → db/memories.rs 持久化到EgoSync DB
    → app.emit("memory:changed")
```

**External Integrations:**
- opencode server (sidecar进程) — 通过 `agent_bridge/` HTTP API通信
- LLM API (OpenAI/Anthropic/Google/等) — 由opencode统一管理，EgoSync不直接调用
- MCP Servers — 由opencode管理，角色可配置
- 系统钥匙串 (keyring) — 通过 `services/` 层读写 API Key，启动时注入opencode环境变量

### Development Workflow

**Development Server:**
```bash
cd GUI
npm run tauri dev
# 同时启动 Vite HMR (port 5173) + Rust 增量编译 + Tauri 窗口
```

**Build Process:**
```bash
cd GUI
npm run tauri build
# Vite 生产构建 → Rust release 编译 → Tauri bundler 打包
# 输出: src-tauri/target/release/bundle/{msi,dmg,appimage}
```

**Test Execution:**
```bash
# 前端测试
cd GUI && npx vitest

# Rust 测试
cd egosync-app/src-tauri && cargo test

# 全量
npm run test:all   # package.json script 组合以上两者
```

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
- Tauri 2.x + React 18 + Vite 5：官方支持的组合，无兼容冲突
- SQLx 0.8 + SQLite + tokio：异步运行时一致
- keyring 3.x：跨三平台，与 Tauri 部署目标对齐
- serde camelCase ↔ SQLite snake_case：通过 `#[serde(rename_all)]` 自动桥接

**Pattern Consistency:**
- 命名规则在 Rust/TS/DB 三层分别明确，无交叉模糊区
- Tauri Event 命名 (`domain:verb_past`) 与前端 hook (`useTauriEvent`) 模式配合
- 错误处理链路完整：Rust `Result<T, AppError>` → JSON → 前端 try-catch

**Structure Alignment:**
- 前端 services/ 与 Rust commands/ 一一对应
- 每个 FR 域都有明确的前端组件 + 后端服务 + DB 表映射

### Requirements Coverage Validation ✅

**Functional Requirements Coverage:**
| FR域 | 架构覆盖 | 验证 |
|------|----------|------|
| FR-1~3 管家对话路由 | agent_engine + llm/ + chat commands | ✅ |
| FR-4~6 角色管理 | role commands + roles DB + CRUD | ✅ |
| FR-7~9 结构化记忆 | memory_pipeline + memories DB + JSON混合 | ✅ |
| FR-10~12 工作循环 | scheduler + suggestion + tokio interval | ✅ |
| FR-13~15 仲裁 | arbitration service + mission DB | ✅ |
| FR-16~18 简报复盘 | briefing service + 定时触发 | ✅ |
| FR-19~21 仪表盘 | role commands + 前端状态聚合 | ✅ |
| FR-22 通知 | notification commands + 三级枚举 | ✅ |
| FR-23~24 四象限 | task commands + quadrant 字段 | ✅ |
| FR-25~27 数据主权 | settings commands + 本地SQLite + 导出/销毁 | ✅ |
| FR-28 LLM配置 | agent_config + opencode.json + keyring | ✅ |
| FR-29~30 透明审计 | agent_engine 上下文组装可溯源 | ✅ |
| FR-31~36 Agent引擎集成 | sidecar + agent_bridge + agent_config + opencode | ✅ |

**Non-Functional Requirements Coverage:**
- ✅ 本地优先：SQLite + 无云端依赖 + opencode本地运行
- ✅ BYOK：keyring + opencode多Provider原生支持
- ✅ Agent能力：opencode Agent Loop + 20+内置工具 + Skill + MCP
- ✅ 性能：Tauri Event流式（桥接opencode SSE）+ 增量状态更新 + 60fps(Tailwind动效)
- ✅ 数据隐私：本地存储 + 不加密(V1, OS权限)
- ✅ 跨平台：Tauri bundler 三平台 + opencode sidecar三平台binary

### Implementation Readiness Validation ✅

**Decision Completeness:**
- 所有 Critical 和 Important 决策均已文档化，含版本号和理由
- 实现模式含代码示例（Rust trait, TS service, 流式处理）

**Structure Completeness:**
- 完整目录树（~80+ 文件），每个组件有明确位置
- 集成点通过数据流图明确指定

**Pattern Completeness:**
- 6 个模式类别覆盖所有潜在冲突点
- 含反模式列表和强制执行规则

### Gap Analysis Results

**Critical Gaps:** 无

**Important Gaps (不阻塞实现):**
1. opencode sidecar binary的构建/打包流程 — 需确定如何从opencode源码编译三平台binary并集成到Tauri资源
2. opencode SSE → Tauri Event桥接的具体解析逻辑 — opencode message/part格式需实现时适配
3. Token 计数策略未细化 — opencode自身管理上下文压缩（compaction），EgoSync需决定何时触发
4. 记忆提炼 prompt 未定义 — memory_pipeline 的具体 prompt 需实现时设计
5. System Prompt 分层模板内容 — 角色system prompt如何注入opencode agent config

**Nice-to-Have Gaps:**
- 性能基准测试定义
- opencode进程异常时的降级策略具体参数

**V1 必需（已从 Nice-to-Have 提升）:**
- E2E 测试：使用 Tauri driver (WebDriver 协议)，覆盖核心用户旅程（冷启动引导、管家对话、角色 CRUD、LLM 流式响应、任务管理），确保应用启动→基本交互→数据持久化的端到端可用性

### Validation Issues Addressed

未发现阻塞性问题。上述 Important Gaps 属于实现细节，在开发对应 Story 时自然解决。

### Architecture Completeness Checklist

**Requirements Analysis**
- [x] 项目上下文深度分析
- [x] 规模与复杂度评估
- [x] 技术约束识别
- [x] 跨切关注点映射

**Architectural Decisions**
- [x] 关键决策含版本号文档化
- [x] 技术栈完整指定
- [x] 集成模式定义
- [x] 性能考量已处理

**Implementation Patterns**
- [x] 命名规范建立
- [x] 结构模式定义
- [x] 通信模式指定
- [x] 流程模式文档化

**Project Structure**
- [x] 完整目录结构定义
- [x] 组件边界建立
- [x] 集成点映射
- [x] 需求到结构映射完成

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION ✅

**Confidence Level:** High

**Key Strengths:**
- 前端已有高保真原型，只需接 API
- opencode提供完整Agent执行引擎，EgoSync专注编排层和领域逻辑
- 三层架构职责清晰：UI(React) → 编排(Rust) → 执行(opencode)
- opencode原生支持30+ LLM Provider，无需自行实现适配层
- 完整 FR 映射，无遗漏
- 丰富代码示例和反模式，AI Agent 可直接参照

**Areas for Future Enhancement:**
- V2: DB 加密 (SQLCipher)
- V2: 前端状态管理 (Zustand)
- V2: 自动更新 (Tauri updater)
- V2: Actor 模型（角色数量增长后）

### Implementation Handoff

**AI Agent Guidelines:**
- 严格遵循本文档的所有架构决策
- 使用实现模式一致性规则，无例外
- 尊重项目结构和边界定义
- 任何架构疑问参考本文档

**First Implementation Priority:**
```bash
# Story 1: Tauri 项目初始化
cd GUI
npm install -D @tauri-apps/cli@latest @tauri-apps/api@latest
npx tauri init
# 配置 Cargo.toml 依赖: tauri, serde, sqlx, tokio, uuid, keyring, reqwest
# 创建 migrations/001_initial_schema.sql
# 运行 npm run tauri dev 验证基础连通

# Story 2: opencode Sidecar集成 (NEW - 最高优先级)
# 将 opencode binary 放入 src-tauri/resources/
# 实现 sidecar.rs (spawn/kill/health check)
# 实现 agent_bridge.rs (HTTP client → opencode API)
# 验证: 应用启动后opencode server可访问，退出后进程清理
```

---

## Incremental Project Context Analysis — FR-37～FR-39

### Requirements Overview

本次增量在既有 `React → Tauri/Rust → opencode` 三层架构上补充三个横切能力，而不是重新设计 Skill、Dashboard 或 MCP 子系统。

**FR-37：对话级 Skill 选择**

- 管家与每个角色已经分别维护 Skill 配置和启用状态。
- 当前 Agent 的可用 Skill 集合只能包含其已添加且已启用的 Skill，不能继承或合并其他 Agent 的配置。
- `@Skill` 是请求级选择，不得修改长期 Skill 配置。
- 未指定 `@Skill` 时，保留现有 Agent 自动发现和按需加载行为。
- 不存在、未添加或未启用的 Skill 必须在任务进入 Agent Runtime 前被拒绝。

**FR-38：仪表盘聚合统计**

- 统一返回任务总数、结构化记忆条目数、对话会话数和待处理任务数。
- 查询维度支持全部、管家、单个角色以及时间范围，两个维度可组合。
- 任务与记忆按 `created_at`；对话会话按 `started_at`。
- 对话按会话计数，不按消息数计数。
- 待处理任务先按 `created_at` 纳入查询范围，再按查询时未完成状态计数，并满足 `pendingTaskCount <= taskCount`。

**FR-39：MCP Server 运行时管理**

- MCP Server 的 `enabled` 状态与角色绑定是两个独立配置维度。
- 切换 Server 启用状态不得修改角色绑定；角色绑定变化不得修改 Server 状态。
- 关闭的 Server 不得投影到 Agent Runtime；重新启用后恢复投影。
- 创建、编辑、删除、启用/关闭和角色绑定变化后，后续会话必须使用最新运行时配置。
- 当前代码已有 MCP 配置同步、运行时刷新入口及 `OpencodeMcpScopeLock`，增量方案应复用这些边界。

### Non-Functional Requirements

- **本地优先**：统计、Skill 配置解析和 MCP 状态管理不得依赖云服务。
- **分层通信**：React 只经 Tauri IPC；Command 只做参数解析；业务规则在 Rust Service；数据访问在 DB 层。
- **任务快照一致性**：Skill/MCP 可用集合必须在任务启动前形成快照，流式执行开始后不改变本次上下文。
- **运行时一致性**：数据库配置、`opencode.json` 配置投影和运行中的 opencode 状态必须有明确同步结果。
- **安全性**：`@Skill` 不得绕过启用校验；MCP 密钥继续使用 keyring 或环境变量引用；关闭的 Server 不得因旧配置残留继续暴露工具。
- **可审计性**：显式 Skill、统计筛选条件及 MCP 刷新结果均应可追踪。
- **显式失败**：不得在数据库更新成功、运行时刷新失败时静默宣称完整成功。

### Scale & Complexity

- Primary domain: Tauri full-stack desktop application with React/TypeScript, Rust and opencode sidecar.
- Overall complexity: High.
- Increment complexity: Medium-high due to request-time context composition, cross-domain aggregation and runtime configuration projection.
- Data complexity: Tasks and memories live in EgoSync-owned stores, while conversation sessions have a separate conversation/opencode boundary that must remain behind an adapter.
- Concurrency complexity: MCP mutations and runtime refresh require serialization; Skill configuration needs snapshot semantics between validation and task start.
- Multi-tenancy/regulatory scope: None for V1; all data remains local.

预计本次需要新增或明确以下组件边界：

- Agent Skill Resolver
- Task Context Builder
- Skill Audit Metadata
- Dashboard Query DTO
- Dashboard Aggregation Service
- Conversation Session Repository/Adapter
- MCP Configuration Projection
- MCP Runtime Refresh Coordinator
- MCP Scope Resolver
- Runtime Refresh Result

### Technical Constraints & Dependencies

1. 复用当前管家和角色的独立 Skill 配置，不建立全局继承模型。
2. Agent 身份必须由后端可信上下文解析，前端不能直接提交未经验证的可用 Skill 集合。
3. Skill 长期配置与任务级选择使用不同语义和数据结构。
4. 仪表盘必须通过统一 Tauri Command 返回聚合 DTO，前端不得直接访问 SQLite 或 opencode。
5. 对话会话统计需沿用现有会话数据边界，通过 Repository/Adapter 接入聚合服务。
6. MCP `enabled` 字段、角色绑定、CRUD、`AgentConfigService` 和运行时刷新链路均优先复用。
7. `opencode.json` 只能由 `AgentConfigService` 程序化维护。
8. MCP 变更与运行时刷新沿用现有锁和 Sidecar/Session 管理边界。
9. 不指定 `@Skill` 的路径必须保持现有自动发现行为。
10. 配置持久化与运行时刷新之间的部分失败必须有明确恢复策略。

### Cross-Cutting Concerns Identified

1. **Agent 身份解析**：Skill 和 MCP 可用集合都以当前 Agent 身份为作用域。
2. **持久化配置与任务快照分离**：长期允许集合与本次显式选择不能相互写回。
3. **上下文注入时机**：所有显式选择在 Agent Runtime 调用前验证并冻结。
4. **跨数据源统计**：Dashboard Service 屏蔽任务、记忆和会话来源差异。
5. **统一筛选语义**：四项指标必须共享同一个 Agent/时间查询快照。
6. **配置事实与运行时投影**：数据库是事实来源，`opencode.json` 是投影，sidecar 是运行状态。
7. **MCP 状态与绑定解耦**：有效工具集合是 Server 启用状态与 Agent 绑定范围的交集。
8. **并发和幂等**：连续 MCP 变更必须串行或可合并，重复刷新不能残留或复制工具。
9. **审计和可解释性**：显式 Skill、统计条件和 MCP 刷新失败均可追踪。
## Incremental Starter Template Evaluation — FR-37～FR-39

### Primary Technology Domain

本项目是现有的 Tauri 桌面端棕地应用：React 18 + TypeScript/Vite 前端、Tauri 2/Rust 后端、SQLite/SQLx 本地数据层，以及 opencode sidecar Agent Runtime。本轮是增量架构设计，不重新初始化项目。

### Technical Preferences Already Established

- React 只负责展示和交互，不直接访问数据库或 opencode。
- 所有前端业务操作通过 Tauri IPC。
- Rust Command 层只做参数解析并调用 Service；业务规则位于 Service；数据访问位于 DB/Repository。
- opencode API 统一通过 AgentBridge；`opencode.json` 统一由 AgentConfigService 程序化维护。
- IPC/JSON 使用 camelCase，Rust 与数据库使用 snake_case。
- 保持本地优先，不增加云端服务或新的后台守护进程。
- 除非现有 hooks 无法承载复杂度，否则不新增全局前端状态库。

### Starter Options Considered

1. **重新使用 create-tauri-app 创建项目：拒绝。** 当前 Tauri、React、Rust、SQLite 和 opencode 基础集成已完成，重建会引入无关迁移和回归风险。
2. **新增独立本地 HTTP 聚合服务：拒绝。** Dashboard、Skill 和 MCP 均属于现有 Rust 编排层职责，Tauri IPC 已满足通信需要。
3. **在现有 EgoSync 代码库中增量实现：选择。** 复用 Skill 配置、Dashboard Service、MCP CRUD、AgentConfigService 和 Runtime 刷新机制，仅补充必要领域对象、DTO、查询和上下文组装逻辑。

### Selected Starter

**Existing EgoSync Tauri codebase — surgical incremental extension**

本轮不执行项目初始化，不生成新应用骨架。

### Baseline Verification Commands

```powershell
cd egosync-app
npm install
npm run build

cd src-tauri
cargo check
cargo test
```

这些命令仅验证现有基线，不代表重建或依赖升级。

### Architectural Decisions Preserved

- 前端继续使用 React 18 + TypeScript/Vite。
- 后端继续使用 Rust 2021 + Tokio、Tauri 2 和 SQLite/SQLx。
- Agent Runtime 继续使用 opencode sidecar。
- 对话会话继续尊重现有独立数据边界，不为统计新增数据副本。
- 通信保持 React → Tauri IPC → Rust Service，以及 Rust → AgentBridge/AgentConfigService → opencode。
- 测试保持 Vitest/Testing Library、Cargo tests 和现有 E2E 体系。

### Dependency Decision

本次不升级 React、Tauri、Vite、SQLx 或 reqwest。Vite 升级属于独立构建系统维护事项，不与 FR-37～FR-39 捆绑。

### Consequence for Implementation

- Skill：扩展 Agent 请求上下文组装，不新建 Skill 执行引擎。
- Dashboard：扩展现有 Dashboard Command/Service/Repository，不新增本地服务。
- MCP：扩展现有 MCP mutation、AgentConfigService 和 Runtime refresh，不新增第二套同步器。
## Incremental Core Architectural Decisions — FR-37～FR-39

### FR-37：Skill 可用集合与原生任务调用

**决策：使用 opencode v1.15.10 原生 Session Command API，不建立自定义 Prompt 注入协议。**

#### 请求与可用集合

- `ChatRequest` 墅加可选字段 `selectedSkillId`；前端 `@Skill` 仅作为结构化选择交互，发送时从任务正文中移除选择标记。
- 前端只提交 EgoSync Skill Registry ID，不得直接提交 opencode command name。
- 当前 Agent 作用域由后端可信上下文解析：无 `roleId` 为管家，有 `roleId` 为指定角色。
- 管家可用集合取其 `enabledSkillIds`；角色可用集合取该角色自己的 `enabledSkillIds`。二者相互独立，不继承、不合并。
- 后端通过统一 `SkillAvailabilityService` 执行 `list_enabled(scope)` 与 `resolve_enabled(scope, skill_id)`；解析条件为 Skill 存在、已添加且在当前作用域启用。
- 任务启动前形成不可变的 Skill 选择快照；任务执行期间的配置变化不影响本次调用。

#### opencode 格式转换

现有 `SkillRegistryEntry` 已提供 `id` 与 `name`，其中 `name` 对应 Skill `SKILL.md` frontmatter name。opencode v1.15.10 会以该 name 注册原生 Command，因此无需增加映射表：

```text
selectedSkillId
  → SkillRegistryEntry.id
  → SkillRegistryEntry.name
  → opencode command
```

指定 Skill 时，新增最小 `AgentBridge::send_command(...)` 封装并调用：

```http
POST /session/{sessionId}/command
Content-Type: application/json

{
  "agent": "<resolved-agent-name>",
  "command": "<skill-frontmatter-name>",
  "arguments": "<normalized-user-content>"
}
```

未指定 Skill 时继续使用现有 `POST /session/{sessionId}/message` 路径。Command 内部进入 opencode 的标准 prompt 执行流程，因此继续复用现有全局 SSE 订阅、流式事件处理、取消、消息持久化和 completed-message fallback。

#### 安全与失败语义

- 后端校验是授权边界；不得仅依赖前端候选列表或 opencode Command 注册表。
- 不存在、未添加或未启用分别返回明确领域错误；不得调用 Runtime。
- Registry 中存在但 Runtime 找不到 Command 时返回 `SkillRuntimeOutOfSync`，记录审计信息，不静默改走普通 Prompt。
- Skill 执行错误保留 opencode 原始错误语义并进入现有消息过程事件审计。
- V1 每条消息最多显式指定一个 Skill；多 Skill 编排不在本轮范围。

#### 明确排除

- 不使用自定义 `[EGOSYNC_TASK_SKILL]` 等 Prompt 协议。
- 不向前端暴露或传输完整 Skill 内容。
- 不为单次任务临时修改 `opencode.json`。
- 不允许前端提交任意 Command 名称。
- 不在 Skill 失败时静默降级。

**职责边界：EgoSync 负责作用域、启用状态、身份校验与格式转换；opencode 负责 Skill 内容展开和原生执行。**

### FR-38：仪表盘统计聚合接口

**决策：保留现有角色状态接口，新增独立 `dashboard_get_metrics` 聚合接口。**

#### API 与 DTO

现有 `dashboard_get_status` 继续返回角色状态卡，不修改 `DashboardStatus[]` 契约。新增：

```rust
dashboard_get_metrics(
    query: DashboardMetricsQuery,
    pool: DbPool,
    conv_pool: ConversationsPool,
) -> Result<DashboardMetrics, AppError>
```

查询作用域使用显式 tagged union：`all`、`butler`、`role { roleId }`；时间范围为可空的 `{ startAt, endAt }`。`null` 表示全部时间，指定范围统一使用 RFC 3339 半开区间 `[startAt, endAt)`，且必须满足 `startAt < endAt`。前端负责根据用户本地日历边界换算绝对时间点。

返回 DTO：

```text
DashboardMetrics {
  taskCount: i64,
  memoryCount: i64,
  conversationCount: i64,
  pendingTaskCount: i64,
  generatedAt: RFC3339
}
```

#### 指标语义

| 指标 | 数据源 | 时间字段 | 条件 |
|---|---|---|---|
| `taskCount` | 主库 `tasks` | `created_at` | `deleted_at IS NULL` |
| `memoryCount` | 主库 `memories` | `created_at` | 无 |
| `conversationCount` | `ConversationsPool` | `started_at` | 按会话计数，不按消息计数 |
| `pendingTaskCount` | 主库 `tasks` | `created_at` | `deleted_at IS NULL AND is_completed = 0` |

待处理任务定义为“在筛选时间内创建、查询执行时仍未完成”，因此必须满足 `pendingTaskCount <= taskCount`，不使用 `completed_at` 回溯历史时点状态。

#### 作用域映射

- `all`：不加 Agent 条件，统计全部非删除业务记录；不依赖活跃角色列表，避免角色停用后历史数据从总数消失。
- `butler`：任务使用 `owner_type = 'butler' AND role_id IS NULL`；记忆和会话使用 `role_id IS NULL`。
- `role`：任务使用 `owner_type = 'role' AND role_id = :role_id`；记忆和会话使用相同 `role_id`。后端验证角色存在，前端默认只展示活跃角色作为筛选项。

#### 聚合与一致性

新增 `DashboardAggregationService`：主库通过单个 SQL statement 返回任务总数、记忆数和待处理任务数；会话计数通过 Conversations Repository/Adapter 查询。两部分可用 `tokio::try_join!` 并行执行，复用同一规范化查询快照。

由于主库与 ConversationsPool 是独立 SQLite 数据库，本轮承诺同一请求的近实时一致性，不承诺跨数据库事务级原子快照；不 attach 数据库、不复制会话数据、不建立统计缓存或物化视图。

#### 失败策略

新指标接口采用全有或全无：任一数据源失败则整个请求返回错误，不使用零值掩盖失败。前端首次失败显示错误；已有成功数据刷新失败时保留上一次结果并显示更新失败状态。

成功日志仅记录规范化筛选条件、生成时间与耗时；失败日志额外记录失败数据源，不记录任务、记忆或会话正文。本轮不新增查询审计表。

#### 性能策略

不预先添加猜测性索引。实现后使用实际数据和 `EXPLAIN QUERY PLAN` 验证；仅在出现证据充分的扫描瓶颈时，增加与作用域和时间字段匹配的最小组合索引。

### FR-39：管家 MCP 绑定能力

**决策：采用最小增量方案，管家仿照角色现有 MCP 处理逻辑增加独立绑定能力，角色实现保持不变。**

#### 有效集合与职责边界

```text
管家有效 MCP = 管家已绑定 Server ∩ enabled Server
角色有效 MCP = 维持现有角色实现
```

`McpServer.enabled` 继续表示 Server 启停状态；管家绑定只表示管家选择使用哪些 Server。Server 关闭时不删除既有绑定，重新启用后原绑定自动恢复有效。管家绑定与所有角色绑定相互独立。

#### 数据与接口

新增独立 `butler_mcp_servers` 关联表，不迁移现有角色绑定表、不引入通用 Agent 绑定模型，也不使用虚拟角色 ID。增加与角色现有接口对称的 Repository、Service、Tauri Command 和前端 API：

```text
list_mcp_servers_for_butler
list_mcp_servers_available_for_butler
add_mcp_server_to_butler
remove_mcp_server_from_butler
```

绑定规则沿用角色当前行为：关闭的 Server 不能新增绑定；已绑定 Server 被关闭后保留绑定，但不进入管家有效集合。

#### Agent 配置与界面

新增 `butler_enabled_mcp_lines()`，按“管家绑定且 enabled”生成管家 MCP 能力说明，并扩展 `build_butler_entry_with_skills(...)` 及其现有同步路径。角色的 `build_agent_entry_with_skills_and_mcp(...)`、`role_enabled_mcp_lines()`、角色绑定 Service 和数据库逻辑不变。

管家设置界面增加已绑定列表、可添加列表、添加和移除操作，交互遵循角色现有 MCP 设置。仅在能够保持外科手术式修改时复用现有 UI，不为本需求重构角色组件。

#### Runtime 与明确排除

管家绑定变化沿用现有 Agent 配置同步方式；MCP Server 的新增、编辑、启用和关闭继续走现有 Runtime 刷新逻辑。本轮不新增 `McpRuntimeCoordinator`、Agent Permission 投影、Runtime revision、通用 Availability Resolver 或新的并发锁。

以下现有行为明确保持不变：

- 不修改角色 MCP 授权和绑定逻辑。
- 不重构或删除 `sync_mcp_scope_for_role()`。
- 不改变 `add_to_role()` 对关闭 Server 的校验。
- 不处理按作用域改写全局 MCP 配置可能产生的角色并发覆盖问题。
- 不建立管家与角色统一的 MCP 权限框架。

Agent Permission 隔离和 MCP Runtime/角色绑定解耦不纳入本轮实现，作为后续技术债候选项，避免与当前角色范式并存形成第二套机制。

#### 验收约束

- 管家可以查看、添加和移除自己的 MCP 绑定。
- 管家只能获得已绑定且 enabled 的 MCP 能力。
- 关闭 Server 后管家绑定保留，重新启用后自动恢复有效。
- 管家绑定变化不改变任何角色绑定，角色绑定变化不改变管家绑定。
- 现有角色 MCP 行为及其测试保持不变。

## Implementation Patterns Addendum — FR-37～FR-39

### Skill 选择与原生调用模式

#### 前端选择状态

`@Skill` 只是一种前端交互形式，不作为后端解析协议。前端必须将选择结果保存为结构化字段：

```typescript
interface ChatSubmitInput {
  content: string;
  selectedSkillId: string | null;
}
```

- `selectedSkillId` 保存 Skill Registry ID，不保存展示名或 OpenCode command name。
- `@Skill` 展示标签不是 `content` 的组成部分；删除标签时同时清空 `selectedSkillId`。
- V1 每条消息最多指定一个 Skill。
- 候选列表只显示当前管家或角色已绑定且 enabled 的 Skill。

#### 后端解析与路径分流

后端统一执行：

```text
selectedSkillId
→ 查询 SkillRegistryEntry
→ 校验属于当前管家或角色且 enabled
→ 读取 SkillRegistryEntry.name
→ 转换为 OpenCode command
```

前端不得提交 OpenCode command name；后端不得通过展示名或聊天文本中的 `@xxx` 执行 Skill。消息分流集中在 AgentBridge 或其直接上层：未选择 Skill 使用 `/message`，已选择 Skill 使用 `/command`。Skill 校验或执行失败时直接返回错误，不得静默降级到普通消息；两条路径复用相同的会话身份解析、流式事件和取消处理。

当前 Agent 身份必须由后端根据 conversation/session 解析，普通消息与 Skill command 使用同一套 Agent 映射函数，前端不得任意指定 Agent。

### Dashboard Metrics 查询模式

#### Scope 与时间 DTO

前端统一使用可辨识联合类型：

```typescript
type DashboardMetricsScope =
  | { type: 'all' }
  | { type: 'butler' }
  | { type: 'role'; roleId: string };

interface DashboardMetricsQuery {
  scope: DashboardMetricsScope;
  startAt: string | null;
  endAt: string | null;
}
```

Rust 使用 `#[serde(tag = "type", rename_all = "camelCase")]` 的对应 enum。禁止用 `roleId = null`、`"all"` 或 `"butler"` 表示不同作用域。

时间校验和规范化只在聚合 Service 实现一次：两个边界均为空表示全部时间；两个边界均存在表示 RFC 3339 半开区间 `[startAt, endAt)`；仅存在一个边界或 `startAt >= endAt` 返回 ValidationError。Repository 不自行解释本地时区、自然日或闭区间。

#### 指标与失败一致性

`taskCount` 和 `pendingTaskCount` 必须复用同一个任务作用域、删除状态和时间谓词，后者只额外增加 `is_completed = 0`，保证 `pendingTaskCount <= taskCount`。`conversationCount` 始终按 conversation/session 记录计数，不得按消息计数。

返回值使用数值字段和 RFC 3339 `generatedAt`。任一数据源失败时整个请求失败，不返回部分 DTO，也不使用 `0` 掩盖错误；前端可保留上次成功结果，但必须显示刷新失败状态。

### 管家 MCP 绑定模式

#### 数据库与 Repository 命名

新增表固定命名为 `butler_mcp_servers`，字段遵循现有 snake_case 规范。不新增通用 `agent_mcp_bindings`、scope 字段、虚拟 Butler role ID 或管家专属 enabled 字段。

Repository 与现有角色实现严格对称：

```rust
list_mcp_servers_for_butler(...)
list_available_mcp_servers_for_butler(...)
add_mcp_server_to_butler(...)
remove_mcp_server_from_butler(...)
```

排序、Server 不存在、重复绑定、移除不存在绑定和级联删除语义均匹配角色对应函数；本轮不抽取通用 Agent MCP Repository。

#### Tauri Command 与 Service

Command 使用现有域前缀：

```rust
mcp_server_list_for_butler
mcp_server_list_available_for_butler
mcp_server_add_to_butler
mcp_server_remove_from_butler
mcp_server_refresh_butler_runtime
```

Command 只解析参数并调用 Service，不直接执行 SQL、修改配置或实现 Sidecar 刷新。管家添加/移除绑定沿用角色当前的校验、Agent 配置同步、`OpencodeMcpScopeLock`、Sidecar 和 Session 刷新调用顺序。管家绑定操作使用 `refresh_opencode_runtime()`（返回 `Result` 的刷新变体）而非 `refresh_opencode_runtime_after_mcp_change()`（吞掉错误的角色变体），以便在部分失败时向前端返回明确错误。`saved_butler_runtime_error()` 将已持久化但刷新失败的情况包装为 `ValidationError`，前端据此区分"配置未保存"与"配置已保存但 Runtime 未刷新"两种失败，并提供"仅重试刷新"入口（`mcp_server_refresh_butler_runtime`）。`sync_butler_agent` 为 `pub` 并返回 `Result<(), AppError>`，不再吞掉配置投影错误。管家 Skill 更新（`app_update_butler_skills`）同样参与 `OpencodeMcpScopeLock`，防止与绑定变更并发覆盖 `opencode.json`。

新增 `butler_enabled_mcp_lines(...)`，按管家绑定集合过滤 enabled Server 并生成能力说明，语义匹配 `role_enabled_mcp_lines(...)`。除编译适配外，不改变 `role_enabled_mcp_lines(...)`、`add_to_role(...)`、`remove_from_role(...)` 或 `sync_mcp_scope_for_role(...)` 的行为。

### 数据主权一致性

新增 `butler_mcp_servers` 后必须同步检查数据导出、导入、全量销毁、Server 删除级联和测试数据库初始化。导出结构与角色 MCP 绑定保持同类风格，但使用独立集合，不能把管家伪装成角色。

### 测试一致性模式

Skill 测试必须覆盖作用域与 enabled 校验、message/command 分流、Registry name 转换、无 Runtime 调用的拒绝路径及禁止静默降级。

Dashboard 测试必须覆盖三种作用域、半开区间、会话而非消息计数、删除过滤、`pendingTaskCount <= taskCount` 以及任一数据源失败时整体失败。

管家 MCP 测试必须覆盖管家与角色绑定隔离、关闭 Server 的新增绑定限制、既有绑定保留与重新启用恢复、配置同步，以及现有角色测试不变。测试名称描述业务意图而非单纯返回值。

### 增量变更边界

所有开发 Agent 必须：

1. 优先扩展现有同类 Service、Repository 和 AgentConfig 路径。
2. 不创建第二套消息流、Runtime 刷新或错误包装机制。
3. 不借管家 MCP 功能重构角色 MCP。
4. 不引入通用 Agent MCP 权限模型。
5. 不使用 Prompt 文本执行可由结构化字段完成的确定性路由。
6. 新增 Tauri Command 时同步增加前端 Service、TypeScript DTO 和注册入口。
7. 新增数据库表时同步检查 migration、导入导出、销毁和测试初始化。
8. 任一步骤失败必须显式返回，禁止记录日志后继续宣称成功。

**禁止模式：**

- 从聊天正文正则解析 `@Skill` 作为唯一选择依据。
- 前端直接发送 OpenCode command name。
- Skill command 失败后自动改发普通消息。
- 使用 `roleId = "butler"` 表示管家。
- 给 `butler_mcp_servers` 增加独立 enabled 字段。
- 为管家 MCP 新建 Runtime Coordinator。
- 顺手重构现有角色 MCP 绑定。
- 将消息数量作为 `conversationCount`。
- 用零值掩盖统计数据源失败。

## Project Structure Addendum — FR-37～FR-39

### 增量目录结构

标记：`[M]` 修改现有文件；`[N]` 新增文件；`[U]` 仅验证且不改变既有行为。

```text
egosync-app/
├── src/
│   ├── components/
│   │   ├── chat/
│   │   │   ├── ChatInput.tsx                         [M] @Skill 选择器和结构化提交
│   │   │   ├── ChatInput.test.tsx                    [M] 选择、清除、禁用状态测试
│   │   │   ├── ChatInput.a11y.test.tsx               [M] 候选列表键盘与读屏测试
│   │   │   ├── ChatStream.tsx                        [M] 持有 selectedSkillId 并提交
│   │   │   └── ChatStream.test.tsx                   [M] message/command 分流测试
│   │   ├── butler/
│   │   │   ├── DashboardTab.tsx                      [M] 指标卡、时间与角色筛选
│   │   │   ├── DashboardTab.test.tsx                 [M] 指标、筛选、失败状态测试
│   │   │   ├── DashboardTab.a11y.test.tsx            [M] 指标和筛选可访问性
│   │   │   ├── ButlerSettingsContent.tsx             [M] 管家 MCP 绑定管理
│   │   │   └── ButlerSettingsContent.test.tsx        [M] 管家绑定交互测试
│   │   └── role/
│   │       ├── SettingsTab.tsx                        [U] 角色 MCP/Skill 行为不变
│   │       └── SettingsTab.test.tsx                   [U] 角色回归测试
│   ├── hooks/
│   │   ├── useDashboard.ts                            [M] 状态指标和筛选加载
│   │   └── useDashboard.test.ts                       [M] 刷新与旧数据保留测试
│   ├── services/
│   │   ├── chatService.ts                             [M] 传递 selectedSkillId
│   │   ├── dashboardService.ts                        [M] 新增 getMetrics(query)
│   │   ├── skillService.ts                            [M] 当前作用域候选 Skill
│   │   └── mcpService.ts                              [M] 管家绑定五个接口（含 refreshButlerRuntime）
│   └── types/
│       ├── chat.ts                                    [M] ChatRequest.selectedSkillId
│       ├── dashboard.ts                               [M] Metrics Query/Scope/Result
│       ├── skill.ts                                   [U/M] 复用 Registry DTO
│       └── mcp.ts                                     [U] 复用 McpServer DTO
├── src-tauri/
│   ├── migrations/
│   │   └── 028_butler_mcp_servers.sql                 [N] 管家 MCP 关联表
│   └── src/
│       ├── commands/
│       │   ├── chat.rs                                [M] 接收 selectedSkillId
│       │   ├── dashboard.rs                           [M] dashboard_get_metrics
│       │   ├── mcp.rs                                 [M] 管家绑定 Commands
│       │   ├── skill.rs                               [M] 必要的作用域候选查询
│       │   └── mod.rs                                 [M] 仅在模块导出需要时更新
│       ├── db/
│       │   ├── skill_bindings.rs                      [U/M] 复用作用域绑定查询
│       │   ├── skills.rs                              [U/M] 复用 enabled 查询
│       │   ├── tasks.rs                               [M] 任务指标聚合
│       │   ├── memories.rs                            [M] 记忆数量聚合
│       │   ├── conversations.rs                       [M] 会话数量聚合
│       │   └── mcp_servers.rs                         [M] 管家绑定 CRUD
│       ├── models/
│       │   ├── chat.rs                                [M] selected_skill_id
│       │   ├── dashboard.rs                           [M] Query/Scope/Metrics DTO
│       │   ├── skill.rs                               [U] 复用 SkillRegistryEntry
│       │   ├── mcp.rs                                 [U] 复用 McpServer
│       │   └── mod.rs                                 [M] 仅在新增导出需要时更新
│       ├── services/
│       │   ├── agent_bridge.rs                        [M] 原生 send_command
│       │   ├── agent_engine.rs                        [M] 统一会话执行流程
│       │   ├── skill_registry.rs                      [M] Skill 作用域与 enabled 校验
│       │   ├── dashboard_service.rs                   [M] 跨库聚合
│       │   ├── mcp_server.rs                          [M] 管家绑定 Service
│       │   ├── agent_config.rs                        [M] 管家 Agent MCP 能力说明
│       │   └── data_export.rs                         [M] 管家绑定导入导出
│       └── lib.rs                                     [M] 注册新增 Tauri Commands
└── tests/e2e/specs/
    ├── butler-conversation.spec.ts                    [M] @Skill 管家主路径
    └── role-crud.spec.ts                              [U/M] 必要角色回归场景
```

### 架构边界

#### FR-37：Skill 原生调用

```text
ChatInput → ChatStream → chatService → chat_send_message
→ skill_registry 校验 → agent_engine
→ agent_bridge.send_command → OpenCode /session/{id}/command
```

`ChatInput` 只负责候选展示和选择；`ChatStream` 持有 `selectedSkillId`；`skill_registry` 负责绑定、enabled 和 Registry name 校验；`agent_engine` 复用当前会话、流式事件和完成回退；`agent_bridge` 只转换已校验参数并发送 HTTP。UI 不决定 OpenCode command，AgentBridge 不查询绑定数据库。Onboarding 的普通消息因 `selectedSkillId` 可空而保持原行为。

#### FR-38：仪表盘统计

```text
DashboardTab → useDashboard → dashboardService.getMetrics
→ dashboard_get_metrics → DashboardAggregationService
├── tasks/memories：主 DbPool
└── conversations：ConversationsPool
```

UI 管理筛选和展示；Hook 管理查询、刷新、loading/error 和旧结果保留；Command 仅反序列化参数；`dashboard_service.rs` 是 scope 和时间规范化的唯一位置；DB 模块只执行已规范化的统计 SQL。现有 `dashboard_get_status` 继续只负责角色状态卡。

#### FR-39：管家 MCP 绑定

```text
ButlerSettingsContent → mcpService
→ mcp_server_*_for_butler → mcp_server Service
→ mcp_servers DB → agent_config 管家同步
→ 现有 Runtime/Session 刷新路径
```

管家 UI 和 Service 与角色接口对称；DB 管理 `butler_mcp_servers`；`agent_config.rs` 只扩展管家 MCP 能力说明；`data_export.rs` 完成数据主权闭环。角色 `SettingsTab` 和角色 MCP 后端路径是回归边界，不属于本轮重构范围。

### 数据边界

主 `DbPool` 继续保存 Skill、绑定、MCP Server、管家/角色 MCP 绑定、任务和记忆。`conversations` 与 `messages` 继续位于独立 `ConversationsPool`。

`DashboardAggregationService` 可以同时读取两个连接池，但不得跨库写入、复制会话数据或让前端分别调用多个计数接口自行聚合；不承诺跨数据库事务级快照。

### Tauri IPC 边界

新增 IPC：

```text
dashboard_get_metrics
mcp_server_list_for_butler
mcp_server_list_available_for_butler
mcp_server_add_to_butler
mcp_server_remove_from_butler
mcp_server_refresh_butler_runtime
```

修改现有 `chat_send_message` 的 `ChatRequest`，增加可空 `selectedSkillId`。每个新增 Command 必须在 Rust Command、`lib.rs generate_handler!`、前端 Service、TypeScript DTO 和测试中闭环。

### 数据库迁移与备份边界

`028_butler_mcp_servers.sql` 只创建管家绑定表、Server 外键、级联删除及必要最小约束，不修改角色 MCP 表。新增表后同步更新 `data_export.rs` 的 JSON 导出/导入、SQLite 恢复清单、数据销毁清单和回滚测试。

### 需求到文件映射

| 需求 | 前端位置 | 后端位置 |
|---|---|---|
| 管家/角色 `@Skill` | `ChatInput.tsx`, `ChatStream.tsx` | `chat.rs`, `skill_registry.rs`, `agent_engine.rs`, `agent_bridge.rs` |
| Skill 有效集合 | `skillService.ts` | `skills.rs`, `skill_bindings.rs`, `skill_registry.rs` |
| 仪表盘四项指标 | `DashboardTab.tsx`, `useDashboard.ts` | `dashboard.rs`, `dashboard_service.rs`, 三个 DB 模块 |
| 时间和角色筛选 | `DashboardTab.tsx`, `dashboard.ts` | `models/dashboard.rs`, `dashboard_service.rs` |
| 管家 MCP 绑定 | `ButlerSettingsContent.tsx`, `mcpService.ts` | migration 028、`mcp_servers.rs`, `mcp_server.rs`, `agent_config.rs` |
| MCP 数据导入导出 | 无新增页面 | `data_export.rs` |
| Tauri 注册 | 前端 Service | `lib.rs` |

### 禁止跨越的边界

- UI 不直接聚合多个数据库计数结果。
- Command 层不写 SQL 或 OpenCode 配置。
- DB 层不调用 Sidecar、AgentConfig 或发送 Tauri Event。
- AgentBridge 不查询 Skill 绑定，SkillRegistry 不发送 HTTP。
- Dashboard Service 不读取任务、记忆或会话正文。
- 管家 MCP 代码不改变角色 MCP 业务语义。
- 不为三项需求新增平行的通用框架或目录层级。

## Architecture Validation Results — FR-37～FR-39

本次验证覆盖架构文档、需求和当前代码结构的一致性；尚未修改业务代码，也未运行 Cargo、Vitest 或 E2E，结论仅表示架构已具备实施指导能力，不表示功能测试通过。

### Coherence Validation

| 检查项 | 结果 | 说明 |
|---|---|---|
| FR-37 与 Skill Registry | 通过 | 复用 Registry、绑定和 enabled 状态，不创建新 Skill 引擎 |
| FR-37 与 opencode | 通过 | Registry name 转为原生 command，普通消息路径不变 |
| FR-38 与现有 Dashboard | 通过 | 保留 `dashboard_get_status`，新增独立 Metrics 接口 |
| FR-38 与双数据库边界 | 通过 | 主库统计任务/记忆，`ConversationsPool` 统计会话 |
| FR-39 与现有角色 MCP | 通过 | 管家新增独立绑定，角色业务逻辑保持不变 |
| FR-39 与最小改动原则 | 通过 | 不增加 Permission 模型、Runtime Coordinator 或通用绑定框架 |
| 数据主权 | 通过 | 新管家绑定表纳入导入、导出、销毁和恢复 |
| 命名与目录规范 | 通过 | DTO、Command、Migration 和前端 Service 遵循既有规范 |

新增实现模式与既有 camelCase/snake_case、薄 Command、Service/DB 分层、co-located tests 和显式错误规范一致。实际代码已经存在 Chat、Dashboard、MCP 三条必要扩展入口，仅需新增 `028_butler_mcp_servers.sql`，其余为现有文件的外科手术式扩展。

### Requirements Coverage

#### FR-37

已覆盖管家和角色 `@Skill` 选择、当前 Agent 已绑定且 enabled 的候选集合、请求级 `selectedSkillId`、后端授权校验、Registry name 到 opencode command 转换、普通 `/message` 保持以及禁止失败静默降级。

#### FR-38

已覆盖任务数、记忆数、对话会话数、待处理任务数、all/butler/role 作用域、RFC 3339 半开时间区间、组合筛选、共享任务谓词及跨数据库全有或全无失败策略。

#### FR-39

已覆盖现有 Server CRUD/测试/启停、管家独立绑定、绑定与 enabled 交集、关闭时保留绑定、重新启用恢复、管家和角色绑定隔离、现有 Runtime 刷新复用以及数据导入导出闭环。

### Gap Analysis

**Critical Gaps：无。**

#### Minor Gap 1：PRD FR-39 文本同步

当前 PRD FR-39 重点仍是 MCP Server enabled 状态与角色绑定独立，尚未明确最新架构增加的管家绑定。最新用户决策优先于旧表述，但在生成 Story 前应在 PRD 增加：

- 管家可添加和移除自己的 MCP Server 绑定。
- 管家绑定与所有角色绑定相互独立。
- 管家只能使用已绑定且 enabled 的 Server。
- 关闭 Server 保留管家绑定，重新启用后恢复有效。

#### Minor Gap 2：管家新增路径的失败语义

角色现有失败语义本轮不修改。新增管家绑定路径沿用角色的调用顺序，但 DB、AgentConfig 或现有 Runtime 刷新失败时必须返回错误，不得静默成功。若数据库绑定已保存但后续刷新失败，错误应明确表示“绑定已保存，但 Agent 配置刷新失败”。本轮不增加跨数据库/配置文件事务或自动回滚机制。

#### Deferred Technical Debt

以下事项经过明确选择，不属于本轮缺口：

- 角色 MCP 改用 Agent Permission。
- 按对话改写全局 MCP 配置的并发覆盖问题。
- 通用管家/角色 MCP Binding 模型。
- MCP Runtime revision 和自动 reconciliation。
- 跨主库与会话库的事务级统计快照。
- 仪表盘物化统计或缓存。
- 多 Skill 同时指定与编排。

实施 Agent 不得顺手处理上述技术债。

### Implementation Readiness

建议顺序：

1. 同步 PRD FR-39 的管家绑定验收条件。
2. 增加共享 DTO 和数据库 Migration。
3. 完成 FR-37 后端及前端垂直链路。
4. 完成 FR-38 Repository、聚合 Service、Command、Hook 和 UI。
5. 完成 FR-39 管家绑定、AgentConfig、UI 和数据导入导出。
6. 运行完整 Rust、前端和 E2E 验证。

### Architecture Completeness Checklist

**Requirements Analysis**

- [x] Project context thoroughly analyzed
- [x] Scale and complexity assessed
- [x] Technical constraints identified
- [x] Cross-cutting concerns mapped

**Architectural Decisions**

- [x] Critical decisions documented with fixed technology versions
- [x] Technology stack fully specified
- [x] Integration patterns defined
- [x] Performance considerations addressed

**Implementation Patterns**

- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Communication patterns specified
- [x] Process patterns documented

**Project Structure**

- [x] Complete incremental directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements-to-structure mapping complete

### Architecture Readiness Assessment

**Overall Status：READY WITH MINOR GAPS**

**Confidence Level：medium**

**Key Strengths：**

- 三项需求均复用现有架构入口，没有增加平行框架。
- Skill 使用 opencode 原生 command，而非自定义 Prompt 协议。
- Dashboard 指标口径和跨库边界清晰。
- 管家 MCP 使用独立关联表，不污染角色模型。
- 实现边界明确限制了无关角色 MCP 重构。
- 测试与数据主权要求已映射到具体文件。

**Remaining Actions Before Story Implementation：**

1. 更新 PRD FR-39 的管家绑定描述和验收条件。
2. 在 Story 中明确管家配置同步失败的返回语义。
3. 实施后运行基线和新增测试；当前验证不代表测试已通过。

**Implementation Handoff：**

实施 Agent 必须遵守本文决策、增量实现模式、文件边界和明确排除项。首要动作是同步 PRD FR-39，然后创建或更新 Epics/Stories，再按 FR-37、FR-38、FR-39 分别实施垂直切片。

## Incremental Project Context Analysis — 手机伴侣基建（FR-40～FR-43）

### Requirements Overview

**功能需求（FR-40~43，全部压在全新基础设施上）：**

| FR | 内容 | 架构影响 |
|----|------|----------|
| FR-40 | 配对与连接管理 | ①扫码/配对码的密钥交换与信任建立；②局域网自动发现直连；③出网云中继加密转发；④直连↔中继↔离线三态连接状态机。云中继 = 项目首个服务端组件 |
| FR-41 | 实时状态同步 | 状态通道（桌面→手机长连接主动推送）+ 指令通道（手机操作=远程命令，结果回流）；流式对话需跨设备桥接；断线重连=最新快照补齐（明确不做历史回放） |
| FR-42 | 移动推送 | 推送通道：三级通知→Android 系统渠道；FCM 与自建的分界待裁决；锁屏快捷操作须等效回流；桌面未运行=无新推送（既定边界） |
| FR-43 | 离线降级 | 快照口径裁决（哪些域、多大上限）+ 速记排队（本地队列、重连自动提交、无丢失） |

隐性需求：一档 18 项核心 FR + 二档 6 项适配 FR 的"移动呈现"——其架构含义全部收敛到上述四条通道（连接/状态/指令/推送），不逐 FR 设计。

**非功能需求：**

- **隐私 = 技术不可能性**：中继必须零知识——只转发端到端加密流量、无法读明文、零落地存储（PRD Privacy 守则的直接延伸）
- **成本红线**：V1 免费且本地优先，中继是首个常驻成本项；PRD 明确要求部署与成本方案在本阶段定论
- **诚实代价原则**：桌面关机→手机降级为只读快照+速记排队，是架构固有属性，UI 必须明示而非掩盖
- **桌面零回归**：所有变更不得动摇基线三层架构、双库边界与 keyring 管理

**Scale & Complexity:**

- Primary domain: 三端系统（桌面 Rust 编排层扩展 + Android 客户端 + 极简云中继服务）
- Complexity level: High —— 新增网络基础设施五件套：服务发现、NAT 穿越回退、E2E 加密、配对信任、移动端生命周期
- 预计新增架构组件：约 8~10 个

### Technical Constraints & Dependencies

| 约束 | 来源 | 影响 |
|------|------|------|
| 桌面唯一事实源 | 决策#17 / FR-25 | 手机无业务库主权，只有快照缓存；引擎/DB/keyring/opencode 全留桌面 |
| 中继仅加密转发、不落地 | PRD ASSUMPTION | 排除一切"中继暂存/离线投递"设计 |
| V1 一台手机 ↔ 一台桌面 | FR-40 ASSUMPTION | 配对模型单对单，无需多设备路由 |
| 仅 Android，iOS=V2 | 决策#17 | 技术栈选型只需覆盖 Android，但不宜堵死 iOS |
| 工作循环仅桌面运行时执行 | FR-10 | 推送的产生方永远是桌面；不存在云端代替桌面跑循环 |
| 基线规范全量继承 | project-context.md | 命名/IPC/错误处理/keyring 规则延伸到新模块 |

### Cross-Cutting Concerns Identified

1. **配对与信任链** — 密钥交换机制、防中间人、重装恢复流程
2. **连接状态机** — mDNS 自动发现、直连/中继无缝切换、状态可见性
3. **E2E 加密通道** — 会话密钥协商、重连续传、四通道共用一条加密承载
4. **中继本体** — 自建 WebSocket 中继 vs 成熟隧道方案；协议、鉴权、部署形态、成本
5. **App 技术栈与推送链路** — Kotlin 原生 vs 跨端复用；FCM 分界；快照口径裁决

## Incremental Starter Template Evaluation — 手机伴侣基建（FR-40～FR-43）

### Primary Technology Domain

三端系统：Android 原生客户端（Kotlin + Jetpack Compose）+ 桌面 Rust 编排层扩展（现有代码库）+ 自建云中继（Rust 极简服务，单 VPS Docker 部署）

### Setup Strategy

**Manual/Minimal Init** —— 无现成脚手架覆盖"三端 + 自定协议"形态；三个子项目各自用官方最小模板起步，不引入全家桶框架。

| 子项目 | 初始化方式 | 理由 |
|--------|-----------|------|
| `companion-android/` | Android Studio 官方 Compose 模板新建 | 原生栈一等公民能力（FCM 渠道、锁屏操作、前台服务、NsdManager） |
| `relay-server/` | `cargo new` + axum 最小骨架 | 与桌面同语言同 tokio 运行时，可共享加密协议 crate；几百行可审计 |
| 桌面端 | 现有 `egosync-app/src-tauri` 增量扩展 | 零重建，新增 connection/pairing 服务模块，沿用基线分层 |

### Initialization Commands

```bash
# Android：Android Studio → New Project → Empty Activity (Compose)，包名 com.egosync.companion
# 中继：
cargo new relay-server
# 依赖：axum 0.8 + tokio + tokio-tungstenite + （E2E 协议库 step-04 定）
# 部署：多阶段 Dockerfile → 单 VPS docker compose up（无状态、无卷挂载）
```

### Architectural Decisions Provided by Starter

**Language & Runtime:**
- Android: Kotlin 2.x（当前 2.3+）+ JDK 17 + Jetpack Compose / Material 3（BOM 取初始化时官方模板内置最新）
- 中继 & 桌面扩展: Rust edition 2021 + tokio（同一异步运行时心智）
- 协议契约: 手机↔桌面消息协议以单一 schema 为事实源，生成 TS/Kotlin/Rust 类型（机制细节 step-04 裁决）

**Build Tooling:** Gradle KTS（Android）/ Cargo（Rust ×2）/ 多阶段 Dockerfile（中继）

**Testing Framework:** Compose UI Test + JUnit（Android）；`#[cfg(test)]` + 集成测试（中继）；基线 Vitest/cargo/E2E 体系不变（桌面）

**Code Organization:**

```
EgoSync/
├── egosync-app/src-tauri/   # [扩展] services/{connection,pairing,snapshot}.rs 等
├── companion-android/       # [新] Kotlin + Compose 单模块起步
└── relay-server/            # [新] 无状态加密转发服务
```

**否决记录：**
- ~~Capacitor 复用 React~~ — 推送/后台需插件桥接，伴侣 UI 本非移植，复用价值低
- ~~Tauri 2 Mobile~~ — Android 推送生态不成熟，双端框架耦合升级风险高
- ~~iroh 1.0~~ — 能力全面但两端嵌 Rust 的 JNI 构建链复杂度不值（记录备选理由，防回头重提）
- ~~Tailscale 类隧道~~ — 第三方账号体系绑定，违背零知识自托管初衷

## Incremental Core Architectural Decisions — 手机伴侣基建（FR-40～FR-43）

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**

| # | 决策 | 裁决 | 理由 |
|---|------|------|------|
| 1 | 配对密钥交换 | Noise XX + 扫码互信（Rust: snow；Android: noise-java） | 握手内双向认证防中间人，无 PKI/证书管理；重装=重新扫码即恢复 |
| 2 | 直连↔中继协议关系 | 同一加密帧协议、双承载 | 手机端只换地址不换逻辑，状态机最简；否决双协议方案 |
| 3 | 云中继本体 | 自建 Rust 无状态转发服务（axum 0.8），内存注册表零持久化 | 零知识可审计哲学落地；断线即丢不做离线投递 |
| 4 | 指令注入点 | 桌面编排层新增 `companion` 服务模块为唯一入口 | 手机永不直接触达 agent_bridge/opencode/DB，基线边界零破坏 |
| 5 | 快照口径 | 大而全落死：活跃/近期会话各最近 200 条 + 本季度简报/复盘 + 未读通知；记忆库不上机；上限 10MB 超限截断明示 | boss 权衡后选体验优先；口径落死防实现漂移 |
| 6 | 推送链路 | **FR-42 正式降级出 V1**；保留 NotificationDispatch 抽象层 | Android FCM 凭证无法分发到用户桌面端；V1 仅应用内通知 |

**Important Decisions (Shape Architecture):**
- 局域网发现：NSD/mDNS 广播 `_egosync._tcp`（PRD"无需人工干预"的标准解）
- 连接状态机：`direct / relay / offline` 三态，prefer-direct 切换策略
  （2026-09-08 增补，SPEC-companion-connection-chat-ux：呈现层细化为 TransportStatus——Connecting/Reconnecting（20s 宽限）/Direct/Relay/Degraded(snapshotAvailable, dataAsOf)，叠加 PairingHealth 凭据健康面与 commandReady 会话就绪门；凭据失效恢复事件原子清态并回配对流）
- 中继鉴权：挑战-应答证明公钥所有权（防 ID 抢占）
- 协议契约：单一 schema 为事实源，生成 TS/Kotlin/Rust 类型

**Deferred Decisions (Post-V1):**
- iOS 伴侣 App（PRD V2 既定）
- 系统推送接入（FCM 中继代理或 UnifiedPush——分发抽象已预留适配器位）
- 多桌面绑定（V1 单对单 [ASSUMPTION] 保持）
- 云端数据同步/托管（Non-Goal 不变）

### 配对与信任链（FR-40）

**配对流程：**
```
桌面：生成 QR = { relay_addr, desktop_static_pubkey, relay_id(=pubkey哈希), pairing_nonce }
手机：扫码 → 经直连或中继发起 Noise XX 握手 → 双向身份认证 → 派生会话密钥
桌面：写入 paired_devices 表 → 后续连接免配对
重装恢复：手机新实例重新扫码即可，桌面无需重置（符合 FR-40 验收）
```

**密钥存储：**
- 手机静态私钥 → Android Keystore
- 桌面静态密钥 → 系统 keyring（与 LLM API Key 同级管理，遵循基线安全规则）
- 手机快照缓存落盘 → Keystore 派生 AES 加密

### 连接与通道架构（FR-40/41）

**四通道收敛为一条加密承载上的帧类型集：**
`HELLO / SNAPSHOT / STATE_DELTA / COMMAND / COMMAND_RESULT / STREAM_TOKEN / NOTICE / PING`

- 连接通道：NSD 发现 → WS 直连桌面端口；出网 → WS 连中继按 relay_id 转发。同一帧协议。
- 状态通道：桌面状态变化 → `STATE_DELTA` 主动推送；断线重连 → 全量 `SNAPSHOT` 替换（不做历史回放，PRD 假设保持）
- 指令通道：手机操作 → `COMMAND` → companion 模块执行现有 services → `COMMAND_RESULT` 回流；流式对话经 `STREAM_TOKEN` 帧镜像 `llm:stream` 语义
- 配对拒绝（2026-09-08 增补）：桌面两拒绝点（配对窗口关闭/nonce 已消费、已配对后设备被移除）先发 `NOTICE{type:pairingRejected,reason}` 再关连接——复用 Notice 帧类型，无 schema 变更；手机配对 probe 据此结构化失败，会话循环收到则触发 PairingRevoked 恢复

### 云中继本体

- axum 0.8 + tokio + tokio-tungstenite；与桌面共享加密协议 crate
- 内存注册表 `{relay_id → 连接}`，零持久化、零落地（进程重启即清空）
- 鉴权：注册时挑战-应答证明持有对应私钥
- 纯二进制加密帧转发，中继不可读明文（Noise 会话密钥仅两端持有）
- 部署：多阶段 Dockerfile → 单 VPS docker compose up；`/healthz` 健康检查；tracing 结构化日志；metrics 延后
- 成本模型：单实例可承载数百并发长连接，$5/月级 VPS 起步

### 快照引擎（FR-41/43）

- 口径（已裁决落死）：角色卡状态（含能量）+ 四象限任务 + 仪表盘指标 + 活跃及近期会话各最近 200 条消息 + 本季度晨间简报/周复盘 + 未读通知；**记忆库不上机**
- 总量上限 10MB，超限按最旧截断并在 UI 明示数据截止时间
- 生成：桌面端相关写操作后节流重建（debounce），内存持有不持久化（重启后首次连接现生成）
- 下发：全量替换式；重连即补齐最新快照
- 手机端：单一版本化文件存储 + 元数据，不引入 Room

### 降级态与离线待发（FR-43，2026-09-08 裁决增补）

- 速记队列（QuickNoteQueue）退役：离线文字录入经对话 composer 进入 ChatOutbox 离线待发箱（幂等 commandId、快照密钥加密落盘，进程被杀不丢）
- 恢复（commandReady && paired）后串行自动续发，桌面确认入库后出队，无丢失
- 降级 UI：非阻断紧凑状态指示（Connecting/Reconnecting/Degraded 横幅携带 dataAsOf）+ 依赖引擎的入口逐项禁用并说明原因；全屏阻断遮罩与速记条范式废止

### 通知分发（FR-42 降级记录）

- **V1 仅应用内通知**（App 前台/连接时呈现三级通知）
- NotificationDispatch 抽象层保留：V1 实现 InAppAdapter；FCM 中继代理 / UnifiedPush 为后续可插适配器
- 锁屏快捷确认/拒绝随 FR-42 一并延后至推送接入时
- ⚠️ **待办（架构定稿随附）**：修订 PRD §4.14 FR-42（移出 MVP 或标 DEFERRED）、§6.1 MVP 范围、§9 假设索引；决策日志新增 #18 记录本次降级裁决及理由（FCM service account 凭证无法安全分发到用户桌面端）

### Data Architecture（增量）

- 桌面新增表 `paired_devices`（SQLx migration）：`id, device_name, device_pubkey, paired_at, last_seen_at`
- 该表纳入 data_export 导入导出与销毁清单（基线数据主权规则延伸）
- 手机端：加密快照文件 + 速记持久化队列；无业务库主权（决策#17 边界不变）
- 中继端：零数据库

### Decision Impact Analysis

**Implementation Sequence:**
1. 加密协议 crate（Noise XX 帧封装）+ 协议 schema 定义
2. 桌面 pairing/connection 服务模块（QR 生成、NSD 广播、WS 监听、paired_devices）
3. relay-server 最小转发服务 + Docker 化
4. Android 骨架：扫码配对 + 连接状态机
5. 快照引擎 + 状态通道
6. 指令通道 + 流式回流
7. 降级态 + 速记队列
8. （预留位）NotificationDispatch 适配器接口

**Cross-Component Dependencies:**
- 加密协议 crate → 被桌面、中继、Android 三方共同依赖（最高优先级，先行冻结）
- companion 模块 → 复用全部现有 services；是手机指令的唯一入口
- paired_devices 表 → 被 pairing、connection、data_export 三处依赖
- NotificationDispatch → V1 只有 InAppAdapter 实装，接口先行稳定

## Implementation Patterns Addendum — 手机伴侣基建（FR-40～FR-43）

### Pattern Categories Defined

基线 100 条规则（project-context.md）继续全量生效；本节只定义伴侣基建引入的 **5 类新冲突点**，未提及处一律继承基线。

### Naming Patterns

**桌面新模块（扁平文件 + 前缀，不建子目录）：**
- `services/companion_pairing.rs` — QR 生成、Noise XX 握手编排、paired_devices 读写
- `services/companion_connection.rs` — NSD 广播、WS 监听、连接状态机、通道切换
- `services/companion_snapshot.rs` — 快照节流重建、SNAPSHOT 帧下发
- `services/companion_dispatch.rs` — COMMAND 解析 → 现有 services 调用 → RESULT 回流；NotificationDispatch 抽象

**共享协议 crate：**
- 位置：`crates/companion-proto/`（package 名 companion-proto，lib 名 companion_proto）
- 引用方式：src-tauri 与 relay-server 均 path 依赖
- **禁止**在仓库根建 Cargo workspace（避免扰动基线构建行为）

**新增 Tauri Commands（域前缀 snake_case）：**
- `pairing_generate_qr / pairing_confirm / paired_device_list / paired_device_remove / companion_get_status`

**Android 包结构（com.egosync.companion）：**
- `pairing/`（扫码、握手）、`connection/`（状态机、NSD）、`sync/`（SnapshotStore、QuickNoteQueue）、`notify/`（InAppAdapter）、`ui/<feature>/`
- 文件命名：`XxxScreen.kt / XxxViewModel.kt / SnapshotStore.kt / QuickNoteQueue.kt`

### Format Patterns

- 帧 = 长度前缀二进制 Noise 传输消息；内层 payload JSON 一律 camelCase（三语言一致）
- 枚举小写字符串；日期 ISO 8601；ID 为 UUID v4 —— 继承基线
- `HELLO` 帧必带 `protocolVersion: u16`；任何帧类型/payload 变更必须 bump 版本并同步 schema 单一事实源

### Communication Patterns

- 手机↔桌面帧类型集冻结为 8 种：`HELLO / SNAPSHOT / STATE_DELTA / COMMAND / COMMAND_RESULT / STREAM_TOKEN / NOTICE / PING`；扩展须走 schema 变更 + 版本 bump
- 桌面 UI 新 Tauri Events 用 `companion:` 命名空间：`companion:paired / companion:connected / companion:disconnected / quicknote:submitted`

### Process Patterns

- 重连退避：指数 1s→30s 封顶 + 随机抖动
- 直连↔中继切换滞回：NSD 消失后探测 3s 才回落中继，防抖动
- 错误分类：`AppError` 新增 `PairingError / ConnectionError / ProtocolError` 变体；序列化形状不变；用户可见文案中文不暴露技术细节
- 日志：两端 Rust 用 tracing；帧内容明文永远不入日志（零知识纪律）；Android 用带 `Companion/` 前缀 tag 的系统 Log

### Structure Patterns

**测试位置：**
- 中继：单元测试同文件 `#[cfg(test)]`；集成测试 `tests/`（含"零持久化"断言：重启后注册表为空）
- Android：单元测试 `src/test/` co-located；仅配对冒烟路径进 `src/androidTest/`
- 桌面：沿用基线（同文件单元测试 + tests/test_{domain}.rs）

### Enforcement Guidelines

**All AI Agents MUST:**
1. 所有手机指令必经 companion_dispatch 模块进入现有 services
2. 协议变更必须改 schema 单一事实源并三语言同步生成
3. 中继保持零持久化——新增任何落盘行为都是违约
4. 新增表/Command/事件遵循本文件与基线命名规范

**Anti-Patterns (禁止):**
- ❌ 在中继记录或存储任何帧内容
- ❌ 绕过 companion 模块直调 agent_bridge/opencode/DB
- ❌ Android 端引入 Room 或全局状态框架
- ❌ 仓库根 Cargo workspace
- ❌ 私增帧类型或绕过 protocolVersion

## Project Structure Addendum — 手机伴侣基建（FR-40～FR-43）

### Complete Project Directory Structure

标记：`[N]` 新增；`[M]` 修改现有文件；未标记处基线不动。

```text
EgoSync/
├── egosync-app/src-tauri/                  # 桌面端增量扩展
│   ├── migrations/031_paired_devices.sql   # [N]
│   ├── src/
│   │   ├── commands/companion.rs           # [N] pairing_*/paired_device_*/companion_get_status
│   │   ├── services/
│   │   │   ├── companion_pairing.rs        # [N]
│   │   │   ├── companion_connection.rs     # [N]
│   │   │   ├── companion_snapshot.rs       # [N]
│   │   │   └── companion_dispatch.rs       # [N]
│   │   ├── db/paired_devices.rs            # [N]
│   │   ├── models/companion.rs             # [N]
│   │   └── lib.rs                          # [M] 仅注册新 commands
│   └── tests/test_companion.rs             # [N]
├── crates/companion-proto/                 # [N] 共享加密协议 crate（Rust 侧唯一可触碰 Noise 库之处）
│   ├── Cargo.toml
│   └── src/{lib.rs, frames.rs, crypto.rs, schema.json}
├── relay-server/                           # [N] 零持久化转发服务
│   ├── Cargo.toml / Dockerfile / docker-compose.yml
│   └── src/{main.rs, registry.rs, forward.rs, auth.rs}
├── companion-android/                      # [N] Kotlin + Compose 单模块
│   ├── build.gradle.kts / settings.gradle.kts
│   ├── src/main/java/com/egosync/companion/
│   │   ├── MainActivity.kt
│   │   ├── pairing/{PairingScreen, PairingViewModel, QrScanner}.kt
│   │   ├── connection/{ConnectionViewModel, ConnectionStatusBanner, NsdDiscovery, RelayClient}.kt
│   │   ├── sync/{SnapshotStore, QuickNoteQueue, StateMerger}.kt
│   │   ├── notify/{NotificationDispatch, InAppNotificationAdapter}.kt
│   │   └── ui/{chat, dashboard, tasks, settings, theme}/…
│   ├── src/test/…                          # 单元测试 co-located
│   └── src/androidTest/…                   # 仅配对冒烟路径
└── .github/workflows/
    ├── ci.yml                              # [M] 增加 android 与 relay job
    ├── android-ci.yml                      # [N]
    └── relay-docker.yml                    # [N] 镜像构建发布
```

### Architectural Boundaries

**四条硬边界：**

1. **加密边界**：Rust 侧 `snow` 仅允许出现在 `crates/companion-proto`（src-tauri 与 relay-server 经 path 依赖复用）；Android 侧 `noise-java` 仅允许出现在 `pairing/`、`connection/` 换装层；三端其余代码只操作帧类型，不见密码学细节
2. **桌面边界**：companion_* 四个 service 只能经 `companion_dispatch` 调用既有 services；commands 保持薄层；现有 services 对伴侣一无所知
3. **中继边界**：只见 relay_id 与密文帧；无 DB、无磁盘写、断线即丢
4. **Android 边界**：UI 不触达连接实现（经 ViewModel → 连接客户端接口）；`sync/` 对快照只读渲染；速记只能进队列

**数据边界：**

| 数据 | 位置 | 规则 |
|------|------|------|
| 配对设备记录 | egosync.db `paired_devices` | 纳入导出/导入/销毁清单 |
| 会话密钥 | 双端内存 | 断线即弃，重连重新派生 |
| 快照缓存 | 手机加密文件 | Keystore 派生 AES；单版本化文件 |
| 速记队列 | 手机本地持久化 FIFO | 幂等 ID，确认后删队 |
| 中继状态 | 内存注册表 | 零持久化，进程重启清空 |

### Requirements to Structure Mapping

| FR | 归属 |
|----|------|
| FR-40 配对与连接 | `companion_pairing` + `companion_connection` + `relay-server/*` + Android `pairing/ connection/` |
| FR-41 状态同步 | `companion_snapshot`（状态通道）+ `companion_dispatch`(指令/流式回流) + Android `sync/ ui/` |
| FR-42（降级 V1） | `notify/NotificationDispatch` 抽象 + InAppAdapter 实装 |
| FR-43 离线降级 | Android `sync/QuickNoteQueue` + 降级态 UI；桌面侧零改动 |

### Integration Points & Data Flow

```text
[指令] 手机 COMMAND →(直连WS|中继WS)→ companion_connection → companion_dispatch → 既有 services → COMMAND_RESULT 回流
[状态] 桌面写操作 → companion_snapshot 节流重建 → STATE_DELTA/SNAPSHOT 推送 → 手机 StateMerger → UI
[速记] 手机本地队列 → 重连后逐条 COMMAND(幂等ID) → 管家处理 → 确认后删队
[配对] 桌面 QR → 手机扫码 → Noise XX 握手(直连或经中继) → paired_devices 落库
```

**外部集成：**
- 云中继 = 唯一新增外部服务端点（自托管）
- NSD/mDNS = 局域网发现（系统服务，无第三方依赖）
- 无其他第三方云依赖（推送延后）

### Development Workflow Integration

```bash
# 中继本地开发
cd relay-server && cargo run          # ws://127.0.0.1:7333（端口可配）
# 桌面：基线流程不变
cd egosync-app && npm run tauri dev   # 伴侣连接层随应用启动
# Android：Android Studio 打开 companion-android/
```

## Architecture Validation Results — 手机伴侣基建（FR-40～FR-43）

_本次验证覆盖架构文档、PRD §4.14 与基线架构的一致性；未修改业务代码，未运行任何测试；结论仅表示具备实施指导能力。_

### Coherence Validation ✅

| 检查项 | 结果 | 说明 |
|--------|------|------|
| Kotlin ↔ Rust 加密互通 | 通过（待首故事验证） | snow 与 noise-java 同属 Noise 框架规范；参数套件需冻结后互通测试 |
| axum 0.8 + tokio + tungstenite | 通过 | 同栈同运行时，与桌面一致 |
| 双端外连中继拓扑 | 通过 | 无入站端口转发需求，零配置约束自洽 |
| 与基线 100 条规则 | 通过 | 命名/分层/keyring/数据主权规则全量继承，无第二套范式 |
| 桌面零回归 | 通过 | 全部为增量模块，现有 services 对伴侣一无所知 |

### Requirements Coverage Validation ✅

- **FR-40**：扫码配对 / NSD 自动发现 / 直连↔中继自动切换（prefer-direct 滞回）/ 三态状态可见 / paired_devices 持久化 / 重装重扫恢复 / 中继不可读明文 —— 六条验收逐条有支撑
- **FR-41**：STATE_DELTA 推送 / COMMAND 执行回流 / STREAM_TOKEN 流式镜像 / 重连全量快照补齐
- **FR-42**：**已按 boss 裁决正式降级出 V1**（FCM service account 凭证无法安全分发到用户桌面端）；NotificationDispatch 抽象保留接入位；随附 PRD 修订待办
- **FR-43**：三态状态机统一覆盖"局域网关机"与"广域网中断"；只读缓存+截止时间标注；幂等速记队列

### Implementation Readiness Validation ✅

- 决策完整性：Critical×6 全部带版本与理由；实现模式含帧类型集、命名、错误分类
- 结构完整性：三子项目目录树到文件级；四条硬边界明确
- 模式完整性：5 类新冲突点全部约定，反模式清单齐备

### Gap Analysis Results

**Critical Gaps：无**

**Important Gaps（不阻塞，首故事内解决）：**
1. Noise 参数套件未冻结 —— 建议 `Noise_XX_25519_ChaChaPoly_BLAKE2s`，首个协议 story 验证 snow↔noise-java 互通后写入 `crates/companion-proto/src/schema.json`
2. 选型留实现期：桌面 mDNS 广播 crate（候选 mdns-sd）、Android WS 客户端（候选 OkHttp）与二维码扫描库（候选 ML Kit Barcode）
3. 桌面本地 WS 监听端口策略与 Windows 防火墙首次弹窗的 UX 处理

**Nice-to-Have Gaps（明确延后）：**
- 中继 metrics 可观测性、多实例横向扩展、快照传输压缩

### Validation Issues Addressed

FR-42 冲突已在决策阶段当面裁决并落档（降级 + PRD 修订待办 + 决策日志 #18 待记），无遗留矛盾。

### Architecture Completeness Checklist

**Requirements Analysis**
- [x] 项目上下文深度分析
- [x] 规模与复杂度评估
- [x] 技术约束识别
- [x] 跨切关注点映射

**Architectural Decisions**
- [x] 关键决策含版本号文档化
- [x] 技术栈完整指定
- [x] 集成模式定义
- [x] 性能考量已处理（节流重建/10MB 上限/退避滞回）

**Implementation Patterns**
- [x] 命名规范建立（增量部分）
- [x] 结构模式定义
- [x] 通信模式指定（8 种帧类型冻结）
- [x] 流程模式文档化

**Project Structure**
- [x] 完整目录结构定义（文件级）
- [x] 组件边界建立（四条硬边界）
- [x] 集成点映射
- [x] 需求到结构映射完成

### Architecture Readiness Assessment

**Overall Status:** READY WITH MINOR GAPS

**Confidence Level:** Medium（主要不确定性：跨语言 Noise 参数互通验证）

**Key Strengths:**
- 中继零知识零持久化，"技术不可能性"隐私哲学贯穿
- 单一加密帧协议双承载，连接层复杂度压至最低
- 手机指令唯一入口 companion_dispatch，基线边界零破坏
- FR-42 降级经显式裁决并留有演进接口，文档与 PRD 无暗雷

**Areas for Future Enhancement:**
- 系统推送接入（FCM 中继代理或 UnifiedPush）
- iOS 伴侣 App、多桌面绑定、云同步（均 Non-Goal/Deferred）

### Implementation Handoff

**AI Agent Guidelines:**
- 本文档（含手机伴侣增量章节）与 `project-context.md` 共同构成实施规范；冲突时以更新者为准并显式记录
- 遵守四条硬边界与反模式清单
- 协议变更必须走 schema 单一事实源

**First Implementation Priority:**
1. Story 1：`crates/companion-proto` —— 冻结 Noise 参数套件、定义 8 种帧 schema、snow/noise-java 互通冒烟测试
2. Story 2：桌面 `companion_pairing`（QR 生成 + paired_devices migration）
3. 后续按 Implementation Sequence 2~8 顺延
---

## Incremental Project Context Analysis — 云端托管版（FR-44～FR-48）

> 本章为 2026-09-17 立项的云端托管版（决策 #19）增量章节，遵循手机伴侣基建章节的增量范式；基线章节与既有增量章节的规则全量继承，未提及处一律沿用。启动时点：V1 收尾后（Epic 15 → {16, 17}，可并行），不与 Epic 12-14 交叉。本章所有耦合点数据来自 2026-09-17 代码验证。

### Requirements Overview

**功能需求（FR-44~48）：**

| FR | 内容 | 架构影响 |
|----|------|----------|
| FR-44 | 云端部署形态（Docker 一键、完整引擎、单实例单用户、单事实源） | 引擎与 Tauri 解耦、抽出可独立运行的 server binary；桌面版与云端版共用同一引擎 |
| FR-45 | WEB 客户端（任意现代浏览器、核心体验功能集） | 前端传输层双 transport（invoke ⇄ HTTP/SSE）；React 双目标；桌面专属能力（原生对话框）需能力门控 |
| FR-46 | 访问认证（单用户凭据、全端点强制认证、TLS、密钥不下发） | 认证中间件 + 令牌哈希存储 + httpOnly 会话 Cookie；服务端 secret store；安全响应头 |
| FR-47 | 常驻工作循环（7×24、按计划时间生成、不依赖客户端在线） | 调度器语义平移 + 触发去重持久化 + 容器时区语义；停机诚实代价 |
| FR-48 | 桌面客户端远程模式（可选、后置） | 传输对等基建的第二消费者；连接状态机与单事实源守卫 |

**隐性需求（决策 #19 边界条件）：**
- **云脑单源**：不做双引擎同步层——云端实例是唯一事实源时，桌面/浏览器一律是客户端
- **桌面零回归**：硬边界，抽取必须分步可验证，每步桌面测试全绿
- **自托管单用户**：无注册体系、无多租户、无计费——一切"以防万一"的多租户抽象都是违约

**非功能需求：**
- 安全 = 公网端点最小攻击面：全端点认证（除 healthz 与静态资源）、TLS、认证失败不泄露信息
- 运维 = 服务器重启自动恢复（数据/配置无损）、healthz、结构化日志、$5-10/月单 VPS 可承载
- 数据主权 = 复用既有导出/导入闭环；官方零持有零落地

**Scale & Complexity:**
- Primary domain: 双宿主系统（无头引擎 crate + Tauri 桌面壳 + axum server binary + 浏览器客户端）
- Complexity level: Medium-High —— 复杂度不在新代码量，在"改动既有引擎代码而不改变桌面行为"的抽取纪律
- 预计新增架构组件：engine crate、server binary、传输抽象层、认证/事件桥——约 6~8 个

### Technical Constraints & Dependencies

| 约束 | 来源 | 影响 |
|------|------|------|
| 引擎-Tauri 耦合现状 | 代码验证（2026-09-17，双评审复核） | 16 个 service 文件引用 tauri::（事件发射/任务派生/托管状态/事件监听四类）；commands/chat.rs（1092 行）持有 StreamingState、CancelTokens、OpencodeSessions、OnboardingConversations、MemoryExtractionState、OpencodeMcpScopeLock 六组托管状态——命令层并非纯薄层，抽取的真正工作量在此 |
| 桌面零回归 | 决策 #19 / 本章硬边界 | 每步抽取以 `npm run test:all` + tests/e2e 全绿为收口条件 |
| 单事实源 | FR-44 ASSUMPTION | 远程模式与桌面本地模式互斥；不出现任何合并/同步路径 |
| 仓库根禁建 Cargo workspace | 伴侣章节反模式 | engine crate 走 crates/ 目录 path 依赖先例（companion-proto） |
| 浏览器 EventSource 不能带 Authorization 头 | Web 平台事实 | SSE 通道的会话凭证必须走 Cookie（httpOnly）或查询串——选 Cookie |
| HTTP/1.1 下 SSE 每浏览器每域名 6 连接上限 | MDN 平台事实 | Caddy h2 场景无碍；用户既有反代若为 HTTP/1.1，多标签+XHR 会触顶——部署文档须提示"反代需支持 HTTP/2 或限制并发标签" |
| 容器时区默认 UTC | Docker 事实 | FR-47 需要 TZ 环境变量语义 + tzdata 进镜像 |

### Cross-Cutting Concerns Identified

1. **引擎宿主无关化** — services 对 Tauri 的耦合收敛为四类：事件发射（AppHandle.emit）、任务派生（async_runtime::spawn，含 setup 同步入口的 runtime 上下文问题）、托管状态（app.manage）、**事件监听（AppHandle.listen——companion_dispatch.rs:707 与 companion_snapshot.rs:796/803 对 `llm:stream` 的 Rust 侧消费，为手机伴侣流式镜像服务；裁决：这两个 service 留桌面壳不迁 engine，见 ①）**
2. **双传输协议对等** — command 面（invoke ⇄ HTTP）与事件面（Tauri Event ⇄ SSE）的对等契约与验证
3. **密钥边界** — keyring 仅桌面；服务端 env/0600 明文文件（见 ④）；浏览器只见过 api_key_ref
4. **时间语义** — chrono::Local 平移到容器 TZ；触发去重从内存态升级为持久化
5. **公网安全面** — 认证、TLS、CSP/CORS、浏览器存储策略

## Incremental Starter Template Evaluation — 云端托管版（FR-44～FR-48）

### Primary Technology Domain

双宿主系统：无头引擎 crate（既有 Rust 代码平移）+ 桌面 Tauri 壳（现有 egosync-app 瘦身）+ 自建 server binary（axum）+ 浏览器客户端（同一 React 应用双目标）

### Setup Strategy

**Manual/Minimal Init（抽取式，非新建式）** —— 无任何脚手架能"新建"出云端版；全部价值在于把既有引擎代码的外壳换掉。三个子项目各自最小起步：

| 子项目 | 初始化方式 | 理由 |
|--------|-----------|------|
| `crates/egosync-engine/` | `cargo new --lib` + 逐模块平移 | companion-proto 的 path 依赖先例；无 workspace |
| `server/` | `cargo new` + axum 最小骨架 | 与 relay-server 同构（axum 0.8 + tokio 1） |
| 前端 | 现有 src/ 增量加传输抽象层 | 零 UI 重写（决策 #19 的复用前提） |

### Initialization Commands

```bash
# 引擎 crate：
cd crates && cargo new egosync-engine --lib
# server：
cargo new server
# 依赖：axum 0.8（SSE 为内建能力，ws feature 预留）+ tokio 1 + sqlx 0.8 + path 依赖 ../crates/egosync-engine
# 部署：多阶段 Dockerfile（web 构建 → server 构建 → runtime 含 opencode 与 tzdata）→ docker compose up
```

### Architectural Decisions Provided by Starter

**Language & Runtime:** Rust edition 2021 + tokio（与桌面/中继同一异步运行时心智）；axum 0.8（与 relay-server 同版本）
**Build Tooling:** Cargo ×3（互相 path 依赖，无 workspace）；Vite 单一构建产物双宿主复用（运行时探测宿主）
**Testing Framework:** cargo test（engine 内聚）；传输对等测试套件（Epic 15.5）；wdio 增加 web 模式（驱动浏览器连 server）
**Code Organization:**

```
EgoSync/
├── crates/egosync-engine/   # [N] 无头引擎（唯一业务逻辑所在）
├── egosync-app/src-tauri/   # [M] 瘦身为 Tauri 壳（command 注册 + 桌面适配器）
├── egosync-app/src/         # [M] 加 transport 抽象（UI 零重写）
└── server/                  # [N] axum server binary + Docker
```

**否决记录：**
- ~~根 Cargo workspace 统一构建~~ — 伴侣章节已裁决禁止（扰动基线构建行为）；path 依赖足够
- ~~Tauri 2 headless/server 模式~~ — 仍绑定 Tauri 运行时与壳概念，公网面失控；axum 独立二进制更无聊更可控
- ~~server 内置 rustls TLS 监听~~ — 自托管用户域名/证书形态多样，反代终结（compose 内置 Caddy）是更无聊的路径；server 进程内只讲 HTTP（FR-46 允许"内置或反向代理终结"）
- ~~GraphQL / 双套 REST 资源建模~~ — 120 个 command 的 1:1 机械映射胜过任何手工资源模型（防映射表漂移）
- ~~WebSocket 双向事件通道~~ — 既有引擎事件全部单向（Rust→UI）；`skill-scope-updated` 是前端→前端事件（代码验证：Rust 侧无监听），浏览器端进程内消化——SSE 单向通道语义完全对齐，V1 不引入 WS

## Incremental Core Architectural Decisions — 云端托管版（FR-44～FR-48）

**本章范式：「同一引擎、双宿主」（single engine, dual host）** —— 全部业务逻辑收敛于无头引擎 crate；桌面 Tauri 壳与云端 axum server 都是薄适配层；宿主差异只允许存在于四处接缝（EngineEvents / SecretStore / sidecar 路径注入 / **runtime Handle 注入**——tauri setup 闭包无 runtime 上下文，同步启动入口必须由宿主传入 Handle）与 command 注册表。任何"云端专属业务逻辑"都是对本范式的违约。

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**

| # | 决策 | 裁决 | 理由 |
|---|------|------|------|
| 1 | 引擎无头化形态 | `crates/egosync-engine` 承载全部业务逻辑（db/models/llm/error/services/ChatSessionRegistry/migrations）；Tauri 壳与 server 都是薄适配层 | 双宿主共用同一引擎是决策 #19 全部增量价值的最小实现；path 依赖沿用 companion-proto 先例 |
| 2 | 事件解耦 | `EngineEvents` trait：`emit(event: &str, payload: serde_json::Value)`；桌面实现=TauriEventBus（转发 app_handle.emit），服务端实现=SseEventBus（tokio broadcast 扇出）；**events.rs 事件名常量是唯一发射源**（现状事件名为 Rust/TS 两侧散落字面量，各 ≥9/≥8 处——收编为常量后禁止再出现字面量发射）；TS 侧事件名与 payload 类型构建期从 events.rs 生成 | AppHandle 耦合收敛到单一接缝；payload 定为 Value 是刻意的——泛型方法使 trait 不可对象安全（非显而易见的坑，落死）；发射源唯一化使"事件形状一致"测试可枚举全集 |
| 3 | 任务派生 | 三个随 service 迁入 engine 的 lib.rs setup 同步启动入口（spawn_scheduler / 两个 spawn_hourly_watch）改为接收宿主注入的 `tokio::runtime::Handle`；service 层任务体内的 `tauri::async_runtime::spawn` → `tokio::spawn`（任务体内已有 runtime 上下文，安全）；register_stream_mirror 所在的 companion_dispatch 留桌面壳，其 spawn 不变 | **tauri setup 闭包在主线程事件循环中同步执行、无 runtime 上下文**（tauri 2.11.5 源码证实；本项目 companion_dispatch.rs 注释即记录"裸 tokio::spawn 在此会 panic 'there is no reactor running'——v0.1.6-alpha.1 启动即退出的根因"）；tauri::async_runtime 经全局单例可从任意线程调用，裸 tokio::spawn 不能——替换非"无行为变更的同义改写"，需第四条接缝 |
| 4 | API 面 | 单一 `POST /api/cmd/{command_name}`（camelCase 参数体、返回值原样 JSON）+ `GET /api/events`（SSE）+ 认证/setup/export/healthz 少量辅助端点；**注册表条目携带参数 schema**（Tauri command 签名经过程宏导出 JSON Schema，HTTP handler 据此驱动反序列化——`State<'_, T>` 注入参数与客户端参数的区分由 schema 标注，禁止 B 侧人肉阅读 120 个签名） | 与 Tauri command 一一对应靠"同一注册表"而非手工路由表：command 清单从 lib.rs generate_handler 抽出为共享清单，桌面注册与 server 路由同源生成；注册表以**构建期 JSON 工件**（commands.json：名字+参数 schema+capability）同时供 vitest 与 server 测试消费——对等测试用例由工件**全量生成**（禁止抽样与手抄清单） |
| 5 | 认证 | 单用户令牌：`EGOSYNC_TOKEN` env 预置或首访 `/api/setup` 设置；服务端只存 Argon2id 哈希；登录换 httpOnly SameSite=Strict 会话 Cookie；SSE 用 Cookie；**16.1 落地补全**：登录 Cookie 加 `Max-Age=2592000`（30 天持久，用户裁决 B 方案——关浏览器重开免重登）；`POST /api/auth/logout` 幂等删会话行 + 过期 Cookie；**env/setup 优先级冻结**：env 存在 ⇒ /api/setup 拒绝（库内哈希忽略）、login 仅常时比对 env；env 不存在 ⇒ 以库内 Argon2id 哈希为准；两态切换须重启进程，已发 Cookie 不随切换失效 | 无注册体系（FR-46）；EventSource 不能带 Authorization 头（平台约束）倒逼 Cookie；常时比较 + 认证失败统一 401 不泄露信息；优先级落死消除"任一通过即可"的 fail-open 读法；**passkey 取舍登记**：PRD FR-46"令牌/passkey"为或然措辞，本章裁决 V1 仅令牌，WebAuthn 延后 V2 |
| 6 | 命令能力门控 | command 清单带 capability 标记（desktop-only：4 个 rfd 对话框类 command + 7 个 companion_* command（云端实例无桌面伴侣连接语义，裁决 C）+ **data_export/data_import 两个 command（桌面版落点=用户经 rfd 选取的目录/文件，属桌面交互链；云端的导出/导入走 /api/export、/api/import 专用 HTTP 流端点，复用 data_export service 的纯逻辑——同名能力双落点，注册表内 command 与 HTTP 端点各自登记）**；web-ok：其余）；server 物理排除 desktop-only，UI 按 capability 门控入口 | `chat_pick_working_directory` 等在无头容器无意义且 rfd 会失败——从注册表源头排除，而非运行时报错 |
| 7 | 触发去重持久化 | FR-47 硬化：调度器内存去重 map（last_triggered_map / last_briefing_trigger_date / last_review_trigger_week，代码验证三处均为循环内局部变量）升级为 egosync.db 持久化（`scheduler_triggers` 小表）；**大石头保护除外**——其现状已是 DB 持久化（bigrock_protection.rs 读表内 last_reminded_at，代码验证），本决策对它是"替换入统一表"而非新增，含数据迁移 | 7×24 常驻 + 升级重启场景下，同分钟重启会重复触发晨间简报（现状内存 map 丢失）——桌面低频重启未暴露，云端必须补 |
| 8 | 时区语义 | 容器 TZ 环境变量（compose 文档化 `TZ=Asia/Shanghai` 示例）+ 镜像装 tzdata；scheduler 的 chrono::Local 语义零改动 | 比给 DB 加用户时区配置更无聊；Local 在 Linux 读 TZ env，行为可预期 |

**Important Decisions (Shape Architecture):**
- 传输选择：单一 Vite 构建产物 + 运行时探测宿主（`window.__TAURI_INTERNALS__` 存在 ⇒ TauriTransport，否则 HttpTransport）——不做双构建配置
- services/*.ts 改造：import 从 `@tauri-apps/api/core` 换 `src/transport/`（每文件一行级改动，函数签名不变）
- `useTauriEvent` → `useEngineEvent`（内部走 transport 事件通道；Tauri 分支行为不变；迁移完成后旧 hook 退役）
- opencode 进 server 镜像：sidecar.rs 生命周期管理原样平移，二进制路径经 `EGOSYNC_OPENCODE_PATH` 注入，agent_bridge 协议零改动（127.0.0.1:port 在同容器网络命名空间成立）
- 备份恢复：卷级（停容器复制卷）+ 逻辑级（复用 data_export service 的 export_all/import_all 纯逻辑，经 /api/export、/api/import HTTP 流——与桌面版 command 的目录落点是同一能力的双落点，见决策 #6）双路径；不新造备份机制

**Deferred Decisions (Post-V1):**
- FR-48 桌面远程模式（第二阶段，状态机已裁决见"开放问题裁决 B"）
- Android 直连云端实例（开放问题裁决 C：远期形态，帧协议收敛方向已定）
- WS 双向通道、多实例横向扩展、官方托管 SaaS（Non-Goal 不变）

### ①引擎无头化抽取（FR-44 前置）

**耦合点清单（代码验证 2026-09-17，复核 2026-09-17 评审）：**
- 事件发射：16 个 service 文件经 `AppHandle.emit` 发事件（agent_engine.rs 耦合最深，22 处）
- 任务派生：service 层 9 处/8 文件 `tauri::async_runtime::spawn`；**全代码库共 19 处/13 文件**（commands 层 5 处 + lib.rs 5 处——后者含 4 个同步启动入口，是 runtime Handle 接缝的由来；commands/lib.rs 的 spawn 随宿主层留壳，不属 engine 抽取范围）
- 托管状态：lib.rs `app.manage()` 16 处；其中 commands/chat.rs 持有**六组**会话状态（StreamingState=Arc<Mutex<HashSet>>、CancelTokens、OpencodeSessions、OnboardingConversations、MemoryExtractionState、OpencodeMcpScopeLock）
- 事件监听（第四类）：companion_dispatch.rs:707 与 companion_snapshot.rs:796/803 经 `app_handle.listen("llm:stream")` 消费引擎事件（Rust→Rust 镜像，供手机伴侣流式转发）
- 对话框：4 个 rfd 原生对话框 command（chat_pick_working_directory、skill_pick_custom_directory、data 侧导出目录/导入文件选取）——desktop-only
- 无耦合部分：db/、models/、llm/、error.rs、agent_bridge（纯 reqwest 客户端）、sidecar（裸 tokio::process::Command，非 Tauri sidecar API）、data_export 等可直接平移

**companion 服务归属裁决：全部四个 companion_* service（pairing/connection/dispatch/snapshot）留桌面壳，不迁 engine。** 理由：①dispatch/snapshot 的耦合点恰是第四类 listen（EngineEvents 为 emit-only，加 subscribe 面只为迁文件——反向妥协）；companion_connection 虽仅 emit 耦合，但其功能（NSD 广播、WS 监听）为桌面宿主专属，云端无消费者，不值得为迁移而改造；companion_pairing 本无 tauri 耦合，同为桌面专属功能随行留壳；②手机伴侣只连桌面引擎（裁决 C：V1 不动任何伴侣代码），这些 service 在云端宿主无消费者；③与 7 个 companion_* command 列 desktop-only、paired_devices 云端闲置自洽。代价登记：若裁决 C 的远期形态（Android 直连云端）落地，流式镜像需在 engine 侧重建——届时 EngineEvents 的 SseEventBus 天然可加内部订阅（broadcast 本就支持），不是单向门。

**抽取顺序（绞杀者模式，每步收口=桌面全绿）：**
1. `crates/egosync-engine` 骨架 + 无 Tauri 依赖模块平移（db/、models/、llm/、error.rs、agent_bridge、data_export 等纯逻辑）——纯代码搬移，src-tauri 以 `pub use` 回引保持既有路径引用不断
2. `EngineEvents` trait + TauriEventBus 落地；按域迁移 13 个耦合 service（16 个名单减去留壳的 companion_dispatch/companion_snapshot/companion_connection；chat/agent_engine 域最重，单独成步收口），lib.rs 构造处注入——事件发射点从强类型 payload（`emit("llm:stream", StreamPayload{..})`）机械改写为"events.rs 常量名 + Value"是抽取期**唯一豁免规则三的改写**（显式豁免：Value 签名与常量源是本章裁决，不改写即违约；现状发射点 20+ 处传强类型，代码验证）；改写前后桌面 e2e 必须全绿
3. 任务派生双轨替换：①service 层任务体内的 `tauri::async_runtime::spawn` → `tokio::spawn`（任务体内已有 runtime 上下文，安全；现状已混用）；②三个迁入 engine 的同步启动入口（spawn_scheduler / 两个 spawn_hourly_watch；register_stream_mirror 随 companion_dispatch 留壳不变）改为接收 `tokio::runtime::Handle` 参数——桌面壳传 `tauri::async_runtime::handle()`，server 传 `tokio::runtime::Handle::current()`。**验收必须含桌面启动路径 e2e 断言**（setup → 各启动入口不 panic——本项目 v0.1.6-alpha.1 曾因裸 spawn 在此崩溃，前车之鉴）
4. ChatSessionRegistry 抽取：**六组**会话状态从 commands/chat.rs 移入 engine（Arc 组合、无 Tauri 类型，含 OpencodeMcpScopeLock）；Tauri command 与 axum handler 都薄调用它；**特征测试先行**——动代码前先落 busy 互斥特征测试：同会话并发双发恰触发一次 LLM 调用、恰一条 busy 落库消息、busy 经 **Ok 通道**返回（现状契约，代码验证 chat.rs:301-309；companion_dispatch 以 `msg.role != "user"` 探测 busy——该隐式协议禁止破坏，busy 禁止改为 Err 通道）
5. command 清单抽为共享注册表（桌面 generate_handler 与 server 路由同源）

**回归兜底（硬边界）：**
- 每步完成后：`npm run test:all`（vitest + cargo test）+ `tests/e2e` 全量（wdio 桌面套件）全绿才进下一步——跳过任何一项即未完成（显式失败原则）
- vitest 既有 mock（@tauri-apps/api 模块）随 transport 抽象同步迁移到 mock transport——测试意图不变，mock 对象替换
- 迁移期间禁止任何"顺手重构"（规则三）；模块平移保持文件内容逐字节等价（import 路径除外；**唯一例外**=事件发射点的"强类型 payload→Value + 字面量→events.rs 常量"机械改写，见抽取顺序第 2 步）
- 空转防线补强（对抗性评审 F6）：busy 互斥与 companion role 探测协议在现状代码中**零测试覆盖**——"桌面全绿"对这些语义是空集，抽取时锁粒度被"细化"或 busy 改 Err 通道都不会红；Registry 抽取故事的第一交付物就是上述特征测试，特征测试不绿不得动 Registry

### ②server binary（axum）

**API 面（与 Tauri command 一一对应）：**

| 端点 | 方法 | 语义 | 对应 Tauri 机制 |
|------|------|------|----------------|
| `/api/cmd/{command}` | POST | 参数体 camelCase（与 invoke args 同构），返回值原样 JSON | `invoke()` |
| `/api/events` | GET (SSE) | 事件名=SSE event 字段，data=payload JSON（与 emit 同构） | `listen()` / Tauri Event |
| `/api/auth/status`、`/api/auth/login` | GET/POST | 初始化状态 / 令牌换会话 Cookie | 无对应（云端新增） |
| `/api/auth/logout` | POST | 删会话行 + 过期 Cookie（幂等 200；16.1 落地） | 无对应（云端新增） |
| `/api/setup` | POST | 首次初始化设置令牌（仅在无凭据时挂载） | 无对应 |
| `/api/export`、`/api/import` | POST | 复用 data_export 全量导出/导入（下载/上传流） | data_* command 的文件落点改为 HTTP 流 |
| `/healthz` | GET | 存活探针（无认证、无数据） | 无 |

**技术要点：**
- axum 0.8 + tokio 1；业务路由由共享 command 注册表生成，非手工维护
- SSE：axum 内建 `axum::response::sse` + KeepAlive（30s 注释心跳）；事件经 tokio broadcast 扇出给全部在线客户端——**Tauri emit 本就是全局广播，多浏览器=多窗口，语义平移而非新设计**
- 慢客户端：broadcast 滞后即断开该连接（EventSource 自动重连 + UI 经 command 全量补齐状态）——对齐伴侣"重连即快照、不做历史回放"哲学
- 限流（收敛到认证面）：`/api/auth/*` 与 `/api/setup` 按 IP 5 次/分钟；业务端点 V1 不限流（自托管单用户，无滥用多租户面）
- TLS：server 进程只讲 HTTP；compose 内置 Caddy 服务（自动 HTTPS，域名自备）或文档化对接用户既有反代；Caddy 对 `text/event-stream` 响应默认即逐写刷新（该类型下 flush_interval 被官方忽略），Caddyfile 仍显式配 `flush_interval -1` 作冗余保险
- 静态资源：server 承载 web dist（axum 静态服务 + SPA 回退 index.html；16.1 落地细节：目录经 env `EGOSYNC_STATIC_DIR` 注入、缺省回退 `../egosync-app/dist`、皆缺则 API-only 警告运行；`/api/*` 未知路径在 fallback 守卫为 JSON 404 不落 SPA 面；index.html 显式路由直出并统一 `Cache-Control: no-cache`——hash 资产可长缓存，index 引用新鲜度是重部署白屏的防线）；Caddy 只做 TLS 终结与代理
- **错误语义冻结（F4）**：全部 AppError variant 一律 `200 + 原样 serde JSON`（单键 map 形状 `{"DbError": "..."}`，14 个 variant 不逐个分类——与 invoke 错误通道同构）；非 200 白名单仅四类传输层错误：401（认证失败）、429（限流）、404（路由/命令不存在）、5xx（进程级故障：panic/启动失败）；对等测试含**错误路径黄金用例**（每 variant 断言双通道解包所得错误对象逐字节同构）
- **请求边界（F11）**：请求体上限 50MB 写进注册表契约（axum DefaultBodyLimit 默认 2MB 会掐断大附件——与 Tauri invoke 无限制的差异显式登记）；业务端点禁用响应超时（chat_send_message 分钟级挂起是正常态），仅保留 idle 超时；Caddyfile 样例同时含 `request_body max_size=50MB` 且不设 write timeout（与 server 侧上限对齐，防代理层第二道暗限）

### ③前端传输对等

```
React UI（同一份组件体系，视觉零分叉）
    │ services/*.ts（函数签名不变）
    ▼
src/transport/ ── Transport 接口：invoke<T>(cmd, args) / on(event, cb) / capabilities / onConnectionStateChange
    ├── TauriTransport：@tauri-apps/api invoke + listen（行为与现状逐字节一致）
    └── HttpTransport：fetch POST /api/cmd/{cmd} + EventSource /api/events
运行时探测：window.__TAURI_INTERNALS__ 存在 ⇒ Tauri；否则 Http
```

- 单一 Vite 构建产物双宿主复用：tauri.conf 的 frontendDist 与 server 镜像内嵌的是同一份 dist
- `skill-scope-updated`（唯一前端→前端事件，代码验证 Rust 侧无监听）：Tauri 分支保持 emit/listen；浏览器分支进程内 EventEmitter——不进传输契约
- 断线重连：EventSource 原生自动重连；连接状态经 healthz 探测呈现（Connecting/Online/Reconnecting，伴侣 TransportStatus 的同构简化）
- 命令能力门控：`transport.capabilities`（来自共享清单）驱动 UI 入口显隐——web 端不出现"选工作目录"等桌面专属入口
- 桌面壳耦合组件：`TitleBar.tsx` 与 `App.tsx` 直接引 `@tauri-apps/api/window`（getCurrentWindow，窗口控制，代码验证）——web 构建按同一宿主探测门控：Tauri 下渲染原样，浏览器下窗口控制区退化为普通标题栏（布局差异属 UX 阶段细化）
- **重连补齐协议（F5，双传输对等契约组成部分）**：HttpTransport 在 SSE onopen-after-error 时触发重连信号（TauriTransport 恒 Online 不触发）；补齐由 **transport 层**（非 UI 层）执行——重放冻结的**只读 query command 白名单**（会话列表/角色/任务/通知等查询类；写入类 command 一律不重放，防 `notification_ack` 类副作用重放）；白名单进 commands.json 工件；UI 只订阅连接状态呈现
- **capabilities 同源（F9）**：capabilities.ts 由构建期从 engine capabilities.rs 生成（禁止手写双清单——漂移的产物是 web 端入口可见但请求 404，用户看到"网络错误"）；对等测试断言"TS 能力清单 == 引擎注册表"

### ④密钥管理

| 形态 | LLM Key 存储 | 读取方 | 浏览器可见性 |
|------|--------------|--------|--------------|
| 桌面版 | 系统 keyring（不变） | 引擎 | N/A（无浏览器） |
| 云端版 | env（`EGOSYNC_SECRET_{api_key_ref}`，原样区分大小写）或数据卷文件 /data/secrets.json（**0600 权限保护的明文**——"加密"措辞废除：加密密钥来源无无聊答案，不造方案；安全边界与 env 同级），实现为 SecretStore trait 的服务端适配器 | 引擎 | **永不可见** |

- **键映射冻结（F3，代码验证 llm_config.rs:120）**：`api_key_ref = llm_{uuid}_api_key`（全小写，UUID 运行时生成）；env 变量名 = `EGOSYNC_SECRET_{api_key_ref}` **原样区分大小写拼接**（ref 段不做任何大小写转换——"前缀大写"等读法全部非法，落死一处）
- **env 定位收窄（评审复核）**：UUID 键用户无法预知——env 对用户创建的 LLM Key 实际不可行，定位为**固定名 secret 的引导通道**；用户创建的 LLM Key 只经 secrets.json（save 路径）。读取顺序 = **文件优先、env 兜底**——UI 重录即时生效，env 不构成运行时遮蔽源（不存在"重录后仍被 env 旧值遮蔽且无测试可发现"的静默失效路径）
- **迁移可达性探测**：导入完成后对每个 api_key_ref 探测 SecretStore 可达性，缺失时返回指明重录路径的结构化错误（现状 task_classifier/mission_inferrer/llm_config 三处各自拼 ValidationError 文案——收敛为一处）

- 结构性保证（代码验证）：`LlmConfig` 出参只有 `api_key_ref`（keyring 引用）；key 仅在 Create/Update 入参与 test_connection 用户主动输入时出现——"key 不下发"在现有模型中已成立，云端版只需守住不破坏（API 响应形状测试断言无 key 字段）
- secret_store 抽 trait：keyring 实现留桌面壳；server 实现为 env/文件适配器（读取顺序与写入语义见上）
- 桌面→云端迁移的用户路径：配置经导出/导入平移，LLM Key 需在云端重录一次（keyring 不导出——安全边界如此，文档明示）

### ⑤opencode 服务端化

- sidecar.rs 平移进 engine，生命周期管理（spawn/健康检查/退出清理/watchdog）零改动；二进制路径从 tauri resources 解析改为路径注入（桌面壳继续传 resources 路径，server 传 `EGOSYNC_OPENCODE_PATH`——同一注入点两个值）
- agent_bridge 协议零改动：reqwest → `http://127.0.0.1:{port}`，同容器内成立
- Dockerfile：opencode 二进制随镜像分发，版本 pin 在 Dockerfile（与 engine 版本同 tag 发布）
- **与提案 §4.2-7 措辞冲突的显式裁决**：提案写"docker compose：server+opencode+WEB 静态"三件并列；本章裁决 **opencode 并入 server 镜像**（保 sidecar 生命周期代码零改动），compose 实际两服务（egosync-server + caddy），WEB 静态由 server 内嵌服务。理由：三容器方案要求把进程管理让渡给 compose restart 策略并改造 agent_bridge 连接目标——改动更大的"分离"没有换来任何能力；WEB 静态与 opencode 的物理位置不影响提案的运维意图（一键部署、单卷数据）

### ⑥数据与备份

- 双 SQLite（egosync.db + conversations.db）挂命名卷 `egosync-data`；WAL 模式单容器单写入者，并发语义与桌面同构（引擎内既有互斥平移）
- 备份双路径：①逻辑级——复用 data_export `export_all`（经 /api/export 下载）/`import_all`（上传恢复），单用户数据主权闭环；②卷级——停容器后复制卷（文档化，适合升级前快照）；不新造备份机制
- 服务器重启自动恢复：compose `restart: unless-stopped` + 引擎启动幂等（migrations 幂等既有）；WAL 卷快照必须停容器或走逻辑级导出（运行中直接复制 WAL 卷有一致性风险——文档明示）
- 秘密文件（secrets.json）与数据卷同卷，0600；不进镜像、不进 git

### ⑦部署（docker compose）

```yaml
# server/docker-compose.yml（示意）
services:
  egosync-server:
    image: egosync/server:tag        # 含 engine + opencode + web dist + tzdata
    volumes: [egosync-data:/data]
    environment:
      - TZ=Asia/Shanghai
      - EGOSYNC_SECRET_llm_9f8e7d6a_api_key=...   # = EGOSYNC_SECRET_{api_key_ref}，ref 形态与映射规则见 ④
    restart: unless-stopped
    healthcheck: { test: ["CMD", "wget", "-q", "http://localhost:8080/healthz"] }
  caddy:                             # 可选：自动 HTTPS（域名自备）；已有反代的用户可去除此服务
    image: caddy:2
    ports: ["443:443", "80:80"]
    volumes: [./Caddyfile:/etc/caddy/Caddyfile]
volumes: { egosync-data: {} }
```

- healthz：两级——存活（进程/端口）与深度（DB 连通、opencode 存活，`?deep=1`）；compose 用存活级，升级前巡检用深度级
- 结构化日志：tracing JSON 到 stdout（`RUST_LOG` 控制），与 relay-server 同范式；密钥与事件 payload 明文永不入日志（零知识纪律继承）
- CI：`server-docker.yml` 多阶段镜像构建发布（实现期故事 17.3）

### ⑧安全威胁模型

| 威胁 | 缓解 |
|------|------|
| 令牌暴力破解 | Argon2id 哈希存储、常时比较、认证/setup 端点 IP 限流 5/min、失败统一 401 不泄露用户存在性 |
| 中间人窃听 | TLS 强制（Caddy 自动 HTTPS 或用户既有反代）；healthz 之外全端点拒绝明文部署——文档明示 |
| XSS 窃取会话/数据 | httpOnly Cookie（JS 不可读）；CSP（16.1 落地实文）：`default-src 'self'; script-src 'self' 'sha256-…'`（hash-source 放行 index.html 内联防 FOUC 主题脚本，byte 对 byte 契约测试守门）；`style-src 'self' 'unsafe-inline' https://fonts.googleapis.com` + `font-src 'self' https://fonts.gstatic.com`（与桌面 tauri csp:null 同链放行 Google Fonts——视觉零分叉，仅此两第三方域）【2026-09-21 Story 17.1 人工裁决修订（自托管字体）：两 Google 字体域撤除——字体改经 @fontsource-variable npm 包自托管随 dist 分发（woff2 本地命中，110 分片），`font-src`/`style-src` 回归 `'self'`，CSP 现为零第三方域；契约测试同步收紧为「全 policy 不得含任何 http(s) 外链源」+ dist 产物 woff2 正向断言。桌面（tauri csp:null）与 web 仍同链本地加载，视觉零分叉不变】；`connect-src 'self'`；另加 `X-Content-Type-Options: nosniff`、`X-Frame-Options: DENY`、`Referrer-Policy: no-referrer` 全响应下发 |
| CSRF | Cookie SameSite=Strict + 全同源架构（静态/API/SSE 同域）；无跨源请求面 |
| 浏览器侧数据残留 | V1 明确策略：业务数据仅内存态（React state），不写 localStorage/IndexedDB；刷新=从服务端重取（FR-45"刷新恢复"由服务端持久化兜底）；登出清 Cookie |
| 密钥泄露至浏览器 | ④的结构性保证 + API 响应形状测试（断言无 key 字段） |
| 公网扫描/未知漏洞 | 攻击面=单进程 + 静态资源；无注册/多租户面；boring 栈（Rust+axum+SQLite）；升级路径=换镜像 tag |

- CORS：同源架构下无需跨源——CORS 中间件显式拒绝一切跨源请求（默认拒绝优于静默允许）
- 单用户缩小攻击面是设计选择而非偷懒：多租户的隔离/计费/合规是另一个量级的运营责任（决策 #19 后议）

### ⑨工作循环常驻化（FR-47）

- 语义平移：scheduler 循环、角色工作循环、晨间简报/周复盘生成逻辑零改动进 engine；桌面版 FR-10 行为不变（同一 engine，桌面壳只在应用运行时启动调度——宿主决定调度启动时机，引擎不知道自己跑在哪）
- 时区：`TZ` env + tzdata（决策 #8）；简报时间=容器时区时间（用户自托管即用户时区），compose 示例落死
- 去重持久化（决策 #7）：触发键（date+hhmm / week 维度）落 egosync.db——升级重启不再重复触发晨间简报
- 停机诚实代价（PRD ASSUMPTION 保持）：错过的时间窗不补跑（桌面同语义：关机错过晨间简报不回溯）；恢复后按计划继续
- 常驻生成的建议仍走"待确认→用户确认"（FR-11 边界不变）；通知在 WEB 端为应用内通知（无 Service Worker 推送，页面关闭即无通知；iOS Safari 普通浏览无 Notification API，16.4+ 仅限加主屏 PWA——降级语义与伴侣 FR-42 一致）
- **时间源三分表（评审 F7 落死，防"顺手一致化"）**：①持久化时间戳一律 `chrono_now_pub` 的 UTC RFC3339 串（现状，代码验证 db/settings.rs:144——**禁止**借 TZ 平移顺手改 Local，桌面/云端行为同构）；②调度判定一律容器 `Local`（决策 #8）；③前端渲染一律按浏览器 TZ 解析 UTC 串（云端浏览器 TZ ≠ 容器 TZ 是常态——海外 VPS + 国内访问）；简报文本内日期为容器 Local 拼好的字符串，与前端格式化并存是现状语义，不在抽取范围
- `scheduler_triggers` 键含时区维度（或文档明示：变更 TZ 后首个周期可能重复/跳过一次触发——date+hhmm 键跨时区会错位）
- **桌面侧边缘行为变化加注（诚实登记）**：决策 #7 的去重持久化是引擎级变更，桌面版同样获得——桌面"重启窗口内不再重复触发"（现状：重启后同分钟可再次触发）是行为变化而非回归，收口时不得误判

### Data Architecture（增量）

- 云端版引擎同库同 schema，仅新增 `scheduler_triggers` 触发去重小表（migration 编号顺延）
- 双库均挂 `/data` 卷；`paired_devices` 表在云端实例中闲置（无桌面伴侣连接）——不迁移不清理，保持 schema 单一
- 无任何跨实例复制/同步路径——单源即无同步

### 开放问题裁决

**A. 多端并发会话语义（多浏览器同时在线）**

裁决：**广播 + 既有互斥平移，不引入会话管理协议**。
- 读：全并发（查询 command 无状态）
- 事件：SSE 全客户端广播（= Tauri emit 对多窗口的本有语义，非新设计）
- 同会话流式互斥：StreamingState 语义平移——流式期间第二个发送请求收到"我还在想上一个问题，请稍等片刻…"busy 消息（现状行为，代码验证），无论请求来自哪个浏览器标签
- 跨会话并发：各自独立 opencode 会话，互不干扰（现状语义）
- 设置/仪表盘并发编辑：单用户现实下令牌级 last-write-wins，不做字段级冲突解决——两台设备同时改同一设置是伪场景
- 不做：设备列表、会话踢出、多光标、presence——单用户产品无此需求（简单至上）

**B. 桌面客户端远程模式（FR-48）连接状态机**

裁决：**本地/远程互斥双态 + 显式切换，永不合并数据**。

```
LOCAL（默认）──用户在设置中配置远端并确认──▶ REMOTE_CONNECTING
LOCAL ◀──用户显式切回（本地引擎以最后本地数据重启）── REMOTE_ONLINE
REMOTE_CONNECTING ──远端连通+认证通过──▶ REMOTE_ONLINE
REMOTE_CONNECTING / REMOTE_ONLINE ──远端不可达──▶ REMOTE_OFFLINE
REMOTE_OFFLINE ──指数退避重连（1s→30s 封顶+抖动，伴侣同范式）──▶ REMOTE_CONNECTING
```

- LOCAL→REMOTE 守卫：本地引擎完整停机（sidecar 杀进程、调度器取消、连接池关闭）后才切传输——单事实源不变式；切换时提示"本地数据停留在切换时点，不会与云端合并"（诚实代价明示，FR-48"不触发任何数据合并"的架构落地）
- REMOTE 态桌面 = 一个浏览器等价物（复用裁决 A 的全部语义）；本地引擎待机不产生任何数据
- REMOTE_OFFLINE：无本地数据兜底（远程模式不落业务数据到磁盘），呈现重连 UI——与伴侣降级态同哲学，但无快照
- 第二阶段交付（FR-48 时序）；传输对等基建（Epic 15）是其唯一前置

**C. 与中继/伴侣协议的远期收敛（Android 直连云端实例）**

裁决：**确认为远期形态，帧协议是收敛契约，V1 不动任何伴侣代码**。
- 现状：手机伴侣 ↔ Noise XX 加密帧 ↔ 桌面引擎（直连/中继双承载）——Epic 12-14 照常，零改动
- 收敛方向：云端实例作为帧协议的第三种承载（WSS + 令牌认证替代 Noise 配对层）——SNAPSHOT/COMMAND/STATE_DELTA 等帧类型与 schema 不变，安全层换（云端已有 TLS+单用户令牌，叠 Noise 无增益）
- 驱动力：云端版用户没有常开的桌面引擎可供配对——手机直连云是自然终点形态；但这是"云端版发布后"的独立立项，不进 Epic 15-17
- 旧章节关系显式登记（只追加不回改原则）：①伴侣章节 Deferred 中"云端数据同步"仍是 Non-Goal（双引擎同步依旧否决）；该 Deferred 里的"托管"一词，语义已被决策 #19 重新定义为"自托管单实例"——以本文与决策日志 #19 为准。②伴侣章节约束表"工作循环仅桌面运行时执行……不存在云端代替桌面跑循环"（FR-10 在当时拓扑下成立）——云端形态下由 FR-47 取代该假设（宿主决定调度启动时机，见 ⑨），桌面版 FR-10 行为不变。③两处以本文为准，不回改旧章节

### Decision Impact Analysis

**Implementation Sequence（对齐 Epic 15 → {16, 17}）:**
1. engine crate 骨架 + 纯模块平移（Epic 15.1 前半）
2. EngineEvents/TauriEventBus + 耦合 service 逐域迁移（15.1 后半，chat/agent_engine 单列）
3. ChatSessionRegistry 抽取（15.1 收尾）
4. tokio::spawn 替换 + command 注册表共享（15.1 硬化）
5. server/ axum 骨架：cmd 路由 + SSE 总线 + healthz（15.2）
6. 前端 transport 抽象 + useEngineEvent + 运行时探测（15.3）
7. 传输对等测试套件（15.4：command 集合双通道一致 + 事件形状一致 + 响应无 key 字段断言）
8. WEB 认证/首访/响应式（16.1-16.4）
9. 触发去重持久化 + TZ 语义（FR-47 硬化，并入 16/17 之间）
10. Docker/compose/CI/备份恢复（17.1-17.4）
11. （后置）FR-48 桌面远程模式（16.5）

**Cross-Component Dependencies:**
- egosync-engine ← 被 src-tauri 与 server 共同 path 依赖（EventBus/SecretStore trait 先行冻结，最高优先级）
- 共享 command 注册表 ← 被桌面 generate_handler、server 路由、前端 capabilities、对等测试四方消费
- SseEventBus ← 依赖 EngineEvents；HttpTransport ← 依赖注册表生成的 URL 面
- data_export ← 备份 API 复用（文件落点改 HTTP 流）

## Implementation Patterns Addendum — 云端托管版（FR-44～FR-48）

### Pattern Categories Defined

基线 100 条规则 + 伴侣章节增量规则继续全量生效；本节只定义云端版引入的 **5 类新冲突点**，未提及处一律继承。

### Naming Patterns

**引擎 crate（crates/egosync-engine）：**
- 平移模块保持原文件名与目录结构（db/、models/、llm/、services/ 原样进 crate）——不借抽取改名
- 新增：`events.rs`（EngineEvents trait + 事件名常量）、`registry.rs`（ChatSessionRegistry）、`capabilities.rs`（command 清单与 desktop-only 标记）

**server/（对齐 relay-server 布局）：**
- `src/{main.rs, routes.rs（cmd 路由生成）, sse.rs, auth.rs, secret_store.rs, static_files.rs}`
- HTTP 辅助端点命名：`/api/{kebab-case}`；command 名在 URL 中保持 snake_case 原名（一一对应的可 grep 性优先于 REST 美观）

**前端：**
- `src/transport/{index.ts, types.ts, tauri.ts, http.ts, capabilities.ts}`；`useEngineEvent`（useTauriEvent 的传输无关继任者）

### Format Patterns

- command 参数/返回：camelCase JSON（与 Tauri invoke 惯例同构，serde rename_all 不变）
- SSE：`event: {事件名}` + `data: {payload JSON}`；心跳为注释行（30s）
- 环境变量：`EGOSYNC_` 前缀大写（EGOSYNC_TOKEN、EGOSYNC_OPENCODE_PATH、EGOSYNC_SECRET_{KEY}、EGOSYNC_DATA_DIR、EGOSYNC_STATIC_DIR（16.1：静态目录注入，缺省回退 `../egosync-app/dist`，皆缺则 API-only 警告运行））
- 事件名继续域前缀冒号风格（llm:stream、notification:new 等）；云端不新增事件名——服务端只是承载，不是事件源

### Communication Patterns

- **双传输对等契约**：同一 command 名、同一参数形状、同一返回形状、同一事件名/载荷——任何一侧单独改契约都是违约；变更必须双通道同步 + 对等测试更新
- command 面：`POST /api/cmd/{command}` 是唯一业务入口；禁止在 server 上新增"更 RESTful"的旁路端点（export/import 的 HTTP 流除外，其语义已在注册表登记）
- 错误语义：HTTP 状态码承载传输层错误（401/429/5xx）；业务错误 = 200 + AppError JSON（与 invoke 错误形状同构）——前端 transport 统一解包，两侧错误路径同构

### Process Patterns

- 重连退避：1s→30s 封顶 + 随机抖动（伴侣同范式）；SSE 由 EventSource 原生处理，应用层只管状态呈现
- 日志：tracing JSON；密钥与事件 payload 明文永不入日志（零知识纪律继承）；桌面壳沿用 tracing 现状
- 触发去重：调度器所有"每窗口一次"判断统一走 scheduler_triggers 持久化键——不允许多套去重机制并存（大石头保护现状 DB 去重随决策 #7 替换迁入，属显式迁移而非并存例外）

### Structure Patterns

**测试位置：**
- engine：随模块 `#[cfg(test)]`（平移时测试跟着文件走）+ `tests/` 集成测试（EventBus 桩、Registry 并发）
- server：`tests/`（路由生成完整性、认证、SSE 广播、能力门控排除 desktop-only）
- 传输对等：`egosync-app/src/transport/` 测试（双通道黄金用例：同一 service 函数集双通道跑，断言响应逐字节同构）
- WEB e2e：tests/e2e 增加 `web` 模式（wdio 驱动浏览器连本地 server，复用既有用例集）

### Enforcement Guidelines

**All AI Agents MUST:**
1. 引擎代码（crates/egosync-engine）禁止依赖 tauri——engine 的 Cargo.toml 不声明 tauri 依赖，物理不可能（CI 兜底）
2. 任何 command/事件契约变更必须双通道同步 + 对等测试更新
3. 抽取故事收口条件：桌面 `npm run test:all` + tests/e2e 全绿——跳过任何一项即未完成（显式失败原则）
4. 云端新增端点必须经共享注册表（禁止旁路 API）

**Anti-Patterns (禁止):**
- ❌ engine 内出现 tauri::（物理封禁）
- ❌ 为多租户/多用户预留任何抽象（单用户是规格不是阶段）
- ❌ 手工维护 120 条路由映射表（必须从注册表生成）
- ❌ 浏览器 localStorage/IndexedDB 缓存业务数据（V1 明确禁止）
- ❌ server 引入第二套日志/配置/错误范式
- ❌ 抽取期顺手重构被平移的代码（规则三全程生效）

## Project Structure Addendum — 云端托管版（FR-44～FR-48）

### Complete Project Directory Structure

标记：`[N]` 新增；`[M]` 修改现有；`[→]` 平移（原位置消失）；未标记处不动。

```text
EgoSync/
├── crates/
│   ├── companion-proto/                    # 既有不动
│   └── egosync-engine/                     # [N] 无头引擎
│       ├── Cargo.toml                      #   依赖 sqlx/tokio/reqwest/chrono；不含 tauri 与 keyring（经 trait 注入）
│       ├── migrations/                     # [→] 自 src-tauri/migrations 平移（新增 scheduler_triggers）
│       └── src/
│           ├── {db,models,llm,services}/…  # [→] 逐模块平移（含内联测试；companion_dispatch/companion_snapshot 留桌面壳）
│           ├── events.rs                   # [N] EngineEvents + 事件名常量
│           ├── registry.rs                 # [N] ChatSessionRegistry（六组会话状态）
│           └── capabilities.rs             # [N] command 清单 + desktop-only 标记
├── egosync-app/
│   ├── src/
│   │   ├── transport/                      # [N] 双传输（index/types/tauri/http/capabilities）
│   │   ├── services/*.ts                   # [M] import 换 transport（签名不变）
│   │   └── hooks/useEngineEvent.ts         # [N]（useTauriEvent 迁移完成后退役）
│   ├── vite.config.ts                      # [M] 仅按需加 dev proxy（/api → localhost:8080）
│   └── src-tauri/
│       ├── Cargo.toml                      # [M] path 依赖 egosync-engine
│       ├── src/lib.rs                      # [M] 注册表驱动 generate_handler + TauriEventBus 注入 + Handle 注入
│       └── src/{commands/, secret_store_keyring.rs, sidecar_path.rs}  # [M] 薄化；rfd 对话框留壳；companion_dispatch/companion_snapshot 留壳（见 ①）
├── server/                                 # [N] axum server binary
│   ├── Cargo.toml                          #   path 依赖 egosync-engine
│   ├── Dockerfile                          #   多阶段：web dist → server → runtime（opencode + tzdata）
│   ├── docker-compose.yml                  #   egosync-server + caddy（可选）+ egosync-data 卷
│   ├── Caddyfile                           #   反代 + flush_interval -1（SSE 冗余保险）+ request_body 50MB
│   └── src/{main,routes,sse,auth,secret_store,static_files}.rs
└── .github/workflows/server-docker.yml     # [N] 镜像构建发布
```

### Architectural Boundaries

**五条硬边界：**

1. **引擎边界**：crates/egosync-engine 无 tauri 依赖（Cargo.toml 物理封禁）；事件只经 EngineEvents，密钥只经 SecretStore trait，opencode 路径只经注入——宿主差异到此为止
2. **API 边界**：server 唯一业务入口 `/api/cmd/{command}` + `/api/events`；端点集合由共享注册表生成；desktop-only command 物理不路由
3. **密钥边界**：LLM Key 只存在于 SecretStore 实现之后（keyring / env / 0600 明文文件，见 ④）；API 出参形状测试断言无 key 字段
4. **浏览器边界**：不落业务数据（仅内存态）；不持有 LLM Key；能力门控外的入口不可见
5. **数据边界**：单事实源不变式——远程模式/多客户端只是视图；无任何合并/同步代码路径存在

**数据边界（云端形态）：**

| 数据 | 位置 | 规则 |
|------|------|------|
| 业务双库 | egosync-data 卷（/data） | WAL；单容器单写者；卷级（停机）+逻辑级双备份 |
| 访问令牌哈希 | egosync.db settings | Argon2id；只增不外发 |
| LLM Key | env 或 /data/secrets.json（0600） | 不进镜像/git/日志/浏览器 |
| 会话 Cookie | 浏览器（httpOnly） | 登出即清；SameSite=Strict |
| opencode 工作区 | /data 下 | 随卷备份；子进程生命周期由 sidecar 管 |
| 浏览器业务数据 | 仅内存 | 刷新即从服务端重取 |

### Requirements to Structure Mapping

| FR | 归属 |
|----|------|
| FR-44 | egosync-engine + server/ + compose |
| FR-45 | src/transport/ + 同一 dist 双宿主 + useEngineEvent + 能力门控 |
| FR-46 | auth.rs + Caddyfile + SecretStore 服务端实现 + CSP 中间件 |
| FR-47 | scheduler 平移 + scheduler_triggers + TZ 语义 |
| FR-48（后置） | 桌面壳传输切换 + 状态机（第二阶段） |

### Integration Points & Data Flow

```text
[命令] 浏览器 UI → transport(http) → POST /api/cmd/{command} → routes → ChatSessionRegistry/services → JSON 返回
[事件] engine emit → SseEventBus → broadcast 扇出 → 各浏览器 SSE → useEngineEvent → UI
[流式] chat_send_message(POST) 挂起至完成；llm:stream 经 /api/events 实时达各端
[桌面同位] Tauri 壳 → TauriEventBus → window emit → useTauriEvent/useEngineEvent（行为与现状逐字节一致）
[备份] export_all → /api/export 下载；反向 import_all 恢复
[认证] 首访 /api/setup 设令牌 → /api/auth/login 换 Cookie → 全端点中间件校验
```

**外部集成：**
- 无新增第三方云依赖（Caddy 与 opencode 均随镜像自托管分发）
- 云端实例不连接伴侣中继（paired_devices 闲置，裁决 C 的远期收敛是后话）

### Development Workflow Integration

```bash
# 引擎：cd crates/egosync-engine && cargo test
# 桌面：流程不变——cd egosync-app && npm run tauri dev / npm run test:all
# server 本地：cd server && cargo run（EGOSYNC_* env 就绪后 http://localhost:8080）
# WEB 本地：cd egosync-app && npm run dev（vite proxy /api → localhost:8080）或直连 server 内嵌静态
# 云端整链路：cd server && docker compose up --build
```

## Architecture Validation Results — 云端托管版（FR-44～FR-48）

_本次验证覆盖本章与基线/伴侣章节、PRD §4.15、决策 #19 的一致性；未修改业务代码，未运行任何测试（抽取尚未开始）；结论仅表示具备实施指导能力。耦合点数据来自 2026-09-17 代码验证。_

### Coherence Validation ✅

| 检查项 | 结果 | 说明 |
|--------|------|------|
| engine crate path 依赖（无 workspace） | 通过 | companion-proto 先例延续；src-tauri/server 各自独立声明 |
| axum 0.8 + tokio 1 与 relay-server 同栈 | 通过 | 同语言同运行时心智；SSE 为 axum 内建能力 |
| tokio::spawn 替换安全性 | 有条件通过 | service 层任务体内安全（现状已混用）；**三个迁入 engine 的同步启动入口必须走 Handle 注入接缝**（register_stream_mirror 随 companion_dispatch 留壳不变）——tauri setup 闭包无 runtime 上下文，裸 spawn 会 panic（本项目 v0.1.6-alpha.1 亲历，companion_dispatch.rs 注释存证）；抽取含桌面启动路径 e2e 断言 |
| companion 服务归属 | 通过 | 全部四个 companion_* service 留桌面壳不迁 engine（裁决见 ①）——listen 是第四类耦合且桌面专属；EngineEvents 保持 emit-only；与 companion_* command desktop-only、paired_devices 云端闲置三方自洽 |
| EventSource→Cookie 认证链 | 通过 | 平台约束（无自定义头）倒逼；httpOnly + SameSite=Strict + 同源架构自洽 |
| 与基线 100 条规则 + 伴侣章节 | 通过 | 全量继承；伴侣 Deferred"云端数据同步"仍 Non-Goal，"托管"语义更新已显式登记（裁决 C） |
| 桌面零回归路径 | 通过 | 绞杀者五步，每步全绿收口；物理封禁 tauri 入 engine |

### Requirements Coverage Validation ✅

- **FR-44**：Docker 一键（compose 两服务）/ 完整引擎（engine+opencode+双库）/ 单实例单用户 / 单事实源（边界 5）——全覆盖
- **FR-45**：浏览器双端访问 / 核心功能集（capability 门控 + 组件复用，视觉零分叉）/ 流式（SSE）/ 刷新恢复（服务端持久化 + 重取）——覆盖；功能分档清单留 UX 阶段（PRD ASSUMPTION 保持）
- **FR-46**：初始化凭据（env 或首访 setup）/ 全端点强制认证（除 healthz 与静态）/ TLS（Caddy 或反代）/ Key 不下发（结构性保证 + 测试断言）——覆盖
- **FR-47**：常驻调度平移 / TZ 语义 / 去重持久化硬化 / 停机诚实代价 / FR-11 确认边界 / 桌面 FR-10 不变——覆盖
- **FR-48**：状态机裁决完毕（裁决 B）；后置交付不阻塞 FR-44~47——与 PRD 时序一致

### Implementation Readiness Validation ✅

- 决策完整性：Critical×8 全带理由；9 决策域 + 3 开放问题全部显式裁决
- 结构完整性：目录树到文件级；五条硬边界；数据边界表
- 模式完整性：5 类新冲突点约定 + 反模式清单齐备

### Gap Analysis Results

**Critical Gaps：无**

**Important Gaps（不阻塞，首故事内解决）：**
1. opencode 二进制获取与版本 pin 方式（官方 standalone 二进制 vs npm 分发进镜像）——Dockerfile 故事内定夺
2. Caddy SSE 反代的回归性验证（text/event-stream 理论上默认逐写刷新，`flush_interval -1` 为冗余保险——compose 起来后实测 token 流不断流即可）
3. vitest 既有 @tauri-apps/api mock 的迁移规模（14 个测试文件级机械替换，代码验证；量大但无歧义）——15.3 故事内一次完成
4. sqlx migrations 路径平移的编译期修正（已确认：db/pool.rs 的 `sqlx::migrate!("./migrations")` 宏按 crate 根解析相对路径，31 个迁移文件随 engine 平移后宏路径自然成立，但需验证 checksum 一致性不触发重跑）——平移故事内处理

**Nice-to-Have Gaps（明确延后）：**
- WS 双向通道、多实例横向扩展、Litestream 流式备份、浏览器通知 Service Worker 化

### Validation Issues Addressed

- 提案 §4.2-7"三件并列"与本章"opencode 并入 server 镜像"的措辞冲突已在 ⑤ 显式裁决（理由记录在案）
- 伴侣章节 Deferred"云端托管"的语义演化与约束表"不存在云端代替桌面跑循环"（FR-10 当时拓扑下成立）均已登记（裁决 C ①②），无静默矛盾
- FR-10（桌面运行时执行）与 FR-47（常驻）的形态分叉在引擎层不存在（同一 engine，宿主决定调度启动时机）——已在 ⑨ 说明
- **双镜头评审（2026-09-17，报告存 `architecture/reviews/`）**：①事实核实（PASS-WITH-FIXES）——初稿决策 #3"spawn 替换无行为变更"经 tauri 2.11.5 源码与本项目 v0.1.6-alpha.1 历史证伪，已改为 Handle 注入接缝；事件监听第四类耦合、六组状态、Caddy flush_interval 过强断言、env 键名、数字勘误均已按报告修正；②对抗性（PASS-WITH-FIXES）——契约形状层 11 条收紧（参数 schema、事件常量源、secret 键映射、AppError 白名单、重连补齐协议、特征测试先行等）已全部落入正文。两评审相互矛盾的 command 计数（120 vs 126）经独立复核取 120

### Architecture Completeness Checklist

**Requirements Analysis**
- [x] 项目上下文深度分析（含代码级耦合点验证）
- [x] 规模与复杂度评估
- [x] 技术约束识别（含 EventSource/TZ/UTC 平台约束）
- [x] 跨切关注点映射

**Architectural Decisions**
- [x] 关键决策含版本与理由（Critical×8）
- [x] 技术栈完整指定（与既有栈对齐）
- [x] 集成模式定义（双传输对等契约）
- [x] 性能考量（SSE 扇出/慢客户端断开/退避）

**Implementation Patterns**
- [x] 命名规范（增量部分）
- [x] 结构模式（测试位置四类）
- [x] 通信模式（对等契约 + 错误码语义）
- [x] 流程模式（重连/日志/收口条件/去重统一）

**Project Structure**
- [x] 完整目录结构（文件级）
- [x] 组件边界（五条硬边界）
- [x] 集成点映射
- [x] 需求到结构映射完成

### Architecture Readiness Assessment

**Overall Status:** READY WITH MINOR GAPS

**Confidence Level:** Medium-High（不确定性集中在 chat/agent_engine 域的事件耦合迁移——22 处 emit 与流式状态纠缠，是抽取故事最大一块；spawn/时区/密钥等事实性风险已经双评审收敛，实现期验证点已登记 Gap 清单）

**Key Strengths:**
- "同一引擎、双宿主"把全部云端价值收敛为三条 trait 接缝 + 一张注册表，桌面零回归有物理封禁与分步收口双保险
- API 面从共享注册表生成，1:1 对应靠机制不靠纪律
- 单用户/单源/无同步的不变式贯穿三个开放问题裁决
- 密钥不下发在现有数据模型中已结构性成立（api_key_ref），云端只需守住不破坏

**Areas for Future Enhancement:**
- FR-48 桌面远程模式（第二阶段，状态机已裁决）
- Android 直连云端实例（裁决 C 的远期形态）
- 官方托管 SaaS（决策 #19 后议）

### Implementation Handoff

**AI Agent Guidelines:**
- 本文档（含云端托管版增量章节）与 `project-context.md`、伴侣章节共同构成实施规范；冲突时以本文（更新者）为准并显式记录
- 抽取期规则三（外科手术式修改）全程生效；每步收口=桌面全绿
- 遵守五条硬边界与反模式清单

**First Implementation Priority:**
1. Story 15.1a：`crates/egosync-engine` 骨架 + 无 Tauri 模块平移（db/models/llm/error/agent_bridge/data_export）+ 桌面全绿收口
2. Story 15.1b：EngineEvents + TauriEventBus + chat/agent_engine 域迁移（最重一块）
3. 后续按 Implementation Sequence 3~11 顺延
