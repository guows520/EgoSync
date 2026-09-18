use crate::error::AppError;

/// 宿主密钥存储接缝（Story 15.1 接缝一）。
///
/// 引擎内所有密钥读写必须经此 trait；桌面壳提供 keyring 实现
/// （`egosync-app/src-tauri/src/services/secret_store_keyring.rs`），
/// 未来 server 宿主提供自己的实现。
///
/// 方法签名与错误语义与桌面壳原 `services::secret_store.rs` 自由函数一致：
/// - `save_secret`：已存在则覆写
/// - `load_secret`：不存在返回 `Ok(None)`
/// - `delete_secret`：不存在幂等返回 `Ok(())`
pub trait SecretStore: Send + Sync {
    /// 保存密钥（已存在则覆写）
    fn save_secret(&self, key: &str, value: &str) -> Result<(), AppError>;

    /// 加载密钥；不存在返回 `Ok(None)`
    fn load_secret(&self, key: &str) -> Result<Option<String>, AppError>;

    /// 删除密钥；不存在幂等返回 `Ok(())`
    fn delete_secret(&self, key: &str) -> Result<(), AppError>;
}
