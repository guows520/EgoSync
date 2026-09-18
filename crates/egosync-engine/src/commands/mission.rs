//! mission 域命令体（Story 15.4 自壳 `commands/mission.rs` 平移，业务逻辑零改动；
//! State/密钥取值改 `&EngineCtx`；内联测试随迁）。

use crate::commands::ctx::EngineCtx;
use crate::db::mission as mission_db;
use crate::error::AppError;
use crate::models::mission::Mission;
use crate::services::mission_inferrer::{self, InferredValues, InferenceEligibility};

fn validate_format(format: &str) -> Result<(), AppError> {
    if format != "free" && format != "structured" {
        return Err(AppError::ValidationError(format!(
            "无效的 format 值: {}，只允许 'free' 或 'structured'",
            format
        )));
    }
    Ok(())
}

pub async fn mission_get(ctx: &EngineCtx) -> Result<Option<Mission>, AppError> {
    mission_db::get_mission(&ctx.pool).await
}

pub async fn mission_update(
    ctx: &EngineCtx,
    content: Option<String>,
    format: String,
) -> Result<Mission, AppError> {
    validate_format(&format)?;
    mission_db::upsert_mission(&ctx.pool, content.as_deref(), &format).await
}

pub async fn mission_infer(ctx: &EngineCtx) -> Result<Option<InferredValues>, AppError> {
    // Story 15.2：密钥经 SecretStore 接缝注入
    let outcome =
        mission_inferrer::infer_values(&ctx.pool, &ctx.conv_pool, &*ctx.secrets).await?;
    Ok(outcome.values)
}

pub async fn mission_infer_eligibility(
    ctx: &EngineCtx,
) -> Result<InferenceEligibility, AppError> {
    mission_inferrer::check_eligibility(&ctx.pool, &ctx.conv_pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_format_free() {
        assert!(validate_format("free").is_ok());
    }

    #[test]
    fn test_validate_format_structured() {
        assert!(validate_format("structured").is_ok());
    }

    #[test]
    fn test_validate_format_invalid_returns_validation_error() {
        let result = validate_format("invalid");
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[test]
    fn test_validate_format_empty_returns_validation_error() {
        let result = validate_format("");
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }
}
