CREATE TABLE weekly_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    week_start TEXT NOT NULL,
    week_end TEXT NOT NULL,
    summary TEXT NOT NULL,
    energy_trends TEXT NOT NULL DEFAULT '{}',
    bigrock_status TEXT NOT NULL DEFAULT '{}',
    new_memories_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE UNIQUE INDEX idx_weekly_reviews_week_start ON weekly_reviews(week_start);
