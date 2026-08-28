//! Noise XX 加密封装——全仓唯一 `use snow` 的位置。
//!
//! 冻结参数套件 [`NOISE_SUITE`]：`Noise_XX_25519_ChaChaPoly_BLAKE2s`。
//! 上层（frames.rs 及后续 story 的消费方）只经由 [`HandshakeSession`] /
//! [`TransportSession`] 操作握手与加解密，不接触任何密码学细节。

use snow::{Builder, HandshakeState, TransportState};

use crate::ProtoError;

/// 冻结的 Noise 参数套件（三端一致；变更等同协议 break，须走版本 bump 流程）。
pub const NOISE_SUITE: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// snow 单条消息缓冲上限（Noise 协议规范的消息尺寸上限）。
const MAX_MESSAGE_LEN: usize = 65535;

/// 单条传输消息的密文上限（字节）——等于 snow 缓冲上限。
pub const MAX_CIPHERTEXT_LEN: usize = MAX_MESSAGE_LEN;

/// 单条传输消息的明文上限（字节）——密文上限减去 16 字节 ChaChaPoly 认证标签。
/// 超过此长度的明文无法编码为单帧，调用方须分片。
pub const MAX_PLAINTEXT_LEN: usize = MAX_MESSAGE_LEN - 16;

/// X25519 私钥长度（字节）。
const KEY_LEN: usize = 32;

fn noise_params() -> Result<snow::params::NoiseParams, ProtoError> {
    NOISE_SUITE
        .parse()
        .map_err(|_| ProtoError::Crypto(format!("非法 Noise 参数套件: {NOISE_SUITE}")))
}

/// 生成一对 X25519 静态密钥，返回 `(私钥, 公钥)`。
pub fn generate_static_keypair() -> Result<(Vec<u8>, Vec<u8>), ProtoError> {
    let kp = Builder::new(noise_params()?)
        .generate_keypair()
        .map_err(|e| ProtoError::Crypto(e.to_string()))?;
    Ok((kp.private, kp.public))
}

/// XX 握手会话（发起方或响应方）。
pub struct HandshakeSession {
    state: HandshakeState,
}

impl HandshakeSession {
    /// 以本地静态私钥构建发起方。
    pub fn initiator(local_static_private: &[u8]) -> Result<Self, ProtoError> {
        Self::build(local_static_private, None, true)
    }

    /// 以本地静态私钥构建响应方。
    pub fn responder(local_static_private: &[u8]) -> Result<Self, ProtoError> {
        Self::build(local_static_private, None, false)
    }

    /// 仅供测试与黄金向量生成：固定静态+临时私钥构建发起方（字节级可重放）。
    pub fn initiator_with_fixed_keys_for_testing(
        local_static_private: &[u8],
        local_ephemeral_private: &[u8],
    ) -> Result<Self, ProtoError> {
        Self::build(local_static_private, Some(local_ephemeral_private), true)
    }

    /// 仅供测试与黄金向量生成：固定静态+临时私钥构建响应方（字节级可重放）。
    pub fn responder_with_fixed_keys_for_testing(
        local_static_private: &[u8],
        local_ephemeral_private: &[u8],
    ) -> Result<Self, ProtoError> {
        Self::build(local_static_private, Some(local_ephemeral_private), false)
    }

    fn build(
        local_static_private: &[u8],
        local_ephemeral_private: Option<&[u8]>,
        initiator: bool,
    ) -> Result<Self, ProtoError> {
        if local_static_private.len() != KEY_LEN {
            return Err(ProtoError::Crypto(format!(
                "静态私钥长度必须为 {KEY_LEN} 字节（实际 {} 字节）",
                local_static_private.len()
            )));
        }
        let mut builder = Builder::new(noise_params()?)
            .local_private_key(local_static_private)
            .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        if let Some(ephemeral) = local_ephemeral_private {
            if ephemeral.len() != KEY_LEN {
                return Err(ProtoError::Crypto(format!(
                    "临时私钥长度必须为 {KEY_LEN} 字节（实际 {} 字节）",
                    ephemeral.len()
                )));
            }
            builder = builder.fixed_ephemeral_key_for_testing_only(ephemeral);
        }
        let state = if initiator {
            builder.build_initiator()
        } else {
            builder.build_responder()
        }
        .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        Ok(HandshakeSession { state })
    }

    /// 写出一条握手消息，返回完整消息字节（XX 模式三步：`-> e`、`<- e,ee,s,es`、`-> s,se`）。
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, ProtoError> {
        let mut buf = [0u8; MAX_MESSAGE_LEN];
        let n = self
            .state
            .write_message(payload, &mut buf)
            .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        Ok(buf[..n].to_vec())
    }

    /// 读入对端的一条握手消息，返回其负载字节。
    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>, ProtoError> {
        let mut buf = [0u8; MAX_MESSAGE_LEN];
        let n = self
            .state
            .read_message(message, &mut buf)
            .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        Ok(buf[..n].to_vec())
    }

    /// 对端静态公钥（XX 握手交换静态密钥的消息之后可读；此前为 `None`）。
    ///
    /// Story 12.2 透传：桌面侧配对决策需要以对端静态公钥为设备身份，
    /// 而 `use snow` 被冻结在 crate 内——上层只能经此 fn 获取。
    pub fn remote_static_pubkey(&self) -> Option<Vec<u8>> {
        self.state.get_remote_static().map(|k| k.to_vec())
    }

    /// 握手三步全部完成后转入传输模式。
    pub fn into_transport(self) -> Result<TransportSession, ProtoError> {
        let state = self
            .state
            .into_transport_mode()
            .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        Ok(TransportSession { state })
    }
}

/// Noise 传输态会话：双向加解密（收发方向各自独立 nonce 计数）。
pub struct TransportSession {
    state: TransportState,
}

impl TransportSession {
    /// 加密一条传输消息（返回 Noise 传输密文）。
    ///
    /// # Errors
    /// - [`ProtoError::Encode`]：明文超过 [`MAX_PLAINTEXT_LEN`]。
    /// - [`ProtoError::Crypto`]：snow 加密失败。
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, ProtoError> {
        if plaintext.len() > MAX_PLAINTEXT_LEN {
            return Err(ProtoError::Encode(format!(
                "明文 {} 字节超过单帧上限 {} 字节（须分片）",
                plaintext.len(),
                MAX_PLAINTEXT_LEN
            )));
        }
        let mut buf = [0u8; MAX_MESSAGE_LEN];
        let n = self
            .state
            .write_message(plaintext, &mut buf)
            .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        Ok(buf[..n].to_vec())
    }

    /// 解密一条传输消息（返回明文字节）。
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, ProtoError> {
        let mut buf = [0u8; MAX_MESSAGE_LEN];
        let n = self
            .state
            .read_message(ciphertext, &mut buf)
            .map_err(|e| ProtoError::Crypto(e.to_string()))?;
        Ok(buf[..n].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC4：进程内 XX 双角色握手 + 派生会话密钥 + 互发加密帧且解密一致。
    #[test]
    fn xx_handshake_both_roles_then_transport_roundtrip() {
        // WHY: Noise XX 握手是三端加密互通的地基——若双角色在进程内都无法完成
        // 握手并派生出一致会话密钥，跨语言互通无从谈起。
        let (i_priv, _i_pub) = generate_static_keypair().unwrap();
        let (r_priv, _r_pub) = generate_static_keypair().unwrap();

        let mut initiator =
            HandshakeSession::initiator(&i_priv).expect("构建发起方失败");
        let mut responder =
            HandshakeSession::responder(&r_priv).expect("构建响应方失败");

        // XX 三步：-> e ； <- e,ee,s,es ； -> s,se
        let m1 = initiator.write_message(&[]).unwrap();
        responder.read_message(&m1).unwrap();
        let m2 = responder.write_message(&[]).unwrap();
        initiator.read_message(&m2).unwrap();
        let m3 = initiator.write_message(&[]).unwrap();
        responder.read_message(&m3).unwrap();

        let mut ti = initiator.into_transport().unwrap();
        let mut tr = responder.into_transport().unwrap();

        // 发起方 → 响应方
        let secret_from_i = b"initiator -> responder secret";
        let ct_i = ti.encrypt(secret_from_i).unwrap();
        assert_eq!(tr.decrypt(&ct_i).unwrap(), secret_from_i);

        // 响应方 → 发起方
        let secret_from_r = b"responder -> initiator secret";
        let ct_r = tr.encrypt(secret_from_r).unwrap();
        assert_eq!(ti.decrypt(&ct_r).unwrap(), secret_from_r);
    }

    /// 非法长度的静态私钥必须被拒绝，而非静默零填充或内部 panic。
    #[test]
    fn wrong_length_private_key_is_rejected() {
        // WHY: X25519 密钥固定 32 字节——超长在 snow 内部切片越界（panic），
        // 短于 32 字节被静默零填充后会以另一把密钥建立会话（静默错钥）。
        // 两种形态都必须在入口以类型化错误拒绝。
        let too_long = [0u8; 33];
        assert!(
            HandshakeSession::initiator(&too_long).is_err(),
            "33 字节私钥必须被拒绝"
        );
        assert!(
            HandshakeSession::responder(&too_long).is_err(),
            "33 字节私钥（响应方）必须被拒绝"
        );
        let too_short = [0u8; 31];
        assert!(
            HandshakeSession::initiator(&too_short).is_err(),
            "31 字节私钥必须被拒绝"
        );
        let empty: [u8; 0] = [];
        assert!(
            HandshakeSession::initiator(&empty).is_err(),
            "空私钥必须被拒绝"
        );
    }

    /// 超过单帧明文上限的加密请求必须返回 Encode 错误（而非不可读的 Crypto 错误）。
    #[test]
    fn oversized_plaintext_is_rejected_with_encode_error() {
        // WHY: 单帧明文上限是协议冻结契约的一部分——超限时若静默截断或报
        // 密码学错误，三端各自实现的帧尺寸行为将漂移；必须显式返回编码错误。
        let (i_priv, _) = generate_static_keypair().unwrap();
        let mut initiator = HandshakeSession::initiator(&i_priv).unwrap();
        // 走完握手进入传输态（此处仅校验长度校验，握手本身由上方测试覆盖）
        let (r_priv, _) = generate_static_keypair().unwrap();
        let mut responder = HandshakeSession::responder(&r_priv).unwrap();
        let m1 = initiator.write_message(&[]).unwrap();
        responder.read_message(&m1).unwrap();
        let m2 = responder.write_message(&[]).unwrap();
        initiator.read_message(&m2).unwrap();
        let m3 = initiator.write_message(&[]).unwrap();
        responder.read_message(&m3).unwrap();
        let mut ti = initiator.into_transport().unwrap();

        let oversized = vec![0u8; MAX_PLAINTEXT_LEN + 1];
        let err = ti
            .encrypt(&oversized)
            .expect_err("超限明文必须被拒绝");
        assert!(
            matches!(err, ProtoError::Encode(_)),
            "超限应返回 Encode 错误，实际: {err:?}"
        );
    }
}
