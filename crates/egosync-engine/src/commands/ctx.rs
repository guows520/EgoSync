//! Story 15.4: EngineCtx —— 宿主注入容器（接缝收拢点）。
//!
//! 命令体自壳 `commands/*.rs` 平移为引擎普通异步函数（首参 `&EngineCtx`），
//! 原先经 Tauri `State`/`AppHandle` 取值的宿主状态在此组合为单一容器：
//! 桌面壳在 setup 构造并 `manage(Arc<EngineCtx>)`（增量，既有 manage 不动），
//! server 在 bootstrap 构造（`server/src/lib.rs`）。
//!
//! 字段语义与桌面壳既有 manage 清单一一对应（同一实例的克隆共享）；
//! 路径字段为宿主注入路径（桌面 = app_data_dir 派生，server =
//! EGOSYNC_DATA_DIR 派生）。

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::registry::ChatSessionRegistry;
use crate::services::agent_bridge::AgentBridge;
use crate::services::agent_config::AgentConfigService;
use crate::services::delegate_bridge::DelegateBridge;
use crate::services::event_bus::EngineEvents;
use crate::services::event_router::EventRouter;
use crate::services::secret_store::SecretStore;
use crate::services::sidecar::SidecarManager;

pub struct EngineCtx {
    /// 主数据库池（egosync.db）。
    pub pool: DbPool,
    /// 对话日志库池（conversations.db）。
    pub conv_pool: ConversationsPool,
    /// 会话状态注册表（chat 域六组状态）。
    pub registry: Arc<ChatSessionRegistry>,
    /// opencode.json 配置服务。
    pub agent_config: AgentConfigService,
    /// opencode sidecar 进程管理器。
    pub sidecar: Arc<Mutex<SidecarManager>>,
    /// opencode HTTP 桥接。
    pub agent_bridge: AgentBridge,
    /// opencode 全局事件路由。
    pub event_router: Arc<EventRouter>,
    /// 委派桥接服务。
    pub delegate_bridge: DelegateBridge,
    /// 事件总线接缝（桌面 = TauriEventBus，server = SseEventBus）。
    pub bus: Arc<dyn EngineEvents>,
    /// 密钥存储接缝（桌面 = KeyringSecretStore，server = env/secrets.json 适配器）。
    pub secrets: Arc<dyn SecretStore>,
    /// 数据目录（桌面 = app_data_dir；server = EGOSYNC_DATA_DIR）。
    pub data_dir: PathBuf,
    /// opencode 工作区目录（`{data_dir}/opencode-workspace`）。
    pub opencode_workspace: PathBuf,
    /// 受控 Skill 根目录（`{opencode_workspace}/.opencode/skills`）。
    pub skills_root: PathBuf,
    /// 用户主目录（Skill 发现的 home 扫描根）。
    pub home_dir: PathBuf,
}

#[cfg(test)]
mod sync_tests {
    //! Story 15.4：Server 宿主接线探针——EngineCtx 必须跨 await 共享
    //! （axum/tower 中间件与 handler 均要求 Send + Sync）。

    fn assert_send_sync<T: Send + Sync + 'static>() {}

    #[test]
    fn engine_ctx_is_send_sync() {
        assert_send_sync::<super::EngineCtx>();
    }
}
