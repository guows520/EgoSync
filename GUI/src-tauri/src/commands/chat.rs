use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tauri::{Manager, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::db::conversations;
use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::chat::{ChatRequest, Conversation, Message, TitleUpdatedPayload};
use crate::services::agent_engine;

#[derive(Default)]
pub struct StreamingState(pub Arc<Mutex<HashSet<String>>>);

#[derive(Default)]
pub struct CancelTokens(pub Arc<Mutex<HashMap<String, CancellationToken>>>);

#[derive(Default)]
pub struct OpencodeSessions(pub Arc<Mutex<HashMap<String, String>>>);

/// Tracks which conversations are in onboarding mode.
/// Key: conversation_id, Value: current onboarding step
#[derive(Default)]
pub struct OnboardingConversations(pub Arc<Mutex<HashMap<String, u8>>>);

#[derive(Default, Clone)]
pub struct MemoryExtractionState(pub Arc<Mutex<HashMap<String, CancellationToken>>>);

const MEMORY_EXTRACTION_IDLE_SECONDS: u64 = 300;

async fn replace_memory_extraction_token(
    memory_state: &MemoryExtractionState,
    conversation_id: &str,
) -> CancellationToken {
    let token = CancellationToken::new();
    let mut tokens = memory_state.0.lock().await;
    if let Some(old_token) = tokens.insert(conversation_id.to_string(), token.clone()) {
        old_token.cancel();
    }
    token
}

async fn cancel_memory_extraction_token(
    memory_state: &MemoryExtractionState,
    conversation_id: &str,
) -> Option<CancellationToken> {
    let mut tokens = memory_state.0.lock().await;
    let token = tokens.remove(conversation_id);
    if let Some(token) = &token {
        token.cancel();
    }
    token
}

async fn remove_memory_extraction_token_if_active(
    memory_state: &MemoryExtractionState,
    conversation_id: &str,
    token: &CancellationToken,
) {
    let mut tokens = memory_state.0.lock().await;
    if token.is_cancelled() {
        return;
    }
    tokens.remove(conversation_id);
}

fn has_enough_complete_user_messages(messages: &[Message]) -> bool {
    messages
        .iter()
        .filter(|m| m.role == "user" && m.is_complete && !m.content.trim().is_empty())
        .count()
        >= 3
}

fn has_delegation_metadata(messages: &[Message]) -> bool {
    messages
        .iter()
        .filter_map(|message| message.routing_metadata.as_deref())
        .filter_map(|metadata| serde_json::from_str::<serde_json::Value>(metadata).ok())
        .filter_map(|parsed| {
            parsed
                .get("delegations")
                .and_then(|d| d.as_array())
                .cloned()
        })
        .any(|delegations| !delegations.is_empty())
}

async fn should_schedule_memory_extraction(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
) -> Result<bool, AppError> {
    let messages = conversations::list_messages(conv_pool, conversation_id).await?;
    Ok(has_enough_complete_user_messages(&messages) || has_delegation_metadata(&messages))
}

fn spawn_memory_extraction_after_idle(
    memory_state: MemoryExtractionState,
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    conversation_id: String,
    token: CancellationToken,
) {
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(MEMORY_EXTRACTION_IDLE_SECONDS)) => {
                if !token.is_cancelled() {
                    let _ = crate::services::memory_pipeline::extract_for_conversation(
                        main_pool,
                        conv_pool,
                        conversation_id.clone(),
                    )
                    .await;
                }
            }
            _ = token.cancelled() => {}
        }
        remove_memory_extraction_token_if_active(&memory_state, &conversation_id, &token).await;
    });
}

async fn schedule_memory_extraction(
    memory_state: MemoryExtractionState,
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    conversation_id: String,
) {
    match should_schedule_memory_extraction(&conv_pool, &conversation_id).await {
        Ok(true) => {
            let token = replace_memory_extraction_token(&memory_state, &conversation_id).await;
            spawn_memory_extraction_after_idle(
                memory_state,
                main_pool,
                conv_pool,
                conversation_id,
                token,
            );
        }
        Ok(false) => {
            cancel_memory_extraction_token(&memory_state, &conversation_id).await;
        }
        Err(err) => {
            tracing::warn!(conversation_id, error = %err, "memory extraction schedule skipped");
        }
    }
}

async fn trigger_memory_extraction_now(
    memory_state: MemoryExtractionState,
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    conversation_id: String,
) {
    cancel_memory_extraction_token(&memory_state, &conversation_id).await;
    match should_schedule_memory_extraction(&conv_pool, &conversation_id).await {
        Ok(true) => {
            tokio::spawn(async move {
                let _ = crate::services::memory_pipeline::extract_for_conversation(
                    main_pool,
                    conv_pool,
                    conversation_id,
                )
                .await;
            });
        }
        Ok(false) => {}
        Err(err) => {
            tracing::warn!(conversation_id, error = %err, "immediate memory extraction skipped");
        }
    }
}

async fn cancel_streaming_token(cancel_tokens: &CancelTokens, conversation_id: &str) -> bool {
    let tokens = cancel_tokens.0.lock().await;
    if let Some(token) = tokens.get(conversation_id) {
        token.cancel();
        true
    } else {
        false
    }
}

async fn opencode_session_id_for_stop(
    opencode_sessions: &OpencodeSessions,
    conversation_id: &str,
) -> Option<String> {
    let sessions = opencode_sessions.0.lock().await;
    sessions.get(conversation_id).cloned()
}

#[tauri::command]
pub async fn chat_send_message(
    request: ChatRequest,
    main_pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
    streaming_state: State<'_, StreamingState>,
    onboarding_convs: State<'_, OnboardingConversations>,
    app_handle: tauri::AppHandle,
) -> Result<Message, AppError> {
    // --- Resolve effective onboarding_step ---
    // If the frontend says this is onboarding (step > 0), trust it and remember.
    // If the frontend sends 0 but we already know this conversation is onboarding, use stored step + 1.
    let effective_onboarding_step: u8;
    let conv_id_for_lookup = request.conversation_id.clone().unwrap_or_default();

    {
        let mut onb_map = onboarding_convs.0.lock().await;
        if request.onboarding_step > 0 {
            // Frontend explicitly says onboarding
            effective_onboarding_step = request.onboarding_step;
            onb_map.insert(conv_id_for_lookup.clone(), request.onboarding_step);
        } else if let Some(stored_step) = onb_map.get(&conv_id_for_lookup) {
            // Frontend didn't send step but we know this is an onboarding conversation
            effective_onboarding_step = (*stored_step + 1).min(5);
            onb_map.insert(conv_id_for_lookup.clone(), effective_onboarding_step);
        } else {
            effective_onboarding_step = 0;
        }
        // Remove entry once onboarding completes to prevent unbounded map growth
        if effective_onboarding_step >= 5 {
            onb_map.remove(&conv_id_for_lookup);
        }
    }

    // --- 诊断日志: 确认 onboarding_step 是否正确传入 ---
    tracing::info!(
        "[chat_send_message] 收到请求: conv_id={:?} onboarding_step={} effective={} content_len={}",
        request.conversation_id,
        request.onboarding_step,
        effective_onboarding_step,
        request.content.len()
    );

    let conversation = match &request.conversation_id {
        Some(id) => Conversation {
            id: id.clone(),
            role_id: request.role_id.clone(),
            title: String::new(),
            started_at: String::new(),
            updated_at: String::new(),
        },
        None => conversations::get_or_create_butler_conversation(&conv_pool).await?,
    };

    let conv_id = conversation.id.clone();

    {
        let memory_state = app_handle.state::<MemoryExtractionState>();
        cancel_memory_extraction_token(&memory_state, &conv_id).await;
    }

    {
        let mut streaming = streaming_state.0.lock().await;
        if streaming.contains(&conv_id) {
            let busy_msg = conversations::insert_message(
                &conv_pool,
                &conv_id,
                "assistant",
                "我还在想上一个问题，请稍等片刻...",
                true,
            )
            .await?;
            return Ok(busy_msg);
        }
        streaming.insert(conv_id.clone());
    }

    let is_onboarding_start = request.content == "__onboarding_start__";
    let display_content = if is_onboarding_start {
        ""
    } else {
        &request.content
    };

    let user_msg = if is_onboarding_start {
        // For onboarding start, create a placeholder user message but don't show it
        conversations::insert_message(&conv_pool, &conv_id, "system", "[onboarding_start]", true)
            .await?
    } else {
        conversations::insert_message(&conv_pool, &conv_id, "user", display_content, true).await?
    };

    let assistant_msg =
        conversations::insert_message(&conv_pool, &conv_id, "assistant", "", false).await?;

    let cancel_token = CancellationToken::new();
    {
        let cancel_tokens = app_handle.state::<CancelTokens>();
        let mut tokens = cancel_tokens.0.lock().await;
        tokens.insert(conv_id.clone(), cancel_token.clone());
    }

    let conv_pool_for_stream = conv_pool.inner().clone();
    let conv_pool_for_memory = conv_pool.inner().clone();
    let main_pool_for_stream = main_pool.inner().clone();
    let main_pool_for_memory = main_pool.inner().clone();
    let opencode_sessions_clone = app_handle.state::<OpencodeSessions>().inner().0.clone();
    let agent_bridge_clone = app_handle
        .state::<crate::services::agent_bridge::AgentBridge>()
        .inner()
        .clone();
    let event_router_clone = app_handle
        .state::<std::sync::Arc<crate::services::event_router::EventRouter>>()
        .inner()
        .clone();
    let delegate_bridge_clone = app_handle
        .state::<crate::services::delegate_bridge::DelegateBridge>()
        .inner()
        .clone();
    let streaming_state_clone = streaming_state.0.clone();
    let memory_state_clone = app_handle.state::<MemoryExtractionState>().inner().clone();
    let conv_id_clone = conv_id.clone();
    let assistant_id = assistant_msg.id.clone();
    let user_content = if is_onboarding_start {
        "__onboarding_start__".to_string()
    } else {
        request.content.clone()
    };
    let app_handle_clone = app_handle.clone();

    let onboarding_step = if effective_onboarding_step > 0 {
        Some(effective_onboarding_step)
    } else {
        None
    };

    let role_id_for_stream = request.role_id.clone();
    let user_message_id_for_stream = user_msg.id.clone();

    tokio::spawn(async move {
        let result = agent_engine::run_stream(
            app_handle_clone.clone(),
            conv_pool_for_stream,
            main_pool_for_stream,
            conv_id_clone.clone(),
            assistant_id,
            user_content,
            cancel_token,
            onboarding_step,
            role_id_for_stream,
            user_message_id_for_stream,
            opencode_sessions_clone,
            agent_bridge_clone,
            event_router_clone,
            delegate_bridge_clone,
        )
        .await;

        if let Err(e) = result {
            tracing::error!("流式对话失败: {}", e);
        }

        let mut streaming = streaming_state_clone.lock().await;
        streaming.remove(&conv_id_clone);
        drop(streaming);

        let cancel_tokens = app_handle_clone.state::<CancelTokens>();
        let mut tokens = cancel_tokens.0.lock().await;
        tokens.remove(&conv_id_clone);
        drop(tokens);

        schedule_memory_extraction(
            memory_state_clone,
            main_pool_for_memory,
            conv_pool_for_memory,
            conv_id_clone,
        )
        .await;
    });

    Ok(user_msg)
}

#[tauri::command]
pub async fn chat_get_history(
    conversation_id: String,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Vec<Message>, AppError> {
    conversations::list_messages(&conv_pool, &conversation_id).await
}

#[tauri::command]
pub async fn chat_get_butler_conversation(
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Conversation, AppError> {
    conversations::get_or_create_butler_conversation(&conv_pool).await
}

#[tauri::command]
pub async fn chat_get_role_conversation(
    role_id: String,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Conversation, AppError> {
    conversations::get_or_create_conversation_by_role(&conv_pool, &role_id).await
}

#[tauri::command]
pub async fn chat_list_conversations(
    role_id: Option<String>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Vec<Conversation>, AppError> {
    match role_id {
        Some(id) => conversations::list_conversations_by_role(&conv_pool, &id).await,
        None => conversations::list_butler_conversations(&conv_pool).await,
    }
}

#[tauri::command]
pub async fn chat_stop_streaming(
    conversation_id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError> {
    let cancel_tokens = app_handle.state::<CancelTokens>();
    cancel_streaming_token(&cancel_tokens, &conversation_id).await;

    let session_id = {
        let opencode_sessions = app_handle.state::<OpencodeSessions>();
        opencode_session_id_for_stop(&opencode_sessions, &conversation_id).await
    };
    if let Some(session_id) = session_id {
        let agent_bridge = app_handle.state::<crate::services::agent_bridge::AgentBridge>();
        if let Err(e) = agent_bridge.abort_session(&session_id).await {
            tracing::warn!(
                "opencode abort failed for conversation {} session {}: {}",
                conversation_id,
                session_id,
                e
            );
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn chat_delete_conversation(
    conversation_id: String,
    conv_pool: State<'_, ConversationsPool>,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError> {
    // Clean up streaming state so orphan tasks don't write to a deleted conversation
    let cancel_tokens = app_handle.state::<CancelTokens>();
    cancel_streaming_token(&cancel_tokens, &conversation_id).await;

    {
        let streaming_state = app_handle.state::<StreamingState>();
        let mut streaming = streaming_state.0.lock().await;
        streaming.remove(&conversation_id);
    }
    {
        let opencode_sessions = app_handle.state::<OpencodeSessions>();
        let mut sessions = opencode_sessions.0.lock().await;
        sessions.remove(&conversation_id);
    }
    {
        let onboarding = app_handle.state::<OnboardingConversations>();
        let mut map = onboarding.0.lock().await;
        map.remove(&conversation_id);
    }
    {
        let memory_state = app_handle.state::<MemoryExtractionState>();
        cancel_memory_extraction_token(&memory_state, &conversation_id).await;
    }

    conversations::delete_conversation(&conv_pool, &conversation_id).await
}

#[tauri::command]
pub async fn chat_new_conversation(
    old_conversation_id: Option<String>,
    role_id: Option<String>,
    conv_pool: State<'_, ConversationsPool>,
    main_pool: State<'_, DbPool>,
    app_handle: tauri::AppHandle,
) -> Result<Conversation, AppError> {
    if let Some(old_id) = old_conversation_id {
        conversations::update_conversation_updated_at(&conv_pool, &old_id).await?;

        let messages = match conversations::list_messages(&conv_pool, &old_id).await {
            Ok(messages) => messages,
            Err(err) => {
                tracing::warn!(conversation_id = old_id, error = %err, "conversation handoff skipped");
                return conversations::create_conversation(&conv_pool, role_id.as_deref()).await;
            }
        };
        let should_extract_old_conversation = has_enough_complete_user_messages(&messages);
        let has_messages = messages.iter().any(|m| m.role == "user");
        let has_title = {
            let convs = conversations::list_all_conversations(&conv_pool).await?;
            convs.iter().any(|c| c.id == old_id && !c.title.is_empty())
        };

        if has_messages && !has_title {
            let conv_pool_clone = conv_pool.inner().clone();
            let main_pool_clone = main_pool.inner().clone();
            let title_old_id = old_id.clone();
            let title_app_handle = app_handle.clone();
            tokio::spawn(async move {
                if let Err(e) = generate_title(
                    title_app_handle,
                    conv_pool_clone,
                    main_pool_clone,
                    title_old_id,
                )
                .await
                {
                    tracing::error!("生成对话标题失败: {}", e);
                }
            });
        }

        if should_extract_old_conversation {
            let memory_state = app_handle.state::<MemoryExtractionState>().inner().clone();
            trigger_memory_extraction_now(
                memory_state,
                main_pool.inner().clone(),
                conv_pool.inner().clone(),
                old_id,
            )
            .await;
        }
    }

    conversations::create_conversation(&conv_pool, role_id.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_conversation_pool() -> ConversationsPool {
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
            .expect("add thinking_content");
        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("add title");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
            .execute(&pool)
            .await
            .expect("add routing_metadata");
        ConversationsPool(pool)
    }

    #[tokio::test]
    async fn cancel_streaming_token_marks_existing_token_cancelled() {
        // WHY: stop must let the user regain control immediately, even if the
        // opencode abort request fails or hangs.
        let cancel_tokens = CancelTokens::default();
        let token = CancellationToken::new();
        {
            let mut tokens = cancel_tokens.0.lock().await;
            tokens.insert("conv-1".to_string(), token.clone());
        }

        let cancelled = cancel_streaming_token(&cancel_tokens, "conv-1").await;

        assert!(cancelled);
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn opencode_session_id_for_stop_returns_matching_conversation_session() {
        // WHY: aborting the wrong opencode session would cut off another
        // conversation; stop must target exactly the active conversation.
        let sessions = OpencodeSessions::default();
        {
            let mut map = sessions.0.lock().await;
            map.insert("conv-1".to_string(), "session-a".to_string());
            map.insert("conv-2".to_string(), "session-b".to_string());
        }

        let session_id = opencode_session_id_for_stop(&sessions, "conv-2").await;

        assert_eq!(session_id.as_deref(), Some("session-b"));
    }

    #[tokio::test]
    async fn replace_memory_extraction_token_cancels_previous_token() {
        let memory_state = MemoryExtractionState::default();
        let first = replace_memory_extraction_token(&memory_state, "conv-1").await;
        let second = replace_memory_extraction_token(&memory_state, "conv-1").await;

        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
        assert_eq!(memory_state.0.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn cancelled_memory_extraction_token_cannot_remove_new_token() {
        let memory_state = MemoryExtractionState::default();
        let first = replace_memory_extraction_token(&memory_state, "conv-1").await;
        let _second = replace_memory_extraction_token(&memory_state, "conv-1").await;

        remove_memory_extraction_token_if_active(&memory_state, "conv-1", &first).await;

        assert_eq!(memory_state.0.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn should_schedule_memory_extraction_requires_three_user_messages_and_allows_onboarding_followup(
    ) {
        let pool = setup_conversation_pool().await;
        let conv = conversations::create_conversation(&pool, None)
            .await
            .unwrap();
        let mut user_messages = Vec::new();
        for i in 0..2 {
            user_messages.push(
                conversations::insert_message(&pool, &conv.id, "user", &format!("msg {}", i), true)
                    .await
                    .unwrap(),
            );
        }
        assert!(!should_schedule_memory_extraction(&pool, &conv.id)
            .await
            .unwrap());

        conversations::update_message_routing_metadata(
            &pool,
            &user_messages[0].id,
            r#"{"delegations":[{"targetRoleId":"role-1","targetRoleName":"产品经理","targetConversationId":"role-conv-1","taskSummary":"准备设计评审","status":"ok"}]}"#,
        )
        .await
        .unwrap();
        assert!(should_schedule_memory_extraction(&pool, &conv.id)
            .await
            .unwrap());

        conversations::insert_message(&pool, &conv.id, "user", "third", true)
            .await
            .unwrap();
        assert!(should_schedule_memory_extraction(&pool, &conv.id)
            .await
            .unwrap());

        let onboarding = conversations::create_conversation(&pool, None)
            .await
            .unwrap();
        conversations::insert_message(&pool, &onboarding.id, "system", "[onboarding_start]", true)
            .await
            .unwrap();
        for i in 0..3 {
            conversations::insert_message(
                &pool,
                &onboarding.id,
                "user",
                &format!("msg {}", i),
                true,
            )
            .await
            .unwrap();
        }
        assert!(should_schedule_memory_extraction(&pool, &onboarding.id)
            .await
            .unwrap());
    }
}

async fn generate_title(
    app_handle: tauri::AppHandle,
    conv_pool: ConversationsPool,
    main_pool: DbPool,
    conversation_id: String,
) -> Result<(), AppError> {
    use crate::llm::traits::{ChatCompletionMessage, ChatOptions, StreamEvent};
    use tauri::Emitter;
    use tokio::sync::mpsc;

    let messages = conversations::list_messages(&conv_pool, &conversation_id).await?;
    let summary: String = messages
        .iter()
        .filter(|m| m.role != "system")
        .take(6)
        .map(|m| {
            let prefix = if m.role == "user" { "用户" } else { "助手" };
            let content = if m.content.len() > 100 {
                format!("{}...", &m.content[..m.content.floor_char_boundary(100)])
            } else {
                m.content.clone()
            };
            format!("{}: {}", prefix, content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt_messages = vec![
        ChatCompletionMessage {
            role: "system".to_string(),
            content: "你是一个标题生成器。用一句简短的中文标题（不超过15个字）总结对话内容。只输出标题，不要任何额外文字、标点或引号。".to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: format!("请为以下对话生成标题：\n\n{}", summary),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let provider = agent_engine::resolve_default_provider(&main_pool).await?;

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let provider_clone = provider.clone();
    tokio::spawn(async move {
        let _ = provider_clone
            .chat_stream(
                prompt_messages,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await;
    });

    let mut title = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::Token(t) => title.push_str(&t),
            StreamEvent::Done => break,
            StreamEvent::Error(e) => {
                tracing::warn!("LLM 生成标题出错: {}", e);
                break;
            }
            _ => {}
        }
    }

    let title = title
        .trim()
        .trim_matches(|c| c == '"' || c == '「' || c == '」' || c == '"' || c == '"')
        .to_string();

    if title.is_empty() {
        let fallback = messages
            .iter()
            .find(|m| m.role == "user")
            .map(|m| {
                let s = &m.content;
                if s.len() > 20 {
                    format!("{}...", &s[..s.floor_char_boundary(20)])
                } else {
                    s.clone()
                }
            })
            .unwrap_or_default();
        conversations::update_conversation_title(&conv_pool, &conversation_id, &fallback).await?;
        let _ = app_handle.emit(
            "conversation:title-updated",
            TitleUpdatedPayload {
                conversation_id,
                title: fallback,
            },
        );
        return Ok(());
    }

    conversations::update_conversation_title(&conv_pool, &conversation_id, &title).await?;

    let _ = app_handle.emit(
        "conversation:title-updated",
        TitleUpdatedPayload {
            conversation_id,
            title,
        },
    );

    Ok(())
}
