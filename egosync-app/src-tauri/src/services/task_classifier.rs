//! Story 3.3: 自动四象限分类服务
//!
//! 职责：
//! - 读取 task / role / 同角色历史任务，构造分类 prompt
//! - 调用默认 LLM provider（复用 services::llm_config + secret_store）
//! - 解析 LLM 返回的 JSON `{ "quadrant", "confidence", "reason" }`
//! - 解析失败/超时/缺少配置 → 降级写入 Q2（confidence < 0.8 + 中文 reason）
//! - 全程不修改 sort_order / is_completed / completed_at / manual_override
//!
//! Command 层不应直接调 LLM、解析 JSON 或写 SQL；本模块是唯一入口。

use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::db;
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::{ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent};
use crate::models::role::Role;
use crate::models::task::Task;
use crate::services::secret_store;

/// LLM 调用超时上限（秒）。failure 后降级到 Q2，不阻塞用户创建任务。
const LLM_TIMEOUT_SECS: u64 = 12;

/// 分类置信度阈值。低于此值的任务在前端显示「不确定」标记。
pub const CONFIDENCE_UNCERTAINTY_THRESHOLD: f64 = 0.8;

/// 降级写入的默认 quadrant 与 confidence。
const FALLBACK_QUADRANT: &str = "Q2";
const FALLBACK_CONFIDENCE: f64 = 0.3;

/// 同角色历史任务样本数量上限（不含当前任务）。
const HISTORY_SAMPLE_LIMIT: u32 = 5;

/// 临期阈值：deadline 距今 <= 2 天的任务自动升入 Q1。
pub const IMMINENT_DAYS: i64 = 2;

/// 分类来源：用于日志与单测断言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationSource {
    Llm,
    FallbackNoConfig,
    FallbackLlmError,
    FallbackParseError,
    FallbackTimeout,
}

#[derive(Debug, Clone)]
pub struct ClassificationOutcome {
    pub quadrant: String,
    pub confidence: f64,
    pub reason: String,
    pub source: ClassificationSource,
}

/// 对单个任务执行自动分类并写回 DB。
///
/// 调用链路：
/// 1. 若任务 `manual_override = true`，直接跳过（返回当前任务），不调用 LLM 也不写库。
/// 2. 否则读取 role + 同角色历史任务 → 构造 prompt → 调用默认 LLM 获取分类。
/// 3. LLM 失败/超时/JSON 非法 → 降级到 Q2 + 中文 reason，仍写入 DB。
/// 4. 始终通过 `db::tasks::update_task_classification` 写入，确保不修改 sort_order 等字段。
pub async fn classify_and_persist(pool: &SqlitePool, task_id: &str) -> Result<Task, AppError> {
    let task = db::tasks::get_active_task_pub(pool, task_id).await?;
    if task.manual_override {
        // 用户已显式选择，不调用 LLM 也不写库。
        return Ok(task);
    }

    let outcome = classify_task(pool, &task).await;
    log_outcome(&task, &outcome);

    db::tasks::update_task_classification(
        pool,
        &task.id,
        &outcome.quadrant,
        outcome.confidence,
        &outcome.reason,
    )
    .await
}

/// 仅做分类（不写库），便于单测与上层组合。
pub async fn classify_task(pool: &SqlitePool, task: &Task) -> ClassificationOutcome {
    let role = match db::roles::get_role(pool, &task.role_id).await {
        Ok(role) => role,
        Err(e) => {
            tracing::warn!(task_id = %task.id, error = %e, "读取角色失败，分类降级到 Q2");
            return fallback_outcome(
                ClassificationSource::FallbackNoConfig,
                "未能读取角色信息，先放入 Q2 等待重新判断",
            );
        }
    };

    let recent = db::tasks::list_recent_tasks_by_role(pool, &task.role_id, &task.id, HISTORY_SAMPLE_LIMIT)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(task_id = %task.id, error = %e, "读取同角色历史任务失败，使用空列表继续");
            Vec::new()
        });

    let today = current_date();
    let prompt = build_classification_prompt(task, &role, &recent, &today);

    let provider = match build_default_provider(pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(task_id = %task.id, error = %e, "未能加载默认 LLM 配置，分类降级到 Q2");
            return fallback_outcome(
                ClassificationSource::FallbackNoConfig,
                "未配置默认 LLM，先放入 Q2 等待人工确认",
            );
        }
    };

    let raw = match call_llm_with_timeout(provider.as_ref(), &prompt).await {
        Ok(text) => text,
        Err(LlmCallError::Timeout) => {
            return fallback_outcome(
                ClassificationSource::FallbackTimeout,
                "LLM 分析超时，先放入 Q2",
            );
        }
        Err(LlmCallError::Provider(e)) => {
            tracing::warn!(task_id = %task.id, error = %e, "LLM 调用失败，分类降级到 Q2");
            return fallback_outcome(
                ClassificationSource::FallbackLlmError,
                "LLM 暂时不可用，先放入 Q2",
            );
        }
    };

    match parse_classification_response(&raw) {
        Some(parsed) => ClassificationOutcome {
            quadrant: parsed.quadrant,
            confidence: parsed.confidence,
            reason: parsed.reason,
            source: ClassificationSource::Llm,
        },
        None => {
            tracing::warn!(task_id = %task.id, raw = %truncate(&raw, 200), "LLM 返回无法解析为分类 JSON，降级到 Q2");
            fallback_outcome(
                ClassificationSource::FallbackParseError,
                "LLM 返回格式不符，先放入 Q2 等待人工确认",
            )
        }
    }
}

fn fallback_outcome(source: ClassificationSource, reason: &str) -> ClassificationOutcome {
    ClassificationOutcome {
        quadrant: FALLBACK_QUADRANT.to_string(),
        confidence: FALLBACK_CONFIDENCE,
        reason: reason.to_string(),
        source,
    }
}

fn log_outcome(task: &Task, outcome: &ClassificationOutcome) {
    tracing::info!(
        task_id = %task.id,
        role_id = %task.role_id,
        quadrant = %outcome.quadrant,
        confidence = outcome.confidence,
        source = ?outcome.source,
        "任务分类完成"
    );
}

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
        tool_calls: None,
        tool_call_id: None,
    }];
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let options = ChatOptions {
        disable_thinking: true,
        ..ChatOptions::default()
    };

    // Spawn a background task to collect stream tokens into a buffer.
    // This decouples the provider future from the buffer collection so
    // we can apply a timeout to the combined operation.
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
        // Run the provider to completion; when it finishes (or errors),
        // the `tx` sender is dropped, causing the collector to see channel
        // closure and return whatever it accumulated.
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

    let provider: Box<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Box::new(AnthropicProvider::new(config.base_url, api_key, config.model)?),
        _ => Box::new(OpenAiProvider::new(config.base_url, api_key, config.model)?),
    };
    Ok(provider)
}

/// 当前日期（UTC，`YYYY-MM-DD`），与全应用 `chrono_now` 的 UTC 约定一致。
/// 作为 prompt 的时间锚点，使 LLM 能据「今天日期」与「截止日期」推断距今天数。
fn current_date() -> String {
    crate::db::settings::chrono_now_pub()
        .get(..10)
        .unwrap_or("")
        .to_string()
}

/// 构造分类 prompt。
///
/// 输入：今天日期 + 任务标题 + 截止日期 + 角色目标 + 最近 5 条同角色任务标题/quadrant。
/// `today` 提供时间锚点，支撑 AC1「截止日期距今天数」的紧迫度判断。
/// 输出要求：严格 JSON 格式 `{ "quadrant", "confidence", "reason" }`，中文 reason。
pub fn build_classification_prompt(task: &Task, role: &Role, recent: &[Task], today: &str) -> String {
    let recent_block = if recent.is_empty() {
        "（暂无）".to_string()
    } else {
        recent
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let deadline = t.deadline.as_deref().unwrap_or("无");
                format!(
                    "{}. 「{}」 quadrant={} deadline={}",
                    i + 1,
                    t.title,
                    t.quadrant,
                    deadline
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let goal = if role.goal.trim().is_empty() {
        "（未设定）".to_string()
    } else {
        role.goal.clone()
    };

    format!(
        "你是 EgoSync 的任务分类助手，负责按 Eisenhower 四象限分类一项任务。\n\n\
今天日期：{today}\n\n\
任务信息：\n\
- 标题：{title}\n\
- 截止日期：{deadline}\n\n\
角色目标：{goal}\n\n\
该角色最近的任务（用于推断模式）：\n{recent}\n\n\
四象限定义：\n\
- Q1 重要且紧急：影响角色目标且短期必须完成。\n\
- Q2 重要不紧急：影响角色目标但时间充裕，长线投资。\n\
- Q3 紧急不重要：时间敏感但与角色目标关联较弱。\n\
- Q4 不重要不紧急：可延后或舍弃。\n\n\
请综合「截止日期距今天数」「与角色目标关联度」「最近任务模式」给出分类。\n\n\
**严格只输出一段 JSON**，不要包裹在代码块里，结构如下：\n\
{{\"quadrant\": \"Q1|Q2|Q3|Q4\", \"confidence\": 0.0-1.0, \"reason\": \"中文短句，<=40字\"}}",
        today = today,
        title = task.title,
        deadline = task.deadline.as_deref().unwrap_or("未设定"),
        goal = goal,
        recent = recent_block,
    )
}

struct ParsedClassification {
    quadrant: String,
    confidence: f64,
    reason: String,
}

/// 解析 LLM 返回文本为 `{ quadrant, confidence, reason }`。
///
/// 容忍 LLM 在 JSON 前后输出额外文字（提取第一个 `{` 到最后一个 `}` 的子串再尝试 parse）。
/// 任何不合法（非法 quadrant、confidence 越界、缺字段）都返回 None，由调用方降级。
pub fn parse_classification_response(raw: &str) -> Option<ParsedClassification> {
    let snippet = extract_json_object(raw)?;
    let value: serde_json::Value = serde_json::from_str(&snippet).ok()?;
    let quadrant = value.get("quadrant")?.as_str()?.trim().to_string();
    if !matches!(quadrant.as_str(), "Q1" | "Q2" | "Q3" | "Q4") {
        return None;
    }
    let confidence_raw = value.get("confidence")?.as_f64()?;
    if !(0.0..=1.0).contains(&confidence_raw) {
        return None;
    }
    let reason = value
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let reason = if reason.is_empty() {
        "LLM 未提供分类原因".to_string()
    } else {
        reason
    };
    Some(ParsedClassification {
        quadrant,
        confidence: confidence_raw,
        reason,
    })
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

    fn sample_task(id: &str, title: &str, deadline: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            role_id: "role-a".to_string(),
            title: title.to_string(),
            deadline: deadline.map(|s| s.to_string()),
            quadrant: "Q2".to_string(),
            is_big_rock: false,
            is_completed: false,
            completed_at: None,
            sort_order: 0,
            protection_status: "normal".to_string(),
            confidence: None,
            manual_override: false,
            classification_reason: None,
            created_at: "2026-06-17T00:00:00Z".to_string(),
            updated_at: "2026-06-17T00:00:00Z".to_string(),
            deleted_at: None,
        }
    }

    fn sample_role() -> Role {
        Role {
            id: "role-a".to_string(),
            name: "产品经理".to_string(),
            icon: "📋".to_string(),
            color: "#4F46E5".to_string(),
            goal: "管理产品规划".to_string(),
            personality_prompt: String::new(),
            status: "active".to_string(),
            energy: 100,
            skills_config: "{}".to_string(),
            proactivity_level: "moderate".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn parse_classification_accepts_clean_json() {
        let raw = r#"{"quadrant":"Q1","confidence":0.92,"reason":"截止日期临近"}"#;
        let parsed = parse_classification_response(raw).expect("should parse");
        assert_eq!(parsed.quadrant, "Q1");
        assert!((parsed.confidence - 0.92).abs() < 1e-9);
        assert_eq!(parsed.reason, "截止日期临近");
    }

    #[test]
    fn parse_classification_strips_surrounding_prose() {
        let raw = "Sure! Here is the classification:\n```json\n{\"quadrant\":\"Q2\",\"confidence\":0.8,\"reason\":\"长期投资\"}\n```";
        let parsed = parse_classification_response(raw).expect("should parse despite wrappers");
        assert_eq!(parsed.quadrant, "Q2");
    }

    #[test]
    fn parse_classification_rejects_invalid_quadrant() {
        let raw = r#"{"quadrant":"Q5","confidence":0.5,"reason":"x"}"#;
        assert!(parse_classification_response(raw).is_none());
    }

    #[test]
    fn parse_classification_rejects_out_of_range_confidence() {
        let raw = r#"{"quadrant":"Q1","confidence":1.5,"reason":"x"}"#;
        assert!(parse_classification_response(raw).is_none());
    }

    #[test]
    fn parse_classification_rejects_missing_field() {
        let raw = r#"{"confidence":0.8,"reason":"x"}"#;
        assert!(parse_classification_response(raw).is_none());
    }

    #[test]
    fn parse_classification_handles_empty_reason() {
        let raw = r#"{"quadrant":"Q3","confidence":0.5,"reason":""}"#;
        let parsed = parse_classification_response(raw).expect("should parse");
        assert!(!parsed.reason.is_empty(), "应填充默认中文 reason");
    }

    #[test]
    fn build_prompt_contains_required_context() {
        let task = sample_task("t-1", "准备季度规划", Some("2026-06-30"));
        let role = sample_role();
        let recent = vec![sample_task("t-2", "上周复盘", None)];

        let prompt = build_classification_prompt(&task, &role, &recent, "2026-06-18");
        assert!(prompt.contains("准备季度规划"), "包含任务标题");
        assert!(prompt.contains("2026-06-30"), "包含截止日期");
        assert!(prompt.contains("2026-06-18"), "包含今天日期作为时间锚点");
        assert!(prompt.contains("管理产品规划"), "包含角色目标");
        assert!(prompt.contains("上周复盘"), "包含历史任务");
        assert!(prompt.contains("Q1") && prompt.contains("Q4"), "包含四象限定义");
        assert!(prompt.contains("严格"), "强调 JSON 格式");
    }

    #[test]
    fn fallback_outcome_uses_q2_with_low_confidence() {
        let outcome = fallback_outcome(
            ClassificationSource::FallbackTimeout,
            "LLM 超时",
        );
        assert_eq!(outcome.quadrant, "Q2");
        assert!(outcome.confidence < CONFIDENCE_UNCERTAINTY_THRESHOLD);
        assert_eq!(outcome.source, ClassificationSource::FallbackTimeout);
    }
}
