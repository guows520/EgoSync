use std::path::PathBuf;
use std::sync::Arc;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::error::AppError;

const DEFAULT_PORT: u16 = 4096;
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(500);
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(5);
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const CONSECUTIVE_FAILURES_BEFORE_RESTART: u32 = 2;

/// Manages the opencode sidecar process lifecycle.
pub struct SidecarManager {
    child: Option<Child>,
    port: u16,
    binary_path: Option<PathBuf>,
    started_at: Option<Instant>,
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
            started_at: None,
        }
    }

    /// Resolve the opencode binary path: resource dir first, then system PATH.
    fn resolve_binary_path(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
        let binary_name = if cfg!(target_os = "windows") {
            "opencode.exe"
        } else {
            "opencode"
        };

        // Try resource directory first
        if let Some(dir) = resource_dir {
            let path = dir.join(binary_name);
            if path.exists() {
                tracing::info!("opencode binary found in resources: {}", path.display());
                return Some(path);
            }
        }

        // Fallback to system PATH — just use the command name, let OS resolve
        tracing::info!("opencode binary not in resources, will try system PATH");
        Some(PathBuf::from(binary_name))
    }

    /// Start the opencode server process.
    pub async fn start(&mut self) -> Result<(), AppError> {
        if self.is_running() {
            tracing::warn!("opencode sidecar is already running");
            return Ok(());
        }

        let binary = match &self.binary_path {
            Some(p) => p.clone(),
            None => {
                return Err(AppError::SidecarError(
                    "No opencode binary found".to_string(),
                ));
            }
        };

        tracing::info!(
            "Starting opencode server on port {} with binary: {}",
            self.port,
            binary.display()
        );

        // On Windows, npm installs a .cmd shim which can't be executed directly
        // by CreateProcessW — must go through cmd.exe.
        let child = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/c")
                .arg(&binary)
                .arg("serve")
                .arg("--port")
                .arg(self.port.to_string())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn()
        } else {
            Command::new(&binary)
                .arg("serve")
                .arg("--port")
                .arg(self.port.to_string())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn()
        }
        .map_err(|e| {
                AppError::SidecarError(format!(
                    "Failed to spawn opencode process ({}): {}",
                    binary.display(),
                    e
                ))
            })?;

        self.child = Some(child);
        self.started_at = Some(Instant::now());

        // Wait for health check to pass
        self.wait_for_healthy().await?;

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
    async fn wait_for_healthy(&self) -> Result<(), AppError> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            if Instant::now() > deadline {
                return Err(AppError::SidecarError(format!(
                    "opencode server did not become healthy within {}s",
                    STARTUP_TIMEOUT.as_secs()
                )));
            }
            if self.health_check().await {
                return Ok(());
            }
            tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
        }
    }
}

/// Start a background watchdog that monitors the sidecar and restarts it on failure.
pub async fn start_watchdog(
    manager: Arc<Mutex<SidecarManager>>,
    cancel: CancellationToken,
) {
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
            assert!(url.contains(":9999"), "url must use configured port: {}", url);
        }
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
        let name = path.unwrap();
        if cfg!(target_os = "windows") {
            assert_eq!(name, PathBuf::from("opencode.exe"));
        } else {
            assert_eq!(name, PathBuf::from("opencode"));
        }
    }

    #[test]
    fn test_resolve_binary_path_nonexistent_resource_dir() {
        // Resource dir exists but no binary inside → falls back to PATH name
        let path =
            SidecarManager::resolve_binary_path(Some(PathBuf::from("/nonexistent/resources")));
        assert!(path.is_some());
        // Should fallback to just the binary name
        let name = path.unwrap();
        let expected = if cfg!(target_os = "windows") {
            "opencode.exe"
        } else {
            "opencode"
        };
        assert_eq!(name, PathBuf::from(expected));
    }
}