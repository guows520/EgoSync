CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY NOT NULL,
    role_id TEXT,
    category TEXT NOT NULL CHECK(category IN ('preference','task_status','cognition_update','fact')),
    content TEXT NOT NULL,
    source_conversation_id TEXT NOT NULL,
    source_message_ids TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memories_role_id ON memories(role_id);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_source_conversation_id ON memories(source_conversation_id);
CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_source_dedupe ON memories(source_conversation_id, category, content);
