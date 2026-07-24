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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MessageProcessEvent {
    pub id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub opencode_session_id: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub status: Option<String>,
    pub summary: String,
    pub raw_json: String,
    pub working_directory: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPayload {
    pub conversation_id: String,
    pub token: String,
    pub done: bool,
    pub thinking: bool,
    /// Story 2.3: 切换到新 assistant 气泡时携带其 id；
    /// 普通单段流式不带（None），前端落到默认气泡桶。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_event: Option<MessageProcessEvent>,
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
    pub working_directory: Option<String>,
    #[serde(default)]
    pub onboarding_step: u8,
    /// Story 10.1: 用户通过 `@Skill` 显式指定本轮任务使用的 统一 Skill key（registry/meta）。
    /// 未指定时为 None，走现有 `/message` 路径（AC-6）。
    #[serde(default)]
    pub selected_skill_id: Option<String>,
}
