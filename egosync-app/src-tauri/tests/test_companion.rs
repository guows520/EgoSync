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
    decode_frame, encode_frame, CommandPayload, Frame, HelloPayload, NoticePayload,
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
// Story 13.3 指令通道测试
use egosync_lib::error::AppError;
use egosync_lib::services::companion_dispatch::{
    mirror_stream_payload, CommandExecutor, CompanionDispatcher,
};
use egosync_lib::models::companion_command::COMMAND_DATA_MAX_BYTES;

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

/// 模拟手机经中继完成 E2E 握手（含重试预算：桌面槽位就绪前发送的 E2E m1
/// 会被 relay 丢弃——无离线投递；预算 14×~2.3s 覆盖桌面中继客户端首个
/// 慢轮询周期 ≤5s + 并行测试负载）。
async fn relay_e2e_connect(
    relay_addr: &str,
    relay_id: &str,
    phone_priv: &[u8],
) -> Option<(
    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    TransportSession,
)> {
    for attempt in 0..14 {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("{}/relay", relay_addr))
            .await
            .expect("phone relay connect");
        let register = format!(
            r#"{{"type":"register","relayId":"{}","role":"phone"}}"#,
            relay_id
        );
        ws.send(Message::text(register)).await.expect("register");
        relay_phone_auth(&mut ws, phone_priv).await;

        let mut initiator = HandshakeSession::initiator(phone_priv).expect("initiator");
        let m1 = initiator.write_message(&[]).expect("m1");
        ws.send(Message::binary(m1.clone())).await.expect("send e2e m1");
        // m2 等待期间周期重发 m1：relay 无离线投递（forward.rs「对端不在即
        // 丢弃」），桌面中继客户端转发态存在超时+退避的 churn 空窗；手机
        // 超时断开还会触发 relay 对端 close 通知、杀死刚重建的桌面槽位——
        // 单发 m1 会与桌面重连形成活锁。重发间隔 500ms 远大于 m2 RTT，
        // 重复投递由桌面 responder 以帧位错判拒收兜底（整轮重试）。
        let m2 = tokio::time::timeout(Duration::from_secs(2), async {
            let mut m2: Option<Vec<u8>> = None;
            while m2.is_none() {
                match tokio::time::timeout(Duration::from_millis(500), ws.next()).await {
                    Ok(Some(Ok(Message::Binary(data)))) => m2 = Some(data.to_vec()),
                    Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                    Ok(_) => break, // 连接关闭/错误：本轮作废，外层整体重试
                    Err(_) => {
                        ws.send(Message::binary(m1.clone())).await.expect("resend m1");
                    }
                }
            }
            m2
        })
        .await;
        let Some(data) = m2.unwrap_or(None) else {
            tracing::debug!(attempt, "中继对端未就绪，重试");
            sleep(Duration::from_millis(300)).await;
            continue;
        };
        initiator.read_message(&data).expect("read m2");
        let m3 = initiator.write_message(&[]).expect("m3");
        ws.send(Message::binary(m3)).await.expect("send m3");
        return Some((ws, initiator.into_transport().expect("transport")));
    }
    None
}

#[tokio::test]
async fn relay_path_pairs_and_pings() {
    // WHY: 中继首配的确认门（Story 12.5 AC2）——origin=relay 的首配若沿用
    // 「扫码即绑定」，QR 泄露（拍照/转发）后攻击者可在窗口内从任意网络完成
    // 绑定，连接即获快照数据（FR-41 全量推送）。全链路贯通：desktop
    // register→鉴权→转发态 → 手机 register→鉴权→E2E 握手→首配入 pending
    //（不落库）→ pairing_confirm → 重连 AlreadyPaired → PING/PONG →
    // 状态 origin=relay（与直连同源决策）。
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

    // QR 透出真实中继地址（AC1 前提：手机扫码才知道去哪注册）
    let payload =
        companion_pairing::generate_qr_payload(&desktop_pub, Some(relay_addr.clone()));
    assert_eq!(payload.relay_addr.as_deref(), Some(relay_addr.as_str()));

    // 第一段：中继 E2E 握手 + HELLO → deviceInfo → pairingAuth（nonce 单次提交）
    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, mut t) = relay_e2e_connect(&relay_addr, &payload.relay_id, &phone_priv)
        .await
        .expect("E2E 握手应在重试预算内完成");
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

    // 确认门：中继首配入 pending、未确认前不落库（AC2）
    wait_for(
        || async {
            get_status(&pool, &state)
                .await
                .unwrap()
                .pending_pairing
                .is_some()
        },
        15000,
        "中继首配进入 pending",
    )
    .await;
    assert!(
        paired_devices_db::get_all(&pool).await.unwrap().is_empty(),
        "中继首配未确认前不得自动落库"
    );
    // pending 期间连接被桌面关闭（不进会话）
    let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
    drop(ws);

    // 桌面确认 → 落库（confirm_pending 复用 upsert_single_device，无首配特例）
    let confirmed = companion_pairing::confirm_pending(&pool, &state.pending)
        .await
        .expect("confirm");
    assert_eq!(confirmed.device_name, "中继测试机");
    let devices = paired_devices_db::get_all(&pool).await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_name, "中继测试机");

    // 第二段：手机重连（同公钥，免 nonce——已确认即 AlreadyPaired 直入会话）
    let (mut ws, mut t) = relay_e2e_connect(&relay_addr, &payload.relay_id, &phone_priv)
        .await
        .expect("确认后重连的 E2E 握手应在重试预算内完成");
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
    wait_for(
        || async { get_status(&pool, &state).await.unwrap().connected.is_some() },
        15000,
        "确认后重连进入 Connected",
    )
    .await;

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
    drop(ws);
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

// ═══════════════════════════════════════════════════════════════════
// Story 13.3：指令通道与流式对话
// ═══════════════════════════════════════════════════════════════════

/// 可记录调用的 fake 执行器（chat/task/suggestion 路由的注入缝）。
/// 响应经闭包按次构造（AppError 不可 Clone）。
struct FakeExecutor {
    calls: std::sync::Mutex<Vec<(String, serde_json::Value)>>,
    respond: Box<dyn Fn() -> Result<serde_json::Value, AppError> + Send + Sync>,
}

impl FakeExecutor {
    fn with_result(result: serde_json::Value) -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
            respond: Box::new(move || Ok(result.clone())),
        }
    }

    fn with_not_found(msg: &str) -> Self {
        let msg = msg.to_string();
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
            respond: Box::new(move || Err(AppError::NotFound(msg.clone()))),
        }
    }

    /// 闸门版：respond 阻塞在 mpsc::Receiver::recv()——释放前持闸，供并发
    /// 排队/过载闸门测试构造「首条在途」窗口（Mutex 包裹：Receiver 非 Sync）。
    fn with_gated_result(
        result: serde_json::Value,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    ) -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
            respond: Box::new(move || {
                release
                    .lock()
                    .unwrap()
                    .recv()
                    .expect("闸门释放信号未到达");
                Ok(result.clone())
            }),
        }
    }

    /// 已记录的执行调用次数（幂等测试断言去重是否真正生效）。
    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// 协变为注入缝所需的 trait 对象（保留具体 Arc 供 call_count 断言）。
    fn as_executor(self: &std::sync::Arc<Self>) -> std::sync::Arc<dyn CommandExecutor> {
        self.clone()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for FakeExecutor {
    async fn execute(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        self.calls
            .lock()
            .unwrap()
            .push((action.to_string(), params.clone()));
        (self.respond)()
    }
}

/// 构造 COMMAND 帧（envelope JSON 塞 data，与手机端 CommandModels 同构）。
fn command_frame(command_id: &str, action: &str, params_json: &str) -> Frame {
    let envelope = format!(
        r#"{{"schemaVersion":1,"commandId":"{command_id}","action":"{action}","params":{params_json}}}"#
    );
    Frame::Command(CommandPayload { data: envelope })
}

/// 等待 ack JSON 并断言（commandId 关联 + ok 形状）。
async fn parse_ack(ack_json: &str) -> serde_json::Value {
    serde_json::from_str(ack_json).expect("ack JSON 解析失败")
}

/// 手机侧等待 COMMAND_RESULT 并解析 ack JSON（跳过快照等其他帧），超时 panic。
async fn phone_receive_ack(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    t: &mut TransportSession,
    timeout_ms: u64,
) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "超时未收到 COMMAND_RESULT");
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("读帧超时")
            .expect("连接已关闭")
            .expect("WS 读取失败");
        let frame = decode_frame(&msg.into_data(), t).expect("帧解码失败");
        if let Frame::CommandResult(p) = &frame {
            return serde_json::from_str(&p.data).expect("ack JSON 解析失败");
        }
    }
}

/// 手机侧按序收集 STREAM_TOKEN 直到 done=true（跳过其他帧），超时 panic。
async fn phone_receive_stream_until_done(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    t: &mut TransportSession,
    timeout_ms: u64,
) -> Vec<serde_json::Value> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    let mut tokens = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "超时未收齐流式 token（done 收口）");
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("读帧超时")
            .expect("连接已关闭")
            .expect("WS 读取失败");
        let frame = decode_frame(&msg.into_data(), t).expect("帧解码失败");
        if let Frame::StreamToken(p) = &frame {
            let payload: serde_json::Value =
                serde_json::from_str(&p.data).expect("STREAM_TOKEN payload 解析失败");
            let done = payload.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
            tokens.push(payload);
            if done {
                return tokens;
            }
        }
    }
}

#[tokio::test]
async fn command_dispatch_routes_via_executor_and_memory_pool() {
    // WHY: dispatch 是手机指令唯一入口——chat/task/suggestion 必须经执行缝
    // 路由（生产=命令层 pub fn 直调），memory 必须纯 pool 真跑；任何旁路
    // 都意味着手机操作绕过桌面命令层的守卫/事件/幂等语义。
    let (pool, conv_pool, _dir) = test_pool().await;
    let _role_id = seed_snapshot_domain_data(&pool, &conv_pool).await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let executor = FakeExecutor::with_result(serde_json::json!({
        "conversationId": "c-1",
        "userMessageId": "u-1",
        "assistantMessageId": "a-1",
    }));
    let dispatcher = CompanionDispatcher::new(
        pool.clone(),
        conv_pool.clone(),
        tx,
        std::sync::Arc::new(executor),
    );

    // chat.send → 执行缝
    let ack = parse_ack(
        &dispatcher
            .execute(
                r#"{"schemaVersion":1,"commandId":"cmd-1","action":"chat.send","params":{"content":"你好"}}"#,
            )
            .await,
    )
    .await;
    assert_eq!(ack["commandId"], "cmd-1");
    assert_eq!(ack["ok"], true);
    assert_eq!(ack["result"]["conversationId"], "c-1");

    // memory.list → 纯 pool 真跑（不经执行缝）
    let ack = parse_ack(
        &dispatcher
            .execute(
                r#"{"schemaVersion":1,"commandId":"cmd-2","action":"memory.list","params":{}}"#,
            )
            .await,
    )
    .await;
    assert_eq!(ack["ok"], true, "memory.list 应真跑成功: {ack}");
    let memories = ack["result"]["memories"].as_array().expect("memories 数组");
    assert_eq!(memories.len(), 1, "种子记忆必须现查现显");
    assert_eq!(memories[0]["content"], "绝密记忆内容XYZ-不上机");
}

#[tokio::test]
async fn command_dispatch_idempotent_serial_replay_returns_first_result() {
    // WHY: 幂等去重是 14.1 速记队列的机制底座——确认丢失后的串行重放若
    // 重复执行，用户会在桌面看到重复入库的管家消息/任务。仅断言两次 ack
    // 相同无法证明去重存在（fake 每次返回同一 JSON），必须断言执行器
    // 只被调用一次。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let executor = std::sync::Arc::new(FakeExecutor::with_result(serde_json::json!({"ok": true})));
    let dispatcher = CompanionDispatcher::new(pool, conv_pool, tx, executor.as_executor());

    let envelope = r#"{"schemaVersion":1,"commandId":"cmd-dup","action":"chat.send","params":{"content":"重放"}}"#;
    let first = dispatcher.execute(envelope).await;
    let second = dispatcher.execute(envelope).await;
    assert_eq!(first, second, "串行重放必须返回首次结果");
    assert_eq!(
        executor.call_count(),
        1,
        "串行重放不得重复执行（去重必须真正拦截第二次路由）"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn command_dispatch_concurrent_same_id_queues_and_shares_first_result() {
    // WHY: 手机超时重发与首次慢执行并发时，同 commandId 各自执行会重复
    // 建任务/消息（幂等若只覆盖串行重放就挡不住这个窗口）——裁决 B：
    // 同号第二条排队等待首次完成并共享同一 ack。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let executor = std::sync::Arc::new(FakeExecutor::with_gated_result(
        serde_json::json!({"ok": 1}),
        std::sync::Mutex::new(release_rx),
    ));
    let dispatcher = std::sync::Arc::new(CompanionDispatcher::new(
        pool,
        conv_pool,
        tx,
        executor.as_executor(),
    ));
    let envelope =
        r#"{"schemaVersion":1,"commandId":"cmd-conc","action":"chat.send","params":{"content":"并发"}}"#
            .to_string();

    // 首条 spawn 后持闸阻塞（占住同号串行门与 1 个并发闸位）
    let d1 = std::sync::Arc::clone(&dispatcher);
    let env1 = envelope.clone();
    let first = tokio::spawn(async move { d1.execute(&env1).await });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while executor.call_count() < 1 {
        assert!(std::time::Instant::now() < deadline, "首条指令未进入执行");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // 并发同号第二条：排队等首条完成（不得先执行，也不得过载报错）
    let d2 = std::sync::Arc::clone(&dispatcher);
    let env2 = envelope.clone();
    let second = tokio::spawn(async move { d2.execute(&env2).await });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await; // 让第二条确实排队在门上
    release_tx.send(()).expect("释放闸门");

    let first = first.await.expect("首条任务 panic");
    let second = second.await.expect("第二条任务 panic");
    assert_eq!(first, second, "排队者必须共享首次结果");
    assert_eq!(
        executor.call_count(),
        1,
        "同 commandId 并发不得重复执行（排队+双检缓存生效）"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn command_dispatch_overload_rejected_beyond_concurrency_limit() {
    // WHY: 每帧无条件 spawn 直通 DB——配对设备帧洪泛可无界耗尽资源。
    // 裁决 A：并发闸门超限立即回过载错误 ack（不断连、不排队、不执行）。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let executor = std::sync::Arc::new(FakeExecutor::with_gated_result(
        serde_json::json!({"ok": 1}),
        std::sync::Mutex::new(release_rx),
    ));
    let dispatcher = std::sync::Arc::new(CompanionDispatcher::with_command_limit(
        pool,
        conv_pool,
        tx,
        executor.as_executor(),
        1,
    ));

    // 首条（cmd-a）持闸占用唯一闸位
    let d1 = std::sync::Arc::clone(&dispatcher);
    let first = tokio::spawn(async move {
        d1.execute(r#"{"schemaVersion":1,"commandId":"cmd-a","action":"chat.send","params":{"content":"占位"}}"#)
            .await
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while executor.call_count() < 1 {
        assert!(std::time::Instant::now() < deadline, "首条指令未进入执行");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // 异号第二条：缓存/串行门都不适用，闸门满 → 立即过载错误
    let second = dispatcher
        .execute(r#"{"schemaVersion":1,"commandId":"cmd-b","action":"chat.send","params":{"content":"超限"}}"#)
        .await;
    let ack = parse_ack(&second).await;
    assert_eq!(ack["commandId"], "cmd-b");
    assert_eq!(ack["ok"], false);
    assert!(
        ack["error"]["ConnectionError"].is_string(),
        "过载必须是 ConnectionError 分类: {ack}"
    );
    assert_eq!(executor.call_count(), 1, "过载指令不得进入执行");

    release_tx.send(()).expect("释放闸门");
    let first = parse_ack(&first.await.expect("首条任务 panic")).await;
    assert_eq!(first["ok"], true, "持闸指令正常完成不受闸门影响");
}

#[tokio::test]
async fn command_dispatch_oversized_result_replaced_with_error_ack() {
    // WHY: 超限 ack 塞进出站通道后会被编码层拒绝——表现为整条连接断开；
    // 必须在 dispatch 出口降级为显式错误 ack（诚实失败而非断流）。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let big = serde_json::json!({ "blob": "x".repeat(70000) });
    let executor = std::sync::Arc::new(FakeExecutor::with_result(big));
    let dispatcher = CompanionDispatcher::new(pool, conv_pool, tx, executor.as_executor());

    let ack_json = dispatcher
        .execute(r#"{"schemaVersion":1,"commandId":"cmd-big","action":"suggestion.list","params":{"conversationId":"c"}}"#)
        .await;
    assert!(
        ack_json.len() <= COMMAND_DATA_MAX_BYTES,
        "ack 必须在单帧上限内: {}",
        ack_json.len()
    );
    let ack = parse_ack(&ack_json).await;
    assert_eq!(ack["commandId"], "cmd-big");
    assert_eq!(ack["ok"], false);
    assert!(
        ack["error"]["ValidationError"].is_string(),
        "超限结果必须降级为 ValidationError: {ack}"
    );
}

#[tokio::test]
async fn command_dispatch_chat_send_blank_content_rejected() {
    // WHY: 命令层无空内容守卫，手机指令路径绕过前端 UI 校验——空消息
    // 会落库并触发一轮空 LLM 流（浪费 token 与流式状态机）。dispatch
    // 层在路由前拦下，执行器不得被触达。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let executor = std::sync::Arc::new(FakeExecutor::with_result(serde_json::json!({"ok": 1})));
    let dispatcher = CompanionDispatcher::new(pool, conv_pool, tx, executor.as_executor());

    let ack = parse_ack(
        &dispatcher
            .execute(r#"{"schemaVersion":1,"commandId":"cmd-blank","action":"chat.send","params":{"content":"   "}}"#)
            .await,
    )
    .await;
    assert_eq!(ack["ok"], false);
    assert_eq!(
        ack["error"],
        serde_json::json!({"ValidationError": "消息内容不能为空"}),
        "空白内容必须显式拒绝: {ack}"
    );
    assert_eq!(executor.call_count(), 0, "空白内容不得触达执行器");
}

#[tokio::test]
async fn command_dispatch_cache_evicts_oldest_beyond_capacity() {
    // WHY: 幂等缓存无界增长 = 内存泄漏（长跑桌面）；容量上限 + 淘汰最旧
    // 保证有界。淘汰后重放重新执行——缓存是优化不是状态，语义不变。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let executor = std::sync::Arc::new(FakeExecutor::with_result(serde_json::json!({"n": 0})));
    let dispatcher = CompanionDispatcher::new(pool, conv_pool, tx, executor.as_executor());

    // 1001 条不同指令——第 1 条（cmd-0）被淘汰出 1000 容量缓存
    for i in 0..1001 {
        let envelope = format!(
            r#"{{"schemaVersion":1,"commandId":"cmd-{i}","action":"chat.stop","params":{{"conversationId":"c"}}}}"#
        );
        dispatcher.execute(&envelope).await;
    }
    // 被淘汰的 cmd-0 重放：重新执行（结果正确即可，不 panic 不悬挂）
    let envelope = r#"{"schemaVersion":1,"commandId":"cmd-0","action":"chat.stop","params":{"conversationId":"c"}}"#;
    let ack = parse_ack(&dispatcher.execute(envelope).await).await;
    assert_eq!(ack["commandId"], "cmd-0");
    assert_eq!(ack["ok"], true);
    // 淘汰语义证明：重放确实重新触达执行器（1001 次 + 重放 1 次）
    assert_eq!(
        executor.call_count(),
        1002,
        "被淘汰的缓存项重放必须重新执行（缓存是优化不是状态）"
    );
}

#[tokio::test]
async fn command_dispatch_unknown_action_and_parse_errors_return_typed_ack() {
    // WHY: 未知 action/缺 commandId/超限都必须显式报错——静默忽略会让
    // 手机端 pending 永远等不到回执（悬挂），违反 AC2。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let executor = FakeExecutor::with_result(serde_json::json!({}));
    let dispatcher = CompanionDispatcher::new(pool, conv_pool, tx, std::sync::Arc::new(executor));

    // 未知 action → ValidationError「不支持的指令类型」单键 map
    let ack = parse_ack(
        &dispatcher
            .execute(r#"{"schemaVersion":1,"commandId":"cmd-x","action":"task.delete","params":{}}"#)
            .await,
    )
    .await;
    assert_eq!(ack["ok"], false);
    assert_eq!(
        ack["error"],
        serde_json::json!({"ValidationError": "不支持的指令类型"}),
        "未知 action 的错误分类形状: {ack}"
    );

    // 缺 commandId → 显式错误（空 commandId 回流）
    let ack = parse_ack(
        &dispatcher
            .execute(r#"{"schemaVersion":1,"action":"chat.send","params":{}}"#)
            .await,
    )
    .await;
    assert_eq!(ack["ok"], false);
    assert!(
        ack["error"]["ValidationError"].is_string(),
        "缺 commandId 必须显式报错: {ack}"
    );

    // 坏 JSON → 显式错误
    let ack = parse_ack(&dispatcher.execute("not json").await).await;
    assert_eq!(ack["ok"], false);

    // payload 超限 → 显式错误
    let oversized = format!(
        r#"{{"schemaVersion":1,"commandId":"c","action":"chat.send","params":{{"content":"{}"}}}}"#,
        "x".repeat(70000)
    );
    let ack = parse_ack(&dispatcher.execute(&oversized).await).await;
    assert_eq!(ack["ok"], false);
    assert!(
        ack["error"]["ValidationError"].is_string(),
        "超限必须显式报错: {ack}"
    );

    // 执行器错误 → AppError 分类原样回流
    let (pool2, conv_pool2, _dir2) = test_pool().await;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let dispatcher = CompanionDispatcher::new(
        pool2,
        conv_pool2,
        tx,
        std::sync::Arc::new(FakeExecutor::with_not_found("任务不存在")),
    );
    let ack = parse_ack(
        &dispatcher
            .execute(r#"{"schemaVersion":1,"commandId":"cmd-e","action":"task.toggle","params":{"taskId":"t","isCompleted":true}}"#)
            .await,
    )
    .await;
    assert_eq!(
        ack["error"],
        serde_json::json!({"NotFound": "任务不存在"}),
        "执行器错误必须按 AppError 单键 map 回流: {ack}"
    );
}

#[tokio::test]
async fn command_dispatch_suggestion_confirm_supplements_write_signal() {
    // WHY: 既有缺口——confirm 建任务但不 emit 事件，手机 STATE_DELTA 永远
    // 不含确认产生的任务（「确认后任务消失」）。dispatch 补发写信号收口。
    let (pool, conv_pool, _dir) = test_pool().await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let executor = FakeExecutor::with_result(serde_json::json!({"id": "s1", "status": "confirmed"}));
    let dispatcher = CompanionDispatcher::new(pool, conv_pool, tx, std::sync::Arc::new(executor));

    dispatcher
        .execute(r#"{"schemaVersion":1,"commandId":"cmd-s","action":"suggestion.confirm","params":{"suggestionId":"s1"}}"#)
        .await;

    let signal = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("确认建议后必须补发写信号")
        .expect("写信号通道未关闭");
    assert_eq!(signal.event, "task:created", "确认建任务的信号语义");

    // 拒绝同样补发（信号事件名区分语义）
    dispatcher
        .execute(r#"{"schemaVersion":1,"commandId":"cmd-s2","action":"suggestion.reject","params":{"suggestionId":"s1","reason":"bad_timing"}}"#)
        .await;
    let signal = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("拒绝建议后必须补发写信号")
        .expect("写信号通道未关闭");
    assert_eq!(signal.event, "suggestion:rejected");

    // 执行失败不补发（错误已按 ack 回流，快照无需重建）
    let (pool2, conv_pool2, _dir2) = test_pool().await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let dispatcher = CompanionDispatcher::new(
        pool2,
        conv_pool2,
        tx,
        std::sync::Arc::new(FakeExecutor::with_not_found("无")),
    );
    dispatcher
        .execute(r#"{"schemaVersion":1,"commandId":"cmd-s3","action":"suggestion.confirm","params":{"suggestionId":"missing"}}"#)
        .await;
    assert!(
        tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .is_err(),
        "执行失败不得补发写信号"
    );
}

#[test]
fn mirror_stream_payload_passes_through_and_guards() {
    // WHY: STREAM_TOKEN data 必须是 llm:stream payload 原样 JSON——手机端
    // 流式状态机按同名字段解析，任何重排/重组都造成双端状态机漂移。
    let token = r#"{"conversationId":"c1","token":"你","done":false,"thinking":false}"#;
    let frame = mirror_stream_payload(token).expect("合法 payload 必须镜像");
    match frame {
        Frame::StreamToken(p) => assert_eq!(p.data, token, "data 必须原样透传"),
        other => panic!("镜像必须是 StreamToken 帧: {other:?}"),
    }

    // done 帧（流结束信号）同样原样镜像
    let done = r#"{"conversationId":"c1","token":"","done":true,"thinking":false,"phase":"done"}"#;
    assert!(
        mirror_stream_payload(done).is_some(),
        "done 帧必须镜像（流收口信号）"
    );

    // 非 JSON / 超限 → None（fail-safe：宁跳过不断流）
    assert!(mirror_stream_payload("not json").is_none(), "非法 JSON 跳过");
    let oversize = format!(
        r#"{{"conversationId":"c","token":"{}","done":false}}"#,
        "x".repeat(70000)
    );
    assert!(mirror_stream_payload(&oversize).is_none(), "超限 payload 跳过");
}

#[tokio::test]
async fn phone_command_memory_list_roundtrip_over_real_socket() {
    // WHY: 真链路验证指令闭环——手机 COMMAND 经 Noise 会话到达 dispatch，
    // 结果以 COMMAND_RESULT 回流；这条链路断一处，手机记忆页就是永远
    // 的空态（AC5）。
    let (pool, conv_pool, state, engine, port, _dir) =
        setup_listener_with_engine(Duration::from_millis(80)).await;
    seed_snapshot_domain_data(&pool, &conv_pool).await;
    state.open_pairing_window("nonce-cmd".to_string()).await;
    // dispatcher 装配：真 pool（memory 真跑）+ fake 执行器（本轮不触达）
    state
        .set_dispatcher(Arc::new(CompanionDispatcher::new(
            pool.clone(),
            conv_pool.clone(),
            engine.notify_signal(),
            std::sync::Arc::new(FakeExecutor::with_result(serde_json::json!({}))),
        )))
        .await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, mut t) =
        phone_pair_and_connect(port, &phone_priv, "nonce-cmd", &pool, &state).await;

    send_frame(&mut ws, &mut t, &command_frame("cmd-mem", "memory.list", "{}")).await;

    let ack = phone_receive_ack(&mut ws, &mut t, 15000).await;
    assert_eq!(ack["commandId"], "cmd-mem", "ack 必须按 commandId 关联");
    assert_eq!(ack["ok"], true, "memory.list 真跑必须成功: {ack}");
    let memories = ack["result"]["memories"].as_array().expect("memories 数组");
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0]["content"], "绝密记忆内容XYZ-不上机");
}

#[tokio::test]
async fn phone_chat_send_ack_then_stream_tokens_in_order() {
    // WHY: 流式收口链路——chat.send 的 COMMAND_RESULT 即时确认（id 对齐），
    // STREAM_TOKEN 按序回流、done 帧收口；顺序错乱会让手机逐字渲染
    // 变成乱序拼接（AC3）。
    let (pool, conv_pool, state, engine, port, _dir) =
        setup_listener_with_engine(Duration::from_millis(80)).await;
    seed_snapshot_domain_data(&pool, &conv_pool).await;
    state.open_pairing_window("nonce-chat".to_string()).await;
    let canned = serde_json::json!({
        "conversationId": "conv-13",
        "userMessageId": "msg-u",
        "assistantMessageId": "msg-a",
    });
    state
        .set_dispatcher(Arc::new(CompanionDispatcher::new(
            pool.clone(),
            conv_pool.clone(),
            engine.notify_signal(),
            std::sync::Arc::new(FakeExecutor::with_result(canned.clone())),
        )))
        .await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, mut t) =
        phone_pair_and_connect(port, &phone_priv, "nonce-chat", &pool, &state).await;

    // 手机发送 chat.send → ack 即时确认（含会话/消息 id 对齐）
    send_frame(
        &mut ws,
        &mut t,
        &command_frame("cmd-chat", "chat.send", r#"{"content":"早上好"}"#),
    )
    .await;
    let ack = phone_receive_ack(&mut ws, &mut t, 15000).await;
    assert_eq!(ack["commandId"], "cmd-chat");
    assert_eq!(ack["ok"], true);
    assert_eq!(ack["result"], canned, "ack 必须携带三 id 对齐字段");

    // 直调 mirror 模拟桌面 llm:stream token 流（生产由 register_stream_mirror
    // 监听驱动；此处注入纯函数产物验证出站顺序与 done 收口）
    let payloads = [
        r#"{"conversationId":"conv-13","token":"早","done":false,"thinking":false}"#,
        r#"{"conversationId":"conv-13","token":"上好","done":false,"thinking":false}"#,
        r#"{"conversationId":"conv-13","token":"呀","done":false,"thinking":false,"phase":"done"}"#,
        r#"{"conversationId":"conv-13","token":"","done":true,"thinking":false,"phase":"done"}"#,
    ];
    for payload in payloads {
        let frame = mirror_stream_payload(payload).expect("镜像");
        assert!(
            state.try_enqueue_single(frame).await,
            "会话在线期间单帧入队必须成功"
        );
    }

    let tokens = phone_receive_stream_until_done(&mut ws, &mut t, 15000).await;
    assert_eq!(tokens.len(), 4, "全部 token + done 按序到达");
    assert_eq!(tokens[0]["token"], "早");
    assert_eq!(tokens[1]["token"], "上好");
    assert_eq!(tokens[2]["token"], "呀");
    assert_eq!(tokens[3]["done"], true, "最后一个必须是 done 收口帧");
}

#[tokio::test]
async fn try_enqueue_single_drops_when_full_without_disconnect() {
    // WHY: 出站双语义——流式 token 满则丢弃不断连（快照兜底收敛）；若误用
    // 整序列语义（满则断连），慢链路下手机每次对话都会被踢下线重连。
    let (pool, conv_pool, state, engine, port, _dir) =
        setup_listener_with_engine(Duration::from_millis(80)).await;
    seed_snapshot_domain_data(&pool, &conv_pool).await;
    state.open_pairing_window("nonce-full".to_string()).await;
    state
        .set_dispatcher(Arc::new(CompanionDispatcher::new(
            pool.clone(),
            conv_pool.clone(),
            engine.notify_signal(),
            std::sync::Arc::new(FakeExecutor::with_result(serde_json::json!({}))),
        )))
        .await;

    let (phone_priv, _) = generate_static_keypair().unwrap();
    let (mut ws, mut t) =
        phone_pair_and_connect(port, &phone_priv, "nonce-full", &pool, &state).await;

    // 手机不读 → TCP 窗口饱和 → 会话出站通道（512 容量）填满。
    // 700 × 60KB ≈ 42MB >> TCP 缓冲（数 MB）——必然溢出。
    let big_token = "x".repeat(60_000);
    let mut dropped = 0usize;
    let mut enqueued = 0usize;
    for i in 0..700 {
        let payload = format!(
            r#"{{"conversationId":"c","token":"{big_token}","done":false,"seq":{i}}}"#
        );
        let frame = mirror_stream_payload(&payload).expect("60KB payload 在单帧上限内");
        if state.try_enqueue_single(frame).await {
            enqueued += 1;
        } else {
            dropped += 1;
        }
    }
    assert!(dropped > 0, "通道填满后必须丢弃（入队 {enqueued}/丢弃 {dropped}）");

    // 手机开始读取（排空通道）与 done 标记入队重试并发进行——标记位于
    // 队尾，其到达即证明此前的入队帧已按序全部送达。
    let reader = tokio::spawn(async move {
        let mut received = 0usize;
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(60), ws.next())
                .await
                .expect("读帧超时")
                .expect("连接已关闭")
                .expect("WS 读取失败");
            let frame = decode_frame(&msg.into_data(), &mut t).expect("帧解码失败");
            if let Frame::StreamToken(p) = &frame {
                let payload: serde_json::Value =
                    serde_json::from_str(&p.data).expect("payload 解析失败");
                if payload.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    break;
                }
                received += 1;
            }
        }
        (ws, t, received)
    });

    let marker_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let marker = mirror_stream_payload(r#"{"conversationId":"c","token":"","done":true}"#)
            .expect("标记镜像");
        if state.try_enqueue_single(marker).await {
            break;
        }
        assert!(
            marker_deadline.saturating_duration_since(tokio::time::Instant::now())
                > Duration::ZERO,
            "超时未能入队 done 标记"
        );
        sleep(Duration::from_millis(50)).await;
    }

    let (mut ws, mut t, received) = reader.await.expect("读取任务完成");
    assert_eq!(
        received, enqueued,
        "入队的帧必须全部按序送达（丢弃的恰为未入队者）"
    );

    // 连接必须仍然存活：排空后 PING/PONG 往返
    send_frame(
        &mut ws,
        &mut t,
        &Frame::Ping(companion_proto::frames::PingPayload {}),
    )
    .await;
    let pong_deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = pong_deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "超时未收到 PONG——满丢弃不得断连");
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("读帧超时")
            .expect("连接已关闭")
            .expect("WS 读取失败");
        let frame = decode_frame(&msg.into_data(), &mut t).expect("帧解码失败");
        if matches!(frame, Frame::Ping(_)) {
            break; // PONG 到达——连接存活
        }
    }
}
