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
    let mut inserted = 0;

    for memory in extracted {
        validate_memory(memory)?;
        let source_message_ids = normalized_source_message_ids(&memory.source_message_ids)?;
        let duplicate_exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM memories WHERE ((role_id IS NULL AND ?1 IS NULL) OR role_id = ?1) AND source_conversation_id = ?2 AND category = ?3 AND source_message_ids = ?4 LIMIT 1",
        )
        .bind(role_id)
        .bind(source_conversation_id)
        .bind(&memory.category)
        .bind(&source_message_ids)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::DbError(format!("查询重复记忆失败: {}", e)))?;
        if duplicate_exists.is_some() {
            continue;
        }

        let id = uuid::Uuid::new_v4().to_string();
        let result = sqlx::query(
            "INSERT OR IGNORE INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(role_id)
        .bind(&memory.category)
        .bind(memory.content.trim())
        .bind(source_conversation_id)
        .bind(&source_message_ids)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("写入记忆失败: {}", e)))?;

        inserted += result.rows_affected() as usize;
    }

    Ok(inserted)
}

pub async fn list_memories(
    pool: &SqlitePool,
    role_id: Option<&str>,
) -> Result<Vec<Memory>, AppError> {
    let rows = if let Some(role_id) = role_id {
        sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE role_id = ?1 ORDER BY created_at DESC",
        )
        .bind(role_id)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Memory>(
            "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories WHERE role_id IS NULL ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    };

    rows.map(dedup_memories)
        .map_err(|e| AppError::DbError(format!("查询记忆失败: {}", e)))
}

pub async fn list_all_memories(pool: &SqlitePool) -> Result<Vec<Memory>, AppError> {
    let rows = sqlx::query_as::<_, Memory>(
        "SELECT id, role_id, category, content, source_conversation_id, source_message_ids, created_at FROM memories ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询全部记忆失败: {}", e)))?;
    Ok(dedup_memories(rows))
}

fn dedup_memories(memories: Vec<Memory>) -> Vec<Memory> {
    let mut seen = HashSet::new();
    memories
        .into_iter()
        .filter(|memory| {
            let key = format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}",
                memory.role_id.as_deref().unwrap_or(""),
                memory.source_conversation_id,
                memory.category,
                normalized_source_message_ids_json(&memory.source_message_ids)
            );
            seen.insert(key)
        })
        .collect()
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
    async fn insert_memories_allows_same_source_in_global_and_role_scope() {
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
            .expect("insert role");
        let role_duplicate = insert_memories(&pool, Some("role-1"), "conv-1", &[memory])
            .await
            .expect("insert duplicate role");

        assert_eq!(global, 1);
        assert_eq!(role, 1);
        assert_eq!(role_duplicate, 0);
        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&pool)
            .await
            .expect("count memories");
        assert_eq!(stored_count, 2);
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
    async fn insert_memories_ignores_same_source_with_reworded_content() {
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
        assert_eq!(second, 0);
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
        assert_eq!(second, 0);
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

        let global = list_memories(&pool, None)
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

        let global = list_memories(&pool, None)
            .await
            .expect("list global memories");
        let role = list_memories(&pool, Some("role-1"))
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
}
