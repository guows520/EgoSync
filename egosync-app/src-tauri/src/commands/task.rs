use tauri::State;

use crate::db::pool::DbPool;
use crate::db::tasks;
use crate::error::AppError;
use crate::models::task::{CreateTaskInput, Task, UpdateTaskInput};
use crate::services::task_classifier;

/// 任务创建后自动分类完成（或降级）时推送的事件名。
/// 前端据此用最新任务替换卡片并清除「分类中」标记。
pub const TASK_CLASSIFIED_EVENT: &str = "task:classified";

#[tauri::command]
pub async fn task_create(
    input: CreateTaskInput,
    app_handle: tauri::AppHandle,
    pool: State<'_, DbPool>,
) -> Result<Task, AppError> {
    use tauri::Emitter;

    validate_title(&input.title)?;
    if let Some(quadrant) = input.quadrant.as_deref() {
        validate_quadrant(quadrant)?;
    }
    let user_chose_quadrant = input.quadrant.is_some();
    let task = tasks::create_task(&pool, &input).await?;

    // 用户没有显式选择 quadrant 时，后台异步触发自动分类，命令立即返回已创建任务（默认 Q2）。
    // 这样前端不会被 LLM 调用（最长 12s）阻塞，可立即关闭弹窗并展示「分类中」过渡态。
    // 分类完成或降级后通过 `task:classified` 事件推送最新任务；
    // 自动分类失败不回滚任务创建，task_classifier 内部会降级到 Q2 + 中文 reason 并写回。
    if !user_chose_quadrant {
        let pool = pool.inner().clone();
        let app = app_handle.clone();
        let task_id = task.id.clone();
        tauri::async_runtime::spawn(async move {
            let classified = match task_classifier::classify_and_persist(&pool, &task_id).await {
                Ok(updated) => Some(updated),
                Err(e) => {
                    tracing::warn!(task_id = %task_id, error = %e, "任务创建后自动分类失败，保留默认 Q2");
                    // 即便失败也回读当前任务并广播，使前端清除「分类中」标记。
                    tasks::get_active_task_pub(&pool, &task_id).await.ok()
                }
            };
            if let Some(updated) = classified {
                let _ = app.emit(TASK_CLASSIFIED_EVENT, updated);
            }
        });
    }
    Ok(task)
}

#[tauri::command]
pub async fn task_list_by_role(
    role_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<Task>, AppError> {
    tasks::list_tasks_by_role(&pool, &role_id).await
}

#[tauri::command]
pub async fn task_list_butler(pool: State<'_, DbPool>) -> Result<Vec<Task>, AppError> {
    tasks::list_butler_tasks(&pool).await
}

#[tauri::command]
pub async fn task_update(
    id: String,
    input: UpdateTaskInput,
    pool: State<'_, DbPool>,
) -> Result<Task, AppError> {
    if let Some(title) = input.title.as_deref() {
        validate_title(title)?;
    }
    if let Some(quadrant) = input.quadrant.as_deref() {
        validate_quadrant(quadrant)?;
    }
    // db 层已根据 input.quadrant.is_some() 设置 manual_override。
    tasks::update_task(&pool, &id, &input).await
}

#[tauri::command]
pub async fn task_delete(id: String, pool: State<'_, DbPool>) -> Result<(), AppError> {
    tasks::soft_delete_task(&pool, &id).await
}

#[tauri::command]
pub async fn task_reorder(
    task_ids: Vec<String>,
    pool: State<'_, DbPool>,
) -> Result<(), AppError> {
    tasks::reorder_tasks(&pool, &task_ids).await
}

#[tauri::command]
pub async fn task_toggle_complete(
    task_id: String,
    is_completed: bool,
    pool: State<'_, DbPool>,
) -> Result<Task, AppError> {
    tasks::set_task_completion(&pool, &task_id, is_completed).await
}

/// Story 3.5：前端启动 / 打开任务面板时主动触发一次 Q2 保护检查。
/// 返回本次被标记 `at_risk` 的任务数量。命令层只薄封装，重算逻辑在 service 内。
#[tauri::command]
pub async fn task_check_protection_status(pool: State<'_, DbPool>) -> Result<u64, AppError> {
    crate::services::task_protection_watch::recompute_protection_status(&pool).await
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
