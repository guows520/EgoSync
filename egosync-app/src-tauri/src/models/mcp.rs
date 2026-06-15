use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub command_or_url: String,
    pub env_refs: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMcpServerInput {
    pub name: String,
    pub server_type: String,
    pub command_or_url: String,
    #[serde(default)]
    pub env_refs: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMcpServerInput {
    pub name: Option<String>,
    pub server_type: Option<String>,
    pub command_or_url: Option<String>,
    pub env_refs: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}
