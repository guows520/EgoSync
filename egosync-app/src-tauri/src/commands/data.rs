//! data 域命令（Story 15.4）：data_destroy 命令体已迁引擎，壳侧薄化为
//! wrapper（伴侣运行时状态回收留壳——CompanionState 为桌面宿主状态）。
//! data_export / pick_import_file / data_import 为 desktop-only 留壳不迁
//! （rfd 对话框交互链；云端走 /api/export、/api/import HTTP 流）。
use std::sync::Arc;

use std::path::Path;

use egosync_engine::commands::ctx::EngineCtx;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::events::DATA_IMPORTED_EVENT;
use crate::services::data_export::{export_all, import_all, ExportFormat, ExportResult, ImportResult};

#[tauri::command]
pub async fn data_export(
    formats: Vec<String>,
    app_handle: AppHandle,
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<ExportResult, AppError> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::ValidationError(format!("获取应用数据目录失败: {}", e)))?;

    let parsed_formats: Vec<ExportFormat> = formats
        .iter()
        .map(|f| match f.to_lowercase().as_str() {
            "sqlite" => Ok(ExportFormat::Sqlite),
            "json" => Ok(ExportFormat::Json),
            "markdown" => Ok(ExportFormat::Markdown),
            other => Err(AppError::ValidationError(format!(
                "不支持的导出格式: {}",
                other
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if parsed_formats.is_empty() {
        return Err(AppError::ValidationError(
            "至少需要选择一种导出格式".to_string(),
        ));
    }

    let selected =
        tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
            .await
            .map_err(|e| AppError::ValidationError(format!("选择导出目录失败: {}", e)))?;
    // 用户取消目录选择属于正常操作，返回空结果而非错误，避免前端误报。
    let Some(dir_path) = selected else {
        return Ok(ExportResult {
            files: Vec::new(),
            sqlite_path: None,
            json_path: None,
            markdown_path: None,
        });
    };

    export_all(
        &app_data_dir,
        &pool,
        &conv_pool,
        Path::new(&dir_path),
        parsed_formats,
    )
    .await
}

#[tauri::command]
pub async fn data_destroy(
    app_handle: AppHandle,
    ctx: State<'_, Arc<EngineCtx>>,
) -> Result<(), AppError> {
    let pool = ctx.pool.clone();
    // Story 15.4：命令主体（destroy_all_data）迁引擎经 ctx 注入路径执行；
    // 伴侣运行时状态回收留壳（CompanionState 为桌面宿主状态，顺序保持——
    // 销毁完成后回收）。
    egosync_engine::commands::data::data_destroy(&ctx).await?;

    // 销毁后回收手机伴侣运行时状态（keyring 不可用时 state 未管理，跳过）：
    // 终止会话、清 pending/配对窗口、注销 NSD、删除静态密钥
    if let Some(companion) =
        app_handle.try_state::<std::sync::Arc<crate::services::companion_connection::CompanionState>>()
    {
        crate::services::companion_connection::reset_after_data_destroy(&pool, &companion).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn pick_import_file() -> Result<Option<String>, AppError> {
    let selected =
        tauri::async_runtime::spawn_blocking(|| {
            rfd::FileDialog::new()
                .add_filter("EgoSync 存档", &["db", "json"])
                .pick_file()
        })
        .await
        .map_err(|e| AppError::ValidationError(format!("选择导入文件失败: {}", e)))?;

    Ok(selected.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn data_import(
    file_path: String,
    app_handle: AppHandle,
    pool: State<'_, DbPool>,
    conv_pool: State<'_, ConversationsPool>,
) -> Result<ImportResult, AppError> {
    let result = import_all(&pool, &conv_pool, Path::new(&file_path)).await?;
    // Story 13.1（评审决策①）：整库导入替换全量数据，必须触发快照重建
    // （STATE_DELTA 载荷本就是全量替换，事件只作触发信号）。
    // Story 15.4：事件名改引 engine 常量（值不变）。
    if let Err(e) = app_handle.emit(DATA_IMPORTED_EVENT, &result) {
        tracing::warn!(event = DATA_IMPORTED_EVENT, error = %e, "data 写事件发射失败");
    }
    Ok(result)
}

