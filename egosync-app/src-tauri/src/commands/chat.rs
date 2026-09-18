use std::sync::Arc;

use tauri::{Manager, State};
use tokio_util::sync::CancellationToken;

use crate::db::conversations;
use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::chat::{ChatRequest, Conversation, Message, MessageProcessEvent, StreamPayload, STREAM_PHASE_DONE, TitleUpdatedPayload};
use crate::services::agent_config::AgentConfigService;
use crate::services::event_bus::EngineEvents;
use crate::services::tauri_event_bus::TauriEventBus;

// Story 15.3：六组会话状态类型已迁引擎 registry（逐字节平移，derive 保持），
// 此处回引使 crate::commands::chat::{...} 路径语义不变（消费者零改动）。
pub use egosync_engine::registry::{
    CancelTokens, ChatSessionRegistry, MemoryExtractionState, OnboardingConversations,
    OpencodeMcpScopeLock, OpencodeSessionState, OpencodeSessions, StreamingState,
};

const MEMORY_EXTRACTION_IDLE_SECONDS: u64 = 300;

/// Story 13.1：命令层补发写事件（快照引擎订阅触发 STATE_DELTA；payload
/// 沿用域对象/裸 id 供前端自由消费——引擎只看事件名不看 payload）。
/// 失败 warn 不阻断（评审 B7：与 role/task 的 emit warn 模式对齐）。
/// Story 15.3：发射改经注入的 EngineEvents 总线 + engine events 常量
/// （事件名值不变；强类型 payload 机械改写为 serde_json::to_value）。
fn emit_chat_event(bus: &dyn EngineEvents, event: &str, payload: &impl serde::Serialize) {
    if let Err(e) = serde_json::to_value(payload)
        .map_err(|e| e.to_string())
        .and_then(|payload| bus.emit(event, payload))
    {
        tracing::warn!(event = event, error = %e, "chat 写事件发射失败");
    }
}

fn sync_role_config_warn(result: Result<(), AppError>, action: &str) {
    if let Err(e) = result {
        tracing::warn!("opencode sync ({}) failed: {}", action, e);
    }
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
    registry: Arc<ChatSessionRegistry>,
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
        // Story 15.3：token 族辅助已迁 Registry 方法。'static 任务持共享
        // Arc——六组状态同一身份（评审补丁：不再按需伪造局部 Registry，
        // 方法将来触及其余字段时不会静默错实例）。
        registry
            .remove_memory_extraction_token_if_active(&conversation_id, &token)
            .await;
    });
}

async fn schedule_memory_extraction(
    registry: &Arc<ChatSessionRegistry>,
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    conversation_id: String,
) {
    match should_schedule_memory_extraction(&conv_pool, &conversation_id).await {
        Ok(true) => {
            let token = registry
                .replace_memory_extraction_token(&conversation_id)
                .await;
            spawn_memory_extraction_after_idle(
                registry.clone(),
                main_pool,
                conv_pool,
                conversation_id,
                token,
            );
        }
        Ok(false) => {
            registry
                .cancel_memory_extraction_token(&conversation_id)
                .await;
        }
        Err(err) => {
            tracing::warn!(conversation_id, error = %err, "memory extraction schedule skipped");
        }
    }
}

async fn trigger_memory_extraction_now(
    registry: &ChatSessionRegistry,
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    conversation_id: String,
) {
    registry
        .cancel_memory_extraction_token(&conversation_id)
        .await;
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
    sessions
        .get(conversation_id)
        .map(|state| state.active_session_id.clone())
        .filter(|id| !id.is_empty())
}

#[tauri::command]
pub async fn chat_send_message<R: tauri::Runtime>(
    request: ChatRequest,
    main_pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
    registry: State<'_, Arc<ChatSessionRegistry>>,
    agent_config: State<'_, AgentConfigService>,
    event_bus: State<'_, TauriEventBus<R>>,
    app_handle: tauri::AppHandle<R>,
) -> Result<Message, AppError> {
    // Explicit Skill validation is a command-level gate: no conversation, lock,
    // runtime, session, or subscription side effect may happen before it succeeds.
    let selected_skill = match request.selected_skill_id.as_deref() {
        Some(skill_key) => Some(
            crate::services::skill_registry::resolve_selectable(
                &main_pool,
                request.role_id.as_deref(),
                skill_key,
            )
            .await?,
        ),
        None => None,
    };

    // --- Resolve effective onboarding_step ---
    // If the frontend says this is onboarding (step > 0), trust it and remember.
    // If the frontend sends 0 but we already know this conversation is onboarding, use stored step + 1.
    let effective_onboarding_step: u8;
    let conv_id_for_lookup = request.conversation_id.clone().unwrap_or_default();

    {
        let mut onb_map = registry.onboarding_conversations.0.lock().await;
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
        "[stage-b-diag] chat request: conv_id={:?} role_id={:?} onboarding_step={} effective={} content_len={}",
        request.conversation_id,
        request.role_id,
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

    registry.cancel_memory_extraction_token(&conv_id).await;

    {
        let mut streaming = registry.streaming_state.0.lock().await;
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

    if let Some(role_id) = request.role_id.as_deref() {
        maybe_enable_requested_meta_skill(
            &main_pool,
            &conv_pool,
            &agent_config,
            &conv_id,
            role_id,
            display_content,
        )
        .await?;
    }

    let user_msg = if is_onboarding_start {
        // For onboarding start, create a placeholder user message but don't show it
        conversations::insert_message(&conv_pool, &conv_id, "system", "[onboarding_start]", true)
            .await?
    } else {
        conversations::insert_message(&conv_pool, &conv_id, "user", display_content, true).await?
    };

    // Story 13.1（评审决策①）：用户消息落库后补发 message:saved（快照引擎
    // 触发 STATE_DELTA）。assistant 消息完成由既有 llm:stream（done=true）
    // 覆盖，无需重复 emit。
    emit_chat_event(
        event_bus.inner(),
        crate::events::MESSAGE_SAVED_EVENT,
        &user_msg,
    );

    let assistant_msg =
        conversations::insert_message(&conv_pool, &conv_id, "assistant", "", false).await?;

    let cancel_token = CancellationToken::new();
    {
        let mut tokens = registry.cancel_tokens.0.lock().await;
        tokens.insert(conv_id.clone(), cancel_token.clone());
    }

    let conv_pool_for_stream = conv_pool.inner().clone();
    let conv_pool_for_memory = conv_pool.inner().clone();
    let main_pool_for_stream = main_pool.inner().clone();
    let main_pool_for_memory = main_pool.inner().clone();
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
    let streaming_state_clone = registry.streaming_state.0.clone();
    let registry_clone = registry.inner().clone();
    let event_bus_clone = event_bus.inner().clone();
    // Story 15.3：run_stream 宿主能力接缝注入（原 AppHandle 内部 try_state 取用）
    let agent_config_clone = app_handle
        .state::<AgentConfigService>()
        .inner()
        .clone();
    let sidecar_clone = app_handle
        .state::<Arc<tokio::sync::Mutex<crate::services::sidecar::SidecarManager>>>()
        .inner()
        .clone();
    let secret_for_stream: Arc<dyn egosync_engine::services::secret_store::SecretStore> = Arc::new(
        crate::services::secret_store_keyring::KeyringSecretStore::new(),
    );
    // Story 15.3：默认 project_dir 由壳解析（原 engine 内 app_data_dir 逻辑迁壳侧）
    let project_dir_for_stream = app_handle
        .path()
        .app_data_dir()
        .map(|dir| {
            let workspace = dir.join("opencode-workspace");
            let _ = std::fs::create_dir_all(&workspace);
            workspace.to_string_lossy().to_string()
        })
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".egosync-workspace")
                .to_string_lossy()
                .to_string()
        });
    let conv_id_clone = conv_id.clone();
    let assistant_id = assistant_msg.id.clone();
    let user_content = if is_onboarding_start {
        "__onboarding_start__".to_string()
    } else {
        request.content.clone()
    };
    let onboarding_step = if effective_onboarding_step > 0 {
        Some(effective_onboarding_step)
    } else {
        None
    };

    let role_id_for_stream = request.role_id.clone();
    let user_message_id_for_stream = user_msg.id.clone();
    let working_directory_for_stream = request.working_directory.clone();
    let selected_skill_for_stream = selected_skill;

    tokio::spawn(async move {
        // Story 15.3：AppHandle → 接缝注入（总线/Registry/配置/sidecar/密钥/宿主路径）
        let result = egosync_engine::services::agent_engine::run_stream(
            Arc::new(event_bus_clone.clone()),
            conv_pool_for_stream,
            main_pool_for_stream,
            conv_id_clone.clone(),
            assistant_id,
            user_content,
            cancel_token,
            onboarding_step,
            role_id_for_stream,
            user_message_id_for_stream,
            registry_clone.clone(),
            agent_bridge_clone,
            event_router_clone,
            delegate_bridge_clone,
            agent_config_clone.clone(),
            sidecar_clone.clone(),
            secret_for_stream.clone(),
            project_dir_for_stream.clone(),
            working_directory_for_stream,
            selected_skill_for_stream,
        )
        .await;

        if let Err(e) = result {
            tracing::error!("流式对话失败: {}", e);
            // 生产 bug 修复：run_stream 在 resolve_default_provider 或其他步骤 Err 时
            // 不会调用 emit_stream_done，前端 isInputLocked 永远保持 true（UI 卡死）。
            // 这里兜底发射 done 事件，让前端复位流式状态。
            let _ = serde_json::to_value(StreamPayload {
                    conversation_id: conv_id_clone.clone(),
                    token: String::new(),
                    done: true,
                    thinking: false,
                    message_id: None,
                    phase: Some(STREAM_PHASE_DONE.to_string()),
                    status_text: None,
                    tool_name: None,
                    process_event: None,
                })
                .map_err(|e| e.to_string())
                .and_then(|payload| {
                    event_bus_clone.emit(crate::events::LLM_STREAM_EVENT, payload)
                });
        }

        let mut streaming = streaming_state_clone.lock().await;
        streaming.remove(&conv_id_clone);
        drop(streaming);

        let mut tokens = registry_clone.cancel_tokens.0.lock().await;
        tokens.remove(&conv_id_clone);
        drop(tokens);

        schedule_memory_extraction(
            &registry_clone,
            main_pool_for_memory,
            conv_pool_for_memory,
            conv_id_clone,
        )
        .await;
    });

    Ok(user_msg)
}

async fn maybe_enable_requested_meta_skill(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    agent_config: &AgentConfigService,
    conversation_id: &str,
    role_id: &str,
    user_content: &str,
) -> Result<(), AppError> {
    if !crate::services::role_config::is_user_confirmation(user_content) {
        return Ok(());
    }

    let messages = conversations::get_recent_messages(conv_pool, conversation_id, 1).await?;
    let Some(last_assistant) = messages
        .iter()
        .rev()
        .find(|message| message.role == "assistant" && message.is_complete)
    else {
        return Ok(());
    };
    let Some(skill_key) =
        crate::services::role_config::requested_meta_skill_from_assistant(&last_assistant.content)
    else {
        return Ok(());
    };

    let role = crate::db::roles::get_role(main_pool, role_id).await?;
    if crate::services::role_config::skill_enabled(&role.skills_config, skill_key) {
        return Ok(());
    }
    let Some(input) = crate::services::role_config::enable_skill(&role.skills_config, skill_key)
    else {
        return Ok(());
    };
    let updated = crate::db::roles::update_role_skills(main_pool, role_id, &input).await?;
    let registry = crate::db::skills::list_skills(main_pool).await.unwrap_or_default();
    sync_role_config_warn(
        crate::services::mcp_server::sync_role_agent_with_mcp(main_pool, agent_config, &updated, &registry).await,
        "auto_enable_skill",
    );
    Ok(())
}

#[tauri::command]
pub async fn chat_get_history(
    conversation_id: String,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Vec<Message>, AppError> {
    conversations::list_messages(&conv_pool, &conversation_id).await
}

#[tauri::command]
pub async fn chat_get_message_process_events(
    message_id: String,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Vec<MessageProcessEvent>, AppError> {
    conversations::list_message_process_events(&conv_pool, &message_id).await
}

#[tauri::command]
pub async fn chat_pick_working_directory() -> Result<Option<String>, AppError> {
    let picked = rfd::AsyncFileDialog::new().pick_folder().await;
    Ok(picked.map(|folder| folder.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn chat_get_conversation(
    conversation_id: String,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Option<Conversation>, AppError> {
    conversations::get_conversation(&conv_pool, &conversation_id).await
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
    let registry = app_handle.state::<Arc<ChatSessionRegistry>>();
    cancel_streaming_token(&registry.cancel_tokens, &conversation_id).await;

    let session_id =
        opencode_session_id_for_stop(&registry.opencode_sessions, &conversation_id).await;
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
pub async fn chat_delete_conversation<R: tauri::Runtime>(
    conversation_id: String,
    conv_pool: State<'_, ConversationsPool>,
    event_bus: State<'_, TauriEventBus<R>>,
    app_handle: tauri::AppHandle<R>,
) -> Result<(), AppError> {
    let registry = app_handle.state::<Arc<ChatSessionRegistry>>();
    // Clean up streaming state so orphan tasks don't write to a deleted conversation
    cancel_streaming_token(&registry.cancel_tokens, &conversation_id).await;

    {
        let mut streaming = registry.streaming_state.0.lock().await;
        streaming.remove(&conversation_id);
    }
    {
        let mut sessions = registry.opencode_sessions.0.lock().await;
        sessions.remove(&conversation_id);
    }
    {
        let mut map = registry.onboarding_conversations.0.lock().await;
        map.remove(&conversation_id);
    }
    registry
        .cancel_memory_extraction_token(&conversation_id)
        .await;

    conversations::delete_conversation(&conv_pool, &conversation_id).await?;
    // Story 13.1（评审决策①）：会话删除须触发 STATE_DELTA，否则手机会话域
    // 保留已删会话。payload 裸 id（对象已删，无域对象可发）。
    emit_chat_event(
        event_bus.inner(),
        crate::events::CONVERSATION_DELETED_EVENT,
        &conversation_id,
    );
    Ok(())
}

#[tauri::command]
pub async fn chat_new_conversation<R: tauri::Runtime>(
    old_conversation_id: Option<String>,
    role_id: Option<String>,
    conv_pool: State<'_, ConversationsPool>,
    main_pool: State<'_, DbPool>,
    event_bus: State<'_, TauriEventBus<R>>,
    app_handle: tauri::AppHandle<R>,
) -> Result<Conversation, AppError> {
    if let Some(old_id) = old_conversation_id {
        let messages = match conversations::list_messages(&conv_pool, &old_id).await {
            Ok(messages) => messages,
            Err(err) => {
                tracing::warn!(conversation_id = old_id, error = %err, "conversation handoff skipped");
                // Story 13.1（评审决策①）：创建路径同样补发 conversation:created
                let conv = conversations::create_conversation(&conv_pool, role_id.as_deref()).await?;
                emit_chat_event(
                    event_bus.inner(),
                    crate::events::CONVERSATION_CREATED_EVENT,
                    &conv,
                );
                return Ok(conv);
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
            let title_bus = event_bus.inner().clone();
            tokio::spawn(async move {
                if let Err(e) = generate_title(
                    title_bus,
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
            let registry = app_handle.state::<Arc<ChatSessionRegistry>>();
            trigger_memory_extraction_now(
                registry.inner(),
                main_pool.inner().clone(),
                conv_pool.inner().clone(),
                old_id,
            )
            .await;
        }
    }

    let conv = conversations::create_conversation(&conv_pool, role_id.as_deref()).await?;
    emit_chat_event(
        event_bus.inner(),
        crate::events::CONVERSATION_CREATED_EVENT,
        &conv,
    );
    Ok(conv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use std::collections::HashMap;

    async fn setup_conversation_pool() -> ConversationsPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create conversations db");
        sqlx::raw_sql(include_str!("../../../../crates/egosync-engine/migrations/002_conversations.sql"))
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
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
        )
        .execute(&pool)
        .await
        .expect("create message_process_events");
        ConversationsPool(pool)
    }

    async fn setup_main_pool() -> DbPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create main db");
        sqlx::raw_sql(include_str!("../../../../crates/egosync-engine/migrations/003_roles.sql"))
            .execute(&pool)
            .await
            .expect("create roles schema");
        sqlx::raw_sql(include_str!("../../../../crates/egosync-engine/migrations/019_energy_updated_at.sql"))
            .execute(&pool)
            .await
            .expect("add energy_updated_at column");
        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                server_type TEXT NOT NULL,
                command_or_url TEXT NOT NULL,
                env_refs TEXT NOT NULL DEFAULT '{}',
                description TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            );
            CREATE TABLE IF NOT EXISTS role_mcp_server_bindings (
                server_id TEXT NOT NULL,
                role_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                PRIMARY KEY (server_id, role_id)
            );",
        )
        .execute(&pool)
        .await
        .expect("create mcp schema");
        pool
    }

    #[tokio::test]
    async fn chat_get_conversation_returns_none_for_deleted_source_conversation() {
        let pool = setup_conversation_pool().await;
        let conv = conversations::create_conversation(&pool, None)
            .await
            .expect("create conversation");
        let found = conversations::get_conversation(&pool, &conv.id)
            .await
            .expect("query existing conversation");
        assert!(found.is_some());

        conversations::delete_conversation(&pool, &conv.id)
            .await
            .expect("delete conversation");

        let missing = conversations::get_conversation(&pool, &conv.id)
            .await
            .expect("query deleted conversation");
        assert!(missing.is_none());
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
            map.insert(
                "conv-1".to_string(),
                OpencodeSessionState {
                    active_session_id: "session-a".to_string(),
                    sessions_by_directory: HashMap::from([(
                        "D:\\Work\\A".to_string(),
                        "session-a".to_string(),
                    )]),
                },
            );
            map.insert(
                "conv-2".to_string(),
                OpencodeSessionState {
                    active_session_id: "session-b".to_string(),
                    sessions_by_directory: HashMap::from([(
                        "D:\\Work\\B".to_string(),
                        "session-b".to_string(),
                    )]),
                },
            );
        }

        let session_id = opencode_session_id_for_stop(&sessions, "conv-2").await;

        assert_eq!(session_id.as_deref(), Some("session-b"));
    }

    // Story 15.3：token 族两测试随辅助函数迁入 engine registry.rs
    // （评审补丁：删除壳侧重复件——两份拷贝锁语义漂移风险）。

    #[tokio::test]
    async fn maybe_enable_requested_meta_skill_updates_role_before_next_turn() {
        let main_pool = setup_main_pool().await;
        let conv_pool = setup_conversation_pool().await;
        let dir = tempfile::tempdir().unwrap();
        let agent_config = AgentConfigService::new(dir.path().join("opencode.json"));
        let role = crate::db::roles::create_role(
            &main_pool,
            &crate::models::role::CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("管理产品规划".to_string()),
            },
        )
        .await
        .unwrap();
        let conv = conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();
        conversations::insert_message(
            &conv_pool,
            &conv.id,
            "assistant",
            "我需要 find-skills 能力才能帮你发现合适的 Skill，要开启吗？",
            true,
        )
        .await
        .unwrap();

        maybe_enable_requested_meta_skill(
            &main_pool,
            &conv_pool,
            &agent_config,
            &conv.id,
            &role.id,
            "好的",
        )
        .await
        .unwrap();

        let updated = crate::db::roles::get_role(&main_pool, &role.id)
            .await
            .unwrap();
        assert!(crate::services::role_config::skill_enabled(
            &updated.skills_config,
            crate::services::role_config::FIND_SKILLS_KEY,
        ));
        let config = agent_config.load().unwrap();
        let agent_key = AgentConfigService::role_to_agent_key(&role.id);
        let prompt = config["agent"][agent_key.as_str()]["prompt"]
            .as_str()
            .unwrap();
        assert!(prompt.contains("find-skills"));
        assert!(prompt.contains("已启用"));
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

async fn generate_title<R: tauri::Runtime>(
    bus: TauriEventBus<R>,
    conv_pool: ConversationsPool,
    main_pool: DbPool,
    conversation_id: String,
) -> Result<(), AppError> {
    use crate::llm::traits::{ChatCompletionMessage, ChatOptions, StreamEvent};
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
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: format!("请为以下对话生成标题：\n\n{}", summary),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // Story 15.3：agent_engine 迁引擎后其 resolve_default_provider 回引断链，
    // 改经 llm_config 模块直引（同一函数，值不变）。
    let provider = crate::services::llm_config::resolve_default_provider(
        &main_pool,
        &crate::services::secret_store_keyring::KeyringSecretStore::new(),
    )
    .await?;

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
        let _ = serde_json::to_value(TitleUpdatedPayload {
            conversation_id,
            title: fallback,
        })
        .map_err(|e| e.to_string())
        .and_then(|payload| {
            bus.emit(crate::events::CONVERSATION_TITLE_UPDATED_EVENT, payload)
        });
        return Ok(());
    }

    conversations::update_conversation_title(&conv_pool, &conversation_id, &title).await?;

    let _ = serde_json::to_value(TitleUpdatedPayload {
        conversation_id,
        title,
    })
    .map_err(|e| e.to_string())
    .and_then(|payload| {
        bus.emit(crate::events::CONVERSATION_TITLE_UPDATED_EVENT, payload)
    });

    Ok(())
}
