---
baseline_commit: 6bf2956ef6ae1f43b6288314348e34d45e2b8780
---

# Story 5.3: 系统自动检测多角色任务的时间冲突

**Status:** done
**Epic:** Epic 5 — 使命宣言与冲突仲裁（Mission & Arbitration）
**Previous Story:** 5-2-behavior-inferred-values (done)
**Next Story:** 5-4-three-step-arbitration (backlog)

---

## 背景与目标

用户有多个角色（如"产品经理"和"家庭"），每个角色下有带 deadline 的任务。当两个或多个不同角色的任务 deadline 落在同一时段（±1 小时内）时，系统需要主动检测冲突、写入 `conflicts` 表、生成"敲门"级通知，让用户不会遗漏时间重叠的任务。

本 Story 是 Epic 5 仲裁链路的**入口**：冲突检测 → (Story 5.4) 三步仲裁 → (Story 5.5) 仲裁可视化 → (Story 5.6) 自动执行方案。本 Story 只负责**检测 + 记录 + 通知**，不涉及仲裁逻辑。

---

## 验收标准（Acceptance Criteria）

1. **调度器定时扫描所有角色的 tasks**
   - Given 调度器每次 tick（60 秒基础 tick）
   - When 发现两个或多个**不同角色**的任务 deadline 在同一时段（±1 小时内）
   - Then 生成冲突记录写入 `conflicts` 表

2. **新冲突产生"敲门"级通知**
   - Given 冲突检测结果
   - When 新冲突产生
   - Then 生成"敲门"级通知，内容如"发现时间冲突：产品经理 vs 家庭，周五 15:00"
   - And 通知中包含"处理仲裁"按钮（前端 UI 由 Story 5.5 实现，本 Story 只需通知内容包含冲突信息）

3. **已处理冲突不重复检测**
   - Given 冲突已被用户处理（采纳/延后/拒绝 — 状态为 `resolved` 或 `dismissed`）
   - Then 不重复检测同一组冲突（相同 `task_id_a` + `task_id_b` 组合）

4. **用户手动调整时间后冲突自动解除**
   - Given 用户手动调整了冲突任务的 deadline（使两者不再重叠）
   - Then 冲突自动解除，`conflicts.status` 更新为 `resolved`，不再提醒

5. **数据库迁移**
   - Given 数据库
   - Then `migrations/021_conflicts.sql` 创建 `conflicts` 表：`id`, `task_id_a`, `task_id_b`, `role_id_a`, `role_id_b`, `conflict_time`, `status`(detected/resolved/dismissed), `resolution`, `created_at`

6. **Rust 后端 — conflict_detector 服务**
   - Given Rust 后端
   - Then `conflict_detector` 服务：扫描 deadline 重叠 → 写入 conflicts 表 → 发送通知
   - And 调度器每次 tick 包含冲突检测步骤（类似 Story 4.6 Q2 保护提醒的模式）

7. **已完成/已删除任务不参与冲突检测**
   - Given 任务 `is_completed = true` 或 `deleted_at IS NOT NULL`
   - Then 不参与冲突检测

8. **同角色内任务不产生冲突**
   - Given 两个任务属于同一角色（`role_id` 相同）
   - Then 不产生冲突记录（冲突只检测跨角色）

---

## 任务分解（Tasks/Subtasks）

### Task 1: 数据库迁移 — `conflicts` 表

- [ ] 创建 `migrations/021_conflicts.sql`
- [ ] 表结构：`id` TEXT PRIMARY KEY, `task_id_a` TEXT NOT NULL, `task_id_b` TEXT NOT NULL, `role_id_a` TEXT NOT NULL, `role_id_b` TEXT NOT NULL, `conflict_time` TEXT NOT NULL, `status` TEXT NOT NULL DEFAULT 'detected' CHECK(status IN ('detected', 'resolved', 'dismissed')), `resolution` TEXT, `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
- [ ] 约束：`task_id_a < task_id_b`（字典序，确保同一对任务只有一条记录，便于去重查询）
- [ ] 外键：`task_id_a` → `tasks(id)` ON DELETE CASCADE, `task_id_b` → `tasks(id)` ON DELETE CASCADE
- [ ] 索引：`idx_conflicts_status` ON `conflicts(status)`, `idx_conflicts_task_ids` ON `conflicts(task_id_a, task_id_b)`

### Task 2: 数据模型 — `models/conflict.rs`

- [ ] 创建 `Conflict` struct（`#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]`, `#[serde(rename_all = "camelCase")]`）
- [ ] 字段：`id`, `taskIdA`, `taskIdB`, `roleIdA`, `roleIdB`, `conflictTime`, `status`, `resolution: Option<String>`, `createdAt`
- [ ] 创建 `ConflictWithDetails` struct（带任务标题、角色名称等 JOIN 查询结果）
- [ ] 在 `models/mod.rs` 注册 `pub mod conflict;`

### Task 3: DB 层 — `db/conflicts.rs`

- [ ] 在 `db/mod.rs` 注册 `pub mod conflicts;`
- [ ] `create_conflict(pool, task_id_a, task_id_b, role_id_a, role_id_b, conflict_time) -> Result<Conflict, AppError>`
  - INSERT OR IGNORE（基于 `task_id_a + task_id_b` 唯一性，避免重复插入）
  - 生成 UUID v4 作为 id
  - 确保 `task_id_a < task_id_b`（字典序交换）
- [ ] `list_conflicts_by_status(pool, status) -> Result<Vec<Conflict>, AppError>`
- [ ] `get_conflict_by_task_ids(pool, task_id_a, task_id_b) -> Result<Option<Conflict>, AppError>`
- [ ] `update_conflict_status(pool, id, status, resolution) -> Result<Conflict, AppError>`
- [ ] `list_active_conflicts_with_details(pool) -> Result<Vec<ConflictWithDetails>, AppError>`
  - JOIN tasks + roles 获取任务标题、角色名称等
  - 只返回 `status = 'detected'` 的冲突
- [ ] `auto_resolve_stale_conflicts(pool) -> Result<u64, AppError>`
  - 检查所有 `status = 'detected'` 的冲突，如果对应任务的 deadline 已不再重叠（±1h），则自动更新为 `resolved`

### Task 4: 服务层 — `services/conflict_detector.rs`

- [ ] 在 `services/mod.rs` 注册 `pub mod conflict_detector;`
- [ ] `detect_conflicts(pool: &SqlitePool, app_handle: Option<&AppHandle>) -> Result<(), AppError>`
  - 查询所有未完成、未删除、有 deadline 的角色任务（`owner_type = 'role'`, `is_completed = 0`, `deleted_at IS NULL`, `deadline IS NOT NULL`）
  - 按 deadline 排序，滑动窗口比较：如果两个任务的 deadline 差值 ≤ 1 小时且 `role_id` 不同，则为一组冲突
  - 对每组冲突：检查 `conflicts` 表是否已有记录（`task_id_a + task_id_b`），如果已有 `detected` 状态的记录则跳过
  - 如果是新冲突：调用 `db::conflicts::create_conflict` 写入记录
  - 调用 `notification_service::create_notification_for_role` 为**管家角色**（或第一个角色）创建"敲门"级通知
  - 通知内容格式：`"发现时间冲突：{角色A名} vs {角色B名}，{冲突时间}"`
  - emit Tauri Event `conflict:new`（payload 含 conflict 信息 + 角色/任务名称）
  - 永不向上抛错：内部所有失败都降级为 `tracing::warn!` + 返回 `Ok(())`
- [ ] `auto_resolve_conflicts(pool: &SqlitePool) -> Result<(), AppError>`
  - 调用 `db::conflicts::auto_resolve_stale_conflicts`
  - 如果有冲突被自动解除，`tracing::info!` 记录

### Task 5: 调度器集成

- [ ] 在 `scheduler.rs` 的 `spawn_scheduler` 主循环中，**在 Q2 保护提醒检查之后**，添加冲突检测步骤
- [ ] 调用 `conflict_detector::detect_conflicts(&pool, Some(&app_handle))`
- [ ] 调用 `conflict_detector::auto_resolve_conflicts(&pool)`
- [ ] 错误只 `tracing::warn!`，不阻塞调度器循环（与 Q2 保护提醒模式一致）

### Task 6: Tauri Command 层 — `commands/conflict.rs`

- [ ] 在 `commands/mod.rs` 注册 `pub mod conflict;`
- [ ] `conflict_list_detected` — 列出所有 `status = 'detected'` 的冲突（含任务/角色详情）
- [ ] `conflict_resolve` — 手动标记冲突为 resolved（供前端用户手动调整后调用）
- [ ] `conflict_dismiss` — 用户主动忽略冲突
- [ ] Command 层保持薄层：参数解析 → 调用 service/db → 返回结果
- [ ] 在 `lib.rs` 的 `invoke_handler!` 中注册新命令

### Task 7: 前端类型与服务

- [ ] 创建 `src/types/conflict.ts` — `Conflict` 和 `ConflictWithDetails` 接口（camelCase）
- [ ] 创建 `src/services/conflictService.ts` — 封装 Tauri invoke 调用
- [ ] 创建 `src/hooks/useConflicts.ts` — 查询 detected 冲突列表 + 监听 `conflict:new` 事件

### Task 8: 前端通知集成

- [ ] 在 `useTauriEvent('conflict:new')` 监听冲突事件
- [ ] 当收到冲突事件时，在管家对话区或通知中心显示冲突信息
- [ ] 通知内容包含角色名称和冲突时间，用户可点击查看详情（ArbitrationModal 接通由 Story 5.5 完成，本 Story 只需事件监听 + 基础显示）

### Task 9: 测试

- [ ] Rust 单元测试（`services/conflict_detector.rs` 底部 `#[cfg(test)] mod tests`）：
  - 冲突检测：两个不同角色任务 deadline 在 ±1h 内 → 生成冲突
  - 无冲突：两个不同角色任务 deadline 相差 > 1h → 不生成冲突
  - 同角色不冲突：同一角色两个任务 deadline 相同 → 不生成冲突
  - 已完成任务排除：`is_completed = true` 的任务不参与检测
  - 已删除任务排除：`deleted_at IS NOT NULL` 的任务不参与检测
  - 去重：同一对任务不重复插入冲突
  - 自动解除：调整 deadline 后不再重叠 → 自动 resolved
- [ ] Rust 单元测试（`db/conflicts.rs` 底部）：
  - CRUD 操作基本覆盖
  - `task_id_a < task_id_b` 字典序保证
  - `auto_resolve_stale_conflicts` 逻辑验证
- [ ] 前端测试（如有必要）：`conflictService.test.ts` 基础调用验证

---

## 开发笔记（Dev Notes）

### 关键技术决策

1. **冲突检测算法 — 滑动窗口**
   - 查询所有有 deadline 的未完成角色任务，按 deadline 升序排序
   - 滑动窗口比较相邻任务的 deadline 差值：如果 ≤ 1 小时（3600 秒）且 `role_id` 不同，则为一组冲突
   - 注意：一个任务可能与多个任务冲突（如 A、B、C 都在 15:00 ±1h），需要生成 A-B、A-C、B-C 三组冲突
   - 实现方式：双重循环遍历所有任务对，或排序后滑动窗口（后者更高效）

2. **去重策略 — `task_id_a < task_id_b`**
   - 插入前确保 `task_id_a` 字典序小于 `task_id_b`，这样同一对任务只有一条记录
   - 使用 `INSERT OR IGNORE` + 唯一索引 `(task_id_a, task_id_b)` 防止重复插入
   - 查询时用 `get_conflict_by_task_ids(pool, min(a,b), max(a,b))` 检查是否已存在

3. **自动解除逻辑**
   - 每次调度器 tick 时，检查所有 `status = 'detected'` 的冲突
   - 重新计算对应两个任务的 deadline 差值，如果 > 1 小时，则自动更新为 `resolved`
   - 这覆盖了"用户手动调整了冲突任务的时间"的场景

4. **通知归属角色**
   - 冲突通知应归属于**管家角色**（butler），因为管家负责协调跨角色事务
   - 但 `notifications` 表的 `role_id` 外键要求引用 `roles` 表，管家不是角色表中的角色
   - **解决方案**：通知归属于冲突中第一个角色（`role_id_a`），通知内容包含两个角色名称
   - 或者：使用 `role_id_a` 作为通知的 `role_id`，因为 notifications 表已有外键约束

5. **deadline 格式**
   - tasks 表的 `deadline` 字段为 `TEXT` 类型，存储 ISO 8601 格式字符串（如 `2026-06-27T15:00:00Z`）
   - 冲突检测时需要解析为 `chrono::DateTime` 进行比较
   - 如果 deadline 为空或格式无效，跳过该任务（不参与冲突检测）

6. **调度器集成模式 — 与 Q2 保护提醒一致**
   - 在 `spawn_scheduler` 的主循环中，每次 tick 都执行冲突检测（不受触发时间点限制）
   - 频率：每 60 秒检查一次，足够实时且不会过于频繁
   - 错误处理：`tracing::warn!` + 继续，绝不阻塞调度器

### 架构合规

- **分层严格**：Command 层只做参数解析和调用 service/db；Service 层包含业务逻辑；DB 层只执行 SQL
- **serde camelCase**：所有 Rust struct 使用 `#[serde(rename_all = "camelCase")]`
- **错误处理**：所有函数返回 `Result<T, AppError>`，禁止 `.unwrap()`
- **迁移文件**：`021_conflicts.sql`，编号紧接 `020_mission.sql`
- **Tauri Event 命名**：`conflict:new`（遵循 `{domain}:{verb_past}` 模式）
- **ID 格式**：UUID v4 字符串
- **日期格式**：ISO 8601 字符串

### 前端 UI 规范

- 本 Story **不实现 ArbitrationModal 接通真实数据**（那是 Story 5.5 的工作）
- 本 Story 只需要：
  1. 监听 `conflict:new` 事件
  2. 在通知中心显示冲突通知（复用现有通知系统）
  3. 提供 `conflictService.ts` 和 `useConflicts.ts` 供后续 Story 使用
- ArbitrationModal 当前为 mock 原型，保持不变

### 反模式（禁止）

- ❌ 在 Command 层写冲突检测业务逻辑
- ❌ 在前端直接操作 SQLite
- ❌ 使用 `.unwrap()` 处理可能失败的操作
- ❌ 硬编码角色名称或任务标题到冲突检测逻辑
- ❌ 在调度器中同步阻塞执行冲突检测（应异步 `tokio::spawn` 或直接 await）
- ❌ 重复检测已存在的冲突（浪费 DB 写入）

### 现有基础设施复用

| 组件 | 文件路径 | 复用方式 |
|------|----------|----------|
| 调度器主循环 | `services/scheduler.rs:299-408` | 在 tick 末尾添加冲突检测调用 |
| 通知服务 | `services/notification_service.rs` | 调用 `create_notification_for_role` |
| 通知模型 | `models/notification.rs` | 复用 `NotificationNewPayload` |
| 任务查询 | `db/tasks.rs` | 参考查询模式，新增按 deadline 查询 |
| 错误类型 | `error.rs` | 复用 `AppError` |
| DB 池 | `db/pool.rs` | 复用 `DbPool` |
| Tauri Event | `AppHandle::emit` | 参考 `notification:new` emit 模式 |

### 项目结构变更

新增文件：
```
src-tauri/
├── migrations/
│   └── 021_conflicts.sql              # 新增
├── src/
│   ├── models/
│   │   ├── mod.rs                     # 添加 pub mod conflict;
│   │   └── conflict.rs                # 新增
│   ├── db/
│   │   ├── mod.rs                     # 添加 pub mod conflicts;
│   │   └── conflicts.rs               # 新增
│   ├── services/
│   │   ├── mod.rs                     # 添加 pub mod conflict_detector;
│   │   └── conflict_detector.rs       # 新增
│   └── commands/
│       ├── mod.rs                     # 添加 pub mod conflict;
│       └── conflict.rs                # 新增
├── src/
│   ├── types/
│   │   └── conflict.ts                # 新增
│   ├── services/
│   │   └── conflictService.ts         # 新增
│   └── hooks/
│       └── useConflicts.ts            # 新增
```

修改文件：
- `src-tauri/src/lib.rs` — 在 `invoke_handler!` 中注册 conflict 命令
- `src-tauri/src/services/scheduler.rs` — 在主循环中添加冲突检测调用
- `src/App.tsx` — 监听 `conflict:new` 事件（可选，也可在 useConflicts 中处理）

---

## 前置 Story 情报

### Story 5.2 (done) — mission_inferrer 服务模式参考

Story 5.2 实现了 `mission_inferrer` 服务，其模式可直接参考：
- 服务函数签名：`async fn infer_values(pool: &SqlitePool, conv_pool: &ConversationsPool) -> Result<..., AppError>`
- 错误降级：内部失败只 `tracing::warn!`，不阻塞调用方
- Tauri Command 层薄封装：参数解析 → 调用 service → 返回结果
- 测试模式：`#[tokio::test]` + 内存 SQLite + 手动建表

### Story 4.6 (done) — Q2 保护提醒调度器集成模式参考

Story 4.6 的 `q2_protection_reminder` 是在调度器主循环中每次 tick 执行的，与本 Story 的冲突检测模式完全一致：
- 在 `spawn_scheduler` 的 `loop` 中，`interval.tick().await` 之前调用
- 错误只 `tracing::warn!`，不阻塞循环
- 参考 `scheduler.rs:394-404` 的 Q2 保护提醒集成方式

### Story 4.5 (done) — 通知系统复用

- `notification_service::create_notification_for_role(pool, role_id, level, content)` 直接调用
- `NotificationLevel::Knock` 对应"敲门"级
- 通知降级逻辑已内置（proactivity 约束 + 每日敲门上限）
- emit `notification:new` 事件已有完整模式

---

## 测试要求

### Rust 单元测试

**`services/conflict_detector.rs` 测试：**
- 测试 DB 需创建 `tasks`、`roles`、`conflicts`、`notifications` 表
- 参考 `scheduler.rs` 测试中的建表模式（手动 `CREATE TABLE`）
- 需要插入带 deadline 的角色任务来模拟冲突场景
- deadline 格式使用 ISO 8601：`2026-06-27T15:00:00Z`

**`db/conflicts.rs` 测试：**
- CRUD 基本覆盖
- `task_id_a < task_id_b` 字典序验证
- `auto_resolve_stale_conflicts` 逻辑验证

### 前端测试

- `conflictService.test.ts` — 验证 invoke 调用参数（如有前端测试基础设施）
- `useConflicts.test.ts` — 验证事件监听和状态更新（如有）

### E2E 测试

- 本 Story 不需要 E2E 测试（UI 变更最小，ArbitrationModal 仍为 mock）

---

## 引用

- Epic 文档：`_bmad-output/planning-artifacts/epics.md:1952-1981`（Story 5.3 AC）
- 架构文档：`_bmad-output/planning-artifacts/architecture.md:588`（arbitration.rs 服务规划）
- 前置 Story：`_bmad-output/implementation-artifacts/5-2-behavior-inferred-values.md`
- 调度器：`egosync-app/src-tauri/src/services/scheduler.rs:299-408`
- 通知服务：`egosync-app/src-tauri/src/services/notification_service.rs:57-102`
- 任务模型：`egosync-app/src-tauri/src/models/task.rs:50-70`
- 任务 DB：`egosync-app/src-tauri/src/db/tasks.rs:60-85`
- 通知模型：`egosync-app/src-tauri/src/models/notification.rs:1-51`
- 通知 DB：`egosync-app/src-tauri/src/db/notifications.rs:80-94`
- 前端 ArbitrationModal（mock）：`egosync-app/src/components/modals/ArbitrationModal.tsx`
- 项目上下文：`_bmad-output/project-context.md`

---

## Dev Agent Record

_（此部分由开发代理在实现过程中填写）_

### Implementation Log
- [日期] 开始实现
- [日期] 完成 Task X
...

### Changes Made
- 文件列表

### Deviations from Story
- 偏差说明

### Testing Results
- 测试输出摘要

---

## Review Findings

_代码审查（2026-06-25，Amelia）— 三层对抗式审查：盲点猎手 / 边界猎手 / 验收审计_

- [x] [Review][Patch] 通知内容缺少冲突时间（违反 AC2）[conflict_detector.rs:244-247] — 已修复：通知内容补上冲突时间
- [x] [Review][Decision→保持] Tauri Event 命名偏离 spec — 决策保持 `conflict:detected`/`conflict:resolved`（前后端一致，拆分更清晰），后续对齐文档
- [x] [Review][Decision→Defer] 自动解决的冲突若 deadline 再次重叠不会重新检测 — 决策 V1 保持现状（避免反复打扰）
- [x] [Review][Patch] build_conflict_payload 出错会中断整个检测循环 [conflict_detector.rs:72] — 已修复：降级为 warn 并跳过该冲突
- [x] [Review][Patch] create_conflict 对乱序任务对静默丢弃 [conflicts.rs:21-38] — 已修复：函数内部用 ordered_pair 强制排序
- [x] [Review][Defer] deadline 解析对非 RFC3339/无秒无时区格式失败致漏检 [conflicts.rs:195-219] — deferred, 依赖存储格式约定（ISO8601 Z）
- [x] [Review][Defer] deadline 字符串字典序排序在混合格式下提前 break 致漏检 [conflict_detector.rs:146-164] — deferred, 同上格式约定前提
- [x] [Review][Defer] useConflicts 每次挂载加载全量冲突历史 + conflicts 表无保留/清理策略 [useConflicts.ts:22, 021_conflicts.sql] — deferred, 无界增长（V1 可接受）
- [x] [Review][Defer] 检测 O(n²) + auto_resolve 每冲突 2 次查询 [conflict_detector.rs:144-175, conflicts.rs:135-154] — deferred, 规模化性能（V1 可接受）
