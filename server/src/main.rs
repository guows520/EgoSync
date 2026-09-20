//! egosync-server bin 入口：tracing init + env 配置 + 完整引导 + 优雅退出。
//!
//! env（Story 15.4 冻结面 + 16.1 静态目录）：
//! - `EGOSYNC_DATA_DIR`（必填）：数据目录（双库 + secrets.json + workspace）；
//! - `EGOSYNC_TOKEN`（可选）：env 态引导令牌——存在即冻结为「login 仅常时
//!   比对 env、setup 不挂载」形态；不存在走库态 Argon2id（首访 setup）；
//! - `EGOSYNC_STATIC_DIR`（可选，16.1）：静态目录（`egosync-app/dist` 构建
//!   产物）——未设时默认 `../egosync-app/dist`（存在即用；缺失则 tracing
//!   警告并 API-only，进程不崩溃）；
//! - `EGOSYNC_HOST`（默认 `127.0.0.1`——安全默认，公网暴露需显式声明）；
//! - `EGOSYNC_PORT`（默认 `8080`）；
//! - `RUST_LOG`（默认 info）。
//!
//! 连接边界（F11，二轮评审修复 #2）：**空闲超时 120s（双向静默才断开，
//! [`IDLE_TIMEOUT_SECS`]）+ 响应超时禁用**（chat 分钟级挂起是正常态）。
//! SSE 的 30s KeepAlive 心跳是写活动——健康 SSE 连接永不触发空闲断开；
//! 优雅停机与 8s 兜底语义不受影响（poll_shutdown 透传不走超时判定）。
//!
//! 优雅退出（relay-server 同款双信号）：Ctrl-C / SIGTERM → 取消全局
//! token（watchdog / delegate 监听 / 调度器随取消收尾）+ sidecar stop。

use std::time::Duration;

use egosync_server::bootstrap::build_app_state;
use egosync_server::build_router;
use egosync_server::idle_timeout::{IdleTimeoutListener, IDLE_TIMEOUT_SECS};

/// 优雅退出兜底上限（秒）：给 SSE 长连接与在途请求留收尾窗口。
const SHUTDOWN_DEADLINE_SECS: u64 = 8;

/// 静态目录默认路径（相对 server 进程 CWD——`cargo run` 自 server/ 发起）。
const DEFAULT_STATIC_DIR: &str = "../egosync-app/dist";

/// 解析静态目录（Story 16.1）：env `EGOSYNC_STATIC_DIR` 显式设置优先
/// （不校验存在性——ServeDir 对缺失目录自然 404，警告交由运行期响应呈现）；
/// 未设时默认 `../egosync-app/dist` 存在即用；两者皆缺 ⇒ API-only +
/// tracing 警告（I/O 矩阵：不崩溃，API 面照常服务）。
fn resolve_static_dir() -> Option<std::path::PathBuf> {
    match std::env::var("EGOSYNC_STATIC_DIR") {
        Ok(v) if !v.is_empty() => {
            let dir = std::path::PathBuf::from(v);
            if !dir.is_dir() {
                tracing::warn!(
                    path = %dir.display(),
                    "EGOSYNC_STATIC_DIR 已设置但目录不存在——静态面将整体 404"
                );
            }
            Some(dir)
        }
        _ => {
            let default = std::path::PathBuf::from(DEFAULT_STATIC_DIR);
            if default.is_dir() {
                tracing::info!(
                    path = %default.display(),
                    "静态目录未显式设置，默认路径存在即用（EGOSYNC_STATIC_DIR 可覆盖）"
                );
                Some(default)
            } else {
                tracing::warn!(
                    "EGOSYNC_STATIC_DIR 未设置且默认路径 {} 不存在——server 以 API-only 模式运行（浏览器入口不可用）",
                    DEFAULT_STATIC_DIR
                );
                None
            }
        }
    }
}

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
    let port = match std::env::var("EGOSYNC_PORT") {
        Ok(v) if v.is_empty() => "8080".to_string(),
        Ok(v) => v,
        Err(_) => "8080".to_string(),
    };
    // 端口非法（非 u16）：明确报错退出，不 panic（评审修复 #9）
    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("EGOSYNC_PORT 非法（须为 0-65535 端口号）: {}", port);
            std::process::exit(2);
        }
    };
    let addr = format!("{}:{}", host, port);

    // 静态目录解析（16.1）：env 显式设置优先；未设时默认 ../egosync-app/dist
    // （存在即用）；两者皆缺 ⇒ API-only + tracing 警告（不崩溃）
    let static_dir = resolve_static_dir();

    let state = build_app_state(data_dir, env_token)
        .await
        .unwrap_or_else(|e| {
            eprintln!("server 引导失败: {}", e);
            std::process::exit(1);
        });
    let app = build_router(state.clone(), static_dir);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        // 二轮评审修复 #7：监听失败不 panic——明确报错 + 退出
        //（与 EGOSYNC_PORT/EGOSYNC_DATA_DIR 同款启动期错误处理）
        eprintln!("监听 {} 失败: {}", addr, e);
        std::process::exit(1);
    });
    // F11 空闲超时：双向静默 > IDLE_TIMEOUT_SECS 断开（见模块文档）
    let listener = IdleTimeoutListener::new(listener, Duration::from_secs(IDLE_TIMEOUT_SECS));
    tracing::info!(addr = %addr, idle_timeout_secs = IDLE_TIMEOUT_SECS, "egosync-server listening");

    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<egosync_server::idle_timeout::RemoteAddr>(),
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
