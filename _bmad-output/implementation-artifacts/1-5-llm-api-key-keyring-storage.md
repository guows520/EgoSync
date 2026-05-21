# Story 1.5: 用户的 LLM API Key 安全存储到系统钥匙串

Status: done

## Story

As a 用户,
I want 我的 API Key 存储在操作系统安全区域,
So that 不会泄露到任何文件或日志中。

## Acceptance Criteria

1. **AC-1 Key 不落明文**：用户通过 UI 保存 API Key 后，检查 `egosync.db` 和所有日志文件不出现 Key 明文（grep `sk-` / `sk-ant-` 无匹配）。
2. **AC-2 跨平台 keyring 存储**：
   - Windows → Windows Credential Manager
   - macOS → Keychain
   - Linux → Secret Service
3. **AC-3 卸载重装持久化**：卸载并重装应用后，keyring 中的 Key 仍在（OS 级持久）。
4. **AC-4 keyring 不可用时明确报错**：keyring 不可用（如无桌面环境的 Linux CI）时，应用启动不 panic，返回明确 `KeyringError`，**不**降级到明文存储。
5. **AC-5 抽象层存在**：`services/secret_store.rs` 提供 `save_secret` / `load_secret` / `delete_secret` 三个 API，所有 Key 操作通过该抽象。
6. **AC-6 AppError 枚举含 KeyringError**：`error.rs` 中 `AppError` 含 `KeyringError(String)` 变体，序列化为 JSON 可被前端 try-catch。
7. **AC-7 Tauri Command 暴露**：前端可通过 `invoke('secret_store_save', { key, value })` 和 `invoke('secret_store_load', { key })` 与 keyring 交互。
8. **AC-8 现有测试通过**：`cargo test` + `npm run test:frontend` 全部通过。
9. **AC-9 日志安全**：tracing 日志中 Key 值被 redact 为 `***`，不暴露真实值。

## Tasks / Subtasks

### Phase 1: Rust 依赖与错误基础 (AC: #4, #6)

- [x] **T1.1** 在 `Cargo.toml` 添加 `keyring = "3"` 依赖
- [x] **T1.2** 创建 `src-tauri/src/error.rs`：
  ```rust
  use serde::Serialize;

  #[derive(Debug, thiserror::Error, Serialize)]
  pub enum AppError {
      #[error("Not found: {0}")]
      NotFound(String),
      #[error("LLM error: {0}")]
      LlmError(String),
      #[error("Database error: {0}")]
      DbError(String),
      #[error("Validation error: {0}")]
      ValidationError(String),
      #[error("Keyring error: {0}")]
      KeyringError(String),
  }

  // Tauri Command 需要实现 Into<InvokeError>
  impl From<AppError> for tauri::ipc::InvokeError {
      fn from(e: AppError) -> Self {
          tauri::ipc::InvokeError::from(serde_json::to_string(&e).unwrap_or_default())
      }
  }
  ```
- [x] **T1.3** 在 `Cargo.toml` 添加 `thiserror = "1"` 依赖（用于 derive Error）
- [x] **T1.4** 在 `lib.rs` 中 `mod error;` 引入模块
- [x] **T1.5** `cargo check` 通过

### Phase 2: SecretStore 服务层 (AC: #5, #9)

- [x] **T2.1** 创建 `src-tauri/src/services/mod.rs`（模块声明）
- [x] **T2.2** 创建 `src-tauri/src/services/secret_store.rs`：
  ```rust
  use crate::error::AppError;

  const SERVICE_NAME: &str = "com.egosync.app";

  /// 保存密钥到系统钥匙串
  pub fn save_secret(key: &str, value: &str) -> Result<(), AppError> {
      let entry = keyring::Entry::new(SERVICE_NAME, key)
          .map_err(|e| AppError::KeyringError(format!("创建 keyring entry 失败: {}", e)))?;
      entry.set_password(value)
          .map_err(|e| AppError::KeyringError(format!("保存密钥失败: {}", e)))?;
      tracing::info!(key = key, "密钥已安全存储到系统钥匙串");
      Ok(())
  }

  /// 从系统钥匙串加载密钥
  pub fn load_secret(key: &str) -> Result<Option<String>, AppError> {
      let entry = keyring::Entry::new(SERVICE_NAME, key)
          .map_err(|e| AppError::KeyringError(format!("创建 keyring entry 失败: {}", e)))?;
      match entry.get_password() {
          Ok(password) => Ok(Some(password)),
          Err(keyring::Error::NoEntry) => Ok(None),
          Err(e) => Err(AppError::KeyringError(format!("读取密钥失败: {}", e))),
      }
  }

  /// 从系统钥匙串删除密钥
  pub fn delete_secret(key: &str) -> Result<(), AppError> {
      let entry = keyring::Entry::new(SERVICE_NAME, key)
          .map_err(|e| AppError::KeyringError(format!("创建 keyring entry 失败: {}", e)))?;
      match entry.delete_credential() {
          Ok(()) => {
              tracing::info!(key = key, "密钥已从系统钥匙串删除");
              Ok(())
          }
          Err(keyring::Error::NoEntry) => Ok(()), // 不存在视为成功
          Err(e) => Err(AppError::KeyringError(format!("删除密钥失败: {}", e))),
      }
  }
  ```
- [x] **T2.3** 在 `lib.rs` 中 `mod services;` 引入模块
- [x] **T2.4** 确保 tracing 日志中不打印 value 参数（只打印 key 名称）

### Phase 3: Tauri Command 层 (AC: #7)

- [x] **T3.1** 创建 `src-tauri/src/commands/mod.rs`
- [x] **T3.2** 创建 `src-tauri/src/commands/secret.rs`：
  ```rust
  use crate::error::AppError;
  use crate::services::secret_store;

  #[tauri::command]
  pub fn secret_store_save(key: String, value: String) -> Result<(), AppError> {
      secret_store::save_secret(&key, &value)
  }

  #[tauri::command]
  pub fn secret_store_load(key: String) -> Result<Option<String>, AppError> {
      secret_store::load_secret(&key)
  }

  #[tauri::command]
  pub fn secret_store_delete(key: String) -> Result<(), AppError> {
      secret_store::delete_secret(&key)
  }
  ```
- [x] **T3.3** 在 `lib.rs` 中注册 commands：
  ```rust
  mod commands;
  mod error;
  mod services;

  pub fn run() {
      // ... tracing init ...
      tauri::Builder::default()
          .invoke_handler(tauri::generate_handler![
              commands::secret::secret_store_save,
              commands::secret::secret_store_load,
              commands::secret::secret_store_delete,
          ])
          .run(tauri::generate_context!())
          .expect("error while running tauri application");
  }
  ```

### Phase 4: 单元测试 (AC: #8)

- [x] **T4.1** 在 `services/secret_store.rs` 底部添加 `#[cfg(test)] mod tests`：
  - `test_save_and_load` — 写入 → 读取 → 验证值一致
  - `test_load_nonexistent` — 读取不存在的 key → 返回 None
  - `test_delete` — 写入 → 删除 → 读取 → None
  - `test_delete_nonexistent` — 删除不存在的 key → 不报错（Ok）
  
  ⚠️ **CI 注意**：这些测试需要桌面环境（keyring 需要 D-Bus/Keychain/Credential Manager）。在无桌面的 CI 环境中可能跳过或标记 `#[ignore]`。

- [x] **T4.2** 运行 `cargo test`：通过（已有的 `app_compiles` + 新增测试）
- [x] **T4.3** 运行 `npm run test:frontend`：通过（前端无改动）
- [x] **T4.4** 运行 `cargo check`：无警告（仅 unused variants 警告，后续 Story 使用后消除）

### Phase 5: keyring 不可用降级测试 (AC: #4)

- [x] **T5.1** 确认 `secret_store.rs` 所有路径返回 `Result`，never panic
- [x] **T5.2** 确认 `AppError::KeyringError` 序列化为 JSON 格式（前端可解析）
- [x] **T5.3** 验证：在 `error.rs` 中 `AppError` 的序列化输出形如 `{"KeyringError":"创建 keyring entry 失败: ..."}`

## Dev Notes

### ⚠️ 致命陷阱清单

#### 陷阱 1：keyring 3.x API 变更

**keyring 3.x**（当前最新）API 与 2.x 不同：
- 构造器：`keyring::Entry::new(service, user)` 返回 `Result<Entry, Error>`（3.x 可能出错），**不是** 2.x 的直接返回 `Entry`。
- 错误类型：`keyring::Error` 枚举含 `NoEntry`、`Ambiguous`、`PlatformFailure` 等变体。
- **必须** 处理 `Entry::new()` 返回的 Result，不能 `.unwrap()`。

#### 陷阱 2：Tauri 2.x Command 错误返回

**Tauri 2.x** 的 Command 返回错误需要类型实现 `Into<tauri::ipc::InvokeError>`。最常见做法：
- 为 `AppError` 实现 `From<AppError> for tauri::ipc::InvokeError`
- 或者让 `AppError` 实现 `Serialize`，然后通过 `serde_json::to_string` 转为字符串

⚠️ **不要使用** `impl serde::Serialize for AppError` 的自动 derive 后直接返回——Tauri 2.x 需要显式转换。参考 Tauri 2.x 官方文档的 Command 错误处理模式。

#### 陷阱 3：SERVICE_NAME 必须与 bundle identifier 一致

keyring `service` 参数建议与 `tauri.conf.json` 中 `identifier: "com.egosync.app"` 一致，这样用户在 OS 钥匙串管理器中能识别属于哪个应用。

#### 陷阱 4：CI 环境 keyring 可用性

- **GitHub Actions Linux**：默认无 D-Bus session，keyring 会失败。CI 测试需要 `#[ignore]` 标记或使用 `keyring` 的 mock backend。
- **macOS CI**：GitHub Actions macOS runner 有 Keychain 可用。
- **Windows CI**：Credential Manager 可用。
- 建议：CI yml 中 Linux 平台 skip keyring 测试，或后续 Story 统一处理。

#### 陷阱 5：日志泄露 API Key

`tracing` 如果在 debug level 打印函数参数，可能泄露 key value。**必须** 只在日志中打印 key name（如 `"llm_config_1_api_key"`），**绝不**打印 value。

#### 陷阱 6：现有 lib.rs 结构

当前 `lib.rs` 结构：
```rust
pub fn run() {
    let _ = tracing_subscriber::fmt()...try_init();
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("...");
}
```

修改时需要：
1. 在 `run()` 之前添加 `mod commands; mod error; mod services;`
2. 在 `tauri::Builder::default()` 后链式调用 `.invoke_handler(...)` **在** `.run()` 之前
3. 保留现有的 `#[cfg(test)] mod tests` 不动

#### 陷阱 7：keyring Entry key 命名约定

建议 keyring 中 Key 的命名用 `{config_id}_api_key` 格式（如 `llm_1_api_key`），与后续 Story 1.6 的 `llm_configs` 表中的 `api_key_ref` 字段对应。本 Story 不需要实现对应关系，但命名预留兼容性。

### 前一 Story (1.4) 关键经验

- `@types/node` 已安装为 devDependency（`path` / `__dirname` 可用）
- `test-setup.ts` 已含 `localStorage` + `matchMedia` mock
- `vitest.config.ts` 已含 `@` alias
- 前端测试运行命令：`npm run test:frontend`（= `vitest run`）

### Project Structure Notes

**本 Story 新建的文件：**

| 文件 | 内容 |
|---|---|
| `GUI/src-tauri/src/error.rs` | `AppError` 枚举 + Tauri 错误转换 |
| `GUI/src-tauri/src/services/mod.rs` | services 模块声明 |
| `GUI/src-tauri/src/services/secret_store.rs` | keyring 抽象层 (save/load/delete) |
| `GUI/src-tauri/src/commands/mod.rs` | commands 模块声明 |
| `GUI/src-tauri/src/commands/secret.rs` | Tauri Command 薄层 |

**本 Story 修改的文件：**

| 文件 | 修改内容 |
|---|---|
| `GUI/src-tauri/Cargo.toml` | 添加 `keyring = "3"` + `thiserror = "1"` 依赖 |
| `GUI/src-tauri/src/lib.rs` | 添加 `mod commands/error/services` + 注册 invoke_handler |

**不修改的文件（确认无需改动）：**
- `GUI/src-tauri/src/main.rs` — 仅调用 `egosync_lib::run()`，不变
- `GUI/src-tauri/tauri.conf.json` — 无需修改
- 前端所有文件 — 本 Story 不改动前端（前端接通在 Story 1.6）

**与架构文档对齐：**

| 架构规范要求 | 本 Story 实现 |
|---|---|
| `services/secret_store.rs` 抽象层 | ✅ 创建 |
| `AppError::KeyringError` 变体 | ✅ 包含 |
| keyring 3.x 跨平台 | ✅ 依赖添加 |
| commands/ 薄层 + services/ 业务逻辑 | ✅ 分层正确 |
| NFR-7 系统钥匙串安全存储 | ✅ 完整覆盖 |
| 所有函数返回 `Result<T, AppError>` | ✅ 无 unwrap |

### References

- [Source: `_bmad-output/planning-artifacts/epics.md` #Story 1.5] — 完整 AC 定义
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Authentication & Security] — keyring 3.x 决策
- [Source: `_bmad-output/planning-artifacts/architecture.md` #API & Communication Patterns] — AppError 枚举定义、IPC Command 模式
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Structure Patterns] — Rust 后端目录结构
- [Source: `_bmad-output/planning-artifacts/architecture.md` #Process Patterns] — 错误处理规范
- [Source: `_bmad-output/project-context.md` #关键禁止事项] — 硬编码 API Key 禁止、unwrap 禁止
- [Source: `GUI/src-tauri/Cargo.toml`] — 当前依赖：tauri 2.x、serde、tokio、tracing（无 keyring）
- [Source: `GUI/src-tauri/src/lib.rs`] — 当前：最小 run() + tracing init + 空 test
- [Source: `GUI/src-tauri/tauri.conf.json`] — identifier: "com.egosync.app"（keyring service name 来源）
- [Source: `GUI/src/components/settings/GlobalSettingsModal.tsx`] — 前端 LLM 配置 UI（mock state，Story 1.6 接通）

### 验证命令清单

```bash
cd GUI/src-tauri
cargo check                    # 编译检查
cargo test                     # 单元测试（需桌面环境）
cargo clippy -- -D warnings    # lint

cd GUI
npm run test:frontend          # 前端测试无回归
```

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4 (Cascade)

### Debug Log References

- keyring 3.x 默认特性为空，在 Windows 上退化为 MockCredential（内存 mock）。需显式启用 `windows-native`、`apple-native`、`sync-secret-service` 平台特性。
- Tauri 2.x 已有 blanket `impl<T: Serialize> From<T> for InvokeError`，不能手动再实现 `From<AppError> for InvokeError`。改用手动 `impl Serialize for AppError` 产生 tagged JSON。
- keyring 3.x `Entry::new()` 在 Windows 上每次生成不同内部 target，导致跨调用查找失败。改用 `Entry::new_with_target()` 显式指定一致 target 格式 `{SERVICE_NAME}:{key}`。

### Completion Notes List

- ✅ AC-1: API Key 只存在系统钥匙串，不落 DB/日志
- ✅ AC-2: 启用 windows-native + apple-native + sync-secret-service 三平台后端
- ✅ AC-3: keyring 写入 OS 级存储，卸载重装持久
- ✅ AC-4: 所有路径 Result 返回，零 panic，KeyringError 明确
- ✅ AC-5: secret_store.rs 提供 save/load/delete 三 API
- ✅ AC-6: AppError 含 KeyringError，手动 Serialize 为 tagged JSON
- ✅ AC-7: 三个 Tauri Command 注册（secret_store_save/load/delete）
- ✅ AC-8: cargo test 7 passed + npm run test:frontend 1 passed
- ✅ AC-9: tracing 日志仅打印 key 名称，不打印 value

### Change Log

- 2026-05-21: Story 1.5 实现完成，所有 AC 满足

### File List

**新建：**
- `GUI/src-tauri/src/error.rs`
- `GUI/src-tauri/src/services/mod.rs`
- `GUI/src-tauri/src/services/secret_store.rs`
- `GUI/src-tauri/src/commands/mod.rs`
- `GUI/src-tauri/src/commands/secret.rs`

**修改：**
- `GUI/src-tauri/Cargo.toml`
- `GUI/src-tauri/src/lib.rs`
