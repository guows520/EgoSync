//! chat 域命令（Story 15.4）：web-ok 命令体已迁引擎
//! （`egosync_engine::commands::chat`，`<R: Runtime>` 泛型随 ctx 化退役），
//! 壳侧薄化为 wrapper；chat_pick_working_directory 为 desktop-only 留壳
//! 不迁（rfd 对话框）；内联测试随命令体迁引擎。
//!
//! 六组会话状态类型与确认编排入口经回引保持
//! `crate::commands::chat::{...}` 路径语义不变（消费者零改动）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::chat::{ChatRequest, Conversation, Message, MessageProcessEvent};

// Story 15.3：六组会话状态类型已迁引擎 registry（逐字节平移，derive 保持），
// 此处回引使 crate::commands::chat::{...} 路径语义不变（消费者零改动）。
// Story 15.4：回引源改为引擎 commands::chat（其自身再回引 registry）。
pub use egosync_engine::commands::chat::{
    CancelTokens, ChatSessionRegistry, MemoryExtractionState, OnboardingConversations,
    OpencodeMcpScopeLock, OpencodeSessionState, OpencodeSessions, StreamingState,
};

#[tauri::command]
pub async fn chat_send_message(
    request: ChatRequest,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Message, AppError> {
    egosync_engine::commands::chat::chat_send_message(&ctx, request).await
}

#[tauri::command]
pub async fn chat_get_history(
    conversation_id: String,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Vec<Message>, AppError> {
    egosync_engine::commands::chat::chat_get_history(&ctx, conversation_id).await
}

#[tauri::command]
pub async fn chat_get_message_process_events(
    message_id: String,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Vec<MessageProcessEvent>, AppError> {
    egosync_engine::commands::chat::chat_get_message_process_events(&ctx, message_id).await
}

#[tauri::command]
pub async fn chat_pick_working_directory() -> Result<Option<String>, AppError> {
    let picked = rfd::AsyncFileDialog::new().pick_folder().await;
    Ok(picked.map(|folder| folder.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn chat_get_conversation(
    conversation_id: String,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Option<Conversation>, AppError> {
    egosync_engine::commands::chat::chat_get_conversation(&ctx, conversation_id).await
}

#[tauri::command]
pub async fn chat_get_butler_conversation(
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Conversation, AppError> {
    egosync_engine::commands::chat::chat_get_butler_conversation(&ctx).await
}

#[tauri::command]
pub async fn chat_get_role_conversation(
    role_id: String,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Conversation, AppError> {
    egosync_engine::commands::chat::chat_get_role_conversation(&ctx, role_id).await
}

#[tauri::command]
pub async fn chat_list_conversations(
    role_id: Option<String>,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Vec<Conversation>, AppError> {
    egosync_engine::commands::chat::chat_list_conversations(&ctx, role_id).await
}

#[tauri::command]
pub async fn chat_stop_streaming(
    conversation_id: String,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<(), AppError> {
    egosync_engine::commands::chat::chat_stop_streaming(&ctx, conversation_id).await
}

#[tauri::command]
pub async fn chat_delete_conversation(
    conversation_id: String,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<(), AppError> {
    egosync_engine::commands::chat::chat_delete_conversation(&ctx, conversation_id).await
}

#[tauri::command]
pub async fn chat_new_conversation(
    old_conversation_id: Option<String>,
    role_id: Option<String>,
    ctx: State<'_, std::sync::Arc<EngineCtx>>,
) -> Result<Conversation, AppError> {
    egosync_engine::commands::chat::chat_new_conversation(&ctx, old_conversation_id, role_id).await
}
