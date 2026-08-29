//! Story 13.1：桌面快照领域投影结构。
//!
//! 这些结构是**投影**而非内部模型的直接复用——只承载「允许上机的字段」：
//! 角色卡不含 `personality_prompt`/`skills_config`（桌面内部数据不上机，
//! 硬边界 #4）；记忆库内容永不进快照（`memoryCount` 仅作仪表盘指标数字出现，
//! AC1 负向约束）。`schemaVersion` 独立于协议层 `protocolVersion`，走此字段演进。

use serde::{Deserialize, Serialize};

/// 快照业务结构版本（独立于协议层 `PROTOCOL_VERSION`，13.2 消费方校验）。
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// 10MB 截断上限（字节）。超出按裁决 6 顺序截断最旧可截断域。
pub const SNAPSHOT_MAX_BYTES: usize = 10 * 1024 * 1024;

/// 桌面状态全量快照（内存持有、不持久化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSnapshot {
    /// 快照业务结构版本（区别于协议层 protocolVersion）。
    pub schema_version: u32,
    /// 快照生成时刻（ISO 8601 UTC）。
    pub generated_at: String,
    /// 数据截止时间：截断后被截断域中保留数据的最旧时间戳；未截断为 None。
    pub data_cutoff_at: Option<String>,
    /// 是否发生截断。
    pub truncated: bool,
    /// 被截断的域名清单（conversations/briefings/weeklyReviews）。
    pub truncated_domains: Vec<String>,
    pub roles: Vec<SnapshotRole>,
    pub tasks: Vec<SnapshotTask>,
    pub dashboard: SnapshotDashboard,
    pub conversations: Vec<SnapshotConversation>,
    pub briefings: Vec<SnapshotBriefing>,
    pub weekly_reviews: Vec<SnapshotWeeklyReview>,
    pub notifications: Vec<SnapshotNotification>,
}

/// 角色卡投影——不含 `personality_prompt`/`skills_config`（桌面内部数据不上机）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRole {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub goal: String,
    pub status: String,
    pub energy: i32,
    pub proactivity_level: String,
}

/// 四象限任务投影（含跨角色视图的 `role_name`/`role_color`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotTask {
    pub id: String,
    pub owner_type: String,
    pub role_id: Option<String>,
    pub title: String,
    pub deadline: Option<String>,
    pub quadrant: String,
    pub is_big_rock: bool,
    pub is_completed: bool,
    pub protection_status: String,
    pub role_name: Option<String>,
    pub role_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub thinking_content: String,
    pub is_complete: bool,
    pub created_at: String,
}

/// 会话投影（每会话最近 200 条消息——`get_recent_messages` 现成入口）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotConversation {
    pub id: String,
    pub role_id: Option<String>,
    pub title: String,
    pub updated_at: String,
    pub messages: Vec<SnapshotMessage>,
}

/// 仪表盘投影：角色卡态数组 + 四项统计指标（`memoryCount` 仅数字）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDashboard {
    /// 复用 [`crate::models::dashboard::DashboardStatus`]（已是角色卡投影）。
    pub statuses: Vec<crate::models::dashboard::DashboardStatus>,
    pub metrics: SnapshotMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMetrics {
    pub task_count: i64,
    pub memory_count: i64,
    pub conversation_count: i64,
    pub pending_task_count: i64,
    pub generated_at: String,
}

/// 晨间简报投影（本季度过滤后）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotBriefing {
    pub id: String,
    pub content: String,
    pub date: String,
}

/// 周复盘投影（本季度过滤后，按 `week_start` 落区间）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotWeeklyReview {
    pub id: String,
    pub week_start: String,
    pub week_end: String,
    pub summary: String,
    pub energy_trends: String,
    pub bigrock_status: String,
    pub new_memories_count: i64,
}

/// 未读通知投影（仅 `is_read == false`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotNotification {
    pub id: String,
    pub role_id: String,
    pub level: String,
    pub content: String,
    pub created_at: String,
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
}
