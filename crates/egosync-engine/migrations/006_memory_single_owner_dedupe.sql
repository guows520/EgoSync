DROP INDEX IF EXISTS idx_memories_source_dedupe;

DELETE FROM memories
WHERE rowid NOT IN (
    SELECT rowid
    FROM (
        SELECT
            rowid,
            ROW_NUMBER() OVER (
                PARTITION BY source_conversation_id, category, source_message_ids
                ORDER BY CASE WHEN role_id IS NULL THEN 0 ELSE 1 END DESC, created_at DESC, rowid DESC
            ) AS rn
        FROM memories
    )
    WHERE rn = 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_source_dedupe
ON memories(source_conversation_id, category, source_message_ids);
