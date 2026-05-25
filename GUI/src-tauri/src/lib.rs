use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

mod commands;
mod db;
mod error;
mod llm;
mod models;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
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

            app.manage(pool);
            app.manage(conv_pool);
            app.manage(commands::chat::StreamingState::default());
            app.manage(commands::chat::CancelTokens::default());
            app.manage(commands::chat::OnboardingConversations::default());

            // ── AgentConfig: sync roles → opencode.json (non-blocking) ──
            let opencode_config_path = app_data_dir.join("opencode.json");
            let agent_config =
                services::agent_config::AgentConfigService::new(opencode_config_path);
            {
                let pool_ref: &sqlx::SqlitePool = app.state::<db::pool::DbPool>().inner();
                let all_roles = tauri::async_runtime::block_on(async {
                    db::roles::list_all_roles(pool_ref).await
                });
                match all_roles {
                    Ok(roles) => {
                        if let Err(e) = agent_config.full_sync(&roles) {
                            tracing::warn!("opencode.json full sync failed (degraded): {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load roles for opencode sync: {}", e);
                    }
                }
            }
            app.manage(agent_config);

            // ── Sidecar: start opencode server (non-blocking, graceful degradation) ──
            let resource_dir = app.path().resource_dir().ok();
            let mut sidecar = services::sidecar::SidecarManager::new(resource_dir, None);

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
            let sidecar_state = Arc::new(Mutex::new(sidecar));
            app.manage(sidecar_state.clone());

            let agent_bridge = services::agent_bridge::AgentBridge::new(sidecar_port);
            app.manage(agent_bridge);

            // Start watchdog only if sidecar started successfully
            let watchdog_cancel = CancellationToken::new();
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
            commands::chat::chat_get_butler_conversation,
            commands::chat::chat_get_role_conversation,
            commands::chat::chat_list_conversations,
            commands::chat::chat_stop_streaming,
            commands::chat::chat_delete_conversation,
            commands::chat::chat_new_conversation,
            commands::role::role_create,
            commands::role::role_list,
            commands::role::role_list_archived,
            commands::role::role_update,
            commands::role::role_archive,
            commands::role::role_restore,
            commands::role::role_delete,
            commands::app::app_is_first_launch,
            commands::app::app_complete_onboarding,
            commands::app::app_is_llm_configured,
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
