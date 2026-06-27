use tauri::{AppHandle, State};

use crate::db::pool::{ConversationsPool, DbPool};
use crate::db::tasks;
use crate::error::AppError;
use crate::models::task::{CreateTaskInput, Task, TaskOwnerType};
use crate::models::weekly_review::WeeklyReview;
use crate::services::review_generator;
use crate::services::task_classifier;

#[tauri::command]
pub async fn review_get_latest(pool: State<'_, DbPool>) -> Result<Option<WeeklyReview>, AppError> {
    crate::db::weekly_reviews::get_latest_weekly_review(&pool).await
}

#[tauri::command]
pub async fn review_get_by_week(
    pool: State<'_, DbPool>,
    week_start: String,
) -> Result<Option<WeeklyReview>, AppError> {
    crate::db::weekly_reviews::get_weekly_review_by_week_start(&pool, &week_start).await
}

#[tauri::command]
pub async fn review_generate_now(
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
    app_handle: AppHandle,
) -> Result<bool, AppError> {
    review_generator::generate_review_if_needed(&pool, &conv_pool, Some(&app_handle)).await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigRockPlanItem {
    pub role_id: String,
    pub title: String,
}

/// 每个角色每周最多大石头数量（与 `db::tasks::MAX_BIG_ROCKS_PER_OWNER` 保持一致）
const MAX_BIG_ROCKS_PER_OWNER: i32 = 3;

#[tauri::command]
pub async fn review_plan_bigrocks(
    items: Vec<BigRockPlanItem>,
    app_handle: AppHandle,
    pool: State<'_, DbPool>,
) -> Result<Vec<Task>, AppError> {
    use std::collections::HashMap;
    use tauri::Emitter;

    // 保存前按角色聚合校验大石头数量：任一角色（已有 + 本次新增）超限则整体拒绝，
    // 避免逐条创建时部分写入产生半成品数据。
    let mut new_counts: HashMap<&str, i32> = HashMap::new();
    for item in &items {
        *new_counts.entry(item.role_id.as_str()).or_insert(0) += 1;
    }
    for (role_id, adding) in &new_counts {
        let existing = tasks::count_big_rocks_by_owner(&pool, "role", Some(role_id)).await?;
        if existing + adding > MAX_BIG_ROCKS_PER_OWNER {
            return Err(AppError::ValidationError(format!(
                "每个角色每周最多 {} 个大石头，当前角色已有 {} 个，无法再添加 {} 个",
                MAX_BIG_ROCKS_PER_OWNER, existing, adding
            )));
        }
    }

    let mut created_tasks = Vec::with_capacity(items.len());
    for item in &items {
        let input = CreateTaskInput {
            owner_type: Some(TaskOwnerType::Role),
            role_id: Some(item.role_id.clone()),
            title: item.title.clone(),
            deadline: None,
            quadrant: None,
            is_big_rock: Some(true),
        };
        let task = tasks::create_task(&pool, &input).await?;
        let task_id = task.id.clone();
        let pool_clone = pool.inner().clone();
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let classified = match task_classifier::classify_and_persist(&pool_clone, &task_id).await {
                Ok(updated) => Some(updated),
                Err(e) => {
                    tracing::warn!(task_id = %task_id, error = %e, "大石头创建后自动分类失败，保留默认 Q2");
                    tasks::get_active_task_pub(&pool_clone, &task_id).await.ok()
                }
            };
            if let Some(updated) = classified {
                let _ = app_clone.emit(crate::commands::task::TASK_CLASSIFIED_EVENT, updated);
            }
        });
        created_tasks.push(task);
    }
    Ok(created_tasks)
}

#[tauri::command]
pub async fn review_get_bigrock_suggestions(
    pool: State<'_, DbPool>,
) -> Result<Vec<review_generator::RoleBigRockSuggestions>, AppError> {
    review_generator::generate_bigrock_suggestions(&pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_rock_plan_item_deserializes_camel_case() {
        let json = r#"{"roleId":"r1","title":"Q3路线图定稿"}"#;
        let item: BigRockPlanItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.role_id, "r1");
        assert_eq!(item.title, "Q3路线图定稿");
    }
}
