PRAGMA foreign_keys=off;

CREATE TABLE tasks_new (
    id TEXT PRIMARY KEY NOT NULL,
    owner_type TEXT NOT NULL DEFAULT 'role' CHECK (owner_type IN ('role', 'butler')),
    role_id TEXT,
    title TEXT NOT NULL,
    deadline TEXT,
    quadrant TEXT NOT NULL DEFAULT 'Q2' CHECK (quadrant IN ('Q1', 'Q2', 'Q3', 'Q4')),
    is_big_rock INTEGER NOT NULL DEFAULT 0,
    is_completed INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    protection_status TEXT NOT NULL DEFAULT 'normal',
    confidence REAL,
    manual_override INTEGER NOT NULL DEFAULT 0,
    classification_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    CHECK ((owner_type = 'role' AND role_id IS NOT NULL) OR (owner_type = 'butler' AND role_id IS NULL)),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

INSERT INTO tasks_new (
    id, owner_type, role_id, title, deadline, quadrant, is_big_rock, is_completed,
    completed_at, sort_order, protection_status, confidence, manual_override,
    classification_reason, created_at, updated_at, deleted_at
)
SELECT
    id, 'role', role_id, title, deadline, quadrant, is_big_rock, is_completed,
    completed_at, sort_order, protection_status, confidence, manual_override,
    classification_reason, created_at, updated_at, deleted_at
FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

PRAGMA foreign_keys=on;

CREATE INDEX IF NOT EXISTS idx_tasks_role_id ON tasks(role_id);
CREATE INDEX IF NOT EXISTS idx_tasks_owner ON tasks(owner_type, role_id);
CREATE INDEX IF NOT EXISTS idx_tasks_quadrant ON tasks(quadrant);
CREATE INDEX IF NOT EXISTS idx_tasks_deleted_at ON tasks(deleted_at);
CREATE INDEX IF NOT EXISTS idx_tasks_sort_order ON tasks(sort_order);
CREATE INDEX IF NOT EXISTS idx_tasks_manual_override ON tasks(manual_override);
