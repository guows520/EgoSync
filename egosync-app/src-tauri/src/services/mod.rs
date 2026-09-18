// Story 15.1：已迁引擎的 11 个泛域 services 经 pub use 回引，
// 留守模块与 commands 层的 crate::services:: 路径引用保持不变。
// secret_store（keyring 自由函数）留守不动；engine 侧同名模块为 SecretStore trait。
// Story 15.2：再迁 14 个泛域 service（含闭包追加的 energy_calculator）+
// event_bus（EngineEvents 接缝）+ role_context（自 agent_engine 提取件），
// agent_engine 的三提取件经其顶部 use 回引。
// Story 15.3：agent_engine 与 delegate_bridge 整体迁引擎（AppHandle → 四接缝注入），
// 此处回引使 crate::services::{agent_engine, delegate_bridge} 路径语义不变（消费者零改动）。
pub use egosync_engine::services::{
    agent_bridge, agent_config, agent_engine, bigrock_protection, bigrock_reminder,
    briefing_generator, butler_config, dashboard_service, data_export, delegate_bridge,
    energy_calculator, event_bus, event_router, llm_config, mcp_server, memory_query,
    mission_inferrer, notification_service, q2_protection_reminder, review_generator, role_config,
    role_context, scheduler, sidecar, skill_registry, suggestion_generator, task_classifier,
    task_deadline_watch, task_protection_watch,
};

pub mod companion_connection;
pub mod companion_dispatch;
pub mod companion_pairing;
pub mod companion_snapshot;
pub mod memory_pipeline;
pub mod secret_store;
pub mod secret_store_keyring;
pub mod tauri_event_bus;
pub mod task_decomposition;
