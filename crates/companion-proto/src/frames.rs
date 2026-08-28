//! 8 种帧类型定义与长度前缀编解码。
//!
//! 传输消息格式：`[u32 大端长度前缀][Noise 传输密文]`，内层为帧的 JSON 编码
//! （形如 `{"type":"hello","protocolVersion":1}`）。
//! 帧类型与 payload 字段的任何变更必须 bump protocolVersion 并同步 `src/schema.json`
//! （变更流程见 lib.rs 顶部文档）。

use serde::{Deserialize, Serialize};

use crate::crypto::{TransportSession, MAX_CIPHERTEXT_LEN};
use crate::{ProtoError, PROTOCOL_VERSION};

/// 帧类型标识，serde 序列化为小写字符串（与 `Frame` 的 `"type"` 标签取值一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameType {
    Hello,
    Snapshot,
    StateDelta,
    Command,
    CommandResult,
    StreamToken,
    Notice,
    Ping,
}

/// 协议帧。JSON 表示为 `{"type":"<小写帧名>", ...payload 字段}`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    Hello(HelloPayload),
    Snapshot(SnapshotPayload),
    StateDelta(StateDeltaPayload),
    Command(CommandPayload),
    CommandResult(CommandResultPayload),
    StreamToken(StreamTokenPayload),
    Notice(NoticePayload),
    Ping(PingPayload),
}

/// HELLO：连接建立后的首帧，必含协议版本（缺失即解码拒绝）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelloPayload {
    pub protocol_version: u16,
}

// 其余帧 payload 为最小可测结构：业务字段冻结由消费方 story 走 schema bump，
// 本 story 不做任何投机预定义（见 Dev Notes 范围外清单）。

/// SNAPSHOT：全量状态快照（内容对协议层 opaque）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotPayload {
    pub data: String,
}

/// STATE_DELTA：增量状态变化（内容对协议层 opaque）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateDeltaPayload {
    pub data: String,
}

/// COMMAND：手机 → 桌面的指令。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandPayload {
    pub data: String,
}

/// COMMAND_RESULT：指令执行结果回流。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResultPayload {
    pub data: String,
}

/// STREAM_TOKEN：流式对话的逐 token 镜像。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamTokenPayload {
    pub data: String,
}

/// NOTICE：桌面 → 手机的通知。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticePayload {
    pub data: String,
}

/// PING：保活探测，无 payload 字段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PingPayload {}

/// 编码：帧 → 内层 JSON → Noise 加密 → 加 u32 大端长度前缀。
pub fn encode_frame(frame: &Frame, session: &mut TransportSession) -> Result<Vec<u8>, ProtoError> {
    let json = serde_json::to_vec(frame).map_err(|e| ProtoError::Encode(e.to_string()))?;
    let ciphertext = session.encrypt(&json)?;
    let mut out = Vec::with_capacity(4 + ciphertext.len());
    out.extend_from_slice(&(ciphertext.len() as u32).to_be_bytes());
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// 解码：剥 u32 大端长度前缀 → Noise 解密 → JSON 反序列化 → 帧。
///
/// HELLO 帧额外执行协议版本协商校验：`protocolVersion` 必须与
/// [`PROTOCOL_VERSION`] 一致，否则拒绝（防止未协商版本的连接混入）。
pub fn decode_frame(bytes: &[u8], session: &mut TransportSession) -> Result<Frame, ProtoError> {
    if bytes.len() < 4 {
        return Err(ProtoError::Decode(format!(
            "帧数据不足 4 字节长度前缀（实际 {} 字节）",
            bytes.len()
        )));
    }
    let declared_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if declared_len > MAX_CIPHERTEXT_LEN {
        return Err(ProtoError::Decode(format!(
            "长度前缀（{declared_len}）超过单帧密文上限（{MAX_CIPHERTEXT_LEN} 字节）"
        )));
    }
    if bytes.len() - 4 != declared_len {
        return Err(ProtoError::Decode(format!(
            "长度前缀（{declared_len}）与实际密文字节数（{}）不符",
            bytes.len() - 4
        )));
    }
    let plaintext = session.decrypt(&bytes[4..])?;
    let frame: Frame =
        serde_json::from_slice(&plaintext).map_err(|e| ProtoError::Decode(e.to_string()))?;
    // 版本协商：HELLO 帧的 protocolVersion 必须与本端一致（Decision-1 裁决 A）。
    if let Frame::Hello(hello) = &frame {
        if hello.protocol_version != PROTOCOL_VERSION {
            return Err(ProtoError::Decode(format!(
                "协议版本不匹配：对端 protocolVersion={}，本端={PROTOCOL_VERSION}",
                hello.protocol_version
            )));
        }
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_static_keypair, HandshakeSession};
    use crate::PROTOCOL_VERSION;

    /// 进程内完成 XX 握手，返回（发起方, 响应方）传输会话。
    fn session_pair() -> (TransportSession, TransportSession) {
        let (i_priv, _) = generate_static_keypair().unwrap();
        let (r_priv, _) = generate_static_keypair().unwrap();
        let mut initiator = HandshakeSession::initiator(&i_priv).unwrap();
        let mut responder = HandshakeSession::responder(&r_priv).unwrap();
        let m1 = initiator.write_message(&[]).unwrap();
        responder.read_message(&m1).unwrap();
        let m2 = responder.write_message(&[]).unwrap();
        initiator.read_message(&m2).unwrap();
        let m3 = initiator.write_message(&[]).unwrap();
        responder.read_message(&m3).unwrap();
        (
            initiator.into_transport().unwrap(),
            responder.into_transport().unwrap(),
        )
    }

    /// 覆盖全部 8 种帧类型的样例。
    fn sample_frames_of_all_types() -> Vec<Frame> {
        let data = "最小可测数据-minimal".to_string();
        vec![
            Frame::Hello(HelloPayload {
                protocol_version: PROTOCOL_VERSION,
            }),
            Frame::Snapshot(SnapshotPayload { data: data.clone() }),
            Frame::StateDelta(StateDeltaPayload { data: data.clone() }),
            Frame::Command(CommandPayload { data: data.clone() }),
            Frame::CommandResult(CommandResultPayload { data: data.clone() }),
            Frame::StreamToken(StreamTokenPayload { data: data.clone() }),
            Frame::Notice(NoticePayload { data }),
            Frame::Ping(PingPayload {}),
        ]
    }

    /// AC2：8 种帧逐类型 encode→decode 往返无损。
    #[test]
    fn roundtrip_all_eight_frame_types() {
        // WHY: 帧编解码必须对所有冻结帧类型无损往返——这是三端数据一致性的地基，
        // 任何一帧往返失败都意味着协议数据在传输中被静默损坏。
        let (mut ti, mut tr) = session_pair();
        for frame in sample_frames_of_all_types() {
            let wire = encode_frame(&frame, &mut ti).expect("编码失败");
            let decoded = decode_frame(&wire, &mut tr).expect("解码失败");
            assert_eq!(decoded, frame, "帧 {:?} 往返不一致", frame);
        }
    }

    /// AC2：HELLO 缺 protocolVersion 的解码被拒绝。
    #[test]
    fn hello_missing_protocol_version_is_rejected() {
        // WHY: protocolVersion 是版本协商安全性的唯一依据——缺失该字段的 HELLO
        // 若被接受，等于允许未协商版本的连接混入，破坏 schema 冻结契约。
        let (mut ti, mut tr) = session_pair();
        let bad_json = br#"{"type":"hello"}"#;
        let ciphertext = ti.encrypt(bad_json).unwrap();
        let mut wire = (ciphertext.len() as u32).to_be_bytes().to_vec();
        wire.extend_from_slice(&ciphertext);
        let err = decode_frame(&wire, &mut tr).expect_err("缺版本字段的 HELLO 必须被拒绝");
        assert!(
            err.to_string().contains("protocolVersion"),
            "错误信息应指明缺失字段 protocolVersion，实际: {err}"
        );
    }

    /// 长度前缀与实际数据不符时解码失败（传输格式自洽性）。
    #[test]
    fn length_prefix_mismatch_is_rejected() {
        // WHY: 长度前缀是分帧的唯一依据——前缀与实际字节数不符却不报错，
        // 意味着流式分帧会静默错位，后续所有帧全部损坏。
        let (mut ti, mut tr) = session_pair();
        let frame = Frame::Ping(PingPayload {});
        let wire = encode_frame(&frame, &mut ti).unwrap();
        let mut corrupted = wire.clone();
        corrupted[3] ^= 0x01; // 扰动长度前缀最低位
        assert!(decode_frame(&corrupted, &mut tr).is_err());
    }

    /// 帧类型小写字符串序列化与 Frame 标签一致（schema.json 事实源的字节形态）。
    #[test]
    fn frame_type_serializes_as_lowercase_snake_string() {
        // WHY: FrameType 与 Frame 的 "type" 标签必须同值——两者漂移意味着
        // schema.json 记录的帧名与实际 wire 格式不一致，三端按 schema 实现即失败。
        for (frame_type, expected) in [
            (FrameType::Hello, "hello"),
            (FrameType::Snapshot, "snapshot"),
            (FrameType::StateDelta, "state_delta"),
            (FrameType::Command, "command"),
            (FrameType::CommandResult, "command_result"),
            (FrameType::StreamToken, "stream_token"),
            (FrameType::Notice, "notice"),
            (FrameType::Ping, "ping"),
        ] {
            assert_eq!(
                serde_json::to_value(frame_type).unwrap(),
                serde_json::json!(expected),
                "FrameType::{frame_type:?} 序列化应为 {expected}"
            );
        }
        // Frame 标签与 FrameType 同值抽查（专挑多词帧名：下划线分野在单词帧上测不出来）。
        let json = serde_json::to_value(Frame::StateDelta(StateDeltaPayload {
            data: "x".to_string(),
        }))
        .unwrap();
        assert_eq!(json["type"], serde_json::json!("state_delta"));
        let json = serde_json::to_value(Frame::CommandResult(CommandResultPayload {
            data: "x".to_string(),
        }))
        .unwrap();
        assert_eq!(json["type"], serde_json::json!("command_result"));
    }

    /// 版本协商：protocolVersion 与 [`PROTOCOL_VERSION`] 不匹配的 HELLO 被拒绝（Decision-1 裁决 A）。
    #[test]
    fn hello_wrong_protocol_version_is_rejected() {
        // WHY: 版本协商的安全性取决于"不匹配即拒绝"——若只校验字段存在而放行
        // 任意版本号，未协商版本的连接会静默混入，破坏 schema 冻结契约。
        let (mut ti, mut tr) = session_pair();
        let bad_json = br#"{"type":"hello","protocolVersion":999}"#;
        let ciphertext = ti.encrypt(bad_json).unwrap();
        let mut wire = (ciphertext.len() as u32).to_be_bytes().to_vec();
        wire.extend_from_slice(&ciphertext);
        let err = decode_frame(&wire, &mut tr)
            .expect_err("protocolVersion 不匹配的 HELLO 必须被拒绝");
        assert!(
            err.to_string().contains("协议版本不匹配"),
            "错误信息应指明版本不匹配，实际: {err}"
        );
    }

    /// schema 冻结的解码层强制：payload 含未知字段的帧被拒绝。
    #[test]
    fn unknown_payload_field_is_rejected() {
        // WHY: schema.json 是协议冻结契约——若未知字段被静默接受，任何一端
        // 擅自加字段都不可检测，"冻结"承诺形同虚设。
        let (mut ti, mut tr) = session_pair();
        let bad_json = br#"{"type":"ping","data":"unexpected"}"#;
        let ciphertext = ti.encrypt(bad_json).unwrap();
        let mut wire = (ciphertext.len() as u32).to_be_bytes().to_vec();
        wire.extend_from_slice(&ciphertext);
        assert!(
            decode_frame(&wire, &mut tr).is_err(),
            "含未知字段的帧必须被拒绝"
        );
    }

    /// 单帧明文超限时编码失败并返回 Encode 错误。
    #[test]
    fn oversized_frame_encode_is_rejected() {
        // WHY: SNAPSHOT 全量快照在真实场景必然超过单帧上限——超限时必须显式
        // 报错（而非产出损坏的帧或不可读的密码学错误），三端行为才有一致性。
        let (mut ti, _tr) = session_pair();
        let oversized = Frame::Snapshot(SnapshotPayload {
            data: "x".repeat(crate::crypto::MAX_PLAINTEXT_LEN + 1),
        });
        let err = encode_frame(&oversized, &mut ti).expect_err("超限帧必须编码失败");
        assert!(
            matches!(err, ProtoError::Encode(_)),
            "超限应返回 Encode 错误，实际: {err:?}"
        );
    }
}
