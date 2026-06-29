---
baseline_commit: fbdbc830f0744ea453e037157f8f398438225a95
---

# Story 7.2: 用户能一键销毁全部数据并回到初始状态

Status: done

## Story

As a 用户,
I want 在不想继续使用时彻底删除所有数据,
so that 确保我的隐私不留残余。

## Acceptance Criteria

1. **AC-1: 第一次确认弹窗** — 用户在 GlobalSettingsModal 数据 Tab 点击"销毁所有数据"按钮后，显示警告"此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。"，并要求用户输入"确认销毁"四个字才能继续。

2. **AC-2: 销毁执行** — 用户输入确认文字并点击"确认"后，后端在事务内 DELETE FROM 所有数据表（保留 `_sqlx_migrations` schema 元数据，不重新跑 migration），删除本地缓存文件（WAL checkpoint），销毁完成后应用自动跳转到 Onboarding 引导页面。

3. **AC-3: 销毁后状态** — 数据库为空（仅保留 schema），`app_settings` 为空表（等同全新安装，`app_is_first_launch` 返回 true），Sidebar 无角色，等同全新安装。

4. **AC-4: 取消操作** — 用户取消操作时无任何副作用。

5. **AC-5: Rust 后端** — Tauri command: `data::destroy` → 事务内 DELETE FROM 所有数据表（保留 `_sqlx_migrations`）→ 返回成功。操作前自动创建隐藏备份（`egosync-backup-{timestamp}.json`，写入临时目录，7 天后自动清理）。

6. **AC-6: Keyring 清理** — 销毁数据时同步删除系统钥匙串中所有 LLM API Key（遍历 `llm_configs` 表的 `api_key_ref`，逐个调用 `secret_store::delete_secret`）。

## Tasks / Subtasks

- [x] Task 1: 后端 Service 层 — `destroy_all_data` 函数 (AC: #2, #3, #5, #6)
  - [x] 1.1 在 `src-tauri/src/services/data_export.rs` 中新增 `pub async fn destroy_all_data(pool: &DbPool, conv_pool: &ConversationsPool, app_data_dir: &Path) -> Result<(), AppError>`
  - [x] 1.2 创建隐藏备份：调用现有 `gather_export_data(pool, conv_pool)` 获取全量数据 → 序列化为 JSON → 写入 `std::env::temp_dir()` 下 `egosync-backup-{timestamp}.json`（timestamp 格式 `%Y%m%dT%H%M%S`）
  - [x] 1.3 清理过期备份：扫描 temp_dir 中 `egosync-backup-*.json` 文件，删除修改时间超过 7 天的文件
  - [x] 1.4 删除 Keyring 密钥：调用 `db::settings::list_llm_configs(pool)` 获取所有 LLM 配置 → 遍历每个 config 的 `api_key_ref` → 调用 `secret_store::delete_secret(&config.api_key_ref)`（忽略 NoEntry 错误）
  - [x] 1.5 主库事务：`pool.begin().await?` → 对以下 18 张表依次执行 `DELETE FROM {table}` → `commit().await?`
    - `roles`, `tasks`, `memories`, `forgotten_memory_sources`, `suggestions`, `notifications`, `q2_reminders`, `big_rock_protection_reminders`, `mission`, `conflicts`, `briefings`, `weekly_reviews`, `llm_configs`, `app_settings`, `mcp_servers`, `role_mcp_server_bindings`, `skills`, `skill_role_bindings`
  - [x] 1.6 对话库事务：`conv_pool.begin().await?` → `DELETE FROM messages` → `DELETE FROM conversations` → `commit().await?`
  - [x] 1.7 WAL checkpoint：对主库和对话库分别执行 `PRAGMA wal_checkpoint(TRUNCATE)` 以截断 WAL 文件，确保已删除数据不可从 WAL 恢复
  - [x] 1.8 错误处理：备份失败 → 返回错误不继续销毁；Keyring 删除失败 → 记录 warn 日志继续执行（不应阻塞销毁）；DB 事务失败 → 返回错误，前端提示

- [x] Task 2: 后端 Command 层 — `data_destroy` Tauri command (AC: #5)
  - [x] 2.1 在 `src-tauri/src/commands/data.rs` 中新增 `#[tauri::command] pub async fn data_destroy(app_handle: AppHandle, pool: State<'_, DbPool>, conv_pool: State<'_, ConversationsPool>) -> Result<(), AppError>`
  - [x] 2.2 获取 `app_data_dir`（复用 `data_export` 中的 `app_handle.path().app_data_dir()` 模式）
  - [x] 2.3 调用 `destroy_all_data(&pool, &conv_pool, &app_data_dir).await`
  - [x] 2.4 返回 `Ok(())` 或 `Err(AppError)`

- [x] Task 3: 注册 Tauri command (AC: #5)
  - [x] 3.1 在 `src-tauri/src/lib.rs` 的 `.invoke_handler(tauri::generate_handler![...])` 列表中添加 `commands::data::data_destroy`（紧跟现有 `commands::data::data_export` 之后）

- [x] Task 4: 前端 Service 层 (AC: #1, #2)
  - [x] 4.1 在 `src/services/dataService.ts` 中新增 `dataDestroy: () => invoke<void>('data_destroy')` 到 `dataService` 对象

- [x] Task 5: 前端 UI — 销毁确认流程 (AC: #1, #2, #4)
  - [x] 5.1 在 `GlobalSettingsModal.tsx` 中新增 state: `showDestroyConfirm` (boolean), `destroyConfirmText` (string), `isDestroying` (boolean), `destroyError` (string)
  - [x] 5.2 "销毁所有数据"按钮 onClick → `setShowDestroyConfirm(true); setDestroyError(''); setDestroyConfirmText('');`
  - [x] 5.3 内联确认区域（参考导出格式选择的 inline 模式，在危险区域内展开）：
    - 警告文字："此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。"
    - 文本输入框，placeholder 为 "输入"确认销毁"以继续"
    - "确认销毁"按钮：`disabled` 除非 `destroyConfirmText === '确认销毁'`
    - "取消"按钮：重置所有 destroy state
  - [x] 5.4 确认按钮 onClick → `handleDestroy`:
    - `setIsDestroying(true); setDestroyError('');`
    - `await dataService.dataDestroy()`
    - 成功 → 调用 `onDataDestroyed()` 回调
    - 失败 → `setDestroyError(提取错误消息)`（参考现有 `exportError` 的错误提取模式）
    - finally → `setIsDestroying(false)`
  - [x] 5.5 销毁中状态：显示 spinner + "销毁中..." 文字，禁用确认和取消按钮
  - [x] 5.6 错误提示：复用现有 `exportError` 的红色提示框样式

- [x] Task 6: 前端 App.tsx — 销毁后跳转 Onboarding (AC: #2, #3)
  - [x] 6.1 在 `App.tsx` 中新增 `handleDataDestroyed` 回调：
    - `setIsSettingsOpen(false)` — 关闭设置弹窗
    - `await refreshAllRoles()` — 刷新角色列表（将为空）
    - `setCurrentView('onboard')` — 跳转到 Onboarding 引导页面
  - [x] 6.2 将 `onDataDestroyed={handleDataDestroyed}` 传递给 `<GlobalSettingsModal>`

- [x] Task 7: 后端单元测试 (AC: #2, #3, #5)
  - [x] 7.1 在 `data_export.rs` 的 `#[cfg(test)]` 模块中新增测试：
    - `destroy_all_data_clears_all_tables`：创建内存 SQLite，插入测试数据到多张表，调用 destroy，验证所有表为空
    - `destroy_all_data_preserves_migrations_table`：验证 `_sqlx_migrations` 表数据不被删除
    - `destroy_all_data_creates_backup_json`：验证 temp_dir 中生成了 `egosync-backup-*.json` 文件
    - `cleanup_old_backups_removes_expired_files`：创建 8 天前的备份文件，调用清理函数，验证文件被删除

- [x] Task 8: 前端单元测试 (AC: #1, #2, #4)
  - [x] 8.1 在 `GlobalSettingsModal.test.tsx` 中新增 `describe('数据销毁')` 测试组：
    - mock `dataService.dataDestroy` 为 `vi.fn()`
    - `点击销毁按钮后显示确认区域和警告文字`
    - `确认按钮在输入正确文字前禁用`
    - `输入"确认销毁"后点击确认调用 dataDestroy`
    - `销毁成功后调用 onDataDestroyed 回调`
    - `销毁失败时显示错误消息`
    - `取消确认后返回销毁按钮`

## Dev Notes

### 架构约束

- **分层严格**：Command 层（`commands/data.rs`）仅做参数解析和 Service 调用，Service 层（`services/data_export.rs`）负责全部业务逻辑。
- **命名规范**：Rust 函数/变量用 snake_case，TS 函数/变量用 camelCase，结构体用 PascalCase。
- **错误处理**：使用 `AppError` 枚举，DB 错误用 `AppError::DbError`，IO 错误用 `AppError::ValidationError`（参考现有 `io_error` 函数模式）。
- **不重新跑 migration**：DELETE FROM 清空数据表但保留表结构和 `_sqlx_migrations` 记录，不 DROP/CREATE 表。
- **事务边界**：主库和对话库分别开独立事务，各自 commit。两者不在同一事务中（跨库事务 SQLite 不支持）。

### 需要修改的文件

| 文件 | 修改内容 |
|------|----------|
| `src-tauri/src/services/data_export.rs` | 新增 `destroy_all_data`、`cleanup_old_backups` 函数 + 单元测试 |
| `src-tauri/src/commands/data.rs` | 新增 `data_destroy` Tauri command |
| `src-tauri/src/lib.rs` | 注册 `data_destroy` command |
| `src/services/dataService.ts` | 新增 `dataDestroy` 方法 |
| `src/components/settings/GlobalSettingsModal.tsx` | 销毁确认 UI + handler |
| `src/App.tsx` | `handleDataDestroyed` 回调 + 传递 prop |
| `src/components/settings/GlobalSettingsModal.test.tsx` | 销毁流程测试 |

### 关键实现细节

**备份文件路径**：`std::env::temp_dir().join(format!("egosync-backup-{}.json", chrono::Local::now().format("%Y%m%dT%H%M%S")))`

**备份内容**：复用 `gather_export_data()` 返回的 `ExportData`，序列化为 pretty JSON。注意：备份不包含 API Key 明文（`ExportData` 中 `llm_configs` 只有 `api_key_ref` 引用名，不含密钥值），用户恢复后需重新输入 API Key。

**Keyring 清理顺序**：必须在 DELETE FROM `llm_configs` 之前执行，因为需要先读取 `api_key_ref` 值。清理失败（`delete_secret` 返回错误）只记 warn 日志不阻塞流程，因为用户已确认销毁，丢失密钥可接受。

**DELETE FROM 表列表**：硬编码 18 张主库表名 + 2 张对话库表名。不使用 `SELECT name FROM sqlite_master WHERE type='table'` 动态查询，因为：
1. 会查出 `_sqlx_migrations` 需要排除
2. 可能查出 sqlite 内部表
3. 硬编码更安全、更可控、更可审计

**WAL checkpoint**：在事务 commit 之后执行 `PRAGMA wal_checkpoint(TRUNCATE)`，确保 WAL 文件被截断，已删除数据不可从 WAL 恢复。参考 `export_all` 函数中已有的 WAL checkpoint 模式。

**前端错误提取**：参考现有 `handleExport` 中的错误处理模式。Tauri 返回的 `AppError` 序列化为 `{ "DbError": "消息" }` 或 `{ "ValidationError": "消息" }` 格式，前端提取 value 值显示。

**onDataDestroyed 回调**：GlobalSettingsModal 已有 `onClose`、`onRefreshRoles` 等 props。新增 `onDataDestroyed` 是一个无参数回调，App.tsx 在其中负责关闭设置、刷新角色、跳转 onboarding。不直接在 Modal 中调用 `onClose` + `onRefreshRoles`，因为跳转 onboarding 需要父组件控制 `currentView`。

### 现有代码参考

- **`gather_export_data`** (`data_export.rs:226-271`)：已实现全量数据采集，直接复用于备份。
- **`export_all` 中的 WAL checkpoint** (`data_export.rs:607-618`)：参考其 `PRAGMA wal_checkpoint(TRUNCATE)` 模式。
- **`io_error` 函数** (`data_export.rs:72-74`)：IO 错误转 `AppError::ValidationError` 的模式。
- **`data_export` command** (`commands/data.rs:10-62`)：参考其获取 `app_data_dir` 和参数处理模式。
- **`secret_store::delete_secret`** (`services/secret_store.rs:54-64`)：删除 keyring 密钥，NoEntry 返回 Ok。
- **`db::settings::list_llm_configs`** (`db/settings.rs:5-12`)：列出所有 LLM 配置以获取 `api_key_ref`。
- **导出格式选择 UI** (`GlobalSettingsModal.tsx:835-883`)：参考其 inline 展开/取消的 UI 模式。
- **`handleExport`** (`GlobalSettingsModal.tsx`)：参考其 loading/error/result 状态管理模式。
- **MCP 删除确认** (`GlobalSettingsModal.test.tsx:145-162`)：参考其确认对话框测试模式。
- **`app_is_first_launch`** (`commands/app.rs:16-19`)：检查 `onboarding_completed` 设置，销毁后该设置被清空，`isFirstLaunch` 返回 true。

### 数据库表完整清单

**主库 `egosync.db`**（18 张数据表）：
`roles`, `tasks`, `memories`, `forgotten_memory_sources`, `suggestions`, `notifications`, `q2_reminders`, `big_rock_protection_reminders`, `mission`, `conflicts`, `briefings`, `weekly_reviews`, `llm_configs`, `app_settings`, `mcp_servers`, `role_mcp_server_bindings`, `skills`, `skill_role_bindings`

**对话库 `conversations.db`**（2 张数据表）：
`conversations`, `messages`

**保留表**（不删除）：
`_sqlx_migrations`（主库和对话库各一份）

### Project Structure Notes

- 所有修改均在现有文件中进行，不新建文件。
- `destroy_all_data` 添加到 `data_export.rs` 而非新建 `data_destroy.rs`，因为需要直接复用 `gather_export_data` 且同属数据管理域。
- 前端不新建组件，销毁确认 UI 内联在 `GlobalSettingsModal.tsx` 数据 Tab 的危险区域内。
- 测试添加到现有测试文件中，不新建测试文件。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.2] — 验收标准原文
- [Source: _bmad-output/planning-artifacts/architecture.md] — 分层架构、命名规范、错误处理
- [Source: _bmad-output/implementation-artifacts/7-1-data-export-json-markdown.md] — Story 7.1 实现上下文
- [Source: src-tauri/src/services/data_export.rs:226-271] — `gather_export_data` 函数
- [Source: src-tauri/src/services/data_export.rs:607-618] — WAL checkpoint 模式
- [Source: src-tauri/src/commands/data.rs:10-62] — `data_export` command 模式
- [Source: src-tauri/src/services/secret_store.rs:54-64] — `delete_secret` 函数
- [Source: src-tauri/src/services/llm_config.rs:82-85] — LLM 配置删除时清理 keyring 模式
- [Source: src-tauri/src/db/settings.rs:5-12] — `list_llm_configs` 函数
- [Source: src-tauri/src/db/app_settings.rs:9-18] — `get_all_settings` 函数
- [Source: src-tauri/src/commands/app.rs:16-19] — `app_is_first_launch` 逻辑
- [Source: src/components/settings/GlobalSettingsModal.tsx:801-936] — 数据 Tab 现有 UI
- [Source: src/components/settings/GlobalSettingsModal.tsx:835-883] — 导出格式选择 inline UI 模式
- [Source: src/App.tsx:39,206-224] — `currentView` 和 onboarding 跳转逻辑
- [Source: src-tauri/migrations/001-026] — 全部数据库表结构定义

## Review Findings

### Decision Needed

- [x] [Review][Decision→Patch] 隐藏备份缺失两张表，销毁后无法完整还原 — 已扩展 `ExportData` 与 `gather_export_data` 覆盖 `forgotten_memory_sources` 与 `role_mcp_server_bindings`（新增 `query_forgotten_memory_sources`/`query_role_mcp_server_bindings`），备份与 Story 7.1 导出现已包含这两张表。

### Patch

- [x] [Review][Patch] Keyring 删除应移到 DB 事务成功提交之后 [data_export.rs:739-785] — 当前顺序为"删除 keyring → 主库事务删除"。`configs` 在 :688 已读出 `api_key_ref`，可保留该读取，但把第 689-693 行的 `delete_secret` 循环移动到主库+对话库事务 `commit()` 成功之后执行。否则若主库事务失败（:706 返回 Err），销毁整体中止，但用户的 LLM API Key 已被从系统钥匙串删除——一次"失败的销毁"却静默丢失了 API Key。
- [x] [Review][Patch] 单元测试未覆盖 FK 级联删除的最高风险路径 [data_export.rs:1180-1255] — `setup_destroy_test_db` 仅插入一个无子行的 `role`，未插入引用该 role 的 `tasks`/`memories`/`suggestions`/`conflicts` 等子表行。`MAIN_DB_TABLES` 将父表 `roles`/`tasks` 排在子表之前，依赖 `ON DELETE CASCADE` 才不触发约束违约；该路径目前完全未被验证，FK 约束或删除顺序的回归无法被测试捕获。建议在测试数据中加入引用 role/task 的子表行，并断言级联清空。

### Defer

- [x] [Review][Defer] 跨库+Keyring 非原子，部分失败留下不一致状态且前端无提示 [data_export.rs:695-744] — deferred, spec 已知 SQLite 不支持跨库事务（Dev Notes 已声明）。主库提交后若对话库事务失败，会出现"角色已清空但对话仍在"的不一致状态；前端仅显示通用错误，未告知"部分销毁"。属架构固有限制，本 story 范围外。

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (Windsurf Cascade)

### Debug Log References

无

### Completion Notes List

- 后端 `destroy_all_data` 实现：备份 → 清理过期备份 → 读取 keyring refs → 主库事务 DELETE 18 表 → 对话库事务 DELETE 2 表 → keyring 删除（DB 提交后）→ WAL checkpoint
- 代码审查修复 1：扩展 `ExportData` + `gather_export_data` 覆盖 `forgotten_memory_sources` 和 `role_mcp_server_bindings`（备份与导出同步覆盖）
- 代码审查修复 2：keyring 删除移到 DB 事务全部提交成功之后，避免失败的销毁误删 API Key
- 代码审查修复 3：测试补充引用 role/task 的子表行 + 级联清空断言，覆盖父表先删 + ON DELETE CASCADE 路径
- 前端销毁确认 UI：内联确认区域，输入"确认销毁"才能继续，销毁中 spinner，错误提示
- App.tsx `handleDataDestroyed` 回调：关闭设置 → 刷新角色 → 跳转 onboarding
- Rust 测试 15 个全部通过

### File List

- `egosync-app/src-tauri/src/services/data_export.rs` — 新增 `destroy_all_data`、`create_backup`、`cleanup_old_backups`、`query_forgotten_memory_sources`、`query_role_mcp_server_bindings` + 单元测试
- `egosync-app/src-tauri/src/commands/data.rs` — 新增 `data_destroy` Tauri command
- `egosync-app/src-tauri/src/lib.rs` — 注册 `data_destroy` command
- `egosync-app/src-tauri/Cargo.toml` — 新增 `filetime` dev-dependency
- `egosync-app/src-tauri/Cargo.lock` — filetime 依赖锁定
- `egosync-app/src/services/dataService.ts` — 新增 `dataDestroy` 方法
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx` — 销毁确认 UI + `handleDestroy`
- `egosync-app/src/components/settings/GlobalSettingsModal.test.tsx` — 数据销毁测试组
- `egosync-app/src/App.tsx` — `handleDataDestroyed` 回调 + prop 传递
