-- 任务表（主库 egosync.db）
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY NOT NULL,
    role_id TEXT NOT NULL,
    title TEXT NOT NULL,
    deadline TEXT,
    quadrant TEXT NOT NULL DEFAULT 'Q2' CHECK (quadrant IN ('Q1', 'Q2', 'Q3', 'Q4')),
    is_big_rock INTEGER NOT NULL DEFAULT 0,
    is_completed INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    protection_status TEXT NOT NULL DEFAULT 'normal',
    confidence REAL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    deleted_at TEXT,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tasks_role_id ON tasks(role_id);
CREATE INDEX IF NOT EXISTS idx_tasks_quadrant ON tasks(quadrant);
CREATE INDEX IF NOT EXISTS idx_tasks_deleted_at ON tasks(deleted_at);
CREATE INDEX IF NOT EXISTS idx_tasks_sort_order ON tasks(sort_order);
