use serde_json::json;
use sqlx::SqlitePool;

use crate::db::app_settings;
use crate::error::AppError;
use crate::models::role::{ButlerSkillsConfig, UpdateRoleSkillsInput};

const BUTLER_SKILLS_CONFIG_KEY: &str = "butler.skills_config";

pub fn default_butler_skills() -> ButlerSkillsConfig {
    ButlerSkillsConfig {
        find_skills: true,
        skill_creator: false,
        enabled_skill_ids: Vec::new(),
    }
}

pub fn skills_config_json(skills: &ButlerSkillsConfig) -> String {
    serde_json::to_string(&json!({
        crate::services::role_config::FIND_SKILLS_KEY: skills.find_skills,
        crate::services::role_config::SKILL_CREATOR_KEY: skills.skill_creator,
        crate::services::role_config::ENABLED_SKILL_IDS_KEY: skills.enabled_skill_ids,
        "meta": {
            "findSkills": skills.find_skills,
            "skillCreator": skills.skill_creator,
        },
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

pub async fn get_butler_skills(pool: &SqlitePool) -> Result<ButlerSkillsConfig, AppError> {
    if let Some(raw) = app_settings::get_setting(pool, BUTLER_SKILLS_CONFIG_KEY).await? {
        Ok(crate::services::role_config::skills_from_config(&raw).into())
    } else {
        Ok(default_butler_skills())
    }
}

pub async fn set_butler_skills_config(
    pool: &SqlitePool,
    skills: &ButlerSkillsConfig,
) -> Result<ButlerSkillsConfig, AppError> {
    let normalized = skills_config_json(skills);
    app_settings::set_setting(pool, BUTLER_SKILLS_CONFIG_KEY, &normalized).await?;
    Ok(skills.clone())
}

pub async fn set_butler_skills(
    pool: &SqlitePool,
    input: &UpdateRoleSkillsInput,
) -> Result<ButlerSkillsConfig, AppError> {
    let normalized = crate::services::role_config::normalize_skills_config(input)?;
    app_settings::set_setting(pool, BUTLER_SKILLS_CONFIG_KEY, &normalized).await?;
    Ok(crate::services::role_config::skills_from_config(&normalized).into())
}

impl From<UpdateRoleSkillsInput> for ButlerSkillsConfig {
    fn from(value: UpdateRoleSkillsInput) -> Self {
        Self {
            find_skills: value.find_skills,
            skill_creator: value.skill_creator,
            enabled_skill_ids: value.enabled_skill_ids.unwrap_or_default(),
        }
    }
}
