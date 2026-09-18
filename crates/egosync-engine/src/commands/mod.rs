//! Story 15.4: 命令层 —— 宿主无关命令体模块。
//!
//! 原壳 `egosync-app/src-tauri/src/commands/*.rs` 中 web-ok 的 106 条命令体
//! 平移至此（首参 `&EngineCtx`）；12 条 desktop-only（rfd 对话框 / data_import /
//! companion_*）与 2 条 perf-test 门控命令留壳不迁。命令体为普通 pub async
//! 函数（零 tauri 依赖），桌面壳以薄 wrapper 注册 generate_handler，server
//! 经生成的 dispatch registry 路由（`server/src/dispatch_gen.rs`）。
//!
//! 平移纪律：业务逻辑/锁结构/错误文案/SQL 零改动；允许改动仅五类——
//! State/AppHandle 取值改 ctx 字段、emit 改总线+events 常量、
//! 壳运行时的 spawn 原语改 tokio::spawn、路径函数改 ctx 注入
//! 路径、use 路径调整。

pub mod app;
pub mod briefing;
pub mod chat;
pub mod ctx;
pub mod dashboard;
pub mod data;
pub mod llm_config;
pub mod memory;
pub mod mcp;
pub mod mission;
pub mod notification;
pub mod review;
pub mod role;
pub mod scheduler;
pub mod secret;
pub mod settings;
pub mod skill;
pub mod suggestion;
pub mod task;
pub mod task_decomposition;
