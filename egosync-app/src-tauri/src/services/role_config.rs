use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::models::role::UpdateRoleSkillsInput;

pub const FIND_SKILLS_KEY: &str = "find-skills";
pub const SKILL_CREATOR_KEY: &str = "skill-creator";
pub const ENABLED_SKILL_IDS_KEY: &str = "enabledSkillIds";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoleMetaSkillConfig {
    pub find_skills: bool,
    pub skill_creator: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoleSkillConfigV2 {
    pub meta: RoleMetaSkillConfig,
    #[serde(default)]
    pub enabled_skill_ids: Vec<String>,
    #[serde(default)]
    pub permissions: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

pub fn normalize_skills_config(input: &UpdateRoleSkillsInput) -> Result<String, AppError> {
    normalize_skills_config_with_existing("{}", input)
}

pub fn normalize_skills_config_with_existing(
    existing_config: &str,
    input: &UpdateRoleSkillsInput,
) -> Result<String, AppError> {
    let mut object = normalized_object(existing_config);
    write_meta_skill_state(&mut object, input.find_skills, input.skill_creator);
    if let Some(enabled_skill_ids) = input.enabled_skill_ids.as_ref() {
        object.insert(
            ENABLED_SKILL_IDS_KEY.to_string(),
            Value::Array(
                unique_skill_ids(enabled_skill_ids.iter().cloned())
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    serde_json::to_string(&Value::Object(object))
        .map_err(|e| AppError::ValidationError(format!("Skill 配置序列化失败: {}", e)))
}

pub fn normalize_existing_skills_config(skills_config: &str) -> Result<String, AppError> {
    serde_json::to_string(&Value::Object(normalized_object(skills_config)))
        .map_err(|e| AppError::ValidationError(format!("Skill 配置序列化失败: {}", e)))
}

pub fn role_skill_config_from_json(skills_config: &str) -> RoleSkillConfigV2 {
    serde_json::from_value(Value::Object(normalized_object(skills_config))).unwrap_or_else(|_| {
        RoleSkillConfigV2 {
            meta: RoleMetaSkillConfig {
                find_skills: false,
                skill_creator: false,
                extra: Map::new(),
            },
            enabled_skill_ids: Vec::new(),
            permissions: None,
            extra: Map::new(),
        }
    })
}

pub fn normalize_proactivity_level(level: &str) -> Result<String, AppError> {
    let trimmed = level.trim();
    if matches!(trimmed, "passive" | "moderate" | "proactive") {
        return Ok(trimmed.to_string());
    }
    Err(AppError::ValidationError(
        "主动性级别必须是 passive、moderate 或 proactive".to_string(),
    ))
}

pub fn skill_enabled(skills_config: &str, key: &str) -> bool {
    let config = role_skill_config_from_json(skills_config);
    match key {
        FIND_SKILLS_KEY => config.meta.find_skills,
        SKILL_CREATOR_KEY => config.meta.skill_creator,
        _ => false,
    }
}

pub fn enabled_skill_ids_from_config(skills_config: &str) -> Vec<String> {
    role_skill_config_from_json(skills_config).enabled_skill_ids
}

/// 从角色 skills_config 中移除指定 skill id（P3：删除 Skill 时清理角色死 id）。
/// 返回 `Some(新配置)` 表示发生了变更需写回；`None` 表示该 id 不存在、无需更新。
/// 仅改写 enabledSkillIds，保留 meta、permissions 与未知扩展字段。
pub fn remove_enabled_skill_id(skills_config: &str, skill_id: &str) -> Option<String> {
    let mut object = normalized_object(skills_config);
    let ids = match object.get(ENABLED_SKILL_IDS_KEY).and_then(Value::as_array) {
        Some(arr) => arr.clone(),
        None => return None,
    };
    let retained: Vec<Value> = ids
        .into_iter()
        .filter(|v| v.as_str() != Some(skill_id))
        .collect();
    let original_len = role_skill_config_from_json(skills_config)
        .enabled_skill_ids
        .len();
    if retained.len() == original_len {
        return None;
    }
    object.insert(ENABLED_SKILL_IDS_KEY.to_string(), Value::Array(retained));
    serde_json::to_string(&Value::Object(object)).ok()
}

pub fn skills_from_config(skills_config: &str) -> UpdateRoleSkillsInput {
    let config = role_skill_config_from_json(skills_config);
    UpdateRoleSkillsInput {
        find_skills: config.meta.find_skills,
        skill_creator: config.meta.skill_creator,
        enabled_skill_ids: Some(config.enabled_skill_ids),
    }
}

pub fn enable_skill(skills_config: &str, key: &str) -> Option<UpdateRoleSkillsInput> {
    let mut skills = skills_from_config(skills_config);
    match key {
        FIND_SKILLS_KEY => skills.find_skills = true,
        SKILL_CREATOR_KEY => skills.skill_creator = true,
        _ => return None,
    }
    Some(skills)
}

pub fn meta_skill_prompt(skills_config: &str) -> String {
    let find_skills = skill_enabled(skills_config, FIND_SKILLS_KEY);
    let skill_creator = skill_enabled(skills_config, SKILL_CREATOR_KEY);
    let mut lines = Vec::new();

    if find_skills {
        lines.push("- find-skills（Vercel 官方，用于发现/推荐可用 Skill）：已启用".to_string());
    }
    if skill_creator {
        lines.push("- skill-creator（Anthropic 官方，用于创建/扩展 Skill）：已启用".to_string());
    }

    if lines.is_empty() {
        return String::new();
    }

    let mut prompt = vec!["[元 Skill 配置]".to_string()];
    prompt.extend(lines);
    if find_skills {
        prompt.push(
            "Skill 发现边界：只能介绍 EgoSync 当前启用或可见的 Skill；如果当前没有可见 Skill，就直接说明当前没有可展示 Skill；不得列出 Claude Code、gstack、opencode 或外部环境中的其它 Skill。".to_string(),
        );
    }
    prompt.join("\n")
}

pub fn requested_meta_skill_from_assistant(text: &str) -> Option<&'static str> {
    if text.contains(FIND_SKILLS_KEY) && text.contains("要开启吗") {
        return Some(FIND_SKILLS_KEY);
    }
    if text.contains(SKILL_CREATOR_KEY) && text.contains("要开启吗") {
        return Some(SKILL_CREATOR_KEY);
    }
    None
}

pub fn is_user_confirmation(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "好" | "好的" | "可以" | "开启" | "打开" | "同意" | "确认" | "yes" | "y" | "ok"
    ) || normalized.contains("开启吧")
        || normalized.contains("打开吧")
        || normalized.contains("可以开启")
        || normalized.contains("同意开启")
}

fn normalized_object(skills_config: &str) -> Map<String, Value> {
    let mut object = serde_json::from_str::<Value>(skills_config)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let find_skills = meta_bool(&object, FIND_SKILLS_KEY, "findSkills");
    let skill_creator = meta_bool(&object, SKILL_CREATOR_KEY, "skillCreator");
    write_meta_skill_state(&mut object, find_skills, skill_creator);
    let enabled_skill_ids = object
        .get(ENABLED_SKILL_IDS_KEY)
        .and_then(Value::as_array)
        .map(|items| {
            unique_skill_ids(
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            )
        })
        .unwrap_or_default();
    object.insert(
        ENABLED_SKILL_IDS_KEY.to_string(),
        Value::Array(enabled_skill_ids.into_iter().map(Value::String).collect()),
    );
    object
}

fn write_meta_skill_state(object: &mut Map<String, Value>, find_skills: bool, skill_creator: bool) {
    let mut meta = object
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    meta.insert("findSkills".to_string(), json!(find_skills));
    meta.insert("skillCreator".to_string(), json!(skill_creator));
    object.insert("meta".to_string(), Value::Object(meta));
    object.insert(FIND_SKILLS_KEY.to_string(), json!(find_skills));
    object.insert(SKILL_CREATOR_KEY.to_string(), json!(skill_creator));
}

fn meta_bool(object: &Map<String, Value>, legacy_key: &str, camel_key: &str) -> bool {
    object
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get(camel_key))
        .and_then(Value::as_bool)
        .or_else(|| object.get(legacy_key).and_then(Value::as_bool))
        .or_else(|| object.get(camel_key).and_then(Value::as_bool))
        .unwrap_or(false)
}

fn unique_skill_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut unique = Vec::new();
    for id in ids {
        let trimmed = id.trim();
        if !trimmed.is_empty() && !unique.iter().any(|existing| existing == trimmed) {
            unique.push(trimmed.to_string());
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_skills_config_writes_meta_skill_keys() {
        let json = normalize_skills_config(&UpdateRoleSkillsInput {
            find_skills: true,
            skill_creator: false,
            enabled_skill_ids: None,
        })
        .unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[FIND_SKILLS_KEY], true);
        assert_eq!(parsed[SKILL_CREATOR_KEY], false);
        assert_eq!(parsed["meta"]["findSkills"], true);
        assert_eq!(parsed["meta"]["skillCreator"], false);
        assert!(parsed[ENABLED_SKILL_IDS_KEY].as_array().unwrap().is_empty());
    }

    #[test]
    fn normalize_skills_config_preserves_extensions() {
        let json = normalize_skills_config_with_existing(
            r#"{"find-skills":false,"skill-creator":true,"enabledSkillIds":["custom-a"],"permissions":{"bash":"ask"},"future":{"mcp":["x"]}}"#,
            &UpdateRoleSkillsInput {
                find_skills: true,
                skill_creator: false,
                enabled_skill_ids: None,
            },
        )
        .unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["meta"]["findSkills"], true);
        assert_eq!(parsed["meta"]["skillCreator"], false);
        assert_eq!(parsed[ENABLED_SKILL_IDS_KEY][0], "custom-a");
        assert_eq!(parsed["permissions"]["bash"], "ask");
        assert_eq!(parsed["future"]["mcp"][0], "x");
    }

    #[test]
    fn normalize_skills_config_updates_custom_skill_ids_when_requested() {
        let json = normalize_skills_config_with_existing(
            r#"{"enabledSkillIds":["old"]}"#,
            &UpdateRoleSkillsInput {
                find_skills: false,
                skill_creator: false,
                enabled_skill_ids: Some(vec![
                    "custom-a".to_string(),
                    "custom-a".to_string(),
                    "".to_string(),
                ]),
            },
        )
        .unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[ENABLED_SKILL_IDS_KEY].as_array().unwrap().len(), 1);
        assert_eq!(parsed[ENABLED_SKILL_IDS_KEY][0], "custom-a");
    }

    #[test]
    fn role_skill_config_reads_legacy_and_v2_shapes() {
        let legacy = role_skill_config_from_json(r#"{"find-skills":true,"skill-creator":false}"#);
        assert!(legacy.meta.find_skills);
        assert!(!legacy.meta.skill_creator);

        let v2 = role_skill_config_from_json(
            r#"{"meta":{"findSkills":false,"skillCreator":true},"enabledSkillIds":["custom-a"]}"#,
        );
        assert!(!v2.meta.find_skills);
        assert!(v2.meta.skill_creator);
        assert_eq!(v2.enabled_skill_ids, vec!["custom-a"]);
    }

    #[test]
    fn normalize_proactivity_level_rejects_legacy_values() {
        assert_eq!(normalize_proactivity_level("moderate").unwrap(), "moderate");
        assert!(matches!(
            normalize_proactivity_level("medium"),
            Err(AppError::ValidationError(_))
        ));
    }

    #[test]
    fn meta_skill_prompt_only_lists_enabled_skills() {
        let prompt = meta_skill_prompt(r#"{"find-skills":true,"skill-creator":false}"#);
        assert!(prompt.contains("find-skills"));
        assert!(prompt.contains("已启用"));
        assert!(!prompt.contains("skill-creator"));
        assert!(!prompt.contains("未启用"));
        assert!(!prompt.contains("要开启吗"));
    }

    #[test]
    fn meta_skill_prompt_limits_discovery_to_egosync_visible_skills() {
        let prompt = meta_skill_prompt(r#"{"find-skills":true,"skill-creator":false}"#);

        assert!(prompt.contains("只能介绍 EgoSync 当前启用或可见的 Skill"));
        assert!(prompt.contains("不得列出 Claude Code、gstack、opencode 或外部环境中的其它 Skill"));
        assert!(prompt.contains("没有可见 Skill"));
    }

    #[test]
    fn meta_skill_prompt_is_empty_when_all_skills_disabled() {
        let prompt = meta_skill_prompt(r#"{"find-skills":false,"skill-creator":false}"#);
        assert!(prompt.is_empty());
    }

    #[test]
    fn confirmation_detection_is_intentional() {
        assert!(is_user_confirmation("好的"));
        assert!(is_user_confirmation("可以开启"));
        assert!(!is_user_confirmation("为什么需要？"));
    }
}
