//! 黄金向量生成期的 snow 响应方：经 stdin/stdout 行协议与 noise-java 生成器交互。
//!
//! 由 `interop/noise-java/run.sh` 间接调用（`cargo run --example gen_snow_responder`），
//! 本程序不产出文件，仅作为交互式响应方。行协议：
//!
//! ```text
//! 响应方输出: KEYS <响应方静态私钥hex> <响应方临时私钥hex>
//! 发起方输入: <m1 hex>            （XX 第一步 -> e）
//! 响应方输出: <m2 hex>            （XX 第二步 <- e,ee,s,es）
//! 发起方输入: <m3 hex>            （XX 第三步 -> s,se）
//! 响应方输出: HANDSHAKE_OK
//! 发起方输入: DEC <密文hex>        （发起方加密，响应方解密并回显明文）
//! 响应方输出: DECRES <明文hex>
//! 发起方输入: ENC <明文hex>        （响应方加密并回显密文）
//! 响应方输出: ENCRES <密文hex>
//! 发起方输入: QUIT
//! ```

use std::io::{BufRead, Write};

use companion_proto::crypto::HandshakeSession;

/// 固定密钥（仅用于黄金向量生成，非生产密钥）。
const RESPONDER_STATIC_HEX: &str =
    "5e93a170f4b8d2c65a01e9f3b78d2c04a6f1e58b9302d7c4a1f8e5b3092d6c74";
const RESPONDER_EPHEMERAL_HEX: &str =
    "c07f2e5a9b1d8436e05a7c2f9d1b4e6803fa5c92b7e0d1436a9f0c58b2e7d316";

fn from_hex_32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("非法 hex 常量");
    }
    out
}

fn from_hex(s: &str) -> Vec<u8> {
    // 奇数长度是非法输入——静默截断末半字节会掩盖数据损坏的根因。
    assert!(s.len().is_multiple_of(2), "奇数长度 hex 输入: {s}");
    (0..s.len() / 2)
        .map(|i| {
            u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("非法 hex 输入")
        })
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let mut responder = HandshakeSession::responder_with_fixed_keys_for_testing(
        &from_hex_32(RESPONDER_STATIC_HEX),
        &from_hex_32(RESPONDER_EPHEMERAL_HEX),
    )
    .expect("构建响应方失败");

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut stdout = std::io::stdout();
    let mut send = |line: String| {
        writeln!(stdout, "{line}").expect("stdout 写入失败");
        stdout.flush().expect("stdout flush 失败");
    };

    // 1. 广播本侧固定密钥（由 noise-java 侧记录进向量文件）
    send(format!("KEYS {RESPONDER_STATIC_HEX} {RESPONDER_EPHEMERAL_HEX}"));

    // 2. 握手：读 m1 → 写 m2 → 读 m3
    let m1 = from_hex(lines.next().expect("输入流关闭").expect("读 m1 失败").trim());
    responder.read_message(&m1).expect("读 m1 失败");
    let m2 = responder.write_message(&[]).expect("写 m2 失败");
    send(to_hex(&m2));
    let m3 = from_hex(lines.next().expect("输入流关闭").expect("读 m3 失败").trim());
    responder.read_message(&m3).expect("读 m3 失败");
    let mut session = responder.into_transport().expect("转入传输模式失败");
    send("HANDSHAKE_OK".to_string());

    // 3. 传输期行协议
    while let Some(Ok(line)) = lines.next() {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let (cmd, arg) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match cmd {
            "DEC" => {
                let pt = session.decrypt(&from_hex(arg)).expect("解密失败");
                send(format!("DECRES {}", to_hex(&pt)));
            }
            "ENC" => {
                let ct = session.encrypt(&from_hex(arg)).expect("加密失败");
                send(format!("ENCRES {}", to_hex(&ct)));
            }
            "QUIT" => break,
            other => {
                eprintln!("未知命令: {other}");
                std::process::exit(1);
            }
        }
    }
}
