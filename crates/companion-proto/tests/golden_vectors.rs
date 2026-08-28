//! 跨语言黄金向量互验（AC5）。
//!
//! 两套向量 fixtures 均在此做字节级重放验证：
//! - `snow_vectors.json`：snow 0.10 双侧生成，验证自身确定性；
//! - `noise_java_vectors.json`：noise-java（发起方）与 snow（响应方）真实跨语言握手
//!   产生的转录与密文——snow 必须能逐字节复现 noise-java 的握手消息与密文。

use companion_proto::crypto::HandshakeSession;
use companion_proto::frames::Frame;
use serde_json::Value;

const SNOW_VECTORS: &str = include_str!("fixtures/snow_vectors.json");
const NOISE_JAVA_VECTORS: &str = include_str!("fixtures/noise_java_vectors.json");

struct VectorSet {
    initiator_static: Vec<u8>,
    initiator_ephemeral: Vec<u8>,
    responder_static: Vec<u8>,
    responder_ephemeral: Vec<u8>,
    handshake: Vec<Vec<u8>>,
    transport: Vec<(String, Vec<u8>, Vec<u8>)>,
    snow_decrypted_java: Option<bool>,
    java_decrypted_snow: Option<bool>,
}

fn from_hex(s: &str) -> Vec<u8> {
    // WHY: 奇数长度是损坏的向量——若静默截断末尾半字节，字节级比对会给出
    // 误导性结果，掩盖向量损坏的真正根因。
    assert!(s.len().is_multiple_of(2), "向量含奇数长度 hex（{s}）");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("向量含非法 hex"))
        .collect()
}

fn load(doc: &str) -> VectorSet {
    let v: Value = serde_json::from_str(doc).expect("向量文件必须是合法 JSON");
    VectorSet {
        initiator_static: from_hex(v["initiator"]["staticPrivate"].as_str().unwrap()),
        initiator_ephemeral: from_hex(v["initiator"]["ephemeralPrivate"].as_str().unwrap()),
        responder_static: from_hex(v["responder"]["staticPrivate"].as_str().unwrap()),
        responder_ephemeral: from_hex(v["responder"]["ephemeralPrivate"].as_str().unwrap()),
        handshake: v["handshakeMessages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| from_hex(m.as_str().unwrap()))
            .collect(),
        transport: v["transport"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["direction"].as_str().unwrap().to_string(),
                    from_hex(t["plaintext"].as_str().unwrap()),
                    from_hex(t["ciphertext"].as_str().unwrap()),
                )
            })
            .collect(),
        snow_decrypted_java: v["generationTimeVerification"]["snowDecryptedJavaCiphertexts"]
            .as_bool(),
        java_decrypted_snow: v["generationTimeVerification"]["javaDecryptedSnowCiphertexts"]
            .as_bool(),
    }
}

/// 字节级重放：用向量中的固定密钥重建双方，逐步断言握手消息与传输密文逐字节一致。
fn replay_and_verify(v: &VectorSet) {
    assert_eq!(v.handshake.len(), 3, "XX 握手必须恰为 3 条消息");
    // WHY: 空的传输段会让互验测试空洞通过——AC5 的互验承诺要求双向都有真实密文，
    // 否则"向量互验"可名存实亡。
    assert!(!v.transport.is_empty(), "向量传输段不能为空");
    assert!(
        v.transport.iter().any(|(d, _, _)| d == "i2r"),
        "向量必须包含至少一条 i2r 传输记录"
    );
    assert!(
        v.transport.iter().any(|(d, _, _)| d == "r2i"),
        "向量必须包含至少一条 r2i 传输记录"
    );
    let mut ti = HandshakeSession::initiator_with_fixed_keys_for_testing(
        &v.initiator_static,
        &v.initiator_ephemeral,
    )
    .expect("构建发起方失败");
    let mut tr = HandshakeSession::responder_with_fixed_keys_for_testing(
        &v.responder_static,
        &v.responder_ephemeral,
    )
    .expect("构建响应方失败");

    let (m1, m2, m3) = (&v.handshake[0], &v.handshake[1], &v.handshake[2]);
    assert_eq!(ti.write_message(&[]).unwrap(), *m1, "发起方 m1 字节不一致");
    tr.read_message(m1).expect("响应方读 m1 失败");
    assert_eq!(tr.write_message(&[]).unwrap(), *m2, "响应方 m2 字节不一致");
    ti.read_message(m2).expect("发起方读 m2 失败");
    assert_eq!(ti.write_message(&[]).unwrap(), *m3, "发起方 m3 字节不一致");
    tr.read_message(m3).expect("响应方读 m3 失败");

    let mut ti = ti.into_transport().expect("发起方转入传输模式失败");
    let mut tr = tr.into_transport().expect("响应方转入传输模式失败");

    for (direction, pt, ct) in &v.transport {
        match direction.as_str() {
            "i2r" => {
                assert_eq!(
                    ti.encrypt(pt).unwrap(),
                    *ct,
                    "i2r 方向重加密密文与向量不一致"
                );
                assert_eq!(tr.decrypt(ct).unwrap(), *pt, "i2r 方向解密明文不一致");
            }
            "r2i" => {
                assert_eq!(
                    tr.encrypt(pt).unwrap(),
                    *ct,
                    "r2i 方向重加密密文与向量不一致"
                );
                assert_eq!(ti.decrypt(ct).unwrap(), *pt, "r2i 方向解密明文不一致");
            }
            other => panic!("向量含未知方向: {other}"),
        }
        // 向量明文必须是合法帧——把加密层验证与帧协议契约挂钩
        serde_json::from_slice::<Frame>(pt).expect("向量明文必须是合法帧 JSON");
    }
}

/// AC5-①：snow 向量字节级重放一致。
#[test]
fn snow_vectors_byte_level_replay() {
    // WHY: 向量重放是协议实现确定性的回归锚点——snow 升级后若握手或加密
    // 产出任何字节级漂移，此测试立即报警，防止静默的密码学行为变更。
    replay_and_verify(&load(SNOW_VECTORS));
}

/// AC5-②：noise-java 向量经 snow 字节级复现 + 解密一致。
#[test]
fn noise_java_vectors_interop() {
    // WHY: 跨语言加密互通是本 story 要出清的最大技术风险——snow 必须能逐字节
    // 复现 noise-java 参与产生的握手转录与密文；任何密码学实现分歧（nonce、
    // MAC、密钥派生）都会在此暴露，而非遗留到 Android 真机联调时才爆发。
    let v = load(NOISE_JAVA_VECTORS);
    replay_and_verify(&v);

    // 生成期互验标记必须为真：向量文件由真实跨语言会话产出，
    // 双向解密在生成时均已验证（snow 解密 java 密文 / java 解密 snow 密文）。
    assert_eq!(
        v.snow_decrypted_java,
        Some(true),
        "生成期 snow 解密 noise-java 密文必须已验证"
    );
    assert_eq!(
        v.java_decrypted_snow,
        Some(true),
        "生成期 noise-java 解密 snow 密文必须已验证"
    );
}
