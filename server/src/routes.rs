//! 命令路由（Story 15.4）：`POST /api/cmd/{command}` —— web-ok 命令唯一
//! 业务入口（无 REST 旁路端点）。
//!
//! - 参数 camelCase body 与桌面 invoke 同构（dispatch_gen 生成物解包）；
//! - 业务错误一律 `200 + AppError 原样单键 map`（与 invoke 错误通道同构），
//!   且追加判别头 `X-Egosync-App-Error: 1`（Story 15.5：HTTP 通道错误信号，
//!   前端 HttpTransport 据此解包 reject——与 Tauri rejection 对偶）；
//! - desktop-only / perf-test 门控 / 未知名 → 404（物理不路由）；
//! - body >50MB → 413（DefaultBodyLimit 传输层原语）；
//! - 业务端点无响应超时（chat 分钟级挂起是正常态）。

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde_json::{json, Value};

use crate::dispatch_gen;
use crate::AppState;

/// AppError 响应判别头（Story 15.5）：`X-Egosync-App-Error: 1`。
///
/// server 现状对 Ok/Err 一律 `200 + Json(value)`，HTTP 层无错误判别信号
/// （成功响应恰为单键 variant 名对象时按形状探测会误判——Design Notes
/// 裁决）。头只是 HTTP 通道的错误信号（与 Tauri rejection 信号对偶），
/// 状态码与 body 冻结款零改动。
pub const APP_ERROR_HEADER: &str = "x-egosync-app-error";
/// 判别头值（非布尔语义，仅「存在即错误」）。
pub const APP_ERROR_HEADER_VALUE: &str = "1";

/// `POST /api/cmd/{command}`：认证后命令分发。
pub async fn cmd_handler(
    State(state): State<Arc<AppState>>,
    Path(command): Path<String>,
    body: Bytes,
) -> Response {
    // 物理路由面：仅 web-ok 名单（desktop-only / perf-test 门控 / 未知名 404）
    if !dispatch_gen::WEB_OK_COMMANDS.contains(&command.as_str()) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "unknown command"})),
        )
            .into_response();
    }

    // 参数解包：空 body ⇒ 空对象（与 invoke 缺省参数语义同构）；
    // 非 JSON / 类型不符 ⇒ 200 + ValidationError 单键 map（错误白名单口径）
    let params: Value = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::OK,
                    [(APP_ERROR_HEADER, APP_ERROR_HEADER_VALUE)],
                    Json(
                        egosync_engine::error::AppError::ValidationError(format!(
                            "请求体不是有效 JSON: {}",
                            e
                        )),
                    ),
                )
                    .into_response();
            }
        }
    };

    match dispatch_gen::dispatch(&state.ctx, &command, params).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        // AppError 单键 map Serialize 与 invoke 错误通道同构（error.rs 既有实现）；
        // Story 15.5：Err 分支追加判别头 `X-Egosync-App-Error: 1`（状态码与
        // body 冻结款零改动）——HTTP 通道的错误信号，与 Tauri 通道的 promise
        // rejection 对偶：前端 HttpTransport 据此把 200 body 解析值转为 reject，
        // 解包后两侧错误对象同构（ApiError 200 恰为单键 variant 名对象时免误判）。
        Err(err) => (
            StatusCode::OK,
            [(APP_ERROR_HEADER, APP_ERROR_HEADER_VALUE)],
            Json(err),
        )
            .into_response(),
    }
}
