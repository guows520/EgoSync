use std::path::Path;

use tauri::{AppHandle, Manager, State};

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::services::data_export::{export_all, ExportFormat, ExportResult};

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
