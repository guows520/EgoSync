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

/// `llm:stream` StreamPayload.phase 值域（跨端契约锚点，SPEC-companion-connection-chat-ux）：
/// Android StreamCoordinator 以 phase 判定正文（answering / null=历史 SSE 路径）、思考、
/// 工具行、过程事件与收口；companion-android 黄金契约 fixture（app/src/test/resources/streaming/）
/// 与本常量集对齐。新增或改名 phase 值必须同步本文件测试与手机端判定，否则旧手机静默错乱。
pub const STREAM_PHASE_THINKING: &str = "thinking";
pub const STREAM_PHASE_ANSWERING: &str = "answering";
pub const STREAM_PHASE_TOOL: &str = "tool";
pub const STREAM_PHASE_PROCESS: &str = "process";
pub const STREAM_PHASE_DONE: &str = "done";

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

#[cfg(test)]
mod tests {
    use super::*;

    /// WHY：phase 值域是与手机伴侣 StreamCoordinator 的跨端契约——answering/null
    /// 判定为正文。字面值一旦漂移（改名/拼写），手机端流式归类会静默失效（表现为
    /// 空光标后整段回填的历史缺陷形态）。本测试是 Android 端黄金契约 fixture
    /// （companion-android app/src/test/resources/streaming/）的桌面侧锚点（用户裁决 A+B 双轨）。
    #[test]
    fn stream_phase_domain_is_locked() {
        assert_eq!(STREAM_PHASE_THINKING, "thinking");
        assert_eq!(STREAM_PHASE_ANSWERING, "answering");
        assert_eq!(STREAM_PHASE_TOOL, "tool");
        assert_eq!(STREAM_PHASE_PROCESS, "process");
        assert_eq!(STREAM_PHASE_DONE, "done");
    }

    /// WHY：手机端按 serde 序列化形状解析 STREAM_TOKEN.data（org.json 逐字段读取）；
    /// 字段名（camelCase）或 Option skip 语义漂移会让 phase/messageId 永远读不到。
    #[test]
    fn stream_payload_serializes_android_contract_shape() {
        let answering = StreamPayload {
            conversation_id: "conv-gold".to_string(),
            token: "早".to_string(),
            done: false,
            thinking: false,
            message_id: Some("m-1".to_string()),
            phase: Some(STREAM_PHASE_ANSWERING.to_string()),
            status_text: None,
            tool_name: None,
            process_event: None,
        };
        let json = serde_json::to_value(&answering).unwrap();
        assert_eq!(json["conversationId"], "conv-gold");
        assert_eq!(json["token"], "早");
        assert_eq!(json["phase"], "answering");
        assert!(json.get("statusText").is_none());

        let legacy = StreamPayload {
            conversation_id: "conv-gold".to_string(),
            token: "历史".to_string(),
            done: true,
            thinking: false,
            message_id: None,
            phase: None,
            status_text: None,
            tool_name: None,
            process_event: None,
        };
        let json = serde_json::to_value(&legacy).unwrap();
        assert!(json.get("phase").is_none());
        assert!(json.get("messageId").is_none());
    }
}
