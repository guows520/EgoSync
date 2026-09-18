//! skill 域命令（Story 15.4）：web-ok 命令体已迁引擎
//! （`egosync_engine::commands::skill`），壳侧薄化为 wrapper；
//! skill_pick_custom_directory 为 desktop-only 留壳不迁（rfd 对话框）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::skill::{
    ImportCustomSkillInput, ImportCustomSkillResult, ImportOpencodeSkillInput,
    ImportOpencodeSkillResult, PickCustomSkillDirectoryResult, PreviewCustomSkillInput,
    SkillImportPreview, SkillRegistryEntry, SelectableSkill,
};

#[tauri::command]
pub async fn skill_list_registry(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    egosync_engine::commands::skill::skill_list_registry(&ctx).await
}

#[tauri::command]
pub async fn skill_list_for_role(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    egosync_engine::commands::skill::skill_list_for_role(&ctx, role_id).await
}

#[tauri::command]
pub async fn skill_list_all_role_skills(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    egosync_engine::commands::skill::skill_list_all_role_skills(&ctx).await
}

#[tauri::command]
pub async fn skill_list_selectable_for_scope(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: Option<String>,
) -> Result<Vec<SelectableSkill>, AppError> {
    egosync_engine::commands::skill::skill_list_selectable_for_scope(&ctx, role_id).await
}

#[tauri::command]
pub async fn skill_pick_custom_directory() -> Result<PickCustomSkillDirectoryResult, AppError> {
    let selected = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|e| AppError::ValidationError(format!("选择 Skill 文件夹失败: {}", e)))?;
    let Some(directory) = selected else {
        return Err(AppError::ValidationError("未选择 Skill 文件夹".to_string()));
    };
    let skill_path = directory.join("SKILL.md");
    let content = std::fs::read_to_string(&skill_path).map_err(|_| {
        AppError::ValidationError("所选文件夹中未找到可读取的 SKILL.md".to_string())
    })?;
    Ok(PickCustomSkillDirectoryResult {
        content,
        source_path: directory.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn skill_preview_custom(
    ctx: State<'_, Arc<EngineCtx>>,
    input: PreviewCustomSkillInput,
) -> Result<SkillImportPreview, AppError> {
    egosync_engine::commands::skill::skill_preview_custom(&ctx, input).await
}

#[tauri::command]
pub async fn skill_discover_opencode(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
) -> Result<crate::models::skill::DiscoverOpencodeSkillsResult, AppError> {
    egosync_engine::commands::skill::skill_discover_opencode(&ctx, role_id).await
}

#[tauri::command]
pub async fn skill_import_opencode(
    ctx: State<'_, Arc<EngineCtx>>,
    role_id: String,
    input: ImportOpencodeSkillInput,
) -> Result<ImportOpencodeSkillResult, AppError> {
    egosync_engine::commands::skill::skill_import_opencode(&ctx, role_id, input).await
}

#[tauri::command]
pub async fn skill_import_custom(
    ctx: State<'_, Arc<EngineCtx>>,
    input: ImportCustomSkillInput,
) -> Result<ImportCustomSkillResult, AppError> {
    egosync_engine::commands::skill::skill_import_custom(&ctx, input).await
}

#[tauri::command]
pub async fn skill_remove_from_role(
    ctx: State<'_, Arc<EngineCtx>>,
    skill_id: String,
    role_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::skill::skill_remove_from_role(&ctx, skill_id, role_id).await
}

#[tauri::command]
pub async fn skill_delete(
    ctx: State<'_, Arc<EngineCtx>>,
    skill_id: String,
) -> Result<(), AppError> {
    egosync_engine::commands::skill::skill_delete(&ctx, skill_id).await
}
