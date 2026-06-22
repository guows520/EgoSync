/// Story 4.5: 通知数据模型
///
/// `NotificationLevel` 枚举不复用此处定义，统一从 `services::suggestion_generator` 导入。
/// 此模块只定义 DB 行映射结构和 IPC payload。

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub role_id: String,
    pub level: String,
    pub content: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NotificationWithRole {
    pub id: String,
    pub role_id: String,
    pub level: String,
    pub content: String,
    pub is_read: bool,
    pub created_at: String,
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationInput {
    pub role_id: String,
    pub level: String,
    pub content: String,
}

/// `notification:new` Tauri Event payload（camelCase，供前端直接使用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationNewPayload {
    pub id: String,
    pub level: String,
    pub content: String,
    pub role_id: String,
    pub role_name: String,
    pub role_icon: String,
    pub role_color: String,
    pub created_at: String,
}
