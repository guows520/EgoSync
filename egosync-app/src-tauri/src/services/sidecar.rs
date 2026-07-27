use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::error::AppError;

/// Windows Job Object FFI — 最小化原生绑定，不引入新 crate 依赖。
///
/// 用途：将 sidecar 子进程加入 Job Object 并设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，
/// 使父进程被外部终止（任务管理器、安装器、崩溃）时 OS 自动结束 sidecar，
/// 防止 sidecar 遗留并锁定安装目录下的 `opencode.exe`。
#[cfg(target_os = "windows")]
mod winapi {
    use std::ffi::c_void;

    type BOOL = i32;
    type DWORD = u32;
    type HANDLE = *mut c_void;
    type ULONG_PTR = usize;

    /// 关闭 Job Object 最后一个句柄时终止其中所有进程
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: ULONG_PTR = 0x2000;
    /// JobObjectExtendedLimitInformation 信息类
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: DWORD = 9;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct IoCounters {
        ReadOperationCount: u64,
        WriteOperationCount: u64,
        OtherOperationCount: u64,
        ReadTransferCount: u64,
        WriteTransferCount: u64,
        OtherTransferCount: u64,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct JobObjectBasicLimitInformation {
        PerProcessUserTimeLimit: i64,
        PerJobUserTimeLimit: i64,
        LimitFlags: DWORD,
        MinimumWorkingSetSize: usize,
        MaximumWorkingSetSize: usize,
        ActiveProcessLimit: DWORD,
        Affinity: ULONG_PTR,
        PriorityClass: DWORD,
        SchedulingClass: DWORD,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct JobObjectExtendedLimitInformation {
        BasicLimitInformation: JobObjectBasicLimitInformation,
        IoInfo: IoCounters,
        ProcessMemoryLimit: usize,
        JobMemoryLimit: usize,
        PeakProcessMemoryUsed: usize,
        PeakJobMemoryUsed: usize,
    }

    extern "system" {
        fn CreateJobObjectW(lpJobAttributes: *mut c_void, lpName: *const u16) -> HANDLE;
        fn SetInformationJobObject(
            hJob: HANDLE,
            InfoClass: DWORD,
            lpJobObjectInfo: *mut c_void,
            cbJobObjectInfoLength: DWORD,
        ) -> BOOL;
        fn AssignProcessToJobObject(hJob: HANDLE, hProcess: HANDLE) -> BOOL;
        fn CloseHandle(hObject: HANDLE) -> BOOL;
        fn OpenProcess(dwDesiredAccess: DWORD, bInheritHandle: BOOL, dwProcessId: DWORD) -> HANDLE;
    }

    const PROCESS_SET_QUOTA: DWORD = 0x0100;
    const PROCESS_TERMINATE: DWORD = 0x0001;

    /// Job Object 句柄的 RAII 包装，drop 时自动 CloseHandle。
    pub struct JobHandle(HANDLE);

    // SAFETY: Windows HANDLE 是不透明指针，可跨线程传递和共享。
    // 所有 WinAPI 调用对同一 handle 的操作是线程安全的。
    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    impl JobHandle {
        fn new(handle: HANDLE) -> Option<Self> {
            if handle.is_null() {
                None
            } else {
                Some(Self(handle))
            }
        }

        pub fn as_raw(&self) -> HANDLE {
            self.0
        }
    }

    impl Drop for JobHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    /// 创建 Job Object 并设置 kill-on-close 标志。
    /// 返回的 JobHandle 在 drop 时自动关闭句柄，触发 Job 内所有进程终止。
    pub fn create_job_object_with_kill_on_close() -> Option<JobHandle> {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
            if job.is_null() {
                tracing::error!(
                    error = %std::io::Error::last_os_error(),
                    "CreateJobObjectW returned null"
                );
                return None;
            }

            let mut info = JobObjectExtendedLimitInformation {
                BasicLimitInformation: JobObjectBasicLimitInformation {
                    PerProcessUserTimeLimit: 0,
                    PerJobUserTimeLimit: 0,
                    LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE as DWORD,
                    MinimumWorkingSetSize: 0,
                    MaximumWorkingSetSize: 0,
                    ActiveProcessLimit: 0,
                    Affinity: 0,
                    PriorityClass: 0,
                    SchedulingClass: 0,
                },
                IoInfo: IoCounters {
                    ReadOperationCount: 0,
                    WriteOperationCount: 0,
                    OtherOperationCount: 0,
                    ReadTransferCount: 0,
                    WriteTransferCount: 0,
                    OtherTransferCount: 0,
                },
                ProcessMemoryLimit: 0,
                JobMemoryLimit: 0,
                PeakProcessMemoryUsed: 0,
                PeakJobMemoryUsed: 0,
            };

            let ok = SetInformationJobObject(
                job,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                &mut info as *mut _ as *mut c_void,
                std::mem::size_of::<JobObjectExtendedLimitInformation>() as DWORD,
            );

            if ok == 0 {
                tracing::error!(
                    error = %std::io::Error::last_os_error(),
                    "SetInformationJobObject failed"
                );
                CloseHandle(job);
                return None;
            }

            JobHandle::new(job)
        }
    }

    /// 将进程分配到 Job Object。
    /// `pid` 是要分配的进程 ID。
    pub fn assign_process_to_job(job: &JobHandle, pid: u32) -> Result<(), String> {
        unsafe {
            let process = OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE,
                0, // bInheritHandle = FALSE
                pid,
            );
            if process.is_null() {
                return Err(format!(
                    "OpenProcess(pid={}) failed (error: {})",
                    pid,
                    std::io::Error::last_os_error()
                ));
            }

            let ok = AssignProcessToJobObject(job.as_raw(), process);
            CloseHandle(process);

            if ok == 0 {
                return Err(format!(
                    "AssignProcessToJobObject(pid={}) failed (error: {})",
                    pid,
                    std::io::Error::last_os_error()
                ));
            }
            Ok(())
        }
    }

    /// 关闭句柄（用于测试）
    pub fn close_handle(job: JobHandle) {
        drop(job);
    }
}

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
    /// Windows Job Object — drop 时自动终止 Job 内所有进程，
    /// 防止父进程被外部终止时 sidecar 遗留。
    #[cfg(target_os = "windows")]
    job: Option<winapi::JobHandle>,
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
            #[cfg(target_os = "windows")]
            job: None,
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

    pub fn update_env(&mut self, key: impl Into<String>, value: impl Into<String>) -> bool {
        let key = key.into(); let value = value.into();
        if self.extra_env.get(&key) == Some(&value) { return false; }
        self.extra_env.insert(key, value); true
    }

    /// Resolve the opencode binary path: resource dir first, then system PATH.
    fn resolve_binary_path(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
        // Try resource directory first.
        // Tauri 2.x: `tauri.conf.json` 的 `bundle.resources` 配置将文件放到
        // `resource_dir/resources/` 子目录，而 `resource_dir()` 返回安装根目录。
        // 因此需要同时搜索 `resources/` 子目录和根目录。
        if let Some(dir) = resource_dir {
            let resources_subdir = dir.join("resources");
            let search_dirs = [resources_subdir.as_path(), dir.as_path()];
            let exe_name = if cfg!(target_os = "windows") {
                "opencode.exe"
            } else {
                "opencode"
            };
            let cmd_name = "opencode.cmd";
            for search_dir in search_dirs {
                let exe_path = search_dir.join(exe_name);
                if exe_path.exists() {
                    tracing::info!("opencode binary found: {}", exe_path.display());
                    return Some(exe_path);
                }
                let cmd_path = search_dir.join(cmd_name);
                if cmd_path.exists() {
                    tracing::info!("opencode cmd found: {}", cmd_path.display());
                    return Some(cmd_path);
                }
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
            tracing::warn!(
                "opencode port {} is already in use before sidecar startup; killing stale process",
                self.port
            );
            kill_process_on_port(self.port, &binary).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            if self.health_check().await {
                return Err(AppError::SidecarError(format!(
                    "opencode port {} is still serving after killing stale process",
                    self.port
                )));
            }
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
        // If NO_PROXY was set via extra_env (from LLM config), use that; otherwise default.
        if !self.extra_env.contains_key("NO_PROXY") {
            command.env("NO_PROXY", "localhost,127.0.0.1");
        }
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

        // Windows: Job Object 是外部终止时的必要兜底；配置失败必须阻止启动，
        // 不能继续运行一个无法在父进程消失后自动回收的 sidecar。
        #[cfg(target_os = "windows")]
        {
            let pid = match child.id() {
                Some(pid) => pid,
                None => {
                    let kill_result = child.kill().await;
                    let wait_result = child.wait().await;
                    return Err(AppError::SidecarError(format!(
                        "无法取得 opencode sidecar PID; cleanup kill={kill_result:?}, wait={wait_result:?}"
                    )));
                }
            };
            let job = match winapi::create_job_object_with_kill_on_close() {
                Some(job) => job,
                None => {
                    let kill_result = child.kill().await;
                    let wait_result = child.wait().await;
                    return Err(AppError::SidecarError(format!(
                        "创建 Windows Job Object 失败，PID={pid}; cleanup kill={kill_result:?}, wait={wait_result:?}"
                    )));
                }
            };
            if let Err(e) = winapi::assign_process_to_job(&job, pid) {
                let kill_result = child.kill().await;
                let _ = child.wait().await;
                return Err(AppError::SidecarError(format!(
                    "将 sidecar PID={pid} 加入 Windows Job Object 失败: {e}; cleanup={kill_result:?}"
                )));
            }
            tracing::info!(
                ?pid,
                "sidecar assigned to Windows Job Object (kill-on-close)"
            );
            self.job = Some(job);
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

    /// Stop the opencode server process.
    ///
    /// 发送 kill 信号后等待进程退出，超时后用 `try_wait` 确认最终状态。
    /// kill 错误不静默丢弃 — 记录上下文日志（PID、阶段、错误信息）。
    /// 结果按现有 `Result<(), AppError>` 约定返回。
    pub async fn stop(&mut self) -> Result<(), AppError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };

        let pid = child.id();
        tracing::info!(?pid, "Stopping opencode server...");
        let mut failure: Option<String> = None;

        if let Err(e) = child.kill().await {
            tracing::warn!(?pid, error = %e, "child.kill() failed");
            failure = Some(format!("kill failed: {e}"));
        }

        let exited = match tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, child.wait()).await {
            Ok(Ok(status)) => {
                tracing::info!(?pid, exit_status = %status, "opencode server exited");
                true
            }
            Ok(Err(e)) => {
                tracing::warn!(?pid, error = %e, "child.wait() failed");
                failure = Some(format!("wait failed: {e}"));
                false
            }
            Err(_) => {
                let message = format!(
                    "stop timed out after {}s",
                    GRACEFUL_SHUTDOWN_TIMEOUT.as_secs()
                );
                tracing::warn!(?pid, error = %message, "opencode server stop timed out");
                failure = Some(message);
                false
            }
        };

        #[cfg(target_os = "windows")]
        if !exited {
            if self.job.take().is_some() {
                tracing::warn!(?pid, "closing Job Object for final sidecar termination");
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    tracing::info!(?pid, exit_status = %status, "sidecar exited before final termination")
                }
                Ok(None) => tracing::warn!(?pid, "sidecar still alive before final termination"),
                Err(e) => {
                    tracing::warn!(?pid, error = %e, "cannot confirm sidecar state before final termination")
                }
            }
            if let Err(e) = child.kill().await {
                tracing::warn!(?pid, error = %e, "final child.kill() failed");
            }
            match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
                Ok(Ok(status)) => {
                    tracing::info!(?pid, exit_status = %status, "sidecar exited after final termination");
                    failure = None;
                }
                Ok(Err(e)) => {
                    failure = Some(format!("final wait failed: {e}"));
                }
                Err(_) => {
                    failure =
                        Some("sidecar remained alive after final termination timeout".to_string());
                }
            }
        }

        self.started_at = None;
        if exited || failure.is_none() {
            Ok(())
        } else {
            Err(AppError::SidecarError(format!(
                "failed to stop opencode PID={pid:?}: {}",
                failure.unwrap()
            )))
        }
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

    /// Get the child process PID, if running.
    /// Used by `app_performance_snapshot` to read sidecar RSS via sysinfo.
    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|c| c.id())
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

/// Kill any process listening on the given port (Windows: via netstat + taskkill).
///
/// 仅终止映像路径属于 EgoSync 安装目录的进程，避免误杀用户手动运行的无关
/// opencode 服务（例如用户在另一个目录手动启动的 `opencode serve`）。
async fn kill_process_on_port(port: u16, expected_binary: &Path) {
    let output = tokio::process::Command::new("cmd")
        .args([
            "/c",
            &format!("netstat -ano | findstr :{port} | findstr LISTENING"),
        ])
        .output()
        .await;
    let Ok(output) = output else {
        tracing::warn!("kill_process_on_port: netstat failed");
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            if let Ok(pid) = parts[4].parse::<u32>() {
                if !is_ego_sync_owned_process(pid, expected_binary) {
                    tracing::warn!(
                        pid,
                        port,
                        "port occupied by non-EgoSync process, skipping kill to avoid误杀"
                    );
                    continue;
                }
                tracing::info!(pid, port, "killing EgoSync-owned stale process on port");
                let _ = tokio::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output()
                    .await;
            }
        }
    }
}

/// 判断进程是否属于 EgoSync（映像路径包含 opencode 且由 EgoSync 管理）。
///
/// 用 sysinfo 读取进程映像路径，仅当路径包含 "opencode" 时认为是 EgoSync 管理的
/// sidecar。用户手动运行的其他同名进程（如独立的 opencode 服务）路径不同，
/// 不会被误杀。
fn is_ego_sync_owned_process(pid: u32, expected_binary: &Path) -> bool {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
    );
    let Some(proc) = sys.process(sysinfo::Pid::from_u32(pid)) else {
        tracing::warn!(pid, "cannot find process for ownership check");
        return false;
    };
    let Some(exe) = proc.exe() else {
        tracing::warn!(pid, "process has no executable path, skipping");
        return false;
    };
    let actual = std::fs::canonicalize(exe);
    let expected = std::fs::canonicalize(expected_binary);
    let owned = match (actual, expected) {
        (Ok(actual), Ok(expected)) => actual
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.to_string_lossy()),
        _ => false,
    };
    if !owned {
        tracing::debug!(pid, actual = ?exe, expected = ?expected_binary, "process executable does not match managed sidecar");
    }
    owned
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
    fn test_update_env_reports_only_real_changes() {
        let mut manager = SidecarManager::new(None, Some(4096)).with_env("NO_PROXY", "localhost");
        assert!(!manager.update_env("NO_PROXY", "localhost"));
        assert!(manager.update_env("NO_PROXY", "localhost,internal.test"));
        assert!(!manager.update_env("NO_PROXY", "localhost,internal.test"));
    }

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

    /// WHY: 如果安装器不在覆盖 resources/opencode.exe 前停止旧 sidecar，
    /// Windows 会因文件被运行中进程锁定而拒绝写入，导致升级失败。
    /// 此测试验证 NSIS 安装前清理边界确实存在，而不是仅靠应用启动后端口清理。
    #[test]
    fn test_nsis_preinstall_hook_configured_and_path_based() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let conf_path = manifest_dir.join("tauri.conf.json");
        let conf = std::fs::read_to_string(&conf_path)
            .unwrap_or_else(|_| panic!("read tauri.conf.json: {}", conf_path.display()));

        let conf_value: serde_json::Value =
            serde_json::from_str(&conf).expect("tauri.conf.json must be valid JSON");

        let hooks_path = conf_value
            .pointer("/bundle/windows/nsis/installerHooks")
            .and_then(|v| v.as_str())
            .expect(
                "tauri.conf.json 必须配置 bundle.windows.nsis.installerHooks，\
                 否则安装器不会在覆盖文件前清理旧 sidecar",
            );

        assert!(
            hooks_path.to_lowercase().ends_with(".nsh"),
            "installerHooks 必须指向 .nsh 文件: {}",
            hooks_path
        );

        let hook_file = manifest_dir.join(hooks_path);
        assert!(
            hook_file.exists(),
            "NSIS installer hook 文件必须存在: {}",
            hook_file.display()
        );

        let hook_src = std::fs::read_to_string(&hook_file)
            .unwrap_or_else(|_| panic!("read hook file: {}", hook_file.display()));

        assert!(
            hook_src.contains("NSIS_HOOK_PREINSTALL"),
            "hook 文件必须定义 NSIS_HOOK_PREINSTALL 宏，\
             该宏在 NSIS 复制文件前执行，是安装前清理的入口"
        );

        // 路径匹配而非端口匹配：仅按端口杀进程会误杀用户手动运行的无关 opencode 服务
        assert!(
            hook_src.to_lowercase().contains("executablepath")
                || hook_src.to_lowercase().contains("processpath")
                || hook_src.contains("Win32_Process"),
            "hook 必须按完整映像路径识别 EgoSync 进程，\
             不能仅按进程名或端口杀进程（会误杀同名无关进程）"
        );
    }

    /// WHY: kill 与 wait 存在竞态时，stop() 仍必须以最终退出状态为准，
    /// 不能因 kill 返回“已退出”类错误而误报失败或静默继续。
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_stop_handles_kill_wait_race() {
        let mut child = Command::new("cmd")
            .arg("/c")
            .arg("exit 0")
            .spawn()
            .expect("spawn exited child");
        let _ = child.wait().await;
        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.child = Some(child);
        assert!(
            mgr.stop().await.is_ok(),
            "已退出 child 的 stop 应以最终状态成功"
        );
        assert!(mgr.child.is_none());
    }

    /// WHY: 超时/强制终止路径必须真正释放子进程，防止安装升级时文件仍被锁定。
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_stop_confirms_final_process_exit() {
        let child = Command::new("cmd")
            .arg("/c")
            .arg("timeout /t 30 /nobreak >nul")
            .spawn()
            .expect("spawn running child");
        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.child = Some(child);
        let result = mgr.stop().await;
        assert!(result.is_ok(), "强制终止后应确认退出并返回 Ok: {result:?}");
        assert!(mgr.child.is_none());
    }

    /// WHY: stop() 日志缺少 PID 时，安装升级失败无法定位是哪个进程拒绝退出。
    /// PID 必须出现在停止流程的结构化日志中。
    #[test]
    fn test_stop_includes_pid_in_logs() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/sidecar.rs"),
        )
        .expect("read sidecar.rs");

        let stop_section = source
            .split("pub async fn stop")
            .nth(1)
            .and_then(|s| s.split("pub async fn").next())
            .expect("找不到 stop() 函数体");

        // tracing 宏中必须包含 pid 字段（结构化日志）
        assert!(
            stop_section.contains("pid") || stop_section.contains("child.id()"),
            "stop() 必须在日志中包含 PID，以便安装升级失败时定位目标进程"
        );
    }

    /// WHY: 超时后仅记录 "force killed" 但不确认进程实际状态，会导致调用方误以为
    /// 进程已退出而继续覆盖文件，但文件仍被锁定。必须用 try_wait 确认最终状态。
    #[test]
    fn test_stop_confirms_state_after_timeout() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/sidecar.rs"),
        )
        .expect("read sidecar.rs");

        let stop_section = source
            .split("pub async fn stop")
            .nth(1)
            .and_then(|s| s.split("pub async fn").next())
            .expect("找不到 stop() 函数体");

        assert!(
            stop_section.contains("try_wait"),
            "stop() 超时后必须用 try_wait 确认进程最终状态，\
             不能仅记录 force killed 而无实际确认"
        );
    }

    /// WHY: stop() 必须真正终止子进程，否则安装升级时文件锁不会释放。
    /// 此测试验证 stop() 后子进程确实退出。
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_stop_terminates_running_child() {
        let child = Command::new("cmd")
            .arg("/c")
            .arg("timeout /t 30 /nobreak >nul")
            .spawn()
            .unwrap();

        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.child = Some(child);
        mgr.started_at = Some(Instant::now());

        let result = mgr.stop().await;
        assert!(result.is_ok(), "stop() 应返回 Ok");
        assert!(mgr.child.is_none(), "stop() 后 child 应被 take 出来");
        assert!(mgr.started_at.is_none(), "stop() 后 started_at 应清零");
    }

    /// WHY: stop() 在子进程已退出时不应报错，否则正常关闭流程会出现假错误。
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_stop_handles_already_exited_child() {
        let mut child = Command::new("cmd").arg("/c").arg("exit 0").spawn().unwrap();
        // 等待子进程自行退出
        let _ = child.wait().await;

        let mut mgr = SidecarManager::new(None, Some(9));
        mgr.child = Some(child);
        mgr.started_at = Some(Instant::now());

        let result = mgr.stop().await;
        assert!(result.is_ok(), "stop() 对已退出子进程应返回 Ok");
    }

    /// WHY: Windows Job Object 确保父进程被外部终止时 sidecar 不会遗留。
    /// 如果 Job Object FFI 未定义，则生命周期兜底机制不存在。
    #[cfg(target_os = "windows")]
    #[test]
    fn test_job_object_ffi_defined() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/sidecar.rs"),
        )
        .expect("read sidecar.rs");

        assert!(
            source.contains("CreateJobObjectW") || source.contains("CreateJobObject"),
            "Windows Job Object FFI 必须定义 CreateJobObjectW，\
             否则父进程被外部终止时 sidecar 会遗留"
        );
        assert!(
            source.contains("AssignProcessToJobObject"),
            "必须定义 AssignProcessToJobObject 将 sidecar 子进程加入 Job Object"
        );
        assert!(
            source.contains("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE"),
            "必须设置 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE 标志，\
             否则 Job Object 不会在父进程退出时自动终止子进程"
        );
    }

    /// WHY: Job Object 必须能成功创建并设置 kill-on-close 标志，
    /// 否则 sidecar 进程树生命周期兜底机制无法生效。
    #[cfg(target_os = "windows")]
    #[test]
    fn test_job_object_creation_and_assignment() {
        // 创建 Job Object
        let job = super::winapi::create_job_object_with_kill_on_close()
            .expect("创建 Job Object 并设置 kill-on-close 应成功");

        // 启动一个短生命周期的子进程并分配到 Job Object
        let mut child = std::process::Command::new("cmd")
            .arg("/c")
            .arg("exit 0")
            .spawn()
            .expect("spawn test child");

        let pid = child.id();
        let assigned = super::winapi::assign_process_to_job(&job, pid);
        assert!(
            assigned.is_ok(),
            "分配子进程到 Job Object 应成功: {:?}",
            assigned.err()
        );

        // 等待子进程退出
        let _ = child.wait();

        // 关闭 Job Object handle
        super::winapi::close_handle(job);
    }

    /// WHY: kill_process_on_port 仅按端口杀进程会误杀用户手动运行的无关 opencode 服务
    /// （例如用户在另一个目录手动启动的 opencode serve）。必须校验占用端口的进程
    /// 映像路径是否属于 EgoSync 安装目录，否则不终止。
    #[test]
    fn test_kill_process_on_port_validates_image_path() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/sidecar.rs"),
        )
        .expect("read sidecar.rs");

        // 隔离 kill_process_on_port 函数体：从函数签名到下一个顶层项 start_watchdog
        let after_fn = source
            .split("async fn kill_process_on_port")
            .nth(1)
            .expect("找不到 kill_process_on_port 函数");
        let func_section = after_fn
            .split("pub async fn start_watchdog")
            .next()
            .expect("找不到 kill_process_on_port 函数结束边界");

        // 必须在 taskkill 前校验映像路径归属
        assert!(
            func_section.contains("exe()")
                || func_section.contains("ExecutablePath")
                || func_section.contains("image_path")
                || func_section.contains("is_ego_sync_owned"),
            "kill_process_on_port 必须在终止前校验进程映像路径归属，\
             否则会误杀用户手动运行的无关 opencode 服务"
        );
    }
}
