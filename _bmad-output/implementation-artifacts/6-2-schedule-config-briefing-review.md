---
baseline_commit: 1d51afb
---

# Story 6.2: 用户能配置晨间简报、周复盘和大石头规划的触发时间

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 自定义各节奏化功能的触发时间,
So that 适配我的作息和生活习惯。

## 背景与现状（务必先读）

**本 story 是 Epic 6 的第二个 story — 在 Story 6.1 已建成的晨间简报生成服务（`briefing_generator.rs`）、`briefings` 表、调度器简报触发逻辑和前端 `ButlerSettingsContent.tsx` 中已有的晨间简报时间配置 UI 基础上，补全周复盘和大石头规划的时间配置 UI，并将三项配置统一通过 `app_settings` 持久化。本 story 不涉及周复盘生成逻辑（Story 6.4）、WeeklyReviewModal 数据接通（Story 6.5）和大石头检测/补触发逻辑（Story 6.3），只负责"时间配置 UI 接通真实数据 + 后端统一读写 + 调度器预留读取入口"。**

### 已建成的基础（本 story 的接入点）

**Story 6.1 已完成的设施：**
- `src-tauri/src/services/briefing_generator.rs:27-29` — 常量 `BRIEFING_TIME_KEY = "briefing_time"`、`DEFAULT_BRIEFING_TIME = "08:00"`
- `src-tauri/src/services/briefing_generator.rs:153-159` — `get_briefing_time(pool)` 函数，从 `app_settings` 读取 `briefing_time`，无则返回默认值
- `src-tauri/src/services/scheduler.rs:420-455` — 调度器 tick 循环中已实现简报触发检查，每次 tick 调用 `briefing_generator::get_briefing_time(&pool)` 读取配置时间，匹配 `current_hhmm` 时触发
- `src-tauri/migrations/023_briefings.sql` — `briefings` 表已创建
- `src-tauri/src/db/briefings.rs` — 简报 DB 操作已实现
- `src-tauri/src/commands/briefing.rs` — `briefing_get_latest` / `briefing_generate_now` commands 已注册

**前端已建成的设施：**
- `src/components/butler/ButlerSettingsContent.tsx:72` — `briefingTime` state，默认 `'08:00'`
- `src/components/butler/ButlerSettingsContent.tsx:153-157` — `useEffect` 加载 `appService.getSetting('briefing_time')`，有则 `setBriefingTime`
- `src/components/butler/ButlerSettingsContent.tsx:877-898` — **晨间简报时间 UI 已接通真实数据**：`<input type="time">` 绑定 `briefingTime` state，`onChange` 调用 `appService.setSetting('briefing_time', value)` 保存
- `src/components/butler/ButlerSettingsContent.tsx:899-910` — **周复盘时间 UI 是 mock**：`<select defaultValue="7">` + `<input type="time" defaultValue="20:00">`，**未接通真实数据，无 state，无 onChange**
- `src/services/appService.ts:10-11` — `getSetting(key)` / `setSetting(key, value)` 通用 app_settings 读写封装，**本 story 直接复用**

**后端通用设施：**
- `src-tauri/src/db/app_settings.rs:9-31` — `get_setting(pool, key)` / `set_setting(pool, key, value)` 通用读写函数
- `src-tauri/src/commands/app.rs:65-79` — `app_get_setting` / `app_set_setting` Tauri commands 已注册
- `src-tauri/src/lib.rs:348-349` — 两个 command 已在 `generate_handler!` 中注册

**调度器现有结构（`scheduler.rs`）：**
- `src-tauri/src/services/scheduler.rs:322` — `spawn_scheduler(pool, conv_pool, app_handle)` 函数
- `src-tauri/src/services/scheduler.rs:334-470` — tick 循环结构：查询角色 → 角色工作循环 → 简报触发检查 → Q2 提醒检查 → `interval.tick().await`
- `src-tauri/src/services/scheduler.rs:420-455` — 简报触发检查已实现，每次 tick 动态读取 `briefing_time` 配置

### 本 story 需要做的事

**核心变更：将三项节奏化时间配置（晨间简报、周复盘、大石头规划）全部接通真实数据持久化。**

1. **前端**：`ButlerSettingsContent.tsx` 中周复盘时间 UI 从 mock 改为 state 驱动 + 接通 `appService.getSetting/setSetting`；新增大石头规划时间配置 UI（星期选择 + 时间选择）
2. **后端**：新增 `settings::update_schedule` Tauri command（统一写入五项配置）；新增 `settings::get_schedule` command（统一读取）；调度器预留周复盘/大石头触发检查的读取入口（实际触发逻辑在 Story 6.3/6.4 实现）
3. **DB**：无需新增表或迁移 — 所有配置通过 `app_settings` 表 key-value 存储

## Acceptance Criteria

1. **AC1**: Given 用户在 ButlerView SettingsTab 配置晨间简报时间，When 修改时间（如 07:30）并保存，Then `app_settings.briefing_time` 更新，And 调度器下次按新时间触发（Story 6.1 已实现此功能，本 story 验证不回归）

2. **AC2**: Given 用户配置周复盘触发时间，When 修改星期和时间（如周日 20:00）并保存，Then `app_settings.review_day` + `app_settings.review_time` 更新

3. **AC3**: Given 用户配置大石头规划补触发时间，When 修改星期和时间（如周一 09:00）并保存，Then `app_settings.bigrock_reminder_day` + `app_settings.bigrock_reminder_time` 更新，And 可选星期：周一 / 周二

4. **AC4**: Given 默认值，Then 晨间简报：08:00 / 周复盘：周日 20:00 / 大石头补触发：周一 09:00

5. **AC5**: Given 前端，Then ButlerSettingsContent 时间配置接通真实数据（替换原型 mock `defaultValue`），And 大石头补触发时间配置新增 UI（星期选择 + 时间选择），And 页面加载时从 `app_settings` 读取已保存的值并填充表单

6. **AC6**: Given Rust 后端，Then Tauri commands: `settings::get_schedule` 返回 `{ briefingTime, reviewDay, reviewTime, bigrockReminderDay, bigrockReminderTime }`，And `settings::update_schedule` 接受部分更新（只更新传入的字段，未传入的保持不变）

7. **AC7**: Given 调度器，Then `spawn_scheduler` 启动时从 `app_settings` 读取所有时间配置（ briefing_time 已实现，review_day/review_time/bigrock_reminder_day/bigrock_reminder_time 为预留读取 — 实际触发逻辑在 Story 6.3/6.4 实现）

## Tasks / Subtasks

- [x] **Task 1: Rust — 新增 `settings::get_schedule` / `update_schedule` commands** (AC: #2, #3, #6)
  - [x] 1.0 新建 `src-tauri/src/commands/settings.rs` 文件，在 `src-tauri/src/commands/mod.rs` 中添加 `pub mod settings;`
  - [x] 1.1 在 `src-tauri/src/commands/settings.rs` 中新增两个 command
  - [x] 1.2 默认值常量
  - [x] 1.3 `review_day` 合法值校验：`"1"` ~ `"7"`（周一~周日）
  - [x] 1.4 `bigrock_reminder_day` 合法值校验：仅 `"1"`（周一）或 `"2"`（周二）
  - [x] 1.5 时间格式校验：`HH:MM` 正则 `^\d{2}:\d{2}$`，且 `00:00` ~ `23:59` 范围合法
  - [x] 1.6 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册 `commands::settings::settings_get_schedule` 和 `commands::settings::settings_update_schedule`（插入在 `commands::briefing::*` 之后）

- [x] **Task 2: 前端 — 新增 `scheduleService.ts`** (AC: #5, #6)
  - [x] 2.1 新建 `src/services/scheduleService.ts`
  - [x] 2.2 `Partial<ScheduleConfig>` 对应后端 `Option<String>` — 只传需要更新的字段

- [x] **Task 3: 前端 — `ButlerSettingsContent.tsx` 周复盘时间 UI 接通真实数据** (AC: #2, #5)
  - [x] 3.1 新增 state：`reviewDay`（默认 `'7'`）、`reviewTime`（默认 `'20:00'`）
  - [x] 3.2 新增 `useEffect` 加载：页面初始化时调用 `scheduleService.getSchedule()` 读取全部配置，填充 `briefingTime` / `reviewDay` / `reviewTime` / `bigrockReminderDay` / `bigrockReminderTime`（替换现有 `appService.getSetting('briefing_time')` 的单独加载，统一用 `getSchedule` 一次性加载）
  - [x] 3.3 修改周复盘 UI（`ButlerSettingsContent.tsx:899-910`）：
    - `<select>` 绑定 `reviewDay` state（替换 `defaultValue="7"`）
    - `<input type="time">` 绑定 `reviewTime` state（替换 `defaultValue="20:00"`）
    - 两者 `onChange` 调用 `scheduleService.updateSchedule({ reviewDay: value })` 或 `scheduleService.updateSchedule({ reviewTime: value })` 保存
    - 保存成功显示 `setSettingsSavedMessage('周复盘时间已保存')`
    - 保存失败显示 `setError('周复盘时间保存失败，请稍后重试')`

- [x] **Task 4: 前端 — 新增大石头规划时间配置 UI** (AC: #3, #5)
  - [x] 4.1 新增 state：`bigrockReminderDay`（默认 `'1'`）、`bigrockReminderTime`（默认 `'09:00'`）
  - [x] 4.2 在周复盘时间 UI 之后新增大石头规划时间配置区块
  - [x] 4.3 `onChange` 调用 `scheduleService.updateSchedule({ bigrockReminderDay: value })` 或 `scheduleService.updateSchedule({ bigrockReminderTime: value })` 保存
  - [x] 4.4 保存成功/失败反馈与周复盘一致

- [x] **Task 5: 前端 — 晨间简报时间 UI 迁移到统一加载** (AC: #1, #5)
  - [x] 5.1 删除 `ButlerSettingsContent.tsx:153-157` 的单独 `useEffect`（`appService.getSetting('briefing_time')`）
  - [x] 5.2 晨间简报时间 `onChange` 从 `appService.setSetting('briefing_time', value)` 改为 `scheduleService.updateSchedule({ briefingTime: value })`
  - [x] 5.3 保留 `briefingTime` state，初始化值由 Task 3.2 的 `getSchedule()` 统一加载填充

- [x] **Task 6: 调度器预留读取入口** (AC: #7)
  - [x] 6.1 在 `src-tauri/src/services/scheduler.rs` 中新增辅助函数 `get_review_schedule` 和 `get_bigrock_reminder_schedule`
  - [x] 6.2 **不在 tick 循环中新增触发逻辑** — 触发逻辑是 Story 6.3/6.4 的职责。本 story 只提供读取入口供后续 story 调用

- [x] **Task 7: 单元测试** (AC: #2, #3, #6)
  - [x] 7.1 Rust command 测试 — 在 `commands/settings.rs` 中测试：
    - `settings_get_schedule` 返回默认值（无配置时）
    - `settings_update_schedule` 部分更新（只传 `review_day`，其他不变）
    - `settings_update_schedule` 全量更新
    - 非法 `review_day`（如 `"8"`）返回 `ValidationError`
    - 非法 `bigrock_reminder_day`（如 `"3"`）返回 `ValidationError`
    - 非法时间格式（如 `"25:00"`）返回 `ValidationError`
  - [x] 7.2 前端 service 测试 — 验证 `scheduleService.getSchedule()` 和 `scheduleService.updateSchedule(input)` 的 invoke 调用参数正确
  - [x] 7.3 前端组件测试 — 验证 `ButlerSettingsContent` 加载时调用 `getSchedule`，周复盘/大石头 UI 响应 state 变化

## Dev Notes

### 关键技术决策

- **统一 command vs 逐个 `app_set_setting`**：epics.md AC 提到 `settings::update_schedule { briefing_time, review_day, review_time, bigrock_reminder_day, bigrock_reminder_time }`，推荐新增统一 command 而非复用 `app_set_setting` 逐个写入。理由：(1) 前端一次调用即可保存；(2) 后端可做跨字段校验；(3) 返回完整的 `ScheduleConfig` 供前端更新 state。但 **前端 onChange 仍按字段逐个调用** `updateSchedule({ fieldName: value })`，因为用户每次只改一个字段（如只改星期不改时间），`Option<String>` 参数支持部分更新

- **`app_settings` key 命名**：
  - `briefing_time` — 已存在（Story 6.1），保持不变
  - `review_day` — 周复盘星期，值 `"1"`~`"7"`（周一~周日）
  - `review_time` — 周复盘时间，值 `HH:MM`
  - `bigrock_reminder_day` — 大石头规划提醒星期，值 `"1"` 或 `"2"`
  - `bigrock_reminder_time` — 大石头规划提醒时间，值 `HH:MM`

- **星期值编码**：`"1"` = 周一，`"2"` = 周二，...，`"7"` = 周日。与前端现有 `<select defaultValue="7">` 的 `<option value="7">周日</option>` 一致，也与 `chrono::Weekday::num()`（Mon=1, Sun=7）一致

- **不新增 DB 迁移**：所有配置通过 `app_settings` 表 key-value 存储，无需新增表或列

- **调度器只预留读取函数，不加触发逻辑**：本 story 的核心是"时间配置 UI + 持久化"，调度器中周复盘和大石头规划的触发检查是 Story 6.3/6.4 的职责。本 story 只提供 `get_review_schedule` / `get_bigrock_reminder_schedule` 读取函数供后续 story 调用

- **前端统一加载**：将现有 `appService.getSetting('briefing_time')` 的单独加载替换为 `scheduleService.getSchedule()` 一次性加载全部五项配置，减少 invoke 调用次数，并保持 state 一致性

- **前端 onChange 保存策略**：每次修改单个字段时调用 `scheduleService.updateSchedule({ fieldName: value })`，与现有晨间简报时间的即时保存模式一致（`ButlerSettingsContent.tsx:882-894`）。不需要"统一保存"按钮

### 架构合规

- **分层规则**：前端 → `scheduleService.getSchedule()` / `scheduleService.updateSchedule(input)` → `invoke('settings_get_schedule')` / `invoke('settings_update_schedule', input)` → `commands/settings.rs`（薄层解析 + 校验）→ `db::app_settings::get_setting` / `set_setting`（SQL 执行）→ SQLite
- **command 层职责**：`settings_get_schedule` 读取五项配置 + 填充默认值 + 返回 `ScheduleConfig`；`settings_update_schedule` 校验输入 + 只更新 `Some` 字段 + 返回完整 `ScheduleConfig`
- **命名规范**：Rust command 使用 `snake_case`（`settings_get_schedule`, `settings_update_schedule`），前端 service 使用 `camelCase`（`scheduleService.getSchedule()`, `scheduleService.updateSchedule()`），serde 自动桥接 `ScheduleConfig` 的 `rename_all = "camelCase"`
- **错误处理**：非法值返回 `AppError::ValidationError(String)`，前端 try-catch 显示友好中文提示
- **serde 桥接**：Rust `ScheduleConfig` 派生 `Serialize + Deserialize` + `#[serde(rename_all = "camelCase")]`，前端 `ScheduleConfig` interface 使用 camelCase

### 前端 UI 规范

- **不新增组件文件**：在 `ButlerSettingsContent.tsx` 现有的晨间简报/周复盘区块内修改和扩展
- **样式一致**：新的大石头规划时间配置区块与周复盘时间区块样式一致（`<select>` + `<input type="time">` 并排，`flex gap-3`，相同的 Tailwind class）
- **反馈模式**：保存成功/失败复用现有 `setSettingsSavedMessage` / `setError` 机制，不使用 toast/snackbar（遵循 UX-DR16）
- **星期选项文案**：周一/周二/.../周日，与现有 `<option>` 标签一致
- **大石头星期限制**：只提供周一和周二两个选项（epics.md AC: "可选星期：周一 / 周二"）

### 反模式警告

- **不要**新增 DB 迁移 — 所有配置通过 `app_settings` key-value 存储
- **不要**在调度器 tick 循环中新增周复盘/大石头触发逻辑 — 那是 Story 6.3/6.4 的职责
- **不要**在 `commands/` 层添加业务逻辑 — `settings_update_schedule` 只做校验 + 调 `app_settings::set_setting` + 返回结果
- **不要**将 `ScheduleConfig` 的默认值硬编码在前端 — 前端 state 初始值可用默认值占位，但最终值由 `getSchedule()` 返回的后端默认值为准
- **不要**移除 `briefing_generator::get_briefing_time` 函数 — 调度器简报触发仍在使用它（`scheduler.rs:424`），本 story 的 `settings_get_schedule` 是前端配置用的统一入口，两者并存
- **不要**修改 `appService.getSetting` / `setSetting` — 保留通用接口，本 story 新增的 `scheduleService` 是专用封装
- **不要**在 `update_schedule` 中强制要求所有字段 — `Option<String>` 支持部分更新，未传入的字段保持不变
- **不要**将星期值编码为 `"0"`~`"6"` 或英文名 — 使用 `"1"`~`"7"` 与前端现有 `<option value>` 和 `chrono::Weekday::num()` 一致

### Project Structure Notes

新增文件：
- `src/services/scheduleService.ts` — 前端节奏化配置 service 层

新增/修改文件：
- `src-tauri/src/commands/settings.rs` — **新建**，`ScheduleConfig` struct + `settings_get_schedule` + `settings_update_schedule` commands
- `src-tauri/src/commands/mod.rs` — 添加 `pub mod settings;`
- `src-tauri/src/lib.rs` — 注册 `settings_get_schedule` 和 `settings_update_schedule`
- `src-tauri/src/services/scheduler.rs` — 新增 `get_review_schedule` / `get_bigrock_reminder_schedule` 辅助函数（预留读取入口）
- `src/components/butler/ButlerSettingsContent.tsx` — 周复盘 UI 接通真实数据 + 新增大石头规划 UI + 晨间简报加载迁移到 `getSchedule()`

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.2] — AC 原文
- [Source: _bmad-output/planning-artifacts/architecture.md#FR-16~18] — 简报复盘 → `briefing.rs` + `app_settings`
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Boundaries] — `app_settings` 表 key-value 存储
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: _bmad-output/implementation-artifacts/6-1-daily-morning-briefing.md] — Story 6.1 完整实现上下文
- [Source: src-tauri/src/services/briefing_generator.rs:27-29] — `BRIEFING_TIME_KEY` / `DEFAULT_BRIEFING_TIME` 常量
- [Source: src-tauri/src/services/briefing_generator.rs:153-159] — `get_briefing_time` 函数（保留不动）
- [Source: src-tauri/src/services/scheduler.rs:322-470] — `spawn_scheduler` tick 循环结构
- [Source: src-tauri/src/services/scheduler.rs:420-455] — 简报触发检查（已实现，不修改）
- [Source: src-tauri/src/db/app_settings.rs:9-31] — `get_setting` / `set_setting` 通用读写
- [Source: src-tauri/src/commands/app.rs:65-79] — `app_get_setting` / `app_set_setting` commands
- [Source: src-tauri/src/commands/scheduler.rs] — `scheduler_get_times` / `scheduler_set_times` 参考（角色调度时间配置模式）
- [Source: src-tauri/src/lib.rs:344-366] — `generate_handler!` command 注册位置
- [Source: src/components/butler/ButlerSettingsContent.tsx:72] — `briefingTime` state
- [Source: src/components/butler/ButlerSettingsContent.tsx:153-157] — 现有 `appService.getSetting('briefing_time')` 加载（将被替换）
- [Source: src/components/butler/ButlerSettingsContent.tsx:877-910] — 晨间简报时间 UI（已接通）+ 周复盘时间 UI（mock，待接通）
- [Source: src/services/appService.ts:10-11] — `getSetting` / `setSetting` 通用封装
- [Source: src/services/schedulerService.ts] — 角色调度 service 参考（模式一致）

## Dev Agent Record

### Agent Model Used

Claude (Amelia / bmad-dev-story)

### Debug Log References

无

### Completion Notes List

- Task 1: 新建 `commands/settings.rs`，实现 `settings_get_schedule` / `settings_update_schedule` 两个 Tauri command，包含 `ScheduleConfig` struct、默认值常量、时间格式校验（HH:MM）、`review_day` 校验（1~7）、`bigrock_reminder_day` 校验（仅 1 或 2）。在 `commands/mod.rs` 注册模块，在 `lib.rs` 注册 command。9 个 Rust 单元测试全部通过。
- Task 2: 新建 `scheduleService.ts`，封装 `getSchedule()` 和 `updateSchedule(input)` invoke 调用，`Partial<ScheduleConfig>` 支持部分更新。6 个 service 测试全部通过。
- Task 3: `ButlerSettingsContent.tsx` 周复盘时间 UI 从 mock `defaultValue` 改为 state 驱动，`<select>` 绑定 `reviewDay`，`<input type="time">` 绑定 `reviewTime`，onChange 调用 `scheduleService.updateSchedule` 保存。
- Task 4: 在周复盘 UI 后新增大石头规划提醒时间配置区块（周一/周二 select + time input），state 驱动 + onChange 即时保存。
- Task 5: 删除原有 `appService.getSetting('briefing_time')` 单独加载 useEffect，替换为 `scheduleService.getSchedule()` 统一加载全部五项配置。晨间简报 onChange 从 `appService.setSetting` 改为 `scheduleService.updateSchedule({ briefingTime: value })`。
- Task 6: 在 `scheduler.rs` 中新增 `get_review_schedule` 和 `get_bigrock_reminder_schedule` 两个 pub async 函数，供 Story 6.3/6.4 调用。不在 tick 循环中新增触发逻辑。
- Task 7: Rust 9 个单元测试 + 前端 6 个 service 测试 + 前端 4 个组件测试（节奏化时间配置）全部通过。Rust 全量测试无回归。前端 23 个组件测试全部通过。
- 预先存在的 TS 错误（`inferenceDismissed` unused、`ActionCard.test.tsx` 类型不匹配）非本次变更引入。

### File List

新增文件：
- `egosync-app/src-tauri/src/commands/settings.rs` — ScheduleConfig struct + settings_get_schedule / settings_update_schedule commands + 单元测试
- `egosync-app/src/services/scheduleService.ts` — 前端节奏化配置 service 层
- `egosync-app/src/services/scheduleService.test.ts` — scheduleService 单元测试

修改文件：
- `egosync-app/src-tauri/src/commands/mod.rs` — 添加 `pub mod settings;`
- `egosync-app/src-tauri/src/lib.rs` — 注册 `settings_get_schedule` 和 `settings_update_schedule`
- `egosync-app/src-tauri/src/services/scheduler.rs` — 新增 `get_review_schedule` / `get_bigrock_reminder_schedule` 辅助函数
- `egosync-app/src/components/butler/ButlerSettingsContent.tsx` — 导入 scheduleService，新增 reviewDay/reviewTime/bigrockReminderDay/bigrockReminderTime state，替换 briefing_time 单独加载为 getSchedule 统一加载，周复盘 UI 接通真实数据，新增大石头规划 UI，晨间简报 onChange 迁移到 scheduleService
- `egosync-app/src/components/butler/ButlerSettingsContent.test.tsx` — 添加 scheduleService mock + 4 个节奏化时间配置测试

### Change Log

- 2026-06-26: Story 6.2 实现完成 — 三项节奏化时间配置（晨间简报/周复盘/大石头规划）全部接通真实数据持久化，后端统一 command 读写，调度器预留读取入口

### Review Findings (2026-06-26)

- [x] [Review][Patch] AC7「spawn_scheduler 启动时读取配置」字面要求未满足 — 已修复：`spawn_scheduler` 启动时调用 `get_review_schedule` / `get_bigrock_reminder_schedule` 各一次并打 `tracing::debug` 日志，坐实「启动时读取」。[scheduler.rs:334-343]
- [x] [Review][Patch] `settings_update_schedule` 多字段更新非原子 — 已修复：拆分为「先全量校验所有 `Some` 字段，校验通过后再统一写入」，避免部分写入。[commands/settings.rs:106-138]
- [x] [Review][Patch] 节奏化默认值三处重复有漂移风险 — 已修复：`settings.rs` 的 review/bigrock 键与默认值常量改为 `pub`，`scheduler.rs` 读取函数复用，消除重复。[commands/settings.rs:8-17 / scheduler.rs:486-511]
