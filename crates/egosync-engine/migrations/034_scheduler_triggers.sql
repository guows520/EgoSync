-- Story 17.2（FR-47）：常驻调度触发去重表 —— 收纳调度器五处循环局部内存去重
-- （last_triggered_map / last_briefing_trigger_date / last_bigrock_trigger_week /
--   last_review_trigger_week / last_bigrock_friday_check_date），
-- 重启不再失忆：同分钟重启不重复触发，停机跨过的计划时刻跳过不补发。
--
-- 单行-per-scope 形态（架构决策 #7）：UNIQUE(job, scope, tz_offset)，行存最新
-- cycle + last_triggered_at + trigger_count——与 last_triggered_map 每 role 单条、
-- bigrock UNIQUE(task_id) 的既有语义同构，天然有界不积行；判定 = cycle 相同即已触发。
--
-- 时间源三分表（架构 ⑨ 冻结，SQLite 侧仅涉其一）：
-- ①持久化时间戳一律 UTC RFC3339（last_triggered_at，Rust 侧 chrono_now_pub 写入）；
-- ②调度判定一律容器 Local（cycle/tz_offset 由 Rust 侧 chrono::Local 派生）；
-- ③前端渲染一律浏览器 TZ（组件不动）。
--
-- tz_offset 取触发时刻 Local 的 "+HH:MM"（%:z）——零新依赖覆盖显式 TZ 变更主场景；
-- DST 偏移变化视为一次 TZ 变更（同语义，部署文档 11.11 明示）。
CREATE TABLE IF NOT EXISTS scheduler_triggers (
    job TEXT NOT NULL,
    scope TEXT NOT NULL,
    cycle TEXT NOT NULL,
    tz_offset TEXT NOT NULL,
    last_triggered_at TEXT NOT NULL,
    trigger_count INTEGER NOT NULL DEFAULT 1,
    UNIQUE(job, scope, tz_offset)
);

-- 决策 #7 替换迁移：big_rock_protection_reminders（migration 026）数据迁入统一表。
-- - scope = task_id（沿用 UNIQUE(task_id) 的任务维度）
-- - cycle = last_reminded_at 按当前 Local 折算日期（SQLite 'localtime' 与 chrono::Local
--   同读容器 TZ；折算结果与旧 is_reminded_today 的 parse-UTC-比-Local-当天语义连续，
--   无法解析的历史脏值折算为空串——永不匹配任何日期，保持旧表"解析失败=今日未提醒"行为）
-- - tz_offset = 迁移时刻的 Local UTC 偏移（与上同源）
-- - last_triggered_at = last_reminded_at（原值即 UTC RFC3339，原样平移）
-- - trigger_count = reminded_count（计数保留）
-- 保真度损失（评审登记，无行为消费方）：
-- - 旧表 id / created_at（首次提醒时间）不迁移，随 DROP 消失——导出段
--   bigRockProtectionReminders.createdAt 自 034 起由 last_triggered_at（最近触发）
--   顶替，跨迁移边界的备份逐字段 diff 会见 createdAt 跳变；导出段行序由
--   created_at ASC 改为 scope ASC（data_export 注记）。
-- - 非法 TZ 名（如 "GMT+8"，部署文档 11.11 警告场景）：SQLite C 库按 POSIX
--   解析（符号反转）、chrono 回落 UTC——迁移折算与运行时判定的 cycle/tz_offset
--   可能分裂；后果仍在「TZ 变更至多一次跳过/重复」的已裁决容忍内。
INSERT INTO scheduler_triggers (job, scope, cycle, tz_offset, last_triggered_at, trigger_count)
SELECT
    'bigrock_protection',
    task_id,
    COALESCE(strftime('%Y-%m-%d', last_reminded_at, 'localtime'), ''),
    CASE
        WHEN CAST(ROUND((julianday('now', 'localtime') - julianday('now')) * 86400) AS INTEGER) < 0
        THEN '-'
        ELSE '+'
    END || printf('%02d', ABS(CAST(ROUND((julianday('now', 'localtime') - julianday('now')) * 86400) AS INTEGER)) / 3600)
        || ':' || printf('%02d', (ABS(CAST(ROUND((julianday('now', 'localtime') - julianday('now')) * 86400) AS INTEGER)) % 3600) / 60),
    last_reminded_at,
    reminded_count
FROM big_rock_protection_reminders;

-- 旧表由统一表取代（决策 #7 裁决替换迁移；q2_reminders 见 migration 018，
-- 决策未裁决纳入、现状已 DB 持久化无失忆缺陷，登记为后续统一候选——不动）。
DROP TABLE big_rock_protection_reminders;
