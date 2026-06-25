-- Story 5.3: 时间冲突检测表
-- 记录跨角色任务的 deadline 时间冲突（±1 小时内）
CREATE TABLE IF NOT EXISTS conflicts (
    id TEXT PRIMARY KEY NOT NULL,
    task_id_a TEXT NOT NULL,
    task_id_b TEXT NOT NULL,
    role_id_a TEXT NOT NULL,
    role_id_b TEXT NOT NULL,
    conflict_time TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'detected' CHECK(status IN ('detected', 'resolved', 'dismissed')),
    resolution TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    -- 约束：task_id_a < task_id_b（字典序），确保同一对任务只有一条记录
    CHECK(task_id_a < task_id_b),
    -- 唯一约束：防止同一对任务重复插入
    UNIQUE(task_id_a, task_id_b),
    FOREIGN KEY (task_id_a) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id_b) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id_a) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id_b) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_conflicts_status ON conflicts(status);
CREATE INDEX IF NOT EXISTS idx_conflicts_task_ids ON conflicts(task_id_a, task_id_b);
