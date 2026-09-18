//! task 域命令体（Story 15.4 自壳 `commands/task.rs` 平移，业务逻辑零改动；
//! State/AppHandle 取值改 `&EngineCtx`，emit 改总线+events 常量
//! （15.4 收编），spawn 改 tokio；内联测试随迁）。

use crate::commands::ctx::EngineCtx;
use crate::db::pool::DbPool;
use crate::db::tasks;
use crate::error::AppError;
use crate::events::{
    TASK_CLASSIFIED_EVENT, TASK_CREATED_EVENT, TASK_DELETED_EVENT, TASK_REORDERED_EVENT,
    TASK_UPDATED_EVENT,
};
use crate::models::task::{CreateTaskInput, CrossRoleTask, Task, UpdateTaskInput};
use crate::services::event_bus::EngineEvents;
use crate::services::task_classifier;

/// Story 13.1：命令层补发写事件（快照引擎订阅触发 STATE_DELTA；payload
/// 沿用域对象供前端自由消费——引擎只看事件名不看 payload）。
/// Story 15.4：发射改经注入的 EngineEvents 总线 + engine events 常量
/// （事件名值不变；强类型 payload 机械改写为 serde_json::to_value）。
fn emit_task_event(bus: &dyn EngineEvents, event: &str, payload: &impl serde::Serialize) {
    if let Err(e) = serde_json::to_value(payload)
        .map_err(|e| e.to_string())
        .and_then(|payload| bus.emit(event, payload))
    {
        tracing::warn!(event = event, error = %e, "task 写事件发射失败");
    }
}

pub async fn task_create(ctx: &EngineCtx, input: CreateTaskInput) -> Result<Task, AppError> {
    let (pool, bus): (&DbPool, &dyn EngineEvents) = (&ctx.pool, &*ctx.bus);

    validate_title(&input.title)?;
    if let Some(quadrant) = input.quadrant.as_deref() {
        validate_quadrant(quadrant)?;
    }
    let user_chose_quadrant = input.quadrant.is_some();
    let task = tasks::create_task(pool, &input).await?;

    // 用户没有显式选择 quadrant 时，后台异步触发自动分类，命令立即返回已创建任务（默认 Q2）。
    // 这样前端不会被 LLM 调用（最长 12s）阻塞，可立即关闭弹窗并展示「分类中」过渡态。
    // 分类完成或降级后通过 `task:classified` 事件推送最新任务；
    // 自动分类失败不回滚任务创建，task_classifier 内部会降级到 Q2 + 中文 reason 并写回。
    if !user_chose_quadrant {
        let pool = pool.clone();
        let bus = ctx.bus.clone();
        let task_id = task.id.clone();
        let secret = ctx.secrets.clone();
        tokio::spawn(async move {
            let classified =
                match task_classifier::classify_and_persist(&pool, &task_id, &*secret).await {
                Ok(updated) => Some(updated),
                Err(e) => {
                    tracing::warn!(task_id = %task_id, error = %e, "任务创建后自动分类失败，保留默认 Q2");
                    // 即便失败也回读当前任务并广播，使前端清除「分类中」标记。
                    tasks::get_active_task_pub(&pool, &task_id).await.ok()
                }
            };
            if let Some(updated) = classified {
                let _ = serde_json::to_value(updated)
                    .map_err(|e| e.to_string())
                    .and_then(|payload| bus.emit(TASK_CLASSIFIED_EVENT, payload));
            }
        });
    }
    emit_task_event(bus, TASK_CREATED_EVENT, &task);
    Ok(task)
}

pub async fn task_list_by_role(ctx: &EngineCtx, role_id: String) -> Result<Vec<Task>, AppError> {
    tasks::list_tasks_by_role(&ctx.pool, &role_id).await
}

pub async fn task_list_butler(ctx: &EngineCtx) -> Result<Vec<Task>, AppError> {
    tasks::list_butler_tasks(&ctx.pool).await
}

pub async fn task_list_all(
    ctx: &EngineCtx,
    quadrant: Option<String>,
    is_big_rock: Option<bool>,
) -> Result<Vec<CrossRoleTask>, AppError> {
    if let Some(ref q) = quadrant {
        validate_quadrant(q)?;
    }
    tasks::list_all_tasks(&ctx.pool, quadrant.as_deref(), is_big_rock).await
}

pub async fn task_update(
    ctx: &EngineCtx,
    id: String,
    input: UpdateTaskInput,
) -> Result<Task, AppError> {
    if let Some(title) = input.title.as_deref() {
        validate_title(title)?;
    }
    if let Some(quadrant) = input.quadrant.as_deref() {
        validate_quadrant(quadrant)?;
    }
    // db 层已根据 input.quadrant.is_some() 设置 manual_override。
    let task = tasks::update_task(&ctx.pool, &id, &input).await?;
    emit_task_event(&*ctx.bus, TASK_UPDATED_EVENT, &task);
    Ok(task)
}

pub async fn task_delete(ctx: &EngineCtx, id: String) -> Result<(), AppError> {
    tasks::soft_delete_task(&ctx.pool, &id).await?;
    emit_task_event(&*ctx.bus, TASK_DELETED_EVENT, &id);
    Ok(())
}

pub async fn task_reorder(ctx: &EngineCtx, task_ids: Vec<String>) -> Result<(), AppError> {
    tasks::reorder_tasks(&ctx.pool, &task_ids).await?;
    emit_task_event(&*ctx.bus, TASK_REORDERED_EVENT, &task_ids);
    Ok(())
}

pub async fn task_toggle_complete(
    ctx: &EngineCtx,
    task_id: String,
    is_completed: bool,
) -> Result<Task, AppError> {
    let task = tasks::set_task_completion(&ctx.pool, &task_id, is_completed).await?;
    // 评审 B9：toggle 双向（完成/取消完成）——事件名统一为 task:updated，
    // 取消完成时发 task:completed 会误导消费方对状态的语义判断。
    emit_task_event(&*ctx.bus, TASK_UPDATED_EVENT, &task);
    Ok(task)
}

/// Story 3.5：前端启动 / 打开任务面板时主动触发一次 Q2 保护检查。
/// 返回本次被标记 `at_risk` 的任务数量。命令层只薄封装，重算逻辑在 service 内。
pub async fn task_check_protection_status(ctx: &EngineCtx) -> Result<u64, AppError> {
    crate::services::task_protection_watch::recompute_protection_status(&ctx.pool).await
}

/// Story 4.6：手动触发 Q2 保护提醒检查。
/// 前端可在打开管家视角时主动调用，确保 at_risk 状态即时反映。
/// 事件经注入的事件总线 emit `q2:reminder`，与调度器路径一致，
/// 保证手动触发生成的提醒也能即时刷新管家对话。
pub async fn task_check_q2_reminders(ctx: &EngineCtx) -> Result<(), AppError> {
    // Story 15.2：emit 改经注入的事件总线（EngineEvents 接缝），
    // 语义与调度器路径一致，手动触发的提醒同样即时刷新管家对话。
    crate::services::q2_protection_reminder::check_and_generate_reminders(
        &ctx.pool,
        &ctx.conv_pool,
        Some(&*ctx.bus),
    )
    .await
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::ValidationError("任务标题不能为空".to_string()));
    }
    Ok(())
}

fn validate_quadrant(quadrant: &str) -> Result<(), AppError> {
    if !matches!(quadrant, "Q1" | "Q2" | "Q3" | "Q4") {
        return Err(AppError::ValidationError("四象限分类无效".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_title_rejects_blank_task_title() {
        let result = validate_title("  ");
        assert!(matches!(result, Err(AppError::ValidationError(message)) if message == "任务标题不能为空"));
    }

    #[test]
    fn validate_quadrant_rejects_unknown_value() {
        let result = validate_quadrant("Q5");
        assert!(matches!(result, Err(AppError::ValidationError(message)) if message == "四象限分类无效"));
    }
}
