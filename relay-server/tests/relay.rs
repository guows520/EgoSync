//! Story 12.3 集成测试（AC2、AC3）。
//!
//! 测试基建：进程内 `axum::serve`（随机端口），测试客户端用 `tokio-tungstenite`
//! 直连 + `companion_proto` 完成 responder 侧握手；零持久化 / 零磁盘写 / 日志纪律
//! 断言使用真子进程（`CARGO_BIN_EXE_relay-server`）。
//!
//! 测试策略（规则九）：双客户端模拟的是「两个诚实的端」——转发态测试载荷直接
//! 喂确定字节（中继视角本就是不可解密文），断言**逐字节相等**而非"能收到"，
//! 验证「中继不可读」是结构性保证。

mod common;

use std::time::Duration;

use common::{InProcessServer, RelayClient};
use relay_server::auth::derive_relay_id;
use relay_server::{build_router, build_router_with, AppState};

/// AC1：`/healthz` 返回 200。
#[tokio::test]
async fn healthz_returns_200() {
    let server = InProcessServer::start(build_router()).await;
    let resp = server.healthz().await;
    assert_eq!(resp.status, 200, "healthz 必须返回 200");
}

/// AC2：注册成功路径——双端注册后双向二进制帧转发往返逐字节一致。
#[tokio::test]
async fn registration_and_bidirectional_forwarding() {
    // WHY: 中继的价值主张是「零知识纯转发」——若中继对帧有任何解析/修改，
    // 逐字节断言立即失败；双向各测一次，排除单向代理的侥幸实现。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    // desktop → phone：逐字节一致
    let payload_d2p: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(7).wrapping_add(3)).collect();
    desktop.send_binary(&payload_d2p).await;
    let got = phone.recv_binary().await;
    assert_eq!(got, payload_d2p, "desktop→phone 转发必须逐字节一致");

    // phone → desktop：逐字节一致
    let payload_p2d: Vec<u8> = (0..48u8).map(|i| i.wrapping_mul(13).wrapping_add(5)).collect();
    phone.send_binary(&payload_p2d).await;
    let got = desktop.recv_binary().await;
    assert_eq!(got, payload_p2d, "phone→desktop 转发必须逐字节一致");
}

/// AC2 防抢占：desktop 槽位错误私钥（公钥 hash ≠ relay_id）→ 拒绝且不登记，
/// 后续正确客户端仍可注册成功。
#[tokio::test]
async fn wrong_desktop_key_rejected_then_correct_client_succeeds() {
    // WHY: relay_id 归属桌面公钥哈希——若任何人持任意密钥就能占住 desktop 槽，
    // 恶意方即可对目标 relay_id 发起拒绝服务。抢占必须被校验挡下，
    // 且抢占失败不得产生持续占用（正确客户端仍能注册）。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (wrong_priv, _) = common::keypair(); // 公钥 hash ≠ relay_id

    // 恶意客户端：持错误私钥、声称目标 relay_id 的 desktop 槽
    let mut squatter =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &wrong_priv).await;
    assert!(
        squatter.is_closed().await,
        "错误私钥的 desktop 注册必须被关闭"
    );

    // 正确客户端仍可注册成功 + 完整转发（抢占无残留）
    let (p_priv, _) = common::keypair();
    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    let payload = b"post-squat forwarding works";
    desktop.send_binary(payload).await;
    assert_eq!(phone.recv_binary().await, payload.to_vec());
}

/// AC2：握手消息损坏（短于 32 字节，12.2 判据）→ 拒绝。
#[tokio::test]
async fn corrupted_handshake_message_rejected() {
    // WHY: Noise XX 首消息（-> e）无 MAC，全零 64 字节会被当合法消息接受导致
    // 死锁（12.2 Debug Log #2）；「短于临时公钥即非法」是握手入口的硬边界。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (_d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);

    let mut client = RelayClient::connect(&url).await;
    client
        .send_text(&format!(
            r#"{{"type":"register","relayId":"{relay_id}","role":"desktop"}}"#
        ))
        .await;
    // 收到中继的 m1（挑战）后，以 8 字节垃圾应答
    let _m1 = client.recv_binary().await;
    client.send_raw_binary(&[0u8; 8]).await;
    assert!(client.is_closed().await, "损坏的握手应答必须被拒绝");
}

/// AC2：首消息非 register JSON → 拒绝（控制协议只在注册首消息出现一次）。
#[tokio::test]
async fn non_register_first_message_rejected() {
    // WHY: register 首消息是控制协议的唯一入口约定——放行任意首消息
    // 意味着中继对未声明身份的连接开放资源。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let mut client = RelayClient::connect(&url).await;
    client.send_raw_binary(&[1, 2, 3]).await;
    assert!(client.is_closed().await, "非 register 首消息必须被拒绝");

    let mut client = RelayClient::connect(&url).await;
    client.send_text(r#"{"type":"hello"}"#).await;
    assert!(client.is_closed().await, "未知消息类型必须被拒绝");
}

/// AC2/AC4 Task 4：转发态收到 text 消息 → 忽略（不转发、不崩溃），binary 通道不受影响。
#[tokio::test]
async fn text_message_in_forwarding_state_is_ignored() {
    // WHY: 控制协议只在注册首消息出现一次——转发态的 text 是协议违例，
    // 中继的态度是忽略+warn 而非断连（帧通道的可用性优先）。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    desktop.send_text("should be ignored").await;
    // text 不被转发：phone 随后收到的必须是之后的 binary，而非 text
    let payload = b"binary still flows";
    desktop.send_binary(payload).await;
    assert_eq!(
        phone.recv_binary().await,
        payload.to_vec(),
        "text 必须被忽略，binary 通道不受影响"
    );
}

/// AC2/AC3：一端断开 → 对端收到 close（感知断线并走重连）。
#[tokio::test]
async fn disconnect_propagates_close_to_peer() {
    // WHY: 中继不做离线投递——对端断线的唯一正确通知方式就是 close，
    // 否则幸存端将面对一个静默黑洞直到自身超时。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    drop(phone); // 手机侧直接断开
    assert!(
        desktop.is_closed().await,
        "对端断开后，desktop 必须收到 close"
    );
}

/// AC2/AC3：单槽替换——同角色二次注册 → 旧连接被关闭，新连接正常接管。
#[tokio::test]
async fn slot_replacement_closes_old_connection() {
    // WHY: 12.2 评审 P6——僵尸会话（TCP 未及时断开）不得阻挡新连接；
    // 新连接必须即时取代旧槽，且旧连接必须感知被取代而释放。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut old_desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut new_desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;

    assert!(
        old_desktop.is_closed().await,
        "同角色二次注册后，旧连接必须被关闭"
    );

    // 新连接正常接管：phone 注册后与 new_desktop 双向转发
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    let payload = b"new desktop owns the slot";
    new_desktop.send_binary(payload).await;
    assert_eq!(phone.recv_binary().await, payload.to_vec());
}

/// AC3 零持久化：进程重启后注册表为空——同 relay_id 全新注册成功、
/// 无旧对端残留转发、无离线投递（未注册期间的消息不补发）。
#[tokio::test]
async fn zero_persistence_across_process_restart() {
    // WHY: 中继的隐私承诺建立在「失忆」上——重启后若残留任何注册状态，
    // 就意味着存在某种未声明的持久化路径（零磁盘写承诺同时被打破）。
    let port = common::free_port();

    // 进程 A：注册 + 转发成功
    let mut server_a = common::SubprocessServer::start(port);
    server_a.wait_ready();
    let url_a = server_a.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url_a, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url_a, &relay_id, "phone", &p_priv).await;
    let payload = b"before restart";
    desktop.send_binary(payload).await;
    assert_eq!(phone.recv_binary().await, payload.to_vec());
    server_a.kill();

    // 进程 B：同端口重启——对该 relay_id 一无所知
    let mut server_b = common::SubprocessServer::start(port);
    server_b.wait_ready();
    let url_b = server_b.relay_url();

    // 同 relay_id + 正确密钥可全新注册成功（旧槽不残留、不拒绝）
    let mut desktop2 =
        RelayClient::register_and_handshake(&url_b, &relay_id, "desktop", &d_priv).await;
    // 无离线投递：phone 注册前发的消息不得被补发
    let stale = b"sent while peer absent";
    desktop2.send_binary(stale).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut phone2 =
        RelayClient::register_and_handshake(&url_b, &relay_id, "phone", &p_priv).await;
    // 新连接只收到注册之后的帧（stale 不得出现）
    let fresh = b"after fresh registration";
    desktop2.send_binary(fresh).await;
    assert_eq!(
        phone2.recv_binary().await,
        fresh.to_vec(),
        "全新注册后转发正常，且无离线补发"
    );
}

/// AC3 零磁盘写：子进程以空临时目录为工作目录，注册+转发+断开全流程后
/// 扫描该目录无任何新增文件。
#[tokio::test]
async fn zero_disk_write_in_subprocess() {
    // WHY: 「中继不落地」是可部署的隐私承诺——若中继在运行中写任何文件
    // （日志、缓存、临时状态），VPS 运营方即可获得远超零知识承诺的数据。
    let dir = std::env::temp_dir().join(format!(
        "egosync-relay-zerodisk-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&dir).expect("创建临时目录失败");

    let port = common::free_port();
    let mut server = common::SubprocessServer::start_with(port, Some(&dir), None);
    server.wait_ready();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();
    let url = server.relay_url();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    let payload = b"forwarded through zero-disk relay";
    desktop.send_binary(payload).await;
    assert_eq!(phone.recv_binary().await, payload.to_vec());
    drop(desktop);
    drop(phone);
    // 给连接清理一点时间
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    server.kill();
    assert!(
        dir_is_empty(&dir),
        "全流程结束后工作目录必须无任何新增文件（零磁盘写）"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// NFR-M7 日志纪律：relay 全部 tracing 输出只含 relay_id/角色/字节数/事件类别，
/// 帧字节与任何密钥材料永不入日志（以 RUST_LOG=debug 全量捕获 stderr 断言）。
#[tokio::test]
async fn log_discipline_no_payload_or_keys_in_logs() {
    // WHY: 日志是零知识边界最常见的意外泄漏面——转发统计若顺手打出帧内容
    // 或密钥，中继就从「零知识」退化为「明文归档」。debug 级全量捕获下
    // 断言哨兵载荷与密钥材料均不可见，是最强的结构性自查。
    let port = common::free_port();
    let mut server = common::SubprocessServer::start_with(port, None, Some("debug"));
    server.wait_ready();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, p_pub) = common::keypair();
    let url = server.relay_url();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    // 哨兵载荷：若任何日志路径打出帧字节（raw 或 hex），此处立即暴露
    let sentinel: &[u8] = b"EGOSYNC-RELAY-SENTINEL-PAYLOAD!!";
    desktop.send_binary(sentinel).await;
    assert_eq!(phone.recv_binary().await, sentinel.to_vec());
    drop(desktop);
    drop(phone);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let logs = server.take_stderr();

    // 正向控制：日志确实在记录（relay_id 出现 = 事件元数据在流动）
    assert!(logs.contains(&relay_id), "relay_id 应作为事件元数据出现");

    // 帧字节不入日志（原始 ASCII 与 hex 两种形态）
    assert!(
        !logs.contains("EGOSYNC-RELAY-SENTINEL"),
        "帧明文不得入日志"
    );
    assert!(
        !logs.contains(&common::hex_encode(sentinel)),
        "帧 hex 形式不得入日志"
    );

    // 任何密钥材料不入日志（双端公私钥 hex）
    for key_hex in [
        common::hex_encode(&d_pub),
        common::hex_encode(&p_pub),
        common::hex_encode(&d_priv),
        common::hex_encode(&p_priv),
    ] {
        assert!(!logs.contains(&key_hex), "密钥材料不得入日志");
    }
}

fn dir_is_empty(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
}

// ══════════════════ 评审整改回归测试（Review Findings） ══════════════════

/// D1-a：phone 槽公钥绑定——第三方持不同公钥抢占被拒，合法会话不受影响。
#[tokio::test]
async fn phone_slot_squatting_with_different_key_rejected() {
    // WHY: relay_id 非秘密（16 位 hex，本地 QR 传输仍可能泄露）——若 phone
    // 槽不绑定公钥，抢占者可把合法手机反复挤下线、令其沦为收不到帧的
    // 黑洞；绑定后抢占者连注册都过不去，且拒绝不产生持续占用。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    // 抢占者：持另一把密钥冒充 phone（握手能完成，但公钥与已绑槽位不符）
    let (evil_priv, _) = common::keypair();
    let mut squatter =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &evil_priv).await;
    assert!(
        squatter.is_closed().await,
        "不同公钥的 phone 注册必须被关闭（防抢占 D1-a）"
    );

    // 合法会话不受影响：转发照常
    let payload = b"legit session unaffected";
    desktop.send_binary(payload).await;
    assert_eq!(phone.recv_binary().await, payload.to_vec());
}

/// D1-a 补充：phone 同公钥重连（断线重连场景）照常走单槽替换。
#[tokio::test]
async fn phone_reconnect_with_same_key_replaces_slot() {
    // WHY: 公钥绑定不得误伤合法手机的断线重连——同一把密钥的二次注册
    // 必须保留 P6 单槽替换语义（旧连接被取代、新连接接管转发）。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let old_phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    drop(old_phone); // 手机断线（不优雅，直接掉）

    // 同公钥重连：必须成功且接管槽位
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    let payload = b"reconnected phone owns the slot";
    desktop.send_binary(payload).await;
    assert_eq!(phone.recv_binary().await, payload.to_vec());
}

/// P2：有界转发通道——对端消费停滞 → 发送方断连（不做无界积压）。
#[tokio::test]
async fn slow_consumer_disconnects_sender() {
    // WHY: 无界通道时代，慢/半死对端让消息无限积压 + 任务永久挂起，
    // AC5「内存与连接数线性」的承诺被打破——有界化后，对端不消费
    // 必须转化为断连（断线即丢语义），而非内存无限增长。
    let server = InProcessServer::start(build_router()).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    // phone 注册后不再读取（模拟消费停滞：不收数据、不回 Pong）

    // 洪泛超过「通道容量 + 内核缓冲」量级，逼满 phone 侧有界通道
    let payload = vec![0xCDu8; 1024];
    for _ in 0..4096 {
        if !desktop.try_send_binary(&payload).await {
            break; // 服务端已断连（正是被测行为）
        }
    }
    assert!(
        desktop.is_closed().await,
        "对端消费停滞时发送方必须断连（有界通道满即断）"
    );
    drop(phone);
}

/// P4 整改：鉴权超时是「阶段整体预算」——register 拖延不得给 m2 续借预算。
#[tokio::test]
async fn auth_timeout_is_a_total_budget() {
    // WHY: 若每步各享独立超时，恶意客户端拖到最后一刻才发 register、
    // 再在握手阶段消失，连接总占用 = 2×预算（P4 防御边界翻倍失守）；
    // 整体预算下无论怎么拖延，占用都封顶在预算内。
    let state = AppState {
        auth_timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let server = InProcessServer::start(build_router_with(state)).await;
    let url = server.relay_url();

    let (_d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);

    let mut client = RelayClient::connect(&url).await;
    // 拖掉预算的 90% 后才发 register（旧「每步独立超时」实现下，
    // m2 还会再享一个完整 2s——总占用 ~3.8s；整体预算下 ~2.0s 即关）
    tokio::time::sleep(Duration::from_millis(1800)).await;
    client
        .send_text(&format!(
            r#"{{"type":"register","relayId":"{relay_id}","role":"desktop"}}"#
        ))
        .await;
    let _m1 = client.recv_binary().await; // 收到挑战后沉默

    assert!(
        client.closed_within(Duration::from_secs(1)).await,
        "鉴权总预算耗尽后必须立即关闭（拖延不得续借预算）"
    );
}

/// P4：超大帧在协议层被拒绝（转发态，防内存滥用）。
#[tokio::test]
async fn oversized_frame_rejected_at_protocol_layer() {
    // WHY: 默认 tungstenite 消息上限约 64MB——超大帧「先分配后处理」
    // 是廉价的内存放大攻击面；上限贴着 E2E 密文帧上限（64KB-1）收紧后，
    // 超限帧必须被协议层拒绝并断连。
    let state = AppState {
        max_message_size: 64 * 1024,
        ..Default::default()
    };
    let server = InProcessServer::start(build_router_with(state)).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    // 常规帧先行：通道正常
    let ok_payload = b"normal frame still flows";
    desktop.send_binary(ok_payload).await;
    assert_eq!(phone.recv_binary().await, ok_payload.to_vec());

    // 超过上限的帧 → 协议层容量错误 → 断连
    let huge = vec![0u8; 256 * 1024];
    desktop.send_binary(&huge).await;
    assert!(
        desktop.is_closed().await,
        "超过 max_message_size 的帧必须导致断连"
    );
}

/// P5：保活——对端完全沉默（不回 Pong、不收数据）→ 空闲超限判死回收。
#[tokio::test]
async fn silent_peer_reaped_by_keepalive() {
    // WHY: 无 FIN 的死对端（NAT 超时/断电）不产生任何入站信号——若无
    // 服务端主动保活，僵尸连接与槽位将永久滞留注册表（AC5「无随时间
    // 增长」被打破）。保活 Ping + 空闲上限让沉默连接在时限内被回收。
    let state = AppState {
        keepalive_ping: Duration::from_millis(100),
        keepalive_idle: Duration::from_millis(400),
        ..Default::default()
    };
    let server = InProcessServer::start(build_router_with(state)).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let _phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;
    // phone 沉默（不再读取 → 服务端的 Ping 得不到 Pong）
    // desktop 持续读取（观测者）：phone 被回收时 desktop 应收到 close

    assert!(
        desktop.is_closed().await,
        "沉默对端必须在空闲上限内被回收，并对 desktop 发送 close"
    );
}

/// P6：优雅退出广播——shutdown_all 后活跃连接收到 Close、槽位回收。
#[tokio::test]
async fn graceful_shutdown_broadcasts_close() {
    // WHY: Docker stop 只给 10s——若不广播关闭信号，活跃连接任务会
    // 挂到被 SIGKILL，客户端永远收不到干净 Close（只能靠自身超时发现
    // 连接已死）。广播后全部连接立即收到 Close 并释放槽位。
    let state = AppState::default();
    let server = InProcessServer::start(build_router_with(state.clone())).await;
    let url = server.relay_url();

    let (d_priv, d_pub) = common::keypair();
    let relay_id = derive_relay_id(&d_pub);
    let (p_priv, _) = common::keypair();

    let mut desktop =
        RelayClient::register_and_handshake(&url, &relay_id, "desktop", &d_priv).await;
    let mut phone =
        RelayClient::register_and_handshake(&url, &relay_id, "phone", &p_priv).await;

    state.registry.shutdown_all();

    assert!(
        desktop.is_closed().await,
        "优雅退出时 desktop 必须收到 Close"
    );
    assert!(phone.is_closed().await, "优雅退出时 phone 必须收到 Close");
    // 槽位清理完毕（连接任务确实退出，注册表回空）
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(state.registry.len().await, 0, "优雅退出后注册表必须回空");
}
