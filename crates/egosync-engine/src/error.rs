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
    /// Story 12.2: 手机伴侣配对流程失败。
    #[error("配对失败: {0}")]
    PairingError(String),
    /// Story 12.2: 手机伴侣连接管理失败。
    #[error("连接服务异常: {0}")]
    ConnectionError(String),
    /// Story 12.2: 伴侣协议帧处理失败。
    #[error("通信协议错误: {0}")]
    ProtocolError(String),
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
            AppError::PairingError(msg) => map.serialize_entry("PairingError", msg)?,
            AppError::ConnectionError(msg) => map.serialize_entry("ConnectionError", msg)?,
            AppError::ProtocolError(msg) => map.serialize_entry("ProtocolError", msg)?,
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Story 15.5：键序等价属性测试（serde_json `preserve_order` 生效证明）。
    ///
    /// 构造字段声明序非字母序的样本：`to_string(to_value(x))` 必须与直接
    /// `to_string(x)` 逐字节一致——默认 BTreeMap（字母序）下两者不同，
    /// preserve_order（IndexMap 保插入序=结构体字段序）下恒等。这是
    /// HTTP body（dispatch 经 Value 往返）与桌面 invoke（直接序列化）
    /// 字节级对等的前提。
    #[test]
    fn value_roundtrip_preserves_field_order() {
        #[derive(serde::Serialize)]
        struct Sample {
            zebra: u32,
            alpha: String,
            middle: bool,
        }
        let sample = Sample {
            zebra: 1,
            alpha: "a".to_string(),
            middle: true,
        };
        let direct = serde_json::to_string(&sample).expect("直接序列化");
        let roundtrip = serde_json::to_string(&serde_json::to_value(&sample).expect("转 Value"))
            .expect("Value 序列化");
        assert_eq!(
            direct, roundtrip,
            "Value 往返不得改变键序（preserve_order 生效证据）：直接={} 往返={}",
            direct, roundtrip
        );
        assert_eq!(direct, r#"{"zebra":1,"alpha":"a","middle":true}"#);
    }

    /// 同款属性对 AppError 单键 map 的形状面：错误通道（桌面 rejection 与
    /// server 200 body）经 Value 往返后形状字节不变。
    #[test]
    fn app_error_value_roundtrip_is_stable() {
        let err = AppError::ValidationError("参数错误".into());
        let direct = serde_json::to_string(&err).expect("直接序列化");
        let value = serde_json::to_value(&err).expect("转 Value");
        let roundtrip = serde_json::to_string(&value).expect("Value 序列化");
        assert_eq!(direct, roundtrip);
    }

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
            AppError::PairingError("x".into()),
            AppError::ConnectionError("x".into()),
            AppError::ProtocolError("x".into()),
        ];
        for err in cases {
            let json = serde_json::to_string(&err).expect("serialize should succeed");
            let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
            assert!(parsed.is_object(), "should be JSON object: {}", json);
        }
    }
}
