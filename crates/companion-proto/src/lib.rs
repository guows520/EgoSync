//! # companion-proto —— EgoSync 手机伴侣三端共享加密协议 crate
//!
//! 本 crate 是桌面（Rust）/中继（Rust）/Android（Kotlin/Java）三端的协议单一事实源：
//!
//! - **Noise 参数套件**冻结为 `Noise_XX_25519_ChaChaPoly_BLAKE2s`（见 [`crypto::NOISE_SUITE`]）；
//! - **8 种帧类型**（HELLO / SNAPSHOT / STATE_DELTA / COMMAND / COMMAND_RESULT /
//!   STREAM_TOKEN / NOTICE / PING）payload schema 冻结于 `src/schema.json`；
//! - 当前协议版本为 [`PROTOCOL_VERSION`]。
//!
//! ## 变更流程（强制，违反即三端协议漂移）
//!
//! 任何帧类型增删或 payload 字段变更，必须：
//!
//! 1. 将 [`PROTOCOL_VERSION`] 加一（bump protocolVersion）；
//! 2. 同步更新 `src/schema.json` 中的 schema 与版本号；
//! 3. 若编解码受影响，同步重新生成 `tests/fixtures/` 下的黄金向量。
//!
//! 未 bump 版本而修改 payload 结构属于协议破坏，一律禁止。

pub mod crypto;
pub mod frames;

/// 当前协议版本（HELLO 帧 protocolVersion 的取值基准）。
pub const PROTOCOL_VERSION: u16 = 1;

/// crate 统一错误类型（桌面侧 AppError 桥接属 Story 12.2，此处不提前实现）。
#[derive(Debug)]
pub enum ProtoError {
    /// Noise 握手或加解密失败。
    Crypto(String),
    /// 帧编码失败（JSON 序列化）。
    Encode(String),
    /// 帧解码失败（长度前缀不符 / JSON 反序列化失败）。
    Decode(String),
}

impl std::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtoError::Crypto(msg) => write!(f, "加密层错误: {msg}"),
            ProtoError::Encode(msg) => write!(f, "帧编码错误: {msg}"),
            ProtoError::Decode(msg) => write!(f, "帧解码错误: {msg}"),
        }
    }
}

impl std::error::Error for ProtoError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// `schema.json` 与代码常量同步校验（AC3）。
    #[test]
    fn schema_json_is_in_sync_with_code() {
        // WHY: schema.json 是三端协议事实源，代码常量（NOISE_SUITE / PROTOCOL_VERSION /
        // 帧类型集）与其漂移意味着某端悄悄改了协议——冻结契约的同步性必须可被测试捕获。
        let schema: serde_json::Value = serde_json::from_str(include_str!("schema.json"))
            .expect("schema.json 必须是合法 JSON");

        assert_eq!(
            schema["noiseSuite"],
            serde_json::Value::String(crypto::NOISE_SUITE.to_string()),
            "schema.json noiseSuite 必须与 crypto::NOISE_SUITE 一致"
        );
        assert_eq!(
            schema["protocolVersion"],
            serde_json::Value::from(PROTOCOL_VERSION),
            "schema.json protocolVersion 必须与 PROTOCOL_VERSION 一致"
        );

        let frame_keys: Vec<&str> = schema["frames"]
            .as_object()
            .expect("schema.json 须含 frames 对象")
            .keys()
            .map(|k| k.as_str())
            .collect();
        let expected = [
            "hello",
            "snapshot",
            "state_delta",
            "command",
            "command_result",
            "stream_token",
            "notice",
            "ping",
        ];
        assert_eq!(frame_keys.len(), expected.len(), "schema.json 帧数量必须为 8");
        for name in expected {
            assert!(
                frame_keys.contains(&name),
                "schema.json 缺少帧类型 {name}"
            );
        }

        // 帧尺寸上限：schema 记录值必须与 crypto 常量一致（三端按 schema 实现，
        // 漂移意味着对端在联调时才撞墙）。
        assert_eq!(
            schema["transport"]["maxCiphertextBytes"],
            serde_json::Value::from(crypto::MAX_CIPHERTEXT_LEN as u64),
            "schema.json maxCiphertextBytes 必须与 crypto::MAX_CIPHERTEXT_LEN 一致"
        );
        assert_eq!(
            schema["transport"]["maxPlaintextBytes"],
            serde_json::Value::from(crypto::MAX_PLAINTEXT_LEN as u64),
            "schema.json maxPlaintextBytes 必须与 crypto::MAX_PLAINTEXT_LEN 一致"
        );
    }
}
