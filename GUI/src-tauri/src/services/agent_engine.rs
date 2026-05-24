use std::sync::Arc;

use tauri::Emitter;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::db::conversations;
use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAiProvider;
use crate::llm::traits::{
    ChatCompletionMessage, ChatOptions, LlmProvider, StreamEvent, ToolCall, ToolDefinition,
};
use crate::models::chat::{RoleProposedPayload, StreamPayload};
use crate::models::role::CreateRoleInput;
use crate::services::secret_store;

const BUTLER_SYSTEM_PROMPT: &str = "\
你是 EgoSync 的数字管家，用户的私人助理和生活协调者。\
你的语调稳重、可靠、有温度，像一位值得信赖的英式管家。\
你帮助用户管理角色、任务和日程，但决定权永远在用户手中。\
用简洁自然的中文回复，不用 emoji，不用 markdown 格式化。";

const HISTORY_LIMIT: i64 = 20;

/// Story 2.3 AC-7: 跨角色全局视野。
/// 用户可能在角色 X 视图私聊后回管家，管家 system prompt 必须自带各 active 角色近况摘要，
/// 否则管家会"失忆"——它只看到自己 conversation 里的内容，看不到角色私聊。
/// 摘要为纯文本注入 prompt，对用户不可见。无 active 角色或全部无历史时返回空串。
const CROSS_ROLE_SUMMARY_PER_ROLE: i64 = 4;
const CROSS_ROLE_SUMMARY_PER_LINE_CHARS: usize = 40;
const CROSS_ROLE_SUMMARY_TOTAL_CHARS: usize = 2000;

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
            let snippet: String = m
                .content
                .chars()
                .take(CROSS_ROLE_SUMMARY_PER_LINE_CHARS)
                .collect();
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

pub async fn build_butler_messages(
    conv_pool: &ConversationsPool,
    main_pool: &DbPool,
    conversation_id: &str,
    user_message: &str,
) -> Result<Vec<ChatCompletionMessage>, AppError> {
    let mut result = Vec::new();

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

    let cross_summary = build_cross_role_summary(conv_pool, main_pool).await?;
    if !cross_summary.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&cross_summary);
    }

    if !active_roles.is_empty() {
        // 行为指南只在有可委派角色时才有意义；零角色时不要诱导 LLM 调用工具。
        system_prompt.push_str(
            "\n\n[行为指南]\n\
            - 当用户的需求清晰指向某个角色时：先用一句话告诉用户你要委派给谁（如『稍等，我让产品经理看一下』），再调用 delegate_to_role 工具。\n\
            - 用户一句话同时涉及多个角色时：可以在同一轮内调用多个 delegate_to_role（并行委派）。\n\
            - 意图模糊或没有合适角色时：不要调用工具，用一句话主动追问用户希望由谁来处理。\n\
            - 收到角色回复（tool result）后：用自己的话向用户转述结果，必要时显式说明这是来自哪个角色的反馈。"
        );
    }

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

/// 角色 system prompt 的基线。注意：这里不再以 BUTLER_SYSTEM_PROMPT 为基础。
/// 原因：管家与角色是两种互斥身份。如果在角色 prompt 里先建立"我是管家"，
/// 模型会因为先入为主自报为管家（Story 2.2 修复后线上观察到的真实问题）。
const ROLE_SYSTEM_PROMPT_PREFIX: &str = "\
你是用户在 EgoSync 中的「{name}」角色，是 TA 自己的一个内在维度。\
你不是数字管家、不是通用助理；你只代表「{name}」这个身份说话，遇到自我介绍务必使用该身份。\
你的语调稳重、可靠、有温度，用简洁自然的中文回复，不用 emoji，不用 markdown 格式化。";

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

    let mut system_prompt = ROLE_SYSTEM_PROMPT_PREFIX.replace("{name}", &role.name);
    if !role.goal.trim().is_empty() {
        system_prompt.push_str("\n该角色的核心目标是：");
        system_prompt.push_str(role.goal.trim());
        system_prompt.push('。');
    }
    let personality = role.personality_prompt.trim();
    if !personality.is_empty() {
        system_prompt.push('\n');
        system_prompt.push_str(personality);
    }

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
- 语调温暖简洁，不用 emoji，不用 markdown。\n\
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
) -> Result<(), AppError> {
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
        // 管家视图：挂 delegate_to_role；tool_choice=None 让 LLM 自主决定（意图模糊时追问）
        (None, None) => ChatOptions {
            disable_thinking: false,
            tools: Some(vec![delegate_to_role_tool_definition()]),
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

    let mut accumulated = String::new();
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
                        },
                    );
                }

                // Handle tool calls if any
                if !tool_calls_received.is_empty() {
                    let tool_results = execute_tool_calls(
                        &app_handle,
                        &main_pool,
                        &conv_pool,
                        &conversation_id,
                        &user_message_id,
                        &tool_calls_received,
                    )
                    .await;

                    // Build follow-up messages with tool results for continuation
                    let mut followup_messages = messages.clone();
                    // Add the assistant message with tool_calls
                    followup_messages.push(ChatCompletionMessage {
                        role: "assistant".to_string(),
                        content: accumulated.clone(),
                        tool_calls: Some(tool_calls_received.clone()),
                        tool_call_id: None,
                    });
                    // Add tool results
                    for (tc, result_text) in tool_calls_received.iter().zip(tool_results.iter()) {
                        followup_messages.push(ChatCompletionMessage {
                            role: "tool".to_string(),
                            content: result_text.clone(),
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

                    // Stream the follow-up tokens
                    while let Some(event2) = rx2.recv().await {
                        match event2 {
                            StreamEvent::Token(token) => {
                                accumulated.push_str(&token);
                                let _ = app_handle.emit(
                                    "llm:stream",
                                    StreamPayload {
                                        conversation_id: conversation_id.clone(),
                                        token,
                                        done: false,
                                        thinking: false,
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
                    },
                );
                break;
            }
        }
    }

    Ok(())
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
struct DelegationRecord {
    target_role_id: String,
    target_role_name: String,
    task_summary: String,
    /// "ok" / "role_not_found"
    status: String,
}

/// Story 2.3 AC-1 / AC-4 / AC-8: 把任务委派给角色 LLM 处理并返回结果。
/// 实现要点：
/// - 校验 role 存在且 status=active；否则走 AC-8 兜底，不污染角色对话
/// - 角色 LLM 调用本地化 drain，**不** emit `llm:stream`（避免污染管家流，见 Dev Notes 设计决策）
/// - 委派交互写入角色对话历史（AC-4：角色"记得"被委派）
async fn execute_delegate_to_role(
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
                        task_summary: args.task_summary,
                        status: "build_messages_error".to_string(),
                    },
                );
            }
        };

    // 关键：本地化 drain，不 emit `llm:stream`（避免污染管家 conversation 流，见 Dev Notes）
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(128);
    let provider_clone = provider.clone();
    tokio::spawn(async move {
        if let Err(e) = provider_clone
            .chat_stream(
                role_messages,
                tx,
                ChatOptions {
                    disable_thinking: true,
                    tools: None,
                    tool_choice: None,
                },
            )
            .await
        {
            tracing::error!("[delegate] 角色流式调用失败: {}", e);
        }
    });

    let mut role_reply = String::new();
    while let Some(ev) = rx.recv().await {
        match ev {
            StreamEvent::Token(t) => role_reply.push_str(&t),
            StreamEvent::Done => break,
            StreamEvent::Error(e) => {
                tracing::error!("[delegate] 角色流出错: {}", e);
                break;
            }
            _ => {}
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
    let _ = app_handle.emit(
        "role:proposed",
        RoleProposedPayload {
            conversation_id: conversation_id.to_string(),
            name: args.name.clone(),
            icon: normalized_icon,
            color: normalized_color,
            goal: args.goal.clone(),
        },
    );

    // 返回给模型的 tool result：明确告知"提议已发给用户，等待确认"，
    // 让模型在后续 follow-up 中用提议语气而不是宣布"创建成功"。
    format!(
        "已向用户发送角色提议「{}」，等待用户在弹窗中确认或修改。请在后续回复中只用提议语气（如\"已经准备好提议，等你确认\"），\
         不要说\"已创建\"或\"创建成功\"。",
        args.name
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
    async fn test_build_role_messages_injects_name_and_goal() {
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
        // 角色身份必须独立于管家身份，不能拷贝管家 prompt 的开头去建立"我是数字管家"，
        // 否则模型会因先入为主自报为管家。这是真实线上观察到的退化模式。
        // 注意：允许 prompt 用反向句式（如"你不是数字管家"），所以匹配的是建立身份的句首。
        assert!(
            !system.content.contains("你是 EgoSync 的数字管家"),
            "角色 prompt 不应以管家基线建立身份"
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
        assert!(!system.ends_with('\n'), "prompt 末尾不应有空行");
        assert!(
            !system.contains("personality"),
            "personality_prompt 为空时不应出现该词的英文残留"
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
}
