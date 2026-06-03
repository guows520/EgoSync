#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub goal: String,
    pub personality_prompt: String,
    pub status: String,
    pub energy: i32,
    pub skills_config: String,
    pub proactivity_level: String,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleInput {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleInput {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub goal: Option<String>,
    pub personality_prompt: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleSkillsInput {
    pub find_skills: bool,
    pub skill_creator: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButlerSkillsConfig {
    pub find_skills: bool,
    pub skill_creator: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleProactivityInput {
    pub proactivity_level: String,
}
