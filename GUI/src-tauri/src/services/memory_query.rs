use std::collections::HashSet;

use crate::db::conversations;
use crate::db::memories;
use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::memory::{Memory, MemorySourceMessage};

pub async fn list_role_memories(
    pool: &DbPool,
    role_id: Option<&str>,
    category: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    memories::list_memories(pool, role_id, category, limit, offset).await
}

pub async fn list_all_memories(
    pool: &DbPool,
    category: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    memories::list_all_memories_with_options(pool, category, limit, offset).await
}

pub async fn count_memories(
    pool: &DbPool,
    role_id: Option<&str>,
    include_role_memories: bool,
    category: Option<&str>,
) -> Result<usize, AppError> {
    memories::count_memories(pool, role_id, include_role_memories, category).await
}

pub async fn get_source_messages(
    main_pool: &DbPool,
    conversations_pool: &ConversationsPool,
    memory_id: &str,
) -> Result<Vec<MemorySourceMessage>, AppError> {
    let memory = memories::get_memory_by_id(main_pool, memory_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("记忆不存在: {}", memory_id)))?;
    let source_message_ids = parse_source_message_ids(&memory.source_message_ids)?;
    if source_message_ids.is_empty() {
        return Ok(Vec::new());
    }

    let source_id_set = source_message_ids.into_iter().collect::<HashSet<_>>();
    let messages =
        conversations::list_messages(conversations_pool, &memory.source_conversation_id).await?;
    let source_messages = messages
        .into_iter()
        .filter(|message| message.role == "user")
        .filter(|message| source_id_set.contains(&message.id))
        .map(|message| MemorySourceMessage {
            id: message.id,
            conversation_id: message.conversation_id,
            role: message.role,
            content: message.content,
            created_at: message.created_at,
            is_source: true,
        })
        .collect::<Vec<_>>();

    if source_messages.len() < source_id_set.len() {
        // 部分来源消息缺失（被删除/重生成）或为内部 system 消息时，
        // 仅记录诊断日志并返回可用子集，而非整体置空，
        // 以便用户仍能看到尚存的有效来源原文。
        tracing::warn!(
            memory_id = %memory.id,
            conversation_id = %memory.source_conversation_id,
            expected = source_id_set.len(),
            found = source_messages.len(),
            "部分来源消息不可用，仅返回可用子集"
        );
    }

    Ok(source_messages)
}

fn parse_source_message_ids(value: &str) -> Result<Vec<String>, AppError> {
    serde_json::from_str::<Vec<String>>(value)
        .map_err(|e| AppError::ValidationError(format!("记忆来源消息不是有效 JSON 数组: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pool::ConversationsPool;
    use crate::models::memory::ExtractedMemory;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn setup_main_pool() -> SqlitePool {
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

    async fn setup_conversations_pool() -> ConversationsPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create conversations db");

        sqlx::raw_sql(include_str!("../../migrations/002_conversations.sql"))
            .execute(&pool)
            .await
            .expect("create conversations schema");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN thinking_content TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("add thinking_content column");
        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("add title column");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
            .execute(&pool)
            .await
            .expect("add routing_metadata column");

        ConversationsPool(pool)
    }

    fn extracted(category: &str, content: &str, source_message_ids: Vec<&str>) -> ExtractedMemory {
        ExtractedMemory {
            category: category.to_string(),
            content: content.to_string(),
            source_message_ids: source_message_ids.into_iter().map(str::to_string).collect(),
        }
    }

    #[tokio::test]
    async fn get_source_messages_returns_matching_messages_in_conversation_order() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversations_pool().await;
        let conversation = conversations::create_conversation(&conv_pool, None)
            .await
            .expect("create conversation");
        let msg_1 =
            conversations::insert_message(&conv_pool, &conversation.id, "user", "第一句", true)
                .await
                .expect("insert msg 1");
        let _msg_2 = conversations::insert_message(
            &conv_pool,
            &conversation.id,
            "assistant",
            "非来源",
            true,
        )
        .await
        .expect("insert msg 2");
        let msg_3 =
            conversations::insert_message(&conv_pool, &conversation.id, "user", "第三句", true)
                .await
                .expect("insert msg 3");
        memories::insert_memories(
            &main_pool,
            None,
            &conversation.id,
            &[extracted(
                "fact",
                "用户提到了两句来源",
                vec![&msg_3.id, &msg_1.id],
            )],
        )
        .await
        .expect("insert memory");
        let memory = memories::list_memories(&main_pool, None, None, None, None)
            .await
            .expect("list memories")
            .remove(0);

        let source_messages = get_source_messages(&main_pool, &conv_pool, &memory.id)
            .await
            .expect("get source messages");

        assert_eq!(source_messages.len(), 2);
        assert_eq!(source_messages[0].id, msg_1.id);
        assert_eq!(source_messages[1].id, msg_3.id);
        assert!(source_messages.iter().all(|message| message.is_source));
        let value = serde_json::to_value(&source_messages[0]).expect("serialize dto");
        assert!(value.get("thinkingContent").is_none());
        assert!(value.get("routingMetadata").is_none());
    }

    #[tokio::test]
    async fn get_source_messages_excludes_non_user_messages_but_keeps_valid_user_messages() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversations_pool().await;
        let conversation = conversations::create_conversation(&conv_pool, None)
            .await
            .expect("create conversation");
        let msg_1 =
            conversations::insert_message(&conv_pool, &conversation.id, "user", "第一句", true)
                .await
                .expect("insert msg 1");
        let assistant_msg = conversations::insert_message(
            &conv_pool,
            &conversation.id,
            "assistant",
            "助手回复",
            true,
        )
        .await
        .expect("insert assistant msg");
        let system_msg = conversations::insert_message(
            &conv_pool,
            &conversation.id,
            "system",
            "内部脚手架",
            true,
        )
        .await
        .expect("insert system msg");
        memories::insert_memories(
            &main_pool,
            None,
            &conversation.id,
            &[extracted(
                "fact",
                "来源包含非用户消息",
                vec![&msg_1.id, &assistant_msg.id, &system_msg.id],
            )],
        )
        .await
        .expect("insert memory");
        let memory = memories::list_memories(&main_pool, None, None, None, None)
            .await
            .expect("list memories")
            .remove(0);

        let source_messages = get_source_messages(&main_pool, &conv_pool, &memory.id)
            .await
            .expect("get source messages");

        assert_eq!(source_messages.len(), 1);
        assert_eq!(source_messages[0].id, msg_1.id);
        assert!(source_messages.iter().all(|message| message.role == "user"));
    }

    #[tokio::test]
    async fn get_source_messages_returns_available_subset_when_some_messages_missing() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversations_pool().await;
        let conversation = conversations::create_conversation(&conv_pool, None)
            .await
            .expect("create conversation");
        let msg_1 =
            conversations::insert_message(&conv_pool, &conversation.id, "user", "尚存来源", true)
                .await
                .expect("insert msg 1");
        // source 引用一条存在的消息与一条已不存在的消息。
        memories::insert_memories(
            &main_pool,
            None,
            &conversation.id,
            &[extracted("fact", "部分来源缺失", vec![&msg_1.id, "missing-msg"])],
        )
        .await
        .expect("insert memory");
        let memory = memories::list_memories(&main_pool, None, None, None, None)
            .await
            .expect("list memories")
            .remove(0);

        let source_messages = get_source_messages(&main_pool, &conv_pool, &memory.id)
            .await
            .expect("get source messages");

        // 部分缺失时返回可用子集，而非整体置空。
        assert_eq!(source_messages.len(), 1);
        assert_eq!(source_messages[0].id, msg_1.id);
    }

    #[tokio::test]
    async fn get_source_messages_missing_memory_returns_not_found() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversations_pool().await;

        let err = get_source_messages(&main_pool, &conv_pool, "missing")
            .await
            .expect_err("missing memory should fail");

        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn get_source_messages_invalid_source_json_returns_validation_error() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversations_pool().await;
        sqlx::query("INSERT INTO memories (id, category, content, source_conversation_id, source_message_ids) VALUES ('memory-1', 'fact', '坏来源', 'conv-1', 'not-json')")
            .execute(&main_pool)
            .await
            .expect("insert invalid memory");

        let err = get_source_messages(&main_pool, &conv_pool, "memory-1")
            .await
            .expect_err("invalid json should fail");

        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn get_source_messages_missing_conversation_or_message_returns_empty() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversations_pool().await;
        memories::insert_memories(
            &main_pool,
            None,
            "deleted-conv",
            &[extracted("fact", "来源已删除", vec!["msg-1"])],
        )
        .await
        .expect("insert memory");
        let memory = memories::list_memories(&main_pool, None, None, None, None)
            .await
            .expect("list memories")
            .remove(0);

        let source_messages = get_source_messages(&main_pool, &conv_pool, &memory.id)
            .await
            .expect("missing source should not fail");

        assert!(source_messages.is_empty());
    }
}
