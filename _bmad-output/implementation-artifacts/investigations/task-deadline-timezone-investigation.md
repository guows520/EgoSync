# Investigation: 任务 deadline 时区处理（本地墙钟被当作 UTC 存储）

## Hand-off Brief

1. **What happened.** 前端 `TaskModal` 把用户本地墙钟时间直接追加 `Z` 存为 deadline（Confirmed：`egosync-app/src/components/modals/TaskModal.tsx:26-28`），即把本地时间错误标记为 UTC；这是 Epic 9 改动前就存在的既有行为。
2. **Where the case stands.** 已诊断并结案。定位唯一的确定性消费方——每小时"临期升 Q1"后台任务（`task_deadline_watch.rs`）——其阈值以 **UTC 日末 `T23:59:59Z`** 做**字符串（日粒度）比较**，因此实际偏差不是"提前 8 小时"，而是"升级窗口锚定 UTC 日期而非用户本地日期"的日界偏移；其它按 now 比较的逻辑均用后端生成的正确 UTC 列（`updated_at` 等），不受影响。**2026-07-22 用户决策：影响小、藏得深、非本次引入，暂不修复，延后单独排期。**
3. **What's needed next.** 未来排期修复时，推荐方案 A（明确 deadline 语义为"用户本地墙钟"：前端不再贴 `Z`，后端临期阈值改用 `Local::now()` 本地日期）；方案 B（存真 UTC 瞬间）改动大且需历史数据迁移，仅在后续引入精细化时间功能时再评估。恢复本案：重新调用 bmad-investigate 并传入本案卷路径。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A（由 Epic 9 代码审查引出） |
| Date opened      | 2026-07-22 |
| Status           | Closed — 已诊断，修复延后排期（2026-07-22 用户决策） |
| System           | EgoSync（Tauri 2.x + React 18 前端 / Rust 后端 + SQLite）；单用户桌面应用 |
| Evidence sources | 源码（前端 TS + 后端 Rust）、git（`git show HEAD`）、迁移文件 |

## Problem Statement

代码审查（Epic 9）发现：`TaskModal` 的 `datetimeLocalToIso` 给本地 `datetime-local` 值直接追加 `:00Z`，把本地时间标记为 UTC 存入 `task.deadline`。初始假设：这会让"临期升 Q1 / 逾期"等按绝对时刻比较 deadline 的自动化逻辑按用户时区偏移量（东八区 8 小时）出错。本调查独立验证该假设的真实影响范围。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 前端 TaskModal.tsx | Available | 转换函数 `datetimeLocalToIso`/`isoToDatetimeLocal`（26-32 行） |
| git HEAD | Available | 确认两函数在 Epic 9 改动前逐字未变 |
| 后端 deadline 消费方 | Available | `task_deadline_watch.rs`、`task_classifier.rs`、`db/tasks.rs` 等 |
| 迁移文件 | Available | deadline 列 TEXT、无时区（`013_tasks.sql`，经子代理引用） |
| 运行时实测（不同时区/跨 UTC 午夜的实际触发时刻） | Missing | 需实机或单测构造时区场景验证日界偏移方向 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | `task_deadline_watch.rs` 临期阈值与 SQL 比较 | High | Done | 确认为 UTC 日末字符串比较 |
| 2 | 是否存在把 deadline 解析为 RFC3339 真瞬间并按 now 细粒度比较的路径 | High | Done | 未发现确定性路径；`task_classifier` 仅把 deadline 文本喂给 LLM |
| 3 | 其它 now 比较逻辑是否误用 deadline | Medium | Done | Q2 保护/大石头/能量/调度均用 `updated_at`/`last_active_at`，不涉及 deadline |
| 4 | 委派任务 deadline 格式（`normalize_delegated_deadline`）与前端格式不一致 | Medium | Open | 生成 `...T15:00:00`（无 Z），与前端带 Z 混存，需评估比较一致性 |
| 5 | 构造跨 UTC 午夜的单测确认日界偏移方向与幅度 | Medium | Open | 缺运行时证据 |

## Confirmed Findings

### Finding 1: 前端把本地墙钟时间标记为 UTC 存储

**Evidence:** `egosync-app/src/components/modals/TaskModal.tsx:26-28`
```ts
function datetimeLocalToIso(value: string): string {
  return value.length === 16 ? `${value}:00Z` : value;
}
```
**Detail:** `datetime-local` 返回本地墙钟（无时区），追加 `:00Z` 后被标记为 UTC。读回 `isoToDatetimeLocal`（30-32 行）用正则剥掉时区后缀，只取墙钟数字，因此**编辑/查看的往返显示自洽**，bug 在 UI 层不可见。

### Finding 2: 该转换为 Epic 9 改动前的既有行为

**Evidence:** `git show HEAD:egosync-app/src/components/modals/TaskModal.tsx` 中两函数与工作区逐字一致。
**Detail:** Story 9.5 仅替换选择器 UI（原生 `datetime-local` → 受控面板），未改转换逻辑；9-5 AC-4 要求 deadline 契约不变，故本次保持既有行为符合验收。缺陷不由本次改动引入。

### Finding 3: 唯一确定性消费方使用 UTC 日末字符串比较（非细粒度瞬间比较）

**Evidence:** `egosync-app/src-tauri/src/services/task_deadline_watch.rs:53-66`
```rust
// 阈值 = (now + 2天) 的 UTC 日期，固定 T23:59:59Z
format!("{:04}-{:02}-{:02}T23:59:59Z", y, m, d)
```
配合 `db::tasks::list_imminent_tasks_for_escalation(pool, &threshold)` 的 SQL `deadline <= ?1` 字符串比较。
**Detail:** `SystemTime::now()` 是真 UTC 瞬间，阈值取其日历日 + 日末。由于阈值时间恒为 `23:59:59`，比较实际退化为**按日期**判断：`deadline_date <= UTC日期(now+2天)`。deadline 的日期部分正是用户输入的本地日期。

### Finding 4: 其它 now 比较逻辑不消费 deadline

**Evidence（子代理取证，逐条 path:line）：**
- Q2 保护 `task_protection_watch.rs:25-34` 比较 `updated_at`
- Q2 提醒频控 `q2_protection_reminder.rs:38-56` 比较 `last_reminded_at`
- 大石头 `bigrock_protection.rs:47-72` 比较 `updated_at`
- 能量 `energy_calculator.rs:35-58` 比较 `last_active_at`
- 调度 `scheduler.rs` 用 `Local::now()` 触发，不用 deadline
**Detail:** 这些列均由后端 `chrono_now_pub()`（`db/settings.rs:141-163`，基于 `SystemTime::now()`）生成为真 UTC，彼此一致，与 deadline 缺陷无关。

## Deduced Conclusions

### Deduction 1: 真实影响是"UTC 日界偏移"，不是"提前 8 小时"

**Based on:** Finding 1 + Finding 3。
**Reasoning:** 临期比较为日粒度（阈值 `T23:59:59Z`）。deadline 的日期部分本就是用户本地日期，阈值的日期却由 **UTC now** 推得。二者仅在"UTC 日期 ≠ 本地日期"的时段（即接近 UTC 午夜）才不同。对 UTC+8：本地 00:00–07:59 期间 UTC 仍是前一天，阈值日期 = 本地日期 − 1，导致 2 天窗口少算一天 → 临期升 Q1 在该时段可能**滞后**触发；其余本地时段 UTC 日期 = 本地日期，无偏差。
**Conclusion:** 偏差是**日粒度、日界边缘、方向依时段**的，而非子代理所述的均匀"提前 8 小时"。子代理该结论被本调查**校正/refuted**。

### Deduction 2: "本地当 UTC"的标注错误目前是潜伏风险

**Based on:** Finding 1 + Finding 3（当前无细粒度瞬间比较消费方）。
**Reasoning:** 只要将来有代码用 `DateTime::parse_from_rfc3339(deadline)` 得到真瞬间并与 `Utc::now()` 做**小时/分钟级**比较，8 小时错标就会立刻显现为 8 小时误差。当前被日末字符串比较"掩盖"。
**Conclusion:** 现状影响有限，但错误的存储语义是一颗潜伏雷，后续任何精细化 deadline 逻辑都会踩中。

## Hypothesized Paths

### Hypothesis 1（用户初始假设）: 临期/逾期自动化按用户时区偏移量（8 小时）出错

**Status:** Refuted（部分）
**Theory:** deadline 被当 UTC，导致临期升 Q1 提前 8 小时触发。
**Supporting indicators:** deadline 确实被错标为 UTC；确有临期升 Q1 逻辑。
**Would confirm:** 存在按瞬间细粒度比较 deadline 的路径。
**Would refute:** 唯一确定性消费方按日粒度/UTC 日末字符串比较。
**Resolution:** 由 Finding 3 + Deduction 1 refuted——实际是日界偏移且方向在 UTC+8 早晨为"滞后"，非均匀提前 8 小时。假设保留记录。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 跨 UTC 午夜的实际触发时刻 | 精确确认日界偏移方向/幅度 | 构造 mock 时钟单测或实机改系统时区观察 |
| 委派任务 deadline（无 Z）与前端（带 Z）混存下的字符串比较行为 | 是否存在格式不一致导致的额外偏差 | 检查 `normalize_delegated_deadline` 产物在 `deadline <= threshold` 比较中的表现 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | `egosync-app/src/components/modals/TaskModal.tsx:26-28`（`datetimeLocalToIso` 错标 UTC） |
| Trigger | 用户在任务弹窗选择截止时间并保存 |
| Condition | 用户所在时区 ≠ UTC，且临期检查发生在"UTC 日期 ≠ 本地日期"的时段 |
| Related files | `services/task_deadline_watch.rs:53-66`、`db/tasks.rs`（`list_imminent_tasks_for_escalation` SQL）、`services/task_classifier.rs:319-374`（LLM 文本，间接）、`db/settings.rs:141-163`（`chrono_now_pub`，UTC 基准） |

## Conclusion

**Confidence:** High（对机制与影响范围）；Medium（对日界偏移的精确方向/幅度，缺运行时实测）

- **Confirmed 根因**：前端把本地墙钟当 UTC 存 deadline（既有行为，非 Epic 9 引入）。
- **Confirmed 影响范围**：唯一确定性消费方是"临期升 Q1"后台任务，按 UTC 日末字符串（日粒度）比较；其余 now 比较不碰 deadline。
- **校正用户假设**：不是均匀"提前 8 小时"，而是 **UTC 日界偏移**——UTC+8 用户在本地早晨（00:00–08:00）时段临期升级可能**滞后**约一天窗口。
- **潜伏风险**：错误的 UTC 标注一旦被细粒度瞬间比较消费，就会放大为 8 小时级误差。

## Recommended Next Steps

### Fix direction（待产品决策后择一）

- **方案 A（轻量、贴合单用户桌面定位）**：明确 deadline 语义为"用户本地墙钟"。前端不再追加 `Z`（存 `YYYY-MM-DDTHH:MM:SS` 或裸 `YYYY-MM-DD`）；后端临期阈值改用 `Local::now()` 的本地日期生成，保持日粒度字符串比较。改动小，消除日界偏移。
- **方案 B（严格正确）**：deadline 存真 UTC 瞬间（前端按本地时区转 UTC 再存），显示时转回本地；后端可安全按瞬间比较。改动大，需处理历史数据迁移与所有读写点。
- 两方案都需统一 `normalize_delegated_deadline`（委派任务）与前端的格式约定。

### Diagnostic

- 加一个带 mock 时钟/时区的单测：构造 UTC+8 本地早晨场景，断言临期升 Q1 的触发日期是否符合"本地 2 天窗口"预期，量化日界偏移方向。
- 检查历史库中既有 deadline 的格式分布（带 Z / 无 Z / 纯日期），评估任一修复方案的数据兼容性。

## Reproduction Plan

1. 将系统时区设为 UTC+8。
2. 本地时间设为某日 03:00。
3. 创建一个 deadline = 本地"后天 10:00"的 Q2 任务（未手动覆盖）。
4. 触发 `escalate_imminent_tasks`。
5. 预期（正确）：应视为 2 天内→升 Q1。观察实际：因阈值取 UTC 日期（当前 UTC 仍为前一天），窗口少算一天，可能不升级 → 暴露日界偏移。

## Side Findings

- `normalize_delegated_deadline`（`agent_engine.rs:4237-4264`）为 LLM 委派任务生成的 deadline 为 `...T15:00:00`（**无 Z**），与前端 `...Z` 格式不一致，混存于同一 TEXT 列，字符串比较语义需单独核查（Backlog #4）。
- `task_classifier.rs:319-374` 把 UTC "今天日期" 与本地墙钟 deadline 一起喂给 LLM，存在语义不一致，可能轻微影响 LLM 紧迫度判断（Deduced，非确定性）。
