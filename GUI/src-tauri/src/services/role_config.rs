use serde_json::{json, Value};

use crate::error::AppError;
use crate::models::role::UpdateRoleSkillsInput;

pub const FIND_SKILLS_KEY: &str = "find-skills";
pub const SKILL_CREATOR_KEY: &str = "skill-creator";

pub fn normalize_skills_config(input: &UpdateRoleSkillsInput) -> Result<String, AppError> {
    serde_json::to_string(&json!({
        FIND_SKILLS_KEY: input.find_skills,
        SKILL_CREATOR_KEY: input.skill_creator,
    }))
    .map_err(|e| AppError::ValidationError(format!("Skill 配置序列化失败: {}", e)))
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
    serde_json::from_str::<Value>(skills_config)
        .ok()
        .and_then(|value| value.get(key).and_then(|enabled| enabled.as_bool()))
        .unwrap_or(false)
}

pub fn skills_from_config(skills_config: &str) -> UpdateRoleSkillsInput {
    UpdateRoleSkillsInput {
        find_skills: skill_enabled(skills_config, FIND_SKILLS_KEY),
        skill_creator: skill_enabled(skills_config, SKILL_CREATOR_KEY),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_skills_config_writes_meta_skill_keys() {
        let json = normalize_skills_config(&UpdateRoleSkillsInput {
            find_skills: true,
            skill_creator: false,
        })
        .unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[FIND_SKILLS_KEY], true);
        assert_eq!(parsed[SKILL_CREATOR_KEY], false);
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
