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

    /// 032 原版（3d0f3b6 血统，`git show 3a5c85c~1` 落盘字节）的完整 sha384
    /// 校验和——96 个 hex 字符 / 48 字节，与历史库 v32 记录逐字节一致：
    /// 7CABF4B156F54E053CA7E43F87438DC57F55259854A12FE1A17173AB26A96FC605BE6F0F4C79875133EEEDD90E1CB15A
    /// 原地改动迁移须新增迁移而非改此钉——改此钉等于掩盖「已应用迁移不可变」违约。
    const ORIGINAL_032_CHECKSUM_HEX: &str =
        "7CABF4B156F54E053CA7E43F87438DC57F55259854A12FE1A17173AB26A96FC605BE6F0F4C79875133EEEDD90E1CB15A";

    /// 当前 033 文件的完整 sha384 校验和——96 个 hex 字符 / 48 字节：
    /// CE31D61212D8A33AD8AB382584143245967F94A6A82ABACBC14219E89B697816AAA4145B1949692B7DAE3D787BE9A123
    /// 原地改动迁移须新增迁移而非改此钉——033 被原地改动时既有库将全砖，
    /// 此钉变红是预期报警而非待修缺陷：应回退原地改动（或另起新迁移），勿改此常量。
    const CURRENT_033_CHECKSUM_HEX: &str =
        "CE31D61212D8A33AD8AB382584143245967F94A6A82ABACBC14219E89B697816AAA4145B1949692B7DAE3D787BE9A123";

    /// hex 字符串 → 字节（测试内常量解码用）。
    fn decode_hex(hex: &str) -> Vec<u8> {
        assert!(hex.len() % 2 == 0, "hex 长度须为偶数: {hex}");
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("合法 hex 字节"))
            .collect()
    }

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

    #[tokio::test]
    async fn init_db_upgrades_legacy_auth_sessions_with_last_seen_at_column() {
        // spec-fix-migration-032-checksum 回归 ①：历史态库升级。
        // 构造历史态 = 全量应用后删 v33 记录 + ALTER TABLE 补回 last_seen_at 列 + 预置会话行
        // （等价于 15.4 违约前按原版 032 落盘的历史开发库）。
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");

        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 33")
            .execute(&pool)
            .await
            .expect("remove v33 record");
        sqlx::raw_sql(
            "ALTER TABLE auth_sessions ADD COLUMN last_seen_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'",
        )
        .execute(&pool)
        .await
        .expect("restore legacy last_seen_at column");
        sqlx::query(
            "INSERT INTO auth_sessions (token_hash, created_at)
             VALUES ('hash-legacy-1', '2026-09-18T00:00:00Z'),
                    ('hash-legacy-2', '2026-09-18T01:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("seed legacy session rows");
        pool.close().await;

        // 历史态 init_db：v33 重建重放，列删、行保留、v33 记录在案
        let upgraded = init_db(&db_path).await.expect("upgrade legacy db");

        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('auth_sessions') ORDER BY cid")
                .fetch_all(&upgraded)
                .await
                .expect("query auth_sessions columns");
        assert_eq!(columns, vec!["token_hash", "created_at"]);

        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT token_hash, created_at FROM auth_sessions ORDER BY token_hash",
        )
        .fetch_all(&upgraded)
        .await
        .expect("query preserved session rows");
        assert_eq!(
            rows,
            vec![
                ("hash-legacy-1".to_string(), "2026-09-18T00:00:00Z".to_string()),
                ("hash-legacy-2".to_string(), "2026-09-18T01:00:00Z".to_string()),
            ]
        );

        // v33 记录在案且 checksum 逐字节等于当前 033 文件的完整 sha384 常量。
        // 刻意不用「与 sqlx::migrate! 嵌入值互证」——那是同二进制同义反复
        // （033 被原地改动时两侧同步变、全套件仍绿而既有库全砖，本 build 内
        // 033 注释 hex 笔误事件即其现实样本）；常量独立于二进制，033 被原地
        // 改动时此钉即红——那是预期报警：原地改动迁移须新增迁移而非改此钉。
        let recorded_33: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 33")
                .fetch_one(&upgraded)
                .await
                .expect("query v33 checksum");
        assert_eq!(
            recorded_33,
            decode_hex(CURRENT_033_CHECKSUM_HEX),
            "v33 checksum 必须逐字节等于当前 033 文件完整 sha384（got {}）",
            recorded_33.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );

        // 032 记录的 checksum 必须逐字节等于还原后原版的完整 sha384 常量——
        // 字节级还原的执行级钉；原地改动 032 须新增迁移而非改此钉。
        let recorded_32: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 32")
                .fetch_one(&upgraded)
                .await
                .expect("query v32 checksum");
        assert_eq!(
            recorded_32,
            decode_hex(ORIGINAL_032_CHECKSUM_HEX),
            "v32 checksum 必须逐字节等于 032 原版完整 sha384（got {}）",
            recorded_32.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
    }

    #[tokio::test]
    async fn init_db_fresh_library_drops_last_seen_at_and_restarts_idempotently() {
        // spec-fix-migration-032-checksum 回归 ②：全新库终态 + 二次 init_db 幂等。
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");

        // 全新库：032 原样建表（含列）→ 033 重建删列，终态恰两列
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('auth_sessions') ORDER BY cid")
                .fetch_all(&pool)
                .await
                .expect("query auth_sessions columns");
        assert_eq!(columns, vec!["token_hash", "created_at"]);

        sqlx::query("INSERT INTO auth_sessions (token_hash) VALUES ('hash-fresh-1')")
            .execute(&pool)
            .await
            .expect("seed sentinel session row");
        let before: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .expect("snapshot migration records");
        assert!(
            before.iter().any(|(version, _)| *version == 33),
            "fresh library must have v33 applied"
        );
        pool.close().await;

        // 二次 init_db：零变更幂等重启
        let reopened = init_db(&db_path).await.expect("idempotent restart");
        let after: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&reopened)
                .await
                .expect("snapshot migration records after restart");
        assert_eq!(after, before, "idempotent restart must not alter migration records");

        let sentinel: Option<String> =
            sqlx::query_scalar("SELECT token_hash FROM auth_sessions WHERE token_hash = 'hash-fresh-1'")
                .fetch_optional(&reopened)
                .await
                .expect("query sentinel row");
        assert_eq!(sentinel.as_deref(), Some("hash-fresh-1"));

        let columns_after: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('auth_sessions') ORDER BY cid")
                .fetch_all(&reopened)
                .await
                .expect("query auth_sessions columns after restart");
        assert_eq!(columns_after, vec!["token_hash", "created_at"]);
    }

    #[tokio::test]
    async fn init_db_rejects_window_state_library_with_modified_032_checksum() {
        // spec-fix-migration-032-checksum 回归 ③：窗口态库（I/O 矩阵第 4 行）。
        // 窗口态 = 3a5c85c 与本修复之间新建的库：v32 记录的是被 3a5c85c 原地改写过的
        // 032 checksum（下方 X'…' 即该历史文件版本的 sha384，文件已不在树中）、
        // auth_sessions 无 last_seen_at 列、v33 未应用。
        // 预期：init_db 在 v32 校验处报「migration 32 was previously applied but
        // has been modified」——不做代码级 checksum 兜底是 spec Never 条款的刻意设计。
        // 随后验证 033 头注释给窗口态库的手工修复指引「方案 A」端到端成立。
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");

        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 33")
            .execute(&pool)
            .await
            .expect("remove v33 record");
        sqlx::query(
            "UPDATE _sqlx_migrations SET checksum = X'CC552550A4FD4B64A8E8255559D47945C29AB83FC1361406745888DB25FAF58BFD25E40F6A6970311A19B40B7732627F' WHERE version = 32",
        )
        .execute(&pool)
        .await
        .expect("tamper v32 checksum to the 3a5c85c-modified 032 sha384");
        sqlx::query(
            "INSERT INTO auth_sessions (token_hash, created_at)
             VALUES ('hash-window-1', '2026-09-19T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("seed window-state session row");
        pool.close().await;

        let err = init_db(&db_path)
            .await
            .expect_err("window-state library must be rejected by the checksum contract");
        // 断言锚定 sqlx 0.8.x（本库锁 0.8.6）MigrateError::VersionMismatch 的英文文案
        // 「migration {version} was previously applied but has been modified」；
        // 升级 sqlx 大版本时需复核此文案。
        let msg = err.to_string();
        assert!(
            msg.contains("migration 32 was previously applied but has been modified"),
            "error must be the sqlx version-mismatch on migration 32, got: {msg}"
        );

        // 033 头注释「方案 A」的手工修复：UPDATE 回原版 checksum。常量不另抄一份，
        // 直接从 033 注释解析——注释 hex 若再出转写错误（本 build 内真实发生过），
        // 解析值即错、后续 init_db 仍拒启、本测试即红。
        let migration_33 = sqlx::migrate!("./migrations")
            .iter()
            .find(|migration| migration.version == 33)
            .expect("migration 33 exists");
        let sql_33: &str = &migration_33.sql;
        let marker = "X'";
        let start = sql_33.find(marker).expect("033 注释含 X'…' hex 常量") + marker.len();
        let end = start + sql_33[start..].find('\'').expect("hex 常量以单引号闭合");
        let documented_hex = sql_33[start..end].to_ascii_uppercase();
        assert_eq!(
            documented_hex,
            ORIGINAL_032_CHECKSUM_HEX,
            "033 注释中的手工修复常量必须是 032 原版完整 sha384"
        );

        // 执行方案 A 的 UPDATE（与注释中的 X'…' 字面量等价的绑定参数形式）
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)
            .expect("parse db url")
            .create_if_missing(true)
            .foreign_keys(true);
        let repair_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect for manual repair");
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = 32")
            .bind(decode_hex(&documented_hex))
            .execute(&repair_pool)
            .await
            .expect("execute documented manual repair SQL");
        repair_pool.close().await;

        // 修复后 init_db：v32 校验通过、v33 应用、终态恰两列、预置行保留
        let repaired = init_db(&db_path).await.expect("init db after manual repair");

        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('auth_sessions') ORDER BY cid")
                .fetch_all(&repaired)
                .await
                .expect("query auth_sessions columns");
        assert_eq!(columns, vec!["token_hash", "created_at"]);

        let rows: Vec<String> =
            sqlx::query_scalar("SELECT token_hash FROM auth_sessions ORDER BY token_hash")
                .fetch_all(&repaired)
                .await
                .expect("query preserved session rows");
        assert_eq!(rows, vec!["hash-window-1".to_string()]);

        let recorded_32: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 32")
                .fetch_one(&repaired)
                .await
                .expect("query v32 checksum");
        assert_eq!(
            recorded_32,
            decode_hex(ORIGINAL_032_CHECKSUM_HEX),
            "修复后 v32 checksum 必须逐字节等于 032 原版完整 sha384（got {}）",
            recorded_32.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );

        let recorded_33: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 33")
                .fetch_one(&repaired)
                .await
                .expect("query v33 checksum");
        assert_eq!(
            recorded_33,
            decode_hex(CURRENT_033_CHECKSUM_HEX),
            "修复后 v33 checksum 必须逐字节等于当前 033 文件完整 sha384（got {}）",
            recorded_33.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
    }

    #[tokio::test]
    async fn init_db_reapplies_v33_when_record_missing_on_two_column_terminal_schema() {
        // spec-fix-migration-032-checksum 回归 ④（评审轮 1 G2 补）：真库实际收敛路径。
        // auth_sessions 已是两列终态（033 曾应用过）但 v33 记录缺失——init_db 须
        // 成功重放 033（INSERT SELECT 显式列清单对两列终态血统同样成立）、
        // 预置行保留、v33 以正确 checksum 重放记录在案。本修复的真历史库即以此
        // 路径收敛到修正后 checksum，此前仅有 Implementation Notes 散文担保。
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("egosync.db");
        let pool = init_db(&db_path).await.expect("init db");

        sqlx::query(
            "INSERT INTO auth_sessions (token_hash, created_at)
             VALUES ('hash-converged-1', '2026-09-19T00:00:00Z'),
                    ('hash-converged-2', '2026-09-19T01:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("seed session rows on two-column terminal schema");
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 33")
            .execute(&pool)
            .await
            .expect("remove v33 record");
        pool.close().await;

        let reopened = init_db(&db_path).await.expect("reapply v33 on terminal schema");

        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('auth_sessions') ORDER BY cid")
                .fetch_all(&reopened)
                .await
                .expect("query auth_sessions columns");
        assert_eq!(columns, vec!["token_hash", "created_at"]);

        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT token_hash, created_at FROM auth_sessions ORDER BY token_hash",
        )
        .fetch_all(&reopened)
        .await
        .expect("query preserved session rows");
        assert_eq!(
            rows,
            vec![
                ("hash-converged-1".to_string(), "2026-09-19T00:00:00Z".to_string()),
                ("hash-converged-2".to_string(), "2026-09-19T01:00:00Z".to_string()),
            ]
        );

        let recorded_33: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 33")
                .fetch_one(&reopened)
                .await
                .expect("query v33 checksum");
        assert_eq!(
            recorded_33,
            decode_hex(CURRENT_033_CHECKSUM_HEX),
            "重放的 v33 checksum 必须逐字节等于当前 033 文件完整 sha384（got {}）",
            recorded_33.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
    }

}
