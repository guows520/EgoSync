#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub role_id: Option<String>,
    pub title: String,
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleUpdatedPayload {
    pub conversation_id: String,
    pub title: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub thinking_content: String,
    pub is_complete: bool,
    pub created_at: String,
    /// Story 2.3: 当此 user message 触发了管家委派时，记录所有 delegation 的审计 JSON。
    /// 形如 `{"delegations":[{"targetRoleId":"R","targetRoleName":"产品经理","taskSummary":"...","status":"ok"}, ...]}`。
    /// 未发起委派的消息此字段为 NULL。
    pub routing_metadata: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPayload {
    pub conversation_id: String,
    pub token: String,
    pub done: bool,
    pub thinking: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleProposedPayload {
    pub conversation_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub conversation_id: Option<String>,
    pub role_id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub onboarding_step: u8,
}
