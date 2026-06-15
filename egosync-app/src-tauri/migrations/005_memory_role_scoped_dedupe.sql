DROP INDEX IF EXISTS idx_memories_source_dedupe;
CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_source_dedupe ON memories(COALESCE(role_id, ''), source_conversation_id, category, source_message_ids);
