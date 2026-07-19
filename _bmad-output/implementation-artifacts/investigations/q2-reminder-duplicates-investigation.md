# Investigation: Q2 停滞提醒重复

## Hand-off Brief

1. **What happened.** 用户观察到同轮出现三条 Q2 停滞提醒，其中同一任务文本连续重复两次；实际触发源尚未由运行时证据确认。
2. **Where the case stands.** 数据库已确认两条同文提醒属于同一 task_id，但相隔 2 小时 5 分钟并跨越北京时间自然日边界；此前的并发解释已被反驳。
3. **What's needed next.** 若进入修复，应明确产品期望是“自然日一次”还是“任意 24 小时最多一次”；本调查无需再扩大取证范围。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-19 |
| Status           | Active |
| System           | Windows，React 18 + Tauri 2 + SQLite |
| Evidence sources | 用户截图文本、源码；运行日志与数据库待查 |

## Problem Statement

用户报告管家对话中连续出现三条“任务已经 3 天没动”提醒，其中前两条任务标题完全相同，第三条为另一任务。需要判断此前提出的并发竞态结论是否有明确证据。

## Evidence Inventory

| Source | Status | Notes |
| ------ | ------ | ----- |
| 用户提供的消息文本 | Partial | 能确认显示结果，缺少消息 ID、task_id 和精确时间戳 |
| 前端源码 | Available | 管家视图挂载调用提醒检查，应用启用 React.StrictMode |
| 后端源码 | Available | 提醒资格检查、消息投递、计数更新分离执行 |
| `egosync.log` | Partial | 已读取，未包含本次 Q2 提醒 info 记录，无法识别具体调用入口 |
| `egosync.db` | Available | 已只读确认任务 ID、提醒计数、通知记录与时间戳 |
| `conversations.db` | Available | 已只读确认三条消息为三次真实插入及精确时间戳 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 对照重复消息的 ID、内容与创建时间 | High | Done | 确认为两次真实插入，相隔 2 小时 5 分钟 |
| 2 | 查询同名任务对应的 task_id | High | Done | 仅有一个有效 task_id，同名任务假设被反驳 |
| 3 | 查询 q2_reminders 计数及最后提醒时间 | High | Done | 计数 2；两次提醒跨本地自然日，符合当前规则 |
| 4 | 检索日志中的 task_id 与通知 ID | High | Done | 日志缺少 Q2 info；数据库通知记录已补足投递证据 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| 2026-07-18 21:55:32 CST | 任务 `df22…afd9` 第一次生成提醒 | conversations.db / notifications | Confirmed |
| 2026-07-19 00:00:32 CST | 同一任务第二次生成提醒，距上次 2 小时 5 分钟 | conversations.db / notifications | Confirmed |
| 2026-07-19 08:55:32 CST | 任务 `18ce…af5b` 生成第三条提醒 | conversations.db / notifications | Confirmed |

## Confirmed Findings

### Finding 1: 管家视图挂载会触发 Q2 提醒检查

**Evidence:** `egosync-app/src/components/butler/ButlerView.tsx:62`

**Detail:** 无参数的 mount effect 调用 `taskService.checkQ2Reminders()`。

### Finding 2: 应用启用了 React.StrictMode

**Evidence:** `egosync-app/src/main.tsx:6`

**Detail:** 根组件位于 `<React.StrictMode>` 内，开发模式可重复执行 effect。

### Finding 3: 后端提醒生成不是原子幂等流程

**Evidence:** `egosync-app/src-tauri/src/services/q2_protection_reminder.rs:96`

**Detail:** 代码先读取提醒记录并判断今日状态，随后写入对话消息，最后在第 206 行 upsert 计数；并发调用可同时通过检查。

### Finding 4: 所有 at_risk Q2 任务按 updated_at 顺序处理

**Evidence:** `egosync-app/src-tauri/src/db/tasks.rs:517`

**Detail:** 查询返回全部符合条件任务并按 `t.updated_at ASC` 排序，没有本轮聚合或唯一化处理。

### Finding 5: 两条相同提醒属于同一任务，但并非近同时写入

**Evidence:** `C:\Users\Admin\AppData\Roaming\com.egosync.desktop\conversations.db`；消息时间 `2026-07-18T13:55:32Z` 与 `2026-07-18T16:00:32Z`

**Detail:** 两条消息 ID 分别为 `2a2ff7ca-...` 与 `e82a43c3-...`，内容完全相同；对应任务表中仅存在一个有效 task_id `df22b088-...`。

### Finding 6: 当前“每日一次”按本地自然日判断，不是滚动 24 小时

**Evidence:** `egosync-app/src-tauri/src/services/q2_protection_reminder.rs:37`

**Detail:** `is_reminded_today` 将 UTC 时间转换为本地日期后与 `Local::now().date_naive()` 比较。北京时间 21:55 与次日 00:00 日期不同，因此允许再次提醒。

### Finding 7: 第三个任务在提醒时刚超过 3 天未更新

**Evidence:** `C:\Users\Admin\AppData\Roaming\com.egosync.desktop\egosync.db`

**Detail:** 任务 `18ceecf2-...` 的 `updated_at` 为 `2026-07-16T00:50:39Z`，提醒时间为 `2026-07-19T00:55:32Z`，相差 3 天 4 分 53 秒。

## Deduced Conclusions

### Deduction 1: 当前实现具备生成“前两条相同、第三条不同”的完整条件

**Based on:** Findings 1–4

**Reasoning:** 两次近同时检查会先处理相同的首个任务并都通过今日检查；两条执行链拉开后，领先者可先处理第二个任务，落后者随后因今日记录而跳过。

**Conclusion:** 并发竞态是与现象高度吻合的可执行解释，但尚未由本次事件的 task_id、时间戳或日志确认。

## Hypothesized Paths

### Hypothesis 1: React.StrictMode 触发两次并发检查，导致同一 task_id 重复投递

**Status:** Refuted（针对本次事件）

**Theory:** 管家视图 mount effect 被重复执行，两次后端调用在首个任务的检查与计数之间交错。

**Supporting indicators:** 源码中同时存在 StrictMode、mount effect 和非原子提醒流程，消息排列与该时序一致。

**Would confirm:** 日志或数据库显示同一 task_id 在极短间隔内生成两条相同消息，且同日 reminded_count 增加两次。

**Would refute:** 两条相同消息对应不同 task_id，或日志表明只有一次提醒检查。

**Resolution:** 两条相同消息相隔 2 小时 5 分钟，且跨越本地自然日；不符合一次近同时并发写入的时间特征。代码中的并发漏洞仍客观存在，但不是本次事件的原因。

### Hypothesis 2: 数据库中存在两个标题相同但 task_id 不同的任务

**Status:** Refuted

**Theory:** 提醒并未重复处理同一任务，而是分别处理了两个同名任务。

**Supporting indicators:** 任务标题没有唯一约束，消息文本不包含 task_id。

**Would confirm:** tasks 表返回两个标题相同、均为 at_risk Q2 的不同 ID。

**Would refute:** 该标题只对应一个有效 task_id，而对话库存在两次相同插入。

**Resolution:** tasks 表中该标题仅对应一个有效 task_id `df22b088-...`。

### Hypothesis 3: 自然日限频在午夜重置，导致短间隔重复提醒

**Status:** Confirmed

**Theory:** 首次提醒发生在午夜前，第二次检查发生在午夜后；虽然仅间隔约两小时，日期比较已经认为是新的一天。

**Supporting indicators:** 数据库时间戳与 `is_reminded_today` 的本地日期比较完全一致。

**Would confirm:** 已由消息时间戳、通知时间戳、提醒计数和源码共同确认。

**Would refute:** 两次消息处于同一本地日期；实际证据相反。

**Resolution:** Confirmed。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 两次检查的具体入口 | 不影响自然日边界根因，但无法说明用户是重开页面、重启应用还是其他入口触发 | 当前日志没有 Q2 检查开始记录；如需追入口，应先经用户确认增加诊断日志 |

## Source Code Trace

| Element | Detail |
| ------- | ------ |
| Error origin | `services/q2_protection_reminder.rs:90`，`process_single_task` |
| Trigger | `components/butler/ButlerView.tsx:62` 的 mount effect |
| Condition | 两次调用在提醒记录更新前同时通过“今日未提醒”检查 |
| Related files | `src/main.tsx`、`db/q2_reminders.rs`、`db/tasks.rs` |

## Conclusion

**Confidence:** High

本次前三条提醒的排列由自然日限频语义造成，而非同轮并发。第一个任务在北京时间 7 月 18 日 21:55 首次提醒，午夜后 00:00 因日期变更再次获得提醒资格；第二个任务到 08:50 才达到 3 天未更新阈值，并在 08:55 生成第三条。StrictMode 并发漏洞依旧存在于代码中，但已由本次运行时数据反驳为此次事件原因。

## Recommended Next Steps

### Fix direction

若产品期望避免用户短时间看到相同提醒，应把自然日判断改为滚动 24 小时（或设置最小间隔），并另行修补非原子并发幂等漏洞；两者是独立问题。

### Diagnostic

本次根因已确认。仅当还需识别 00:00 那次检查的具体调用入口时，才需要增加诊断日志；现有日志不足以确认入口。

## Reproduction Plan

在隔离测试数据库中并发调用两次提醒检查，使用两个按 updated_at 排序的 at_risk Q2 任务，断言消息条数和 reminded_count；当前未执行。

## Side Findings

- `q2_reminders` 的 task_id 冲突处理只增加计数，不能撤销已经写入 conversations.db 的重复消息。
