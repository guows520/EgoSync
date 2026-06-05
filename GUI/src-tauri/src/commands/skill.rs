use tauri::{AppHandle, Manager, State};

use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::skill::{
    ImportCustomSkillInput, ImportCustomSkillResult, PickCustomSkillDirectoryResult,
    PreviewCustomSkillInput, SkillImportPreview, SkillRegistryEntry,
};
use crate::services::agent_config::AgentConfigService;

#[tauri::command]
pub async fn skill_list_registry(
    pool: State<'_, DbPool>,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    crate::services::skill_registry::list_registry(&pool).await
}

#[tauri::command]
pub async fn skill_list_for_role(
    role_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    crate::services::skill_registry::list_for_role(&pool, &role_id).await
}

#[tauri::command]
pub async fn skill_list_all_role_skills(
    pool: State<'_, DbPool>,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    crate::services::skill_registry::list_all_role_skills(&pool).await
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
    let content = std::fs::read_to_string(&skill_path)
        .map_err(|_| AppError::ValidationError("所选文件夹中未找到可读取的 SKILL.md".to_string()))?;
    Ok(PickCustomSkillDirectoryResult { content })
}

#[tauri::command]
pub async fn skill_preview_custom(
    input: PreviewCustomSkillInput,
    pool: State<'_, DbPool>,
) -> Result<SkillImportPreview, AppError> {
    crate::services::skill_registry::preview_custom_skill(&pool, &input).await
}

#[tauri::command]
pub async fn skill_import_custom(
    input: ImportCustomSkillInput,
    pool: State<'_, DbPool>,
    app: AppHandle,
    agent_config: State<'_, AgentConfigService>,
) -> Result<ImportCustomSkillResult, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::ValidationError(format!("获取应用数据目录失败: {}", e)))?;
    let skills_root = app_data_dir
        .join("opencode-workspace")
        .join(".opencode")
        .join("skills");
    let result = crate::services::skill_registry::import_custom_skill(&pool, &skills_root, &input).await?;
    let registry = crate::db::skills::list_skills(&pool).await.unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&pool).await.unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    if let Err(e) = agent_config.full_sync_with_skills(&roles, &butler_skills, &registry) {
        tracing::warn!("opencode sync after skill import failed: {}", e);
    }
    Ok(result)
}

#[tauri::command]
pub async fn skill_remove_from_role(
    skill_id: String,
    role_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    crate::services::skill_registry::remove_skill_from_role(&pool, &skill_id, &role_id).await?;

    let registry = crate::db::skills::list_skills(&pool).await.unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&pool).await.unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    if let Err(e) = agent_config.full_sync_with_skills(&roles, &butler_skills, &registry) {
        tracing::error!("opencode sync after skill remove from role failed: {}", e);
    }
    Ok(())
}

#[tauri::command]
pub async fn skill_delete(
    skill_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    crate::services::skill_registry::delete_custom_skill(&pool, &skill_id).await?;

    let registry = crate::db::skills::list_skills(&pool).await.unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&pool).await.unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    if let Err(e) = agent_config.full_sync_with_skills(&roles, &butler_skills, &registry) {
        tracing::error!("opencode sync after skill delete failed: {}", e);
    }
    Ok(())
}
