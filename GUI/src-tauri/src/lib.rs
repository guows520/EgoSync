use tauri::Manager;

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_compiles() {
        // 验证 crate 可编译，依赖无冲突
        assert!(true);
    }
}
