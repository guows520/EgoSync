pub const BUTLER_SCOPE_ID: &str = "__butler__";
pub const SOURCE_TYPE_CUSTOM: &str = "custom";
pub const SOURCE_TYPE_OPENCODE: &str = "opencode";

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
pub struct OpencodeSkillCandidate {
    pub name: String,
    pub description: String,
    pub source_location: String,
    pub source_path: String,
    pub source_type: String,
    pub content_hash: String,
    pub already_imported: bool,
    pub duplicate: Option<SkillDuplicateInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpencodeSkillSkippedSummary {
    pub total: usize,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverOpencodeSkillsResult {
    pub items: Vec<OpencodeSkillCandidate>,
    pub skipped: OpencodeSkillSkippedSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOpencodeSkillInput {
    pub source_path: String,
    #[serde(default)]
    pub role_scope: Option<SkillRoleScope>,
    /// 发现时展示给用户的 content_hash。导入时用于比对源文件是否在
    /// discover→import 之间被替换（TOCTOU 一致性校验）。为兼容旧调用允许缺省。
    #[serde(default)]
    pub expected_content_hash: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportOpencodeSkillResult {
    pub status: String,
    pub entry: Option<SkillRegistryEntry>,
    /// registry 写入后是否已成功同步到 opencode agent 配置。
    /// command 层在 full_sync 失败时置为 false，前端据此避免谎称"已启用"。
    pub synced: bool,
    pub runtime_ready: bool,
    pub runtime_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PickCustomSkillDirectoryResult {
    pub content: String,
    pub source_path: String,
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
    pub source_path: Option<String>,
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
    pub runtime_ready: bool,
    pub runtime_error: Option<String>,
}
