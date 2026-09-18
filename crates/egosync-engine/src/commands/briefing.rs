//! briefing 域命令体（Story 15.4 自壳 `commands/briefing.rs` 平移，
//! 业务逻辑零改动；State/总线/密钥取值改 `&EngineCtx`）。

use crate::commands::ctx::EngineCtx;
use crate::error::AppError;
use crate::models::briefing::Briefing;
use crate::services::briefing_generator;

pub async fn briefing_get_latest(ctx: &EngineCtx) -> Result<Option<Briefing>, AppError> {
    crate::db::briefings::get_latest_briefing(&ctx.pool).await
}

pub async fn briefing_generate_now(ctx: &EngineCtx) -> Result<bool, AppError> {
    // Story 15.2：事件/密钥经 EngineEvents / SecretStore 接缝注入
    briefing_generator::generate_briefing_if_needed(
        &ctx.pool,
        &ctx.conv_pool,
        Some(&*ctx.bus),
        &*ctx.secrets,
    )
    .await
}
