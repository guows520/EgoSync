---
baseline_commit: 22ef393437a68ba7f57284b99c5907fa15a5fcec
---

# Story 7.1: 用户能一键导出全部数据

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 随时将我的所有数据导出为标准格式,
So that 我拥有数据的完全控制权且可迁移。

## 背景与现状（务必先读）

**本 story 是 Epic 7（数据主权）的第一个 story — 实现完整数据导出功能。Epic 1-6 已全部完成，应用具备角色、任务、记忆、对话、使命宣言、建议、通知、简报、周复盘等完整数据。**

**核心交付：**
1. **Rust 后端导出 service**：支持三种格式（可多选）：
   - **SQLite 备份（推荐）**：直接复制 `egosync.db` + `conversations.db` 文件，速度 <1 秒，数据 100% 保真
   - **JSON 数据**：查询主 DB 所有表 + 对话 DB 所有表 → 序列化为完整 JSON（跨版本迁移用）
   - **Markdown 报告**：查询所有表 → 生成人类可读 Markdown（按角色分章节）
2. **Tauri command**：`data_export` 接收目录路径 + 格式列表，执行导出，返回导出文件路径列表
3. **前端接通**：GlobalSettingsModal 数据 Tab 的"导出存档"按钮从 mock 变为真实调用，点击后先选格式（多选），再选保存目录

### 已建成的基础（本 story 的接入点）

**前端 GlobalSettingsModal 数据 Tab（直接修改）：**
- `src/components/settings/GlobalSettingsModal.tsx:765-820` — `tab === 'data'` 区域，已有 mock "导出存档"按钮（无 onClick handler）和"销毁所有数据"按钮（无 onClick handler）
- `src/components/settings/GlobalSettingsModal.tsx:409` — 侧边栏"数据与主权"tab 按钮
- `src/components/settings/GlobalSettingsModal.tsx:413` — tab 标题渲染（`tab === 'data'` 时显示"数据与隐私"）
- **本 story 只接通"导出存档"按钮**，"销毁所有数据"按钮属于 Story 7.2

**前端 service 层模式（直接参照）：**
- `src/services/appService.ts` — 现有 service 模式：`import { invoke } from '@tauri-apps/api/core'` + 导出对象封装 invoke 调用
- 新建 `src/services/dataService.ts`，封装 `data_export` invoke 调用

**Rust 后端 — rfd 文件对话框（直接参照）：**
- `src-tauri/src/commands/skill.rs:36` — `rfd::FileDialog::new().pick_folder()` 同步选择文件夹（spawn_blocking 模式）
- `src-tauri/src/commands/chat.rs:471` — `rfd::AsyncFileDialog::new().pick_folder().await` 异步选择文件夹
- **本 story 使用 `rfd::FileDialog::new().pick_folder()`** 让用户选择保存目录（与 skill.rs 模式一致）

**Rust 后端 — DB 层现有查询函数（直接复用）：**
- `src-tauri/src/db/roles.rs:44` — `list_all_roles(pool) -> Vec<Role>` 查所有角色
- `src-tauri/src/db/tasks.rs:60` — `list_all_tasks(pool, quadrant, is_big_rock) -> Vec<CrossRoleTask>` 查所有任务（传 None, None 查全部）
- `src-tauri/src/db/memories.rs:357` — `list_all_memories(pool) -> Vec<Memory>` 查所有记忆
- `src-tauri/src/db/suggestions.rs` — 需新增 `list_all_suggestions` 或直接在 export service 中查询
- `src-tauri/src/db/notifications.rs:44` — `list_notifications(pool) -> Vec<NotificationWithRole>` 查所有通知
- `src-tauri/src/db/mission.rs:6` — `get_mission(pool) -> Option<Mission>` 查使命宣言
- `src-tauri/src/db/briefings.rs:46` — `get_latest_briefing(pool) -> Option<Briefing>` 查最新简报（需新增 `list_all_briefings`）
- `src-tauri/src/db/weekly_reviews.rs` — 需新增 `list_all_weekly_reviews` 或直接查询
- `src-tauri/src/db/app_settings.rs` — 需新增 `get_all_settings` 查所有设置
- `src-tauri/src/db/settings.rs:6` — `list_llm_configs(pool) -> Vec<LlmConfig>` 查所有 LLM 配置
- `src-tauri/src/db/mcp_servers.rs:8` — `list_mcp_servers(pool) -> Vec<McpServer>` 查所有 MCP server
- `src-tauri/src/db/conversations.rs:253` — `list_all_conversations(conv_pool) -> Vec<Conversation>` 查所有对话
- `src-tauri/src/db/conversations.rs:288` — `list_messages(conv_pool, conversation_id) -> Vec<Message>` 查指定对话的消息

**Rust 后端 — conflicts 表（5.3 已实现但 5.4-5.6 延迟）：**
- `src-tauri/migrations/021_conflicts.sql` — conflicts 表已存在，需导出
- `src-tauri/src/db/` — 无 conflicts.rs 模块（conflict 检测在 `services/conflict_detector.rs` 中直接查询），导出时需直接 SQL 查询

**Rust 后端 — command 注册模式（直接参照）：**
- `src-tauri/src/lib.rs:278-373` — `invoke_handler` 中注册所有 commands
- `src-tauri/src/commands/mod.rs` — 注册 `pub mod data;`
- `src-tauri/src/commands/app.rs` — 简单 command 模式：`#[tauri::command] pub async fn xxx(pool: State<'_, DbPool>) -> Result<T, AppError>`

**Rust 后端 — 双 DB pool 模式（直接参照）：**
- `src-tauri/src/lib.rs:56-65` — 主 pool（egosync.db）和 conv_pool（conversations.db）都通过 `app.manage()` 注册为 Tauri State
- `src-tauri/src/db/pool.rs:11` — `ConversationsPool` 是 `SqlitePool` 的 newtype wrapper
- command 同时注入两个 pool：`pool: State<'_, DbPool>, conv_pool: State<'_, ConversationsPool>`

### 数据库表清单（导出范围）

**主数据库 (egosync.db) — 13 张表：**
| 表名 | 用途 | 现有查询函数 |
|------|------|-------------|
| `roles` | 角色定义 | `list_all_roles` |
| `tasks` | 任务 | `list_all_tasks(None, None)` |
| `memories` | 记忆 | `list_all_memories` |
| `suggestions` | 建议 | 需新增或直接查询 |
| `notifications` | 通知 | `list_notifications` |
| `mission` | 使命宣言 | `get_mission` |
| `conflicts` | 冲突记录 | 需直接 SQL 查询 |
| `briefings` | 晨间简报 | 需新增 `list_all_briefings` |
| `weekly_reviews` | 周复盘 | 需新增 `list_all_weekly_reviews` |
| `llm_configs` | LLM 配置 | `list_llm_configs` |
| `app_settings` | 全局设置 | 需新增 `get_all_settings` |
| `mcp_servers` | MCP server | `list_mcp_servers` |
| `skill_role_bindings` | Skill 绑定 | 需直接 SQL 查询 |
| `skills_registry` | Skill 注册 | 需直接 SQL 查询或复用 |
| `q2_reminders` | Q2 提醒记录 | 需直接 SQL 查询 |
| `big_rock_protection_reminders` | 大石头保护提醒 | 需直接 SQL 查询 |

**对话日志库 (conversations.db) — 2 张表：**
| 表名 | 用途 | 现有查询函数 |
|------|------|-------------|
| `conversations` | 对话会话 | `list_all_conversations` |
| `messages` | 消息 | `list_messages`（需逐对话查询） |

**注意：`_sqlx_migrations` 表为 schema 元数据，不导出。**

## Acceptance Criteria

1. **AC1**: Given 用户在 GlobalSettingsModal 数据 Tab 点击"导出存档"，When 弹出格式选择（可多选）：SQLite 备份 / JSON 数据 / Markdown 报告，Then 用户勾选格式后 Tauri 文件保存对话框弹出，用户选择保存目录，And 根据勾选的格式生成对应文件：
   - SQLite：复制 `egosync.db` + `conversations.db` 到目标目录（重命名为 `egosync-export-{date}.db` + `egosync-export-{date}-conversations.db`）
   - JSON：`egosync-export-{date}.json`（完整 schema 的 JSON，含所有表数据）
   - Markdown：`egosync-export-{date}.md`（人类可读的 Markdown 版本）

2. **AC2**: Given SQLite 备份导出，Then 直接复制数据库文件到目标目录，不经过查询/序列化，And 速度 <1 秒，数据 100% 保真（含所有索引、外键、触发器）

3. **AC3**: Given JSON 导出内容，Then 包含：角色定义、任务、记忆、对话历史、使命宣言、建议、通知、冲突仲裁记录、周复盘、app_settings、LLM 配置、MCP server、Skill 注册与绑定、简报，And 每个实体保留完整字段和关联关系，And 含 `export_version` 字段用于版本兼容性校验

4. **AC4**: Given Markdown 导出内容，Then 按角色分章节，每章包含：角色信息 → 任务列表 → 记忆条目 → 对话摘要，And 可读性优先（无 UUID，使用角色名/任务标题）

5. **AC5**: Given 导出性能，Then SQLite 备份 <1 秒；JSON + Markdown 30 秒内完成（含 1000+ 条记忆和 500+ 条对话），And 导出期间显示 loading 状态

6. **AC6**: Given 导出成功，Then 显示成功提示 + 生成的文件路径列表

7. **AC7**: Given Rust 后端，Then Tauri command: `data_export { dir_path, formats }` → 根据格式执行对应导出逻辑 → 返回文件路径列表，And SQLite 格式：直接复制 .db 文件，And JSON 格式：查询所有表 → 序列化 → 写入，And Markdown 格式：查询所有表 → 生成 Markdown → 写入

## Tasks / Subtasks

- [ ] **Task 1: 新建 Rust 导出 service — `data_export.rs`** (AC: #1, #2, #3, #4, #7)
  - [ ] 1.0 新建 `src-tauri/src/services/data_export.rs`
  - [ ] 1.1 定义 `ExportFormat` 枚举（`Serialize + Deserialize`）：`Sqlite`、`Json`、`Markdown`，支持多选（`Vec<ExportFormat>` 或 bitflags）
  - [ ] 1.2 定义 `ExportData` 结构体（`Serialize + Deserialize + rename_all = "camelCase"`），包含所有表数据字段（仅 JSON/Markdown 格式使用）：
    - `roles: Vec<Role>`
    - `tasks: Vec<CrossRoleTask>`
    - `memories: Vec<Memory>`
    - `suggestions: Vec<serde_json::Value>`（直接 SQL 查询，无现有 model 可复用时用 Value）
    - `notifications: Vec<NotificationWithRole>`
    - `mission: Option<Mission>`
    - `conflicts: Vec<serde_json::Value>`（直接 SQL 查询）
    - `briefings: Vec<Briefing>`
    - `weekly_reviews: Vec<WeeklyReview>`
    - `llm_configs: Vec<LlmConfig>`
    - `app_settings: Vec<(String, Option<String>)>`（key-value 对）
    - `mcp_servers: Vec<McpServer>`
    - `skills: Vec<serde_json::Value>`（直接 SQL 查询 skills_registry）
    - `skill_bindings: Vec<serde_json::Value>`（直接 SQL 查询 skill_role_bindings）
    - `q2_reminders: Vec<serde_json::Value>`（直接 SQL 查询）
    - `big_rock_protection_reminders: Vec<serde_json::Value>`（直接 SQL 查询）
    - `conversations: Vec<Conversation>`
    - `messages: Vec<Message>`（所有对话的所有消息，展平为一个数组或按 conversation_id 分组）
    - `exported_at: String`（导出时间戳）
    - `export_version: String`（导出格式版本，如 `"1.0"`）
  - [ ] 1.3 定义 `ExportResult` 结构体（`Serialize + Deserialize + rename_all = "camelCase"`）：
    - `files: Vec<String>` — 生成的文件路径列表
    - `sqlite_path: Option<String>` — SQLite 备份路径（如果生成了）
    - `json_path: Option<String>` — JSON 文件路径（如果生成了）
    - `markdown_path: Option<String>` — Markdown 文件路径（如果生成了）
  - [ ] 1.4 实现 `export_sqlite(app_data_dir: &Path, dir_path: &Path) -> Result<Vec<String>, AppError>`：
    - 获取 `egosync.db` 和 `conversations.db` 文件路径
    - 使用 `std::fs::copy` 复制到目标目录，重命名为 `egosync-export-{date}.db` 和 `egosync-export-{date}-conversations.db`
    - 返回复制的文件路径列表
    - **注意**：SQLite 备份不需要查询数据库，直接复制文件即可。需要从 Tauri State 获取 `app_data_dir` 路径（在 command 层传入）
  - [ ] 1.5 实现 `export_json(pool: &DbPool, conv_pool: &ConversationsPool, dir_path: &Path) -> Result<String, AppError>`：
    - 查询主 DB 所有表数据
    - 查询对话 DB 所有对话和消息
    - 构建 `ExportData` 结构体
    - 使用 `serde_json::to_string_pretty` 序列化
    - 写入 `egosync-export-{date}.json` 到 `dir_path`
    - 返回文件路径
  - [ ] 1.6 实现 `export_markdown(pool: &DbPool, conv_pool: &ConversationsPool, dir_path: &Path) -> Result<String, AppError>`：
    - 查询主 DB 所有表数据（同 `export_json` 的查询逻辑，可抽取公共函数 `gather_export_data`）
    - 调用 `generate_markdown(&export_data)` 生成 Markdown 字符串
    - 写入 `egosync-export-{date}.md` 到 `dir_path`
    - 返回文件路径
  - [ ] 1.7 实现 `generate_markdown(export_data: &ExportData) -> String`：
    - 顶部：导出时间 + 版本 + 统计摘要（N 个角色、N 个任务、N 条记忆等）
    - 按角色分章节：每个角色一个 `## {角色名}` 章节
      - 角色信息：名称、图标、颜色、目标、能量、主动性级别
      - 任务列表：标题、象限、大石头标记、完成状态、截止日期
      - 记忆条目：类别、内容、创建时间
      - 对话摘要：对话 ID、消息数、最近更新时间（不导出完整对话内容到 Markdown，太长）
    - 底部：全局数据（使命宣言、简报、周复盘、LLM 配置、MCP server、设置）
  - [ ] 1.8 实现 `export_all(app_data_dir: &Path, pool: &DbPool, conv_pool: &ConversationsPool, dir_path: &Path, formats: Vec<ExportFormat>) -> Result<ExportResult, AppError>` 主函数：
    - 遍历 `formats`，根据每种格式调用对应的导出函数
    - SQLite 格式：调用 `export_sqlite`
    - JSON 格式：调用 `export_json`
    - Markdown 格式：调用 `export_markdown`
    - 汇总所有生成的文件路径到 `ExportResult`
  - [ ] 1.9 抽取 `gather_export_data(pool: &DbPool, conv_pool: &ConversationsPool) -> Result<ExportData, AppError>` 公共函数：JSON 和 Markdown 共用查询逻辑
  - [ ] 1.10 在 `src-tauri/src/services/mod.rs` 中注册 `pub mod data_export;`
  - [ ] 1.11 单元测试：`generate_markdown` 输出格式验证、`ExportData` 序列化/反序列化往返测试、`export_sqlite` 文件复制验证

- [ ] **Task 2: 新增 DB 查询函数** (AC: #2, #6)
  - [ ] 2.0 在 `src-tauri/src/db/briefings.rs` 中新增 `list_all_briefings(pool) -> Vec<Briefing>`
  - [ ] 2.1 在 `src-tauri/src/db/weekly_reviews.rs` 中新增 `list_all_weekly_reviews(pool) -> Vec<WeeklyReview>`
  - [ ] 2.2 在 `src-tauri/src/db/app_settings.rs` 中新增 `get_all_settings(pool) -> Vec<(String, Option<String>)>`
  - [ ] 2.3 在 `src-tauri/src/db/suggestions.rs` 中新增 `list_all_suggestions(pool) -> Vec<serde_json::Value>` 或复用现有函数
  - [ ] 2.4 在 `data_export.rs` 中直接 SQL 查询 `conflicts`、`skills_registry`、`skill_role_bindings`、`q2_reminders`、`big_rock_protection_reminders` 表（无现有 model 的用 `serde_json::Value`）
  - [ ] 2.5 在 `src-tauri/src/db/conversations.rs` 中新增 `list_all_messages(conv_pool) -> Vec<Message>` 查所有消息（或逐对话查询后合并）

- [ ] **Task 3: 新建 Tauri command** (AC: #1, #7)
  - [ ] 3.0 新建 `src-tauri/src/commands/data.rs`
  - [ ] 3.1 实现 `#[tauri::command] pub async fn data_export(dir_path: String, formats: Vec<String>, app_data_dir: String, pool: State<'_, DbPool>, conv_pool: State<'_, ConversationsPool>) -> Result<ExportResult, AppError>`
    - `formats` 参数：`["sqlite"]` / `["json"]` / `["markdown"]` / `["sqlite", "json", "markdown"]` 等组合
    - 将 `formats` 字符串列表转为 `Vec<ExportFormat>`
    - 调用 `services::data_export::export_all(&Path::new(&app_data_dir), &pool, &conv_pool, &Path::new(&dir_path), formats)`
    - 返回 `ExportResult`（serde camelCase）
    - **注意**：`app_data_dir` 需要在 command 层通过 `app.path().app_data_dir()` 获取并传入，或通过 `app_handle` 在 command 中直接获取
  - [ ] 3.2 在 `src-tauri/src/commands/mod.rs` 中注册 `pub mod data;`
  - [ ] 3.3 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册 `commands::data::data_export`

- [ ] **Task 4: 前端 service 层** (AC: #1, #6)
  - [ ] 4.0 新建 `src/services/dataService.ts`
  - [ ] 4.1 封装 `dataExport: (dirPath: string, formats: ExportFormat[]) => invoke<ExportResult>('data_export', { dirPath, formats })`
  - [ ] 4.2 定义 `ExportFormat` 类型（`'sqlite' | 'json' | 'markdown'`）
  - [ ] 4.3 定义 `ExportResult` 类型（`{ files: string[]; sqlitePath: string | null; jsonPath: string | null; markdownPath: string | null }`）

- [ ] **Task 5: 前端 GlobalSettingsModal 接通导出按钮** (AC: #1, #5, #6)
  - [ ] 5.0 在 `GlobalSettingsModal.tsx` 中新增导出状态：`isExporting: boolean`、`exportResult: ExportResult | null`、`exportError: string`、`selectedFormats: ExportFormat[]`
  - [ ] 5.1 实现 `handleExport` 函数：
    - 点击"导出存档"后先显示格式选择 UI（三个 checkbox：SQLite 备份（推荐） / JSON 数据 / Markdown 报告）
    - 用户勾选格式后点击"确认导出"按钮
    - 使用 `@tauri-apps/api/dialog` 的 `open({ directory: true })` 选择保存目录
    - 设置 `isExporting = true` → 调用 `dataService.dataExport(dirPath, selectedFormats)` → 成功设 `exportResult` / 失败设 `exportError` → `isExporting = false`
    - **注意**：`rfd` 在 Rust 端调用。前端有两种方案：
      - **方案 A**（推荐）：前端直接调用 `dataService.dataExport(dirPath, formats)`，dirPath 由前端通过 `@tauri-apps/api/dialog` 的 `open({ directory: true })` 获取
      - **方案 B**：Rust command 内部调用 `rfd::FileDialog` 选择目录，前端只调用 `dataService.dataExport(formats)` 无 dirPath 参数
    - 采用 **方案 A**：前端用 `@tauri-apps/api/dialog` 的 `open` 选择目录 → 传给 `data_export` command
  - [ ] 5.2 修改"导出存档"按钮：
    - 添加 `onClick={handleExport}`
    - 点击后先显示格式选择 UI（内联 checkbox + "确认导出"按钮）
    - 导出中：`disabled` + spinner + "导出中..."
    - 导出成功：显示 ✓ "导出完成" + 生成的文件路径列表（根据选择的格式显示 1-3 个路径）
    - 导出失败：显示错误提示
  - [ ] 5.3 导出 loading 状态（AC5）：由于导出在 Rust 端同步执行，前端无法获取实时进度。用 loading spinner 满足"导出期间有反馈"的需求

- [ ] **Task 6: 前端测试** (AC: #1, #6)
  - [ ] 6.0 在 `GlobalSettingsModal.test.tsx` 中新增导出按钮测试：
    - mock `dataService.dataExport` → 验证点击后调用
    - 验证格式选择 UI 出现
    - 验证 loading 状态
    - 验证成功后显示文件路径列表
    - 验证失败后显示错误

- [ ] **Task 7: Rust 集成测试** (AC: #1, #2, #3, #4)
  - [ ] 7.0 在 `data_export.rs` 的 `#[cfg(test)] mod tests` 中编写测试：
    - `export_sqlite` 测试：创建临时 .db 文件 → 复制 → 验证文件存在且大小一致
    - `export_json` 端到端测试：创建测试 DB → 插入测试数据 → 导出 → 验证 JSON 文件存在 + 可解析 + 含 `export_version`
    - `export_markdown` 测试：导出 → 验证 Markdown 含预期内容（角色名、任务标题等）
    - `generate_markdown` 格式测试
    - `ExportData` 序列化往返测试

## Dev Notes

### 关键技术决策

- **三种格式的定位**：
  - **SQLite 备份（推荐）**：直接复制 .db 文件，速度 <1 秒，数据 100% 保真（含索引、外键、触发器）。用于同版本恢复/迁移。这是最简单、最可靠的备份方式。
  - **JSON 数据**：查询所有表 → 序列化。用于跨版本迁移（未来 schema 变更时可通过 `export_version` 做兼容处理）。代码量大但可移植性好。
  - **Markdown 报告**：查询所有表 → 生成人类可读文档。用于用户查看数据内容，不适合恢复。

- **SQLite 备份实现**：使用 `std::fs::copy` 直接复制 `egosync.db` 和 `conversations.db` 文件。不需要关闭数据库连接——SQLite 的 WAL 模式下，复制 .db 文件可能不包含 WAL 中的最新数据。**解决方案**：复制前执行 `PRAGMA wal_checkpoint(TRUNCATE)` 强制将 WAL 数据写入主 .db 文件，然后复制。或者使用 `sqlx::query("VACUUM INTO ?")` 导出为新文件（更安全，但需要 SQLite 3.27+）。

- **前端选择目录方案**：采用方案 A — 前端通过 `@tauri-apps/api/dialog` 的 `open({ directory: true })` 选择目录，传给 Rust command。原因：前端可控制对话框 UI 和取消逻辑，Rust command 只负责导出逻辑。需确认 `@tauri-apps/api/dialog` 已安装（检查 `package.json`）。

- **导出文件命名**：
  - SQLite：`egosync-export-{YYYY-MM-DD}.db` + `egosync-export-{YYYY-MM-DD}-conversations.db`
  - JSON：`egosync-export-{YYYY-MM-DD}.json`
  - Markdown：`egosync-export-{YYYY-MM-DD}.md`
  - 日期用 `chrono::Local::now().format("%Y-%m-%d")`

- **JSON 格式**：使用 `serde_json::to_string_pretty` 输出格式化 JSON，便于人工检查。顶层结构为 `ExportData` 结构体的序列化形式。含 `export_version: "1.0"` 字段用于版本兼容性校验。

- **Markdown 格式**：按角色分章节。对话历史在 Markdown 中只放摘要（对话数、消息数、时间范围），完整对话内容在 JSON/SQLite 中。原因：500+ 条对话的完整内容放入 Markdown 会过于冗长，违反"可读性优先"原则。

- **无现有 model 的表**：`conflicts`、`skills_registry`、`skill_role_bindings`、`q2_reminders`、`big_rock_protection_reminders` 无对应 Rust model 结构体。在 `data_export.rs` 中直接用 `sqlx::query_as::<_, (serde_json::Value,)>()` 或 `sqlx::query` + 手动构建 `serde_json::Value` 查询。这避免为一次性导出创建 5 个 model 文件。

- **对话消息导出**：`list_all_conversations` 获取所有对话，然后逐对话调用 `list_messages` 获取消息。或者新增 `list_all_messages` 一次性查所有消息（更高效）。采用新增 `list_all_messages`。

- **导出性能**：SQLite 备份 <1 秒（文件复制）。JSON + Markdown 需要查询所有表 + 序列化，1000+ 记忆 + 500+ 对话的查询应在秒级完成，30 秒内完成无压力。

- **`@tauri-apps/api/dialog` 依赖**：检查 `package.json` 是否已有 `@tauri-apps/api`。如果已有（应该有，因为 Tauri 项目），则 `@tauri-apps/api/dialog` 可直接 import。如果没有，需要 `npm install @tauri-apps/api`。

- **Tauri capabilities/permissions**：Tauri 2.x 需要在 `src-tauri/capabilities/` 中声明 dialog 权限。检查 `src-tauri/capabilities/` 下是否已有 dialog 权限声明。如果没有，需新增。

- **app_data_dir 获取**：SQLite 备份需要知道 .db 文件路径。在 command 层通过 `app_handle.path().app_data_dir()` 获取，或直接传入 `app_data_dir: String` 参数。参照 `lib.rs:49-53` 中 `app.path().app_data_dir()` 的用法。

### 架构合规

- **分层规则**：Command 层（`commands/data.rs`）只做参数解析 → 调 Service（`services/data_export.rs`）→ 返回结果。Service 层负责查询所有 DB + 序列化 + 写文件。
- **Service 层职责**：`data_export.rs` 负责三种格式的导出逻辑：SQLite 文件复制、JSON 序列化、Markdown 生成
- **Command 层**：`commands/data.rs` 接收 `dir_path` + `formats` + `app_data_dir` 参数，调用 service，返回结果
- **命名规范**：Rust 文件 `data_export.rs`（snake_case），结构体 `ExportData` / `ExportResult`（PascalCase），Tauri command `data_export`（snake_case），前端 service `dataService.ts`（camelCase），前端类型 `ExportResult`（PascalCase）
- **错误处理**：所有 DB 查询失败用 `?` 传播 `AppError`，文件写入失败用 `AppError::DbError` 或新增 `AppError::IoError`（建议复用 `DbError` 或用 `ValidationError`，避免修改 error.rs 枚举）。**建议：文件 IO 错误用 `AppError::ValidationError(format!("导出文件写入失败: {}", e))`，避免修改 error.rs 枚举。**
- **serde 桥接**：`ExportData` 和 `ExportResult` 派生 `Serialize + Deserialize` + `#[serde(rename_all = "camelCase")]`
- **双 DB pool**：command 同时注入 `DbPool` 和 `ConversationsPool`，与 `chat.rs` 中的模式一致

### 前端 UI 规范

- **修改现有组件**：只修改 `GlobalSettingsModal.tsx` 的 `tab === 'data'` 区域
- **不新增组件文件**：导出状态和 UI 都在 `GlobalSettingsModal.tsx` 内
- **Tailwind 样式**：遵循现有按钮样式模式（参照 LLM 配置 tab 的按钮风格）
- **Loading 状态**：`isExporting` + `Loader2` 图标 + `animate-loading-spin` class（与现有 loading 模式一致）
- **成功反馈**：`Check` 图标 + 绿色文字 + 文件路径（参照 scheduler tab 的"已保存"反馈模式）
- **错误反馈**：`AlertCircle` 图标 + 红色文字（参照现有 error 显示模式）
- **空状态文案**：遵循 UX-DR18 — 不显示"暂无数据"

### 反模式警告

- **不要**修改"销毁所有数据"按钮 — 那属于 Story 7.2
- **不要**修改 `error.rs` 枚举 — 文件 IO 错误用 `ValidationError` 或 `DbError` 包装
- **不要**为 `conflicts`、`skills_registry`、`skill_role_bindings`、`q2_reminders`、`big_rock_protection_reminders` 新建 model 文件 — 直接在 `data_export.rs` 中用 `serde_json::Value` 查询
- **不要**在导出中包含 `_sqlx_migrations` 表 — 那是 schema 元数据
- **不要**在 Markdown 中导出完整对话内容 — 只放摘要，完整内容在 JSON/SQLite 中
- **不要**在导出中包含 API Key 明文 — `llm_configs` 表的 `api_key_ref` 字段是 keyring 引用名（如 `llm_config_{id}`），不是 API Key 本身，可以导出
- **不要**使用 `@tauri-apps/api/dialog` 之前不检查依赖是否安装
- **不要**忘记在 `src-tauri/capabilities/` 中声明 dialog 权限（如果需要）
- **不要**在 Command 层写业务逻辑 — `commands/data.rs` 只做参数解析和调 service
- **不要**修改现有 DB 查询函数 — 只新增不修改

### Project Structure Notes

新增文件：
- `src-tauri/src/services/data_export.rs` — 导出 service（SQLite 复制 + JSON 序列化 + Markdown 生成）
- `src-tauri/src/commands/data.rs` — Tauri command
- `src/services/dataService.ts` — 前端 service 封装

修改文件：
- `src-tauri/src/services/mod.rs` — 注册 `pub mod data_export;`
- `src-tauri/src/commands/mod.rs` — 注册 `pub mod data;`
- `src-tauri/src/lib.rs` — 在 `invoke_handler` 中注册 `commands::data::data_export`
- `src-tauri/src/db/briefings.rs` — 新增 `list_all_briefings`
- `src-tauri/src/db/weekly_reviews.rs` — 新增 `list_all_weekly_reviews`
- `src-tauri/src/db/app_settings.rs` — 新增 `get_all_settings`
- `src-tauri/src/db/suggestions.rs` — 新增 `list_all_suggestions`（或确认现有函数可用）
- `src-tauri/src/db/conversations.rs` — 新增 `list_all_messages`
- `src/components/settings/GlobalSettingsModal.tsx` — 接通"导出存档"按钮
- `src/components/settings/GlobalSettingsModal.test.tsx` — 新增导出测试

不修改文件：
- `src-tauri/src/error.rs` — 不新增错误变体
- `src-tauri/src/db/roles.rs` — 直接复用 `list_all_roles`
- `src-tauri/src/db/tasks.rs` — 直接复用 `list_all_tasks`
- `src-tauri/src/db/memories.rs` — 直接复用 `list_all_memories`
- `src-tauri/src/db/notifications.rs` — 直接复用 `list_notifications`
- `src-tauri/src/db/mission.rs` — 直接复用 `get_mission`
- `src-tauri/src/db/settings.rs` — 直接复用 `list_llm_configs`
- `src-tauri/src/db/mcp_servers.rs` — 直接复用 `list_mcp_servers`
- `src-tauri/src/db/conversations.rs` — 复用 `list_all_conversations`，只新增 `list_all_messages`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.1] — AC 原文
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7] — Epic 7 上下文（FR-25, FR-26）
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture] — 数据库表设计
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: _bmad-output/implementation-artifacts/6-6-big-rock-daily-protection.md] — Story 6.6 实现上下文（最近完成的 story，参照 story 文件格式和 dev notes 风格）
- [Source: src/components/settings/GlobalSettingsModal.tsx:765-820] — 数据 Tab UI（导出按钮接入点）
- [Source: src/components/settings/GlobalSettingsModal.tsx:409] — "数据与主权"tab 按钮
- [Source: src/services/appService.ts] — 前端 service 模式参照
- [Source: src-tauri/src/commands/app.rs] — 简单 command 模式参照
- [Source: src-tauri/src/commands/skill.rs:36] — `rfd::FileDialog::new().pick_folder()` 模式参照
- [Source: src-tauri/src/commands/chat.rs:471] — `rfd::AsyncFileDialog` 模式参照
- [Source: src-tauri/src/lib.rs:278-373] — `invoke_handler` command 注册位置
- [Source: src-tauri/src/lib.rs:56-65] — 双 DB pool 初始化和注册
- [Source: src-tauri/src/db/pool.rs:11] — `ConversationsPool` newtype wrapper
- [Source: src-tauri/src/db/roles.rs:44] — `list_all_roles` 函数
- [Source: src-tauri/src/db/tasks.rs:60] — `list_all_tasks` 函数
- [Source: src-tauri/src/db/memories.rs:357] — `list_all_memories` 函数
- [Source: src-tauri/src/db/notifications.rs:44] — `list_notifications` 函数
- [Source: src-tauri/src/db/mission.rs:6] — `get_mission` 函数
- [Source: src-tauri/src/db/briefings.rs:46] — `get_latest_briefing` 函数（参照新增 `list_all_briefings`）
- [Source: src-tauri/src/db/weekly_reviews.rs] — 周复盘 DB 模块（参照新增 `list_all_weekly_reviews`）
- [Source: src-tauri/src/db/app_settings.rs:9-18] — `get_setting` 函数（参照新增 `get_all_settings`）
- [Source: src-tauri/src/db/settings.rs:6] — `list_llm_configs` 函数
- [Source: src-tauri/src/db/mcp_servers.rs:8] — `list_mcp_servers` 函数
- [Source: src-tauri/src/db/conversations.rs:253] — `list_all_conversations` 函数
- [Source: src-tauri/src/db/conversations.rs:288] — `list_messages` 函数（参照新增 `list_all_messages`）
- [Source: src-tauri/src/db/suggestions.rs:61] — `list_pending_suggestions` 函数（参照新增 `list_all_suggestions`）
- [Source: src-tauri/migrations/001_initial_schema.sql] — `llm_configs` + `app_settings` 表结构
- [Source: src-tauri/migrations/002_conversations.sql] — `conversations` + `messages` 表结构
- [Source: src-tauri/migrations/003_roles.sql] — `roles` 表结构
- [Source: src-tauri/migrations/004_memories.sql] — `memories` 表结构
- [Source: src-tauri/migrations/013_tasks.sql] — `tasks` 表结构
- [Source: src-tauri/migrations/016_suggestions.sql] — `suggestions` 表结构
- [Source: src-tauri/migrations/017_notifications.sql] — `notifications` 表结构
- [Source: src-tauri/migrations/020_mission.sql] — `mission` 表结构
- [Source: src-tauri/migrations/021_conflicts.sql] — `conflicts` 表结构
- [Source: src-tauri/migrations/023_briefings.sql] — `briefings` 表结构
- [Source: src-tauri/migrations/025_weekly_reviews.sql] — `weekly_reviews` 表结构
- [Source: src-tauri/Cargo.toml] — 依赖清单（`rfd`、`serde_json`、`chrono` 已存在）

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

### Review Findings (2026-06-27)

- [x] [Review][Decision-Resolved] 目录选择偏离 spec 指定方案 — spec Task 5.1 写"采用方案 A"，实际用方案 B（Rust `rfd::FileDialog` 在 `commands/data.rs:41` 内选目录）。**裁决（2026-06-27）：接受现状**，方案 B 已满足 AC1 且避免 dialog capability 声明，更优。spec 计划与实现差异已知会接受。
- [x] [Review][Patch] `export_all` Sqlite 分支冗余条件 [services/data_export.rs:522] — 已在 `ExportFormat::Sqlite` 匹配臂内，`if formats.contains(&ExportFormat::Sqlite)` 恒为真，属死逻辑。**已修复**：移除冗余条件。
- [x] [Review][Patch] WAL checkpoint 错误处理不一致 [services/data_export.rs:522-533] — 主 pool 用 `?` 传播错误，conv_pool 用 `let _ =` 静默忽略。**已修复**：两者统一为失败时记 `tracing::warn!` 不中断导出。
- [x] [Review][Patch] 取消目录选择被当作错误 [commands/data.rs:44-52] — 用户取消 rfd 文件夹对话框时返回 `ValidationError("未选择导出目录")`，前端显示红色错误框。**已修复**：command 取消时返回空 `ExportResult`，前端检测 `files` 为空时静默处理；新增前端回归测试。
- [x] [Review][Defer] 复制活动数据库一致性 [services/data_export.rs:531] — `std::fs::copy` 复制存在打开连接的 .db，`wal_checkpoint(TRUNCATE)` 已降低风险，单用户桌面场景可接受 — deferred, 设计权衡。
- [x] [Review][Defer] 通知 JOIN 反规范化导出 [services/data_export.rs:234] — `list_notifications` 返回 `NotificationWithRole`（含冗余 role 字段），未来 7-4 导入需处理 — deferred, 非本 story 范围。

### Post-Review Fixes (2026-06-27)

- [x] 导出中圆箭头不转动 — `GlobalSettingsModal.tsx` 使用 `animate-spin`（Tailwind 内置类），项目中自定义动画类为 `animate-loading-spin`（`index.css:122`）。**已修复**：改为 `animate-loading-spin`。
- [x] Markdown 文件无对话记录 — 原实现只输出对话摘要（标题+消息数），不包含消息内容。**已修复**：改为输出完整对话消息（发送者、时间、正文），思考过程用 `<details>` 折叠。
- [x] 管家数据缺失记忆 — 管家记忆 `role_id IS NULL` 未被导出。**已修复**：新增管家记忆部分。
- [x] 管家输出格式与角色不一致 — 管家数据分散为独立 `##` section，且无数据时不显示标题。**已修复**：管家部分统一为 `## 管家` → `### 任务列表` / `### 记忆条目` / `### 对话记录`，放在所有角色之前，无数据时显示"（暂无任务）"等占位符。
- [x] 导出文案优化 — "导出完整数据"→"导出数据"；"（推荐，100% 保真）"→"（推荐，可用于数据恢复）"；"JSON 数据（跨版本迁移）"→"JSON 格式（更适合跨版本数据迁移）"；"（人类可读）"→"（可读性高，不可用于数据恢复）"。
