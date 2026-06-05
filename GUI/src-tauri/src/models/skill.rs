pub const BUTLER_SCOPE_ID: &str = "__butler__";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRoleScope {
    #[serde(default)]
    pub all_roles: bool,
    #[serde(default)]
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_type: String,
    pub managed_path: String,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillImportPreview {
    pub name: String,
    pub description: String,
    pub content_hash: String,
    pub duplicate: Option<SkillDuplicateInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillDuplicateInfo {
    pub kind: String,
    pub existing: SkillRegistryEntry,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PickCustomSkillDirectoryResult {
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCustomSkillInput {
    pub content: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCustomSkillInput {
    pub content: Option<String>,
    #[serde(default)]
    pub overwrite_existing: bool,
    #[serde(default)]
    pub role_scope: Option<SkillRoleScope>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCustomSkillResult {
    pub status: String,
    pub entry: Option<SkillRegistryEntry>,
    pub preview: SkillImportPreview,
}
