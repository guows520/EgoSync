//! 逻辑级备份端点（Story 17.3，FR-44 数据主权闭环）：
//! `GET /api/export` + `POST /api/import`。
//!
//! - **server 独立路由，不经 cmd 分发**：`data_export` / `data_import`
//!   保持 desktop-only（本机文件对话框语义），云端走本模块的 HTTP 流
//!   形态——同一引擎纯逻辑（`gather_export_data` / `import_all`）双落点；
//! - **导出包与桌面同格式**（跨形态互导）：body 即桌面 `export_json`
//!   写盘的同一 JSON 字符串（`serde_json::to_string_pretty(gather_
//!   export_data())` 逐字节同源），`exportVersion="1.0"`、ExportData
//!   段形状零改动；密钥值永不进响应（包内仅 `apiKeyRef` 引用）；
//! - **导入走临时文件而非改 engine 签名**：body 落 tempfile（.json 后缀
//!   ⇒ `import_all` 按内容分发进 `import_json_data`）——「先备份再分发 +
//!   双库各自单事务 + 失败不半写」语义零侵入复用（20+ 既有测试钉死）；
//! - **导入后密钥可达性探测**（17.1 deferred 收口）：逐配置
//!   `load_secret` 双通道存在性，缺失项以 `missing_api_key_error` 同款
//!   文案入报告（不阻塞导入——重录是修复动作不是前置条件）；
//! - **错误形状冻结**：引擎错误一律 `200 + AppError 单键 map + 判别头
//!   X-Egosync-App-Error: 1`（与 cmd_handler 同构）；非 200 白名单仅
//!   401（未认证，require_auth 中间件）/ 429 / 404 / 413（body >50MB，
//!   DefaultBodyLimit 传输层）/ 5xx；
//! - **成功后 SSE 广播** `data:imported`（engine 常量，与桌面壳同事件名、
//!   同 ImportResult payload——当前前端无消费方，前瞻兼容）。

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use egosync_engine::db::pool::{ConversationsPool, DbPool};
use egosync_engine::db::settings;
use egosync_engine::error::AppError;
use egosync_engine::events::DATA_IMPORTED_EVENT;
use egosync_engine::services::data_export::{gather_export_data, import_all, ImportResult};
use egosync_engine::services::llm_config::missing_api_key_error;
use egosync_engine::services::secret_store::SecretStore;
use serde::Serialize;

use crate::routes::APP_ERROR_HEADER;
use crate::routes::APP_ERROR_HEADER_VALUE;
use crate::sse::SseEvent;
use crate::AppState;

/// `GET /api/export`：流式返回导出包 JSON（attachment 文件名与桌面
/// `export_json` 的落盘命名同构：`egosync-export-YYYY-MM-DD.json`）。
pub async fn export_handler(State(state): State<Arc<AppState>>) -> Response {
    // 并发互斥（评审修复）：与导入共享锁——导出双库快照期间不得被并发导入
    // 交错替换（混装包见 import_handler 注释）。
    let _export_guard = state.import_lock.lock().await;
    let json = match export_package_json(&state.ctx.pool, &state.ctx.conv_pool).await {
        Ok(json) => json,
        Err(err) => return app_error_response(err),
    };

    let filename = format!(
        "egosync-export-{}.json",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let mut response = (StatusCode::OK, Body::from(json)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    // attachment 头：浏览器触发下载而非页面渲染（curl 消费不受影响）
    if let Ok(disposition) =
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
    {
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            disposition,
        );
    }
    response
}

/// 导出包 JSON 字符串——与桌面 `export_json` 写盘内容逐字节同源
/// （`to_string_pretty(gather_export_data())`），跨形态互导由格式同构保证。
async fn export_package_json(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
) -> Result<String, AppError> {
    let data = gather_export_data(pool, conv_pool).await?;
    serde_json::to_string_pretty(&data)
        .map_err(|e| AppError::ValidationError(format!("JSON 序列化失败: {}", e)))
}

/// `POST /api/import`：body=导出包 JSON → tempfile → `import_all`
/// （备份先行 + 原子语义）→ 200 返回 ImportResult + 密钥可达性报告。
///
/// body >50MB 由传输层 `DefaultBodyLimit` 拒 413（非 200 白名单成员）；
/// 损坏包 / 版本不符走 `import_json_data` 既有 ValidationError 路径
/// （解析在事务开启**之前**——失败不半写）。
pub async fn import_handler(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
    // 并发互斥（评审修复）：导入是双库全量替换，两个并发导入（双标签页/
    // curl+UI）交错提交会让主库与对话库来自不同导出包（SQLite 只保证单库
    // 事务原子，不保证跨请求串行）；导出侧同锁共享——导入进行中的导出等待
    // 完成，避免读到替换中间态（主库新/对话库旧的混装包）。
    let _import_guard = state.import_lock.lock().await;

    // body 落临时文件（.json 后缀 ⇒ import_all 按内容分发；桌面 data_import
    // 的 file_path 语义同构——engine 签名零改动）
    let temp_path = std::env::temp_dir().join(format!(
        "egosync-import-{}.json",
        uuid::Uuid::new_v4()
    ));
    // 权限显式 0600（评审修复）：body 是用户全量个人数据（会话/任务/记忆），
    // 缺省 umask 下的 0644 会让同机其它进程/用户可读——多用户开发机与
    // 容器内最小权限纪律。
    let mut temp_file = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp_path)
    {
        Ok(f) => f,
        Err(e) => {
            return app_error_response(AppError::ValidationError(format!(
                "创建导入临时文件失败: {}",
                e
            )));
        }
    };
    if let Err(e) = temp_file.write_all(&body) {
        let _ = std::fs::remove_file(&temp_path);
        return app_error_response(AppError::ValidationError(format!(
            "写入导入临时文件失败: {}",
            e
        )));
    }
    drop(temp_file);

    let import_outcome = import_all(&state.ctx.pool, &state.ctx.conv_pool, &temp_path).await;

    // 临时文件清理（成败皆删——body 已在引擎侧消费完毕；best-effort）
    let _ = std::fs::remove_file(&temp_path);

    let imported = match import_outcome {
        Ok(result) => result,
        Err(err) => return app_error_response(err),
    };

    // 密钥可达性探测（导入后：配置清单来自**导入后**的库——探测对象即
    // 刚恢复的 apiKeyRef 集合）；探测失败本身不阻塞响应（导入已成功，
    // 报告缺失属「后续动作指引」语义——缺失清单空时报错面也无法回滚）
    let missing_secrets = match probe_missing_secrets(&state.ctx.pool, state.ctx.secrets.as_ref())
        .await
    {
        Ok(report) => report,
        Err(e) => {
            tracing::warn!("导入后密钥可达性探测失败（不阻塞导入结果）: {}", e);
            Vec::new()
        }
    };

    // SSE 广播 data:imported（engine 常量，与桌面壳同事件名 + 同 payload
    // 形状；无订阅者时 send Err 是广播语义正常态）
    let _ = state.events_tx.send(SseEvent {
        event: DATA_IMPORTED_EVENT.to_string(),
        payload: serde_json::to_value(&imported).unwrap_or(serde_json::Value::Null),
    });
    tracing::info!(
        roles = imported.roles_count,
        tasks = imported.tasks_count,
        memories = imported.memories_count,
        missing_secrets = missing_secrets.len(),
        "云端导入完成"
    );

    (
        StatusCode::OK,
        axum::Json(ImportResponse {
            imported,
            missing_secrets,
        }),
    )
        .into_response()
}

/// 密钥缺失报告项（`#[serde(rename_all = "camelCase")]`——camelCase JSON
/// 与前端 TS 侧约定一致）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingSecretReport {
    /// 配置名（用户能对上是哪条配置——与 missing_api_key_error 文案同源）。
    pub config_name: String,
    /// 密钥引用（`llm_{uuid}_api_key`——EGOSYNC_SECRET_{ref} env 引导键名）。
    pub api_key_ref: String,
    /// 重录路径文案（`missing_api_key_error` 同款逐字——文案一致性由
    /// 构造保证，非第二拼装点）。
    pub message: String,
}

/// 导入响应（ImportResult + 缺失报告——结构化、不阻塞导入结果返回）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResponse {
    pub imported: ImportResult,
    pub missing_secrets: Vec<MissingSecretReport>,
}

/// 密钥可达性批量探测（17.1 deferred 收口）：`list_llm_configs` ref 清单
/// 逐项 `load_secret`（secrets.json + env 双通道存在性）。
///
/// - **探测面刻意为存在性**（非网络级连通测试——deferred-work 冻结口径）；
/// - env 空串现状语义不动：`Ok(Some(""))` 按「有值」计（SecretStore 行为
///   面冻结，空串语义变更属独立裁决）；
/// - `load_secret` 读错误（secrets.json 损坏等）按缺失上报——重录即重写
///   secrets.json，与缺失同一修复路径。
pub async fn probe_missing_secrets(
    pool: &DbPool,
    secrets: &dyn SecretStore,
) -> Result<Vec<MissingSecretReport>, AppError> {
    let configs = settings::list_llm_configs(pool).await?;
    let mut missing = Vec::new();
    for config in configs {
        let reachable = match secrets.load_secret(&config.api_key_ref) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => {
                tracing::warn!(
                    config = %config.name,
                    key_ref = %config.api_key_ref,
                    "密钥探测读取失败（按缺失上报）: {}",
                    e
                );
                false
            }
        };
        if !reachable {
            // 文案复用 missing_api_key_error（NFR-C7 单一拼装点——此处经
            // 重组装取文案，非第二拼装源）
            let message = match missing_api_key_error(&config.name) {
                AppError::KeyringError(msg) => msg,
                // 评审修复：跨模块错误形状依赖不 panic——missing_api_key_error
                // 变体一旦演化（引擎重构），unreachable! 会让一次已成功的导入
                // 崩在响应构造期。降级为保留重录路径指引的最小文案。
                _ => format!(
                    "未找到配置 '{}' 的 API Key，请前往 设置 → 模型服务配置 重新保存密钥",
                    config.name
                ),
            };
            missing.push(MissingSecretReport {
                config_name: config.name,
                api_key_ref: config.api_key_ref,
                message,
            });
        }
    }
    Ok(missing)
}

/// 冻结错误形状：`200 + AppError 单键 map + 判别头`（cmd_handler Err 分支
/// 同构；前端 HttpTransport 据此解包 reject）。
fn app_error_response(err: AppError) -> Response {
    (
        StatusCode::OK,
        [(APP_ERROR_HEADER, APP_ERROR_HEADER_VALUE)],
        axum::Json(err),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用内存密钥库（secret_store_test 同款）：load_secret 存在性可注入。
    #[derive(Default)]
    struct ProbeSecretStore(std::sync::Mutex<std::collections::HashMap<String, String>>);

    impl SecretStore for ProbeSecretStore {
        fn save_secret(&self, key: &str, value: &str) -> Result<(), AppError> {
            self.0
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }
        fn load_secret(&self, key: &str) -> Result<Option<String>, AppError> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        fn delete_secret(&self, key: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    async fn setup_probe_db() -> DbPool {
        let dir = tempfile::tempdir().expect("create temp dir");
        egosync_engine::db::pool::init_db(&dir.path().join("probe.db"))
            .await
            .expect("init db")
    }

    async fn insert_llm_config(pool: &DbPool, id: &str, name: &str, api_key_ref: &str) {
        sqlx::query(
            "INSERT INTO llm_configs (id, name, provider, base_url, model, api_key_ref, is_default, network_location, created_at, updated_at)
             VALUES (?1, ?2, 'deepseek', '', 'm', ?3, 0, 'external', '2026-09-21T00:00:00Z', '2026-09-21T00:00:00Z')",
        )
        .bind(id)
        .bind(name)
        .bind(api_key_ref)
        .execute(pool)
        .await
        .expect("insert llm_config");
    }

    /// 探测报告：缺失项列出 configName + apiKeyRef + 重录文案（与
    /// missing_api_key_error 同款——含「设置 → 模型服务配置」路径），
    /// 在场项零误报。
    #[tokio::test]
    async fn probe_reports_missing_refs_with_reentry_message() {
        let pool = setup_probe_db().await;
        let secrets = ProbeSecretStore::default();
        insert_llm_config(&pool, "cfg-a", "DeepSeek 主力", "llm_a_api_key").await;
        insert_llm_config(&pool, "cfg-b", "备用 Kimi", "llm_b_api_key").await;
        secrets.save_secret("llm_b_api_key", "sk-present").unwrap();

        let report = probe_missing_secrets(&pool, &secrets).await.expect("probe");

        assert_eq!(report.len(), 1, "仅缺失项入报告: {report:?}");
        assert_eq!(report[0].config_name, "DeepSeek 主力");
        assert_eq!(report[0].api_key_ref, "llm_a_api_key");
        assert!(
            report[0].message.contains("DeepSeek 主力"),
            "文案含配置名: {}",
            report[0].message
        );
        assert!(
            report[0].message.contains("设置 → 模型服务配置"),
            "文案含重录路径: {}",
            report[0].message
        );
    }

    /// env 空串现状语义不动（17.1 冻结）：`EGOSYNC_SECRET_{ref}` 设为空串
    /// 时 `load_secret` 返回 `Some("")`——探测按「在场」计，不因空值误报
    /// 缺失（空串语义变更属独立裁决，deferred-work 已登记）。
    /// 经真实 ServerSecretStore（env 兜底是 store 行为面，非探测的）；
    /// 独立 env 键（uuid 后缀）防并行测试互踩（secret_store_test 同款范式）。
    #[tokio::test]
    async fn probe_treats_empty_env_value_as_present_current_semantics() {
        let pool = setup_probe_db().await;
        let dir = tempfile::tempdir().expect("create temp dir");
        let store = crate::secret_store::ServerSecretStore::new(dir.path().to_path_buf());
        let ref_key = format!("llm_envempty_{}_api_key", uuid::Uuid::new_v4());
        insert_llm_config(&pool, "cfg-empty-env", "空串 env 配置", &ref_key).await;

        let env_name = format!("EGOSYNC_SECRET_{}", ref_key);
        let prev = std::env::var(&env_name).ok();
        std::env::set_var(&env_name, "");

        let report = probe_missing_secrets(&pool, &store).await.expect("probe");

        match prev {
            Some(v) => std::env::set_var(&env_name, v),
            None => std::env::remove_var(&env_name),
        }

        assert!(
            report.is_empty(),
            "env 空串按在场计（现状语义不动）: {report:?}"
        );
    }
}
