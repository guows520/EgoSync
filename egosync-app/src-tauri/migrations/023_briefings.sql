CREATE TABLE briefings (
    id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL,
    date TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
