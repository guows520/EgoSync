use tauri::State;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::db::roles;
use crate::error::AppError;
use crate::models::role::{
    CreateRoleInput, Role, UpdateRoleInput, UpdateRoleProactivityInput, UpdateRoleSkillsInput,
};
use crate::services::agent_config::AgentConfigService;

const MIN_ACTIVE_ROLE_ERROR: &str = "至少保留一个角色";

/// Best-effort sync to opencode.json — warn on failure, never block CRUD.
fn sync_warn(result: Result<(), AppError>, action: &str) {
    if let Err(e) = result {
        tracing::warn!("opencode sync ({}) failed: {}", action, e);
    }
}

async fn registry_for_sync(pool: &DbPool) -> Vec<crate::models::skill::SkillRegistryEntry> {
    match crate::db::skills::list_skills(pool).await {
        Ok(registry) => registry,
        Err(e) => {
            // P6: registry 加载失败会导致该角色已启用的自定义 Skill 无法注入 opencode 配置
            //（虽 best-effort 不阻断 CRUD，但属配置不完整），升级为 error 级别便于排查。
            tracing::error!(
                "load skill registry for opencode sync failed: {}; 角色自定义 Skill 同步可能不完整",
                e
            );
            Vec::new()
        }
    }
}

#[tauri::command]
pub async fn role_create(
    input: CreateRoleInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<Role, AppError> {
    validate_role_name(input.name.as_str())?;
    let mut role = roles::create_role(&pool, &input).await?;
    let all_role_skill_ids = crate::db::skill_bindings::all_role_skill_ids(&pool)
        .await
        .unwrap_or_default();
    if !all_role_skill_ids.is_empty() {
        let input = UpdateRoleSkillsInput {
            find_skills: crate::services::role_config::skill_enabled(
                &role.skills_config,
                crate::services::role_config::FIND_SKILLS_KEY,
            ),
            skill_creator: crate::services::role_config::skill_enabled(
                &role.skills_config,
                crate::services::role_config::SKILL_CREATOR_KEY,
            ),
            enabled_skill_ids: Some(all_role_skill_ids),
        };
        role = roles::update_role_skills(&pool, &role.id, &input).await?;
    }
    let registry = registry_for_sync(&pool).await;
    sync_warn(
        crate::services::mcp_server::sync_role_agent_with_mcp(&pool, &agent_config, &role, &registry).await,
        "create",
    );
    Ok(role)
}

#[tauri::command]
pub async fn role_list(pool: State<'_, DbPool>) -> Result<Vec<Role>, AppError> {
    roles::list_active_roles(&pool).await
}

#[tauri::command]
pub async fn role_list_archived(pool: State<'_, DbPool>) -> Result<Vec<Role>, AppError> {
    roles::list_archived_roles(&pool).await
}

#[tauri::command]
pub async fn role_update(
    id: String,
    input: UpdateRoleInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<Role, AppError> {
    if let Some(name) = input.name.as_deref() {
        validate_role_name(name)?;
    }

    let role = roles::update_role(&pool, &id, &input).await?;
    let registry = registry_for_sync(&pool).await;
    sync_warn(
        crate::services::mcp_server::sync_role_agent_with_mcp(&pool, &agent_config, &role, &registry).await,
        "update",
    );
    Ok(role)
}

#[tauri::command]
pub async fn role_update_skills(
    id: String,
    input: UpdateRoleSkillsInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<Role, AppError> {
    let role = roles::update_role_skills(&pool, &id, &input).await?;
    let registry = registry_for_sync(&pool).await;
    sync_warn(
        crate::services::mcp_server::sync_role_agent_with_mcp(&pool, &agent_config, &role, &registry).await,
        "update_skills",
    );
    Ok(role)
}

#[tauri::command]
pub async fn role_update_proactivity(
    id: String,
    input: UpdateRoleProactivityInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<Role, AppError> {
    let role = roles::update_role_proactivity(&pool, &id, &input).await?;
    let registry = registry_for_sync(&pool).await;
    sync_warn(
        crate::services::mcp_server::sync_role_agent_with_mcp(&pool, &agent_config, &role, &registry).await,
        "update_proactivity",
    );
    Ok(role)
}

#[tauri::command]
pub async fn role_archive(
    id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<Role, AppError> {
    ensure_can_remove_active_role(&pool).await?;
    let role = roles::archive_role(&pool, &id).await?;
    sync_warn(agent_config.sync_role_archived(&role.id), "archive");
    Ok(role)
}

#[tauri::command]
pub async fn role_restore(
    id: String,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<Role, AppError> {
    let role = roles::restore_role(&pool, &id).await?;
    let registry = registry_for_sync(&pool).await;
    sync_warn(
        crate::services::mcp_server::sync_role_agent_with_mcp(&pool, &agent_config, &role, &registry).await,
        "restore",
    );
    Ok(role)
}

#[tauri::command]
pub async fn role_delete(
    id: String,
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<(), AppError> {
    let role = roles::get_role(&pool, &id).await?;
    if role.status == "active" {
        ensure_can_remove_active_role(&pool).await?;
    }

    roles::delete_role(&pool, &conv_pool, &id).await?;
    sync_warn(agent_config.sync_role_deleted(&id), "delete");
    Ok(())
}

fn validate_role_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::ValidationError("角色名称不能为空".to_string()));
    }
    Ok(())
}

async fn ensure_can_remove_active_role(pool: &DbPool) -> Result<(), AppError> {
    let active_count = roles::count_active_roles(pool).await?;
    if active_count <= 1 {
        return Err(AppError::ValidationError(MIN_ACTIVE_ROLE_ERROR.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_role_command_test_db(active_count: usize) -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT 'target',
                color TEXT NOT NULL DEFAULT '#4F46E5',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
                skills_config TEXT NOT NULL DEFAULT '{}',
                proactivity_level TEXT NOT NULL DEFAULT 'moderate',
                archived_at TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create roles table");

        for index in 0..active_count {
            sqlx::query("INSERT INTO roles (id, name) VALUES (?1, ?2)")
                .bind(format!("role-{}", index))
                .bind(format!("角色 {}", index))
                .execute(&pool)
                .await
                .expect("failed to insert active role");
        }

        pool
    }

    #[test]
    fn validate_role_name_rejects_blank_name() {
        let result = validate_role_name("  ");
        assert!(
            matches!(result, Err(AppError::ValidationError(message)) if message == "角色名称不能为空")
        );
    }

    #[tokio::test]
    async fn ensure_can_remove_active_role_rejects_last_active_role() {
        let pool = setup_role_command_test_db(1).await;
        let result = ensure_can_remove_active_role(&pool).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(message)) if message == MIN_ACTIVE_ROLE_ERROR)
        );
    }

    #[tokio::test]
    async fn ensure_can_remove_active_role_allows_when_multiple_active_roles_exist() {
        let pool = setup_role_command_test_db(2).await;
        ensure_can_remove_active_role(&pool).await.unwrap();
    }
}
