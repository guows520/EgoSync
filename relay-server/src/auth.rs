//! relay 控制协议 + 挑战-应答注册鉴权（AC2）。
//!
//! # relay 控制协议（relay 内部 app 层约定，不进 companion-proto schema.json）
//!
//! 8 帧 schema 是手机↔桌面 E2E 载荷，与本协议正交（转发态里跑的才是 E2E 帧字节）：
//!
//! 1. 客户端连上 `/relay` WS 后第一条消息必须是 **text JSON**
//!    `{"type":"register","relayId":"<16 hex>","role":"desktop"|"phone"}`；
//! 2. 随后双方交换 **3 条 binary WS 消息**承载 Noise XX 握手（每条一条 Noise
//!    消息，无长度前缀——与 12.2 桌面 WS 承载约定完全一致）：
//!    中继为 initiator（每连接一次性密钥对），客户端为 responder（真实静态私钥）；
//! 3. 握手完成即证明客户端持有其所出示静态公钥对应的私钥（挑战=握手，
//!    应答=以正确私钥完成握手；错误私钥必然握手失败）；
//! 4. 角色校验：`role=desktop` → `hex(SHA-256(pubkey))[..16] == relayId`
//!    （逐字镜像 12.2 落地实现）；`role=phone` → 仅记录公钥入槽
//!    （relay_id 归属桌面公钥哈希，手机真身份认证由 E2E Noise XX 承担）；
//! 5. 验证通过 → 登记；任何失败 → 关闭连接且**不登记**（防 ID 抢占）。
//!
//! transport 会话在鉴权完成后即弃（中继不参与后续加密——零知识纪律）。

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use companion_proto::crypto::{generate_static_keypair, HandshakeSession};
use sha2::{Digest, Sha256};
use tokio::time::Instant;

use crate::registry::Role;

/// 鉴权阶段整体超时（镜像 12.2 评审 P4：未鉴权连接不得长期占用）。
/// 整体预算——register、握手各步共享同一截止点，而非每步各 10s。
pub const AUTH_TIMEOUT_SECS: u64 = 10;

/// 握手消息下限：短于临时公钥（32 字节）即非法（12.2「短于临时公钥」判据）。
pub const MIN_HANDSHAKE_MSG_LEN: usize = 32;

/// 握手消息上限：防内存滥用（Noise XX 握手消息实际 < 200 字节）。
pub const MAX_HANDSHAKE_MSG_LEN: usize = 4096;

/// register 首消息（text JSON）尺寸上限：同上，未鉴权阶段的防御边界。
pub const MAX_REGISTER_TEXT_LEN: usize = 512;

/// 鉴权失败类别（tracing warn 只记 relay_id + 此类别，不记任何密钥材料）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    /// 首消息非 register JSON / 未知 role / relayId 非 16 位小写 hex。
    MalformedRegister,
    /// XX 握手失败（消息损坏 / 尺寸非法 / 非二进制）。
    HandshakeFailed,
    /// desktop 槽位 hash(pubkey) != relay_id（防 ID 抢占）。
    DesktopIdMismatch,
    /// 鉴权阶段超时。
    Timeout,
    /// 对端提前断开。
    Disconnected,
}

/// 鉴权通过的客户端身份。
pub struct AuthenticatedClient {
    pub relay_id: String,
    pub role: Role,
    pub remote_static_pubkey: Vec<u8>,
}

/// 解析并校验 register 首消息。
pub fn parse_register(text: &str) -> Result<(String, Role), AuthError> {
    #[derive(serde::Deserialize)]
    struct RawRegister {
        #[serde(rename = "type")]
        msg_type: String,
        #[serde(rename = "relayId")]
        relay_id: String,
        role: String,
    }
    let raw: RawRegister = serde_json::from_str(text).map_err(|_| AuthError::MalformedRegister)?;
    if raw.msg_type != "register" {
        return Err(AuthError::MalformedRegister);
    }
    if !is_valid_relay_id(&raw.relay_id) {
        return Err(AuthError::MalformedRegister);
    }
    let role = match raw.role.as_str() {
        "desktop" => Role::Desktop,
        "phone" => Role::Phone,
        _ => return Err(AuthError::MalformedRegister),
    };
    Ok((raw.relay_id, role))
}

/// relay_id 推导（逐字镜像 12.2 落地实现：
/// `hex_encode(&Sha256::digest(desktop_static_pubkey))[..16]`，小写 hex、前 16 字符）。
pub fn derive_relay_id(desktop_static_pubkey: &[u8]) -> String {
    hex_encode(&Sha256::digest(desktop_static_pubkey))[..16].to_string()
}

/// 16 位小写 hex 判定（relay_id 合法格式）。
fn is_valid_relay_id(s: &str) -> bool {
    s.len() == 16
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 距鉴权截止点的剩余预算；已到/已过截止点返回 `Err(Timeout)`。
fn remaining(deadline: &Instant) -> Result<Duration, AuthError> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or(AuthError::Timeout)
}

/// 在 WS 连接上执行完整挑战-应答鉴权（register → XX 握手 → 角色校验）。
///
/// `timeout` 为**鉴权阶段整体预算**：register、握手收发的每一步都从同一
/// 截止点取剩余时间（评审整改：此前每步各享独立超时，最坏占用翻倍）。
/// 失败即返回错误，由调用方关闭连接（不登记）。
pub async fn authenticate(
    socket: &mut WebSocket,
    timeout: Duration,
) -> Result<AuthenticatedClient, AuthError> {
    let deadline = Instant::now() + timeout;

    // 1. 首消息必须是 register JSON（text）
    let first = tokio::time::timeout(remaining(&deadline)?, socket.recv())
        .await
        .map_err(|_| AuthError::Timeout)?
        .ok_or(AuthError::Disconnected)?;
    let text = match first {
        Ok(Message::Text(text)) => text,
        Ok(_) => return Err(AuthError::MalformedRegister),
        Err(_) => return Err(AuthError::Disconnected),
    };
    if text.len() > MAX_REGISTER_TEXT_LEN {
        return Err(AuthError::MalformedRegister);
    }
    let (relay_id, role) = parse_register(&text)?;

    // 2. 中继生成一次性密钥对，作为 XX initiator 发起挑战
    let (relay_priv, _relay_pub) =
        generate_static_keypair().map_err(|_| AuthError::HandshakeFailed)?;
    let mut session =
        HandshakeSession::initiator(&relay_priv).map_err(|_| AuthError::HandshakeFailed)?;

    // XX 三步：-> e ； <- e,ee,s,es ； -> s,se
    let m1 = session
        .write_message(&[])
        .map_err(|_| AuthError::HandshakeFailed)?;
    tokio::time::timeout(remaining(&deadline)?, socket.send(Message::Binary(m1.into())))
        .await
        .map_err(|_| AuthError::Timeout)?
        .map_err(|_| AuthError::Disconnected)?;

    // 3. 客户端应答（以真实静态私钥完成的 m2；错误私钥必然握手失败）
    let m2_msg = tokio::time::timeout(remaining(&deadline)?, socket.recv())
        .await
        .map_err(|_| AuthError::Timeout)?
        .ok_or(AuthError::Disconnected)?;
    let m2 = match m2_msg {
        Ok(Message::Binary(bytes)) => bytes,
        Ok(_) => return Err(AuthError::HandshakeFailed),
        Err(_) => return Err(AuthError::Disconnected),
    };
    if !(MIN_HANDSHAKE_MSG_LEN..=MAX_HANDSHAKE_MSG_LEN).contains(&m2.len()) {
        return Err(AuthError::HandshakeFailed);
    }
    session
        .read_message(&m2)
        .map_err(|_| AuthError::HandshakeFailed)?;

    let m3 = session
        .write_message(&[])
        .map_err(|_| AuthError::HandshakeFailed)?;
    tokio::time::timeout(remaining(&deadline)?, socket.send(Message::Binary(m3.into())))
        .await
        .map_err(|_| AuthError::Timeout)?
        .map_err(|_| AuthError::Disconnected)?;

    // 4. 对端静态公钥（m2 携带 responder 静态公钥，此刻可读）
    let remote_static_pubkey = session
        .remote_static_pubkey()
        .ok_or(AuthError::HandshakeFailed)?;

    // 5. 角色校验：desktop 严格校验 hash==relay_id；phone 记录公钥入槽
    //（phone 槽的公钥绑定判据由 Registry.register 承担，D1-a）
    match role {
        Role::Desktop => {
            if derive_relay_id(&remote_static_pubkey) != relay_id {
                return Err(AuthError::DesktopIdMismatch);
            }
        }
        Role::Phone => {}
    }

    Ok(AuthenticatedClient {
        relay_id,
        role,
        remote_static_pubkey,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// relay_id 必须是 16 位小写 hex（与 12.2 桌面 QR payload 口径一致）。
    #[test]
    fn derive_relay_id_is_16_lowercase_hex() {
        // WHY: relay_id 是三端约定标识——长度/大小写任一漂移都会导致
        // 桌面推导值与中继校验值不相等，配对永远失败。
        let pubkey = [7u8; 32];
        let relay_id = derive_relay_id(&pubkey);
        assert_eq!(relay_id.len(), 16);
        assert!(
            relay_id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "必须是小写 hex: {relay_id}"
        );
    }

    /// 推导逻辑必须与 12.2 桌面落地实现逐字一致（SHA-256 → 小写 hex → 前 16 字符）。
    #[test]
    fn derive_relay_id_mirrors_reference_impl() {
        // WHY: 同一公钥在桌面（QR 生成）与中继（槽位校验）两侧必须推导出
        // 同一 relay_id——截断长度或 hex 大小写任何差异都是配对协议破坏。
        let pubkey: Vec<u8> = (0u8..32).collect();
        let digest = Sha256::digest(&pubkey);
        let mut expected = String::new();
        for b in digest.iter() {
            expected.push_str(&format!("{b:02x}"));
        }
        let expected = expected[..16].to_string();
        assert_eq!(derive_relay_id(&pubkey), expected);
    }

    /// 合法 register JSON（desktop / phone 两种角色）解析成功。
    #[test]
    fn parse_register_accepts_valid_messages() {
        // WHY: 控制协议是中继与两端客户端的唯一入口约定——
        // 合法消息被误拒会让所有正常配对无法建立。
        let (relay_id, role) =
            parse_register(r#"{"type":"register","relayId":"aabbccddeeff0011","role":"desktop"}"#)
                .expect("合法 desktop register 必须通过");
        assert_eq!(relay_id, "aabbccddeeff0011");
        assert_eq!(role, Role::Desktop);

        let (_, role) =
            parse_register(r#"{"type":"register","relayId":"aabbccddeeff0011","role":"phone"}"#)
                .expect("合法 phone register 必须通过");
        assert_eq!(role, Role::Phone);
    }

    /// 非法 register（错误 type / 未知 role / 非法 relayId / 畸形 JSON）一律拒绝。
    #[test]
    fn parse_register_rejects_malformed_messages() {
        // WHY: 注册入口是 ID 抢占的唯一通道——格式校验在最外层收紧，
        // 任何非法输入都不得进入握手阶段消耗资源。
        // 错误 type
        assert_eq!(
            parse_register(r#"{"type":"hello","relayId":"aabbccddeeff0011","role":"desktop"}"#),
            Err(AuthError::MalformedRegister)
        );
        // 未知 role
        assert_eq!(
            parse_register(r#"{"type":"register","relayId":"aabbccddeeff0011","role":"tablet"}"#),
            Err(AuthError::MalformedRegister)
        );
        // relayId 长度错误
        assert_eq!(
            parse_register(r#"{"type":"register","relayId":"aabb","role":"desktop"}"#),
            Err(AuthError::MalformedRegister)
        );
        // relayId 非法字符
        assert_eq!(
            parse_register(r#"{"type":"register","relayId":"zzbbccddeeff0011","role":"desktop"}"#),
            Err(AuthError::MalformedRegister)
        );
        // relayId 大写（必须小写 hex，与推导口径一致）
        assert_eq!(
            parse_register(r#"{"type":"register","relayId":"AABBCCDDEEFF0011","role":"desktop"}"#),
            Err(AuthError::MalformedRegister)
        );
        // 畸形 JSON
        assert_eq!(
            parse_register("not json at all"),
            Err(AuthError::MalformedRegister)
        );
    }

    /// remaining()：截止点已过 → Timeout；未到 → 返回剩余时长。
    #[test]
    fn remaining_budget_enforces_single_deadline() {
        // WHY: 「整体超时」承诺的落点——若 remaining 对已过期截止点仍返回
        // Ok，某一步就能在总预算耗尽后继续占用连接（P4 防御边界失守）。
        let past = Instant::now() - Duration::from_secs(1);
        assert_eq!(remaining(&past), Err(AuthError::Timeout));

        let future = Instant::now() + Duration::from_secs(5);
        let budget = remaining(&future).expect("未到截止点必须返回剩余预算");
        assert!(budget <= Duration::from_secs(5), "剩余预算不得超过总预算");
    }
}
