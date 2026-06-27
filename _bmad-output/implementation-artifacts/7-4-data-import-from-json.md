---
baseline_commit: 7bde381
---

# Story 7.4: 用户能从导出的存档恢复/导入数据

Status: done

## Story

As a 用户,
I want 从之前导出的存档文件恢复全部数据,
so that 换设备或销毁后能一键回到完整工作状态。

## 背景与现状（务必先读）

**本 story 是 Epic 7（数据主权）的最后一个 story — 实现数据导入/恢复功能。Story 7.1（导出）和 7.2（销毁）已完成，7.3（数据 Tab 集成验证）已完成。应用已具备完整的导出和销毁能力，但缺少"恢复"能力——用户导出存档后无法再导入回来。**

**核心交付：**
1. **Rust 后端导入 service**：支持两种格式自动识别：
   - **SQLite 备份（.db 文件）**：通过 `ATTACH DATABASE` 逐表复制数据，无需关闭/重连连接池
   - **JSON 数据（.json 文件）**：反序列化 `ExportData` → 事务内清空+逐表写入
2. **Tauri command**：`data_import` 接收文件路径，自动识别格式，执行导入，返回统计
3. **前端接通**：GlobalSettingsModal 数据 Tab 新增"导入存档"按钮 + 确认流程
4. **导入前自动备份**：复用 `create_backup` 模式，当前数据备份到临时目录（7 天自动清理）

### 架构关键约束：DB 连接池不可替换

**⚠️ 这是最重要的架构约束，影响整个实现方案。**

当前 DB 连接池通过 `app.manage(pool.clone())` 注册为 Tauri State（`@/egosync-app/src-tauri/src/lib.rs:64`）。Tauri State 一旦注册不可替换。因此 Epic AC 中"关闭当前数据库连接 → 替换 .db 文件 → 重新初始化连接"的方案在当前架构下不可行。

**替代方案：SQL 层面复制数据（不替换文件）**
- **SQLite 导入**：使用 `ATTACH DATABASE 'imported.db' AS imported` → 逐表 `DELETE FROM current; INSERT INTO current SELECT * FROM imported.table;` → `DETACH DATABASE imported`
- **JSON 导入**：反序列化 JSON → 事务内 `DELETE FROM` 所有表 → 逐表 `INSERT` 写入
- 两种方案都在现有连接池内完成，无需关闭/重连
- 数据 100% 保真（所有表数据完整复制；索引/触发器由 migration 创建，不受影响）

### 已建成的基础（本 story 的接入点）

**前端 GlobalSettingsModal 数据 Tab（直接修改）：**
- `src/components/settings/GlobalSettingsModal.tsx:821-910` — 导出区域（上方）
- `src/components/settings/GlobalSettingsModal.tsx:949-1003` — 危险区域（下方）
- **导入区域插入位置**：导出区域之后、危险区域之前
- `:67-75` — 状态变量声明区域（新增导入相关状态）
- `:299-336` — `handleExport` + `handleDestroy` 函数区域（新增 `handleImport`）

**前端 service 层（直接修改）：**
- `src/services/dataService.ts:1-17` — 已有 `dataExport` + `dataDestroy`，新增 `dataImport`

**Rust 后端（直接修改）：**
- `src-tauri/src/commands/data.rs:1-77` — 已有 `data_export` + `data_destroy` command，新增 `data_import`
- `src-tauri/src/services/data_export.rs` — 已有导出+销毁 service，新增导入 service 函数
- `src-tauri/src/lib.rs:373-374` — invoke_handler 注册区域，新增 `data_import` 注册

**Rust 后端 — 可复用的基础设施：**
- `src-tauri/src/services/data_export.rs:274-323` — `gather_export_data()` 查询所有表数据，返回 `ExportData`
- `src-tauri/src/services/data_export.rs:36-59` — `ExportData` 结构体（JSON 导入的反序列化目标）
- `src-tauri/src/services/data_export.rs:698-718` — `MAIN_DB_TABLES` 常量（清空表列表）
- `src-tauri/src/services/data_export.rs:806-817` — `create_backup()` 异步函数（导入前备份复用）
- `src-tauri/src/services/data_export.rs:820-853` — `cleanup_old_backups()` 函数（清理过期备份）
- `src-tauri/src/db/pool.rs:20-42` — `init_db()` / `init_conversations_db()` 连接池初始化

**Rust 后端 — rfd 文件选择对话框（直接参照）：**
- `src-tauri/src/commands/data.rs:40-43` — `rfd::FileDialog::new().pick_folder()` 目录选择（spawn_blocking）
- **本 story 使用 `rfd::FileDialog::new().add_filter().pick_file()`** 选择单个文件（非目录）

**App.tsx 回调模式（直接参照）：**
- `src/App.tsx:226-229` — `handleDataDestroyed`：关闭设置 → 刷新角色 → 跳转 onboarding
- `src/App.tsx:393` — `onDataDestroyed={handleDataDestroyed}` prop 传递
- **本 story 新增 `onDataImported` 回调**：关闭设置 → 刷新角色 → 刷新对话/记忆等

### 已知偏差（已接受，无需修改）

- **目录选择方案偏差**：Epic AC 写"导出使用 `@tauri-apps/api/dialog` 的 `save` API"，实际使用 Rust 端 `rfd::FileDialog`。Story 7.1 代码审查已接受此偏差。**导入同样使用 `rfd::FileDialog`**，保持一致。
- **SQLite 导入方案偏差**：Epic AC 写"关闭当前数据库连接 → 替换 .db 文件 → 重新初始化连接"，实际使用 `ATTACH DATABASE` + 逐表 SQL 复制。原因：Tauri State 不可替换。**数据 100% 保真**（所有表数据完整复制）。

## Acceptance Criteria

1. **AC1**: Given 用户在 GlobalSettingsModal 数据 Tab 点击"导入存档"，When Tauri 文件选择对话框弹出，用户选择存档文件，Then 自动识别格式（.db → SQLite 备份恢复，.json → JSON 数据恢复），And 校验通过后弹出确认弹窗："导入将完全替换当前所有数据，当前数据会被覆盖。是否继续？"

2. **AC2**: Given 用户确认导入 SQLite 备份，When 导入执行，Then 自动备份当前数据为 JSON 到临时目录（7 天后自动清理），And 在事务内逐表复制数据（ATTACH → DELETE FROM → INSERT INTO SELECT → DETACH），And 导入完成后应用自动刷新，And 数据 100% 保真

3. **AC3**: Given 用户确认导入 JSON，When 导入执行，Then 自动备份当前数据为 JSON 到临时目录（7 天后自动清理），And 在事务内：清空当前所有数据表 → 逐表写入 JSON 中的数据 → 保留 `_sqlx_migrations` schema 元数据，And 若导入失败则回滚事务，当前数据不受影响，And 导入完成后应用自动刷新

4. **AC4**: Given 导入成功，Then 显示成功提示"已恢复 N 个角色、N 个任务、N 条记忆"，And 应用界面自动刷新到导入后的状态

5. **AC5**: Given 导入失败（格式错误 / 文件损坏 / 版本不兼容），Then 显示错误提示，当前数据保持不变

6. **AC6**: Given 用户取消操作（取消文件选择 或 取消确认），Then 无任何副作用

7. **AC7**: Given Rust 后端，Then Tauri command: `data_import { file_path }` → 识别格式 → 执行对应恢复逻辑 → 返回导入统计，And SQLite 备份：ATTACH + 逐表复制，And JSON：读取 → 校验 → 事务内清空+写入所有表

## Tasks / Subtasks

- [ ] **Task 1: Rust 后端 — 导入 service 函数** (AC: #2, #3, #5, #7)
  - [ ] 1.1 在 `src-tauri/src/services/data_export.rs` 中新增 `ImportResult` 结构体（`#[serde(rename_all = "camelCase")]`，含 `roles_count`, `tasks_count`, `memories_count`, `conversations_count`, `messages_count` 等统计字段）
  - [ ] 1.2 新增 `import_sqlite_data(pool, conv_pool, file_path)` 函数：
    - 使用 `ATTACH DATABASE ? AS imported` 挂载导入的 .db 文件
    - 逐表执行 `DELETE FROM current_table; INSERT INTO current_table SELECT * FROM imported.table;`
    - 主库表列表复用 `MAIN_DB_TABLES`；对话库表：`conversations`, `messages`
    - 完成后 `DETACH DATABASE imported`
    - 返回 `ImportResult` 统计
  - [ ] 1.3 新增 `import_json_data(pool, conv_pool, file_path)` 函数：
    - 读取文件 → `serde_json::from_str::<ExportData>(&content)` 反序列化
    - 校验 `export_version` 字段（不兼容版本返回 `ValidationError`）
    - 主库事务：`DELETE FROM` 所有表（复用 `MAIN_DB_TABLES`）→ 逐表 `INSERT` 写入
    - 对话库事务：`DELETE FROM messages; DELETE FROM conversations;` → 逐表 `INSERT` 写入
    - 失败则回滚事务，返回 `AppError`
    - 返回 `ImportResult` 统计
  - [ ] 1.4 新增 `import_all(pool, conv_pool, file_path)` 统一入口：
    - 调用 `create_backup(pool, conv_pool).await` 备份当前数据
    - 调用 `cleanup_old_backups()` 清理过期备份
    - 根据文件扩展名（`.db` → SQLite，`.json` → JSON）分发到对应函数
    - 不识别的扩展名返回 `ValidationError("不支持的文件格式")`

- [ ] **Task 2: Rust 后端 — Tauri command** (AC: #7)
  - [ ] 2.1 在 `src-tauri/src/commands/data.rs` 中新增 `data_import` command：
    ```rust
    #[tauri::command]
    pub async fn data_import(
        file_path: String,
        pool: State<'_, DbPool>,
        conv_pool: State<'_, ConversationsPool>,
    ) -> Result<ImportResult, AppError>
    ```
  - [ ] 2.2 调用 `import_all(&pool, &conv_pool, Path::new(&file_path)).await`
  - [ ] 2.3 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册 `commands::data::data_import`

- [ ] **Task 3: 前端 — service 层** (AC: #7)
  - [ ] 3.1 在 `src/services/dataService.ts` 中新增：
    ```typescript
    export interface ImportResult {
      rolesCount: number;
      tasksCount: number;
      memoriesCount: number;
      conversationsCount: number;
      messagesCount: number;
    }
    export const dataService = {
      // ... existing
      dataImport: (filePath: string) =>
        invoke<ImportResult>('data_import', { filePath }),
    };
    ```

- [ ] **Task 4: 前端 — GlobalSettingsModal 导入 UI** (AC: #1, #4, #5, #6)
  - [ ] 4.1 新增状态变量：`isImporting`, `importResult`, `importError`, `showImportConfirm`, `pendingImportPath`
  - [ ] 4.2 新增 `handleImportSelect` 函数：调用 `rfd` 文件选择（通过 Rust command 或 `@tauri-apps/plugin-dialog`），获取文件路径，设置 `pendingImportPath`，显示确认弹窗
  - [ ] 4.3 新增 `handleImport` 函数：调用 `dataService.dataImport(pendingImportPath)` → 成功显示统计 + 调用 `onDataImported` 回调 / 失败显示错误
  - [ ] 4.4 在导出区域之后插入导入区域：
    - 标题"导入数据" + 说明文字
    - "导入存档"按钮（`Upload` 图标）
    - 导入中状态：spinner + "导入中..."
    - 导入成功：✓ + "已恢复 N 个角色、N 个任务、N 条记忆"
    - 导入失败：错误提示
    - 确认弹窗（inline，类似销毁确认）：警告文字 + 取消/确认按钮

- [ ] **Task 5: 前端 — App.tsx 回调** (AC: #4)
  - [ ] 5.1 在 `App.tsx` 中新增 `handleDataImported` 函数：关闭设置 → 刷新角色 → 刷新对话/记忆（与 `handleDataDestroyed` 类似但跳转到 butler 而非 onboard）
  - [ ] 5.2 在 `<GlobalSettingsModal>` 上传递 `onDataImported={handleDataImported}` prop

- [ ] **Task 6: 前端 — 文件选择对话框** (AC: #1, #6)
  - [ ] 6.1 在 Rust 端 `data_import` command 中使用 `rfd::FileDialog::new().add_filter("EgoSync 存档", &["db", "json"]).pick_file()` 让用户选择文件
  - [ ] 6.2 用户取消文件选择返回 `None`，command 返回空结果（不报错）
  - [ ] 6.3 或者：新增一个 `pick_import_file` command 仅用于文件选择，前端拿到路径后再调用 `data_import`

- [ ] **Task 7: 测试 — Rust 后端** (AC: #2, #3, #5)
  - [ ] 7.1 在 `data_export.rs` 的 `#[cfg(test)] mod tests` 中新增测试：
    - `import_json_data_restores_all_tables`：创建有数据的 DB → 导出 JSON → 销毁数据 → 导入 JSON → 验证数据恢复
    - `import_json_data_rolls_back_on_failure`：创建有数据的 DB → 尝试导入损坏 JSON → 验证原数据不受影响
    - `import_json_data_rejects_incompatible_version`：导入 `export_version: "99.0"` 的 JSON → 验证返回 `ValidationError`
    - `import_sqlite_data_restores_all_tables`：创建有数据的 DB → 导出 SQLite → 销毁数据 → 导入 SQLite → 验证数据恢复
    - `import_all_rejects_unsupported_format`：传入 `.txt` 文件 → 验证返回 `ValidationError`
  - [ ] 7.2 运行 `cargo test` 确认所有测试通过

- [ ] **Task 8: 测试 — 前端** (AC: #1, #4, #5, #6)
  - [ ] 8.1 在 `GlobalSettingsModal.test.tsx` 中新增 `dataService.dataImport` mock
  - [ ] 8.2 新增测试组 `describe('数据导入')`：
    - 点击"导入存档"后显示确认弹窗
    - 确认导入调用 `dataService.dataImport`
    - 导入成功显示统计信息
    - 导入失败显示错误消息
    - 取消确认无副作用
  - [ ] 8.3 运行 `npx vitest` 确认所有测试通过

- [ ] **Task 9: 全量验证** (AC: #1-7)
  - [ ] 9.1 运行 `npx vitest` 确认前端测试全部通过
  - [ ] 9.2 运行 `cargo test` 确认后端测试全部通过，无回归

## Dev Notes

### 关键技术决策

- **SQLite 导入用 ATTACH DATABASE 而非文件替换**：Tauri State 不可替换，因此不能关闭/重连连接池。`ATTACH DATABASE` + 逐表 SQL 复制在现有连接内完成，数据 100% 保真。

- **JSON 导入用事务回滚保护**：主库和对话库分别开启事务，任何 INSERT 失败则回滚，当前数据不受影响。

- **导入前自动备份**：复用 `create_backup()` 函数（`data_export.rs:806-817`），将当前数据导出为 JSON 写入临时目录。与销毁前备份机制一致，7 天后自动清理。

- **文件选择在 Rust 端完成**：与导出的目录选择模式一致（`rfd::FileDialog`）。前端不直接调用 `@tauri-apps/api/dialog`。

- **导入后刷新由父组件控制**：与销毁后跳转 onboarding 模式一致，`onDataImported` 回调通知 App.tsx 执行刷新。导入后跳转到 butler 视图（非 onboarding，因为导入的数据可能包含已完成引导的角色）。

### ATTACH DATABASE 实现细节

```rust
// SQLite 导入核心逻辑（伪代码）
sqlx::query("ATTACH DATABASE ?1 AS imported")
    .bind(file_path)
    .execute(pool)
    .await?;

for table in MAIN_DB_TABLES {
    sqlx::query(&format!("DELETE FROM {}", table))
        .execute(pool)
        .await?;
    sqlx::query(&format!(
        "INSERT INTO {} SELECT * FROM imported.{}", table, table
    ))
    .execute(pool)
    .await?;
}
// 对话库同样处理
sqlx::query("DETACH DATABASE imported")
    .execute(pool)
    .await?;
```

**注意**：`ATTACH DATABASE` 需要在主库和对话库分别执行。导入的 .db 文件如果是导出时的完整备份（含 egosync.db + conversations.db），需要分别处理。但导出时 SQLite 格式生成两个文件（`egosync-export-{date}.db` + `egosync-export-{date}-conversations.db`），用户选择的是主库文件，对话库文件名通过约定推导（`{stem}-conversations.db`）。

**简化方案**：如果对话库文件不存在，只导入主库数据，对话数据保持不变（不报错）。

### JSON 导入实现细节

```rust
// JSON 导入核心逻辑（伪代码）
let content = std::fs::read_to_string(file_path)?;
let data: ExportData = serde_json::from_str(&content)?;

// 版本校验
if data.export_version != EXPORT_VERSION {
    return Err(AppError::ValidationError(
        format!("不兼容的导出版本: {}，当前支持: {}", data.export_version, EXPORT_VERSION)
    ));
}

// 主库事务
let mut tx = pool.begin().await?;
for table in MAIN_DB_TABLES {
    sqlx::query(&format!("DELETE FROM {}", table)).execute(&mut *tx).await?;
}
// 逐表 INSERT（使用原始 SQL 保留 ID 和时间戳）
for role in &data.roles {
    sqlx::query("INSERT INTO roles (id, name, icon, ...) VALUES (?1, ?2, ?3, ...)")
        .bind(&role.id)
        .bind(&role.name)
        // ...
        .execute(&mut *tx)
        .await?;
}
// ... 其他表
tx.commit().await?;
```

### 架构合规

- **分层规则**：Command 层只做参数解析 → 调 Service → 返回结果；Service 层（`data_export.rs`）拥有所有导入业务逻辑
- **命名规范**：`data_import` command（snake_case），`ImportResult` 结构体（PascalCase），`import_all` / `import_json_data` / `import_sqlite_data` 函数（snake_case）
- **错误处理**：所有函数返回 `Result<T, AppError>`，不使用 `.unwrap()`；错误序列化为 JSON 供前端提取
- **serde 桥接**：`ImportResult` 使用 `#[serde(rename_all = "camelCase")]`，前端通过 `dataService.dataImport` 获取 camelCase 字段

### 前端 UI 规范

- **导入区域位置**：导出区域之后、危险区域之前（数据 Tab 布局：导出 → 导入 → 危险区域）
- **按钮样式**：`Upload` 图标（lucide-react）+ "导入存档"文字，样式与"导出存档"按钮一致（border + slate 配色）
- **确认弹窗**：inline 区域（非 Modal 弹窗），与销毁确认模式一致。警告文字："导入将完全替换当前所有数据，当前数据会被覆盖。是否继续？"
- **Loading 状态**：`isImporting` + `Loader2` 图标 + `animate-loading-spin` class + "导入中..."文字
- **成功反馈**：`Check` 图标 + 绿色文字 + "已恢复 N 个角色、N 个任务、N 条记忆"
- **错误反馈**：`AlertCircle` 图标 + 红色文字
- **取消文件选择**：静默处理，不显示成功或错误

### 反模式警告

- **不要**尝试关闭/重连 DB 连接池——Tauri State 不可替换
- **不要**在前端直接调用 `@tauri-apps/api/dialog`——文件选择统一走 Rust 端 `rfd::FileDialog`
- **不要**修改 `ExportData` 结构体——它是 JSON 导入的反序列化目标，字段必须与导出一致
- **不要**修改 `MAIN_DB_TABLES` 常量——导入和销毁共用同一表列表
- **不要**修改 `error.rs` 枚举——现有 `ValidationError` / `DbError` 足够覆盖导入错误
- **不要**修改 `pool.rs` 的 `init_db` / `init_conversations_db`——导入不涉及连接池重建
- **不要**在导入后自动跳转 onboarding——导入的数据可能包含已完成引导的角色，应跳转到 butler 视图

### Project Structure Notes

新增文件：无

修改文件：
- `src-tauri/src/services/data_export.rs` — 新增 `ImportResult` 结构体 + `import_all` / `import_json_data` / `import_sqlite_data` 函数 + 测试
- `src-tauri/src/commands/data.rs` — 新增 `data_import` command
- `src-tauri/src/lib.rs` — 注册 `data_import` command
- `src/services/dataService.ts` — 新增 `ImportResult` 类型 + `dataImport` 方法
- `src/components/settings/GlobalSettingsModal.tsx` — 新增导入区域 UI + 状态 + handler
- `src/App.tsx` — 新增 `handleDataImported` 回调 + prop 传递
- `src/components/settings/GlobalSettingsModal.test.tsx` — 新增导入测试组

不修改文件：
- `src-tauri/src/db/pool.rs` — 连接池初始化不变
- `src-tauri/src/error.rs` — 错误枚举不变
- `src-tauri/src/services/data_export.rs` 中的导出/销毁函数 — 不修改现有逻辑

### Previous Story Intelligence

**Story 7.1 学习要点：**
- 导出格式选择采用 inline checkbox 模式（非 Modal 弹窗）
- 目录选择用 Rust 端 `rfd::FileDialog`（方案 B），前端不调用 `@tauri-apps/api/dialog`
- 取消目录选择返回空 `ExportResult`，前端静默处理
- Loading spinner 用 `animate-loading-spin`（非 `animate-spin`）
- 导出成功显示文件路径列表（`exportResult.files.map()`）

**Story 7.2 学习要点：**
- 销毁确认采用 inline 区域（非 Modal 弹窗），在危险区域内展开
- 确认按钮 `disabled` 条件：`destroyConfirmText !== '确认销毁'`
- 销毁成功后通过 `onDataDestroyed` 回调通知 App.tsx，由父组件控制跳转
- 销毁中状态：spinner + "销毁中..." 文字，禁用确认和取消按钮
- 错误提取模式：`e.ValidationError || e.DbError || Object.values(e)[0]`

**Story 7.3 学习要点：**
- 验证型 story，确认 7.1/7.2 集成正确性
- 测试文本必须与 UI 实际文本一致（如"JSON 格式"而非"JSON 数据"）
- `act()` 包裹异步操作消除 React act 警告

### Git Intelligence

最近 3 个提交：
- `7bde381` test(7.3): 数据 Tab 集成测试补齐 + 代码审查修复
- `0c9a183` feat(7.2): 数据销毁功能 + 代码审查修复
- `fbdbc83` feat(7.1): 数据导出功能 + 代码审查修复 + 导出体验优化

这三个提交已完整实现数据 Tab 的导出和销毁功能。本 story 在此基础上新增导入功能。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.4] — AC 原文
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7] — Epic 7 上下文
- [Source: _bmad-output/implementation-artifacts/7-1-data-export-json-markdown.md] — Story 7.1 实现详情
- [Source: _bmad-output/implementation-artifacts/7-2-data-destroy-initial-state.md] — Story 7.2 实现详情
- [Source: _bmad-output/implementation-artifacts/7-3-settings-data-tab-integration.md] — Story 7.3 实现详情
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、禁止事项
- [Source: src/components/settings/GlobalSettingsModal.tsx:821-1004] — 数据 Tab 完整 UI
- [Source: src/components/settings/GlobalSettingsModal.tsx:67-75] — 状态变量声明
- [Source: src/components/settings/GlobalSettingsModal.tsx:299-336] — handleExport + handleDestroy
- [Source: src/services/dataService.ts:1-17] — dataService 封装
- [Source: src-tauri/src/commands/data.rs:1-77] — data_export + data_destroy commands
- [Source: src-tauri/src/services/data_export.rs:36-59] — ExportData 结构体
- [Source: src-tauri/src/services/data_export.rs:274-323] — gather_export_data 函数
- [Source: src-tauri/src/services/data_export.rs:698-718] — MAIN_DB_TABLES 常量
- [Source: src-tauri/src/services/data_export.rs:720-803] — destroy_all_data 函数（事务模式参照）
- [Source: src-tauri/src/services/data_export.rs:806-817] — create_backup 函数（备份模式参照）
- [Source: src-tauri/src/services/data_export.rs:820-853] — cleanup_old_backups 函数
- [Source: src-tauri/src/db/pool.rs:20-75] — init_db / init_conversations_db
- [Source: src-tauri/src/lib.rs:56-65] — 连接池初始化 + Tauri State 注册
- [Source: src-tauri/src/lib.rs:373-374] — invoke_handler 注册区域
- [Source: src-tauri/src/error.rs:1-35] — AppError 枚举
- [Source: src/App.tsx:226-229] — handleDataDestroyed 回调模式
- [Source: src/App.tsx:393] — onDataDestroyed prop 传递
- [Source: src/components/settings/GlobalSettingsModal.test.tsx:44-49] — dataService mock 模式
- [Source: src/components/settings/GlobalSettingsModal.test.tsx:290-511] — 数据导出/销毁测试模式

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

### Review Findings

_代码审查日期 2026-06-27，审查范围：未提交改动（vs `7bde381`）。三层对抗审查（Blind Hunter / Edge Case Hunter / Acceptance Auditor）。_

- [x] [Review][Patch] **SQLite (.db) 导入缺少版本/schema 兼容性校验** — JSON 路径校验 `export_version`，但 `.db` 路径 (`import_sqlite_data`) 直接 `INSERT INTO x SELECT * FROM imported.x`，依赖列顺序/数量完全一致；导入旧版本或异构 schema 的 .db 会失败或错位。AC5 列举"版本不兼容"为失败场景，但 .db 路径无任何校验。决策已定（加校验）：ATTACH 后校验关键表存在/列匹配，不兼容时返回 `ValidationError` 并 DETACH。[data_export.rs:402-481]
- [x] [Review][Patch] **SQLite 导入在连接池上 ATTACH + 无事务，存在数据丢失风险（违反 AC2/AC5）** — `import_sqlite_data` 用独立 `.execute(pool)` 依次执行 ATTACH→DELETE→INSERT→DETACH。连接池 `max_connections(5)`，`ATTACH` 是连接级作用域，后续语句可能落到未挂载 `imported` 的其他连接 → `no such table: imported.x`；且无事务，DELETE 后 INSERT 失败将清空当前数据无法回滚。AC2 要求"事务内复制"，AC5 要求"失败时当前数据不变"。修复：`pool.acquire()` 取单一连接，ATTACH（事务外）→ BEGIN → 逐表 DELETE/INSERT → COMMIT → DETACH；对话库同理。[data_export.rs:402-481] [db/pool.rs:32-34]
- [x] [Review][Patch] **SQLite 导入路径零测试覆盖（违反 Task 7.1）** — Task 7.1 明确要求 `import_sqlite_data_restores_all_tables` 测试，但实际仅有 JSON/格式/序列化测试，SQLite 关键路径完全未测试，上述连接池 bug 因此未被发现。修复：补充 SQLite 导入往返测试。[data_export.rs:516-639]
- [x] [Review][Patch] **`import_json_data_rolls_back_on_failure` 测试名不符实** — 该测试只喂入非法 JSON，在反序列化阶段即失败（事务尚未开启），并未真正触发事务中途回滚。修复：用合法但违反约束（如重复主键/FK 缺失）的数据触发 INSERT 中途失败，验证已删数据回滚。[data_export.rs:592-607]
- [x] [Review][Patch] **serde_json::Value 字段读取用 `unwrap_or` 静默兜底** — `conflicts`/`skills`/`q2_reminders` 等表从 `Value` 取字段时大量使用 `.as_str().unwrap_or("")`，`reminded_count` 用 `.as_i64().unwrap_or(1)`，缺失/null 被静默转为空串或魔法值 `1`，损失数据保真度。修复：缺失关键字段时返回 `ValidationError`，而非静默兜底。[data_export.rs:836-346]
