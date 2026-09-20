use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Story 12.2: db/services/models/error 声明为 pub 以供 tests/test_companion.rs
// 集成测试访问（rlib 仅被测试消费，无运行时影响）
pub mod commands;
// Story 15.1：db/models/error 自引擎 crate 回引，路径语义不变
pub use egosync_engine::{db, error, events, models};
// llm 可见性语义保持：原先即为私有 mod，私有 use 使 crate::llm 照常可用
use egosync_engine::llm;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 日志同时输出到 stderr 和 app_data_dir/egosync.log
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let fmt_stderr = tracing_subscriber::fmt::layer();

    let log_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.egosync.app");
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::never(&log_dir, "egosync.log");
    let fmt_file = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(file_appender);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_stderr)
        .with(fmt_file)
        .try_init();

    // ── Story 16.3：模式预读（builder 构造前） ──
    // 模式事实源在 DB 外（desktop-mode.json——远程模式不打开 egosync.db，
    // app_settings 表在引擎启动后才可得，先有鸡问题）。此处的
    // app_data_dir 派生与 tauri PathResolver desktop 实现同式
    // （`dirs::data_dir()/identifier`），与 setup 内 `app.path().app_data_dir()`
    // 指向同一目录。
    // 预读结果决定两件事：invoke_handler 注册面（远程 = 仅壳命令）与
    // setup 装配序列（远程 = 引擎整体跳过）——模式在进程生命周期内恒定
    // （切换必经重启，见 remote_mode_restart）。
    let context = tauri::generate_context!();
    let pre_app_data_dir = dirs::data_dir().map(|dir| dir.join(&context.config().identifier));
    let boot_mode = pre_app_data_dir
        .as_deref()
        .map(services::desktop_mode::read_mode)
        .unwrap_or(services::desktop_mode::DesktopMode::Local);
    let remote_mode = boot_mode.is_remote();
    if remote_mode {
        tracing::info!(
            "桌面远程模式（desktop-mode.json=remote）：本地引擎装配整体跳过，桌面 = 远端实例客户端"
        );
    }

    // invoke_handler 注册面按模式二选一：远程模式仅注册双模式壳命令
    // （desktop_mode 三命令零引擎 state 依赖；远程态引擎 state 未 manage，
    // 业务命令在此模式物理不可达，不得注册——取未 manage 的 state 会
    // panic）。
    let builder = if remote_mode {
        tauri::Builder::default().invoke_handler(tauri::generate_handler![
            commands::desktop_mode::desktop_get_boot_config,
            commands::desktop_mode::remote_mode_save_config,
            commands::desktop_mode::remote_mode_restart,
        ])
    } else {
        tauri::Builder::default()
        .setup(move |app| {
            // Windows: 移除 DWM 边框，消除无边框窗口左/下/右的黑色边线
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_shadow(false);
                }
            }

            // ── Story 16.3：远程模式守卫（架构裁决 B：LOCAL→REMOTE 守卫 =
            // 本地引擎完整停机；重启式切换以进程退出达成，强于运行中停机）。
            // 引擎装配整体跳过 = 远程模式零本地业务写入：不打开
            // egosync.db/conversations.db、不写 opencode-workspace、不启
            // sidecar/调度器/伴侣/委派桥/双 watch。前端经壳命令
            // desktop_get_boot_config 获取 remoteUrl/keyring 令牌直连远端。
            if remote_mode {
                return Ok(());
            }

            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
            let db_path = app_data_dir.join("egosync.db");
            let conv_db_path = app_data_dir.join("conversations.db");

            let pool = tauri::async_runtime::block_on(async { db::pool::init_db(&db_path).await })
                .map_err(|e| format!("数据库初始化失败: {}", e))?;

            let conv_pool = tauri::async_runtime::block_on(async {
                db::pool::init_conversations_db(&conv_db_path).await
            })
            .map_err(|e| format!("对话数据库初始化失败: {}", e))?;

            app.manage(pool.clone());
            app.manage(conv_pool.clone());
            // Story 15.1 接缝一：注入桌面侧 SecretStore（keyring 实现）
            app.manage(services::secret_store_keyring::KeyringSecretStore::new());
            // Story 15.2 接缝二：注入桌面侧事件总线（转发 app_handle.emit），
            // engine 侧服务经 EngineEvents 发射，命令层可经 State 取用。
            app.manage(services::tauri_event_bus::TauriEventBus::new(app.handle().clone()));
            // Story 15.3：六组会话状态合并为单 ChatSessionRegistry（Arc 包装，
            // spawn 的后台任务与 agent_engine 共享同一实例）
            app.manage(Arc::new(commands::chat::ChatSessionRegistry::default()));
            let bridge_token = services::delegate_bridge::generate_bridge_token();
            let (delegate_listener, delegate_port) = tauri::async_runtime::block_on(async {
                services::delegate_bridge::bind_random_listener().await
            })
            .map_err(|e| format!("委派桥接服务初始化失败: {}", e))?;
            let opencode_workspace_dir = app_data_dir.join("opencode-workspace");

            // ── 清理 legacy app-root opencode.json 中的 EgoSync 托管 MCP ──
            // opencode 会沿父目录链向上合并 `opencode.json`，legacy
            // `%APPDATA%\com.egosync.app\opencode.json` 里残留的 managedByEgosync
            // MCP 会与私有 workspace 的同一 MCP 重复注册（Invalid session id / 404）。
            // 只删除 managed key，保留 provider/model 与用户自有 MCP。
            {
                let legacy_config_path = app_data_dir.join("opencode.json");
                if let Err(e) =
                    services::agent_config::purge_legacy_managed_mcp(&legacy_config_path)
                {
                    tracing::warn!("清理 legacy opencode.json 失败（降级继续）: {}", e);
                }
            }

            // ── AgentConfig: sync roles → opencode.json (non-blocking) ──
            // All opencode config (agents, provider, model) goes into
            // the project-level config that opencode reads for EgoSync sessions.
            // Custom tools (.opencode/tools/) provide egosync-specific tools
            // directly, replacing the previous MCP approach.
            let opencode_config_path = opencode_workspace_dir.join("opencode.json");
            let agent_config =
                services::agent_config::AgentConfigService::new(opencode_config_path);
            {
                let pool_ref: &sqlx::SqlitePool = app.state::<db::pool::DbPool>().inner();
                let managed_skills_root = opencode_workspace_dir.join(".opencode").join("skills");
                let managed_skills_ready = match tauri::async_runtime::block_on(async {
                    services::skill_registry::migrate_legacy_managed_paths(
                        pool_ref,
                        &managed_skills_root,
                    )
                    .await
                }) {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::warn!("迁移 legacy Skill 受控路径失败（相关 Skill 保持禁用）: {}", e);
                        false
                    }
                };
                let all_roles = tauri::async_runtime::block_on(async {
                    db::roles::list_all_roles(pool_ref).await
                });
                let butler_skills = tauri::async_runtime::block_on(async {
                    services::butler_config::get_butler_skills(pool_ref).await
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load butler Skill config: {}", e);
                    services::butler_config::default_butler_skills()
                });
                let skill_registry = if managed_skills_ready {
                    tauri::async_runtime::block_on(async { db::skills::list_skills(pool_ref).await })
                        .unwrap_or_else(|e| {
                            tracing::warn!("Failed to load Skill registry for opencode sync: {}", e);
                            Vec::new()
                        })
                } else {
                    Vec::new()
                };
                let role_mcp_prompts = tauri::async_runtime::block_on(async {
                    services::mcp_server::role_mcp_prompt_map(pool_ref).await
                });
                let butler_mcp_lines = tauri::async_runtime::block_on(async {
                    db::mcp_servers::butler_enabled_mcp_lines(pool_ref).await
                });
                tauri::async_runtime::block_on(async {
                    services::mcp_server::sync_enabled_mcp_to_opencode(pool_ref, &agent_config).await;
                });
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
                    (_, Err(e), _) => tracing::warn!("Failed to load role MCP bindings for opencode sync: {}", e),
                    (_, _, Err(e)) => tracing::warn!("Failed to load butler MCP bindings for opencode sync: {}", e),
                }
            }
            app.manage(agent_config);
            // Story 15.3：委派桥接迁引擎——AppHandle → 接缝注入
            // （事件总线/Agent 配置/Skill 根目录/密钥；构造点随依赖后移）
            let delegate_bridge = services::delegate_bridge::DelegateBridge::new(
                pool.clone(),
                conv_pool.clone(),
                bridge_token.clone(),
                Some(Arc::new(
                    app.state::<services::tauri_event_bus::TauriEventBus>()
                        .inner()
                        .clone(),
                )),
                app.state::<services::agent_config::AgentConfigService>()
                    .inner()
                    .clone(),
                opencode_workspace_dir.join(".opencode").join("skills"),
                Arc::new(services::secret_store_keyring::KeyringSecretStore::new()),
            );
            app.manage(delegate_bridge.clone());

            // ── LLM provider sync: write default provider/model/apiKey into opencode.json ──
            // Must happen before sidecar start so opencode picks up the config on boot.
            {
                let pool_ref: &sqlx::SqlitePool = app.state::<db::pool::DbPool>().inner();
                let ac_ref: &services::agent_config::AgentConfigService = app
                    .state::<services::agent_config::AgentConfigService>()
                    .inner();
                // Story 15.1 接缝一：密钥读取经注入的桌面侧 SecretStore
                let secrets: &services::secret_store_keyring::KeyringSecretStore = app
                    .state::<services::secret_store_keyring::KeyringSecretStore>()
                    .inner();
                if let Err(error) = tauri::async_runtime::block_on(async {
                    services::llm_config::sync_default_to_opencode(pool_ref, secrets, ac_ref).await
                }) { tracing::warn!("同步默认 LLM 配置到 opencode.json 失败: {}", error); }
            }

            // ── Custom tools: write .opencode/tools/ into opencode-workspace ──
            // opencode discovers custom tools from .opencode/tools/*.ts in the
            // project directory. We write them at startup so opencode loads them
            // when sessions are created.
            {
                if let Err(e) = services::agent_config::write_custom_tools(&opencode_workspace_dir)
                {
                    tracing::warn!("Failed to write custom tools (tools degraded): {}", e);
                }
            }

            // ── Sidecar: start opencode server (non-blocking, graceful degradation) ──
            let resource_dir = app.path().resource_dir().ok();
            let pool_ref: &sqlx::SqlitePool = app.state::<db::pool::DbPool>().inner();
            let original_no_proxy = services::llm_config::process_no_proxy_value();
            let no_proxy_value = tauri::async_runtime::block_on(async {
                services::llm_config::generate_no_proxy_value(pool_ref, original_no_proxy.as_deref()).await
            }).map_err(|e| format!("生成 sidecar NO_PROXY 失败: {}", e))?;
            let mut sidecar = services::sidecar::SidecarManager::new(resource_dir, None)
                .with_working_dir(opencode_workspace_dir.clone())
                .with_env(
                    services::delegate_bridge::BRIDGE_TOKEN_ENV,
                    bridge_token.clone(),
                )
                .with_env(
                    services::delegate_bridge::BRIDGE_PORT_ENV,
                    delegate_port.to_string(),
                )
                .with_env("NO_PROXY", no_proxy_value);

            let sidecar_started = tauri::async_runtime::block_on(async {
                match sidecar.start().await {
                    Ok(()) => {
                        tracing::info!("opencode sidecar started on port {}", sidecar.port());
                        true
                    }
                    Err(e) => {
                        tracing::warn!("opencode sidecar failed to start (degraded mode): {}", e);
                        false
                    }
                }
            });

            if sidecar_started {
                app.state::<services::agent_config::AgentConfigService>()
                    .mark_runtime_loaded();
            }

            let sidecar_port = sidecar.port();
            let opencode_available = if sidecar_started {
                true
            } else {
                let healthy =
                    tauri::async_runtime::block_on(async { sidecar.health_check().await });
                if healthy {
                    tracing::info!(
                        "opencode server already available on port {}; event router will attach",
                        sidecar_port
                    );
                }
                healthy
            };
            let sidecar_state = Arc::new(Mutex::new(sidecar));
            app.manage(sidecar_state.clone());

            let agent_bridge = services::agent_bridge::AgentBridge::new(sidecar_port);
            app.manage(agent_bridge.clone());

            // Global opencode event router: subscribes to /event SSE and
            // demultiplexes to per-session subscribers whenever an opencode
            // server is reachable, including an already-running external server.
            let event_router = Arc::new(services::event_router::EventRouter::new());
            app.manage(event_router.clone());
            if opencode_available {
                let router_clone = event_router.clone();
                let bridge_clone = agent_bridge.clone();
                tauri::async_runtime::spawn(async move {
                    router_clone.run_pump(bridge_clone).await;
                });
            }

            // Start watchdog only if sidecar started successfully
            let watchdog_cancel = CancellationToken::new();
            {
                let bridge = delegate_bridge.clone();
                let cancel_clone = watchdog_cancel.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = services::delegate_bridge::start_server_on_listener(
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
                let cancel_clone = watchdog_cancel.clone();
                let mgr_clone = sidecar_state.clone();
                tauri::async_runtime::spawn(async move {
                    services::sidecar::start_watchdog(mgr_clone, cancel_clone).await;
                });
            }
            app.manage(watchdog_cancel);

            // Story 3.3: 每小时检查临期任务，自动升入 Q1。
            // 内部错误只 warn，不阻塞 Tauri setup。
            // Story 15.2 接缝四：注入宿主 runtime Handle 派生任务
            // （setup 同步上下文无 reactor，裸 tokio::spawn 会 panic）。
            // tauri 全局运行时经 handle() 取出，inner() 即其 tokio Handle。
            services::task_deadline_watch::spawn_hourly_watch(
                pool.clone(),
                tauri::async_runtime::handle().inner().clone(),
            );

            // Story 3.5: 每小时检查 Q2 任务保护状态，连续被挤压标记 at_risk。
            // 内部错误只 warn，不阻塞 Tauri setup。
            services::task_protection_watch::spawn_hourly_watch(
                pool.clone(),
                tauri::async_runtime::handle().inner().clone(),
            );

            // ── Story 12.2: 手机伴侣 WS 监听 + NSD 广播（非阻塞降级） ──
            // keyring 不可用时降级：companion_* 命令调用会因 state 未管理而失败，
            // 不影响桌面其余功能。
            match services::companion_connection::CompanionState::new(Some(app.handle().clone())) {
                Ok(companion_state) => {
                    let companion_state = Arc::new(companion_state);
                    app.manage(companion_state.clone());
                    // ── Story 13.1: 快照引擎（debounce 重建 + 建连全量补发）──
                    let snapshot_engine = Arc::new(
                        services::companion_snapshot::CompanionSnapshotEngine::new(
                            pool.clone(),
                            conv_pool.clone(),
                            companion_state.clone(),
                        ),
                    );
                    tauri::async_runtime::block_on(async {
                        companion_state
                            .set_snapshot_request_tx(snapshot_engine.snapshot_request_tx())
                            .await;
                    });
                    app.manage(snapshot_engine.clone());
                    let engine_for_run = snapshot_engine.clone();
                    tauri::async_runtime::spawn(async move {
                        engine_for_run.run().await;
                    });
                    services::companion_snapshot::register_write_signal_listeners(
                        app.handle().clone(),
                        snapshot_engine.notify_signal(),
                    );
                    // ── Story 13.3：指令 dispatcher + llm:stream 镜像监听 ──
                    // dispatch 需要 engine 写信号（suggestion 确认/拒绝补发）；
                    // 镜像监听复用 try_enqueue_single 单帧出站。装配在监听启动前，
                    // 确保 COMMAND 帧到达时 dispatcher 已就位。
                    let dispatcher = Arc::new(
                        services::companion_dispatch::CompanionDispatcher::production(
                            pool.clone(),
                            conv_pool.clone(),
                            app.handle().clone(),
                            snapshot_engine.notify_signal(),
                        ),
                    );
                    tauri::async_runtime::block_on(async {
                        companion_state.set_dispatcher(dispatcher).await;
                    });
                    services::companion_dispatch::register_stream_mirror(
                        app.handle().clone(),
                        companion_state.clone(),
                    );
                    let pool_c = pool.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) =
                            services::companion_connection::start_companion_listener(
                                pool_c, companion_state.clone(),
                            )
                            .await
                        {
                            // 监听失败如实标记（状态查询不再伪装 Listening）
                            companion_state.set_failed();
                            tracing::warn!("companion 连接服务启动失败（降级继续）: {}", e);
                        }
                    });
                }
                Err(e) => tracing::warn!("companion 状态初始化失败（降级继续）: {}", e),
            }

            // Story 4.1: 角色后台调度器，按 proactivity_level 配置频率运行工作循环。
            // 60 秒基础 tick，每次 tick 动态查询角色列表，passive 跳过。
            // 内部错误只 warn，不阻塞 Tauri setup。
            // Story 15.2 接缝二/四：事件经 EngineEvents、密钥经 SecretStore
            // 注入；注入宿主 runtime Handle 派生任务（setup 同步上下文无
            // reactor，裸 tokio::spawn 会 panic）。KeyringSecretStore 为 unit
            // struct，即席构造零成本（与 agent_engine 调用点同款手法）。
            // Story 15.3：与 manage 的 TauriEventBus 单实例统一（deferred-work），
            // 不再独立构造第二实例。
            let scheduler_event_bus: Arc<dyn services::event_bus::EngineEvents> = Arc::new(
                app.state::<services::tauri_event_bus::TauriEventBus>()
                    .inner()
                    .clone(),
            );
            let scheduler_secret: Arc<dyn egosync_engine::services::secret_store::SecretStore> =
                Arc::new(services::secret_store_keyring::KeyringSecretStore::new());
            services::scheduler::spawn_scheduler(
                pool.clone(),
                conv_pool.clone(),
                scheduler_event_bus,
                scheduler_secret,
                tauri::async_runtime::handle().inner().clone(),
            );

            // ── Story 15.4：EngineCtx 单容器装配 ──
            // 聚合既有 manage 的同一实例（Arc 克隆，非二次构造），web-ok 命令
            // 体迁引擎后经此容器取依赖。既有 manage 全部保留——companion_dispatch、
            // 集成测试与既有 State<T> 取用零改动。
            let engine_ctx = Arc::new(egosync_engine::commands::ctx::EngineCtx {
                pool: pool.clone(),
                conv_pool: conv_pool.clone(),
                registry: app
                    .state::<Arc<commands::chat::ChatSessionRegistry>>()
                    .inner()
                    .clone(),
                agent_config: app
                    .state::<services::agent_config::AgentConfigService>()
                    .inner()
                    .clone(),
                sidecar: sidecar_state.clone(),
                agent_bridge: agent_bridge.clone(),
                event_router: event_router.clone(),
                delegate_bridge: delegate_bridge.clone(),
                bus: Arc::new(
                    app.state::<services::tauri_event_bus::TauriEventBus>()
                        .inner()
                        .clone(),
                ),
                secrets: Arc::new(services::secret_store_keyring::KeyringSecretStore::new()),
                data_dir: app_data_dir.clone(),
                opencode_workspace: opencode_workspace_dir.clone(),
                skills_root: opencode_workspace_dir.join(".opencode").join("skills"),
                home_dir: dirs::home_dir().unwrap_or_else(|| {
                    // 二轮评审修复 #4：兜底分支保留但补 warn（与 server
                    // bootstrap 同款——旧壳为显式 ValidationError，语义差由
                    // Design Notes 登记裁决）
                    tracing::warn!(
                        "无法获取用户主目录（HOME-less 环境？），Skill 发现根目录兜底为数据目录: {}",
                        app_data_dir.display()
                    );
                    app_data_dir.clone()
                }),
            });
            app.manage(engine_ctx);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Story 16.3：双模式壳命令（本地态同样注册——本地→远程切换入口；
            // 远程态见上方 builder 分支）
            commands::desktop_mode::desktop_get_boot_config,
            commands::desktop_mode::remote_mode_save_config,
            commands::desktop_mode::remote_mode_restart,
            commands::secret::secret_store_save,
            commands::secret::secret_store_load,
            commands::secret::secret_store_delete,
            commands::llm_config::llm_config_list,
            commands::llm_config::llm_config_create,
            commands::llm_config::llm_config_update,
            commands::llm_config::llm_config_delete,
            commands::llm_config::llm_config_set_default,
            commands::llm_config::llm_config_test_connection,
            commands::llm_config::llm_config_list_models,
            commands::llm_config::llm_config_list_models_by_params,
            commands::chat::chat_send_message,
            commands::chat::chat_get_history,
            commands::chat::chat_get_message_process_events,
            commands::chat::chat_pick_working_directory,
            commands::chat::chat_get_conversation,
            commands::chat::chat_get_butler_conversation,
            commands::chat::chat_get_role_conversation,
            commands::chat::chat_list_conversations,
            commands::chat::chat_stop_streaming,
            commands::chat::chat_delete_conversation,
            commands::chat::chat_new_conversation,
            commands::memory::memory_list,
            commands::memory::memory_list_all,
            commands::memory::memory_count,
            commands::memory::memory_get_source_messages,
            commands::memory::memory_delete,
            commands::role::role_create,
            commands::role::role_list,
            commands::role::role_list_archived,
            commands::role::role_update,
            commands::role::role_update_skills,
            commands::role::role_update_proactivity,
            commands::role::role_archive,
            commands::role::role_restore,
            commands::role::role_delete,
            commands::task::task_create,
            commands::task::task_list_by_role,
            commands::task::task_list_butler,
            commands::task::task_list_all,
            commands::task::task_update,
            commands::task::task_delete,
            commands::task::task_reorder,
            commands::task::task_toggle_complete,
            commands::task::task_check_protection_status,
            commands::task::task_check_q2_reminders,
            commands::task_decomposition::task_decomposition_list_pending,
            commands::task_decomposition::task_decomposition_accept,
            commands::task_decomposition::task_decomposition_keep_single,
            commands::mcp::mcp_server_list,
            commands::mcp::mcp_server_list_for_role,
            commands::mcp::mcp_server_list_available_for_role,
            commands::mcp::mcp_server_create,
            commands::mcp::mcp_server_update,
            commands::mcp::mcp_server_delete,
            commands::mcp::mcp_server_test,
            commands::mcp::mcp_server_add_to_role,
            commands::mcp::mcp_server_remove_from_role,
            commands::mcp::mcp_server_list_for_butler,
            commands::mcp::mcp_server_list_available_for_butler,
            commands::mcp::mcp_server_add_to_butler,
            commands::mcp::mcp_server_remove_from_butler,
            commands::mcp::mcp_server_refresh_butler_runtime,
            commands::skill::skill_list_registry,
            commands::skill::skill_list_for_role,
            commands::skill::skill_list_all_role_skills,
            commands::skill::skill_list_selectable_for_scope,
            commands::skill::skill_pick_custom_directory,
            commands::skill::skill_preview_custom,
            commands::skill::skill_discover_opencode,
            commands::skill::skill_import_opencode,
            commands::skill::skill_import_custom,
            commands::skill::skill_remove_from_role,
            commands::skill::skill_delete,
            commands::app::app_is_first_launch,
            commands::app::app_complete_onboarding,
            commands::app::app_is_llm_configured,
            commands::app::app_get_butler_skills,
            commands::app::app_update_butler_skills,
            commands::app::app_sidecar_status,
            commands::app::app_get_setting,
            commands::app::app_set_setting,
            commands::app::app_performance_snapshot,
            #[cfg(feature = "perf-test")]
            commands::app::app_emit_test_stream,
            #[cfg(feature = "perf-test")]
            commands::app::app_seed_perf_data,
            commands::scheduler::scheduler_get_times,
            commands::scheduler::scheduler_set_times,
            commands::suggestion::suggestion_list_pending,
            commands::suggestion::suggestion_confirm,
            commands::suggestion::suggestion_reject,
            commands::notification::notification_create,
            commands::notification::notification_list,
            commands::notification::notification_mark_read,
            commands::notification::notification_count_unread,
            commands::dashboard::dashboard_get_status,
            commands::dashboard::dashboard_get_metrics,
            commands::mission::mission_get,
            commands::mission::mission_update,
            commands::mission::mission_infer,
            commands::mission::mission_infer_eligibility,
            commands::briefing::briefing_get_latest,
            commands::briefing::briefing_generate_now,
            commands::review::review_get_latest,
            commands::review::review_get_by_week,
            commands::review::review_generate_now,
            commands::review::review_plan_bigrocks,
            commands::review::review_get_bigrock_suggestions,
            commands::settings::settings_get_schedule,
            commands::settings::settings_update_schedule,
            commands::data::data_export,
            commands::data::data_destroy,
            commands::data::pick_import_file,
            commands::data::data_import,
            commands::companion::pairing_generate_qr,
            commands::companion::pairing_confirm,
            commands::companion::paired_device_list,
            commands::companion::paired_device_remove,
            commands::companion::companion_get_status,
            commands::companion::companion_get_relay_addr,
            commands::companion::companion_set_relay_addr,
        ])
    };

    builder
        .build(context)
        .expect("error while building tauri application")
        .run(move |app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Story 16.3：远程模式 Exit 分支——引擎 state 未 manage，
                // 不得取（state() 会 panic）；本地引擎从未启动，零清理面。
                if remote_mode {
                    return;
                }
                tracing::info!("Tauri exiting — stopping opencode sidecar");
                let cancel = app_handle.state::<CancellationToken>();
                cancel.cancel();
                let sidecar = app_handle.state::<Arc<Mutex<services::sidecar::SidecarManager>>>();
                tauri::async_runtime::block_on(async {
                    if let Err(e) = sidecar.lock().await.stop().await {
                        tracing::warn!("Failed to stop opencode sidecar cleanly: {}", e);
                    }
                });
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_compiles() {
        // 验证 crate 可编译，依赖无冲突
        assert!(true);
    }

    #[test]
    fn opencode_config_path_lives_in_sidecar_workspace() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
        )
        .expect("read lib.rs");

        assert!(source.contains(
            "let opencode_config_path = opencode_workspace_dir.join(\"opencode.json\");"
        ));
        assert!(!source.contains(
            "let opencode_config_path = app_data_dir.join(\"opencode.json\");"
        ));
    }

    #[test]
    fn startup_purges_legacy_managed_mcp_from_app_root() {
        // WHY: legacy `%APPDATA%\com.egosync.app\opencode.json` 会被 opencode 向上
        // 合并，残留 managed MCP 导致天气 MCP 重复注册。启动时必须清理它，且作用于
        // app_data_dir 根目录（而非私有 workspace）。
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
        )
        .expect("read lib.rs");

        assert!(source.contains(
            "let legacy_config_path = app_data_dir.join(\"opencode.json\");"
        ));
        assert!(source.contains("purge_legacy_managed_mcp(&legacy_config_path)"));
    }

    // ── Story 16.3：远程模式装配守卫（源码扫描钉——「重启清理序」与
    //    远程态注册面不变量；与上方既有源码扫描测试同款纪律） ──

    fn lib_source() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
        )
        .expect("read lib.rs")
    }

    #[test]
    fn remote_mode_restart_cleans_up_before_restart() {
        // WHY（spec「重启清理序」守卫）：remote_mode_restart 必须在
        // app.restart() **之前**显式执行既有退出清理（watchdog cancel +
        // sidecar.stop）——RunEvent::Exit 在 restart 路径不保证触发，
        // 清理先行是唯一可靠面；且用 try_state（远程态引擎 state 未
        // manage，state() 会 panic）。commands/desktop_mode.rs 源码序钉。
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/commands/desktop_mode.rs"),
        )
        .expect("read commands/desktop_mode.rs");

        // 清理序：try_state 探测（不 panic）→ cancel → sidecar.stop → restart
        let cancel_pos = source
            .find("app.try_state::<CancellationToken>()")
            .expect("watchdog 取消须以 try_state 探测（远程态未 manage 不 panic）");
        let sidecar_pos = source
            .find("app.try_state::<Arc<Mutex<SidecarManager>>>()")
            .expect("sidecar 停机须以 try_state 探测");
        let stop_pos = source
            .find("sidecar.lock().await.stop().await")
            .expect("sidecar 显式停机调用");
        // 实调用点（带分号——文档注释中的 `app.restart()` 不带分号）
        let restart_pos = source
            .find("app.restart();")
            .expect("restart 调用点");
        assert!(cancel_pos < restart_pos, "cancel 须先于 restart");
        assert!(sidecar_pos < stop_pos, "try_state 探测须先于 stop");
        assert!(stop_pos < restart_pos, "sidecar.stop 须先于 restart（清理先行）");
        // 禁用面：不得使用会 panic 的 state()（远程态引擎 state 未 manage）
        assert!(
            !source.contains("app.state::<"),
            "壳命令不得使用 state()——远程态未 manage 会 panic（须 try_state）"
        );
    }

    #[test]
    fn remote_mode_exit_branch_does_not_touch_unmanaged_state() {
        // WHY（spec Code Map 明示）：远程模式 RunEvent::Exit 分支不得取
        // 未 manage 的 state（state() 会 panic）——先守卫后清理的序钉。
        let source = lib_source();
        let exit_branch = source
            .find("if let tauri::RunEvent::Exit = event {")
            .expect("Exit 事件分支存在");
        let guard = source[exit_branch..]
            .find("if remote_mode {")
            .expect("Exit 分支内的远程模式守卫存在");
        let cleanup = source[exit_branch..]
            .find("app_handle.state::<CancellationToken>()")
            .expect("Exit 分支内的本地清理面存在");
        // 守卫（远程 return）必须先于 state() 取用——远程态先短路
        assert!(
            guard < cleanup,
            "Exit 分支的远程守卫须先于 state() 取用（远程态未 manage，后取会 panic）"
        );
    }

    #[test]
    fn remote_builder_registers_only_dual_mode_shell_commands() {
        // WHY：远程模式 invoke_handler 只注册双模式壳命令（零引擎 state
        // 依赖）——业务命令在此模式物理不可达（引擎未装配、state 未
        // manage），注册即 panic 面。lib.rs 的 builder 分支结构钉。
        let source = lib_source();
        let remote_branch = source
            .find("let builder = if remote_mode {")
            .expect("模式二选一 builder 分支存在");
        let local_branch = source
            .find(".setup(move |app| {")
            .expect("本地分支 setup 存在");
        // 远程分支片段（到本地分支为止）只含壳命令注册
        let remote_fragment = &source[remote_branch..local_branch];
        assert!(remote_fragment.contains("commands::desktop_mode::desktop_get_boot_config"));
        assert!(remote_fragment.contains("commands::desktop_mode::remote_mode_save_config"));
        assert!(remote_fragment.contains("commands::desktop_mode::remote_mode_restart"));
        // 片段内不得注册任何非 desktop_mode 命令（引擎依赖面零注册）
        for line in remote_fragment.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("commands::") {
                assert!(
                    trimmed.starts_with("commands::desktop_mode::"),
                    "远程分支只可注册 desktop_mode 壳命令，发现: {}",
                    trimmed
                );
            }
        }
    }

    #[test]
    fn remote_mode_skips_engine_assembly_before_db_open() {
        // WHY（架构裁决 B 守卫）：远程模式引擎装配整体跳过须发生在
        // **打开数据库之前**（零本地业务写入的硬前提——init_db 是首个
        // 本地写入面）。setup 闭包内的守卫序钉。
        let source = lib_source();
        let guard = source
            .find("if remote_mode {\n                return Ok(());")
            .or_else(|| source.find("if remote_mode {"))
            .expect("setup 内远程守卫存在");
        let db_init = source
            .find("db::pool::init_db(&db_path)")
            .expect("主库初始化存在");
        assert!(
            guard < db_init,
            "远程守卫须先于 init_db（远程模式零本地业务写入——不打开 egosync.db）"
        );
    }
}
