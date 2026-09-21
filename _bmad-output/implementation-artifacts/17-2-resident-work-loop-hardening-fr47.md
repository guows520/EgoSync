---
title: '17-2 常驻工作循环硬化（FR-47）'
type: 'feature'
created: '2026-09-21'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '76122291b022f20e6f6bfbc7170b5a163ee65f78'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-17-context.md'
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 调度循环的触发去重是循环局部变量（规划口径"三处"，代码实测**五处**：last_triggered_map / last_briefing_trigger_date / last_bigrock_trigger_week / last_review_trigger_week / last_bigrock_friday_check_date，scheduler.rs:359-371），重启即失忆——云端 7×24 场景同分钟重启会重复触发简报/建议/大石头提醒；大石头每日保护去重虽已入库（big_rock_protection_reminders 表）但独占一套机制。FR-47 要求重启不重复不遗漏。

**Approach:** 新增 `scheduler_triggers` 持久化小表（migration 034，主库 MIGRATOR）统一收纳五处内存去重，并按架构决策 #7 将 bigrock 表**替换迁移**入统一表（含数据迁移）；触发判定改读表；时间源三分表口径以代码注释+架构文档双重标注；web e2e 断言无客户端常驻生成与建议待确认态。

## Boundaries & Constraints

**Always:**
- 时间源三分表（架构 ⑨ 冻结）：①持久化时间戳一律 `chrono_now_pub` UTC RFC3339（禁改 Local）；②调度判定一律容器 `Local::now()` 语义零改动；③前端渲染浏览器 TZ（组件不动）。三分边界代码注释标注。
- 60s tick、HH:MM 分钟级精确匹配、spawn 前置位（LLM 瞬时失败当天不重试，deferred-work 已登记）、停机跨过计划时刻 ⇒ 跳过不补发——全部保持既有语义。
- 生成侧去重（briefings `date UNIQUE`、weekly_reviews `week_start UNIQUE`、建议近 7 天标题查重）不动——与新表是触发侧/生成侧两层去重。
- 建议 INSERT 恒 `'pending'`、确认唯一走 `suggestion_confirm` 原子守卫——确认纪律零改动（FR-11）。
- 已应用 migration 001-033 一字不改；032/033 校验钉（pool.rs:198/205）及其测试不动。

**Never:**
- 不动 `q2_reminders`（migration 018）——决策 #7 未裁决纳入，现状已 DB 持久化无失忆缺陷；登记为后续统一候选。
- 不做启动补检测/补发（错过窗口不回溯，PRD 假设保持）。
- 不改桌面/server 调度启动接线（lib.rs:468 / bootstrap.rs:328）与 spawn 结构。
- 不引入新依赖（tz 维度用 Local UTC 偏移串，不加 iana-time-zone crate）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 同分钟重启 | 简报触发后 60s 内重启进程 | 读表识别当日已触发，不再重复触发 | 表读失败 ⇒ warn 降级跳过本 tick（不 panic） |
| 停机跨过计划时刻 | 08:00 停机、09:00 恢复 | 该次跳过，不补发不堆积 | N/A（诚实代价） |
| TZ 变更 | 改 TZ env 重启 | 键含时区维度 ⇒ 首个周期至多一次跳过或重复；DST 偏移变化同语义（文档明示） | N/A |
| bigrock 数据迁移 | 033→034 升级，旧表含 count/last_reminded_at | 迁入统一表，`is_reminded_today` 语义连续、计数保留 | 空表迁移幂等 |
| 旧导出包导入 | 导出 JSON 含 bigRockProtectionReminders 段 | 正常落统一表（导出形状保持兼容） | 形状不符 ⇒ 既有导入错误路径 |
| 角色归档/删除 | 该角色 work_loop 触发行 | 清理（对齐既有 retain 语义） | 删除失败 warn 不阻断 |
| LLM 失败 | 触发已置位、生成降级失败 | 当天不重试（现状语义保持） | 既有 warn 降级 |

</frozen-after-approval>

## Code Map

- `crates/egosync-engine/src/services/scheduler.rs:346-680` — `spawn_scheduler` 循环本体；五处内存去重 :359-371；判定点 :424-429（work_loop，键=role_id→"YYYY-MM-DD HH:MM"）、:481-490（简报，today_date+HH:MM 匹配）、:544、:584-594（周复盘 iso_week_key :676）、:625；`Local::now()` :395；retain 清理 :476。**改动主战场**
- `crates/egosync-engine/src/services/bigrock_protection.rs:36` — `is_reminded_today`（parse UTC→Local 比当天）；`db/big_rock_protection_reminders.rs` — upsert(count++/刷新时间)/get/delete，UNIQUE(task_id)
- `crates/egosync-engine/migrations/` — 主库 MIGRATOR 001-033 连续，**下一个 034**（"032"系规划陈旧信息）；026=bigrock 表、018=q2_reminders（不动）、023/025=简报/复盘
- `crates/egosync-engine/src/db/pool.rs:12,55-63,139-187,550-570` — `sqlx::migrate!` 嵌入、run_migrations、**过滤版 MIGRATOR 模拟升级前状态的数据迁移测试范式（直接可抄）**；校验钉 :198/205 勿动
- `crates/egosync-engine/src/db/settings.rs:146` — `chrono_now_pub`（UTC RFC3339 手写实现，全库 30+ 调用）
- `crates/egosync-engine/src/services/data_export.rs:189-227,1048-1069` — bigrock/q2 两表导出/导入落点（**改 bigrock 段为统一表合成，形状不变**；q2 段不动）
- `crates/egosync-engine/src/db/suggestions.rs:17` — INSERT 恒 'pending'；`commands/suggestion.rs:17` — `suggestion_list_pending`（e2e 断言用）
- `crates/egosync-engine/src/commands/settings.rs` — briefing_time 等 app_settings 读写命令（e2e 播种用）；`scheduler.rs:79-99` — `get_trigger_times` 读 app_settings 键（JSON 数组 HH:MM）
- `crates/egosync-engine/src/services/briefing_generator.rs:53` / `suggestion_generator.rs:170,304` — 生成路径；无 provider/LLM 失败均 `Ok(false)` 降级不落库 ⇒ e2e 需 LLM stub；`llm/openai.rs:60` — POST `{base_url}/chat/completions` SSE 流
- `server/src/bootstrap.rs:328` / `egosync-app/src-tauri/src/lib.rs:468` — 双端调度启动（**零改动**）
- `egosync-app/tests/e2e/` — `wdio.web.conf.ts`（onPrepare 起 server+node:sqlite 播种先例 :175-249）、`helpers/web-helper.ts:114,175,219`（openWebAppAndLogin 61s 限流等待 / webInvoke 浏览器内 fetch / seedRoleViaCommand）、`web-specs/` 新 spec 挂点
- `docs/user-guide/11-云端自托管部署.md:245-255` — 11.11 时区语义（17.1 已预写 17.2 句，核对+补 DST 句）；`_bmad-output/planning-artifacts/architecture.md` ⑨ L2661-2670、决策 #7 L2509（双重标注核对）
- `.github/workflows/release.yml:17-34` — release note 由 conventional commit 自动生成（桌面行为变化注记 ⇒ commit 标题 `!:` + BREAKING CHANGE footer，无手写落点）

## Tasks & Acceptance

**Execution:**
- [x] `crates/egosync-engine/migrations/034_scheduler_triggers.sql` — 建表（job/scope/cycle/tz_offset/last_triggered_at/trigger_count，`UNIQUE(job,scope,tz_offset)`，行存最新 cycle）+ big_rock_protection_reminders 数据迁入（scope=task_id、cycle=last_reminded_at 按当前 Local 折算日期、count 保留）+ DROP 旧表 — 决策 #7 替换迁移
- [x] `crates/egosync-engine/src/db/scheduler_triggers.rs`（新）— get / record（upsert 前移 cycle 与 last_triggered_at；count 按 job 语义：bigrock_protection 递增累计（对齐旧表 count++）、其余恒 1；同 cycle 幂等）/ prune（仅 job='work_loop' 按活跃角色集清理，勿伤 bigrock 的 task 作用域行）+ :memory: 单测 — 触发侧唯一存取层
- [x] `crates/egosync-engine/src/services/scheduler.rs` — 五处内存态替换为表存取；判定抽为可注入 `now: DateTime<Local>` 的纯函数（含 tz_offset 键）；保留 tick/分钟匹配/spawn 前置位语义，retain 清理改走 prune（仅 work_loop 行）；三分表代码注释 — FR-47 核心
- [x] `crates/egosync-engine/src/services/bigrock_protection.rs`（收编 `db/big_rock_protection_reminders.rs`）— `is_reminded_today`/记录改走统一表 job='bigrock_protection'，count 语义经 trigger_count 保留 — 替换迁移消费侧
- [x] `crates/egosync-engine/src/services/data_export.rs` — bigrock 导出段改由统一表合成（JSON 形状不变）、导入兼容新旧两落点 — 导出包跨形态互导
- [x] engine tests（scheduler.rs / db/scheduler_triggers.rs / pool.rs 数据迁移测试抄 :550-570 范式）— 同分钟重启不重复、停机跳过、TZ 变更至多一次、bigrock 迁移数据保留 — 判定级全覆盖
- [x] `egosync-app/tests/e2e/web-specs/web-resident-loop.spec.ts`（新）— in-spec 起 LLM SSE stub（openai 兼容固定响应）+ webInvoke 建 openai_compatible 默认配置 + 设 briefing_time/触发时间键=now+1min + role_create 直调建带 goal 角色（评审修正：原任务行写 seedRoleViaCommand，实现因建议生成的「无目标/任务/记忆短路」改为带 goal 直调）→ `about:blank` 断连等待触发 → 断言 `briefing_get_latest` 当日简报可见、`suggestion_list_pending` ≥1 且 UI 待确认态 → 清理（删配置/复位/删角色）— 常驻+确认纪律 e2e
- [x] `docs/user-guide/11-云端自托管部署.md` + architecture.md — 11.11 核对既有句、补 DST 偏移变化句；⑨/决策 #7 与实现对齐（追加注记式） — NFR-C7 文档化
- [x] 收口提交携带 BREAKING CHANGE 落点 — commit 标题 `feat(engine)!: …` + footer `BREAKING CHANGE: 调度触发去重持久化（scheduler_triggers）——重启窗口内不再重复触发当日简报/工作循环`（release note 由 conventional commit 自动生成，见 `release.yml:17-34`；AC「桌面行为变化以 BREAKING CHANGE 注记」的唯一机制，本构建流程不代提交，收口提交时人工落实）

**Acceptance Criteria:**
- Given 无任何客户端连接的 server 运行过计划时刻，when 用户随后打开 WEB 端，then 当日简报已生成落库并可见、生成的建议 status='pending'（e2e 断言，FR-11 不豁免）
- Given 简报触发后同分钟重启，when 循环再跑，then 不再触发当日简报（引擎测试注入 now 断言；桌面同引擎同步生效，release note 以 BREAKING CHANGE 注记"重启窗口内不再重复触发"）
- Given 停机跨过计划时刻，when 恢复，then 该次跳过、不补发不堆积
- Given TZ env 变更后重启，when 首个周期，then 至多一次跳过或重复（键含 tz 维度）
- Given 033 库升级 034，when 迁移，then bigrock 既有去重与计数无损迁入、q2_reminders 原样不动
- Given 含 bigRockProtectionReminders 段的旧导出包，when 导入新实例，then 落统一表且导出往返形状不变
- Given 全量回归，when `npm run test:all` + e2e `npm run test:web` + `npm run build`，then 全绿

## Implementation Notes

（实现时填入：五处内存态实测与规划"三处"口径的偏差——last_bigrock_trigger_week/last_bigrock_friday_check_date 与前三处同类失忆缺陷，按 FR-47 意图一并纳入；此发现已同步 architecture.md 注记。）

**实现过程记录（2026-09-21）**：
- 实现子代理在引擎侧改造完成后因磁盘瞬时写满（ENOSPC）报错终止——工作区改动完好，主会话以全量 diff 审读后接管收尾（e2e + 文档 + 验证），无返工。
- 引擎侧落地：`db/scheduler_triggers.rs`（has_triggered / record_trigger / get_trigger / prune_work_loop_rows + 六个 job 常量 + SCOPE_GLOBAL）；判定纯函数 `should_trigger_*` 族 + `TriggerClock`（单一 now 派生 hhmm/date/iso_week/tz_offset/weekday/work_loop_cycle，测试注入 Local 时刻）；五处内存态判定/置位全部改走统一表；`prune_work_loop_rows` 仅清理 work_loop 行（对齐原 retain 语义）。
- migration 034：建表 + big_rock_protection_reminders 数据迁入（scope=task_id、cycle 由 last_reminded_at 按迁移时刻 Local 折算日期、trigger_count=reminded_count 保留）+ DROP 旧表；pool.rs 新增过滤版 MIGRATOR 数据迁移测试（<34 起旧库 → 播种旧表行 → 全量升级 → 断言行迁移与 count 保留）。
- bigrock 消费侧：`is_reminded_today` 删除（等价折算入统一表判定），process_single_bigrock 直接判 `has_triggered`；`db/big_rock_protection_reminders.rs` 与 `models/big_rock_protection_reminder.rs` 删除收编。
- data_export：导出段 `bigRockProtectionReminders` JSON 形状不变（由统一表 job='bigrock_protection' 行合成 taskId/remindedCount/lastRemindedAt/createdAt）；导入段兼容该形状写入统一表。
- e2e（主会话补齐）：`web-resident-loop.spec.ts`——in-spec node:http SSE stub（按 system prompt「主动建议生成器」判别简报/建议两形态，双 delta 分片 + finish_reason stop + [DONE]）；LLM 配置 networkLocation='internal'（reqwest no_proxy 直连 127.0.0.1，规避环境代理变量）；计划时刻取下一分钟（距当前 <10s 时顺延一分钟防播种竞态）；断连用 about:blank；回连免重登（Cookie 持久）；清理含设置复位/删配置/删角色（唯一活跃角色守卫失败降级归档）。已知窗口：运行跨当地 08:00 时当日简报已被默认时刻触发（cycle=日期），标记简报不生成——诚实代价，spec 头注已登记。
- 文档：部署指南 11.11 时区段落补 DST 偏移跳变句 + 升级重启不重复触发句 + 桌面行为变化注记；architecture.md ⑨ 段与决策 #7 追加落地注记（五处实测、034 编号、q2 不动口径）。
- 测试新增计数：engine 827 → 847（+20，含统一表单测/判定矩阵/TZ 变更/bigrock 迁移/TriggerClock）。

## Spec Change Log

（2026-09-21 评审回环：冻结块（Intent/Boundaries/I-O Matrix）零变更；冻结区外的变更——任务区补 BREAKING CHANGE 收口项与 seedRole 文本对齐、Implementation Notes/Triage Log/Verification 记录填充、e2e 用例从 2 条扩为 3 条（新增同分钟重启执行级验收，G2 评审 patch）——均属实现与记录层，无目标/边界偏移，未触发重推导。）

## Review Triage Log

（2026-09-21 三层评审：盲扫 13 项 / 边缘 14 项 / 缺口 4+1 项；同位置同主张三层合并为一行，逐项裁决如下）

| # | 发现（层） | 裁决 | 证据/理由 | 路由 |
|---|-----------|------|----------|------|
| 1 | weekly 去重测试被星期门短路：断言用周三 clock 对 day="1"，weekday 门在 has_triggered 前短路，去重分支零执行；紧邻「非配置星期不触发」断言与上一条完全重复（盲扫/边缘/缺口三层同报） | **high** | scheduler.rs:1470 实证；mutation 演示删去重反转三测试仍绿；FR-47 头条场景对 weekly job 无真路径断言 | patch G1 |
| 2 | spawn_scheduler 接线（should→record→spawn 链）无任何执行级测试：删 briefing 块 record 调用，引擎测试与 e2e 全绿（边缘/缺口） | **high** | 全仓 spawn_scheduler 仅两生产启动 + 字符串匹配元测试；e2e 单生命周期无重启，「同分钟重启不重复」核心验收在执行层无覆盖 | patch G2 |
| 3 | e2e 清理在 about:blank 失败路径全灭：webInvoke 走相对 URL fetch，断连页不可解析 → catch 吞 → 残留污染同轮后续 spec（盲扫/边缘） | **medium** | web-helper.ts:175 相对 URL 语义；失败后残留 stub 配置/角色/被改设置 | patch G3 |
| 4 | e2e 超时预算 150s < 内部等待最坏和 ~205s（盲扫/边缘） | **medium** | pause 最长 135s + 徽标 30s + online 30s + 简报轮询 30s + 渲染 20s；慢环境以测试超时而非内部明确报错收场 | patch G3 |
| 5 | stub listen 未挂 error 事件：18081 被占时 unhandled error 崩 runner（边缘） | **low** | node http 语义 | patch G3 |
| 6 | 播种 10s 守卫偏紧：慢环境 7 次 webInvoke 可能溢入目标分钟（盲扫） | **low** | 顺延阈值与播种耗时的竞态 | patch G3 |
| 7 | 午夜窗口 briefing date 与断言 todayStr 差一天（23:58 触发 + 00:0x 断言）（盲扫） | **low** | todayStr 取回连时刻而非目标分钟日期 | patch G3 |
| 8 | 08:00 默认简报假失败窗口（每日约 60s，头注已登记）（缺口 Other） | **low** | boot 分钟落在 07:59-08:00 时默认时刻先烧掉当日 cycle | patch G3 |
| 9 | work_loop 置位失败注释「烧掉本槽位」与行为相反：record 失败 continue 无写入，下 tick 会重试（盲扫） | **low** | scheduler.rs:562；简报分支「跳过本 tick」表述才准确 | patch G4 |
| 10 | migration 034 静默丢弃 id/created_at，导出 createdAt 用 last_triggered_at 顶替，保真度损失未登记（盲扫） | **low** | 跨 034 边界备份 diff 会见 createdAt 跳变；无行为消费方 | patch G5 |
| 11 | 导出行序 created_at ASC → scope ASC + id/createdAt 合成值，与「形状不变」承诺有字面出入（盲扫） | **low** | 字段同形，行序与合成值语义变化未注明 | patch G5 |
| 12 | 迁移 SQLite localtime 与运行时 chrono::Local 对非法 TZ（如 GMT+8）解析分裂（盲扫） | **low** | 后果仍在「至多一次跳过/重复」已裁决容忍内；注释一句收口 | patch G5 |
| 13 | sprint-status 仍 in-progress，spec 已 in-review（盲扫） | **medium** | yaml 头注自身工作流要求同步 review | patch G6 |
| 14 | BREAKING CHANGE 发布注记无任务落点（AC 有要求、清单无 checkbox）；任务行 seedRoleViaCommand 文本与实际直调漂移（盲扫/边缘） | **medium** | release note 唯一机制=commit `!:` + footer，无 checklist 会漏 | patch G7 |
| 15 | DoD 验证记录未写回：Verification 段只有 expected 无实际结果（盲扫） | **medium** | 全链实际已跑全绿但工件无记录，违反项目验证记录纪律 | patch G8 |
| 16 | epic-17-context.md 仍写「三处」与本故事/architecture 五处口径矛盾（盲扫） | **low** | agent-context 编译产物，按分诊规则固定 defer | defer D1 |
| 17 | test:web 不进任何 CI workflow（缺口） | **medium** | 17.3 明文范围（CI 三链路含 web e2e 冒烟）；空窗期缺口真实 | defer D2 |
| 18 | weekly/friday 循环块 job 常量接线无执行级验证（两个结构相同块常量混用检测不出）（缺口） | **low** | hypothetical wiring slip 非观测缺陷；闭合需注入化重构或周级 e2e，超出本故事验证级别 | defer D3 |
| 19 | prune 每 tick 一次 DELETE 写放大（盲扫） | **low** | 空匹配 DELETE 不产生脏页/WAL 帧，空事务微秒级；1C1G 无感 | 驳回 |
| 20 | 全局 job 陈旧 tz 行无清理路径「无限积」（盲扫） | **low** | 速率 ≈8 行/年（DST 跳变），十年量级 10²，无行为影响 | 驳回 |
| 21 | bigrock 判定与置位间跨午夜 → cycle 存昨天双提醒（边缘） | **low** | 单任务处理毫秒级，命中午夜瞬间概率趋零；不可达窗口 | 驳回 |
| 22 | 处理中任务删除 → 孤儿 bigrock 行累积（边缘） | **low** | 任务删除已有清理路径（tasks.rs 改动行）；竞态窗口毫秒级且 UUID 不复用，无害垃圾行 | 驳回 |
| 23 | 导入行带非规范时间戳变体 → latest 行选错（边缘） | **low** | 本产品导出包恒为 chrono_now_pub 规范串；仅手工篡改输入可触发 | 驳回 |
| 24 | 导入 lastRemindedAt 未来时间 → 同日重提醒一次（边缘） | **low** | 需源实例时钟错误；后果一条多余提醒非数据损坏；clamp 属防御性 guard | 驳回 |
| 25 | 导入包重复 taskId → 后者静默覆盖（边缘） | **low** | 源表 UNIQUE(task_id)，本产品不产生重复条目 | 驳回 |
| 26 | 活跃角色数超 SQLite bind 上限 → prune 每 tick 失败（边缘） | **false** | SQLite ≥3.32 变量上限 32766（本仓 libsqlite3-sys 版本核实在其上）；个人多智能体应用角色量级不可达 | 驳回 |
| 27 | TZ 变更后 bigrock 新键行 trigger_count 从 1 重来（边缘） | **low** | bigrock count 无行为消费方（仅导出统计），新键重置只影响统计值 | 驳回 |
| 28 | 迁移测试多 now() 调用点午夜/DST flake（边缘） | **low** | CI 时刻均匀分布下概率 ~10⁻⁵，多年一遇级 | 驳回 |
| 29 | spec 硬编码引擎默认 moderate/proactive 时间做清理复位，默认漂移即写入陈旧值（盲扫） | **low** | 已有 scheduler_get_times/settings_get_schedule 读取命令，快照/恢复零成本 | patch G3 |
| 30 | stub 的 req/res 流错误无监听，客户端中途断开即未处理异常崩 wdio worker（边缘） | **low** | node http 语义 | patch G3 |

**分组汇总**：patch 组 G1-G8（G1 weekly 测试补强 / G2 e2e restart leg / G3 e2e 健壮性批 / G4 注释修正 / G5 迁移导出注释批 / G6 sprint-status 同步 / G7 spec 任务区补 BREAKING CHANGE 落点+文本对齐 / G8 Verification 结果写回）；defer 组 D1-D3；驳回 10 项（#19-28）。无 intent_gap、无 bad_spec → 不回环，patch/defer 正常处理。实现子代理因先前 ENOSPC 终止且不可续（dispose 失败）——按规则由主会话自行打 patch。

**Patch 执行记录（2026-09-21）**：G1 weekly 测试改同周一时刻重断言（真路径）+周三断言正名为「非配置星期」；G2 新增 e2e 第三用例「同分钟重启不重复触发」（日志实证 restartedInMinute 路径走通）；G3 批量落地——超时按实际等待派生、stub listen/req/res 错误处理、播种改「无时间依赖先行 + 溢出守卫顺延」、todayStr 取目标分钟日期（午夜守卫）、08:00 守卫（当日 cycle 已消费即跳过）、清理前先回应用页 + 快照复位（scheduler_get_times/settings_get_schedule）；G4 置位失败注释改为与行为一致；G5 三处注释（034 保真度/非法 TZ 分裂、导出段非逐字节同形）；G6 sprint-status → review；G7 任务区补 BREAKING CHANGE 收口项 + seedRole 文本对齐；G8 Verification 实际结果写回（全部命令真实执行记录）。patch 后复验：engine 847/0、e2e 最终全量 4 spec 全过 exit 0、重启腿日志证据在案。G3 执行中发现并同步修复 spec 评审期间暴露的两处实现笔误：SuggestionWithRole camelCase 过滤（roleId）与并行 worker 流式瞬态（等待式断言）。

## Design Notes

- **单行-per-scope 形态**：`UNIQUE(job, scope, tz_offset)`，行存最新 cycle + last_triggered_at + trigger_count——与 last_triggered_map 每 role 单条、bigrock UNIQUE(task_id) 的既有语义同构，天然有界不积行；判定 = `row.cycle == current_cycle`。work_loop 的 cycle 沿用 trigger_key 原样 `"YYYY-MM-DD HH:MM"`（同分钟重启去重精确成立，同日不同时刻照常触发）。
- **tz_offset 取触发时刻 Local 的 `%:z`（"+08:00"）**：零新依赖覆盖显式 TZ 变更主场景；DST 偏移变化视为一次 TZ 变更（同语义，文档明示），属"宁可一次跳过/重复"已裁决权衡。
- **置位保持 spawn 前**：deferred-work #225 登记的既有权衡，持久化后语义不变（失败当天烧掉槽位），生成侧 UNIQUE 兜底防重复落库。
- **e2e 的 LLM stub**：webInvoke 走浏览器内 fetch（需登录页在场），断连用 `about:blank` 导航实现（SSE 断开、零客户端连接）；stub 返回 SSE 分片固定内容，建议响应须满足 `parse_suggestions_response` 的 JSON 形状。

## Verification

**Commands:**
- `cd egosync-app && npm run test:all` — expected: vitest + 双 cargo test 全绿（含新表/判定/迁移测试）
- `cd egosync-app/tests/e2e && npm run test:web` — expected: 4 个 web spec 全绿（含新 web-resident-loop）
- `cd egosync-app && npm run build` — expected: tsc 零类型错误

**Verification Results（2026-09-21，实际执行记录）：**
- engine `cargo test`（crates/egosync-engine）：**847 passed / 0 failed**（基线 824，+23；评审 patch 后复跑仍 847/0，含 G1 修复后的 weekly 同 cycle 真路径断言）
- server `cargo test`：全绿（exit 0，含 web_entry/auth/集成测试）
- 桌面壳 `cargo test`（egosync-app/src-tauri）：全绿（exit 0）
- `npx vitest run`（egosync-app）：**877 passed / 67 文件 / 0 failed**
- `npm run build`：tsc 零类型错误 + vite 构建成功（15.5s，chunk 警告为既有基线非阻塞）
- `npm run test:web`（评审 patch 后最终轮，输出重定向到文件避免管道 SIGPIPE 干扰 onComplete 清理）：**4 spec 全过 exit 0** —— events 1 / reconnect 3 / **resident-loop 3（常驻简报+建议生成、FR-11 待确认态、同分钟重启执行级）** / streaming 4；重启腿日志实证：目标分钟内建议写入（.083）→ kill+重启（.145，同一分钟内）→ 新进程首 tick 同分钟判定 → **无第二条建议**（持久化去重生效，基线内存实现会二连发）
- 过程诚实登记：评审轮全量 e2e 的两次失败均定位为测试基建问题而非产品缺陷——①spec 自身 bug（SuggestionWithRole 为 camelCase 序列化，过滤用了 snake_case role_id；修复）+②并行 worker 共享管家会话的流式瞬态隐藏建议卡（ChatStream showActions=!isStreaming，等待式断言修复）+③运维失误（wdio 输出经 `| head` 管道致 SIGPIPE 提前终止、onComplete 清理未执行 → 僵尸服务器持旧库 inode 污染下一轮——杀僵尸后干净复跑全绿）

**Manual checks (if no CLI):**
- 升级演练：取含 bigrock 旧表数据的库跑 034，核对迁入行与 count；部署指南 11.11 与 architecture ⑨ 注记在案（已由 pool.rs 过滤版 MIGRATOR 迁移测试覆盖执行，见 `scheduler_triggers_migration_preserves_bigrock_data_and_drops_legacy_table`）
