---
baseline_commit: 0ec5563a43b677de604bc4dab7567b857afce626
---

# Story 6.1: 管家每日生成晨间简报并以自然语言呈现

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 每天打开应用就能看到管家为我汇总的今日概览,
So that 快速了解各角色状态和今天最重要的事。

## 背景与现状（务必先读）

**本 story 是 Epic 6 的第一个 story — 在 Epic 4 已建成的调度器（`scheduler.rs`）、建议生成（`suggestion_generator.rs`）、通知系统（`notification_service.rs`）和前端 `ButlerView` / `ChatStream` / `ActionCard` 组件基础上，新增 `briefing_generator` Rust 服务，每日按配置时间自动生成自然语言段落简报，写入 `briefings` 表，并以管家对话消息形式呈现在 ButlerView 对话区。本 story 不涉及时间配置 UI（Story 6.2）、大石头规划（Story 6.3）、周复盘（Story 6.4/6.5）和大石头保护（Story 6.6），只负责"简报生成 + 持久化 + 呈现 + 去重"。**

### 已建成的基础（本 story 的接入点）

**Epic 4 已完成的设施：**
- `src-tauri/src/services/scheduler.rs` — 后台调度器，60 秒 tick，按角色 proactivity_level 的配置时间点触发 `run_work_loop_for_role`。**本 story 需在调度器中新增独立的简报触发逻辑**（不依赖角色 proactivity，全局单一时间点）
- `src-tauri/src/services/suggestion_generator.rs` — 建议生成服务，含 `generate_suggestions`、`filter_suggestions_by_proactivity`、`NotificationLevel` 枚举
- `src-tauri/src/services/notification_service.rs` — 三级通知创建服务（Whisper < Tap < Knock）
- `src-tauri/src/services/q2_protection_reminder.rs` — **关键参考模式**：`check_and_generate_reminders` 函数展示了"查询数据 → 生成文案 → 写入管家对话消息 → emit Tauri Event"的完整模式，本 story 的简报生成应复用此模式
- `src-tauri/src/db/conversations.rs:32-46` — `get_or_create_butler_conversation` 获取管家对话
- `src-tauri/src/db/conversations.rs:48-88` — `insert_message` 写入对话消息（`role="assistant"`, `is_complete=true`）
- `src-tauri/src/commands/scheduler.rs` — `scheduler_get_times` / `scheduler_set_times` Tauri commands，使用 `app_settings` 表存储配置
- `src-tauri/src/db/app_settings.rs` — `get_setting` / `set_setting` 读写 `app_settings` 表

**前端已建成的设施：**
- `src/components/butler/ButlerView.tsx` — 管家主视图，含 `ChatStream` 对话区 + `ActionCard` 建议卡片
- `src/components/chat/ChatStream.tsx` — 对话流组件，支持 `refreshTrigger` prop 触发历史重新加载（Story 4.6 已实现 Q2 提醒刷新机制）
- `src/components/butler/ActionCard.tsx` — 建议卡片组件，支持 confirm/reject/dismiss
- `src/hooks/useSuggestions.ts` — 建议管理 hook，按 conversationId 获取 pending 建议
- `src/services/suggestionService.ts` — 建议相关 Tauri command 封装
- `src/services/chatService.ts` — 对话相关 Tauri command 封装
- `src/App.tsx:82-90` — **关键参考**：`butlerChatRefreshTrigger` state + `useTauriEvent('q2:reminder')` 监听事件递增 trigger，传入 `ButlerView` 的 `chatRefreshTrigger` prop。本 story 需复用此模式监听 `briefing:generated` 事件

### 本 story 需要新增的核心服务 — `briefing_generator`

- 新建 `src-tauri/src/services/briefing_generator.rs` — 简报生成 LLM 服务
- 需在 `src-tauri/src/services/mod.rs` 添加 `pub mod briefing_generator;`
- 新建 `src-tauri/src/commands/briefing.rs` — Tauri command 层
- 需在 `src-tauri/src/commands/mod.rs` 添加 `pub mod briefing;`（若 mod.rs 不存在则检查 lib.rs 的模块声明方式）
- 需在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册新 commands
- 新建 `src-tauri/migrations/023_briefings.sql` — `briefings` 表迁移

**LLM 调用模式参考 — `suggestion_generator.rs`：**
- `src-tauri/src/services/suggestion_generator.rs:299-350` — `call_llm` 函数，使用 mpsc channel 收集流式 token + timeout，**本 story 应复用此模式**
- `src-tauri/src/services/suggestion_generator.rs:230-241` — `agent_engine::resolve_default_provider(main_pool)` 获取默认 LLM provider（返回 `Arc<dyn LlmProvider>`）
- `src-tauri/src/services/agent_engine.rs:1928-1952` — `resolve_default_provider` 函数，从 DB 读取默认 LLM 配置，创建 provider 实例
- `src-tauri/src/llm/traits.rs:46-55` — `LlmProvider` trait，`chat_stream` 方法
- `src-tauri/src/llm/traits.rs:37-44` — `ChatOptions` struct，简报生成应使用 `disable_thinking: true, tools: None, tool_choice: None`

**简报数据收集 — 已有 DB 查询函数：**
- `src-tauri/src/db/roles.rs:35-37` — `list_active_roles(pool)` 查询活跃角色列表
- `src-tauri/src/db/tasks.rs:579-620` — `get_task_stats_for_roles(pool, &role_ids)` 批量查询角色 pending 任务数和紧急任务数
- `src-tauri/src/db/tasks.rs:469-489` — `list_imminent_tasks_for_escalation(pool, deadline_threshold)` 查询临期任务（可复用于"今日截止任务"查询，但需新建查询函数，见 Dev Notes）
- `src-tauri/src/db/memories.rs:357-387` — `list_all_memories_with_options(pool, category, limit, offset)` 查询全部记忆（排除 task_status 类别）
- `src-tauri/src/db/notifications.rs:44-55` — `list_notifications(pool)` 列出所有通知（含角色信息），可筛选 whisper 级别
- `src-tauri/src/services/dashboard_service.rs:10-71` — `get_dashboard_status` 函数，已实现"角色状态 + 任务统计 + 排序"逻辑，**本 story 可直接调用此函数获取各角色状态摘要**

**简报写入管家对话 — 参考 `q2_protection_reminder.rs`：**
- `src-tauri/src/services/q2_protection_reminder.rs:167-193` — 写入管家对话消息的完整模式：`get_or_create_butler_conversation` → `insert_message(conv_pool, &conv.id, "assistant", &message, true)`
- 本 story 需复用此模式将简报内容写入管家对话

**前端事件监听 — 参考 `App.tsx`：**
- `src/App.tsx:82-90` — `butlerChatRefreshTrigger` + `useTauriEvent('q2:reminder')` 模式
- `src/components/chat/ChatStream.tsx:641-652` — `refreshTrigger` useEffect 重新加载对话历史
- 本 story 需新增 `useTauriEvent('briefing:generated')` 监听，递增 `butlerChatRefreshTrigger`

**DB 迁移：**
- 现有迁移文件编号到 `022_suggestion_conversation.sql`
- **本 story 需新建 `023_briefings.sql`** — 创建 `briefings` 表
- epics.md 中提到 `010_briefings.sql`，但实际编号 010 已被 `010_skills_opencode_source_type.sql` 占用，因此使用下一个可用编号 `023`

**Error 处理：**
- `src-tauri/src/error.rs` — `AppError` 枚举，含 `LlmError(String)` / `DbError(String)` / `ValidationError(String)` / `NotFound(String)` 等变体
- **本 story 沿用现有 AppError，无需新增变体**

## Acceptance Criteria

1. **AC1**: Given 到达用户配置的晨间简报时间（默认 08:00），When 调度器触发简报生成，Then `briefing_generator` 服务收集以下数据：各角色当前状态（能量值 + 待处理任务数，通过 `dashboard_service::get_dashboard_status` 获取）、今日截止的任务列表（deadline = 今天的未完成任务）、昨日新增的记忆（`memories` 表 created_at 在昨天的记录）、未读的耳语级通知汇总（`notifications` 表 level='whisper' AND is_read=0），And LLM 基于以上输入生成自然语言段落简报

2. **AC2**: Given 简报内容生成完成，Then 简报写入 `briefings` 表（`id`, `content`, `date`, `created_at`，`date` 为当天本地日期 `YYYY-MM-DD`），And 简报内容以管家对话消息形式写入管家对话（`insert_message(conv_pool, &butler_conv.id, "assistant", &briefing_content, true)`），And emit Tauri Event `briefing:generated`（payload: `{ briefingId, date }`）供前端监听刷新

3. **AC3**: Given 简报内容，Then 以管家对话消息形式呈现在 ButlerView 对话区（通过 `refreshTrigger` 机制触发 `ChatStream` 重新加载对话历史），And 段落中嵌入角色名称（纯文本，不要求链接跳转 — 角色跳转在 V2），And 结尾含"今天最重要的一件事"建议（LLM 在简报 prompt 中被要求从今日截止任务或最高能量角色中识别一件事）

4. **AC4**: Given 简报中有待确认建议（pending suggestions 绑定到管家对话），Then 前端 `useSuggestions(butlerConversationId)` 自动获取并渲染 `ActionCard` 组件（复用 Epic 4 Story 4.4 已建成机制，无需额外开发）

5. **AC5**: Given 用户当日已查看简报（`briefings` 表已有当天 `date` 记录），Then 不重复生成（调度器在触发前检查 `briefings` 表是否已有当天记录）

6. **AC6**: Given 用户在简报时间前打开应用，Then 显示上次简报内容（若有 — 通过管家对话历史自然呈现，因为简报已写入管家对话消息），And 到达简报时间后自动追加新简报（`briefing:generated` 事件触发 `ChatStream` 刷新）

7. **AC7**: Given 数据库，Then `migrations/023_briefings.sql` 创建 `briefings` 表：`id TEXT PRIMARY KEY, content TEXT NOT NULL, date TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL`

8. **AC8**: Given Rust 后端，Then `briefing_generator` 服务：收集各角色数据 → 构造简报 prompt → 调用默认 LLM provider（复用 `agent_engine::resolve_default_provider`）→ 返回自然语言段落，And 简报写入 `briefings` 表 + 管家对话消息，And LLM 超时/错误/解析失败时降级（不写入，不阻塞，`tracing::warn!` 记录错误）

9. **AC9**: Given 前端，Then `App.tsx` 新增 `useTauriEvent('briefing:generated')` 监听，回调中递增 `butlerChatRefreshTrigger`（复用现有 Q2 提醒的 refresh 机制），And `ButlerView` 的 `chatRefreshTrigger` prop 传入 `ChatStream` 触发对话历史重新加载

10. **AC10**: Given 调度器，Then `spawn_scheduler` 的 tick 循环中新增简报触发检查：每次 tick 检查当前本地时间是否等于 `app_settings.briefing_time`（默认 `08:00`），And 同一天只触发一次（通过 `briefings` 表 `date` 唯一约束 + 内存去重键双重保障），And 简报生成独立于角色工作循环（不依赖角色 proactivity_level）

## Tasks / Subtasks

- [ ] **Task 1: DB 迁移 — `briefings` 表** (AC: #7)
  - [ ] 1.1 新建 `src-tauri/migrations/023_briefings.sql`：
    ```sql
    CREATE TABLE briefings (
        id TEXT PRIMARY KEY NOT NULL,
        content TEXT NOT NULL,
        date TEXT NOT NULL UNIQUE,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
    ```
  - [ ] 1.2 `date` 列存储本地日期 `YYYY-MM-DD`（非 UTC ISO 8601），用于按天去重
  - [ ] 1.3 `UNIQUE` 约束确保每天只有一条简报

- [ ] **Task 2: Rust DB 层 — `db/briefings.rs`** (AC: #2, #5, #7)
  - [ ] 2.1 新建 `src-tauri/src/db/briefings.rs`：
    - `pub async fn create_briefing(pool: &SqlitePool, content: &str, date: &str) -> Result<Briefing, AppError>` — 插入简报，`id` = UUID v4，`created_at` = `chrono_now_pub()`
    - `pub async fn get_briefing_by_date(pool: &SqlitePool, date: &str) -> Result<Option<Briefing>, AppError>` — 按日期查询简报（用于去重检查）
    - `pub async fn get_latest_briefing(pool: &SqlitePool) -> Result<Option<Briefing>, AppError>` — 查询最新简报（按 `created_at DESC LIMIT 1`）
  - [ ] 2.2 在 `src-tauri/src/db/mod.rs` 添加 `pub mod briefings;`
  - [ ] 2.3 新建 `src-tauri/src/models/briefing.rs`：
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Briefing {
        pub id: String,
        pub content: String,
        pub date: String,
        pub created_at: String,
    }
    ```
  - [ ] 2.4 在 `src-tauri/src/models/mod.rs` 添加 `pub mod briefing;`

- [ ] **Task 3: Rust service 层 — `briefing_generator` 服务** (AC: #1, #2, #8, #10)
  - [ ] 3.1 新建 `src-tauri/src/services/briefing_generator.rs`：
    - 常量：`LLM_TIMEOUT_SECS = 30`（简报生成比单条建议更复杂，给更多时间）、`BRIEFING_TIME_KEY = "briefing_time"`（app_settings key）、`DEFAULT_BRIEFING_TIME = "08:00"`
    - `pub async fn generate_briefing_if_needed(pool: &SqlitePool, conv_pool: &ConversationsPool, app_handle: Option<&AppHandle>) -> Result<bool, AppError>` — 主入口函数，返回 `true` 表示生成了新简报，`false` 表示跳过（当天已有）
  - [ ] 3.2 在 `src-tauri/src/services/mod.rs` 添加 `pub mod briefing_generator;`（按字母序插入 `agent_config` 和 `dashboard_service` 之间）
  - [ ] 3.3 实现 `generate_briefing_if_needed` 函数逻辑：
    1. 获取当前本地日期 `YYYY-MM-DD`：`chrono::Local::now().date_naive().format("%Y-%m-%d").to_string()`
    2. 调用 `db::briefings::get_briefing_by_date(pool, &today)` — 已有则返回 `Ok(false)`
    3. 收集简报数据（见 3.4）
    4. 构造简报 prompt（见 3.5）
    5. 调用 `agent_engine::resolve_default_provider(pool)` → `call_llm(provider, prompt)` → 获取自然语言段落
    6. 写入 `briefings` 表：`db::briefings::create_briefing(pool, &content, &today)`
    7. 写入管家对话消息：`db::conversations::get_or_create_butler_conversation(conv_pool)` → `db::conversations::insert_message(conv_pool, &conv.id, "assistant", &content, true)`
    8. emit Tauri Event `briefing:generated`（payload: `{ briefingId, date }`）
    9. LLM 超时/错误/解析失败 → `tracing::warn!` + 返回 `Ok(false)`（不阻塞调度器）
  - [ ] 3.4 实现 `collect_briefing_data` 函数：
    - 调用 `dashboard_service::get_dashboard_status(pool, conv_pool)` 获取各角色状态（能量值 + pending 任务数 + 紧急任务标记）
    - 查询今日截止任务：新建 `db::tasks::list_tasks_due_today(pool, today_date) -> Result<Vec<Task>, AppError>`，SQL: `SELECT ... FROM tasks WHERE deadline = ?1 AND is_completed = 0 AND deleted_at IS NULL ORDER BY quadrant ASC`（deadline 存储为 `YYYY-MM-DD` 格式）
    - 查询昨日新增记忆：`db::memories::list_all_memories_with_options(pool, None, Some(50), None)` 然后在 Rust 侧按 `created_at` 过滤昨天的记录（或新建 DB 查询函数 `list_memories_since(pool, since_iso) -> Result<Vec<Memory>, AppError>`）
    - 查询未读耳语通知：`db::notifications::list_notifications(pool)` 然后在 Rust 侧筛选 `level == "whisper" && !is_read`（或新建 DB 查询函数 `list_unread_whispers(pool) -> Result<Vec<NotificationWithRole>, AppError>`）
  - [ ] 3.5 实现 `build_briefing_prompt` 函数：
    - 输入：角色状态列表 + 今日截止任务列表 + 昨日记忆列表 + 耳语通知列表 + 当前日期
    - System message：说明 EgoSync 管家角色，要求生成自然语言晨间简报（中文，温暖但简洁的语气），包含：各角色状态概览、今日截止任务提醒、昨日新增记忆要点、耳语通知汇总，结尾识别"今天最重要的一件事"
    - 输出要求：自然语言段落（非 JSON），约 200-400 字中文
    - 参照 `suggestion_generator.rs:1-159` 的 `build_suggestion_prompt` 格式风格
  - [ ] 3.6 实现 `call_llm` 函数（复制 `suggestion_generator.rs:299-350` 的实现，调整超时常量和函数名）：
    - 使用 `mpsc::channel::<StreamEvent>(128)` + `tokio::spawn` + `timeout(Duration::from_secs(LLM_TIMEOUT_SECS), ...)`
    - `ChatOptions { disable_thinking: true, tools: None, tool_choice: None }`
    - 收集所有 `StreamEvent::Token` 到 `String`，遇到 `Done` 返回
  - [ ] 3.7 实现 `get_briefing_time` 函数：
    - 从 `app_settings` 读取 `briefing_time` key，无则返回 `DEFAULT_BRIEFING_TIME`
    - `pub async fn get_briefing_time(pool: &SqlitePool) -> Result<String, AppError>`

- [ ] **Task 4: Rust command 层 — `briefing` Tauri commands** (AC: #2, #6, #8)
  - [ ] 4.1 新建 `src-tauri/src/commands/briefing.rs`：
    ```rust
    #[tauri::command]
    pub async fn briefing_get_latest(
        pool: State<'_, DbPool>,
    ) -> Result<Option<Briefing>, AppError> {
        db::briefings::get_latest_briefing(&pool).await
    }
    ```
  - [ ] 4.2 新增 `briefing_generate_now` command（手动触发简报生成，用于前端"立即生成"按钮或调试）：
    ```rust
    #[tauri::command]
    pub async fn briefing_generate_now(
        pool: State<'_, DbPool>,
        conv_pool: State<'_, ConversationsPool>,
        app_handle: tauri::AppHandle,
    ) -> Result<bool, AppError> {
        services::briefing_generator::generate_briefing_if_needed(
            &pool, &conv_pool, Some(&app_handle),
        ).await
    }
    ```
  - [ ] 4.3 在 `src-tauri/src/commands/mod.rs` 添加 `pub mod briefing;`（若 mod.rs 不存在，检查 lib.rs 中的模块声明方式 — 现有代码在 lib.rs 顶部使用 `mod commands;` 且 commands/mod.rs 声明子模块）
  - [ ] 4.4 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册 `commands::briefing::briefing_get_latest` 和 `commands::briefing::briefing_generate_now`（插入在 `commands::mission::*` 之后）

- [ ] **Task 5: 调度器集成 — 简报触发** (AC: #1, #5, #10)
  - [ ] 5.1 在 `scheduler.rs` 的 `spawn_scheduler` 函数的 tick 循环中，**在角色工作循环之后、Q2 提醒检查之前**，新增简报触发检查：
    ```rust
    // Story 6.1: 晨间简报触发检查（独立于角色 proactivity）
    let briefing_time = match briefing_generator::get_briefing_time(&pool).await {
        Ok(time) => time,
        Err(e) => {
            tracing::warn!(error = %e, "读取简报时间配置失败（降级跳过）");
            interval.tick().await;
            continue;
        }
    };
    if current_hhmm == briefing_time {
        // 内存去重：同一本地日期只触发一次
        let today_key = now_local.date_naive().format("%Y-%m-%d").to_string();
        if last_briefing_trigger_date.as_ref() != Some(&today_key) {
            last_briefing_trigger_date = Some(today_key.clone());
            let pool_clone = pool.clone();
            let conv_pool_clone = conv_pool.clone();
            let handle_clone = app_handle.clone();
            tokio::spawn(async move {
                match briefing_generator::generate_briefing_if_needed(
                    &pool_clone, &conv_pool_clone, Some(&handle_clone),
                ).await {
                    Ok(true) => tracing::info!("晨间简报已生成"),
                    Ok(false) => tracing::info!("晨间简报跳过（当天已有）"),
                    Err(e) => tracing::warn!(error = %e, "晨间简报生成失败"),
                }
            });
        }
    }
    ```
  - [ ] 5.2 在 `spawn_scheduler` 函数体内、`loop` 之前声明 `let mut last_briefing_trigger_date: Option<String> = None;`
  - [ ] 5.3 **注意**：`briefings` 表的 `UNIQUE(date)` 约束是去重的最终保障，内存去重键只是避免重复触发 LLM 调用（性能优化）。即使内存去重失效（如应用重启后同一天再次到达简报时间），DB 约束会阻止重复写入，`generate_briefing_if_needed` 函数内部的 `get_briefing_by_date` 检查会跳过生成

- [ ] **Task 6: 前端类型定义** (AC: #9)
  - [ ] 6.1 新建 `src/types/briefing.ts`：
    ```typescript
    export interface Briefing {
      id: string;
      content: string;
      date: string;
      createdAt: string;
    }
    export interface BriefingGeneratedPayload {
      briefingId: string;
      date: string;
    }
    ```

- [ ] **Task 7: 前端 service 层** (AC: #9)
  - [ ] 7.1 新建 `src/services/briefingService.ts`：
    ```typescript
    import { invoke } from '@tauri-apps/api/core';
    import type { Briefing } from '../types/briefing';

    export const briefingService = {
      getLatest: () => invoke<Briefing | null>('briefing_get_latest'),
      generateNow: () => invoke<boolean>('briefing_generate_now'),
    };
    ```

- [ ] **Task 8: 前端事件监听 — `briefing:generated` 刷新管家对话** (AC: #3, #6, #9)
  - [ ] 8.1 在 `src/App.tsx` 中，**在现有 `useTauriEvent('q2:reminder')` 之后**，新增 `useTauriEvent('briefing:generated')` 监听：
    ```typescript
    useTauriEvent<BriefingGeneratedPayload>(
      'briefing:generated',
      useCallback((_payload: BriefingGeneratedPayload) => {
        setButlerChatRefreshTrigger(t => t + 1);
      }, []),
      []
    );
    ```
  - [ ] 8.2 导入 `BriefingGeneratedPayload` 类型
  - [ ] 8.3 **不需要新增组件** — `butlerChatRefreshTrigger` 已通过 `ButlerView` 的 `chatRefreshTrigger` prop 传入 `ChatStream`，`ChatStream` 的 `refreshTrigger` useEffect 会自动重新加载管家对话历史，新简报消息自然出现在对话流中

- [ ] **Task 9: 单元测试** (AC: #1, #2, #5, #7, #8)
  - [ ] 9.1 Rust DB 测试 — 在 `db/briefings.rs` 添加 `#[cfg(test)] mod tests`：
    - `create_briefing_inserts_and_returns` — 正常插入
    - `get_briefing_by_date_returns_existing` — 按日期查询
    - `get_briefing_by_date_returns_none_when_missing` — 不存在时返回 None
    - `get_latest_briefing_returns_most_recent` — 最新简报查询
    - `create_briefing_duplicate_date_fails` — UNIQUE 约束生效
  - [ ] 9.2 Rust service 测试 — 在 `briefing_generator.rs` 添加 `#[cfg(test)] mod tests`：
    - `build_briefing_prompt_contains_role_status` — prompt 包含角色状态
    - `build_briefing_prompt_contains_due_tasks` — prompt 包含今日截止任务
    - `build_briefing_prompt_contains_memories` — prompt 包含昨日记忆
    - `build_briefing_prompt_contains_whisper_notifications` — prompt 包含耳语通知
    - `build_briefing_prompt_contains_date` — prompt 包含当前日期
    - `build_briefing_prompt_contains_most_important_thing_instruction` — prompt 包含"今天最重要的一件事"指令
  - [ ] 9.3 前端 service 测试 — 验证 `briefingService.getLatest()` 和 `briefingService.generateNow()` 的 invoke 调用参数正确

## Dev Notes

### 关键技术决策

- **简报时间配置**：本 story 使用 `app_settings` 表 key `briefing_time`（默认 `08:00`），与调度器的 `scheduler.moderate_times` / `scheduler.proactive_times` 独立。时间配置 UI 是 Story 6.2 的职责，本 story 只读取配置（无配置时用默认值），不提供配置 UI
- **简报触发独立于角色 proactivity**：简报是全局功能，不依赖任何角色的 proactivity_level。调度器 tick 循环中新增独立的简报时间检查，与角色工作循环并行
- **双 Pool 访问**：简报生成需同时访问主 DB（角色/任务/记忆/通知/简报）和对话 DB（管家对话消息）。`generate_briefing_if_needed` 需注入 `&SqlitePool` 和 `&ConversationsPool`
- **简报数据收集策略**：
  - 角色状态：直接调用 `dashboard_service::get_dashboard_status(pool, conv_pool)` — 已实现角色状态聚合 + 排序，返回 `Vec<DashboardStatus>`（含 role_id, role_name, energy, pending_tasks_count, has_urgent）
  - 今日截止任务：需新建 DB 查询函数 `list_tasks_due_today(pool, today_date)`。deadline 列存储格式为 `YYYY-MM-DD`（纯日期，非 ISO 8601），因此用 `deadline = ?1` 精确匹配
  - 昨日记忆：`list_all_memories_with_options(pool, None, Some(50), None)` 返回最近 50 条记忆（排除 task_status），在 Rust 侧按 `created_at` 的日期部分过滤为昨天的记录。若 50 条不够覆盖昨天，可增大 limit 或新建 `list_memories_by_date_range(pool, start_iso, end_iso)` 查询函数
  - 耳语通知：`list_notifications(pool)` 返回全部通知，在 Rust 侧筛选 `level == "whisper" && !is_read`。或新建 `list_unread_whispers(pool)` 查询函数
- **LLM 降级策略**：LLM 超时/错误/解析失败时不写入空简报，`tracing::warn!` 记录错误，返回 `Ok(false)`。这与 `suggestion_generator` 的降级策略一致（不阻塞调度器）
- **`call_llm` 复用**：`suggestion_generator.rs:299-350` 中的 `call_llm` 是 private 函数。推荐方案 A（复制实现），因为：(1) 简报生成的 prompt 和 response 处理与建议生成不同；(2) 避免修改已测试通过的 `suggestion_generator.rs`；(3) 代码量小（约 50 行）。超时常量调整为 30 秒（简报比单条建议更复杂）
- **简报 prompt 设计**：prompt 需包含足够的上下文让 LLM 生成有意义简报，但不能过长（token 限制）。角色状态取全部活跃角色（通常 3-8 个），今日截止任务取全部（通常 0-5 个），昨日记忆取最近 20 条，耳语通知取全部未读（通常 0-10 条）。prompt 要求输出自然语言段落（非 JSON），约 200-400 字中文
- **"今天最重要的一件事"识别**：在 prompt 中明确要求 LLM 从今日截止任务或最高能量角色的首要任务中识别一件事，作为简报结尾的建议段落。这不是独立的 suggestion 记录，而是简报文本的一部分
- **简报写入管家对话**：简报内容作为 `role="assistant"` 的完整消息写入管家对话（`is_complete=true`），与 Q2 提醒的写入模式完全一致。前端 `ChatStream` 通过 `refreshTrigger` 重新加载对话历史时自然显示
- **去重机制**：三层去重保障：(1) 调度器内存去重键 `last_briefing_trigger_date`（避免同一天重复触发 LLM）；(2) `generate_briefing_if_needed` 函数内部检查 `get_briefing_by_date`（应用重启后仍生效）；(3) `briefings` 表 `UNIQUE(date)` 约束（最终 DB 层保障）
- **简报日期格式**：`date` 列存储本地日期 `YYYY-MM-DD`（非 UTC ISO 8601），因为简报按用户本地日期去重。`created_at` 列仍使用 UTC ISO 8601（与全应用约定一致）

### 架构合规

- **分层规则**：前端 → `briefingService.getLatest()` / `briefingService.generateNow()` → `invoke('briefing_get_latest')` / `invoke('briefing_generate_now')` → `commands/briefing.rs`（薄层解析）→ `services/briefing_generator.rs`（业务逻辑 + LLM 调用）→ `db/briefings.rs`（SQL 执行）→ SQLite
- **services/ 层职责**：`briefing_generator` 拥有所有简报业务逻辑（数据收集、prompt 构造、LLM 调用、简报写入、事件 emit）。command 层只做参数校验 + 调 service + 返回结果
- **调度器集成**：`scheduler.rs` 的 `spawn_scheduler` tick 循环中新增简报触发检查，与角色工作循环和 Q2 提醒检查并行。简报生成独立 `tokio::spawn`，不阻塞调度器 tick
- **命名规范**：Rust command 使用 `snake_case`（`briefing_get_latest`, `briefing_generate_now`），前端 service 使用 `camelCase`（`briefingService.getLatest()`, `briefingService.generateNow()`）
- **错误处理**：command 返回 `Result<Option<Briefing>, AppError>` 或 `Result<bool, AppError>`，前端 try-catch。简报生成失败返回 `Ok(false)` 而非 `Err(...)`，因为"当天已有简报"是正常业务状态而非错误
- **serde 桥接**：Rust `Briefing` 需派生 `serde::Serialize` + `#[serde(rename_all = "camelCase")]`，前端 `Briefing` interface 使用 camelCase
- **事件命名**：`briefing:generated`（遵循 `{domain}:{verb_past}` 规范），payload `{ briefingId, date }`

### 前端 UI 规范

- **不新增 UI 组件**：简报通过管家对话消息自然呈现，复用 `ChatStream` 的消息渲染。`ActionCard` 由 `useSuggestions` hook 自动获取 pending 建议并渲染（Epic 4 已建成）
- **事件监听**：`App.tsx` 新增 `useTauriEvent('briefing:generated')`，回调中递增 `butlerChatRefreshTrigger`（复用现有 Q2 提醒的 refresh 机制）
- **简报时间前的行为**：用户在简报时间前打开应用，`ChatStream` 初始化时加载管家对话历史，历史中包含昨天的简报消息（如果有），自然显示"上次简报内容"。到达简报时间后，`briefing:generated` 事件触发 `ChatStream` 刷新，新简报追加到对话流末尾

### 反模式警告

- **不要**在 `suggestion_generator.rs` 中提取 `call_llm` 为 pub — 推荐在 `briefing_generator.rs` 中复制实现，保持模块独立性
- **不要**在 `commands/` 层添加业务逻辑 — `briefing_generate_now` command 只做调 service + 返回结果
- **不要**在简报生成中创建 suggestion 记录 — "今天最重要的一件事"是简报文本的一部分，不是独立的 suggestion。pending suggestions 由角色工作循环独立生成（Epic 4 已建成）
- **不要**在简报生成中创建通知 — 简报本身就是一种通知形式（通过管家对话消息呈现），不需要额外创建 notification 记录
- **不要**将简报时间配置 UI 放在本 story — 时间配置 UI 是 Story 6.2 的职责
- **不要**在简报 prompt 中要求 JSON 输出 — 简报是自然语言段落，不是结构化数据
- **不要**忽略 `ConversationsPool` — 简报需写入管家对话消息，需要同时注入 `DbPool` 和 `ConversationsPool`
- **不要**在调度器中同步调用简报生成 — 必须 `tokio::spawn` 异步执行，不阻塞 60 秒 tick 循环
- **不要**将简报日期存储为 UTC ISO 8601 — 使用本地日期 `YYYY-MM-DD`，因为简报按用户本地日期去重
- **不要**在 `briefings` 表中存储 `role_id` — 简报是全局的，不属于任何角色

### Project Structure Notes

新增文件：
- `src-tauri/migrations/023_briefings.sql` — briefings 表迁移
- `src-tauri/src/db/briefings.rs` — 简报 DB 操作
- `src-tauri/src/models/briefing.rs` — Briefing struct
- `src-tauri/src/services/briefing_generator.rs` — 简报生成 LLM 服务
- `src-tauri/src/commands/briefing.rs` — Tauri command 层
- `src/types/briefing.ts` — 前端 Briefing 类型
- `src/services/briefingService.ts` — 前端 service 层

修改文件：
- `src-tauri/src/db/mod.rs` — 添加 `pub mod briefings;`
- `src-tauri/src/models/mod.rs` — 添加 `pub mod briefing;`
- `src-tauri/src/services/mod.rs` — 添加 `pub mod briefing_generator;`
- `src-tauri/src/commands/mod.rs` — 添加 `pub mod briefing;`（确认 mod.rs 存在）
- `src-tauri/src/lib.rs` — 注册 `commands::briefing::briefing_get_latest` 和 `commands::briefing::briefing_generate_now`
- `src-tauri/src/services/scheduler.rs` — `spawn_scheduler` tick 循环中新增简报触发检查
- `src/App.tsx` — 新增 `useTauriEvent('briefing:generated')` 监听
- `src-tauri/src/db/tasks.rs` — 新增 `list_tasks_due_today` 查询函数（若需要）
- `src-tauri/src/db/notifications.rs` — 新增 `list_unread_whispers` 查询函数（若需要）

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.1] — AC 原文
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping] — FR-16~18 → `briefing.rs` + `briefings` DB 表
- [Source: _bmad-output/planning-artifacts/architecture.md#Layer Rules] — 分层规则
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Boundaries] — 主 DB vs 对话 DB 的访问规则
- [Source: _bmad-output/project-context.md] — 技术栈、命名规范、错误处理、禁止事项
- [Source: src-tauri/src/services/suggestion_generator.rs:299-350] — LLM 调用模式参考（call_llm, timeout, StreamEvent 收集）
- [Source: src-tauri/src/services/agent_engine.rs:1928-1952] — resolve_default_provider 函数
- [Source: src-tauri/src/services/q2_protection_reminder.rs:167-193] — 管家对话消息写入模式参考
- [Source: src-tauri/src/services/dashboard_service.rs:10-71] — 角色状态聚合函数
- [Source: src-tauri/src/services/scheduler.rs:319-428] — spawn_scheduler tick 循环结构
- [Source: src-tauri/src/db/conversations.rs:32-88] — get_or_create_butler_conversation + insert_message
- [Source: src-tauri/src/db/app_settings.rs] — get_setting / set_setting
- [Source: src/App.tsx:82-90] — butlerChatRefreshTrigger + useTauriEvent 模式
- [Source: src/components/chat/ChatStream.tsx:641-652] — refreshTrigger useEffect
- [Source: src/components/butler/ButlerView.tsx:130-144] — ChatStream + refreshTrigger + onConversationIdChange
- [Source: src/hooks/useSuggestions.ts] — 建议管理 hook（自动渲染 ActionCard）

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

### Review Findings

#### Code Review 2026-06-25

- [x] [Review][Patch] 昨日记忆过滤时区错配 [egosync-app/src-tauri/src/services/briefing_generator.rs:184-194] — 已修复：`yesterday` 改用 `chrono::Utc::now()` 计算，与 UTC 存储的 `memories.created_at` 一致。`cargo check` 通过，18 个简报单测全绿。
- [x] [Review][Defer] briefing_time 无格式校验，错误配置导致永久不生成 [egosync-app/src-tauri/src/services/briefing_generator.rs:154] — deferred，配置入口属 Story 6.2，当前仅走默认值 `08:00`，无实际风险
- [x] [Review][Defer] 内存去重键在 spawn 前置，瞬时 LLM 失败当天不再重试 [egosync-app/src-tauri/src/services/scheduler.rs:431-432] — deferred，符合 AC10"同一天只触发一次"语义，避免 LLM 故障时每分钟重试；可改为仅成功时置位作为未来优化
- [x] [Review][Defer] tick 精确分钟匹配可能跳过简报触发 [egosync-app/src-tauri/src/services/scheduler.rs:431] — deferred，与现有角色调度器同一模式（`current_hhmm == ...`），系统休眠/负载导致整分钟漏 tick 时当天不触发，属既有设计
- [x] [Review][Defer] 50 条记忆窗口可能挤掉昨日记忆 [egosync-app/src-tauri/src/services/briefing_generator.rs:186-196] — deferred，Dev Notes 已知约束（line 301），重度用户当日记忆 >50 条时昨日记忆可能为空
