use std::sync::Arc;

use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::db::pool::DbPool;
use crate::db;
use crate::error::AppError;
use crate::llm::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent};
use crate::models::role::Role;
use crate::models::suggestion::{CreateSuggestionInput, Suggestion};
use crate::services::agent_engine;

const LLM_SUGGESTION_TIMEOUT_SECS: u64 = 60;
const MAX_SUGGESTION_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_SUGGESTIONS: usize = 3;
const DEDUP_WINDOW_DAYS: i64 = 7;
const ALLOWED_PRIORITIES: &[&str] = &["high", "medium", "low"];

fn strip_json_code_fence(response: &str) -> &str {
    let Some(start) = response.find('{') else {
        return response;
    };
    let Some(end) = response.rfind('}') else {
        return response;
    };
    if start > end {
        return response;
    }
    response[start..=end].trim()
}

fn normalize_title(title: &str) -> String {
    title.trim().to_lowercase()
}

pub fn is_exact_title_duplicate(
    candidate_title: &str,
    recent_suggestions: &[Suggestion],
) -> bool {
    let normalized = normalize_title(candidate_title);
    if normalized.is_empty() {
        return false;
    }
    recent_suggestions
        .iter()
        .any(|s| normalize_title(&s.title) == normalized)
}

pub fn build_suggestion_prompt(
    role: &Role,
    task_summary: &str,
    memory_summary: &str,
    recent_suggestions: &[Suggestion],
    rejected_suggestions: &[Suggestion],
) -> Vec<ChatCompletionMessage> {
    let system = "你是 EgoSync 的角色主动建议生成器。只输出严格 JSON 对象，不要 Markdown、code fence 或解释文本。\
顶层格式必须是 {\"suggestions\":[{\"title\":\"...\",\"content\":\"...\",\"priority\":\"high|medium|low\"}]}。\
最多 3 条建议；没有有价值的建议时返回 {\"suggestions\":[]}。\
建议必须基于提供的角色目标、任务状态和记忆上下文，不得凭空捏造。\
title 是简短标题（不超过 30 字），content 是具体建议内容（100-200 字），priority 是优先级。";

    let mut user = format!(
        "[角色信息]\n名称：{}\n核心目标：{}\n",
        role.name,
        role.goal.trim()
    );

    if !role.personality_prompt.trim().is_empty() {
        user.push_str(&format!("个性描述：{}\n", role.personality_prompt.trim()));
    }

    if !task_summary.is_empty() {
        user.push_str(&format!("\n[当前任务状态]\n{}\n", task_summary));
    }

    if !memory_summary.is_empty() {
        user.push_str(&format!("\n[角色记忆]\n{}\n", memory_summary));
    }

    if !recent_suggestions.is_empty() {
        let existing_lines: Vec<String> = recent_suggestions
            .iter()
            .map(|s| format!("- 标题：{} | 内容：{}", s.title, s.content))
            .collect();
        user.push_str(&format!(
            "\n[近{}天已有建议]\n{}\n",
            DEDUP_WINDOW_DAYS,
            existing_lines.join("\n")
        ));
        user.push_str("\n请不要生成与以上已有建议重复或高度相似的新建议。若没有新的有价值建议，返回空数组。\n");
    }

    if !rejected_suggestions.is_empty() {
        let rejected_lines: Vec<String> = rejected_suggestions
            .iter()
            .map(|s| {
                let reason = s.rejection_reason.as_deref().unwrap_or("未知");
                format!("- 标题：{} | 拒绝原因：{}", s.title, reason)
            })
            .collect();
        user.push_str(&format!(
            "\n[用户曾拒绝的建议]\n{}\n请避免生成与以上被拒绝建议类似的内容。\n",
            rejected_lines.join("\n")
        ));
    }

    user.push_str("\n请基于以上上下文，为该角色生成 0-3 条有价值的主动建议。");

    vec![
        ChatCompletionMessage {
            role: "system".to_string(),
            content: system.to_string(),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatCompletionMessage {
            role: "user".to_string(),
            content: user,
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

#[derive(Deserialize)]
struct SuggestionsResponse {
    #[serde(default)]
    suggestions: Vec<RawSuggestion>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSuggestion {
    title: String,
    content: String,
    priority: String,
}

pub fn parse_suggestions_response(response: &str) -> Result<Vec<RawSuggestion>, AppError> {
    let cleaned = strip_json_code_fence(response.trim());
    let parsed: SuggestionsResponse = serde_json::from_str(cleaned)
        .map_err(|e| AppError::ValidationError(format!("建议 JSON 解析失败: {}", e)))?;

    let filtered: Vec<RawSuggestion> = parsed
        .suggestions
        .into_iter()
        .filter(|s| {
            let priority = s.priority.trim().to_lowercase();
            ALLOWED_PRIORITIES.contains(&priority.as_str())
        })
        .take(MAX_SUGGESTIONS)
        .collect();

    Ok(filtered)
}

fn seven_days_ago_iso() -> String {
    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::days(DEDUP_WINDOW_DAYS);
    cutoff.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub async fn generate_suggestions(
    main_pool: &DbPool,
    role: &Role,
) -> Result<Vec<CreateSuggestionInput>, AppError> {
    let task_summary = match agent_engine::build_role_task_summary(main_pool, &role.id).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                error = %e,
                "构建任务摘要失败，继续生成建议（无任务上下文）"
            );
            String::new()
        }
    };

    let memory_summary = match agent_engine::build_role_memory_summary(main_pool, &role.id).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                error = %e,
                "构建记忆摘要失败，继续生成建议（无记忆上下文）"
            );
            String::new()
        }
    };

    if role.goal.trim().is_empty() && task_summary.is_empty() && memory_summary.is_empty() {
        tracing::info!(
            role_id = %role.id,
            role_name = %role.name,
            "角色无目标、无任务、无记忆，跳过 LLM 调用，本次无建议生成"
        );
        return Ok(Vec::new());
    }

    let since_iso = seven_days_ago_iso();
    let recent_suggestions = match db::suggestions::list_recent_suggestions(main_pool, &role.id, &since_iso).await {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                error = %e,
                "查询近期建议失败，继续生成建议（无去重上下文）"
            );
            Vec::new()
        }
    };

    let rejected_suggestions = match db::suggestions::list_rejected_suggestions(main_pool, &role.id, &since_iso).await {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                error = %e,
                "查询已拒绝建议失败，继续生成建议（无拒绝反馈上下文）"
            );
            Vec::new()
        }
    };

    let prompt = build_suggestion_prompt(role, &task_summary, &memory_summary, &recent_suggestions, &rejected_suggestions);

    let provider = match agent_engine::resolve_default_provider(main_pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                role_name = %role.name,
                error = %e,
                "解析 LLM provider 失败，降级返回空建议"
            );
            return Ok(Vec::new());
        }
    };

    let response = match call_llm(provider, prompt).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                role_name = %role.name,
                error = %e,
                "LLM 调用失败，降级返回空建议"
            );
            return Ok(Vec::new());
        }
    };

    let raw_suggestions = match parse_suggestions_response(&response) {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(
                role_id = %role.id,
                role_name = %role.name,
                error = %e,
                "建议 JSON 解析失败，降级返回空建议"
            );
            return Ok(Vec::new());
        }
    };

    // 去重：先用 is_exact_title_duplicate 拦截近 7 天历史重复，
    // 再在本批次内逐条登记归一化 title，避免同一次 LLM 返回的多条同名建议同时写入。
    let mut batch_titles: Vec<String> = Vec::new();
    let mut result: Vec<CreateSuggestionInput> = Vec::new();
    for s in raw_suggestions {
        let title = s.title.trim().to_string();
        let content = s.content.trim().to_string();
        if title.is_empty() || content.is_empty() {
            continue;
        }
        if is_exact_title_duplicate(&title, &recent_suggestions) {
            continue;
        }
        let normalized = normalize_title(&title);
        if batch_titles.contains(&normalized) {
            continue;
        }
        batch_titles.push(normalized);
        result.push(CreateSuggestionInput {
            role_id: role.id.clone(),
            title,
            content,
            priority: s.priority.trim().to_lowercase(),
            conversation_id: None,
        });
    }

    Ok(result)
}

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
            tracing::warn!(error = %err, "suggestion generation provider stream failed");
        }
    });

    let mut response = String::new();
    let stream_result = timeout(Duration::from_secs(LLM_SUGGESTION_TIMEOUT_SECS), async {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(token) => {
                    if response.len() + token.len() > MAX_SUGGESTION_RESPONSE_BYTES {
                        tracing::warn!("suggestion generation response exceeded size limit");
                        return Ok(());
                    }
                    response.push_str(&token);
                }
                StreamEvent::Done => return Ok::<(), AppError>(()),
                StreamEvent::Error(err) => {
                    tracing::warn!(error = %err, "suggestion generation stream error");
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
        return Err(AppError::LlmError("建议生成 LLM 调用超时".to_string()));
    }

    Ok(response)
}

/// 通知级别枚举 — Story 4.5 三级通知系统的消费类型。
///
/// `Whisper` < `Tap` < `Knock`，由 `max_notification_level_for_proactivity` 按主动性档位约束。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationLevel {
    Whisper,
    Tap,
    Knock,
}

/// 根据主动性档位返回允许的最高通知级别。
///
/// - `proactive` → `Knock`（允许"敲门"）
/// - `moderate` → `Tap`（最高"轻触"，不允许"敲门"）
/// - `passive` / 未知值 → `Whisper`（安全降级）
///
/// 此函数无当前消费者，供 Story 4.5 直接调用。
pub fn max_notification_level_for_proactivity(proactivity_level: &str) -> NotificationLevel {
    match proactivity_level {
        "proactive" => NotificationLevel::Knock,
        "moderate" => NotificationLevel::Tap,
        _ => NotificationLevel::Whisper,
    }
}

/// 根据主动性档位过滤建议列表。
///
/// - `proactive` → 保留全部（high / medium / low）
/// - `moderate` → 过滤掉 `priority = "low"`，保留 `high` + `medium`
/// - `passive` / 未知值 → 返回空 `Vec`（安全降级）
///
/// 不修改 `CreateSuggestionInput` 的任何字段，只做过滤。
pub fn filter_suggestions_by_proactivity(
    suggestions: Vec<CreateSuggestionInput>,
    proactivity_level: &str,
) -> Vec<CreateSuggestionInput> {
    match proactivity_level {
        "proactive" => suggestions,
        "moderate" => suggestions
            .into_iter()
            .filter(|s| s.priority != "low")
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::role::Role;
    use crate::models::suggestion::Suggestion;

    fn make_test_role() -> Role {
        Role {
            id: "test-role-id".to_string(),
            name: "产品经理".to_string(),
            icon: "🎯".to_string(),
            color: "#6366F1".to_string(),
            goal: "推进产品落地".to_string(),
            personality_prompt: "简洁专业".to_string(),
            status: "active".to_string(),
            energy: 100,
            energy_updated_at: None,
            skills_config: "{}".to_string(),
            proactivity_level: "moderate".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn make_suggestion(id: &str, title: &str, content: &str) -> Suggestion {
        Suggestion {
            id: id.to_string(),
            role_id: "test-role-id".to_string(),
            title: title.to_string(),
            content: content.to_string(),
            priority: "medium".to_string(),
            status: "pending".to_string(),
            rejection_reason: None,
            converted_task_id: None,
            conversation_id: None,
            created_at: "2026-06-20T00:00:00Z".to_string(),
        }
    }

    // --- parse_suggestions_response tests ---

    #[test]
    fn parse_valid_json_without_code_fence() {
        let json = r#"{"suggestions":[{"title":"建议A","content":"内容A","priority":"high"}]}"#;
        let result = parse_suggestions_response(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "建议A");
        assert_eq!(result[0].priority, "high");
    }

    #[test]
    fn parse_valid_json_with_code_fence() {
        let json = r#"```json
{"suggestions":[{"title":"建议B","content":"内容B","priority":"low"}]}
```"#;
        let result = parse_suggestions_response(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "建议B");
    }

    #[test]
    fn parse_more_than_3_suggestions_truncates() {
        let json = r#"{"suggestions":[
            {"title":"A","content":"a","priority":"high"},
            {"title":"B","content":"b","priority":"medium"},
            {"title":"C","content":"c","priority":"low"},
            {"title":"D","content":"d","priority":"high"}
        ]}"#;
        let result = parse_suggestions_response(json).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn parse_invalid_priority_filtered() {
        let json = r#"{"suggestions":[
            {"title":"A","content":"a","priority":"high"},
            {"title":"B","content":"b","priority":"urgent"}
        ]}"#;
        let result = parse_suggestions_response(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "A");
    }

    #[test]
    fn parse_empty_suggestions() {
        let json = r#"{"suggestions":[]}"#;
        let result = parse_suggestions_response(json).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_non_json_returns_error() {
        let result = parse_suggestions_response("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_suggestions_field_defaults_empty() {
        let json = r#"{"other":"value"}"#;
        let result = parse_suggestions_response(json).unwrap();
        assert!(result.is_empty());
    }

    // --- is_exact_title_duplicate tests ---

    #[test]
    fn dedup_exact_match_returns_true() {
        let recent = vec![make_suggestion("1", "检查产品进度", "内容")];
        assert!(is_exact_title_duplicate("检查产品进度", &recent));
    }

    #[test]
    fn dedup_case_insensitive_match_returns_true() {
        let recent = vec![make_suggestion("1", "Review Code", "内容")];
        assert!(is_exact_title_duplicate("review code", &recent));
    }

    #[test]
    fn dedup_different_title_returns_false() {
        let recent = vec![make_suggestion("1", "检查产品进度", "内容")];
        assert!(!is_exact_title_duplicate("测试新功能", &recent));
    }

    #[test]
    fn dedup_empty_recent_returns_false() {
        let recent: Vec<Suggestion> = vec![];
        assert!(!is_exact_title_duplicate("任何建议", &recent));
    }

    #[test]
    fn dedup_trim_whitespace_match() {
        let recent = vec![make_suggestion("1", "  检查进度  ", "内容")];
        assert!(is_exact_title_duplicate("检查进度", &recent));
    }

    // --- build_suggestion_prompt tests ---

    #[test]
    fn prompt_system_contains_strict_json_constraint() {
        let role = make_test_role();
        let prompt = build_suggestion_prompt(&role, "", "", &[], &[]);
        assert_eq!(prompt[0].role, "system");
        assert!(prompt[0].content.contains("严格 JSON"));
        assert!(prompt[0].content.contains("suggestions"));
    }

    #[test]
    fn prompt_user_contains_role_goal() {
        let role = make_test_role();
        let prompt = build_suggestion_prompt(&role, "", "", &[], &[]);
        assert_eq!(prompt[1].role, "user");
        assert!(prompt[1].content.contains("推进产品落地"));
        assert!(prompt[1].content.contains("产品经理"));
    }

    #[test]
    fn prompt_user_contains_task_summary() {
        let role = make_test_role();
        let prompt = build_suggestion_prompt(&role, "[未完成] 写需求文档", "", &[], &[]);
        assert!(prompt[1].content.contains("写需求文档"));
    }

    #[test]
    fn prompt_user_contains_memory_summary() {
        let role = make_test_role();
        let prompt = build_suggestion_prompt(&role, "", "用户偏好简洁汇报", &[], &[]);
        assert!(prompt[1].content.contains("简洁汇报"));
    }

    #[test]
    fn prompt_user_contains_recent_suggestions_dedup_instruction() {
        let role = make_test_role();
        let recent = vec![
            make_suggestion("1", "已有建议A", "已有内容A"),
            make_suggestion("2", "已有建议B", "已有内容B"),
        ];
        let prompt = build_suggestion_prompt(&role, "", "", &recent, &[]);
        assert!(prompt[1].content.contains("已有建议A"));
        assert!(prompt[1].content.contains("已有建议B"));
        assert!(prompt[1].content.contains("重复"));
    }

    #[test]
    fn prompt_user_no_dedup_section_when_empty() {
        let role = make_test_role();
        let prompt = build_suggestion_prompt(&role, "", "", &[], &[]);
        assert!(!prompt[1].content.contains("已有建议"));
    }

    // --- strip_json_code_fence tests ---

    #[test]
    fn strip_fence_extracts_json_from_code_block() {
        let input = "```json\n{\"key\":\"value\"}\n```";
        assert_eq!(strip_json_code_fence(input), "{\"key\":\"value\"}");
    }

    #[test]
    fn strip_fence_returns_as_is_for_pure_json() {
        let input = "{\"key\":\"value\"}";
        assert_eq!(strip_json_code_fence(input), "{\"key\":\"value\"}");
    }

    #[test]
    fn strip_fence_returns_as_is_for_non_json() {
        let input = "not json";
        assert_eq!(strip_json_code_fence(input), "not json");
    }

    // --- filter_suggestions_by_proactivity tests ---

    fn make_create_input(priority: &str) -> CreateSuggestionInput {
        CreateSuggestionInput {
            role_id: "test-role-id".to_string(),
            title: "测试建议".to_string(),
            content: "测试内容".to_string(),
            priority: priority.to_string(),
            conversation_id: None,
        }
    }

    #[test]
    fn filter_moderate_removes_low_keeps_high_and_medium() {
        let inputs = vec![
            make_create_input("high"),
            make_create_input("medium"),
            make_create_input("low"),
        ];
        let result = filter_suggestions_by_proactivity(inputs, "moderate");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].priority, "high");
        assert_eq!(result[1].priority, "medium");
    }

    #[test]
    fn filter_proactive_keeps_all() {
        let inputs = vec![
            make_create_input("high"),
            make_create_input("medium"),
            make_create_input("low"),
        ];
        let result = filter_suggestions_by_proactivity(inputs, "proactive");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn filter_passive_returns_empty() {
        let inputs = vec![
            make_create_input("high"),
            make_create_input("medium"),
        ];
        let result = filter_suggestions_by_proactivity(inputs, "passive");
        assert!(result.is_empty());
    }

    #[test]
    fn filter_unknown_level_returns_empty() {
        let inputs = vec![make_create_input("high")];
        let result = filter_suggestions_by_proactivity(inputs, "unknown");
        assert!(result.is_empty());
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let inputs: Vec<CreateSuggestionInput> = vec![];
        let result = filter_suggestions_by_proactivity(inputs, "moderate");
        assert!(result.is_empty());
    }

    #[test]
    fn filter_mixed_priorities_correct_filtering() {
        let inputs = vec![
            make_create_input("low"),
            make_create_input("high"),
            make_create_input("low"),
            make_create_input("medium"),
            make_create_input("high"),
        ];
        let result = filter_suggestions_by_proactivity(inputs, "moderate");
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().filter(|s| s.priority == "low").count(), 0);
        assert_eq!(result.iter().filter(|s| s.priority == "high").count(), 2);
        assert_eq!(result.iter().filter(|s| s.priority == "medium").count(), 1);
    }

    // --- max_notification_level_for_proactivity tests ---

    #[test]
    fn notification_level_passive_returns_whisper() {
        assert_eq!(
            max_notification_level_for_proactivity("passive"),
            NotificationLevel::Whisper
        );
    }

    #[test]
    fn notification_level_moderate_returns_tap() {
        assert_eq!(
            max_notification_level_for_proactivity("moderate"),
            NotificationLevel::Tap
        );
    }

    #[test]
    fn notification_level_proactive_returns_knock() {
        assert_eq!(
            max_notification_level_for_proactivity("proactive"),
            NotificationLevel::Knock
        );
    }

    #[test]
    fn notification_level_unknown_returns_whisper() {
        assert_eq!(
            max_notification_level_for_proactivity("unknown"),
            NotificationLevel::Whisper
        );
        assert_eq!(
            max_notification_level_for_proactivity(""),
            NotificationLevel::Whisper
        );
    }

    // --- rejected suggestions injection tests ---

    fn make_rejected_suggestion(id: &str, title: &str, reason: &str) -> Suggestion {
        Suggestion {
            id: id.to_string(),
            role_id: "test-role-id".to_string(),
            title: title.to_string(),
            content: "内容".to_string(),
            priority: "medium".to_string(),
            status: "rejected".to_string(),
            rejection_reason: Some(reason.to_string()),
            converted_task_id: None,
            conversation_id: None,
            created_at: "2026-06-20T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn prompt_contains_rejected_suggestions_section_when_non_empty() {
        let role = make_test_role();
        let rejected = vec![
            make_rejected_suggestion("1", "建议A", "irrelevant"),
            make_rejected_suggestion("2", "建议B", "bad_timing"),
        ];
        let prompt = build_suggestion_prompt(&role, "", "", &[], &rejected);
        assert!(prompt[1].content.contains("用户曾拒绝的建议"));
        assert!(prompt[1].content.contains("建议A"));
        assert!(prompt[1].content.contains("irrelevant"));
        assert!(prompt[1].content.contains("建议B"));
        assert!(prompt[1].content.contains("bad_timing"));
        assert!(prompt[1].content.contains("请避免生成与以上被拒绝建议类似的内容"));
    }

    #[test]
    fn prompt_no_rejected_section_when_empty() {
        let role = make_test_role();
        let prompt = build_suggestion_prompt(&role, "", "", &[], &[]);
        assert!(!prompt[1].content.contains("用户曾拒绝的建议"));
    }

    #[test]
    fn prompt_rejected_suggestion_with_none_reason_shows_unknown() {
        let role = make_test_role();
        let mut rejected = make_rejected_suggestion("1", "建议A", "irrelevant");
        rejected.rejection_reason = None;
        let prompt = build_suggestion_prompt(&role, "", "", &[], &[rejected]);
        assert!(prompt[1].content.contains("未知"));
    }
}
