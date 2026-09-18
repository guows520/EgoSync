//! skill 域命令体（Story 15.4 自壳 `commands/skill.rs` 平移，业务逻辑零
//! 改动；State/AppHandle 取值改 `&EngineCtx`，AppHandle 路径改 ctx 注入路径。
//! skill_pick_custom_directory 为 desktop-only 留壳不迁）。

use std::sync::Arc;

use tokio::sync::Mutex;

// Story 15.3：六组状态合并为单 Registry（opencode_sessions 经其字段取用）
use crate::commands::chat::ChatSessionRegistry;
use crate::commands::ctx::EngineCtx;
use crate::db::pool::DbPool;
use crate::error::AppError;
use crate::models::skill::{
    DiscoverOpencodeSkillsResult, ImportCustomSkillInput, ImportCustomSkillResult,
    ImportOpencodeSkillInput, ImportOpencodeSkillResult, PreviewCustomSkillInput,
    SkillImportPreview, SkillRegistryEntry, SelectableSkill, BUTLER_SCOPE_ID,
};
use crate::services::agent_config::AgentConfigService;
use crate::services::sidecar::SidecarManager;

pub async fn skill_list_registry(ctx: &EngineCtx) -> Result<Vec<SkillRegistryEntry>, AppError> {
    crate::services::skill_registry::list_registry(&ctx.pool).await
}

pub async fn skill_list_for_role(
    ctx: &EngineCtx,
    role_id: String,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    crate::services::skill_registry::list_for_role(&ctx.pool, &role_id).await
}

pub async fn skill_list_all_role_skills(
    ctx: &EngineCtx,
) -> Result<Vec<SkillRegistryEntry>, AppError> {
    crate::services::skill_registry::list_all_role_skills(&ctx.pool).await
}

pub async fn skill_list_selectable_for_scope(
    ctx: &EngineCtx,
    role_id: Option<String>,
) -> Result<Vec<SelectableSkill>, AppError> {
    crate::services::skill_registry::list_selectable(&ctx.pool, role_id.as_deref()).await
}

pub async fn skill_preview_custom(
    ctx: &EngineCtx,
    input: PreviewCustomSkillInput,
) -> Result<SkillImportPreview, AppError> {
    crate::services::skill_registry::preview_custom_skill(&ctx.pool, &input).await
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

pub async fn skill_discover_opencode(
    ctx: &EngineCtx,
    role_id: String,
) -> Result<DiscoverOpencodeSkillsResult, AppError> {
    ensure_find_skills_enabled(&ctx.pool, &role_id).await?;
    // Story 15.4：AppHandle 路径改 ctx 注入路径（opencode workspace / 用户主目录）
    let project_dir = ctx.opencode_workspace.clone();
    let home_dir = ctx.home_dir.clone();
    crate::services::skill_registry::discover_opencode_skills(
        &ctx.pool,
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
    let mcp_prompts = match crate::services::mcp_server::role_mcp_prompt_map(pool).await {
        Ok(prompts) => prompts,
        Err(e) => {
            tracing::warn!("load role MCP bindings after {} failed: {}", action, e);
            return false;
        }
    };
    let butler_mcp_lines = match crate::db::mcp_servers::butler_enabled_mcp_lines(pool).await {
        Ok(lines) => lines,
        Err(e) => {
            tracing::warn!("load butler MCP bindings after {} failed: {}", action, e);
            return false;
        }
    };
    if let Err(e) =
        agent_config.full_sync_with_skills_and_mcp(roles, butler_skills, registry, &mcp_prompts, &butler_mcp_lines)
    {
        tracing::warn!("opencode sync after {} failed: {}", action, e);
        return false;
    }
    true
}

pub async fn skill_import_opencode(
    ctx: &EngineCtx,
    role_id: String,
    input: ImportOpencodeSkillInput,
) -> Result<ImportOpencodeSkillResult, AppError> {
    let (agent_config, sidecar, session_registry): (
        &AgentConfigService,
        &Arc<Mutex<SidecarManager>>,
        &Arc<ChatSessionRegistry>,
    ) = (&ctx.agent_config, &ctx.sidecar, &ctx.registry);
    ensure_find_skills_enabled(&ctx.pool, &role_id).await?;
    // Story 15.4：AppHandle 路径改 ctx 注入路径
    let project_dir = ctx.opencode_workspace.clone();
    let home_dir = ctx.home_dir.clone();
    let skills_root = project_dir.join(".opencode").join("skills");
    let mut result = crate::services::skill_registry::import_opencode_skill(
        &ctx.pool,
        &input,
        &skills_root,
        &project_dir,
        &home_dir,
    )
    .await?;
    // full_sync 会按传入集合整体重建 opencode.json 的 agent 配置；若以
    // unwrap_or_default 兜底，瞬时 DB 错误会传入空集并清空所有角色已同步的
    // Skill 声明。因此这里用 `?` 直接失败，绝不以空集触发破坏性全量同步。
    let registry = crate::db::skills::list_skills(&ctx.pool).await?;
    let roles = crate::db::roles::list_all_roles(&ctx.pool).await?;
    let butler_skills = crate::services::butler_config::get_butler_skills(&ctx.pool).await?;
    if !sync_all_agents_with_mcp(
        &ctx.pool,
        agent_config,
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
        match crate::commands::mcp::refresh_opencode_runtime(sidecar, &session_registry.opencode_sessions).await {
            Ok(()) => result.runtime_ready = true,
            Err(e) => result.runtime_error = Some(e.to_string()),
        }
    }
    Ok(result)
}

pub async fn skill_import_custom(
    ctx: &EngineCtx,
    input: ImportCustomSkillInput,
) -> Result<ImportCustomSkillResult, AppError> {
    let (agent_config, sidecar, session_registry): (
        &AgentConfigService,
        &Arc<Mutex<SidecarManager>>,
        &Arc<ChatSessionRegistry>,
    ) = (&ctx.agent_config, &ctx.sidecar, &ctx.registry);
    // Story 15.4：AppHandle 路径改 ctx 注入路径（受控 Skill 根目录）
    let skills_root = ctx.skills_root.clone();
    let mut result =
        crate::services::skill_registry::import_custom_skill(&ctx.pool, &skills_root, &input).await?;
    let registry = crate::db::skills::list_skills(&ctx.pool)
        .await
        .unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&ctx.pool)
        .await
        .unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&ctx.pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    let synced = sync_all_agents_with_mcp(
        &ctx.pool,
        agent_config,
        &roles,
        &butler_skills,
        &registry,
        "skill import",
    )
    .await;
    if synced {
        match crate::commands::mcp::refresh_opencode_runtime(sidecar, &session_registry.opencode_sessions).await {
            Ok(()) => result.runtime_ready = true,
            Err(e) => result.runtime_error = Some(e.to_string()),
        }
    } else {
        result.runtime_error = Some("opencode agent 配置同步失败".to_string());
    }
    Ok(result)
}

pub async fn skill_remove_from_role(
    ctx: &EngineCtx,
    skill_id: String,
    role_id: String,
) -> Result<(), AppError> {
    crate::services::skill_registry::remove_skill_from_role(&ctx.pool, &skill_id, &role_id).await?;

    let registry = crate::db::skills::list_skills(&ctx.pool)
        .await
        .unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&ctx.pool)
        .await
        .unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&ctx.pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    sync_all_agents_with_mcp(
        &ctx.pool,
        &ctx.agent_config,
        &roles,
        &butler_skills,
        &registry,
        "skill remove from role",
    )
    .await;
    Ok(())
}

pub async fn skill_delete(ctx: &EngineCtx, skill_id: String) -> Result<(), AppError> {
    crate::services::skill_registry::delete_custom_skill(&ctx.pool, &skill_id).await?;

    let registry = crate::db::skills::list_skills(&ctx.pool)
        .await
        .unwrap_or_default();
    let roles = crate::db::roles::list_all_roles(&ctx.pool)
        .await
        .unwrap_or_default();
    let butler_skills = crate::services::butler_config::get_butler_skills(&ctx.pool)
        .await
        .unwrap_or_else(|_| crate::services::butler_config::default_butler_skills());
    sync_all_agents_with_mcp(
        &ctx.pool,
        &ctx.agent_config,
        &roles,
        &butler_skills,
        &registry,
        "skill delete",
    )
    .await;
    Ok(())
}
