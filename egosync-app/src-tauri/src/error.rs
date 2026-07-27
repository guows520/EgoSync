#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Keyring error: {0}")]
    KeyringError(String),
    #[error("Sidecar error: {0}")]
    SidecarError(String),
    #[error("配置已保存，但运行时刷新失败: {0}")]
    RuntimeRefreshError(String),
    /// Story 10.1: 指定的 Skill 在 Registry 中不存在（已被删除或 ID 错误）。
    #[error("Skill not found: {0}")]
    SkillNotFound(String),
    /// Story 10.1: Skill 存在但未添加到当前 Agent 作用域（管家/角色未绑定）。
    #[error("Skill not added to scope: {0}")]
    SkillNotAddedToScope(String),
    /// Story 10.1: Skill 已添加到作用域但被关闭（不在 enabledSkillIds 中）。
    #[error("Skill disabled: {0}")]
    SkillDisabled(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            AppError::NotFound(msg) => map.serialize_entry("NotFound", msg)?,
            AppError::LlmError(msg) => map.serialize_entry("LlmError", msg)?,
            AppError::DbError(msg) => map.serialize_entry("DbError", msg)?,
            AppError::ValidationError(msg) => map.serialize_entry("ValidationError", msg)?,
            AppError::KeyringError(msg) => map.serialize_entry("KeyringError", msg)?,
            AppError::SidecarError(msg) => map.serialize_entry("SidecarError", msg)?,
            AppError::RuntimeRefreshError(msg) => map.serialize_entry("RuntimeRefreshError", msg)?,
            AppError::SkillNotFound(msg) => map.serialize_entry("SkillNotFound", msg)?,
            AppError::SkillNotAddedToScope(msg) => map.serialize_entry("SkillNotAddedToScope", msg)?,
            AppError::SkillDisabled(msg) => map.serialize_entry("SkillDisabled", msg)?,
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring_error_serializes_to_json() {
        let err = AppError::KeyringError("创建 keyring entry 失败: no backend".to_string());
        let json = serde_json::to_string(&err).expect("serialize should succeed");
        assert!(json.contains("KeyringError"));
        assert!(json.contains("创建 keyring entry 失败"));
    }

    #[test]
    fn test_all_variants_serialize() {
        let cases = vec![
            AppError::NotFound("x".into()),
            AppError::LlmError("x".into()),
            AppError::DbError("x".into()),
            AppError::ValidationError("x".into()),
            AppError::KeyringError("x".into()),
            AppError::SidecarError("x".into()),
            AppError::RuntimeRefreshError("x".into()),
            AppError::SkillNotFound("x".into()),
            AppError::SkillNotAddedToScope("x".into()),
            AppError::SkillDisabled("x".into()),
        ];
        for err in cases {
            let json = serde_json::to_string(&err).expect("serialize should succeed");
            let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
            assert!(parsed.is_object(), "should be JSON object: {}", json);
        }
    }
}
