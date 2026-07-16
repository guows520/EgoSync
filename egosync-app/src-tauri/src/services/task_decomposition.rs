use crate::db::pool::DbPool;
use crate::db::task_decomposition;
use crate::error::AppError;
use crate::models::task_decomposition::{
    TaskDecompositionProposal, TaskDecompositionProposalWithRole,
};

pub async fn list_pending(
    pool: &DbPool,
    conversation_id: &str,
) -> Result<Vec<TaskDecompositionProposalWithRole>, AppError> {
    task_decomposition::list_pending_by_conversation(pool, conversation_id).await
}

pub async fn accept(
    pool: &DbPool,
    id: &str,
) -> Result<TaskDecompositionProposal, AppError> {
    let (tasks, created) = task_decomposition::accept_proposal_once(pool, id).await?;
    if created {
        for task in tasks {
            let pool = pool.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::services::task_classifier::classify_and_persist(&pool, &task.id).await
                {
                    tracing::warn!(
                        "拆分任务自动分类失败，保留默认 Q2: task_id={} error={}",
                        task.id,
                        e
                    );
                }
            });
        }
    }
    task_decomposition::get_proposal(pool, id).await
}

pub async fn keep_single(
    pool: &DbPool,
    id: &str,
) -> Result<TaskDecompositionProposal, AppError> {
    let (task, created) = task_decomposition::keep_single_proposal_once(pool, id).await?;
    if created {
        let classify_pool = pool.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::services::task_classifier::classify_and_persist(&classify_pool, &task.id).await
            {
                tracing::warn!(
                    "单任务自动分类失败，保留默认 Q2: task_id={} error={}",
                    task.id,
                    e
                );
            }
        });
    }
    task_decomposition::get_proposal(pool, id).await
}
