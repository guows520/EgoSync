PRAGMA foreign_keys=OFF;

CREATE TEMP TABLE IF NOT EXISTS mcp_servers_backup AS
SELECT id,
       name,
       CASE server_type
           WHEN 'http_sse' THEN 'sse'
           WHEN 'command' THEN 'stdio'
           ELSE server_type
       END AS server_type,
       command_or_url,
       env_refs,
       description,
       enabled,
       created_at,
       updated_at
FROM mcp_servers;

CREATE TEMP TABLE IF NOT EXISTS role_mcp_server_bindings_backup AS
SELECT server_id, role_id, created_at
FROM role_mcp_server_bindings;

DROP TABLE role_mcp_server_bindings;
DROP TABLE mcp_servers;

CREATE TABLE mcp_servers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    server_type TEXT NOT NULL CHECK(server_type IN ('sse', 'streamable_http', 'stdio')),
    command_or_url TEXT NOT NULL,
    env_refs TEXT NOT NULL DEFAULT '{}',
    description TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO mcp_servers (id, name, server_type, command_or_url, env_refs, description, enabled, created_at, updated_at)
SELECT id, name, server_type, command_or_url, env_refs, description, enabled, created_at, updated_at
FROM mcp_servers_backup;

CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_type ON mcp_servers(server_type);

CREATE TABLE role_mcp_server_bindings (
    server_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (server_id, role_id),
    FOREIGN KEY (server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO role_mcp_server_bindings (server_id, role_id, created_at)
SELECT server_id, role_id, created_at
FROM role_mcp_server_bindings_backup
WHERE server_id IN (SELECT id FROM mcp_servers);

CREATE INDEX IF NOT EXISTS idx_role_mcp_server_bindings_role_id ON role_mcp_server_bindings(role_id);

DROP TABLE role_mcp_server_bindings_backup;
DROP TABLE mcp_servers_backup;

PRAGMA foreign_keys=ON;
