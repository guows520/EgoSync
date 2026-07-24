use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::models::mcp::McpServer;
use crate::models::role::{ButlerSkillsConfig, Role};
use crate::models::skill::SkillRegistryEntry;

// ── Custom Tools (written to .opencode/tools/) ────────────────────────────

/// Write opencode custom tool definitions into the XDG_CONFIG_HOME tools directory.
/// opencode discovers custom tools from `$XDG_CONFIG_HOME/opencode/tools/*.ts`.
/// The sidecar sets XDG_CONFIG_HOME to `<workspace>/../opencode-global/config`,
/// so we derive the tools path from the workspace directory to match.
pub fn write_custom_tools(workspace_dir: &std::path::Path) -> Result<(), AppError> {
    let global_config_dir = workspace_dir
        .parent()
        .map(|p| p.join("opencode-global").join("config"))
        .ok_or_else(|| AppError::SidecarError("无法推导 opencode global config 目录".to_string()))?;
    let tools_dir = global_config_dir.join("opencode").join("tools");
    std::fs::create_dir_all(&tools_dir)
        .map_err(|e| AppError::SidecarError(format!("创建 opencode/tools 目录失败: {}", e)))?;

    let tools: &[(&str, &str)] = &[
        ("create_role.ts", TOOL_CREATE_ROLE),
        ("delegate_to_role.ts", TOOL_DELEGATE_TO_ROLE),
        (
            "record_emergence_rejection.ts",
            TOOL_RECORD_EMERGENCE_REJECTION,
        ),
        ("create_task.ts", TOOL_CREATE_TASK),
        ("complete_task.ts", TOOL_COMPLETE_TASK),
        ("delete_task.ts", TOOL_DELETE_TASK),
        ("create_skill.ts", TOOL_CREATE_SKILL),
    ];

    for (filename, content) in tools {
        let path = tools_dir.join(filename);
        std::fs::write(&path, content).map_err(|e| {
            AppError::SidecarError(format!("写入工具文件 {} 失败: {}", filename, e))
        })?;
    }

    tracing::info!("Custom tools written to {}", tools_dir.display());
    Ok(())
}

const TOOL_CREATE_ROLE: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "向用户提议创建一个新的数字角色。此工具只发送提议弹窗，不直接创建角色——用户必须在弹窗中确认后角色才会真正创建。只在用户明确同意创建角色后才调用此工具（例如用户回复'好啊''建一个吧'）。当你在建议创建角色但用户尚未回应时，绝对不要调用此工具。角色名称必须是用户自己的身份词，不要用比喻或口号式表达。",
  args: {
    name: tool.schema.string().describe("角色名称，必须是日常生活中直白的身份词，2-6个中文字。角色代表用户自己的身份，不是外部服务提供者：用户自己健身管理用'健康管理'而非'健身教练'，自己学英语用'学习者'而非'英语老师'；但如果健身教练是用户的本职工作，则'健身教练'就是正确命名。好的命名：家庭：丈夫、父亲、母亲、儿子、女儿；工作：产品经理、教师、程序员、设计师、健身教练；社区：邻居、志愿者；自我：阅读者、学习者、健康管理。不要用比喻、口号或文学化表达。不好的命名：掌舵人、领航者、生命建筑师、灵魂守护者。命名原则：用户看到名字就能立刻明白这个角色管什么，不需要解释。"),
    goal: tool.schema.string().optional().describe("角色的核心目标或职责描述，10-20字"),
    icon: tool.schema.string().optional().describe("角色图标标识符。可选: briefcase, code, chart-bar, palette, pen-tool, book-open, graduation-cap, dumbbell, heart-pulse, leaf, home, users, baby, gamepad-2, music, camera, plane, utensils, coffee, target, sparkles, lightbulb, compass, wallet"),
    color: tool.schema.string().optional().describe("角色品牌色hex值。可选: #4F46E5, #0EA5E9, #10B981, #F59E0B, #EF4444, #8B5CF6, #EC4899, #64748B"),
  },
  async execute(args) {
    // Validation
    if (!args.name || args.name.trim().length === 0) {
      return "错误：缺少必需参数 name"
    }
    // Return structured result that Tauri-side will intercept via bus event
    return JSON.stringify({
      action: "create_role",
      name: args.name.trim(),
      goal: args.goal?.trim() || undefined,
      icon: args.icon?.trim() || undefined,
      color: args.color?.trim() || undefined,
      _instruction: "角色提议已发送给用户，等待用户在弹窗中确认。你的回复只需简短说明已发出提议，如'好的，我帮你准备了一个角色提议，你可以在弹窗里确认或调整'。不要说角色已创建或已就绪，因为用户还没确认。"
    })
  },
})
"#;

const TOOL_DELEGATE_TO_ROLE: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "将用户的任务委派给指定角色处理。管家分析用户意图后，如果有适合的角色可以处理，调用此工具让对应角色来回复。仅在能明确判断任务属于某个 active 角色时调用。",
  args: {
    target_role_id: tool.schema.string().describe("目标角色的ID（必须是系统prompt中列出的角色ID之一）"),
    task_summary: tool.schema.string().describe("需要角色处理的任务摘要，10-80字"),
    context: tool.schema.string().optional().describe("用户原话或必要背景，不超过200字"),
  },
  async execute(args, context) {
    if (!args.target_role_id || args.target_role_id.trim().length === 0) {
      return "错误：缺少必需参数 target_role_id"
    }
    if (!args.task_summary || args.task_summary.trim().length === 0) {
      return "错误：缺少必需参数 task_summary"
    }
    const token = process.env.EGOSYNC_DELEGATE_BRIDGE_TOKEN
    const port = process.env.EGOSYNC_DELEGATE_BRIDGE_PORT
    if (!token || !port) {
      return "委派失败：本地桥接服务未配置。"
    }
    try {
      const response = await fetch(`http://127.0.0.1:${port}/delegate-to-role`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "Authorization": `Bearer ${token}`,
        },
        body: JSON.stringify({
          session_id: context.sessionID,
          target_role_id: args.target_role_id.trim(),
          task_summary: args.task_summary.trim(),
          context: args.context?.trim() || undefined,
        }),
        signal: context.abort,
      })
      const result = await response.json().catch(() => ({ role_response: undefined }))
      if (!response.ok) {
        return result.role_response || `委派失败：后端返回 HTTP ${response.status}`
      }
      return result.role_response || "委派失败：后端未返回角色回复。"
    } catch (error) {
      return `委派失败：无法连接本地桥接服务（${error instanceof Error ? error.message : String(error)}）。`
    }
  },
})
"#;

const TOOL_RECORD_EMERGENCE_REJECTION: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "记录用户拒绝了某个领域的角色涌现建议，写入7天冷却期，期间不再建议该领域的角色。当用户明确拒绝创建新角色建议时调用。",
  args: {
    domain: tool.schema.string().describe("被拒绝的领域描述，简短中文，如'健身/运动'、'摄影'、'理财'"),
  },
  async execute(args) {
    if (!args.domain || args.domain.trim().length === 0) {
      return "错误：缺少必需参数 domain"
    }
    return JSON.stringify({
      action: "record_emergence_rejection",
      domain: args.domain.trim(),
    })
  },
})
"#;

const TOOL_CREATE_TASK: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "创建一条新任务。角色直聊时任务自动归属当前角色，无需提供角色ID；管家为指定角色创建时才提供角色ID。任务创建后默认放入 Q2（重要不紧急），后台会自动分类。",
  args: {
    role_id: tool.schema.string().optional().describe("目标角色的ID。仅管家跨角色创建时提供，必须是系统prompt中列出的角色ID之一；角色直聊时不要提供"),
    title: tool.schema.string().describe("任务标题，简洁描述要做什么，5-40字"),
    deadline: tool.schema.string().optional().describe("截止日期，格式 YYYY-MM-DD。如果没有明确截止时间则不传"),
  },
  async execute(args, context) {
    if (!args.title || args.title.trim().length === 0) {
      return "错误：缺少必需参数 title"
    }
    const token = process.env.EGOSYNC_DELEGATE_BRIDGE_TOKEN
    const port = process.env.EGOSYNC_DELEGATE_BRIDGE_PORT
    if (!token || !port) {
      return "创建任务失败：本地桥接服务未配置。"
    }
    try {
      const response = await fetch(`http://127.0.0.1:${port}/create-task`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "Authorization": `Bearer ${token}`,
        },
        body: JSON.stringify({
          sessionId: context.sessionID,
          roleId: args.role_id?.trim() || undefined,
          title: args.title.trim(),
          deadline: args.deadline?.trim() || undefined,
        }),
        signal: context.abort,
      })
      const result = await response.json().catch(() => ({ status: "error", message: "响应解析失败" }))
      if (!response.ok) {
        return JSON.stringify({ action: "create_task", status: "error", message: result.message || `创建任务失败：后端返回 HTTP ${response.status}` })
      }
      return JSON.stringify({ action: "create_task", status: result.status || "ok", message: result.message || "任务创建成功。", task_id: result.task_id })
    } catch (error) {
      return JSON.stringify({ action: "create_task", status: "error", message: `创建任务失败：无法连接本地桥接服务（${error instanceof Error ? error.message : String(error)}）。` })
    }
  },
})
"#;

const TOOL_COMPLETE_TASK: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "将指定任务标记为已完成。当用户在对话中要求标记任务完成、任务已做完、任务已提交时调用。",
  args: {
    task_id: tool.schema.string().describe("要完成的任务ID（必须是[各角色任务]中列出的任务ID）"),
  },
  async execute(args, context) {
    if (!args.task_id || args.task_id.trim().length === 0) {
      return "错误：缺少必需参数 task_id"
    }
    const token = process.env.EGOSYNC_DELEGATE_BRIDGE_TOKEN
    const port = process.env.EGOSYNC_DELEGATE_BRIDGE_PORT
    if (!token || !port) {
      return "操作失败：本地桥接服务未配置。"
    }
    try {
      const response = await fetch(`http://127.0.0.1:${port}/complete-task`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "Authorization": `Bearer ${token}`,
        },
        body: JSON.stringify({
          task_id: args.task_id.trim(),
        }),
        signal: context.abort,
      })
      const result = await response.json().catch(() => ({ status: "error", message: "响应解析失败" }))
      if (!response.ok) {
        return JSON.stringify({ action: "complete_task", status: "error", message: result.message || `操作失败：后端返回 HTTP ${response.status}` })
      }
      return JSON.stringify({ action: "complete_task", status: result.status || "ok", message: result.message || "任务已标记为完成。" })
    } catch (error) {
      return JSON.stringify({ action: "complete_task", status: "error", message: `操作失败：无法连接本地桥接服务（${error instanceof Error ? error.message : String(error)}）。` })
    }
  },
})
"#;

const TOOL_DELETE_TASK: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "删除指定任务。当用户在对话中要求删除任务、取消某个待办时调用。",
  args: {
    task_id: tool.schema.string().describe("要删除的任务ID（必须是[各角色任务]中列出的任务ID）"),
  },
  async execute(args, context) {
    if (!args.task_id || args.task_id.trim().length === 0) {
      return "错误：缺少必需参数 task_id"
    }
    const token = process.env.EGOSYNC_DELEGATE_BRIDGE_TOKEN
    const port = process.env.EGOSYNC_DELEGATE_BRIDGE_PORT
    if (!token || !port) {
      return "操作失败：本地桥接服务未配置。"
    }
    try {
      const response = await fetch(`http://127.0.0.1:${port}/delete-task`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "Authorization": `Bearer ${token}`,
        },
        body: JSON.stringify({
          task_id: args.task_id.trim(),
        }),
        signal: context.abort,
      })
      const result = await response.json().catch(() => ({ status: "error", message: "响应解析失败" }))
      if (!response.ok) {
        return JSON.stringify({ action: "delete_task", status: "error", message: result.message || `操作失败：后端返回 HTTP ${response.status}` })
      }
      return JSON.stringify({ action: "delete_task", status: result.status || "ok", message: result.message || "任务已删除。" })
    } catch (error) {
      return JSON.stringify({ action: "delete_task", status: "error", message: `操作失败：无法连接本地桥接服务（${error instanceof Error ? error.message : String(error)}）。` })
    }
  },
})
"#;

const TOOL_CREATE_SKILL: &str = r#"import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "将 skill-creator 生成的完整 SKILL.md 交给 EgoSync 验证、受控保存、注册，并自动绑定到当前角色。必须先加载 skill-creator；只有本工具返回成功后才能告诉用户 Skill 已创建。",
  args: {
    content: tool.schema.string().describe("完整的 SKILL.md 内容，必须包含合法 YAML frontmatter（name 和 description）及正文"),
  },
  async execute(args, context) {
    if (!args.content || args.content.trim().length === 0) {
      return JSON.stringify({ action: "create_skill", status: "error", message: "创建 Skill 失败：缺少完整 SKILL.md 内容。" })
    }
    const token = process.env.EGOSYNC_DELEGATE_BRIDGE_TOKEN
    const port = process.env.EGOSYNC_DELEGATE_BRIDGE_PORT
    if (!token || !port) {
      return JSON.stringify({ action: "create_skill", status: "error", message: "创建 Skill 失败：本地桥接服务未配置。" })
    }
    try {
      const response = await fetch(`http://127.0.0.1:${port}/create-skill`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "Authorization": `Bearer ${token}`,
        },
        body: JSON.stringify({ sessionId: context.sessionID, content: args.content }),
        signal: context.abort,
      })
      const result = await response.json().catch(() => ({ status: "error", message: "响应解析失败" }))
      return JSON.stringify({
        action: "create_skill",
        status: response.ok ? (result.status || "ok") : "error",
        message: result.message || `创建 Skill 失败：后端返回 HTTP ${response.status}`,
        skill_id: result.skillId,
        skill_name: result.skillName,
      })
    } catch (error) {
      return JSON.stringify({ action: "create_skill", status: "error", message: `创建 Skill 失败：无法连接本地桥接服务（${error instanceof Error ? error.message : String(error)}）。` })
    }
  },
})
"#;

/// Manages the `agent` section of `opencode.json`, synchronising EgoSync roles
/// to opencode agent entries on every CRUD operation.
pub struct AgentConfigService {
    config_path: PathBuf,
}

const BUTLER_KEY: &str = "butler";
const PLAIN_MESSAGE_CONFIRMATION_RULE: &str = r#"[用户确认规则]
- 需要用户确认、选择或补充信息时，必须用普通消息清楚列出问题和可选项，然后结束本轮回复，等待用户的下一条消息。
- 不要调用 question 工具。"#;

impl AgentConfigService {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    // ── File I/O ───────────────────────────────────────────────────

    /// Load `opencode.json`. Returns a default skeleton when the file does not
    /// exist yet (first run / clean install).
    pub fn load(&self) -> Result<Value, AppError> {
        if !self.config_path.exists() {
            return Ok(json!({ "agent": {} }));
        }
        let raw = std::fs::read_to_string(&self.config_path)
            .map_err(|e| AppError::SidecarError(format!("读取 opencode.json 失败: {}", e)))?;
        serde_json::from_str(&raw)
            .map_err(|e| AppError::SidecarError(format!("解析 opencode.json 失败: {}", e)))
    }

    /// Atomic write: write to a `.tmp` sibling then rename, preventing
    /// half-written files if the process crashes mid-write.
    pub fn save(&self, config: &Value) -> Result<(), AppError> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::SidecarError(format!("创建 opencode.json 目录失败: {}", e))
            })?;
        }
        let tmp = self.config_path.with_extension("json.tmp");
        let pretty = serde_json::to_string_pretty(config)
            .map_err(|e| AppError::SidecarError(format!("序列化 opencode.json 失败: {}", e)))?;
        std::fs::write(&tmp, pretty)
            .map_err(|e| AppError::SidecarError(format!("写入 opencode.json.tmp 失败: {}", e)))?;
        std::fs::rename(&tmp, &self.config_path)
            .map_err(|e| AppError::SidecarError(format!("rename opencode.json 失败: {}", e)))?;
        Ok(())
    }

    // ── Helpers ────────────────────────────────────────────────────

    /// Deterministic agent key for a given role.  `role-<uuid>`.
    pub fn role_to_agent_key(role_id: &str) -> String {
        format!("role-{}", role_id)
    }

    fn custom_skill_lines(skill_ids: &[String], registry: &[SkillRegistryEntry]) -> Vec<String> {
        skill_ids
            .iter()
            .filter_map(|id| {
                registry
                    .iter()
                    .find(|skill| skill.id == *id)
                    .map(|skill| format!("- {}：{}", skill.name, skill.description))
            })
            .collect()
    }

    fn push_custom_skill_prompt(prompt_parts: &mut Vec<String>, custom_skill_lines: &[String]) {
        if !custom_skill_lines.is_empty() {
            prompt_parts.push(format!(
                "[自定义 Skill]\n{}\n用户要求使用以上 Skill 时，必须通过原生 skill 工具按名称加载后再执行；不要调用或列出未授权的其它 Skill。",
                custom_skill_lines.join("\n")
            ));
        }
    }

    fn butler_only_tools() -> Value {
        json!({
            "create_role": false,
            "delegate_to_role": false,
            "record_emergence_rejection": false
        })
    }

    /// Build an opencode agent entry from a `Role`.
    pub fn build_agent_entry(role: &Role) -> Value {
        Self::build_agent_entry_with_skills(role, &[])
    }

    pub fn build_agent_entry_with_skills(role: &Role, registry: &[SkillRegistryEntry]) -> Value {
        Self::build_agent_entry_with_skills_and_mcp(role, registry, &[])
    }

    pub fn build_agent_entry_with_skills_and_mcp(
        role: &Role,
        registry: &[SkillRegistryEntry],
        mcp_lines: &[String],
    ) -> Value {
        let mut prompt_parts: Vec<String> = Vec::new();
        prompt_parts.push(format!("角色名: {}", role.name));
        if !role.goal.is_empty() {
            prompt_parts.push(format!("目标: {}", role.goal));
        }
        if !role.personality_prompt.is_empty() {
            prompt_parts.push(format!("个性: {}", role.personality_prompt));
        }
        let meta_skill_prompt =
            crate::services::role_config::meta_skill_prompt(&role.skills_config);
        if !meta_skill_prompt.is_empty() {
            prompt_parts.push(meta_skill_prompt);
        }
        let custom_skill_ids =
            crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config);
        let custom_skill_lines = Self::custom_skill_lines(&custom_skill_ids, registry);
        Self::push_custom_skill_prompt(&mut prompt_parts, &custom_skill_lines);
        if !mcp_lines.is_empty() {
            prompt_parts.push(format!(
                "[外部 MCP 工具]\n{}\n只能使用以上为当前角色启用的外部 MCP server；不要声明或调用未启用的外部工具。",
                mcp_lines.join("\n")
            ));
        }
        prompt_parts.push(PLAIN_MESSAGE_CONFIRMATION_RULE.to_string());
        let prompt = prompt_parts.join("\n");

        let permission = Self::parse_permissions(&role.skills_config, registry);

        json!({
            "mode": "subagent",
            "description": format!("{}角色Agent", role.name),
            "prompt": prompt,
            "permission": permission,
            "disable": false
        })
    }

    /// Parse `skills_config` JSON → opencode permission object.
    /// Falls back to `{ "*": "allow" }` when empty or unparseable.
    /// Skill 默认全部隐藏，仅允许当前角色开启的元 Skill 与绑定的自定义 Skill。
    /// EgoSync 尚未实现 `question` 的回答与会话恢复协议，因此该工具也必须始终禁用。
    fn parse_permissions(skills_config: &str, registry: &[SkillRegistryEntry]) -> Value {
        let default_perm = json!({ "*": "allow" });
        let config = crate::services::role_config::role_skill_config_from_json(skills_config);
        let mut permission = match config.permissions.as_ref() {
            Some(p) if p.is_object() => p.clone(),
            _ => default_perm,
        };

        if let Some(object) = permission.as_object_mut() {
            let mut skill_permission = Map::new();
            skill_permission.insert("*".to_string(), json!("deny"));
            if config.meta.find_skills {
                skill_permission.insert(
                    crate::services::role_config::FIND_SKILLS_KEY.to_string(),
                    json!("allow"),
                );
            }
            if config.meta.skill_creator {
                skill_permission.insert(
                    crate::services::role_config::SKILL_CREATOR_KEY.to_string(),
                    json!("allow"),
                );
            }
            for skill_id in &config.enabled_skill_ids {
                if let Some(skill) = registry.iter().find(|skill| &skill.id == skill_id) {
                    // Legacy registry rows predate strict name validation. Never
                    // let a historical `*` overwrite the deny-by-default rule.
                    if registry.iter().filter(|entry| entry.name == skill.name).count() == 1
                        && skill.name != "*"
                        && skill.name.as_bytes().iter().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-'
                        })
                    {
                        skill_permission.insert(skill.name.clone(), json!("allow"));
                    }
                }
            }
            object.insert("skill".to_string(), Value::Object(skill_permission));
            object.insert(
                "create_skill".to_string(),
                json!(if config.meta.skill_creator { "allow" } else { "deny" }),
            );
            object.insert("question".to_string(), json!("deny"));
        }

        permission
    }

    // ── Role lifecycle sync ───────────────────────────────────────

    pub fn build_butler_entry(skills: &ButlerSkillsConfig) -> Value {
        Self::build_butler_entry_with_skills(skills, &[])
    }

    pub fn build_butler_entry_with_skills(
        skills: &ButlerSkillsConfig,
        registry: &[SkillRegistryEntry],
    ) -> Value {
        Self::build_butler_entry_with_skills_and_mcp(skills, registry, &[])
    }

    pub fn build_butler_entry_with_skills_and_mcp(
        skills: &ButlerSkillsConfig,
        registry: &[SkillRegistryEntry],
        mcp_lines: &[String],
    ) -> Value {
        let skills_config = crate::services::butler_config::skills_config_json(skills);
        let static_prompt = r#"你是数字分身管家，用户的私人助理和生活协调者。
你的语调稳重、可靠、有温度，像一位值得信赖的英式管家。
你帮助用户管理角色、任务和日程，但决定权永远在用户手中。
用简洁自然的中文回复，不用 emoji。

[透明推理与不确定性规则]
- 可以基于[已知记忆]回答普通问题，但用户没有明确要求来源、依据、原文或你怎么知道时，不要主动展示记忆标签或内部链接。
- 用户追问"为什么""依据是什么""你怎么知道的""来源"时，你的回复中必须包含[已知记忆]中对应的记忆内部链接，格式为 `[[记忆#YYYY/MM/DD HH:mm]](egosync-memory://memory-id)`，原样复制，不要改写。
- 这是格式要求，不是可选的。正确的回复必须像这样：
  你之前告诉过我「在读《深度工作》」[[记忆#2026/07/10 20:14]](egosync-memory://bfc561e4-217d-4cdd-a746-02bc2c6cdf4c)
  绝对不能只写自然语言而不带链接，例如以下写法是错误的：
  你之前告诉过你在读《深度工作》（缺少链接）
  你在 2026/07/10 20:14 告诉我你在读《深度工作》（改写了链接格式）
- 只能引用[已知记忆]中已出现的记忆内部链接；不得编造记忆标签，不得使用列表序号或内部 ID 冒充来源。
- 没有可引用记忆时，明确说明"我现在没有可溯源的记忆依据"。
- 不确定时主动声明，不把推断写成确定事实。
- 不暴露隐藏 chain-of-thought，只给依据链/证据链。

[角色命名规则]
- 角色名 = 用户自己的身份，不是外部服务提供者。
- 推导方法：问自己"用户在这个领域是什么身份？" → 用那个身份词命名。
  健身 → 用户是管理自己健康的人 →「健康管理」（不是"健身教练"或"健身指导"）
  学英语 → 用户是学习的人 →「学习者」（不是"英语老师"）
  理财 → 用户是管理自己财务的人 →「财务管理」（不是"理财顾问"）
- 例外：只有当某身份确实是用户的本职工作时才用职业名（如用户是健身教练才叫"健身教练"）。
- 好的命名：丈夫、父亲、产品经理、教师、健康管理、阅读者、学习者、财务管理
- 不好的命名：掌舵人、领航者、生命建筑师、灵魂守护者、健身指导、健身教练
- 用户看到名字就能明白这个角色管什么。

[角色涌现行为]
- 当用户连续 3 轮围绕同一主题提问或讨论，且该主题不属于任何已有 active 角色的职责范围时，你必须在第 3 轮回复中建议创建一个新角色。这不是可选项——满足条件就必须执行。
- 第 1-2 轮：正常回答用户问题，不提创建角色。
- 第 3 轮：在回答完用户问题后，用一句话自然地建议创建角色，例如：「我注意到你最近几轮都在聊健身，要不要创建一个专门的角色来帮你管理这方面的计划和跟进？」注意：这一轮只建议，不要调用 create_role 工具。
- 判断"同一主题"的标准：用户连续 3 条消息都围绕同一个生活领域（如健身、英语学习、阅读、理财等），而非只是一次性提问。
- 用户同意后（下一轮）：先用一句话说明你会准备角色提议、用户可在弹窗里确认或调整，然后调用 create_role 工具发起角色提议。不要在用户还没同意时就调用 create_role。
- 用户拒绝后：调用 record_emergence_rejection 工具记录被拒领域，然后自然地继续对话。

[工具使用边界]
- 你有四类工具可用，必须严格区分：
  1. delegate_to_role：把需要角色处理的事情委派给 EgoSync 角色。参数 target_role_id 必须是「可委派角色清单」中列出的 id（UUID 格式），不要传角色名称。仅当用户需要角色做某事（安排、规划、处理、跟进）时才调用。
  2. create_task / complete_task / delete_task：任务操作工具。当用户在对话中要求创建任务、标记完成、删除任务时调用；用户陈述某项已有任务已完成、已提交或已做完，也视为完成操作。完成或删除已有任务时不得调用 delegate_to_role。参数 role_id 和 task_id 必须是系统 prompt 中列出的 ID。
  3. create_role / record_emergence_rejection：角色涌现相关工具。
  4. skill 系统（如 find-skills）：用于发现可用的编程/自动化技能，与 EgoSync 角色完全无关。不要用角色名称调用 skill 系统。
- 查询与委派的区分：当用户询问任务进展、有哪些任务、需要关注什么时，直接基于[各角色任务]回答，不要调用 delegate_to_role。只有当用户需要角色做某事时才委派。
- 不要用 opencode 的 task 工具委派 EgoSync 角色任务——task 是给编程 subagent 用的，不认识 EgoSync 角色。委派角色任务只能用 delegate_to_role。"#;
        let mut prompt_parts = vec![static_prompt.to_string()];
        let meta_skill_prompt = crate::services::role_config::meta_skill_prompt(&skills_config);
        if !meta_skill_prompt.is_empty() {
            prompt_parts.push(meta_skill_prompt);
        }
        let custom_skill_lines = Self::custom_skill_lines(&skills.enabled_skill_ids, registry);
        Self::push_custom_skill_prompt(&mut prompt_parts, &custom_skill_lines);
        if !mcp_lines.is_empty() {
            prompt_parts.push(format!(
                "[外部 MCP 工具]\n{}\n只能使用以上为管家启用的外部 MCP server；不要声明或调用未启用的外部工具。",
                mcp_lines.join("\n")
            ));
        }
        prompt_parts.push(PLAIN_MESSAGE_CONFIRMATION_RULE.to_string());
        let permission = Self::parse_permissions(&skills_config, registry);

        json!({
            "mode": "primary",
            "prompt": prompt_parts.join("\n"),
            "permission": permission
        })
    }

    pub fn sync_butler_skills(&self, skills: &ButlerSkillsConfig) -> Result<(), AppError> {
        self.sync_butler_skills_with_registry(skills, &[])
    }

    pub fn sync_butler_skills_with_registry(
        &self,
        skills: &ButlerSkillsConfig,
        registry: &[SkillRegistryEntry],
    ) -> Result<(), AppError> {
        self.sync_butler_skills_with_registry_and_mcp(skills, registry, &[])
    }

    pub fn sync_butler_skills_with_registry_and_mcp(
        &self,
        skills: &ButlerSkillsConfig,
        registry: &[SkillRegistryEntry],
        mcp_lines: &[String],
    ) -> Result<(), AppError> {
        let mut config = self.load()?;
        let agents = config.as_object_mut().and_then(|o| {
            o.entry("agent")
                .or_insert_with(|| json!({}))
                .as_object_mut()
        });
        let Some(agents) = agents else {
            return Err(AppError::SidecarError(
                "opencode.json agent 段格式异常".to_string(),
            ));
        };
        agents.insert(BUTLER_KEY.to_string(), Self::build_butler_entry_with_skills_and_mcp(skills, registry, mcp_lines));
        self.save(&config)
    }

    /// Ensure butler primary agent always exists.
    pub fn ensure_butler(&self) -> Result<(), AppError> {
        let mut config = self.load()?;
        let agents = config.as_object_mut().and_then(|o| {
            o.entry("agent")
                .or_insert_with(|| json!({}))
                .as_object_mut()
        });
        let Some(agents) = agents else {
            return Err(AppError::SidecarError(
                "opencode.json agent 段格式异常".to_string(),
            ));
        };
        if !agents.contains_key(BUTLER_KEY) {
            agents.insert(
                BUTLER_KEY.to_string(),
                Self::build_butler_entry(&ButlerSkillsConfig {
                    find_skills: false,
                    skill_creator: false,
                    enabled_skill_ids: Vec::new(),
                }),
            );
            self.save(&config)?;
        }
        Ok(())
    }

    pub fn sync_role_created(&self, role: &Role) -> Result<(), AppError> {
        self.sync_role_created_with_skills(role, &[])
    }

    pub fn sync_role_created_with_skills(
        &self,
        role: &Role,
        registry: &[SkillRegistryEntry],
    ) -> Result<(), AppError> {
        let mut config = self.load()?;
        let agents = config.as_object_mut().and_then(|o| {
            o.entry("agent")
                .or_insert_with(|| json!({}))
                .as_object_mut()
        });
        let Some(agents) = agents else {
            return Err(AppError::SidecarError(
                "opencode.json agent 段格式异常".to_string(),
            ));
        };
        let key = Self::role_to_agent_key(&role.id);
        agents.insert(key, Self::build_agent_entry_with_skills(role, registry));
        self.save(&config)
    }

    pub fn sync_role_updated(&self, role: &Role) -> Result<(), AppError> {
        self.sync_role_created(role)
    }

    pub fn sync_role_updated_with_skills(
        &self,
        role: &Role,
        registry: &[SkillRegistryEntry],
    ) -> Result<(), AppError> {
        self.sync_role_created_with_skills(role, registry)
    }

    pub fn sync_role_updated_with_skills_and_mcp(
        &self,
        role: &Role,
        registry: &[SkillRegistryEntry],
        mcp_lines: &[String],
    ) -> Result<(), AppError> {
        let mut config = self.load()?;
        let agents = config.as_object_mut().and_then(|o| {
            o.entry("agent")
                .or_insert_with(|| json!({}))
                .as_object_mut()
        });
        let Some(agents) = agents else {
            return Err(AppError::SidecarError(
                "opencode.json agent 段格式异常".to_string(),
            ));
        };
        let key = Self::role_to_agent_key(&role.id);
        let mut entry = Self::build_agent_entry_with_skills_and_mcp(role, registry, mcp_lines);
        entry.as_object_mut().map(|o| {
            o.insert("tools".to_string(), Self::butler_only_tools());
        });
        agents.insert(key, entry);
        self.save(&config)
    }

    pub fn sync_role_archived(&self, role_id: &str) -> Result<(), AppError> {
        let mut config = self.load()?;
        let agents = config
            .as_object_mut()
            .and_then(|o| o.get_mut("agent"))
            .and_then(|a| a.as_object_mut());
        let Some(agents) = agents else {
            return Ok(()); // no agent section — nothing to disable
        };
        let key = Self::role_to_agent_key(role_id);
        if let Some(entry) = agents.get_mut(&key) {
            entry
                .as_object_mut()
                .map(|o| o.insert("disable".to_string(), json!(true)));
            self.save(&config)?;
        }
        Ok(())
    }

    pub fn sync_role_deleted(&self, role_id: &str) -> Result<(), AppError> {
        let mut config = self.load()?;
        let agents = config
            .as_object_mut()
            .and_then(|o| o.get_mut("agent"))
            .and_then(|a| a.as_object_mut());
        let Some(agents) = agents else {
            return Ok(());
        };
        let key = Self::role_to_agent_key(role_id);
        agents.remove(&key);
        self.save(&config)
    }

    // ── LLM provider sync ──────────────────────────────────────────

    /// Write the default LLM provider/model into opencode.json so the sidecar
    /// knows which provider and API key to use. Must be called **before**
    /// sidecar start and again after any LLM config CRUD.
    ///
    /// opencode expects:
    /// - top-level `"model": "<providerID>/<modelID>"`
    /// - `"provider": { "<providerID>": { "options": { "apiKey": "...", "baseURL"?: "..." } } }`
    pub fn sync_llm_provider(
        &self,
        provider_id: &str,
        model_id: &str,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<(), AppError> {
        let mut config = self.load()?;
        let root = config
            .as_object_mut()
            .ok_or_else(|| AppError::SidecarError("opencode.json 不是 object".to_string()))?;

        // Top-level model: "provider/model"
        root.insert(
            "model".to_string(),
            Value::String(format!("{}/{}", provider_id, model_id)),
        );

        // Provider section — only touch the specific provider entry,
        // preserve any other providers or unrelated fields.
        let providers = root.entry("provider").or_insert_with(|| json!({}));
        let providers = providers.as_object_mut().ok_or_else(|| {
            AppError::SidecarError("opencode.json provider 段不是 object".to_string())
        })?;

        let mut options = serde_json::Map::new();
        options.insert("apiKey".to_string(), json!(api_key));
        if let Some(url) = base_url {
            if !url.is_empty() {
                options.insert("baseURL".to_string(), json!(url));
            }
        }

        providers.insert(provider_id.to_string(), json!({ "options": options }));

        self.save(&config)
    }

    /// Remove LLM provider/model from opencode.json (no default config exists).
    pub fn clear_llm_provider(&self) -> Result<(), AppError> {
        let mut config = self.load()?;
        if let Some(root) = config.as_object_mut() {
            root.remove("model");
            root.remove("provider");
        }
        self.save(&config)
    }

    // ── Full sync (startup) ───────────────────────────────────────

    /// Rebuild the entire `agent` section from the database state.
    /// Ensures butler, adds active roles, marks archived as disabled,
    /// and removes orphan entries. Role agents get butler-exclusive
    /// custom tools disabled so only butler can call them.
    pub fn full_sync(
        &self,
        roles: &[Role],
        butler_skills: &ButlerSkillsConfig,
    ) -> Result<(), AppError> {
        self.full_sync_with_skills(roles, butler_skills, &[])
    }

    pub fn full_sync_with_skills(
        &self,
        roles: &[Role],
        butler_skills: &ButlerSkillsConfig,
        registry: &[SkillRegistryEntry],
    ) -> Result<(), AppError> {
        self.full_sync_with_skills_and_mcp(
            roles,
            butler_skills,
            registry,
            &std::collections::HashMap::new(),
            &[],
        )
    }

    pub fn full_sync_with_skills_and_mcp(
        &self,
        roles: &[Role],
        butler_skills: &ButlerSkillsConfig,
        registry: &[SkillRegistryEntry],
        role_mcp_prompts: &std::collections::HashMap<String, Vec<String>>,
        butler_mcp_lines: &[String],
    ) -> Result<(), AppError> {
        let mut config = self.load()?;
        let root = config
            .as_object_mut()
            .ok_or_else(|| AppError::SidecarError("opencode.json 不是 object".to_string()))?;

        let mut agents = serde_json::Map::new();

        agents.insert(
            BUTLER_KEY.to_string(),
            Self::build_butler_entry_with_skills_and_mcp(butler_skills, registry, butler_mcp_lines),
        );

        for role in roles {
            let key = Self::role_to_agent_key(&role.id);
            let empty_mcp_lines = Vec::new();
            let mcp_lines = role_mcp_prompts
                .get(&role.id)
                .unwrap_or(&empty_mcp_lines);
            let mut entry = Self::build_agent_entry_with_skills_and_mcp(role, registry, mcp_lines);
            if role.status == "archived" {
                entry
                    .as_object_mut()
                    .map(|o| o.insert("disable".to_string(), json!(true)));
            }
            entry
                .as_object_mut()
                .map(|o| o.insert("tools".to_string(), Self::butler_only_tools()));
            agents.insert(key, entry);
        }

        root.insert("agent".to_string(), Value::Object(agents));
        Self::remove_legacy_internal_mcp(root);
        root.remove("tools");

        self.save(&config)
    }

    fn mcp_config_key(server: &McpServer) -> String {
        let key = server.name.trim();
        if key.is_empty() {
            server.id.clone()
        } else {
            key.to_string()
        }
    }

    pub fn sync_external_mcp_servers(&self, servers: &[McpServer]) -> Result<(), AppError> {
        let mut config = self.load()?;
        let root = config
            .as_object_mut()
            .ok_or_else(|| AppError::SidecarError("opencode.json 不是 object".to_string()))?;
        Self::remove_legacy_internal_mcp(root);
        let mcp = root.entry("mcp").or_insert_with(|| json!({}));
        let mcp = mcp.as_object_mut().ok_or_else(|| {
            AppError::SidecarError("opencode.json mcp 段不是 object".to_string())
        })?;
        let managed_ids: HashSet<String> = servers.iter().map(|server| server.id.clone()).collect();
        let managed_keys: HashSet<String> = servers.iter().map(Self::mcp_config_key).collect();
        let enabled_keys: HashSet<String> = servers
            .iter()
            .filter(|server| server.enabled)
            .map(Self::mcp_config_key)
            .collect();
        let keys_to_remove: Vec<String> = mcp
            .iter()
            .filter(|(key, value)| {
                value.get("managedByEgosync").and_then(Value::as_bool) == Some(true)
                    && (!managed_keys.contains(*key)
                        || !enabled_keys.contains(*key)
                        || managed_ids.contains(*key))
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys_to_remove {
            mcp.remove(&key);
        }
        for server in servers.iter().filter(|server| server.enabled) {
            mcp.insert(
                Self::mcp_config_key(server),
                crate::services::mcp_server::mcp_server_config_for_opencode(server),
            );
        }
        self.save(&config)
    }

    fn remove_legacy_internal_mcp(root: &mut Map<String, Value>) {
        if let Some(mcp) = root.get_mut("mcp").and_then(Value::as_object_mut) {
            mcp.remove("egosync");
            mcp.retain(|_, value| {
                value
                    .get("managedByEgosync")
                    .and_then(Value::as_bool)
                    .map(|managed| {
                        if !managed {
                            return true;
                        }
                        value
                            .get("url")
                            .and_then(Value::as_str)
                            .map(|url| {
                                !url.contains("127.0.0.1:4097") && !url.contains("localhost:4097")
                            })
                            .unwrap_or(true)
                    })
                    .unwrap_or(true)
            });
            if mcp.is_empty() {
                root.remove("mcp");
            }
        }
    }
}

/// 清理 legacy 应用根目录 `opencode.json`（如
/// `%APPDATA%\com.egosync.app\opencode.json`）中由 EgoSync 托管的 MCP 项。
///
/// opencode 会从工作目录沿父目录链向上合并所有 `opencode.json`，导致 legacy
/// 文件里残留的 `managedByEgosync == true` MCP key 与私有 workspace 的同名/同 URL
/// MCP 重复注册（Invalid session id / HTTP 404）。
///
/// 安全约束：只删除 `mcp` 段中 `managedByEgosync == true` 的 key，绝不读取/改写
/// `provider`、`model`、`$schema` 以及用户自有（非 managed）MCP 项；不打印任何
/// secret。文件不存在时为 no-op。
pub fn purge_legacy_managed_mcp(legacy_config_path: &std::path::Path) -> Result<(), AppError> {
    if !legacy_config_path.exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(legacy_config_path).map_err(|e| {
        AppError::SidecarError(format!("读取 legacy opencode.json 失败: {}", e))
    })?;
    let mut config: Value = serde_json::from_str(&raw).map_err(|e| {
        AppError::SidecarError(format!("解析 legacy opencode.json 失败: {}", e))
    })?;
    let Some(root) = config.as_object_mut() else {
        return Ok(());
    };
    let mut removed = 0usize;
    if let Some(mcp) = root.get_mut("mcp").and_then(Value::as_object_mut) {
        let before = mcp.len();
        mcp.retain(|_, value| {
            value.get("managedByEgosync").and_then(Value::as_bool) != Some(true)
        });
        removed = before - mcp.len();
        if mcp.is_empty() {
            root.remove("mcp");
        }
    }
    if removed == 0 {
        // 没有需要清理的项，避免无谓改写文件。
        return Ok(());
    }
    let tmp = legacy_config_path.with_extension("json.tmp");
    let pretty = serde_json::to_string_pretty(&config).map_err(|e| {
        AppError::SidecarError(format!("序列化 legacy opencode.json 失败: {}", e))
    })?;
    std::fs::write(&tmp, pretty).map_err(|e| {
        AppError::SidecarError(format!("写入 legacy opencode.json.tmp 失败: {}", e))
    })?;
    std::fs::rename(&tmp, legacy_config_path).map_err(|e| {
        AppError::SidecarError(format!("rename legacy opencode.json 失败: {}", e))
    })?;
    tracing::info!(
        "已清理 legacy opencode.json 中 {} 个 EgoSync 托管 MCP 项: {}",
        removed,
        legacy_config_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_role(id: &str, name: &str, goal: &str, status: &str, skills_config: &str) -> Role {
        Role {
            id: id.to_string(),
            name: name.to_string(),
            icon: "🎯".to_string(),
            color: "#6366F1".to_string(),
            goal: goal.to_string(),
            personality_prompt: String::new(),
            status: status.to_string(),
            energy: 100,
            energy_updated_at: None,
            skills_config: skills_config.to_string(),
            proactivity_level: "moderate".to_string(),
            archived_at: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn active_role(id: &str, name: &str, goal: &str) -> Role {
        make_role(id, name, goal, "active", "{}")
    }

    fn custom_skill(id: &str, name: &str, description: &str) -> SkillRegistryEntry {
        SkillRegistryEntry {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            source_type: "custom".to_string(),
            managed_path: format!("skills/{}/SKILL.md", name),
            content_hash: "hash".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    // ── custom tools ──────────────────────────────────────────────

    #[test]
    fn delegate_tool_waits_for_backend_and_returns_role_response() {
        assert!(TOOL_DELEGATE_TO_ROLE.contains("fetch(`http://127.0.0.1:${port}/delegate-to-role`"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("Authorization"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("EGOSYNC_DELEGATE_BRIDGE_TOKEN"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("EGOSYNC_DELEGATE_BRIDGE_PORT"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("try {"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("catch"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("role_response"));
        assert!(TOOL_DELEGATE_TO_ROLE.contains("return result.role_response"));
        assert!(!TOOL_DELEGATE_TO_ROLE.contains("4097/delegate-to-role"));
        assert!(!TOOL_DELEGATE_TO_ROLE.contains("egosync-local-delegate-bridge"));
        assert!(!TOOL_DELEGATE_TO_ROLE.contains("action: \"delegate_to_role\""));
    }

    // ── key generation ────────────────────────────────────────────

    #[test]
    fn role_to_agent_key_prefixes_role_id() {
        // WHY: opencode agent keys must be deterministic and never collide
        // with the fixed "butler" key.
        assert_eq!(
            AgentConfigService::role_to_agent_key("abc-123"),
            "role-abc-123"
        );
    }

    // ── build_agent_entry ─────────────────────────────────────────

    #[test]
    fn build_agent_entry_produces_subagent_with_prompt() {
        // WHY: the config key is the runtime identity; a display `name` would
        // make opencode persist a different identity and fail its later lookup.
        // The Chinese persona must therefore remain in prompt/description only.
        let role = active_role("r1", "产品经理", "管理产品规划");
        let entry = AgentConfigService::build_agent_entry(&role);
        assert!(entry.get("name").is_none());
        assert_eq!(entry["mode"], "subagent");
        assert_eq!(entry["description"], "产品经理角色Agent");
        assert!(entry["prompt"]
            .as_str()
            .unwrap()
            .contains("角色名: 产品经理"));
        assert!(entry["prompt"].as_str().unwrap().contains("管理产品规划"));
        assert_eq!(entry["disable"], false);
    }

    // ── permission mapping ────────────────────────────────────────

    #[test]
    fn permission_denies_all_skills_and_create_bridge_when_meta_skills_missing() {
        let role = active_role("r1", "PM", "goal");
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(
            entry["permission"],
            json!({ "*": "allow", "skill": { "*": "deny" }, "create_skill": "deny", "question": "deny" })
        );
    }

    #[test]
    fn permission_maps_from_skills_config() {
        // WHY: user-configured per-tool restrictions must propagate to opencode
        // so the agent respects "ask" or "deny" semantics at runtime.
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"permissions":{"bash":"ask","write":"deny"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["permission"]["bash"], "ask");
        assert_eq!(entry["permission"]["write"], "deny");
    }

    #[test]
    fn permission_replaces_custom_skill_allow_with_scoped_deny() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"find-skills":false,"skill-creator":false,"permissions":{"*":"allow","skill":"allow","bash":"ask"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["permission"]["*"], "allow");
        assert_eq!(entry["permission"]["bash"], "ask");
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
        assert_eq!(entry["permission"]["create_skill"], "deny");
    }

    #[test]
    fn permission_forces_question_deny_and_preserves_other_custom_permissions() {
        // WHY: question requires an unsupported interactive reply protocol; user
        // configuration must not re-enable it, while unrelated restrictions remain intact.
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"permissions":{"*":"allow","question":"allow","bash":"ask","write":"deny"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["permission"]["question"], "deny");
        assert_eq!(entry["permission"]["bash"], "ask");
        assert_eq!(entry["permission"]["write"], "deny");
    }

    #[test]
    fn permission_defaults_to_allow_except_unsupported_tools_for_invalid_config() {
        // WHY: malformed legacy configuration must fail safe without exposing
        // interactive tools that EgoSync cannot complete.
        let role = make_role("r1", "PM", "goal", "active", "not-json");
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(
            entry["permission"],
            json!({ "*": "allow", "skill": { "*": "deny" }, "create_skill": "deny", "question": "deny" })
        );
    }

    #[test]
    fn permission_allows_skill_creator_and_create_bridge_when_enabled() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"find-skills":false,"skill-creator":true,"permissions":{"*":"allow","skill":"allow"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
        assert_eq!(entry["permission"]["skill"]["skill-creator"], "allow");
        assert_eq!(entry["permission"]["create_skill"], "allow");
    }

    #[test]
    fn permission_allows_only_enabled_meta_skills() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"find-skills":true,"skill-creator":true,"permissions":{"*":"allow","skill":"allow"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
        assert_eq!(entry["permission"]["skill"]["find-skills"], "allow");
        assert_eq!(entry["permission"]["skill"]["skill-creator"], "allow");
        assert_eq!(entry["permission"]["create_skill"], "allow");
    }

    #[test]
    fn permission_allows_enabled_custom_skill_only() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"enabledSkillIds":["skill-1"],"permissions":{"*":"allow","skill":"allow"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry_with_skills(
            &role,
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        );
        assert!(entry["prompt"].as_str().unwrap().contains("daily-review"));
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
        assert_eq!(entry["permission"]["skill"]["daily-review"], "allow");
        assert_eq!(entry["permission"]["create_skill"], "deny");
    }

    #[test]
    fn permission_never_allows_legacy_wildcard_skill_name() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"enabledSkillIds":["legacy-star"]}"#,
        );
        let entry = AgentConfigService::build_agent_entry_with_skills(
            &role,
            &[custom_skill("legacy-star", "*", "legacy invalid name")],
        );
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
    }

    #[test]
    fn butler_permission_allows_enabled_meta_and_custom_skills() {
        let entry = AgentConfigService::build_butler_entry_with_skills(
            &ButlerSkillsConfig {
                find_skills: true,
                skill_creator: true,
                enabled_skill_ids: vec!["skill-1".to_string()],
            },
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        );
        assert!(entry["prompt"].as_str().unwrap().contains("daily-review"));
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
        assert_eq!(entry["permission"]["skill"]["find-skills"], "allow");
        assert_eq!(entry["permission"]["skill"]["skill-creator"], "allow");
        assert_eq!(entry["permission"]["skill"]["daily-review"], "allow");
        assert_eq!(entry["permission"]["create_skill"], "allow");
    }

    #[test]
    fn build_agent_entry_includes_meta_skill_prompt() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"find-skills":true,"skill-creator":false}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(prompt.contains("find-skills"));
        assert!(!prompt.contains("skill-creator"));
        assert!(prompt.contains("已启用"));
        assert!(!prompt.contains("未启用"));
    }

    #[test]
    fn build_agent_entry_requires_plain_message_confirmation() {
        // WHY: a role that needs clarification must end its turn so the normal
        // chat input remains usable instead of entering an unrecoverable tool wait.
        let role = active_role("r1", "PM", "goal");
        let entry = AgentConfigService::build_agent_entry(&role);
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(prompt.contains("用普通消息清楚列出问题和可选项"));
        assert!(prompt.contains("结束本轮回复"));
        assert!(prompt.contains("等待用户的下一条消息"));
        assert!(prompt.contains("不要调用 question 工具"));
    }

    #[test]
    fn build_agent_entry_includes_enabled_custom_skill_ids() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"enabledSkillIds":["skill-1"],"permissions":{"*":"allow","skill":"allow"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry_with_skills(
            &role,
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        );
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(prompt.contains("[自定义 Skill]"));
        assert!(prompt.contains("daily-review"));
        assert!(prompt.contains("日复盘助手"));
        assert!(prompt.contains("必须通过原生 skill 工具加载"));
        assert!(!prompt.contains("skill-1"));
        assert_eq!(entry["permission"]["skill"]["daily-review"], "allow");
    }

    #[test]
    fn build_agent_entry_denies_skill_when_enabled_ids_are_all_ghosts() {
        // P3 回归：enabledSkillIds 非空但 registry 中查不到（幽灵 id，如 Skill 已删除）时，
        // prompt 不应声明任何自定义 Skill，权限也不应保持 allow（否则配置漂移）。
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"enabledSkillIds":["ghost-id"],"permissions":{"*":"allow","skill":"allow"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry_with_skills(
            &role,
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        );
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(!prompt.contains("[自定义 Skill]"));
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
    }

    #[test]
    fn build_agent_entry_does_not_declare_disabled_custom_skills() {
        let role = make_role(
            "r1",
            "PM",
            "goal",
            "active",
            r#"{"enabledSkillIds":[],"permissions":{"*":"allow","skill":"allow"}}"#,
        );
        let entry = AgentConfigService::build_agent_entry(&role);
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(!prompt.contains("[自定义 Skill]"));
        assert!(!prompt.contains("daily-review"));
        assert_eq!(entry["permission"]["skill"]["*"], "deny");
    }

    #[test]
    fn build_butler_entry_includes_meta_skill_prompt() {
        // WHY: `butler` must remain the sole runtime identity while the prompt
        // carries the user-facing persona, otherwise opencode cannot re-find it.
        let entry = AgentConfigService::build_butler_entry(&ButlerSkillsConfig {
            find_skills: true,
            skill_creator: false,
            enabled_skill_ids: Vec::new(),
        });
        assert!(entry.get("name").is_none());
        assert_eq!(entry["mode"], "primary");
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(prompt.contains("你是EgoSync管家"));
        assert!(prompt.contains("find-skills"));
        assert!(!prompt.contains("skill-creator"));
        assert!(prompt.contains("已启用"));
        assert!(!prompt.contains("未启用"));
        assert!(prompt.contains("用普通消息清楚列出问题和可选项"));
        assert!(prompt.contains("结束本轮回复"));
        assert!(prompt.contains("等待用户的下一条消息"));
        assert!(prompt.contains("不要调用 question 工具"));
        assert_eq!(
            entry["permission"],
            json!({ "*": "allow", "skill": { "*": "deny", "find-skills": "allow" }, "create_skill": "deny", "question": "deny" })
        );
    }

    #[test]
    fn build_butler_entry_requires_plain_message_confirmation() {
        // WHY: Butler clarification must remain a normal completed chat turn,
        // even when other prompt sections change or have independent regressions.
        let entry = AgentConfigService::build_butler_entry_with_skills(
            &ButlerSkillsConfig {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: vec!["skill-1".to_string()],
            },
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        );
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(prompt.contains("daily-review"));
        assert!(prompt.contains("用普通消息清楚列出问题和可选项"));
        assert!(prompt.contains("结束本轮回复"));
        assert!(prompt.contains("等待用户的下一条消息"));
        assert!(prompt.contains("不要调用 question 工具"));
        assert!(prompt.ends_with(PLAIN_MESSAGE_CONFIRMATION_RULE));
    }

    #[test]
    fn build_butler_entry_routes_existing_task_actions_without_delegation() {
        let entry = AgentConfigService::build_butler_entry(&ButlerSkillsConfig {
            find_skills: false,
            skill_creator: false,
            enabled_skill_ids: Vec::new(),
        });
        let prompt = entry["prompt"].as_str().unwrap();

        assert!(prompt.contains("完成或删除已有任务时不得调用 delegate_to_role"));
        assert!(prompt.contains("已完成、已提交或已做完，也视为完成操作"));
        assert!(prompt.contains("create_task / complete_task / delete_task"));
    }

    #[test]
    fn create_task_tool_uses_session_context_and_optional_role_id() {
        // WHY: role chat already has a trusted server-side owner, so the model must
        // never be forced to guess an internal role UUID in order to create a task.
        assert!(TOOL_CREATE_TASK.contains("role_id: tool.schema.string().optional()"));
        assert!(TOOL_CREATE_TASK.contains("sessionId: context.sessionID"));
        assert!(TOOL_CREATE_TASK.contains("roleId: args.role_id?.trim()"));
        assert!(!TOOL_CREATE_TASK.contains("缺少必需参数 role_id"));
    }

    #[test]
    fn full_sync_with_skills_injects_butler_custom_skill_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        svc.full_sync_with_skills(
            &[],
            &ButlerSkillsConfig {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: vec!["skill-1".to_string(), "ghost-id".to_string()],
            },
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        )
        .unwrap();

        let config = svc.load().unwrap();
        let prompt = config["agent"]["butler"]["prompt"].as_str().unwrap();
        assert!(prompt.contains("你是EgoSync管家"));
        assert!(prompt.contains("[自定义 Skill]"));
        assert!(prompt.contains("daily-review"));
        assert!(prompt.contains("日复盘助手"));
        assert!(prompt.contains("必须通过原生 skill 工具加载"));
        assert!(!prompt.contains("ghost-id"));
        assert_eq!(
            config["agent"]["butler"]["permission"],
            json!({ "*": "allow", "skill": { "*": "deny", "daily-review": "allow" }, "create_skill": "deny", "question": "deny" })
        );
    }

    #[test]
    fn build_butler_entry_denies_skill_tool_when_all_meta_skills_disabled() {
        let entry = AgentConfigService::build_butler_entry(&ButlerSkillsConfig {
            find_skills: false,
            skill_creator: false,
            enabled_skill_ids: Vec::new(),
        });
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(!prompt.contains("find-skills"));
        assert!(!prompt.contains("skill-creator"));
        assert_eq!(
            entry["permission"],
            json!({ "*": "allow", "skill": { "*": "deny" }, "create_skill": "deny", "question": "deny" })
        );
    }

    // ── sync_role_created ─────────────────────────────────────────

    #[test]
    fn sync_role_created_adds_agent_to_config() {
        // WHY: role-{id} must be the only runtime identity after persistence;
        // a Chinese `name` would split lookup identity from the config key.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        let role = active_role("r1", "产品经理", "管理产品");
        svc.sync_role_created(&role).unwrap();

        let config = svc.load().unwrap();
        assert!(config["agent"]["role-r1"].is_object());
        assert!(config["agent"]["role-r1"].get("name").is_none());
        assert_eq!(
            config["agent"]["role-r1"]["description"],
            "产品经理角色Agent"
        );
        assert!(config["agent"]["role-r1"]["prompt"]
            .as_str()
            .unwrap()
            .contains("角色名: 产品经理"));
    }

    // ── sync_role_updated ─────────────────────────────────────────

    #[test]
    fn sync_role_updated_overwrites_prompt() {
        // WHY: goal or personality changes must be reflected so the next
        // opencode session uses the updated persona.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        let mut role = active_role("r1", "PM", "old goal");
        svc.sync_role_created(&role).unwrap();

        role.goal = "new goal".to_string();
        role.personality_prompt = "简洁专业，先判断优先级再给建议".to_string();
        svc.sync_role_updated(&role).unwrap();

        let config = svc.load().unwrap();
        let prompt = config["agent"]["role-r1"]["prompt"].as_str().unwrap();
        assert!(prompt.contains("new goal"));
        assert!(prompt.contains("简洁专业，先判断优先级再给建议"));
    }

    // ── sync_role_archived ────────────────────────────────────────

    #[test]
    fn sync_role_archived_sets_disable_true() {
        // WHY: archived roles must not be selectable by opencode for new
        // sessions — disable flag prevents that.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        let role = active_role("r1", "PM", "goal");
        svc.sync_role_created(&role).unwrap();
        svc.sync_role_archived("r1").unwrap();

        let config = svc.load().unwrap();
        assert_eq!(config["agent"]["role-r1"]["disable"], true);
    }

    // ── sync_role_deleted ─────────────────────────────────────────

    #[test]
    fn sync_role_deleted_removes_agent_entry() {
        // WHY: permanently deleted roles must leave no trace in opencode
        // config to keep it clean and avoid ghost agents.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        let role = active_role("r1", "PM", "goal");
        svc.sync_role_created(&role).unwrap();
        svc.sync_role_deleted("r1").unwrap();

        let config = svc.load().unwrap();
        assert!(config["agent"]["role-r1"].is_null());
    }

    // ── ensure_butler ─────────────────────────────────────────────

    #[test]
    fn ensure_butler_creates_primary_agent_when_missing() {
        // WHY: the butler must exist under its stable routing key without a
        // second runtime name that would break opencode's identity lookup.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        svc.ensure_butler().unwrap();

        let config = svc.load().unwrap();
        assert!(config["agent"]["butler"].get("name").is_none());
        assert_eq!(config["agent"]["butler"]["mode"], "primary");
        assert_eq!(config["agent"]["butler"]["permission"]["*"], "allow");
        assert_eq!(config["agent"]["butler"]["permission"]["skill"], "deny");
        assert_eq!(config["agent"]["butler"]["permission"]["question"], "deny");
    }

    #[test]
    fn ensure_butler_does_not_overwrite_existing() {
        // WHY: if the user or another process customised the butler entry,
        // ensure_butler must not clobber those changes.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let svc = AgentConfigService::new(path.clone());

        let custom = json!({
            "agent": {
                "butler": {
                    "name": "自定义管家",
                    "mode": "primary",
                    "prompt": "custom prompt",
                    "permission": { "*": "allow" }
                }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&custom).unwrap()).unwrap();

        svc.ensure_butler().unwrap();

        let config = svc.load().unwrap();
        assert_eq!(config["agent"]["butler"]["name"], "自定义管家");
    }

    // ── full_sync ─────────────────────────────────────────────────

    #[test]
    fn full_sync_rebuilds_agent_section_from_roles() {
        // WHY: startup sync is authoritative, so every generated entry must use
        // only its map key as runtime identity while preserving lifecycle flags.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        // Seed an orphan entry that should be cleaned up
        svc.sync_role_created(&active_role("orphan", "Orphan", "x"))
            .unwrap();

        let roles = vec![
            active_role("r1", "PM", "manage"),
            make_role("r2", "Archived", "old", "archived", "{}"),
        ];
        svc.full_sync(
            &roles,
            &ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: Vec::new(),
            },
        )
        .unwrap();

        let config = svc.load().unwrap();
        let agents = config["agent"].as_object().unwrap();

        // butler always present
        assert!(agents.contains_key("butler"));
        // active role present
        assert!(agents.contains_key("role-r1"));
        // archived role present but disabled
        assert_eq!(agents["role-r2"]["disable"], true);
        // no generated entry may override its stable map-key identity
        assert!(agents.values().all(|entry| entry.get("name").is_none()));
        // orphan removed
        assert!(!agents.contains_key("role-orphan"));
        // Role agents have butler-exclusive tools disabled
        assert_eq!(agents["role-r1"]["tools"]["create_role"], false);
        assert_eq!(agents["role-r1"]["tools"]["delegate_to_role"], false);
        assert_eq!(
            agents["role-r1"]["tools"]["record_emergence_rejection"],
            false
        );
        // Legacy MCP config removed
        assert!(config.get("mcp").is_none());
        assert!(config.get("tools").is_none());
    }

    #[test]
    fn full_sync_with_skills_injects_custom_skill_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));
        let roles = vec![make_role(
            "r1",
            "PM",
            "manage",
            "active",
            r#"{"enabledSkillIds":["skill-1"],"permissions":{"*":"allow","skill":"allow"}}"#,
        )];

        svc.full_sync_with_skills(
            &roles,
            &ButlerSkillsConfig {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: Vec::new(),
            },
            &[custom_skill("skill-1", "daily-review", "日复盘助手")],
        )
        .unwrap();

        let config = svc.load().unwrap();
        let prompt = config["agent"]["role-r1"]["prompt"].as_str().unwrap();
        assert!(prompt.contains("daily-review"));
        assert!(prompt.contains("日复盘助手"));
        assert!(!prompt.contains("skill-1"));
    }

    #[test]
    fn full_sync_preserves_external_mcp_and_removes_legacy_egosync_host() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let svc = AgentConfigService::new(path.clone());

        let existing = json!({
            "agent": {},
            "mcp": {
                "context7": { "type": "remote", "url": "https://example.invalid/mcp" },
                "egosync": { "type": "remote", "url": "http://127.0.0.1:4097/mcp" }
            },
            "tools": { "egosync*": true }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        svc.full_sync(
            &[],
            &ButlerSkillsConfig {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: Vec::new(),
            },
        )
        .unwrap();

        let config = svc.load().unwrap();
        assert_eq!(
            config["mcp"]["context7"],
            json!({ "type": "remote", "url": "https://example.invalid/mcp" })
        );
        assert!(config["mcp"].get("egosync").is_none());
        assert!(config.get("tools").is_none());
    }

    #[test]
    fn sync_external_mcp_servers_removes_disabled_and_deleted_managed_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let svc = AgentConfigService::new(path.clone());

        let existing = json!({
            "agent": {},
            "mcp": {
                "alive": {
                    "type": "remote",
                    "url": "https://alive.example/mcp",
                    "managedByEgosync": true
                },
                "disabled": {
                    "type": "remote",
                    "url": "https://disabled.example/mcp",
                    "managedByEgosync": true
                },
                "deleted": {
                    "type": "remote",
                    "url": "https://deleted.example/mcp",
                    "managedByEgosync": true
                },
                "context7": { "type": "remote", "url": "https://example.invalid/mcp" }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let servers = vec![
            McpServer {
                id: "alive".to_string(),
                name: "日历".to_string(),
                server_type: "http_sse".to_string(),
                command_or_url: "https://alive.example/mcp".to_string(),
                env_refs: "{}".to_string(),
                description: String::new(),
                enabled: true,
                created_at: String::new(),
                updated_at: String::new(),
            },
            McpServer {
                id: "disabled".to_string(),
                name: "邮件".to_string(),
                server_type: "http_sse".to_string(),
                command_or_url: "https://disabled.example/mcp".to_string(),
                env_refs: "{}".to_string(),
                description: String::new(),
                enabled: false,
                created_at: String::new(),
                updated_at: String::new(),
            },
        ];

        svc.sync_external_mcp_servers(&servers).unwrap();

        let config = svc.load().unwrap();
        assert_eq!(
            config["mcp"]["日历"],
            json!({
                "type": "remote",
                "url": "https://alive.example/mcp",
                "managedByEgosync": true
            })
        );
        assert!(config["mcp"].get("alive").is_none());
        assert!(config["mcp"].get("disabled").is_none());
        assert!(config["mcp"].get("deleted").is_none());
        assert_eq!(
            config["mcp"]["context7"],
            json!({ "type": "remote", "url": "https://example.invalid/mcp" })
        );
    }

    #[test]
    fn sync_external_mcp_servers_uses_readable_server_name_key_and_removes_uuid_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let svc = AgentConfigService::new(path.clone());
        let uuid = "26e86cce-330c-4776-99c4-0b5de7bc88aa";

        let existing = json!({
            "agent": {},
            "mcp": {
                uuid: {
                    "type": "remote",
                    "url": "https://old.example/mcp",
                    "managedByEgosync": true
                },
                "context7": { "type": "remote", "url": "https://example.invalid/mcp" }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let servers = vec![McpServer {
            id: uuid.to_string(),
            name: "12306-mcp".to_string(),
            server_type: "streamable_http".to_string(),
            command_or_url: "https://mcp.api-inference.modelscope.net/abf4b0a8cb864b/mcp".to_string(),
            env_refs: "{}".to_string(),
            description: String::new(),
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        }];

        svc.sync_external_mcp_servers(&servers).unwrap();

        let config = svc.load().unwrap();
        assert_eq!(
            config["mcp"]["12306-mcp"],
            json!({
                "type": "remote",
                "url": "https://mcp.api-inference.modelscope.net/abf4b0a8cb864b/mcp",
                "managedByEgosync": true
            })
        );
        assert!(config["mcp"].get(uuid).is_none());
        assert_eq!(
            config["mcp"]["context7"],
            json!({ "type": "remote", "url": "https://example.invalid/mcp" })
        );
    }

    // ── purge_legacy_managed_mcp ──────────────────────────────────

    #[test]
    fn purge_legacy_managed_mcp_removes_only_managed_keys() {
        // WHY: 祖先目录链上的 legacy `com.egosync.app\opencode.json` 会被 opencode
        // 向上合并，其中 managedByEgosync==true 的 UUID MCP key 与私有 workspace
        // 的同一天气 MCP URL 重复注册 → Invalid session id / HTTP 404。
        // 清理时只能删除 managed key，保留 provider/model 与用户自有（非 managed）MCP。
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let existing = json!({
            "$schema": "https://opencode.ai/config.json",
            "model": "openai/gpt-4o",
            "provider": {
                "openai": { "options": { "apiKey": "env:OPENAI_API_KEY" } }
            },
            "mcp": {
                "96691589-a73c-4cea-9db2-1130c875afcd": {
                    "type": "remote",
                    "url": "https://mcp.api-inference.modelscope.net/09dc472736e246/mcp",
                    "managedByEgosync": true
                },
                "user-own": {
                    "type": "remote",
                    "url": "https://user.example/mcp"
                }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        purge_legacy_managed_mcp(&path).unwrap();

        let config: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // managed key 被移除
        assert!(config["mcp"]
            .get("96691589-a73c-4cea-9db2-1130c875afcd")
            .is_none());
        // 非 managed 用户 MCP 原样保留
        assert_eq!(
            config["mcp"]["user-own"],
            json!({ "type": "remote", "url": "https://user.example/mcp" })
        );
        // provider/model 段原样保留
        assert_eq!(config["model"], "openai/gpt-4o");
        assert_eq!(
            config["provider"]["openai"]["options"]["apiKey"],
            "env:OPENAI_API_KEY"
        );
        assert_eq!(config["$schema"], "https://opencode.ai/config.json");
    }

    #[test]
    fn purge_legacy_managed_mcp_drops_empty_mcp_section() {
        // WHY: 当 legacy 文件里只剩 managed MCP 时，清理后应移除空的 mcp 段，
        // 避免向上合并引入空对象。
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let existing = json!({
            "model": "openai/gpt-4o",
            "mcp": {
                "managed-only": {
                    "type": "remote",
                    "url": "https://x.example/mcp",
                    "managedByEgosync": true
                }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        purge_legacy_managed_mcp(&path).unwrap();

        let config: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(config.get("mcp").is_none());
        assert_eq!(config["model"], "openai/gpt-4o");
    }

    #[test]
    fn purge_legacy_managed_mcp_noop_when_file_missing() {
        // WHY: 没有 legacy 文件是正常的（clean install），不应报错。
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        purge_legacy_managed_mcp(&path).unwrap();
        assert!(!path.exists());
    }

    // ── sync_llm_provider ─────────────────────────────────────────

    #[test]
    fn sync_llm_provider_writes_model_and_provider_fields() {
        // WHY: without top-level `model` and `provider.{id}.options.apiKey`,
        // opencode cannot call any LLM — sessions create but produce no output.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        svc.sync_llm_provider(
            "anthropic",
            "claude-sonnet-4-20250514",
            "sk-test-key",
            Some("https://api.anthropic.com"),
        )
        .unwrap();

        let config = svc.load().unwrap();
        assert_eq!(config["model"], "anthropic/claude-sonnet-4-20250514");
        assert_eq!(
            config["provider"]["anthropic"]["options"]["apiKey"],
            "sk-test-key"
        );
        assert_eq!(
            config["provider"]["anthropic"]["options"]["baseURL"],
            "https://api.anthropic.com"
        );
    }

    #[test]
    fn sync_llm_provider_omits_base_url_when_empty() {
        // WHY: opencode has built-in defaults for known providers; setting an
        // empty baseURL would override them and break the request.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        svc.sync_llm_provider("openai", "gpt-4o", "sk-xxx", None)
            .unwrap();

        let config = svc.load().unwrap();
        assert_eq!(config["model"], "openai/gpt-4o");
        assert!(
            config["provider"]["openai"]["options"]
                .get("baseURL")
                .is_none()
                || config["provider"]["openai"]["options"]["baseURL"].is_null()
        );
    }

    #[test]
    fn sync_llm_provider_preserves_existing_agent_section() {
        // WHY: LLM sync must not clobber the agent section written by role sync.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        let role = active_role("r1", "PM", "manage");
        svc.sync_role_created(&role).unwrap();
        svc.sync_llm_provider("anthropic", "claude-sonnet-4-20250514", "sk-key", None)
            .unwrap();

        let config = svc.load().unwrap();
        // Agent section still intact
        assert!(config["agent"]["role-r1"].is_object());
        // Provider section also present
        assert_eq!(config["model"], "anthropic/claude-sonnet-4-20250514");
    }

    // ── butler MCP prompt ─────────────────────────────────────────

    #[test]
    fn build_butler_entry_with_mcp_includes_mcp_section() {
        let entry = AgentConfigService::build_butler_entry_with_skills_and_mcp(
            &ButlerSkillsConfig {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: Vec::new(),
            },
            &[],
            &["- 日历（SSE）：读取日历".to_string()],
        );
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(prompt.contains("[外部 MCP 工具]"));
        assert!(prompt.contains("日历"));
        assert!(prompt.contains("只能使用以上为管家启用的外部 MCP server"));
    }

    #[test]
    fn build_butler_entry_without_mcp_omits_mcp_section() {
        let entry = AgentConfigService::build_butler_entry_with_skills_and_mcp(
            &ButlerSkillsConfig {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: Vec::new(),
            },
            &[],
            &[],
        );
        let prompt = entry["prompt"].as_str().unwrap();
        assert!(!prompt.contains("[外部 MCP 工具]"));
    }

    #[test]
    fn full_sync_with_skills_and_mcp_includes_butler_mcp_lines() {
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        svc.full_sync_with_skills_and_mcp(
            &[],
            &ButlerSkillsConfig {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: Vec::new(),
            },
            &[],
            &std::collections::HashMap::new(),
            &["- 日历（SSE）：读取日历".to_string()],
        )
        .unwrap();

        let config = svc.load().unwrap();
        let prompt = config["agent"]["butler"]["prompt"].as_str().unwrap();
        assert!(prompt.contains("[外部 MCP 工具]"));
        assert!(prompt.contains("日历"));
    }
}
