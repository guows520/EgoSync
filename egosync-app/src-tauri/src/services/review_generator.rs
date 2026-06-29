//! Story 6.4: 周复盘成绩单生成服务 — 收集本周数据，调用 LLM 生成正向叙事复盘
//!
//! 职责：
//! - 计算本周日期范围（周一 ~ 周日），去重检查
//! - 收集各角色能量值、大石头完成情况、新记忆条数、新启用 Skill、任务完成统计
//! - 构造正向叙事风格 prompt，调用默认 LLM provider 生成自然语言复盘摘要
//! - 写入 `weekly_reviews` 表 + 管家对话消息 + "轻触"通知 + emit `review:generated` 事件
//! - 错误只 `tracing::warn!`，不阻塞调度器

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
use crate::models::role::Role;
use crate::models::task::CrossRoleTask;
use chrono::{Datelike, Local, NaiveDate, TimeZone, Utc};
use crate::services::agent_engine;

const LLM_TIMEOUT_SECS: u64 = 60;
const MAX_REVIEW_RESPONSE_BYTES: usize = 128 * 1024;

/// Tauri Event 名称
pub const REVIEW_GENERATED_EVENT: &str = "review:generated";

/// `review:generated` 事件 payload
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewGeneratedPayload {
    pub review_id: String,
    pub week_start: String,
    pub week_end: String,
}

/// 任务完成统计
#[derive(Debug, Clone)]
struct TaskCompletionStats {
    total_tasks: usize,
    completed_tasks: usize,
}

impl TaskCompletionStats {
    fn completion_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            0.0
        } else {
            self.completed_tasks as f64 / self.total_tasks as f64 * 100.0
        }
    }
}

/// 周复盘数据集合
struct ReviewData {
    role_statuses: Vec<DashboardStatus>,
    roles: Vec<Role>,
    bigrock_tasks: Vec<CrossRoleTask>,
    new_memories_count: usize,
    new_skill_names: Vec<String>,
    task_completion_stats: TaskCompletionStats,
}

/// 主入口函数。返回 `true` 表示生成了新复盘，`false` 表示跳过（本周已有）。
///
/// LLM 超时/错误/解析失败 → `tracing::warn!` + 返回 `Ok(false)`（不阻塞调度器）
pub async fn generate_review_if_needed(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    app_handle: Option<&AppHandle>,
) -> Result<bool, AppError> {
    let today = chrono::Local::now().date_naive();
    let week_start = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let week_end = week_start + chrono::Duration::days(6);
    let week_start_str = week_start.format("%Y-%m-%d").to_string();
    let week_end_str = week_end.format("%Y-%m-%d").to_string();

    // 去重检查：本周已有复盘则跳过
    if let Some(existing) =
        db::weekly_reviews::get_weekly_review_by_week_start(pool, &week_start_str).await?
    {
        tracing::info!(
            week_start = %week_start_str,
            review_id = %existing.id,
            "本周复盘已存在，跳过生成"
        );
        return Ok(false);
    }

    // 收集复盘数据
    let data = match collect_review_data(pool, conv_pool, &week_start_str, &week_end_str).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "复盘数据收集失败（降级跳过）");
            return Ok(false);
        }
    };

    // 构造 prompt
    let prompt = build_review_prompt(&data, &week_start_str, &week_end_str);

    // 解析 LLM provider
    let provider = match agent_engine::resolve_default_provider(pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "解析 LLM provider 失败（降级跳过复盘生成）");
            return Ok(false);
        }
    };

    // 调用 LLM
    let content = match call_llm(provider, prompt).await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!(error = %e, "复盘 LLM 调用失败（降级跳过）");
            return Ok(false);
        }
    };

    let content = content.trim();
    if content.is_empty() {
        tracing::warn!("复盘 LLM 返回空内容（降级跳过）");
        return Ok(false);
    }

    // 构造 energy_trends JSON 和 bigrock_status JSON
    let energy_trends = build_energy_trends_json(&data.roles);
    let bigrock_status = build_bigrock_status_json(&data.bigrock_tasks);

    // 写入 weekly_reviews 表
    let review = db::weekly_reviews::create_weekly_review(
        pool,
        &week_start_str,
        &week_end_str,
        content,
        &energy_trends,
        &bigrock_status,
        data.new_memories_count as i64,
    )
    .await?;

    // 写入管家对话消息
    match db::conversations::get_or_create_butler_conversation(conv_pool).await {
        Ok(conv) => {
            if let Err(e) =
                db::conversations::insert_message(conv_pool, &conv.id, "assistant", content, true)
                    .await
            {
                tracing::warn!(
                    error = %e,
                    review_id = %review.id,
                    "复盘写入管家对话消息失败"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                review_id = %review.id,
                "获取/创建管家对话失败"
            );
        }
    }

    // 创建"轻触"通知
    create_review_notification(pool).await;

    // emit Tauri Event
    if let Some(handle) = app_handle {
        let payload = ReviewGeneratedPayload {
            review_id: review.id.clone(),
            week_start: review.week_start.clone(),
            week_end: review.week_end.clone(),
        };
        if let Err(e) = handle.emit(REVIEW_GENERATED_EVENT, &payload) {
            tracing::warn!(error = %e, "emit review:generated 事件失败");
        }
    }

    tracing::info!(
        review_id = %review.id,
        week_start = %review.week_start,
        week_end = %review.week_end,
        "周复盘成绩单已生成"
    );

    Ok(true)
}

/// 收集周复盘数据
async fn collect_review_data(
    pool: &SqlitePool,
    conv_pool: &ConversationsPool,
    week_start: &str,
    week_end: &str,
) -> Result<ReviewData, AppError> {
    // 各角色状态（能量值 + 待处理任务数）
    let role_statuses = crate::services::dashboard_service::get_dashboard_status(pool, conv_pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询角色状态失败（降级空列表）");
            Vec::new()
        });

    // 活跃角色（用于 energy_trends 快照，含 energy_updated_at）
    let roles = db::roles::list_active_roles(pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询活跃角色失败（降级空列表）");
            Vec::new()
        });

    // 大石头任务
    let bigrock_tasks = db::tasks::list_all_tasks(pool, None, Some(true))
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询大石头任务失败（降级空列表）");
            Vec::new()
        });

    // 新沉淀记忆条数：全量查询后在 Rust 侧按 created_at 过滤本周范围
    // memories.created_at 存储为 UTC（strftime(...,'now')）；week_start/week_end 为本地日期，
    // 故先将「本地日的起止时刻」换算为对应 UTC 时刻，再做字符串比较，避免时区边界漏算/多算
    let week_start_utc = local_day_bound_to_utc(week_start, false)
        .unwrap_or_else(|| format!("{}T00:00:00Z", week_start));
    let week_end_utc = local_day_bound_to_utc(week_end, true)
        .unwrap_or_else(|| format!("{}T23:59:59Z", week_end));
    let all_memories = db::memories::list_all_memories_with_options(pool, None, None, None)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询记忆失败（降级空列表）");
            Vec::new()
        });
    let new_memories_count = all_memories
        .iter()
        .filter(|m| m.created_at >= week_start_utc && m.created_at <= week_end_utc)
        .count();

    // 新启用 Skill：查询 skill_role_bindings 表中 created_at 在本周范围内的记录
    let new_skill_names = collect_new_skill_names(pool, &week_start_utc, &week_end_utc).await;

    // 任务完成统计：查询 tasks 表中 completed_at 在本周范围内的已完成任务数 + 总任务数
    let all_tasks = db::tasks::list_all_tasks(pool, None, None)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询全部任务失败（降级空列表）");
            Vec::new()
        });
    let completed_this_week = all_tasks
        .iter()
        .filter(|t| {
            t.is_completed
                && t.completed_at
                    .as_ref()
                    .map(|c| c.as_str() >= week_start_utc.as_str() && c.as_str() <= week_end_utc.as_str())
                    .unwrap_or(false)
        })
        .count();
    let task_completion_stats = TaskCompletionStats {
        total_tasks: all_tasks.len(),
        completed_tasks: completed_this_week,
    };

    Ok(ReviewData {
        role_statuses,
        roles,
        bigrock_tasks,
        new_memories_count,
        new_skill_names,
        task_completion_stats,
    })
}

/// 查询本周新启用的 Skill 名称列表
async fn collect_new_skill_names(
    pool: &SqlitePool,
    week_start_utc: &str,
    week_end_utc: &str,
) -> Vec<String> {
    // 查询 skill_role_bindings 中 created_at 在本周范围内的 skill_id
    let skill_ids: Vec<String> = match sqlx::query_scalar(
        "SELECT DISTINCT skill_id FROM skill_role_bindings WHERE created_at >= ?1 AND created_at <= ?2",
    )
    .bind(week_start_utc)
    .bind(week_end_utc)
    .fetch_all(pool)
    .await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!(error = %e, "查询本周新启用 Skill 绑定失败（降级空列表）");
            return Vec::new();
        }
    };

    // 关联 skills 表获取名称
    let mut names = Vec::new();
    for skill_id in &skill_ids {
        match db::skills::get_skill(pool, skill_id).await {
            Ok(skill) => names.push(skill.name),
            Err(e) => {
                tracing::warn!(
                    skill_id = %skill_id,
                    error = %e,
                    "查询 Skill 详情失败（跳过）"
                );
            }
        }
    }
    names
}

/// 将「本地日期的起止时刻」换算为对应 UTC 时刻字符串（`%Y-%m-%dT%H:%M:%SZ`）
fn local_day_bound_to_utc(date_str: &str, end_of_day: bool) -> Option<String> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    let naive = if end_of_day {
        date.and_hms_opt(23, 59, 59)?
    } else {
        date.and_hms_opt(0, 0, 0)?
    };
    let dt_utc = match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(dt, _) => dt.with_timezone(&Utc),
        chrono::LocalResult::None => Utc.from_utc_datetime(&naive),
    };
    Some(dt_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// 构造 energy_trends JSON：各角色当前能量值快照（含 energy_updated_at，供 Story 6.5 渲染）
fn build_energy_trends_json(roles: &[Role]) -> String {
    let mut map = serde_json::Map::new();
    for role in roles {
        let mut role_map = serde_json::Map::new();
        role_map.insert("energy".to_string(), serde_json::Value::from(role.energy));
        if let Some(ref updated_at) = role.energy_updated_at {
            role_map.insert(
                "energyUpdatedAt".to_string(),
                serde_json::Value::from(updated_at.as_str()),
            );
        }
        map.insert(role.id.clone(), serde_json::Value::Object(role_map));
    }
    serde_json::to_string(&serde_json::Value::Object(map))
        .unwrap_or_else(|_| "{}".to_string())
}

/// 构造 bigrock_status JSON：大石头任务完成状态
fn build_bigrock_status_json(bigrock_tasks: &[CrossRoleTask]) -> String {
    let arr: Vec<serde_json::Value> = bigrock_tasks
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "title": t.title,
                "isCompleted": t.is_completed,
                "completedAt": t.completed_at,
                "roleName": t.role_name,
            })
        })
        .collect();
    serde_json::to_string(&serde_json::Value::Array(arr))
        .unwrap_or_else(|_| "[]".to_string())
}

/// 构造复盘 prompt（正向叙事风格）
pub fn build_review_prompt(
    data: &ReviewData,
    week_start: &str,
    week_end: &str,
) -> Vec<ChatCompletionMessage> {
    let system = "你是 EgoSync 的管家，负责为用户生成每周复盘成绩单。\
请用温暖鼓励的中文语气，生成一段自然语言复盘段落（约 300-500 字），不要使用 JSON 格式或 Markdown 标题。\
\
【叙事风格规则 — 必须严格遵守】\
1. 使用「已完成」（✓）而非「未完成」\
2. 使用「继续推进」（→）而非「失败」或「延期」\
3. 禁止使用「未完成」「失败」「落后」等负面措辞\
4. 用温暖鼓励的语气，强调进步和满足感\
5. 对仍在进行中的大石头用「继续推进」框架描述\
\
复盘应包含以下部分：\
1. 各角色能量值概览（肯定本周的投入）\
2. 大石头完成情况（已完成的用 ✓ 标注，仍在推进的用 → 标注）\
3. 本周新沉淀的记忆（肯定学习与成长）\
4. 新启用的 Skill（如有）\
5. 任务完成统计（完成率 + 肯定努力）\
6. 结尾为下周送上温暖的鼓励\
段落中嵌入角色名称（纯文本即可）。整体风格自然流畅，像管家在跟用户温暖对话。";

    let mut user = format!("[复盘周期]\n{} ~ {}\n", week_start, week_end);

    // 各角色能量值
    if !data.role_statuses.is_empty() {
        user.push_str("\n[各角色能量值]\n");
        for status in &data.role_statuses {
            user.push_str(&format!(
                "- {}：能量 {}\n",
                status.role_name, status.energy
            ));
        }
    }

    // 大石头完成情况
    if !data.bigrock_tasks.is_empty() {
        user.push_str("\n[大石头完成情况]\n");
        for task in &data.bigrock_tasks {
            let role_tag = task
                .role_name
                .as_ref()
                .map(|n| format!("（{}）", n))
                .unwrap_or_default();
            let status_tag = if task.is_completed { "✓ 已完成" } else { "→ 继续推进" };
            user.push_str(&format!(
                "- {}{}：{}\n",
                task.title, role_tag, status_tag
            ));
        }
    }

    // 新记忆条数
    user.push_str(&format!(
        "\n[本周新沉淀记忆]\n{} 条\n",
        data.new_memories_count
    ));

    // 新启用 Skill
    if !data.new_skill_names.is_empty() {
        user.push_str("\n[本周新启用 Skill]\n");
        for name in &data.new_skill_names {
            user.push_str(&format!("- {}\n", name));
        }
    }

    // 任务完成统计
    user.push_str(&format!(
        "\n[任务完成统计]\n总任务 {} 个，已完成 {} 个，完成率 {:.0}%\n",
        data.task_completion_stats.total_tasks,
        data.task_completion_stats.completed_tasks,
        data.task_completion_stats.completion_rate()
    ));

    user.push_str(
        "\n请基于以上信息生成本周复盘成绩单。结尾用温暖的一段话为下周送上鼓励。",
    );

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

/// 调用 LLM 生成复盘（复制自 briefing_generator.rs，调整超时常量）
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
            tracing::warn!(error = %err, "review generation provider stream failed");
        }
    });

    let mut response = String::new();
    let stream_result = timeout(Duration::from_secs(LLM_TIMEOUT_SECS), async {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Token(token) => {
                    if response.len() + token.len() > MAX_REVIEW_RESPONSE_BYTES {
                        tracing::warn!("review generation response exceeded size limit");
                        return Err(AppError::LlmError("复盘生成响应超出大小限制".to_string()));
                    }
                    response.push_str(&token);
                }
                StreamEvent::Done => return Ok::<(), AppError>(()),
                StreamEvent::Error(err) => {
                    tracing::warn!(error = %err, "review generation stream error");
                    return Ok(());
                }
                StreamEvent::Thinking(_) | StreamEvent::ToolCall(_) => {}
            }
        }
        Ok(())
    })
    .await;

    match stream_result {
        Err(_) => {
            stream_handle.abort();
            return Err(AppError::LlmError("复盘生成 LLM 调用超时".to_string()));
        }
        Ok(Err(e)) => return Err(e),
        Ok(Ok(())) => {}
    }

    Ok(response)
}

/// 使用第一个活跃角色的 ID 创建"轻触"通知。
/// 若没有活跃角色则跳过通知创建。
async fn create_review_notification(pool: &SqlitePool) {
    let roles = match db::roles::list_active_roles(pool).await {
        Ok(roles) => roles,
        Err(e) => {
            tracing::warn!(error = %e, "查询活跃角色失败，跳过通知创建");
            return;
        }
    };

    let Some(first_role) = roles.first() else {
        tracing::info!("无活跃角色，跳过通知创建，只写入管家对话消息");
        return;
    };

    match crate::services::notification_service::create_notification_for_role(
        pool,
        &first_role.id,
        crate::services::suggestion_generator::NotificationLevel::Tap,
        "本周复盘已准备好",
    )
    .await
    {
        Ok(notification) => {
            tracing::info!(
                role_id = %first_role.id,
                notification_id = %notification.id,
                level = %notification.level,
                "周复盘通知已创建"
            );
        }
        Err(e) => {
            tracing::warn!(
                role_id = %first_role.id,
                error = %e,
                "周复盘通知创建失败（继续写入对话消息）"
            );
        }
    }
}

/// Story 6.5: AI 大石头建议返回结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleBigRockSuggestions {
    pub role_id: String,
    pub role_name: String,
    pub suggestions: Vec<String>,
}

/// Story 6.5: 为每个活跃角色生成 1-2 个大石头建议。
/// LLM 失败/超时/解析失败 → 返回空列表（前端降级为手动输入）
pub async fn generate_bigrock_suggestions(
    pool: &SqlitePool,
) -> Result<Vec<RoleBigRockSuggestions>, AppError> {
    let roles = db::roles::list_active_roles(pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询活跃角色失败（降级空列表）");
            Vec::new()
        });

    if roles.is_empty() {
        return Ok(Vec::new());
    }

    let bigrock_tasks = db::tasks::list_all_tasks(pool, None, Some(true))
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "查询大石头任务失败（降级空列表）");
            Vec::new()
        });
    let incomplete_bigrocks: Vec<&CrossRoleTask> =
        bigrock_tasks.iter().filter(|t| !t.is_completed).collect();

    let prompt = build_suggestion_prompt(&roles, &incomplete_bigrocks);

    let provider = match agent_engine::resolve_default_provider(pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "解析 LLM provider 失败（大石头建议降级为空列表）");
            return Ok(Vec::new());
        }
    };

    let content = match call_llm(provider, prompt).await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!(error = %e, "大石头建议 LLM 调用失败（降级为空列表）");
            return Ok(Vec::new());
        }
    };

    parse_suggestions(&content, &roles)
}

/// Story 6.5: 构造大石头建议 prompt
pub fn build_suggestion_prompt(
    roles: &[Role],
    incomplete_bigrocks: &[&CrossRoleTask],
) -> Vec<ChatCompletionMessage> {
    let system = "你是 EgoSync 的管家，负责为用户下周的大石头规划提供建议。\
请根据每个角色的目标、当前能量值和本周未完成的大石头，为每个角色建议 1-2 个下周大石头。\
\
【输出格式规则 — 必须严格遵守】\
必须返回合法 JSON 数组，格式如下：\
[{\"roleId\": \"角色ID\", \"suggestions\": [\"建议1\", \"建议2\"]}]\
\
【建议规则】\
1. 每个角色建议 1-2 个大石头\
2. 建议应基于角色目标，是具体可执行的任务\
3. 考虑角色当前能量值 — 能量低时建议聚焦核心目标\
4. 考虑本周未完成的大石头 — 可建议继续推进\
5. 建议用中文，简洁明了，每个不超过 20 字\
6. 只返回 JSON，不要附加任何其他文字";

    let mut user = String::new();

    user.push_str("[角色列表]\n");
    for role in roles {
        user.push_str(&format!(
            "- ID: {} | 名称: {} | 目标: {} | 能量值: {}\n",
            role.id, role.name, role.goal, role.energy
        ));
    }

    if !incomplete_bigrocks.is_empty() {
        user.push_str("\n[本周未完成大石头]\n");
        for task in incomplete_bigrocks {
            let role_tag = task
                .role_name
                .as_ref()
                .map(|n| format!("（{}）", n))
                .unwrap_or_default();
            user.push_str(&format!("- {}{}\n", task.title, role_tag));
        }
    }

    user.push_str("\n请为以上每个角色生成下周大石头建议，返回 JSON 数组。");

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

/// Story 6.5: 解析 LLM 返回的大石头建议 JSON
fn parse_suggestions(
    content: &str,
    roles: &[Role],
) -> Result<Vec<RoleBigRockSuggestions>, AppError> {
    let trimmed = content.trim();
    let parsed: Vec<serde_json::Value> = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "大石头建议 JSON 解析失败（降级为空列表）");
            return Ok(Vec::new());
        }
    };

    let role_map: std::collections::HashMap<String, &Role> =
        roles.iter().map(|r| (r.id.clone(), r)).collect();

    let mut result = Vec::new();
    for item in &parsed {
        let role_id = match item.get("roleId").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        let role = match role_map.get(&role_id) {
            Some(r) => *r,
            None => continue,
        };
        let suggestions: Vec<String> = item
            .get("suggestions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        result.push(RoleBigRockSuggestions {
            role_id: role.id.clone(),
            role_name: role.name.clone(),
            suggestions,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dashboard::DashboardStatus;
    use crate::models::task::CrossRoleTask;

    fn make_dashboard_status(id: &str, name: &str, energy: i32) -> DashboardStatus {
        DashboardStatus {
            role_id: id.to_string(),
            role_name: name.to_string(),
            role_icon: "🎯".to_string(),
            role_color: "#6366F1".to_string(),
            energy,
            pending_tasks_count: 3,
            last_active_at: None,
            has_urgent: false,
        }
    }

    fn make_cross_role_task(
        id: &str,
        title: &str,
        is_completed: bool,
        role_name: Option<&str>,
    ) -> CrossRoleTask {
        CrossRoleTask {
            id: id.to_string(),
            owner_type: "role".to_string(),
            role_id: Some("r1".to_string()),
            title: title.to_string(),
            deadline: None,
            quadrant: "Q1".to_string(),
            is_big_rock: true,
            is_completed,
            completed_at: if is_completed {
                Some("2026-06-25T10:00:00Z".to_string())
            } else {
                None
            },
            sort_order: 0,
            protection_status: "normal".to_string(),
            confidence: None,
            manual_override: false,
            classification_reason: None,
            created_at: "2026-06-01T00:00:00Z".to_string(),
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            deleted_at: None,
            role_name: role_name.map(|s| s.to_string()),
            role_color: None,
        }
    }

    fn make_review_data(
        roles: Vec<DashboardStatus>,
        bigrocks: Vec<CrossRoleTask>,
        memories: usize,
        skills: Vec<String>,
        total: usize,
        completed: usize,
    ) -> ReviewData {
        ReviewData {
            role_statuses: roles,
            roles: Vec::new(),
            bigrock_tasks: bigrocks,
            new_memories_count: memories,
            new_skill_names: skills,
            task_completion_stats: TaskCompletionStats {
                total_tasks: total,
                completed_tasks: completed,
            },
        }
    }

    #[test]
    fn build_review_prompt_contains_role_energy() {
        let data = make_review_data(
            vec![make_dashboard_status("r1", "产品经理", 85)],
            vec![],
            0,
            vec![],
            0,
            0,
        );
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert!(prompt[1].content.contains("产品经理"));
        assert!(prompt[1].content.contains("85"));
    }

    #[test]
    fn build_review_prompt_contains_bigrock_status() {
        let data = make_review_data(
            vec![],
            vec![
                make_cross_role_task("t1", "竞品分析", true, Some("产品经理")),
                make_cross_role_task("t2", "用户调研", false, Some("产品经理")),
            ],
            0,
            vec![],
            0,
            0,
        );
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert!(prompt[1].content.contains("竞品分析"));
        assert!(prompt[1].content.contains("✓ 已完成"));
        assert!(prompt[1].content.contains("用户调研"));
        assert!(prompt[1].content.contains("→ 继续推进"));
    }

    #[test]
    fn build_review_prompt_contains_memory_count() {
        let data = make_review_data(vec![], vec![], 7, vec![], 0, 0);
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert!(prompt[1].content.contains("7"));
    }

    #[test]
    fn build_review_prompt_system_contains_positive_narrative() {
        let data = make_review_data(vec![], vec![], 0, vec![], 0, 0);
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert_eq!(prompt[0].role, "system");
        assert!(prompt[0].content.contains("已完成"));
        assert!(prompt[0].content.contains("继续推进"));
        assert!(prompt[0].content.contains("禁止"));
        assert!(prompt[0].content.contains("失败"));
    }

    #[test]
    fn build_review_prompt_contains_date_range() {
        let data = make_review_data(vec![], vec![], 0, vec![], 0, 0);
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert!(prompt[1].content.contains("2026-06-22"));
        assert!(prompt[1].content.contains("2026-06-28"));
    }

    #[test]
    fn build_review_prompt_contains_task_stats() {
        let data = make_review_data(vec![], vec![], 0, vec![], 10, 6);
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert!(prompt[1].content.contains("10"));
        assert!(prompt[1].content.contains("6"));
        assert!(prompt[1].content.contains("60%"));
    }

    #[test]
    fn build_review_prompt_contains_new_skills() {
        let data = make_review_data(vec![], vec![], 0, vec!["代码审查".to_string()], 0, 0);
        let prompt = build_review_prompt(&data, "2026-06-22", "2026-06-28");
        assert!(prompt[1].content.contains("代码审查"));
    }

    #[test]
    fn review_generated_event_name_is_correct() {
        assert_eq!(REVIEW_GENERATED_EVENT, "review:generated");
    }

    #[test]
    fn payload_serializes_with_camel_case() {
        let payload = ReviewGeneratedPayload {
            review_id: "abc-123".to_string(),
            week_start: "2026-06-22".to_string(),
            week_end: "2026-06-28".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("reviewId"));
        assert!(json.contains("weekStart"));
        assert!(json.contains("weekEnd"));
    }

    fn make_role(id: &str, energy: i32, energy_updated_at: Option<&str>) -> Role {
        Role {
            id: id.to_string(),
            name: format!("role-{}", id),
            icon: "🎯".to_string(),
            color: "#6366F1".to_string(),
            goal: String::new(),
            personality_prompt: String::new(),
            status: "active".to_string(),
            energy,
            energy_updated_at: energy_updated_at.map(|s| s.to_string()),
            skills_config: "{}".to_string(),
            proactivity_level: "balanced".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn build_energy_trends_json_correct_format() {
        let roles = vec![
            make_role("r1", 85, Some("2026-06-20T08:00:00Z")),
            make_role("r2", 60, None),
        ];
        let json = build_energy_trends_json(&roles);
        assert!(json.contains("r1"));
        assert!(json.contains("r2"));
        assert!(json.contains("85"));
        assert!(json.contains("60"));
        assert!(json.contains("energyUpdatedAt"));
        assert!(json.contains("2026-06-20T08:00:00Z"));
    }

    #[test]
    fn build_bigrock_status_json_correct_format() {
        let tasks = vec![
            make_cross_role_task("t1", "竞品分析", true, Some("产品经理")),
            make_cross_role_task("t2", "用户调研", false, None),
        ];
        let json = build_bigrock_status_json(&tasks);
        assert!(json.contains("竞品分析"));
        assert!(json.contains("用户调研"));
        assert!(json.contains("isCompleted"));
    }

    #[test]
    fn task_completion_stats_rate() {
        let stats = TaskCompletionStats {
            total_tasks: 10,
            completed_tasks: 6,
        };
        assert_eq!(stats.completion_rate(), 60.0);
    }

    #[test]
    fn task_completion_stats_zero_division() {
        let stats = TaskCompletionStats {
            total_tasks: 0,
            completed_tasks: 0,
        };
        assert_eq!(stats.completion_rate(), 0.0);
    }

    // ===== Story 6.5 tests =====

    #[test]
    fn build_suggestion_prompt_contains_role_name_goal_energy() {
        let roles = vec![make_role("r1", 85, None)];
        let prompt = build_suggestion_prompt(&roles, &[]);
        assert!(prompt[1].content.contains("role-r1"));
        assert!(prompt[1].content.contains("85"));
        assert!(prompt[0].content.contains("JSON"));
    }

    #[test]
    fn build_suggestion_prompt_contains_incomplete_bigrocks() {
        let roles = vec![make_role("r1", 80, None)];
        let task = make_cross_role_task("t1", "竞品分析", false, Some("产品经理"));
        let task_ref: &CrossRoleTask = &task;
        let prompt = build_suggestion_prompt(&roles, &[task_ref]);
        assert!(prompt[1].content.contains("竞品分析"));
        assert!(prompt[1].content.contains("未完成"));
    }

    #[test]
    fn parse_suggestions_valid_json() {
        let roles = vec![
            make_role("r1", 80, None),
            make_role("r2", 60, None),
        ];
        let content = r#"[{"roleId":"r1","suggestions":["Q3路线图定稿","竞品分析"]},{"roleId":"r2","suggestions":["周末陪孩子"]}]"#;
        let result = parse_suggestions(content, &roles).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role_id, "r1");
        assert_eq!(result[0].suggestions.len(), 2);
        assert_eq!(result[0].suggestions[0], "Q3路线图定稿");
        assert_eq!(result[1].role_id, "r2");
        assert_eq!(result[1].suggestions[0], "周末陪孩子");
    }

    #[test]
    fn parse_suggestions_invalid_json_returns_empty() {
        let roles = vec![make_role("r1", 80, None)];
        let result = parse_suggestions("not valid json", &roles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_suggestions_unknown_role_id_skipped() {
        let roles = vec![make_role("r1", 80, None)];
        let content = r#"[{"roleId":"unknown-id","suggestions":["test"]}]"#;
        let result = parse_suggestions(content, &roles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn role_bigrock_suggestions_serializes_camel_case() {
        let s = RoleBigRockSuggestions {
            role_id: "r1".to_string(),
            role_name: "产品经理".to_string(),
            suggestions: vec!["建议1".to_string()],
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("roleId"));
        assert!(json.contains("roleName"));
        assert!(json.contains("suggestions"));
    }
}
