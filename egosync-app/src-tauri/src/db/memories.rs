use std::collections::{BTreeSet, HashSet};

use crate::error::AppError;
use crate::models::memory::{ExtractedMemory, Memory};
use sqlx::SqlitePool;

const ALLOWED_CATEGORIES: &[&str] = &["preference", "task_status", "cognition_update", "fact"];

pub async fn insert_memories(
    pool: &SqlitePool,
    role_id: Option<&str>,
    source_conversation_id: &str,
    extracted: &[ExtractedMemory],
) -> Result<usize, AppError> {
    let mut changed = 0;

    for memory in extracted {
        validate_memory(memory)?;
        let source_message_ids = normalized_source_message_ids(&memory.source_message_ids)?;
        let content = memory.content.trim();

        if forgotten_memory_source_exists(
            pool,
            source_conversation_id,
            &memory.category,
            &source_message_ids,
        )
        .await?
        {
            continue;
        }

        if let Some(existing) = find_memory_by_source(
            pool,
            source_conversation_id,
            &memory.category,
            &source_message_ids,
        )
        .await?
        {
            if should_refresh_existing(&existing, role_id, content) {
                refresh_memory(
                    pool,
                    &existing,
                    role_id,
                    &memory.category,
                    content,
                    source_conversation_id,
                    &source_message_ids,
                )
                .await?;
                changed += 1;
            } else if existing.role_id.as_deref() != role_id
                && existing.role_id.is_some()
                && role_id.is_some()
            {
                tracing::warn!(
                    memory_id = %existing.id,
                    existing_role_id = ?existing.role_id,
                    incoming_role_id = ?role_id,
                    source_conversation_id,
                    category = %memory.category,
                    "同源记忆已归属其他角色，跳过跨角色刷新"
                );
            }
            continue;
        }

        if normalized_content_duplicate_exists(pool, role_id, &memory.category, content).await? {
            continue;
        }

        let id = uuid::Uuid::new_v4().to_string();
        let result = sqlx::query(
            "INSERT OR IGNORE INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(role_id)
        .bind(&memory.category)
        .bind(content)
        .bind(source_conversation_id)
        .bind(&source_message_ids)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("写入记忆失败: {}", e)))?;

        changed += result.rows_affected() as usize;
    }

    Ok(changed)
}

struct ExistingMemory {
    id: String,
    role_id: Option<String>,
    content: String,
}

async fn find_memory_by_source(
    pool: &SqlitePool,
    source_conversation_id: &str,
    category: &str,
    source_message_ids: &str,
) -> Result<Option<ExistingMemory>, AppError> {
    sqlx::query_as::<_, (String, Option<String>, String)>(
        "SELECT id, role_id, content FROM memories WHERE source_conversation_id = ?1 AND category = ?2 AND source_message_ids = ?3 ORDER BY CASE WHEN role_id IS NULL THEN 0 ELSE 1 END DESC, created_at DESC, rowid DESC LIMIT 1",
    )
    .bind(source_conversation_id)
    .bind(category)
    .bind(source_message_ids)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(|(id, role_id, content)| ExistingMemory {
            id,
            role_id,
            content,
        })
    })
    .map_err(|e| AppError::DbError(format!("查询同源记忆失败: {}", e)))
}

async fn forgotten_memory_source_exists(
    pool: &SqlitePool,
    source_conversation_id: &str,
    category: &str,
    source_message_ids: &str,
) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM forgotten_memory_sources WHERE source_conversation_id = ?1 AND category = ?2 AND source_message_ids = ?3)",
    )
    .bind(source_conversation_id)
    .bind(category)
    .bind(source_message_ids)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询已遗忘记忆来源失败: {}", e)))?;
    Ok(exists != 0)
}

fn should_refresh_existing(
    existing: &ExistingMemory,
    incoming_role_id: Option<&str>,
    content: &str,
) -> bool {
    if existing.role_id.is_some()
        && incoming_role_id.is_some()
        && existing.role_id.as_deref() != incoming_role_id
    {
        return false;
    }

    let owner_changes = existing.role_id.is_none() && incoming_role_id.is_some();
    let content_changes =
        normalize_memory_content(&existing.content) != normalize_memory_content(content);
    owner_changes || content_changes
}

async fn refresh_memory(
    pool: &SqlitePool,
    existing: &ExistingMemory,
    incoming_role_id: Option<&str>,
    category: &str,
    content: &str,
    source_conversation_id: &str,
    source_message_ids: &str,
) -> Result<(), AppError> {
    let next_role_id = match (existing.role_id.as_deref(), incoming_role_id) {
        (None, Some(role_id)) => Some(role_id),
        (Some(role_id), _) => Some(role_id),
        (None, None) => None,
    };

    sqlx::query(
        "UPDATE memories SET role_id = ?1, category = ?2, content = ?3, source_conversation_id = ?4, source_message_ids = ?5, created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?6",
    )
    .bind(next_role_id)
    .bind(category)
    .bind(content)
    .bind(source_conversation_id)
    .bind(source_message_ids)
    .bind(&existing.id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("刷新记忆失败: {}", e)))?;

    Ok(())
}

async fn normalized_content_duplicate_exists(
    pool: &SqlitePool,
    role_id: Option<&str>,
    category: &str,
    content: &str,
) -> Result<bool, AppError> {
    let rows = if let Some(role_id) = role_id {
        sqlx::query_scalar::<_, String>(
            "SELECT content FROM memories WHERE role_id = ?1 AND category = ?2",
        )
        .bind(role_id)
        .bind(category)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_scalar::<_, String>(
            "SELECT content FROM memories WHERE role_id IS NULL AND category = ?1",
        )
        .bind(category)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| AppError::DbError(format!("查询同内容记忆失败: {}", e)))?;

    let normalized = normalize_memory_content(content);
    Ok(rows
        .iter()
        .any(|existing| normalize_memory_content(existing) == normalized))
}

pub async fn update_memory_from_extracted(
    pool: &SqlitePool,
    memory_id: &str,
    role_id: Option<&str>,
    source_conversation_id: &str,
    memory: &ExtractedMemory,
) -> Result<bool, AppError> {
    validate_memory(memory)?;
    let source_message_ids = normalized_source_message_ids(&memory.source_message_ids)?;
    let content = memory.content.trim();

    if forgotten_memory_source_exists(
        pool,
        source_conversation_id,
        &memory.category,
        &source_message_ids,
    )
    .await?
    {
        return Ok(false);
    }

    let result = sqlx::query(
        "UPDATE memories SET role_id = ?1, category = ?2, content = ?3, source_conversation_id = ?4, source_message_ids = ?5, created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?6",
    )
    .bind(role_id)
    .bind(&memory.category)
    .bind(content)
    .bind(source_conversation_id)
    .bind(&source_message_ids)
    .bind(memory_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("刷新记忆失败: {}", e)))?;

    Ok(result.rows_affected() > 0)
}

pub async fn get_memory_by_id(
    pool: &SqlitePool,
    memory_id: &str,
) -> Result<Option<Memory>, AppError> {
    sqlx::query_as::<_, Memory>(
        "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE id = ?1",
    )
    .bind(memory_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询记忆失败: {}", e)))
}

pub async fn delete_memory(pool: &SqlitePool, memory_id: &str) -> Result<bool, AppError> {
    let Some(memory) = get_memory_by_id(pool, memory_id).await? else {
        return Ok(false);
    };
    let normalized_content = normalize_memory_content(&memory.content);
    let tombstone_id = uuid::Uuid::new_v4().to_string();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开始遗忘记忆事务失败: {}", e)))?;

    sqlx::query(
        "INSERT INTO forgotten_memory_sources (id, role_id, category, content, normalized_content, source_conversation_id, source_message_ids, forgotten_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         ON CONFLICT(source_conversation_id, category, source_message_ids) DO UPDATE SET
           role_id = excluded.role_id,
           content = excluded.content,
           normalized_content = excluded.normalized_content,
           forgotten_at = excluded.forgotten_at",
    )
    .bind(&tombstone_id)
    .bind(memory.role_id.as_deref())
    .bind(&memory.category)
    .bind(&memory.content)
    .bind(&normalized_content)
    .bind(&memory.source_conversation_id)
    .bind(&memory.source_message_ids)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::DbError(format!("记录已遗忘记忆来源失败: {}", e)))?;

    let result = sqlx::query("DELETE FROM memories WHERE id = ?")
        .bind(memory_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("删除记忆失败: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交遗忘记忆事务失败: {}", e)))?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_memories(
    pool: &SqlitePool,
    role_id: Option<&str>,
    category: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    validate_category_filter(category)?;
    let rows = match (role_id, category) {
        (Some(role_id), Some(category)) => sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE role_id = ?1 AND category = ?2 ORDER BY created_at DESC, rowid DESC",
        )
        .bind(role_id)
        .bind(category)
        .fetch_all(pool)
        .await,
        (Some(role_id), None) => sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE role_id = ?1 AND category != 'task_status' ORDER BY created_at DESC, rowid DESC",
        )
        .bind(role_id)
        .fetch_all(pool)
        .await,
        (None, Some(category)) => sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE role_id IS NULL AND category = ?1 ORDER BY created_at DESC, rowid DESC",
        )
        .bind(category)
        .fetch_all(pool)
        .await,
        (None, None) => sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE role_id IS NULL AND category != 'task_status' ORDER BY created_at DESC, rowid DESC",
        )
        .fetch_all(pool)
        .await,
    };

    let visible = rows
        .map(dedup_memories)
        .map_err(|e| AppError::DbError(format!("查询记忆失败: {}", e)))?;
    paginate_memories(visible, limit, offset)
}

pub async fn list_all_memories(pool: &SqlitePool) -> Result<Vec<Memory>, AppError> {
    list_all_memories_with_options(pool, None, None, None).await
}

pub async fn list_all_memories_with_options(
    pool: &SqlitePool,
    category: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    validate_category_filter(category)?;
    let rows = if let Some(category) = category {
        sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE category = ?1 ORDER BY created_at DESC, rowid DESC",
        )
        .bind(category)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE category != 'task_status' ORDER BY created_at DESC, rowid DESC",
        )
        .fetch_all(pool)
        .await
    };

    let visible = rows
        .map(dedup_memories)
        .map_err(|e| AppError::DbError(format!("查询全部记忆失败: {}", e)))?;
    paginate_memories(visible, limit, offset)
}

pub async fn count_memories(
    pool: &SqlitePool,
    role_id: Option<&str>,
    include_role_memories: bool,
    category: Option<&str>,
) -> Result<usize, AppError> {
    let memories = if include_role_memories {
        list_all_memories_with_options(pool, category, None, None).await?
    } else {
        list_memories(pool, role_id, category, None, None).await?
    };
    Ok(memories.len())
}

fn dedup_memories(memories: Vec<Memory>) -> Vec<Memory> {
    let mut seen = HashSet::new();
    memories
        .into_iter()
        .filter(|memory| {
            let key = format!(
                "{}\u{1f}{}\u{1f}{}",
                memory.source_conversation_id,
                memory.category,
                normalized_source_message_ids_json(&memory.source_message_ids)
            );
            seen.insert(key)
        })
        .collect()
}

fn paginate_memories(
    memories: Vec<Memory>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    let offset = offset.unwrap_or(0);
    if offset < 0 {
        return Err(AppError::ValidationError("offset 不能小于 0".to_string()));
    }
    if matches!(limit, Some(limit) if limit < 0) {
        return Err(AppError::ValidationError("limit 不能小于 0".to_string()));
    }

    let skipped = memories.into_iter().skip(offset as usize);
    Ok(if let Some(limit) = limit {
        skipped.take(limit as usize).collect()
    } else {
        skipped.collect()
    })
}

fn validate_category_filter(category: Option<&str>) -> Result<(), AppError> {
    if let Some(category) = category {
        if !ALLOWED_CATEGORIES.contains(&category) {
            return Err(AppError::ValidationError(format!(
                "无效记忆分类: {}",
                category
            )));
        }
    }
    Ok(())
}

fn normalized_source_message_ids_json(source_message_ids: &str) -> String {
    serde_json::from_str::<Vec<String>>(source_message_ids)
        .ok()
        .and_then(|ids| normalized_source_message_ids(&ids).ok())
        .unwrap_or_else(|| source_message_ids.trim().to_string())
}

fn normalized_source_message_ids(source_message_ids: &[String]) -> Result<String, AppError> {
    let unique = source_message_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    serde_json::to_string(&unique)
        .map_err(|e| AppError::ValidationError(format!("记忆来源消息序列化失败: {}", e)))
}

fn normalize_memory_content(content: &str) -> String {
    let trimmed = content
        .trim()
        .trim_end_matches(|ch| matches!(ch, '.' | '。' | '!' | '！' | '?' | '？'));
    trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn validate_memory(memory: &ExtractedMemory) -> Result<(), AppError> {
    if !ALLOWED_CATEGORIES.contains(&memory.category.as_str()) {
        return Err(AppError::ValidationError(format!(
            "无效记忆分类: {}",
            memory.category
        )));
    }
    if memory.content.trim().is_empty() {
        return Err(AppError::ValidationError("记忆内容不能为空".to_string()));
    }
    if memory.source_message_ids.is_empty() {
        return Err(AppError::ValidationError(
            "记忆必须包含来源消息".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::memory::ExtractedMemory;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create memory db");

        sqlx::raw_sql(include_str!("../../migrations/003_roles.sql"))
            .execute(&pool)
            .await
            .expect("create roles");
        sqlx::raw_sql(include_str!("../../migrations/004_memories.sql"))
            .execute(&pool)
            .await
            .expect("create memories");
        sqlx::raw_sql(include_str!(
            "../../migrations/005_memory_role_scoped_dedupe.sql"
        ))
        .execute(&pool)
        .await
        .expect("migrate memory dedupe index");
        sqlx::raw_sql(include_str!(
            "../../migrations/006_memory_single_owner_dedupe.sql"
        ))
        .execute(&pool)
        .await
        .expect("migrate memory single-owner dedupe");
        sqlx::raw_sql(include_str!(
            "../../migrations/007_forgotten_memory_sources.sql"
        ))
        .execute(&pool)
        .await
        .expect("create forgotten memory sources");

        pool
    }

    fn extracted(category: &str, content: &str, source_message_ids: Vec<&str>) -> ExtractedMemory {
        ExtractedMemory {
            category: category.to_string(),
            content: content.to_string(),
            source_message_ids: source_message_ids.into_iter().map(str::to_string).collect(),
        }
    }

    #[tokio::test]
    async fn insert_memories_writes_global_and_role_memories() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&pool)
            .await
            .expect("insert role");

        let global_count = insert_memories(
            &pool,
            None,
            "conv-global",
            &[extracted("fact", "用户偏好中文沟通", vec!["msg-1"])],
        )
        .await
        .expect("insert global memory");
        let role_count = insert_memories(
            &pool,
            Some("role-1"),
            "conv-role",
            &[extracted(
                "preference",
                "产品规划需要表格输出",
                vec!["msg-2"],
            )],
        )
        .await
        .expect("insert role memory");

        assert_eq!(global_count, 1);
        assert_eq!(role_count, 1);

        let rows: Vec<(Option<String>, String, String, String)> = sqlx::query_as(
            "SELECT role_id, category, content, source_message_ids FROM memories ORDER BY source_conversation_id",
        )
        .fetch_all(&pool)
        .await
        .expect("query memories");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, None);
        assert_eq!(rows[0].1, "fact");
        assert_eq!(rows[0].3, "[\"msg-1\"]");
        assert_eq!(rows[1].0.as_deref(), Some("role-1"));
        assert_eq!(rows[1].1, "preference");
    }

    #[tokio::test]
    async fn insert_memories_keeps_single_owner_for_same_source() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '父亲')")
            .execute(&pool)
            .await
            .expect("insert role");
        let memory = extracted("fact", "儿子喜欢吃薯条", vec!["msg-1"]);

        let global = insert_memories(&pool, None, "conv-1", &[memory.clone()])
            .await
            .expect("insert global");
        let role = insert_memories(&pool, Some("role-1"), "conv-1", &[memory.clone()])
            .await
            .expect("refresh owner to role");
        let role_duplicate = insert_memories(&pool, Some("role-1"), "conv-1", &[memory])
            .await
            .expect("insert duplicate role");

        assert_eq!(global, 1);
        assert_eq!(role, 1);
        assert_eq!(role_duplicate, 0);
        let rows: Vec<(Option<String>, String)> =
            sqlx::query_as("SELECT role_id, content FROM memories")
                .fetch_all(&pool)
                .await
                .expect("query memories");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.as_deref(), Some("role-1"));
        assert_eq!(rows[0].1, "儿子喜欢吃薯条");
    }

    #[tokio::test]
    async fn insert_memories_does_not_downgrade_role_owner_to_global() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '父亲')")
            .execute(&pool)
            .await
            .expect("insert role");
        let memory = extracted("fact", "儿子喜欢吃薯条", vec!["msg-1"]);

        let role = insert_memories(&pool, Some("role-1"), "conv-1", &[memory.clone()])
            .await
            .expect("insert role");
        let global = insert_memories(&pool, None, "conv-1", &[memory])
            .await
            .expect("skip global downgrade");

        assert_eq!(role, 1);
        assert_eq!(global, 0);
        let role_id: Option<String> = sqlx::query_scalar("SELECT role_id FROM memories")
            .fetch_one(&pool)
            .await
            .expect("query owner");
        assert_eq!(role_id.as_deref(), Some("role-1"));
    }

    #[tokio::test]
    async fn insert_memories_rejects_invalid_category_empty_content_and_empty_sources() {
        let pool = setup_test_db().await;

        for memory in [
            extracted("invalid", "有效内容", vec!["msg-1"]),
            extracted("fact", "   ", vec!["msg-1"]),
            extracted("fact", "有效内容", vec![]),
        ] {
            let err = insert_memories(&pool, None, "conv-1", &[memory])
                .await
                .expect_err("invalid memory should fail");
            assert!(matches!(err, crate::error::AppError::ValidationError(_)));
        }
    }

    #[tokio::test]
    async fn insert_memories_ignores_duplicate_source_category_content() {
        let pool = setup_test_db().await;
        let memory = extracted("task_status", "用户正在推进 Story 2.6", vec!["msg-1"]);

        let first = insert_memories(&pool, None, "conv-1", &[memory.clone()])
            .await
            .expect("first insert");
        let second = insert_memories(&pool, None, "conv-1", &[memory])
            .await
            .expect("duplicate insert");

        assert_eq!(first, 1);
        assert_eq!(second, 0);
    }

    #[tokio::test]
    async fn insert_memories_refreshes_same_source_with_reworded_content() {
        let pool = setup_test_db().await;
        let first = insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted(
                "fact",
                "儿子喜欢书法课，每次上课态度都很认真。",
                vec!["msg-1"],
            )],
        )
        .await
        .expect("first insert");
        let second = insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted(
                "fact",
                "儿子喜欢书法课，每次上课都写得很认真。",
                vec!["msg-1"],
            )],
        )
        .await
        .expect("reworded duplicate insert");

        assert_eq!(first, 1);
        assert_eq!(second, 1);
        let rows: Vec<String> = sqlx::query_scalar("SELECT content FROM memories")
            .fetch_all(&pool)
            .await
            .expect("query contents");
        assert_eq!(rows, vec!["儿子喜欢书法课，每次上课都写得很认真。"]);
    }

    #[tokio::test]
    async fn insert_memories_normalizes_source_message_order_for_dedupe() {
        let pool = setup_test_db().await;
        let first = insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted(
                "task_status",
                "周日有书法课和家庭日",
                vec!["msg-2", "msg-1"],
            )],
        )
        .await
        .expect("first insert");
        let second = insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted(
                "task_status",
                "每周周日安排书法课与家庭日",
                vec!["msg-1", "msg-2"],
            )],
        )
        .await
        .expect("reordered duplicate insert");

        assert_eq!(first, 1);
        assert_eq!(second, 1);
        let rows: Vec<String> = sqlx::query_scalar("SELECT content FROM memories")
            .fetch_all(&pool)
            .await
            .expect("query contents");
        assert_eq!(rows, vec!["每周周日安排书法课与家庭日"]);
    }

    #[tokio::test]
    async fn insert_memories_skips_normalized_same_content_in_same_owner() {
        let pool = setup_test_db().await;
        let first = insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted("fact", "Prefers concise updates.", vec!["msg-1"])],
        )
        .await
        .expect("first insert");
        let second = insert_memories(
            &pool,
            None,
            "conv-2",
            &[extracted(
                "fact",
                "  prefers   concise updates  ",
                vec!["msg-2"],
            )],
        )
        .await
        .expect("same normalized content");

        assert_eq!(first, 1);
        assert_eq!(second, 0);
        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&pool)
            .await
            .expect("count memories");
        assert_eq!(stored_count, 1);
    }

    #[tokio::test]
    async fn list_memories_hides_historical_reworded_duplicates() {
        let pool = setup_test_db().await;
        sqlx::query("DROP INDEX idx_memories_source_dedupe")
            .execute(&pool)
            .await
            .expect("drop dedupe index to simulate historical duplicates");
        sqlx::query(
            "INSERT INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids, created_at) VALUES
            ('old', NULL, 'fact', '儿子喜欢书法课，每次上课态度都很认真。', 'conv-1', '[\"msg-1\"]', '2026-05-31T03:39:24Z'),
            ('new', NULL, 'fact', '儿子喜欢书法课，每次上课都写得很认真。', 'conv-1', '[\"msg-1\"]', '2026-05-31T03:40:02Z')",
        )
        .execute(&pool)
        .await
        .expect("insert duplicated historical memories");

        let global = list_memories(&pool, None, None, None, None)
            .await
            .expect("list global memories");
        let all = list_all_memories(&pool).await.expect("list all memories");

        assert_eq!(global.len(), 1);
        assert_eq!(global[0].id, "new");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "new");
    }

    #[tokio::test]
    async fn list_memories_filters_global_and_role_memories() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&pool)
            .await
            .expect("insert role");

        insert_memories(
            &pool,
            None,
            "conv-global",
            &[extracted("fact", "用户喜欢早晨写 PRD", vec!["msg-1"])],
        )
        .await
        .expect("insert global memory");
        insert_memories(
            &pool,
            Some("role-1"),
            "conv-role",
            &[extracted(
                "preference",
                "产品规划需要表格输出",
                vec!["msg-2"],
            )],
        )
        .await
        .expect("insert role memory");

        let global = list_memories(&pool, None, None, None, None)
            .await
            .expect("list global memories");
        let role = list_memories(&pool, Some("role-1"), None, None, None)
            .await
            .expect("list role memories");
        let all = list_all_memories(&pool).await.expect("list all memories");

        assert_eq!(global.len(), 1);
        assert_eq!(global[0].role_id, None);
        assert_eq!(global[0].content, "用户喜欢早晨写 PRD");
        assert_eq!(role.len(), 1);
        assert_eq!(role[0].role_id.as_deref(), Some("role-1"));
        assert_eq!(role[0].content, "产品规划需要表格输出");
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|m| m.role_id.is_none()));
        assert!(all.iter().any(|m| m.role_id.as_deref() == Some("role-1")));
    }

    #[tokio::test]
    async fn list_memories_filters_category_and_paginates_after_dedupe() {
        let pool = setup_test_db().await;
        sqlx::query("DROP INDEX idx_memories_source_dedupe")
            .execute(&pool)
            .await
            .expect("drop dedupe index to simulate historical duplicates");
        sqlx::query(
            "INSERT INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids, created_at) VALUES
            ('m-1', NULL, 'fact', '最早事实', 'conv-1', '[\"msg-1\"]', '2026-05-31T03:39:24Z'),
            ('m-2', NULL, 'preference', '用户喜欢表格', 'conv-2', '[\"msg-2\"]', '2026-05-31T03:40:24Z'),
            ('m-3', NULL, 'preference', '用户喜欢短句', 'conv-3', '[\"msg-3\"]', '2026-05-31T03:41:24Z'),
            ('m-4', NULL, 'preference', '用户喜欢短句改写', 'conv-3', '[\"msg-3\"]', '2026-05-31T03:42:24Z')",
        )
        .execute(&pool)
        .await
        .expect("insert memories");

        let page = list_memories(&pool, None, Some("preference"), Some(1), Some(1))
            .await
            .expect("list paged memories");
        let count = count_memories(&pool, None, false, Some("preference"))
            .await
            .expect("count memories");

        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, "m-2");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn list_all_memories_filters_category_across_global_and_role_scopes() {
        let pool = setup_test_db().await;
        sqlx::query("INSERT INTO roles (id, name) VALUES ('role-1', '产品经理')")
            .execute(&pool)
            .await
            .expect("insert role");
        insert_memories(
            &pool,
            None,
            "conv-global",
            &[extracted("preference", "全局偏好", vec!["msg-1"])],
        )
        .await
        .expect("insert global memory");
        insert_memories(
            &pool,
            Some("role-1"),
            "conv-role",
            &[extracted("preference", "角色偏好", vec!["msg-2"])],
        )
        .await
        .expect("insert role memory");
        insert_memories(
            &pool,
            None,
            "conv-fact",
            &[extracted("fact", "全局事实", vec!["msg-3"])],
        )
        .await
        .expect("insert fact memory");

        let all_preferences = list_all_memories_with_options(&pool, Some("preference"), None, None)
            .await
            .expect("list all preferences");
        let all_count = count_memories(&pool, None, true, Some("preference"))
            .await
            .expect("count all preferences");

        assert_eq!(all_preferences.len(), 2);
        assert_eq!(all_count, 2);
        assert!(all_preferences.iter().any(|m| m.role_id.is_none()));
        assert!(all_preferences
            .iter()
            .any(|m| m.role_id.as_deref() == Some("role-1")));
    }

    #[tokio::test]
    async fn list_memories_hides_task_status_by_default() {
        let pool = setup_test_db().await;
        insert_memories(
            &pool,
            None,
            "conv-fact",
            &[extracted("fact", "通用事实", vec!["msg-1"])],
        )
        .await
        .expect("insert fact memory");
        insert_memories(
            &pool,
            None,
            "conv-task",
            &[extracted("task_status", "后续任务", vec!["msg-2"])],
        )
        .await
        .expect("insert task memory");

        let visible = list_memories(&pool, None, None, None, None)
            .await
            .expect("list visible memories");
        let explicit_task_status = list_memories(&pool, None, Some("task_status"), None, None)
            .await
            .expect("list explicit task status memories");
        let visible_count = count_memories(&pool, None, false, None)
            .await
            .expect("count visible memories");

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].category, "fact");
        assert_eq!(visible_count, 1);
        assert_eq!(explicit_task_status.len(), 1);
        assert_eq!(explicit_task_status[0].category, "task_status");
    }

    #[tokio::test]
    async fn list_memories_orders_same_timestamp_by_newer_rowid() {
        let pool = setup_test_db().await;
        sqlx::query(
            "INSERT INTO memories (id, category, content, source_conversation_id, source_message_ids, created_at) VALUES
            ('old-row', 'fact', '较早写入', 'conv-1', '[\"msg-1\"]', '2026-05-31T03:39:24Z'),
            ('new-row', 'fact', '较晚写入', 'conv-2', '[\"msg-2\"]', '2026-05-31T03:39:24Z')",
        )
        .execute(&pool)
        .await
        .expect("insert same timestamp memories");

        let visible = list_memories(&pool, None, None, None, None)
            .await
            .expect("list memories");

        assert_eq!(
            visible
                .iter()
                .map(|memory| memory.id.as_str())
                .collect::<Vec<_>>(),
            vec!["new-row", "old-row"]
        );
    }

    #[tokio::test]
    async fn list_memories_rejects_invalid_category_filter() {
        let pool = setup_test_db().await;

        let err = list_memories(&pool, None, Some("invalid"), None, None)
            .await
            .expect_err("invalid category should fail");

        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn delete_memory_removes_only_target_and_updates_count() {
        let pool = setup_test_db().await;
        insert_memories(
            &pool,
            None,
            "conv-1",
            &[
                extracted("fact", "用户喜欢早晨写 PRD", vec!["msg-1"]),
                extracted("preference", "用户希望输出简短", vec!["msg-2"]),
            ],
        )
        .await
        .expect("insert memories");
        let target = list_memories(&pool, None, Some("fact"), None, None)
            .await
            .expect("list fact memories")
            .remove(0);

        let deleted = delete_memory(&pool, &target.id)
            .await
            .expect("delete memory");
        let target_after_delete = get_memory_by_id(&pool, &target.id)
            .await
            .expect("get deleted memory");
        let remaining_count = count_memories(&pool, None, false, None)
            .await
            .expect("count memories");
        let remaining = list_memories(&pool, None, None, None, None)
            .await
            .expect("list remaining memories");

        assert!(deleted);
        assert!(target_after_delete.is_none());
        assert_eq!(remaining_count, 1);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].content, "用户希望输出简短");
    }

    #[tokio::test]
    async fn delete_memory_records_forgotten_source_and_removes_visible_memory() {
        let pool = setup_test_db().await;
        insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted("preference", "儿子喜欢吃薯条", vec!["msg-1"])],
        )
        .await
        .expect("insert memory");
        let target = list_memories(&pool, None, Some("preference"), None, None)
            .await
            .expect("list memories")
            .remove(0);

        let deleted = delete_memory(&pool, &target.id)
            .await
            .expect("delete memory");
        let visible = list_memories(&pool, None, None, None, None)
            .await
            .expect("list visible memories");
        let tombstone: (String, String, String, String) = sqlx::query_as(
            "SELECT category, content, source_conversation_id, source_message_ids FROM forgotten_memory_sources",
        )
        .fetch_one(&pool)
        .await
        .expect("query tombstone");

        assert!(deleted);
        assert!(visible.is_empty());
        assert_eq!(tombstone.0, "preference");
        assert_eq!(tombstone.1, "儿子喜欢吃薯条");
        assert_eq!(tombstone.2, "conv-1");
        assert_eq!(tombstone.3, "[\"msg-1\"]");
    }

    #[tokio::test]
    async fn insert_memories_skips_forgotten_same_source() {
        let pool = setup_test_db().await;
        insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted("preference", "儿子喜欢吃薯条", vec!["msg-1"])],
        )
        .await
        .expect("insert memory");
        let target = list_memories(&pool, None, Some("preference"), None, None)
            .await
            .expect("list memories")
            .remove(0);
        delete_memory(&pool, &target.id)
            .await
            .expect("delete memory");

        let reinserted = insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted("preference", "儿子真的喜欢吃薯条", vec!["msg-1"])],
        )
        .await
        .expect("reinsert from same source");
        let visible = list_memories(&pool, None, Some("preference"), None, None)
            .await
            .expect("list visible memories");

        assert_eq!(reinserted, 0);
        assert!(visible.is_empty());
    }

    #[tokio::test]
    async fn insert_memories_allows_same_content_from_new_source_after_forget() {
        let pool = setup_test_db().await;
        insert_memories(
            &pool,
            None,
            "conv-1",
            &[extracted("preference", "儿子喜欢吃薯条", vec!["msg-1"])],
        )
        .await
        .expect("insert memory");
        let target = list_memories(&pool, None, Some("preference"), None, None)
            .await
            .expect("list memories")
            .remove(0);
        delete_memory(&pool, &target.id)
            .await
            .expect("delete memory");

        let inserted = insert_memories(
            &pool,
            None,
            "conv-2",
            &[extracted("preference", "儿子喜欢吃薯条", vec!["msg-2"])],
        )
        .await
        .expect("insert from new source");
        let visible = list_memories(&pool, None, Some("preference"), None, None)
            .await
            .expect("list visible memories");

        assert_eq!(inserted, 1);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].source_conversation_id, "conv-2");
    }

    #[tokio::test]
    async fn delete_memory_returns_false_for_missing_memory() {
        let pool = setup_test_db().await;

        let deleted = delete_memory(&pool, "missing-memory")
            .await
            .expect("delete missing memory");

        assert!(!deleted);
    }
}
