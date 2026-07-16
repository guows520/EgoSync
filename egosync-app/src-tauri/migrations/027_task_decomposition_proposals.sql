CREATE TABLE IF NOT EXISTS task_decomposition_proposals (
    id TEXT PRIMARY KEY NOT NULL,
    role_id TEXT NOT NULL,
    source_conversation_id TEXT NOT NULL,
    task_summary TEXT NOT NULL,
    items_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'kept_single')),
    single_task_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    resolved_at TEXT,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (single_task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS task_decomposition_items (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    title TEXT NOT NULL,
    deadline TEXT,
    created_task_id TEXT,
    FOREIGN KEY (proposal_id) REFERENCES task_decomposition_proposals(id) ON DELETE CASCADE,
    FOREIGN KEY (created_task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    UNIQUE (proposal_id, position)
);

CREATE INDEX IF NOT EXISTS idx_task_decomposition_proposals_conversation_status
    ON task_decomposition_proposals(source_conversation_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_task_decomposition_items_proposal
    ON task_decomposition_items(proposal_id, position);
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_decomposition_pending_dedupe
    ON task_decomposition_proposals(source_conversation_id, role_id, task_summary, items_fingerprint)
    WHERE status = 'pending';
