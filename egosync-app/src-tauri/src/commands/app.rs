//! app 域命令（Story 15.4）：9 条 web-ok 命令体已迁引擎
//! （`egosync_engine::commands::app`），壳侧薄化为 wrapper（内联测试随迁）；
//! 2 条 perf-test 门控命令（app_emit_test_stream / app_seed_perf_data）留壳。

use std::sync::Arc;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::State;
#[cfg(feature = "perf-test")]
use tauri::{AppHandle, Emitter};

use crate::error::AppError;
#[cfg(feature = "perf-test")]
use crate::models::chat::{StreamPayload, STREAM_PHASE_ANSWERING};
use crate::models::role::{ButlerSkillsConfig, UpdateRoleSkillsInput};

#[tauri::command]
pub async fn app_is_first_launch(ctx: State<'_, Arc<EngineCtx>>) -> Result<bool, AppError> {
    egosync_engine::commands::app::app_is_first_launch(&ctx).await
}

#[tauri::command]
pub async fn app_complete_onboarding(ctx: State<'_, Arc<EngineCtx>>) -> Result<(), AppError> {
    egosync_engine::commands::app::app_complete_onboarding(&ctx).await
}

#[tauri::command]
pub async fn app_is_llm_configured(ctx: State<'_, Arc<EngineCtx>>) -> Result<bool, AppError> {
    egosync_engine::commands::app::app_is_llm_configured(&ctx).await
}

#[tauri::command]
pub async fn app_get_butler_skills(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<ButlerSkillsConfig, AppError> {
    egosync_engine::commands::app::app_get_butler_skills(&ctx).await
}

#[tauri::command]
pub async fn app_update_butler_skills(
    ctx: State<'_, Arc<EngineCtx>>,
    input: UpdateRoleSkillsInput,
) -> Result<ButlerSkillsConfig, AppError> {
    egosync_engine::commands::app::app_update_butler_skills(&ctx, input).await
}

#[tauri::command]
pub async fn app_sidecar_status(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<crate::models::agent::SidecarStatus, AppError> {
    egosync_engine::commands::app::app_sidecar_status(&ctx).await
}

#[tauri::command]
pub async fn app_get_setting(
    ctx: State<'_, Arc<EngineCtx>>,
    key: String,
) -> Result<Option<String>, AppError> {
    egosync_engine::commands::app::app_get_setting(&ctx, key).await
}

#[tauri::command]
pub async fn app_set_setting(
    ctx: State<'_, Arc<EngineCtx>>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    egosync_engine::commands::app::app_set_setting(&ctx, key, value).await
}

// ── Performance benchmark commands (Story 8.4) ──

// PerformanceSnapshot 结构与 app_performance_snapshot 命令体已迁引擎
// （egosync_engine::commands::app），壳侧 wrapper 转发。
pub use egosync_engine::commands::app::PerformanceSnapshot;

#[tauri::command]
pub async fn app_performance_snapshot(
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<PerformanceSnapshot, AppError> {
    egosync_engine::commands::app::app_performance_snapshot(&ctx).await
}

/// 注入 mock llm:stream 事件用于流式渲染延迟测量（AC3）。
/// **仅 perf-test feature 构建可用**，生产构建排除此 command。
#[cfg(feature = "perf-test")]
#[tauri::command]
pub async fn app_emit_test_stream(
    tokens: Vec<String>,
    conversation_id: Option<String>,
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

    let conv_id = conversation_id.unwrap_or_else(|| "perf-test".to_string());

    for token in &tokens {
        if let Err(e) = app_handle.emit(
            // Story 15.3：发射源唯一化——字面量改引 engine 常量（payload 不变）
            crate::events::LLM_STREAM_EVENT,
            StreamPayload {
                conversation_id: conv_id.clone(),
                token: token.clone(),
                done: false,
                thinking: false,
                message_id: None,
                phase: Some(STREAM_PHASE_ANSWERING.to_string()),
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
        crate::events::LLM_STREAM_EVENT,
        StreamPayload {
            conversation_id: conv_id,
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
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<(), AppError> {
    use crate::db::pool::DbPool;
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

    let pool: &DbPool = &ctx.pool;

    // 创建角色
    let mut role_ids: Vec<String> = Vec::new();
    for i in 0..role_count {
        let input = CreateRoleInput {
            name: format!("perf-role-{}", i),
            icon: Some("🎯".to_string()),
            color: Some("#6366F1".to_string()),
            goal: Some(format!("性能测试角色 {}", i)),
        };
        let role: Role = crate::db::roles::create_role(pool, &input).await?;
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
