//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::role`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::role::{
    CreateRoleInput, Role, UpdateRoleInput, UpdateRoleProactivityInput, UpdateRoleSkillsInput,
};

#[tauri::command]
pub async fn role_create(
    ctx: State<'_, Arc<EngineCtx>>,
    input: CreateRoleInput,
) -> Result<Role, AppError> {
    egosync_engine::commands::role::role_create(&ctx, input).await
}

#[tauri::command]
pub async fn role_list(ctx: State<'_, Arc<EngineCtx>>) -> Result<Vec<Role>, AppError> {
    egosync_engine::commands::role::role_list(&ctx).await
}

#[tauri::command]
pub async fn role_list_archived(ctx: State<'_, Arc<EngineCtx>>) -> Result<Vec<Role>, AppError> {
    egosync_engine::commands::role::role_list_archived(&ctx).await
}

#[tauri::command]
pub async fn role_update(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    input: UpdateRoleInput,
) -> Result<Role, AppError> {
    egosync_engine::commands::role::role_update(&ctx, id, input).await
}

#[tauri::command]
pub async fn role_update_skills(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    input: UpdateRoleSkillsInput,
) -> Result<Role, AppError> {
    egosync_engine::commands::role::role_update_skills(&ctx, id, input).await
}

#[tauri::command]
pub async fn role_update_proactivity(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
    input: UpdateRoleProactivityInput,
) -> Result<Role, AppError> {
    egosync_engine::commands::role::role_update_proactivity(&ctx, id, input).await
}

#[tauri::command]
pub async fn role_archive(ctx: State<'_, Arc<EngineCtx>>, id: String) -> Result<Role, AppError> {
    egosync_engine::commands::role::role_archive(&ctx, id).await
}

#[tauri::command]
pub async fn role_restore(ctx: State<'_, Arc<EngineCtx>>, id: String) -> Result<Role, AppError> {
    egosync_engine::commands::role::role_restore(&ctx, id).await
}

#[tauri::command]
pub async fn role_delete(ctx: State<'_, Arc<EngineCtx>>, id: String) -> Result<(), AppError> {
    egosync_engine::commands::role::role_delete(&ctx, id).await
}
