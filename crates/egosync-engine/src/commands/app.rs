//! app 域命令体（Story 15.4 自壳 `commands/app.rs` 平移——9 条 web-ok 命令；
//! 2 条 perf-test 门控命令（app_emit_test_stream / app_seed_perf_data）留壳
//! 不迁）。业务逻辑零改动；State 取值改 `&EngineCtx`；内联测试随迁）。

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::commands::ctx::EngineCtx;
use crate::db::app_settings;
use crate::db::settings;
use crate::error::AppError;
use crate::models::agent::SidecarStatus;
use crate::models::role::{ButlerSkillsConfig, UpdateRoleSkillsInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::sidecar::SidecarManager;

pub async fn app_is_first_launch(ctx: &EngineCtx) -> Result<bool, AppError> {
    let value = app_settings::get_setting(&ctx.pool, "onboarding_completed").await?;
    Ok(value.as_deref() != Some("true"))
}

pub async fn app_complete_onboarding(ctx: &EngineCtx) -> Result<(), AppError> {
    app_settings::set_setting(&ctx.pool, "onboarding_completed", "true").await
}

pub async fn app_is_llm_configured(ctx: &EngineCtx) -> Result<bool, AppError> {
    let count = settings::count_llm_configs(&ctx.pool).await?;
    Ok(count > 0)
}

pub async fn app_get_butler_skills(ctx: &EngineCtx) -> Result<ButlerSkillsConfig, AppError> {
    crate::services::butler_config::get_butler_skills(&ctx.pool).await
}

pub async fn app_update_butler_skills(
    ctx: &EngineCtx,
    input: UpdateRoleSkillsInput,
) -> Result<ButlerSkillsConfig, AppError> {
    let (agent_config, registry): (&AgentConfigService, &Arc<crate::registry::ChatSessionRegistry>) =
        (&ctx.agent_config, &ctx.registry);
    let _guard = registry.opencode_mcp_scope_lock.0.lock().await;
    let skills = crate::services::butler_config::set_butler_skills(&ctx.pool, &input).await?;
    let registry = crate::db::skills::list_skills(&ctx.pool).await?;
    let mcp_lines = crate::db::mcp_servers::butler_enabled_mcp_lines(&ctx.pool).await?;
    agent_config.sync_butler_skills_with_registry_and_mcp(&skills, &registry, &mcp_lines)?;
    Ok(skills)
}

pub async fn app_sidecar_status(
    ctx: &EngineCtx,
) -> Result<SidecarStatus, AppError> {
    let sidecar: &Arc<Mutex<SidecarManager>> = &ctx.sidecar;
    let mut mgr = sidecar.lock().await;
    Ok(SidecarStatus {
        running: mgr.is_running(),
        port: mgr.port(),
        uptime_secs: mgr.uptime_secs(),
    })
}

pub async fn app_get_setting(ctx: &EngineCtx, key: String) -> Result<Option<String>, AppError> {
    // 保留键拒绝读（评审修复 #6：server_token_hash 等凭据材料只经
    // 专用通道，防离线爆破读取面——双宿主同行为）
    if app_settings::is_reserved_setting_key(&key) {
        return Err(AppError::ValidationError(format!("保留键不可读取: {}", key)));
    }
    app_settings::get_setting(&ctx.pool, &key).await
}

pub async fn app_set_setting(ctx: &EngineCtx, key: String, value: String) -> Result<(), AppError> {
    // 保留键拒绝写（评审修复 #6：防库态凭据被命令覆写接管）
    if app_settings::is_reserved_setting_key(&key) {
        return Err(AppError::ValidationError(format!("保留键不可写入: {}", key)));
    }
    app_settings::set_setting(&ctx.pool, &key, &value).await
}

// ── Performance benchmark commands (Story 8.4) ──

/// 进程内存与运行时快照，供 E2E 性能基准测量使用（AC5）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSnapshot {
    /// 主进程 RSS（MB）
    pub rss_mb: u64,
    /// WebView JS 堆已用（MB），WebView 无 Node process，此处返回 null
    pub js_heap_used_mb: Option<f64>,
    /// 进程运行时长（秒）
    pub process_uptime_secs: u64,
    /// sidecar 进程 RSS（MB），未运行时为 null
    pub sidecar_rss_mb: Option<u64>,
}

/// 读取当前进程 + sidecar 进程的内存快照（AC5）。
/// 使用 sysinfo crate 跨平台读取 RSS。
pub async fn app_performance_snapshot(
    ctx: &EngineCtx,
) -> Result<PerformanceSnapshot, AppError> {
    let sidecar: &Arc<Mutex<SidecarManager>> = &ctx.sidecar;
    let current_pid = std::process::id();
    let sidecar_pid = {
        let mgr = sidecar.lock().await;
        mgr.child_pid()
    };

    let pids: Vec<sysinfo::Pid> = [Some(current_pid), sidecar_pid]
        .into_iter()
        .flatten()
        .map(sysinfo::Pid::from_u32)
        .collect();

    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&pids), true);

    let rss_mb = sys
        .process(sysinfo::Pid::from_u32(current_pid))
        .map(|p| p.memory() / 1024 / 1024)
        .unwrap_or(0);

    let sidecar_rss_mb = sidecar_pid.and_then(|pid| {
        sys.process(sysinfo::Pid::from_u32(pid))
            .map(|p| p.memory() / 1024 / 1024)
    });

    let process_uptime_secs = sys
        .process(sysinfo::Pid::from_u32(current_pid))
        .map(|p| p.start_time())
        .map(|start| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now.saturating_sub(start)
        })
        .unwrap_or(0);

    Ok(PerformanceSnapshot {
        rss_mb,
        js_heap_used_mb: None, // WebView 无 Node process.memoryUsage()
        process_uptime_secs,
        sidecar_rss_mb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_snapshot_serializes_camel_case() {
        let snap = PerformanceSnapshot {
            rss_mb: 150,
            js_heap_used_mb: Some(45.5),
            process_uptime_secs: 120,
            sidecar_rss_mb: Some(80),
        };
        let json = serde_json::to_value(&snap).unwrap();
        assert_eq!(json["rssMb"], 150);
        assert_eq!(json["jsHeapUsedMb"], 45.5);
        assert_eq!(json["processUptimeSecs"], 120);
        assert_eq!(json["sidecarRssMb"], 80);
    }

    /// 评审修复 #6：保留键读写各拒绝一次（server_token_hash 经命令面
    /// 不可读——防离线爆破；不可写——防库态凭据覆写接管）。
    #[tokio::test]
    async fn reserved_setting_key_read_and_write_are_rejected() {
        use crate::commands::ctx::EngineCtx;
        use crate::db::pool::{ConversationsPool, DbPool};
        use crate::services::agent_config::AgentConfigService;
        use crate::services::agent_bridge::AgentBridge;
        use crate::services::delegate_bridge::DelegateBridge;
        use crate::services::event_bus::EngineEvents;
        use crate::services::event_router::EventRouter;
        use crate::services::secret_store::SecretStore;
        use crate::services::sidecar::SidecarManager;
        use std::path::PathBuf;
        use std::sync::Arc;

        /// 测试桩：密钥存储（守卫在 SQL 之前，桩不被触达）
        struct StubSecrets;
        impl SecretStore for StubSecrets {
            fn save_secret(&self, _: &str, _: &str) -> Result<(), crate::error::AppError> { Ok(()) }
            fn load_secret(&self, _: &str) -> Result<Option<String>, crate::error::AppError> { Ok(None) }
            fn delete_secret(&self, _: &str) -> Result<(), crate::error::AppError> { Ok(()) }
        }
        /// 测试桩：事件总线（同上）
        struct StubBus;
        impl EngineEvents for StubBus {
            fn emit(&self, _: &str, _: serde_json::Value) -> Result<(), String> { Ok(()) }
        }

        let pool: DbPool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        // app_settings 表由迁移建；此处建最小表（命令守卫在 SQL 之前）
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let secrets: Arc<StubSecrets> = Arc::new(StubSecrets);
        let config = AgentConfigService::new(PathBuf::from("/tmp/test-opencode.json"));
        let ctx = EngineCtx {
            pool,
            conv_pool: ConversationsPool(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(),
            ),
            registry: Arc::new(crate::registry::ChatSessionRegistry::default()),
            agent_config: config.clone(),
            sidecar: Arc::new(tokio::sync::Mutex::new(SidecarManager::new(None, None))),
            agent_bridge: AgentBridge::new(0),
            event_router: Arc::new(EventRouter::new()),
            delegate_bridge: DelegateBridge::new(
                sqlx::sqlite::SqlitePoolOptions::new()
                    .connect("sqlite::memory:")
                    .await
                    .unwrap(),
                ConversationsPool(
                    sqlx::sqlite::SqlitePoolOptions::new()
                        .connect("sqlite::memory:")
                        .await
                        .unwrap(),
                ),
                "t".to_string(),
                None,
                config,
                PathBuf::from("/tmp/test-skills"),
                secrets.clone(),
            ),
            bus: Arc::new(StubBus),
            secrets,
            data_dir: PathBuf::from("/tmp"),
            opencode_workspace: PathBuf::from("/tmp"),
            skills_root: PathBuf::from("/tmp"),
            home_dir: PathBuf::from("/tmp"),
        };

        // 读拒绝：ValidationError（200 单键 map——错误白名单口径）
        let err = app_get_setting(&ctx, "server_token_hash".to_string())
            .await
            .expect_err("保留键读必须拒绝");
        match err {
            crate::error::AppError::ValidationError(msg) => {
                assert!(msg.contains("保留键不可读取"), "文案对齐: {}", msg)
            }
            other => panic!("读拒绝必须 ValidationError，实得 {:?}", other),
        }

        // 写拒绝：ValidationError
        let err = app_set_setting(
            &ctx,
            "server_token_hash".to_string(),
            "attacker-chosen-hash".to_string(),
        )
        .await
        .expect_err("保留键写必须拒绝");
        match err {
            crate::error::AppError::ValidationError(msg) => {
                assert!(msg.contains("保留键不可写入"), "文案对齐: {}", msg)
            }
            other => panic!("写拒绝必须 ValidationError，实得 {:?}", other),
        }
    }

    #[test]
    fn performance_snapshot_allows_null_js_heap_and_sidecar() {
        let snap = PerformanceSnapshot {
            rss_mb: 100,
            js_heap_used_mb: None,
            process_uptime_secs: 10,
            sidecar_rss_mb: None,
        };
        let json = serde_json::to_value(&snap).unwrap();
        assert!(json.get("jsHeapUsedMb").unwrap().is_null());
        assert!(json.get("sidecarRssMb").unwrap().is_null());
    }
}
