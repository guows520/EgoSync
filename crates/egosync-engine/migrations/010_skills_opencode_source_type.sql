PRAGMA foreign_keys=OFF;

CREATE TEMP TABLE IF NOT EXISTS skill_role_bindings_backup AS
SELECT skill_id, role_id, created_at FROM skill_role_bindings;

CREATE TABLE IF NOT EXISTS skills_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source_type TEXT NOT NULL CHECK(source_type IN ('custom', 'opencode')),
    managed_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO skills_new (id, name, description, source_type, managed_path, content_hash, created_at, updated_at)
SELECT id, name, description, source_type, managed_path, content_hash, created_at, updated_at
FROM skills;

DROP TABLE skills;
ALTER TABLE skills_new RENAME TO skills;

CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_content_hash ON skills(content_hash);
CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_name_source_type ON skills(name, source_type);
CREATE INDEX IF NOT EXISTS idx_skills_source_type ON skills(source_type);

INSERT OR IGNORE INTO skill_role_bindings (skill_id, role_id, created_at)
SELECT skill_id, role_id, created_at FROM skill_role_bindings_backup;
DROP TABLE skill_role_bindings_backup;
CREATE INDEX IF NOT EXISTS idx_skill_role_bindings_role_id ON skill_role_bindings(role_id);

PRAGMA foreign_keys=ON;
