#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDecompositionItem {
    pub title: String,
    pub deadline: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskDecompositionProposalInput {
    pub role_id: String,
    pub source_conversation_id: String,
    pub task_summary: String,
    pub items: Vec<TaskDecompositionItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDecompositionProposal {
    pub id: String,
    pub role_id: String,
    pub source_conversation_id: String,
    pub task_summary: String,
    pub items: Vec<TaskDecompositionItem>,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDecompositionProposalWithRole {
    #[serde(flatten)]
    pub proposal: TaskDecompositionProposal,
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
}
