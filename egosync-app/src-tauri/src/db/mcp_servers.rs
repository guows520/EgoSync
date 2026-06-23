use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::mcp::McpServer;

const MCP_SELECT_COLUMNS: &str = "id, name, server_type, command_or_url, env_refs, description, enabled, created_at, updated_at";

pub async fn list_mcp_servers(pool: &SqlitePool) -> Result<Vec<McpServer>, AppError> {
    sqlx::query_as::<_, McpServer>(&format!(
        "SELECT {} FROM mcp_servers ORDER BY created_at ASC",
        MCP_SELECT_COLUMNS
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 MCP server 失败: {}", e)))
}

pub async fn get_mcp_server(pool: &SqlitePool, id: &str) -> Result<McpServer, AppError> {
    sqlx::query_as::<_, McpServer>(&format!(
        "SELECT {} FROM mcp_servers WHERE id = ?1",
        MCP_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 MCP server 失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("MCP server {} 不存在", id)))
}

pub async fn insert_mcp_server(pool: &SqlitePool, server: &McpServer) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mcp_servers (id, name, server_type, command_or_url, env_refs, description, enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(&server.id)
    .bind(&server.name)
    .bind(&server.server_type)
    .bind(&server.command_or_url)
    .bind(&server.env_refs)
    .bind(&server.description)
    .bind(server.enabled)
    .bind(&server.created_at)
    .bind(&server.updated_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("保存 MCP server 失败: {}", e)))?;
    Ok(())
}

pub async fn update_mcp_server(pool: &SqlitePool, server: &McpServer) -> Result<(), AppError> {
    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE mcp_servers
         SET name = ?1,
             server_type = ?2,
             command_or_url = ?3,
             env_refs = ?4,
             description = ?5,
             enabled = ?6,
             updated_at = ?7
         WHERE id = ?8",
    )
    .bind(&server.name)
    .bind(&server.server_type)
    .bind(&server.command_or_url)
    .bind(&server.env_refs)
    .bind(&server.description)
    .bind(server.enabled)
    .bind(&now)
    .bind(&server.id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新 MCP server 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("MCP server {} 不存在", server.id)));
    }
    Ok(())
}

pub async fn delete_mcp_server(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM mcp_servers WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除 MCP server 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("MCP server {} 不存在", id)));
    }
    Ok(())
}

pub async fn list_mcp_servers_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<Vec<McpServer>, AppError> {
    sqlx::query_as::<_, McpServer>(&format!(
        "SELECT {} FROM mcp_servers
         INNER JOIN role_mcp_server_bindings ON role_mcp_server_bindings.server_id = mcp_servers.id
         WHERE role_mcp_server_bindings.role_id = ?1
         ORDER BY role_mcp_server_bindings.created_at ASC",
        prefixed_columns()
    ))
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色 MCP server 失败: {}", e)))
}

pub async fn list_available_mcp_servers_for_role(
    pool: &SqlitePool,
    role_id: &str,
) -> Result<Vec<McpServer>, AppError> {
    sqlx::query_as::<_, McpServer>(&format!(
        "SELECT {} FROM mcp_servers
         WHERE enabled = 1
           AND id NOT IN (SELECT server_id FROM role_mcp_server_bindings WHERE role_id = ?1)
         ORDER BY created_at ASC",
        MCP_SELECT_COLUMNS
    ))
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询可添加 MCP server 失败: {}", e)))
}

pub async fn add_mcp_server_to_role(
    pool: &SqlitePool,
    role_id: &str,
    server_id: &str,
) -> Result<(), AppError> {
    get_mcp_server(pool, server_id).await?;
    crate::db::roles::get_role(pool, role_id).await?;
    let now = crate::db::settings::chrono_now_pub();
    sqlx::query(
        "INSERT OR IGNORE INTO role_mcp_server_bindings (server_id, role_id, created_at)
         VALUES (?1, ?2, ?3)",
    )
    .bind(server_id)
    .bind(role_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("启用角色 MCP server 失败: {}", e)))?;
    Ok(())
}

pub async fn remove_mcp_server_from_role(
    pool: &SqlitePool,
    role_id: &str,
    server_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM role_mcp_server_bindings WHERE server_id = ?1 AND role_id = ?2")
        .bind(server_id)
        .bind(role_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("移除角色 MCP server 失败: {}", e)))?;
    Ok(())
}

pub async fn role_enabled_mcp_lines(pool: &SqlitePool, role_id: &str) -> Result<Vec<String>, AppError> {
    let servers = list_mcp_servers_for_role(pool, role_id).await?;
    Ok(servers
        .into_iter()
        .filter(|server| server.enabled)
        .map(|server| format!("- {}（{}）：{}", server.name, crate::services::mcp_server::mcp_server_type_label(&server.server_type), server.description))
        .collect())
}

fn prefixed_columns() -> String {
    MCP_SELECT_COLUMNS
        .split(", ")
        .map(|column| format!("mcp_servers.{}", column))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT 'target',
                color TEXT NOT NULL DEFAULT '#4F46E5',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
                energy_updated_at TEXT,
                skills_config TEXT NOT NULL DEFAULT '{}',
                proactivity_level TEXT NOT NULL DEFAULT 'moderate',
                archived_at TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            );
            CREATE TABLE mcp_servers (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                server_type TEXT NOT NULL,
                command_or_url TEXT NOT NULL,
                env_refs TEXT NOT NULL DEFAULT '{}',
                description TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            );
            CREATE TABLE role_mcp_server_bindings (
                server_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                PRIMARY KEY (server_id, role_id)
            );
            INSERT INTO roles (id, name) VALUES ('role-1', '产品经理');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn server(id: &str, enabled: bool) -> McpServer {
        McpServer {
            id: id.to_string(),
            name: "日历".to_string(),
            server_type: "http_sse".to_string(),
            command_or_url: "http://localhost:8000/sse".to_string(),
            env_refs: r#"{"CALENDAR_TOKEN":"env:CALENDAR_TOKEN"}"#.to_string(),
            description: "读取日历".to_string(),
            enabled,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn role_binding_keeps_server_config_out_of_role_skills_config() {
        let pool = setup_test_db().await;
        insert_mcp_server(&pool, &server("mcp-1", true)).await.unwrap();
        add_mcp_server_to_role(&pool, "role-1", "mcp-1").await.unwrap();

        let bound = list_mcp_servers_for_role(&pool, "role-1").await.unwrap();
        let role = crate::db::roles::get_role(&pool, "role-1").await.unwrap();

        assert_eq!(bound.len(), 1);
        assert_eq!(role.skills_config, "{}");
    }

    #[tokio::test]
    async fn available_role_servers_includes_disabled_bound_server_for_removal() {
        let pool = setup_test_db().await;
        insert_mcp_server(&pool, &server("mcp-disabled", false)).await.unwrap();
        add_mcp_server_to_role(&pool, "role-1", "mcp-disabled").await.unwrap();

        let bound = list_mcp_servers_for_role(&pool, "role-1").await.unwrap();
        let available = list_available_mcp_servers_for_role(&pool, "role-1").await.unwrap();

        assert_eq!(bound.len(), 1);
        assert!(!bound[0].enabled);
        assert!(available.iter().all(|server| server.id != "mcp-disabled"));
    }
}
