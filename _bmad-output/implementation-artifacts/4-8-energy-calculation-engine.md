---
baseline_commit: 92a316e7f554017b93e41e54462e9e59c95c391d
---

# Story 4.8: 能量值计算引擎

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 每个角色有一个动态变化的"能量值"反映角色健康度,
so that 我能直觉地感知哪个角色需要更多关注。

## 背景与现状（务必先读）

**本 story 是纯后端 story — 新建 `energy_calculator` 服务，在调度器工作循环完成后自动计算角色能量值并写入 `roles.energy` + `roles.energy_updated_at`。无新增前端组件、无新增 Tauri 命令（能量值已由 Story 4.7 仪表盘展示）。需新增 DB 迁移为 roles 表添加 `energy_updated_at` 列。**

### 已建成的基础（本 story 的接入点）

**roles 表 — 已有 `energy` 列但缺 `energy_updated_at`：**
- `migrations/003_roles.sql:10` — `energy INTEGER NOT NULL DEFAULT 100`（当前所有角色默认 100，从未被计算引擎更新过）
- `models/role.rs:1-17` — `Role` 结构体含 `energy: i32` 字段，但无 `energy_updated_at`
- `db/roles.rs:9` — `ROLE_SELECT_COLUMNS` 常量包含 `energy` 但不包含 `energy_updated_at`
- **epics.md 中 AC 写的是 `roles.energy_value`，但实际 DB 列名是 `roles.energy` — 以实际 DB 列名为准，不重命名**
- **本 story 需新增 migration `019_energy_updated_at.sql` 添加 `energy_updated_at TEXT` 列**

**调度器 — 能量计算的触发点：**
- `services/scheduler.rs:148-274` — `run_work_loop_for_role(pool, role, app_handle)` 在建议生成和通知写入完成后返回 `Ok(())`
- `services/scheduler.rs:160-173` — 先调用 `suggestion_generator::generate_suggestions`，再写入建议 + 通知
- `services/scheduler.rs:265-273` — 工作循环结束日志
- **本 story 需在 `run_work_loop_for_role` 返回前调用 `energy_calculator::calculate_and_update_energy`**

**任务数据 — 能量公式输入源：**
- `db/tasks.rs:51-53` — `list_tasks_by_role(pool, role_id)` 返回该角色所有未删除任务
- `db/tasks.rs:257-295` — `set_task_completion` 完成任务时设置 `is_big_rock = 0`（方案 D），**这意味着已完成的大石头不再保留 `is_big_rock = 1` 标记**
- `db/tasks.rs:492-510` — `list_at_risk_q2_tasks(pool)` 返回所有 `protection_status = 'at_risk'` 的 Q2 未完成任务（跨角色）
- `db/tasks.rs:314-333` — `count_big_rocks_by_owner(pool, owner_type, role_id)` 统计当前活跃大石头数量
- `tasks` 表 schema（`migrations/013_tasks.sql`）：`quadrant`、`is_completed`、`completed_at`、`is_big_rock`、`protection_status`、`deleted_at`、`owner_type`、`role_id`
- **本 story 需新增按角色维度的统计查询函数**

**对话数据 — recent_activity_score 输入源：**
- `db/conversations.rs:411-436` — `get_last_active_for_roles(pool, role_ids)` 批量查询每个角色的 `MAX(updated_at)`
- `ConversationsPool` — 独立于主 `SqlitePool`，需在 service 层注入
- **本 story 可复用此函数获取角色最近活跃时间，用于计算 `recent_activity_score`**

**通知服务 — 低能量通知：**
- `services/notification_service.rs:57-102` — `create_notification_for_role(pool, role_id, requested_level, content)` 创建通知（含降级逻辑）
- `services/suggestion_generator.rs` — `NotificationLevel::Tap` 对应"轻触"级通知
- `services/q2_protection_reminder.rs:127-165` — Q2 提醒创建通知的模式参考（先创建通知，失败则 warn 不阻塞）
- **本 story 需在能量值从 ≥ 40 降至 < 40 时生成"轻触"通知**

**仪表盘 — 能量值的展示端（已完成，不需修改）：**
- `services/dashboard_service.rs:46` — `energy: role.energy` 直接从 Role 结构体读取
- `commands/dashboard.rs` — `dashboard_get_status` 命令已注册
- 前端 `DashboardTab.tsx` — 已展示能量值百分比 + 进度条 + 色谱
- **本 story 计算并写入 `roles.energy` 后，仪表盘下次查询自动获取新值，无需前端改动**

## Acceptance Criteria

1. **AC1**: Given 调度器每次工作循环完成后，When 重新计算角色能量值，Then 新值写入 `roles.energy`（0-100 整数）+ `roles.energy_updated_at`（ISO 8601 时间戳）

2. **AC2**: Given 能量值计算公式（V1 线性加权，对应 PRD §8 Resolved Question 4），Then `energy = 0.4 * task_completion_rate + 0.3 * recent_activity_score + 0.2 * goal_progress + 0.1 * (100 - at_risk_penalty)`
> PRD 追溯：任务完成率(40%)=task_completion_rate / 大石头推进度(30%)≈goal_progress / 用户互动频率(20%)=recent_activity_score / 目标更新活跃度(10%)=(100-at_risk_penalty)
- And `task_completion_rate` = 最近 7 天完成任务数 / 总任务数 * 100（总任务数 = 未删除任务总数，包括已完成和未完成；若总任务数为 0，则 task_completion_rate = 0）
- And `recent_activity_score` = 基于最近对话时间衰减（今天=100, 昨天=80, 3天前=50, 7天+=10；若无对话记录则=0）
- And `goal_progress` = 角色目标相关大石头完成率 * 100（V1 简化：`(1 - active_big_rocks / 3) * 100`，clamped [0, 100]；0 个活跃大石头=100，3 个=0）
- And `at_risk_penalty` = at_risk 的 Q2 任务数 * 20（最高扣 60）

3. **AC3**: Given 能量值 < 40%，When 从 ≥ 40 下降到 < 40（跨越阈值），Then 生成"轻触"通知"你的XX角色能量值较低，可能需要关注"

4. **AC4**: Given 计算过程，Then 每次计算结果存入 `roles.energy` + `roles.energy_updated_at`，And tracing 日志记录计算明细（各子项值 + 最终能量值，便于调试和调参）

5. **AC5**: Given 用户手动操作后（完成任务、对话等），Then 不实时重算（等待下次调度循环），And V1 不追求实时性，允许延迟

6. **AC6**: Given Rust 后端，Then `energy_calculator` 服务：读取角色任务/对话数据 → 公式计算 → 更新 roles 表 → 必要时生成低能量通知

7. **AC7**: Given 数据库迁移，Then `migrations/019_energy_updated_at.sql` 为 `roles` 表添加 `energy_updated_at TEXT` 列（允许 NULL，首次计算后写入 ISO 8601 时间戳）

## Tasks / Subtasks

- [x] **Task 1: DB 迁移 — 添加 energy_updated_at 列** (AC: #7)
  - [x] 1.1 新建 `migrations/019_energy_updated_at.sql`：
    ```sql
    -- Story 4.8: 为 roles 表添加 energy_updated_at 列，记录能量值最后计算时间
    ALTER TABLE roles ADD COLUMN energy_updated_at TEXT;
    ```
  - [x] 1.2 验证 `sqlx::migrate!("./migrations")` 自动执行新迁移（`db/pool.rs:44-51` 已配置）

- [x] **Task 2: Rust model 层 — Role 结构体更新** (AC: #1, #7)
  - [x] 2.1 在 `models/role.rs` 的 `Role` 结构体添加 `pub energy_updated_at: Option<String>` 字段（在 `energy` 之后）
  - [x] 2.2 确认 `#[serde(rename_all = "camelCase")]` 自动将字段序列化为 `energyUpdatedAt`（前端兼容）
  - [x] 2.3 确认 `#[derive(sqlx::FromRow)]` 自动从 DB 行映射新字段

- [x] **Task 3: Rust DB 层 — roles 表查询与更新** (AC: #1, #7)
  - [x] 3.1 在 `db/roles.rs` 更新 `ROLE_SELECT_COLUMNS` 常量，追加 `energy_updated_at`：
    ```rust
    const ROLE_SELECT_COLUMNS: &str = "id, name, icon, color, goal, personality_prompt, status, energy, energy_updated_at, skills_config, proactivity_level, archived_at, created_at, updated_at";
    ```
  - [x] 3.2 在 `db/roles.rs` 新增 `update_energy(pool, role_id, energy, energy_updated_at) -> Result<(), AppError>` 函数：
    ```rust
    pub async fn update_energy(
        pool: &SqlitePool,
        role_id: &str,
        energy: i32,
        energy_updated_at: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE roles SET energy = ?1, energy_updated_at = ?2 WHERE id = ?3",
        )
        .bind(energy)
        .bind(energy_updated_at)
        .bind(role_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新能量值失败: {}", e)))?;
        if result.rows_affected() != 1 {
            return Err(AppError::NotFound(format!("角色 {} 不存在", role_id)));
        }
        Ok(())
    }
    ```
  - [x] 3.3 更新 `db/roles.rs` 测试中的 `setup_test_db` CREATE TABLE 语句，追加 `energy_updated_at TEXT` 列
  - [x] 3.4 更新 `db/roles.rs` 测试中 `run_work_loop_for_role_returns_ok_without_provider` 的 `scheduler.rs` 测试 Role 构造，追加 `energy_updated_at: None`

- [x] **Task 4: Rust DB 层 — 任务统计查询函数** (AC: #2)
  - [x] 4.1 在 `db/tasks.rs` 新增 `count_completed_tasks_in_last_7_days_for_role(pool, role_id) -> Result<i64, AppError>`：
    ```rust
    pub async fn count_completed_tasks_in_last_7_days_for_role(
        pool: &SqlitePool,
        role_id: &str,
    ) -> Result<i64, AppError> {
        let threshold = compute_7_days_ago_threshold();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks
             WHERE owner_type = 'role' AND role_id = ?1
               AND is_completed = 1 AND completed_at IS NOT NULL
               AND completed_at >= ?2
               AND deleted_at IS NULL",
        )
        .bind(role_id)
        .bind(&threshold)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("查询最近7天完成任务数失败: {}", e)))?;
        Ok(count)
    }
    ```
  - [x] 4.2 在 `db/tasks.rs` 新增 `count_total_tasks_for_role(pool, role_id) -> Result<i64, AppError>`：
    ```rust
    pub async fn count_total_tasks_for_role(
        pool: &SqlitePool,
        role_id: &str,
    ) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks
             WHERE owner_type = 'role' AND role_id = ?1
               AND deleted_at IS NULL",
        )
        .bind(role_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("查询角色任务总数失败: {}", e)))?;
        Ok(count)
    }
    ```
  - [x] 4.3 在 `db/tasks.rs` 新增 `count_at_risk_q2_tasks_for_role(pool, role_id) -> Result<i64, AppError>`：
    ```rust
    pub async fn count_at_risk_q2_tasks_for_role(
        pool: &SqlitePool,
        role_id: &str,
    ) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks
             WHERE owner_type = 'role' AND role_id = ?1
               AND protection_status = 'at_risk'
               AND quadrant = 'Q2'
               AND is_completed = 0
               AND deleted_at IS NULL",
        )
        .bind(role_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("查询 at_risk Q2 任务数失败: {}", e)))?;
        Ok(count)
    }
    ```
  - [x] 4.4 在 `db/tasks.rs` 新增 `count_active_big_rocks_for_role(pool, role_id) -> Result<i64, AppError>`：
    ```rust
    pub async fn count_active_big_rocks_for_role(
        pool: &SqlitePool,
        role_id: &str,
    ) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks
             WHERE owner_type = 'role' AND role_id = ?1
               AND is_big_rock = 1
               AND is_completed = 0
               AND deleted_at IS NULL",
        )
        .bind(role_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("查询活跃大石头数量失败: {}", e)))?;
        Ok(count)
    }
    ```
  - [x] 4.5 在 `db/tasks.rs` 新增私有辅助函数 `compute_7_days_ago_threshold() -> String`，返回 `now - 7 days` 的 ISO 8601 时间戳（参考 `services/task_protection_watch.rs:38-56` 的 `compute_at_risk_threshold` 模式）

- [x] **Task 5: Rust service 层 — energy_calculator 服务** (AC: #1, #2, #3, #4, #6)
  - [x] 5.1 新建 `services/energy_calculator.rs`
  - [x] 5.2 在 `services/mod.rs` 注册 `energy_calculator` 模块
  - [x] 5.3 定义能量计算公式常量：
    ```rust
    const WEIGHT_TASK_COMPLETION: f64 = 0.4;
    const WEIGHT_RECENT_ACTIVITY: f64 = 0.3;
    const WEIGHT_GOAL_PROGRESS: f64 = 0.2;
    const WEIGHT_AT_RISK: f64 = 0.1;
    const LOW_ENERGY_THRESHOLD: i32 = 40;
    const AT_RISK_PENALTY_PER_TASK: i32 = 20;
    const MAX_AT_RISK_PENALTY: i32 = 60;
    const MAX_BIG_ROCKS: i32 = 3;
    ```
  - [x] 5.4 实现 `calculate_and_update_energy(pool: &SqlitePool, conv_pool: &ConversationsPool, role: &Role) -> Result<(), AppError>`：
    - 读取旧能量值 `old_energy = role.energy`
    - 调用 `db::tasks::count_completed_tasks_in_last_7_days_for_role(pool, &role.id)` 获取最近 7 天完成数
    - 调用 `db::tasks::count_total_tasks_for_role(pool, &role.id)` 获取总任务数
    - 计算 `task_completion_rate`：若总任务数 = 0 则 = 0，否则 = completed_7d / total * 100
    - 调用 `db::conversations::get_last_active_for_roles(conv_pool, &[role.id.clone()])` 获取最近活跃时间
    - 计算 `recent_activity_score`：基于 `last_active_at` 时间衰减（今天=100, 昨天=80, 3天前=50, 7天+=10, 无记录=0）
    - 调用 `db::tasks::count_active_big_rocks_for_role(pool, &role.id)` 获取活跃大石头数
    - 计算 `goal_progress` = `(1 - active_br / MAX_BIG_ROCKS) * 100`，clamped [0, 100]
    - 调用 `db::tasks::count_at_risk_q2_tasks_for_role(pool, &role.id)` 获取 at_risk Q2 任务数
    - 计算 `at_risk_penalty` = min(at_risk_count * 20, 60)
    - 计算 `energy = 0.4 * task_completion_rate + 0.3 * recent_activity_score + 0.2 * goal_progress + 0.1 * (100 - at_risk_penalty)`
    - 四舍五入为整数：`energy = energy.round() as i32`，clamped [0, 100]
    - tracing 日志记录计算明细（各子项值 + 最终能量值）
    - 调用 `db::roles::update_energy(pool, &role.id, energy, &now)` 写入 DB
    - 若 `old_energy >= 40 && energy < 40`：调用 `notification_service::create_notification_for_role(pool, &role.id, NotificationLevel::Tap, &format!("你的{}角色能量值较低，可能需要关注", role.name))`，通知创建失败只 warn 不阻塞
    - Ok(())
  - [x] 5.5 实现 `recent_activity_score_from_last_active(last_active_at: Option<&str>) -> f64`：
    - None → 0.0
    - 解析 ISO 8601 时间戳，计算距今天数
    - 0 天（今天）→ 100.0
    - 1 天（昨天）→ 80.0
    - 2 天 → 65.0
    - 3 天 → 50.0
    - 4-6 天 → 30.0
    - ≥ 7 天 → 10.0
    - 解析失败 → 0.0

- [x] **Task 6: Rust service 层 — 调度器集成** (AC: #1, #5, #6)
  - [x] 6.1 在 `services/scheduler.rs` 的 `run_work_loop_for_role` 函数中，在建议写入完成日志之后、`Ok(())` 返回之前，添加能量计算调用：
    ```rust
    // Story 4.8: 工作循环完成后重新计算角色能量值
    if let Err(e) = crate::services::energy_calculator::calculate_and_update_energy(
        pool, conv_pool, role,
    ).await {
        tracing::warn!(
            role_id = %role.id,
            role_name = %role.name,
            error = %e,
            "能量值计算失败（不影响工作循环结果）"
        );
    }
    ```
  - [x] 6.2 修改 `run_work_loop_for_role` 函数签名，添加 `conv_pool: &ConversationsPool` 参数（能量计算需要查询对话数据）
  - [x] 6.3 在 `spawn_scheduler` 函数中 `run_work_loop_for_role` 调用处传入 `&conv_pool`（`scheduler.rs:349`）
  - [x] 6.4 在 `scheduler.rs` 测试 `run_work_loop_for_role_returns_ok_without_provider` 中添加 `conv_pool` 参数（需创建测试 ConversationsPool）

- [x] **Task 7: Rust 单元测试** (AC: #1-#6)
  - [x] 7.1 `services/energy_calculator.rs` 测试：
    - `recent_activity_score_from_last_active`：今天=100 / 昨天=80 / 3天=50 / 7天=10 / None=0 / 无效字符串=0
    - `calculate_and_update_energy`：无任务无对话 → 能量值合理（task_completion=0, activity=0, goal_progress=100, at_risk_penalty=0 → energy = 0.2*100 + 0.1*100 = 30）
    - 有完成任务 → task_completion_rate 正确计算
    - 有 at_risk Q2 任务 → at_risk_penalty 正确扣减
    - 能量值从 ≥ 40 降至 < 40 → 生成"轻触"通知
    - 能量值持续 < 40 → 不重复生成通知（仅跨越阈值时通知）
    - 能量值 clamp [0, 100]
  - [x] 7.2 `db/tasks.rs` 测试：
    - `count_completed_tasks_in_last_7_days_for_role`：正确统计 7 天内完成 / 排除 7 天前完成 / 排除未完成 / 排除已删除
    - `count_total_tasks_for_role`：正确统计 / 排除已删除
    - `count_at_risk_q2_tasks_for_role`：正确统计 at_risk Q2 / 排除非 at_risk / 排除非 Q2 / 排除已完成
    - `count_active_big_rocks_for_role`：正确统计活跃大石头 / 排除已完成 / 排除已删除
  - [x] 7.3 `db/roles.rs` 测试：
    - `update_energy`：正确写入 energy + energy_updated_at / 角色不存在返回 NotFound
  - [x] 7.4 更新 `services/scheduler.rs` 测试中 `run_work_loop_for_role_returns_ok_without_provider`：追加 `conv_pool` 参数 + `energy_updated_at: None` 字段

- [x] **Task 8: 更新 sprint-status.yaml**
  - [x] 8.1 将 `4-8-energy-calculation-engine` 状态更新为 `review`

### Review Findings

_代码审查 (2026-06-23) — 审查范围：未提交改动 / 模式：full_

- [x] [Review][Patch] `recent_activity_score` 时间衰减改为本地日历日期差（决策已定：按 spec）— ✅ 已修复[egosync-app/src-tauri/src/services/energy_calculator.rs:40-62] — 当前用 `(now - utc_time).num_days()`（瞬时 24h 间隔），需改为 spec Task 5.5 / Dev Notes 规定的 `parse_iso_to_local_date` + `(now_local.date_naive() - last_active_date).num_days()`（本地日历日期差）。修复后需相应更新/补充单测以覆盖跨午夜边界。
- [x] [Review][Patch] `insert_energy_test_task` 测试夹具 INSERT 列/值错位且占位符与绑定数不匹配 — ✅ 已修复 [egosync-app/src-tauri/src/db/tasks.rs:1943-1957] — 列首 `id` 被写死字面量 `'role'`（每行同 id → 第二次插入触发主键冲突），`owner_type/role_id/title/quadrant/...` 整体右移错位，且 SQL 有 9 个占位符 `?1..?9` 但只 `.bind` 了 8 个值（`?9` 无绑定）。Story 4.8 的 4 个 DB 统计测试运行时必然 panic。
- [x] [Review][Patch] 测试名与断言/spec 示例不一致 — ✅ 已修复（重命名为 `..._returns_30` + 修正 spec 示例） [egosync-app/src-tauri/src/services/energy_calculator.rs:320-339] — 函数名 `energy_no_tasks_no_conversation_returns_20` 实际断言 `energy == 30`；spec AC 示例写 energy=20 系漏算 `0.1*(100-0)=10` 项。代码结果 30 正确，建议重命名测试 + 修正 spec 示例注释。
- [x] [Review][Patch] 提交代码编译失败：测试中对 `ConversationsPool` 直接执行 SQL — ✅ 已修复 [energy_calculator.rs:527 / scheduler.rs:688] — `sqlx::query(...).execute(&conv_pool)` 中 `&ConversationsPool` 不满足 `Executor`（newtype 包装，只有内层 `&SqlitePool` 是）。导致 `cargo test` 6 个 E0277 编译错误，Story 4.8 提交代码从未编译通过。修复：两处 `.execute(&conv_pool)` → `.execute(&*conv_pool)`（解引用到内层连接池）。
- [x] [Review][Dismiss] ~~`compute_7_days_ago_threshold` 手写历法换算疑似不一致~~ — 误报，已驳回。复查发现 `days_to_ymd` + 手写 epoch 阈值计算是全代码库既定模式（`settings.rs` / `llm_config.rs` / `task_deadline_watch.rs` / `task_protection_watch.rs` 均同此实现），且 spec Dev Notes 明确要求参照 `task_protection_watch.rs::compute_at_risk_threshold`。本实现遵循约定，功能正确，不修改。

_代码审查 (2026-06-23 第二轮) — 审查范围：未提交改动 / 模式：full / 三层并行（Blind / Edge / Auditor）/ 16+4 单测全绿、lib 编译干净_

- [x] [Review][Decision→Patch] 新建/低活跃角色首次能量计算会从默认 100 跌至 30 并误发"能量偏低"通知 — ✅ 已修复 [egosync-app/src-tauri/src/services/energy_calculator.rs:134-137]。决策：视为缺陷修复。方案：首次计算（`role.energy_updated_at.is_none()`）只建立能量基线、跳过低能量通知；后续计算维持原阈值跨越逻辑。新增单测 `energy_first_calculation_skips_low_energy_notification`，并更新 `energy_crossing_threshold_generates_notification` / `energy_staying_below_threshold_no_duplicate_notification` 两测试为非首次计算（设 `energy_updated_at = Some(..)`）。17 个相关单测全绿。

_代码审查后补充修复 (2026-06-23) — 前端能量值颜色一致性_

- [x] [Post-Review][Patch] 左侧角色栏低能量颜色与仪表盘不一致 — ✅ 已修复 [egosync-app/src/components/layout/RoleSidebarIcon.tsx:20]。问题：左侧角色栏低能量（<40）用灰色 `#9CA3AF`，仪表盘用红色 `red-500`，视觉不一致。修复：将 `getEnergyColor` 低能量返回值从 `#9CA3AF` 改为 `#EF4444`（与仪表盘 `red-500` 一致），同步更新测试断言。12 个前端单测全绿。注：此为 story 范围外的前端一致性修复。

## Dev Notes

### 项目背景

本 Story 属于 Epic 4（主动循环、通知与仪表盘），是能量值从静态默认值（100）到动态计算的引擎 story。Story 4.1-4.7 已完成后台调度器、主动建议、通知系统、Q2 保护提醒和仪表盘数据接通。本 Story 聚焦于后端能量计算引擎的实现，无前端改动（仪表盘已在 Story 4.7 中展示 `roles.energy` 字段）。

### 技术栈

- **后端**: Rust + Tauri 2.x + SQLx (SQLite) + tokio + chrono
- **IPC**: 无新增 Tauri 命令（能量计算在调度器后台执行）
- **前端**: 无改动（能量值已由 Story 4.7 仪表盘展示）

### 关键架构约束

**Rust 三层架构（严格遵守）**：
- `services/energy_calculator.rs` — 业务逻辑（读取数据 → 公式计算 → 写入 DB → 生成通知）
- `db/roles.rs` — 新增 `update_energy` 函数
- `db/tasks.rs` — 新增 4 个统计查询函数
- `db/conversations.rs` — 复用 `get_last_active_for_roles`（不新增）
- `services/scheduler.rs` — 修改 `run_work_loop_for_role` 添加能量计算调用
- `models/role.rs` — 添加 `energy_updated_at` 字段
- `migrations/019_energy_updated_at.sql` — ALTER TABLE 添加列

**无新增 Tauri 命令**：能量计算在调度器工作循环中自动执行，不需要前端触发。仪表盘通过已有的 `dashboard_get_status` 命令读取 `roles.energy` 字段。

**serde 序列化**: `Role` 结构体已有 `#[serde(rename_all = "camelCase")]`，新增 `energy_updated_at` 自动序列化为 `energyUpdatedAt`。

**ConversationsPool 获取方式**: `run_work_loop_for_role` 需新增 `conv_pool: &ConversationsPool` 参数。`spawn_scheduler` 已持有 `conv_pool: ConversationsPool`（`scheduler.rs:285`），传入即可。

### 能量计算公式详解

**公式**: `energy = 0.4 * task_completion_rate + 0.3 * recent_activity_score + 0.2 * goal_progress + 0.1 * (100 - at_risk_penalty)`

**各子项计算规则**：

1. **task_completion_rate**（任务完成率，权重 40%）：
   - = 最近 7 天完成任务数 / 总任务数 * 100
   - 总任务数 = 该角色所有未删除任务（包括已完成和未完成）
   - 若总任务数 = 0 → task_completion_rate = 0
   - "最近 7 天"以 `completed_at` 时间戳为准

2. **recent_activity_score**（用户互动频率，权重 30%）：
   - 基于角色最近一条对话的 `updated_at` 时间衰减
   - 今天 = 100 / 昨天 = 80 / 2 天前 = 65 / 3 天前 = 50 / 4-6 天前 = 30 / ≥ 7 天 = 10 / 无对话 = 0
   - 复用 `db::conversations::get_last_active_for_roles` 获取 `MAX(updated_at)`

3. **goal_progress**（大石头推进度，权重 20%）：
   - V1 简化：`(1 - active_big_rocks / 3) * 100`，clamped [0, 100]
   - 0 个活跃大石头 → 100（全部完成或未设置）
   - 1 个 → 67 / 2 个 → 33 / 3 个 → 0
   - **V1 限制**：`set_task_completion` 完成任务时清除 `is_big_rock` 标记（方案 D），导致无法追踪历史大石头完成记录。V1 以当前活跃大石头数量推算进度，V2 可通过添加 `was_big_rock` 列提升精度

4. **at_risk_penalty**（目标更新活跃度反向指标，权重 10%）：
   - = at_risk 的 Q2 任务数 * 20，最高扣 60
   - 0 个 at_risk → penalty = 0 → 子项 = 100
   - 1 个 → penalty = 20 → 子项 = 80
   - 3 个 → penalty = 60 → 子项 = 40
   - 4+ 个 → penalty = 60（封顶）→ 子项 = 40

**最终能量值**：四舍五入为整数，clamp [0, 100]

### 低能量通知逻辑

**仅在跨越阈值时通知**：
- 旧能量值 ≥ 40 且 新能量值 < 40 → 生成"轻触"通知
- 旧能量值 < 40 且 新能量值 < 40 → 不通知（避免每次工作循环都发通知）
- 旧能量值 ≥ 40 且 新能量值 ≥ 40 → 不通知

通知文案：`"你的{角色名}角色能量值较低，可能需要关注"`
通知级别：`NotificationLevel::Tap`（"轻触"）
通知创建失败只 `tracing::warn!`，不阻塞能量值写入

### 已有代码复用（关键 — 避免重复造轮子）

**1. `db::conversations::get_last_active_for_roles`**：`db/conversations.rs:411-436` 已实现批量查询角色最近活跃时间。传入单角色 ID 列表即可复用。

**2. `services::notification_service::create_notification_for_role`**：`services/notification_service.rs:57-102` 已实现通知创建 + 降级逻辑。直接调用，传入 `NotificationLevel::Tap`。

**3. `services::suggestion_generator::NotificationLevel`**：`services/suggestion_generator.rs` 已定义 `NotificationLevel` 枚举（Whisper/Tap/Knock）。复用 `NotificationLevel::Tap`。

**4. `services::task_protection_watch::compute_at_risk_threshold`**：`services/task_protection_watch.rs:38-56` 已实现 `now - N days` 的 ISO 8601 时间戳计算。`compute_7_days_ago_threshold` 可参考此模式（改 `AT_RISK_DAYS` 为 7）。

**5. `db::settings::chrono_now_pub`**：`db/settings.rs` 已实现当前时间的 ISO 8601 格式化。复用此函数生成 `energy_updated_at` 时间戳。

**6. `AppError` 枚举**：`error.rs` 已定义 NotFound/DbError/ValidationError 等变体，复用这些变体，不新增。

**7. `services::q2_protection_reminder` 模式参考**：`services/q2_protection_reminder.rs` 展示了"查询数据 → 创建通知 → emit 事件 → 错误降级"的完整模式。`energy_calculator` 遵循同样模式，但不 emit 事件（能量值更新不需要实时推送到前端，仪表盘下次查询自动获取新值）。

### 时间衰减计算实现

`recent_activity_score_from_last_active` 需解析 ISO 8601 时间戳并计算距今天数。参考 `services/q2_protection_reminder.rs:47-56` 的 `parse_iso_to_local_date` 函数模式：

```rust
fn parse_iso_to_local_date(s: &str) -> Option<chrono::NaiveDate> {
    let utc = chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
                .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
        })
        .ok()?;
    Some(utc.with_timezone(&chrono::Local).date_naive())
}
```

计算天数差：`(now_local.date_naive() - last_active_date).num_days()`

### Previous Story Intelligence（Story 4.7 + 4.6 + 4.5）

- **4.7 仪表盘能量值展示**：`services/dashboard_service.rs:46` 直接从 `role.energy` 读取。本 story 计算并写入 `roles.energy` 后，仪表盘下次查询自动获取新值。
- **4.6 ConversationsPool 传递模式**：`services/q2_protection_reminder.rs` 中 `check_and_generate_reminders` 同时接收 `pool: &SqlitePool` 和 `conv_pool: &ConversationsPool`。本 story 的 `calculate_and_update_energy` 遵循同样模式。
- **4.6 通知创建模式**：`q2_protection_reminder.rs:127-165` 展示了"先创建通知，失败则 warn 不阻塞"的模式。本 story 低能量通知遵循同样模式。
- **4.5 NotificationLevel 枚举**：`services/suggestion_generator.rs` 定义了 `NotificationLevel::Tap`。本 story 复用此枚举。
- **4.1 调度器工作循环**：`run_work_loop_for_role` 已在 `scheduler.rs:148-274` 实现。本 story 在其末尾添加能量计算调用。
- **4.6 测试命令**：`cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` — 本 story 新增测试后应维持 0 failed。

### Git Intelligence

- `92a316e`（HEAD）— feat(4.7): 仪表盘接通真实角色状态数据（最新提交，本 story 的直接前置）
- `0139d7a` — feat(4.6): Q2 保护管家提醒 + 通知中心已读折叠
- `1b21b4b` — feat: 三级通知系统（耳语/轻触/敲门）完整实现
- 范本来源：`services/q2_protection_reminder.rs`（service 层 + 通知创建模式）、`services/task_protection_watch.rs`（时间阈值计算模式）、`db/tasks.rs`（统计查询模式）

### Testing Requirements

- **Rust 单测**（`services/energy_calculator.rs` ~7 个测试，`db/tasks.rs` ~4 个测试，`db/roles.rs` ~1 个测试）：
  - `recent_activity_score_from_last_active`：今天/昨天/3天/7天/None/无效字符串
  - `calculate_and_update_energy`：无任务无对话 / 有完成任务 / 有 at_risk 任务 / 跨越阈值通知 / 不重复通知 / clamp
  - `count_completed_tasks_in_last_7_days_for_role`：7天内 / 7天前 / 未完成 / 已删除
  - `count_total_tasks_for_role`：正确统计 / 排除已删除
  - `count_at_risk_q2_tasks_for_role`：at_risk Q2 / 非 at_risk / 非 Q2 / 已完成
  - `count_active_big_rocks_for_role`：活跃大石头 / 已完成 / 已删除
  - `update_energy`：正确写入 / 角色不存在
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`
  - `npm --prefix "egosync-app" run build`（验证无编译错误）
- **无前端测试** — 本 story 无前端改动

### Project Structure Notes

- **新增文件（2）**：
  - `egosync-app/src-tauri/migrations/019_energy_updated_at.sql` — ALTER TABLE 添加 energy_updated_at 列
  - `egosync-app/src-tauri/src/services/energy_calculator.rs` — 能量计算服务
- **修改文件（6）**：
  - `egosync-app/src-tauri/src/models/role.rs` — 添加 `energy_updated_at: Option<String>` 字段
  - `egosync-app/src-tauri/src/db/roles.rs` — 更新 `ROLE_SELECT_COLUMNS` + 新增 `update_energy` + 更新测试 schema
  - `egosync-app/src-tauri/src/db/tasks.rs` — 新增 4 个统计查询函数 + `compute_7_days_ago_threshold` + 测试
  - `egosync-app/src-tauri/src/services/mod.rs` — 注册 `energy_calculator` 模块
  - `egosync-app/src-tauri/src/services/scheduler.rs` — `run_work_loop_for_role` 添加 `conv_pool` 参数 + 能量计算调用 + 更新测试
  - `egosync-app/src-tauri/src/services/scheduler.rs` 测试 — `run_work_loop_for_role_returns_ok_without_provider` 追加 `conv_pool` + `energy_updated_at: None`
- **无新增 Tauri 命令** — 能量计算在调度器后台执行
- **无新增前端文件** — 能量值已由 Story 4.7 仪表盘展示
- **无新增依赖** — 复用现有 sqlx/chrono/tracing 依赖
- 符合项目规则：Rust 三层、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块 snake_case

### References

- `_bmad-output/project-context.md`（Rust 三层 / serde camelCase / 无 unwrap / tracing / 数据边界）
- `_bmad-output/planning-artifacts/epics.md:1836-1871`（Story 4.8 定义）
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-05-20.md:565-567`（Mn-3: Story 4.8 公式与 PRD 追溯注释）
- `_bmad-output/implementation-artifacts/4-7-dashboard-real-role-status.md`（Story 4.7 — 仪表盘能量值展示，直接前置）
- `_bmad-output/implementation-artifacts/4-6-q2-protection-butler-reminder.md`（Story 4.6 — ConversationsPool 传递 + 通知创建模式）
- `egosync-app/src-tauri/migrations/003_roles.sql:10`（roles 表 energy 列定义）
- `egosync-app/src-tauri/src/models/role.rs:1-17`（Role 结构体 — energy 字段）
- `egosync-app/src-tauri/src/db/roles.rs:9`（ROLE_SELECT_COLUMNS 常量）
- `egosync-app/src-tauri/src/db/roles.rs:284-524`（roles 测试 — setup_test_db CREATE TABLE）
- `egosync-app/src-tauri/src/services/scheduler.rs:148-274`（run_work_loop_for_role — 能量计算插入点）
- `egosync-app/src-tauri/src/services/scheduler.rs:285-394`（spawn_scheduler — conv_pool 持有者）
- `egosync-app/src-tauri/src/services/scheduler.rs:565-580`（测试 Role 构造 — 需追加 energy_updated_at）
- `egosync-app/src-tauri/src/db/tasks.rs:257-295`（set_task_completion — is_big_rock 清除逻辑）
- `egosync-app/src-tauri/src/db/tasks.rs:492-510`（list_at_risk_q2_tasks — 跨角色查询模式参考）
- `egosync-app/src-tauri/src/db/tasks.rs:314-333`（count_big_rocks_by_owner — 大石头统计模式参考）
- `egosync-app/src-tauri/src/db/conversations.rs:411-436`（get_last_active_for_roles — 可复用）
- `egosync-app/src-tauri/src/services/notification_service.rs:57-102`（create_notification_for_role — 通知创建）
- `egosync-app/src-tauri/src/services/suggestion_generator.rs`（NotificationLevel 枚举）
- `egosync-app/src-tauri/src/services/q2_protection_reminder.rs:38-56`（parse_iso_to_local_date 模式参考）
- `egosync-app/src-tauri/src/services/task_protection_watch.rs:38-56`（compute_at_risk_threshold — 时间阈值计算模式）
- `egosync-app/src-tauri/src/services/dashboard_service.rs:46`（energy 字段读取 — 仪表盘已接通）
- `egosync-app/src-tauri/src/db/pool.rs:44-51`（run_migrations — sqlx::migrate! 自动执行）
- `egosync-app/src-tauri/src/db/settings.rs`（chrono_now_pub — ISO 8601 时间戳生成）
- `egosync-app/src-tauri/src/services/mod.rs`（service 模块注册）
- `egosync-app/src-tauri/src/error.rs`（AppError 枚举）

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
