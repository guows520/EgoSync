---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
baselineStepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments: ['prd-egosync.md', 'ux-design-specification.md', 'brainstorming-session-2026-05-18-1000.md', 'egosync-app/src/App.tsx']
workflowType: 'architecture'
lastStep: 8
status: 'complete'
baselineCompletedAt: '2026-05-25'
updated: '2026-07-23'
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

