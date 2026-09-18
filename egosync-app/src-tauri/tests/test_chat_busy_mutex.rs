//! Story 15.3 特征测试：同会话并发 busy 互斥（护栏先行——抽取前代码上全绿）。
//!
//! WHY: 同会话并发 busy 语义（恰一次 LLM 调用、恰一条 busy 落库、busy 经
//! Ok 通道返回）此前零测试覆盖，chat/agent_engine 域迁移的「桌面零回归」
//! 防线对它是空集。本测试在抽取前把行为钉死，迁移后断言逐项重跑一致，
//! 即锁粒度守恒的执行级证据：
//! - 不变量①：busy check+insert 与完成 remove 同锁（互斥根）
//! - 不变量②：忙分支唯一跨 await 持锁点（恰一条 busy 的根）
//! - companion 隐式协议：busy 消息 role=assistant + 固定文案
//!   （companion_dispatch 以 `msg.role != "user"` 探测 busy）
//!
//! hold 机制：AgentBridge reqwest 无超时（services/agent_bridge.rs:20-26），
//! 挂起 TcpListener accept 后不响应 → run_stream 停在 create_session await，
//! busy 窗口确定性开启；断点后 drop 连接 → run_stream Err → 兜底 done 帧
//! → 清理闭包移除 streaming 标记（测试尾部等待清空，验证全生命周期）。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use egosync_lib::commands::chat::{chat_send_message, ChatSessionRegistry};
use egosync_lib::db::conversations as conversations_db;
use egosync_lib::db::pool::{init_conversations_db, init_db, ConversationsPool, DbPool};
use egosync_lib::models::chat::ChatRequest;
use egosync_lib::models::chat::Message;
use egosync_lib::error::AppError;
use egosync_lib::services::agent_bridge::AgentBridge;
use egosync_lib::services::agent_config::AgentConfigService;
use egosync_lib::services::delegate_bridge::DelegateBridge;
use egosync_lib::services::event_router::EventRouter;
use egosync_lib::services::sidecar::SidecarManager;
use egosync_lib::services::tauri_event_bus::TauriEventBus;
use tokio::sync::Mutex;
use tauri::Manager;

/// 密钥接缝测试替身：busy 判定路径不触真实密钥库（空配置降级由 provider 层兜底）。
struct NoopSecretStore;
impl egosync_engine::services::secret_store::SecretStore for NoopSecretStore {
    fn save_secret(&self, _key: &str, _value: &str) -> Result<(), AppError> { Ok(()) }
    fn load_secret(&self, _key: &str) -> Result<Option<String>, AppError> { Ok(None) }
    fn delete_secret(&self, _key: &str) -> Result<(), AppError> { Ok(()) }
}

const BUSY_TEXT: &str = "我还在想上一个问题，请稍等片刻...";
const WAIT_MS: u64 = 50;
const WAIT_ROUNDS: usize = 200; // 10s 上限

/// 挂起式 mock opencode：accept 计数并持有连接不响应。
struct HangingOpencode {
    port: u16,
    accepted: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl HangingOpencode {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock opencode");
        let port = listener.local_addr().expect("mock addr").port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = accepted.clone();
        let task = tokio::spawn(async move {
            let mut held = Vec::new();
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        counter.fetch_add(1, Ordering::SeqCst);
                        held.push(stream);
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            port,
            accepted,
            task,
        }
    }

    /// 断点清理：终止 accept 循环 → 持有的连接全部关闭 → 客户端 Err。
    fn drop_connections(self) {
        self.task.abort();
    }
}

type TestApp = tauri::App<tauri::test::MockRuntime>;

/// mock_app 注入 run_stream 依赖清单全量状态（双池走完整迁移的真实 schema）。
async fn setup() -> (TestApp, DbPool, ConversationsPool, HangingOpencode, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let pool = init_db(&dir.path().join("main.db"))
        .await
        .expect("init main db with migrations");
    let conv_pool = init_conversations_db(&dir.path().join("conv.db"))
        .await
        .expect("init conversations db with migrations");
    let opencode = HangingOpencode::start().await;

    let app = tauri::test::mock_app();
    app.manage(pool.clone());
    app.manage(conv_pool.clone());
    // Story 15.3：六组会话状态合并为单 Registry（Arc，与 lib.rs 单 manage 同构）
    app.manage(Arc::new(ChatSessionRegistry::default()));
    // 命令签名 +State<TauriEventBus>（事件经注入总线发射）
    app.manage(TauriEventBus::new(app.handle().clone()));
    app.manage(AgentConfigService::new(dir.path().join("opencode.json")));
    app.manage(Arc::new(Mutex::new(SidecarManager::new(None, None))));
    app.manage(AgentBridge::new(opencode.port));
    app.manage(Arc::new(EventRouter::new()));
    // Story 15.3：DelegateBridge 迁引擎后接缝注入（总线/配置/Skill 根/密钥测试替身）
    app.manage(DelegateBridge::new(
        pool.clone(),
        conv_pool.clone(),
        "test-token".into(),
        None,
        AgentConfigService::new(dir.path().join("opencode.json")),
        dir.path().join("skills"),
        Arc::new(NoopSecretStore),
    ));
    (app, pool, conv_pool, opencode, dir)
}

fn chat_request(conversation_id: &str, content: &str, workspace: &str) -> ChatRequest {
    ChatRequest {
        conversation_id: Some(conversation_id.to_string()),
        role_id: None,
        content: content.to_string(),
        // 注入显式工作目录：绕开 app_data_dir 依赖，busy 窗口不依赖宿主路径
        working_directory: Some(workspace.to_string()),
        onboarding_step: 0,
        selected_skill_id: None,
    }
}

async fn send(app: &TestApp, request: ChatRequest) -> Result<Message, AppError> {
    chat_send_message(
        request,
        app.state::<DbPool>(),
        app.state::<ConversationsPool>(),
        app.state::<Arc<ChatSessionRegistry>>(),
        app.state::<AgentConfigService>(),
        app.state::<TauriEventBus<tauri::test::MockRuntime>>(),
        app.handle().clone(),
    )
    .await
}

async fn wait_for_accepted(counter: &AtomicUsize, expected: usize) {
    for _ in 0..WAIT_ROUNDS {
        if counter.load(Ordering::SeqCst) >= expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_MS)).await;
    }
    panic!(
        "mock opencode 连接数未达到 {expected}（当前 {}，{}ms 超时）",
        counter.load(Ordering::SeqCst),
        WAIT_ROUNDS as u64 * WAIT_MS
    );
}

/// 等待清理闭包移除 streaming 标记（断点后 run_stream Err → 兜底 done → 清理）。
async fn wait_streaming_empty(app: &TestApp) {
    for _ in 0..WAIT_ROUNDS {
        if app.state::<Arc<ChatSessionRegistry>>().inner().streaming_state.0.lock().await.is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_MS)).await;
    }
    panic!("断开 mock opencode 后 streaming 标记未被清理闭包移除");
}

/// 等待清理闭包移除 cancel token（评审补丁：清理生命周期另一半——
/// 模块头宣称「全生命周期」，token 不归零即闭包未走完）。
async fn wait_cancel_tokens_empty(app: &TestApp) {
    for _ in 0..WAIT_ROUNDS {
        if app.state::<Arc<ChatSessionRegistry>>().inner().cancel_tokens.0.lock().await.is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(WAIT_MS)).await;
    }
    panic!("断开 mock opencode 后 cancel token 未被清理闭包移除");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_conversation_concurrent_sends_yield_exactly_one_busy_reply() {
    let (app, _pool, conv_pool, opencode, dir) = setup().await;
    let conv = conversations_db::create_conversation(&conv_pool, None)
        .await
        .expect("create conversation");
    let workspace = dir.path().to_string_lossy().to_string();

    // 并发双发：join! 同批派发两条命令，锁竞争决定先后——无屏障，
    // busy 语义由「锁内 contains→insert」保证而非同刻起跑（评审补丁：
    // 删除从未 await 的死屏障及其误导注释）。
    let (ra, rb) = tokio::join!(
        send(&app, chat_request(&conv.id, "第一条消息", &workspace)),
        send(&app, chat_request(&conv.id, "第二条消息", &workspace)),
    );
    let msg_a = ra.expect("并发双发双方都经 Ok 通道返回（busy 不是 Err）");
    let msg_b = rb.expect("并发双发双方都经 Ok 通道返回（busy 不是 Err）");

    // 一条走 user 路径、另一条 busy assistant 提示（companion 探测协议根）
    let (busy, _user) = if msg_a.role == "assistant" {
        (&msg_a, &msg_b)
    } else {
        (&msg_b, &msg_a)
    };
    assert_eq!(busy.role, "assistant");
    assert_eq!(busy.content, BUSY_TEXT);

    // 拒绝方零副作用：不插 user 消息（库里恰一条本轮 user 消息）
    let messages = conversations_db::list_messages(&conv_pool, &conv.id)
        .await
        .expect("list messages");
    let user_rows: Vec<_> = messages
        .iter()
        .filter(|m| m.role == "user" && (m.content == "第一条消息" || m.content == "第二条消息"))
        .collect();
    assert_eq!(user_rows.len(), 1, "拒绝方不得插入 user 消息");
    // 恰一条 busy 落库（持锁跨 await 的 insert 是唯一 busy 写点）
    let busy_rows: Vec<_> = messages
        .iter()
        .filter(|m| m.role == "assistant" && m.content == BUSY_TEXT)
        .collect();
    assert_eq!(busy_rows.len(), 1, "恰一条 busy 落库");

    // 恰一次 LLM 调用：拒绝方不 spawn run_stream → mock 只收到一次连接
    wait_for_accepted(&opencode.accepted, 1).await;
    assert_eq!(opencode.accepted.load(Ordering::SeqCst), 1);

    // 恰一个 cancel token（接受方）；拒绝方不建 token
    let tokens = app.state::<Arc<ChatSessionRegistry>>().inner().cancel_tokens.0.lock().await;
    assert_eq!(tokens.len(), 1, "拒绝方不得创建 cancel token");
    drop(tokens);

    // busy 窗口仍开启（首条在途挂起）：streaming 集合含本会话
    let streaming = app.state::<Arc<ChatSessionRegistry>>().inner().streaming_state.0.lock().await;
    assert!(
        streaming.contains(&conv.id),
        "mock 挂起期间会话保持 streaming（busy 互斥根）"
    );
    drop(streaming);

    // 断点清理：drop 连接 → run_stream Err → 兜底 done 帧 → 清理闭包移除标记
    opencode.drop_connections();
    wait_streaming_empty(&app).await;
    // 清理生命周期收尾（评审补丁）：token 同被闭包移除，归零才闭环保全。
    wait_cancel_tokens_empty(&app).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn distinct_conversations_stream_concurrently_without_busy() {
    let (app, _pool, conv_pool, opencode, dir) = setup().await;
    let conv_a = conversations_db::create_conversation(&conv_pool, None)
        .await
        .expect("create conversation A");
    let conv_b = conversations_db::create_conversation(&conv_pool, None)
        .await
        .expect("create conversation B");
    let workspace = dir.path().to_string_lossy().to_string();

    let (ra, rb) = tokio::join!(
        send(&app, chat_request(&conv_a.id, "A 的问题", &workspace)),
        send(&app, chat_request(&conv_b.id, "B 的问题", &workspace)),
    );
    let msg_a = ra.expect("异会话并发双方正常进入流式");
    let msg_b = rb.expect("异会话并发双方正常进入流式");

    // 锁粒度非全局串行的证据：双方都是 user 路径（busy 零命中）
    assert_eq!(msg_a.role, "user");
    assert_eq!(msg_b.role, "user");
    for conv in [&conv_a, &conv_b] {
        let messages = conversations_db::list_messages(&conv_pool, &conv.id)
            .await
            .expect("list messages");
        assert!(
            !messages.iter().any(|m| m.content == BUSY_TEXT),
            "异会话并发不得触发 busy"
        );
    }

    // 各自进入流式：busy 互斥锁按会话粒度互不影响，streaming 集合同持两个会话。
    // （McpScopeLock 串行化「MCP 同步+会话创建」是另一条独立不变量④——
    // 首条挂起期间第二条的 create_session 在其后排队，连接数不作为本测试断言。）
    let streaming = app.state::<Arc<ChatSessionRegistry>>().inner().streaming_state.0.lock().await;
    assert!(
        streaming.contains(&conv_a.id) && streaming.contains(&conv_b.id),
        "异会话互不阻塞，各自保持 streaming"
    );
    drop(streaming);
    wait_for_accepted(&opencode.accepted, 1).await;

    opencode.drop_connections();
    wait_streaming_empty(&app).await;
    wait_cancel_tokens_empty(&app).await;
}

/// 评审补丁（验证缺口层）：chat 域事件发射的执行级验证——命令层 emit
/// 经 TauriEventBus 广播后监听端真实可达（值钉/contains 钉/source-scan
/// 均为静态断言，删任一 emit 全绿——本测试补上「发射↔订阅」执行半边）。
/// 同时钉死冻结协议：busy 分支不发射 message:saved（busy 提示若发
/// message:saved 会触发 companion STATE_DELTA，改变快照推送语义）。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chat_events_reach_listeners_and_busy_branch_emits_nothing() {
    use egosync_lib::commands::chat::{chat_delete_conversation, chat_new_conversation};
    use egosync_lib::events::{
        CONVERSATION_CREATED_EVENT, CONVERSATION_DELETED_EVENT, MESSAGE_SAVED_EVENT,
    };
    use tauri::Listener;

    let (app, _pool, conv_pool, opencode, dir) = setup().await;
    let workspace = dir.path().to_string_lossy().to_string();

    // 监听端：MockRuntime 事件环上收 message:saved（计数而非内容——
    // payload 形状已由 engine models/chat.rs 既有序列化测试钉住）。
    let saved_count = Arc::new(AtomicUsize::new(0));
    let sink = saved_count.clone();
    app.handle().listen(MESSAGE_SAVED_EVENT, move |_event| {
        sink.fetch_add(1, Ordering::SeqCst);
    });

    // 会话命令对：created 与 deleted 事件经注入 TauriEventBus 到达监听端。
    let created_count = Arc::new(AtomicUsize::new(0));
    let sink_c = created_count.clone();
    app.handle().listen(CONVERSATION_CREATED_EVENT, move |_event| {
        sink_c.fetch_add(1, Ordering::SeqCst);
    });
    let deleted_count = Arc::new(AtomicUsize::new(0));
    let sink_d = deleted_count.clone();
    app.handle().listen(CONVERSATION_DELETED_EVENT, move |_event| {
        sink_d.fetch_add(1, Ordering::SeqCst);
    });

    let conv = chat_new_conversation(
        None,
        None,
        app.state::<ConversationsPool>(),
        app.state::<DbPool>(),
        app.state::<TauriEventBus<tauri::test::MockRuntime>>(),
        app.handle().clone(),
    )
    .await
    .expect("new conversation");

    // 同会话并发双发：接受方恰发一条 message:saved（user 消息落库广播），
    // busy 方零发射——恰一条而非两条是 busy 协议静默性的执行级证据。
    let (ra, rb) = tokio::join!(
        send(&app, chat_request(&conv.id, "第一条消息", &workspace)),
        send(&app, chat_request(&conv.id, "第二条消息", &workspace)),
    );
    let _ = ra.expect("Ok 通道");
    let _ = rb.expect("Ok 通道");
    assert_eq!(
        saved_count.load(Ordering::SeqCst),
        1,
        "busy 分支不得发射 message:saved（恰一条 = 接受方 user 消息）"
    );

    chat_delete_conversation(
        conv.id.clone(),
        app.state::<ConversationsPool>(),
        app.state::<TauriEventBus<tauri::test::MockRuntime>>(),
        app.handle().clone(),
    )
    .await
    .expect("delete conversation");

    assert_eq!(created_count.load(Ordering::SeqCst), 1, "conversation:created 必达监听端");
    assert_eq!(deleted_count.load(Ordering::SeqCst), 1, "conversation:deleted 必达监听端");
    assert_eq!(
        saved_count.load(Ordering::SeqCst),
        1,
        "删除会话不得再发 message:saved"
    );

    // title-updated 走同一 emit_chat_event 助手（三个事件已执行级验证其
    // 总线通路；该事件的 generate_title 路径需真实 LLM 响应，不在本测试
    // 覆盖内——订阅侧由 WRITE_SIGNAL_EVENTS contains 钉守）。
    opencode.drop_connections();
    wait_streaming_empty(&app).await;
}
