//! server 引导（Story 15.4）：对等复刻桌面壳 setup 序列
//! （`egosync-app/src-tauri/src/lib.rs:42-416` 蓝本，剔除桌面专属）。
//!
//! 复刻序列：init_db 双池（`EGOSYNC_DATA_DIR`）→ 四接缝注入
//! （SseEventBus / 服务端 SecretStore / sidecar 注入 / Handle::current()）
//! → ChatSessionRegistry → AgentConfig 全量同步 → LLM provider 同步 →
//! custom tools 写盘 → sidecar 启动+watchdog（PATH 无 opencode 时优雅
//! 降级——桌面同款路径）→ DelegateBridge 监听 → EventRouter pump →
//! 两个 hourly watch → spawn_scheduler → EngineCtx 装配。
//!
//! Story 17.1 两个部署前置注入：
//! - `EGOSYNC_BEHIND_PROXY`（main.rs 读取，参数下发）：反代感知门控，
//!   security/auth 消费（XFP 感知同源 + Cookie Secure）；
//! - `EGOSYNC_OPENCODE_PATH`：opencode 二进制注入（架构 ⑤「同一注入点
//!   两个值」——桌面传 tauri resource_dir，云端传本 env 解析值；缺省
//!   保持 PATH fallback，见 [`opencode_resource_dir`]）。
//!
//! 剔除：companion_*（手机伴侣桌面宿主专属）、窗口/keyring、legacy
//! app-root opencode.json 清理（桌面历史数据关切，server 数据目录全新）。
//! 退出 = 取消 token + sidecar stop（main.rs 收口）。
//!
//! 测试轻量通道：[`build_test_state`] 只装配双池 + EngineCtx + 认证态 +
//! 事件扇出（无 sidecar/delegate/scheduler 后台任务）——I/O 矩阵集成测试
//! 不拉起完整运行时；[`build_test_state_with_proxy`] 为 17.1 反代感知
//! 测试通道（behind_proxy=true）。

use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use egosync_engine::db::pool::{init_conversations_db, init_db};
use egosync_engine::registry::ChatSessionRegistry;
use egosync_engine::services::agent_config::AgentConfigService;
use egosync_engine::services::agent_bridge::AgentBridge;
use egosync_engine::services::delegate_bridge::DelegateBridge;
use egosync_engine::services::event_router::EventRouter;
use egosync_engine::services::sidecar::SidecarManager;
use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;

use crate::auth::AuthState;
use crate::secret_store::ServerSecretStore;
use crate::sse::{SseEvent, SseEventBus, SseTicketStore, SSE_TICKET_TTL};
use crate::AppState;

/// `EGOSYNC_OPENCODE_PATH` → SidecarManager 注入目录（架构 ⑤「同一注入
/// 点两个值」：桌面传 tauri `resource_dir()`，云端传本 env 解析值）。
///
/// - **文件路径** → 取父目录（引擎 `resolve_binary_path` 复用既有
///   `resources/` 子目录 → 根搜索序，资源相对布局两端一致）；
/// - **目录** → 直传；
/// - **未设 / 空 / 裸文件名**（无目录分量）→ `None`（缺省 PATH fallback，
///   行为与 15.4 一致——opencode 不可用时降级运行不崩）。
///
/// 不校验存在性：路径错了引擎搜索序自然落空回落 PATH（warn 日志），
/// 不构成启动失败。
fn opencode_resource_dir(env_value: Option<&str>) -> Option<std::path::PathBuf> {
    let raw = env_value?.trim();
    if raw.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(raw);
    if path.is_dir() {
        return Some(path);
    }
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(std::path::Path::to_path_buf)
}

/// `EGOSYNC_BEHIND_PROXY` env 值 → 反代感知开关（Story 17.1；评审 #27
/// 从 main.rs 抽取为纯函数——解析回归曾可静默关闭 TLS 修复全链且全部
/// 测试绿，现由单测钉死取值映射）。
///
/// `1` / `true`（trim + 大小写不敏感）启用；其余取值按直连语义运行
///（拼错不拒启，与 RUST_LOG 等宽松 env 同款口径）——非空垃圾值与
/// 非 UTF-8 字节告警可观测（评审 #21），空串/未设置静默关闭。
pub fn parse_behind_proxy(raw: Result<String, std::env::VarError>) -> bool {
    match raw {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            let enabled = v == "1" || v == "true";
            if enabled {
                tracing::info!(
                    "反代感知启用（EGOSYNC_BEHIND_PROXY）：同源判定读取 X-Forwarded-Proto，TLS 面 Cookie 加 Secure"
                );
            } else if !v.is_empty() {
                tracing::warn!(
                    value = %v,
                    "EGOSYNC_BEHIND_PROXY 仅识别 1/true——按直连语义运行（X-Forwarded-* 被忽略）"
                );
            }
            enabled
        }
        Err(std::env::VarError::NotUnicode(bytes)) => {
            tracing::warn!(
                bytes = ?bytes,
                "EGOSYNC_BEHIND_PROXY 含非法 UTF-8 字节——按直连语义运行（X-Forwarded-* 被忽略）"
            );
            false
        }
        Err(std::env::VarError::NotPresent) => false,
    }
}

/// 生产引导：完整桌面序列对等复刻（main.rs 与进程内复用）。
pub async fn build_app_state(
    data_dir: std::path::PathBuf,
    env_token: Option<String>,
    behind_proxy: bool,
) -> Result<Arc<AppState>, String> {
    let _ = std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("创建数据目录失败: {}", e))?;
    let pool = init_db(&data_dir.join("egosync.db"))
        .await
        .map_err(|e| format!("数据库初始化失败: {}", e))?;
    let conv_pool = init_conversations_db(&data_dir.join("conversations.db"))
        .await
        .map_err(|e| format!("对话数据库初始化失败: {}", e))?;

    // ── 认证态先行（二轮评审修复 #3）──
    // AuthState 只依赖 pool + env_token；装配失败（库哈希读错误 ⇒ 拒启，
    // 一轮修复 #2）时排在任何子资源启动**之前**——sidecar 子进程与
    // 后台任务（下方 spawn 族）零泄漏。
    let auth = AuthState::new(pool.clone(), env_token)
        .await
        .map_err(|e| format!("认证态装配失败: {}", e))?;

    // ── 接缝注入：事件总线（SSE 广播）+ 密钥（secrets.json + env 兜底）──
    let bus = Arc::new(SseEventBus::new());
    let events_tx: broadcast::Sender<SseEvent> = bus.sender().clone();
    let secrets: Arc<ServerSecretStore> = Arc::new(ServerSecretStore::new(data_dir.clone()));

    let registry = Arc::new(ChatSessionRegistry::default());
    let opencode_workspace_dir = data_dir.join("opencode-workspace");

    // ── AgentConfig: 全量同步 roles/butler skills/skill registry/MCP → opencode.json ──
    let opencode_config_path = opencode_workspace_dir.join("opencode.json");
    let agent_config = AgentConfigService::new(opencode_config_path);
    {
        let managed_skills_root = opencode_workspace_dir.join(".opencode").join("skills");
        let managed_skills_ready =
            egosync_engine::services::skill_registry::migrate_legacy_managed_paths(
                &pool,
                &managed_skills_root,
            )
            .await
            .is_ok();
        let all_roles = egosync_engine::db::roles::list_all_roles(&pool).await;
        let butler_skills = egosync_engine::services::butler_config::get_butler_skills(&pool)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to load butler Skill config: {}", e);
                egosync_engine::services::butler_config::default_butler_skills()
            });
        let skill_registry = if managed_skills_ready {
            // 失败降级留 warn（对齐桌面 lib.rs:135 诊断口径——静默吞错
            // 会让 opencode 同步缺 Skill 注册而无从排查）
            egosync_engine::db::skills::list_skills(&pool)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load Skill registry for opencode sync: {}", e);
                    Vec::new()
                })
        } else {
            Vec::new()
        };
        let role_mcp_prompts =
            egosync_engine::services::mcp_server::role_mcp_prompt_map(&pool).await;
        let butler_mcp_lines = egosync_engine::db::mcp_servers::butler_enabled_mcp_lines(&pool).await;
        egosync_engine::services::mcp_server::sync_enabled_mcp_to_opencode(&pool, &agent_config)
            .await;
        match (all_roles, role_mcp_prompts, butler_mcp_lines) {
            (Ok(roles), Ok(role_mcp_prompts), Ok(butler_mcp_lines)) => {
                if let Err(e) = agent_config.full_sync_with_skills_and_mcp(
                    &roles,
                    &butler_skills,
                    &skill_registry,
                    &role_mcp_prompts,
                    &butler_mcp_lines,
                ) {
                    tracing::warn!("opencode.json full sync failed (degraded): {}", e);
                }
            }
            (Err(e), _, _) => tracing::warn!("Failed to load roles for opencode sync: {}", e),
            (_, Err(e), _) => {
                tracing::warn!("Failed to load role MCP bindings for opencode sync: {}", e)
            }
            (_, _, Err(e)) => {
                tracing::warn!("Failed to load butler MCP bindings for opencode sync: {}", e)
            }
        }
    }

    // ── LLM provider sync：默认 provider/model/apiKey 落 opencode.json ──
    if let Err(error) = egosync_engine::services::llm_config::sync_default_to_opencode(
        &pool,
        secrets.as_ref(),
        &agent_config,
    )
    .await
    {
        tracing::warn!("同步默认 LLM 配置到 opencode.json 失败: {}", error);
    }

    // ── Custom tools：.opencode/tools/ 写盘（sidecar 启动前）──
    if let Err(e) =
        egosync_engine::services::agent_config::write_custom_tools(&opencode_workspace_dir)
    {
        tracing::warn!("Failed to write custom tools (tools degraded): {}", e);
    }

    // ── Sidecar：opencode 注入（17.1 EGOSYNC_OPENCODE_PATH；缺省 PATH
    // fallback）+ 优雅降级 ──
    let opencode_dir = opencode_resource_dir(std::env::var("EGOSYNC_OPENCODE_PATH").ok().as_deref());
    match &opencode_dir {
        Some(dir) => tracing::info!(
            dir = %dir.display(),
            "opencode 经 EGOSYNC_OPENCODE_PATH 注入（资源目录搜索序）"
        ),
        None => tracing::info!(
            "EGOSYNC_OPENCODE_PATH 未设置——opencode 按系统 PATH 解析（缺省 fallback）"
        ),
    }
    let cancel = CancellationToken::new();
    let bridge_token = egosync_engine::services::delegate_bridge::generate_bridge_token();
    let (delegate_listener, delegate_port) =
        egosync_engine::services::delegate_bridge::bind_random_listener()
            .await
            .map_err(|e| format!("委派桥接服务初始化失败: {}", e))?;
    let original_no_proxy = egosync_engine::services::llm_config::process_no_proxy_value();
    let no_proxy_value = egosync_engine::services::llm_config::generate_no_proxy_value(
        &pool,
        original_no_proxy.as_deref(),
    )
    .await
    .map_err(|e| format!("生成 sidecar NO_PROXY 失败: {}", e))?;
    let mut sidecar = SidecarManager::new(opencode_dir, None)
        .with_working_dir(opencode_workspace_dir.clone())
        .with_env(
            egosync_engine::services::delegate_bridge::BRIDGE_TOKEN_ENV,
            bridge_token.clone(),
        )
        .with_env(
            egosync_engine::services::delegate_bridge::BRIDGE_PORT_ENV,
            delegate_port.to_string(),
        )
        .with_env("NO_PROXY", no_proxy_value);
    let sidecar_started = match sidecar.start().await {
        Ok(()) => {
            tracing::info!("opencode sidecar started on port {}", sidecar.port());
            true
        }
        Err(e) => {
            tracing::warn!("opencode sidecar failed to start (degraded mode): {}", e);
            false
        }
    };
    if sidecar_started {
        agent_config.mark_runtime_loaded();
    }
    let sidecar_port = sidecar.port();
    let opencode_available = if sidecar_started {
        true
    } else {
        let healthy = sidecar.health_check().await;
        if healthy {
            tracing::info!(
                "opencode server already available on port {}; event router will attach",
                sidecar_port
            );
        }
        healthy
    };
    let sidecar = Arc::new(Mutex::new(sidecar));

    // ── AgentBridge + EventRouter pump ──
    let agent_bridge = AgentBridge::new(sidecar_port);
    let event_router = Arc::new(EventRouter::new());
    if opencode_available {
        let router_clone = event_router.clone();
        let bridge_clone = agent_bridge.clone();
        tokio::spawn(async move {
            router_clone.run_pump(bridge_clone).await;
        });
    }

    // ── DelegateBridge 监听 ──
    let delegate_bridge = DelegateBridge::new(
        pool.clone(),
        conv_pool.clone(),
        bridge_token,
        Some(bus.clone()),
        agent_config.clone(),
        opencode_workspace_dir.join(".opencode").join("skills"),
        secrets.clone(),
    );
    {
        let bridge = delegate_bridge.clone();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            if let Err(e) =
                egosync_engine::services::delegate_bridge::start_server_on_listener(
                    delegate_listener,
                    bridge,
                    cancel_clone,
                )
                .await
            {
                tracing::warn!("delegate bridge failed (delegate tool degraded): {}", e);
            }
        });
    }
    if sidecar_started {
        let cancel_clone = cancel.clone();
        let mgr_clone = sidecar.clone();
        tokio::spawn(async move {
            egosync_engine::services::sidecar::start_watchdog(mgr_clone, cancel_clone).await;
        });
    }

    // ── 两个 hourly watch + 角色后台调度器 ──
    let handle = tokio::runtime::Handle::current();
    egosync_engine::services::task_deadline_watch::spawn_hourly_watch(pool.clone(), handle.clone());
    egosync_engine::services::task_protection_watch::spawn_hourly_watch(
        pool.clone(),
        handle.clone(),
    );
    egosync_engine::services::scheduler::spawn_scheduler(
        pool.clone(),
        conv_pool.clone(),
        bus.clone(),
        secrets.clone(),
        handle,
    );

    // ── EngineCtx 单容器装配（与桌面壳同字段同语义）──
    let ctx = Arc::new(EngineCtx {
        pool: pool.clone(),
        conv_pool: conv_pool.clone(),
        registry,
        agent_config,
        sidecar: sidecar.clone(),
        agent_bridge,
        event_router,
        delegate_bridge,
        bus,
        secrets,
        data_dir: data_dir.clone(),
        opencode_workspace: opencode_workspace_dir.clone(),
        skills_root: opencode_workspace_dir.join(".opencode").join("skills"),
        home_dir: dirs::home_dir().unwrap_or_else(|| {
            // 二轮评审修复 #4：兜底分支保留但补 warn（平移等价偏差——
            // 旧壳为显式 ValidationError，命令级错误→构造期可观测的语义差
            // 由父代理在 Design Notes 登记）
            tracing::warn!(
                "无法获取用户主目录（HOME-less 环境？），Skill 发现根目录兜底为数据目录: {}",
                data_dir.display()
            );
            data_dir.clone()
        }),
    });

    // auth 已在池创建后先行装配（函数头部，二轮评审修复 #3）——
    // 装配失败时下方 sidecar/spawn 族尚未启动，零子资源泄漏。
    Ok(Arc::new(AppState {
        ctx,
        auth,
        // [T5 修订] Bearer 失败面限流器（独立实例——与 auth 路由限流器隔离）
        bearer_rate: crate::auth::RateLimiter::default(),
        events_tx,
        // Story 16.3：SSE 一次性票据表（内存态，与事件扇出同生命周期）
        sse_tickets: SseTicketStore::new(SSE_TICKET_TTL),
        sidecar,
        cancel,
        // Story 17.1：反代感知门控（main.rs 读取 env 后参数下发）
        behind_proxy,
        // Story 17.3 评审修复：逻辑级备份导入/导出互斥（见 AppState 字段注）
        import_lock: tokio::sync::Mutex::new(()),
    }))
}

/// 测试轻量引导：双池 + EngineCtx + 认证态 + 事件扇出（无 sidecar 启动 /
/// delegate 监听 / 调度器等后台任务；healthz deep 的 opencode 位自然为
/// false——未启动即不健康，探针语义不受影响）。behind_proxy=false
/// （直连语义——既有 I/O 矩阵测试行为不变）。
pub async fn build_test_state(
    data_dir: std::path::PathBuf,
    env_token: Option<String>,
) -> Result<Arc<AppState>, String> {
    build_test_state_with_proxy(data_dir, env_token, false).await
}

/// 测试轻量引导（反代感知通道，Story 17.1）：`behind_proxy=true` 装配——
/// XFP 感知同源判定与 Cookie Secure 的集成测试消费。
pub async fn build_test_state_with_proxy(
    data_dir: std::path::PathBuf,
    env_token: Option<String>,
    behind_proxy: bool,
) -> Result<Arc<AppState>, String> {
    let _ = std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("创建数据目录失败: {}", e))?;
    let pool = init_db(&data_dir.join("egosync.db"))
        .await
        .map_err(|e| format!("数据库初始化失败: {}", e))?;
    let conv_pool = init_conversations_db(&data_dir.join("conversations.db"))
        .await
        .map_err(|e| format!("对话数据库初始化失败: {}", e))?;

    let bus = Arc::new(SseEventBus::new());
    let events_tx: broadcast::Sender<SseEvent> = bus.sender().clone();
    let secrets: Arc<ServerSecretStore> = Arc::new(ServerSecretStore::new(data_dir.clone()));
    let opencode_workspace_dir = data_dir.join("opencode-workspace");
    let agent_config = AgentConfigService::new(opencode_workspace_dir.join("opencode.json"));
    let sidecar = Arc::new(Mutex::new(SidecarManager::new(None, None)));

    let ctx = Arc::new(EngineCtx {
        pool: pool.clone(),
        conv_pool: conv_pool.clone(),
        registry: Arc::new(ChatSessionRegistry::default()),
        agent_config,
        sidecar: sidecar.clone(),
        agent_bridge: AgentBridge::new(sidecar.lock().await.port()),
        event_router: Arc::new(EventRouter::new()),
        delegate_bridge: DelegateBridge::new(
            pool.clone(),
            conv_pool.clone(),
            "test-token".to_string(),
            Some(bus.clone()),
            AgentConfigService::new(opencode_workspace_dir.join("opencode.json")),
            opencode_workspace_dir.join(".opencode").join("skills"),
            secrets.clone(),
        ),
        bus,
        secrets,
        data_dir: data_dir.clone(),
        opencode_workspace: opencode_workspace_dir.clone(),
        skills_root: opencode_workspace_dir.join(".opencode").join("skills"),
        home_dir: data_dir.clone(),
    });

    // 同上：读失败 ⇒ Err（测试态与生产态同语义）
    let auth = AuthState::new(pool, env_token)
        .await
        .map_err(|e| format!("认证态装配失败: {}", e))?;

    Ok(Arc::new(AppState {
        ctx,
        auth,
        // [T5 修订] Bearer 失败面限流器（测试态与生产态同语义）
        bearer_rate: crate::auth::RateLimiter::default(),
        events_tx,
        // Story 16.3：SSE 一次性票据表（测试态与生产态同语义）
        sse_tickets: SseTicketStore::new(SSE_TICKET_TTL),
        sidecar,
        cancel: CancellationToken::new(),
        // Story 17.1：反代感知门控（测试通道参数化）
        behind_proxy,
        // Story 17.3 评审修复：备份互斥（测试态与生产态同语义）
        import_lock: tokio::sync::Mutex::new(()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Story 17.1：EGOSYNC_OPENCODE_PATH → SidecarManager 注入目录解析
    ///（架构 ⑤「同一注入点两个值」）。
    ///
    /// | 输入形态 | 期望 |
    /// |---------|------|
    /// | 文件路径 | 父目录（复用引擎 resources/ → 根搜索序） |
    /// | 目录 | 直传 |
    /// | 未设 / 空 | None（PATH fallback） |
    /// | 裸文件名 | None（无目录分量——PATH fallback） |
    #[test]
    fn opencode_resource_dir_maps_env_to_injection_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let bin = dir.path().join("opencode");
        std::fs::write(&bin, b"stub").expect("write stub binary");

        // 文件形态 → 父目录
        assert_eq!(
            opencode_resource_dir(bin.to_str()),
            Some(dir.path().to_path_buf())
        );
        // 目录形态 → 直传
        assert_eq!(
            opencode_resource_dir(dir.path().to_str()),
            Some(dir.path().to_path_buf())
        );
        // 未设 / 空 / 纯空白 → None（PATH fallback）
        assert_eq!(opencode_resource_dir(None), None);
        assert_eq!(opencode_resource_dir(Some("")), None);
        assert_eq!(opencode_resource_dir(Some("   ")), None);
        // 裸文件名（无目录分量）→ None（交给 PATH 解析）
        assert_eq!(opencode_resource_dir(Some("opencode")), None);
        // 不存在的文件路径 → 仍取父目录（不校验存在性——引擎搜索序
        // 自然落空回落 PATH，warn 可观测）
        assert_eq!(
            opencode_resource_dir(Some("/opt/nonexistent/opencode")),
            Some(std::path::PathBuf::from("/opt/nonexistent"))
        );
    }

    /// Story 17.1（评审 #27）：EGOSYNC_BEHIND_PROXY 取值映射——
    /// 解析回归曾可静默关闭 TLS 修复全链（同源放行 + Cookie Secure）
    /// 且全部测试绿（proxy_tls_test 注入的是解析后的布尔）。逐值钉死：
    ///
    /// | 取值 | 期望 |
    /// |------|------|
    /// | `1` / `true`（含大小写、trim 后） | 启用 |
    /// | `0` / 垃圾值 | 关闭（warn 可观测） |
    /// | 空串 / 未设置 | 关闭（静默） |
    /// | 非法 UTF-8 字节 | 关闭（warn 可观测——评审 #21） |
    #[test]
    fn parse_behind_proxy_maps_env_values() {
        // 启用态：1 / true / 大小写 / 带空白
        assert!(parse_behind_proxy(Ok("1".into())));
        assert!(parse_behind_proxy(Ok("true".into())));
        assert!(parse_behind_proxy(Ok("TRUE".into())));
        assert!(parse_behind_proxy(Ok("  True  ".into())));
        // 关闭态：显式否定 / 垃圾值 / 空串
        assert!(!parse_behind_proxy(Ok("0".into())));
        assert!(!parse_behind_proxy(Ok("yes".into())));
        assert!(!parse_behind_proxy(Ok("  ".into())));
        assert!(!parse_behind_proxy(Ok("".into())));
        // 未设置：关闭；非 UTF-8：关闭且不 panic（OsString 非法字节仅
        // unix 可构造——Windows 编译跳过该断言）
        assert!(!parse_behind_proxy(Err(std::env::VarError::NotPresent)));
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            assert!(!parse_behind_proxy(Err(std::env::VarError::NotUnicode(
                std::ffi::OsString::from_vec(b"\xff\xfe".to_vec())
            ))));
        }
    }
}
