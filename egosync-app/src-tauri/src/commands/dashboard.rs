use tauri::State;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::dashboard::{DashboardMetrics, DashboardMetricsQuery, DashboardStatus};
use crate::services::dashboard_service;

#[tauri::command]
pub async fn dashboard_get_status(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<Vec<DashboardStatus>, AppError> {
    dashboard_service::get_dashboard_status(&pool, &conv_pool).await
}

#[tauri::command]
pub async fn dashboard_get_metrics(
    query: DashboardMetricsQuery,
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<DashboardMetrics, AppError> {
    dashboard_service::get_dashboard_metrics(&pool, &conv_pool, query).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::dashboard::{DashboardMetricsQuery, DashboardMetricsScope};

    // AC-15：验证 IPC 边界参数反序列化 — tagged union 三种形态
    #[test]
    fn deserializes_all_scope_query() {
        let json = r#"{"scope":{"type":"all"},"startAt":null,"endAt":null}"#;
        let q: DashboardMetricsQuery = serde_json::from_str(json).expect("parse all scope");
        assert!(matches!(q.scope, DashboardMetricsScope::All));
        assert!(q.start_at.is_none());
        assert!(q.end_at.is_none());
    }

    #[test]
    fn deserializes_butler_scope_query() {
        let json = r#"{"scope":{"type":"butler"},"startAt":null,"endAt":null}"#;
        let q: DashboardMetricsQuery = serde_json::from_str(json).expect("parse butler scope");
        assert!(matches!(q.scope, DashboardMetricsScope::Butler));
    }

    #[test]
    fn deserializes_role_scope_query() {
        let json = r#"{"scope":{"type":"role","roleId":"r-123"},"startAt":"2026-06-01T00:00:00Z","endAt":null}"#;
        let q: DashboardMetricsQuery = serde_json::from_str(json).expect("parse role scope");
        match q.scope {
            DashboardMetricsScope::Role { role_id } => assert_eq!(role_id, "r-123"),
            _ => panic!("应为 Role scope"),
        }
        assert_eq!(q.start_at.as_deref(), Some("2026-06-01T00:00:00Z"));
        assert!(q.end_at.is_none());
    }

    // AC-15：验证错误类型可通过 serde 在 IPC 边界序列化传播
    #[test]
    fn validation_error_serializes_for_ipc() {
        let err = AppError::ValidationError("startAt 必须早于 endAt".to_string());
        let json = serde_json::to_string(&err).expect("serialize error");
        assert!(json.contains("startAt 必须早于 endAt"));
    }
}
