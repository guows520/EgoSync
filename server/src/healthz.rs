//! 存活/深度探针（Story 15.4）：`GET /healthz`（+ `?deep=1` / `?deep=true`）。
//!
//! - 无认证豁免（「静态资源豁免」指认证豁免面，静态服务归 16.1）；
//! - `?deep=1` / `?deep=true`：DB SELECT 1（双池）+ sidecar health_check
//!   全过 ⇒ 200+flags，任一败 ⇒ 503+flags（5xx 家族——升级前巡检语义）。
//!   同时接受 `1` 与 `true`（二轮评审修复 #8：监控方写 `deep=true` 曾
//!   静默拿到浅探针假 200——严格匹配 `1` 反成可用性陷阱）。

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::AppState;

#[derive(Deserialize)]
pub struct HealthzQuery {
    /// `?deep=1` 或 `?deep=true` 触发深度探针（大小写不敏感）。
    deep: Option<String>,
}

/// deep 参数是否触发深度探针：`1` / `true`（大小写不敏感——监控方
/// 常见两种写法都得拿到真实探针结果）。
fn is_deep(deep: Option<&str>) -> bool {
    matches!(deep.map(str::to_ascii_lowercase).as_deref(), Some("1") | Some("true"))
}

/// `GET /healthz`：存活探针 + 可选深度探针。
pub async fn healthz(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HealthzQuery>,
) -> Response {
    if !is_deep(query.deep.as_deref()) {
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
