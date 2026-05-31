use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::AppError;
use crate::models::role::Role;

// ── Custom Tools (written to .opencode/tools/) ────────────────────────────

/// Write opencode custom tool definitions into the global tools directory.
/// Global tools (`~/.config/opencode/tools/`) are available to all opencode
/// project instances regardless of working directory.
pub fn write_custom_tools(_workspace_dir: &std::path::Path) -> Result<(), AppError> {
    // opencode uses ~/.config/opencode/ on all platforms (including Windows)
    let home =
        dirs::home_dir().ok_or_else(|| AppError::SidecarError("无法获取用户主目录".to_string()))?;
    let tools_dir = home.join(".config").join("opencode").join("tools");
    std::fs::create_dir_all(&tools_dir)
        .map_err(|e| AppError::SidecarError(format!("创建 opencode/tools 目录失败: {}", e)))?;

    let tools: &[(&str, &str)] = &[
        ("create_role.ts", TOOL_CREATE_ROLE),
        ("delegate_to_role.ts", TOOL_DELEGATE_TO_ROLE),
        (
            "record_emergence_rejection.ts",
            TOOL_RECORD_EMERGENCE_REJECTION,
        ),
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
  description: "向用户提议创建一个新的数字角色。此工具只发送提议弹窗，不直接创建角色——用户必须在弹窗中确认后角色才会真正创建。当用户要求创建角色时必须调用此工具，而不是假装创建。",
  args: {
    name: tool.schema.string().describe("角色名称，简洁中文名词，如'产品经理'、'写作助手'"),
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
      _instruction: "角色已创建成功。你的回复只需简短确认，如'好的，已创建'或'角色已就绪'。不要列出角色细节（名称、目标、图标、配色），不要提及弹窗。"
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

/// Manages the `agent` section of `opencode.json`, synchronising EgoSync roles
/// to opencode agent entries on every CRUD operation.
pub struct AgentConfigService {
    config_path: PathBuf,
}

const BUTLER_KEY: &str = "butler";

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

    /// Build an opencode agent entry from a `Role`.
    pub fn build_agent_entry(role: &Role) -> Value {
        let mut prompt_parts: Vec<String> = Vec::new();
        prompt_parts.push(format!("角色名: {}", role.name));
        if !role.goal.is_empty() {
            prompt_parts.push(format!("目标: {}", role.goal));
        }
        if !role.personality_prompt.is_empty() {
            prompt_parts.push(format!("个性: {}", role.personality_prompt));
        }
        let prompt = prompt_parts.join("\n");

        let permission = Self::parse_permissions(&role.skills_config);

        json!({
            "name": role.name,
            "mode": "subagent",
            "description": format!("{}角色Agent", role.name),
            "prompt": prompt,
            "permission": permission,
            "disable": false
        })
    }

    /// Parse `skills_config` JSON → opencode permission object.
    /// Falls back to `{ "*": "allow" }` when empty or unparseable.
    fn parse_permissions(skills_config: &str) -> Value {
        let default_perm = json!({ "*": "allow" });
        let Ok(parsed) = serde_json::from_str::<Value>(skills_config) else {
            return default_perm;
        };
        match parsed.get("permissions") {
            Some(p) if p.is_object() => p.clone(),
            _ => default_perm,
        }
    }

    // ── Role lifecycle sync ───────────────────────────────────────

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
                json!({
                    "name": "管家",
                    "mode": "primary",
                    "prompt": "你是EgoSync管家",
                    "permission": { "*": "allow" },
                    "permission": { "*": "allow" }
                    // Note: egosync tools are provided via .opencode/tools/ custom tools
                }),
            );
            self.save(&config)?;
        }
        Ok(())
    }

    pub fn sync_role_created(&self, role: &Role) -> Result<(), AppError> {
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
        agents.insert(key, Self::build_agent_entry(role));
        self.save(&config)
    }

    pub fn sync_role_updated(&self, role: &Role) -> Result<(), AppError> {
        // Same mechanics as create — overwrites the entry.
        self.sync_role_created(role)
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
    pub fn full_sync(&self, roles: &[Role]) -> Result<(), AppError> {
        let mut config = self.load()?;
        let root = config
            .as_object_mut()
            .ok_or_else(|| AppError::SidecarError("opencode.json 不是 object".to_string()))?;

        let mut agents = serde_json::Map::new();

        // Butler — always present. Remove legacy tools config if present.
        let existing_butler = root.get("agent").and_then(|a| a.get(BUTLER_KEY)).cloned();
        let mut butler_entry = existing_butler.unwrap_or_else(|| {
            json!({
                "name": "管家",
                "mode": "primary",
                "prompt": "你是EgoSync管家",
                "permission": { "*": "allow" }
            })
        });
        // Clean up legacy MCP tools config from butler
        if let Some(obj) = butler_entry.as_object_mut() {
            obj.remove("tools");
        }
        agents.insert(BUTLER_KEY.to_string(), butler_entry);

        // Butler-exclusive tools that role agents must NOT see
        let butler_only_tools = json!({
            "create_role": false,
            "delegate_to_role": false,
            "record_emergence_rejection": false
        });

        // Sync each role
        for role in roles {
            let key = Self::role_to_agent_key(&role.id);
            let mut entry = Self::build_agent_entry(role);
            if role.status == "archived" {
                entry
                    .as_object_mut()
                    .map(|o| o.insert("disable".to_string(), json!(true)));
            }
            // Disable butler-exclusive custom tools for role agents
            entry
                .as_object_mut()
                .map(|o| o.insert("tools".to_string(), butler_only_tools.clone()));
            agents.insert(key, entry);
        }

        root.insert("agent".to_string(), Value::Object(agents));

        // Remove legacy MCP config if present (migration from MCP → custom tools)
        root.remove("mcp");
        root.remove("tools");

        self.save(&config)
    }
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
        // WHY: opencode needs mode=subagent + composite prompt containing
        // the role's goal so the LLM receives the correct persona.
        let role = active_role("r1", "产品经理", "管理产品规划");
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["mode"], "subagent");
        assert!(entry["prompt"].as_str().unwrap().contains("管理产品规划"));
        assert_eq!(entry["disable"], false);
    }

    // ── permission mapping ────────────────────────────────────────

    #[test]
    fn permission_defaults_to_allow_all_when_skills_config_empty() {
        // WHY: a role with no explicit permission config must not accidentally
        // be locked out of all tools — safe default is allow-all.
        let role = active_role("r1", "PM", "goal");
        let entry = AgentConfigService::build_agent_entry(&role);
        assert_eq!(entry["permission"], json!({ "*": "allow" }));
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

    // ── sync_role_created ─────────────────────────────────────────

    #[test]
    fn sync_role_created_adds_agent_to_config() {
        // WHY: creating an EgoSync role must immediately make a matching
        // opencode agent available so the role can converse via opencode.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        let role = active_role("r1", "产品经理", "管理产品");
        svc.sync_role_created(&role).unwrap();

        let config = svc.load().unwrap();
        assert!(config["agent"]["role-r1"].is_object());
        assert_eq!(config["agent"]["role-r1"]["name"], "产品经理");
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
        // WHY: the butler must always exist as the primary agent — it is the
        // user's default conversational entry point.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        svc.ensure_butler().unwrap();

        let config = svc.load().unwrap();
        assert_eq!(config["agent"]["butler"]["mode"], "primary");
        assert_eq!(config["agent"]["butler"]["permission"]["*"], "allow");
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
        // WHY: startup sync must produce an opencode.json that exactly matches
        // DB state — stale/orphan entries cause ghost agents.
        let dir = tempfile::tempdir().unwrap();
        let svc = AgentConfigService::new(dir.path().join("opencode.json"));

        // Seed an orphan entry that should be cleaned up
        svc.sync_role_created(&active_role("orphan", "Orphan", "x"))
            .unwrap();

        let roles = vec![
            active_role("r1", "PM", "manage"),
            make_role("r2", "Archived", "old", "archived", "{}"),
        ];
        svc.full_sync(&roles).unwrap();

        let config = svc.load().unwrap();
        let agents = config["agent"].as_object().unwrap();

        // butler always present
        assert!(agents.contains_key("butler"));
        // active role present
        assert!(agents.contains_key("role-r1"));
        // archived role present but disabled
        assert_eq!(agents["role-r2"]["disable"], true);
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
    fn full_sync_removes_legacy_mcp_config() {
        // WHY: migration from MCP to custom tools must clean up old config.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let svc = AgentConfigService::new(path.clone());

        // Seed legacy MCP config
        let legacy = json!({
            "agent": {},
            "mcp": { "egosync": { "type": "remote", "url": "http://127.0.0.1:4097/mcp" } },
            "tools": { "egosync*": true }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&legacy).unwrap()).unwrap();

        svc.full_sync(&[]).unwrap();

        let config = svc.load().unwrap();
        assert!(config.get("mcp").is_none(), "MCP config must be removed");
        assert!(
            config.get("tools").is_none(),
            "global tools config must be removed"
        );
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
}
