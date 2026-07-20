use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::db;
use crate::error::AppError;
use crate::models::skill::{ImportCustomSkillInput, PreviewCustomSkillInput, SkillRoleScope, BUTLER_SCOPE_ID};
use crate::models::task::{CreateTaskInput, TaskOwnerType};
use crate::services::agent_config::AgentConfigService;
use tauri::{AppHandle, Emitter, Manager};

const DEFAULT_MAX_BODY_BYTES: usize = 16 * 1024;
const SKILL_MAX_BODY_BYTES: usize = 128 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const DELEGATION_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_CONCURRENT_DELEGATIONS: usize = 2;
const MAX_CONCURRENT_CONNECTIONS: usize = 8;
pub const BRIDGE_TOKEN_ENV: &str = "EGOSYNC_DELEGATE_BRIDGE_TOKEN";
pub const BRIDGE_PORT_ENV: &str = "EGOSYNC_DELEGATE_BRIDGE_PORT";

#[derive(Clone)]
pub struct DelegateBridge {
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    sessions: Arc<Mutex<HashMap<String, DelegateSessionContext>>>,
    metadata_lock: Arc<Mutex<()>>,
    skill_creation_lock: Arc<Mutex<()>>,
    runtime_refresh_pending: Arc<Mutex<bool>>,
    delegation_limit: Arc<Semaphore>,
    connection_limit: Arc<Semaphore>,
    token: String,
    app_handle: Option<AppHandle>,
}

#[derive(Clone)]
enum DelegateSessionContext {
    Butler { butler_user_message_id: String },
    Role { role_id: String },
}

#[derive(Deserialize)]
struct DelegateRequest {
    session_id: String,
    target_role_id: String,
    task_summary: String,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Serialize)]
struct DelegateResponse {
    status: String,
    role_response: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskRequest {
    session_id: String,
    #[serde(default)]
    role_id: Option<String>,
    title: String,
    #[serde(default)]
    deadline: Option<String>,
}

#[derive(Deserialize)]
struct CompleteTaskRequest {
    task_id: String,
}

#[derive(Deserialize)]
struct DeleteTaskRequest {
    task_id: String,
}

#[derive(Serialize)]
struct TaskActionResponse {
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSkillRequest {
    session_id: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSkillResponse {
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    skill_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skill_name: Option<String>,
}

struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl DelegateBridge {
    pub fn new(main_pool: DbPool, conv_pool: ConversationsPool, token: String, app_handle: Option<AppHandle>) -> Self {
        Self {
            main_pool,
            conv_pool,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            metadata_lock: Arc::new(Mutex::new(())),
            skill_creation_lock: Arc::new(Mutex::new(())),
            runtime_refresh_pending: Arc::new(Mutex::new(false)),
            delegation_limit: Arc::new(Semaphore::new(MAX_CONCURRENT_DELEGATIONS)),
            connection_limit: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS)),
            token,
            app_handle,
        }
    }

    fn validate_request(&self, request: &HttpRequest) -> Result<(), String> {
        validate_request_token(request, &self.token)
    }

    pub async fn register_session(&self, session_id: &str, butler_user_message_id: &str) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            session_id.to_string(),
            DelegateSessionContext::Butler {
                butler_user_message_id: butler_user_message_id.to_string(),
            },
        );
        tracing::info!(
            session_id,
            registered_sessions = sessions.len(),
            "[stage-b-diag] delegate context registered"
        );
    }

    pub async fn register_role_session(&self, session_id: &str, role_id: &str) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(
            session_id.to_string(),
            DelegateSessionContext::Role { role_id: role_id.to_string() },
        );
        tracing::info!(session_id, role_id, registered_sessions = sessions.len(), "role task context registered");
    }

    pub async fn unregister_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        let removed = sessions.remove(session_id).is_some();
        tracing::info!(
            session_id,
            removed,
            registered_sessions = sessions.len(),
            "[stage-b-diag] delegate context unregistered"
        );
    }

    pub async fn request_runtime_refresh(&self) {
        *self.runtime_refresh_pending.lock().await = true;
    }

    pub async fn take_runtime_refresh(&self) -> bool {
        let mut pending = self.runtime_refresh_pending.lock().await;
        std::mem::take(&mut *pending)
    }

    async fn create_skill(&self, req: CreateSkillRequest) -> CreateSkillResponse {
        let session_id = req.session_id.trim();
        let content = req.content.trim();
        if session_id.is_empty() || content.is_empty() {
            return CreateSkillResponse {
                status: "bad_request".to_string(),
                message: "创建 Skill 失败：缺少当前会话或完整 SKILL.md 内容。".to_string(),
                skill_id: None,
                skill_name: None,
            };
        }

        let session = {
            let sessions = self.sessions.lock().await;
            sessions.get(session_id).cloned()
        };
        let Some(session) = session else {
            return CreateSkillResponse {
                status: "session_not_found".to_string(),
                message: "创建 Skill 失败：未找到当前会话上下文，请稍后重试。".to_string(),
                skill_id: None,
                skill_name: None,
            };
        };
        let owner_id = match &session {
            DelegateSessionContext::Role { role_id } => role_id.as_str(),
            DelegateSessionContext::Butler { .. } => BUTLER_SCOPE_ID,
        };

        let Some(app_handle) = self.app_handle.as_ref() else {
            return CreateSkillResponse {
                status: "runtime_unavailable".to_string(),
                message: "创建 Skill 失败：应用运行时不可用。".to_string(),
                skill_id: None,
                skill_name: None,
            };
        };
        let Some(agent_config) = app_handle.try_state::<AgentConfigService>() else {
            return CreateSkillResponse {
                status: "runtime_unavailable".to_string(),
                message: "创建 Skill 失败：Agent 配置服务不可用。".to_string(),
                skill_id: None,
                skill_name: None,
            };
        };

        let enabled = match &session {
            DelegateSessionContext::Role { role_id } => crate::db::roles::get_role(&self.main_pool, role_id)
                .await
                .map(|role| crate::services::role_config::skill_enabled(&role.skills_config, crate::services::role_config::SKILL_CREATOR_KEY))
                .unwrap_or(false),
            DelegateSessionContext::Butler { .. } => crate::services::butler_config::get_butler_skills(&self.main_pool)
                .await
                .map(|skills| skills.skill_creator)
                .unwrap_or(false),
        };
        if !enabled {
            return CreateSkillResponse {
                status: "forbidden".to_string(),
                message: "创建 Skill 失败：当前角色未启用 skill-creator。".to_string(),
                skill_id: None,
                skill_name: None,
            };
        }

        // Serialize create operations so two concurrent requests cannot race
        // while writing the same OpenCode discovery directory.
        let _creation_guard = self.skill_creation_lock.lock().await;

        let preview = match crate::services::skill_registry::preview_custom_skill(
            &self.main_pool,
            &PreviewCustomSkillInput { content: Some(content.to_string()) },
        )
        .await
        {
            Ok(preview) => preview,
            Err(error) => return CreateSkillResponse {
                status: "validation_error".to_string(),
                message: format!("创建 Skill 失败：{}", error),
                skill_id: None,
                skill_name: None,
            },
        };
        let existing_id = preview.duplicate.as_ref().map(|duplicate| duplicate.existing.id.clone());
        if let Some(duplicate) = preview.duplicate.as_ref() {
            if duplicate.existing.name != preview.name
                || duplicate.existing.content_hash != preview.content_hash
            {
                return CreateSkillResponse {
                    status: "conflict".to_string(),
                    message: "创建 Skill 失败：同名或同内容的 Skill 已存在，请更换名称或在设置页确认覆盖。".to_string(),
                    skill_id: None,
                    skill_name: None,
                };
            }
        }
        let was_enabled = match (&session, existing_id.as_deref()) {
            (DelegateSessionContext::Role { role_id }, Some(skill_id)) => crate::db::roles::get_role(&self.main_pool, role_id)
                .await
                .map(|role| crate::services::role_config::enabled_skill_ids_from_config(&role.skills_config).iter().any(|id| id == skill_id))
                .unwrap_or(false),
            (DelegateSessionContext::Butler { .. }, Some(skill_id)) => crate::services::butler_config::get_butler_skills(&self.main_pool)
                .await
                .map(|skills| skills.enabled_skill_ids.iter().any(|id| id == skill_id))
                .unwrap_or(false),
            _ => false,
        };

        let app_data_dir = match app_handle.path().app_data_dir() {
            Ok(path) => path,
            Err(error) => return CreateSkillResponse {
                status: "runtime_unavailable".to_string(),
                message: format!("创建 Skill 失败：无法获取应用目录（{}）", error),
                skill_id: None,
                skill_name: None,
            },
        };
        let skills_root = app_data_dir.join("opencode-workspace").join(".opencode").join("skills");
        let (entry, newly_imported) = if let Some(duplicate) = preview.duplicate.as_ref() {
            if let Err(error) = crate::services::skill_registry::enable_skill_for_owner(
                &self.main_pool,
                &duplicate.existing.id,
                owner_id,
            )
            .await
            {
                tracing::error!(error = %error, "绑定已有 Skill 到当前角色失败");
                return CreateSkillResponse {
                    status: "error".to_string(),
                    message: "创建 Skill 失败：无法绑定到当前角色。".to_string(),
                    skill_id: None,
                    skill_name: None,
                };
            }
            (duplicate.existing.clone(), false)
        } else {
            let input = ImportCustomSkillInput {
                content: Some(content.to_string()),
                source_path: None,
                overwrite_existing: false,
                role_scope: Some(SkillRoleScope { all_roles: false, role_ids: vec![owner_id.to_string()] }),
            };
            match crate::services::skill_registry::import_custom_skill(&self.main_pool, &skills_root, &input).await {
                Ok(result) => match result.entry {
                    Some(entry) => (entry, true),
                    None => return CreateSkillResponse {
                        status: "error".to_string(),
                        message: "创建 Skill 失败：导入结果不完整。".to_string(),
                        skill_id: None,
                        skill_name: None,
                    },
                },
                Err(error) => {
                    tracing::error!(skill_name = %preview.name, error = %error, "Skill 导入失败，执行补偿清理");
                    let rollback = if let Ok(Some(partial)) = crate::db::skills::find_skill_by_name(&self.main_pool, &preview.name).await {
                        crate::services::skill_registry::delete_custom_skill(&self.main_pool, &partial.id).await.map(|_| ())
                    } else {
                        std::fs::remove_dir_all(skills_root.join(&preview.name))
                            .or_else(|cleanup_error| if cleanup_error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(cleanup_error) })
                            .map_err(|cleanup_error| AppError::ValidationError(cleanup_error.to_string()))
                    };
                    let rollback_complete = rollback.is_ok();
                    if let Err(rollback_error) = rollback {
                        tracing::error!(skill_name = %preview.name, error = %rollback_error, "Skill 导入失败后的补偿清理未完成");
                    }
                    return CreateSkillResponse {
                        status: "error".to_string(),
                        message: if rollback_complete {
                            "创建 Skill 失败：导入未完成，已回滚。".to_string()
                        } else {
                            "创建 Skill 失败，且自动回滚未完成；请在设置页检查残留状态。".to_string()
                        },
                        skill_id: None,
                        skill_name: None,
                    };
                }
            }
        };

        let (registry, registry_read_failed) = match crate::db::skills::list_skills(&self.main_pool).await {
            Ok(registry) => (registry, false),
            Err(error) => {
                tracing::error!(error = %error, "创建 Skill 后读取 registry 失败");
                (Vec::new(), true)
            }
        };
        let sync_result = if registry_read_failed {
            Err(AppError::DbError("读取 Skill registry 失败".to_string()))
        } else { match &session {
            DelegateSessionContext::Role { role_id } => match crate::db::roles::get_role(&self.main_pool, role_id).await {
                Ok(role) => crate::services::mcp_server::sync_role_agent_with_mcp(&self.main_pool, &agent_config, &role, &registry).await,
                Err(error) => Err(error),
            },
            DelegateSessionContext::Butler { .. } => match crate::services::butler_config::get_butler_skills(&self.main_pool).await {
                Ok(skills) => agent_config.sync_butler_skills_with_registry(&skills, &registry),
                Err(error) => Err(error),
            },
        }};
        if registry_read_failed || sync_result.is_err() {
            let rollback = if newly_imported {
                crate::services::skill_registry::delete_custom_skill(&self.main_pool, &entry.id).await.map(|_| ())
            } else if !was_enabled {
                crate::services::skill_registry::remove_skill_from_role(&self.main_pool, &entry.id, owner_id).await
            } else {
                Ok(())
            };
            let rollback_complete = rollback.is_ok();
            if let Err(rollback_error) = rollback {
                tracing::error!(skill_id = %entry.id, error = %rollback_error, "Skill 同步失败后的回滚也失败");
            }
            return CreateSkillResponse {
                status: "sync_error".to_string(),
                message: if rollback_complete {
                    "创建 Skill 失败：配置同步未完成，已回滚。".to_string()
                } else {
                    "创建 Skill 失败，且自动回滚未完成；请在设置页检查残留状态。".to_string()
                },
                skill_id: None,
                skill_name: None,
            };
        }

        self.request_runtime_refresh().await;
        if let Some(app_handle) = self.app_handle.as_ref() {
            let _ = app_handle.emit(
                "skill-registry-updated",
                serde_json::json!({ "ownerId": owner_id, "skillId": entry.id }),
            );
        }

        CreateSkillResponse {
            status: "ok".to_string(),
            message: format!("Skill {} 已创建并启用到当前角色。", entry.name),
            skill_id: Some(entry.id),
            skill_name: Some(entry.name),
        }
    }

    async fn create_task(&self, req: CreateTaskRequest) -> TaskActionResponse {
        let session_id = req.session_id.trim();
        let title = req.title.trim();
        if session_id.is_empty() {
            return TaskActionResponse {
                status: "bad_request".to_string(),
                message: "未找到当前会话上下文，请稍后重试。".to_string(),
                task_id: None,
            };
        }
        if title.is_empty() {
            return TaskActionResponse {
                status: "bad_request".to_string(),
                message: "缺少任务标题。".to_string(),
                task_id: None,
            };
        }

        let session = {
            let sessions = self.sessions.lock().await;
            sessions.get(session_id).cloned()
        };
        let Some(session) = session else {
            return TaskActionResponse {
                status: "session_not_found".to_string(),
                message: "未找到当前会话上下文，请稍后重试。".to_string(),
                task_id: None,
            };
        };
        let requested_role_id = req.role_id.as_deref().map(str::trim).filter(|id| !id.is_empty());
        let role_id = match session {
            DelegateSessionContext::Role { role_id } => {
                tracing::info!(session_id, bound_role_id = %role_id, requested_role_id = ?requested_role_id, "role task owner resolved from session context");
                role_id
            }
            DelegateSessionContext::Butler { .. } => match requested_role_id {
                Some(role_id) => role_id.to_string(),
                None => return TaskActionResponse {
                    status: "bad_request".to_string(),
                    message: "管家创建角色任务时必须指定目标角色。".to_string(),
                    task_id: None,
                },
            },
        };
        let role_available = match db::roles::get_role(&self.main_pool, &role_id).await {
            Ok(role) => role.status == "active",
            Err(AppError::NotFound(_)) => false,
            Err(error) => {
                tracing::warn!(role_id = %role_id, error = %error, "failed to validate task owner role");
                return TaskActionResponse {
                    status: "error".to_string(),
                    message: "验证目标角色失败，请稍后重试。".to_string(),
                    task_id: None,
                };
            }
        };
        if !role_available {
            return TaskActionResponse {
                status: "role_not_found".to_string(),
                message: "目标角色不存在或已不可用。".to_string(),
                task_id: None,
            };
        }

        let input = CreateTaskInput {
            owner_type: Some(TaskOwnerType::Role),
            role_id: Some(role_id.clone()),
            title: title.to_string(),
            deadline: req.deadline.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string()),
            quadrant: None,
            is_big_rock: None,
        };

        match db::tasks::create_task(&self.main_pool, &input).await {
            Ok(task) => {
                tracing::info!(
                    "[bridge] create_task: role_id={} title={} task_id={}",
                    role_id, title, task.id
                );
                let pool = self.main_pool.clone();
                let task_id = task.id.clone();
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let classified = match crate::services::task_classifier::classify_and_persist(&pool, &task_id).await {
                        Ok(updated) => Some(updated),
                        Err(e) => {
                            tracing::warn!(task_id = %task_id, error = %e, "bridge 创建任务后自动分类失败，保留默认 Q2");
                            db::tasks::get_active_task_pub(&pool, &task_id).await.ok()
                        }
                    };
                    if let Some(updated) = classified {
                        if let Some(ref app) = app_handle {
                            let _ = app.emit("task:classified", &updated);
                        }
                    }
                });
                TaskActionResponse {
                    status: "ok".to_string(),
                    message: format!("任务「{}」已创建，默认放入 Q2，后台正在自动分类。", title),
                    task_id: Some(task.id),
                }
            }
            Err(e) => {
                tracing::warn!("[bridge] create_task failed: role_id={} title={} error={}", role_id, title, e);
                TaskActionResponse {
                    status: "error".to_string(),
                    message: format!("创建任务失败：{}", e),
                    task_id: None,
                }
            }
        }
    }

    async fn complete_task(&self, req: CompleteTaskRequest) -> TaskActionResponse {
        let task_id = req.task_id.trim();
        if task_id.is_empty() {
            return TaskActionResponse {
                status: "bad_request".to_string(),
                message: "缺少任务ID。".to_string(),
                task_id: None,
            };
        }

        match db::tasks::set_task_completion(&self.main_pool, task_id, true).await {
            Ok(task) => {
                tracing::info!("[bridge] complete_task: task_id={} title={}", task_id, task.title);
                TaskActionResponse {
                    status: "ok".to_string(),
                    message: format!("任务「{}」已标记为完成。", task.title),
                    task_id: Some(task.id),
                }
            }
            Err(e) => {
                tracing::warn!("[bridge] complete_task failed: task_id={} error={}", task_id, e);
                TaskActionResponse {
                    status: "error".to_string(),
                    message: format!("标记完成失败：{}", e),
                    task_id: None,
                }
            }
        }
    }

    async fn delete_task(&self, req: DeleteTaskRequest) -> TaskActionResponse {
        let task_id = req.task_id.trim();
        if task_id.is_empty() {
            return TaskActionResponse {
                status: "bad_request".to_string(),
                message: "缺少任务ID。".to_string(),
                task_id: None,
            };
        }

        let task = match db::tasks::get_active_task_pub(&self.main_pool, task_id).await {
            Ok(t) => t,
            Err(e) => {
                return TaskActionResponse {
                    status: "error".to_string(),
                    message: format!("任务不存在：{}", e),
                    task_id: None,
                };
            }
        };

        match db::tasks::soft_delete_task(&self.main_pool, task_id).await {
            Ok(()) => {
                tracing::info!("[bridge] delete_task: task_id={} title={}", task_id, task.title);
                TaskActionResponse {
                    status: "ok".to_string(),
                    message: format!("任务「{}」已删除。", task.title),
                    task_id: Some(task.id),
                }
            }
            Err(e) => {
                tracing::warn!("[bridge] delete_task failed: task_id={} error={}", task_id, e);
                TaskActionResponse {
                    status: "error".to_string(),
                    message: format!("删除任务失败：{}", e),
                    task_id: None,
                }
            }
        }
    }

    async fn delegate_to_role(&self, request: DelegateRequest) -> DelegateResponse {
        tracing::info!(
            "[bridge] delegate_to_role: session_id={} target_role_id={}",
            request.session_id, request.target_role_id
        );
        let session = {
            let sessions = self.sessions.lock().await;
            let count = sessions.len();
            let s = sessions.get(&request.session_id).cloned();
            tracing::debug!(
                "[bridge] session lookup: registered_sessions={} found={}",
                count, s.is_some()
            );
            s
        };
        let Some(session) = session else {
            tracing::warn!(
                "[bridge] session_not_found: session_id={} (no matching registered session)",
                request.session_id
            );
            return DelegateResponse {
                status: "session_not_found".to_string(),
                role_response: "委派失败：未找到当前会话上下文，请稍后重试。".to_string(),
            };
        };

        let target_role_id = request.target_role_id.trim();
        let task_summary = request.task_summary.trim();
        let context = request
            .context
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if target_role_id.is_empty() || target_role_id.len() > 128 {
            return DelegateResponse {
                status: "bad_request".to_string(),
                role_response: "委派失败：目标角色参数无效。".to_string(),
            };
        }
        if task_summary.is_empty() || task_summary.chars().count() > 200 {
            return DelegateResponse {
                status: "bad_request".to_string(),
                role_response: "委派失败：任务摘要为空或过长。".to_string(),
            };
        }
        if context.map(|s| s.chars().count()).unwrap_or(0) > 1000 {
            return DelegateResponse {
                status: "bad_request".to_string(),
                role_response: "委派失败：上下文过长。".to_string(),
            };
        }

        let args = serde_json::json!({
            "target_role_id": target_role_id,
            "task_summary": task_summary,
            "context": context,
        })
        .to_string();

        let permit = match self.delegation_limit.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                return DelegateResponse {
                    status: "bridge_stopped".to_string(),
                    role_response: "委派失败：本地桥接服务已停止。".to_string(),
                };
            }
        };
        tracing::info!("[bridge] calling execute_delegate_to_role: role_id={}", target_role_id);
        let DelegateSessionContext::Butler { butler_user_message_id } = session else {
            return DelegateResponse {
                status: "bad_request".to_string(),
                role_response: "委派失败：角色会话不能发起跨角色委派。".to_string(),
            };
        };
        let source_conversation_id = sqlx::query_scalar::<_, String>(
            "SELECT conversation_id FROM messages WHERE id = ?1",
        )
        .bind(&butler_user_message_id)
        .fetch_optional(&*self.conv_pool)
        .await
        .ok()
        .flatten();
        let (role_response, record) = match timeout(
            DELEGATION_TIMEOUT,
            crate::services::agent_engine::execute_delegate_to_role_with_source(
                &self.main_pool,
                &self.conv_pool,
                &args,
                source_conversation_id.as_deref(),
            ),
        )
        .await
        {
            Ok((resp, rec)) => {
                tracing::info!(
                    "[bridge] delegate_to_role completed: role_id={} response_len={}",
                    target_role_id, resp.len()
                );
                (resp, rec)
            }
            Err(_) => {
                tracing::warn!("[bridge] delegate_to_role timed out after {}s", DELEGATION_TIMEOUT.as_secs());
                return DelegateResponse {
                    status: "timeout".to_string(),
                    role_response: "委派失败：角色处理超时，请稍后重试。".to_string(),
                };
            }
        };
        drop(permit);
        let _metadata_guard = self.metadata_lock.lock().await;
        crate::services::agent_engine::append_delegation_metadata(
            &self.conv_pool,
            &butler_user_message_id,
            &record,
        )
        .await;

        DelegateResponse {
            status: "ok".to_string(),
            role_response,
        }
    }
}

pub fn generate_bridge_token() -> String {
    uuid::Uuid::new_v4().as_simple().to_string()
}

pub async fn bind_random_listener() -> Result<(TcpListener, u16), AppError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|e| AppError::SidecarError(format!("启动委派桥接服务失败: {}", e)))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::SidecarError(format!("读取委派桥接端口失败: {}", e)))?
        .port();
    Ok((listener, port))
}

pub async fn start_server_on_listener(
    listener: TcpListener,
    bridge: DelegateBridge,
    cancel: CancellationToken,
) -> Result<(), AppError> {
    let port = listener
        .local_addr()
        .map_err(|e| AppError::SidecarError(format!("读取委派桥接端口失败: {}", e)))?
        .port();
    tracing::info!(port, "delegate bridge listening");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let bridge = bridge.clone();
                        tokio::spawn(async move {
                            let permit = bridge.connection_limit.clone().try_acquire_owned();
                            match permit {
                                Ok(permit) => handle_connection_with_timeout(stream, bridge, permit).await,
                                Err(_) => write_response(
                                    stream,
                                    json_response(
                                        503,
                                        "Service Unavailable",
                                        &serde_json::json!({ "status": "busy", "role_response": "委派失败：本地桥接服务正忙，请稍后重试。" }),
                                    ),
                                )
                                .await,
                            }
                        });
                    }
                    Err(e) => tracing::warn!("delegate bridge accept failed: {}", e),
                }
            }
        }
    }

    Ok(())
}

async fn handle_connection_with_timeout(
    stream: TcpStream,
    bridge: DelegateBridge,
    _permit: OwnedSemaphorePermit,
) {
    if timeout(
        READ_TIMEOUT + DELEGATION_TIMEOUT,
        handle_connection(stream, bridge),
    )
    .await
    .is_err()
    {
        tracing::warn!("delegate bridge request timed out");
    }
}

async fn handle_connection(mut stream: TcpStream, bridge: DelegateBridge) {
    let response = match read_http_request(&mut stream).await {
        Ok(request) if request.method == "POST" && request.path == "/delegate-to-role" => {
            if let Err(response) = bridge.validate_request(&request) {
                response
            } else {
                match serde_json::from_slice::<DelegateRequest>(&request.body) {
                    Ok(delegate_request) => {
                        let response = bridge.delegate_to_role(delegate_request).await;
                        json_response(200, "OK", &response)
                    }
                    Err(_) => json_response(
                        400,
                        "Bad Request",
                        &serde_json::json!({ "status": "bad_request", "role_response": "委派参数解析失败。" }),
                    ),
                }
            }
        }
        Ok(request) if request.method == "POST" && request.path == "/create-task" => {
            if let Err(response) = bridge.validate_request(&request) {
                response
            } else {
                match serde_json::from_slice::<CreateTaskRequest>(&request.body) {
                    Ok(req) => {
                        let response = bridge.create_task(req).await;
                        json_response(200, "OK", &response)
                    }
                    Err(_) => json_response(
                        400,
                        "Bad Request",
                        &serde_json::json!({ "status": "bad_request", "message": "创建任务参数解析失败。" }),
                    ),
                }
            }
        }
        Ok(request) if request.method == "POST" && request.path == "/create-skill" => {
            if let Err(response) = bridge.validate_request(&request) {
                response
            } else {
                match serde_json::from_slice::<CreateSkillRequest>(&request.body) {
                    Ok(req) => json_response(200, "OK", &bridge.create_skill(req).await),
                    Err(_) => json_response(
                        400,
                        "Bad Request",
                        &serde_json::json!({ "status": "bad_request", "message": "创建 Skill 参数解析失败。" }),
                    ),
                }
            }
        }
        Ok(request) if request.method == "POST" && request.path == "/complete-task" => {
            if let Err(response) = bridge.validate_request(&request) {
                response
            } else {
                match serde_json::from_slice::<CompleteTaskRequest>(&request.body) {
                    Ok(req) => {
                        let response = bridge.complete_task(req).await;
                        json_response(200, "OK", &response)
                    }
                    Err(_) => json_response(
                        400,
                        "Bad Request",
                        &serde_json::json!({ "status": "bad_request", "message": "完成任务参数解析失败。" }),
                    ),
                }
            }
        }
        Ok(request) if request.method == "POST" && request.path == "/delete-task" => {
            if let Err(response) = bridge.validate_request(&request) {
                response
            } else {
                match serde_json::from_slice::<DeleteTaskRequest>(&request.body) {
                    Ok(req) => {
                        let response = bridge.delete_task(req).await;
                        json_response(200, "OK", &response)
                    }
                    Err(_) => json_response(
                        400,
                        "Bad Request",
                        &serde_json::json!({ "status": "bad_request", "message": "删除任务参数解析失败。" }),
                    ),
                }
            }
        }
        Ok(_) => json_response(
            404,
            "Not Found",
            &serde_json::json!({ "status": "not_found", "role_response": "未知桥接路径" }),
        ),
        Err(e) => json_response(
            400,
            "Bad Request",
            &serde_json::json!({ "status": "bad_request", "role_response": e }),
        ),
    };

    write_response(stream, response).await;
}

async fn write_response(mut stream: TcpStream, response: String) {
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

async fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("读取请求失败：{}", e))?;
        if n == 0 {
            return Err("请求过早结束".to_string());
        }
        buffer.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buffer) {
            break pos;
        }
        if buffer.len() > DEFAULT_MAX_BODY_BYTES {
            return Err("请求头过大".to_string());
        }
    };

    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let headers = parse_headers(&header_text);
    let mut lines = header_text.lines();
    let first_line = lines.next().ok_or_else(|| "请求行缺失".to_string())?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let content_length = parse_content_length(&header_text).unwrap_or(0);
    let max_body_bytes = if path == "/create-skill" {
        SKILL_MAX_BODY_BYTES
    } else {
        DEFAULT_MAX_BODY_BYTES
    };
    if content_length > max_body_bytes {
        return Err("请求体过大".to_string());
    }

    while buffer.len() < header_end + content_length {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("读取请求体失败：{}", e))?;
        if n == 0 {
            return Err("请求体不完整".to_string());
        }
        buffer.extend_from_slice(&chunk[..n]);
    }

    Ok(HttpRequest {
        method,
        path,
        headers,
        body: buffer[header_end..header_end + content_length].to_vec(),
    })
}

fn parse_headers(headers: &str) -> HashMap<String, String> {
    headers
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

fn validate_request_token(request: &HttpRequest, token: &str) -> Result<(), String> {
    let expected = format!("Bearer {}", token);
    if request.headers.get("authorization").map(String::as_str) != Some(expected.as_str()) {
        return Err(json_response(
            401,
            "Unauthorized",
            &serde_json::json!({ "status": "unauthorized", "role_response": "委派失败：桥接鉴权失败。" }),
        ));
    }
    Ok(())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|pos| pos + 4)
}

fn parse_content_length(headers: &str) -> Option<usize> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

fn json_response<T: Serialize>(status: u16, reason: &str, value: &T) -> String {
    let body = serde_json::to_string(value).unwrap_or_else(|_| {
        "{\"status\":\"serialization_error\",\"role_response\":\"委派桥接响应序列化失败\"}"
            .to_string()
    });
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        reason,
        body.as_bytes().len(),
        body
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_bridge() -> (DelegateBridge, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let main = crate::db::pool::init_db(&dir.path().join("main.db")).await.unwrap();
        let conv = crate::db::pool::init_conversations_db(&dir.path().join("conv.db")).await.unwrap();
        (DelegateBridge::new(main, conv, "token".into(), None), dir)
    }

    async fn test_role(bridge: &DelegateBridge, name: &str) -> String {
        crate::db::roles::create_role(&bridge.main_pool, &crate::models::role::CreateRoleInput {
            name: name.into(), icon: None, color: None, goal: None,
        }).await.unwrap().id
    }

    fn task_request(session_id: &str, role_id: Option<String>, title: &str) -> CreateTaskRequest {
        CreateTaskRequest { session_id: session_id.into(), role_id, title: title.into(), deadline: None }
    }

    #[test]
    fn parse_content_length_accepts_case_insensitive_header() {
        let headers = "POST /delegate-to-role HTTP/1.1\r\ncontent-length: 42\r\n\r\n";
        assert_eq!(parse_content_length(headers), Some(42));
    }

    #[test]
    fn parse_headers_normalizes_names_and_trims_values() {
        let headers = "POST /delegate-to-role HTTP/1.1\r\nAuthorization:  Bearer token-123  \r\nContent-Type: application/json\r\n\r\n";
        let parsed = parse_headers(headers);

        assert_eq!(
            parsed.get("authorization").map(String::as_str),
            Some("Bearer token-123")
        );
        assert_eq!(
            parsed.get("content-type").map(String::as_str),
            Some("application/json")
        );
    }

    #[test]
    fn validate_bridge_request_rejects_missing_authorization() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/delegate-to-role".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };
        let response = validate_request_token(&request, "secret-token")
            .expect_err("request without token must fail");
        assert!(response.contains("401 Unauthorized"));
        assert!(response.contains("unauthorized"));
    }

    #[test]
    fn validate_bridge_request_accepts_instance_authorization_token() {
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Bearer secret-token".to_string(),
        );
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/delegate-to-role".to_string(),
            headers,
            body: Vec::new(),
        };

        assert!(validate_request_token(&request, "secret-token").is_ok());
    }

    #[test]
    fn validate_bridge_request_rejects_default_token() {
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Bearer egosync-local-delegate-bridge".to_string(),
        );
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/delegate-to-role".to_string(),
            headers,
            body: Vec::new(),
        };

        assert!(validate_request_token(&request, "secret-token").is_err());
    }

    #[test]
    fn find_header_end_returns_body_start() {
        let request = b"POST / HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(find_header_end(request), Some(38));
    }

    #[tokio::test]
    async fn runtime_refresh_flag_is_consumed_once() {
        // WHY: a newly created Skill must restart the process-level OpenCode
        // registry exactly once before the next message is sent.
        let (bridge, _dir) = test_bridge().await;
        bridge.request_runtime_refresh().await;
        assert!(bridge.take_runtime_refresh().await);
        assert!(!bridge.take_runtime_refresh().await);
    }

    #[test]
    fn create_task_request_accepts_camel_case_tool_payload() {
        // WHY: the opencode custom tool and Rust bridge must share the public
        // camelCase JSON contract or valid role-chat requests fail at decoding.
        let request: CreateTaskRequest = serde_json::from_value(serde_json::json!({
            "sessionId": "role-session",
            "roleId": "role-1",
            "title": "家长会"
        }))
        .unwrap();
        assert_eq!(request.session_id, "role-session");
        assert_eq!(request.role_id.as_deref(), Some("role-1"));
    }

    #[test]
    fn create_skill_request_accepts_camel_case_tool_payload() {
        // WHY: generated create_skill.ts sends camelCase and the local bridge
        // must retain the complete SKILL.md rather than a model-written path.
        let request: CreateSkillRequest = serde_json::from_value(serde_json::json!({
            "sessionId": "role-session",
            "content": "---\nname: uat-greeting\ndescription: 问候语\n---\n"
        }))
        .unwrap();
        assert_eq!(request.session_id, "role-session");
        assert!(request.content.contains("name: uat-greeting"));
    }

    #[tokio::test]
    async fn create_skill_rejects_unknown_session_before_writing() {
        let (bridge, _dir) = test_bridge().await;
        let response = bridge
            .create_skill(CreateSkillRequest {
                session_id: "missing-session".into(),
                content: "---\nname: uat-greeting\ndescription: 问候语\n---\n".into(),
            })
            .await;
        assert_eq!(response.status, "session_not_found");
        assert!(crate::db::skills::list_skills(&bridge.main_pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn role_sessions_ignore_spoofed_owner_and_do_not_cross() {
        // WHY: role identity is trusted session state, never model-controlled input.
        let (bridge, _dir) = test_bridge().await;
        let family = test_role(&bridge, "家庭").await;
        let work = test_role(&bridge, "工作").await;
        bridge.register_role_session("family-session", &family).await;
        bridge.register_role_session("work-session", &work).await;

        assert_eq!(bridge.create_task(task_request("family-session", Some(work.clone()), "家长会")).await.status, "ok");
        assert_eq!(bridge.create_task(task_request("work-session", None, "项目周报")).await.status, "ok");
        let owners: Vec<String> = sqlx::query_scalar("SELECT role_id FROM tasks ORDER BY title")
            .fetch_all(&bridge.main_pool).await.unwrap();
        assert_eq!(owners, vec![family, work]);
    }

    #[tokio::test]
    async fn butler_keeps_explicit_role_creation() {
        let (bridge, _dir) = test_bridge().await;
        let family = test_role(&bridge, "家庭").await;
        bridge.register_session("butler-session", "message-1").await;
        let response = bridge.create_task(task_request("butler-session", Some(family.clone()), "家长会")).await;
        assert_eq!(response.status, "ok");
        let owner: String = sqlx::query_scalar("SELECT role_id FROM tasks WHERE id = ?1")
            .bind(response.task_id.unwrap()).fetch_one(&bridge.main_pool).await.unwrap();
        assert_eq!(owner, family);
    }

    #[tokio::test]
    async fn unknown_session_and_deleted_role_create_no_tasks() {
        let (bridge, _dir) = test_bridge().await;
        let family = test_role(&bridge, "家庭").await;
        let missing = bridge.create_task(task_request("missing", Some(family.clone()), "家长会")).await;
        assert_eq!(missing.status, "session_not_found");
        assert!(missing.message.contains("未找到当前会话上下文"));

        bridge.register_role_session("family-session", &family).await;
        sqlx::query("DELETE FROM roles WHERE id = ?1").bind(&family)
            .execute(&bridge.main_pool).await.unwrap();
        let deleted = bridge.create_task(task_request("family-session", None, "家长会")).await;
        assert_eq!(deleted.status, "role_not_found");
        assert_eq!(deleted.message, "目标角色不存在或已不可用。");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&bridge.main_pool).await.unwrap();
        assert_eq!(count, 0);
    }
}
