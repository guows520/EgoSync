//! Story 15.4 二轮评审修复 #14：壳 wrapper 薄化层往返测试。
//!
//! WHY: wrapper（`#[tauri::command]` + `State<Arc<EngineCtx>>` 透传）此前
//! 零自动化——同型参数互换（如 app_set_setting 的 key/value 对调）不会
//! 令任何门禁变红，因为引擎单测按位置直传而 wrapper 按名解包。本测试
//! 经壳 wrapper 写一个正常键再取回断言相等（互换会把值写进键位、取回
//! None ⇒ 红），顺带断言保留键经 wrapper 仍被拒（引擎守卫跨 wrapper
//! 生效的接线证据）。
//!
//! 范式沿用 test_chat_busy_mutex.rs：mock_app + manage(Arc<EngineCtx>)
//! 直调——busy-mutex 之外的装配（HangingOpencode 等 chat 专属依赖）
//! 不需要，settings 路径只触双池与 ctx 容器。

use std::sync::Arc;

use egosync_lib::commands::app::{app_get_setting, app_set_setting};
use egosync_lib::db::pool::{init_conversations_db, init_db};
use egosync_lib::error::AppError;
use egosync_lib::services::agent_bridge::AgentBridge;
use egosync_lib::services::agent_config::AgentConfigService;
use egosync_lib::services::delegate_bridge::DelegateBridge;
use egosync_lib::services::event_router::EventRouter;
use egosync_lib::services::sidecar::SidecarManager;
use egosync_lib::services::tauri_event_bus::TauriEventBus;
use tokio::sync::Mutex;
use tauri::Manager;

use egosync_engine::commands::ctx::EngineCtx;
use egosync_engine::commands::chat::ChatSessionRegistry;

/// 密钥接缝测试替身：settings 路径不触真实密钥库。
struct NoopSecretStore;
impl egosync_engine::services::secret_store::SecretStore for NoopSecretStore {
    fn save_secret(&self, _key: &str, _value: &str) -> Result<(), AppError> { Ok(()) }
    fn load_secret(&self, _key: &str) -> Result<Option<String>, AppError> { Ok(None) }
    fn delete_secret(&self, _key: &str) -> Result<(), AppError> { Ok(()) }
}

/// mock_app 注入 EngineCtx 单容器（与 lib.rs 生产装配同构——settings
/// 路径只依赖双池，sidecar/bridge 为容器完整性占位）。
async fn setup() -> (tauri::App<tauri::test::MockRuntime>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pool = init_db(&dir.path().join("main.db"))
        .await
        .expect("init main db with migrations");
    let conv_pool = init_conversations_db(&dir.path().join("conv.db"))
        .await
        .expect("init conversations db with migrations");

    let app = tauri::test::mock_app();
    let registry = Arc::new(ChatSessionRegistry::default());
    let event_bus = TauriEventBus::new(app.handle().clone());
    let agent_config = AgentConfigService::new(dir.path().join("opencode.json"));
    let sidecar = Arc::new(Mutex::new(SidecarManager::new(None, None)));
    let agent_bridge = AgentBridge::new(0);
    let event_router = Arc::new(EventRouter::new());
    let delegate_bridge = DelegateBridge::new(
        pool.clone(),
        conv_pool.clone(),
        "test-token".into(),
        None,
        agent_config.clone(),
        dir.path().join("skills"),
        Arc::new(NoopSecretStore),
    );
    app.manage(Arc::new(EngineCtx {
        pool,
        conv_pool,
        registry,
        agent_config,
        sidecar,
        agent_bridge,
        event_router,
        delegate_bridge,
        bus: Arc::new(event_bus),
        secrets: Arc::new(NoopSecretStore),
        data_dir: dir.path().to_path_buf(),
        opencode_workspace: dir.path().join("opencode-workspace"),
        skills_root: dir.path().join("skills"),
        home_dir: dir.path().to_path_buf(),
    }));
    (app, dir)
}

#[tokio::test]
async fn wrapper_set_get_setting_roundtrip_and_reserved_key_rejected() {
    let (app, _dir) = setup().await;
    let ctx = app.state::<Arc<EngineCtx>>();

    // 正常键往返：经壳 wrapper 写 → 经壳 wrapper 读 → 相等
    //（key/value 若在 wrapper 解包层被互换，取回将为 None ⇒ 红）
    app_set_setting(ctx.clone(), "wrapper_test_key".to_string(), "wrapper-test-value".to_string())
        .await
        .expect("wrapper 写正常键");
    let got = app_get_setting(ctx.clone(), "wrapper_test_key".to_string())
        .await
        .expect("wrapper 读正常键");
    assert_eq!(got.as_deref(), Some("wrapper-test-value"), "往返必须相等（互换即 None）");

    // 未知键读取：None（不误报默认值）
    let absent = app_get_setting(ctx.clone(), "wrapper_absent_key".to_string())
        .await
        .expect("wrapper 读未知键");
    assert_eq!(absent, None, "未知键必须 None");

    // 保留键经 wrapper 仍被拒（引擎守卫跨接线生效）
    let err = app_get_setting(ctx.clone(), "server_token_hash".to_string())
        .await
        .expect_err("保留键读必须拒绝（经 wrapper）");
    match err {
        AppError::ValidationError(msg) => {
            assert!(msg.contains("保留键不可读取"), "文案对齐: {}", msg)
        }
        other => panic!("读拒绝必须 ValidationError，实得 {:?}", other),
    }
    let err = app_set_setting(
        ctx.clone(),
        "server_token_hash".to_string(),
        "attacker".to_string(),
    )
    .await
    .expect_err("保留键写必须拒绝（经 wrapper）");
    match err {
        AppError::ValidationError(msg) => {
            assert!(msg.contains("保留键不可写入"), "文案对齐: {}", msg)
        }
        other => panic!("写拒绝必须 ValidationError，实得 {:?}", other),
    }
}
