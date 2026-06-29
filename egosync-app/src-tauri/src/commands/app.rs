use std::sync::Arc;

use tauri::State;
#[cfg(feature = "perf-test")]
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::db::app_settings;
use crate::db::pool::DbPool;
use crate::db::settings;
use crate::error::AppError;
use crate::models::agent::SidecarStatus;
#[cfg(feature = "perf-test")]
use crate::models::chat::StreamPayload;
use crate::models::role::{ButlerSkillsConfig, UpdateRoleSkillsInput};
use crate::services::agent_config::AgentConfigService;
use crate::services::sidecar::SidecarManager;

#[tauri::command]
pub async fn app_is_first_launch(pool: State<'_, DbPool>) -> Result<bool, AppError> {
    let value = app_settings::get_setting(&pool, "onboarding_completed").await?;
    Ok(value.as_deref() != Some("true"))
}

#[tauri::command]
pub async fn app_complete_onboarding(pool: State<'_, DbPool>) -> Result<(), AppError> {
    app_settings::set_setting(&pool, "onboarding_completed", "true").await
}

#[tauri::command]
pub async fn app_is_llm_configured(pool: State<'_, DbPool>) -> Result<bool, AppError> {
    let count = settings::count_llm_configs(&pool).await?;
    Ok(count > 0)
}

#[tauri::command]
pub async fn app_get_butler_skills(
    pool: State<'_, DbPool>,
) -> Result<ButlerSkillsConfig, AppError> {
    crate::services::butler_config::get_butler_skills(&pool).await
}

#[tauri::command]
pub async fn app_update_butler_skills(
    input: UpdateRoleSkillsInput,
    pool: State<'_, DbPool>,
    agent_config: State<'_, AgentConfigService>,
) -> Result<ButlerSkillsConfig, AppError> {
    let skills = crate::services::butler_config::set_butler_skills(&pool, &input).await?;
    let registry = crate::db::skills::list_skills(&pool).await.unwrap_or_default();
    if let Err(e) = agent_config.sync_butler_skills_with_registry(&skills, &registry) {
        tracing::warn!("opencode sync (butler_update_skills) failed: {}", e);
    }
    Ok(skills)
}

#[tauri::command]
pub async fn app_sidecar_status(
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<SidecarStatus, AppError> {
    let mut mgr = sidecar.lock().await;
    Ok(SidecarStatus {
        running: mgr.is_running(),
        port: mgr.port(),
        uptime_secs: mgr.uptime_secs(),
    })
}

#[tauri::command]
pub async fn app_get_setting(
    key: String,
    pool: State<'_, DbPool>,
) -> Result<Option<String>, AppError> {
    app_settings::get_setting(&pool, &key).await
}

#[tauri::command]
pub async fn app_set_setting(
    key: String,
    value: String,
    pool: State<'_, DbPool>,
) -> Result<(), AppError> {
    app_settings::set_setting(&pool, &key, &value).await
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
#[tauri::command]
pub async fn app_performance_snapshot(
    sidecar: State<'_, Arc<Mutex<SidecarManager>>>,
) -> Result<PerformanceSnapshot, AppError> {
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

/// 注入 mock llm:stream 事件用于流式渲染延迟测量（AC3）。
/// **仅 perf-test feature 构建可用**，生产构建排除此 command。
#[cfg(feature = "perf-test")]
#[tauri::command]
pub async fn app_emit_test_stream(
    tokens: Vec<String>,
    app_handle: AppHandle,
) -> Result<(), AppError> {
    const MAX_PERF_TOKENS: usize = 1_000;
    if tokens.len() > MAX_PERF_TOKENS {
        return Err(AppError::ValidationError(format!(
            "tokens count {} exceeds max {}",
            tokens.len(),
            MAX_PERF_TOKENS
        )));
    }

    for token in &tokens {
        if let Err(e) = app_handle.emit(
            "llm:stream",
            StreamPayload {
                conversation_id: "perf-test".to_string(),
                token: token.clone(),
                done: false,
                thinking: false,
                message_id: None,
                phase: Some("answering".to_string()),
                status_text: None,
                tool_name: None,
                process_event: None,
            },
        ) {
            tracing::warn!("app_emit_test_stream: failed to emit token event: {}", e);
        }
        // 10ms 间隔模拟真实流式节奏
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    // 发射 done 信号
    if let Err(e) = app_handle.emit(
        "llm:stream",
        StreamPayload {
            conversation_id: "perf-test".to_string(),
            token: String::new(),
            done: true,
            thinking: false,
            message_id: None,
            phase: None,
            status_text: None,
            tool_name: None,
            process_event: None,
        },
    ) {
        tracing::warn!("app_emit_test_stream: failed to emit done event: {}", e);
    }
    Ok(())
}

/// 注入测试数据用于稳态内存测量（AC5）。
/// 直接通过 DB 层插入角色和记忆，绕过 LLM 提取流程。
/// **仅 perf-test feature 构建可用**，生产构建排除此 command。
#[cfg(feature = "perf-test")]
#[tauri::command]
pub async fn app_seed_perf_data(
    role_count: u32,
    memory_count: u32,
    pool: State<'_, DbPool>,
) -> Result<(), AppError> {
    use crate::db::roles;
    use crate::models::role::{CreateRoleInput, Role};

    const MAX_PERF_ROLES: u32 = 1_000;
    const MAX_PERF_MEMORIES: u32 = 10_000;

    if role_count == 0 {
        return Err(AppError::ValidationError(
            "role_count must be > 0".to_string(),
        ));
    }
    if role_count > MAX_PERF_ROLES {
        return Err(AppError::ValidationError(format!(
            "role_count {} exceeds max {}",
            role_count, MAX_PERF_ROLES
        )));
    }
    if memory_count > MAX_PERF_MEMORIES {
        return Err(AppError::ValidationError(format!(
            "memory_count {} exceeds max {}",
            memory_count, MAX_PERF_MEMORIES
        )));
    }

    // 创建角色
    let mut role_ids: Vec<String> = Vec::new();
    for i in 0..role_count {
        let input = CreateRoleInput {
            name: format!("perf-role-{}", i),
            icon: Some("🎯".to_string()),
            color: Some("#6366F1".to_string()),
            goal: Some(format!("性能测试角色 {}", i)),
        };
        let role: Role = roles::create_role(&pool, &input).await?;
        role_ids.push(role.id);
    }

    // 插入记忆（直接 SQL，绕过 extract 流程）
    let memories_per_role = memory_count / role_count.max(1);
    let mut total_inserted = 0u32;
    for role_id in &role_ids {
        for j in 0..memories_per_role {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT OR IGNORE INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(&id)
            .bind(role_id)
            .bind("fact")
            .bind(format!("perf-test memory #{}", j))
            .bind("perf-test-conv")
            .bind("[]")
            .execute(&*pool)
            .await
            .map_err(|e| AppError::DbError(format!("perf seed memory failed: {}", e)))?;
            total_inserted += 1;
        }
    }
    // 如果 memory_count 不能均分，补齐剩余
    let remainder = memory_count.saturating_sub(total_inserted);
    if remainder > 0 && !role_ids.is_empty() {
        for _ in 0..remainder {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT OR IGNORE INTO memories (id, role_id, category, content, source_conversation_id, source_message_ids) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(&id)
            .bind(&role_ids[0])
            .bind("fact")
            .bind("perf-test memory remainder")
            .bind("perf-test-conv")
            .bind("[]")
            .execute(&*pool)
            .await
            .map_err(|e| AppError::DbError(format!("perf seed memory failed: {}", e)))?;
        }
    }

    tracing::info!(
        "perf-test: seeded {} roles + {} memories",
        role_count,
        memory_count
    );
    Ok(())
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
