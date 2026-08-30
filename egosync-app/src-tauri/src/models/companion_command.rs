//! Story 13.3：手机伴侣指令 envelope 模型。
//!
//! 指令业务结构走 app 层 JSON 塞进 `Frame::Command/CommandResult.data`
//! （对协议层 opaque），不进 companion-proto crate、不 bump
//! PROTOCOL_VERSION（13.1 裁决 1 先例）。schema 冻结于本 story Dev Notes §2，
//! 双端共同事实源。

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 指令 envelope schema 版本（与手机端 CommandModels.kt 对齐）。
pub const COMMAND_SCHEMA_VERSION: u16 = 1;

/// 单帧明文上限：直接引用 companion-proto 事实源（评审整改：硬编码副本
/// 会在 proto 调整上限时静默漂移）；app 层在此显式校验，超限报错而非
/// 产出会被编码层拒绝的帧。
pub const COMMAND_DATA_MAX_BYTES: usize = companion_proto::crypto::MAX_PLAINTEXT_LEN;

/// COMMAND.data（手机 → 桌面）：`{"schemaVersion":1,"commandId":"<uuid>","action":"...","params":{...}}`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEnvelope {
    pub schema_version: u16,
    pub command_id: String,
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// COMMAND_RESULT.data（桌面 → 手机）：成功/失败单形态。
/// error 为 AppError 单键 map 序列化形状 `{"<Variant>":"<msg>"}`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAck {
    pub schema_version: u16,
    pub command_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

impl CommandEnvelope {
    /// 解析并校验 envelope：超限/坏 JSON/schema 版本漂移/缺 commandId
    /// 均显式报错（AC2——不静默、不悬挂）。
    pub fn parse(data: &str) -> Result<Self, AppError> {
        if data.len() > COMMAND_DATA_MAX_BYTES {
            return Err(AppError::ValidationError(format!(
                "指令超出单帧上限（{} > {} 字节）",
                data.len(),
                COMMAND_DATA_MAX_BYTES
            )));
        }
        let envelope: Self = serde_json::from_str(data)
            .map_err(|e| AppError::ValidationError(format!("指令格式无效: {}", e)))?;
        if envelope.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(AppError::ValidationError(format!(
                "不支持的指令 schema 版本: {}",
                envelope.schema_version
            )));
        }
        if envelope.command_id.trim().is_empty() {
            return Err(AppError::ValidationError("缺少指令 ID".to_string()));
        }
        Ok(envelope)
    }

}

impl CommandAck {
    pub fn success(command_id: &str, result: serde_json::Value) -> Self {
        Self {
            schema_version: COMMAND_SCHEMA_VERSION,
            command_id: command_id.to_string(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    /// 失败 ack：error 为 AppError 的单键 map 序列化形状。
    pub fn failure(command_id: &str, error: AppError) -> Self {
        Self {
            schema_version: COMMAND_SCHEMA_VERSION,
            command_id: command_id.to_string(),
            ok: false,
            result: None,
            error: Some(serde_json::to_value(&error).unwrap_or_else(|_| {
                serde_json::json!({"ValidationError": "指令执行失败"})
            })),
        }
    }

    /// 序列化为 COMMAND_RESULT.data 的 JSON 字符串。
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"schemaVersion":1,"commandId":"","ok":false,"error":{"ValidationError":"指令结果序列化失败"}}"#.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_parses_valid_camel_case_payload() {
        // WHY: envelope 是双端冻结契约——字段名/嵌套形状任何一侧漂移，
        // 对端解码即失败（deny_unknown_fields 下加字段=三端全炸）。
        let data = r#"{"schemaVersion":1,"commandId":"cmd-1","action":"chat.send","params":{"content":"你好"}}"#;
        let env = CommandEnvelope::parse(data).expect("合法 envelope 必须解析成功");
        assert_eq!(env.command_id, "cmd-1");
        assert_eq!(env.action, "chat.send");
        assert_eq!(env.params["content"], "你好");
    }

    #[test]
    fn envelope_rejects_missing_command_id() {
        // WHY: commandId 是幂等去重与结果关联的唯一键——缺失时若静默
        // 接受，执行结果将永远无法回流给等待方（手机悬挂）。
        // 字段缺失由 serde 必填校验拒绝、空串由显式校验拒绝，均为
        // ValidationError。
        let data = r#"{"schemaVersion":1,"action":"chat.send","params":{}}"#;
        let err = CommandEnvelope::parse(data).expect_err("缺 commandId 必须被拒绝");
        assert!(matches!(err, AppError::ValidationError(_)));

        let data = r#"{"schemaVersion":1,"commandId":"  ","action":"chat.send","params":{}}"#;
        let err = CommandEnvelope::parse(data).expect_err("空白 commandId 必须被拒绝");
        assert!(
            matches!(err, AppError::ValidationError(ref m) if m.contains("指令 ID")),
            "空白 commandId 应给出缺 ID 错误: {err:?}"
        );
    }

    #[test]
    fn envelope_rejects_oversized_payload() {
        // WHY: 超限 envelope 塞进 data 后编码层才会拒绝——届时表现为整条
        // 连接断开；app 层前置校验让手机拿到显式错误而非断流。
        let big_params = "x".repeat(COMMAND_DATA_MAX_BYTES);
        let data = format!(
            r#"{{"schemaVersion":1,"commandId":"c","action":"chat.send","params":{{"content":"{}"}}}}"#,
            big_params
        );
        let err = CommandEnvelope::parse(&data).expect_err("超限必须被拒绝");
        assert!(matches!(err, AppError::ValidationError(ref m) if m.contains("单帧上限")));
    }

    #[test]
    fn envelope_rejects_bad_json_and_version_drift_and_unknown_fields() {
        assert!(CommandEnvelope::parse("not json").is_err(), "坏 JSON 必须被拒绝");
        let wrong_version =
            r#"{"schemaVersion":2,"commandId":"c","action":"chat.send","params":{}}"#;
        assert!(
            CommandEnvelope::parse(wrong_version).is_err(),
            "schema 版本漂移必须被拒绝"
        );
        let unknown_field =
            r#"{"schemaVersion":1,"commandId":"c","action":"chat.send","params":{},"extra":1}"#;
        assert!(
            CommandEnvelope::parse(unknown_field).is_err(),
            "未知字段必须被拒绝（deny_unknown_fields 语义对齐）"
        );
    }

    #[test]
    fn ack_serializes_to_frozen_shapes() {
        // WHY: ack 形状是手机 CommandChannel 解析的事实源——ok/result/error
        // 的 camelCase 与单键 error map 任何漂移都会让手机端解析失败。
        let ok = CommandAck::success("cmd-1", serde_json::json!({"conversationId": "c1"}));
        let json = ok.to_json();
        assert!(json.contains(r#""schemaVersion":1"#));
        assert!(json.contains(r#""commandId":"cmd-1""#));
        assert!(json.contains(r#""ok":true"#));
        assert!(json.contains(r#""result":{"conversationId":"c1"}"#));
        assert!(!json.contains("error"), "成功 ack 不携带 error 字段");

        let fail = CommandAck::failure("cmd-2", AppError::ValidationError("参数无效".into()));
        let json = fail.to_json();
        assert!(json.contains(r#""ok":false"#));
        assert!(
            json.contains(r#""error":{"ValidationError":"参数无效"}"#),
            "error 必须是 AppError 单键 map 形状: {json}"
        );
        assert!(!json.contains("result"), "失败 ack 不携带 result 字段");
    }
}
