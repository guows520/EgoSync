---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments: ['prd-egosync.md', 'ux-design-specification.md', 'brainstorming-session-2026-05-18-1000.md', 'egosync-app/src/App.tsx']
workflowType: 'architecture'
lastStep: 8
status: 'complete'
completedAt: '2026-05-25'
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
│       ├── ci.yml                    # Lint + Test (三平台)
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
