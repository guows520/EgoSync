//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::mcp`），
//! 壳侧薄化为 wrapper（内联 source-scan 测试随命令体迁引擎并机械适配）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::mcp::{CreateMcpServerInput, McpServer, UpdateMcpServerInput};

#[tauri::command]
pub async fn mcp_server_list(ctx: State<'_, Arc<EngineCtx>>) -> Result<Vec<McpServer>, AppError> {
    egosync_engine::commands::mcp::mcp_server_list(&ctx).await
}

#[tauri::command]
pub async fn mcp_server_list_for_role(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
) -> Result<Vec<McpServer>, AppError> {
    egosync_engine::commands::mcp::mcp_server_list_for_role(&ctx, role_id).await
}

#[tauri::command]
pub async fn mcp_server_list_available_for_role(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
) -> Result<Vec<McpServer>, AppError> {
    egosync_engine::commands::mcp::mcp_server_list_available_for_role(&ctx, role_id).await
}

#[tauri::command]
pub async fn mcp_server_create(
    ctx: State<'_, Arc<EngineCtx>>,
    input: CreateMcpServerInput,
) -> Result<McpServer, AppError> {
    egosync_engine::commands::mcp::mcp_server_create(&ctx, input).await
}

#[tauri::command]
pub async fn mcp_server_update(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    input: UpdateMcpServerInput,
) -> Result<McpServer, AppError> {
    egosync_engine::commands::mcp::mcp_server_update(&ctx, id, input).await
}

#[tauri::command]
pub async fn mcp_server_delete(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_delete(&ctx, id).await
}

#[tauri::command]
pub async fn mcp_server_test(ctx: State<'_, Arc<EngineCtx>>, id: String) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_test(&ctx, id).await
}

#[tauri::command]
pub async fn mcp_server_list_for_butler(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<McpServer>, AppError> {
    egosync_engine::commands::mcp::mcp_server_list_for_butler(&ctx).await
}

#[tauri::command]
pub async fn mcp_server_list_available_for_butler(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<McpServer>, AppError> {
    egosync_engine::commands::mcp::mcp_server_list_available_for_butler(&ctx).await
}

#[tauri::command]
pub async fn mcp_server_add_to_role(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
    server_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_add_to_role(&ctx, role_id, server_id).await
}

#[tauri::command]
pub async fn mcp_server_remove_from_role(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
    server_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_remove_from_role(&ctx, role_id, server_id).await
}

#[tauri::command]
pub async fn mcp_server_add_to_butler(
    ctx: State<'_, Arc<EngineCtx>>,
    server_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_add_to_butler(&ctx, server_id).await
}

#[tauri::command]
pub async fn mcp_server_remove_from_butler(
    ctx: State<'_, Arc<EngineCtx>>,
    server_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_remove_from_butler(&ctx, server_id).await
}

#[tauri::command]
pub async fn mcp_server_refresh_butler_runtime(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<(), AppError> {
    egosync_engine::commands::mcp::mcp_server_refresh_butler_runtime(&ctx).await
}
