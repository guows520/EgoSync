//! Story 12.2 手机伴侣集成测试。
//!
//! 覆盖：QR payload 字段、WS Noise XX 握手成功/失败路径、paired_devices CRUD
//! 与免配对重连、移除后拒绝、换绑 pending→confirm 替换、pending 超时、
//! 事件 payload 形状、data_export 配对设备字段与旧档兼容。
//!
//! 所有 socket 测试在本机 loopback 起真实 WS listener；事件发射以纯函数
//! 断言（不依赖 AppHandle）。

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use companion_proto::crypto::{generate_static_keypair, HandshakeSession, TransportSession};
use companion_proto::frames::{
    decode_frame, encode_frame, Frame, HelloPayload, NoticePayload,
};
use companion_proto::PROTOCOL_VERSION;

use egosync_lib::db::paired_devices as paired_devices_db;
use egosync_lib::db::pool::{init_conversations_db, init_db, ConversationsPool, DbPool};
use egosync_lib::services::companion_connection::{
    get_status, remove_paired_device, start_companion_listener, CompanionState,
};
use egosync_lib::services::companion_pairing;
use egosync_lib::services::data_export::{
    gather_export_data, import_json_data, ExportFormat, export_all,
};

async fn test_pool() -> (DbPool, ConversationsPool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pool = init_db(&dir.path().join("egosync.db"))
        .await
        .expect("init db with migrations");
    let conv = init_conversations_db(&dir.path().join("conversations.db"))
        .await
        .expect("init conversations db");
    (pool, conv, dir)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 模拟手机的 WS 客户端，完成 XX 握手并返回 transport session。
async fn phone_handshake(
    ws: &mut WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    phone_priv: &[u8],
) -> TransportSession {
    let mut initiator = HandshakeSession::initiator(phone_priv).expect("initiator");
    let m1 = initiator.write_message(&[]).unwrap();
    ws.send(Message::binary(m1)).await.expect("send m1");
    let m2 = ws
        .next()
        .await
        .expect("recv m2")
        .expect("m2 ok")
        .into_data();
    initiator.read_message(&m2).expect("read m2");
    let m3 = initiator.write_message(&[]).unwrap();
    ws.send(Message::binary(m3)).await.expect("send m3");
    initiator.into_transport().expect("into transport")
}

async fn send_frame(ws: &mut WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, t: &mut TransportSession, frame: &Frame) {
    let bytes = encode_frame(frame, t).expect("encode");
    ws.send(Message::binary(bytes)).await.expect("send frame");
}

/// 模拟手机提交 QR 配对 nonce（app 层 pairingAuth 契约，D1 裁决）。
async fn send_pairing_nonce(
    ws: &mut WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    t: &mut TransportSession,
    nonce: &str,
) {
    send_frame(
        ws,
        t,
        &Frame::Notice(NoticePayload {
            data: format!(r#"{{"type":"pairingAuth","nonce":"{nonce}"}}"#),
        }),
    )
    .await;
}

async fn setup_listener() -> (DbPool, Arc<CompanionState>, u16, Vec<u8>, Vec<u8>, tempfile::TempDir) {
    let (pool, _conv, dir) = test_pool().await;
    let (priv_key, pub_key) = generate_static_keypair().unwrap();
    let state = Arc::new(CompanionState::with_static_keypair_for_testing(
        None,
        priv_key.clone(),
        pub_key.clone(),
    ));
    let port = start_companion_listener(pool.clone(), state.clone())
        .await
        .expect("start listener");
    (pool, state, port, priv_key, pub_key, dir)
}

async fn wait_for<F, Fut>(check: F, timeout_ms: u64, what: &str)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let mut waited = 0u64;
    while waited < timeout_ms {
        if check().await {
            return;
        }
        sleep(Duration::from_millis(200)).await;
        waited += 200;
    }
    panic!("超时未达成：{what}");
}

// ── QR payload ──

#[tokio::test]
async fn qr_payload_four_fields_contract() {
    // WHY: QR 是手机唯一配对入口——四字段任一缺失/错位，手机无法握手。
    let (_pool, state, _port, _priv, pub_key, _dir) = setup_listener().await;
    let payload = companion_pairing::generate_qr_payload(&pub_key);
    assert!(payload.relay_addr.is_none(), "V1 relay_addr 必须为空");
    assert!(!payload.desktop_static_pubkey.is_empty());
    assert!(!payload.relay_id.is_empty());
    assert!(!payload.pairing_nonce.is_empty());
    let expected = hex_encode(&Sha256::digest(&pub_key))[..16].to_string();
    assert_eq!(payload.relay_id, expected, "relay_id 必须是 pubkey 哈希前 16");
    let _ = state;
}

// ── 握手成功路径 ──

#[tokio::test]
async fn handshake_success_pairs_and_lands_paired_device() {
    // WHY: 握手成功是整条信任链的入口——落库失败意味着"扫了码却没配对"，
    // 用户从 UI 无法定位（二维码已失效、连接已断）。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-hs".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect");
    let mut t = phone_handshake(&mut ws, &phone_priv).await;

    send_frame(
        &mut ws,
        &mut t,
        &Frame::Hello(HelloPayload {
            protocol_version: PROTOCOL_VERSION,
        }),
    )
    .await;
    send_pairing_nonce(&mut ws, &mut t, "nonce-hs").await;
    send_frame(
        &mut ws,
        &mut t,
        &Frame::Notice(NoticePayload {
            data: r#"{"type":"deviceInfo","deviceName":"Pixel 测试机"}"#.to_string(),
        }),
    )
    .await;

    wait_for(
        || async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 },
        15000,
        "等待配对设备落库",
    )
    .await;

    let devices = paired_devices_db::get_all(&pool).await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_name, "Pixel 测试机");

    // 状态查询：连接中
    wait_for(
        || async { get_status(&pool, &state).await.unwrap().connected.is_some() },
        15000,
        "等待进入 Connected 状态",
    )
    .await;
    let status = get_status(&pool, &state).await.unwrap();
    assert!(status.listening == false);
    assert!(status.connected.unwrap().device_name == "Pixel 测试机");

    // 事件 payload 形状（纯函数断言，不依赖 AppHandle）
    let payload = companion_pairing::paired_event_payload(&devices[0]);
    assert_eq!(payload["deviceId"], devices[0].id);
    assert_eq!(payload["deviceName"], "Pixel 测试机");
    assert!(payload["pubkeyPrefix"].as_str().unwrap().len() == 8);

    drop(ws);
    wait_for(
        || async { get_status(&pool, &state).await.unwrap().listening },
        15000,
        "等待断开后回 Listening",
    )
    .await;
}

// ── 握手失败路径 ──

#[tokio::test]
async fn handshake_failure_leaves_no_db_record() {
    // WHY: 握手失败若静默落库，意味着任意未通过认证的连接都获得配对身份，
    // 加密边界形同虚设。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-fail".to_string()).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect");
    // 发送非法短消息作为 m1（XX 首条 -> e 无 MAC，但短于 32 字节临时公钥必失败）
    ws.send(Message::binary(vec![0u8; 10])).await.expect("send garbage");

    // 等待对端关闭（要么主动断开，要么握手报错后断开）
    let _ = tokio::time::timeout(Duration::from_secs(5), ws.next()).await;

    sleep(Duration::from_millis(300)).await;
    assert_eq!(
        paired_devices_db::get_all(&pool).await.unwrap().len(),
        0,
        "握手失败不得落库"
    );
}

// ── 免配对重连：同公钥二次握手直接进入会话 ──

#[tokio::test]
async fn same_pubkey_reconnect_enters_session_without_duplicate_pairing() {
    // WHY: "配对一次，之后免配对"——重连若再次写库/发 paired 事件，
    // 前端会弹出第二次配对提示，信任持久化在用户眼里就是坏的。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-reconnect".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();

    // 首次配对
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect first");
        let mut t = phone_handshake(&mut ws, &phone_priv).await;
        send_frame(
            &mut ws,
            &mut t,
            &Frame::Hello(HelloPayload {
                protocol_version: PROTOCOL_VERSION,
            }),
        )
        .await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-reconnect").await;
        wait_for(
            || async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 },
            15000,
            "首次配对落库",
        )
        .await;
        drop(ws);
        wait_for(|| async { get_status(&pool, &state).await.unwrap().listening }, 15000, "断开回 Listening").await;
    }

    let first_id = paired_devices_db::get_all(&pool).await.unwrap()[0].id.clone();

    // 二次连接（同公钥）——窗口已关闭但同公钥允许，不落新库
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect second");
        let mut t = phone_handshake(&mut ws, &phone_priv).await;
        send_frame(
            &mut ws,
            &mut t,
            &Frame::Hello(HelloPayload {
                protocol_version: PROTOCOL_VERSION,
            }),
        )
        .await;
        wait_for(
            || async { get_status(&pool, &state).await.unwrap().connected.is_some() },
            15000,
            "重连进入 Connected",
        )
        .await;
        drop(ws);
    }

    let all = paired_devices_db::get_all(&pool).await.unwrap();
    assert_eq!(all.len(), 1, "重连不得产生新记录");
    assert_eq!(all[0].id, first_id, "重连不得改写配对记录 id");
}

// ── 移除后拒绝 ──

#[tokio::test]
async fn removed_device_pubkey_is_rejected() {
    // WHY: "移除即拒绝"是用户主权底线——移除后旧公钥若仍能连上，
    // 等同移除功能形同虚设（手机看似解绑实则可连）。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-remove".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    // 先完成首次配对
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect");
        let mut t = phone_handshake(&mut ws, &phone_priv).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-remove").await;
        wait_for(|| async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 }, 15000, "首次配对").await;
        drop(ws);
        wait_for(|| async { get_status(&pool, &state).await.unwrap().listening }, 15000, "断开").await;
    }

    // 经真实移除路径（命令层同款 service 函数）：删库 + 清窗口/pending +
    // 终止会话 + 回收 NSD——不再手工清窗口（P1 修复前该测试靠手工清窗口才通过）
    let device = paired_devices_db::get_all(&pool).await.unwrap().pop().unwrap();
    remove_paired_device(&pool, &state, &device.id).await.expect("remove");

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect after remove");
    let mut t = phone_handshake(&mut ws, &phone_priv).await;
    send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
    // 对端拒绝后关闭连接
    let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
    sleep(Duration::from_millis(300)).await;
    assert_eq!(
        paired_devices_db::get_all(&pool).await.unwrap().len(),
        0,
        "移除后同公钥不得重新落库"
    );
}

// ── 换绑 pending → confirm 替换（单对单） ──

#[tokio::test]
async fn rebind_pending_requires_confirm_and_replaces_old_device() {
    // WHY: "换绑需可见同意"——第三台设备静默顶替绑定等于任何拿到 QR 的
    // 人都能偷走配对位；pending+confirm 是最小可见同意闸门。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-rebind".to_string()).await;

    // 设备 A 首配
    let (phone_a_priv, _) = generate_static_keypair().unwrap();
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect A");
        let mut t = phone_handshake(&mut ws, &phone_a_priv).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-rebind").await;
        wait_for(|| async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 }, 15000, "A 首配落库").await;
        drop(ws);
        wait_for(|| async { get_status(&pool, &state).await.unwrap().listening }, 15000, "A 断开").await;
    }

    // 设备 B（不同公钥）扫码 → pending，不落库、连接被关闭。
    // A 的首配已消费旧窗口（nonce 单次有效），B 扫的是重新生成的新 QR → 新窗口新 nonce
    state.open_pairing_window("nonce-rebind-b".to_string()).await;
    let (phone_b_priv, phone_b_pub) = generate_static_keypair().unwrap();
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect B");
        let mut t = phone_handshake(&mut ws, &phone_b_priv).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-rebind-b").await;
        wait_for(
            || async { get_status(&pool, &state).await.unwrap().pending_pairing.is_some() },
            15000,
            "B 进入 pending",
        )
        .await;
        // pending 未确认前：库中仍是 A 的记录
        let all = paired_devices_db::get_all(&pool).await.unwrap();
        assert_eq!(all.len(), 1, "pending 期间不得写入 B");
        // 对端连接被关闭（换绑待确认不进会话）
        let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
    }

    // confirm → B 替换 A
    let confirmed = companion_pairing::confirm_pending(&pool, &state.pending)
        .await
        .expect("confirm");
    assert_eq!(
        confirmed.device_pubkey,
        hex_encode(&phone_b_pub),
        "确认后必须是 B 的公钥"
    );
    let all = paired_devices_db::get_all(&pool).await.unwrap();
    assert_eq!(all.len(), 1, "单对单：确认后必须替换");
    assert_eq!(all[0].device_pubkey, hex_encode(&phone_b_pub));

    // confirm 之后再次 confirm → ValidationError
    let err = companion_pairing::confirm_pending(&pool, &state.pending)
        .await
        .expect_err("槽已清空，二次 confirm 必须失败");
    assert!(matches!(err, egosync_lib::error::AppError::ValidationError(_)));
}


// ── pending 超时作废 ──

#[tokio::test]
async fn expired_pending_is_voided_on_confirm() {
    // WHY: pending 120s 作废——无限期的待确认槽会让"曾经的扫码"
    // 在任意未来时刻被确认，等同重放窗口。
    let (pool, _conv, _dir) = test_pool().await;
    let (priv_key, pub_key) = generate_static_keypair().unwrap();
    let state = Arc::new(CompanionState::with_static_keypair_for_testing(
        None, priv_key, pub_key,
    ));
    let stale = egosync_lib::models::companion::PendingPairing {
        device_name: "过期手机".to_string(),
        device_pubkey: "pub-stale".to_string(),
        created_at: (chrono::Utc::now() - chrono::Duration::seconds(999)).to_rfc3339(),
    };
    *state.pending.lock().await = Some(stale);
    let err = companion_pairing::confirm_pending(&pool, &state.pending)
        .await
        .expect_err("过期 pending 必须拒绝");
    assert!(matches!(err, egosync_lib::error::AppError::ValidationError(_)));
    assert!(state.pending.lock().await.is_none(), "过期 pending 必须清槽");
}

// ── data_export 集成 ──

#[tokio::test]
async fn export_json_contains_paired_devices_field() {
    // WHY: 配对设备进导出清单——换机/重装时"重扫恢复"依赖旧绑定可迁移；
    // 导出缺字段意味着用户换机会静默丢失全部配对。
    let (pool, conv_pool, dir) = test_pool().await;
    paired_devices_db::upsert_single_device(
        &pool,
        &egosync_lib::models::companion::PairedDevice {
            id: "d-export".to_string(),
            device_name: "导出测试机".to_string(),
            device_pubkey: "pub-export".to_string(),
            paired_at: "2026-08-28T10:00:00Z".to_string(),
            last_seen_at: "2026-08-28T10:00:00Z".to_string(),
        },
    )
    .await
    .expect("upsert");

    let export_data = gather_export_data(&pool, &conv_pool).await.expect("gather");
    assert_eq!(export_data.paired_devices.len(), 1);
    assert_eq!(export_data.paired_devices[0]["devicePubkey"], "pub-export");
    assert_eq!(export_data.paired_devices[0]["deviceName"], "导出测试机");

    // markdown 用户报告不含配对设备（含公钥技术数据，无消费价值）
    let md = egosync_lib::services::data_export::generate_markdown(&export_data);
    assert!(!md.contains("pub-export"), "markdown 报告不得包含配对公钥");

    let _ = dir;
}

#[tokio::test]
async fn export_import_roundtrip_does_not_restore_paired_devices() {
    // WHY（评审裁决 D3）：配对绑定与桌面静态密钥（keyring，不随档迁移）绑定，
    // 跨机恢复必然不可用，且库非空会把新机首配从「扫码即绑定」降级为换绑确认
    // （违反 FR-40）——导入一律清空配对表，恢复后需重新扫码配对。
    let (pool, conv_pool, dir) = test_pool().await;
    paired_devices_db::upsert_single_device(
        &pool,
        &egosync_lib::models::companion::PairedDevice {
            id: "d-rt".to_string(),
            device_name: "往返测试机".to_string(),
            device_pubkey: "pub-rt".to_string(),
            paired_at: "2026-08-28T10:00:00Z".to_string(),
            last_seen_at: "2026-08-28T10:00:00Z".to_string(),
        },
    )
    .await
    .expect("upsert");

    let export_dir = dir.path().join("export");
    std::fs::create_dir_all(&export_dir).unwrap();
    let json_path = export_all(&dir.path(), &pool, &conv_pool, &export_dir, vec![ExportFormat::Json])
        .await
        .expect("export")
        .json_path
        .expect("json path");

    // 导入到全新库：配对设备不得恢复
    let (pool2, conv_pool2, dir2) = test_pool().await;
    import_json_data(&pool2, &conv_pool2, std::path::Path::new(&json_path))
        .await
        .expect("import");
    let restored = paired_devices_db::get_all(&pool2).await.unwrap();
    assert!(
        restored.is_empty(),
        "导入不得恢复配对绑定（配对不可跨机迁移），实际恢复了 {} 条",
        restored.len()
    );
    let _ = dir2;
}

#[tokio::test]
async fn old_export_without_paired_devices_field_imports_fine() {
    // WHY: 旧档兼容——升级用户的存量导出不含 pairedDevices 字段，
    // 导入若失败则数据主权承诺（随时可迁入）被打破。
    let (pool, conv_pool, dir) = test_pool().await;
    let old_json = serde_json::json!({
        "roles": [],
        "tasks": [],
        "memories": [],
        "suggestions": [],
        "notifications": [],
        "mission": null,
        "conflicts": [],
        "briefings": [],
        "weeklyReviews": [],
        "llmConfigs": [],
        "appSettings": [],
        "mcpServers": [],
        "skills": [],
        "skillBindings": [],
        "q2Reminders": [],
        "bigRockProtectionReminders": [],
        "forgottenMemorySources": [],
        "roleMcpServerBindings": [],
        "conversations": [],
        "messages": [],
        "exportedAt": "2026-01-01T00:00:00Z",
        "exportVersion": "1.0"
    });
    let json_path = dir.path().join("old_export.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(&old_json).unwrap()).unwrap();
    let result = import_json_data(&pool, &conv_pool, &json_path).await;
    assert!(result.is_ok(), "旧版 JSON 导入应成功（serde default 兜底）");
}

// ── nonce 校验闭环（D1 裁决）──

#[tokio::test]
async fn missing_or_wrong_nonce_is_rejected() {
    // WHY: pairing_nonce 是「二维码单次有效 / 不能被旁人重扫」的唯一凭据——
    // 窗口期内无有效 nonce 的连接若放行，任意端口扫描者都能完成首配。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-secret".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();

    // 无 nonce → 拒绝、不落库
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect no-nonce");
        let mut t = phone_handshake(&mut ws, &phone_priv).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_frame(&mut ws, &mut t, &Frame::Notice(NoticePayload {
            data: r#"{"type":"deviceInfo","deviceName":"无 nonce"}"#.to_string(),
        })).await;
        let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
        sleep(Duration::from_millis(300)).await;
        assert_eq!(paired_devices_db::get_all(&pool).await.unwrap().len(), 0, "无 nonce 不得落库");
    }

    // 错误 nonce → 拒绝、不落库
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect wrong-nonce");
        let mut t = phone_handshake(&mut ws, &phone_priv).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "wrong").await;
        let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
        sleep(Duration::from_millis(300)).await;
        assert_eq!(paired_devices_db::get_all(&pool).await.unwrap().len(), 0, "错误 nonce 不得落库");
    }

    // 窗口仍开（前两次失败不消费）：正确 nonce 仍可配对（前后一致性）
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect correct-nonce");
        let mut t = phone_handshake(&mut ws, &phone_priv).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-secret").await;
        wait_for(|| async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 }, 15000, "正确 nonce 配对").await;
    }
}

#[tokio::test]
async fn nonce_is_consumed_after_pairing() {
    // WHY: QR 单次有效——首配消费 nonce 后，同 nonce 的第二台设备不得再配对，
    // 否则重放窗口被打开（旁人复刻旧 nonce 即可配对）。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-once".to_string()).await;

    // 手机 A 以 nonce-once 首配成功
    let (priv_a, _) = generate_static_keypair().unwrap();
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect A");
        let mut t = phone_handshake(&mut ws, &priv_a).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-once").await;
        wait_for(|| async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 }, 15000, "A 首配").await;
        drop(ws);
        wait_for(|| async { get_status(&pool, &state).await.unwrap().listening }, 15000, "A 断开").await;
    }

    // 手机 B 持同一（已消费）nonce 连接 → 拒绝
    let (priv_b, _) = generate_static_keypair().unwrap();
    {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
            .await
            .expect("connect B");
        let mut t = phone_handshake(&mut ws, &priv_b).await;
        send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
        send_pairing_nonce(&mut ws, &mut t, "nonce-once").await;
        let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
        sleep(Duration::from_millis(300)).await;
        assert_eq!(
            paired_devices_db::get_all(&pool).await.unwrap().len(),
            1,
            "已消费 nonce 不得再次配对（库中仍应只有 A）"
        );
    }
}

// ── 移除即终止活跃会话（P3）──

#[tokio::test]
async fn remove_terminates_active_session() {
    // WHY: 「移除即拒绝」须覆盖存量会话——只挡新握手不掐旧连接，
    // 被移除的手机仍能收发帧，移除功能形同虚设。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-term".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let mut ws = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect")
        .0;
    let mut t = phone_handshake(&mut ws, &phone_priv).await;
    send_frame(&mut ws, &mut t, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
    send_pairing_nonce(&mut ws, &mut t, "nonce-term").await;
    wait_for(|| async { get_status(&pool, &state).await.unwrap().connected.is_some() }, 15000, "进入 Connected").await;

    let device = paired_devices_db::get_all(&pool).await.unwrap().pop().unwrap();
    remove_paired_device(&pool, &state, &device.id).await.expect("remove");

    // 会话被主动终止而非等待 TCP 断开 → 状态回 Listening
    wait_for(|| async { get_status(&pool, &state).await.unwrap().listening }, 15000, "移除后回 Listening").await;
    assert_eq!(paired_devices_db::get_all(&pool).await.unwrap().len(), 0);
}

// ── 重连取代僵尸会话（P6 状态竞态）──

#[tokio::test]
async fn reconnect_replaces_stale_session_state() {
    // WHY: 手机网络切换遗留僵尸连接——若旧会话退出时无条件回写 Listening，
    // 在线的新连接会被错标离线，前端状态与真实连接相反。
    let (pool, state, port, _desktop_priv, _desktop_pub, _dir) = setup_listener().await;
    state.open_pairing_window("nonce-stale".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    // 旧连接（保持打开）
    let mut ws1 = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect 1")
        .0;
    let mut t1 = phone_handshake(&mut ws1, &phone_priv).await;
    send_frame(&mut ws1, &mut t1, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
    send_pairing_nonce(&mut ws1, &mut t1, "nonce-stale").await;
    wait_for(|| async { get_status(&pool, &state).await.unwrap().connected.is_some() }, 15000, "旧连接 Connected").await;

    // 同公钥二次连接（模拟重连，旧连接未断）
    let mut ws2 = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect 2")
        .0;
    let mut t2 = phone_handshake(&mut ws2, &phone_priv).await;
    send_frame(&mut ws2, &mut t2, &Frame::Hello(HelloPayload { protocol_version: PROTOCOL_VERSION })).await;
    // 桌面注册新会话并终止旧会话 → 旧连接被服务端关闭
    let _ = tokio::time::timeout(Duration::from_secs(5), ws1.next()).await;
    sleep(Duration::from_millis(300)).await;

    // 关键断言：旧连接已退出，但状态仍是 Connected（被新连接持有，未被旧退出打回 Listening）
    let status = get_status(&pool, &state).await.unwrap();
    assert!(status.connected.is_some(), "旧会话退出不得把在线新连接错标为 Listening");

    drop(ws2);
    wait_for(|| async { get_status(&pool, &state).await.unwrap().listening }, 15000, "新连接断开后回 Listening").await;
}
