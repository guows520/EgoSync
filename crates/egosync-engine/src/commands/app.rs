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
    app_settings::get_setting(&ctx.pool, &key).await
}

pub async fn app_set_setting(ctx: &EngineCtx, key: String, value: String) -> Result<(), AppError> {
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
