use crate::error::AppError;

const SERVICE_NAME: &str = "com.egosync.app";

/// 校验 key 名称合法性
fn validate_key(key: &str) -> Result<(), AppError> {
    if key.is_empty() {
        return Err(AppError::ValidationError("密钥名称不能为空".to_string()));
    }
    if key.len() > 255 {
        return Err(AppError::ValidationError(format!(
            "密钥名称过长: {} 字符（最大 255）",
            key.len()
        )));
    }
    if key.contains(':') {
        return Err(AppError::ValidationError(format!(
            "密钥名称不能包含 ':' 字符: '{}'",
            key
        )));
    }
    Ok(())
}

/// 创建 keyring Entry，使用显式 target 确保跨调用一致性
fn create_entry(key: &str) -> Result<keyring::Entry, AppError> {
    validate_key(key)?;
    let target = format!("{}:{}", SERVICE_NAME, key);
    keyring::Entry::new_with_target(&target, SERVICE_NAME, key)
        .map_err(|e| AppError::KeyringError(format!("创建 keyring entry 失败: {}", e)))
}

/// 保存密钥到系统钥匙串（已存在则覆写）
pub fn save_secret(key: &str, value: &str) -> Result<(), AppError> {
    let entry = create_entry(key)?;
    entry
        .set_password(value)
        .map_err(|e| AppError::KeyringError(format!("保存密钥失败: {}", e)))?;
    tracing::info!(key = key, "密钥已安全存储到系统钥匙串");
    Ok(())
}

/// 从系统钥匙串加载密钥
pub fn load_secret(key: &str) -> Result<Option<String>, AppError> {
    let entry = create_entry(key)?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::KeyringError(format!("读取密钥失败: {}", e))),
    }
}

/// 从系统钥匙串删除密钥
pub fn delete_secret(key: &str) -> Result<(), AppError> {
    let entry = create_entry(key)?;
    match entry.delete_credential() {
        Ok(()) => {
            tracing::info!(key = key, "密钥已从系统钥匙串删除");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::KeyringError(format!("删除密钥失败: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &str = "egosync_test_key";
    const TEST_VALUE: &str = "sk-test-secret-value-12345";

    #[test]
    fn test_save_and_load() {
        // 清理可能残留的测试数据
        let _ = delete_secret(TEST_KEY);

        // 保存
        save_secret(TEST_KEY, TEST_VALUE).expect("save_secret should succeed");

        // 读取并验证
        let loaded = load_secret(TEST_KEY).expect("load_secret should succeed");
        assert_eq!(loaded, Some(TEST_VALUE.to_string()));

        // 清理
        let _ = delete_secret(TEST_KEY);
    }

    #[test]
    fn test_load_nonexistent() {
        let key = "egosync_test_nonexistent_key";
        // 确保不存在
        let _ = delete_secret(key);

        let result = load_secret(key).expect("load_secret should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn test_delete() {
        let key = "egosync_test_delete_key";
        // 写入
        save_secret(key, "temp_value").expect("save should succeed");
        // 删除
        delete_secret(key).expect("delete should succeed");
        // 验证已删除
        let result = load_secret(key).expect("load should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn test_delete_nonexistent() {
        let key = "egosync_test_delete_nonexistent";
        // 确保不存在
        let _ = delete_secret(key);
        // 删除不存在的 key 应该返回 Ok
        let result = delete_secret(key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_key_rejected() {
        let result = save_secret("", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_colon_in_key_rejected() {
        let result = save_secret("bad:key", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_overlong_key_rejected() {
        let long_key = "a".repeat(256);
        let result = save_secret(&long_key, "value");
        assert!(result.is_err());
    }
}
