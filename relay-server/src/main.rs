//! relay-server bin 入口：tracing init + env 配置 + 优雅退出。
//!
//! 监听地址：`RELAY_HOST`（默认 `0.0.0.0`）+ `RELAY_PORT`（默认 `7333`）。
//! 日志级别：`RUST_LOG`（默认 info）。
//!
//! 优雅退出（评审整改）：SIGTERM/SIGINT 时先经 Registry 广播关闭信号——
//! 全部活跃 WS 连接向客户端发送 Close 后退出，`axum::serve` 才能真正返回；
//! 并设 8s 兜底超时（Docker stop 默认 10s），防个别客户端拖住进程到 SIGKILL。

use std::time::Duration;

use relay_server::{build_router_with, AppState};

/// 优雅退出兜底上限（秒）：Docker stop 默认给 10s，留 2s 余量强制返回。
const SHUTDOWN_DEADLINE_SECS: u64 = 8;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let host = std::env::var("RELAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("RELAY_PORT").unwrap_or_else(|_| "7333".to_string());
    let addr = format!("{host}:{port}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("监听 {addr} 失败: {e}"));

    tracing::info!(addr = %addr, "relay-server listening");

    let state = AppState::default();
    let app = build_router_with(state.clone());
    let shutdown_state = state;
    let serve = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown_signal().await;
        // 广播关闭：活跃连接发 Close 后退出，serve 才会返回（不挂到 SIGKILL）
        shutdown_state.registry.shutdown_all();
    });

    // 兜底：个别客户端若对 Close 无响应，serve 不得无限等待
    match tokio::time::timeout(Duration::from_secs(SHUTDOWN_DEADLINE_SECS), serve).await {
        Ok(result) => result.expect("server error"),
        Err(_) => tracing::warn!(
            deadline_secs = SHUTDOWN_DEADLINE_SECS,
            "优雅退出超时，强制退出（仍有连接未关闭）"
        ),
    }
}

/// SIGTERM / SIGINT → 优雅退出（Docker stop 场景）。
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("安装 SIGINT handler 失败");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("安装 SIGTERM handler 失败")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
