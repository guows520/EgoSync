---
baseline_commit: c83d6dd
---

# Story 4.1: 角色后台调度器按配置频率运行工作循环

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 角色在应用运行期间按我设定的频率自动审视目标和任务,
so that 不需要我主动发起对话角色也能持续工作。

## 背景与现状（务必先读）

**本 story 是纯 Rust 后端 story — 新增 `scheduler` 模块，不涉及前端改动。**

当前应用已有两个后台定时任务作为范本：
- `services/task_deadline_watch.rs`：每小时检查临期任务自动升入 Q1（`tokio::interval(3600s)`）
- `services/task_protection_watch.rs`：每小时检查 Q2 任务保护状态标记 at_risk

两者均在 `lib.rs:255-261` 的 `setup` 闭包中通过 `spawn_hourly_watch(pool.clone())` 启动，模式一致：`tauri::async_runtime::spawn` → `tokio::time::interval` → 循环执行 → 错误只 `tracing::warn!` 不 panic。

`roles` 表已有 `proactivity_level` 字段（`TEXT NOT NULL DEFAULT 'moderate'`），取值 `passive` / `moderate` / `proactive`，由 Story 2.10 实现。`Role` 模型（`models/role.rs:13`）已含 `proactivity_level: String` 字段。`db::roles::list_active_roles` 可获取所有活跃角色。

**本 story 要做的事**：新建 `services/scheduler.rs` 模块，实现一个后台调度器，根据每个角色的 `proactivity_level` 决定工作循环触发频率，为后续 Story 4.2（建议生成）提供调度基础。**本 story 只实现调度框架和日志记录，不实现实际的工作循环业务逻辑**（如 LLM 调用、建议生成等 — 那是 4.2 的职责）。

## Acceptance Criteria

> 既有后台任务（task_deadline_watch、task_protection_watch）和所有前端功能必须零回归。

1. **调度器启动与生命周期（AC1）**
   - **Given** 应用启动且数据库初始化完成
   - **When** Tauri `setup` 执行
   - **Then** 调度器以后台 `tokio::spawn` 任务启动，不阻塞 Tauri setup
   - **And** 调度器内部使用 `tokio::time::interval` 实现定时循环
   - **And** 应用关闭时调度器自然停止（随进程退出，无需显式 kill）

2. **按 proactivity_level 调度频率（AC2）**
   - **Given** 角色主动性设为 `moderate`
   - **When** 调度器运行
   - **Then** 每日触发 1-2 次工作循环（间隔约 8-12 小时）
   - **Given** 角色主动性设为 `proactive`
   - **When** 调度器运行
   - **Then** 每日触发 3-4 次工作循环（间隔约 4-6 小时）
   - **Given** 角色主动性设为 `passive`
   - **When** 调度器运行
   - **Then** 跳过该角色（不触发工作循环）

3. **每个角色独立 spawn 执行（AC3）**
   - **Given** 应用启动且有 2+ 个活跃角色
   - **When** 到达调度时间点
   - **Then** 调度器逐一触发每个角色的工作循环
   - **And** 每个角色独立 `tokio::spawn` 执行，互不阻塞
   - **And** 单个角色工作循环失败不影响其他角色

4. **动态读取 proactivity_level（AC4）**
   - **Given** 用户在 RoleView SettingsTab 切换角色主动性档位
   - **When** 保存成功后
   - **Then** 下次调度循环立即生效（无需重启应用）
   - **And** 循环频率从 `roles.proactivity_level` 动态读取（每次 tick 重新查询数据库，不缓存）

5. **tracing 日志记录（AC5）**
   - **Given** 每次工作循环触发
   - **Then** 记录 tracing 日志：`role_id`, `triggered_at`, `duration_ms`, `result`
   - **And** 日志级别：正常完成为 `info`，失败为 `warn`
   - **And** 日志包含角色名称便于调试

6. **工作循环占位实现（AC6）**
   - **Given** 本 story 只实现调度框架
   - **Then** 工作循环的实际业务逻辑（LLM 调用、建议生成）为占位实现
   - **And** 占位实现只记录 tracing 日志并返回 Ok(())
   - **And** 占位实现的位置和接口设计好，便于 Story 4.2 直接填充业务逻辑

7. **应用重启行为（AC7）**
   - **Given** 应用关闭后重新启动
   - **When** 调度器初始化
   - **Then** 重新初始化调度器，不保留上次运行状态
   - **And** 首次 tick 有延迟（不立即执行，避免启动时并发太多后台任务）

## Tasks / Subtasks

- [x] Rust：新建 `services/scheduler.rs` 模块（AC: 1, 2, 3, 4, 5, 6, 7）
  - [x] 定义频率常量：`MODERATE_INTERVAL_SECS: u64 = 8 * 3600`（8 小时，每日 ~2 次）、`PROACTIVE_INTERVAL_SECS: u64 = 4 * 3600`（4 小时，每日 ~4 次）
  - [x] 实现 `pub fn spawn_scheduler(pool: SqlitePool)`：启动后台 tokio task
  - [x] 调度器主循环：使用较短的基础 interval（如 60 秒）做 tick，每次 tick 检查各角色是否到达下次触发时间
  - [x] 实现 `async fn run_work_loop_for_role(pool: &SqlitePool, role: &Role) -> Result<(), AppError>`：占位实现 — 只记录 tracing 日志并返回 Ok(())
  - [x] 实现 `fn interval_for_proactivity(level: &str) -> Option<Duration>`：`passive` → None（跳过），`moderate` → Some(8h)，`proactive` → Some(4h)
  - [x] 维护 `HashMap<String, Instant>` 记录每个角色的上次触发时间（role_id → last_triggered），每次 tick 检查 `now - last_triggered >= interval` 决定是否触发
  - [x] 每次触发：`tokio::spawn` 独立 task 执行 `run_work_loop_for_role`，记录开始时间，完成后记录 `duration_ms` 和 `result`
  - [x] 首次启动延迟：使用 `interval.tick()` 消耗首次立即 tick，然后设置 `last_triggered = now`（首次触发需等待一个完整 interval）

- [x] Rust：注册调度器到 `lib.rs`（AC: 1）
  - [x] `lib.rs` setup 闭包中，在 `task_protection_watch::spawn_hourly_watch` 之后添加 `services::scheduler::spawn_scheduler(pool.clone())`
  - [x] `services/mod.rs` 添加 `pub mod scheduler;`

- [x] Rust：单元测试（AC: 2, 4, 5, 6）
  - [x] `scheduler.rs` 底部 `#[cfg(test)] mod tests`
  - [x] 测试 `interval_for_proactivity`：`passive` → None，`moderate` → Some(8h)，`proactive` → Some(4h)，未知值 → None
  - [x] 测试 `run_work_loop_for_role` 占位实现：传入 mock role，验证返回 Ok(()) 且不 panic
  - [x] 测试调度逻辑：给定 `last_triggered` 时间和当前 interval，验证是否应该触发（`should_trigger` 函数）

- [x] 验证（AC: 1-7）
  - [x] `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` 通过（408 passed, 0 failed）
  - [x] `npm --prefix "egosync-app" run build` 通过（确认无前端影响）
  - [x] `npm --prefix "egosync-app" run test:frontend` 通过（216 passed, 0 failed）

## Dev Notes

### Current State（基于当前代码 @ c83d6dd）

- **后台任务范本**（`services/task_deadline_watch.rs:85-98`）：`spawn_hourly_watch(pool)` → `tauri::async_runtime::spawn` → `tokio::time::interval(3600s)` → `interval.tick().await`（首次立即返回）→ `loop { escalate(); interval.tick().await }`。错误只 `tracing::warn!`，不 panic，不阻塞 Tauri setup。**本 story 照此模式**。
- **`services/task_protection_watch.rs:75-88`**：同上模式，`spawn_hourly_watch(pool)` → `tokio::interval(3600s)` → 循环 `recompute_protection_status`。
- **`lib.rs:255-261`**：两个后台任务在 setup 闭包中启动，位置在 `Ok(())` 之前。本 story 在此处追加 `scheduler::spawn_scheduler(pool.clone())`。
- **`services/mod.rs:1-18`**：模块声明列表，本 story 追加 `pub mod scheduler;`。
- **`models/role.rs:1-17`**：`Role` 结构体含 `proactivity_level: String`（`#[serde(rename_all="camelCase")]` + `FromRow`）。
- **`db/roles.rs:9`**：`ROLE_SELECT_COLUMNS` 含 `proactivity_level`；`list_active_roles`（`:35-37`）返回 `status='active'` 的角色。
- **`services/role_config.rs:77-85`**：`normalize_proactivity_level` 校验取值 ∈ {passive, moderate, proactive}。
- **`commands/role.rs:114-128`**：`role_update_proactivity` 命令已存在，更新后角色 `proactivity_level` 立即生效（本 story 调度器每次 tick 动态读取，自然生效）。

### What This Story Changes

**Rust（新增 1 + 修改 2 文件）：**
1. `services/scheduler.rs`：**新建** — 调度器模块（`spawn_scheduler` + `run_work_loop_for_role` + `interval_for_proactivity` + 单测）
2. `services/mod.rs`：追加 `pub mod scheduler;`
3. `lib.rs`：setup 闭包追加 `services::scheduler::spawn_scheduler(pool.clone())`

**无前端改动、无新增依赖、无新增迁移、无 DB schema 改动。**

### What Must Be Preserved（防回归）

- **既有后台任务不回归**：`task_deadline_watch` 和 `task_protection_watch` 的启动和运行不受影响。新调度器在它们之后启动，互不干扰。
- **Tauri setup 不阻塞**：调度器启动是 `spawn` 异步的，不阻塞 setup 闭包返回。
- **前端功能零回归**：本 story 纯后端，不触碰任何前端文件。
- **既有测试通过**：`cargo test` 和前端测试均不受影响。

### 关键正确性要点（极易踩坑）

- **调度器设计模式 — 短 tick + 状态跟踪，而非每角色独立 interval**：不要为每个角色创建独立的 `tokio::interval`（角色动态增减时管理复杂）。应使用一个较短的基础 tick（如 60 秒），每次 tick 查询活跃角色列表，检查各角色是否到达触发时间。这样新建/归档/删除角色自动生效，`proactivity_level` 变更也自动生效。
- **`passive` 角色跳过**：`interval_for_proactivity("passive")` 返回 `None`，调度器遇到 `None` 跳过该角色（不 spawn、不记日志）。
- **每个角色独立 `tokio::spawn`**：到达触发时间的角色，`tokio::spawn(run_work_loop_for_role(pool, role))` 独立执行。单个角色失败（panic/error）不影响调度器主循环和其他角色。使用 `tokio::spawn` 而非 `await`，确保并行。
- **动态读取不缓存**：每次 tick 调用 `db::roles::list_active_roles(&pool)` 获取最新角色列表和 `proactivity_level`。不缓存角色列表，确保 CRUD 和 proactivity 变更立即生效。
- **首次启动延迟**：调度器首次 tick 不立即触发任何角色。消耗 `interval.tick()` 的首次立即返回后，设置各角色的 `last_triggered = Instant::now()`（即所有角色首次触发需等待一个完整 interval）。
- **tracing 日志格式**：`tracing::info!(role_id = %role.id, role_name = %role.name, triggered_at = %now, duration_ms = elapsed, "工作循环完成")`。失败时 `tracing::warn!(role_id = %role.id, role_name = %role.name, error = %e, duration_ms = elapsed, "工作循环失败")`。
- **占位实现接口设计**：`run_work_loop_for_role` 签名应设计为便于 4.2 扩展：`async fn run_work_loop_for_role(pool: &SqlitePool, role: &Role) -> Result<(), AppError>`。4.2 将在此函数内添加 LLM 调用、建议生成逻辑。当前只 `tracing::info!` 并返回 `Ok(())`。
- **错误处理**：调度器主循环错误只 `tracing::warn!`，绝不 panic，绝不阻塞。`run_work_loop_for_role` 返回 `Result`，spawn 的 task 内 `match` 结果记日志。
- **`Instant` vs `SystemTime`**：使用 `tokio::time::Instant` 记录 `last_triggered`（单调时钟，不受系统时间调整影响），不使用 `SystemTime`。
- **无 `.unwrap()`**：所有数据库查询和操作用 `?` 或 `match` 处理错误。

### Previous Story Intelligence

- **3.3/3.5 后台任务模式**（`task_deadline_watch.rs` / `task_protection_watch.rs`）：已建立 `spawn_hourly_watch(pool) → tauri::async_runtime::spawn → tokio::interval → loop` 的后台任务模式。本 story 照此模式但更复杂（多角色、动态频率），需在此基础上增加角色遍历和状态跟踪。
- **3.7 全角色查询**（`db::tasks::list_all_tasks`）：已建立 LEFT JOIN roles 查询模式。本 story 只需 `list_active_roles`（已存在），不需要新查询。
- **2.10 proactivity_level**：`proactivity_level` 字段和 `role_update_proactivity` 命令已实现。本 story 是该字段的第一个实际行为消费方（Story 2.10 只存储和显示 UI，行为接通在 Epic 4）。

### Git Intelligence

- `c83d6dd`（HEAD，本 story baseline）— docs: refresh task overview story
- `96020df` — feat: add butler task overview
- `76a65ac` — docs: 记录 story 3.6 实现与代码评审结果
- `6dd4af1` — feat(tasks): 四象限分组显示（story 3.6）
- `068c84b` — feat(tasks): Q2 任务保护属性，连续被挤压标记 at_risk 预警（story 3.5）— **`task_protection_watch.rs` 范本来源**
- `c4248c6` — feat(tasks): async classification with event notification（story 3.3）— **`task_deadline_watch.rs` 范本来源**
- 范本提交：3.3/3.5 展示了「spawn 后台 tokio task + interval 循环 + 错误降级」的模式，可直接借鉴。

### Testing Requirements

- **Rust 单测**（`scheduler.rs` 测试模块）：
  - `interval_for_proactivity` 返回值：`"passive"` → `None`，`"moderate"` → `Some(Duration::from_secs(8 * 3600))`，`"proactive"` → `Some(Duration::from_secs(4 * 3600))`，无效值 → `None`（或 panic，取决于实现选择）
  - `run_work_loop_for_role` 占位实现：构造 mock `Role`，调用后返回 `Ok(())` 且不 panic
  - `should_trigger` 逻辑（如抽取为独立函数）：给定 `last_triggered`、`now`、`interval`，验证返回 `true`/`false` 正确
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`（Rust 全量测试）
  - `npm --prefix "egosync-app" run build`（确认前端不受影响）
  - `npm --prefix "egosync-app" run test:frontend`（确认前端测试不回归）

### Project Structure Notes

- **新增文件（1）**：`services/scheduler.rs`
- **修改文件（2）**：`services/mod.rs`（追加模块声明）、`lib.rs`（setup 中追加启动调用）
- **无新增依赖**：使用已有的 `tokio`、`sqlx`、`tracing`、`uuid` crates
- **无新增迁移、无 DB schema 改动**：`roles.proactivity_level` 字段已存在
- 符合项目规则：Rust 三层（command→service→db）、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块 snake_case

### References

- `_bmad-output/project-context.md`（前后端规则：Rust 三层/serde camelCase/无 unwrap/tracing 日志）
- `_bmad-output/planning-artifacts/epics.md:1584-1617`（Story 4.1 定义）
- `_bmad-output/planning-artifacts/architecture.md:159`（`scheduler.rs` 在项目结构中的位置）、`:479,954`（scheduler + tokio::interval 实现工作循环）、`:589`（services/scheduler.rs）
- `egosync-app/src-tauri/src/services/task_deadline_watch.rs:85-98`（后台任务 spawn 范本）
- `egosync-app/src-tauri/src/services/task_protection_watch.rs:75-88`（后台任务 spawn 范本）
- `egosync-app/src-tauri/src/lib.rs:255-261`（后台任务启动位置）
- `egosync-app/src-tauri/src/services/mod.rs:1-18`（模块声明列表）
- `egosync-app/src-tauri/src/models/role.rs:1-17`（`Role` 结构体含 `proactivity_level`）
- `egosync-app/src-tauri/src/db/roles.rs:9,35-37`（`ROLE_SELECT_COLUMNS` + `list_active_roles`）
- `egosync-app/src-tauri/src/services/role_config.rs:77-85`（`normalize_proactivity_level` 校验）
- `egosync-app/src-tauri/src/commands/role.rs:114-128`（`role_update_proactivity` 命令）

## Dev Agent Record

### Agent Model Used
Claude Sonnet 4 (via Windsurf Cascade)

### Debug Log References
- `cargo test` 首次运行时 `egosync.exe` 被进程锁定（os error 5），`taskkill /PID 112088 /F` 后重试成功

### Completion Notes List
- 新建 `services/scheduler.rs`：实现后台调度器，60 秒基础 tick，动态查询活跃角色，按 `proactivity_level` 决定触发频率
- `interval_for_proactivity`：`passive` → None（跳过），`moderate` → Some(8h)，`proactive` → Some(4h)，未知值 → None（安全降级）
- `should_trigger`：使用 `tokio::time::Instant`（单调时钟）比较 `now - last_triggered >= interval`
- `run_work_loop_for_role`：占位实现，只记录 tracing info 日志并返回 Ok(())，接口设计便于 Story 4.2 扩展
- `spawn_scheduler`：消耗首次 tick 后设置 `last_triggered = now`，确保首次触发需等待一个完整 interval（AC7）
- 每个角色独立 `tokio::spawn`，单个角色失败不影响其他角色（AC3）
- 每次 tick 动态读取 `db::roles::list_active_roles`，角色 CRUD 和 proactivity 变更立即生效（AC4）
- tracing 日志包含 `role_id`、`role_name`、`triggered_at`、`duration_ms`、`result`（AC5）
- 8 个单元测试全部通过：覆盖 `interval_for_proactivity`（4 个）、`should_trigger`（3 个）、`run_work_loop_for_role`（1 个）
- 全量 Rust 测试 408 passed / 0 failed，前端构建成功，前端测试 216 passed / 0 failed

### File List
- `egosync-app/src-tauri/src/services/scheduler.rs` — **修改**：调度器从硬编码小时改为 DB 驱动的 HH:MM 时间点，支持用户自定义
- `egosync-app/src-tauri/src/services/mod.rs` — **修改**：追加 `pub mod scheduler;`
- `egosync-app/src-tauri/src/lib.rs` — **修改**：setup 闭包追加 `services::scheduler::spawn_scheduler(pool.clone())`；注册 `scheduler_get_times` / `scheduler_set_times` 命令
- `egosync-app/src-tauri/src/commands/scheduler.rs` — **新建**：`scheduler_get_times` 和 `scheduler_set_times` Tauri 命令
- `egosync-app/src-tauri/src/commands/mod.rs` — **修改**：追加 `pub mod scheduler;`
- `egosync-app/src/services/schedulerService.ts` — **新建**：前端调度时间服务封装
- `egosync-app/src/components/role/SettingsTab.tsx` — **修改**：新增调度时间编辑 UI（时间选择器、增删、保存）

### Change Log
- 2026-06-20: Story 4.1 实现完成 — 新建 scheduler 模块，按 proactivity_level 调度角色工作循环，8 个单测通过，全量测试零回归
- 2026-06-21: 调度器改为可配置时间点 — 从硬编码小时改为 DB 驱动 HH:MM 精度，新增 get/set Tauri 命令和前端编辑 UI，16 个单测通过

### Review Findings
- [x] [Review][Patch] `triggered_at` 记录单调时钟 Instant 而非可读时间戳，偏离 AC5（Dev Notes 第 147 行约定 `triggered_at = %now`）[egosync-app/src-tauri/src/services/scheduler.rs:121] — 已修复：改用 `db::settings::chrono_now_pub()` ISO 8601 墙钟时间戳，并补充到失败日志
- [x] [Review][Patch] `last_triggered_map` 未清理已归档/删除角色条目，长期运行内存只增不减 [egosync-app/src-tauri/src/services/scheduler.rs:81] — 已修复：每次 tick 末尾按当前活跃角色 id 集合 retain
- [x] [Review][Defer] 同一 tick 多角色到期并发 spawn，4.2 接入 LLM 后可能瞬时高并发 [egosync-app/src-tauri/src/services/scheduler.rs:111] — deferred, 属 Story 4.2 范畴
