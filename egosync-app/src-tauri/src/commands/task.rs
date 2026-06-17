use tauri::State;

use crate::db::pool::DbPool;
use crate::db::tasks;
use crate::error::AppError;
use crate::models::task::{CreateTaskInput, Task, UpdateTaskInput};

#[tauri::command]
pub async fn task_create(
    input: CreateTaskInput,
    pool: State<'_, DbPool>,
) -> Result<Task, AppError> {
    validate_title(&input.title)?;
    if let Some(quadrant) = input.quadrant.as_deref() {
        validate_quadrant(quadrant)?;
    }
    tasks::create_task(&pool, &input).await
}

#[tauri::command]
pub async fn task_list_by_role(
    role_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<Task>, AppError> {
    tasks::list_tasks_by_role(&pool, &role_id).await
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
