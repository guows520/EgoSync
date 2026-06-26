---
baseline_commit: d7c489b
---

# Story 6.3: 每周初管家检测并补充引导大石头规划

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 如果我在周复盘中没有规划大石头系统会提醒我补上,
So that 每周都有明确的优先级锚点。

## 背景与现状（务必先读）

**本 story 是 Epic 6 的第三个 story — 在 Story 6.1 已建成的晨间简报生成服务（`briefing_generator.rs`）、调度器简报触发逻辑和 Story 6.2 已建成的时间配置 UI（`settings_get_schedule` / `settings_update_schedule` command + `scheduleService.ts` + `ButlerSettingsContent.tsx` 节奏化配置）基础上，实现大石头规划的"补触发"检测逻辑。当用户在周复盘中跳过规划阶段（或完全未打开周复盘），调度器在配置的提醒时间检测本周是否有大石头任务，若无则发送"轻触"通知引导用户规划。**

**重要依赖说明：本 story 的主入口是 WeeklyReviewModal 的规划阶段（`phase='plan'`），但 WeeklyReviewModal 的数据接通和真实规划功能是 Story 6.5 的职责。本 story 只负责"检测 + 通知 + 打开 Modal"这条链路，不涉及 WeeklyReviewModal 内部规划功能的实现。前端打开 WeeklyReviewModal 时传入 `initialPhase='plan'` 即可，Modal 内部仍使用现有 mock 数据。**

### 已建成的基础（本 story 的接入点）

**Story 6.2 已完成的设施：**
- `src-tauri/src/commands/settings.rs:8-17` — 常量 `DEFAULT_BIGROCK_REMINDER_DAY = "1"`（周一）、`DEFAULT_BIGROCK_REMINDER_TIME = "09:00"`、`KEY_BIGROCK_REMINDER_DAY`、`KEY_BIGROCK_REMINDER_TIME`
- `src-tauri/src/commands/settings.rs:57-64` — `validate_bigrock_reminder_day` 函数，校验值仅 `"1"` 或 `"2"`
- `src-tauri/src/commands/settings.rs:92-141` — `settings_get_schedule` / `settings_update_schedule` commands 已注册，前端已接通
- `src-tauri/src/services/scheduler.rs:498-511` — `get_bigrock_reminder_schedule(pool)` 函数已实现，从 `app_settings` 读取 `bigrock_reminder_day` + `bigrock_reminder_time`，返回 `(day, time)` 元组
- `src-tauri/src/services/scheduler.rs:340-343` — `spawn_scheduler` 启动时已调用 `get_bigrock_reminder_schedule` 并打日志（预留读取入口）
- `src/services/scheduleService.ts` — 前端 `scheduleService.getSchedule()` / `scheduleService.updateSchedule(input)` 已实现
- `src/components/butler/ButlerSettingsContent.tsx` — 大石头规划提醒时间配置 UI 已接通真实数据（星期选择 + 时间选择）

**调度器现有结构（`scheduler.rs`）：**
- `src-tauri/src/services/scheduler.rs:322` — `spawn_scheduler(pool, conv_pool, app_handle)` 函数
- `src-tauri/src/services/scheduler.rs:345-482` — tick 循环结构：查询角色 → 角色工作循环 → 简报触发检查 → Q2 提醒检查 → `interval.tick().await`
- `src-tauri/src/services/scheduler.rs:331-332` — `last_briefing_trigger_date: Option<String>` 去重模式（每天只触发一次），**本 story 需参照此模式实现大石头提醒每周去重**
- `src-tauri/src/services/scheduler.rs:431-467` — 简报触发检查已实现，每次 tick 动态读取 `briefing_time` 配置，匹配 `current_hhmm` 时 spawn 异步生成

**任务 DB 现有设施：**
- `src-tauri/src/db/tasks.rs:60-85` — `list_all_tasks(pool, quadrant, is_big_rock)` 函数，可传 `is_big_rock=Some(true)` 查询所有大石头任务（含跨角色 + butler 任务）
- `src-tauri/src/db/tasks.rs:315-334` — `count_big_rocks_by_owner(pool, owner_type, role_id)` 函数，按 owner 统计大石头数量
- `src-tauri/migrations/005_tasks.sql` — `tasks` 表含 `is_big_rock INTEGER NOT NULL DEFAULT 0` 列

**通知服务现有设施：**
- `src-tauri/src/services/notification_service.rs:57-102` — `create_notification_for_role(pool, role_id, requested_level, content)` 函数，含两层降级逻辑
- `src-tauri/src/services/suggestion_generator.rs:355-361` — `NotificationLevel` 枚举（`Whisper` < `Tap` < `Knock`）
- `src-tauri/src/models/notification.rs` — `Notification` / `CreateNotificationInput` / `NotificationNewPayload` 结构体

**前端 WeeklyReviewModal 现状：**
- `src/components/modals/WeeklyReviewModal.tsx:6-7` — 组件签名 `WeeklyReviewModal({ roles, onClose })`，内部 `phase` state 默认 `'review'`
- `src/App.tsx:39` — `isReviewOpen` state 控制 Modal 显示
- `src/App.tsx:357` — `<WeeklyReviewModal roles={roles} onClose={() => setIsReviewOpen(false)} />`

**Q2 提醒服务（参照模式）：**
- `src-tauri/src/services/q2_protection_reminder.rs` — 完整的"检测 + 创建通知 + 写入管家对话 + emit Tauri Event"模式，本 story 的大石头提醒服务应参照此模式实现

### 本 story 需要做的事

**核心变更：实现大石头规划的"补触发"检测 + 通知 + 前端打开 Modal 链路。**

1. **Rust 后端**：新增 `bigrock_reminder.rs` 服务，检测本周是否有 `is_big_rock=true` 的任务，若无则创建"轻触"通知 + 写入管家对话 + emit `bigrock:reminder` 事件
2. **Rust 调度器**：在 tick 循环中新增大石头提醒触发检查，读取 `bigrock_reminder_day/time` 配置，匹配星期 + 时间时触发检测，每周只触发一次（去重）
3. **前端**：监听 `bigrock:reminder` 事件，打开 WeeklyReviewModal 并传入 `initialPhase='plan'`
4. **前端 WeeklyReviewModal**：新增 `initialPhase` prop，允许外部指定初始阶段

## Acceptance Criteria

1. **AC1**: Given 大石头规划的主入口是 WeeklyReviewModal 的规划阶段（Story 6.5 phase='plan'），Then 用户在周复盘中完成规划后，本周大石头已设定 → 无需额外触发

2. **AC2**: Given 用户在周复盘中跳过了规划阶段（关闭 Modal 未进入 plan 阶段），When 到达 `app_settings.bigrock_reminder_day` + `bigrock_reminder_time`（默认周一 09:00），Then 调度器检测本周是否有 `is_big_rock=true` 的任务

3. **AC3**: Given 检测结果为无大石头，When 补触发，Then 管家发送"轻触"通知"还没规划本周大石头，要安排一下吗？"，And 通知中包含"开始规划"按钮

4. **AC4**: Given 用户点击"开始规划"，When 操作，Then 直接打开 WeeklyReviewModal 的规划阶段（phase='plan'）

5. **AC5**: Given 用户完全未打开过周复盘，Then 同样在补触发时间检测并提醒

6. **AC6**: Given 本周已有大石头（无论来源：周复盘规划 / 手动标记 / 上周延续），Then 不触发补提醒

7. **AC7**: Given Rust 后端，Then 调度器增加大石头检测步骤：查询本周 `tasks` where `is_big_rock = true`，And 检测时间从 `app_settings.bigrock_reminder_day/time` 读取

## Tasks / Subtasks

- [ ] **Task 1: Rust — 新增 `bigrock_reminder.rs` 服务** (AC: #2, #3, #6, #7)
  - [ ] 1.0 新建 `src-tauri/src/services/bigrock_reminder.rs`，在 `src-tauri/src/services/mod.rs` 中添加 `pub mod bigrock_reminder;`
  - [ ] 1.1 定义 `BigrockReminderPayload` struct（`serde::Serialize` + `rename_all = "camelCase"`），字段：`message: String`、`notification_id: String`
  - [ ] 1.2 定义 `BIGROCK_REMINDER_EVENT: &str = "bigrock:reminder"` 常量
  - [ ] 1.3 实现核心函数 `check_and_remind_if_needed(pool, conv_pool, app_handle) -> Result<bool, AppError>`：
    - 调用 `db::tasks::list_all_tasks(pool, None, Some(true))` 查询所有大石头任务（`list_all_tasks` 已过滤 `deleted_at IS NULL`）
    - 在 Rust 侧过滤掉 `is_completed = true` 的任务，得到当前未完成的大石头列表
    - **AC6 关键逻辑**：若有任何未完成的大石头任务（无论来源：周复盘规划 / 手动标记 / 上周延续）→ 返回 `Ok(false)`（不触发）
    - 若未完成的大石头列表为空 → 创建"轻触"通知 + 写入管家对话消息 + emit `bigrock:reminder` 事件，返回 `Ok(true)`
  - [ ] 1.4 通知创建：需要一个 `role_id` 来调用 `create_notification_for_role`。由于大石头提醒是管家级别的全局提醒（不针对特定角色），**使用第一个活跃角色的 ID** 创建通知。若没有活跃角色则跳过通知创建，只写入管家对话消息
  - [ ] 1.5 通知文案：`"还没规划本周大石头，要安排一下吗？"`
  - [ ] 1.6 管家对话消息：复用 `db::conversations::get_or_create_butler_conversation` + `insert_message`，消息内容与通知文案一致
  - [ ] 1.7 emit `bigrock:reminder` 事件，payload 含 `message` 和 `notification_id`
  - [ ] 1.8 错误处理参照 `q2_protection_reminder.rs`：错误只 `tracing::warn!`，不 panic，不阻塞调度器

- [ ] **Task 2: Rust — 调度器 tick 循环新增大石头提醒触发检查** (AC: #2, #5, #7)
  - [ ] 2.1 在 `spawn_scheduler` 函数的 tick 循环中，在 Q2 提醒检查之后、`interval.tick().await` 之前，新增大石头提醒触发检查
  - [ ] 2.2 新增去重 state：`last_bigrock_trigger_week: Option<String>`（格式 `"YYYY-Www"`，如 `"2026-W23"`），确保每周只触发一次
  - [ ] 2.3 读取 `get_bigrock_reminder_schedule(&pool)` 获取配置的星期 + 时间
  - [ ] 2.4 检查条件：当前本地时间的星期（`chrono::Local::now().weekday().num_days_from_monday() + 1`，返回 1=周一 ~ 7=周日，与 `app_settings` 中的 `bigrock_reminder_day` 编码一致）等于配置的 `day`，且当前 `HH:MM` 等于配置的 `time`，且本周尚未触发
  - [ ] 2.5 条件满足时 spawn 异步调用 `bigrock_reminder::check_and_remind_if_needed`，错误只 warn
  - [ ] 2.6 触发后更新 `last_bigrock_trigger_week` 去重 state

- [ ] **Task 3: 前端 — 监听 `bigrock:reminder` 事件并打开 WeeklyReviewModal** (AC: #3, #4)
  - [ ] 3.1 在 `src/types/` 中新增 `bigrockReminder.ts` 类型定义：`BigrockReminderPayload { message: string; notificationId: string }`
  - [ ] 3.2 在 `src/App.tsx` 中新增 `useTauriEvent<BigrockReminderPayload>('bigrock:reminder', ...)` 监听
  - [ ] 3.3 事件回调中：`setIsReviewOpen(true)` + 设置一个 `reviewInitialPhase` state 为 `'plan'`
  - [ ] 3.4 新增 `reviewInitialPhase` state（`'review' | 'plan'`，默认 `'review'`），传给 `WeeklyReviewModal` 的 `initialPhase` prop
  - [ ] 3.5 `WeeklyReviewModal` 关闭时重置 `reviewInitialPhase` 为 `'review'`

- [ ] **Task 4: 前端 — WeeklyReviewModal 新增 `initialPhase` prop** (AC: #4)
  - [ ] 4.1 修改 `WeeklyReviewModal` 组件签名：`WeeklyReviewModal({ roles, onClose, initialPhase = 'review' })`
  - [ ] 4.2 `phase` state 初始值从 `initialPhase` prop 传入：`useState<'review' | 'plan'>(initialPhase)`
  - [ ] 4.3 **不修改 Modal 内部任何其他逻辑** — 规划功能的数据接通是 Story 6.5 的职责

- [ ] **Task 5: 前端 — 通知中"开始规划"按钮交互** (AC: #3, #4)
  - [ ] 5.1 由于现有通知系统（`NotificationPanel.tsx`）的通知卡片不支持 action button，本 story 采用事件驱动方案：`bigrock:reminder` 事件到达时直接弹出 WeeklyReviewModal（无需用户先点击通知再操作）
  - [ ] 5.2 通知作为"留痕"存在通知中心，用户后续可在通知中心看到提醒文案
  - [ ] 5.3 **不修改 `NotificationPanel.tsx`** — 通知卡片的 action button 增强是后续 story 的职责，本 story 通过事件直接打开 Modal 实现 AC4

- [ ] **Task 6: 单元测试** (AC: #2, #3, #6, #7)
  - [ ] 6.1 Rust 测试 — 在 `bigrock_reminder.rs` 中测试：
    - `check_and_remind_if_needed` 无大石头时创建通知 + 写入对话 + 返回 `true`
    - `check_and_remind_if_needed` 有大石头时跳过 + 返回 `false`
    - 无活跃角色时跳过通知创建，只写入对话消息
  - [ ] 6.2 Rust 测试 — 在 `scheduler.rs` 中测试星期匹配逻辑（mock 当前时间为周一 09:00，验证触发条件判断正确）
  - [ ] 6.3 前端测试 — 验证 `bigrock:reminder` 事件到达时 `isReviewOpen` 变为 `true` 且 `reviewInitialPhase` 变为 `'plan'`
  - [ ] 6.4 前端测试 — 验证 `WeeklyReviewModal` 接收 `initialPhase='plan'` 时初始显示规划阶段

## Dev Notes

### 关键技术决策

- **大石头检测逻辑（AC6 核心解读）**：AC6 明确"本周已有大石头（无论来源：周复盘规划 / 手动标记 / 上周延续）"— 这意味着检测的是**所有未完成的大石头任务**，而非仅"本周创建"的。检测逻辑：调用 `list_all_tasks(pool, None, Some(true))` 查询所有大石头任务 → 在 Rust 侧过滤掉 `is_completed = true` → 若列表非空则不触发。`list_all_tasks` 已过滤 `deleted_at IS NULL`，无需重复过滤。

- **大石头检测查询策略**：复用 `db::tasks::list_all_tasks(pool, None, Some(true))` 查询所有大石头任务，在 Rust 侧过滤 `is_completed = false` 的任务。若结果为空 → 无大石头 → 触发提醒。**不新增 DB 函数**，复用现有 `list_all_tasks`。

- **通知创建的 role_id 问题**：`create_notification_for_role` 需要 `role_id` 参数。大石头提醒是全局管家提醒，不针对特定角色。方案：使用第一个活跃角色的 ID 创建通知。若没有活跃角色（极端边界），跳过通知创建，只写入管家对话消息（参照 `q2_protection_reminder.rs:158-165` 的 butler 任务无 role_id 处理模式）。

- **去重策略**：使用 ISO 周编号 `"YYYY-Www"` 作为去重键（如 `"2026-W23"`），确保每周只触发一次。使用 `now_local.date_naive().iso_week()` 获取 `IsoWeek`，再 `format!("{}-W{:02}", iso_week.year(), iso_week.week())` 生成去重键。**不使用日期字符串**，因为配置的触发日可能是周一或周二，用日期字符串无法区分"本周已触发"。

- **事件驱动 vs 通知 action button**：AC3 提到"通知中包含'开始规划'按钮"，AC4 提到"用户点击'开始规划'→ 直接打开 WeeklyReviewModal"。当前通知系统（`NotificationPanel.tsx`）的通知卡片不支持 action button。本 story 采用**事件驱动方案**：`bigrock:reminder` 事件到达前端时直接打开 WeeklyReviewModal（phase='plan'），无需用户先看到通知再点击按钮。通知作为"留痕"存在通知中心。这样既满足 AC4（用户可以直接开始规划），又避免修改通知卡片组件（保持外科手术式修改）。

- **WeeklyReviewModal 的 `initialPhase` prop**：新增可选 prop `initialPhase`，默认 `'review'`。当 `bigrock:reminder` 事件触发时传入 `'plan'`，使 Modal 直接显示规划阶段。**不修改 Modal 内部任何其他逻辑**。

### 架构合规

- **分层规则**：调度器 tick → `bigrock_reminder::check_and_remind_if_needed` → `db::tasks::list_all_tasks` + `notification_service::create_notification_for_role` + `db::conversations::insert_message` + `app_handle.emit("bigrock:reminder")`
- **服务层职责**：`bigrock_reminder.rs` 负责"检测 + 通知 + 对话消息 + 事件"，参照 `q2_protection_reminder.rs` 模式
- **调度器职责**：tick 循环中读取配置时间，匹配星期 + HH:MM 时 spawn 异步调用 `bigrock_reminder`，参照简报触发模式
- **命名规范**：Rust 文件 `bigrock_reminder.rs`（snake_case），Tauri Event `"bigrock:reminder"`（kebab-case + 冒号分隔），前端类型 `BigrockReminderPayload`（PascalCase），前端 state `reviewInitialPhase`（camelCase）
- **错误处理**：所有错误只 `tracing::warn!`，不 panic，不阻塞调度器 tick 循环
- **serde 桥接**：`BigrockReminderPayload` 派生 `Serialize + Deserialize` + `#[serde(rename_all = "camelCase")]`

### 前端 UI 规范

- **不新增组件文件**：在 `App.tsx` 中新增事件监听 + state，在 `WeeklyReviewModal.tsx` 中新增 `initialPhase` prop
- **不修改 `NotificationPanel.tsx`**：通知卡片保持原样，"开始规划"交互通过事件直接打开 Modal 实现
- **事件监听模式**：参照现有 `useTauriEvent<BriefingGeneratedPayload>('briefing:generated', ...)` 模式

### 反模式警告

- **不要**新增 DB 迁移 — 大石头检测复用现有 `list_all_tasks` 函数
- **不要**修改 `WeeklyReviewModal` 内部规划功能 — 那是 Story 6.5 的职责，本 story 只新增 `initialPhase` prop
- **不要**修改 `NotificationPanel.tsx` — 通知 action button 增强是后续 story 的职责
- **不要**在 `commands/` 层添加业务逻辑 — 大石头提醒是服务层职责，不需要新增 Tauri command
- **不要**将去重键用日期字符串 — 用 ISO 周编号 `"YYYY-Www"` 确保每周只触发一次
- **不要**在 tick 循环中同步调用 `check_and_remind_if_needed` — 必须 `tokio::spawn` 异步调用，避免阻塞 tick 循环（参照简报触发模式 `scheduler.rs:447-466`）
- **不要**修改 `get_bigrock_reminder_schedule` 函数 — Story 6.2 已实现，直接复用
- **不要**修改 `settings_get_schedule` / `settings_update_schedule` commands — Story 6.2 已实现，直接复用
- **不要**过滤已完成的大石头任务时遗漏 `is_completed` 检查 — `list_all_tasks` 返回含已完成任务，需在 Rust 侧过滤 `is_completed = false`
- **不要**只查询"本周创建"的大石头任务 — AC6 明确"无论来源"，包括上周延续的未完成大石头，应查询所有未完成的大石头任务
- **不要**使用 `weekday().num()` — chrono 的 `Weekday` 没有 `num()` 方法，正确 API 是 `weekday().num_days_from_monday() + 1`（返回 1=周一 ~ 7=周日）

### Project Structure Notes

新增文件：
- `src-tauri/src/services/bigrock_reminder.rs` — 大石头提醒检测服务
- `src/types/bigrockReminder.ts` — 前端事件 payload 类型定义

修改文件：
- `src-tauri/src/services/mod.rs` — 添加 `pub mod bigrock_reminder;`
- `src-tauri/src/services/scheduler.rs` — tick 循环新增大石头提醒触发检查 + `last_bigrock_trigger_week` 去重 state
- `src/App.tsx` — 新增 `bigrock:reminder` 事件监听 + `reviewInitialPhase` state + 传给 `WeeklyReviewModal`
- `src/components/modals/WeeklyReviewModal.tsx` — 新增 `initialPhase` prop

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.3] — AC 原文
- [Source: _bmad-output/planning-artifacts/architecture.md#FR-17] — 大石头周规划功能需求
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: _bmad-output/implementation-artifacts/6-1-daily-morning-briefing.md] — Story 6.1 实现上下文（简报生成模式）
- [Source: _bmad-output/implementation-artifacts/6-2-schedule-config-briefing-review.md] — Story 6.2 实现上下文（时间配置 + 调度器预留读取入口）
- [Source: src-tauri/src/services/scheduler.rs:322-484] — `spawn_scheduler` tick 循环结构
- [Source: src-tauri/src/services/scheduler.rs:331-332] — `last_briefing_trigger_date` 去重模式（参照实现每周去重）
- [Source: src-tauri/src/services/scheduler.rs:431-467] — 简报触发检查（参照实现大石头触发检查）
- [Source: src-tauri/src/services/scheduler.rs:498-511] — `get_bigrock_reminder_schedule` 函数（已实现，直接复用）
- [Source: src-tauri/src/services/q2_protection_reminder.rs:65-236] — Q2 提醒服务（参照实现大石头提醒服务）
- [Source: src-tauri/src/services/notification_service.rs:57-102] — `create_notification_for_role` 函数
- [Source: src-tauri/src/services/suggestion_generator.rs:355-361] — `NotificationLevel` 枚举
- [Source: src-tauri/src/db/tasks.rs:60-85] — `list_all_tasks` 函数（复用查询大石头任务）
- [Source: src-tauri/src/db/tasks.rs:315-334] — `count_big_rocks_by_owner` 函数
- [Source: src-tauri/src/commands/settings.rs:8-17] — 大石头提醒时间配置常量
- [Source: src-tauri/src/commands/settings.rs:57-64] — `validate_bigrock_reminder_day` 函数
- [Source: src-tauri/src/lib.rs:366-367] — `settings_get_schedule` / `settings_update_schedule` command 注册位置
- [Source: src/App.tsx:39] — `isReviewOpen` state
- [Source: src/App.tsx:94-100] — `briefing:generated` 事件监听模式（参照实现 `bigrock:reminder` 事件监听）
- [Source: src/App.tsx:357] — `WeeklyReviewModal` 渲染位置
- [Source: src/components/modals/WeeklyReviewModal.tsx:6-7] — 组件签名 + `phase` state
- [Source: src/components/notifications/NotificationPanel.tsx] — 通知面板（不修改）
- [Source: src/hooks/useTauriEvent.ts] — Tauri 事件监听 hook

## Review Findings (2026-06-26)

- [x] [Review][Decision] AC3/AC4「通知含『开始规划』按钮 + 用户点击打开」被「事件到达自动弹出 Modal」替代，侵入性超出「轻触」语义 — 实现未做通知 action button，而是 `bigrock:reminder` 事件到达前端时直接 `setIsReviewOpen(true)`（App.tsx:105-113）。Dev Notes 已说明此为有意取舍（通知卡片不支持 action button）。**裁决（2026-06-26）：接受现状**，认可自动弹出周复盘规划页的取舍。
- [x] [Review][Decision] AC6「本周已有大石头（无论来源）则不提醒」存在漏检 — `check_and_remind_if_needed` 调用 `db::tasks::list_all_tasks`，该查询含 `(t.owner_type = 'butler' OR r.status = 'active')` 过滤（tasks.rs:74）。归档/非活跃角色名下的未完成大石头不会被计入。**裁决（2026-06-26）：接受现状**，归档角色的大石头不计入检测属合理行为。[bigrock_reminder.rs:46]
- [x] [Review][Defer] 错过精确触发分钟则当周不再提醒 — 触发条件为 `current_hhmm == bigrock_time` 精确匹配，若 App 在配置分钟未运行（关闭/休眠/tick 错过该分钟），本周不会补提醒。此为轮询调度器固有限制，且与 Story 6.1 简报触发（scheduler.rs:445）同模式，非本次改动引入。[scheduler.rs:492-493] — deferred, pre-existing

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
