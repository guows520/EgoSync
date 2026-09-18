-- Story 3.3: 任务分类元数据
--
-- 1) manual_override：标记 quadrant 是否由用户显式选择。后台自动分类与每小时
--    临期检查必须跳过 manual_override = 1 的任务，避免覆盖用户决定。
-- 2) classification_reason：记录最近一次 quadrant 变更原因（自动分类原因、临期升 Q1 原因
--    或 LLM 失败降级原因）。中文短句，由后端服务写入。
-- 3) idx_tasks_manual_override：每小时定时检查筛选 manual_override = 0
--    的临期未完成任务时使用。

ALTER TABLE tasks ADD COLUMN manual_override INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tasks ADD COLUMN classification_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_tasks_manual_override ON tasks(manual_override);
