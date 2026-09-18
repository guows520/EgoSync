//! egosync-server bin 入口：tracing init + env 配置 + 完整引导 + 优雅退出。
//!
//! env（Story 15.4 冻结面）：
//! - `EGOSYNC_DATA_DIR`（必填）：数据目录（双库 + secrets.json + workspace）；
//! - `EGOSYNC_TOKEN`（可选）：env 态引导令牌——存在即冻结为「login 仅常时
//!   比对 env、setup 不挂载」形态；不存在走库态 Argon2id（首访 setup）；
//! - `EGOSYNC_HOST`（默认 `127.0.0.1`——安全默认，公网暴露需显式声明）；
//! - `EGOSYNC_PORT`（默认 `8080`）；
//! - `RUST_LOG`（默认 info）。
//!
//! 优雅退出（relay-server 同款双信号）：Ctrl-C / SIGTERM → 取消全局
//! token（watchdog / delegate 监听 / 调度器随取消收尾）+ sidecar stop。

use std::time::Duration;

use egosync_server::bootstrap::build_app_state;
use egosync_server::build_router;

/// 优雅退出兜底上限（秒）：给 SSE 长连接与在途请求留收尾窗口。
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

    let data_dir = match std::env::var("EGOSYNC_DATA_DIR") {
        Ok(v) if !v.is_empty() => std::path::PathBuf::from(v),
        _ => {
            eprintln!("EGOSYNC_DATA_DIR 未设置——server 需要显式数据目录（双库 + secrets.json + opencode workspace）");
            std::process::exit(2);
        }
    };
    let env_token = std::env::var("EGOSYNC_TOKEN").ok().filter(|t| !t.is_empty());
    if env_token.is_some() {
        tracing::info!("引导形态：env 令牌（EGOSYNC_TOKEN）——setup 不挂载，login 仅常时比对 env");
    } else {
        tracing::info!("引导形态：库态 Argon2id（首访 /api/setup 写入令牌哈希）");
    }

    let host = std::env::var("EGOSYNC_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("EGOSYNC_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    let state = build_app_state(data_dir, env_token)
        .await
        .unwrap_or_else(|e| {
            eprintln!("server 引导失败: {}", e);
            std::process::exit(1);
        });
    let app = build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("监听 {addr} 失败: {e}"));
    tracing::info!(addr = %addr, "egosync-server listening");

    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        // 退出清理：取消全局 token（watchdog/delegate/调度器收尾）+ sidecar stop
        state.cancel.cancel();
        let sidecar = state.sidecar.clone();
        let stop_result = sidecar.lock().await.stop().await;
        if let Err(e) = stop_result {
            tracing::warn!("opencode sidecar 停止失败: {}", e);
        }
        // 持有 state 至关闭完成（serve 与 shutdown 回调共享生命周期）
        drop(state);
    });

    // 兜底：个别连接若对关闭无响应，serve 不得无限等待——退出信号后最多 8s
    tokio::select! {
        result = serve => result.expect("server error"),
        _ = shutdown_deadline() => tracing::warn!(
            deadline_secs = SHUTDOWN_DEADLINE_SECS,
            "优雅退出超时，强制退出（仍有连接未关闭）"
        ),
    }
}

/// 兜底计时：等退出信号，再等 SHUTDOWN_DEADLINE_SECS 秒。
async fn shutdown_deadline() {
    shutdown_signal().await;
    tokio::time::sleep(Duration::from_secs(SHUTDOWN_DEADLINE_SECS)).await;
}

/// SIGTERM / SIGINT → 优雅退出。
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
