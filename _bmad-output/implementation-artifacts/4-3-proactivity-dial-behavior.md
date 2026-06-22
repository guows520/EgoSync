---
baseline_commit: c9106649c6fe0a98c34d2a68b26c0162af30dff6
---

# Story 4.3: 主动性三档刻度盘行为接通

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 用户,
I want 不同主动性档位带来明显不同的角色行为体验,
so that 我能精细控制角色的打扰程度。

## 背景与现状（务必先读）

**本 story 是纯 Rust 后端 story — 在 Story 4.1 调度器和 Story 4.2 建议生成器的基础上，按 `proactivity_level` 对生成的建议进行优先级过滤，并定义通知级别上限规则供 Story 4.5 消费。不涉及前端改动、不涉及新增迁移。**

### 已建成的基础（本 story 的接入点）

**Story 2.10（主动性 UI + 持久化）：**
- `roles.proactivity_level` 字段已存在，取值 `passive` / `moderate` / `proactive`，默认 `moderate`
- `ProactivityToggle.tsx` 已是受控组件，切换后调 `roleService.updateProactivity()` 持久化
- `commands/role.rs:114-128` `role_update_proactivity` 命令已注册
- `services/role_config.rs:77-85` `normalize_proactivity_level` 校验函数已存在
- **本 story 不动 UI 和持久化层 — 它们已经完成**

**Story 4.1（调度器）：**
- `services/scheduler.rs` 已实现 60 秒基础 tick + HH:MM 时间点匹配调度
- `get_trigger_times(pool, level)` 已实现：
  - `passive` → 返回 `None` → 调度器 `continue` 跳过该角色（**AC1 的 passive 行为已由 4.1 实现**）
  - `moderate` → 默认 3 个时间点（09:00 / 14:00 / 21:00）
  - `proactive` → 默认 5 个时间点（09:00 / 11:00 / 14:00 / 16:00 / 21:00）
- **每次 tick 动态读取 `db::roles::list_active_roles`**，角色 CRUD 和 proactivity 变更立即生效（**AC4 的"立即生效"已由 4.1 实现**）
- **核心接入点 = `services/scheduler.rs:144-211` 的 `run_work_loop_for_role`**：当前调用 `generate_suggestions` → 逐条写入 DB，**不过滤优先级**

**Story 4.2（建议生成器）：**
- `services/suggestion_generator.rs` 已实现完整的建议生成流程
- `generate_suggestions(pool, role) -> Result<Vec<CreateSuggestionInput>, AppError>` 返回带 `priority`（high/medium/low）的建议列表
- `CreateSuggestionInput` 含 `role_id`、`title`、`content`、`priority` 字段
- **当前 `generate_suggestions` 不根据 `proactivity_level` 过滤优先级 — 所有 priority 的建议都会写入 DB**

### 通知系统的现状（影响 AC2 / AC3 的通知部分）

**Story 4.5（三级通知系统）尚未实现** — 当前代码库中不存在 `notifications` 表、通知服务或通知级别枚举。因此：
- AC2 中"通知级别最高为轻触"和 AC3 中"high priority 建议可触发敲门通知"的**实际通知行为无法在本 story 中端到端实现**
- **本 story 的方案**：定义 `max_notification_level_for_proactivity` 纯函数作为通知级别上限规则，供 Story 4.5 消费。这确保 4.5 实现通知时只需调用此函数即可遵守主动性档位约束，无需回头修改 4.3
- 实际的通知创建、级别降级、敲门次数限制等行为是 Story 4.5 的职责

## Acceptance Criteria

> 既有调度器（Story 4.1）、建议生成器（Story 4.2）、记忆管线、任务管理和所有前端功能必须零回归。

1. **passive — 静默执行（AC1）**
   - **Given** 角色设为 `passive`
   - **When** 后台调度 tick
   - **Then** 调度器跳过该角色（不 spawn 工作循环，不生成建议，不发送通知）
   - **And** 仅在用户主动对话时响应
   - **Note** 此行为已由 Story 4.1 的 `get_trigger_times("passive") → None` 实现，本 story 验证不回归即可

2. **moderate — 适度建议（AC2）**
   - **Given** 角色设为 `moderate`
   - **When** 后台生成建议并写入 DB
   - **Then** 仅保留 `priority = high` 或 `priority = medium` 的建议
   - **And** `priority = low` 的建议在写入 DB 前被过滤掉（不写入 `suggestions` 表）
   - **And** 通知级别上限为"轻触"（tap），不使用"敲门"（knock）— **由 `max_notification_level_for_proactivity` 函数定义，实际通知行为在 Story 4.5 实现**

3. **proactive — 积极主动（AC3）**
   - **Given** 角色设为 `proactive`
   - **When** 后台生成建议并写入 DB
   - **Then** 保留所有优先级的建议（high / medium / low 均写入）
   - **And** `high` priority 建议可触发"敲门"（knock）通知 — **由 `max_notification_level_for_proactivity` 函数定义，实际通知行为在 Story 4.5 实现**

4. **变更立即生效（AC4）**
   - **Given** 用户在 RoleView SettingsTab 切换主动性档位
   - **When** 保存成功后
   - **Then** 下次调度循环立即生效（无需重启应用）
   - **Note** 此行为已由 Story 4.1 的动态读取实现，本 story 验证不回归即可

5. **Rust 后端过滤逻辑（AC5）**
   - **Given** Rust 后端
   - **Then** `run_work_loop_for_role` 在调用 `generate_suggestions` 后、写入 DB 前，根据 `role.proactivity_level` 过滤建议优先级
   - **And** 提供 `filter_suggestions_by_proactivity` 纯函数（可单测）
   - **And** 提供 `max_notification_level_for_proactivity` 纯函数（可单测，供 Story 4.5 消费）

6. **零回归（AC6）**
   - **Given** 本 story 完成
   - **Then** `cargo test` 全量通过，既有调度器/建议生成器/记忆管线/任务测试不回归
   - **And** 前端 `build` 和 `test:frontend` 通过（确认无前端影响）

## Tasks / Subtasks

- [x] Rust：在 `services/suggestion_generator.rs` 新增 `filter_suggestions_by_proactivity` 纯函数（AC: 2, 3, 5）
  - [x] 签名：`pub fn filter_suggestions_by_proactivity(suggestions: Vec<CreateSuggestionInput>, proactivity_level: &str) -> Vec<CreateSuggestionInput>`
  - [x] `moderate` → 过滤掉 `priority = "low"` 的建议，保留 `high` + `medium`
  - [x] `proactive` → 保留全部（原样返回）
  - [x] `passive` → 返回空 `Vec`（安全降级，虽然调度器已跳过 passive，但函数本身应正确处理）
  - [x] 未知值 → 返回空 `Vec`（安全降级）
  - [x] 不修改 `CreateSuggestionInput` 的任何字段，只做过滤

- [x] Rust：在 `services/suggestion_generator.rs` 新增 `max_notification_level_for_proactivity` 纯函数（AC: 2, 3, 5）
  - [x] 签名：`pub fn max_notification_level_for_proactivity(proactivity_level: &str) -> NotificationLevel`
  - [x] 定义 `NotificationLevel` 枚举：`Whisper` / `Tap` / `Knock`（derive `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`，`#[serde(rename_all = "camelCase"]]`）
  - [x] `passive` → `Whisper`（最低级别，passive 不应发通知但安全降级）
  - [x] `moderate` → `Tap`（最高"轻触"，不允许"敲门"）
  - [x] `proactive` → `Knock`（允许"敲门"）
  - [x] 未知值 → `Whisper`（安全降级）
  - [x] 此函数无当前消费者，但供 Story 4.5 直接调用

- [x] Rust：在 `services/scheduler.rs` 的 `run_work_loop_for_role` 中接入过滤逻辑（AC: 2, 3, 5）
  - [x] 在 `generate_suggestions` 返回后、写入 DB 前，调用 `suggestion_generator::filter_suggestions_by_proactivity(suggestions, &role.proactivity_level)`
  - [x] 过滤后的空列表走现有的"本次无建议生成"路径（`tracing::info!` + `return Ok(())`）
  - [x] 过滤掉的建议不需要额外日志（保持简洁，过滤是正常行为）
  - [x] **不改变 `run_work_loop_for_role` 的函数签名和错误处理策略**

- [x] Rust：单元测试（AC: 1, 2, 3, 5）
  - [x] `filter_suggestions_by_proactivity` 测试（在 `suggestion_generator.rs` 底部测试模块）：
    - `moderate` 过滤掉 `low`，保留 `high` + `medium`
    - `proactive` 保留全部
    - `passive` 返回空
    - 未知值返回空
    - 空输入返回空
    - 混合优先级输入正确过滤
  - [x] `max_notification_level_for_proactivity` 测试：
    - `passive` → `Whisper`
    - `moderate` → `Tap`
    - `proactive` → `Knock`
    - 未知值 → `Whisper`
  - [x] 既有 `suggestion_generator` 测试不回归
  - [x] 既有 `scheduler` 测试不回归

- [x] 验证（AC: 1-6）
  - [x] `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml` 通过（447 passed, 0 failed）
  - [x] `npm --prefix "egosync-app" run build` 通过
  - [x] `npm --prefix "egosync-app" run test:frontend` — 4 个既有前端测试失败（SettingsTab/TasksTab），与本 story 无关（本 story 纯 Rust 后端，未触碰前端文件）；212 passed

## Dev Notes

### Current State（基于当前代码 @ c910664）

- **调度器接入点**（`services/scheduler.rs:144-211`）：`run_work_loop_for_role` 当前流程：
  1. `tracing::info!` 记录触发
  2. `generate_suggestions(pool, role)` → 获取建议列表
  3. 空列表 → `tracing::info!` + `return Ok(())`
  4. 逐条 `db::suggestions::create_suggestion` 写入
  5. `tracing::info!` 记录写入完成
  **本 story 在步骤 2 和步骤 4 之间插入过滤步骤。**

- **建议生成器**（`services/suggestion_generator.rs`）：
  - `generate_suggestions` 返回 `Vec<CreateSuggestionInput>`，每个元素含 `priority: String`（已归一化为小写）
  - `CreateSuggestionInput` 定义在 `models/suggestion.rs:17-22`
  - `ALLOWED_PRIORITIES: &[&str] = &["high", "medium", "low"]`（`:20`）

- **Role 模型**（`models/role.rs:13`）：`proactivity_level: String`，取值 `passive` / `moderate` / `proactive`

- **proactivity 校验**（`services/role_config.rs:77-85`）：`normalize_proactivity_level` 确保只有合法值写入 DB

### What This Story Changes

**Rust（修改 2 文件，无新增文件）：**
1. `services/suggestion_generator.rs` — **修改**：新增 `filter_suggestions_by_proactivity` + `max_notification_level_for_proactivity` + `NotificationLevel` 枚举 + 单元测试
2. `services/scheduler.rs` — **修改**：`run_work_loop_for_role` 在 `generate_suggestions` 后插入过滤调用

**无前端改动、无新增迁移、无新增依赖、无 DB schema 改动。**

### What Must Be Preserved（防回归）

- **调度器 Story 4.1 行为不回归**：`run_work_loop_for_role` 签名、调度频率、`passive` 跳过、每角色独立 spawn、失败隔离、`triggered_at`/`duration_ms`/`result` 日志全部不变。过滤逻辑只是插入一步，不改变控制流结构。
- **`run_work_loop_for_role` 必须永不向上抛错**：过滤是纯内存操作，不会产生错误。过滤后的空列表走现有"本次无建议生成"路径。
- **建议生成器 Story 4.2 行为不回归**：`generate_suggestions` 函数本身不改，过滤在外部调用方（`run_work_loop_for_role`）进行。既有 21 个 `suggestion_generator` 单测全部通过。
- **记忆管线/任务/角色既有测试通过**：本 story 只修改 `suggestion_generator.rs`（新增函数+测试）和 `scheduler.rs`（插入一行过滤调用），不改既有模块对外行为。
- **前端零回归**：本 story 不触碰任何前端文件。

### 实现细节

**`filter_suggestions_by_proactivity` 实现要点：**
```rust
pub fn filter_suggestions_by_proactivity(
    suggestions: Vec<CreateSuggestionInput>,
    proactivity_level: &str,
) -> Vec<CreateSuggestionInput> {
    match proactivity_level {
        "proactive" => suggestions, // 保留全部
        "moderate" => suggestions
            .into_iter()
            .filter(|s| s.priority != "low")
            .collect(),
        _ => Vec::new(), // passive 或未知值，安全降级
    }
}
```

**`max_notification_level_for_proactivity` 实现要点：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationLevel {
    Whisper,
    Tap,
    Knock,
}

pub fn max_notification_level_for_proactivity(proactivity_level: &str) -> NotificationLevel {
    match proactivity_level {
        "proactive" => NotificationLevel::Knock,
        "moderate" => NotificationLevel::Tap,
        _ => NotificationLevel::Whisper,
    }
}
```

**`run_work_loop_for_role` 接入点（scheduler.rs:152-165 附近）：**
```rust
// 现有代码：
let suggestions = match crate::services::suggestion_generator::generate_suggestions(pool, role).await {
    Ok(list) => list,
    Err(e) => { /* ... */ return Ok(()); }
};

// 新增过滤（在此处插入）：
let suggestions = crate::services::suggestion_generator::filter_suggestions_by_proactivity(
    suggestions,
    &role.proactivity_level,
);

// 后续代码不变（空列表走现有路径）
```

### Previous Story Intelligence（Story 4.1 + 4.2）

- **4.1 已处理 passive 跳过**：`get_trigger_times("passive")` 返回 `None`，调度器 `continue`。`run_work_loop_for_role` 不会被 passive 角色调用。但 `filter_suggestions_by_proactivity` 仍需正确处理 `passive`（返回空），作为安全降级。
- **4.1 已处理立即生效**：每次 tick 动态读取 `list_active_roles`，proactivity 变更在下次 tick 自动生效。本 story 的过滤逻辑读取 `role.proactivity_level`（已在 `run_work_loop_for_role` 入参中），天然继承立即生效特性。
- **4.2 已建立优先级体系**：`generate_suggestions` 返回的建议 `priority` 已归一化为小写（`suggestion_generator.rs:264`：`s.priority.trim().to_lowercase()`）。`filter_suggestions_by_proactivity` 可直接与 `"low"` 字符串比较，无需额外归一化。
- **4.2 Review Findings**：空上下文角色短路跳过 LLM（已修复）、批次内同名建议去重（已修复）。本 story 不涉及这些路径。

### Git Intelligence

- `c910664`（HEAD）— Refine role settings and task filter UI（最新前端 UI 调整，不影响后端）
- `2afbdf7` — fix(4.2): patch AC4 empty-role short-circuit and batch dedup（4.2 评审修复）
- `e764743` — feat(4.1): 可配置调度时间点（4.1 最终实现）
- `b4ffb73` — docs: 同步 Story 4.1 文档与实际代码
- 范本来源：`suggestion_generator.rs`（4.2 实现）和 `scheduler.rs`（4.1 实现）是本 story 的直接修改对象

### Testing Requirements

- **Rust 单测**（`suggestion_generator.rs` 测试模块新增 ~8 个测试）：
  - `filter_suggestions_by_proactivity`：
    - `moderate` 过滤 `low` 保留 `high`+`medium`（核心测试）
    - `proactive` 保留全部
    - `passive` 返回空
    - 未知值返回空
    - 空输入返回空
    - 混合优先级正确过滤
  - `max_notification_level_for_proactivity`：
    - 三档正确返回 + 未知值降级
- **必跑命令**：
  - `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml`
  - `npm --prefix "egosync-app" run build`
  - `npm --prefix "egosync-app" run test:frontend`

### Project Structure Notes

- **修改文件（2）**：`services/suggestion_generator.rs`、`services/scheduler.rs`
- **无新增文件、无新增依赖、无新增迁移、无 DB schema 改动**
- 符合项目规则：Rust 三层（本 story 无 command 层）、serde camelCase、`Result<T,AppError>` + 无 `.unwrap()`、tracing 日志、模块 snake_case
- `NotificationLevel` 枚举放在 `suggestion_generator.rs` 中，Story 4.5 实现通知系统时可按需移动到独立模块

### References

- `_bmad-output/project-context.md`（Rust 三层 / serde camelCase / 无 unwrap / tracing / 数据边界）
- `_bmad-output/planning-artifacts/epics.md:1653-1683`（Story 4.3 定义）
- `_bmad-output/planning-artifacts/architecture.md:478`（实现序列「工作循环调度器 + 建议系统」）、`:479`（通知系统 + 仲裁引擎 — Story 4.5）
- `_bmad-output/implementation-artifacts/4-1-background-scheduler-work-loop.md`（Story 4.1 — 调度器基础，passive 跳过 + 立即生效已实现）
- `_bmad-output/implementation-artifacts/4-2-proactive-suggestion-generation.md`（Story 4.2 — 建议生成器，priority 体系已建立）
- `_bmad-output/implementation-artifacts/2-10-role-skill-config-proactivity-ui.md`（Story 2.10 — 主动性 UI + 持久化已完成）
- `egosync-app/src-tauri/src/services/scheduler.rs:144-211`（`run_work_loop_for_role` — 本 story 插入过滤调用处）
- `egosync-app/src-tauri/src/services/suggestion_generator.rs:151-269`（`generate_suggestions` — 返回带 priority 的建议列表）
- `egosync-app/src-tauri/src/models/suggestion.rs:17-22`（`CreateSuggestionInput` — 含 priority 字段）
- `egosync-app/src-tauri/src/models/role.rs:13`（`Role.proactivity_level` 字段）
- `egosync-app/src-tauri/src/services/role_config.rs:77-85`（`normalize_proactivity_level` 校验）

### Review Findings (2026-06-22)

- [x] [Review][Defer] `moderate` 档仅排除精确 `"low"`，未校验 priority 取值域，未知优先级会被保留 [egosync-app/src-tauri/src/services/suggestion_generator.rs:264] — deferred, pre-existing（4.2 `generate_suggestions` 仅 `trim().to_lowercase()`，不校验是否属 `ALLOWED_PRIORITIES`；非本 story 引入，且保留语义对 AC2 无害）

> 审查结论：0 待决策 / 0 待修补 / 1 已延后 / 1 噪音已忽略。`filter_suggestions_by_proactivity` 与 `max_notification_level_for_proactivity` 纯函数实现正确，覆盖 AC2/AC3/AC5；`scheduler.rs` 过滤接入点位置正确（`generate_suggestions` 之后、`is_empty` 之前），未改函数签名与错误处理策略，AC1/AC4 路径未触碰（4.1 行为不回归）。

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4 (Windsurf Cascade)

### Debug Log References

无调试问题——实现一次通过。

### Completion Notes List

- 在 `suggestion_generator.rs` 新增 `NotificationLevel` 枚举（`Whisper`/`Tap`/`Knock`，serde camelCase）
- 在 `suggestion_generator.rs` 新增 `filter_suggestions_by_proactivity` 纯函数：`proactive` 保留全部，`moderate` 过滤 `low`，`passive`/未知值返回空 Vec
- 在 `suggestion_generator.rs` 新增 `max_notification_level_for_proactivity` 纯函数：`proactive`→`Knock`，`moderate`→`Tap`，`passive`/未知→`Whisper`（供 Story 4.5 消费）
- 在 `scheduler.rs` 的 `run_work_loop_for_role` 中 `generate_suggestions` 返回后、空列表检查前插入过滤调用，不改变函数签名和错误处理策略
- 新增 10 个单元测试：6 个 `filter_suggestions_by_proactivity` 测试 + 4 个 `max_notification_level_for_proactivity` 测试
- `cargo test` 447 passed, 0 failed — 既有 21 个 suggestion_generator 测试和 17 个 scheduler 测试全部不回归
- `npm run build` 通过
- `npm run test:frontend` 4 个既有失败（SettingsTab/TasksTab 前端组件测试），与本 story 无关，本 story 未触碰任何前端文件

### File List

- `egosync-app/src-tauri/src/services/suggestion_generator.rs` — 修改：新增 `NotificationLevel` 枚举 + `filter_suggestions_by_proactivity` + `max_notification_level_for_proactivity` + 10 个单元测试
- `egosync-app/src-tauri/src/services/scheduler.rs` — 修改：`run_work_loop_for_role` 插入过滤调用
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 修改：story 状态更新

### Change Log

- 2026-06-22: Story 4.3 实现完成 — 主动性三档刻度盘行为接通，按 proactivity_level 过滤建议优先级 + 通知级别上限规则定义
