use sqlx::SqlitePool;

use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::models::role::{
    CreateRoleInput, Role, UpdateRoleInput, UpdateRoleProactivityInput, UpdateRoleSkillsInput,
};

const ROLE_SELECT_COLUMNS: &str = "id, name, icon, color, goal, personality_prompt, status, energy, energy_updated_at, skills_config, proactivity_level, archived_at, created_at, updated_at";

pub async fn create_role(pool: &SqlitePool, input: &CreateRoleInput) -> Result<Role, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let icon = input.icon.as_deref().unwrap_or("🎯");
    let color = input.color.as_deref().unwrap_or("#6366F1");
    let goal = input.goal.as_deref().unwrap_or("");
    let now = crate::db::settings::chrono_now_pub();

    sqlx::query(
        "INSERT INTO roles (id, name, icon, color, goal, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(icon)
    .bind(color)
    .bind(goal)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建角色失败: {}", e)))?;

    get_role(pool, &id).await
}

pub async fn list_active_roles(pool: &SqlitePool) -> Result<Vec<Role>, AppError> {
    list_roles_by_status(pool, "active", "created_at ASC").await
}

pub async fn list_archived_roles(pool: &SqlitePool) -> Result<Vec<Role>, AppError> {
    list_roles_by_status(pool, "archived", "archived_at DESC, updated_at DESC").await
}

/// All roles regardless of status — used for full sync to opencode.json.
pub async fn list_all_roles(pool: &SqlitePool) -> Result<Vec<Role>, AppError> {
    let roles = sqlx::query_as::<_, Role>(&format!(
        "SELECT {} FROM roles ORDER BY created_at ASC",
        ROLE_SELECT_COLUMNS
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询全部角色失败: {}", e)))?;

    Ok(roles)
}

async fn list_roles_by_status(
    pool: &SqlitePool,
    status: &str,
    order_by: &str,
) -> Result<Vec<Role>, AppError> {
    let roles = sqlx::query_as::<_, Role>(&format!(
        "SELECT {} FROM roles WHERE status = ?1 ORDER BY {}",
        ROLE_SELECT_COLUMNS, order_by
    ))
    .bind(status)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色列表失败: {}", e)))?;

    Ok(roles)
}

pub async fn get_role(pool: &SqlitePool, id: &str) -> Result<Role, AppError> {
    sqlx::query_as::<_, Role>(&format!(
        "SELECT {} FROM roles WHERE id = ?1",
        ROLE_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("角色 {} 不存在", id)))
}

pub async fn update_role(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateRoleInput,
) -> Result<Role, AppError> {
    get_role(pool, id).await?;

    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE roles
         SET name = COALESCE(?1, name),
             icon = COALESCE(?2, icon),
             color = COALESCE(?3, color),
             goal = COALESCE(?4, goal),
             personality_prompt = COALESCE(?5, personality_prompt),
             updated_at = ?6
         WHERE id = ?7",
    )
    .bind(input.name.as_deref())
    .bind(input.icon.as_deref())
    .bind(input.color.as_deref())
    .bind(input.goal.as_deref())
    .bind(input.personality_prompt.as_deref())
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新角色失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }

    get_role(pool, id).await
}

pub async fn update_role_skills(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateRoleSkillsInput,
) -> Result<Role, AppError> {
    let current = get_role(pool, id).await?;

    let skills_config = crate::services::role_config::normalize_skills_config_with_existing(
        &current.skills_config,
        input,
    )?;
    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE roles
         SET skills_config = ?1,
             updated_at = ?2
         WHERE id = ?3",
    )
    .bind(&skills_config)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新角色 Skill 配置失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }

    get_role(pool, id).await
}

/// 直接写入角色 skills_config 原文（P3：删除 Skill 后清理 enabledSkillIds 死 id 时使用，
/// 配置已由 role_config helper 规范化，故不再二次 normalize）。
pub async fn set_role_skills_config_raw(
    pool: &SqlitePool,
    id: &str,
    skills_config: &str,
) -> Result<(), AppError> {
    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE roles
         SET skills_config = ?1,
             updated_at = ?2
         WHERE id = ?3",
    )
    .bind(skills_config)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新角色 Skill 配置失败: {}", e)))?;
    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }
    Ok(())
}

pub async fn update_role_proactivity(
    pool: &SqlitePool,
    id: &str,
    input: &UpdateRoleProactivityInput,
) -> Result<Role, AppError> {
    get_role(pool, id).await?;

    let proactivity_level =
        crate::services::role_config::normalize_proactivity_level(&input.proactivity_level)?;
    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE roles
         SET proactivity_level = ?1,
             updated_at = ?2
         WHERE id = ?3",
    )
    .bind(&proactivity_level)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新角色主动性失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }

    get_role(pool, id).await
}

pub async fn archive_role(pool: &SqlitePool, id: &str) -> Result<Role, AppError> {
    get_role(pool, id).await?;

    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE roles
         SET status = 'archived', archived_at = ?1, updated_at = ?1
         WHERE id = ?2",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("归档角色失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }

    get_role(pool, id).await
}

pub async fn restore_role(pool: &SqlitePool, id: &str) -> Result<Role, AppError> {
    get_role(pool, id).await?;

    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE roles
         SET status = 'active', archived_at = NULL, updated_at = ?1
         WHERE id = ?2",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("恢复角色失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }

    get_role(pool, id).await
}

pub async fn delete_role(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    id: &str,
) -> Result<(), AppError> {
    get_role(pool, id).await?;

    crate::db::conversations::delete_conversations_by_role(conv_pool, id).await?;

    let result = sqlx::query("DELETE FROM roles WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除角色失败: {}", e)))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", id)));
    }

    Ok(())
}

pub async fn update_energy(
    pool: &SqlitePool,
    role_id: &str,
    energy: i32,
    energy_updated_at: &str,
) -> Result<(), AppError> {
    let result = sqlx::query(
        "UPDATE roles SET energy = ?1, energy_updated_at = ?2 WHERE id = ?3",
    )
    .bind(energy)
    .bind(energy_updated_at)
    .bind(role_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("更新能量值失败: {}", e)))?;
    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("角色 {} 不存在", role_id)));
    }
    Ok(())
}

pub async fn count_active_roles(pool: &SqlitePool) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles WHERE status = 'active'")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DbError(format!("统计 active 角色失败: {}", e)))?;

    Ok(count)
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
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT '🎯',
                color TEXT NOT NULL DEFAULT '#6366F1',
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
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create roles table");

        pool
    }

    async fn create_test_role(pool: &SqlitePool, name: &str) -> Role {
        create_role(
            pool,
            &CreateRoleInput {
                name: name.to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("管理产品规划".to_string()),
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_and_list_roles() {
        let pool = setup_test_db().await;
        let input = CreateRoleInput {
            name: "产品经理".to_string(),
            icon: Some("📋".to_string()),
            color: Some("#4F46E5".to_string()),
            goal: Some("管理产品规划".to_string()),
        };

        let role = create_role(&pool, &input).await.unwrap();
        assert_eq!(role.name, "产品经理");
        assert_eq!(role.icon, "📋");
        assert_eq!(role.color, "#4F46E5");
        assert_eq!(role.goal, "管理产品规划");
        assert_eq!(role.status, "active");
        assert_eq!(role.energy, 100);

        let roles = list_active_roles(&pool).await.unwrap();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].id, role.id);
    }

    #[tokio::test]
    async fn test_create_role_with_defaults() {
        let pool = setup_test_db().await;
        let input = CreateRoleInput {
            name: "学习者".to_string(),
            icon: None,
            color: None,
            goal: None,
        };

        let role = create_role(&pool, &input).await.unwrap();
        assert_eq!(role.name, "学习者");
        assert_eq!(role.icon, "🎯");
        assert_eq!(role.color, "#6366F1");
        assert_eq!(role.goal, "");
    }

    #[tokio::test]
    async fn test_get_nonexistent_role() {
        let pool = setup_test_db().await;
        let result = get_role(&pool, "nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_role_allowed_fields() {
        let pool = setup_test_db().await;
        let role = create_test_role(&pool, "产品经理").await;

        let updated = update_role(
            &pool,
            &role.id,
            &UpdateRoleInput {
                name: Some("学习者".to_string()),
                icon: Some("book-open".to_string()),
                color: Some("#10B981".to_string()),
                goal: Some("保持学习节奏".to_string()),
                personality_prompt: Some("用好奇、鼓励的语气回应".to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "学习者");
        assert_eq!(updated.icon, "book-open");
        assert_eq!(updated.color, "#10B981");
        assert_eq!(updated.goal, "保持学习节奏");
        assert_eq!(updated.personality_prompt, "用好奇、鼓励的语气回应");
        assert_eq!(updated.status, "active");
        assert_ne!(updated.updated_at, "2026-01-01T00:00:00Z");
    }

    #[tokio::test]
    async fn test_archive_and_restore_role() {
        let pool = setup_test_db().await;
        let role = create_test_role(&pool, "产品经理").await;

        let archived = archive_role(&pool, &role.id).await.unwrap();
        assert_eq!(archived.status, "archived");
        assert!(archived.archived_at.is_some());

        let active_roles = list_active_roles(&pool).await.unwrap();
        assert!(active_roles.is_empty());

        let archived_roles = list_archived_roles(&pool).await.unwrap();
        assert_eq!(archived_roles.len(), 1);
        assert_eq!(archived_roles[0].id, role.id);

        let restored = restore_role(&pool, &role.id).await.unwrap();
        assert_eq!(restored.status, "active");
        assert!(restored.archived_at.is_none());
    }

    #[tokio::test]
    async fn test_update_role_skills_persists_meta_skill_json() {
        let pool = setup_test_db().await;
        let role = create_test_role(&pool, "产品经理").await;

        let updated = update_role_skills(
            &pool,
            &role.id,
            &UpdateRoleSkillsInput {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: None,
            },
        )
        .await
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&updated.skills_config).unwrap();
        assert_eq!(parsed[crate::services::role_config::FIND_SKILLS_KEY], true);
        assert_eq!(
            parsed[crate::services::role_config::SKILL_CREATOR_KEY],
            false
        );
        assert_ne!(updated.updated_at, "2026-01-01T00:00:00Z");
    }

    #[tokio::test]
    async fn test_update_role_skills_preserves_existing_extensions() {
        let pool = setup_test_db().await;
        let role = create_test_role(&pool, "产品经理").await;
        sqlx::query("UPDATE roles SET skills_config = ?1 WHERE id = ?2")
            .bind(r#"{"enabledSkillIds":["custom-a"],"permissions":{"bash":"ask"},"future":{"mcp":["x"]}}"#)
            .bind(&role.id)
            .execute(&pool)
            .await
            .unwrap();

        let updated = update_role_skills(
            &pool,
            &role.id,
            &UpdateRoleSkillsInput {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: None,
            },
        )
        .await
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&updated.skills_config).unwrap();
        assert_eq!(parsed["enabledSkillIds"][0], "custom-a");
        assert_eq!(parsed["permissions"]["bash"], "ask");
        assert_eq!(parsed["future"]["mcp"][0], "x");
        assert_eq!(parsed["meta"]["findSkills"], true);
        assert_eq!(parsed["meta"]["skillCreator"], false);
    }

    #[tokio::test]
    async fn test_update_role_proactivity_rejects_invalid_level() {
        let pool = setup_test_db().await;
        let role = create_test_role(&pool, "产品经理").await;

        let err = update_role_proactivity(
            &pool,
            &role.id,
            &UpdateRoleProactivityInput {
                proactivity_level: "medium".to_string(),
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::ValidationError(_)));

        let updated = update_role_proactivity(
            &pool,
            &role.id,
            &UpdateRoleProactivityInput {
                proactivity_level: "proactive".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.proactivity_level, "proactive");
    }

    #[tokio::test]
    async fn test_count_active_roles() {
        let pool = setup_test_db().await;
        create_test_role(&pool, "产品经理").await;
        create_test_role(&pool, "学习者").await;

        assert_eq!(count_active_roles(&pool).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_update_energy_writes_energy_and_timestamp() {
        let pool = setup_test_db().await;
        let role = create_test_role(&pool, "产品经理").await;

        update_energy(&pool, &role.id, 42, "2026-06-23T12:00:00Z")
            .await
            .unwrap();

        let updated = get_role(&pool, &role.id).await.unwrap();
        assert_eq!(updated.energy, 42);
        assert_eq!(
            updated.energy_updated_at.as_deref(),
            Some("2026-06-23T12:00:00Z")
        );
    }

    #[tokio::test]
    async fn test_update_energy_returns_not_found_for_missing_role() {
        let pool = setup_test_db().await;

        let err = update_energy(&pool, "nonexistent", 50, "2026-06-23T12:00:00Z")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
