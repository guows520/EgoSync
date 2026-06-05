use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::skill::SkillRegistryEntry;

const SKILL_SELECT_COLUMNS: &str =
    "id, name, description, source_type, managed_path, content_hash, created_at, updated_at";

pub async fn list_skills(pool: &SqlitePool) -> Result<Vec<SkillRegistryEntry>, AppError> {
    sqlx::query_as::<_, SkillRegistryEntry>(&format!(
        "SELECT {} FROM skills ORDER BY created_at ASC",
        SKILL_SELECT_COLUMNS
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Skill 列表失败: {}", e)))
}

pub async fn get_skill(pool: &SqlitePool, id: &str) -> Result<SkillRegistryEntry, AppError> {
    sqlx::query_as::<_, SkillRegistryEntry>(&format!(
        "SELECT {} FROM skills WHERE id = ?1",
        SKILL_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询 Skill 失败: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("Skill {} 不存在", id)))
}

pub async fn find_skill_by_content_hash(
    pool: &SqlitePool,
    content_hash: &str,
) -> Result<Option<SkillRegistryEntry>, AppError> {
    sqlx::query_as::<_, SkillRegistryEntry>(&format!(
        "SELECT {} FROM skills WHERE content_hash = ?1",
        SKILL_SELECT_COLUMNS
    ))
    .bind(content_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("按内容查询 Skill 失败: {}", e)))
}

pub async fn find_skill_by_name(
    pool: &SqlitePool,
    name: &str,
) -> Result<Option<SkillRegistryEntry>, AppError> {
    sqlx::query_as::<_, SkillRegistryEntry>(&format!(
        "SELECT {} FROM skills WHERE name = ?1 AND source_type = 'custom'",
        SKILL_SELECT_COLUMNS
    ))
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("按名称查询 Skill 失败: {}", e)))
}

pub async fn create_skill(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    managed_path: &str,
    content_hash: &str,
) -> Result<SkillRegistryEntry, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::settings::chrono_now_pub();
    sqlx::query(
        "INSERT INTO skills (id, name, description, source_type, managed_path, content_hash, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'custom', ?4, ?5, ?6, ?7)",
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .bind(managed_path)
    .bind(content_hash)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| map_skill_unique_error(e, name))?;

    get_skill(pool, &id).await
}

/// 将 SQLite 唯一约束冲突映射为友好的 ValidationError，避免向用户暴露裸 DbError。
/// 触发场景：并发导入（TOCTOU，预览查重与插入非原子）撞 content_hash/name 唯一索引。
fn map_skill_unique_error(e: sqlx::Error, name: &str) -> AppError {
    let message = e.to_string();
    if message.contains("UNIQUE constraint failed") {
        if message.contains("content_hash") {
            AppError::ValidationError("该 Skill 内容已存在，请勿重复导入".to_string())
        } else {
            AppError::ValidationError(format!("已存在同名 Skill「{}」", name))
        }
    } else {
        AppError::DbError(format!("创建 Skill 记录失败: {}", e))
    }
}

pub async fn update_skill_metadata(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: &str,
    managed_path: &str,
    content_hash: &str,
) -> Result<SkillRegistryEntry, AppError> {
    get_skill(pool, id).await?;
    // P1: 覆盖前检测 name 是否被「其它」记录占用，避免撞 idx_skills_name_source_type
    // 唯一约束后返回裸 DbError。命中则返回友好提示。
    if let Some(existing) = find_skill_by_name(pool, name).await? {
        if existing.id != id {
            return Err(AppError::ValidationError(format!(
                "已存在同名 Skill「{}」，无法覆盖",
                name
            )));
        }
    }
    let now = crate::db::settings::chrono_now_pub();
    let result = sqlx::query(
        "UPDATE skills
         SET name = ?1,
             description = ?2,
             managed_path = ?3,
             content_hash = ?4,
             updated_at = ?5
         WHERE id = ?6",
    )
    .bind(name)
    .bind(description)
    .bind(managed_path)
    .bind(content_hash)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| map_skill_unique_error(e, name))?;

    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("Skill {} 不存在", id)));
    }

    get_skill(pool, id).await
}

/// 删除 registry 中的一条 Skill 记录（P3）。
pub async fn delete_skill(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM skills WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除 Skill 记录失败: {}", e)))?;
    if result.rows_affected() != 1 {
        return Err(AppError::NotFound(format!("Skill {} 不存在", id)));
    }
    Ok(())
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
            "CREATE TABLE skills (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                source_type TEXT NOT NULL CHECK(source_type IN ('custom')),
                managed_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create skills table");
        sqlx::query("CREATE UNIQUE INDEX idx_skills_content_hash ON skills(content_hash)")
            .execute(&pool)
            .await
            .expect("failed to create hash index");
        sqlx::query("CREATE UNIQUE INDEX idx_skills_name_source_type ON skills(name, source_type)")
            .execute(&pool)
            .await
            .expect("failed to create name index");

        pool
    }

    #[tokio::test]
    async fn create_and_list_skills() {
        let pool = setup_test_db().await;
        let skill = create_skill(
            &pool,
            "daily-review",
            "日复盘",
            "skills/daily-review/SKILL.md",
            "hash-1",
        )
        .await
        .unwrap();

        assert_eq!(skill.name, "daily-review");
        assert_eq!(skill.source_type, "custom");
        let skills = list_skills(&pool).await.unwrap();
        assert_eq!(skills, vec![skill]);
    }

    #[tokio::test]
    async fn find_duplicate_by_hash_and_name() {
        let pool = setup_test_db().await;
        let skill = create_skill(&pool, "daily-review", "日复盘", "path", "hash-1")
            .await
            .unwrap();

        assert_eq!(
            find_skill_by_content_hash(&pool, "hash-1").await.unwrap(),
            Some(skill.clone())
        );
        assert_eq!(
            find_skill_by_name(&pool, "daily-review").await.unwrap(),
            Some(skill)
        );
    }

    #[tokio::test]
    async fn update_skill_metadata_preserves_id_and_created_at() {
        let pool = setup_test_db().await;
        let skill = create_skill(&pool, "daily-review", "日复盘", "path", "hash-1")
            .await
            .unwrap();

        let updated = update_skill_metadata(
            &pool,
            &skill.id,
            "daily-review",
            "更新后的说明",
            "path2",
            "hash-2",
        )
        .await
        .unwrap();

        assert_eq!(updated.id, skill.id);
        assert_eq!(updated.created_at, skill.created_at);
        assert_eq!(updated.description, "更新后的说明");
        assert_eq!(updated.managed_path, "path2");
        assert_eq!(updated.content_hash, "hash-2");
    }
}
