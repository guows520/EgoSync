//! 生成 snow 侧黄金向量：`cargo run --example gen_snow_vectors`（在 crate 根目录运行）。
//!
//! 使用固定静态+临时密钥，产出完全确定性的字节级可重放向量，
//! 写入 `tests/fixtures/snow_vectors.json`（产物随仓库提交，仅开发期重新生成）。

use companion_proto::crypto::HandshakeSession;
use companion_proto::frames::Frame;
use companion_proto::{crypto, frames, PROTOCOL_VERSION};

/// 固定密钥（仅用于黄金向量，非生产密钥）。
const INITIATOR_STATIC_HEX: &str =
    "8f3a9c21d47be65a10fc93d2a7b48e1f5c6d0a93e27f4b18c5d92036af71be4d";
const INITIATOR_EPHEMERAL_HEX: &str =
    "01a4f8c92b73e5d60418ba9f3d27c5e81047f9a2b6c38d51e094f7a2c1b5e830";
const RESPONDER_STATIC_HEX: &str =
    "37c2e81a5f9d0b46628ad13e9c05f7b48a3d1e6072c9f4b5a8d3e01629f7c4b8";
const RESPONDER_EPHEMERAL_HEX: &str =
    "b9d4702e1c8a5f3694bd07e23a5c9f180d6b42e97a1c05f3d8e62094b7a3c5f1";

fn from_hex_32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("非法 hex 常量");
    }
    out
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 覆盖 8 种帧类型的明文（与 frames.rs 契约一致）。
fn frame_plaintexts() -> Vec<Vec<u8>> {
    let data = |s: &str| s.to_string();
    let frames = [
        Frame::Hello(frames::HelloPayload {
            protocol_version: PROTOCOL_VERSION,
        }),
        Frame::Snapshot(frames::SnapshotPayload {
            data: data("snapshot-全量快照"),
        }),
        Frame::StateDelta(frames::StateDeltaPayload {
            data: data("delta-增量-1"),
        }),
        Frame::Command(frames::CommandPayload {
            data: data("cmd-1"),
        }),
        Frame::CommandResult(frames::CommandResultPayload {
            data: data("result-ok"),
        }),
        Frame::StreamToken(frames::StreamTokenPayload {
            data: data("token-1"),
        }),
        Frame::Notice(frames::NoticePayload {
            data: data("notice-1"),
        }),
        Frame::Ping(frames::PingPayload {}),
    ];
    frames
        .iter()
        .map(|f| serde_json::to_vec(f).expect("帧 JSON 序列化失败"))
        .collect()
}

fn main() -> std::io::Result<()> {
    let mut initiator = HandshakeSession::initiator_with_fixed_keys_for_testing(
        &from_hex_32(INITIATOR_STATIC_HEX),
        &from_hex_32(INITIATOR_EPHEMERAL_HEX),
    )
    .expect("构建发起方失败");
    let mut responder = HandshakeSession::responder_with_fixed_keys_for_testing(
        &from_hex_32(RESPONDER_STATIC_HEX),
        &from_hex_32(RESPONDER_EPHEMERAL_HEX),
    )
    .expect("构建响应方失败");

    // XX 三步：-> e ； <- e,ee,s,es ； -> s,se
    let m1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&m1).unwrap();
    let m2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&m2).unwrap();
    let m3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&m3).unwrap();

    let mut ti = initiator.into_transport().unwrap();
    let mut tr = responder.into_transport().unwrap();

    let plaintexts = frame_plaintexts();
    // 先全部 i2r 再全部 r2i，保证 nonce 计数顺序与测试重放一致
    let mut transport = Vec::new();
    for pt in &plaintexts {
        let ct = ti.encrypt(pt).unwrap();
        transport.push(serde_json::json!({
            "direction": "i2r",
            "plaintext": to_hex(pt),
            "ciphertext": to_hex(&ct),
        }));
    }
    for pt in &plaintexts {
        let ct = tr.encrypt(pt).unwrap();
        transport.push(serde_json::json!({
            "direction": "r2i",
            "plaintext": to_hex(pt),
            "ciphertext": to_hex(&ct),
        }));
    }

    let doc = serde_json::json!({
        "suite": crypto::NOISE_SUITE,
        "generator": "snow 0.10 (examples/gen_snow_vectors.rs)",
        "initiator": {
            "staticPrivate": INITIATOR_STATIC_HEX,
            "ephemeralPrivate": INITIATOR_EPHEMERAL_HEX,
        },
        "responder": {
            "staticPrivate": RESPONDER_STATIC_HEX,
            "ephemeralPrivate": RESPONDER_EPHEMERAL_HEX,
        },
        "handshakeMessages": [to_hex(&m1), to_hex(&m2), to_hex(&m3)],
        "transport": transport,
    });

    std::fs::create_dir_all("tests/fixtures")?;
    std::fs::write(
        "tests/fixtures/snow_vectors.json",
        serde_json::to_string_pretty(&doc)?,
    )?;
    println!("已生成 tests/fixtures/snow_vectors.json");
    Ok(())
}
