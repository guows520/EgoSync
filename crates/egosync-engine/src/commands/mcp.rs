//! mcp 域命令体（Story 15.4 自壳 `commands/mcp.rs` 平移，业务逻辑/锁结构零
//! 改动；State 取值改 `&EngineCtx`；内联 source-scan 测试随迁并按 ctx 取值
//! 形态机械适配——断言意图不变）。

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::commands::chat::{ChatSessionRegistry, OpencodeSessions};
use crate::commands::ctx::EngineCtx;
use crate::error::AppError;
use crate::models::mcp::{CreateMcpServerInput, McpServer, UpdateMcpServerInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::sidecar::SidecarManager;

pub async fn mcp_server_list(ctx: &EngineCtx) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_servers(&ctx.pool).await
}

pub async fn mcp_server_list_for_role(
    ctx: &EngineCtx,
    role_id: String,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_servers_for_role(&ctx.pool, &role_id).await
}

pub async fn mcp_server_list_available_for_role(
    ctx: &EngineCtx,
    role_id: String,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_available_servers_for_role(&ctx.pool, &role_id).await
}

pub async fn mcp_server_create(
    ctx: &EngineCtx,
    input: CreateMcpServerInput,
) -> Result<McpServer, AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    let server = crate::services::mcp_server::create_server(&ctx.pool, agent_config, input).await?;
    refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions).await;
    Ok(server)
}

pub async fn mcp_server_update(
    ctx: &EngineCtx,
    id: String,
    input: UpdateMcpServerInput,
) -> Result<McpServer, AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    let server =
        crate::services::mcp_server::update_server(&ctx.pool, agent_config, &id, input).await?;
    refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions).await;
    Ok(server)
}

pub async fn mcp_server_delete(ctx: &EngineCtx, id: String) -> Result<(), AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::delete_server(&ctx.pool, agent_config, &id).await?;
    refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions).await;
    Ok(())
}

pub async fn mcp_server_test(ctx: &EngineCtx, id: String) -> Result<(), AppError> {
    let (registry, sidecar): (&Arc<ChatSessionRegistry>, &Arc<Mutex<SidecarManager>>) =
        (&ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::test_server(&ctx.pool, &id).await?;
    refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions).await;
    Ok(())
}

pub async fn mcp_server_list_for_butler(ctx: &EngineCtx) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_servers_for_butler(&ctx.pool).await
}

pub async fn mcp_server_list_available_for_butler(
    ctx: &EngineCtx,
) -> Result<Vec<McpServer>, AppError> {
    crate::services::mcp_server::list_available_servers_for_butler(&ctx.pool).await
}

pub async fn mcp_server_add_to_role(
    ctx: &EngineCtx,
    role_id: String,
    server_id: String,
) -> Result<(), AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::add_to_role(&ctx.pool, agent_config, &role_id, &server_id).await?;
    refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions).await;
    Ok(())
}

pub async fn mcp_server_remove_from_role(
    ctx: &EngineCtx,
    role_id: String,
    server_id: String,
) -> Result<(), AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::remove_from_role(&ctx.pool, agent_config, &role_id, &server_id)
        .await?;
    refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions).await;
    Ok(())
}

pub async fn mcp_server_add_to_butler(ctx: &EngineCtx, server_id: String) -> Result<(), AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::add_to_butler(&ctx.pool, agent_config, &server_id)
        .await
        .map_err(saved_butler_runtime_error)?;
    refresh_opencode_runtime(sidecar, &registry.opencode_sessions)
        .await
        .map_err(saved_butler_runtime_error)
}

pub async fn mcp_server_remove_from_butler(
    ctx: &EngineCtx,
    server_id: String,
) -> Result<(), AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::remove_from_butler(&ctx.pool, agent_config, &server_id)
        .await
        .map_err(saved_butler_runtime_error)?;
    refresh_opencode_runtime(sidecar, &registry.opencode_sessions)
        .await
        .map_err(saved_butler_runtime_error)
}

pub async fn mcp_server_refresh_butler_runtime(ctx: &EngineCtx) -> Result<(), AppError> {
    let (agent_config, registry, sidecar): (
        &AgentConfigService,
        &Arc<ChatSessionRegistry>,
        &Arc<Mutex<SidecarManager>>,
    ) = (&ctx.agent_config, &ctx.registry, &ctx.sidecar);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    crate::services::mcp_server::sync_butler_agent(&ctx.pool, agent_config).await?;
    refresh_opencode_runtime(sidecar, &registry.opencode_sessions).await
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
        // Story 15.4 随迁：State 取值改 ctx 字段后，扫描断言按 ctx 形态
        // 机械适配（意图不变：MCP 变更命令须刷新 opencode runtime 并清
        // stale session 缓存——否则旧 MCP 配置残留到下一次重启）。
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
            let end = rest.find("\npub async fn").unwrap_or(rest.len());
            let body = &rest[..end];

            assert!(
                body.contains("&ctx.sidecar"),
                "{} must receive sidecar state so runtime config can reload",
                command
            );
            assert!(
                body.contains("&ctx.registry"),
                "{} must receive the session registry so stale sessions are cleared after restart",
                command
            );
            let refresh_call = if command.ends_with("_butler") {
                "refresh_opencode_runtime(sidecar, &registry.opencode_sessions)"
            } else {
                "refresh_opencode_runtime_after_mcp_change(sidecar, &registry.opencode_sessions"
            };
            assert!(
                body.contains(refresh_call),
                "{} must refresh opencode runtime and clear stale session cache after MCP config changes",
                command
            );
        }
    }
}
