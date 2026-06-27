use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod commands;
mod db;
mod error;
mod llm;
mod models;
mod services;

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

    tauri::Builder::default()
        .setup(|app| {
            // Windows: 移除 DWM 边框，消除无边框窗口左/下/右的黑色边线
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_shadow(false);
                }
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
            app.manage(commands::chat::OpencodeMcpScopeLock::default());
            app.manage(commands::chat::StreamingState::default());
            app.manage(commands::chat::CancelTokens::default());
            app.manage(commands::chat::OpencodeSessions::default());
            app.manage(commands::chat::OnboardingConversations::default());
            app.manage(commands::chat::MemoryExtractionState::default());
            let bridge_token = services::delegate_bridge::generate_bridge_token();
            let (delegate_listener, delegate_port) = tauri::async_runtime::block_on(async {
                services::delegate_bridge::bind_random_listener().await
            })
            .map_err(|e| format!("委派桥接服务初始化失败: {}", e))?;
            let delegate_bridge = services::delegate_bridge::DelegateBridge::new(
                pool.clone(),
                conv_pool.clone(),
                bridge_token.clone(),
            );
            app.manage(delegate_bridge.clone());

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
                let skill_registry = tauri::async_runtime::block_on(async {
                    db::skills::list_skills(pool_ref).await
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load Skill registry for opencode sync: {}", e);
                    Vec::new()
                });
                let role_mcp_prompts = tauri::async_runtime::block_on(async {
                    services::mcp_server::role_mcp_prompt_map(pool_ref).await
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to load role MCP bindings for opencode sync: {}", e);
                    std::collections::HashMap::new()
                });
                tauri::async_runtime::block_on(async {
                    services::mcp_server::sync_enabled_mcp_to_opencode(pool_ref, &agent_config).await;
                });
                match all_roles {
                    Ok(roles) => {
                        if let Err(e) = agent_config.full_sync_with_skills_and_mcp(
                            &roles,
                            &butler_skills,
                            &skill_registry,
                            &role_mcp_prompts,
                        ) {
                            tracing::warn!("opencode.json full sync failed (degraded): {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load roles for opencode sync: {}", e);
                    }
                }
            }
            app.manage(agent_config);

            // ── LLM provider sync: write default provider/model/apiKey into opencode.json ──
            // Must happen before sidecar start so opencode picks up the config on boot.
            {
                let pool_ref: &sqlx::SqlitePool = app.state::<db::pool::DbPool>().inner();
                let ac_ref: &services::agent_config::AgentConfigService = app
                    .state::<services::agent_config::AgentConfigService>()
                    .inner();
                tauri::async_runtime::block_on(async {
                    services::llm_config::sync_default_to_opencode(pool_ref, ac_ref).await;
                });
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
            let mut sidecar = services::sidecar::SidecarManager::new(resource_dir, None)
                .with_working_dir(opencode_workspace_dir.clone())
                .with_env(
                    services::delegate_bridge::BRIDGE_TOKEN_ENV,
                    bridge_token.clone(),
                )
                .with_env(
                    services::delegate_bridge::BRIDGE_PORT_ENV,
                    delegate_port.to_string(),
                );

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
            services::task_deadline_watch::spawn_hourly_watch(pool.clone());

            // Story 3.5: 每小时检查 Q2 任务保护状态，连续被挤压标记 at_risk。
            // 内部错误只 warn，不阻塞 Tauri setup。
            services::task_protection_watch::spawn_hourly_watch(pool.clone());

            // Story 4.1: 角色后台调度器，按 proactivity_level 配置频率运行工作循环。
            // 60 秒基础 tick，每次 tick 动态查询角色列表，passive 跳过。
            // 内部错误只 warn，不阻塞 Tauri setup。
            services::scheduler::spawn_scheduler(pool.clone(), conv_pool.clone(), app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::secret::secret_store_save,
            commands::secret::secret_store_load,
            commands::secret::secret_store_delete,
            commands::llm_config::llm_config_list,
            commands::llm_config::llm_config_create,
            commands::llm_config::llm_config_update,
            commands::llm_config::llm_config_delete,
            commands::llm_config::llm_config_set_default,
            commands::llm_config::llm_config_test_connection,
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
            commands::mcp::mcp_server_list,
            commands::mcp::mcp_server_list_for_role,
            commands::mcp::mcp_server_list_available_for_role,
            commands::mcp::mcp_server_create,
            commands::mcp::mcp_server_update,
            commands::mcp::mcp_server_delete,
            commands::mcp::mcp_server_test,
            commands::mcp::mcp_server_add_to_role,
            commands::mcp::mcp_server_remove_from_role,
            commands::skill::skill_list_registry,
            commands::skill::skill_list_for_role,
            commands::skill::skill_list_all_role_skills,
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
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
}
