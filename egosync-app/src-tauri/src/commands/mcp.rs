use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::commands::chat::{OpencodeMcpScopeLock, OpencodeSessions};
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::mcp::{CreateMcpServerInput, McpServer, UpdateMcpServerInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::sidecar::SidecarManager;

#[tauri::command]
pub async fn mcp_server_list(pool: State<'_, DbPool>) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_servers(&pool).await
}

#[tauri::command]
pub async fn mcp_server_list_for_role(
    role_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_servers_for_role(&pool, &role_id).await
}

#[tauri::command]
pub async fn mcp_server_list_available_for_role(
    role_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_available_servers_for_role(&pool, &role_id).await
}

#[tauri::command]
pub async fn mcp_server_create(
    input: CreateMcpServerInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<McpServer, AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    let server = crate::services::mcp_server::create_server(&pool, &agent_config, input).await?;
    refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions).await;
    Ok(server)
}

#[tauri::command]
pub async fn mcp_server_update(
    id: String,
    input: UpdateMcpServerInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<McpServer, AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    let server =
        crate::services::mcp_server::update_server(&pool, &agent_config, &id, input).await?;
    refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions).await;
    Ok(server)
}

#[tauri::command]
pub async fn mcp_server_delete(
    id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::delete_server(&pool, &agent_config, &id).await?;
    refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions).await;
    Ok(())
}

#[tauri::command]
pub async fn mcp_server_test(
    id: String,
    pool: State<'_, DbPool>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::test_server(&pool, &id).await?;
    refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions).await;
    Ok(())
}

#[tauri::command]
pub async fn mcp_server_list_for_butler(
    pool: State<'_, DbPool>,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_servers_for_butler(&pool).await
}

#[tauri::command]
pub async fn mcp_server_list_available_for_butler(
    pool: State<'_, DbPool>,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_available_servers_for_butler(&pool).await
}

#[tauri::command]
pub async fn mcp_server_add_to_role(
    role_id: String,
    server_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::add_to_role(&pool, &agent_config, &role_id, &server_id).await?;
    refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions).await;
    Ok(())
}

#[tauri::command]
pub async fn mcp_server_remove_from_role(
    role_id: String,
    server_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::remove_from_role(&pool, &agent_config, &role_id, &server_id)
        .await?;
    refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions).await;
    Ok(())
}

#[tauri::command]
pub async fn mcp_server_add_to_butler(
    server_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::add_to_butler(&pool, &agent_config, &server_id)
        .await
        .map_err(saved_butler_runtime_error)?;
    refresh_opencode_runtime(&sidecar, &opencode_sessions)
        .await
        .map_err(saved_butler_runtime_error)
}

#[tauri::command]
pub async fn mcp_server_remove_from_butler(
    server_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::remove_from_butler(&pool, &agent_config, &server_id)
        .await
        .map_err(saved_butler_runtime_error)?;
    refresh_opencode_runtime(&sidecar, &opencode_sessions)
        .await
        .map_err(saved_butler_runtime_error)
}

#[tauri::command]
pub async fn mcp_server_refresh_butler_runtime(
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
    mcp_scope_lock: State<'_, OpencodeMcpScopeLock>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<(), AppError> {
    let _guard = mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::sync_butler_agent(&pool, &agent_config).await?;
    refresh_opencode_runtime(&sidecar, &opencode_sessions).await
}

fn saved_butler_runtime_error(error: AppError) -> AppError {
    tracing::warn!("butler MCP binding saved but runtime refresh failed: {}", error);
    AppError::ValidationError(format!("配置已保存，但 Agent Runtime 尚未刷新：{}", error))
}

pub(crate) async fn refresh_opencode_runtime(
    sidecar: &Arc<Mutex<SidecarManager>>,
    opencode_sessions: &OpencodeSessions,
) -> Result<(), AppError> {
    let mut manager = sidecar.lock().await;
    manager.restart().await?;
    opencode_sessions.0.lock().await.clear();
    Ok(())
}

async fn refresh_opencode_runtime_after_mcp_change(
    sidecar: &Arc<Mutex<SidecarManager>>,
    opencode_sessions: &OpencodeSessions,
) {
    if let Err(e) = refresh_opencode_runtime(sidecar, opencode_sessions).await {
        tracing::warn!("opencode runtime refresh after MCP change failed: {}", e);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mutating_mcp_commands_refresh_opencode_runtime() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/mcp.rs"),
        )
        .expect("read mcp commands");

        for command in [
            "mcp_server_create",
            "mcp_server_update",
            "mcp_server_delete",
            "mcp_server_test",
            "mcp_server_add_to_role",
            "mcp_server_remove_from_role",
            "mcp_server_add_to_butler",
            "mcp_server_remove_from_butler",
        ] {
            let start = source
                .find(&format!("pub async fn {}", command))
                .unwrap_or_else(|| panic!("missing command {}", command));
            let rest = &source[start..];
            let end = rest.find("\n#[tauri::command]").unwrap_or(rest.len());
            let body = &rest[..end];

            assert!(
                body.contains("sidecar: State<'_, Arc<Mutex<SidecarManager>>>"),
                "{} must receive sidecar state so runtime config can reload",
                command
            );
            assert!(
                body.contains("opencode_sessions: State<'_, OpencodeSessions>"),
                "{} must receive opencode session cache state so stale sessions are cleared after restart",
                command
            );
            let refresh_call = if command.ends_with("_butler") {
                "refresh_opencode_runtime(&sidecar, &opencode_sessions)"
            } else {
                "refresh_opencode_runtime_after_mcp_change(&sidecar, &opencode_sessions"
            };
            assert!(
                body.contains(refresh_call),
                "{} must refresh opencode runtime and clear stale session cache after MCP config changes",
                command
            );
        }
    }
}
