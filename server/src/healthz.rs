//! 存活/深度探针（Story 15.4）：`GET /healthz`（+ `?deep=1`）。
//!
//! - 无认证豁免（「静态资源豁免」指认证豁免面，静态服务归 16.1）；
//! - `?deep=1`：DB SELECT 1（双池）+ sidecar health_check 全过 ⇒ 200+flags，
//!   任一败 ⇒ 503+flags（5xx 家族——升级前巡检语义）。

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::AppState;

#[derive(Deserialize)]
pub struct HealthzQuery {
    /// `?deep=1` 触发深度探针。
    deep: Option<String>,
}

/// `GET /healthz`：存活探针 + 可选深度探针。
pub async fn healthz(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HealthzQuery>,
) -> Response {
    if query.deep.as_deref() != Some("1") {
        return (StatusCode::OK, Json(json!({"status": "ok"}))).into_response();
    }

    // 深度探针：双池 SELECT 1 + sidecar health_check
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.ctx.pool)
        .await
        .is_ok()
        && sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&*state.ctx.conv_pool)
            .await
            .is_ok();
    let opencode_ok = {
        let mut mgr = state.sidecar.lock().await;
        mgr.health_check().await
    };

    let healthy = db_ok && opencode_ok;
    let status = if healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (
        status,
        Json(json!({
            "status": if healthy { "ok" } else { "degraded" },
            "db": db_ok,
            "opencode": opencode_ok,
        })),
    )
        .into_response()
}
