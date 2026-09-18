use sqlx::SqlitePool;

use crate::error::AppError;

pub const ALL_ROLES_BINDING: &str = "__all_roles__";

pub async fn replace_bindings(
    pool: &SqlitePool,
    skill_id: &str,
    all_roles: bool,
    role_ids: &[String],
) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启 Skill 角色绑定事务失败: {}", e)))?;

    sqlx::query("DELETE FROM skill_role_bindings WHERE skill_id = ?1")
        .bind(skill_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("清理 Skill 角色绑定失败: {}", e)))?;

    let targets = if all_roles {
        vec![ALL_ROLES_BINDING.to_string()]
    } else {
        unique_role_ids(role_ids)
    };
    let now = crate::db::settings::chrono_now_pub();
    for role_id in targets {
        sqlx::query(
            "INSERT INTO skill_role_bindings (skill_id, role_id, created_at) VALUES (?1, ?2, ?3)",
        )
        .bind(skill_id)
        .bind(role_id)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("写入 Skill 角色绑定失败: {}", e)))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交 Skill 角色绑定事务失败: {}", e)))?;
    Ok(())
}

pub async fn skill_ids_for_role(pool: &SqlitePool, role_id: &str) -> Result<Vec<String>, AppError> {
    sqlx::query_scalar(
        "SELECT DISTINCT skill_id FROM skill_role_bindings WHERE role_id = ?1 OR role_id = ?2 ORDER BY skill_id ASC",
    )
    .bind(role_id)
    .bind(ALL_ROLES_BINDING)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色可用 Skill 绑定失败: {}", e)))
}

pub async fn all_role_skill_ids(pool: &SqlitePool) -> Result<Vec<String>, AppError> {
    sqlx::query_scalar(
        "SELECT skill_id FROM skill_role_bindings WHERE role_id = ?1 ORDER BY skill_id ASC",
    )
    .bind(ALL_ROLES_BINDING)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询全部角色 Skill 绑定失败: {}", e)))
}

pub async fn role_ids_for_skill(pool: &SqlitePool, skill_id: &str) -> Result<Vec<String>, AppError> {
    sqlx::query_scalar(
        "SELECT role_id FROM skill_role_bindings WHERE skill_id = ?1 ORDER BY role_id ASC",
    )
    .bind(skill_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Skill 角色绑定失败: {}", e)))
}

pub async fn remove_binding_for_role(
    pool: &SqlitePool,
    skill_id: &str,
    role_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM skill_role_bindings WHERE skill_id = ?1 AND role_id = ?2")
        .bind(skill_id)
        .bind(role_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除 Skill 角色绑定失败: {}", e)))?;
    Ok(())
}

pub async fn delete_bindings_for_skill(pool: &SqlitePool, skill_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM skill_role_bindings WHERE skill_id = ?1")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除 Skill 角色绑定失败: {}", e)))?;
    Ok(())
}

fn unique_role_ids(role_ids: &[String]) -> Vec<String> {
    let mut unique = Vec::new();
    for id in role_ids {
        let trimmed = id.trim();
        if !trimmed.is_empty() && trimmed != ALL_ROLES_BINDING && !unique.iter().any(|existing| existing == trimmed) {
            unique.push(trimmed.to_string());
        }
    }
    unique
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
            "CREATE TABLE skill_role_bindings (
                skill_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                PRIMARY KEY (skill_id, role_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create bindings table");

        pool
    }

    #[tokio::test]
    async fn all_roles_binding_applies_to_any_role() {
        let pool = setup_test_db().await;
        replace_bindings(&pool, "skill-1", true, &["role-1".to_string()])
            .await
            .unwrap();

        assert_eq!(skill_ids_for_role(&pool, "future-role").await.unwrap(), vec!["skill-1"]);
        assert_eq!(all_role_skill_ids(&pool).await.unwrap(), vec!["skill-1"]);
        assert_eq!(role_ids_for_skill(&pool, "skill-1").await.unwrap(), vec![ALL_ROLES_BINDING]);
    }

    #[tokio::test]
    async fn explicit_bindings_only_apply_to_selected_roles() {
        let pool = setup_test_db().await;
        replace_bindings(
            &pool,
            "skill-1",
            false,
            &["role-2".to_string(), "role-2".to_string()],
        )
        .await
        .unwrap();

        assert!(skill_ids_for_role(&pool, "role-1").await.unwrap().is_empty());
        assert_eq!(skill_ids_for_role(&pool, "role-2").await.unwrap(), vec!["skill-1"]);
    }
}
