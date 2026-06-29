---
baseline_commit: fbbc751
---

# Story 6.4: 每周末管家生成周复盘成绩单（正向叙事）

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 每周收到一份温暖的成绩单回顾本周进展,
So that 感受到进步的满足感并为下周做准备。

## 背景与现状（务必先读）

**本 story 是 Epic 6 的第四个 story — 在 Story 6.1（晨间简报生成服务 `briefing_generator.rs`）、Story 6.2（节奏化时间配置 `settings_get_schedule` / `settings_update_schedule` + 调度器预留读取入口 `get_review_schedule`）、Story 6.3（大石头规划提醒 `bigrock_reminder.rs` + 调度器 tick 循环大石头触发检查 + 前端 `bigrock:reminder` 事件监听）基础上，实现每周末的周复盘成绩单自动生成。**

**核心交付：** 调度器在配置的周复盘时间（默认周日 20:00）触发 `review_generator` 服务，收集本周各角色能量值、大石头完成情况、新沉淀记忆条数、新启用 Skill 等数据，构造复盘 prompt 调用 LLM 生成正向叙事的复盘摘要，写入 `weekly_reviews` 表 + 管家对话消息 + 发送"轻触"通知 + emit `review:generated` 事件供前端监听刷新。

### 已建成的基础（本 story 的接入点）

**Story 6.2 已完成的设施（直接复用）：**
- `src-tauri/src/commands/settings.rs:8-9` — 常量 `DEFAULT_REVIEW_DAY = "7"`（周日）、`DEFAULT_REVIEW_TIME = "20:00"`
- `src-tauri/src/commands/settings.rs:14-15` — `KEY_REVIEW_DAY = "review_day"`、`KEY_REVIEW_TIME = "review_time"`
- `src-tauri/src/commands/settings.rs:48-55` — `validate_review_day` 函数，校验值 `"1"` ~ `"7"`
- `src-tauri/src/services/scheduler.rs:529-539` — `get_review_schedule(pool)` 函数已实现，从 `app_settings` 读取 `review_day` + `review_time`，返回 `(day, time)` 元组
- `src-tauri/src/services/scheduler.rs:339-342` — `spawn_scheduler` 启动时已调用 `get_review_schedule` 并打日志（预留读取入口）
- 前端 `scheduleService.ts` + `ButlerSettingsContent.tsx` — 周复盘时间配置 UI 已接通真实数据

**Story 6.1 简报生成服务（参照模式）：**
- `src-tauri/src/services/briefing_generator.rs` — 完整的"去重检查 → 收集数据 → 构造 prompt → 调用 LLM → 写入 DB + 管家对话 → emit Tauri Event"模式，本 story 的 `review_generator.rs` 应参照此模式实现
- `src-tauri/src/services/briefing_generator.rs:303-354` — `call_llm` 函数（stream + timeout + size limit），可直接复制复用
- `src-tauri/src/services/agent_engine.rs:1928-1957` — `resolve_default_provider(pool)` 函数，解析默认 LLM provider
- `src-tauri/src/db/briefings.rs` — `briefings` 表 DB 操作（参照实现 `weekly_reviews` 表 DB 操作）
- `src-tauri/src/commands/briefing.rs` — 简报 Tauri commands（参照实现周复盘 commands）
- `src-tauri/migrations/023_briefings.sql` — `briefings` 表迁移（参照实现 `weekly_reviews` 表迁移）

**Story 6.3 大石头提醒（参照模式）：**
- `src-tauri/src/services/bigrock_reminder.rs` — 完整的"检测 + 创建通知 + 写入管家对话 + emit Tauri Event"模式
- `src-tauri/src/services/scheduler.rs:484-521` — 调度器 tick 循环中大石头提醒触发检查（参照实现周复盘触发检查）
- `src-tauri/src/services/scheduler.rs:334-335` — `last_bigrock_trigger_week: Option<String>` 去重模式（ISO 周编号 `"YYYY-Www"`），本 story 需参照此模式实现周复盘每周去重
- `src-tauri/src/services/scheduler.rs:556-560` — `iso_week_key(now)` 函数已实现，直接复用

**调度器现有结构（`scheduler.rs`）：**
- `src-tauri/src/services/scheduler.rs:322` — `spawn_scheduler(pool, conv_pool, app_handle)` 函数
- `src-tauri/src/services/scheduler.rs:348-525` — tick 循环结构：查询角色 → 角色工作循环 → 简报触发检查 → Q2 提醒检查 → 大石头提醒检查 → `interval.tick().await`
- **本 story 需在大石头提醒检查之后、`interval.tick().await` 之前新增周复盘触发检查**

**任务 DB 现有设施：**
- `src-tauri/src/db/tasks.rs:60-85` — `list_all_tasks(pool, quadrant, is_big_rock)` 函数，可传 `is_big_rock=Some(true)` 查询所有大石头任务（含跨角色 + butler 任务）
- 任务表含 `is_completed`、`completed_at` 字段，可用于统计本周完成情况

**记忆 DB 现有设施：**
- `src-tauri/src/db/memories.rs:361-387` — `list_all_memories_with_options(pool, category, limit, offset)` 函数，可查询全部记忆
- `src-tauri/src/db/memories.rs:389-405` — `count_memories(pool, role_id, include_role_memories, category)` 函数

**Skill DB 现有设施：**
- `src-tauri/src/db/skill_bindings.rs` — `skill_role_bindings` 表操作函数
- `src-tauri/migrations/008_skills_registry.sql` — `skills` 表（id, name, description, source_type, managed_path, content_hash, created_at, updated_at）
- `src-tauri/migrations/009_skill_role_bindings.sql` — `skill_role_bindings` 表（skill_id, role_id, created_at）

**角色 DB 现有设施：**
- `src-tauri/src/db/roles.rs` — `list_active_roles(pool)` 函数
- `src-tauri/src/models/role.rs:3-18` — `Role` 结构体含 `energy: i32`、`energy_updated_at: Option<String>`
- **注意：当前系统不保存能量值历史快照，只有当前值。`energy_updated_at` 记录最后一次计算时间。无法直接查询"上周能量值"。**

**前端现有设施：**
- `src/App.tsx:40-41` — `isReviewOpen` + `reviewInitialPhase` state 控制 `WeeklyReviewModal` 显示
- `src/App.tsx:370` — `<WeeklyReviewModal>` 渲染位置
- `src/App.tsx:104-113` — `bigrock:reminder` 事件监听模式（参照实现 `review:generated` 事件监听）
- `src/components/modals/WeeklyReviewModal.tsx` — 组件已存在（mock 数据），Story 6.5 负责接通真实数据

**通知服务现有设施：**
- `src-tauri/src/services/notification_service.rs:57-102` — `create_notification_for_role(pool, role_id, requested_level, content)` 函数
- `src-tauri/src/services/suggestion_generator.rs:355-361` — `NotificationLevel` 枚举（`Whisper` < `Tap` < `Knock`）

### 本 story 需要做的事

**核心变更：实现周复盘成绩单的自动生成 + 通知 + 前端事件监听链路。**

1. **数据库迁移**：新增 `025_weekly_reviews.sql` 创建 `weekly_reviews` 表
2. **Rust 后端**：新增 `review_generator.rs` 服务，收集本周数据 → 构造复盘 prompt → LLM 生成正向叙事复盘 → 写入 `weekly_reviews` 表 + 管家对话 + 通知 + emit 事件
3. **Rust 调度器**：在 tick 循环中新增周复盘触发检查，读取 `review_day/time` 配置，匹配星期 + 时间时触发，每周只触发一次（去重）
4. **Rust Commands**：新增 `review_get_weekly` command 供前端查询复盘数据
5. **前端**：监听 `review:generated` 事件，递增管家对话刷新触发器

## Acceptance Criteria

1. **AC1**: Given 到达周复盘触发时间，When 调度器触发复盘生成，Then LLM 基于以下输入生成复盘内容：各角色能量值变化趋势（本周 vs 上周）、大石头完成情况、新沉淀的记忆条数、新启用/使用的 Skill、任务完成统计

2. **AC2**: Given 复盘叙事风格，Then 使用正向框架：已完成 = ✓ 图标 + 翠绿色、继续推进 = → 图标 + 琥珀色、**禁止使用"未完成""失败"等负面措辞**

3. **AC3**: Given 复盘以反思对话形式呈现，Then 管家对话区展示复盘摘要段落，And 末尾显示"查看详细复盘"按钮 → 打开 WeeklyReviewModal

4. **AC4**: Given 复盘生成后，Then 发送"轻触"通知"本周复盘已准备好"

5. **AC5**: Given 数据库，Then `migrations/025_weekly_reviews.sql` 创建 `weekly_reviews` 表：`id`, `week_start`, `week_end`, `summary`, `energy_trends`(JSON), `bigrock_status`(JSON), `new_memories_count`, `created_at`

6. **AC6**: Given Rust 后端，Then `review_generator` 服务：收集本周数据 → 构造复盘 prompt → LLM 返回结构化复盘 JSON，And 复盘写入 `weekly_reviews` 表

## Tasks / Subtasks

- [ ] **Task 1: 数据库迁移 — 新增 `weekly_reviews` 表** (AC: #5)
  - [ ] 1.0 新建 `src-tauri/migrations/025_weekly_reviews.sql`
  - [ ] 1.1 创建 `weekly_reviews` 表：
    ```sql
    CREATE TABLE weekly_reviews (
        id TEXT PRIMARY KEY NOT NULL,
        week_start TEXT NOT NULL,
        week_end TEXT NOT NULL,
        summary TEXT NOT NULL,
        energy_trends TEXT NOT NULL DEFAULT '{}',
        bigrock_status TEXT NOT NULL DEFAULT '{}',
        new_memories_count INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
    CREATE UNIQUE INDEX idx_weekly_reviews_week_start ON weekly_reviews(week_start);
    ```
  - [ ] 1.2 `week_start` 加 UNIQUE 约束，确保同一周只有一条复盘记录（参照 `briefings.date UNIQUE`）

- [ ] **Task 2: Rust — 新增 `weekly_review.rs` model** (AC: #5)
  - [ ] 2.0 新建 `src-tauri/src/models/weekly_review.rs`
  - [ ] 2.1 定义 `WeeklyReview` 结构体（derive `Debug, Clone, Serialize, Deserialize, sqlx::FromRow` + `#[serde(rename_all = "camelCase")]`）
    - 字段：`id: String`, `week_start: String`, `week_end: String`, `summary: String`, `energy_trends: String`, `bigrock_status: String`, `new_memories_count: i64`, `created_at: String`
  - [ ] 2.2 在 `src-tauri/src/models/mod.rs` 中添加 `pub mod weekly_review;`

- [ ] **Task 3: Rust — 新增 `db/weekly_reviews.rs` DB 操作** (AC: #5, #6)
  - [ ] 3.0 新建 `src-tauri/src/db/weekly_reviews.rs`
  - [ ] 3.1 实现 `create_weekly_review(pool, week_start, week_end, summary, energy_trends, bigrock_status, new_memories_count) -> Result<WeeklyReview, AppError>` — 参照 `db/briefings.rs::create_briefing`
  - [ ] 3.2 实现 `get_weekly_review_by_week_start(pool, week_start) -> Result<Option<WeeklyReview>, AppError>` — 参照 `db/briefings.rs::get_briefing_by_date`
  - [ ] 3.3 实现 `get_latest_weekly_review(pool) -> Result<Option<WeeklyReview>, AppError>` — 参照 `db/briefings.rs::get_latest_briefing`
  - [ ] 3.4 在 `src-tauri/src/db/mod.rs` 中添加 `pub mod weekly_reviews;`

- [ ] **Task 4: Rust — 新增 `review_generator.rs` 服务** (AC: #1, #2, #4, #6)
  - [ ] 4.0 新建 `src-tauri/src/services/review_generator.rs`，在 `src-tauri/src/services/mod.rs` 中添加 `pub mod review_generator;`
  - [ ] 4.1 定义 `REVIEW_GENERATED_EVENT: &str = "review:generated"` 常量
  - [ ] 4.2 定义 `ReviewGeneratedPayload` struct（`serde::Serialize` + `rename_all = "camelCase"`），字段：`review_id: String`, `week_start: String`, `week_end: String`
  - [ ] 4.3 定义 `ReviewData` 内部结构体，字段：
    - `role_statuses: Vec<DashboardStatus>` — 各角色当前状态（能量值 + 待处理任务数）
    - `bigrock_tasks: Vec<CrossRoleTask>` — 本周大石头任务（含完成状态）
    - `new_memories_count: usize` — 本周新沉淀记忆条数
    - `new_skill_names: Vec<String>` — 本周新启用的 Skill 名称列表
    - `task_completion_stats: TaskCompletionStats` — 任务完成统计（总任务数、已完成数、完成率）
  - [ ] 4.4 实现主入口函数 `generate_review_if_needed(pool, conv_pool, app_handle) -> Result<bool, AppError>`：
    - 计算本周一到本周日的日期范围（`week_start` / `week_end`）
    - 去重检查：`get_weekly_review_by_week_start(pool, &week_start)` 已有则跳过
    - 调用 `collect_review_data(pool, conv_pool, &week_start, &week_end)` 收集数据
    - 构造复盘 prompt（`build_review_prompt`）
    - 解析 LLM provider（`agent_engine::resolve_default_provider(pool)`）
    - 调用 LLM（复用 `call_llm` 模式）
    - 构造 `energy_trends` JSON 和 `bigrock_status` JSON
    - 写入 `weekly_reviews` 表
    - 写入管家对话消息（`db::conversations::get_or_create_butler_conversation` + `insert_message`）
    - 创建"轻触"通知（使用第一个活跃角色 ID，参照 `bigrock_reminder.rs` 模式）
    - emit `review:generated` 事件
    - 返回 `Ok(true)`
  - [ ] 4.5 实现 `collect_review_data(pool, conv_pool, week_start, week_end) -> Result<ReviewData, AppError>`：
    - 角色状态：调用 `dashboard_service::get_dashboard_status(pool, conv_pool)` 获取各角色当前能量值和待处理任务数
    - 大石头任务：调用 `db::tasks::list_all_tasks(pool, None, Some(true))` 获取所有大石头任务
    - 新记忆条数：调用 `db::memories::list_all_memories_with_options(pool, None, None, None)` 获取全部记忆，在 Rust 侧按 `created_at` 过滤本周范围内的条数
    - 新启用 Skill：查询 `skill_role_bindings` 表中 `created_at` 在本周范围内的记录，关联 `skills` 表获取 Skill 名称
    - 任务完成统计：查询 `tasks` 表中 `completed_at` 在本周范围内的已完成任务数 + 总任务数
  - [ ] 4.6 实现 `build_review_prompt(data, week_start, week_end) -> Vec<ChatCompletionMessage>`：
    - System prompt 强调**正向叙事风格**：使用"已完成"而非"未完成"、使用"继续推进"而非"失败"、用温暖鼓励的语气
    - User prompt 包含：日期范围、各角色能量值、大石头完成情况、新记忆条数、新 Skill、任务完成统计
    - 要求 LLM 返回自然语言复盘摘要段落（约 300-500 字）
  - [ ] 4.7 实现 `call_llm` 函数 — 直接复制 `briefing_generator.rs:303-354` 的 `call_llm` 实现（stream + timeout + size limit）
  - [ ] 4.8 错误处理：所有 LLM/数据收集错误降级为 `tracing::warn!` + 返回 `Ok(false)`，不阻塞调度器

- [ ] **Task 5: Rust — 调度器 tick 循环新增周复盘触发检查** (AC: #1)
  - [ ] 5.1 在 `spawn_scheduler` 函数的 tick 循环中，在大石头提醒触发检查之后、`interval.tick().await` 之前，新增周复盘触发检查
  - [ ] 5.2 新增去重 state：`last_review_trigger_week: Option<String>`（格式 `"YYYY-Www"`，复用 `iso_week_key` 函数）
  - [ ] 5.3 读取 `get_review_schedule(&pool)` 获取配置的星期 + 时间
  - [ ] 5.4 检查条件：当前本地时间的星期等于配置的 `day`，且当前 `HH:MM` 等于配置的 `time`，且本周尚未触发
  - [ ] 5.5 条件满足时 `tokio::spawn` 异步调用 `review_generator::generate_review_if_needed`，错误只 warn
  - [ ] 5.6 触发后更新 `last_review_trigger_week` 去重 state

- [ ] **Task 6: Rust — 新增 Tauri Command** (AC: #6)
  - [ ] 6.0 新建 `src-tauri/src/commands/review.rs`，在 `src-tauri/src/commands/mod.rs` 中添加 `pub mod review;`
  - [ ] 6.1 实现 `review_get_latest` command — 返回最新一条周复盘记录
  - [ ] 6.2 实现 `review_get_by_week` command — 按 `week_start` 查询指定周的复盘
  - [ ] 6.3 实现 `review_generate_now` command — 手动触发复盘生成（参照 `briefing_generate_now`）
  - [ ] 6.4 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册三个 commands

- [ ] **Task 7: 前端 — 监听 `review:generated` 事件** (AC: #3, #4)
  - [ ] 7.1 在 `src/types/` 中新增 `review.ts` 类型定义：`ReviewGeneratedPayload { reviewId: string; weekStart: string; weekEnd: string }` 和 `WeeklyReview { id: string; weekStart: string; weekEnd: string; summary: string; energyTrends: string; bigrockStatus: string; newMemoriesCount: number; createdAt: string }`
  - [ ] 7.2 在 `src/App.tsx` 中新增 `useTauriEvent<ReviewGeneratedPayload>('review:generated', ...)` 监听
  - [ ] 7.3 事件回调中：递增 `butlerChatRefreshTrigger` 触发管家对话刷新（参照 `briefing:generated` 事件处理模式）
  - [ ] 7.4 新增 `src/services/reviewService.ts` 封装 Tauri invoke 调用（`getLatestReview`、`getReviewByWeek`、`generateReviewNow`）

- [ ] **Task 8: 单元测试** (AC: #1, #2, #5, #6)
  - [ ] 8.1 Rust 测试 — 在 `review_generator.rs` 中测试 `build_review_prompt`：
    - 包含各角色能量值
    - 包含大石头完成情况
    - 包含新记忆条数
    - System prompt 包含正向叙事风格指令（禁止"未完成""失败"）
  - [ ] 8.2 Rust 测试 — 在 `db/weekly_reviews.rs` 中测试 CRUD：
    - `create_weekly_review` 插入成功
    - `get_weekly_review_by_week_start` 返回已有记录 / None
    - `get_latest_weekly_review` 返回最新记录
    - 同一 `week_start` 重复插入失败（UNIQUE 约束）
  - [ ] 8.3 Rust 测试 — 在 `review_generator.rs` 中测试 `generate_review_if_needed` 去重逻辑（已有复盘则跳过）
  - [ ] 8.4 Rust 测试 — 验证 `REVIEW_GENERATED_EVENT` 常量值为 `"review:generated"`
  - [ ] 8.5 前端测试 — 验证 `review:generated` 事件到达时 `butlerChatRefreshTrigger` 递增

### Review Findings

> 代码审查于 2026-06-27（Blind Hunter / Edge Case Hunter / Acceptance Auditor 三层对抗审查，基线 `fbbc751`）

- [x] [Review][Patch] `energy_trends` JSON 字段语义偏离规格 — 已修复：`build_energy_trends_json` 改用 `roles.energy_updated_at`，JSON 键 `energyUpdatedAt`，新增 `roles` 数据收集。[review_generator.rs:333-349]
- [x] [Review][Patch] 时区边界误差导致本周统计漏算/多算 — 已修复：新增 `local_day_bound_to_utc`，将本地日起止时刻换算为 UTC 后再做字符串比较。[review_generator.rs:317-331, 226-232]
- [x] [Review][Patch] LLM 超限静默截断仍持久化 — 已修复：超限改为返回 `Err`，并在 `call_llm` 传播内层错误，降级跳过不再持久化残缺内容。[review_generator.rs:494-519]
- [x] [Review][Patch] 前端测试弱于 Task 8.5 意图 — 已修复：mock `ButlerView` 暴露 `chatRefreshTrigger`，断言事件到达后该值递增 1。[App.test.tsx:36-51, 229-245]
- [x] [Review][Defer] 单点触发时刻无重试窗口 [scheduler.rs:530-540] — deferred, pre-existing（briefing/bigrock 共有设计）
- [x] [Review][Defer] `collect_new_skill_names` N+1 查询 [review_generator.rs:289-300] — deferred, pre-existing（Dev Notes 已说明表数据量小可接受）
- [x] [Review][Defer] 故事文件元信息未更新（Status/File List/Dev Agent Record 仍为占位）[6-4-weekly-review-scorecard.md:7,337] — deferred, dev-story 收尾流程职责

## Dev Notes

### 关键技术决策

- **能量值变化趋势（本周 vs 上周）的数据来源限制**：当前系统不保存能量值历史快照，`roles.energy` 只有当前值，`roles.energy_updated_at` 记录最后一次计算时间。**无法直接查询"上周能量值"**。方案：在 `collect_review_data` 中收集各角色当前能量值 + `energy_updated_at`，在 prompt 中提供当前能量值和最近更新时间，让 LLM 基于这些信息生成趋势描述。`energy_trends` JSON 字段存储各角色当前能量值快照（`{ "role_id": { "energy": 85, "energy_updated_at": "2026-..." } }`），供 Story 6.5 前端展示。**不新增能量值历史表** — 那超出本 story 范围。

- **正向叙事风格的 prompt 设计**：System prompt 必须明确要求 LLM：
  - 使用"已完成"（✓）而非"未完成"
  - 使用"继续推进"（→）而非"失败"或"延期"
  - 用温暖鼓励的语气，强调进步和满足感
  - 禁止使用"未完成""失败""落后"等负面措辞
  - 对未完成的大石头用"继续推进"框架描述

- **周日期范围计算**：本周一 00:00:00 到本周日 23:59:59。使用 `chrono::Local::now().date_naive()` 获取今天，然后计算本周一：`today - chrono::Duration::days((today.weekday().num_days_from_monday()) as i64)`。`week_end` = `week_start + chrono::Duration::days(6)`。

- **去重策略**：使用 ISO 周编号 `"YYYY-Wss"` 作为去重键（如 `"2026-W23"`），复用 `scheduler.rs` 中已有的 `iso_week_key` 函数。同时 `weekly_reviews` 表的 `week_start` UNIQUE 约束作为数据库层去重保障。

- **新启用 Skill 的查询**：查询 `skill_role_bindings` 表中 `created_at` 在本周范围内的记录，关联 `skills` 表获取 Skill 名称。需要新增一个 DB 查询函数（或在 Rust 侧过滤）。由于 `skill_role_bindings` 表数据量小，可以全量查询后在 Rust 侧按 `created_at` 过滤。

- **任务完成统计**：查询 `tasks` 表中 `completed_at` 在本周范围内的已完成任务数（`is_completed = 1 AND completed_at BETWEEN week_start AND week_end`），以及总任务数。需要新增一个 DB 查询函数或在 Rust 侧用现有 `list_all_tasks` 过滤。

- **LLM 返回格式**：AC6 提到"LLM 返回结构化复盘 JSON"，但考虑到 `briefing_generator.rs` 使用自然语言段落（非 JSON），且 Story 6.5 才负责前端展示，本 story 的 LLM 返回**自然语言复盘摘要段落**（约 300-500 字），写入 `weekly_reviews.summary` 字段。`energy_trends` 和 `bigrock_status` 字段由 Rust 代码构造为 JSON（非 LLM 生成），供 Story 6.5 前端渲染使用。

- **通知创建的 role_id 问题**：与 `bigrock_reminder.rs` 相同模式 — 使用第一个活跃角色的 ID 创建通知。若没有活跃角色则跳过通知创建，只写入管家对话消息。

- **"查看详细复盘"按钮（AC3）**：当前管家对话区（`ButlerView`）的消息不支持 action button。本 story 采用与 Story 6.3 相同的事件驱动方案：`review:generated` 事件到达前端时递增管家对话刷新触发器，用户在管家对话区看到复盘摘要后，可手动打开 WeeklyReviewModal 查看详细内容。**不修改 `ButlerView` 组件** — "查看详细复盘"按钮的增强是 Story 6.5 的职责。

### 架构合规

- **分层规则**：调度器 tick → `review_generator::generate_review_if_needed` → `db::weekly_reviews::create_weekly_review` + `db::conversations::insert_message` + `notification_service::create_notification_for_role` + `app_handle.emit("review:generated")`
- **服务层职责**：`review_generator.rs` 负责"去重 → 数据收集 → prompt 构造 → LLM 调用 → 写入 DB + 对话 + 通知 + 事件"，参照 `briefing_generator.rs` 模式
- **调度器职责**：tick 循环中读取 `review_day/time` 配置，匹配星期 + HH:MM 时 spawn 异步调用 `review_generator`，参照大石头提醒触发模式
- **命名规范**：Rust 文件 `review_generator.rs`（snake_case），Tauri Event `"review:generated"`（kebab-case + 冒号分隔），前端类型 `ReviewGeneratedPayload`（PascalCase），前端 service `reviewService.ts`（camelCase），DB 表 `weekly_reviews`（snake_case 复数），DB 列 `week_start`、`energy_trends`、`bigrock_status`（snake_case）
- **错误处理**：所有错误只 `tracing::warn!`，不 panic，不阻塞调度器 tick 循环
- **serde 桥接**：`ReviewGeneratedPayload` 和 `WeeklyReview` 均派生 `Serialize + Deserialize` + `#[serde(rename_all = "camelCase")]`
- **迁移文件命名**：`025_weekly_reviews.sql`（下一个序号，参照现有 `024_llm_provider_minimax.sql`）

### 前端 UI 规范

- **不新增组件文件**：在 `App.tsx` 中新增事件监听，在 `src/types/review.ts` 中新增类型，在 `src/services/reviewService.ts` 中封装 invoke
- **不修改 `WeeklyReviewModal.tsx`** — Modal 内部数据接通是 Story 6.5 的职责
- **不修改 `ButlerView.tsx`** — 管家对话区消息展示复用现有机制（刷新触发器递增 → 重新加载消息列表）
- **事件监听模式**：参照现有 `useTauriEvent<BriefingGeneratedPayload>('briefing:generated', ...)` 模式

### 反模式警告

- **不要**新增能量值历史表 — 那超出本 story 范围，当前只存储能量值快照到 `energy_trends` JSON
- **不要**修改 `WeeklyReviewModal.tsx` 内部功能 — 那是 Story 6.5 的职责
- **不要**修改 `ButlerView.tsx` — 复盘摘要通过管家对话消息展示，刷新机制已由 `butlerChatRefreshTrigger` 实现
- **不要**修改 `NotificationPanel.tsx` — 通知卡片保持原样
- **不要**在 `commands/` 层添加业务逻辑 — Command 只做参数解析 → 调 Service → 返回结果
- **不要**将去重键用日期字符串 — 用 ISO 周编号 `"YYYY-Wss"` 确保每周只触发一次，复用 `iso_week_key` 函数
- **不要**在 tick 循环中同步调用 `generate_review_if_needed` — 必须 `tokio::spawn` 异步调用，避免阻塞 tick 循环
- **不要**修改 `get_review_schedule` 函数 — Story 6.2 已实现，直接复用
- **不要**修改 `settings_get_schedule` / `settings_update_schedule` commands — Story 6.2 已实现，直接复用
- **不要**使用 `weekday().num()` — chrono 的 `Weekday` 没有 `num()` 方法，正确 API 是 `weekday().num_days_from_monday() + 1`（返回 1=周一 ~ 7=周日）
- **不要**让 LLM 返回 JSON — LLM 返回自然语言复盘摘要段落，`energy_trends` 和 `bigrock_status` 由 Rust 代码构造
- **不要**在 prompt 中省略正向叙事风格指令 — AC2 明确要求禁止"未完成""失败"等负面措辞

### Project Structure Notes

新增文件：
- `src-tauri/migrations/025_weekly_reviews.sql` — `weekly_reviews` 表迁移
- `src-tauri/src/models/weekly_review.rs` — `WeeklyReview` 结构体
- `src-tauri/src/db/weekly_reviews.rs` — `weekly_reviews` 表 DB 操作
- `src-tauri/src/services/review_generator.rs` — 周复盘生成服务
- `src-tauri/src/commands/review.rs` — 周复盘 Tauri commands
- `src/types/review.ts` — 前端类型定义
- `src/services/reviewService.ts` — 前端 service 封装

修改文件：
- `src-tauri/src/models/mod.rs` — 添加 `pub mod weekly_review;`
- `src-tauri/src/db/mod.rs` — 添加 `pub mod weekly_reviews;`
- `src-tauri/src/services/mod.rs` — 添加 `pub mod review_generator;`
- `src-tauri/src/commands/mod.rs` — 添加 `pub mod review;`
- `src-tauri/src/services/scheduler.rs` — tick 循环新增周复盘触发检查 + `last_review_trigger_week` 去重 state
- `src-tauri/src/lib.rs` — 注册 `review_get_latest` / `review_get_by_week` / `review_generate_now` commands
- `src/App.tsx` — 新增 `review:generated` 事件监听

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.4] — AC 原文
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6] — Epic 6 上下文（FR-16, FR-17, FR-18, UX-DR11）
- [Source: _bmad-output/planning-artifacts/architecture.md#weekly_reviews] — `weekly_reviews` 表架构定义
- [Source: _bmad-output/planning-artifacts/architecture.md#Chart Library] — 自绘 SVG 能量趋势图（Story 6.5 职责）
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: _bmad-output/implementation-artifacts/6-3-big-rock-planning-reminder.md] — Story 6.3 实现上下文（调度器触发模式 + 事件驱动模式）
- [Source: src-tauri/src/services/briefing_generator.rs] — 简报生成服务（参照实现周复盘生成服务）
- [Source: src-tauri/src/services/briefing_generator.rs:303-354] — `call_llm` 函数（直接复制复用）
- [Source: src-tauri/src/services/briefing_generator.rs:220-300] — `build_briefing_prompt` 函数（参照构造复盘 prompt）
- [Source: src-tauri/src/services/agent_engine.rs:1928-1957] — `resolve_default_provider` 函数
- [Source: src-tauri/src/services/bigrock_reminder.rs] — 大石头提醒服务（参照通知创建 + 管家对话写入模式）
- [Source: src-tauri/src/services/scheduler.rs:322-525] — `spawn_scheduler` tick 循环结构
- [Source: src-tauri/src/services/scheduler.rs:484-521] — 大石头提醒触发检查（参照实现周复盘触发检查）
- [Source: src-tauri/src/services/scheduler.rs:529-539] — `get_review_schedule` 函数（已实现，直接复用）
- [Source: src-tauri/src/services/scheduler.rs:556-560] — `iso_week_key` 函数（直接复用）
- [Source: src-tauri/src/services/notification_service.rs:57-102] — `create_notification_for_role` 函数
- [Source: src-tauri/src/services/suggestion_generator.rs:355-361] — `NotificationLevel` 枚举
- [Source: src-tauri/src/db/briefings.rs] — `briefings` 表 DB 操作（参照实现 `weekly_reviews` 表 DB 操作）
- [Source: src-tauri/src/commands/briefing.rs] — 简报 Tauri commands（参照实现周复盘 commands）
- [Source: src-tauri/migrations/023_briefings.sql] — `briefings` 表迁移（参照实现 `weekly_reviews` 表迁移）
- [Source: src-tauri/src/db/tasks.rs:60-85] — `list_all_tasks` 函数
- [Source: src-tauri/src/db/memories.rs:361-387] — `list_all_memories_with_options` 函数
- [Source: src-tauri/src/db/skill_bindings.rs] — `skill_role_bindings` 表操作
- [Source: src-tauri/src/db/roles.rs] — `list_active_roles` 函数
- [Source: src-tauri/src/models/role.rs:3-18] — `Role` 结构体（energy, energy_updated_at 字段）
- [Source: src-tauri/src/models/dashboard.rs] — `DashboardStatus` 结构体
- [Source: src-tauri/src/services/dashboard_service.rs:10-60] — `get_dashboard_status` 函数
- [Source: src-tauri/src/commands/settings.rs:8-17] — 周复盘时间配置常量
- [Source: src-tauri/src/lib.rs:364-367] — Command 注册位置
- [Source: src/App.tsx:40-41] — `isReviewOpen` + `reviewInitialPhase` state
- [Source: src/App.tsx:104-113] — `bigrock:reminder` 事件监听模式（参照实现 `review:generated` 事件监听）
- [Source: src/App.tsx:370] — `WeeklyReviewModal` 渲染位置
- [Source: src/components/modals/WeeklyReviewModal.tsx] — 组件现状（不修改）
- [Source: src/hooks/useTauriEvent.ts] — Tauri 事件监听 hook

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

（待开发完成后填写）
