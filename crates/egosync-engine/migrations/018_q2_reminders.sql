-- Story 4.6: Q2 保护提醒记录表 — 每个任务一行，UNIQUE(task_id) 确保去重
CREATE TABLE IF NOT EXISTS q2_reminders (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    reminded_count INTEGER NOT NULL DEFAULT 1,
    last_reminded_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(task_id)
);
