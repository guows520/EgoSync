---
project_name: '探索 (EgoSync)'
user_name: 'boss'
date: '2026-05-20'
sections_completed: ['technology_stack', 'language_rules', 'framework_rules', 'testing_rules', 'code_quality', 'dev_workflow', 'critical_rules']
status: 'complete'
rule_count: 100
optimized_for_llm: true
---

# AI Agent 项目上下文

_本文件包含 AI Agent 在本项目中实现代码时必须遵循的关键规则和模式。重点是 Agent 容易遗漏的非显而易见细节。_

---

## 技术栈与版本

### 前端 (GUI/)
- **React** 18.2 + **TypeScript** 5.2 (strict mode, noUnusedLocals, noUnusedParameters)
- **Vite** 5.0 (HMR dev, 生产构建)
- **TailwindCSS** 3.3.5 + **tailwindcss-animate** 1.0.7 (darkMode: 'class')
- **Lucide React** 0.292 (图标库)
- **clsx** 2.0 + **tailwind-merge** 2.0 (样式合并工具)
- **字体**: Inter + Noto Sans SC (Google Fonts CDN)
- **目标**: ES2020, jsx: react-jsx, moduleResolution: bundler

### 后端 (GUI/src-tauri/) — 计划中
- **Tauri** 2.x (Rust 后端 + WebView 前端)
- **Rust** edition 2021 + **tokio** 异步运行时
- **SQLx** 0.8+ (SQLite, 编译时 SQL 校验)
- **serde** (JSON 序列化, `rename_all = "camelCase"`)
- **keyring** 3.x (跨平台密钥管理)
- **tracing** (结构化日志)

### 构建与分发
- **Tauri bundler**: MSI (Windows) / DMG (macOS) / AppImage (Linux)
- **CI/CD**: GitHub Actions 三平台并行

### 版本约束
- Tauri 2.x 与 React 18 + Vite 5 为官方支持组合
- SQLx 0.8 + tokio 运行时必须一致
- 前端当前无 Tauri 依赖（原型阶段），接入时需安装 @tauri-apps/cli + @tauri-apps/api

## 关键实现规则

### 语言特定规则

**TypeScript (前端):**
- strict mode 已启用 — 所有变量必须有类型，禁止隐式 `any`
- `noUnusedLocals` + `noUnusedParameters` 已启用 — 未使用的变量/参数将报编译错误
- 使用 `cn()` 工具函数合并样式类：`cn(...inputs: ClassValue[])` = `twMerge(clsx(inputs))`
- 接口/类型命名 PascalCase：`Role`, `ChatMessage`, `CreateRoleInput`
- 函数/变量命名 camelCase：`sendMessage`, `roleId`
- 常量命名 SCREAMING_SNAKE_CASE：`ROLES`, `COLOR_OPTIONS`
- JSON 字段一律 camelCase（与 Rust 端通过 serde 自动转换）
- 日期格式统一 ISO 8601：`2026-05-20T10:00:00Z`
- ID 格式统一 UUID v4 字符串
- 空值用 `T | null`，对应 Rust 的 `Option<T>`
- 枚举值用小写字符串：`"pending"`, `"accepted"`, `"whisper"`

**Rust (后端):**
- 所有函数返回 `Result<T, AppError>`，Command 层禁止 `.unwrap()`
- 结构体必须 derive `Serialize, Deserialize` 并使用 `#[serde(rename_all = "camelCase")]`
- 模块/文件命名 snake_case：`llm_provider.rs`, `chat_commands.rs`
- 结构体/枚举命名 PascalCase：`RoleModel`, `AppError`
- Tauri Command 命名 snake_case 带域前缀：`chat_send_message`, `role_create`
- 常量 SCREAMING_SNAKE_CASE：`DEFAULT_WORK_LOOP_INTERVAL`
- 日志用 `tracing` crate，包含完整上下文

### 框架特定规则

**React 前端:**
- 状态管理：useState/useEffect（V1），V2 视复杂度引入 Zustand
- 不可变更新：`setState(prev => [...prev, newItem])` 或 `setState(prev => prev.map(...))`，禁止直接 mutation
- 自定义 hooks 封装 Tauri 通信：`useTauriCommand`（invoke + loading/error）、`useTauriEvent`（listener 注册/清理）
- 组件文件 PascalCase.tsx：`ButlerView.tsx`, `RoleCard.tsx`
- Hook 文件 camelCase.ts + use 前缀：`useTauriEvent.ts`, `useRoles.ts`
- Service 文件 camelCase.ts：`chatService.ts`, `roleService.ts`
- Type 文件 camelCase.ts：`role.ts`, `chat.ts`
- CSS 样式一律用 Tailwind utility class，禁止写自定义 CSS class
- Loading 状态命名：`isLoading`, `isSubmitting`, `isSending`, `isStreaming`（组件级，非全局）
- 流式对话：`isStreaming` 布尔 + `streamContent` 累积字符串

**Tauri IPC:**
- 前端 service 层封装所有 `invoke()` 调用，组件不直接调用 invoke
- 事件监听统一通过 `useTauriEvent` hook，自动处理 cleanup
- Event 命名：`{domain}:{verb_past}` — `role:created`, `role:proposed`, `task:updated`, `llm:stream`
- LLM 流式 payload：`{ roleId, token, done }`
- `role:proposed` payload：`{ name, icon, color, goal }`（引导中 Function Calling 触发，前端弹确认 modal）
- 写操作返回确认 + Event 推送变更通知，前端监听刷新

**Tauri Rust 后端:**
- Command 层只做参数解析 → 调 Service → 返回结果，禁止含业务逻辑
- Service 层拥有所有业务逻辑，可调 db/ 和 llm/
- db/ 只做 SQL 执行，不含业务判断
- llm/ 只做 HTTP 请求和流式解析，不含业务逻辑
- 并发模型：每次对话独立 `tokio::spawn`
- 上下文管理：后端全权负责 system prompt + 记忆 + 上下文裁剪 + token 计数

### 测试规则

**前端测试 (Vitest + React Testing Library):**
- 测试文件 co-located 同目录：`{Component}.test.tsx`, `{service}.test.ts`
- 测试命令：`cd GUI && npx vitest`

**Rust 后端测试:**
- 单元测试：同文件底部 `#[cfg(test)] mod tests`
- 集成测试：`src-tauri/tests/test_{domain}.rs`（如 `test_chat.rs`, `test_roles.rs`）
- 集成测试公共模块：`tests/common/mod.rs`
- 测试命令：`cd GUI/src-tauri && cargo test`
- SQLx 测试：使用 sqlx test fixtures

**E2E 测试 (V1 必需):**
- 使用 Tauri driver (WebDriver 协议)
- 覆盖核心用户旅程：冷启动引导、管家对话、角色 CRUD、LLM 流式响应、任务管理
- 确保应用启动→基本交互→数据持久化的端到端可用性

**全量测试:**
- `npm run test:all`（组合前端 vitest + Rust cargo test + E2E）

### 代码质量与风格规则

**文件与目录组织:**
- 前端组件按域分目录：`butler/`, `role/`, `chat/`, `modals/`, `notifications/`, `settings/`, `onboarding/`
- 共享布局组件：`layout/` (Sidebar, MainLayout, Header)
- 新增组件必须放在对应域文件夹，禁止在 App.tsx 中新增组件
- App.tsx 仅做顶层路由/场景管理

**数据库命名:**
- 表名：snake_case 复数 — `roles`, `memories`, `tasks`
- 列名：snake_case — `role_id`, `created_at`, `content_json`
- 外键：`{被引用表单数}_id` — `role_id`, `conversation_id`
- 索引：`idx_{table}_{column}` — `idx_memories_role_id`
- 迁移文件：`{seq}_{description}.sql` — `001_initial_schema.sql`
- 新增表/列必须通过 SQLx migration 文件，禁止手动改 DB

**错误处理:**
- Rust：类型化错误枚举 `AppError { NotFound, LlmError, DbError, ValidationError, KeyringError }`
- 错误序列化为 JSON：`{ "NotFound": "Role with id xxx not found" }`
- 前端：每个 service 调用用 try-catch，Hook 层统一管理 loading/error
- 用户可见错误：友好中文提示，不暴露技术细节

**文档要求:**
- 保持现有代码注释风格，不主动增删注释
- Rust 公共 API 使用 `///` 文档注释

### 开发工作流规则

**开发服务器:**
- `cd GUI && npm run tauri dev` — 同时启动 Vite HMR (port 5173) + Rust 增量编译 + Tauri 窗口
- 纯前端开发：`cd GUI && npm run dev`（原型阶段，Tauri 接入前）

**构建流程:**
- `cd GUI && npm run tauri build` — Vite 生产构建 → Rust release 编译 → Tauri bundler 打包
- 输出：`src-tauri/target/release/bundle/{msi,dmg,appimage}`

**CI/CD (GitHub Actions):**
- 三平台并行构建 (Windows/macOS/Linux)
- 自动化测试：Rust tests + Vitest + E2E
- Release artifact 自动发布

**数据库迁移:**
- 使用 `sqlx migrate` 管理
- 迁移文件存放：`GUI/src-tauri/migrations/`
- 命名格式：`{seq}_{description}.sql`

**新增 Tauri Command 检查清单:**
1. Rust: `commands/{domain}.rs` 中新增 command 函数
2. Rust: `services/` 中实现业务逻辑
3. Rust: `main.rs` 中注册 command
4. TS: `types/{domain}.ts` 中定义类型
5. TS: `services/{domain}Service.ts` 中封装 invoke
6. TS: `hooks/use{Domain}.ts` 中封装 hook（如需）

### 关键禁止事项

**绝对禁止 (Anti-Patterns):**
- ❌ 在前端直接操作 SQLite（必须走 Tauri IPC）
- ❌ 硬编码 API Key 到源码（使用 keyring 系统钥匙串）
- ❌ 在 Rust Command 层写业务逻辑（Command 只做参数解析 → 调 Service → 返回结果）
- ❌ 使用 `.unwrap()` 处理可能失败的操作（用 `Result` + `?`）
- ❌ 混用 camelCase 和 snake_case（遵守各层约定：TS=camelCase, Rust/DB=snake_case, serde 自动桥接）
- ❌ 在前端写自定义 CSS class（使用 Tailwind utility）
- ❌ 前端直接调用 LLM API（必须走 Rust 后端）
- ❌ 在 App.tsx 中新增组件定义（拆分到对应域文件夹）

**安全规则:**
- API Key 存储使用 keyring crate（Windows Credential Manager / macOS Keychain / Linux Secret Service）
- V1 不加密 DB，依赖 OS 文件权限
- LLM 调用不在服务端存储用户对话内容

**性能注意:**
- LLM 流式传输必须用 Tauri Event（`app.emit`），不轮询
- 前端状态更新用增量方式，骨架屏仅首次加载使用
- 60fps 动效依赖 Tailwind + CSS transition，避免 JS 动画阻塞渲染

**数据边界:**
- 主数据库 `egosync.db`：所有 services 可读写
- 对话日志库 `conversations.db`：仅 `db/conversations.rs` 直接访问，其他模块通过 service 层
- 前端永远不直接访问 DB 或 LLM API

---

## 使用指南

**AI Agent:**
- 实现任何代码前先读本文件
- 严格遵循所有规则，无例外
- 有疑问时选择更严格的方案
- 架构细节参考 `_bmad-output/planning-artifacts/architecture.md`

**维护:**
- 技术栈变更时同步更新
- 定期审查，移除过时规则
- 保持精简，聚焦 Agent 容易遗漏的细节

最后更新：2026-05-20
