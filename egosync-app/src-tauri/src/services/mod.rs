// Story 15.1：已迁引擎的 11 个泛域 services 经 pub use 回引，
// 留守模块与 commands 层的 crate::services:: 路径引用保持不变。
// secret_store（keyring 自由函数）留守不动；engine 侧同名模块为 SecretStore trait。
pub use egosync_engine::services::{
    agent_bridge, agent_config, butler_config, dashboard_service, data_export, llm_config,
    mcp_server, memory_query, role_config, sidecar, skill_registry,
};

pub mod agent_engine;
pub mod bigrock_protection;
pub mod bigrock_reminder;
pub mod briefing_generator;
pub mod companion_connection;
pub mod companion_dispatch;
pub mod companion_pairing;
pub mod companion_snapshot;
pub mod delegate_bridge;
pub mod energy_calculator;
pub mod event_router;
pub mod memory_pipeline;
pub mod mission_inferrer;
pub mod notification_service;
pub mod q2_protection_reminder;
pub mod review_generator;
pub mod secret_store;
pub mod secret_store_keyring;
pub mod suggestion_generator;
pub mod task_classifier;
pub mod task_decomposition;
pub mod task_deadline_watch;
pub mod scheduler;
pub mod task_protection_watch;
