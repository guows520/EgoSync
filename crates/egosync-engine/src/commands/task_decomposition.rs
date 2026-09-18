//! task_decomposition 域命令体（Story 15.4 自壳 `commands/task_decomposition.rs`
//! 平移，业务逻辑零改动；State/AppHandle 取值改 `&EngineCtx`，emit 改总线+常量）。
//! 其 service（services/task_decomposition.rs）随迁 engine（SecretStore 接缝适配）。

use crate::commands::ctx::EngineCtx;
use crate::error::AppError;
use crate::events::TASK_TOOL_ACTION_EVENT;
use crate::models::task_decomposition::{
    TaskDecompositionProposal, TaskDecompositionProposalWithRole,
};
use crate::services::task_decomposition;

pub async fn task_decomposition_list_pending(
    ctx: &EngineCtx,
    conversation_id: String,
) -> Result<Vec<TaskDecompositionProposalWithRole>, AppError> {
    task_decomposition::list_pending(&ctx.pool, &conversation_id).await
}

pub async fn task_decomposition_accept(
    ctx: &EngineCtx,
    id: String,
) -> Result<TaskDecompositionProposal, AppError> {
    let proposal = task_decomposition::accept(&ctx.pool, &ctx.secrets, &id).await?;
    // 原 emit 错误经 `let _ =` 丢弃（静默语义保持），事件名收编为常量
    let _ = ctx
        .bus
        .emit(TASK_TOOL_ACTION_EVENT, serde_json::json!({ "action": "create" }));
    Ok(proposal)
}

pub async fn task_decomposition_keep_single(
    ctx: &EngineCtx,
    id: String,
) -> Result<TaskDecompositionProposal, AppError> {
    let proposal = task_decomposition::keep_single(&ctx.pool, &ctx.secrets, &id).await?;
    let _ = ctx
        .bus
        .emit(TASK_TOOL_ACTION_EVENT, serde_json::json!({ "action": "create" }));
    Ok(proposal)
}
