use sha2::{Digest, Sha384};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

use crate::error::AppError;

pub type DbPool = SqlitePool;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

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
    repair_legacy_crlf_migration_checksums(pool).await?;
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| AppError::DbError(format!("数据库迁移失败: {}", e)))?;
    tracing::info!("数据库迁移执行完成");
    Ok(())
}

async fn repair_legacy_crlf_migration_checksums(pool: &DbPool) -> Result<(), AppError> {
    let migrations_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::DbError(format!("检查迁移记录失败: {}", e)))?;
    if !migrations_table_exists {
        return Ok(());
    }

    let applied: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations WHERE success = 1")
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::DbError(format!("读取迁移记录失败: {}", e)))?;

    for migration in MIGRATOR.iter() {
        let Some((_, applied_checksum)) = applied
            .iter()
            .find(|(version, _)| *version == migration.version)
        else {
            continue;
        };
        if applied_checksum.as_slice() == migration.checksum.as_ref()
            || migration.sql.contains("\r\n")
        {
            continue;
        }

        let legacy_sql = migration.sql.replace('\n', "\r\n");
        let legacy_checksum = Sha384::digest(legacy_sql.as_bytes());
        if applied_checksum.as_slice() != legacy_checksum.as_slice() {
            continue;
        }

        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ? AND checksum = ?")
            .bind(migration.checksum.as_ref())
            .bind(migration.version)
            .bind(applied_checksum)
            .execute(pool)
            .await
            .map_err(|e| AppError::DbError(format!("修复迁移 {} 校验值失败: {}", migration.version, e)))?;
        tracing::warn!(version = migration.version, "已修复仅由 CRLF/LF 换行差异导致的历史迁移校验值");
    }

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
        crate::db::skills::create_skill_with_source(
            &pool,
            "writer",
            "写作助手",
            "opencode/skills/writer/SKILL.md",
            "hash-2",
            crate::models::skill::SOURCE_TYPE_OPENCODE,
        )
        .await
        .expect("create opencode skill");
        pool.close().await;

        let reopened = init_db(&db_path).await.expect("reopen db");
        let skills = crate::db::skills::list_skills(&reopened)
            .await
            .expect("list skills");
        assert_eq!(skills.len(), 2);
        assert!(skills.iter().any(|skill| skill.name == "daily-review" && skill.source_type == "custom"));
        assert!(skills.iter().any(|skill| skill.name == "writer" && skill.source_type == "opencode"));
    }

    #[tokio::test]
    async fn init_db_preserves_skill_bindings_when_upgrading_source_type_constraint() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)
            .expect("parse db url")
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect pre-upgrade db");
        sqlx::raw_sql(
            "CREATE TABLE skills (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                source_type TEXT NOT NULL CHECK(source_type IN ('custom')),
                managed_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            );
            CREATE TABLE skill_role_bindings (
                skill_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                PRIMARY KEY (skill_id, role_id),
                FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
            );
            INSERT INTO skills (id, name, description, source_type, managed_path, content_hash)
            VALUES ('skill-1', 'daily-review', '日复盘助手', 'custom', 'skills/daily-review/SKILL.md', 'hash-1');
            INSERT INTO skill_role_bindings (skill_id, role_id) VALUES ('skill-1', 'role-1');",
        )
        .execute(&pool)
        .await
        .expect("seed pre-upgrade skills schema");
        pool.close().await;

        let upgraded = init_db(&db_path).await.expect("upgrade db");
        let bindings: Vec<(String, String)> = sqlx::query_as(
            "SELECT skill_id, role_id FROM skill_role_bindings ORDER BY skill_id, role_id",
        )
        .fetch_all(&upgraded)
        .await
        .expect("query bindings");
        assert_eq!(bindings, vec![("skill-1".to_string(), "role-1".to_string())]);
        crate::db::skills::create_skill_with_source(
            &upgraded,
            "writer",
            "写作助手",
            "opencode/skills/writer/SKILL.md",
            "hash-2",
            crate::models::skill::SOURCE_TYPE_OPENCODE,
        )
        .await
        .expect("create opencode skill after upgrade");
    }

    #[tokio::test]
    async fn init_db_repairs_legacy_crlf_migration_checksum_without_losing_data() {
        use sha2::{Digest, Sha384};

        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");
        sqlx::query("INSERT INTO roles (id, name, goal, status) VALUES ('role-1', '保留数据', '验证无损升级', 'active')")
            .execute(&pool)
            .await
            .expect("seed user data");

        let migration = sqlx::migrate!("./migrations")
            .iter()
            .find(|migration| migration.version == 3)
            .expect("migration 3 exists");
        let legacy_sql = migration.sql.replace('\n', "\r\n");
        let legacy_checksum = Sha384::digest(legacy_sql.as_bytes()).to_vec();
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 3")
            .bind(legacy_checksum)
            .execute(&pool)
            .await
            .expect("simulate legacy Windows checksum");
        pool.close().await;

        let reopened = init_db(&db_path)
            .await
            .expect("line-ending-only checksum difference should be repaired");
        let role_name: String = sqlx::query_scalar("SELECT name FROM roles WHERE id = 'role-1'")
            .fetch_one(&reopened)
            .await
            .expect("user data remains readable");
        assert_eq!(role_name, "保留数据");

        let repaired_checksum: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 3")
                .fetch_one(&reopened)
                .await
                .expect("query repaired checksum");
        assert_eq!(repaired_checksum, migration.checksum.as_ref());
    }

    #[tokio::test]
    async fn llm_provider_extension_preserves_data_and_enforces_app_provider_contract() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect pre-upgrade db");
        let pre_upgrade_migrator = Migrator {
            migrations: std::borrow::Cow::Owned(
                MIGRATOR
                    .iter()
                    .filter(|migration| migration.version < 30)
                    .cloned()
                    .collect(),
            ),
            ..Migrator::DEFAULT
        };
        pre_upgrade_migrator
            .run(&pool)
            .await
            .expect("apply migrations 001-029");

        sqlx::query(
            "INSERT INTO llm_configs (
                id, name, provider, base_url, model, api_key_ref, is_default,
                created_at, updated_at, network_location
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind("existing")
        .bind("Existing MiniMax")
        .bind("minimax")
        .bind("https://example.test/v1")
        .bind("model-1")
        .bind("secret-1")
        .bind(1_i64)
        .bind("2026-07-01T00:00:00Z")
        .bind("2026-07-02T00:00:00Z")
        .bind("internal")
        .execute(&pool)
        .await
        .expect("seed migration 029 data");

        MIGRATOR
            .run(&pool)
            .await
            .expect("apply provider extension migration 030");

        let existing: (
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            String,
        ) = sqlx::query_as(
            "SELECT id, name, provider, base_url, model, api_key_ref, is_default,
                    created_at, updated_at, network_location
             FROM llm_configs WHERE id = 'existing'",
        )
        .fetch_one(&pool)
        .await
        .expect("read preserved config");
        assert_eq!(
            existing,
            (
                "existing".to_string(),
                "Existing MiniMax".to_string(),
                "minimax".to_string(),
                "https://example.test/v1".to_string(),
                "model-1".to_string(),
                "secret-1".to_string(),
                1,
                "2026-07-01T00:00:00Z".to_string(),
                "2026-07-02T00:00:00Z".to_string(),
                "internal".to_string(),
            )
        );

        for provider in [
            "openai_compatible",
            "anthropic",
            "minimax",
            "zhipu",
            "deepseek",
            "kimi",
            "bailian",
        ] {
            let id = format!("provider-{provider}");
            sqlx::query(
                "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref)
                 VALUES (?1, ?2, ?3, '', 'model', 'secret')",
            )
            .bind(&id)
            .bind(provider)
            .bind(provider)
            .execute(&pool)
            .await
            .unwrap_or_else(|error| {
                panic!("application provider '{provider}' must persist: {error}")
            });

            let stored: (String, i64, String, String, String) = sqlx::query_as(
                "SELECT provider, is_default, created_at, updated_at, network_location
                 FROM llm_configs WHERE id = ?1",
            )
            .bind(&id)
            .fetch_one(&pool)
            .await
            .expect("read persisted provider config");
            assert_eq!(stored.0, provider, "provider ID must round-trip unchanged");
            assert_eq!(stored.1, 0, "is_default default must remain unchanged");
            assert!(!stored.2.is_empty(), "created_at default must remain active");
            assert!(!stored.3.is_empty(), "updated_at default must remain active");
            assert_eq!(
                stored.4, "external",
                "network_location default must remain unchanged"
            );
        }

        let invalid_provider = sqlx::query(
            "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref)
             VALUES ('invalid-provider', 'Invalid', 'unknown', '', 'model', 'secret')",
        )
        .execute(&pool)
        .await
        .expect_err("unknown providers must remain rejected")
        .to_string();
        assert!(
            invalid_provider.contains("CHECK constraint failed")
                && invalid_provider.contains("provider IN"),
            "unknown provider must fail specifically at the provider CHECK: {invalid_provider}"
        );

        let invalid_network_location = sqlx::query(
            "INSERT INTO llm_configs (
                id, name, provider, base_url, model, api_key_ref, network_location
             ) VALUES ('invalid-network', 'Invalid', 'deepseek', '', 'model', 'secret', 'unknown')",
        )
        .execute(&pool)
        .await
        .expect_err("unknown network locations must remain rejected")
        .to_string();
        assert!(
            invalid_network_location.contains("CHECK constraint failed")
                && invalid_network_location.contains("network_location IN"),
            "network_location CHECK must survive the table rebuild: {invalid_network_location}"
        );
    }

}
