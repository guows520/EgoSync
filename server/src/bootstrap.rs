//! server 引导（Story 15.4）：对等复刻桌面壳 setup 序列
//! （`egosync-app/src-tauri/src/lib.rs:42-416` 蓝本，剔除桌面专属）。
//!
//! 复刻序列：init_db 双池（`EGOSYNC_DATA_DIR`）→ 四接缝注入
//! （SseEventBus / 服务端 SecretStore / sidecar PATH fallback /
//! Handle::current()）→ ChatSessionRegistry → AgentConfig 全量同步 →
//! LLM provider 同步 → custom tools 写盘 → sidecar 启动+watchdog（PATH 无
//! opencode 时优雅降级——桌面同款路径）→ DelegateBridge 监听 → EventRouter
//! pump → 两个 hourly watch → spawn_scheduler → EngineCtx 装配。
//!
//! 剔除：companion_*（手机伴侣桌面宿主专属）、窗口/keyring、legacy
//! app-root opencode.json 清理（桌面历史数据关切，server 数据目录全新）。
//! 退出 = 取消 token + sidecar stop（main.rs 收口）。
//!
//! 测试轻量通道：[`build_test_state`] 只装配双池 + EngineCtx + 认证态 +
//! 事件扇出（无 sidecar/delegate/scheduler 后台任务）——I/O 矩阵集成测试
//! 不拉起完整运行时。

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
use crate::sse::{SseEvent, SseEventBus};
use crate::AppState;

/// 生产引导：完整桌面序列对等复刻（main.rs 与进程内复用）。
pub async fn build_app_state(
    data_dir: std::path::PathBuf,
    env_token: Option<String>,
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

    // ── Sidecar：opencode PATH fallback（None, None）+ 优雅降级 ──
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
    let mut sidecar = SidecarManager::new(None, None)
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
        events_tx,
        sidecar,
        cancel,
    }))
}

/// 测试轻量引导：双池 + EngineCtx + 认证态 + 事件扇出（无 sidecar 启动 /
/// delegate 监听 / 调度器等后台任务；healthz deep 的 opencode 位自然为
/// false——未启动即不健康，探针语义不受影响）。
pub async fn build_test_state(
    data_dir: std::path::PathBuf,
    env_token: Option<String>,
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
        events_tx,
        sidecar,
        cancel: CancellationToken::new(),
    }))
}
