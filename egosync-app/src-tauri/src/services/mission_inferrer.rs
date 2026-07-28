//! Story 5.2: 未设定使命时管家基于行为推断隐含价值观
//!
//! 职责：
//! - 当用户未设定使命宣言时，收集最近 30 天对话/任务/记忆/角色数据
//! - 调用默认 LLM provider 推断 2-3 条隐含价值观优先级
//! - 返回结构化 JSON（values + summary + confidence）
//! - LLM 超时/错误/解析失败 → 返回 None（降级，不阻塞用户）
//! - 数据不足（对话 < 5 轮或任务 < 5 个）→ 跳过推断
//! - 手动触发模式：用户随时可重新推断，使命宣言是否已设定不影响可点击性

use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::db;
use crate::db::pool::ConversationsPool;
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent};
use crate::services::secret_store;

/// LLM 调用超时上限（秒）。与 task_classifier 一致。
const LLM_TIMEOUT_SECS: u64 = 12;

/// 最小对话轮数阈值。低于此值不执行推断。
const MIN_CONVERSATIONS: usize = 5;

/// 最小任务数阈值。低于此值不执行推断。
const MIN_TASKS: usize = 5;

/// 历史数据天数窗口。
const HISTORY_DAYS: i64 = 30;

/// 每个对话取最近 user 消息条数上限。
const MSG_PER_CONVERSATION: usize = 5;

/// 每个对话拉取的最近消息条数（含非 user 消息），过滤后再截断到 MSG_PER_CONVERSATION。
const RECENT_MESSAGES_FETCH: i64 = 10;

/// 推断来源：用于日志与前端决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InferenceSource {
    Llm,
    SkippedHasMission,
    SkippedInsufficientData,
    FallbackLlmError,
    FallbackTimeout,
    FallbackParseError,
}

/// LLM 推断的隐含价值观。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferredValues {
    pub values: Vec<String>,
    /// 第一人称使命宣言文本（以「我」开头），可被用户直接采纳为个人使命宣言（format='free'）。
    pub summary: String,
    pub confidence: f64,
}

/// 推断结果：包含值（可能为 None）和来源。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceOutcome {
    pub values: Option<InferredValues>,
    pub source: InferenceSource,
}

/// 推断资格预检结果（不调用 LLM，仅检查数据量）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceEligibility {
    pub eligible: bool,
    pub reason: String,
}

/// 预检：不调用 LLM，仅检查数据是否充足。
///
/// 前端用于决定"推断使命宣言"按钮是否可点击。
/// 注意：手动触发模式下，使命宣言是否已设定不影响可点击性——用户可随时重新推断。
pub async fn check_eligibility(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
) -> Result<InferenceEligibility, AppError> {
    let conversations = db::conversations::list_all_conversations(conv_pool).await?;
    if conversations.len() < MIN_CONVERSATIONS {
        return Ok(InferenceEligibility {
            eligible: false,
            reason: format!(
                "对话记录不足（当前 {} 轮，需至少 {} 轮）",
                conversations.len(),
                MIN_CONVERSATIONS
            ),
        });
    }

    let tasks = db::tasks::list_all_tasks(pool, None, None).await?;
    if tasks.len() < MIN_TASKS {
        return Ok(InferenceEligibility {
            eligible: false,
            reason: format!(
                "任务数量不足（当前 {} 个，需至少 {} 个）",
                tasks.len(),
                MIN_TASKS
            ),
        });
    }

    Ok(InferenceEligibility {
        eligible: true,
        reason: String::new(),
    })
}

/// 主入口：推断用户隐含价值观。
///
/// 调用链路：
/// 1. 若对话数 < MIN_CONVERSATIONS 或任务数 < MIN_TASKS → SkippedInsufficientData
/// 2. 收集历史数据 → 构造 prompt → 调用 LLM → 解析响应
/// 3. LLM 失败/超时/解析失败 → 对应 Fallback source（None values）
pub async fn infer_values(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
) -> Result<InferenceOutcome, AppError> {
    // 1. 检查数据充足性
    let conversations = db::conversations::list_all_conversations(conv_pool).await?;
    if conversations.len() < MIN_CONVERSATIONS {
        return Ok(InferenceOutcome {
            values: None,
            source: InferenceSource::SkippedInsufficientData,
        });
    }

    let tasks = db::tasks::list_all_tasks(pool, None, None).await?;
    if tasks.len() < MIN_TASKS {
        return Ok(InferenceOutcome {
            values: None,
            source: InferenceSource::SkippedInsufficientData,
        });
    }

    // 3. 收集历史数据
    let conversation_text = collect_conversation_summary(conv_pool, &conversations).await;
    let task_text = format_task_list(&tasks);
    let memories = db::memories::list_all_memories_with_options(pool, None, Some(50), None)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "读取记忆失败，使用空列表继续");
            Vec::new()
        });
    let memory_text = format_memory_list(&memories);
    let roles = db::roles::list_active_roles(pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "读取活跃角色失败，使用空列表继续");
            Vec::new()
        });
    let role_text = format_role_list(&roles);

    // 4. 构造 prompt 并调用 LLM
    let prompt = build_inference_prompt(&conversation_text, &task_text, &memory_text, &role_text);

    let provider = match build_default_provider(pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "未能加载默认 LLM 配置，价值观推断降级为 None");
            return Ok(InferenceOutcome {
                values: None,
                source: InferenceSource::FallbackLlmError,
            });
        }
    };

    let raw = match call_llm_with_timeout(provider.as_ref(), &prompt).await {
        Ok(text) => text,
        Err(LlmCallError::Timeout) => {
            return Ok(InferenceOutcome {
                values: None,
                source: InferenceSource::FallbackTimeout,
            });
        }
        Err(LlmCallError::Provider(e)) => {
            tracing::warn!(error = %e, "LLM 调用失败，价值观推断降级为 None");
            return Ok(InferenceOutcome {
                values: None,
                source: InferenceSource::FallbackLlmError,
            });
        }
    };

    match parse_inference_response(&raw) {
        Some(values) => Ok(InferenceOutcome {
            values: Some(values),
            source: InferenceSource::Llm,
        }),
        None => {
            tracing::warn!(raw = %truncate(&raw, 200), "LLM 返回无法解析为推断 JSON，降级为 None");
            Ok(InferenceOutcome {
                values: None,
                source: InferenceSource::FallbackParseError,
            })
        }
    }
}

/// 收集最近 30 天对话摘要：遍历全部对话，每个取最近 5 条 user 消息。
async fn collect_conversation_summary(
    conv_pool: &ConversationsPool,
    conversations: &[crate::models::chat::Conversation],
) -> String {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(HISTORY_DAYS);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut lines = Vec::new();
    for conv in conversations {
        let messages = db::conversations::get_recent_messages(conv_pool, &conv.id, RECENT_MESSAGES_FETCH)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(conv_id = %conv.id, error = %e, "读取对话消息失败，跳过");
                Vec::new()
            });

        let user_msgs: Vec<_> = messages
            .iter()
            .filter(|m| m.role == "user" && m.created_at >= cutoff_str)
            .take(MSG_PER_CONVERSATION)
            .collect();

        if user_msgs.is_empty() {
            continue;
        }

        let title = if conv.title.is_empty() { "无标题" } else { &conv.title };
        let msg_block = user_msgs
            .iter()
            .enumerate()
            .map(|(i, m)| format!("  {}. {}", i + 1, truncate(&m.content, 200)))
            .collect::<Vec<_>>()
            .join("\n");
        lines.push(format!("对话「{}」：\n{}", title, msg_block));
    }

    if lines.is_empty() {
        "（最近 30 天无对话记录）".to_string()
    } else {
        lines.join("\n\n")
    }
}

/// 格式化任务列表为文本。
fn format_task_list(tasks: &[crate::models::task::CrossRoleTask]) -> String {
    if tasks.is_empty() {
        return "（暂无任务）".to_string();
    }
    tasks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let role_name = t.role_name.as_deref().unwrap_or("管家");
            let deadline = t.deadline.as_deref().unwrap_or("无");
            format!(
                "{}. 「{}」 角色={} 象限={} 截止={} 大石头={}",
                i + 1,
                t.title,
                role_name,
                t.quadrant,
                deadline,
                if t.is_big_rock { "是" } else { "否" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 格式化记忆列表为文本。
fn format_memory_list(memories: &[crate::models::memory::Memory]) -> String {
    if memories.is_empty() {
        return "（暂无记忆）".to_string();
    }
    memories
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{}. [{}] {}", i + 1, m.category, truncate(&m.content, 150)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 格式化角色列表为文本。
fn format_role_list(roles: &[crate::models::role::Role]) -> String {
    if roles.is_empty() {
        return "（暂无活跃角色）".to_string();
    }
    roles
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let goal = if r.goal.trim().is_empty() {
                "（未设定）"
            } else {
                &r.goal
            };
            format!("{}. {} {} — 目标：{}", i + 1, r.icon, r.name, goal)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 构造推断 prompt。
///
/// 输入：对话摘要 + 任务列表 + 记忆列表 + 角色列表。
/// 输出要求：严格 JSON `{ "values": [...], "summary": "...", "confidence": 0.0-1.0 }`。
pub fn build_inference_prompt(
    conversation_text: &str,
    task_text: &str,
    memory_text: &str,
    role_text: &str,
) -> String {
    format!(
        "你是 EgoSync 管家的价值观推断助手。用户尚未设定使命宣言，请根据用户的行为模式推断其隐含的价值观优先级。\n\n\
## 用户行为数据\n\n\
### 最近 30 天对话记录\n{conversations}\n\n\
### 全量任务\n{tasks}\n\n\
### 记忆摘要\n{memories}\n\n\
### 活跃角色\n{roles}\n\n\
## 推断要求\n\n\
1. 分析用户的对话主题、任务优先级、记忆内容和角色配置，推断 2-3 条隐含价值观优先级\n\
2. 每条价值观用「A > B > C」格式表示优先级排序\n\
3. summary 字段写成一段第一人称的使命宣言（必须以「我」开头，<=80字），自然概括用户的核心身份、要事优先级与所看重的角色，使其可直接作为用户的个人使命宣言；不要使用「用户」「该用户」等第三人称\n\
4. confidence 表示推断置信度（0.0-1.0），数据越充分置信度越高\n\n\
**严格只输出一段 JSON**，不要包裹在代码块里，结构如下：\n\
{{\"values\": [\"价值观1 > 价值观2 > 价值观3\"], \"summary\": \"我……（第一人称使命宣言，<=80字）\", \"confidence\": 0.0-1.0}}",
        conversations = conversation_text,
        tasks = task_text,
        memories = memory_text,
        roles = role_text,
    )
}

/// 解析 LLM 返回文本为 `InferredValues`。
///
/// 容忍 LLM 在 JSON 前后输出额外文字（提取第一个 `{` 到最后一个 `}` 的子串再尝试 parse）。
/// 任何不合法（values 为空、confidence 越界、缺字段）都返回 None。
pub fn parse_inference_response(raw: &str) -> Option<InferredValues> {
    let snippet = extract_json_object(raw)?;
    let value: serde_json::Value = serde_json::from_str(&snippet).ok()?;

    let values_arr = value.get("values")?.as_array()?;
    if values_arr.is_empty() {
        return None;
    }
    let values: Vec<String> = values_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if values.is_empty() {
        return None;
    }

    let summary = value.get("summary")?.as_str()?.trim().to_string();
    if summary.is_empty() {
        return None;
    }

    let confidence = value.get("confidence")?.as_f64()?;
    if !(0.0..=1.0).contains(&confidence) {
        return None;
    }

    Some(InferredValues {
        values,
        summary,
        confidence,
    })
}

// ── LLM 调用基础设施（从 task_classifier.rs 复制，保持模块独立性） ──

#[derive(Debug)]
enum LlmCallError {
    Timeout,
    Provider(AppError),
}

async fn call_llm_with_timeout(
    provider: &dyn LlmProvider,
    prompt: &str,
) -> Result<String, LlmCallError> {
    let messages = vec![ChatCompletionMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    }];
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let options = ChatOptions {
        disable_thinking: true,
        ..ChatOptions::default()
    };

    let collector = tauri::async_runtime::spawn(async move {
        let mut buffer = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(t) => buffer.push_str(&t),
                StreamEvent::Done => break,
                StreamEvent::Error(e) => return Err(AppError::LlmError(e)),
                StreamEvent::Thinking(_) | StreamEvent::ToolCall(_) => {}
            }
        }
        Ok(buffer)
    });

    let combined = async {
        match provider.chat_stream(messages, tx, options).await {
            Ok(()) => collector.await.unwrap_or_else(|_| Err(AppError::LlmError("收集器任务异常终止".to_string()))),
            Err(e) => Err(e),
        }
    };

    match timeout(Duration::from_secs(LLM_TIMEOUT_SECS), combined).await {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(LlmCallError::Provider(e)),
        Err(_) => Err(LlmCallError::Timeout),
    }
}

async fn build_default_provider(pool: &SqlitePool) -> Result<Box<dyn LlmProvider>, AppError> {
    let config = db::settings::get_default_llm_config(pool).await?;
    let api_key = secret_store::load_secret(&config.api_key_ref)?
        .ok_or_else(|| AppError::KeyringError(format!("未找到配置 '{}' 的 API Key", config.name)))?;

    use crate::models::settings::NetworkLocation;
    let net_loc = config.network_location.clone();
    let no_proxy = net_loc == NetworkLocation::Internal;

    let provider: Box<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Box::new(AnthropicProvider::new(config.base_url, api_key, config.model, no_proxy)?),
        "minimax" => Box::new(OpenAiProvider::new_with_reasoning_split(config.base_url, api_key, config.model, no_proxy)?),
        _ => Box::new(OpenAiProvider::new(config.base_url, api_key, config.model, no_proxy)?),
    };
    Ok(provider)
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end < start {
        return None;
    }
    Some(raw[start..=end].to_string())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inference_response_accepts_clean_json() {
        let raw = r#"{"values":["家庭陪伴 > 工作效率 > 个人学习"],"summary":"用户频繁优先处理家庭相关任务","confidence":0.85}"#;
        let parsed = parse_inference_response(raw).expect("should parse");
        assert_eq!(parsed.values, vec!["家庭陪伴 > 工作效率 > 个人学习"]);
        assert_eq!(parsed.summary, "用户频繁优先处理家庭相关任务");
        assert!((parsed.confidence - 0.85).abs() < 1e-9);
    }

    #[test]
    fn parse_inference_response_strips_surrounding_prose() {
        let raw = "根据分析，以下是推断结果：\n```json\n{\"values\":[\"健康 > 事业\"],\"summary\":\"注重健康优先\",\"confidence\":0.7}\n```\n希望对您有帮助。";
        let parsed = parse_inference_response(raw).expect("should parse despite wrappers");
        assert_eq!(parsed.values, vec!["健康 > 事业"]);
        assert_eq!(parsed.summary, "注重健康优先");
        assert!((parsed.confidence - 0.7).abs() < 1e-9);
    }

    #[test]
    fn parse_inference_response_rejects_empty_values() {
        let raw = r#"{"values":[],"summary":"测试","confidence":0.5}"#;
        assert!(parse_inference_response(raw).is_none());
    }

    #[test]
    fn parse_inference_response_rejects_out_of_range_confidence() {
        let raw_high = r#"{"values":["A > B"],"summary":"测试","confidence":1.5}"#;
        assert!(parse_inference_response(raw_high).is_none());

        let raw_low = r#"{"values":["A > B"],"summary":"测试","confidence":-0.1}"#;
        assert!(parse_inference_response(raw_low).is_none());
    }

    #[test]
    fn parse_inference_response_rejects_missing_field() {
        let no_values = r#"{"summary":"测试","confidence":0.5}"#;
        assert!(parse_inference_response(no_values).is_none());

        let no_summary = r#"{"values":["A > B"],"confidence":0.5}"#;
        assert!(parse_inference_response(no_summary).is_none());

        let no_confidence = r#"{"values":["A > B"],"summary":"测试"}"#;
        assert!(parse_inference_response(no_confidence).is_none());
    }

    #[test]
    fn build_inference_prompt_contains_required_context() {
        let conv = "对话「家庭规划」：\n  1. 下周家庭旅行怎么安排";
        let tasks = "1. 「接送孩子」 角色=父亲 象限=Q1 截止=2026-06-30 大石头=是";
        let memories = "1. [preference] 用户偏好周末家庭活动";
        let roles = "1. 🏠 家庭管理者 — 目标：维护家庭和谐";

        let prompt = build_inference_prompt(conv, tasks, memories, roles);
        assert!(prompt.contains("家庭规划"), "包含对话上下文");
        assert!(prompt.contains("接送孩子"), "包含任务上下文");
        assert!(prompt.contains("偏好周末家庭活动"), "包含记忆上下文");
        assert!(prompt.contains("家庭管理者"), "包含角色上下文");
    }

    #[test]
    fn build_inference_prompt_contains_json_format_instruction() {
        let prompt = build_inference_prompt("无", "无", "无", "无");
        assert!(prompt.contains("严格"), "强调 JSON 格式");
        assert!(prompt.contains("\"values\""), "包含 values 字段要求");
        assert!(prompt.contains("\"summary\""), "包含 summary 字段要求");
        assert!(prompt.contains("\"confidence\""), "包含 confidence 字段要求");
        assert!(prompt.contains("第一人称"), "要求 summary 为第一人称使命宣言");
    }

    /// 构造仅含 mission 表的主测试库。
    async fn setup_main_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create main test pool");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS mission (
                id TEXT PRIMARY KEY NOT NULL DEFAULT 'singleton',
                content TEXT,
                format TEXT NOT NULL DEFAULT 'free' CHECK(format IN ('free', 'structured')),
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create mission table");
        pool
    }

    /// 构造仅含 conversations 表的对话测试库。
    async fn setup_conv_pool() -> ConversationsPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create conv test pool");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY NOT NULL,
                role_id TEXT,
                title TEXT NOT NULL DEFAULT '',
                started_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create conversations table");
        ConversationsPool(pool)
    }

    #[tokio::test]
    async fn infer_values_ignores_mission_and_checks_data() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;
        db::mission::upsert_mission(&pool, Some("家庭第一，事业第二"), "free")
            .await
            .expect("failed to set mission");

        // mission 已设定但数据不足，仍应跳过（不再因 mission 跳过）。
        let outcome = infer_values(&pool, &conv_pool).await.expect("should not error");
        assert!(outcome.values.is_none(), "数据不足时不应返回推断值");
        assert_eq!(outcome.source, InferenceSource::SkippedInsufficientData);
    }

    #[tokio::test]
    async fn infer_values_skips_when_conversations_insufficient() {
        let pool = setup_main_pool().await;
        let conv_pool = setup_conv_pool().await;
        // mission 为空（无记录）且对话数为 0 < MIN_CONVERSATIONS。

        let outcome = infer_values(&pool, &conv_pool).await.expect("should not error");
        assert!(outcome.values.is_none(), "数据不足时不应返回推断值");
        assert_eq!(outcome.source, InferenceSource::SkippedInsufficientData);
    }
}
