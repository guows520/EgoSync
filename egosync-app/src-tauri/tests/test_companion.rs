//! Story 12.2 手机伴侣集成测试。
//!
//! 覆盖：QR payload 字段、WS Noise XX 握手成功/失败路径、paired_devices CRUD
//! 与免配对重连、移除后拒绝、换绑 pending→confirm 替换、pending 超时、
//! 事件 payload 形状、data_export 配对设备字段与旧档兼容。
//! Story 13.1：快照口径/记忆排除、debounce 合并、10MB 截断、建连全量
//! SNAPSHOT、写信号 STATE_DELTA、断线重连补最新快照、分帧 roundtrip。
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
// Story 13.1 快照测试
use egosync_lib::db::conversations as conversations_db;
use egosync_lib::db::notifications as notifications_db;
use egosync_lib::db::roles as roles_db;
use egosync_lib::db::tasks as tasks_db;
use egosync_lib::models::notification::CreateNotificationInput;
use egosync_lib::models::role::CreateRoleInput;
use egosync_lib::models::snapshot::DesktopSnapshot;
use egosync_lib::models::task::CreateTaskInput;
use egosync_lib::services::companion_snapshot::{
    build_snapshot, frame_snapshot, reassemble, ChunkEnvelope, CompanionSnapshotEngine,
    SnapshotFrameKind, WriteSignal,
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
    let payload = companion_pairing::generate_qr_payload(&pub_key, None);
    assert!(payload.relay_addr.is_none(), "未配置中继时 relay_addr 必须为空");
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

// ── 中继路径（Story 12.4 AC4）──

/// 进程内拉起 relay（复用 `relay_server::build_router`，完整路由零裁剪）。
async fn spawn_relay() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind relay");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, relay_server::build_router())
            .await
            .expect("relay serve");
    });
    port
}

/// 经中继转发态取下一条 binary（跳过 relay keepalive 的 Ping/Pong——
/// tungstenite 自动应答但仍会透传给应用层，与业务帧不可混读）。
async fn next_binary(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
) -> Vec<u8> {
    loop {
        match ws.next().await {
            Some(Ok(Message::Binary(data))) => return data.to_vec(),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                panic!("中继连接在等待业务帧时关闭/出错")
            }
            Some(Ok(_)) => panic!("转发态收到非 binary 帧"),
        }
    }
}

/// 模拟手机完成中继鉴权：register(text) 已由调用方发送，
/// 中继为 initiator（m1→m2→m3），手机以真实静态私钥作 responder 应答。
async fn relay_phone_auth(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    phone_priv: &[u8],
) {
    let m1 = next_binary(ws).await;
    let mut responder = HandshakeSession::responder(phone_priv).expect("responder");
    responder.read_message(&m1).expect("read m1");
    let m2 = responder.write_message(&[]).expect("write m2");
    ws.send(Message::binary(m2)).await.expect("send m2");
    let m3 = next_binary(ws).await;
    responder.read_message(&m3).expect("read m3");
    // 鉴权 transport 弃用（relay 零知识：不参与后续加密）
    let _ = responder.into_transport();
}

#[tokio::test]
async fn relay_path_pairs_and_pings() {
    // WHY: 离开局域网（无 NSD）时中继是唯一承载——这条链路断一处，
    // "出门在外手机伴侣永久离线"且用户无从定位。全链路贯通：
    // desktop register→鉴权→转发态 → 手机 register→鉴权→E2E 握手
    // →首配落库→PING/PONG→状态 origin=relay（与直连同源决策）。
    let relay_port = spawn_relay().await;
    let relay_addr = format!("ws://127.0.0.1:{relay_port}");
    let (pool, state, _port, _desktop_priv, desktop_pub, _dir) = setup_listener().await;

    // 配置中继 + 打开配对窗口（门控与 NSD 同口径：窗口打开即允许中继注册）
    egosync_lib::db::app_settings::set_setting(
        &pool,
        companion_pairing::RELAY_ADDR_SETTING_KEY,
        &relay_addr,
    )
    .await
    .expect("set relay addr");
    state.open_pairing_window("nonce-relay".to_string()).await;

    // 桌面中继客户端：setup_listener → start_companion_listener 已 spawn
    // run_relay_client（唯一实例；多实例会因 relay 单槽替换语义互踢抖动）。
    // 配置写入后首个慢轮询周期（≤5s）内连接中继。
    // 手机侧重试预算（10×2.3s）覆盖该窗口。

    // QR 透出真实中继地址（AC4 前提：手机扫码才知道去哪注册）
    let payload =
        companion_pairing::generate_qr_payload(&desktop_pub, Some(relay_addr.clone()));
    assert_eq!(payload.relay_addr.as_deref(), Some(relay_addr.as_str()));

    // 模拟手机：连中继 → register → 中继鉴权 → E2E initiator 握手（经转发）。
    // 桌面槽位就绪前发送的 E2E m1 会被 relay 丢弃（无离线投递），故整体重试。
    let (phone_priv, _) = generate_static_keypair().unwrap();
    let mut t = None;
    // 预算 14×~2.3s：覆盖桌面中继客户端首个慢轮询周期（≤5s）+ 并行测试负载
    for attempt in 0..14 {
        let (mut ws, _) =
            tokio_tungstenite::connect_async(format!("{}/relay", relay_addr))
                .await
                .expect("phone relay connect");
        let register = format!(
            r#"{{"type":"register","relayId":"{}","role":"phone"}}"#,
            payload.relay_id
        );
        ws.send(Message::text(register)).await.expect("register");
        relay_phone_auth(&mut ws, &phone_priv).await;

        let mut initiator = HandshakeSession::initiator(&phone_priv).expect("initiator");
        let m1 = initiator.write_message(&[]).expect("m1");
        ws.send(Message::binary(m1)).await.expect("send e2e m1");
        let m2 = tokio::time::timeout(Duration::from_secs(2), next_binary(&mut ws)).await;
        match m2 {
            Ok(data) => {
                initiator.read_message(&data).expect("read m2");
                let m3 = initiator.write_message(&[]).expect("m3");
                ws.send(Message::binary(m3)).await.expect("send m3");
                t = Some((ws, initiator.into_transport().expect("transport")));
                break;
            }
            _ => {
                // 桌面槽位尚未就绪：断开重来（≤10 次 × ~2s）
                tracing::debug!(attempt, "中继对端未就绪，重试");
                sleep(Duration::from_millis(300)).await;
            }
        }
    }
    let (mut ws, mut t) = t.expect("E2E 握手应在重试预算内完成");

    // app 层：HELLO → deviceInfo → pairingAuth（与直连同款序列）
    send_frame(
        &mut ws,
        &mut t,
        &Frame::Hello(HelloPayload {
            protocol_version: PROTOCOL_VERSION,
        }),
    )
    .await;
    send_frame(
        &mut ws,
        &mut t,
        &Frame::Notice(NoticePayload {
            data: r#"{"type":"deviceInfo","deviceName":"中继测试机"}"#.to_string(),
        }),
    )
    .await;
    send_pairing_nonce(&mut ws, &mut t, "nonce-relay").await;

    // 首配直接落库（confirm 仅用于换绑，与直连同源决策）
    wait_for(
        || async { paired_devices_db::get_all(&pool).await.unwrap().len() == 1 },
        15000,
        "等待中继路径配对设备落库",
    )
    .await;
    let devices = paired_devices_db::get_all(&pool).await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_name, "中继测试机");

    // PING/PONG 往返（转发态全双工，与直连同协议）
    send_frame(&mut ws, &mut t, &Frame::Ping(companion_proto::frames::PingPayload {})).await;
    let pong = tokio::time::timeout(Duration::from_secs(10), next_binary(&mut ws))
        .await
        .expect("等待 PONG 超时");
    let frame = decode_frame(&pong, &mut t).expect("decode pong");
    assert!(matches!(frame, Frame::Ping(_)), "PONG 必须是 Ping 帧");

    // 状态如实反映中继来源（origin=relay，get_status 不得硬编码 direct）
    let status = get_status(&pool, &state).await.unwrap();
    let info = status.connected.expect("应处于 Connected");
    assert_eq!(info.origin, "relay", "中继会话 origin 必须如实标记");
}

// ═══════════════════════════════════════════════════════════════════
// Story 13.1：快照引擎
// ═══════════════════════════════════════════════════════════════════

/// 带快照引擎的监听装配：注入短 debounce 窗口 + 建连请求通道 + spawn run()。
async fn setup_listener_with_engine(
    debounce: Duration,
) -> (
    DbPool,
    ConversationsPool,
    Arc<CompanionState>,
    Arc<CompanionSnapshotEngine>,
    u16,
    tempfile::TempDir,
) {
    let (pool, conv_pool, dir) = test_pool().await;
    let (priv_key, pub_key) = generate_static_keypair().unwrap();
    let state = Arc::new(CompanionState::with_static_keypair_for_testing(
        None,
        priv_key,
        pub_key,
    ));
    let engine = Arc::new(CompanionSnapshotEngine::with_debounce(
        pool.clone(),
        conv_pool.clone(),
        state.clone(),
        debounce,
    ));
    state
        .set_snapshot_request_tx(engine.snapshot_request_tx())
        .await;
    let engine_for_run = engine.clone();
    tokio::spawn(async move {
        engine_for_run.run().await;
    });
    let port = start_companion_listener(pool.clone(), state.clone())
        .await
        .expect("start listener");
    (pool, conv_pool, state, engine, port, dir)
}

/// 种子数据：一个角色（含机密 personality_prompt）+ 任务 + 会话/消息 +
/// 本季度与跨季度简报/复盘 + 未读/已读通知 + 一条记忆。
/// 返回角色 id（供通知外键使用）。
async fn seed_snapshot_domain_data(pool: &DbPool, conv_pool: &ConversationsPool) -> String {
    let role = roles_db::create_role(
        pool,
        &CreateRoleInput {
            name: "快照测试角色".to_string(),
            icon: None,
            color: None,
            goal: Some("验证快照口径".to_string()),
        },
    )
    .await
    .expect("create role");
    // 桌面内部数据：不得出现在快照中（负向断言目标）
    sqlx::query("UPDATE roles SET personality_prompt = 'SECRET-PERSONALITY-PROMPT' WHERE id = ?1")
        .bind(&role.id)
        .execute(pool)
        .await
        .expect("set personality_prompt");

    tasks_db::create_task(
        pool,
        &CreateTaskInput {
            owner_type: None,
            role_id: Some(role.id.clone()),
            title: "快照口径验证任务".to_string(),
            deadline: None,
            quadrant: Some("Q2".to_string()),
            is_big_rock: None,
        },
    )
    .await
    .expect("create task");

    let conv = conversations_db::create_conversation(conv_pool, Some(&role.id))
        .await
        .expect("create conversation");
    conversations_db::insert_message(conv_pool, &conv.id, "user", "早上好，今天做什么？", true)
        .await
        .expect("insert user message");
    conversations_db::insert_message(conv_pool, &conv.id, "assistant", "先专注大石头任务。", true)
        .await
        .expect("insert assistant message");

    // 简报：本季度 + 远古（本季度过滤的负向目标）
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let month_start = chrono::Local::now().format("%Y-%m-01").to_string();
    sqlx::query(
        "INSERT INTO briefings (id, content, date) VALUES ('b-current', '本季度晨间简报内容', ?1)",
    )
    .bind(&today)
    .execute(pool)
    .await
    .expect("insert current briefing");
    sqlx::query(
        "INSERT INTO briefings (id, content, date) VALUES ('b-old', '远古旧简报-不应上机', '2020-01-10')",
    )
    .execute(pool)
    .await
    .expect("insert old briefing");

    // 周复盘：本周（月首周必在本季度内）+ 远古
    sqlx::query(
        "INSERT INTO weekly_reviews (id, week_start, week_end, summary) \
         VALUES ('wr-current', ?1, '2026-01-07', '本季度周复盘内容')",
    )
    .bind(&month_start)
    .execute(pool)
    .await
    .expect("insert current review");
    sqlx::query(
        "INSERT INTO weekly_reviews (id, week_start, week_end, summary) \
         VALUES ('wr-old', '2020-01-06', '2020-01-12', '远古旧复盘-不应上机')",
    )
    .execute(pool)
    .await
    .expect("insert old review");

    // 通知：未读 + 已读（快照只含未读）
    let unread = notifications_db::create_notification(
        pool,
        &CreateNotificationInput {
            role_id: role.id.clone(),
            level: "tap".to_string(),
            content: "未读通知-应上机".to_string(),
        },
    )
    .await
    .expect("create unread notification");
    let read = notifications_db::create_notification(
        pool,
        &CreateNotificationInput {
            role_id: role.id.clone(),
            level: "whisper".to_string(),
            content: "已读通知-不应上机".to_string(),
        },
    )
    .await
    .expect("create read notification");
    notifications_db::mark_read(pool, &read.id)
        .await
        .expect("mark read");
    assert_eq!(
        notifications_db::count_unread(pool).await.unwrap(),
        1,
        "种子校验：恰一条未读"
    );
    let _ = unread;

    // 记忆：内容不得上机（memoryCount 仅数字出现）
    sqlx::query(
        "INSERT INTO memories (id, role_id, category, content, source_conversation_id) \
         VALUES ('m-secret', ?1, 'fact', '绝密记忆内容XYZ-不上机', 'conv-fake')",
    )
    .bind(&role.id)
    .execute(pool)
    .await
    .expect("insert memory");

    role.id
}

/// 手机侧收帧并重组出目标类型的快照（跳过其他帧类型），超时即 panic。
async fn phone_receive_snapshot(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    t: &mut TransportSession,
    want_snapshot: bool,
    timeout_ms: u64,
) -> DesktopSnapshot {
    let mut frames: Vec<Frame> = Vec::new();
    let mut total: Option<u32> = None;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "超时未收齐全部分帧");
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("读帧超时")
            .expect("连接已关闭")
            .expect("WS 读取失败");
        let bytes = msg.into_data();
        let frame = decode_frame(&bytes, t).expect("帧解码失败");
        let is_wanted = match &frame {
            Frame::Snapshot(_) => want_snapshot,
            Frame::StateDelta(_) => !want_snapshot,
            _ => false,
        };
        if !is_wanted {
            continue;
        }
        let data = match &frame {
            Frame::Snapshot(p) => &p.data,
            Frame::StateDelta(p) => &p.data,
            _ => unreachable!(),
        };
        let envelope: ChunkEnvelope = serde_json::from_str(data).expect("envelope 解析");
        match total {
            None => total = Some(envelope.total),
            Some(tot) => assert_eq!(tot, envelope.total, "分帧 total 漂移"),
        }
        frames.push(frame);
        if frames.len() == total.unwrap() as usize {
            return reassemble(&frames).expect("重组快照失败");
        }
    }
}

/// 进程内完成 XX 握手，返回（发起方, 响应方）传输会话（单帧编码上限断言用）。
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

/// 手机完成配对握手（HELLO + nonce），等待进入 Connected 状态。
async fn phone_pair_and_connect(
    port: u16,
    phone_priv: &[u8],
    nonce: &str,
    pool: &DbPool,
    state: &Arc<CompanionState>,
) -> (WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, TransportSession) {
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("connect");
    let mut t = phone_handshake(&mut ws, phone_priv).await;
    send_frame(
        &mut ws,
        &mut t,
        &Frame::Hello(HelloPayload {
            protocol_version: PROTOCOL_VERSION,
        }),
    )
    .await;
    send_pairing_nonce(&mut ws, &mut t, nonce).await;
    wait_for(
        || async { get_status(pool, state).await.unwrap().connected.is_some() },
        15000,
        "等待进入 Connected 状态",
    )
    .await;
    (ws, t)
}

#[tokio::test]
async fn snapshot_scope_completeness_and_memory_exclusion() {
    // WHY: 桌面是唯一事实源，手机只该拿到口径内数据——七域齐全是「打开
    // 即见一致状态」的前提；记忆内容/人格提示若泄上机，等于把桌面内部
    // 数据边界打穿（NFR-M4/硬边界 #4），且用户无从察觉。
    let (pool, conv_pool, _dir) = test_pool().await;
    let _role_id = seed_snapshot_domain_data(&pool, &conv_pool).await;

    let snapshot = build_snapshot(&pool, &conv_pool).await.expect("build snapshot");

    // 七域齐全
    assert_eq!(snapshot.roles.len(), 1, "角色域");
    assert_eq!(snapshot.tasks.len(), 1, "任务域");
    assert_eq!(snapshot.dashboard.statuses.len(), 1, "仪表盘角色卡态");
    assert_eq!(snapshot.conversations.len(), 1, "会话域");
    assert_eq!(snapshot.conversations[0].messages.len(), 2, "会话消息");
    assert_eq!(snapshot.briefings.len(), 1, "简报域（仅本季度）");
    assert_eq!(snapshot.briefings[0].id, "b-current");
    assert_eq!(snapshot.weekly_reviews.len(), 1, "周复盘域（仅本季度）");
    assert_eq!(snapshot.weekly_reviews[0].id, "wr-current");
    assert_eq!(snapshot.notifications.len(), 1, "通知域（仅未读）");
    assert_eq!(snapshot.notifications[0].content, "未读通知-应上机");

    // 未截断元数据
    assert!(!snapshot.truncated);
    assert!(snapshot.data_cutoff_at.is_none());
    assert!(snapshot.truncated_domains.is_empty());
    assert_eq!(snapshot.schema_version, 1);

    // 负向断言：记忆内容 / 人格提示 / 跨季度数据 / 已读通知 不在快照中
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.contains("绝密记忆内容XYZ"), "记忆内容不得上机");
    assert!(!json.contains("SECRET-PERSONALITY-PROMPT"), "人格提示不得上机");
    assert!(!json.contains("远古旧简报"), "跨季度简报不得上机");
    assert!(!json.contains("远古旧复盘"), "跨季度复盘不得上机");
    assert!(!json.contains("已读通知-不应上机"), "已读通知不得上机");
    // memoryCount 仅作为仪表盘指标数字出现
    assert_eq!(snapshot.dashboard.metrics.memory_count, 1);
    assert!(json.contains("\"memoryCount\":1"));
}

#[tokio::test]
async fn debounce_coalesces_rapid_writes() {
    // WHY: 高频写（连续对话落库）若逐条触发 10MB 级重建，桌面主库会被
    // 聚合查询反复拖慢（sprint 风险 #1）——N 次快速写必须合并为 1 次重建。
    let (pool, conv_pool, _dir) = test_pool().await;
    let _role_id = seed_snapshot_domain_data(&pool, &conv_pool).await;
    let (priv_key, pub_key) = generate_static_keypair().unwrap();
    let state = Arc::new(CompanionState::with_static_keypair_for_testing(
        None,
        priv_key,
        pub_key,
    ));
    let engine = CompanionSnapshotEngine::with_debounce(
        pool.clone(),
        conv_pool.clone(),
        state,
        Duration::from_millis(80),
    );
    let engine = Arc::new(engine);
    let engine_for_run = engine.clone();
    tokio::spawn(async move {
        engine_for_run.run().await;
    });

    let notify = engine.notify_signal();
    for _ in 0..5 {
        notify
            .try_send(WriteSignal {
                event: "task:created",
            })
            .expect("try_send 写信号");
    }

    wait_for(
        || async { engine.rebuild_count() >= 1 },
        5000,
        "等待 debounce 窗口结束并完成首次重建",
    )
    .await;
    // 窗口结束后再等一个完整 debounce 周期，确认没有第二次重建
    sleep(Duration::from_millis(300)).await;
    assert_eq!(
        engine.rebuild_count(),
        1,
        "5 次快速写信号必须合并为恰 1 次重建"
    );
}

#[tokio::test]
async fn snapshot_respects_10mb_cap_and_truncates_oldest() {
    // WHY: 截断是诚实性机制而非静默丢数据——超限必须置 truncated 元数据并
    // 保留核心域（角色/任务/指标），让手机 UI 能明示「数据截至何时」；
    // 静默截断 = 用户看到残缺状态却以为看到了全部。
    let (pool, conv_pool, _dir) = test_pool().await;
    let role = roles_db::create_role(
        &pool,
        &CreateRoleInput {
            name: "大快照角色".to_string(),
            icon: None,
            color: None,
            goal: None,
        },
    )
    .await
    .expect("create role");
    tasks_db::create_task(
        &pool,
        &CreateTaskInput {
            owner_type: None,
            role_id: Some(role.id.clone()),
            title: "核心域任务-不可截断".to_string(),
            deadline: None,
            quadrant: Some("Q1".to_string()),
            is_big_rock: None,
        },
    )
    .await
    .expect("create task");

    // 3 个会话 × 70 条 × 60KB ≈ 12.6MB（> 10MB 上限）；丢最旧会话后
    // ≈ 8.4MB（< 上限）——恰触发一次整段丢弃。
    let big_content = "x".repeat(60_000);
    let mut conv_ids = Vec::new();
    for label in ["old", "mid", "new"] {
        let conv = conversations_db::create_conversation(&conv_pool, Some(&role.id))
            .await
            .expect("create conversation");
        for _ in 0..70 {
            conversations_db::insert_message(&conv_pool, &conv.id, "user", &big_content, true)
                .await
                .expect("insert big message");
        }
        // 显式时间戳：会话按 updated_at DESC 排序、消息时间确定性可断言
        let updated_at = format!("2026-08-{:02}T00:00:00Z", match label {
            "old" => 1,
            "mid" => 10,
            _ => 20,
        });
        let message_at = updated_at.clone();
        sqlx::query("UPDATE conversations SET updated_at = ?1 WHERE id = ?2")
            .bind(&updated_at)
            .bind(&conv.id)
            .execute(&*conv_pool)
            .await
            .expect("set conversation updated_at");
        sqlx::query("UPDATE messages SET created_at = ?1 WHERE conversation_id = ?2")
            .bind(&message_at)
            .bind(&conv.id)
            .execute(&*conv_pool)
            .await
            .expect("set message created_at");
        conv_ids.push(conv.id);
    }

    let snapshot = build_snapshot(&pool, &conv_pool).await.expect("build snapshot");

    assert!(snapshot.truncated, "超限快照必须标记 truncated");
    let cutoff = snapshot.data_cutoff_at.clone().expect("截断必须写 dataCutoffAt");
    assert_eq!(cutoff, "2026-08-10T00:00:00Z", "cutoff = 保留数据的最旧时间戳");
    assert_eq!(snapshot.truncated_domains, vec!["conversations".to_string()]);
    assert_eq!(snapshot.conversations.len(), 2, "恰丢弃最旧一个会话");
    assert!(
        !snapshot.conversations.iter().any(|c| c.id == conv_ids[0]),
        "最旧会话必须被整段丢弃"
    );
    // 核心域不缺失（AC4）
    assert_eq!(snapshot.roles.len(), 1);
    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.dashboard.statuses.len(), 1);
    // 截断后体积达标
    let len = serde_json::to_vec(&snapshot).unwrap().len();
    assert!(len <= 10 * 1024 * 1024, "截断后必须 ≤ 10MB，实际 {len}");
}

#[tokio::test]
async fn snapshot_stage_b_halves_messages_and_keeps_newest() {
    // WHY: 单会话超限时阶段 B（消息减半）必须保留最新一半——保旧丢新会让
    // 手机看到早已结束的话题却看不到最新进展；且阶段 A 的「保留最新 1 个
    // 会话」守卫不得把唯一会话整段清空（评审 P4/P7：截断方向必须有测试，
    // 原实现 drain 方向反了也无测试可报错）。
    let (pool, conv_pool, _dir) = test_pool().await;
    let role = roles_db::create_role(
        &pool,
        &CreateRoleInput {
            name: "单会话超限角色".to_string(),
            icon: None,
            color: None,
            goal: None,
        },
    )
    .await
    .expect("create role");
    let conv = conversations_db::create_conversation(&conv_pool, Some(&role.id))
        .await
        .expect("create conversation");

    // 200 条 × 60KB = 12MB（> 10MB）——单会话触发阶段 B；减半后 6MB 达标。
    // 200 恰为 MESSAGES_PER_CONVERSATION 上限，全部进入快照。
    let big_content = "x".repeat(60_000);
    for _ in 0..200 {
        conversations_db::insert_message(&conv_pool, &conv.id, "user", &big_content, true)
            .await
            .expect("insert big message");
    }
    // 逐条递增时间戳（rowid 顺序 = 插入顺序 = 时间顺序），方向断言可确定性
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT rowid FROM messages WHERE conversation_id = ?1 ORDER BY rowid",
    )
    .bind(&conv.id)
    .fetch_all(&*conv_pool)
    .await
    .expect("select rowids");
    let ts_of = |i: usize| format!("2026-08-01T{:02}:{:02}:00Z", i / 60, i % 60);
    for (i, (rowid,)) in rows.iter().enumerate() {
        sqlx::query("UPDATE messages SET created_at = ?1 WHERE rowid = ?2")
            .bind(ts_of(i))
            .bind(rowid)
            .execute(&*conv_pool)
            .await
            .expect("set message created_at");
    }

    let snapshot = build_snapshot(&pool, &conv_pool).await.expect("build snapshot");

    assert_eq!(snapshot.conversations.len(), 1, "唯一会话不得被整段丢弃");
    let messages = &snapshot.conversations[0].messages;
    assert_eq!(messages.len(), 100, "200 条减半后恰保留 100 条");
    assert_eq!(
        messages.first().map(|m| m.created_at.as_str()),
        Some(ts_of(100).as_str()),
        "最旧保留消息必须是原第 101 条（保留最新一半）"
    );
    assert_eq!(
        messages.last().map(|m| m.created_at.as_str()),
        Some(ts_of(199).as_str()),
        "最新消息必须保留"
    );
    assert!(snapshot.truncated, "减半截断必须标记 truncated");
    assert_eq!(snapshot.truncated_domains, vec!["conversations".to_string()]);
    assert_eq!(
        snapshot.data_cutoff_at.as_deref(),
        Some(ts_of(100).as_str()),
        "cutoff = 保留数据的最旧时间戳"
    );
    let len = serde_json::to_vec(&snapshot).unwrap().len();
    assert!(len <= 10 * 1024 * 1024, "减半后必须 ≤ 10MB，实际 {len}");
}

#[tokio::test]
async fn snapshot_oversize_core_domain_is_flagged_not_silent() {
    // WHY: 可截域耗尽仍超限（核心域自身 >10MB）时静默照发 = 谎报数据完整
    // ——手机会把残缺快照当全量渲染。必须 truncated=true 如实标记（评审
    // P4 诚实性：显式失败优于静默违约），快照本身照发（完整核心域优于失明）。
    let (pool, conv_pool, _dir) = test_pool().await;
    let _role = roles_db::create_role(
        &pool,
        &CreateRoleInput {
            name: "超限角色".to_string(),
            icon: None,
            color: None,
            goal: Some("g".repeat(11 * 1024 * 1024)),
        },
    )
    .await
    .expect("create role");

    let snapshot = build_snapshot(&pool, &conv_pool).await.expect("build snapshot");

    assert!(snapshot.truncated, "可截域耗尽仍超限必须标记 truncated");
    assert!(
        snapshot.truncated_domains.is_empty(),
        "无域被裁——超限来自不可截的核心域"
    );
    assert_eq!(snapshot.roles.len(), 1, "核心域照发不裁");
    let len = serde_json::to_vec(&snapshot).unwrap().len();
    assert!(
        len > 10 * 1024 * 1024,
        "核心域超限时快照如实超限（诚实标记优先于静默截断），实际 {len}"
    );
}

#[tokio::test]
async fn connected_phone_receives_full_snapshot_on_connect() {
    // WHY: 「打开手机即见桌面一致状态」——建连（含首配）后桌面必须主动下发
    // 全量 SNAPSHOT，手机无需轮询；这条链路断一处，手机端就是永远的白屏。
    let (pool, conv_pool, state, _engine, port, _dir) =
        setup_listener_with_engine(Duration::from_millis(80)).await;
    seed_snapshot_domain_data(&pool, &conv_pool).await;
    state.open_pairing_window("nonce-snap".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, mut t) =
        phone_pair_and_connect(port, &phone_priv, "nonce-snap", &pool, &state).await;

    // 真 WS 路径：手机收齐全部分帧并重组出合法快照
    let snapshot = phone_receive_snapshot(&mut ws, &mut t, true, 15000).await;
    assert_eq!(snapshot.roles.len(), 1, "快照须含角色域");
    assert_eq!(snapshot.tasks.len(), 1, "快照须含任务域");
    assert_eq!(snapshot.conversations.len(), 1, "快照须含会话域");
    assert_eq!(snapshot.schema_version, 1);
    assert!(!snapshot.truncated);
}

#[tokio::test]
async fn write_signal_pushes_state_delta() {
    // WHY: 「变化即时刷新、无需下拉」——会话在线期间写操作必须触发
    // STATE_DELTA 主动推送；若只有建连快照，手机就成了静态截图。
    let (pool, conv_pool, state, engine, port, _dir) =
        setup_listener_with_engine(Duration::from_millis(80)).await;
    seed_snapshot_domain_data(&pool, &conv_pool).await;
    state.open_pairing_window("nonce-delta".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, mut t) =
        phone_pair_and_connect(port, &phone_priv, "nonce-delta", &pool, &state).await;

    // 先消费建连全量快照（本轮目标之前的流量）
    let initial = phone_receive_snapshot(&mut ws, &mut t, true, 15000).await;
    assert!(
        !initial.tasks.iter().any(|task| task.title == "增量新任务"),
        "初始快照不应包含尚未创建的任务"
    );

    // 在线期间发生写操作 → 写信号 → debounce → STATE_DELTA
    tasks_db::create_task(
        &pool,
        &CreateTaskInput {
            owner_type: Some(egosync_lib::models::task::TaskOwnerType::Butler),
            role_id: None,
            title: "增量新任务".to_string(),
            deadline: None,
            quadrant: Some("Q2".to_string()),
            is_big_rock: None,
        },
    )
    .await
    .expect("create delta task");
    engine
        .notify_signal()
        .try_send(WriteSignal {
            event: "task:created",
        })
        .expect("try_send 写信号");

    let delta = phone_receive_snapshot(&mut ws, &mut t, false, 15000).await;
    assert!(
        delta.tasks.iter().any(|task| task.title == "增量新任务"),
        "STATE_DELTA 载荷必须反映最新变更（全量替换式）"
    );
}

#[tokio::test]
async fn reconnect_receives_latest_snapshot_after_gap() {
    // WHY: 「断线补最新快照、不做历史回放」——断线期间的变更要在重连后
    // 以一次全量 SNAPSHOT 补齐；若做增量回放或漏变更，手机状态将永久
    // 落后于桌面且无法自愈。
    let (pool, conv_pool, state, _engine, port, _dir) =
        setup_listener_with_engine(Duration::from_millis(80)).await;
    seed_snapshot_domain_data(&pool, &conv_pool).await;
    state.open_pairing_window("nonce-reconnect-snap".to_string()).await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    {
        let (mut ws, mut t) =
            phone_pair_and_connect(port, &phone_priv, "nonce-reconnect-snap", &pool, &state)
                .await;
        let first = phone_receive_snapshot(&mut ws, &mut t, true, 15000).await;
        assert!(
            !first.tasks.iter().any(|task| task.title == "断线期间新任务"),
            "首次快照不应包含断线期间才创建的任务"
        );
        drop(ws);
    }
    wait_for(
        || async { get_status(&pool, &state).await.unwrap().listening },
        15000,
        "等待断开后回 Listening",
    )
    .await;

    // 断线期间变更数据
    tasks_db::create_task(
        &pool,
        &CreateTaskInput {
            owner_type: Some(egosync_lib::models::task::TaskOwnerType::Butler),
            role_id: None,
            title: "断线期间新任务".to_string(),
            deadline: None,
            quadrant: Some("Q3".to_string()),
            is_big_rock: None,
        },
    )
    .await
    .expect("create gap task");

    // 重连（同公钥已配对，免 nonce）→ OnConnect 全量 SNAPSHOT 补齐
    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .expect("reconnect");
    let mut t2 = phone_handshake(&mut ws2, &phone_priv).await;
    send_frame(
        &mut ws2,
        &mut t2,
        &Frame::Hello(HelloPayload {
            protocol_version: PROTOCOL_VERSION,
        }),
    )
    .await;

    let latest = phone_receive_snapshot(&mut ws2, &mut t2, true, 15000).await;
    assert!(
        latest.tasks.iter().any(|task| task.title == "断线期间新任务"),
        "重连后的全量快照必须包含断线期间的变更"
    );
    // 单一全量（非增量序列）：载荷是完整七域快照
    assert_eq!(latest.roles.len(), 1);
    assert_eq!(latest.conversations.len(), 1);
    assert_eq!(latest.briefings.len(), 1);
    assert!(!latest.truncated);
}

#[tokio::test]
async fn chunking_roundtrip_and_size_bound() {
    // WHY: 分帧对协议层必须透明——10MB 级快照分帧后经真实 Noise 编解码
    // 往返无损、单帧不触 65519 上限；任何一帧超限都会被 crate 既有的
    // oversized_frame_encode_is_rejected 拒绝，手机端收到的就是断流。
    let (pool, conv_pool, _dir) = test_pool().await;
    let role = roles_db::create_role(
        &pool,
        &CreateRoleInput {
            name: "分帧测试角色".to_string(),
            icon: None,
            color: None,
            goal: None,
        },
    )
    .await
    .expect("create role");
    let conv = conversations_db::create_conversation(&conv_pool, Some(&role.id))
        .await
        .expect("create conversation");
    let content = "y".repeat(600);
    for _ in 0..200 {
        conversations_db::insert_message(&conv_pool, &conv.id, "user", &content, true)
            .await
            .expect("insert message");
    }

    let snapshot = build_snapshot(&pool, &conv_pool).await.expect("build snapshot");
    let frames = frame_snapshot(&snapshot, SnapshotFrameKind::Snapshot).expect("frame");
    assert!(frames.len() >= 2, "200×600B 消息必须分成多帧");

    // 单帧经真实 Noise 会话编码不触限（roundtrip 无损）
    let (mut initiator, mut responder) = session_pair();
    for frame in &frames {
        let wire = encode_frame(frame, &mut initiator).expect("单帧编码不得触限");
        assert!(
            wire.len() <= 65535 + 4,
            "线材长度超限: {}",
            wire.len()
        );
        let back = decode_frame(&wire, &mut responder).expect("decode roundtrip");
        assert_eq!(back, *frame, "单帧往返无损");
    }

    // 重组 roundtrip 无损（经序列化值对比，避免为测试加 PartialEq 派生）
    let reassembled = reassemble(&frames).expect("重组");
    assert_eq!(
        serde_json::to_value(&reassembled).unwrap(),
        serde_json::to_value(&snapshot).unwrap(),
        "分帧重组必须无损还原快照"
    );
}
