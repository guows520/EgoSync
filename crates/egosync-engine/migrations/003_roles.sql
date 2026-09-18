-- 角色表（主库 egosync.db）
CREATE TABLE IF NOT EXISTS roles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT '🎯',
    color TEXT NOT NULL DEFAULT '#6366F1',
    goal TEXT NOT NULL DEFAULT '',
    personality_prompt TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
    energy INTEGER NOT NULL DEFAULT 100,
    skills_config TEXT NOT NULL DEFAULT '{}',
    proactivity_level TEXT NOT NULL DEFAULT 'moderate' CHECK(proactivity_level IN ('passive', 'moderate', 'proactive')),
    archived_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_roles_status ON roles(status);