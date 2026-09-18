-- Story 5.1: 使命宣言表，仅允许单行记录（id 固定为 'singleton'）
CREATE TABLE IF NOT EXISTS mission (
    id TEXT PRIMARY KEY NOT NULL DEFAULT 'singleton',
    content TEXT,
    format TEXT NOT NULL DEFAULT 'free' CHECK(format IN ('free', 'structured')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
