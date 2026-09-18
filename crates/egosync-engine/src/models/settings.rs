use serde::{Deserialize, Serialize};

/// 模型网络位置：internal = 内网直连（绕过代理），external = 外网（走系统代理）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkLocation {
    Internal,
    External,
}

impl NetworkLocation {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkLocation::Internal => "internal",
            NetworkLocation::External => "external",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "internal" => Ok(NetworkLocation::Internal),
            "external" => Ok(NetworkLocation::External),
            other => Err(format!("无效的网络位置 '{}', 仅允许 'internal' 或 'external'", other)),
        }
    }
}

impl TryFrom<String> for NetworkLocation {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> { Self::from_str(&value) }
}

impl Default for NetworkLocation {
    fn default() -> Self {
        NetworkLocation::External
    }
}

impl std::fmt::Display for NetworkLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key_ref: String,
    pub is_default: bool,
    #[sqlx(try_from = "String")]
    pub network_location: NetworkLocation,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLlmConfigInput {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub network_location: NetworkLocation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLlmConfigInput {
    pub name: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub network_location: Option<NetworkLocation>,
}
