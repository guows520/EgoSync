use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::models::chat::{Conversation, Message};

pub async fn create_conversation(
    pool: &ConversationsPool,
    role_id: Option<&str>,
) -> Result<Conversation, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono_now();

    sqlx::query(
        "INSERT INTO conversations (id, role_id, started_at, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(role_id)
    .bind(&now)
    .bind(&now)
    .execute(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("创建对话失败: {}", e)))?;

    Ok(Conversation {
        id,
        role_id: role_id.map(|s| s.to_string()),
        title: String::new(),
        started_at: now.clone(),
        updated_at: now,
    })
}

pub async fn get_or_create_butler_conversation(
    pool: &ConversationsPool,
) -> Result<Conversation, AppError> {
    let row = sqlx::query_as::<_, Conversation>(
        "SELECT id, role_id, title, started_at, updated_at FROM conversations WHERE role_id IS NULL ORDER BY updated_at DESC LIMIT 1",
    )
    .fetch_optional(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询管家对话失败: {}", e)))?;

    match row {
        Some(conv) => Ok(conv),
        None => create_conversation(pool, None).await,
    }
}

pub async fn insert_message(
    pool: &ConversationsPool,
    conversation_id: &str,
    role: &str,
    content: &str,
    is_complete: bool,
) -> Result<Message, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono_now();

    sqlx::query(
        "INSERT INTO messages (id, conversation_id, role, content, thinking_content, is_complete, created_at) VALUES (?, ?, ?, ?, '', ?, ?)",
    )
    .bind(&id)
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(is_complete as i32)
    .bind(&now)
    .execute(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("插入消息失败: {}", e)))?;

    sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(conversation_id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新对话时间戳失败: {}", e)))?;

    Ok(Message {
        id,
        conversation_id: conversation_id.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        thinking_content: String::new(),
        is_complete,
        created_at: now,
        routing_metadata: None,
    })
}

pub async fn update_message_content(
    pool: &ConversationsPool,
    id: &str,
    content: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
        .bind(content)
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新消息内容失败: {}", e)))?;
    Ok(())
}

pub async fn delete_message(pool: &ConversationsPool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM messages WHERE id = ?")
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除消息失败: {}", e)))?;
    Ok(())
}

pub async fn update_conversation_updated_at(
    pool: &ConversationsPool,
    id: &str,
) -> Result<(), AppError> {
    let now = chrono_now();
    sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新对话时间失败: {}", e)))?;
    Ok(())
}

pub async fn update_conversation_title(
    pool: &ConversationsPool,
    id: &str,
    title: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE conversations SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新对话标题失败: {}", e)))?;
    Ok(())
}

pub async fn delete_conversation(pool: &ConversationsPool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除对话消息失败: {}", e)))?;
    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除对话失败: {}", e)))?;
    Ok(())
}

pub async fn delete_conversations_by_role(
    pool: &ConversationsPool,
    role_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM messages WHERE conversation_id IN (SELECT id FROM conversations WHERE role_id = ?)",
    )
    .bind(role_id)
    .execute(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("删除角色对话消息失败: {}", e)))?;

    sqlx::query("DELETE FROM conversations WHERE role_id = ?")
        .bind(role_id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除角色对话失败: {}", e)))?;

    Ok(())
}

pub async fn get_or_create_conversation_by_role(
    pool: &ConversationsPool,
    role_id: &str,
) -> Result<Conversation, AppError> {
    let row = sqlx::query_as::<_, Conversation>(
        "SELECT id, role_id, title, started_at, updated_at FROM conversations WHERE role_id = ? ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(role_id)
    .fetch_optional(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色对话失败: {}", e)))?;

    match row {
        Some(conv) => Ok(conv),
        None => create_conversation(pool, Some(role_id)).await,
    }
}

pub async fn list_conversations_by_role(
    pool: &ConversationsPool,
    role_id: &str,
) -> Result<Vec<Conversation>, AppError> {
    let rows = sqlx::query_as::<_, Conversation>(
        "SELECT id, role_id, title, started_at, updated_at FROM conversations WHERE role_id = ? ORDER BY updated_at DESC",
    )
    .bind(role_id)
    .fetch_all(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询角色对话列表失败: {}", e)))?;
    Ok(rows)
}

pub async fn list_butler_conversations(
    pool: &ConversationsPool,
) -> Result<Vec<Conversation>, AppError> {
    let rows = sqlx::query_as::<_, Conversation>(
        "SELECT id, role_id, title, started_at, updated_at FROM conversations WHERE role_id IS NULL ORDER BY updated_at DESC",
    )
    .fetch_all(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询管家对话列表失败: {}", e)))?;
    Ok(rows)
}

pub async fn list_all_conversations(
    pool: &ConversationsPool,
) -> Result<Vec<Conversation>, AppError> {
    let rows = sqlx::query_as::<_, Conversation>(
        "SELECT id, role_id, title, started_at, updated_at FROM conversations ORDER BY updated_at DESC",
    )
    .fetch_all(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询对话列表失败: {}", e)))?;
    Ok(rows)
}

pub async fn update_message_thinking(
    pool: &ConversationsPool,
    id: &str,
    thinking_content: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE messages SET thinking_content = ? WHERE id = ?")
        .bind(thinking_content)
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新思考内容失败: {}", e)))?;
    Ok(())
}

pub async fn mark_message_complete(pool: &ConversationsPool, id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE messages SET is_complete = 1 WHERE id = ?")
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("标记消息完成失败: {}", e)))?;
    Ok(())
}

pub async fn list_messages(
    pool: &ConversationsPool,
    conversation_id: &str,
) -> Result<Vec<Message>, AppError> {
    let rows = sqlx::query_as::<_, Message>(
        "SELECT id, conversation_id, role, content, thinking_content, is_complete, created_at, routing_metadata FROM messages WHERE conversation_id = ? ORDER BY created_at ASC, rowid ASC",
    )
    .bind(conversation_id)
    .fetch_all(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询消息列表失败: {}", e)))?;
    Ok(rows)
}

pub async fn get_recent_messages(
    pool: &ConversationsPool,
    conversation_id: &str,
    limit: i64,
) -> Result<Vec<Message>, AppError> {
    let rows = sqlx::query_as::<_, Message>(
        "SELECT id, conversation_id, role, content, thinking_content, is_complete, created_at, routing_metadata FROM messages WHERE conversation_id = ? AND is_complete = 1 ORDER BY created_at DESC, rowid DESC LIMIT ?",
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(&**pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询最近消息失败: {}", e)))?;

    let mut rows = rows;
    rows.reverse();
    Ok(rows)
}

/// Story 2.3: 写入触发管家委派的 user message 的审计元数据。
/// `metadata_json` 必须是序列化好的 JSON 字符串（结构见 `Message::routing_metadata` 注释）。
pub async fn update_message_routing_metadata(
    pool: &ConversationsPool,
    id: &str,
    metadata_json: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE messages SET routing_metadata = ? WHERE id = ?")
        .bind(metadata_json)
        .bind(id)
        .execute(&**pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新路由元数据失败: {}", e)))?;
    Ok(())
}

fn chrono_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_pool() -> ConversationsPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        let schema = include_str!("../../migrations/002_conversations.sql");
        sqlx::raw_sql(schema)
            .execute(&pool)
            .await
            .expect("Failed to run migrations");

        // 补充后续迁移添加的列（与 pool.rs 中 run_conversations_migrations 保持一致）
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN thinking_content TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("Failed to add thinking_content column");

        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("Failed to add title column");

        // Story 2.3：与 run_conversations_migrations 保持一致 —— 测试库也要有此列，
        // 否则下面所有 SELECT 都会因列缺失失败。
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
            .execute(&pool)
            .await
            .expect("Failed to add routing_metadata column");

        ConversationsPool(pool)
    }

    #[tokio::test]
    async fn test_create_conversation() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, None).await.unwrap();
        assert!(!conv.id.is_empty());
        assert!(conv.role_id.is_none());
    }

    #[tokio::test]
    async fn test_create_conversation_with_role() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, Some("role-123")).await.unwrap();
        assert_eq!(conv.role_id, Some("role-123".to_string()));
    }

    #[tokio::test]
    async fn test_get_or_create_butler_conversation() {
        let pool = setup_test_pool().await;
        let conv1 = get_or_create_butler_conversation(&pool).await.unwrap();
        let conv2 = get_or_create_butler_conversation(&pool).await.unwrap();
        assert_eq!(conv1.id, conv2.id);
    }

    #[tokio::test]
    async fn test_insert_and_list_messages() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, None).await.unwrap();

        insert_message(&pool, &conv.id, "user", "你好", true)
            .await
            .unwrap();
        insert_message(
            &pool,
            &conv.id,
            "assistant",
            "你好！有什么可以帮你的？",
            true,
        )
        .await
        .unwrap();

        let messages = list_messages(&pool, &conv.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_update_message_content_and_mark_complete() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, None).await.unwrap();

        let msg = insert_message(&pool, &conv.id, "assistant", "", false)
            .await
            .unwrap();
        assert!(!msg.is_complete);

        update_message_content(&pool, &msg.id, "你好！")
            .await
            .unwrap();
        mark_message_complete(&pool, &msg.id).await.unwrap();

        let messages = list_messages(&pool, &conv.id).await.unwrap();
        assert_eq!(messages[0].content, "你好！");
        assert!(messages[0].is_complete);
    }

    #[tokio::test]
    async fn test_get_recent_messages() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, None).await.unwrap();

        for i in 0..5 {
            insert_message(&pool, &conv.id, "user", &format!("msg {}", i), true)
                .await
                .unwrap();
        }

        let recent = get_recent_messages(&pool, &conv.id, 3).await.unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "msg 2");
        assert_eq!(recent[2].content, "msg 4");
    }

    #[tokio::test]
    async fn test_delete_conversations_by_role_removes_messages_and_keeps_other_roles() {
        let pool = setup_test_pool().await;
        let target_conv = create_conversation(&pool, Some("role-target"))
            .await
            .unwrap();
        let other_conv = create_conversation(&pool, Some("role-other"))
            .await
            .unwrap();

        insert_message(&pool, &target_conv.id, "user", "target message", true)
            .await
            .unwrap();
        insert_message(&pool, &other_conv.id, "user", "other message", true)
            .await
            .unwrap();

        delete_conversations_by_role(&pool, "role-target")
            .await
            .unwrap();

        let conversations = list_all_conversations(&pool).await.unwrap();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].id, other_conv.id);

        let target_messages = list_messages(&pool, &target_conv.id).await.unwrap();
        assert!(target_messages.is_empty());

        let other_messages = list_messages(&pool, &other_conv.id).await.unwrap();
        assert_eq!(other_messages.len(), 1);
        assert_eq!(other_messages[0].content, "other message");
    }

    /// AC-2 / AC-7: 角色空间内新建对话时，持久化归属必须继续是该角色；
    /// 否则后续历史过滤会把这条会话漏到管家或别的角色列表里。
    #[tokio::test]
    async fn test_create_conversation_preserves_role_id() {
        let pool = setup_test_pool().await;

        let conv = create_conversation(&pool, Some("role-pm")).await.unwrap();

        assert_eq!(conv.role_id.as_deref(), Some("role-pm"));
        let role_list = list_conversations_by_role(&pool, "role-pm").await.unwrap();
        assert_eq!(role_list.len(), 1);
        assert_eq!(role_list[0].id, conv.id);
    }

    /// AC-2: 角色独立对话历史 — 第一次进入角色视图必须创建归属该角色的会话；
    /// 第二次进入必须**复用**同一会话，否则用户每次切角色都会看到空白历史，
    /// 失去"角色记得我"的核心体验。
    #[tokio::test]
    async fn test_get_or_create_conversation_by_role_reuses_existing() {
        let pool = setup_test_pool().await;

        let first = get_or_create_conversation_by_role(&pool, "role-pm")
            .await
            .unwrap();
        let second = get_or_create_conversation_by_role(&pool, "role-pm")
            .await
            .unwrap();

        assert_eq!(first.id, second.id, "同一 role_id 必须复用最新会话");
        assert_eq!(first.role_id.as_deref(), Some("role-pm"));
    }

    /// AC-7: 历史对话列表按角色过滤 —
    /// 角色 A 的对话不能漏到角色 B 的列表里，否则会暴露其他角色的私密上下文，
    /// 破坏"每个角色一个独立人格"的信任承诺。
    #[tokio::test]
    async fn test_list_conversations_by_role_only_returns_target_role() {
        let pool = setup_test_pool().await;
        let pm_conv = create_conversation(&pool, Some("role-pm")).await.unwrap();
        let _learner_conv = create_conversation(&pool, Some("role-learner"))
            .await
            .unwrap();
        let _butler_conv = create_conversation(&pool, None).await.unwrap();

        let pm_list = list_conversations_by_role(&pool, "role-pm").await.unwrap();

        assert_eq!(pm_list.len(), 1);
        assert_eq!(pm_list[0].id, pm_conv.id);
    }

    /// AC-7: 管家视角 ChatHeader 历史下拉只能看到管家自己的对话；
    /// 若混入角色对话会让用户误以为管家"窥视"了角色私聊。
    #[tokio::test]
    async fn test_list_butler_conversations_excludes_role_conversations() {
        let pool = setup_test_pool().await;
        let butler_conv = create_conversation(&pool, None).await.unwrap();
        let _role_conv = create_conversation(&pool, Some("role-pm")).await.unwrap();

        let butler_list = list_butler_conversations(&pool).await.unwrap();

        assert_eq!(butler_list.len(), 1);
        assert_eq!(butler_list[0].id, butler_conv.id);
        assert!(butler_list[0].role_id.is_none());
    }

    /// Story 2.3 AC-6: 路由审计字段默认为 NULL。
    /// 普通的用户消息不应该被错误地打上 routing_metadata 标签，
    /// 否则历史回溯会把无关消息误判成"曾发起过委派"。
    #[tokio::test]
    async fn test_insert_message_routing_metadata_defaults_to_none() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, None).await.unwrap();

        let msg = insert_message(&pool, &conv.id, "user", "你好", true)
            .await
            .unwrap();
        assert!(msg.routing_metadata.is_none());

        let listed = list_messages(&pool, &conv.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].routing_metadata.is_none());
    }

    /// Story 2.3 AC-6: 委派执行后，触发本轮的 user message 行必须能持久化审计 JSON。
    /// 这是事后追溯"管家把哪个任务派给了谁"的唯一证据 —— 字段丢失就等于审计断链。
    #[tokio::test]
    async fn test_update_message_routing_metadata_persists_json() {
        let pool = setup_test_pool().await;
        let conv = create_conversation(&pool, None).await.unwrap();
        let user_msg = insert_message(&pool, &conv.id, "user", "帮我跟进 OKR", true)
            .await
            .unwrap();

        let metadata = r#"{"delegations":[{"targetRoleId":"R1","targetRoleName":"产品经理","taskSummary":"跟进 OKR","status":"ok"}]}"#;
        update_message_routing_metadata(&pool, &user_msg.id, metadata)
            .await
            .unwrap();

        let listed = list_messages(&pool, &conv.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        // 原样存回 —— DB 层不解析 JSON，保留给上层
        assert_eq!(listed[0].routing_metadata.as_deref(), Some(metadata));
    }
}
