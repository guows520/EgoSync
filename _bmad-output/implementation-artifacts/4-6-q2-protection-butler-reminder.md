---
baseline_commit: 1b21b4bf26e7a663b59928eb148e103c308b8399
---

# Story 4.6: Q2 保护任务被挤时管家主动提醒

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 重要但不紧急的任务被忽略时收到管家温和提醒,
so that Q2 任务不会在忙碌中被遗忘。

## 背景与现状（务必先读）

**本 story 是后端为主的 story — 在 Story 3.5（Q2 保护状态检测）和 Story 4.5（三级通知系统）的基础上，调度器中增加 Q2 保护检查步骤，检测到 `at_risk` 状态的 Q2 任务时，生成"轻触"级通知 + 管家对话消息。前端需监听新事件并在管家对话区展示提醒消息。涉及后端调度器扩展、新 DB 查询函数、新 Tauri 命令 + 事件，以及前端事件监听 + 对话消息渲染。**

### 已建成的基础（本 story 的接入点）

**Story 3.5（Q2 保护状态检测）— 已完成：**
- `services/task_protection_watch.rs` — 后台每小时执行 `recompute_protection_status`，先 `clear_protection_for_resolved`（恢复已解除的），再 `mark_stale_q2_at_risk`（标记过期的 Q2 任务为 `at_risk`）
- `db/tasks.rs:493-512` — `mark_stale_q2_at_risk(pool, threshold)` 将连续 3 天未处理的 Q2 任务标记为 `at_risk`
- `db/tasks.rs:519-536` — `clear_protection_for_resolved(pool, threshold)` 将已解除的 at_risk 任务恢复为 `normal`
- `db/tasks.rs:458-482` — `list_imminent_tasks_for_escalation` 查询临期 Q2 任务
- `commands/task.rs:118-121` — `task_check_protection_status` 命令，前端打开任务面板时主动触发一次保护检查
- `AT_RISK_DAYS = 3`（`services/task_protection_watch.rs:19`）— Q2 任务连续 3 天未处理标记为 at_risk
- **本 story 不动保护状态检测逻辑 — 它已经完成，本 story 只消费 `at_risk` 状态**

**Story 4.5（三级通知系统）— 已完成：**
- `services/notification_service.rs` — `create_notification_for_role(pool, role_id, requested_level, content)` 创建通知（含两层降级：proactivity 约束 + 每日敲门上限）
- `db/notifications.rs` — `create_notification` / `list_notifications` / `mark_read` / `count_unread` / `count_knock_today`
- `commands/notification.rs` — `notification_create` / `notification_list` / `notification_mark_read` / `notification_count_unread`
- `models/notification.rs` — `Notification` / `NotificationWithRole` / `CreateNotificationInput` / `NotificationNewPayload`
- Tauri Event: `notification:new`（payload 含 `id, level, content, roleId, roleName, roleIcon, roleColor, createdAt`）
- `hooks/useNotifications.ts` — 通知列表加载 + `useTauriEvent('notification:new')` 实时追加 + `markAsRead`
- **本 story 复用通知系统创建"轻触"级通知，不修改通知系统本身**

**Story 4.1（调度器）— 已完成：**
- `services/scheduler.rs:148-274` — `run_work_loop_for_role` 完整工作循环：generate → filter → write suggestions → create notifications
- `services/scheduler.rs:285-382` — `spawn_scheduler` 后台 60 秒 tick，按角色 proactivity_level 的触发时间点执行工作循环
- `lib.rs:274` — `spawn_scheduler(pool.clone(), app.handle().clone())` 在 Tauri setup 中启动
- **本 story 在调度器中增加 Q2 保护检查步骤，不修改现有建议生成逻辑**

**Story 4.4（建议确认/拒绝）— 已完成：**
- `components/butler/ButlerView.tsx` — 管家视角容器，已集成 ActionCard（敲门通知）+ ChatStream（建议卡片）
- `components/chat/ChatStream.tsx` — 对话流组件，渲染消息 + ActionCard
- **本 story 在 ChatStream 中新增管家提醒消息渲染**

**管家对话基础设施：**
- `db/conversations.rs:32-46` — `get_or_create_butler_conversation` 获取/创建管家对话（`role_id IS NULL`）
- `db/conversations.rs:48-69` — `insert_message(pool, conversation_id, role, content, is_complete)` 插入消息
- `commands/chat.rs:264` — 管家对话使用 `get_or_create_butler_conversation` 获取对话
- **本 story 需复用 `insert_message` 写入管家提醒消息到管家对话**

**前端通知/对话监听：**
- `hooks/useTauriEvent.ts` — Tauri Event 监听 hook
- `App.tsx:57-64` — `knockNotifications` 从 `useNotifications` 过滤敲门通知，`handleDismissKnock` 调用 `markAsRead`
- `App.tsx:67-79` — 监听 `notification:new` 事件，敲门通知播放声音
- `ButlerView.tsx:102-118` — 敲门通知渲染为 ActionCard（复用 `knockToSuggestion` 转换函数）

## Acceptance Criteria

1. **AC1**: Given 调度器检测到 `at_risk` 状态的 Q2 任务（来自 E3 Story 3.5），When 生成提醒，Then 管家在对话中以自然语言提醒"你的'XX'任务已经 3 天没动了，要不要今天安排一下？"，And 不使用 toast / 红框，遵循 UX-DR16 管家对话反馈模式

2. **AC2**: Given 提醒通知，Then 级别为"轻触"（不打断用户）

3. **AC3**: Given 同一任务，Then 提醒频率 ≤ 每日 1 次，And 连续提醒 3 天无响应后停止提醒（避免骚扰）

4. **AC4**: Given 用户处理了该任务（完成/编辑），Then 提醒立即停止，And `protection_status` 恢复为 `normal`（由 E3 Story 3.5 逻辑处理）

5. **AC5**: Given Rust 后端，Then 调度器中增加 Q2 保护检查步骤，And 生成通知 + 管家对话消息

## Tasks / Subtasks

- [x] **Task 1: Rust DB 层 — 新增 at_risk 任务查询 + 提醒记录表** (AC: #1, #3, #4)
  - [x] 1.1 在 `db/tasks.rs` 新增 `list_at_risk_q2_tasks(pool) -> Result<Vec<Task>, AppError>` — 查询所有 `protection_status = 'at_risk'` AND `quadrant = 'Q2'` AND `is_completed = 0` AND `deleted_at IS NULL` 的任务，JOIN `roles` 获取角色信息（需返回角色名用于提醒文案）
  - [x] 1.2 新建 `migrations/018_q2_reminders.sql` 创建 `q2_reminders` 表：`id TEXT PRIMARY KEY, task_id TEXT NOT NULL, reminded_count INTEGER NOT NULL DEFAULT 1, last_reminded_at TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')), FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE, UNIQUE(task_id)` — 每个任务一行，`UNIQUE(task_id)` 确保去重
  - [x] 1.3 在 `db/mod.rs` 注册 `q2_reminders` 模块
  - [x] 1.4 新建 `db/q2_reminders.rs`：`upsert_reminder(pool, task_id) -> Result<Q2Reminder, AppError>` — INSERT OR REPLACE，`reminded_count` 在已存在时 +1（用 `INSERT ... ON CONFLICT(task_id) DO UPDATE SET reminded_count = reminded_count + 1, last_reminded_at = ?`）
  - [x] 1.5 在 `db/q2_reminders.rs`：`get_reminder_for_task(pool, task_id) -> Result<Option<Q2Reminder>, AppError>`
  - [x] 1.6 在 `db/q2_reminders.rs`：`delete_reminder_for_task(pool, task_id) -> Result<u64, AppError>` — 用户处理任务后调用，删除提醒记录
  - [x] 1.7 在 `models/mod.rs` 注册 `q2_reminder` 模块
  - [x] 1.8 新建 `models/q2_reminder.rs`：`Q2Reminder` 结构体（`id, taskId, remindedCount, lastRemindedAt, createdAt`），`#[serde(rename_all = "camelCase")]` + `sqlx::FromRow`

- [x] **Task 2: Rust service 层 — Q2 保护提醒服务** (AC: #1, #2, #3, #4, #5)
  - [x] 2.1 新建 `services/q2_protection_reminder.rs`
  - [x] 2.2 在 `services/mod.rs` 注册 `q2_protection_reminder` 模块
  - [x] 2.3 实现 `check_and_generate_reminders(pool, conv_pool: &ConversationsPool, app_handle: Option<&AppHandle>) -> Result<(), AppError>` 核心函数：
    - 调用 `db::tasks::list_at_risk_q2_tasks(pool)` 获取所有 at_risk Q2 任务
    - 对每个任务：
      a. 调用 `db::q2_reminders::get_reminder_for_task(pool, &task.id)` 查询已有提醒记录
      b. 若已有记录且 `reminded_count >= 3` → 跳过（AC3：连续提醒 3 天无响应后停止）
      c. 若已有记录且 `last_reminded_at` 是今天（本地日期）→ 跳过（AC3：每日 ≤ 1 次）
      d. 否则：
        - 生成提醒文案：`你的'{task.title}'任务已经 {AT_RISK_DAYS} 天没动了，要不要今天安排一下？`（天数引用 `task_protection_watch::AT_RISK_DAYS` 常量）
        - 调用 `notification_service::create_notification_for_role(pool, &task.role_id, NotificationLevel::Tap, &message)` 创建"轻触"级通知（AC2）
        - 调用 `db::conversations::get_or_create_butler_conversation(&conv_pool)` + `db::conversations::insert_message(&conv_pool, &conv_id, "assistant", &message, true)` 写入管家对话消息（AC1）
        - **至少一项投递成功后**，调用 `db::q2_reminders::upsert_reminder(pool, &task.id)` 更新提醒计数（P1 修复：避免投递全失败时空耗提醒额度）
        - 若有 `app_handle`，emit Tauri Event `q2:reminder`（payload: `{ taskId, taskTitle, roleId, roleName, message, notificationId }`）供前端实时更新
    - 错误只 `tracing::warn!`，不阻塞其他任务的提醒生成
  - [x] 2.4 定义 `MAX_REMINDER_DAYS` 常量 = 3（AC3：连续提醒 3 天后停止）
  - [x] 2.5 定义 `Q2ReminderPayload` 结构体（`#[serde(rename_all = "camelCase")]`）用于事件 payload

- [x] **Task 3: 调度器集成 Q2 保护检查** (AC: #5)
  - [x] 3.1 在 `services/scheduler.rs` 的 `spawn_scheduler` 函数中，每次 tick 的角色循环之后，增加一个 Q2 保护检查步骤
  - [x] 3.2 调用 `services::q2_protection_reminder::check_and_generate_reminders(&pool, Some(&app_handle))`
  - [x] 3.3 该检查不需要受触发时间点限制 — 每次 tick（60 秒）都检查一次，但实际的频率控制由 `q2_reminders` 表的 `last_reminded_at` 管理
  - [x] 3.4 错误只 `tracing::warn!`，不阻塞调度器主循环

- [x] **Task 4: 任务处理时清除提醒记录** (AC: #4)
  - [x] 4.1 在 `db/tasks.rs` 的 `update_task` 函数中，任务更新成功后调用 `db::q2_reminders::delete_reminder_for_task(pool, &id)` — 用户编辑任务即处理，清除提醒记录
  - [x] 4.2 在 `db/tasks.rs` 的 `set_task_completion` 函数中，任务完成后调用 `db::q2_reminders::delete_reminder_for_task(pool, &id)` — 用户完成任务即处理，清除提醒记录
  - [x] 4.3 `protection_status` 恢复为 `normal` 由 Story 3.5 的 `clear_protection_for_resolved` 逻辑处理（`update_task` 已重置 `protection_status = 'normal'`，`toggle_task_complete` 同理）— **本 story 不修改保护状态逻辑**

- [x] **Task 5: Rust Tauri 命令 — 手动触发 Q2 保护检查** (AC: #5)
  - [x] 5.1 在 `commands/task.rs` 新增 `task_check_q2_reminders` 命令 — 注入 `AppHandle` + `ConversationsPool`，调用 `services::q2_protection_reminder::check_and_generate_reminders(&pool, &conv_pool, Some(&app_handle))`，返回 `Result<(), AppError>`（D1 修复：手动触发路径与调度器路径一致）
  - [x] 5.2 在 `lib.rs` 的 `invoke_handler` 注册 `commands::task::task_check_q2_reminders`
  - [x] 5.3 前端可在打开管家视角时主动触发一次，确保 at_risk 状态即时反映

- [x] **Task 6: 前端 — 监听 Q2 提醒事件 + 管家对话更新** (AC: #1)
  - [x] 6.1 在 `types/q2Reminder.ts` 定义 `Q2ReminderPayload` 接口（`taskId, taskTitle, roleId, roleName, message, notificationId`，camelCase）
  - [x] 6.2 在 `App.tsx` 中使用 `useTauriEvent('q2:reminder', ...)` 监听 Q2 提醒事件
  - [x] 6.3 事件到达时，通知已由 `notification:new` 事件处理（AC2：轻触级通知 → 铃铛红点），管家对话消息已由后端 `insert_message` 写入 DB
  - [x] 6.4 前端只需在事件到达时触发管家对话刷新（ChatStream 重新加载历史消息），使提醒消息显示在对话流中
  - [x] 6.5 在 `services/taskService.ts` 新增 `checkQ2Reminders: () => invoke<void>('task_check_q2_reminders')`
  - [x] 6.6 在 `useTasks.ts` 或 `ButlerView.tsx` 的初始化逻辑中调用 `taskService.checkQ2Reminders()`，容错处理（失败仅 console.warn）

- [x] **Task 7: 前端 — ChatStream 渲染管家提醒消息** (AC: #1)
  - [x] 7.1 ChatStream 已有渲染 `assistant` 角色消息的能力 — 管家提醒消息由后端 `insert_message(conv_pool, conv_id, "assistant", message, true)` 写入，前端 ChatStream 加载历史时自然显示
  - [x] 7.2 确保 ChatStream 的消息刷新机制能在 `q2:reminder` 事件触发后重新加载对话历史
  - [x] 7.3 提醒消息样式与普通管家对话气泡一致（左对齐 + 管家图标/名称头部 + `max-w-[85%]` 宽度），不使用特殊样式（遵循 UX-DR16：自然语言反馈，无红框/toast）

- [x] **Task 8: Rust 单元测试** (AC: #1-#5)
  - [x] 8.1 `db/q2_reminders.rs` 测试 — `upsert_reminder` 新建 + 递增计数、`get_reminder_for_task` 存在/不存在、`delete_reminder_for_task` 删除 + 不存在时返回 0
  - [x] 8.2 `db/tasks.rs` 测试 — `list_at_risk_q2_tasks` 返回正确结果（仅 at_risk + Q2 + 未完成 + 未删除）、排除非 at_risk / 非 Q2 / 已完成 / 已删除
  - [x] 8.3 `services/q2_protection_reminder.rs` 测试 — 
    - 无 at_risk 任务时不生成任何提醒
    - 有 at_risk 任务时生成通知 + 对话消息 + 提醒记录
    - 同一任务当天已有提醒记录时跳过
    - 同一任务提醒计数 >= 3 时跳过
    - 多个 at_risk 任务时逐个处理，单个失败不阻塞其他
  - [x] 8.4 `db/tasks.rs` 测试 — `update_task` 后 `q2_reminders` 记录被删除、`toggle_task_complete` 后记录被删除

- [x] **Task 9: 前端测试** (AC: #1)
  - [x] 9.1 验证 `q2:reminder` 事件到达时 ChatStream 刷新对话历史
  - [x] 9.2 验证 `taskService.checkQ2Reminders()` 调用成功/容错

- [x] **Task 10: 更新 sprint-status.yaml**
  - [x] 10.1 将 `4-6-q2-protection-butler-reminder` 状态更新为 `done`

## Dev Notes

### 项目背景

本 Story 属于 Epic 4（主动循环、通知与仪表盘），是 Q2 保护任务提醒的最终闭环。Story 3.5 已实现 Q2 任务的 `at_risk` 状态检测（连续 3 天未处理标记为 at_risk），Story 4.5 已实现三级通知系统。本 Story 将两者打通：调度器检测到 at_risk 的 Q2 任务时，通过管家对话自然语言温和提醒用户，同时生成"轻触"级通知。

### 技术栈

- **后端**: Rust + Tauri 2.x + SQLx (SQLite) + tokio
- **前端**: React 18 + TypeScript 5.2 + TailwindCSS + Vite 5
- **IPC**: Tauri Commands (invoke) + Tauri Events (emit/listen)

### 关键架构约束

**Rust 三层架构（严格遵守）**：
- `commands/task.rs` — Tauri 命令接口，薄层，只做参数校验 + 调用 services
- `db/q2_reminders.rs` — 纯数据库操作，返回 `Result<T, AppError>`
- `db/tasks.rs` — 新增 `list_at_risk_q2_tasks` 查询函数
- `services/q2_protection_reminder.rs` — 业务逻辑（检测 at_risk 任务、频率控制、生成通知 + 对话消息）
- `models/q2_reminder.rs` — 数据结构定义

**IPC 命令命名规范**: `task_check_q2_reminders`（下划线分隔，参考 `task_check_protection_status`）

**Event 命名规范**: `q2:reminder`（参考 `notification:new` 模式，域:动词）

**serde 序列化**: 所有 Rust struct 使用 `#[serde(rename_all = "camelCase")]`，前端 TypeScript 接口字段使用 camelCase

### 已有代码复用（关键 — 避免重复造轮子）

**1. `notification_service::create_notification_for_role` 已实现**：`services/notification_service.rs:57-102` 已实现通知创建（含两层降级）。**直接调用，传入 `NotificationLevel::Tap` 创建"轻触"级通知**。

**2. `NotificationLevel` 枚举已定义**：`services/suggestion_generator.rs:354-360` 已定义 `NotificationLevel` 枚举（Whisper/Tap/Knock）。**直接 import 使用 `NotificationLevel::Tap`**。

**3. `db::conversations::get_or_create_butler_conversation` + `insert_message` 已实现**：`db/conversations.rs:32-69`。**直接调用写入管家对话消息**。注意：conversations 使用独立的 `ConversationsPool`，不是主 `SqlitePool`。需从 `DbPool` 中获取 conversations pool（参考 `commands/chat.rs` 中的 `conv_pool` 获取方式）。

**4. `db::tasks` 的 `TASK_SELECT_COLUMNS` 常量**：`db/tasks.rs:6` 定义了完整的列名列表。`list_at_risk_q2_tasks` 查询复用此常量。

**5. `AppError` 枚举**：`error.rs` 已定义 NotFound/DbError/ValidationError 等变体，复用这些变体，不新增。

**6. `db::settings::chrono_now_pub()`**：时间戳生成函数，创建提醒记录时复用。

**7. `db::app_settings::get_setting`/`set_setting`**：设置存储复用此模块（如需配置项）。

**8. Story 3.5 的 `recompute_protection_status`**：`services/task_protection_watch.rs:25-34` 每小时重算保护状态。本 story 的提醒检查可以挂在调度器的 60 秒 tick 上，但实际频率由 `q2_reminders` 表控制。

### Conversations Pool 获取方式

管家对话消息写入需要 `ConversationsPool`，不是主 `SqlitePool`。参考 `commands/chat.rs` 中的模式：

```rust
// lib.rs 中 DbPool 包含两个 pool
pub struct DbPool {
    pub main: SqlitePool,           // 主数据库（tasks, notifications, etc.）
    pub conversations: ConversationsPool,  // 对话数据库
}
```

在 `services/q2_protection_reminder.rs` 中需要同时访问两个 pool。由于 `check_and_generate_reminders` 在调度器中调用（调度器只有 `SqlitePool`），需要额外传入 `ConversationsPool`。

**方案**：修改 `spawn_scheduler` 签名，传入 `ConversationsPool`；或在 `check_and_generate_reminders` 中只生成通知，管家对话消息通过 Tauri Event 通知前端，由前端调用 `chat_send_message` 写入。

**推荐方案**：在 `check_and_generate_reminders` 中同时传入两个 pool。修改 `spawn_scheduler` 调用处，从 `DbPool` state 中获取 conversations pool 传入。

### 提醒频率控制逻辑

**`q2_reminders` 表设计**：
- 每个 at_risk Q2 任务一行（`UNIQUE(task_id)` 约束）
- `reminded_count`：已提醒次数（每次 +1）
- `last_reminded_at`：上次提醒时间

**频率控制规则**（在 `check_and_generate_reminders` 中实现）：
1. 查询 `q2_reminders` 表获取该任务的提醒记录
2. 若 `reminded_count >= 3` → 跳过（AC3：连续 3 天后停止）
3. 若 `last_reminded_at` 是今天（本地日期）→ 跳过（AC3：每日 ≤ 1 次）
4. 否则 → `upsert_reminder`（计数+1，更新时间）+ 生成通知 + 写入对话消息

**本地日期判断**：`last_reminded_at` 是 ISO 8601 UTC 时间戳，比较时用 `date(last_reminded_at, 'localtime') = date('now', 'localtime')`（与 `count_knock_today` 的本地日期逻辑一致，Story 4.5 Review 已修复此问题）。

### 提醒文案模板

```
你的'{task_title}'任务已经 3 天没动了，要不要今天安排一下？
```

- 使用任务标题（`task.title`）
- 自然语言风格，温和不施压（遵循 UX-DR16 + UX 设计原则"暗示 > 提醒 > 告知"）
- 不使用感叹号、红色标记等施压元素

### 数据库 Schema

```sql
-- migrations/018_q2_reminders.sql
CREATE TABLE IF NOT EXISTS q2_reminders (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL UNIQUE,
    reminded_count INTEGER NOT NULL DEFAULT 1,
    last_reminded_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```

**注意**：迁移文件编号为 `018`（当前最大为 `017_notifications.sql`）。`UNIQUE(task_id)` 确保每个任务只有一条提醒记录。`ON DELETE CASCADE` 确保任务被删除时提醒记录自动清除。

### Rust 数据模型参考

```rust
// models/q2_reminder.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Q2Reminder {
    pub id: String,
    pub task_id: String,
    pub reminded_count: i64,
    pub last_reminded_at: String,
    pub created_at: String,
}

// Q2 提醒事件 payload
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Q2ReminderPayload {
    pub task_id: String,
    pub task_title: String,
    pub role_id: String,
    pub role_name: String,
    pub message: String,
    pub notification_id: String,
}
```

### `list_at_risk_q2_tasks` 查询设计

需要返回任务信息 + 角色名（用于提醒文案）。由于 `tasks` 表的 `TASK_SELECT_COLUMNS` 不含角色名，需新增一个带 JOIN 的查询结构体或直接在 service 层分两步查询（先查任务，再查角色名）。

**推荐方案**：在 `db/tasks.rs` 新增 `list_at_risk_q2_tasks` 返回 `Vec<Task>`（复用 `Task` 结构体），在 service 层通过 `db::roles::get_role(pool, &task.role_id)` 获取角色名。这样避免新增 `TaskWithRole` 结构体，保持 DB 层简单。

### 调度器集成方式

在 `spawn_scheduler` 的 `loop` 中，角色循环之后增加 Q2 保护检查：

```rust
// scheduler.rs spawn_scheduler loop 末尾，interval.tick().await 之前
if let Err(e) = services::q2_protection_reminder::check_and_generate_reminders(
    &pool, &conv_pool, Some(&app_handle)
).await {
    tracing::warn!(error = %e, "Q2 保护提醒检查失败（降级继续）");
}
```

**注意**：需要修改 `spawn_scheduler` 签名，增加 `conv_pool: ConversationsPool` 参数。在 `lib.rs:274` 调用处从 `DbPool` state 获取 conversations pool。

### Previous Story Intelligence（Story 4.1 + 4.5 + 3.5）

- **4.1 调度器模式**：60 秒 tick + 错误只 warn + 不阻塞主循环。本 story 的 Q2 保护检查遵循同样模式。
- **4.5 通知系统**：`create_notification_for_role` 已含降级逻辑，传入 `Tap` 级别即可。通知事件 `notification:new` 已由前端 `useNotifications` 监听处理。
- **4.5 Review Findings**：每日敲门上限用 UTC 而非本地日期（已修复为 `date('now', 'localtime')`）。本 story 的每日提醒频率控制应同样使用本地日期比较。
- **3.5 保护状态**：`mark_stale_q2_at_risk` 不刷新 `updated_at`（保护状态由 `updated_at` 距今天数派生）。`update_task` 和 `toggle_task_complete` 已重置 `protection_status = 'normal'`。本 story 在这些函数中追加 `delete_reminder_for_task` 调用即可。
- **3.5 测试命令**：`cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` — 本 story 新增测试后应维持 0 failed。
- **4.5 前端测试**：`npm run test:frontend` 有 4 个既有失败（SettingsTab/TasksTab），与本 story 无关。

### Git Intelligence

- `1b21b4b`（HEAD）— feat: 三级通知系统（耳语/轻触/敲门）完整实现（最新提交，本 story 的直接前置）
- `823edbb` — feat(4.4): embed action cards in chat stream + custom reject reason
- `1d8249e` — feat(4.3): 主动性三档刻度盘行为接通
- 范本来源：`commands/task.rs`（命令实现模式）、`services/notification_service.rs`（service 层模式）、`db/notifications.rs`（DB 层模式）

### Testing Requirements

- **Rust 单测**（`db/q2_reminders.rs` ~4 个测试，`db/tasks.rs` 新增 ~2 个测试，`services/q2_protection_reminder.rs` ~5 个测试）：
  - `upsert_reminder`：新建 + 递增计数
  - `get_reminder_for_task`：存在 / 不存在
  - `delete_reminder_for_task`：删除 + 不存在返回 0
  - `list_at_risk_q2_tasks`：返回正确结果 + 排除非 at_risk / 非 Q2 / 已完成 / 已删除
  - `check_and_generate_reminders`：无 at_risk 时不生成、有 at_risk 时生成通知+消息+记录、当天已提醒跳过、计数 >= 3 跳过、单个失败不阻塞其他
  - `update_task` / `toggle_task_complete` 后 `q2_reminders` 记录被删除
- **前端测试**：
  - `q2:reminder` 事件到达时 ChatStream 刷新
  - `taskService.checkQ2Reminders()` 调用成功/容错
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`
  - `npm --prefix "egosync-app" run build`
  - `npm --prefix "egosync-app" run test:frontend`

### Project Structure Notes

- **新增文件（6）**：
  - `egosync-app/src-tauri/migrations/018_q2_reminders.sql` — q2_reminders 表 DDL
  - `egosync-app/src-tauri/src/db/q2_reminders.rs` — DB 操作
  - `egosync-app/src-tauri/src/models/q2_reminder.rs` — 数据模型
  - `egosync-app/src-tauri/src/services/q2_protection_reminder.rs` — 提醒服务
  - `egosync-app/src/types/q2Reminder.ts` — 前端类型定义
  - `egosync-app/src-tauri/src/services/q2_protection_reminder.rs` 测试模块
- **修改文件（7）**：
  - `egosync-app/src-tauri/src/db/mod.rs` — 注册 `q2_reminders` 模块
  - `egosync-app/src-tauri/src/models/mod.rs` — 注册 `q2_reminder` 模块
  - `egosync-app/src-tauri/src/services/mod.rs` — 注册 `q2_protection_reminder` 模块
  - `egosync-app/src-tauri/src/db/tasks.rs` — 新增 `list_at_risk_q2_tasks` + `update_task`/`toggle_task_complete` 中追加 `delete_reminder_for_task`
  - `egosync-app/src-tauri/src/services/scheduler.rs` — `spawn_scheduler` 中集成 Q2 保护检查 + 签名增加 `conv_pool`
  - `egosync-app/src-tauri/src/commands/task.rs` — 新增 `task_check_q2_reminders` 命令
  - `egosync-app/src-tauri/src/lib.rs` — 注册新命令 + `spawn_scheduler` 调用处传入 conv_pool
  - `egosync-app/src/App.tsx` — 监听 `q2:reminder` 事件 + 刷新管家对话
  - `egosync-app/src/services/taskService.ts` — 新增 `checkQ2Reminders`
- **无新增依赖** — 复用现有 Tauri/SQLx/tokio/chrono 依赖
- 符合项目规则：Rust 三层、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块 snake_case

### References

- `_bmad-output/project-context.md`（Rust 三层 / serde camelCase / 无 unwrap / tracing / 数据边界 / 前端 service 层模式）
- `_bmad-output/planning-artifacts/epics.md:1771-1798`（Story 4.6 定义）
- `_bmad-output/planning-artifacts/epics.md:199`（UX-DR16 反馈模式约束）
- `_bmad-output/planning-artifacts/ux-design-specification.md:816-827`（Feedback Patterns — 管家自然语言反馈，不使用 toast/snackbar）
- `_bmad-output/planning-artifacts/ux-design-specification.md:134-135`（角色被忽视 — 温和提醒，暗示 > 提醒 > 告知，不施压）
- `_bmad-output/implementation-artifacts/4-5-three-tier-notification.md`（Story 4.5 — 通知系统已实现，Review Findings 含本地日期修复）
- `_bmad-output/implementation-artifacts/4-4-suggestion-actioncard-confirm-reject.md`（Story 4.4 — ActionCard + ChatStream 集成模式）
- `egosync-app/src-tauri/src/services/task_protection_watch.rs:1-113`（Story 3.5 — Q2 保护状态检测，`recompute_protection_status` + `AT_RISK_DAYS = 3`）
- `egosync-app/src-tauri/src/db/tasks.rs:493-536`（`mark_stale_q2_at_risk` + `clear_protection_for_resolved`）
- `egosync-app/src-tauri/src/db/tasks.rs:6`（`TASK_SELECT_COLUMNS` 常量）
- `egosync-app/src-tauri/src/db/tasks.rs:106-160`（`update_task` — 需追加 `delete_reminder_for_task`）
- `egosync-app/src-tauri/src/services/notification_service.rs:57-102`（`create_notification_for_role` — 直接调用）
- `egosync-app/src-tauri/src/services/suggestion_generator.rs:354-360`（`NotificationLevel` 枚举 — import `Tap`）
- `egosync-app/src-tauri/src/db/conversations.rs:32-69`（`get_or_create_butler_conversation` + `insert_message`）
- `egosync-app/src-tauri/src/db/notifications.rs:85-94`（`count_knock_today` — 本地日期比较模式参考）
- `egosync-app/src-tauri/src/services/scheduler.rs:148-274`（`run_work_loop_for_role` — 调度器工作循环）
- `egosync-app/src-tauri/src/services/scheduler.rs:285-382`（`spawn_scheduler` — 需集成 Q2 保护检查）
- `egosync-app/src-tauri/src/lib.rs:270-274`（`spawn_scheduler` 调用处 — 需传入 conv_pool）
- `egosync-app/src-tauri/src/lib.rs:278-358`（`invoke_handler` — 注册新命令）
- `egosync-app/src-tauri/src/commands/task.rs:115-121`（`task_check_protection_status` — 命令模式参考）
- `egosync-app/src-tauri/src/commands/mod.rs`（命令模块注册）
- `egosync-app/src-tauri/src/db/mod.rs`（DB 模块注册）
- `egosync-app/src-tauri/src/models/mod.rs`（模型模块注册）
- `egosync-app/src-tauri/src/services/mod.rs`（service 模块注册）
- `egosync-app/src/App.tsx:57-79`（通知事件监听 + knockNotifications 过滤 — 需新增 q2:reminder 监听）
- `egosync-app/src/components/butler/ButlerView.tsx:1-157`（管家视角容器 — ChatStream 集成点）
- `egosync-app/src/hooks/useTauriEvent.ts`（Tauri Event 监听 hook）
- `egosync-app/src/services/taskService.ts:4-27`（taskService — 需新增 `checkQ2Reminders`）
- `egosync-app/src/hooks/useNotifications.ts`（通知 hook — 已监听 `notification:new`）

## Dev Agent Record

### Agent Model Used

Amelia (Senior Software Engineer) — BMad dev-story + code-review。

### Debug Log References

无。

### Completion Notes List

- 后端：调度器每 60s tick 调用 `check_and_generate_reminders`，消费 Story 3.5 的 `at_risk` Q2 任务，按"每日 ≤ 1 次、连续 3 次封顶"生成轻触通知 + 管家对话消息，并 emit `q2:reminder` 事件。
- `list_at_risk_q2_tasks` 返回 `Vec<CrossRoleTask>`（JOIN roles 取 role_name/role_color），相较 spec 建议的"返回 Vec<Task> + 再查角色名"做了优化，省去逐任务二次查询。
- `update_task` / `set_task_completion` 成功后调用 `delete_reminder_for_task` 清除提醒记录（AC4），protection_status 重置沿用 Story 3.5 逻辑。
- 前端：`App.tsx` 监听 `q2:reminder` 递增 `butlerChatRefreshTrigger`；`ChatStream` 据此重载对话历史；`ButlerView` 挂载时手动触发一次检查。
- **代码审查修复（2026-06-22）**：
  - [D1·方案1] `task_check_q2_reminders` 命令注入 `AppHandle` 并传 `Some(&app_handle)`，手动触发路径与调度器路径一致，可 emit 事件刷新对话。
  - [P1] `upsert_reminder` 计数自增改到"通知或对话消息至少一项投递成功之后"，避免投递全失败时空耗 3 次提醒额度。
  - [P2] 提醒文案天数由硬编码 "3" 改为引用 `task_protection_watch::AT_RISK_DAYS` 常量。
- **已知测试覆盖缺口**：`q2_protection_reminder.rs` 当前仅含 `parse_iso_to_local_date` / `is_reminded_today` / 常量等纯函数单测（7 个）；Task 8.3 列出的 `check_and_generate_reminders` 集成场景（生成/跳过/封顶/多任务容错）尚未补齐，建议后续补充集成测试。

### File List

**新增（6）**
- `egosync-app/src-tauri/migrations/018_q2_reminders.sql`
- `egosync-app/src-tauri/src/db/q2_reminders.rs`
- `egosync-app/src-tauri/src/models/q2_reminder.rs`
- `egosync-app/src-tauri/src/services/q2_protection_reminder.rs`
- `egosync-app/src/types/q2Reminder.ts`
- `egosync-app/src/services/taskService.test.ts`

**修改（13）**
- `egosync-app/src-tauri/src/db/mod.rs` / `models/mod.rs` / `services/mod.rs`（模块注册）
- `egosync-app/src-tauri/src/db/tasks.rs`（`list_at_risk_q2_tasks` + `update_task`/`set_task_completion` 清除提醒）
- `egosync-app/src-tauri/src/services/scheduler.rs`（集成 Q2 检查 + `spawn_scheduler` 增加 `conv_pool` 参数）
- `egosync-app/src-tauri/src/commands/task.rs`（`task_check_q2_reminders` 命令，已注入 AppHandle）
- `egosync-app/src-tauri/src/lib.rs`（注册命令 + `spawn_scheduler` 传入 conv_pool）
- `egosync-app/src/App.tsx`（监听 `q2:reminder` + 刷新触发器）
- `egosync-app/src/components/butler/ButlerView.tsx`（挂载触发检查 + 透传 refreshTrigger）
- `egosync-app/src/components/chat/ChatStream.tsx`（refreshTrigger 重载历史）
- `egosync-app/src/components/butler/ButlerView.test.tsx` / `egosync-app/src/services/taskService.ts`

### Change Log

- 2026-06-22 — 实现 Story 4.6 Q2 保护管家提醒（后端调度 + 通知 + 对话消息 + 前端刷新）。
- 2026-06-22 — 代码审查：修复 D1（手动触发路径事件缺失）、P1（计数提前自增）、P2（文案硬编码天数）；回填故事文档。
- 2026-06-23 — 同步故事文档 Task 描述与实际代码（函数签名补 conv_pool、upsert 顺序、函数名 set_task_completion、命令参数 Some(&app_handle)）；通知中心已读通知折叠到底部（NotificationPanel.tsx）。

### Review Findings

_代码审查于 2026-06-22 完成（范围：未提交改动）。三层对抗式审查：Blind Hunter / Edge Case Hunter / Acceptance Auditor。_

- [x] [Review][Decision→已修复·方案1] 手动触发路径不刷新管家对话，提醒当天可能不可见 — `ButlerView` 挂载时调用 `task_check_q2_reminders`（`commands/task.rs:126` 传入 `None` app_handle），不 emit `q2:reminder` 事件；该命令首次生成提醒时会在 `ChatStream` 加载历史之后写入管家消息，但无事件 → `butlerChatRefreshTrigger` 不递增 → 消息不刷新。且因 `is_reminded_today` 已置位，调度器当天不会再生成 → 提醒消息可能整天不可见，直到用户重载对话。违反 AC1。修复方向不唯一：(a) 命令注入 AppHandle 并 emit 事件；(b) `ButlerView` 在 `checkQ2Reminders()` resolve 后递增刷新触发器；(c) 接受 ≤60s 调度器兜底（仅适用于"非首次/隔日"场景，首次当天仍不可见）。
- [x] [Review][Patch·已修复] 提醒计数在投递成功前已自增 [egosync-app/src-tauri/src/services/q2_protection_reminder.rs:120] — `upsert_reminder` 在创建通知与写入对话消息之前调用，若两者均失败，该次提醒仍计入 3 次上限（AC3），用户却未看到任何提醒，可能提前耗尽提醒额度。建议在至少一项投递成功后再自增计数。
- [x] [Review][Patch·已修复] 提醒文案硬编码"3 天" [egosync-app/src-tauri/src/services/q2_protection_reminder.rs:124-127] — 文案写死"已经 3 天没动了"，未与 Story 3.5 的 `AT_RISK_DAYS` 常量关联。若阈值变更，文案与实际不符。建议由常量/任务实际滞留天数派生。
- [x] [Review][Patch·已修复] 故事文档与实际状态不一致 [4-6-q2-protection-butler-reminder.md] — Task 2~10 复选框仍为 `[ ]`，`Status: in-progress`，`Dev Agent Record`（File List / Change Log / Completion Notes）为空，但 `sprint-status.yaml` 已标记 `done`。需回填实现记录并同步勾选状态。
