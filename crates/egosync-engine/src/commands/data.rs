//! data 域命令体（Story 15.4 自壳 `commands/data.rs` 平移——仅 web-ok 的
//! data_destroy；data_export / pick_import_file / data_import 为 desktop-only
//! 留壳不迁）。业务逻辑零改动；State/AppHandle 取值改 `&EngineCtx` 路径注入。

use crate::commands::ctx::EngineCtx;
use crate::error::AppError;
use crate::services::data_export::destroy_all_data;

pub async fn data_destroy(ctx: &EngineCtx) -> Result<(), AppError> {
    let app_data_dir = &ctx.data_dir;

    // Story 15.1 接缝一：keyring 密钥删除经注入的宿主侧 SecretStore
    destroy_all_data(&ctx.pool, &ctx.conv_pool, &*ctx.secrets, app_data_dir).await
}
