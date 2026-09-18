use crate::error::AppError;
use egosync_engine::services::secret_store::SecretStore;

/// Story 15.1 接缝一桌面侧：keyring 实现的 SecretStore。
///
/// 委托既有 `services::secret_store` 自由函数（keyring 唯一直接引用点），
/// 错误语义与 KeyringError 路径保持现状；引擎经 trait 解除对 keyring 的物理依赖。
#[derive(Default)]
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn new() -> Self {
        Self
    }
}

impl SecretStore for KeyringSecretStore {
    fn save_secret(&self, key: &str, value: &str) -> Result<(), AppError> {
        super::secret_store::save_secret(key, value)
    }

    fn load_secret(&self, key: &str) -> Result<Option<String>, AppError> {
        super::secret_store::load_secret(key)
    }

    fn delete_secret(&self, key: &str) -> Result<(), AppError> {
        super::secret_store::delete_secret(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 接缝契约：委托不得改变既有自由函数的错误语义。
    /// keyring 服务不可用的环境下与自由函数同样降级跳过。
    #[test]
    fn keyring_secret_store_delegates_with_same_error_semantics() {
        // 空 key / 含 ':' 的 key：与自由函数同为 ValidationError
        assert!(matches!(
            KeyringSecretStore.save_secret("", "value"),
            Err(AppError::ValidationError(_))
        ));
        assert!(matches!(
            KeyringSecretStore.save_secret("bad:key", "value"),
            Err(AppError::ValidationError(_))
        ));
        // keyring 可用时 roundtrip；不可用时与自由函数同样跳过
        if KeyringSecretStore
            .save_secret("egosync_seam_test_key", "seam-value")
            .is_ok()
        {
            let loaded = KeyringSecretStore
                .load_secret("egosync_seam_test_key")
                .expect("load after save should succeed");
            assert_eq!(loaded, Some("seam-value".to_string()));
            KeyringSecretStore
                .delete_secret("egosync_seam_test_key")
                .expect("delete should succeed");
        } else {
            eprintln!("跳过: keyring 服务不可用");
        }
    }
}
