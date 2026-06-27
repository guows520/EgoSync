---
baseline_commit: f20301b58d138f47419e57488a7f11a6c04338de
---

# Story 6.6: 管家在工作日保护大石头任务不被低优先级挤掉

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 管家帮我守住本周最重要的事,
So that 大石头不会被琐事淹没。

## 背景与现状（务必先读）

**本 story 是 Epic 6 的第六个也是最后一个 story — 在 Story 6.1（晨间简报）、6.2（节奏化时间配置）、6.3（大石头规划提醒）、6.4（周复盘成绩单）、6.5（WeeklyReviewModal 真实数据）基础上，实现"工作日大石头保护提醒"和"周五大石头未完成检查"。**

**核心交付：**
1. **工作日大石头保护检查**（AC1/AC3）：调度器每次 tick 检查未完成大石头任务，若本周无任何进展（`updated_at` 早于本周一），管家温和提醒"你的大石头'XX'这周还没动，要不要今天安排一点时间？"
2. **周五大石头未完成检查**（AC4）：周五调度触发时，若仍有未完成大石头，管家提醒"本周大石头还有 N 个未完成，周末要安排时间吗？"
3. **频率控制**（AC3）：每个大石头每日最多提醒 1 次，已完成或当日有进展的不提醒

### ⚠️ Scope 裁剪：AC2 和 AC5 仲裁部分延迟至 V2

**AC2**（低优先级任务与大石头时间冲突检测 + 仲裁加权）和 **AC5 的仲裁引擎联动部分**依赖 Epic 5 的冲突检测引擎（Story 5.3）和三步仲裁协议（Story 5.4），这两项已标记为 `deferred-v2`（git commit `aa1c95f`："移除冲突检测逻辑(5-3~5-6延迟至V2)"）。

**本 story 只实现：**
- ✅ AC1：工作日大石头进展检测 + 温和提醒
- ✅ AC3：频率控制（每日 ≤ 1 次/大石头，已完成/当日有进展不提醒）
- ✅ AC4：周五大石头未完成数量提醒
- ✅ AC5（调度器部分）：调度器增加大石头保护检查步骤

**延迟到 V2（等 Epic 5 仲裁引擎实现后）：**
- ⏸ AC2：Q3/Q4 与大石头时间冲突检测 + 仲裁中大石头权重加成
- ⏸ AC5（仲裁联动部分）：大石头 `is_big_rock` 在仲裁 prompt 中加权

### 已建成的基础（本 story 的接入点）

**调度器（直接扩展）：**
- `src-tauri/src/services/scheduler.rs` — 60 秒基础 tick 循环，已集成 Q2 保护检查、晨间简报、大石头规划提醒、周复盘触发
- `scheduler.rs:475-485` — Q2 保护检查每次 tick 都调用 `q2_protection_reminder::check_and_generate_reminders`，频率由 DB 记录控制
- `scheduler.rs:322` — `spawn_scheduler(pool, conv_pool, app_handle)` 在 Tauri setup 中启动
- `lib.rs:274` — `services::scheduler::spawn_scheduler(pool.clone(), conv_pool.clone(), app.handle().clone())`

**Q2 保护提醒（参照模式 — 本 story 几乎复刻此结构）：**
- `src-tauri/src/services/q2_protection_reminder.rs` — 281 行，完整的"检查 at_risk 任务 → 频率控制 → 创建通知 + 写管家对话 + emit 事件"模式
- `q2_protection_reminder.rs:65-88` — `check_and_generate_reminders` 主函数：查询 at_risk 任务 → 逐任务处理
- `q2_protection_reminder.rs:90-236` — `process_single_task`：查提醒记录 → 连续 3 天停止 → 今日已提醒跳过 → 创建通知 + 写对话 + upsert 计数
- `q2_protection_reminder.rs:38-56` — `is_reminded_today` / `parse_iso_to_local_date` 日期比较工具函数

**Q2 提醒频率控制表（参照模式）：**
- `src-tauri/migrations/018_q2_reminders.sql` — `q2_reminders` 表：`id`, `task_id` UNIQUE, `reminded_count`, `last_reminded_at`, `created_at`
- `src-tauri/src/db/q2_reminders.rs` — `upsert_reminder`（INSERT ON CONFLICT 递增计数）/ `get_reminder_for_task` / `delete_reminder_for_task`
- `src-tauri/src/models/q2_reminder.rs` — `Q2Reminder` 结构体

**大石头任务数据（直接复用）：**
- `src-tauri/src/db/tasks.rs:60-85` — `list_all_tasks(pool, quadrant, is_big_rock)` 函数，传 `is_big_rock=Some(true)` 查所有大石头任务（含 `role_name` / `role_color` JOIN）
- `src-tauri/src/models/task.rs:50-70` — `Task` 结构体，含 `is_big_rock` / `is_completed` / `updated_at` / `role_id`
- `src-tauri/src/models/task.rs:84-106` — `CrossRoleTask` 结构体，含 `role_name` / `role_color`

**通知服务（直接复用）：**
- `src-tauri/src/services/notification_service.rs:57` — `create_notification_for_role(pool, role_id, NotificationLevel::Tap, message)` 创建"轻触"级通知
- 通知自动处理 proactivity 约束和敲门上限降级

**管家对话写入（直接复用）：**
- `src-tauri/src/db/conversations.rs` — `get_or_create_butler_conversation` + `insert_message(conv_pool, conv_id, "assistant", message, true)`

**前端事件监听模式（直接复用）：**
- `src/App.tsx:87-94` — `useTauriEvent('q2:reminder', ...)` 递增 `butlerChatRefreshTrigger`
- `src/App.tsx:96-103` — `useTauriEvent('briefing:generated', ...)` 递增 `butlerChatRefreshTrigger`
- `src/App.tsx:105-114` — `useTauriEvent('bigrock:reminder', ...)` 打开 WeeklyReviewModal + 递增 trigger
- 前端已有 `notification:new` 事件监听，通知自动出现在通知面板

### "进展"定义的技术约束

AC1 原文："大石头任务本周未有任何进展（无编辑/无对话提及/未完成子步骤）"

**当前系统可检测的进展信号：**
- ✅ **无编辑**：`tasks.updated_at` 字段记录最后一次编辑时间。若 `updated_at` 早于本周一，说明本周无任何编辑
- ❌ **无对话提及**：系统无"任务被对话提及"的跟踪机制（需要 NLP + 任务-对话关联表，超出 V1 scope）
- ❌ **未完成子步骤**：tasks 表无子任务模型（无 `parent_task_id` 或 `subtasks` 表）

**本 story 采用的"进展"判定：** `updated_at` 早于本周一 00:00:00（本地时间）→ 判定"本周无进展"。这是当前数据模型下最务实的实现。未来 V2 可扩展对话提及检测和子任务跟踪。

## Acceptance Criteria

1. **AC1**: Given 工作日调度循环中检测大石头进展，When 大石头任务本周未有任何进展（`updated_at` 早于本周一），Then 管家在对话中温和提醒"你的大石头'XX'这周还没动，要不要今天安排一点时间？"

2. **AC2** ⏸ **deferred-v2**: Given 低优先级任务（Q3/Q4）与大石头时间冲突，When 冲突检测触发（复用 E5 Story 5.3），Then 仲裁中大石头权重自动加成，And 方案倾向保护大石头时间。**延迟原因：E5 Story 5.3/5.4 已标记 deferred-v2，冲突检测和仲裁引擎不存在。**

3. **AC3**: Given 大石头保护提醒，Then 级别为"轻触"（`NotificationLevel::Tap`），And 每个大石头每日最多提醒 1 次（DB 频率控制），And 大石头已完成或已有当日进展（`updated_at` 为今天）→ 不提醒

4. **AC4**: Given 周五仍有未完成大石头，When 调度触发，Then 管家提醒"本周大石头还有 N 个未完成，周末要安排时间吗？"

5. **AC5**: Given Rust 后端，Then 调度器增加大石头保护检查步骤（每次 tick 调用，频率由 DB 记录控制）。**仲裁引擎联动部分（大石头 `is_big_rock` 在仲裁 prompt 中加权）延迟至 V2。**

## Tasks / Subtasks

- [x] **Task 1: 新建 migration — `big_rock_protection_reminders` 表** (AC: #3)
  - [x] 1.0 新建 `src-tauri/migrations/026_big_rock_protection_reminders.sql`（使用 026 而非 spec 中的 029，因为实际最新 migration 为 025，避免序号空缺）
  - [x] 1.1 表结构（参照 `018_q2_reminders.sql`）：
    ```sql
    CREATE TABLE IF NOT EXISTS big_rock_protection_reminders (
        id TEXT PRIMARY KEY NOT NULL,
        task_id TEXT NOT NULL,
        reminded_count INTEGER NOT NULL DEFAULT 1,
        last_reminded_at TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
        UNIQUE(task_id)
    );
    ```

- [x] **Task 2: 新建 model + DB 层** (AC: #3)
  - [x] 2.0 新建 `src-tauri/src/models/big_rock_protection_reminder.rs`，定义 `BigRockProtectionReminder` 结构体（参照 `q2_reminder.rs`）：`id`, `task_id`, `reminded_count`, `last_reminded_at`, `created_at`，derive `Serialize, Deserialize, sqlx::FromRow` + `#[serde(rename_all = "camelCase")]`
  - [x] 2.1 在 `src-tauri/src/models/mod.rs` 中注册 `pub mod big_rock_protection_reminder;`
  - [x] 2.2 新建 `src-tauri/src/db/big_rock_protection_reminders.rs`，实现三个函数（参照 `db/q2_reminders.rs`）：
    - `upsert_reminder(pool, task_id) -> Result<BigRockProtectionReminder, AppError>` — INSERT ON CONFLICT 递增 `reminded_count` + 刷新 `last_reminded_at`
    - `get_reminder_for_task(pool, task_id) -> Result<Option<BigRockProtectionReminder>, AppError>`
    - `delete_reminder_for_task(pool, task_id) -> Result<u64, AppError>` — 任务完成或删除时清理
  - [x] 2.3 在 `src-tauri/src/db/mod.rs` 中注册 `pub mod big_rock_protection_reminders;`
  - [x] 2.4 单元测试：upsert 新建、upsert 冲突递增、get 不存在返回 None、delete 删除（4 个测试全部通过）

- [x] **Task 3: 新建 service — `bigrock_protection.rs`** (AC: #1, #3, #4, #5)
  - [x] 3.0 新建 `src-tauri/src/services/bigrock_protection.rs`
  - [x] 3.1 定义 Tauri Event 常量和 payload：
    - `BIGROCK_PROTECTION_EVENT: &str = "bigrock:protection"`
    - `BigRockProtectionPayload` 结构体（`Serialize + Deserialize + rename_all = "camelCase"`）：`task_id`, `task_title`, `role_id: Option<String>`, `role_name: Option<String>`, `message`, `notification_id`
  - [x] 3.2 实现日期工具函数（参照 `q2_protection_reminder.rs:38-56`）：
    - `is_reminded_today(last_reminded_at: &str) -> bool` — 复用 `parse_iso_to_local_date` 逻辑
    - `is_updated_this_week(updated_at: &str) -> bool` — 解析 `updated_at` 为本地日期，与本周一比较
    - `compute_this_week_monday() -> chrono::NaiveDate` — 计算本周一日期
    - `is_updated_today(updated_at: &str) -> bool` — 判断当日是否有进展
    - `is_weekday() -> bool` / `is_friday() -> bool` — 工作日/周五判断
  - [x] 3.3 实现 `check_and_generate_protection_reminders(pool, conv_pool, app_handle) -> Result<(), AppError>`（主函数，AC1/AC3）：
    - 工作日守卫：非工作日直接返回
    - 核心逻辑抽取为 `check_and_generate_protection_reminders_inner`（供测试直接调用，避免日期依赖）
  - [x] 3.4 实现 `process_single_bigrock(pool, conv_pool, app_handle, task) -> Result<(), AppError>`（AC3）
  - [x] 3.5 实现 `check_friday_bigrock_status(pool, conv_pool, app_handle) -> Result<bool, AppError>`（AC4）
  - [x] 3.6 在 `src-tauri/src/services/mod.rs` 中注册 `pub mod bigrock_protection;`
  - [x] 3.7 单元测试 + 集成测试（19 个测试全部通过）

- [x] **Task 4: 调度器集成** (AC: #1, #4, #5)
  - [x] 4.0 在 `scheduler.rs` Q2 保护检查之后新增大石头保护检查调用
  - [x] 4.1 新增周五大检查去重变量 `last_bigrock_friday_check_date`
  - [x] 4.2 在周复盘触发之后新增周五大检查触发逻辑

- [x] **Task 5: 前端事件监听** (AC: #1, #4)
  - [x] 5.0 在 `src/App.tsx` 中 `bigrock:reminder` 监听之后新增 `bigrock:protection` 事件监听（使用内联类型，未新建类型文件）
  - [x] 5.1 跳过（使用内联类型，无需新建类型文件）

- [x] **Task 6: 前端测试** (AC: #1, #4)
  - [x] 6.0 在 `src/App.test.tsx` 中新增 `bigrock:protection` 事件测试（验证 refreshTrigger 递增）

- [x] **Task 7: Rust 集成测试** (AC: #1, #3, #4)
  - [x] 7.0 在 `bigrock_protection.rs` 的 `#[cfg(test)] mod tests` 中编写集成测试（19 个测试全部通过）

## Dev Notes

### 关键技术决策

- **"进展"判定基于 `updated_at`**：AC1 原文提到"无编辑/无对话提及/未完成子步骤"，但系统无对话提及跟踪和子任务模型。使用 `updated_at` 早于本周一作为"本周无进展"的判定标准，是当前数据模型下最务实方案。`updated_at` 在任务标题/deadline/quadrant/完成状态变更时都会刷新。

- **频率控制复刻 Q2 提醒模式**：新建 `big_rock_protection_reminders` 表，结构与 `q2_reminders` 完全一致（`task_id` UNIQUE, `reminded_count`, `last_reminded_at`）。`upsert_reminder` 使用 INSERT ON CONFLICT 递增计数。每日最多 1 次提醒通过 `is_reminded_today(last_reminded_at)` 判断。

- **工作日检查**：AC1 说"工作日调度循环中检测"。`check_and_generate_protection_reminders` 内部判断当前是否为工作日（周一~周五），非工作日直接返回。使用 `chrono::Local::now().weekday()` 判断（`Mon=1 ~ Sun=7`，`weekday() >= 1 && weekday() <= 5` 为工作日）。

- **周五检查独立于日常检查**：AC4 的"周五大石头未完成检查"是一个汇总提醒（"还有 N 个未完成"），与 AC1 的逐任务提醒（"XX 这周还没动"）不同。使用独立的 `check_friday_bigrock_status` 函数，在调度器中用日期去重键控制每天只触发一次。

- **不设连续提醒上限**：Q2 提醒有"连续 3 天无响应后停止"的限制（`MAX_REMINDER_DAYS = 3`），但 AC 未要求大石头保护提醒有此限制。大石头是本周最重要的事，应持续提醒直到完成或本周结束。

- **通知级别为"轻触"**：AC3 明确要求 `NotificationLevel::Tap`。`notification_service::create_notification_for_role` 会自动处理 proactivity 约束（passive 角色降级为 whisper）和敲门上限。

- **butler 任务无 role_id**：`list_all_tasks` 返回的 `CrossRoleTask` 中 `role_id` 可能为 None（butler 任务）。创建通知需要 `role_id`，butler 大石头跳过通知创建，只写管家对话消息（参照 `q2_protection_reminder.rs:158-165`）。

- **事件命名**：`bigrock:protection`（域:动词过去式，遵循 Tauri Event 命名规范 `{domain}:{verb_past}`）。与 Story 6.3 的 `bigrock:reminder`（规划提醒）区分。

- **前端只需递增 refreshTrigger**：`bigrock:protection` 事件到达时，前端递增 `butlerChatRefreshTrigger` 刷新管家对话即可。通知已由现有 `notification:new` 机制自动处理。不需要打开 Modal 或其他 UI 变更。

### 架构合规

- **分层规则**：调度器 `scheduler.rs` → `bigrock_protection::check_and_generate_protection_reminders` → `db::tasks::list_all_tasks` / `db::big_rock_protection_reminders::upsert_reminder` / `notification_service::create_notification_for_role` / `db::conversations::insert_message`
- **Service 层职责**：`bigrock_protection.rs` 负责"查询大石头任务 → 判断进展 → 频率控制 → 创建通知 + 写对话 + emit 事件"
- **Command 层**：本 story 无新增 Tauri Command（纯后台调度器驱动，不需要前端主动调用）
- **命名规范**：Rust 文件 `bigrock_protection.rs`（snake_case），结构体 `BigRockProtectionReminder` / `BigRockProtectionPayload`（PascalCase），DB 表 `big_rock_protection_reminders`（snake_case 复数），Tauri Event `bigrock:protection`
- **错误处理**：所有失败降级为 `tracing::warn!`，不阻塞调度器循环
- **serde 桥接**：`BigRockProtectionPayload` 派生 `Serialize + Deserialize` + `#[serde(rename_all = "camelCase")]`

### 前端 UI 规范

- **不新增组件文件**：只在 `App.tsx` 中新增一个 `useTauriEvent` 监听
- **不修改现有 UI**：提醒通过管家对话消息和通知面板呈现，已有 UI 组件支持
- **空状态文案**：遵循 UX-DR18 — 不显示"暂无数据"
- **反馈模式**：遵循 UX-DR16 — 通过管家自然语言传达，不使用 toast/snackbar

### 反模式警告

- **不要**引入冲突检测或仲裁逻辑 — AC2 已确认延迟至 V2
- **不要**修改 `q2_protection_reminder.rs` — 只参照其模式新建 `bigrock_protection.rs`
- **不要**修改 `bigrock_reminder.rs`（Story 6.3）— 那是"无大石头时提醒规划"，本 story 是"有大石头但无进展时提醒保护"，职责不同
- **不要**修改 `db/tasks.rs` 中的 `list_all_tasks` 函数 — 直接复用
- **不要**在 `commands/` 层添加业务逻辑 — 本 story 无新增 Command
- **不要**让提醒失败阻塞调度器 — 所有错误降级为 `tracing::warn!`
- **不要**设置连续提醒上限 — 大石头应持续提醒直到完成或本周结束（与 Q2 提醒的 3 天上限不同）
- **不要**在非工作日生成 AC1 的逐任务提醒 — `check_and_generate_protection_reminders` 内部判断工作日
- **不要**在 `check_friday_bigrock_status` 中逐任务提醒 — 它是汇总提醒（"还有 N 个未完成"），只发一条
- **不要**在 `bigrock_protection.rs` 中直接调用 `invoke` — 这是纯后端 service
- **不要**修改 `scheduler.rs` 中已有的 Q2 保护检查、简报触发、大石头规划提醒、周复盘触发逻辑 — 只新增大石头保护检查调用

### Project Structure Notes

新增文件：
- `src-tauri/migrations/029_big_rock_protection_reminders.sql` — 频率控制表
- `src-tauri/src/models/big_rock_protection_reminder.rs` — model 结构体
- `src-tauri/src/db/big_rock_protection_reminders.rs` — DB 操作
- `src-tauri/src/services/bigrock_protection.rs` — 保护提醒服务

修改文件：
- `src-tauri/src/models/mod.rs` — 注册 `pub mod big_rock_protection_reminder;`
- `src-tauri/src/db/mod.rs` — 注册 `pub mod big_rock_protection_reminders;`
- `src-tauri/src/services/mod.rs` — 注册 `pub mod bigrock_protection;`
- `src-tauri/src/services/scheduler.rs` — 新增大石头保护检查调用 + 周五大检查触发
- `src/App.tsx` — 新增 `bigrock:protection` 事件监听
- `src/App.test.tsx` — 新增事件监听测试

不修改文件：
- `src-tauri/src/services/q2_protection_reminder.rs` — 只参照模式
- `src-tauri/src/services/bigrock_reminder.rs` — Story 6.3 的规划提醒，职责不同
- `src-tauri/src/db/tasks.rs` — 直接复用 `list_all_tasks`
- `src-tauri/src/services/notification_service.rs` — 直接复用
- `src-tauri/src/db/conversations.rs` — 直接复用
- `src-tauri/src/db/q2_reminders.rs` — 只参照模式

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.6] — AC 原文
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6] — Epic 6 上下文（FR-17, FR-18, FR-24）
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: _bmad-output/implementation-artifacts/6-5-weekly-review-modal-real-data.md] — Story 6.5 实现上下文（大石头批量创建 + AI 建议）
- [Source: _bmad-output/implementation-artifacts/6-3-big-rock-planning-reminder.md] — Story 6.3 实现上下文（大石头规划提醒模式）
- [Source: src-tauri/src/services/scheduler.rs:475-485] — Q2 保护检查调度器集成位置（参照插入点）
- [Source: src-tauri/src/services/scheduler.rs:322-570] — `spawn_scheduler` 完整结构
- [Source: src-tauri/src/services/q2_protection_reminder.rs] — 完整的"检查→频率控制→通知→对话→emit"模式（281 行，本 story 几乎复刻）
- [Source: src-tauri/src/services/q2_protection_reminder.rs:38-56] — `is_reminded_today` / `parse_iso_to_local_date` 日期工具函数
- [Source: src-tauri/src/services/q2_protection_reminder.rs:65-88] — `check_and_generate_reminders` 主函数模式
- [Source: src-tauri/src/services/q2_protection_reminder.rs:90-236] — `process_single_task` 逐任务处理模式
- [Source: src-tauri/src/services/bigrock_reminder.rs] — Story 6.3 大石头规划提醒（"无大石头→提醒规划"，与本 story "有大石头无进展→提醒保护" 职责不同）
- [Source: src-tauri/src/services/bigrock_reminder.rs:74-114] — `create_notification_if_possible` 模式（无 role_id 时跳过通知）
- [Source: src-tauri/migrations/018_q2_reminders.sql] — `q2_reminders` 表结构（参照建表）
- [Source: src-tauri/src/db/q2_reminders.rs] — `upsert_reminder` / `get_reminder_for_task` / `delete_reminder_for_task` DB 模式
- [Source: src-tauri/src/models/q2_reminder.rs] — `Q2Reminder` 结构体（参照定义）
- [Source: src-tauri/src/db/tasks.rs:60-85] — `list_all_tasks(pool, quadrant, is_big_rock)` 函数
- [Source: src-tauri/src/models/task.rs:50-70] — `Task` 结构体（`is_big_rock` / `is_completed` / `updated_at`）
- [Source: src-tauri/src/models/task.rs:84-106] — `CrossRoleTask` 结构体（含 `role_name` / `role_color`）
- [Source: src-tauri/src/services/notification_service.rs:57] — `create_notification_for_role` 函数
- [Source: src-tauri/src/services/task_protection_watch.rs:19] — `AT_RISK_DAYS = 3` 常量（Q2 保护判定阈值）
- [Source: src-tauri/src/lib.rs:274] — `spawn_scheduler` 注册位置
- [Source: src/App.tsx:87-94] — `useTauriEvent('q2:reminder', ...)` 事件监听模式
- [Source: src/App.tsx:105-114] — `useTauriEvent('bigrock:reminder', ...)` 事件监听模式
- [Source: src/App.test.tsx:130-132] — `bigrock:reminder` 事件测试模式

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (Windsurf Cascade)

### Debug Log References

### Completion Notes List

1. **Migration 序号调整**：故事 spec 指定 `029_big_rock_protection_reminders.sql`，但实际最新 migration 为 `025_weekly_reviews.sql`（026-028 不存在）。使用 `026` 避免序号空缺，SQLx migration 按文件名排序执行，功能不受影响。

2. **测试日期依赖修复**：`check_and_generate_protection_reminders` 包含工作日守卫（`is_weekday()`），集成测试在周末运行时会因守卫提前返回而失败。将核心逻辑抽取为 `check_and_generate_protection_reminders_inner`（私有函数），公开函数保留工作日守卫，测试直接调用内部函数验证核心逻辑。`is_weekday()` / `is_friday()` 由独立单元测试覆盖。

3. **周五检查测试条件化**：`check_friday_bigrock_status` 的集成测试使用 `is_friday()` 运行时判断，非周五时验证返回 `false`，周五时验证消息内容含"N 个未完成"。测试在任何日期运行都能通过。

4. **所有测试通过**：Rust 服务测试 19 passed、DB 层测试 4 passed、前端 App.test.tsx 7 passed（含新增 `bigrock:protection` 测试）。

### File List

**新增文件：**
- `egosync-app/src-tauri/migrations/026_big_rock_protection_reminders.sql` — 频率控制表
- `egosync-app/src-tauri/src/models/big_rock_protection_reminder.rs` — model 结构体
- `egosync-app/src-tauri/src/db/big_rock_protection_reminders.rs` — DB 操作（含 4 个单元测试）
- `egosync-app/src-tauri/src/services/bigrock_protection.rs` — 保护提醒服务（含 19 个单元/集成测试）

**修改文件：**
- `egosync-app/src-tauri/src/models/mod.rs` — 注册 `pub mod big_rock_protection_reminder;`
- `egosync-app/src-tauri/src/db/mod.rs` — 注册 `pub mod big_rock_protection_reminders;`
- `egosync-app/src-tauri/src/services/mod.rs` — 注册 `pub mod bigrock_protection;`
- `egosync-app/src-tauri/src/services/scheduler.rs` — 新增大石头保护检查调用 + 周五大检查触发 + 去重变量
- `egosync-app/src-tauri/src/db/tasks.rs` — `update_task` / `set_completed` 中增加大石头保护提醒记录清理（Review F2 修复）
- `egosync-app/src/App.tsx` — 新增 `bigrock:protection` 事件监听
- `egosync-app/src/App.test.tsx` — 新增 `bigrock:protection` 事件测试

### Review Findings

- [x] [Review][Patch] 接入大石头提醒清理（决策已定：接入清理）[egosync-app/src-tauri/src/db/tasks.rs:163,293] — 仿照 `q2_reminders::delete_reminder_for_task`，在 `update_task`(line 163 旁) 与 `set_completed`(line 293 旁) 各加一行调用 `big_rock_protection_reminders::delete_reminder_for_task`，避免 `big_rock_protection_reminders` 表为已完成/已编辑大石头残留无用行。注：本改动修改了 spec 标注"不要修改 db/tasks.rs"的文件，属与既有 Q2 清理约定一致的合理偏离，已记录。
- [x] [Review][Patch] `is_updated_today` 守卫不可达且测试名不副实 [egosync-app/src-tauri/src/services/bigrock_protection.rs] — 已移除不可达守卫、未使用的 `is_updated_today` 私有函数及其 3 个冗余/误导测试。AC3「当日有进展不提醒」由上游 `!is_updated_this_week` 周过滤器保证，coverage 保留在 `check_protection_skips_when_bigrock_updated_this_week`。34 个相关测试通过。
- [x] [Review][Defer] 周五去重为内存变量，应用重启会重复 [egosync-app/src-tauri/src/services/scheduler.rs:1088] — `last_bigrock_friday_check_date` 在内存中，周五当天重启会重置导致重复发汇总；与既有周复盘去重模式一致，非本故事独创。deferred, pre-existing pattern
- [x] [Review][Defer] 周五对陈旧大石头双重提醒 [egosync-app/src-tauri/src/services/bigrock_protection.rs] — 周五同一陈旧大石头既收 AC1 逐任务提醒又被计入 AC4 汇总；spec 定义为不同职责，属设计取舍。deferred, by-design
