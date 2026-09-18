-- 扩展 llm_configs.provider CHECK 约束，使数据库与应用层 provider 集合保持一致
-- SQLite 无法直接修改 CHECK 约束，需要重建表；保留 migration 029 的 network_location

CREATE TABLE llm_configs_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    provider TEXT NOT NULL CHECK(provider IN ('openai_compatible', 'anthropic', 'minimax', 'zhipu', 'deepseek', 'kimi', 'bailian')),
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    api_key_ref TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    network_location TEXT NOT NULL DEFAULT 'external'
        CHECK(network_location IN ('internal', 'external'))
);

INSERT INTO llm_configs_new (
    id, name, provider, base_url, model, api_key_ref, is_default,
    created_at, updated_at, network_location
)
SELECT
    id, name, provider, base_url, model, api_key_ref, is_default,
    created_at, updated_at, network_location
FROM llm_configs;

DROP TABLE llm_configs;

ALTER TABLE llm_configs_new RENAME TO llm_configs;
