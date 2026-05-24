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

/// Tracks which conversations are in onboarding mode.
/// Key: conversation_id, Value: current onboarding step
#[derive(Default)]
pub struct OnboardingConversations(pub Arc<Mutex<HashMap<String, u8>>>);

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
        let streaming = streaming_state.0.lock().await;
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

    {
        let mut streaming = streaming_state.0.lock().await;
        streaming.insert(conv_id.clone());
    }

    let cancel_token = CancellationToken::new();
    {
        let cancel_tokens = app_handle.state::<CancelTokens>();
        let mut tokens = cancel_tokens.0.lock().await;
        tokens.insert(conv_id.clone(), cancel_token.clone());
    }

    let conv_pool_clone = conv_pool.inner().clone();
    let main_pool_clone = main_pool.inner().clone();
    let streaming_state_clone = streaming_state.0.clone();
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
            conv_pool_clone,
            main_pool_clone,
            conv_id_clone.clone(),
            assistant_id,
            user_content,
            cancel_token,
            onboarding_step,
            role_id_for_stream,
            user_message_id_for_stream,
        )
        .await;

        if let Err(e) = result {
            tracing::error!("流式对话失败: {}", e);
        }

        let mut streaming = streaming_state_clone.lock().await;
        streaming.remove(&conv_id_clone);

        let cancel_tokens = app_handle_clone.state::<CancelTokens>();
        let mut tokens = cancel_tokens.0.lock().await;
        tokens.remove(&conv_id_clone);
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
    let tokens = cancel_tokens.0.lock().await;
    if let Some(token) = tokens.get(&conversation_id) {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn chat_delete_conversation(
    conversation_id: String,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<(), AppError> {
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

        let messages = conversations::list_messages(&conv_pool, &old_id).await?;
        let has_messages = messages.iter().any(|m| m.role == "user");
        let has_title = {
            let convs = conversations::list_all_conversations(&conv_pool).await?;
            convs.iter().any(|c| c.id == old_id && !c.title.is_empty())
        };

        if has_messages && !has_title {
            let conv_pool_clone = conv_pool.inner().clone();
            let main_pool_clone = main_pool.inner().clone();
            tokio::spawn(async move {
                if let Err(e) =
                    generate_title(app_handle, conv_pool_clone, main_pool_clone, old_id).await
                {
                    tracing::error!("生成对话标题失败: {}", e);
                }
            });
        }
    }

    conversations::create_conversation(&conv_pool, role_id.as_deref()).await
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
