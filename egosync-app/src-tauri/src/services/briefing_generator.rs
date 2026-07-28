//! Story 6.1: 晨间简报生成服务 — 收集各角色数据，调用 LLM 生成自然语言简报
//!
//! 职责：
//! - 检查当天是否已生成简报（去重）
//! - 收集各角色状态、今日截止任务、昨日记忆、未读耳语通知
//! - 构造简报 prompt，调用默认 LLM provider 生成自然语言段落
//! - 写入 `briefings` 表 + 管家对话消息
//! - emit Tauri Event `briefing:generated` 供前端监听刷新

use std::sync::Arc;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::llm::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent};
use crate::models::dashboard::DashboardStatus;
use crate::models::memory::Memory;
use crate::models::notification::NotificationWithRole;
use crate::models::task::Task;
use crate::services::agent_engine;

const LLM_TIMEOUT_SECS: u64 = 30;
const MAX_BRIEFING_RESPONSE_BYTES: usize = 128 * 1024;
const BRIEFING_TIME_KEY: &str = "briefing_time";
const DEFAULT_BRIEFING_TIME: &str = "08:00";

/// Tauri Event 名称
pub const BRIEFING_GENERATED_EVENT: &str = "briefing:generated";

/// `briefing:generated` 事件 payload
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingGeneratedPayload {
    pub briefing_id: String,
    pub date: String,
}

/// 简报数据集合
struct BriefingData {
    role_statuses: Vec<DashboardStatus>,
    due_tasks: Vec<Task>,
    yesterday_memories: Vec<Memory>,
    unread_whispers: Vec<NotificationWithRole>,
}

/// 主入口函数。返回 `true` 表示生成了新简报，`false` 表示跳过（当天已有）。
///
/// LLM 超时/错误/解析失败 → `tracing::warn!` + 返回 `Ok(false)`（不阻塞调度器）
pub async fn generate_briefing_if_needed(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<bool, AppError> {
    let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();

    // 去重检查：当天已有简报则跳过
    if let Some(existing) = db::briefings::get_briefing_by_date(pool, &today).await? {
        tracing::info!(
            date = %today,
            briefing_id = %existing.id,
            "当天简报已存在，跳过生成"
        );
        return Ok(false);
    }

    // 收集简报数据
    let data = match collect_briefing_data(pool, conv_pool, &today).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "简报数据收集失败（降级跳过）");
            return Ok(false);
        }
    };

    // 构造 prompt
    let prompt = build_briefing_prompt(&data, &today);

    // 解析 LLM provider
    let provider = match agent_engine::resolve_default_provider(pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "解析 LLM provider 失败（降级跳过简报生成）");
            return Ok(false);
        }
    };

    // 调用 LLM
    let content = match call_llm(provider, prompt).await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!(error = %e, "简报 LLM 调用失败（降级跳过）");
            return Ok(false);
        }
    };

    let content = content.trim();
    if content.is_empty() {
        tracing::warn!("简报 LLM 返回空内容（降级跳过）");
        return Ok(false);
    }

    // 写入 briefings 表
    let briefing = db::briefings::create_briefing(pool, content, &today).await?;

    // 写入管家对话消息
    match db::conversations::get_or_create_butler_conversation(conv_pool).await {
        Ok(conv) => {
            if let Err(e) =
                db::conversations::insert_message(conv_pool, &conv.id, "assistant", content, true)
                    .await
            {
                tracing::warn!(
                    error = %e,
                    briefing_id = %briefing.id,
                    "简报写入管家对话消息失败"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                briefing_id = %briefing.id,
                "获取/创建管家对话失败"
            );
        }
    }

    // emit Tauri Event
    if let Some(handle) = app_handle {
        let payload = BriefingGeneratedPayload {
            briefing_id: briefing.id.clone(),
            date: briefing.date.clone(),
        };
        if let Err(e) = handle.emit(BRIEFING_GENERATED_EVENT, &payload) {
            tracing::warn!(error = %e, "emit briefing:generated 事件失败");
        }
    }

    tracing::info!(
        briefing_id = %briefing.id,
        date = %briefing.date,
        "晨间简报已生成"
    );

    Ok(true)
}

/// 从 app_settings 读取简报时间，无则返回默认值 `08:00`
pub async fn get_briefing_time(pool: &SqlitePool) -> Result<String, AppError> {
    match db::app_settings::get_setting(pool, BRIEFING_TIME_KEY).await? {
        Some(time) => Ok(time),
        None => Ok(DEFAULT_BRIEFING_TIME.to_string()),
    }
}

/// 收集简报数据
async fn collect_briefing_data(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    today: &str,
) -> Result<BriefingData, AppError> {
    // 各角色状态
    let role_statuses = crate::services::dashboard_service::get_dashboard_status(pool, conv_pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询角色状态失败（降级空列表）");
            Vec::new()
        });

    // 今日截止任务
    let due_tasks = db::tasks::list_tasks_due_today(pool, today)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询今日截止任务失败（降级空列表）");
            Vec::new()
        });

    // 昨日新增记忆：取最近 50 条，在 Rust 侧过滤昨天的记录。
    // memories.created_at 存储为 UTC（strftime(...,'now')），故 yesterday 也以 UTC 计算，保持一致。
    let yesterday = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let all_memories = db::memories::list_all_memories_with_options(pool, None, Some(50), None)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询记忆失败（降级空列表）");
            Vec::new()
        });
    let yesterday_memories: Vec<Memory> = all_memories
        .into_iter()
        .filter(|m| m.created_at.starts_with(&yesterday_str))
        .take(20)
        .collect();

    // 未读耳语通知
    let all_notifications = db::notifications::list_notifications(pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询通知失败（降级空列表）");
            Vec::new()
        });
    let unread_whispers: Vec<NotificationWithRole> = all_notifications
        .into_iter()
        .filter(|n| n.level == "whisper" && !n.is_read)
        .take(10)
        .collect();

    Ok(BriefingData {
        role_statuses,
        due_tasks,
        yesterday_memories,
        unread_whispers,
    })
}

/// 构造简报 prompt
pub fn build_briefing_prompt(data: &BriefingData, today: &str) -> Vec<ChatCompletionMessage> {
    let system = "你是 EgoSync 的管家，负责为用户生成每日晨间简报。\
请用温暖但简洁的中文语气，生成一段自然语言段落简报（约 200-400 字），不要使用 JSON 格式或 Markdown 标题。\
简报应包含以下部分：\
1. 各角色状态概览（能量值和待处理任务数）\
2. 今日截止任务提醒（如有）\
3. 昨日新增记忆要点（如有）\
4. 未读耳语通知汇总（如有）\
5. 结尾识别「今天最重要的一件事」——从今日截止任务或最高能量角色的首要任务中选出一件\
段落中嵌入角色名称（纯文本即可）。整体风格自然流畅，像管家在跟用户说话。";

    let mut user = format!("[日期]\n{}\n", today);

    // 角色状态
    if !data.role_statuses.is_empty() {
        user.push_str("\n[各角色状态]\n");
        for status in &data.role_statuses {
            let urgent_tag = if status.has_urgent { " ⚠️有紧急任务" } else { "" };
            user.push_str(&format!(
                "- {}：能量 {}，待处理任务 {} 个{}\n",
                status.role_name, status.energy, status.pending_tasks_count, urgent_tag
            ));
        }
    }

    // 今日截止任务
    if !data.due_tasks.is_empty() {
        user.push_str("\n[今日截止任务]\n");
        for task in &data.due_tasks {
            let role_tag = task
                .role_id
                .as_deref()
                .map(|rid| {
                    data.role_statuses
                        .iter()
                        .find(|r| r.role_id == rid)
                        .map(|r| format!("（{}）", r.role_name))
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            user.push_str(&format!(
                "- [{}] {}{}\n",
                task.quadrant, task.title, role_tag
            ));
        }
    }

    // 昨日记忆
    if !data.yesterday_memories.is_empty() {
        user.push_str("\n[昨日新增记忆]\n");
        for memory in &data.yesterday_memories {
            user.push_str(&format!("- [{}] {}\n", memory.category, memory.content));
        }
    }

    // 未读耳语通知
    if !data.unread_whispers.is_empty() {
        user.push_str("\n[未读耳语通知]\n");
        for notif in &data.unread_whispers {
            user.push_str(&format!("- {}：{}\n", notif.role_name, notif.content));
        }
    }

    user.push_str("\n请基于以上信息生成今日晨间简报。结尾用单独一段指出「今天最重要的一件事」。");

    vec![
        ChatCompletionMessage {
            role: "system".to_string(),
            content: system.to_string(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: user,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

/// 调用 LLM 生成简报（复制自 suggestion_generator.rs，调整超时常量）
async fn call_llm(
    provider: Arc<dyn LlmProvider>,
    prompt: Vec<ChatCompletionMessage>,
) -> Result<String, AppError> {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let stream_handle = tokio::spawn(async move {
        if let Err(err) = provider
            .chat_stream(
                prompt,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::warn!(error = %err, "briefing generation provider stream failed");
        }
    });

    let mut response = String::new();
    let stream_result = timeout(Duration::from_secs(LLM_TIMEOUT_SECS), async {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(token) => {
                    if response.len() + token.len() > MAX_BRIEFING_RESPONSE_BYTES {
                        tracing::warn!("briefing generation response exceeded size limit");
                        return Ok(());
                    }
                    response.push_str(&token);
                }
                StreamEvent::Done => return Ok::<(), AppError>(()),
                StreamEvent::Error(err) => {
                    tracing::warn!(error = %err, "briefing generation stream error");
                    return Ok(());
                }
                StreamEvent::Thinking(_) | StreamEvent::ToolCall(_) => {}
            }
        }
        Ok(())
    })
    .await;

    if stream_result.is_err() {
        stream_handle.abort();
        return Err(AppError::LlmError("简报生成 LLM 调用超时".to_string()));
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dashboard::DashboardStatus;
    use crate::models::memory::Memory;
    use crate::models::notification::NotificationWithRole;
    use crate::models::task::Task;

    fn make_dashboard_status(
        id: &str,
        name: &str,
        energy: i32,
        pending: i64,
        urgent: bool,
    ) -> DashboardStatus {
        DashboardStatus {
            role_id: id.to_string(),
            role_name: name.to_string(),
            role_icon: "🎯".to_string(),
            role_color: "#6366F1".to_string(),
            energy,
            pending_tasks_count: pending,
            last_active_at: None,
            has_urgent: urgent,
        }
    }

    fn make_task(id: &str, title: &str, quadrant: &str, role_id: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            owner_type: "role".to_string(),
            role_id: role_id.map(|s| s.to_string()),
            title: title.to_string(),
            deadline: Some("2026-06-25".to_string()),
            quadrant: quadrant.to_string(),
            is_big_rock: false,
            is_completed: false,
            completed_at: None,
            sort_order: 0,
            protection_status: "normal".to_string(),
            confidence: None,
            manual_override: false,
            classification_reason: None,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
        }
    }

    fn make_memory(id: &str, category: &str, content: &str, created_at: &str) -> Memory {
        Memory {
            id: id.to_string(),
            role_id: None,
            category: category.to_string(),
            content: content.to_string(),
            source_conversation_id: "conv-1".to_string(),
            source_message_ids: "[]".to_string(),
            created_at: created_at.to_string(),
        }
    }

    fn make_notification(id: &str, role_name: &str, content: &str, level: &str, is_read: bool) -> NotificationWithRole {
        NotificationWithRole {
            id: id.to_string(),
            role_id: "r1".to_string(),
            level: level.to_string(),
            content: content.to_string(),
            is_read,
            created_at: "2026-06-24T10:00:00Z".to_string(),
            role_name: role_name.to_string(),
            role_icon: "🎯".to_string(),
            role_color: "#6366F1".to_string(),
        }
    }

    fn make_briefing_data(
        roles: Vec<DashboardStatus>,
        tasks: Vec<Task>,
        memories: Vec<Memory>,
        whispers: Vec<NotificationWithRole>,
    ) -> BriefingData {
        BriefingData {
            role_statuses: roles,
            due_tasks: tasks,
            yesterday_memories: memories,
            unread_whispers: whispers,
        }
    }

    #[test]
    fn build_briefing_prompt_contains_role_status() {
        let data = make_briefing_data(
            vec![make_dashboard_status("r1", "产品经理", 80, 3, false)],
            vec![],
            vec![],
            vec![],
        );
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("产品经理"));
        assert!(prompt[1].content.contains("80"));
        assert!(prompt[1].content.contains("3"));
    }

    #[test]
    fn build_briefing_prompt_contains_due_tasks() {
        let data = make_briefing_data(
            vec![],
            vec![make_task("t1", "写需求文档", "Q1", None)],
            vec![],
            vec![],
        );
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("写需求文档"));
        assert!(prompt[1].content.contains("Q1"));
    }

    #[test]
    fn build_briefing_prompt_contains_memories() {
        let data = make_briefing_data(
            vec![],
            vec![],
            vec![make_memory("m1", "preference", "用户喜欢简洁汇报", "2026-06-24T10:00:00Z")],
            vec![],
        );
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("用户喜欢简洁汇报"));
        assert!(prompt[1].content.contains("preference"));
    }

    #[test]
    fn build_briefing_prompt_contains_whisper_notifications() {
        let data = make_briefing_data(
            vec![],
            vec![],
            vec![],
            vec![make_notification("n1", "产品经理", "有个新建议", "whisper", false)],
        );
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("产品经理"));
        assert!(prompt[1].content.contains("有个新建议"));
    }

    #[test]
    fn build_briefing_prompt_contains_date() {
        let data = make_briefing_data(vec![], vec![], vec![], vec![]);
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("2026-06-25"));
    }

    #[test]
    fn build_briefing_prompt_contains_most_important_thing_instruction() {
        let data = make_briefing_data(vec![], vec![], vec![], vec![]);
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("今天最重要的一件事"));
    }

    #[test]
    fn build_briefing_prompt_system_message_contains_role_description() {
        let data = make_briefing_data(vec![], vec![], vec![], vec![]);
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert_eq!(prompt[0].role, "system");
        assert!(prompt[0].content.contains("EgoSync"));
        assert!(prompt[0].content.contains("管家"));
    }

    #[test]
    fn build_briefing_prompt_empty_data_still_has_sections() {
        let data = make_briefing_data(vec![], vec![], vec![], vec![]);
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("2026-06-25"));
        assert!(prompt[1].content.contains("今天最重要的一件事"));
    }

    #[test]
    fn build_briefing_prompt_urgent_tag_shown() {
        let data = make_briefing_data(
            vec![make_dashboard_status("r1", "紧急角色", 50, 2, true)],
            vec![],
            vec![],
            vec![],
        );
        let prompt = build_briefing_prompt(&data, "2026-06-25");
        assert!(prompt[1].content.contains("⚠️有紧急任务"));
    }

    #[test]
    fn briefing_generated_event_name_is_correct() {
        assert_eq!(BRIEFING_GENERATED_EVENT, "briefing:generated");
    }

    #[test]
    fn default_briefing_time_is_08_00() {
        assert_eq!(DEFAULT_BRIEFING_TIME, "08:00");
    }

    #[test]
    fn briefing_time_key_is_correct() {
        assert_eq!(BRIEFING_TIME_KEY, "briefing_time");
    }
}
