//! review 域命令体（Story 15.4 自壳 `commands/review.rs` 平移，业务逻辑零改动；
//! State/AppHandle 取值改 `&EngineCtx`，emit 改总线+常量，spawn 改 tokio；
//! 内联测试随迁）。

use std::collections::HashMap;

use crate::commands::ctx::EngineCtx;
use crate::db::pool::DbPool;
use crate::db::tasks;
use crate::error::AppError;
use crate::events::TASK_CLASSIFIED_EVENT;
use crate::models::task::{CreateTaskInput, Task, TaskOwnerType};
use crate::models::weekly_review::WeeklyReview;
use crate::services::review_generator;
use crate::services::task_classifier;

pub async fn review_get_latest(ctx: &EngineCtx) -> Result<Option<WeeklyReview>, AppError> {
    crate::db::weekly_reviews::get_latest_weekly_review(&ctx.pool).await
}

pub async fn review_get_by_week(
    ctx: &EngineCtx,
    week_start: String,
) -> Result<Option<WeeklyReview>, AppError> {
    crate::db::weekly_reviews::get_weekly_review_by_week_start(&ctx.pool, &week_start).await
}

pub async fn review_generate_now(ctx: &EngineCtx) -> Result<bool, AppError> {
    // Story 15.2：事件/密钥经 EngineEvents / SecretStore 接缝注入
    review_generator::generate_review_if_needed(
        &ctx.pool,
        &ctx.conv_pool,
        Some(&*ctx.bus),
        &*ctx.secrets,
    )
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigRockPlanItem {
    pub role_id: String,
    pub title: String,
}

/// 每个角色每周最多大石头数量（与 `db::tasks::MAX_BIG_ROCKS_PER_OWNER` 保持一致）
const MAX_BIG_ROCKS_PER_OWNER: i32 = 3;

pub async fn review_plan_bigrocks(
    ctx: &EngineCtx,
    items: Vec<BigRockPlanItem>,
) -> Result<Vec<Task>, AppError> {
    let pool: &DbPool = &ctx.pool;

    // 保存前按角色聚合校验大石头数量：任一角色（已有 + 本次新增）超限则整体拒绝，
    // 避免逐条创建时部分写入产生半成品数据。
    let mut new_counts: HashMap<&str, i32> = HashMap::new();
    for item in &items {
        *new_counts.entry(item.role_id.as_str()).or_insert(0) += 1;
    }
    for (role_id, adding) in &new_counts {
        let existing = tasks::count_big_rocks_by_owner(pool, "role", Some(role_id)).await?;
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
        let task = tasks::create_task(pool, &input).await?;
        let task_id = task.id.clone();
        let pool_clone = pool.clone();
        let bus_clone = ctx.bus.clone();
        let secret = ctx.secrets.clone();
        tokio::spawn(async move {
            let classified =
                match task_classifier::classify_and_persist(&pool_clone, &task_id, &*secret).await {
                    Ok(updated) => Some(updated),
                    Err(e) => {
                        tracing::warn!(task_id = %task_id, error = %e, "大石头创建后自动分类失败，保留默认 Q2");
                        tasks::get_active_task_pub(&pool_clone, &task_id).await.ok()
                    }
                };
            if let Some(updated) = classified {
                let _ = serde_json::to_value(updated)
                    .map_err(|e| e.to_string())
                    .and_then(|payload| bus_clone.emit(TASK_CLASSIFIED_EVENT, payload));
            }
        });
        created_tasks.push(task);
    }
    Ok(created_tasks)
}

pub async fn review_get_bigrock_suggestions(
    ctx: &EngineCtx,
) -> Result<Vec<review_generator::RoleBigRockSuggestions>, AppError> {
    review_generator::generate_bigrock_suggestions(&ctx.pool, &*ctx.secrets).await
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
