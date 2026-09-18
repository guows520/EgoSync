//! Story 15.4：随 task_decomposition 命令体入 engine（原壳
//! `services/task_decomposition.rs` 逐字节平移）；密钥读取改经注入的
//! SecretStore 接缝（原 KeyringSecretStore 即席构造由调用方传入）。

use std::sync::Arc;

use crate::db::pool::DbPool;
use crate::db::task_decomposition;
use crate::error::AppError;
use crate::models::task_decomposition::{
    TaskDecompositionProposal, TaskDecompositionProposalWithRole,
};
use crate::services::secret_store::SecretStore;

pub async fn list_pending(
    pool: &DbPool,
    conversation_id: &str,
) -> Result<Vec<TaskDecompositionProposalWithRole>, AppError> {
    task_decomposition::list_pending_by_conversation(pool, conversation_id).await
}

pub async fn accept(
    pool: &DbPool,
    secrets: &Arc<dyn SecretStore>,
    id: &str,
) -> Result<TaskDecompositionProposal, AppError> {
    let (tasks, created) = task_decomposition::accept_proposal_once(pool, id).await?;
    if created {
        for task in tasks {
            let pool = pool.clone();
            // Story 15.2：unit struct 即席构造零成本（接缝签名需 'static 移入闭包）
            let secret = secrets.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::services::task_classifier::classify_and_persist(&pool, &task.id, &*secret).await
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
    secrets: &Arc<dyn SecretStore>,
    id: &str,
) -> Result<TaskDecompositionProposal, AppError> {
    let (task, created) = task_decomposition::keep_single_proposal_once(pool, id).await?;
    if created {
        let classify_pool = pool.clone();
        // Story 15.2：unit struct 即席构造零成本（接缝签名需 'static 移入闭包）
        let secret = secrets.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::services::task_classifier::classify_and_persist(&classify_pool, &task.id, &*secret).await
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
