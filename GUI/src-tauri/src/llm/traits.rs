use crate::error::AppError;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Token(String),
    Thinking(String),
    ToolCall(ToolCall),
    Done,
    Error(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct ChatOptions {
    pub disable_thinking: bool,
    pub tools: Option<Vec<ToolDefinition>>,
    /// OpenAI-compatible tool_choice: "auto", "required", or "none".
    /// When None, the field is omitted from the request (API default = "auto").
    pub tool_choice: Option<String>,
}

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn test_connection(&self) -> Result<(), AppError>;
    async fn chat_stream(
        &self,
        messages: Vec<ChatCompletionMessage>,
        tx: mpsc::Sender<StreamEvent>,
        options: ChatOptions,
    ) -> Result<(), AppError>;
}
