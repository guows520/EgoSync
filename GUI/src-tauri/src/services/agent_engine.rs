use std::sync::Arc;

use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::db::{conversations, memories};
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::{
    ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent, ToolCall, ToolDefinition,
};
use crate::models::chat::{RoleProposedPayload, StreamPayload};
use crate::models::role::CreateRoleInput;
use crate::services::secret_store;

const OPENCODE_FALLBACK_NOTICE: &str = "Agent 引擎暂时不可用，当前为基础对话模式。\n\n";

/// Resolve the opencode project directory to a stable path outside the dev
/// project tree. Using `"."` previously caused opencode to write `.opencode/`
/// session files into the tauri/vite source directory, triggering cargo-watch
/// or vite HMR full-page reloads during active sessions.
fn resolve_opencode_project_dir(app_handle: &tauri::AppHandle) -> String {
    use tauri::Manager;
    if let Ok(dir) = app_handle.path().app_data_dir() {
        let workspace = dir.join("opencode-workspace");
        let _ = std::fs::create_dir_all(&workspace);
        workspace.to_string_lossy().to_string()
    } else {
        // Fallback: use user home to avoid polluting source tree
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".egosync-workspace")
            .to_string_lossy()
            .to_string()
    }
}

fn opencode_agent_key(role_id: Option<&str>) -> String {
    match role_id {
        Some(id) => crate::services::agent_config::AgentConfigService::role_to_agent_key(id),
        None => "butler".to_string(), // butler agent has egosync* tools enabled
    }
}

fn remember_opencode_session(
    sessions: &mut std::collections::HashMap<String, String>,
    conversation_id: &str,
    session_id: &str,
) -> String {
    sessions
        .entry(conversation_id.to_string())
        .or_insert_with(|| session_id.to_string())
        .clone()
}

fn stream_payload_from_sse(
    conversation_id: &str,
    message_id: Option<&str>,
    event: crate::models::agent::SseEvent,
) -> Option<StreamPayload> {
    match event {
        crate::models::agent::SseEvent::Text { content } => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: content,
            done: false,
            thinking: false,
            message_id: message_id.map(str::to_string),
        }),
        crate::models::agent::SseEvent::Thinking { content } => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: content,
            done: false,
            thinking: true,
            message_id: message_id.map(str::to_string),
        }),
        crate::models::agent::SseEvent::Done => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: String::new(),
            done: true,
            thinking: false,
            message_id: message_id.map(str::to_string),
        }),
        crate::models::agent::SseEvent::Error { message } => Some(StreamPayload {
            conversation_id: conversation_id.to_string(),
            token: format!("抱歉，Agent 引擎返回错误：{}", summarize_error(&message)),
            done: true,
            thinking: false,
            message_id: message_id.map(str::to_string),
        }),
        crate::models::agent::SseEvent::ToolCall { name, arguments } => {
            tracing::info!(
                "[opencode] tool_call received: name={} args_len={}",
                name,
                arguments.len()
            );
            None
        }
    }
}

const BUTLER_SYSTEM_PROMPT: &str = "\
你是 EgoSync 的数字管家，用户的私人助理和生活协调者。\
你的语调稳重、可靠、有温度，像一位值得信赖的英式管家。\
你帮助用户管理角色、任务和日程，但决定权永远在用户手中。\
用简洁自然的中文回复，不用 emoji。";

const HISTORY_LIMIT: i64 = 20;

/// Story 2.3 AC-7: 跨角色全局视野。
/// 用户可能在角色 X 视图私聊后回管家，管家 system prompt 必须自带各 active 角色近况摘要，
/// 否则管家会"失忆"——它只看到自己 conversation 里的内容，看不到角色私聊。
/// 摘要为纯文本注入 prompt，对用户不可见。无 active 角色或全部无历史时返回空串。
const CROSS_ROLE_SUMMARY_PER_ROLE: i64 = 4;
const CROSS_ROLE_SUMMARY_PER_LINE_CHARS: usize = 40;
const CROSS_ROLE_SUMMARY_TOTAL_CHARS: usize = 2000;
const BUTLER_MEMORY_PER_ROLE: usize = 6;
const BUTLER_MEMORY_PER_LINE_CHARS: usize = 90;
const BUTLER_MEMORY_TOTAL_CHARS: usize = 3000;

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

pub async fn build_butler_memory_summary(main_pool: &DbPool) -> Result<String, AppError> {
    let all_memories = memories::list_all_memories(main_pool).await?;
    if all_memories.is_empty() {
        return Ok(String::new());
    }

    let roles = crate::db::roles::list_active_roles(main_pool).await?;
    let role_names = roles
        .into_iter()
        .map(|role| (role.id, role.name))
        .collect::<std::collections::HashMap<_, _>>();

    let mut global_lines = Vec::new();
    let mut role_lines = std::collections::BTreeMap::<String, Vec<String>>::new();
    for memory in all_memories {
        let line = format!(
            "- [{}] {}",
            memory.category,
            truncate_chars(memory.content.trim(), BUTLER_MEMORY_PER_LINE_CHARS)
        );
        if let Some(role_id) = memory.role_id {
            let role_name = role_names
                .get(&role_id)
                .cloned()
                .unwrap_or_else(|| role_id.clone());
            let lines = role_lines.entry(role_name).or_default();
            if lines.len() < BUTLER_MEMORY_PER_ROLE {
                lines.push(line);
            }
        } else if global_lines.len() < BUTLER_MEMORY_PER_ROLE {
            global_lines.push(line);
        }
    }

    let mut sections = Vec::new();
    if !global_lines.is_empty() {
        sections.push(format!("全局记忆：\n{}", global_lines.join("\n")));
    }
    for (role_name, lines) in role_lines {
        if !lines.is_empty() {
            sections.push(format!("{}：\n{}", role_name, lines.join("\n")));
        }
    }
    if sections.is_empty() {
        return Ok(String::new());
    }

    let mut summary = format!("[已知记忆]\n{}", sections.join("\n"));
    if summary.chars().count() > BUTLER_MEMORY_TOTAL_CHARS {
        summary = truncate_chars(&summary, BUTLER_MEMORY_TOTAL_CHARS) + "…";
    }
    Ok(summary)
}

pub async fn build_cross_role_summary(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
) -> Result<String, AppError> {
    let roles = crate::db::roles::list_active_roles(main_pool).await?;
    if roles.is_empty() {
        return Ok(String::new());
    }

    let mut blocks: Vec<String> = Vec::new();
    for role in &roles {
        // 找该角色最近一条 conversation；不存在直接跳过（不要 get_or_create，避免无意义建空会话）
        let convs =
            crate::db::conversations::list_conversations_by_role(conv_pool, &role.id).await?;
        let Some(conv) = convs.first() else { continue };
        let recent = crate::db::conversations::get_recent_messages(
            conv_pool,
            &conv.id,
            CROSS_ROLE_SUMMARY_PER_ROLE,
        )
        .await?;
        if recent.is_empty() {
            continue;
        }

        let mut lines: Vec<String> = Vec::with_capacity(recent.len() + 1);
        lines.push(format!("- {}：", role.name));
        for m in &recent {
            let speaker = match m.role.as_str() {
                "user" => "用户",
                "assistant" => "角色",
                _ => continue, // system 类内部消息（如 onboarding 锚点）不进摘要
            };
            let snippet = truncate_chars(&m.content, CROSS_ROLE_SUMMARY_PER_LINE_CHARS);
            lines.push(format!("  {}: {}", speaker, snippet));
        }
        blocks.push(lines.join("\n"));
    }

    if blocks.is_empty() {
        return Ok(String::new());
    }

    let mut joined = format!("[各角色近况]\n{}", blocks.join("\n"));
    if joined.chars().count() > CROSS_ROLE_SUMMARY_TOTAL_CHARS {
        joined = joined
            .chars()
            .take(CROSS_ROLE_SUMMARY_TOTAL_CHARS)
            .collect::<String>()
            + "…";
    }
    Ok(joined)
}

/// Build the full butler system prompt: baseline identity + role roster + cross-role summary
/// + delegation guidelines + emergence instructions. Used by both the direct LLM path and
/// the opencode path to ensure the butler persona is consistent regardless of backend.
pub async fn build_butler_system_prompt(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
) -> Result<String, AppError> {
    // Story 2.3: butler system prompt 拼接顺序：基线 + 可委派角色清单 + 各角色近况 + 行为指南。
    // 顺序固定，保证 LLM 先建立身份，再看到资源，最后被告诉怎么用资源。
    let mut system_prompt = String::from(BUTLER_SYSTEM_PROMPT);

    let active_roles = crate::db::roles::list_active_roles(main_pool).await?;
    if !active_roles.is_empty() {
        system_prompt.push_str("\n\n[可委派角色清单]");
        for r in &active_roles {
            system_prompt.push_str(&format!(
                "\n- id={} | 名称={} | 目标={}",
                r.id,
                r.name,
                if r.goal.trim().is_empty() {
                    "（未设定）"
                } else {
                    r.goal.trim()
                }
            ));
        }
    }

    let memory_summary = build_butler_memory_summary(main_pool).await?;
    if !memory_summary.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&memory_summary);
    }

    let cross_summary = build_cross_role_summary(conv_pool, main_pool).await?;
    if !cross_summary.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&cross_summary);
    }

    if !active_roles.is_empty() {
        // 行为指南只在有可委派角色时才有意义；零角色时不要诱导 LLM 调用工具。
        system_prompt.push_str(
            "\n\n[行为指南]\n\
            - 先判断用户是在陈述事实/偏好，还是在交代任务/安排/待办/需要后续行动。\n\
            - 如果用户只是陈述某个角色相关事实或偏好（例如孩子叫什么、喜欢什么、学习表现如何），直接用自然口吻确认，不要调用工具，不要解释系统会如何记录或同步。\n\
            - 如果用户交代的是某个角色相关任务、安排、日程、待办、规划或需要跟进的事项（例如家长会、约定、准备材料、制定练习计划），只要能匹配 active 角色，就调用 delegate_to_role。\n\
            - 不要向用户暴露内部机制：不要说“系统会同步”“同步到某角色”“角色已收到”“委派成功”“工具调用”等。\n\
            - 当用户只是询问已知事实（例如某个孩子喜欢什么、某个固定安排是什么）时，优先基于[已知记忆]回答，并可说明目前只知道这些。\n\
            - 用户一句话同时涉及多个角色且需要角色处理时：可以在同一轮内调用多个 delegate_to_role（并行委派）。\n\
            - 意图模糊或没有合适角色时：不要调用工具，用一句话主动追问用户希望由谁来处理。\n\
            - 收到角色回复（tool result）后：融合成自然回复，不要强调内部流转；只有用户明确问是谁处理时，才说明对应角色。"
        );

        // Story 2.5: 角色涌现行为指令
        let cooldowns = crate::db::app_settings::get_emergence_cooldowns(main_pool)
            .await
            .unwrap_or_default();
        let now = chrono::Utc::now();
        let active_cooldowns: Vec<String> = cooldowns
            .into_iter()
            .filter(|(_, ts)| {
                chrono::DateTime::parse_from_rfc3339(ts)
                    .map(|dt| now.signed_duration_since(dt).num_days() < 7)
                    .unwrap_or(false)
            })
            .map(|(domain, _)| domain)
            .collect();

        let mut emergence_prompt = String::from(
            "\n\n[角色涌现行为]\n\
            - 当你发现用户在最近几轮对话中反复提到某个尚未被任何 active 角色覆盖的领域时，可以用自然对话的方式建议创建一个新角色。\n\
            - 不要在第一轮就建议，至少观察到用户 2-3 次提及同一领域后再提议。\n\
            - 建议时用自然口吻，例如：「我注意到你最近经常聊到 X，要不要创建一个专门的角色来帮你？」\n\
            - 用户同意后：先用一句话说明你会准备角色提议、用户可在弹窗里确认或调整，然后调用 create_role 工具发起角色提议；不要在工具调用后再追加确认话术。\n\
            - 用户拒绝后：调用 record_emergence_rejection 工具记录被拒领域，然后自然地继续对话。"
        );

        if !active_cooldowns.is_empty() {
            emergence_prompt.push_str(&format!(
                "\n- 最近被拒绝的领域（7天内不要再建议）：{}",
                active_cooldowns.join("、")
            ));
        }

        system_prompt.push_str(&emergence_prompt);
    }

    Ok(system_prompt)
}

pub async fn build_butler_messages(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    user_message: &str,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let mut result = Vec::new();

    let system_prompt = build_butler_system_prompt(conv_pool, main_pool).await?;

    result.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: system_prompt,
        tool_calls: None,
        tool_call_id: None,
    });

    let history =
        conversations::get_recent_messages(conv_pool, conversation_id, HISTORY_LIMIT).await?;
    for msg in history {
        result.push(ChatCompletionMessage {
            role: msg.role,
            content: msg.content,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Only append user_message if it's not already the last message in history
    // (it may have been inserted into DB before this function is called)
    if !user_message.is_empty() {
        let already_in_history = result
            .last()
            .map(|m| m.role == "user" && m.content == user_message)
            .unwrap_or(false);
        if !already_in_history {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(result)
}

const ROLE_BASE_PERSONA_PROMPT: &str = "\
你是用户的「{name}」分身——用户在这个身份维度下的 AI 助手。\
用户自己就是「{name}」，你帮助 TA 以这个身份思考、规划和执行相关目标与任务。\
你不是独立于用户的另一个人，你是用户作为「{name}」时的延伸。\
用简洁自然的中文回复，不用 emoji。";

fn build_role_system_prompt(role: &crate::models::role::Role) -> String {
    let mut sections = vec![ROLE_BASE_PERSONA_PROMPT.replace("{name}", &role.name)];

    let mut role_definition = format!("[role_definition]\n角色名称：{}", role.name);
    if !role.goal.trim().is_empty() {
        role_definition.push_str("\n核心目标：");
        role_definition.push_str(role.goal.trim());
    }

    let personality = role.personality_prompt.trim();
    if !personality.is_empty() {
        role_definition.push_str("\n个性描述：");
        role_definition.push_str(personality);
    } else {
        role_definition.push_str("\n默认语调：根据角色名称与核心目标选择自然语气；产品/工作类偏简洁专业，家庭/生活类偏温暖关怀，学习/成长类偏好奇探索。");
    }
    sections.push(role_definition);

    sections.push(
        "[context_injection]\n以下历史消息是当前对话上下文；不要引入未提供的记忆。".to_string(),
    );
    sections.join("\n\n")
}

/// 构建角色对话上下文。system prompt 完全独立于管家基线，
/// 让 LLM 知道当前在扮演谁。这是 Story 2.2 AC-2 / AC-6 的核心：
/// 角色之间个性化语调差异从此 prompt 注入开始（FR-6）。
pub async fn build_role_messages(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    role_id: &str,
    user_message: &str,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let role = crate::db::roles::get_role(main_pool, role_id).await?;

    let system_prompt = build_role_system_prompt(&role);

    let mut result = Vec::new();
    result.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: system_prompt,
        tool_calls: None,
        tool_call_id: None,
    });

    let history =
        conversations::get_recent_messages(conv_pool, conversation_id, HISTORY_LIMIT).await?;
    for msg in history {
        result.push(ChatCompletionMessage {
            role: msg.role,
            content: msg.content,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    if !user_message.is_empty() {
        let already_in_history = result
            .last()
            .map(|m| m.role == "user" && m.content == user_message)
            .unwrap_or(false);
        if !already_in_history {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(result)
}

const ONBOARDING_SYSTEM_PROMPT: &str = "\
你是 EgoSync 的数字管家，正在引导新用户完成首次设置。\n\n\
【你的唯一任务】\n\
通过自然对话，帮用户确定一个「角色」并调用 create_role 工具向用户【提议】这个角色。\n\
角色 = 用户生活/工作中的一个身份维度（如：产品经理、父亲、健身者）。\n\
每个角色有：名称、图标（line-icon 标识符）、品牌色（hex）、一句话目标。\n\n\
【关键认知】\n\
create_role 工具不会立即创建角色。它只是【向用户发起一个提议】，前端会弹出一个确认窗口，\n\
让用户编辑后点击「创建」按钮才真正创建。所以：\n\
- 你绝对不要说\"已创建\"、\"创建成功\"、\"已经为你建好了\"。\n\
- 你应该说\"我帮你拟了一个，看看怎么样？\"、\"已经为你准备好提议，需要可以在弹窗里调整。\"\n\n\
【对话流程】严格按以下阶段推进，不要跳步也不要卡步：\n\n\
第1步 - 问名字：\n\
  用一句温暖的话自我介绍，问用户怎么称呼。\n\n\
第2步 - 了解方向：\n\
  用户回答名字后，热情回应，然后直接问：\n\
  \"你最近在忙什么？工作还是生活方面有什么特别关注的事情？\"\n\n\
第3步 - 提议角色（一次性完成）：\n\
  从用户的回答中提炼出一个身份维度，直接调用 create_role 工具发起提议。\n\
  不要先问\"我帮你创建一个 XXX 角色怎么样？\"再等用户回答 —— 直接调用工具，前端会弹窗让用户选择。\n\
  你的文字回复只需一句话，例如：\"听起来你在 XXX 方面投入很多，我准备了一个角色提议，看看是否合适？\"\n\n\
第4步 - 等待用户操作：\n\
  调用工具后，等用户在弹窗中点「创建」或「不需要」。\n\
  - 如果用户在聊天里说\"再换一个\"/\"我想要别的\" → 重新调用 create_role 提议新的角色。\n\
  - 如果用户在聊天里说\"不用了\" → 简短回应，等他下一步指示。\n\n\
第5步 - 完成（前端会通知）：\n\
  用户真正确认创建后，你会在历史里看到通知。这时用一句话祝贺。\n\n\
【行为红线】\n\
- 绝对不要在文字中说\"已创建\"、\"创建成功\"。必须用\"提议\"、\"准备\"、\"看看\"这类未完成时态。\n\
- 绝对不要在一条消息里既提议又自己代用户同意。提议归提议，确认归用户。\n\
- 绝对不要变成通用助理（不帮列清单、不帮做规划、不回答知识问题）。\n\
- 如果用户跑题，一句话拉回来：\"这个我之后可以帮你，现在我们先把你的第一个角色定下来。\"\n\
- 每次回复不超过 2 句话。\n\
- 语调温暖简洁，不用 emoji。\n\
- 不要问用户喜欢什么图标、颜色 —— 你自己从白名单选最合适的。\n\n\
【工具使用规则】\n\
你有一个 create_role 工具。识别到合适的身份维度后【直接】调用：\n\
1. name：从对话中提炼的简洁角色名（中文 2–5 字）\n\
2. icon：从下列标识符中选最贴合的：\n\
   briefcase（工作/职业）、code（编程/技术）、chart-bar（数据/分析）、palette（设计/创作）、\n\
   pen-tool（写作）、book-open（阅读/学习）、graduation-cap（教育）、dumbbell（健身/运动）、\n\
   heart-pulse（健康/医疗）、leaf（自然/环保）、home（家庭/居家）、users（团队/朋友）、\n\
   baby（育儿）、gamepad-2（游戏）、music（音乐）、camera（摄影）、plane（旅行）、\n\
   utensils（美食）、coffee（休闲）、target（目标/通用）、sparkles（灵感）、lightbulb（想法）、\n\
   compass（探索/规划）、wallet（财务）\n\
3. color：从下列品牌色中选最贴合的（hex 大写）：\n\
   #4F46E5 靛蓝（理性/专业）、#0EA5E9 天蓝（科技/沟通）、#10B981 翠绿（健康/成长）、\n\
   #F59E0B 琥珀（活力/创意）、#EF4444 玫红（热情/家人）、#8B5CF6 紫罗兰（艺术/灵感）、\n\
   #EC4899 粉（生活/情感）、#64748B 石板灰（稳重/中性）\n\
4. goal：从对话中总结一句目标（10–20 字）";

const ONBOARDING_HISTORY_LIMIT: i64 = 10;

pub async fn build_onboarding_messages(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    user_message: &str,
    _step: u8,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let mut result = Vec::new();

    result.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: ONBOARDING_SYSTEM_PROMPT.to_string(),
        tool_calls: None,
        tool_call_id: None,
    });

    let history =
        conversations::get_recent_messages(conv_pool, conversation_id, ONBOARDING_HISTORY_LIMIT)
            .await?;
    for msg in history {
        // Skip system messages like [onboarding_start] to avoid confusing the model
        if msg.role == "system" {
            continue;
        }
        result.push(ChatCompletionMessage {
            role: msg.role,
            content: msg.content,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Only append user_message if it's not already the last message in history
    // (it may have been inserted into DB before this function is called)
    if !user_message.is_empty() && user_message != "__onboarding_start__" {
        let already_in_history = result
            .last()
            .map(|m| m.role == "user" && m.content == user_message)
            .unwrap_or(false);
        if !already_in_history {
            result.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(result)
}

/// 受支持的图标标识符白名单（黑白线框 Lucide React 风格）。
/// 注意：必须与前端 `GUI/src/lib/roleIcons.ts` 中的 `ROLE_ICONS` 完全同步。
const SUPPORTED_ICONS: &[&str] = &[
    "briefcase",      // 工作/职业
    "code",           // 编程/技术
    "chart-bar",      // 数据/分析
    "palette",        // 设计/创作
    "pen-tool",       // 写作/编辑
    "book-open",      // 阅读/学习
    "graduation-cap", // 教育/进修
    "dumbbell",       // 健身/运动
    "heart-pulse",    // 健康/医疗
    "leaf",           // 自然/环保
    "home",           // 家庭/居家
    "users",          // 团队/朋友
    "baby",           // 育儿/孩子
    "gamepad-2",      // 游戏/娱乐
    "music",          // 音乐
    "camera",         // 摄影
    "plane",          // 旅行
    "utensils",       // 美食/烹饪
    "coffee",         // 咖啡/休闲
    "target",         // 目标/通用
    "sparkles",       // 灵感/创意
    "lightbulb",      // 想法
    "compass",        // 探索/规划
    "wallet",         // 财务
];

/// 受支持的品牌色色板。必须与前端 `ROLE_COLORS` 同步。
const SUPPORTED_COLORS: &[&str] = &[
    "#4F46E5", // indigo
    "#0EA5E9", // sky
    "#10B981", // emerald
    "#F59E0B", // amber
    "#EF4444", // rose
    "#8B5CF6", // violet
    "#EC4899", // pink
    "#64748B", // slate
];

/// Story 2.3: 管家委派工具。
/// LLM 在管家视图判断用户意图明确指向某个 active 角色时调用；
/// 单轮可并行触发多个 tool_calls（一次回合内同时委派多个角色）。
/// follow-up（看到 tool_results 后的回合）必须 tools=None，禁止嵌套。
/// Story 2.5: 涌现拒绝记录工具。
/// 管家在用户拒绝创建角色建议后调用，写入冷却记录避免短期内重复建议同一领域。
fn record_emergence_rejection_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "record_emergence_rejection".to_string(),
        description: "当用户明确拒绝了你提出的创建新角色建议时调用此工具，记录被拒领域。同一领域短期内不会再次建议。".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "domain": {
                    "type": "string",
                    "description": "被用户拒绝的角色领域描述，简短中文，如「健身/运动」「摄影」「理财」"
                }
            },
            "required": ["domain"]
        }),
    }
}

fn delegate_to_role_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "delegate_to_role".to_string(),
        description: "把用户的具体任务委派给当前活跃的某个角色处理。仅在能明确判断任务属于某个 active 角色时调用；意图模糊时不要硬猜，改为追问。允许同一轮内多次调用以委派给多个角色。"
            .to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "target_role_id": {
                    "type": "string",
                    "description": "要委派给的角色 id（必须是 system prompt 「可委派角色清单」中列出的 id 之一）"
                },
                "task_summary": {
                    "type": "string",
                    "description": "交给该角色处理的任务摘要，简洁中文，建议 10-80 字"
                },
                "context": {
                    "type": "string",
                    "description": "可选：用户原话或必要背景，建议不超过 200 字"
                }
            },
            "required": ["target_role_id", "task_summary"]
        }),
    }
}

fn create_role_tool_definition() -> ToolDefinition {
    let icon_enum: Vec<serde_json::Value> = SUPPORTED_ICONS
        .iter()
        .map(|s| serde_json::Value::String((*s).to_string()))
        .collect();
    let color_enum: Vec<serde_json::Value> = SUPPORTED_COLORS
        .iter()
        .map(|s| serde_json::Value::String((*s).to_string()))
        .collect();

    ToolDefinition {
        name: "create_role".to_string(),
        description:
            "当用户最近在关注的事情，对应用户的一个明确的角色，或者用户明确阐述了自己的角色身份时，调用此工具，为用户创建一个虚拟角色。"
                .to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "角色名称，简洁的中文名词，如「产品经理」「健身教练」「父亲」"
                },
                "icon": {
                    "type": "string",
                    "description": "最贴合该角色的图标标识符，从白名单中选择，例如：briefcase=工作、dumbbell=健身、code=编程、book-open=阅读、users=家人。",
                    "enum": icon_enum,
                },
                "color": {
                    "type": "string",
                    "description": "该角色的品牌色，从白名单中选择最贴合的十六进制色值。",
                    "enum": color_enum,
                },
                "goal": {
                    "type": "string",
                    "description": "该角色的一句话目标描述，来自对话上下文，约 10–20 字"
                }
            },
            "required": ["name", "icon", "color", "goal"]
        }),
    }
}

fn get_onboarding_chat_options(step: u8) -> ChatOptions {
    // Only provide the create_role tool from step 3 onwards.
    // Steps 1-2 are for greeting and asking about interests — the model must NOT
    // be able to call create_role until it has gathered enough context.
    if step >= 3 {
        ChatOptions {
            disable_thinking: false,
            tools: Some(vec![create_role_tool_definition()]),
            // Force the model to call the tool — prevents it from endlessly
            // asking clarifying questions instead of proposing a role.
            tool_choice: Some("required".to_string()),
        }
    } else {
        ChatOptions {
            disable_thinking: false,
            tools: None,
            tool_choice: None,
        }
    }
}

pub async fn resolve_default_provider(
    main_pool: &DbPool,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    use crate::db::settings as db;

    let config = db::get_default_llm_config(main_pool).await?;

    let api_key = secret_store::load_secret(&config.api_key_ref)?.ok_or_else(|| {
        AppError::KeyringError(format!(
            "未找到配置 '{}' 的 API Key，请在设置中重新保存",
            config.name
        ))
    })?;

    let provider: Arc<dyn LlmProvider> = match config.provider.as_str() {
        "anthropic" => Arc::new(AnthropicProvider::new(
            config.base_url,
            api_key,
            config.model,
        )?),
        _ => Arc::new(OpenAiProvider::new(config.base_url, api_key, config.model)?),
    };

    Ok(provider)
}

async fn try_run_opencode_stream(
    app_handle: &tauri::AppHandle,
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    assistant_message_id: &str,
    user_message_id: &str,
    user_message: &str,
    cancel_token: &CancellationToken,
    role_id: Option<&str>,
    opencode_sessions: Arc<Mutex<std::collections::HashMap<String, String>>>,
    agent_bridge: crate::services::agent_bridge::AgentBridge,
    event_router: Arc<crate::services::event_router::EventRouter>,
    delegate_bridge: crate::services::delegate_bridge::DelegateBridge,
) -> Result<(), AppError> {
    let session_id = {
        let sessions = opencode_sessions.lock().await;
        sessions.get(conversation_id).cloned()
    };
    let session_id = match session_id {
        Some(id) => id,
        None => {
            let agent = opencode_agent_key(role_id);
            let project_dir = resolve_opencode_project_dir(app_handle);
            let session = agent_bridge.create_session(&agent, &project_dir).await?;
            let mut sessions = opencode_sessions.lock().await;
            remember_opencode_session(&mut sessions, conversation_id, &session.id);
            session.id
        }
    };
    let delegation_session_registered = role_id.is_none() && !user_message_id.is_empty();
    if delegation_session_registered {
        delegate_bridge
            .register_session(&session_id, user_message_id)
            .await;
    }
    tracing::debug!(
        session_id,
        ?role_id,
        conversation_id,
        "stream session resolved"
    );

    // Subscribe BEFORE triggering the prompt so we don't miss early events.
    let mut event_rx = event_router.subscribe(&session_id).await;

    // Build the message content — inject full system prompt prefix.
    // For butler: includes role roster + emergence instructions (same as direct-LLM path).
    // For roles: includes the role's personality/goal prompt.
    let content = if let Some(rid) = role_id {
        if let Ok(role) = crate::db::roles::get_role(main_pool, rid).await {
            let role_prompt = build_role_system_prompt(&role);
            format!("[系统指示]\n{}\n---\n{}", role_prompt, user_message)
        } else {
            user_message.to_string()
        }
    } else {
        // Butler — full system prompt with roster + emergence (consistent with direct-LLM path)
        let butler_prompt = build_butler_system_prompt(conv_pool, main_pool)
            .await
            .unwrap_or_else(|_| String::from(BUTLER_SYSTEM_PROMPT));
        format!("[系统指示]\n{}\n---\n{}", butler_prompt, user_message)
    };

    // Trigger the prompt. POST /session/{id}/message is synchronous (returns
    // the completed message as JSON). Live tokens stream via the event bus,
    // so we run this in a side task and ignore its body.
    let bridge = agent_bridge.clone();
    let send_session_id = session_id.clone();
    let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let result = bridge.send_message(&send_session_id, &content).await;
        let _ = result_tx.send(result);
    });

    // Track the latest text per part so we can emit *deltas* to ChatStream
    // (which appends each token to a bucket). opencode's part.updated event
    // carries the full part text each time, so we diff locally.
    let mut part_text: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut accumulated_text = String::new();
    let mut accumulated_thinking = String::new();
    let mut completed = false;
    // Fix 2: messageID → role mapping. opencode sends message.part.updated for
    // *both* user and assistant messages. We must learn each message's role from
    // "message.updated" events and only stream assistant-role parts to the UI.
    let mut message_roles: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // Track which partIDs are reasoning (thinking) vs text
    let mut reasoning_parts: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut processed_tool_parts: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    // 委派两气泡：第一段"稍等…"落 assistant_message_id；delegate_to_role 激活后，
    // 第二段（委派回复）落新建的 followup 消息，并在激活瞬间唤醒前端弹跳点。
    let mut bubble_state = DelegationBubbleState::default();
    let mut followup_id: Option<String> = None;
    let mut followup_text = String::new();
    let mut delegate_workers: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    let delegate_lock = Arc::new(Mutex::new(()));

    loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                event_router.unsubscribe(&session_id).await;
                if delegation_session_registered {
                    delegate_bridge.unregister_session(&session_id).await;
                }
                if let Some(fid) = followup_id.clone() {
                    // 委派已拆分：第一气泡在拆分时已冻结，这里收尾第二气泡，避免弹跳点卡住。
                    conversations::update_message_content(conv_pool, &fid, &followup_text).await.ok();
                    conversations::mark_message_complete(conv_pool, &fid).await.ok();
                    let _ = app_handle.emit("llm:stream", StreamPayload {
                        conversation_id: conversation_id.to_string(),
                        token: String::new(),
                        done: true,
                        thinking: false,
                        message_id: Some(fid),
                    });
                } else {
                    conversations::update_message_content(conv_pool, assistant_message_id, &accumulated_text).await.ok();
                    if !accumulated_thinking.is_empty() {
                        conversations::update_message_thinking(conv_pool, assistant_message_id, &accumulated_thinking).await.ok();
                    }
                    conversations::mark_message_complete(conv_pool, assistant_message_id).await.ok();
                    let _ = app_handle.emit("llm:stream", StreamPayload {
                        conversation_id: conversation_id.to_string(),
                        token: String::new(),
                        done: true,
                        thinking: false,
                        message_id: None,
                    });
                }
                return Ok(());
            }
            maybe_event = event_rx.recv() => {
                let Some(event) = maybe_event else { break };
                match event.event_type.as_str() {
                    "message.part.delta" => {
                        // Incremental text delta — the primary streaming path.
                        let field = event.properties.get("field")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        if field != "text" { continue; }
                        let delta = event.properties.get("delta")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        if delta.is_empty() { continue; }
                        let msg_id = event.properties.get("messageID")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        // Skip user-role deltas
                        if !msg_id.is_empty() {
                            match message_roles.get(msg_id).map(|s| s.as_str()) {
                                Some("user") => continue,
                                _ => {} // unknown or assistant — proceed
                            }
                        }
                        // Determine thinking vs text by partID tracking
                        let part_id = event.properties.get("partID")
                            .and_then(|v| v.as_str()).unwrap_or("");
                        let thinking = reasoning_parts.contains(part_id);
                        // 委派两气泡路由：thinking 始终归第一气泡；text 按 messageID 分桶。
                        let emit_message_id = if thinking {
                            accumulated_thinking.push_str(delta);
                            None
                        } else {
                            match bubble_state.classify_message(msg_id) {
                                BubbleSlot::First => {
                                    accumulated_text.push_str(delta);
                                    None
                                }
                                BubbleSlot::Followup => {
                                    followup_text.push_str(delta);
                                    followup_id.clone()
                                }
                            }
                        };
                        // Update part_text so message.updated/part.updated won't re-emit
                        if !part_id.is_empty() {
                            let entry = part_text.entry(part_id.to_string()).or_default();
                            entry.push_str(delta);
                        }
                        let _ = app_handle.emit("llm:stream", StreamPayload {
                            conversation_id: conversation_id.to_string(),
                            token: delta.to_string(),
                            done: false,
                            thinking,
                            message_id: emit_message_id,
                        });
                    }
                    "message.updated" => {
                        // Learn messageID → role so we can filter parts later.
                        // Also extract assistant text from completed messages (some
                        // models/providers don't emit incremental message.part.updated
                        // events — the full text arrives here directly).
                        if let Some(info) = event.properties.get("info") {
                            if let (Some(id), Some(role)) = (
                                info.get("id").and_then(|v| v.as_str()),
                                info.get("role").and_then(|v| v.as_str()),
                            ) {
                                message_roles.insert(id.to_string(), role.to_string());
                                // Extract text from completed assistant messages
                                if role == "assistant" {
                                    if let Some(parts) = info.get("parts").and_then(|v| v.as_array()) {
                                        for p in parts {
                                            let ptype = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                            let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                            if !text.is_empty() {
                                                // 委派两气泡路由：reasoning 归 thinking；text 按 messageID 分桶。
                                                let (thinking, emit_message_id, accum): (bool, Option<String>, &mut String) = match ptype {
                                                    "text" => match bubble_state.classify_message(id) {
                                                        BubbleSlot::First => (false, None, &mut accumulated_text),
                                                        BubbleSlot::Followup => (false, followup_id.clone(), &mut followup_text),
                                                    },
                                                    "reasoning" => (true, None, &mut accumulated_thinking),
                                                    _ => continue,
                                                };
                                                let part_id = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                                let prev = part_text.entry(part_id.to_string()).or_default();
                                                if text.len() > prev.len() && text.starts_with(prev.as_str()) {
                                                    let delta = text[prev.len()..].to_string();
                                                    *prev = text.to_string();
                                                    accum.push_str(&delta);
                                                    let _ = app_handle.emit("llm:stream", StreamPayload {
                                                        conversation_id: conversation_id.to_string(),
                                                        token: delta,
                                                        done: false,
                                                        thinking,
                                                        message_id: emit_message_id,
                                                    });
                                                } else if text != prev.as_str() {
                                                    *prev = text.to_string();
                                                    accum.clear();
                                                    accum.push_str(text);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "message.part.updated" => {
                        let Some(part_raw) = event.properties.get("part") else { continue };
                        let Ok(part) = serde_json::from_value::<crate::models::agent::BusPart>(part_raw.clone()) else { continue };
                        tracing::debug!(part_type = %part.part_type, part_id = %part.id, msg_id = %part.message_id, text_len = part.text.len(), "stream part updated");
                        // Register reasoning parts so delta handler knows to set thinking=true
                        if part.part_type == "reasoning" {
                            reasoning_parts.insert(part.id.clone());
                        }
                        // Skip user-role parts — they echo the user's own input.
                        if !part.message_id.is_empty() {
                            match message_roles.get(&part.message_id).map(|s| s.as_str()) {
                                Some("user") => continue,
                                _ => {} // unknown or assistant — proceed
                            }
                        }
                        // Only stream user-visible content. Tool parts are intercepted
                        // to trigger Tauri-side effects (role proposals, delegation, etc.)
                        let (thinking, emit_message_id, accum): (bool, Option<String>, &mut String) = match part.part_type.as_str() {
                            "text" => match bubble_state.classify_message(&part.message_id) {
                                BubbleSlot::First => (false, None, &mut accumulated_text),
                                BubbleSlot::Followup => (false, followup_id.clone(), &mut followup_text),
                            },
                            "reasoning" => (true, None, &mut accumulated_thinking),
                            "tool" => {
                                // Intercept custom tool results from opencode.
                                // Tool part structure: { tool: "create_role", state: { status, output, input } }
                                // We extract the output when status=completed and parse our JSON action.
                                let tool_name = part_raw
                                    .get("tool")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let tool_status = part_raw
                                    .get("state")
                                    .and_then(|s| s.get("status"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                // 委派两气泡：delegate_to_role 进入活动态（running/completed，取最早一次）时，
                                // 冻结第一气泡（"稍等…"）、新建 followup 气泡并发空 token 唤醒弹跳点，
                                // 覆盖委派同步等待 PM 回复的 ~40s 窗口。仅 butler 路径启用。
                                let delegate_active = tool_name == "delegate_to_role"
                                    && (tool_status == "running" || tool_status == "completed");
                                if delegate_active
                                    && role_id.is_none()
                                    && bubble_state.note_delegate_active()
                                {
                                    if !accumulated_text.trim().is_empty() {
                                        conversations::update_message_content(
                                            conv_pool,
                                            assistant_message_id,
                                            &accumulated_text,
                                        )
                                        .await
                                        .ok();
                                        if !accumulated_thinking.is_empty() {
                                            conversations::update_message_thinking(
                                                conv_pool,
                                                assistant_message_id,
                                                &accumulated_thinking,
                                            )
                                            .await
                                            .ok();
                                        }
                                        conversations::mark_message_complete(
                                            conv_pool,
                                            assistant_message_id,
                                        )
                                        .await
                                        .ok();
                                        let _ = app_handle.emit("llm:stream", StreamPayload {
                                            conversation_id: conversation_id.to_string(),
                                            token: String::new(),
                                            done: true,
                                            thinking: false,
                                            message_id: Some(assistant_message_id.to_string()),
                                        });
                                    }
                                    match conversations::insert_message(
                                        conv_pool,
                                        conversation_id,
                                        "assistant",
                                        "",
                                        false,
                                    )
                                    .await
                                    {
                                        Ok(m) => {
                                            let new_id = m.id.clone();
                                            // 空 token 唤醒第二气泡的弹跳点（等待期间持续可见）。
                                            let _ = app_handle.emit("llm:stream", StreamPayload {
                                                conversation_id: conversation_id.to_string(),
                                                token: String::new(),
                                                done: false,
                                                thinking: false,
                                                message_id: Some(new_id.clone()),
                                            });
                                            followup_id = Some(new_id);
                                        }
                                        Err(e) => {
                                            tracing::error!("[delegate] 创建 follow-up 消息失败: {}", e);
                                        }
                                    }
                                }
                                if !claim_completed_tool_part(&mut processed_tool_parts, &part.id, part_raw) {
                                    continue;
                                }
                                if let Some(worker) = handle_tool_part(
                                    app_handle,
                                    conv_pool,
                                    main_pool,
                                    conversation_id,
                                    user_message_id,
                                    part_raw,
                                    delegate_lock.clone(),
                                )
                                .await
                                {
                                    delegate_workers.push(worker);
                                }
                                continue;
                            }
                            _ => continue,
                        };
                        let prev = part_text.entry(part.id.clone()).or_default();
                        if part.text.len() > prev.len() && part.text.starts_with(prev.as_str()) {
                            let delta = part.text[prev.len()..].to_string();
                            *prev = part.text.clone();
                            accum.push_str(&delta);
                            let _ = app_handle.emit("llm:stream", StreamPayload {
                                conversation_id: conversation_id.to_string(),
                                token: delta,
                                done: false,
                                thinking,
                                message_id: emit_message_id,
                            });
                        } else if part.text != *prev {
                            // Non-monotonic update (replacement). Replay full text.
                            *prev = part.text.clone();
                            accum.clear();
                            accum.push_str(&part.text);
                        }
                    }
                    "session.idle" | "session.status" => {
                        // session.status carries { status: { type: "idle" } }
                        if event.event_type == "session.status" {
                            let is_idle = event.properties.get("status")
                                .and_then(|v| v.get("type"))
                                .and_then(|v| v.as_str())
                                == Some("idle");
                            if !is_idle {
                                continue;
                            }
                        }
                        // One prompt round complete — finalize and exit.
                        completed = true;
                        break;
                    }
                    "session.error" => {
                        let detail = event.properties.get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("opencode session error");
                        let friendly = format!("抱歉，Agent 引擎返回错误：{}", summarize_error(detail));
                        accumulated_text.push_str(&friendly);
                        completed = true;
                        break;
                    }
                    _ => { /* ignore unrelated events */ }
                }
            }
            send_done = &mut result_rx => {
                // The POST returned (success or failure). For success this just
                // means the LLM finished; the closing `session.idle` event may
                // or may not have arrived yet — keep draining briefly.
                if let Ok(Err(e)) = send_done {
                    event_router.unsubscribe(&session_id).await;
                    if delegation_session_registered {
                        delegate_bridge.unregister_session(&session_id).await;
                    }
                    if accumulated_text.is_empty() && accumulated_thinking.is_empty() {
                        let mut sessions = opencode_sessions.lock().await;
                        sessions.remove(conversation_id);
                        return Err(e);
                    }
                    let friendly = format!("\n\n抱歉，Agent 引擎返回错误：{}", summarize_error(&e.to_string()));
                    accumulated_text.push_str(&friendly);
                    completed = true;
                    break;
                }
                // POST success: the synchronous endpoint has returned,
                // meaning LLM finished. All streaming deltas already arrived
                // before POST returns (it waits for completion).
                completed = true;
                break;
            }
        }
    }

    event_router.unsubscribe(&session_id).await;
    if delegation_session_registered {
        delegate_bridge.unregister_session(&session_id).await;
    }

    for worker in delegate_workers {
        if let Err(err) = worker.await {
            tracing::warn!(error = %err, "delegate worker join failed");
        }
    }

    if !completed {
        // Stream ended without explicit completion (channel dropped, etc.) —
        // still finalize so the UI doesn't get stuck.
        completed = true;
    }

    if let Some(fid) = followup_id.clone() {
        // 委派已拆分：第一气泡（"稍等…"）在拆分时已冻结+done，这里收尾第二气泡（委派回复）。
        conversations::update_message_content(conv_pool, &fid, &followup_text)
            .await
            .ok();
        conversations::mark_message_complete(conv_pool, &fid)
            .await
            .ok();
        let _ = app_handle.emit(
            "llm:stream",
            StreamPayload {
                conversation_id: conversation_id.to_string(),
                token: String::new(),
                done: true,
                thinking: false,
                message_id: Some(fid),
            },
        );
    } else {
        // 普通单段对话：单气泡收尾（改动前的原行为）。
        conversations::update_message_content(conv_pool, assistant_message_id, &accumulated_text)
            .await
            .ok();
        if !accumulated_thinking.is_empty() {
            conversations::update_message_thinking(
                conv_pool,
                assistant_message_id,
                &accumulated_thinking,
            )
            .await
            .ok();
        }
        conversations::mark_message_complete(conv_pool, assistant_message_id)
            .await
            .ok();
        let _ = app_handle.emit(
            "llm:stream",
            StreamPayload {
                conversation_id: conversation_id.to_string(),
                token: String::new(),
                done: true,
                thinking: false,
                message_id: None,
            },
        );
    }
    let _ = completed;
    Ok(())
}

pub async fn run_stream(
    app_handle: tauri::AppHandle,
    conv_pool: ConversationsPool,
    main_pool: DbPool,
    conversation_id: String,
    assistant_message_id: String,
    user_message: String,
    cancel_token: CancellationToken,
    onboarding_step: Option<u8>,
    role_id: Option<String>,
    // Story 2.3: 触发本轮的 user message id —— 用于在 delegate 执行后把 routing_metadata
    // 写回触发这一轮的那条 user message。仅 butler 委派路径使用，其他路径忽略。
    user_message_id: String,
    opencode_sessions: Arc<Mutex<std::collections::HashMap<String, String>>>,
    agent_bridge: crate::services::agent_bridge::AgentBridge,
    event_router: Arc<crate::services::event_router::EventRouter>,
    delegate_bridge: crate::services::delegate_bridge::DelegateBridge,
) -> Result<(), AppError> {
    if let Err(e) = try_run_opencode_stream(
        &app_handle,
        &conv_pool,
        &main_pool,
        &conversation_id,
        &assistant_message_id,
        &user_message_id,
        &user_message,
        &cancel_token,
        role_id.as_deref(),
        opencode_sessions.clone(),
        agent_bridge,
        event_router,
        delegate_bridge,
    )
    .await
    {
        tracing::warn!(
            "opencode stream unavailable, falling back to LlmProvider: {}",
            e
        );
    } else {
        return Ok(());
    }

    let messages = match (onboarding_step, role_id.as_deref()) {
        (Some(step), _) => {
            build_onboarding_messages(&conv_pool, &conversation_id, &user_message, step).await?
        }
        (None, Some(rid)) => {
            build_role_messages(&conv_pool, &main_pool, &conversation_id, rid, &user_message)
                .await?
        }
        (None, None) => {
            build_butler_messages(&conv_pool, &main_pool, &conversation_id, &user_message).await?
        }
    };
    let provider = resolve_default_provider(&main_pool).await?;

    let chat_options = match (onboarding_step, role_id.as_deref()) {
        // onboarding：原有 create_role 工具（分阶段开启），与本 story 互斥
        (Some(step), _) => get_onboarding_chat_options(step),
        // 角色视图：不挂任何工具（Story 2.3 AC-4：角色私聊不再触发跨角色委派）
        (None, Some(_)) => ChatOptions::default(),
        // 管家视图：挂 delegate_to_role + create_role + record_emergence_rejection
        // tool_choice=None 让 LLM 自主决定（意图模糊时追问）
        (None, None) => ChatOptions {
            disable_thinking: false,
            tools: Some(vec![
                delegate_to_role_tool_definition(),
                create_role_tool_definition(),
                record_emergence_rejection_tool_definition(),
            ]),
            tool_choice: None,
        },
    };

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);

    let provider_clone = provider.clone();
    let messages_clone = messages.clone();
    let options_clone = chat_options.clone();
    let stream_start = Instant::now();
    tokio::spawn(async move {
        if let Err(e) = provider_clone
            .chat_stream(messages_clone, tx, options_clone)
            .await
        {
            tracing::error!("LLM 流式调用失败: {}", e);
        }
    });

    let mut accumulated = OPENCODE_FALLBACK_NOTICE.to_string();
    let mut accumulated_thinking = String::new();
    let mut pending = String::new();
    let mut pending_thinking = String::new();
    let mut is_thinking = false;
    let mut last_emit = Instant::now();
    let mut tool_calls_received: Vec<ToolCall> = Vec::new();
    const EMIT_INTERVAL: Duration = Duration::from_millis(33);

    loop {
        let event = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                tracing::info!("流式回复被用户中断");
                None
            }
            event = rx.recv() => event,
            _ = tokio::time::sleep(EMIT_INTERVAL) => {
                if !pending_thinking.is_empty() {
                    let batch = std::mem::take(&mut pending_thinking);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: true,
                            message_id: None,
                        },
                    );
                    last_emit = Instant::now();
                }
                if !pending.is_empty() {
                    let batch = std::mem::take(&mut pending);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: false,
                            message_id: None,
                        },
                    );
                    last_emit = Instant::now();
                }
                continue;
            }
        };

        match event {
            Some(StreamEvent::Thinking(token)) => {
                if !is_thinking {
                    is_thinking = true;
                    tracing::info!("思考开始，耗时: {:?}", stream_start.elapsed());
                }
                accumulated_thinking.push_str(&token);
                pending_thinking.push_str(&token);

                if last_emit.elapsed() >= EMIT_INTERVAL {
                    let batch = std::mem::take(&mut pending_thinking);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: true,
                            message_id: None,
                        },
                    );
                    last_emit = Instant::now();
                }
            }
            Some(StreamEvent::Token(token)) => {
                if accumulated.is_empty() {
                    tracing::info!(
                        "首个 content token 到达，耗时: {:?}",
                        stream_start.elapsed()
                    );
                }
                accumulated.push_str(&token);
                pending.push_str(&token);

                if last_emit.elapsed() >= EMIT_INTERVAL {
                    let batch = std::mem::take(&mut pending);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: false,
                            message_id: None,
                        },
                    );
                    last_emit = Instant::now();
                }
            }
            Some(StreamEvent::ToolCall(tc)) => {
                tracing::info!(
                    "[run_stream] 收到 ToolCall: name={} id={} args={}",
                    tc.name,
                    tc.id,
                    tc.arguments
                );
                tool_calls_received.push(tc);
            }
            Some(StreamEvent::Done) => {
                tracing::info!(
                    "[run_stream] done: conv={} assistant_msg={} accumulated_len={} thinking_len={} tool_calls={}",
                    conversation_id,
                    assistant_message_id,
                    accumulated.len(),
                    accumulated_thinking.len(),
                    tool_calls_received.len()
                );
                // Flush pending tokens
                if !pending_thinking.is_empty() {
                    let batch = std::mem::take(&mut pending_thinking);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: true,
                            message_id: None,
                        },
                    );
                }
                if !pending.is_empty() {
                    let batch = std::mem::take(&mut pending);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: false,
                            message_id: None,
                        },
                    );
                }

                // Handle tool calls if any
                if !tool_calls_received.is_empty() {
                    // Story 2.3 P1b: 先冻结第一段（管家「稍等，我让 X 看一下」）—
                    // 否则 follow-up 累加进同一变量会让两段在同一气泡里拼接显示。
                    let first_bubble_text = accumulated.clone();
                    tracing::info!(
                        "[run_stream] handling_tools: conv={} assistant_msg={} first_bubble_len={} thinking_len={} tool_calls={}",
                        conversation_id,
                        assistant_message_id,
                        first_bubble_text.len(),
                        accumulated_thinking.len(),
                        tool_calls_received.len()
                    );
                    let first_bubble_visible = !first_bubble_text.trim().is_empty();
                    if first_bubble_visible {
                        conversations::update_message_content(
                            &conv_pool,
                            &assistant_message_id,
                            &first_bubble_text,
                        )
                        .await
                        .ok();
                        if !accumulated_thinking.is_empty() {
                            conversations::update_message_thinking(
                                &conv_pool,
                                &assistant_message_id,
                                &accumulated_thinking,
                            )
                            .await
                            .ok();
                        }
                        conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                            .await
                            .ok();
                        // 通知前端第一气泡 done（仍带其 message_id，便于前端定型这桶）。
                        let _ = app_handle.emit(
                            "llm:stream",
                            StreamPayload {
                                conversation_id: conversation_id.clone(),
                                token: String::new(),
                                done: true,
                                thinking: false,
                                message_id: Some(assistant_message_id.clone()),
                            },
                        );
                    } else {
                        tracing::warn!(
                            "[run_stream] empty_first_bubble_deleted: conv={} assistant_msg={} tool_calls={}",
                            conversation_id,
                            assistant_message_id,
                            tool_calls_received.len()
                        );
                        conversations::delete_message(&conv_pool, &assistant_message_id)
                            .await
                            .ok();
                    }

                    let only_create_role_tool = tool_calls_received.len() == 1
                        && tool_calls_received
                            .first()
                            .map(|tc| tc.name == "create_role")
                            .unwrap_or(false);

                    let mut precomputed_tool_results: Option<Vec<String>> = None;
                    if only_create_role_tool {
                        let tool_results = execute_tool_calls(
                            &app_handle,
                            &main_pool,
                            &conv_pool,
                            &conversation_id,
                            &user_message_id,
                            &tool_calls_received,
                        )
                        .await;
                        let proposal_emitted = tool_results
                            .first()
                            .map(|result| result.starts_with("role_proposal_emitted:"))
                            .unwrap_or(false);

                        if proposal_emitted {
                            tracing::info!(
                                "[run_stream] create_role_followup_skipped: conv={} assistant_msg={}",
                                conversation_id,
                                assistant_message_id
                            );
                            if !first_bubble_visible {
                                let _ = app_handle.emit(
                                    "llm:stream",
                                    StreamPayload {
                                        conversation_id: conversation_id.clone(),
                                        token: String::new(),
                                        done: true,
                                        thinking: false,
                                        message_id: None,
                                    },
                                );
                            }
                            break;
                        }

                        tracing::warn!(
                            "[run_stream] create_role_followup_retained: conv={} assistant_msg={} result={:?}",
                            conversation_id,
                            assistant_message_id,
                            tool_results.first()
                        );
                        precomputed_tool_results = Some(tool_results);
                    }

                    // 第二段独立消息：先建 DB 占位 + 唤醒前端弹跳点，再执行耗时的委派调用。
                    // 这样用户在委派期间就能看到等待提示，而不是干等十几秒。
                    let followup_msg = match conversations::insert_message(
                        &conv_pool,
                        &conversation_id,
                        "assistant",
                        "",
                        false,
                    )
                    .await
                    {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::error!("[delegate] 创建 follow-up 消息失败: {}", e);
                            break;
                        }
                    };
                    let followup_id = followup_msg.id.clone();
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: String::new(),
                            done: false,
                            thinking: false,
                            message_id: Some(followup_id.clone()),
                        },
                    );

                    let tool_results = match precomputed_tool_results {
                        Some(results) => results,
                        None => {
                            execute_tool_calls(
                                &app_handle,
                                &main_pool,
                                &conv_pool,
                                &conversation_id,
                                &user_message_id,
                                &tool_calls_received,
                            )
                            .await
                        }
                    };

                    // Build follow-up messages with tool results for continuation
                    let mut followup_messages = messages.clone();
                    // Add the assistant message with tool_calls
                    followup_messages.push(ChatCompletionMessage {
                        role: "assistant".to_string(),
                        content: first_bubble_text,
                        tool_calls: Some(tool_calls_received.clone()),
                        tool_call_id: None,
                    });
                    // Add tool results
                    for (tc, result_text) in tool_calls_received.iter().zip(tool_results.iter()) {
                        let content = if tc.name == "create_role"
                            && result_text.starts_with("role_proposal_emitted:")
                        {
                            "角色提议已发送给用户，等待用户在弹窗中确认或修改。不要宣布创建成功。"
                                .to_string()
                        } else {
                            result_text.clone()
                        };
                        followup_messages.push(ChatCompletionMessage {
                            role: "tool".to_string(),
                            content,
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        });
                    }

                    // Stream the follow-up response (LLM will generate text after seeing tool results)
                    let (tx2, mut rx2) = mpsc::channel::<StreamEvent>(128);
                    let provider_clone2 = provider.clone();
                    let followup_clone = followup_messages;
                    tokio::spawn(async move {
                        // Story 2.3 AC-3: follow-up 必须 tools=None，禁止嵌套委派/嵌套 create_role
                        if let Err(e) = provider_clone2
                            .chat_stream(
                                followup_clone,
                                tx2,
                                ChatOptions {
                                    disable_thinking: true,
                                    tools: None,
                                    tool_choice: None,
                                },
                            )
                            .await
                        {
                            tracing::error!("工具结果后续流式调用失败: {}", e);
                        }
                    });

                    // Stream follow-up tokens into the second bubble
                    let mut followup_accumulated = String::new();
                    while let Some(event2) = rx2.recv().await {
                        match event2 {
                            StreamEvent::Token(token) => {
                                followup_accumulated.push_str(&token);
                                let _ = app_handle.emit(
                                    "llm:stream",
                                    StreamPayload {
                                        conversation_id: conversation_id.clone(),
                                        token,
                                        done: false,
                                        thinking: false,
                                        message_id: Some(followup_id.clone()),
                                    },
                                );
                            }
                            StreamEvent::Done => break,
                            StreamEvent::Error(e) => {
                                tracing::error!("后续流式回复出错: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }

                    // 落库第二段 + 通知 done（带 followup id 让前端定型这桶并触发历史回拉）。
                    tracing::info!(
                        "[run_stream] followup_done: conv={} followup_msg={} followup_len={}",
                        conversation_id,
                        followup_id,
                        followup_accumulated.len()
                    );
                    if followup_accumulated.trim().is_empty() {
                        tracing::warn!(
                            "[run_stream] empty_followup: conv={} followup_msg={} tool_calls={}",
                            conversation_id,
                            followup_id,
                            tool_calls_received.len()
                        );
                    }
                    conversations::update_message_content(
                        &conv_pool,
                        &followup_id,
                        &followup_accumulated,
                    )
                    .await
                    .ok();
                    conversations::mark_message_complete(&conv_pool, &followup_id)
                        .await
                        .ok();
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: String::new(),
                            done: true,
                            thinking: false,
                            message_id: Some(followup_id),
                        },
                    );
                    break;
                } else if onboarding_step.is_some() && looks_like_fake_role_creation(&accumulated) {
                    // ========== 方案 A 兜底：模型伪装了"角色创建成功"但实际没发 tool_calls ==========
                    tracing::warn!(
                        "[fallback] onboarding 期间检测到伪造的角色创建文本，启动二次提取。accumulated_preview={:?}",
                        accumulated.chars().take(120).collect::<String>()
                    );
                    // 先尝试直接从文本里抠 JSON（极少出现但便宜）；否则再发一次 LLM 调用做提取
                    let role_input = match parse_role_json(&accumulated) {
                        Some(r) => {
                            tracing::info!(
                                "[fallback] 直接从 assistant 文本解析到角色 JSON: name={}",
                                r.name
                            );
                            Some(r)
                        }
                        None => try_fallback_extract_role(&provider, &messages, &accumulated).await,
                    };
                    if let Some(input) = role_input {
                        // 兜底分支同样改为「仅提议」：不直接写库，发 role:proposed 让用户在弹窗里确认。
                        // 归一化 icon/color 到白名单（与 execute_create_role 保持一致）
                        let normalized_icon = input.icon.as_deref().and_then(|s| {
                            let t = s.trim();
                            if SUPPORTED_ICONS.iter().any(|w| *w == t) {
                                Some(t.to_string())
                            } else {
                                None
                            }
                        });
                        let normalized_color = input.color.as_deref().and_then(|s| {
                            let t = s.trim().to_ascii_uppercase();
                            if SUPPORTED_COLORS.iter().any(|w| w.eq_ignore_ascii_case(&t)) {
                                Some(t)
                            } else {
                                None
                            }
                        });
                        tracing::info!(
                            "[fallback] 兜底提议角色: name={} icon={:?} color={:?}",
                            input.name,
                            normalized_icon,
                            normalized_color,
                        );
                        let _ = app_handle.emit(
                            "role:proposed",
                            RoleProposedPayload {
                                conversation_id: conversation_id.clone(),
                                name: input.name.clone(),
                                icon: normalized_icon,
                                color: normalized_color,
                                goal: input.goal.clone(),
                            },
                        );
                    } else {
                        tracing::warn!("[fallback] 二次提取未得到有效角色，跳过提议");
                    }
                    // ========== 兜底结束 ==========
                }

                // Save final message
                conversations::update_message_content(
                    &conv_pool,
                    &assistant_message_id,
                    &accumulated,
                )
                .await
                .ok();
                if !accumulated_thinking.is_empty() {
                    conversations::update_message_thinking(
                        &conv_pool,
                        &assistant_message_id,
                        &accumulated_thinking,
                    )
                    .await
                    .ok();
                }
                conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                    .await
                    .ok();

                let _ = app_handle.emit(
                    "llm:stream",
                    StreamPayload {
                        conversation_id: conversation_id.clone(),
                        token: String::new(),
                        done: true,
                        thinking: false,
                        message_id: None,
                    },
                );
                break;
            }
            Some(StreamEvent::Error(err_msg)) => {
                let friendly = format!(
                    "抱歉，我现在无法回应。原因：{}。请检查一下模型配置是否正确。",
                    summarize_error(&err_msg)
                );

                conversations::update_message_content(&conv_pool, &assistant_message_id, &friendly)
                    .await
                    .ok();
                conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                    .await
                    .ok();

                let _ = app_handle.emit(
                    "llm:stream",
                    StreamPayload {
                        conversation_id: conversation_id.clone(),
                        token: friendly,
                        done: true,
                        thinking: false,
                        message_id: None,
                    },
                );
                break;
            }
            None => {
                if !pending.is_empty() {
                    let batch = std::mem::take(&mut pending);
                    let _ = app_handle.emit(
                        "llm:stream",
                        StreamPayload {
                            conversation_id: conversation_id.clone(),
                            token: batch,
                            done: false,
                            thinking: false,
                            message_id: None,
                        },
                    );
                }

                conversations::update_message_content(
                    &conv_pool,
                    &assistant_message_id,
                    &accumulated,
                )
                .await
                .ok();
                if !accumulated_thinking.is_empty() {
                    conversations::update_message_thinking(
                        &conv_pool,
                        &assistant_message_id,
                        &accumulated_thinking,
                    )
                    .await
                    .ok();
                }
                conversations::mark_message_complete(&conv_pool, &assistant_message_id)
                    .await
                    .ok();

                let _ = app_handle.emit(
                    "llm:stream",
                    StreamPayload {
                        conversation_id: conversation_id.clone(),
                        token: String::new(),
                        done: true,
                        thinking: false,
                        message_id: None,
                    },
                );
                break;
            }
        }
    }

    Ok(())
}

/// 委派两气泡路由的纯状态机。
///
/// 实测时间线（运行时诊断确认）：管家委派会产生两条不同 messageID 的
/// assistant 消息——第一段"稍等…"（msg_A）与委派回复"产品经理…"（msg_B），
/// 中间 `delegate_to_role` 工具经历 pending→running→completed（约 40s 等待）。
///
/// 路由规则：
/// - 委派激活前，所有 assistant 文本归第一气泡（`First`）。
/// - `delegate_to_role` 进入活动态（running/completed，取最早一次）时切换，
///   调用方据此冻结第一气泡、新建 followup 气泡并发空 token 唤醒弹跳点。
/// - 委派激活后出现的**新** messageID 归 followup 气泡（`Followup`）；
///   已知属于第一气泡的 messageID 仍保持 `First`（其残余 delta 不串桶）。
///
/// 不依赖具体 DB id，只做"该归哪个气泡"的决策，便于单测；I/O 留在调用方。
#[derive(Default)]
struct DelegationBubbleState {
    delegated: bool,
    first_message_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BubbleSlot {
    First,
    Followup,
}

impl DelegationBubbleState {
    /// 判定某条 assistant messageID 的文本应落入哪个气泡。
    fn classify_message(&mut self, message_id: &str) -> BubbleSlot {
        if !self.delegated {
            // 委派尚未激活：记住第一条 messageID，全部归第一气泡。
            if self.first_message_id.is_none() {
                self.first_message_id = Some(message_id.to_string());
            }
            return BubbleSlot::First;
        }
        // 委派已激活：第一气泡的原 messageID 仍归 First，其它新 messageID 归 Followup。
        match &self.first_message_id {
            Some(first) if first == message_id => BubbleSlot::First,
            _ => BubbleSlot::Followup,
        }
    }

    /// 标记 `delegate_to_role` 进入活动态。首次调用返回 true（调用方据此执行
    /// 一次性的"切第二气泡 + 唤醒弹跳点"），后续重复上报返回 false。
    fn note_delegate_active(&mut self) -> bool {
        if self.delegated {
            return false;
        }
        self.delegated = true;
        true
    }

    /// 委派是否已激活（调用方用于决定 delta 是否需要路由到 followup 气泡）。
    #[allow(dead_code)]
    fn is_delegated(&self) -> bool {
        self.delegated
    }
}

fn claim_completed_tool_part(
    processed_tool_parts: &mut std::collections::HashSet<String>,
    part_id: &str,
    part_raw: &serde_json::Value,
) -> bool {
    let status = part_raw
        .get("state")
        .and_then(|s| s.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let output = part_raw
        .get("state")
        .and_then(|s| s.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    status == "completed"
        && !part_id.is_empty()
        && !output.is_empty()
        && processed_tool_parts.insert(part_id.to_string())
}

fn spawn_delegate_worker<F>(lock: Arc<Mutex<()>>, work: F) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let _guard = lock.lock().await;
        work.await;
    })
}

fn non_json_tool_output_preview(output: &str) -> String {
    output.chars().take(100).collect()
}

/// Handle a tool bus part from opencode's custom tools.
/// opencode tool parts have this structure:
/// ```json
/// { "type": "tool", "tool": "create_role",
///   "state": { "status": "completed", "output": "{\"action\":...}", "input": {...} } }
/// ```
/// We only act on status=completed parts where output contains our action JSON.
async fn handle_tool_part<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    butler_user_message_id: &str,
    part_raw: &serde_json::Value,
    delegate_lock: Arc<Mutex<()>>,
) -> Option<tokio::task::JoinHandle<()>> {
    // Only process completed tool calls
    let status = part_raw
        .get("state")
        .and_then(|s| s.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if status != "completed" {
        return None;
    }

    let tool_name = part_raw.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let output = part_raw
        .get("state")
        .and_then(|s| s.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if output.is_empty() {
        return None;
    }

    tracing::info!(
        "[opencode-tool] completed: tool={} output_len={}",
        tool_name,
        output.len()
    );

    // Parse the JSON result from our custom tool's execute()
    let Ok(result_json) = serde_json::from_str::<serde_json::Value>(output) else {
        tracing::debug!(
            "[opencode-tool] non-JSON output, ignoring: {:?}",
            non_json_tool_output_preview(output)
        );
        return None;
    };

    let action = result_json
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    tracing::info!("[opencode-tool] action={} conv={}", action, conversation_id);

    match action {
        "create_role" => {
            let name = result_json
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if name.is_empty() {
                return None;
            }
            let goal = result_json
                .get("goal")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let icon = result_json
                .get("icon")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let t = s.trim();
                    if SUPPORTED_ICONS.contains(&t) {
                        Some(t.to_string())
                    } else {
                        None
                    }
                });
            let color = result_json
                .get("color")
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    let t = s.trim().to_ascii_uppercase();
                    if SUPPORTED_COLORS.iter().any(|c| c.eq_ignore_ascii_case(&t)) {
                        Some(t)
                    } else {
                        None
                    }
                });

            tracing::info!(
                "[opencode-tool] create_role proposal: name={} icon={:?} color={:?}",
                name,
                icon,
                color
            );

            let _ = app_handle.emit(
                "role:proposed",
                RoleProposedPayload {
                    conversation_id: conversation_id.to_string(),
                    name: name.to_string(),
                    icon,
                    color,
                    goal,
                },
            );
        }
        "delegate_to_role" => {
            let role_id = result_json
                .get("target_role_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let task_summary = result_json
                .get("task_summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if role_id.is_empty() || task_summary.is_empty() {
                return None;
            }
            let conv_pool = conv_pool.clone();
            let main_pool = main_pool.clone();
            let butler_user_message_id = butler_user_message_id.to_string();
            let result_json = result_json.clone();
            let worker = spawn_delegate_worker(delegate_lock, async move {
                let _ = handle_delegate_tool_result(
                    &conv_pool,
                    &main_pool,
                    &butler_user_message_id,
                    &result_json,
                )
                .await;
            });
            tracing::info!(
                "[opencode-tool] delegate_to_role: role_id={} task={}",
                role_id,
                task_summary
            );

            let _ = app_handle.emit(
                "role:delegated",
                serde_json::json!({
                    "roleId": role_id,
                    "taskSummary": task_summary,
                    "conversationId": conversation_id,
                }),
            );
            return Some(worker);
        }
        "record_emergence_rejection" => {
            let domain = result_json
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if domain.is_empty() {
                return None;
            }
            let now = chrono::Utc::now().to_rfc3339();
            if let Err(e) =
                crate::db::app_settings::set_emergence_cooldown(main_pool, domain, &now).await
            {
                tracing::error!("[opencode-tool] emergence rejection write failed: {}", e);
            }
            let _ = crate::db::app_settings::clear_expired_cooldowns(main_pool, 7).await;
            tracing::info!(
                "[opencode-tool] emergence rejection recorded: domain={}",
                domain
            );
        }
        _ => {
            tracing::debug!("[opencode-tool] unknown action={}, ignoring", action);
        }
    }
    None
}

async fn handle_delegate_tool_result(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    butler_user_message_id: &str,
    result_json: &serde_json::Value,
) -> Option<DelegationRecord> {
    let role_id = result_json
        .get("target_role_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let task_summary = result_json
        .get("task_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if role_id.is_empty() || task_summary.is_empty() {
        return None;
    }

    let args = serde_json::json!({
        "target_role_id": role_id,
        "task_summary": task_summary,
        "context": result_json
            .get("context")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    })
    .to_string();
    let (_text, record) = execute_delegate_to_role(main_pool, conv_pool, &args).await;
    append_delegation_metadata(conv_pool, butler_user_message_id, &record).await;
    Some(record)
}

pub async fn append_delegation_metadata(
    conv_pool: &ConversationsPool,
    butler_user_message_id: &str,
    record: &DelegationRecord,
) {
    let mut delegations = existing_delegations(conv_pool, butler_user_message_id).await;
    delegations.push(serde_json::to_value(record).unwrap_or_else(|_| serde_json::json!({})));
    match serde_json::to_string(&serde_json::json!({ "delegations": delegations })) {
        Ok(json) => {
            if let Err(e) = crate::db::conversations::update_message_routing_metadata(
                conv_pool,
                butler_user_message_id,
                &json,
            )
            .await
            {
                tracing::error!("[opencode-tool] 写入 routing_metadata 失败: {}", e);
            }
        }
        Err(e) => tracing::error!("[opencode-tool] 序列化 routing_metadata 失败: {}", e),
    }
}

async fn existing_delegations(
    conv_pool: &ConversationsPool,
    butler_user_message_id: &str,
) -> Vec<serde_json::Value> {
    if butler_user_message_id.is_empty() {
        return Vec::new();
    }
    let row = sqlx::query_scalar::<_, Option<String>>(
        "SELECT routing_metadata FROM messages WHERE id = ? LIMIT 1",
    )
    .bind(butler_user_message_id)
    .fetch_optional(&**conv_pool)
    .await;
    let Ok(Some(Some(metadata))) = row else {
        return Vec::new();
    };
    serde_json::from_str::<serde_json::Value>(&metadata)
        .ok()
        .and_then(|v| v.get("delegations").and_then(|d| d.as_array()).cloned())
        .unwrap_or_default()
}

/// Execute tool calls and return result strings for each
async fn execute_tool_calls(
    app_handle: &tauri::AppHandle,
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    conversation_id: &str,
    butler_user_message_id: &str,
    tool_calls: &[ToolCall],
) -> Vec<String> {
    let mut results = Vec::new();
    // Story 2.3 AC-6: 收集本轮所有 delegate 的审计记录，最后合并写入 routing_metadata
    let mut delegations: Vec<DelegationRecord> = Vec::new();

    for tc in tool_calls {
        tracing::info!(
            "[tool] executing: conv={} user_msg={} name={} id={} args_len={}",
            conversation_id,
            butler_user_message_id,
            tc.name,
            tc.id,
            tc.arguments.len()
        );
        let result = match tc.name.as_str() {
            "create_role" => {
                execute_create_role(app_handle, main_pool, conversation_id, &tc.arguments).await
            }
            "delegate_to_role" => {
                // Story 2.3 AC-2: 单轮内多个 delegate_to_role 串行执行（V1）
                let (text, record) =
                    execute_delegate_to_role(main_pool, conv_pool, &tc.arguments).await;
                delegations.push(record);
                text
            }
            "record_emergence_rejection" => {
                execute_record_emergence_rejection(main_pool, &tc.arguments).await
            }
            _ => {
                tracing::warn!("未知工具调用: {}", tc.name);
                format!("错误：未知工具 {}", tc.name)
            }
        };
        results.push(result);
    }

    // Story 2.3 AC-6: 把本轮的所有 delegation 合并写入触发 user message
    if !delegations.is_empty() && !butler_user_message_id.is_empty() {
        match serde_json::to_string(&serde_json::json!({ "delegations": delegations })) {
            Ok(json) => {
                if let Err(e) = crate::db::conversations::update_message_routing_metadata(
                    conv_pool,
                    butler_user_message_id,
                    &json,
                )
                .await
                {
                    tracing::error!("[delegate] 写入 routing_metadata 失败: {}", e);
                }
            }
            Err(e) => tracing::error!("[delegate] 序列化 routing_metadata 失败: {}", e),
        }
    }

    results
}

/// Story 2.3 AC-6: 单条委派的审计记录，多个合并进 routing_metadata.delegations[]。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationRecord {
    target_role_id: String,
    target_role_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_conversation_id: Option<String>,
    task_summary: String,
    /// "ok" / "role_not_found"
    status: String,
}

/// Story 2.3 AC-1 / AC-4 / AC-8: 把任务委派给角色 LLM 处理并返回结果。
/// 实现要点：
/// - 校验 role 存在且 status=active；否则走 AC-8 兜底，不污染角色对话
/// - 角色 LLM 调用本地化 drain，**不** emit `llm:stream`（避免污染管家流，见 Dev Notes 设计决策）
/// - 委派交互写入角色对话历史（AC-4：角色"记得"被委派）
pub async fn execute_delegate_to_role(
    main_pool: &DbPool,
    conv_pool: &ConversationsPool,
    arguments: &str,
) -> (String, DelegationRecord) {
    #[derive(serde::Deserialize)]
    struct DelegateArgs {
        target_role_id: String,
        task_summary: String,
        #[serde(default)]
        context: Option<String>,
    }

    let args: DelegateArgs = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("[delegate] 解析参数失败: {} (原始: {})", e, arguments);
            return (
                format!(
                    "参数解析失败：{}。请检查 target_role_id 和 task_summary。",
                    e
                ),
                DelegationRecord {
                    target_role_id: String::new(),
                    target_role_name: "未知".to_string(),
                    target_conversation_id: None,
                    task_summary: String::new(),
                    status: "parse_error".to_string(),
                },
            );
        }
    };

    // AC-8: 校验角色有效性
    let role = match crate::db::roles::get_role(main_pool, &args.target_role_id).await {
        Ok(r) if r.status == "active" => r,
        Ok(r) => {
            tracing::warn!(
                "[delegate] 目标角色 {} 已归档（status={}），拒绝委派",
                args.target_role_id,
                r.status
            );
            return (
                "目标角色已归档，无法委派。请用文字直接告诉用户该角色不可用。".to_string(),
                DelegationRecord {
                    target_role_id: args.target_role_id,
                    target_role_name: r.name,
                    target_conversation_id: None,
                    task_summary: args.task_summary,
                    status: "role_not_found".to_string(),
                },
            );
        }
        Err(_) => {
            tracing::warn!("[delegate] 目标角色 id={} 不存在", args.target_role_id);
            return (
                "目标角色不可用，请用文字直接告诉用户。".to_string(),
                DelegationRecord {
                    target_role_id: args.target_role_id,
                    target_role_name: "未知".to_string(),
                    target_conversation_id: None,
                    task_summary: args.task_summary,
                    status: "role_not_found".to_string(),
                },
            );
        }
    };

    // AC-4: 拼接委派消息写入角色对话
    let context_text = args.context.as_deref().unwrap_or("").trim();
    let user_content = if context_text.is_empty() {
        format!("[管家委派] {}", args.task_summary)
    } else {
        format!(
            "[管家委派] {}\n\n上下文：{}",
            args.task_summary, context_text
        )
    };

    let role_conv =
        match crate::db::conversations::get_or_create_conversation_by_role(conv_pool, &role.id)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[delegate] 获取/创建角色会话失败: {}", e);
                return (
                    "角色处理失败（无法准备会话），请告诉用户稍后再试。".to_string(),
                    DelegationRecord {
                        target_role_id: role.id,
                        target_role_name: role.name,
                        target_conversation_id: None,
                        task_summary: args.task_summary,
                        status: "conv_init_error".to_string(),
                    },
                );
            }
        };

    if let Err(e) = crate::db::conversations::insert_message(
        conv_pool,
        &role_conv.id,
        "user",
        &user_content,
        true,
    )
    .await
    {
        tracing::error!("[delegate] 写入委派 user 消息失败: {}", e);
        return (
            "角色处理失败（无法记录委派），请告诉用户稍后再试。".to_string(),
            DelegationRecord {
                target_role_id: role.id,
                target_role_name: role.name,
                target_conversation_id: Some(role_conv.id.clone()),
                task_summary: args.task_summary,
                status: "user_msg_insert_error".to_string(),
            },
        );
    }

    let provider = match resolve_default_provider(main_pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("[delegate] 获取 provider 失败: {}", e);
            return (
                "角色处理失败（模型未就绪），请告诉用户稍后再试。".to_string(),
                DelegationRecord {
                    target_role_id: role.id,
                    target_role_name: role.name,
                    target_conversation_id: Some(role_conv.id.clone()),
                    task_summary: args.task_summary,
                    status: "provider_error".to_string(),
                },
            );
        }
    };

    let role_assistant_msg = match crate::db::conversations::insert_message(
        conv_pool,
        &role_conv.id,
        "assistant",
        "",
        false,
    )
    .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("[delegate] 创建角色 assistant 占位失败: {}", e);
            return (
                "角色处理失败（无法准备回复），请告诉用户稍后再试。".to_string(),
                DelegationRecord {
                    target_role_id: role.id,
                    target_role_name: role.name,
                    target_conversation_id: Some(role_conv.id.clone()),
                    task_summary: args.task_summary,
                    status: "assistant_msg_insert_error".to_string(),
                },
            );
        }
    };

    // 跑角色 LLM —— 用 build_role_messages 与管家分支同源 system prompt
    let role_messages =
        match build_role_messages(conv_pool, main_pool, &role_conv.id, &role.id, &user_content)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("[delegate] 构建角色消息失败: {}", e);
                return (
                    "角色处理失败（无法构造上下文），请告诉用户稍后再试。".to_string(),
                    DelegationRecord {
                        target_role_id: role.id,
                        target_role_name: role.name,
                        target_conversation_id: Some(role_conv.id.clone()),
                        task_summary: args.task_summary,
                        status: "build_messages_error".to_string(),
                    },
                );
            }
        };

    // 关键：本地化 drain，不 emit `llm:stream`（避免污染管家 conversation 流，见 Dev Notes）
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let stream_result = provider.chat_stream(
        role_messages,
        tx,
        ChatOptions {
            disable_thinking: true,
            tools: None,
            tool_choice: None,
        },
    );
    tokio::pin!(stream_result);

    let mut role_reply = String::new();
    loop {
        tokio::select! {
            result = &mut stream_result => {
                if let Err(e) = result {
                    tracing::error!("[delegate] 角色流式调用失败: {}", e);
                }
                break;
            }
            event = rx.recv() => {
                match event {
                    Some(StreamEvent::Token(t)) => role_reply.push_str(&t),
                    Some(StreamEvent::Done) => break,
                    Some(StreamEvent::Error(e)) => {
                        tracing::error!("[delegate] 角色流出错: {}", e);
                        break;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
        }
    }

    let role_reply = if role_reply.trim().is_empty() {
        format!("（角色「{}」没有给出明确回应）", role.name)
    } else {
        role_reply
    };

    // 持久化角色 assistant 消息
    let _ = crate::db::conversations::update_message_content(
        conv_pool,
        &role_assistant_msg.id,
        &role_reply,
    )
    .await;
    let _ =
        crate::db::conversations::mark_message_complete(conv_pool, &role_assistant_msg.id).await;

    // 返回给管家 LLM 的 tool result：明确这是来自哪个角色的回复，引导管家转述
    let tool_result = format!(
        "来自角色「{}」的回复：\n{}\n\n（请用自己的话向用户转述这段反馈，必要时说明这是来自「{}」的建议。）",
        role.name, role_reply, role.name
    );
    (
        tool_result,
        DelegationRecord {
            target_role_id: role.id,
            target_role_name: role.name,
            target_conversation_id: Some(role_conv.id),
            task_summary: args.task_summary,
            status: "ok".to_string(),
        },
    )
}

async fn execute_create_role(
    app_handle: &tauri::AppHandle,
    _main_pool: &DbPool,
    conversation_id: &str,
    arguments: &str,
) -> String {
    #[derive(serde::Deserialize)]
    struct CreateRoleArgs {
        name: String,
        icon: Option<String>,
        color: Option<String>,
        goal: Option<String>,
    }

    let args: CreateRoleArgs = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("解析 create_role 参数失败: {} (原始: {})", e, arguments);
            return format!("参数解析失败: {}", e);
        }
    };

    // 校验/归一化 icon 与 color。落到白名单之外的值用 None 回退（前端会用默认值）。
    let normalized_icon = args.icon.as_deref().and_then(|s| {
        let t = s.trim();
        if SUPPORTED_ICONS.iter().any(|w| *w == t) {
            Some(t.to_string())
        } else {
            tracing::warn!("LLM 返回了不在白名单中的 icon: {:?}, 已回退", t);
            None
        }
    });
    let normalized_color = args.color.as_deref().and_then(|s| {
        let t = s.trim().to_ascii_uppercase();
        if SUPPORTED_COLORS.iter().any(|w| w.eq_ignore_ascii_case(&t)) {
            Some(t)
        } else {
            tracing::warn!("LLM 返回了不在白名单中的 color: {:?}, 已回退", s);
            None
        }
    });

    tracing::info!(
        "[propose] 收到角色提议: name={} icon={:?} color={:?} goal_len={}",
        args.name,
        normalized_icon,
        normalized_color,
        args.goal.as_deref().map(|s| s.len()).unwrap_or(0),
    );

    // 不写库，仅向前端发提议事件；前端弹窗由用户确认后再调 roleService.create()
    match app_handle.emit(
        "role:proposed",
        RoleProposedPayload {
            conversation_id: conversation_id.to_string(),
            name: args.name.clone(),
            icon: normalized_icon,
            color: normalized_color,
            goal: args.goal.clone(),
        },
    ) {
        Ok(()) => {
            tracing::info!(
                "[propose] role_proposed_emitted: conv={} name={}",
                conversation_id,
                args.name
            );
            format!("role_proposal_emitted:{}", args.name)
        }
        Err(e) => {
            tracing::error!(
                "[propose] role_proposed_emit_failed: conv={} name={} error={}",
                conversation_id,
                args.name,
                e
            );
            format!("角色提议发送失败：{}", e)
        }
    }
}

/// Story 2.5: 记录用户拒绝了涌现角色建议，写入冷却。
async fn execute_record_emergence_rejection(main_pool: &DbPool, arguments: &str) -> String {
    #[derive(serde::Deserialize)]
    struct RejectArgs {
        domain: String,
    }

    let args: RejectArgs = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(
                "[emergence] 解析 rejection 参数失败: {} (原始: {})",
                e,
                arguments
            );
            return format!("参数解析失败: {}", e);
        }
    };

    let domain = args.domain.trim();
    if domain.is_empty() {
        return "领域描述为空，未记录冷却。".to_string();
    }

    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = crate::db::app_settings::set_emergence_cooldown(main_pool, domain, &now).await {
        tracing::error!("[emergence] 写入冷却记录失败: {}", e);
        return "已记录用户拒绝（存储失败但不影响对话）。".to_string();
    }

    // 顺带清理过期记录
    let _ = crate::db::app_settings::clear_expired_cooldowns(main_pool, 7).await;

    tracing::info!("[emergence] 记录涌现拒绝: domain={}", domain);
    format!(
        "已记录：用户拒绝了「{}」领域的角色建议，7天内不再提议此领域。请自然地继续对话。",
        domain
    )
}

// ========== 方案 A：后备文本检测 + 二次提取 ==========

/// 启发式检测 assistant 文本是否在「假装」执行了角色创建。
/// 当出现典型创建语义的关键词组合时返回 true。
fn looks_like_fake_role_creation(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let strong_signals = [
        "**创建角色**",
        "角色名称：",
        "角色名称:",
        "角色创建成功",
        "已为您创建",
        "已经为您创建",
        "已为你创建",
        "已经为你创建",
        "角色已创建",
        "创建中...",
        "创建中…",
    ];
    if strong_signals.iter().any(|s| text.contains(s)) {
        return true;
    }
    // 弱信号组合：同时出现「创建」/「建好」 + 「角色」 + 「成功」/「完成」
    let has_create = text.contains("创建") || text.contains("建好") || text.contains("建立");
    let has_role = text.contains("角色");
    let has_done = text.contains("成功")
        || text.contains("完成")
        || text.contains("好了")
        || text.contains("OK")
        || text.contains("ok");
    has_create && has_role && has_done
}

/// 兜底解析：从任意文本中尝试抽出 `{ "name":..., "icon":..., "color":..., "goal":... }` 形式的 JSON。
/// 处理 markdown 代码块包裹、前后文字噪音的情况。
fn parse_role_json(text: &str) -> Option<CreateRoleInput> {
    let cleaned = text
        .replace("```json", "")
        .replace("```JSON", "")
        .replace("```", "");
    // 找第一个 '{' 与之后最远的 '}'
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')?;
    if end <= start {
        return None;
    }
    let candidate = &cleaned[start..=end];

    #[derive(serde::Deserialize)]
    struct ExtractedRole {
        name: Option<String>,
        icon: Option<String>,
        color: Option<String>,
        goal: Option<String>,
    }
    let parsed: ExtractedRole = serde_json::from_str(candidate).ok()?;
    let name = parsed.name?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(CreateRoleInput {
        name,
        icon: parsed.icon.filter(|s| !s.trim().is_empty()),
        color: parsed.color.filter(|s| !s.trim().is_empty()),
        goal: parsed.goal.filter(|s| !s.trim().is_empty()),
    })
}

/// 二次 LLM 调用：把对话历史 + 最后那条「假装创建」的 assistant 文本投喂给模型，
/// 要求它只输出一行 JSON，再解析为 CreateRoleInput。
async fn try_fallback_extract_role(
    provider: &Arc<dyn LlmProvider>,
    onboarding_messages: &[ChatCompletionMessage],
    assistant_text: &str,
) -> Option<CreateRoleInput> {
    let mut messages: Vec<ChatCompletionMessage> = Vec::new();
    messages.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: "你是一个严格的 JSON 提取器。仅输出一行 JSON，不要任何解释、前后缀或代码块包裹。"
            .to_string(),
        tool_calls: None,
        tool_call_id: None,
    });
    // 仅保留对话内容（去掉原 onboarding system 提示，避免冲突）
    for m in onboarding_messages.iter().filter(|m| m.role != "system") {
        messages.push(m.clone());
    }
    // 加入这次"假装创建"的 assistant 输出
    messages.push(ChatCompletionMessage {
        role: "assistant".to_string(),
        content: assistant_text.to_string(),
        tool_calls: None,
        tool_call_id: None,
    });
    messages.push(ChatCompletionMessage {
        role: "user".to_string(),
        content: "请从以上对话中提取要创建的角色信息，只输出一行 JSON，键固定为 name/icon/color/goal。\
                  name 必填（中文角色名）；icon 从这些标识符里选一个最贴合的：briefcase/code/chart-bar/palette/pen-tool/book-open/graduation-cap/dumbbell/heart-pulse/leaf/home/users/baby/gamepad-2/music/camera/plane/utensils/coffee/target/sparkles/lightbulb/compass/wallet；color 从这些 hex 里选最贴合：#4F46E5/#0EA5E9/#10B981/#F59E0B/#EF4444/#8B5CF6/#EC4899/#64748B；goal 为一句话。\
                  如果无法确定角色名，输出 {}。".to_string(),
        tool_calls: None,
        tool_call_id: None,
    });

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let provider_clone = provider.clone();
    tokio::spawn(async move {
        if let Err(e) = provider_clone
            .chat_stream(
                messages,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::error!("[fallback] 二次提取流式调用失败: {}", e);
        }
    });

    let mut full = String::new();
    while let Some(ev) = rx.recv().await {
        match ev {
            StreamEvent::Token(t) => full.push_str(&t),
            StreamEvent::Done => break,
            StreamEvent::Error(e) => {
                tracing::error!("[fallback] 二次提取出错: {}", e);
                return None;
            }
            _ => {}
        }
    }

    tracing::info!("[fallback] 二次提取原始输出: {:?}", full);
    let role = parse_role_json(&full);
    if role.is_none() {
        tracing::warn!("[fallback] 二次提取 JSON 解析失败，放弃创建");
    }
    role
}

fn summarize_error(err: &str) -> &str {
    if err.contains("401") || err.contains("无效") {
        "API Key 无效或已过期"
    } else if err.contains("429") || err.contains("额度") {
        "请求额度不足"
    } else if err.contains("超时") || err.contains("timeout") {
        "连接超时"
    } else if err.contains("连接") || err.contains("connect") {
        "无法连接到模型服务"
    } else {
        "模型服务暂时不可用"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delegation_bubble_state_splits_into_two_bubbles() {
        // WHY: 管家委派两段（"稍等…" / 委派回复）来自不同 opencode messageID，
        // 必须路由到两个气泡。实测时间线：msg_A 先出现并产出第一段文本，
        // delegate_to_role 进入 running 时切第二气泡（唤醒弹跳点），随后 msg_B 产出第二段。
        let mut state = DelegationBubbleState::default();
        // round1：第一条 assistant 消息的文本属于第一气泡
        assert_eq!(state.classify_message("msg_A"), BubbleSlot::First);
        assert_eq!(state.classify_message("msg_A"), BubbleSlot::First);
        // delegate_to_role 进入 running → 第一次返回 true（调用方据此建 followup 气泡 + 发空 token 唤醒弹跳点）
        assert!(state.note_delegate_active());
        // completed 再次上报同一工具 → 不应再次触发 split
        assert!(!state.note_delegate_active());
        // round2：委派回复来自新的 messageID，路由到 followup 气泡
        assert_eq!(state.classify_message("msg_B"), BubbleSlot::Followup);
        assert_eq!(state.classify_message("msg_B"), BubbleSlot::Followup);
    }

    #[test]
    fn test_delegation_bubble_state_no_split_without_delegation() {
        // WHY: 普通单段对话（无委派）即使 opencode 偶发多条 assistant 消息，
        // 也不得拆分气泡——保持改动前的单气泡语义，避免回归。
        let mut state = DelegationBubbleState::default();
        assert_eq!(state.classify_message("msg_X"), BubbleSlot::First);
        assert_eq!(state.classify_message("msg_Y"), BubbleSlot::First);
    }

    #[test]
    fn test_opencode_agent_key_routes_butler_and_roles() {
        // WHY: butler sessions use the "butler" agent key so opencode enables
        // egosync* MCP tools; roles use a prefixed key.
        assert_eq!(opencode_agent_key(None), "butler");
        assert_eq!(opencode_agent_key(Some("abc-123")), "role-abc-123");
    }

    #[test]
    fn test_remember_opencode_session_reuses_existing_conversation_session() {
        // WHY: one EgoSync conversation must keep one opencode session so the
        // Agent Loop retains context across turns instead of starting fresh.
        let mut sessions = std::collections::HashMap::new();
        let first = remember_opencode_session(&mut sessions, "conv-1", "session-a");
        let second = remember_opencode_session(&mut sessions, "conv-1", "session-b");

        assert_eq!(first, "session-a");
        assert_eq!(second, "session-a");
        assert_eq!(
            sessions.get("conv-1").map(String::as_str),
            Some("session-a")
        );
    }

    #[test]
    fn test_sse_text_maps_to_stream_payload_contract() {
        // WHY: ChatStream depends on this exact payload shape; changing it would
        // break streaming without any frontend compile error.
        let payload = stream_payload_from_sse(
            "conv-1",
            Some("msg-1"),
            crate::models::agent::SseEvent::Text {
                content: "hello".to_string(),
            },
        )
        .expect("text should map to payload");

        assert_eq!(payload.conversation_id, "conv-1");
        assert_eq!(payload.token, "hello");
        assert!(!payload.done);
        assert!(!payload.thinking);
        assert_eq!(payload.message_id.as_deref(), Some("msg-1"));
    }

    #[test]
    fn test_sse_done_maps_to_stream_completion() {
        // WHY: the frontend only unlocks the input after a done payload or
        // history refresh; opencode completion must preserve that contract.
        let payload = stream_payload_from_sse("conv-1", None, crate::models::agent::SseEvent::Done)
            .expect("done should map to payload");

        assert_eq!(payload.token, "");
        assert!(payload.done);
        assert!(!payload.thinking);
        assert!(payload.message_id.is_none());
    }

    #[test]
    fn test_sse_thinking_maps_to_thinking_payload() {
        // WHY: opencode reasoning must stay visually separated from final text;
        // otherwise ChatStream would render internal thinking as normal content.
        let payload = stream_payload_from_sse(
            "conv-1",
            Some("msg-1"),
            crate::models::agent::SseEvent::Thinking {
                content: "plan".to_string(),
            },
        )
        .expect("thinking should map to payload");

        assert_eq!(payload.token, "plan");
        assert!(!payload.done);
        assert!(payload.thinking);
        assert_eq!(payload.message_id.as_deref(), Some("msg-1"));
    }

    #[test]
    fn test_sse_error_maps_to_done_payload() {
        // WHY: opencode stream errors must still close the streaming UI so the
        // user can recover and send the next message.
        let payload = stream_payload_from_sse(
            "conv-1",
            None,
            crate::models::agent::SseEvent::Error {
                message: "timeout".to_string(),
            },
        )
        .expect("error should map to payload");

        assert!(payload.done);
        assert!(!payload.thinking);
        assert!(payload.token.contains("Agent 引擎返回错误"));
    }

    #[test]
    fn non_json_tool_output_preview_truncates_on_char_boundary() {
        let output = "产品经理已经收到委派任务，并给出了真实回复。";
        let preview = non_json_tool_output_preview(output);

        assert_eq!(preview, output);
    }

    #[test]
    fn test_butler_system_prompt_not_empty() {
        assert!(!BUTLER_SYSTEM_PROMPT.is_empty());
        assert!(BUTLER_SYSTEM_PROMPT.contains("管家"));
    }

    #[test]
    fn test_onboarding_prompt_defined() {
        assert!(!ONBOARDING_SYSTEM_PROMPT.is_empty());
        assert!(ONBOARDING_SYSTEM_PROMPT.contains("引导"));
        assert!(ONBOARDING_SYSTEM_PROMPT.contains("create_role"));
        assert!(ONBOARDING_SYSTEM_PROMPT.contains("行为红线"));
    }

    #[test]
    fn test_summarize_error_categories() {
        assert_eq!(summarize_error("401 Unauthorized"), "API Key 无效或已过期");
        assert_eq!(
            summarize_error("429 Too Many Requests 额度不足"),
            "请求额度不足"
        );
        assert_eq!(summarize_error("连接超时"), "连接超时");
        assert_eq!(
            summarize_error("无法连接到 api.openai.com"),
            "无法连接到模型服务"
        );
        assert_eq!(summarize_error("unknown error"), "模型服务暂时不可用");
    }

    #[test]
    fn test_create_role_tool_definition() {
        let tool = create_role_tool_definition();
        assert_eq!(tool.name, "create_role");
        assert!(tool.description.contains("创建"));
        let params = tool.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["name"].is_object());
        assert!(params["properties"]["icon"].is_object());
        assert!(params["properties"]["color"].is_object());
        assert!(params["properties"]["goal"].is_object());
    }

    // ===== 方案 A：兜底逻辑相关测试 =====

    #[test]
    fn test_looks_like_fake_role_creation_strong_signals() {
        // 用户实际遇到的伪造文本
        let fake1 = "**创建角色**\n角色名称：产品经理\n创建中...\n角色创建成功";
        assert!(looks_like_fake_role_creation(fake1));

        assert!(looks_like_fake_role_creation(
            "好的，我已为您创建「健身教练」角色。"
        ));
        assert!(looks_like_fake_role_creation("已经为你创建好了"));
        assert!(looks_like_fake_role_creation("角色已创建，祝你顺利"));
        assert!(looks_like_fake_role_creation("正在创建中…"));
    }

    #[test]
    fn test_looks_like_fake_role_creation_weak_combo() {
        // 弱信号组合（创建 + 角色 + 成功/完成）
        assert!(looks_like_fake_role_creation(
            "好的，我帮你建立这个角色，已经完成。"
        ));
        assert!(looks_like_fake_role_creation("角色创建成功！"));
    }

    #[test]
    fn test_looks_like_fake_role_creation_negative_cases() {
        // 正常 onboarding 引导语不应触发
        assert!(!looks_like_fake_role_creation(""));
        assert!(!looks_like_fake_role_creation("你好，请问怎么称呼你？"));
        assert!(!looks_like_fake_role_creation(
            "你最近在忙什么？工作还是生活方面有什么特别关注的事情？"
        ));
        assert!(!looks_like_fake_role_creation(
            "听起来你在产品方面投入很多，我帮你创建一个「产品经理」角色来管理相关事务，怎么样？"
        ));
        // 提议但还没说"成功/完成"，不应触发
        assert!(!looks_like_fake_role_creation("我建议创建一个角色"));
    }

    #[test]
    fn test_parse_role_json_plain() {
        let s = r##"{"name":"产品经理","icon":"📋","color":"#4F46E5","goal":"打磨产品"}"##;
        let r = parse_role_json(s).expect("应能解析");
        assert_eq!(r.name, "产品经理");
        assert_eq!(r.icon.as_deref(), Some("📋"));
        assert_eq!(r.color.as_deref(), Some("#4F46E5"));
        assert_eq!(r.goal.as_deref(), Some("打磨产品"));
    }

    #[test]
    fn test_parse_role_json_with_markdown_fence() {
        let s = "```json\n{\"name\":\"健身教练\",\"icon\":\"💪\",\"color\":\"#10B981\",\"goal\":\"保持健康\"}\n```";
        let r = parse_role_json(s).expect("应能解析含 fence 的 JSON");
        assert_eq!(r.name, "健身教练");
        assert_eq!(r.icon.as_deref(), Some("💪"));
    }

    #[test]
    fn test_parse_role_json_with_surrounding_text() {
        let s = "好的，这是提取结果：\n{\"name\":\"父亲\",\"icon\":\"👨\",\"color\":\"#F59E0B\",\"goal\":\"陪伴家人\"}\n谢谢。";
        let r = parse_role_json(s).expect("应能从前后噪音里抠出 JSON");
        assert_eq!(r.name, "父亲");
    }

    #[test]
    fn test_parse_role_json_only_name() {
        let s = r##"{"name":"读者"}"##;
        let r = parse_role_json(s).expect("仅 name 也应成功");
        assert_eq!(r.name, "读者");
        assert!(r.icon.is_none());
        assert!(r.color.is_none());
        assert!(r.goal.is_none());
    }

    #[test]
    fn test_parse_role_json_empty_name_rejected() {
        let s = r##"{"name":"   "}"##;
        assert!(parse_role_json(s).is_none());
    }

    #[test]
    fn test_parse_role_json_empty_object_rejected() {
        // 二次提取协议里规定无法确定时输出 {}，应被拒绝
        assert!(parse_role_json("{}").is_none());
    }

    #[test]
    fn test_parse_role_json_garbage_rejected() {
        assert!(parse_role_json("not a json").is_none());
        assert!(parse_role_json("").is_none());
        assert!(parse_role_json("{").is_none());
    }

    #[test]
    fn test_parse_role_json_trims_blank_optional_fields() {
        let s = r##"{"name":"作家","icon":"","color":"  ","goal":"写作"}"##;
        let r = parse_role_json(s).expect("应能解析");
        assert_eq!(r.name, "作家");
        assert!(r.icon.is_none(), "空 icon 应被过滤为 None");
        assert!(r.color.is_none(), "空白 color 应被过滤为 None");
        assert_eq!(r.goal.as_deref(), Some("写作"));
    }

    // ===== AC-2 / AC-6: 角色 system prompt 注入 =====

    use crate::db::pool::ConversationsPool;
    use crate::models::role::CreateRoleInput;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn setup_test_main_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test main db");

        sqlx::query(
            "CREATE TABLE roles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT '🎯',
                color TEXT NOT NULL DEFAULT '#6366F1',
                goal TEXT NOT NULL DEFAULT '',
                personality_prompt TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'active',
                energy INTEGER NOT NULL DEFAULT 100,
                skills_config TEXT NOT NULL DEFAULT '{}',
                proactivity_level TEXT NOT NULL DEFAULT 'moderate',
                archived_at TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create roles table");

        sqlx::raw_sql(include_str!("../../migrations/004_memories.sql"))
            .execute(&pool)
            .await
            .expect("failed to create memories table");

        pool
    }

    async fn setup_test_conv_pool() -> ConversationsPool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create test conv db");

        let schema = include_str!("../../migrations/002_conversations.sql");
        sqlx::raw_sql(schema)
            .execute(&pool)
            .await
            .expect("failed to apply conv schema");
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN thinking_content TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("failed to add thinking_content");
        sqlx::raw_sql("ALTER TABLE conversations ADD COLUMN title TEXT NOT NULL DEFAULT ''")
            .execute(&pool)
            .await
            .expect("failed to add title");

        // Story 2.3: 与 run_conversations_migrations 同步，否则 list_messages SELECT 会爆
        sqlx::raw_sql("ALTER TABLE messages ADD COLUMN routing_metadata TEXT")
            .execute(&pool)
            .await
            .expect("failed to add routing_metadata");

        ConversationsPool(pool)
    }

    /// AC-6: role-aware system prompt 必须含 role.name 与 goal —
    /// 这是 FR-6 「角色个性化语调」的载体。如果 prompt 没有这些字段，
    /// LLM 会退化成通用管家口吻，用户切换角色就感觉不到差异。
    #[tokio::test]
    async fn test_build_role_messages_injects_name_goal_and_personality_layers() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("打磨产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let role = crate::db::roles::update_role(
            &main_pool,
            &role.id,
            &crate::models::role::UpdateRoleInput {
                name: None,
                icon: None,
                color: None,
                goal: None,
                personality_prompt: Some("简洁专业，先判断优先级再给建议".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "你好")
            .await
            .unwrap();

        let system = msgs.first().expect("应至少有 system prompt");
        assert_eq!(system.role, "system");
        assert!(
            system.content.contains("产品经理"),
            "system prompt 必须含角色名"
        );
        assert!(
            system.content.contains("打磨产品节奏"),
            "system prompt 必须含角色目标"
        );
        assert!(
            system.content.contains("简洁专业，先判断优先级再给建议"),
            "system prompt 必须含个性描述"
        );
        assert!(system.content.contains("[role_definition]"));
        assert!(system.content.contains("[context_injection]"));
        assert!(
            !system.content.contains("你是 EgoSync 的数字管家"),
            "角色 prompt 不应含管家身份"
        );
        assert!(
            system.content.contains("分身"),
            "角色 prompt 必须明确是用户的分身"
        );
    }

    /// AC-6: personality_prompt 为空时不应在 prompt 末尾留空行或 `personality:` 残骸 —
    /// 早期角色没有人格描述，prompt 必须仍然干净，否则 LLM 会被 trailing 空白困惑。
    #[tokio::test]
    async fn test_build_role_messages_skips_empty_personality() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "学习者".to_string(),
                icon: None,
                color: None,
                goal: Some("保持学习节奏".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&role.id))
            .await
            .unwrap();

        let msgs = build_role_messages(&conv_pool, &main_pool, &conv.id, &role.id, "")
            .await
            .unwrap();

        let system = &msgs.first().unwrap().content;
        assert!(
            system.contains("默认语调"),
            "personality_prompt 为空时应提供轻量默认语调方向"
        );
        assert!(
            system.contains("[context_injection]"),
            "prompt 应保留三层结构中的上下文注入边界"
        );
    }

    /// AC-6: 若传入不存在的 role_id 必须立刻 fail-loud 返回 NotFound —
    /// 否则 LLM 会用空 prompt 静默裸聊，用户体验等同于"角色不存在但你看不见"，
    /// 与 PRD 透明可控原则相违。
    #[tokio::test]
    async fn test_build_role_messages_unknown_role_returns_not_found() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let conv = crate::db::conversations::create_conversation(&conv_pool, Some("ghost"))
            .await
            .unwrap();

        let result = build_role_messages(&conv_pool, &main_pool, &conv.id, "ghost", "hi").await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ===== Story 2.3 AC-7: 跨角色全局摘要 =====

    /// AC-7 退化场景：无 active 角色时管家不应被拼一个空标题段误导。
    /// 返回空串是 caller 决定是否拼接的契约 —— 不能返回 `[各角色近况]\n`。
    #[tokio::test]
    async fn test_butler_memory_summary_includes_global_and_role_memories() {
        let main_pool = setup_test_main_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, status) VALUES ('role-1', '父亲', 'baby', '#EF4444', '成为小孩的榜样', 'active')",
        )
        .execute(&main_pool)
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            None,
            "global-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "preference".to_string(),
                content: "用户希望用中文沟通".to_string(),
                source_message_ids: vec!["global-msg".to_string()],
            }],
        )
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some("role-1"),
            "role-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "儿子喜欢书法课".to_string(),
                source_message_ids: vec!["role-msg".to_string()],
            }],
        )
        .await
        .unwrap();

        let summary = build_butler_memory_summary(&main_pool).await.unwrap();

        assert!(summary.contains("[已知记忆]"));
        assert!(summary.contains("全局记忆"));
        assert!(summary.contains("用户希望用中文沟通"));
        assert!(summary.contains("父亲"));
        assert!(summary.contains("儿子喜欢书法课"));
    }

    #[tokio::test]
    async fn test_butler_prompt_tells_model_to_answer_from_memory_before_delegation() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        sqlx::query(
            "INSERT INTO roles (id, name, icon, color, goal, status) VALUES ('role-1', '父亲', 'baby', '#EF4444', '成为小孩的榜样', 'active')",
        )
        .execute(&main_pool)
        .await
        .unwrap();
        crate::db::memories::insert_memories(
            &main_pool,
            Some("role-1"),
            "role-conv",
            &[crate::models::memory::ExtractedMemory {
                category: "fact".to_string(),
                content: "儿子喜欢书法课".to_string(),
                source_message_ids: vec!["role-msg".to_string()],
            }],
        )
        .await
        .unwrap();

        let prompt = build_butler_system_prompt(&conv_pool, &main_pool)
            .await
            .unwrap();

        assert!(prompt.contains("儿子喜欢书法课"));
        assert!(prompt.contains("陈述某个角色相关事实或偏好"));
        assert!(prompt.contains("直接用自然口吻确认"));
        assert!(prompt.contains("不要向用户暴露内部机制"));
        assert!(prompt.contains("不要说“系统会同步”“同步到某角色”“角色已收到”“委派成功”“工具调用”等"));
        assert!(!prompt.contains("系统会把这类记忆同步到对应角色"));
        assert!(prompt.contains("角色相关任务、安排、日程、待办、规划或需要跟进"));
        assert!(prompt.contains("只要能匹配 active 角色，就调用 delegate_to_role"));
        assert!(!prompt.contains("当用户的需求清晰指向某个角色时"));
    }

    #[tokio::test]
    async fn test_cross_role_summary_empty_when_no_roles() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.is_empty(), "无角色时不能输出任何文字");
    }

    /// AC-7 退化场景：有 active 角色但还没人跟它说过话 → 仍然返回空串。
    /// 不能让一个"什么都没发生"的角色出现在管家 prompt 里，否则管家会被迫复述虚无。
    #[tokio::test]
    async fn test_cross_role_summary_empty_when_role_has_no_messages() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "学习者".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.is_empty(), "角色无对话历史时不应进入摘要");
    }

    /// AC-7 核心：用户主动跟角色聊过后回到管家，管家必须能看到那段对话。
    /// 否则用户回管家说"我刚才跟产品经理聊了啥"管家就要装失忆。
    #[tokio::test]
    async fn test_cross_role_summary_includes_recent_messages_per_role() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("打磨产品".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::create_conversation(&conv_pool, Some(&pm.id))
            .await
            .unwrap();
        crate::db::conversations::insert_message(
            &conv_pool,
            &conv.id,
            "user",
            "本周 OKR 怎么排",
            true,
        )
        .await
        .unwrap();
        crate::db::conversations::insert_message(
            &conv_pool,
            &conv.id,
            "assistant",
            "建议先看用户访谈",
            true,
        )
        .await
        .unwrap();

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.contains("[各角色近况]"));
        assert!(summary.contains("产品经理"), "摘要必须含角色名");
        assert!(summary.contains("本周 OKR 怎么排"), "user 消息必须可见");
        assert!(
            summary.contains("建议先看用户访谈"),
            "assistant 消息必须可见"
        );
    }

    /// AC-7 多角色：用户在 N 个角色私聊后回管家，摘要必须涵盖所有有历史的角色。
    /// 漏掉任何一个就是管家"偏听偏信"，破坏全局视野承诺。
    #[tokio::test]
    async fn test_cross_role_summary_covers_multiple_roles() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        for name in ["产品经理", "学习者"] {
            let r = crate::db::roles::create_role(
                &main_pool,
                &CreateRoleInput {
                    name: name.to_string(),
                    icon: None,
                    color: None,
                    goal: None,
                },
            )
            .await
            .unwrap();
            let c = crate::db::conversations::create_conversation(&conv_pool, Some(&r.id))
                .await
                .unwrap();
            crate::db::conversations::insert_message(
                &conv_pool,
                &c.id,
                "user",
                &format!("hi {}", name),
                true,
            )
            .await
            .unwrap();
        }

        let summary = build_cross_role_summary(&conv_pool, &main_pool)
            .await
            .unwrap();
        assert!(summary.contains("产品经理"));
        assert!(summary.contains("学习者"));
    }

    // ===== Story 2.3 AC-1 / AC-5: delegate_to_role 工具与 butler prompt 增强 =====

    /// AC-1: 工具定义必须能让 LLM 在管家视图明确传达"我要把任务派给谁、派什么"。
    /// 缺 target_role_id 或 task_summary 会导致后端无法定位角色或写入审计 —— 缺字段不可妥协。
    #[test]
    fn test_delegate_to_role_tool_definition_has_required_fields() {
        let tool = delegate_to_role_tool_definition();
        assert_eq!(tool.name, "delegate_to_role");
        let params = tool.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["target_role_id"].is_object());
        assert!(params["properties"]["task_summary"].is_object());
        assert!(params["properties"]["context"].is_object());

        let required = params["required"].as_array().expect("required 应为数组");
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            required_strs.contains(&"target_role_id"),
            "target_role_id 必填"
        );
        assert!(required_strs.contains(&"task_summary"), "task_summary 必填");
        assert!(!required_strs.contains(&"context"), "context 仅可选");
    }

    /// AC-1: butler prompt 必须把可委派的角色 id+name+goal 全部展示给 LLM —
    /// 否则 LLM 只能瞎猜 id，导致 AC-8 的"目标无效"分支被大量触发。
    #[tokio::test]
    async fn test_build_butler_messages_includes_role_roster() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("打磨产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "帮我看下 OKR")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("[可委派角色清单]"), "需要可委派清单段落");
        assert!(
            system.contains(&pm.id),
            "角色 id 必须可见，否则 LLM 无法引用"
        );
        assert!(system.contains("产品经理"), "角色名必须可见");
        assert!(system.contains("打磨产品节奏"), "角色目标必须可见");
        assert!(system.contains("[行为指南]"), "行为指南段落必须出现");
        assert!(system.contains("delegate_to_role"), "行为指南要点名工具");
    }

    /// AC-5 退化场景：零 active 角色时管家不应被"行为指南"诱导调用工具 —
    /// 否则 LLM 会持续触发空 tool_call 导致 follow-up 死循环。
    #[tokio::test]
    async fn test_build_butler_messages_omits_guidance_when_no_active_roles() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "你好")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(system.contains("数字管家"), "管家身份必须保留");
        assert!(!system.contains("[可委派角色清单]"));
        assert!(!system.contains("[行为指南]"));
        assert!(!system.contains("delegate_to_role"));
    }

    // ===== Story 2.3 AC-8: execute_delegate_to_role 兜底分支（不依赖 LLM provider）=====

    /// AC-8 核心：LLM 给出不存在的 target_role_id 时必须返回 role_not_found 记录，
    /// 不写入角色对话 —— 否则会污染一个不存在角色的对话历史，引发后续追溯混乱。
    #[tokio::test]
    async fn test_execute_delegate_role_not_found_returns_audit_only() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let args = serde_json::json!({
            "target_role_id": "ghost-role-id",
            "task_summary": "跟进 OKR"
        })
        .to_string();

        let (text, record) = execute_delegate_to_role(&main_pool, &conv_pool, &args).await;

        assert_eq!(record.status, "role_not_found");
        assert_eq!(record.target_role_id, "ghost-role-id");
        assert_eq!(record.task_summary, "跟进 OKR");
        assert!(
            text.contains("不可用"),
            "tool result 必须明确告知 LLM 该角色不可用"
        );

        // 关键：不应该创建任何 conversation/message —— 否则就污染了"不存在的角色"
        let convs =
            crate::db::conversations::list_conversations_by_role(&conv_pool, "ghost-role-id")
                .await
                .unwrap();
        assert!(convs.is_empty(), "无效角色不应留下任何对话痕迹");
    }

    /// AC-8 + AC-4 边界：归档角色不接受委派 —— 否则用户归档角色后还在被偷偷调用，
    /// 违反用户主动归档的明确意图（与 Story 2.1 归档语义一致）。
    #[tokio::test]
    async fn test_execute_delegate_archived_role_rejected() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let role = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "前同事".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        crate::db::roles::archive_role(&main_pool, &role.id)
            .await
            .unwrap();

        let args = serde_json::json!({
            "target_role_id": role.id.clone(),
            "task_summary": "敲门"
        })
        .to_string();

        let (_text, record) = execute_delegate_to_role(&main_pool, &conv_pool, &args).await;
        assert_eq!(
            record.status, "role_not_found",
            "已归档角色等同于『不可用』，必须走同一兜底分支"
        );
        // 同样不能给归档角色生成新对话
        let convs = crate::db::conversations::list_conversations_by_role(&conv_pool, &role.id)
            .await
            .unwrap();
        assert!(convs.is_empty());
    }

    /// AC-1 参数协议：parse 失败时返回 parse_error，不进入兜底业务路径 —
    /// 否则 LLM 可能用错误参数反复重试，污染审计日志。
    #[tokio::test]
    async fn test_execute_delegate_invalid_json_returns_parse_error() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let (text, record) = execute_delegate_to_role(&main_pool, &conv_pool, "not-json").await;
        assert_eq!(record.status, "parse_error");
        assert!(text.contains("参数解析失败"));
    }

    #[test]
    fn test_completed_tool_part_is_claimed_once_by_part_id() {
        let mut processed = std::collections::HashSet::new();
        let part_raw = serde_json::json!({
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "completed",
                "output": "{}"
            }
        });

        assert!(claim_completed_tool_part(
            &mut processed,
            "part-1",
            &part_raw
        ));
        assert!(!claim_completed_tool_part(
            &mut processed,
            "part-1",
            &part_raw
        ));
    }

    #[test]
    fn test_unfinished_tool_part_is_not_claimed() {
        let mut processed = std::collections::HashSet::new();
        let part_raw = serde_json::json!({
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "running",
                "output": "{}"
            }
        });

        assert!(!claim_completed_tool_part(
            &mut processed,
            "part-1",
            &part_raw
        ));
        assert!(processed.is_empty());
    }

    #[tokio::test]
    async fn test_delegate_worker_runs_in_background_and_serializes() {
        let lock = Arc::new(Mutex::new(()));
        let (first_started_tx, first_started_rx) = tokio::sync::oneshot::channel();
        let (release_first_tx, release_first_rx) = tokio::sync::oneshot::channel::<()>();
        let second_ran = Arc::new(Mutex::new(false));

        let first = spawn_delegate_worker(lock.clone(), async move {
            let _ = first_started_tx.send(());
            let _ = release_first_rx.await;
        });
        tokio::time::timeout(Duration::from_millis(100), first_started_rx)
            .await
            .expect("第一个后台委派任务应立即开始")
            .expect("第一个后台委派任务应发送 started 信号");

        let second_ran_for_task = second_ran.clone();
        let second = spawn_delegate_worker(lock, async move {
            *second_ran_for_task.lock().await = true;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!*second_ran.lock().await, "第二个委派任务必须等待同一锁");
        assert!(!first.is_finished(), "第一个委派任务不应阻塞调用方等待完成");

        release_first_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), first)
            .await
            .expect("第一个后台委派任务应完成")
            .expect("第一个后台委派任务不应 panic");
        tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("第二个后台委派任务应完成")
            .expect("第二个后台委派任务不应 panic");
        assert!(*second_ran.lock().await);
    }

    #[tokio::test]
    async fn test_duplicate_completed_delegate_tool_part_persists_once() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();
        let butler_user_msg = crate::db::conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "明天有产品设计评审",
            true,
        )
        .await
        .unwrap();
        let output = serde_json::json!({
            "action": "delegate_to_role",
            "target_role_id": pm.id,
            "task_summary": "准备明天的产品设计评审",
        })
        .to_string();
        let part_raw = serde_json::json!({
            "type": "tool",
            "tool": "delegate_to_role",
            "state": {
                "status": "completed",
                "output": output,
            }
        });
        let result_json: serde_json::Value =
            serde_json::from_str(part_raw["state"]["output"].as_str().unwrap()).unwrap();
        let mut processed = std::collections::HashSet::new();

        for _ in 0..2 {
            if claim_completed_tool_part(&mut processed, "part-1", &part_raw) {
                handle_delegate_tool_result(
                    &conv_pool,
                    &main_pool,
                    &butler_user_msg.id,
                    &result_json,
                )
                .await;
            }
        }

        let role_convs = crate::db::conversations::list_conversations_by_role(&conv_pool, &pm.id)
            .await
            .unwrap();
        let role_messages = crate::db::conversations::list_messages(&conv_pool, &role_convs[0].id)
            .await
            .unwrap();
        let delegated_user_messages = role_messages
            .iter()
            .filter(|m| m.role == "user" && m.content.contains("[管家委派]"))
            .count();
        assert_eq!(delegated_user_messages, 1);

        let messages = crate::db::conversations::list_messages(&conv_pool, &butler_conv.id)
            .await
            .unwrap();
        let metadata = messages
            .iter()
            .find(|m| m.id == butler_user_msg.id)
            .and_then(|m| m.routing_metadata.as_deref())
            .expect("routing_metadata 应存在");
        let parsed: serde_json::Value = serde_json::from_str(metadata).unwrap();
        assert_eq!(parsed["delegations"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_opencode_delegate_tool_part_persists_role_history_and_routing_metadata() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: Some("briefcase".to_string()),
                color: Some("#4F46E5".to_string()),
                goal: Some("把控产品节奏".to_string()),
            },
        )
        .await
        .unwrap();
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();
        let butler_user_msg = crate::db::conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "明天有产品设计评审",
            true,
        )
        .await
        .unwrap();

        let tool_output = serde_json::json!({
            "action": "delegate_to_role",
            "target_role_id": pm.id,
            "task_summary": "准备明天的产品设计评审",
            "context": "明天有产品设计评审"
        })
        .to_string();

        handle_delegate_tool_result(
            &conv_pool,
            &main_pool,
            &butler_user_msg.id,
            &serde_json::from_str(&tool_output).unwrap(),
        )
        .await;

        let role_convs = crate::db::conversations::list_conversations_by_role(&conv_pool, &pm.id)
            .await
            .unwrap();
        assert_eq!(role_convs.len(), 1, "opencode 委派必须创建/复用角色对话");
        let role_messages = crate::db::conversations::list_messages(&conv_pool, &role_convs[0].id)
            .await
            .unwrap();
        assert!(
            role_messages.iter().any(|m| {
                m.role == "user" && m.content.contains("[管家委派] 准备明天的产品设计评审")
            }),
            "opencode 委派必须写入角色 user 消息"
        );

        let butler_messages = crate::db::conversations::list_messages(&conv_pool, &butler_conv.id)
            .await
            .unwrap();
        let routed_user_msg = butler_messages
            .iter()
            .find(|m| m.id == butler_user_msg.id)
            .expect("触发委派的 user message 应仍存在");
        let metadata = routed_user_msg
            .routing_metadata
            .as_deref()
            .expect("opencode 委派必须写 routing_metadata");
        assert!(metadata.contains("\"targetRoleName\":\"产品经理\""));
        assert!(metadata.contains("\"taskSummary\":\"准备明天的产品设计评审\""));
        assert!(metadata.contains("\"status\":"));
    }

    #[tokio::test]
    async fn test_opencode_delegate_tool_result_appends_routing_metadata() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;
        let butler_conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();
        let butler_user_msg = crate::db::conversations::insert_message(
            &conv_pool,
            &butler_conv.id,
            "user",
            "安排产品评审和学习计划",
            true,
        )
        .await
        .unwrap();

        let pm = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();
        let learner = crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "学习者".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();

        for (role_id, task_summary) in [(&pm.id, "准备产品评审"), (&learner.id, "安排学习计划")]
        {
            let result = serde_json::json!({
                "action": "delegate_to_role",
                "target_role_id": role_id,
                "task_summary": task_summary,
            });
            handle_delegate_tool_result(&conv_pool, &main_pool, &butler_user_msg.id, &result).await;
        }

        let messages = crate::db::conversations::list_messages(&conv_pool, &butler_conv.id)
            .await
            .unwrap();
        let metadata = messages
            .iter()
            .find(|m| m.id == butler_user_msg.id)
            .and_then(|m| m.routing_metadata.as_deref())
            .expect("routing_metadata 应存在");
        let parsed: serde_json::Value = serde_json::from_str(metadata).unwrap();
        let delegations = parsed["delegations"].as_array().unwrap();
        assert_eq!(delegations.len(), 2);
        assert_eq!(delegations[0]["targetRoleName"], "产品经理");
        assert_eq!(delegations[1]["targetRoleName"], "学习者");
    }

    // ===== Story 2.5: 涌现工具与 prompt 增强 =====

    #[test]
    fn test_record_emergence_rejection_tool_definition() {
        let tool = record_emergence_rejection_tool_definition();
        assert_eq!(tool.name, "record_emergence_rejection");
        assert!(tool.description.contains("拒绝"));
        let params = tool.parameters;
        let required = params["required"].as_array().expect("required 应为数组");
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(required_strs.contains(&"domain"), "domain 必填");
    }

    /// Story 2.5 AC-1: butler prompt 在有 active 角色时必须含涌现行为指令，
    /// 否则 LLM 不知道可以建议创建新角色。
    #[tokio::test]
    async fn test_build_butler_messages_includes_emergence_prompt() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        // 需要 app_settings 表用于冷却查询
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&main_pool)
        .await
        .expect("failed to create app_settings table");

        crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: Some("打磨产品".to_string()),
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "最近想健身")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(
            system.contains("[角色涌现行为]"),
            "butler prompt 必须含涌现行为段落"
        );
        assert!(
            system.contains("create_role"),
            "涌现段落须提及 create_role 工具"
        );
        assert!(
            system.contains("record_emergence_rejection"),
            "涌现段落须提及 rejection 工具"
        );
    }

    /// Story 2.5 AC-3: 冷却列表注入 butler prompt
    #[tokio::test]
    async fn test_build_butler_messages_includes_cooldown_list() {
        let main_pool = setup_test_main_pool().await;
        let conv_pool = setup_test_conv_pool().await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&main_pool)
        .await
        .unwrap();

        // 写入一条冷却记录（1 天前，在 7 天内）
        let recent = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        crate::db::app_settings::set_emergence_cooldown(&main_pool, "健身/运动", &recent)
            .await
            .unwrap();

        crate::db::roles::create_role(
            &main_pool,
            &CreateRoleInput {
                name: "产品经理".to_string(),
                icon: None,
                color: None,
                goal: None,
            },
        )
        .await
        .unwrap();

        let conv = crate::db::conversations::get_or_create_butler_conversation(&conv_pool)
            .await
            .unwrap();

        let msgs = build_butler_messages(&conv_pool, &main_pool, &conv.id, "想健身")
            .await
            .unwrap();
        let system = &msgs.first().unwrap().content;

        assert!(
            system.contains("健身/运动"),
            "冷却中的领域必须出现在 prompt 中"
        );
        assert!(system.contains("被拒绝的领域"), "冷却列表应有说明文案");
    }
}
