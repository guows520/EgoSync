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

            // ── AgentConfig: sync roles → opencode.json (non-blocking) ──
            // All opencode config (agents, provider, model) goes into
            // the project-level config that opencode reads for EgoSync sessions.
            // Custom tools (.opencode/tools/) provide egosync-specific tools
            // directly, replacing the previous MCP approach.
            let opencode_config_path = app_data_dir.join("opencode.json");
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
                match all_roles {
                    Ok(roles) => {
                        if let Err(e) = agent_config.full_sync_with_skills(
                            &roles,
                            &butler_skills,
                            &skill_registry,
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

            let opencode_workspace_dir = app_data_dir.join("opencode-workspace");

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
            commands::skill::skill_list_registry,
            commands::skill::skill_list_for_role,
            commands::skill::skill_list_all_role_skills,
            commands::skill::skill_pick_custom_directory,
            commands::skill::skill_preview_custom,
            commands::skill::skill_import_custom,
            commands::skill::skill_remove_from_role,
            commands::skill::skill_delete,
            commands::app::app_is_first_launch,
            commands::app::app_complete_onboarding,
            commands::app::app_is_llm_configured,
            commands::app::app_get_butler_skills,
            commands::app::app_update_butler_skills,
            commands::app::app_sidecar_status,
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
}
