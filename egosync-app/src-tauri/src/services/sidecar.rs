use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::error::AppError;

const DEFAULT_PORT: u16 = 4096;
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(500);
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(5);
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const CONSECUTIVE_FAILURES_BEFORE_RESTART: u32 = 2;

/// Manages the opencode sidecar process lifecycle.
pub struct SidecarManager {
    child: Option<Child>,
    port: u16,
    binary_path: Option<PathBuf>,
    working_dir: Option<PathBuf>,
    started_at: Option<Instant>,
    stderr_buf: Arc<Mutex<String>>,
    extra_env: HashMap<String, String>,
}

impl SidecarManager {
    /// Create a new SidecarManager.
    /// `resource_dir` is the Tauri resource directory (production) or None (dev fallback to PATH).
    pub fn new(resource_dir: Option<PathBuf>, port: Option<u16>) -> Self {
        let port = port.unwrap_or(DEFAULT_PORT);
        let binary_path = Self::resolve_binary_path(resource_dir);
        Self {
            child: None,
            port,
            binary_path,
            working_dir: None,
            started_at: None,
            stderr_buf: Arc::new(Mutex::new(String::new())),
            extra_env: HashMap::new(),
        }
    }

    pub fn with_working_dir(mut self, working_dir: impl Into<PathBuf>) -> Self {
        let working_dir = working_dir.into();
        let global_dir = working_dir
            .parent()
            .map(|parent| parent.join("opencode-global"))
            .unwrap_or_else(|| working_dir.join("opencode-global"));
        self.extra_env.insert(
            "XDG_CONFIG_HOME".to_string(),
            global_dir.join("config").to_string_lossy().to_string(),
        );
        self.extra_env.insert(
            "XDG_DATA_HOME".to_string(),
            global_dir.join("data").to_string_lossy().to_string(),
        );
        self.extra_env.insert(
            "XDG_CACHE_HOME".to_string(),
            global_dir.join("cache").to_string_lossy().to_string(),
        );
        self.working_dir = Some(working_dir);
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_env.insert(key.into(), value.into());
        self
    }

    /// Resolve the opencode binary path: resource dir first, then system PATH.
    fn resolve_binary_path(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
        // Try resource directory first
        if let Some(dir) = resource_dir {
            let exe_path = dir.join("opencode.exe");
            if exe_path.exists() {
                tracing::info!("opencode binary found in resources: {}", exe_path.display());
                return Some(exe_path);
            }
            let cmd_path = dir.join("opencode.cmd");
            if cmd_path.exists() {
                tracing::info!("opencode cmd found in resources: {}", cmd_path.display());
                return Some(cmd_path);
            }
        }

        // On Windows, resolve the .cmd shim to find the actual .exe binary.
        // npm .cmd shims launch cmd.exe which exits immediately, leaving
        // the real process untracked by our Child handle.
        if cfg!(target_os = "windows") {
            if let Some(exe_path) = Self::resolve_exe_from_cmd_shim() {
                tracing::info!(
                    "opencode.exe resolved from npm shim: {}",
                    exe_path.display()
                );
                return Some(exe_path);
            }
        }

        // Fallback to system PATH — just use the command name, let OS resolve
        let binary_name = if cfg!(target_os = "windows") {
            "opencode.cmd"
        } else {
            "opencode"
        };
        tracing::info!("opencode binary not in resources, will try system PATH");
        Some(PathBuf::from(binary_name))
    }

    /// On Windows, parse the npm .cmd shim to extract the actual .exe path.
    /// The shim typically contains: `"%dp0%\node_modules\opencode-ai\bin\opencode.exe" %*`
    fn resolve_exe_from_cmd_shim() -> Option<PathBuf> {
        // Find opencode.cmd in PATH using platform-specific lookup
        let cmd_path = Self::find_in_path("opencode.cmd")?;
        let content = std::fs::read_to_string(&cmd_path).ok()?;
        // Look for a line referencing opencode.exe
        for line in content.lines() {
            let trimmed = line.trim().trim_start_matches('"');
            if trimmed.contains("opencode.exe") {
                // Replace %dp0% with the directory containing the .cmd file
                let dp0 = cmd_path.parent()?;
                // Extract path: strip leading quote, %dp0%\, trailing " %*
                let path_part = trimmed.replace("%dp0%\\", "").replace("%dp0%/", "");
                // Remove trailing `"   %*` or similar
                let path_part = path_part
                    .split('"')
                    .next()
                    .unwrap_or(&path_part)
                    .trim()
                    .to_string();
                let exe_path = dp0.join(&path_part);
                if exe_path.exists() {
                    return Some(exe_path);
                }
            }
        }
        None
    }

    /// Simple PATH lookup for a given filename (Windows).
    fn find_in_path(name: &str) -> Option<PathBuf> {
        let path_var = std::env::var("PATH").ok()?;
        for dir in path_var.split(';') {
            let candidate = PathBuf::from(dir).join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Start the opencode server process.
    pub async fn start(&mut self) -> Result<(), AppError> {
        if self.is_running() {
            if self.health_check().await {
                tracing::warn!("opencode sidecar is already running");
                return Ok(());
            }
            tracing::warn!("opencode child is running but unhealthy; restarting sidecar");
            self.stop().await?;
        }

        let binary = match &self.binary_path {
            Some(p) => p.clone(),
            None => {
                return Err(AppError::SidecarError(
                    "No opencode binary found".to_string(),
                ));
            }
        };

        if self.health_check().await {
            return Err(AppError::SidecarError(format!(
                "opencode port {} is already serving before sidecar startup",
                self.port
            )));
        }

        tracing::info!(
            "Starting opencode server on port {} with binary: {}",
            self.port,
            binary.display()
        );

        // On Windows, npm installs a .cmd shim which can't be executed directly
        // by CreateProcessW — must go through cmd.exe. However, if we resolved
        // the actual .exe binary, we can run it directly (and track its PID).
        let use_cmd_wrapper =
            cfg!(target_os = "windows") && binary.extension().map_or(false, |ext| ext == "cmd");

        let mut command = if use_cmd_wrapper {
            let mut command = Command::new("cmd");
            command
                .arg("/c")
                .arg(&binary)
                .arg("serve")
                .arg("--pure")
                .arg("--port")
                .arg(self.port.to_string());
            command
        } else {
            let mut command = Command::new(&binary);
            command
                .arg("serve")
                .arg("--pure")
                .arg("--port")
                .arg(self.port.to_string());
            command
        };
        if let Some(working_dir) = &self.working_dir {
            std::fs::create_dir_all(working_dir).map_err(|e| {
                AppError::SidecarError(format!(
                    "Failed to create opencode working directory ({}): {}",
                    working_dir.display(),
                    e
                ))
            })?;
            command.current_dir(working_dir);
        }
        command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        for (key, value) in &self.extra_env {
            command.env(key, value);
        }
        // Ensure localhost traffic bypasses any system proxy (Clash, V2Ray, etc.)
        command.env("NO_PROXY", "localhost,127.0.0.1");
        self.stderr_buf.lock().await.clear();
        let mut child = command.spawn().map_err(|e| {
            AppError::SidecarError(format!(
                "Failed to spawn opencode process ({}): {}",
                binary.display(),
                e
            ))
        })?;

        // Drain child stdout/stderr to prevent pipe blocking and capture stderr for error reports.
        {
            let pid = child.id();
            tracing::debug!(?pid, "opencode child spawned");

            use tokio::io::{AsyncBufReadExt, BufReader};
            if let Some(stdout) = child.stdout.take() {
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        tracing::debug!(target: "opencode::stdout", "{}", line);
                    }
                    tracing::debug!(target: "opencode::stdout", "EOF");
                });
            }
            if let Some(stderr) = child.stderr.take() {
                let buf = self.stderr_buf.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        tracing::debug!(target: "opencode::stderr", "{}", line);
                        let mut s = buf.lock().await;
                        s.push_str(&line);
                        s.push('\n');
                    }
                    tracing::debug!(target: "opencode::stderr", "EOF");
                });
            }
        }

        self.child = Some(child);
        self.started_at = Some(Instant::now());

        // Wait for this child process to stay alive and serve the configured port.
        if let Err(e) = self.wait_for_healthy().await {
            let _ = self.stop().await;
            return Err(e);
        }

        tracing::info!("opencode server started successfully on port {}", self.port);
        Ok(())
    }

    /// Stop the opencode server process gracefully.
    pub async fn stop(&mut self) -> Result<(), AppError> {
        if let Some(mut child) = self.child.take() {
            tracing::info!("Stopping opencode server...");

            // Try graceful shutdown first (kill_on_drop handles cleanup too)
            let _ = child.kill().await;

            // Wait briefly for process to exit
            match tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, child.wait()).await {
                Ok(Ok(status)) => {
                    tracing::info!("opencode server exited with status: {}", status);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Error waiting for opencode exit: {}", e);
                }
                Err(_) => {
                    tracing::warn!("opencode server did not exit within timeout, force killed");
                }
            }

            self.started_at = None;
        }
        Ok(())
    }

    /// Check if the opencode server is responding to health checks.
    /// Tries `/health` first, falls back to `/` — opencode's actual endpoint
    /// is unverified, so accept any 2xx from either path as "alive".
    pub async fn health_check(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(HEALTH_CHECK_TIMEOUT)
            .no_proxy()
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        for url in self.health_check_urls() {
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    return true;
                }
            }
        }
        false
    }

    /// Restart the opencode server.
    pub async fn restart(&mut self) -> Result<(), AppError> {
        tracing::info!("Restarting opencode server...");
        self.stop().await?;
        self.start().await
    }

    /// Check if the child process is still alive.
    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            // try_wait: None = still running, Some = exited
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => {
                    self.child = None;
                    self.started_at = None;
                    false
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Get the port this sidecar is configured to use.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get uptime in seconds, if running.
    pub fn uptime_secs(&self) -> Option<u64> {
        self.started_at.map(|s| s.elapsed().as_secs())
    }

    /// Construct candidate health check URLs (tried in order).
    pub fn health_check_urls(&self) -> Vec<String> {
        vec![
            format!("http://127.0.0.1:{}/health", self.port),
            format!("http://127.0.0.1:{}/", self.port),
        ]
    }

    /// Wait for the server to become healthy after starting.
    async fn wait_for_healthy(&mut self) -> Result<(), AppError> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            if !self.is_running() {
                let stderr = self.stderr_buf.lock().await.trim().to_string();
                let detail = if stderr.is_empty() {
                    "opencode process exited during startup".to_string()
                } else {
                    format!("opencode process exited during startup: {}", stderr)
                };
                return Err(AppError::SidecarError(detail));
            }
            if Instant::now() > deadline {
                return Err(AppError::SidecarError(format!(
                    "opencode server did not become healthy within {}s",
                    STARTUP_TIMEOUT.as_secs()
                )));
            }
            if self.health_check().await {
                if self.is_running() {
                    return Ok(());
                }
                let stderr = self.stderr_buf.lock().await.trim().to_string();
                let detail = if stderr.is_empty() {
                    "opencode process exited during startup".to_string()
                } else {
                    format!("opencode process exited during startup: {}", stderr)
                };
                return Err(AppError::SidecarError(detail));
            }
            tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
        }
    }
}

/// Start a background watchdog that monitors the sidecar and restarts it on failure.
pub async fn start_watchdog(manager: Arc<Mutex<SidecarManager>>, cancel: CancellationToken) {
    let mut interval = tokio::time::interval(WATCHDOG_INTERVAL);
    let mut consecutive_failures: u32 = 0;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::info!("Sidecar watchdog cancelled");
                break;
            }
            _ = interval.tick() => {
                let mut mgr = manager.lock().await;
                if !mgr.is_running() {
                    tracing::warn!("opencode sidecar process not running, skipping health check");
                    continue;
                }
                drop(mgr); // release lock during network call

                let healthy = {
                    let mgr = manager.lock().await;
                    mgr.health_check().await
                };

                if healthy {
                    consecutive_failures = 0;
                } else {
                    consecutive_failures += 1;
                    tracing::warn!(
                        "opencode health check failed ({}/{})",
                        consecutive_failures,
                        CONSECUTIVE_FAILURES_BEFORE_RESTART
                    );

                    if consecutive_failures >= CONSECUTIVE_FAILURES_BEFORE_RESTART {
                        tracing::error!("opencode unresponsive, attempting restart...");
                        let mut mgr = manager.lock().await;
                        match mgr.restart().await {
                            Ok(()) => {
                                tracing::info!("opencode restarted successfully");
                                consecutive_failures = 0;
                            }
                            Err(e) => {
                                tracing::error!("opencode restart failed: {}", e);
                                // Will retry next watchdog cycle
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_manager_new_defaults() {
        let mgr = SidecarManager::new(None, None);
        assert_eq!(mgr.port, DEFAULT_PORT);
        assert!(mgr.child.is_none());
        assert!(mgr.started_at.is_none());
    }

    #[test]
    fn test_sidecar_manager_custom_port() {
        let mgr = SidecarManager::new(None, Some(5000));
        assert_eq!(mgr.port, 5000);
    }

    #[test]
    fn test_sidecar_manager_stores_working_dir() {
        let mgr = SidecarManager::new(None, Some(5000)).with_working_dir("C:\\egosync-workspace");

        assert_eq!(
            mgr.working_dir.as_deref(),
            Some(std::path::Path::new("C:\\egosync-workspace"))
        );
    }

    #[test]
    fn test_sidecar_manager_isolates_opencode_global_paths_from_working_dir() {
        let working_dir = std::path::Path::new("C:\\egosync-workspace");
        let mgr = SidecarManager::new(None, Some(5000)).with_working_dir(working_dir);

        let global_dir = working_dir
            .parent()
            .map(|parent| parent.join("opencode-global"))
            .unwrap_or_else(|| working_dir.join("opencode-global"));

        assert_eq!(
            mgr.extra_env.get("XDG_CONFIG_HOME").map(String::as_str),
            Some(global_dir.join("config").to_string_lossy().to_string()).as_deref()
        );
        assert_eq!(
            mgr.extra_env.get("XDG_DATA_HOME").map(String::as_str),
            Some(global_dir.join("data").to_string_lossy().to_string()).as_deref()
        );
        assert_eq!(
            mgr.extra_env.get("XDG_CACHE_HOME").map(String::as_str),
            Some(global_dir.join("cache").to_string_lossy().to_string()).as_deref()
        );
    }

    #[test]
    fn test_sidecar_manager_stores_extra_env() {
        let mgr = SidecarManager::new(None, Some(5000))
            .with_env("EGOSYNC_DELEGATE_BRIDGE_TOKEN", "secret")
            .with_env("EGOSYNC_DELEGATE_BRIDGE_PORT", "5010");

        assert_eq!(
            mgr.extra_env
                .get("EGOSYNC_DELEGATE_BRIDGE_TOKEN")
                .map(String::as_str),
            Some("secret")
        );
        assert_eq!(
            mgr.extra_env
                .get("EGOSYNC_DELEGATE_BRIDGE_PORT")
                .map(String::as_str),
            Some("5010")
        );
    }

    #[test]
    fn test_health_check_urls_includes_both_endpoints() {
        // WHY: opencode's actual health endpoint is unverified at spec time;
        // accepting either /health or / lets the sidecar work regardless of
        // which path opencode chose, preventing a 10s startup timeout when
        // the assumed endpoint is wrong.
        let mgr = SidecarManager::new(None, Some(4096));
        let urls = mgr.health_check_urls();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().any(|u| u.ends_with("/health")));
        assert!(urls.iter().any(|u| u.ends_with(":4096/")));
    }

    #[test]
    fn test_health_check_urls_use_configured_port() {
        // WHY: port mismatch in URL construction would cause silent health
        // check failure even when opencode is alive on a custom port.
        let mgr = SidecarManager::new(None, Some(9999));
        let urls = mgr.health_check_urls();
        for url in &urls {
            assert!(
                url.contains(":9999"),
                "url must use configured port: {}",
                url
            );
        }
    }

    #[test]
    fn test_start_invokes_opencode_serve_with_pure_config() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/sidecar.rs"),
        )
        .expect("read sidecar.rs");

        assert!(source.contains(".arg(\"--pure\")"));
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_start_rejects_running_child_without_healthy_server() {
        let child = Command::new("cmd")
            .arg("/c")
            .arg("timeout /t 5 /nobreak >nul")
            .spawn()
            .unwrap();

        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.binary_path = None;
        mgr.child = Some(child);
        mgr.started_at = Some(Instant::now());

        let result = mgr.start().await;
        if let Some(mut child) = mgr.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        assert!(
            result.is_err(),
            "已有 child 但端口不健康时，start 不能只凭进程存活返回成功"
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_start_clears_child_after_startup_failure() {
        let temp = tempfile::tempdir().unwrap();
        let fake_binary = temp.path().join("fake-opencode.cmd");
        std::fs::write(&fake_binary, "@echo off\r\ntimeout /t 5 /nobreak >nul\r\n").unwrap();

        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.binary_path = Some(fake_binary);
        let result = mgr.start().await;

        assert!(result.is_err());
        assert!(
            mgr.child.is_none(),
            "启动失败后必须清理 child，避免下次 start 误报成功"
        );
        assert!(mgr.started_at.is_none());
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_start_clears_previous_stderr_before_spawn() {
        let temp = tempfile::tempdir().unwrap();
        let fake_binary = temp.path().join("fake-opencode.cmd");
        std::fs::write(
            &fake_binary,
            "@echo off\r\necho current failure 1>&2\r\nexit /b 1\r\n",
        )
        .unwrap();

        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.binary_path = Some(fake_binary);
        mgr.stderr_buf.lock().await.push_str("old failure\n");
        let result = mgr.start().await;
        let message = result.unwrap_err().to_string();

        assert!(
            message.contains("current failure"),
            "应返回本次 stderr: {message}"
        );
        assert!(
            !message.contains("old failure"),
            "不应混入上次启动残留 stderr: {message}"
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_start_rejects_preexisting_healthy_listener() {
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                    .await;
            }
        });

        let temp = tempfile::tempdir().unwrap();
        let fake_binary = temp.path().join("fake-opencode.cmd");
        std::fs::write(&fake_binary, "@echo off\r\nexit /b 0\r\n").unwrap();

        let mut mgr = SidecarManager::new(None, Some(port));
        mgr.binary_path = Some(fake_binary);
        let result = mgr.start().await;
        server.abort();

        assert!(
            result.is_err(),
            "start 不能把已存在的旧健康监听服务误判为当前 sidecar 启动成功"
        );
    }

    #[tokio::test]
    async fn test_wait_for_healthy_rejects_listener_not_owned_by_child() {
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                    .await;
            }
        });

        let mut mgr = SidecarManager::new(None, Some(port));
        let result = mgr.wait_for_healthy().await;
        server.abort();

        assert!(
            result.is_err(),
            "健康检查不能把非当前 child 拥有的旧监听服务判定为启动成功"
        );
    }

    #[test]
    fn test_is_running_no_child() {
        let mut mgr = SidecarManager::new(None, None);
        assert!(!mgr.is_running());
    }

    #[test]
    fn test_uptime_none_when_not_started() {
        let mgr = SidecarManager::new(None, None);
        assert!(mgr.uptime_secs().is_none());
    }

    #[test]
    fn test_resolve_binary_path_no_resource_dir() {
        let path = SidecarManager::resolve_binary_path(None);
        assert!(path.is_some());
        // Should resolve to either the actual .exe (if npm shim is installed)
        // or fallback to the .cmd/.exe name for PATH resolution
        let name = path.unwrap();
        let name_str = name.to_string_lossy();
        assert!(
            name_str.contains("opencode"),
            "resolved path must contain 'opencode': {}",
            name_str
        );
    }

    #[test]
    fn test_resolve_binary_path_nonexistent_resource_dir() {
        // Resource dir exists but no binary inside → falls back to PATH/shim resolution
        let path =
            SidecarManager::resolve_binary_path(Some(PathBuf::from("/nonexistent/resources")));
        assert!(path.is_some());
        let name = path.unwrap();
        let name_str = name.to_string_lossy();
        assert!(
            name_str.contains("opencode"),
            "resolved path must contain 'opencode': {}",
            name_str
        );
    }
}
