//! memory 域命令体（Story 15.4 自壳 `commands/memory.rs` 平移，
//! 业务逻辑零改动；`State` 取值改 `&EngineCtx`）。

use crate::commands::ctx::EngineCtx;
use crate::error::AppError;
use crate::models::memory::{Memory, MemorySourceMessage};
use crate::services::memory_query;

pub async fn memory_list(
    ctx: &EngineCtx,
    role_id: Option<String>,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    memory_query::list_role_memories(
        &ctx.pool,
        role_id.as_deref(),
        category.as_deref(),
        limit,
        offset,
    )
    .await
}

pub async fn memory_list_all(
    ctx: &EngineCtx,
    category: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memory>, AppError> {
    memory_query::list_all_memories(&ctx.pool, category.as_deref(), limit, offset).await
}

pub async fn memory_count(
    ctx: &EngineCtx,
    role_id: Option<String>,
    include_role_memories: Option<bool>,
    category: Option<String>,
) -> Result<usize, AppError> {
    memory_query::count_memories(
        &ctx.pool,
        role_id.as_deref(),
        include_role_memories.unwrap_or(false),
        category.as_deref(),
    )
    .await
}

pub async fn memory_get_source_messages(
    ctx: &EngineCtx,
    memory_id: String,
) -> Result<Vec<MemorySourceMessage>, AppError> {
    memory_query::get_source_messages(&ctx.pool, &ctx.conv_pool, &memory_id).await
}

pub async fn memory_delete(ctx: &EngineCtx, memory_id: String) -> Result<(), AppError> {
    memory_query::delete_memory(&ctx.pool, &memory_id).await
}
