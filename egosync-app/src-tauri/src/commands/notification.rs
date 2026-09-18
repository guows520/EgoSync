//! Story 15.4：命令体已迁引擎（`egosync_engine::commands::notification`），
//! 壳侧薄化为 wrapper（内联测试随命令体迁引擎）。
use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;

use crate::error::AppError;
use crate::models::notification::{CreateNotificationInput, Notification, NotificationWithRole};

#[tauri::command]
pub async fn notification_create(
    ctx: State<'_, Arc<EngineCtx>>,
    input: CreateNotificationInput,
) -> Result<Notification, AppError> {
    egosync_engine::commands::notification::notification_create(&ctx, input).await
}

#[tauri::command]
pub async fn notification_list(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<Vec<NotificationWithRole>, AppError> {
    egosync_engine::commands::notification::notification_list(&ctx).await
}

#[tauri::command]
pub async fn notification_mark_read(
    ctx: State<'_, Arc<EngineCtx>>,
    id: String,
) -> Result<Notification, AppError> {
    egosync_engine::commands::notification::notification_mark_read(&ctx, id).await
}

#[tauri::command]
pub async fn notification_count_unread(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<i64, AppError> {
    egosync_engine::commands::notification::notification_count_unread(&ctx).await
}
