use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

use crate::commands::chat::OpencodeSessions;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::skill::{
    DiscoverOpencodeSkillsResult, ImportCustomSkillInput, ImportCustomSkillResult,
    ImportOpencodeSkillInput, ImportOpencodeSkillResult, PickCustomSkillDirectoryResult,
    PreviewCustomSkillInput, SkillImportPreview, SkillRegistryEntry, SelectableSkill, BUTLER_SCOPE_ID,
};
use crate::services::agent_config::AgentConfigService;
use crate::services::sidecar::SidecarManager;

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
pub async fn skill_list_selectable_for_scope(
    role_id: Option<String>,
    pool: State<'_, DbPool>,
) -> Result<Vec<SelectableSkill>, AppError> {
    crate::services::skill_registry::list_selectable(&pool, role_id.as_deref()).await
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
    input: PreviewCustomSkillInput,
    pool: State<'_, DbPool>,
) -> Result<SkillImportPreview, AppError> {
    crate::services::skill_registry::preview_custom_skill(&pool, &input).await
}

fn opencode_workspace_dir(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::ValidationError(format!("获取应用数据目录失败: {}", e)))?
        .join("opencode-workspace"))
}

fn user_home_dir() -> Result<std::path::PathBuf, AppError> {
    dirs::home_dir().ok_or_else(|| AppError::ValidationError("无法获取用户主目录".to_string()))
}

async fn ensure_find_skills_enabled(pool: &DbPool, role_id: &str) -> Result<(), AppError> {
    if role_id == BUTLER_SCOPE_ID {
        let skills = crate::services::butler_config::get_butler_skills(pool).await?;
        return if skills.find_skills {
            Ok(())
        } else {
            Err(AppError::ValidationError(
                "需要先启用 find-skills 才能发现可用 Skill".to_string(),
            ))
        };
    }

    let role = crate::db::roles::get_role(pool, role_id).await?;
    if crate::services::role_config::skill_enabled(
        &role.skills_config,
        crate::services::role_config::FIND_SKILLS_KEY,
    ) {
        Ok(())
    } else {
        Err(AppError::ValidationError(
            "需要先启用 find-skills 才能发现可用 Skill".to_string(),
        ))
    }
}

#[tauri::command]
pub async fn skill_discover_opencode(
    role_id: String,
    pool: State<'_, DbPool>,
    app: AppHandle,
) -> Result<DiscoverOpencodeSkillsResult, AppError> {
    ensure_find_skills_enabled(&pool, &role_id).await?;
    let project_dir = opencode_workspace_dir(&app)?;
    let home_dir = user_home_dir()?;
    crate::services::skill_registry::discover_opencode_skills(
        &pool,
        &role_id,
        &project_dir,
        &home_dir,
    )
    .await
}

async fn sync_all_agents_with_mcp(
    pool: &DbPool,
    agent_config: &AgentConfigService,
    roles: &[crate::models::role::Role],
    butler_skills: &crate::models::role::ButlerSkillsConfig,
    registry: &[crate::models::skill::SkillRegistryEntry],
    action: &str,
) -> bool {
    let mcp_prompts = crate::services::mcp_server::role_mcp_prompt_map(pool)
        .await
        .unwrap_or_default();
    if let Err(e) =
        agent_config.full_sync_with_skills_and_mcp(roles, butler_skills, registry, &mcp_prompts)
    {
        tracing::warn!("opencode sync after {} failed: {}", action, e);
        return false;
    }
    true
}

#[tauri::command]
pub async fn skill_import_opencode(
    role_id: String,
    input: ImportOpencodeSkillInput,
    pool: State<'_, DbPool>,
    app: AppHandle,
    agent_config: State<'_, AgentConfigService>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<ImportOpencodeSkillResult, AppError> {
    ensure_find_skills_enabled(&pool, &role_id).await?;
    let project_dir = opencode_workspace_dir(&app)?;
    let home_dir = user_home_dir()?;
    let skills_root = project_dir.join(".opencode").join("skills");
    let mut result = crate::services::skill_registry::import_opencode_skill(
        &pool,
        &input,
        &skills_root,
        &project_dir,
        &home_dir,
    )
    .await?;
    // full_sync 会按传入集合整体重建 opencode.json 的 agent 配置；若以
    // unwrap_or_default 兜底，瞬时 DB 错误会传入空集并清空所有角色已同步的
    // Skill 声明。因此这里用 `?` 直接失败，绝不以空集触发破坏性全量同步。
    let registry = crate::db::skills::list_skills(&pool).await?;
    let roles = crate::db::roles::list_all_roles(&pool).await?;
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool).await?;
    if !sync_all_agents_with_mcp(
        &pool,
        &agent_config,
        &roles,
        &butler_skills,
        &registry,
        "opencode skill import",
    )
    .await
    {
        // registry 写入已成功，但 opencode agent 未同步：如实告知前端 synced=false，
        // 避免谎称"已启用"。下一次同步路径会重新落地（AC5 最终一致）。
        result.synced = false;
    }
    if result.synced {
        match crate::commands::mcp::refresh_opencode_runtime(&sidecar, &opencode_sessions).await {
            Ok(()) => result.runtime_ready = true,
            Err(e) => result.runtime_error = Some(e.to_string()),
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn skill_import_custom(
    input: ImportCustomSkillInput,
    pool: State<'_, DbPool>,
    app: AppHandle,
    agent_config: State<'_, AgentConfigService>,
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
    opencode_sessions: State<'_, OpencodeSessions>,
) -> Result<ImportCustomSkillResult, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::ValidationError(format!("获取应用数据目录失败: {}", e)))?;
    let skills_root = app_data_dir
        .join("opencode-workspace")
        .join(".opencode")
        .join("skills");
    let mut result =
        crate::services::skill_registry::import_custom_skill(&pool, &skills_root, &input).await?;
    let registry = crate::db::skills::list_skills(&pool)
        .await
        .unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&pool)
        .await
        .unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    let synced = sync_all_agents_with_mcp(
        &pool,
        &agent_config,
        &roles,
        &butler_skills,
        &registry,
        "skill import",
    )
    .await;
    if synced {
        match crate::commands::mcp::refresh_opencode_runtime(&sidecar, &opencode_sessions).await {
            Ok(()) => result.runtime_ready = true,
            Err(e) => result.runtime_error = Some(e.to_string()),
        }
    } else {
        result.runtime_error = Some("opencode agent 配置同步失败".to_string());
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

    let registry = crate::db::skills::list_skills(&pool)
        .await
        .unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&pool)
        .await
        .unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    sync_all_agents_with_mcp(
        &pool,
        &agent_config,
        &roles,
        &butler_skills,
        &registry,
        "skill remove from role",
    )
    .await;
    Ok(())
}

#[tauri::command]
pub async fn skill_delete(
    skill_id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    crate::services::skill_registry::delete_custom_skill(&pool, &skill_id).await?;

    let registry = crate::db::skills::list_skills(&pool)
        .await
        .unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&pool)
        .await
        .unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    sync_all_agents_with_mcp(
        &pool,
        &agent_config,
        &roles,
        &butler_skills,
        &registry,
        "skill delete",
    )
    .await;
    Ok(())
}
