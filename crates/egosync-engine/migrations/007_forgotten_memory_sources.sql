CREATE TABLE IF NOT EXISTS forgotten_memory_sources (
    id TEXT PRIMARY KEY NOT NULL,
    role_id TEXT,
    category TEXT NOT NULL CHECK(category IN ('preference','task_status','cognition_update','fact')),
    content TEXT NOT NULL,
    normalized_content TEXT NOT NULL,
    source_conversation_id TEXT NOT NULL,
    source_message_ids TEXT NOT NULL,
    forgotten_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_forgotten_memory_sources_source
ON forgotten_memory_sources(source_conversation_id, category, source_message_ids);

CREATE INDEX IF NOT EXISTS idx_forgotten_memory_sources_role_id
ON forgotten_memory_sources(role_id);

CREATE INDEX IF NOT EXISTS idx_forgotten_memory_sources_forgotten_at
ON forgotten_memory_sources(forgotten_at);
