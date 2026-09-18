//! Story 15.1: EgoSync 引擎 crate —— 业务逻辑与宿主（Tauri/服务器）物理分层。
//!
//! 迁自 `egosync-app/src-tauri`，文件内容逐字节平移；宿主能力经接缝注入：
//! - 密钥读写：`services::secret_store::SecretStore` trait（桌面壳提供 keyring 实现）
//! - sidecar 路径：`SidecarManager::new(resource_dir, port)` 入参注入
//!
//! 物理约束（CI 断言兜底）：本 crate 不得声明 tauri 与 keyring 依赖。

pub mod db;
pub mod error;
pub mod llm;
pub mod models;
pub mod services;
