use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

use crate::error::AppError;

pub type DbPool = SqlitePool;

#[derive(Clone)]
pub struct ConversationsPool(pub SqlitePool);

impl std::ops::Deref for ConversationsPool {
    type Target = SqlitePool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn init_db(db_path: &Path) -> Result<DbPool, AppError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::DbError(format!("创建数据目录失败: {}", e)))?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| AppError::DbError(format!("数据库连接选项解析失败: {}", e)))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| AppError::DbError(format!("数据库连接失败: {}", e)))?;

    run_migrations(&pool).await?;

    tracing::info!("数据库初始化完成: {}", db_path.display());
    Ok(pool)
}

async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::DbError(format!("数据库迁移失败: {}", e)))?;
    tracing::info!("数据库迁移执行完成");
    Ok(())
}

pub async fn init_conversations_db(db_path: &Path) -> Result<ConversationsPool, AppError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::DbError(format!("创建数据目录失败: {}", e)))?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| AppError::DbError(format!("对话数据库连接选项解析失败: {}", e)))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| AppError::DbError(format!("对话数据库连接失败: {}", e)))?;

    run_conversations_migrations(&pool).await?;

    tracing::info!("对话数据库初始化完成: {}", db_path.display());
    Ok(ConversationsPool(pool))
}

async fn run_conversations_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    let schema = include_str!("../../migrations/002_conversations.sql");
    sqlx::raw_sql(schema)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("对话数据库迁移失败: {}", e)))?;

    // 为已有数据库添加 thinking_content 列（新建库已包含此列）
    let _ =
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN thinking_content TEXT NOT NULL DEFAULT ''")
            .execute(pool)
            .await;

    // 为已有数据库添加 title 列
    let _ = sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
        .execute(pool)
        .await;

    // Story 2.3: routing_metadata（管家委派审计 JSON），允许 NULL
    let _ = sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
        .execute(pool)
        .await;

    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS message_process_events (
            id TEXT PRIMARY KEY NOT NULL,
            conversation_id TEXT NOT NULL,
            message_id TEXT NOT NULL,
            opencode_session_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            tool_name TEXT,
            status TEXT,
            summary TEXT NOT NULL,
            raw_json TEXT NOT NULL,
            working_directory TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_message_process_events_message_id ON message_process_events(message_id);
        CREATE INDEX IF NOT EXISTS idx_message_process_events_conversation_id ON message_process_events(conversation_id);",
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::DbError(format!("对话过程事件迁移失败: {}", e)))?;

    tracing::info!("对话数据库迁移执行完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn init_db_runs_memories_migration_with_indexes() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");

        let table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'memories'",
        )
        .fetch_optional(&pool)
        .await
        .expect("query memories table");
        assert_eq!(table.as_deref(), Some("memories"));

        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('memories') ORDER BY cid")
                .fetch_all(&pool)
                .await
                .expect("query memories columns");
        assert_eq!(
            columns,
            vec![
                "id",
                "role_id",
                "category",
                "content",
                "source_conversation_id",
                "source_message_ids",
                "created_at",
            ]
        );

        let forgotten_table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'forgotten_memory_sources'",
        )
        .fetch_optional(&pool)
        .await
        .expect("query forgotten memory sources table");
        assert_eq!(forgotten_table.as_deref(), Some("forgotten_memory_sources"));

        let skills_table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'skills'",
        )
        .fetch_optional(&pool)
        .await
        .expect("query skills table");
        assert_eq!(skills_table.as_deref(), Some("skills"));

        let skills_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('skills') ORDER BY cid")
                .fetch_all(&pool)
                .await
                .expect("query skills columns");
        assert_eq!(
            skills_columns,
            vec![
                "id",
                "name",
                "description",
                "source_type",
                "managed_path",
                "content_hash",
                "created_at",
                "updated_at",
            ]
        );

        let skill_role_bindings_table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'skill_role_bindings'",
        )
        .fetch_optional(&pool)
        .await
        .expect("query skill role bindings table");
        assert_eq!(skill_role_bindings_table.as_deref(), Some("skill_role_bindings"));

        let skill_role_binding_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('skill_role_bindings') ORDER BY cid")
                .fetch_all(&pool)
                .await
                .expect("query skill role binding columns");
        assert_eq!(
            skill_role_binding_columns,
            vec!["skill_id", "role_id", "created_at"]
        );

        let indexes: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'memories' AND name LIKE 'idx_memories_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .expect("query memories indexes");
        assert_eq!(
            indexes,
            vec![
                "idx_memories_category",
                "idx_memories_created_at",
                "idx_memories_role_id",
                "idx_memories_source_conversation_id",
                "idx_memories_source_dedupe",
            ]
        );

        let dedupe_sql: String = sqlx::query_scalar(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_memories_source_dedupe'",
        )
        .fetch_one(&pool)
        .await
        .expect("query memory dedupe index sql");
        assert!(dedupe_sql.contains("source_conversation_id"));
        assert!(dedupe_sql.contains("category"));
        assert!(dedupe_sql.contains("source_message_ids"));
        assert!(!dedupe_sql.contains("COALESCE(role_id"));

        let forgotten_indexes: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'forgotten_memory_sources' AND name LIKE 'idx_forgotten_memory_sources_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .expect("query forgotten memory sources indexes");
        assert_eq!(
            forgotten_indexes,
            vec![
                "idx_forgotten_memory_sources_forgotten_at",
                "idx_forgotten_memory_sources_role_id",
                "idx_forgotten_memory_sources_source",
            ]
        );

        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .expect("query foreign_keys pragma");
        assert_eq!(foreign_keys, 1);
    }

    #[tokio::test]
    async fn init_conversations_db_enables_foreign_keys_for_process_event_cleanup() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("conversations.db");
        let pool = init_conversations_db(&db_path).await.expect("init conversations db");

        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool.0)
            .await
            .expect("query foreign_keys pragma");
        assert_eq!(foreign_keys, 1);
    }

    #[tokio::test]
    async fn init_db_persists_skill_registry_across_reopen() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");
        crate::db::skills::create_skill(
            &pool,
            "daily-review",
            "日复盘助手",
            "skills/daily-review/SKILL.md",
            "hash-1",
        )
        .await
        .expect("create skill");
        pool.close().await;

        let reopened = init_db(&db_path).await.expect("reopen db");
        let skills = crate::db::skills::list_skills(&reopened)
            .await
            .expect("list skills");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "daily-review");
        assert_eq!(skills[0].content_hash, "hash-1");
    }
}
