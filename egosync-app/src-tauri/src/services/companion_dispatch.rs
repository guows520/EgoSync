//! Story 13.3：手机指令 dispatch 服务——手机远程操作桌面引擎的唯一入口。
//!
//! 硬边界：
//! - dispatch 是手机指令唯一入口：解析 envelope → 路由到既有命令层 pub fn /
//!   memory_query pub fn → 结果以 COMMAND_RESULT 回流；不出现任何绕过它
//!   直调 agent_bridge/opencode/DB 的路径。
//! - chat/task/会话/suggestion 路由经 [`CommandExecutor`] 注入缝（生产实现
//!   直调命令层 pub fn，测试注入 fake）；memory 路由纯 pool 可真跑。
//! - 执行入口 `execute` 由 `enter_session` 的 Command 臂 `tokio::spawn`
//!   调用，绝不阻塞会话帧循环（PING 应答/出站排空依赖帧循环活着）。
//! - 日志纪律（NFR-M7）：只记 action 名/计数，指令 JSON 与结果内容永不入日志。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use companion_proto::frames::{CommandResultPayload, Frame, StreamTokenPayload};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::chat::ChatRequest;
use crate::models::companion_command::{CommandAck, CommandEnvelope, COMMAND_DATA_MAX_BYTES};
use crate::models::task::CreateTaskInput;
use crate::services::companion_connection::CompanionState;
use crate::services::companion_snapshot::WriteSignal;
use crate::services::memory_query;

/// 幂等缓存容量上限：超限淘汰最旧（缓存内存持有，桌面重启清空——
/// 已声明边界，见 story Dev Notes §9.3）。
const ACK_CACHE_CAPACITY: usize = 1000;

/// 指令并发闸门上限（评审裁决 A）：超出立即回「指令过载」错误 ack，
/// 不排队不执行——防配对设备帧洪泛无界 spawn 直通 DB。
pub const COMMAND_CONCURRENCY_LIMIT: usize = 8;

/// STREAM_TOKEN 镜像中转通道容量：满则丢弃单帧 + warn（语义与
/// `try_enqueue_single` 一致——快照在 done 后兜底收敛，不断连）。
const MIRROR_CHANNEL_CAPACITY: usize = 256;

/// 指令集合（冻结——手机原型 UI 可触发的操作，勿扩）。
const KNOWN_ACTIONS: &[&str] = &[
    "chat.send",
    "chat.stop",
    "conversation.new",
    "conversation.delete",
    "task.toggle",
    "task.create",
    "suggestion.list",
    "suggestion.confirm",
    "suggestion.reject",
    "memory.list",
    "memory.sources",
    "memory.forget",
];

/// 生产执行器可受理的指令（单一事实源，评审整改）：字符串在此解析为
/// 枚举，执行器 match 对枚举穷尽——编译器强制覆盖，新增 action 漏臂
/// = 编译失败而非运行期「不支持的指令类型」矛盾。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProdAction {
    ChatSend,
    ChatStop,
    ConversationNew,
    ConversationDelete,
    TaskToggle,
    TaskCreate,
    SuggestionList,
    SuggestionConfirm,
    SuggestionReject,
}

impl ProdAction {
    fn parse(action: &str) -> Option<Self> {
        Some(match action {
            "chat.send" => Self::ChatSend,
            "chat.stop" => Self::ChatStop,
            "conversation.new" => Self::ConversationNew,
            "conversation.delete" => Self::ConversationDelete,
            "task.toggle" => Self::TaskToggle,
            "task.create" => Self::TaskCreate,
            "suggestion.list" => Self::SuggestionList,
            "suggestion.confirm" => Self::SuggestionConfirm,
            "suggestion.reject" => Self::SuggestionReject,
            _ => return None,
        })
    }
}

/// 非memory 类指令的执行缝：生产实现直调命令层 pub fn（经
/// `AppHandle::state` 现场取 State 参数）；测试注入 fake 断言路由。
#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError>;
}

/// 幂等缓存：commandId → 完成的 ack JSON（插入序淘汰最旧）。
struct AckCache {
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

impl AckCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, command_id: &str) -> Option<&String> {
        self.map.get(command_id)
    }

    fn insert(&mut self, command_id: String, ack: String) {
        if self.map.insert(command_id.clone(), ack).is_some() {
            return; // 已存在（串行重放不会走到；防御并发重复入队）
        }
        self.order.push_back(command_id);
        while self.order.len() > ACK_CACHE_CAPACITY {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
    }
}

/// 指令 dispatcher：幂等去重 + 路由 + 错误分类回流。
pub struct CompanionDispatcher {
    main_pool: DbPool,
    conv_pool: ConversationsPool,
    /// 快照引擎写信号（suggestion.confirm/reject 成功后补发——堵既有
    /// 「确认建任务不 emit」缺口；task/chat 路径命令层自带 emit 勿重复补）。
    write_signal_tx: mpsc::Sender<WriteSignal>,
    executor: Arc<dyn CommandExecutor>,
    ack_cache: std::sync::Mutex<AckCache>,
    /// 同 commandId 并发串行门（评审裁决 B）：第一条执行期间，后续同号
    /// 指令排队等待，完成后直接共享缓存中的首次结果，不重复执行。
    inflight: std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// 指令并发闸门（评审裁决 A）：限制同时在途的指令执行数，超限回过载错误。
    command_slots: Arc<Semaphore>,
}

impl CompanionDispatcher {
    /// 通用构造（测试注入 fake executor）；并发闸门用默认上限。
    pub fn new(
        main_pool: DbPool,
        conv_pool: ConversationsPool,
        write_signal_tx: mpsc::Sender<WriteSignal>,
        executor: Arc<dyn CommandExecutor>,
    ) -> Self {
        Self::with_command_limit(
            main_pool,
            conv_pool,
            write_signal_tx,
            executor,
            COMMAND_CONCURRENCY_LIMIT,
        )
    }

    /// 测试构造：注入并发闸门上限（覆盖过载路径——闸门值可调以便
    /// 用上限 1 触发过载分支，无需堆满 8 路并发）。
    pub fn with_command_limit(
        main_pool: DbPool,
        conv_pool: ConversationsPool,
        write_signal_tx: mpsc::Sender<WriteSignal>,
        executor: Arc<dyn CommandExecutor>,
        command_limit: usize,
    ) -> Self {
        Self {
            main_pool,
            conv_pool,
            write_signal_tx,
            executor,
            ack_cache: std::sync::Mutex::new(AckCache::new()),
            inflight: std::sync::Mutex::new(HashMap::new()),
            command_slots: Arc::new(Semaphore::new(command_limit)),
        }
    }

    /// 生产构造：AppHandle 直调命令层 pub fn 的执行器。
    pub fn production(
        main_pool: DbPool,
        conv_pool: ConversationsPool,
        app_handle: tauri::AppHandle,
        write_signal_tx: mpsc::Sender<WriteSignal>,
    ) -> Self {
        Self::new(
            main_pool,
            conv_pool,
            write_signal_tx,
            Arc::new(AppHandleCommandExecutor { app_handle }),
        )
    }

    /// 执行入口：解析 → 幂等查缓存 → 同号排队 → 闸门 → 路由 → ack JSON。
    /// 所有失败（含解析失败）都收敛为错误 ack 字符串，绝不 Err——
    /// 调用方（帧循环 spawn）只负责回发。
    pub async fn execute(&self, data: &str) -> String {
        let envelope = match CommandEnvelope::parse(data) {
            Ok(env) => env,
            Err(e) => {
                // 解析失败拿不到 commandId——空 id 回流，手机侧 pending
                // 按 commandId 匹配自然忽略（其自身发送前已本地校验）。
                tracing::warn!(error = %e, "companion 指令解析失败");
                return CommandAck::failure("", e).to_json();
            }
        };
        // 仅记 action 判别式（截断防超长 action 灌爆日志），不落 params/结果内容（NFR-M7）
        tracing::info!(action = %loggable_action(&envelope.action), "companion 指令到达");

        if let Some(cached) = self
            .ack_cache
            .lock()
            .unwrap()
            .get(&envelope.command_id)
            .cloned()
        {
            tracing::info!(
                action = %loggable_action(&envelope.action),
                "companion 指令幂等命中，返回首次结果"
            );
            return cached;
        }

        // 同号排队（裁决 B）：并发同 commandId 串行化，第二条等待首次
        // 完成后走缓存命中，共享同一 ack（不重复执行）。
        let entry = {
            let mut inflight = self.inflight.lock().unwrap();
            Arc::clone(
                inflight
                    .entry(envelope.command_id.clone())
                    .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
            )
        };
        let _guard = entry.lock().await;
        // 双检：排队期间首次可能已完成并写入缓存
        if let Some(cached) = self
            .ack_cache
            .lock()
            .unwrap()
            .get(&envelope.command_id)
            .cloned()
        {
            tracing::info!(
                action = %loggable_action(&envelope.action),
                "companion 指令幂等命中（排队后），返回首次结果"
            );
            self.release_inflight(&envelope.command_id, &entry);
            return cached;
        }

        // 并发闸门（裁决 A）：超出上限立即回过载错误，不排队不执行
        let _permit = match self.command_slots.try_acquire() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    action = %loggable_action(&envelope.action),
                    "companion 指令并发超限，回执过载错误"
                );
                return CommandAck::failure(
                    &envelope.command_id,
                    AppError::ConnectionError("指令过载，请稍后重试".to_string()),
                )
                .to_json();
            }
        };

        let result = self.route(&envelope).await;
        let ack = match result {
            Ok(value) => CommandAck::success(&envelope.command_id, value),
            Err(e) => CommandAck::failure(&envelope.command_id, e),
        };
        let mut json = ack.to_json();
        // 出口尺寸守卫：超限结果换成显式错误 ack——编码层拒绝会导致整条
        // 连接断开（不可接受），前置降级为「结果过大」错误 ack 更诚实。
        if json.len() > COMMAND_DATA_MAX_BYTES {
            tracing::warn!(
                action = %loggable_action(&envelope.action),
                bytes = json.len(),
                "companion 指令结果超单帧上限，回执错误而非断连"
            );
            json = CommandAck::failure(
                &envelope.command_id,
                AppError::ValidationError(format!(
                    "指令结果超出单帧上限（{} > {} 字节）",
                    json.len(),
                    COMMAND_DATA_MAX_BYTES
                )),
            )
            .to_json();
        }
        self.ack_cache
            .lock()
            .unwrap()
            .insert(envelope.command_id.clone(), json.clone());
        drop(_guard);
        drop(_permit);
        self.release_inflight(&envelope.command_id, &entry);
        json
    }

    /// 清理 inflight 表项：无排队者（仅映射表持有）时移除，防无界增长。
    /// 排队者在途时保留表项——其唤醒后经缓存命中自然出队。
    fn release_inflight(&self, command_id: &str, entry: &Arc<tokio::sync::Mutex<()>>) {
        let mut inflight = self.inflight.lock().unwrap();
        if let Some(current) = inflight.get(command_id) {
            if Arc::ptr_eq(current, entry) && Arc::strong_count(entry) == 2 {
                inflight.remove(command_id);
            }
        }
    }

    /// 路由：memory.* 直调 memory_query pub fn（纯 pool 可真跑）；
    /// 其余经 executor 注入缝（生产 = 命令层 pub fn 直调）。
    async fn route(&self, envelope: &CommandEnvelope) -> Result<serde_json::Value, AppError> {
        if !KNOWN_ACTIONS.contains(&envelope.action.as_str()) {
            return Err(AppError::ValidationError("不支持的指令类型".to_string()));
        }
        // chat.send 内容防空白：空消息会触发一轮空 LLM 流（命令层无此
        // 守卫，手机指令路径绕过前端 UI 校验——dispatch 层补位）。
        if envelope.action == "chat.send" {
            let content = envelope
                .params
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if content.trim().is_empty() {
                return Err(AppError::ValidationError("消息内容不能为空".to_string()));
            }
        }
        let empty = serde_json::json!({});
        let params = if envelope.params.is_null() {
            &empty
        } else {
            &envelope.params
        };
        match envelope.action.as_str() {
            "memory.list" => self.route_memory_list(params).await,
            "memory.sources" => self.route_memory_sources(params).await,
            "memory.forget" => self.route_memory_forget(params).await,
            action => {
                let result = self.executor.execute(action, params).await;
                // 写信号补充：仅 suggestion.confirm/reject（confirm 建任务但
                // 不 emit 的既有缺口）；task/chat 命令层 emit 齐全勿重复补。
                if result.is_ok() && matches!(action, "suggestion.confirm" | "suggestion.reject") {
                    let event = if action == "suggestion.confirm" {
                        "task:created"
                    } else {
                        "suggestion:rejected"
                    };
                    // 通道满/关闭：记 warn 留痕（显式失败，不静默吞），
                    // 由后续写信号/重连快照兜底收敛。
                    if let Err(e) = self.write_signal_tx.try_send(WriteSignal { event }) {
                        tracing::warn!(
                            error = %e,
                            "companion 写信号发送失败（快照由后续写信号/重连兜底）"
                        );
                    }
                }
                result
            }
        }
    }

    async fn route_memory_list(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct MemoryListParams {
            role_id: Option<String>,
            category: Option<String>,
            #[serde(default)]
            limit: Option<i64>,
            #[serde(default)]
            offset: Option<i64>,
        }
        let p: MemoryListParams = parse_params(params)?;
        // limit 缺省 50 防大结果；上限 200 防病态请求撑爆单帧；
        // offset 负数无意义，归一为 0（与 limit 防御对称）
        let limit = Some(p.limit.unwrap_or(50).clamp(1, 200));
        let offset = p.offset.map(|o| o.max(0));
        let memories = match p.role_id.as_deref() {
            Some(role_id) => {
                memory_query::list_role_memories(
                    &self.main_pool,
                    Some(role_id),
                    p.category.as_deref(),
                    limit,
                    offset,
                )
                .await?
            }
            None => {
                memory_query::list_all_memories(
                    &self.main_pool,
                    p.category.as_deref(),
                    limit,
                    offset,
                )
                .await?
            }
        };
        Ok(serde_json::json!({ "memories": memories }))
    }

    async fn route_memory_sources(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct MemorySourcesParams {
            memory_id: String,
        }
        let p: MemorySourcesParams = parse_params(params)?;
        let messages =
            memory_query::get_source_messages(&self.main_pool, &self.conv_pool, &p.memory_id)
                .await?;
        Ok(serde_json::json!({ "messages": messages }))
    }

    async fn route_memory_forget(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct MemoryForgetParams {
            memory_id: String,
        }
        let p: MemoryForgetParams = parse_params(params)?;
        memory_query::delete_memory(&self.main_pool, &p.memory_id).await?;
        Ok(serde_json::json!({}))
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(params: &serde_json::Value) -> Result<T, AppError> {
    serde_json::from_value(params.clone())
        .map_err(|e| AppError::ValidationError(format!("指令参数无效: {}", e)))
}

/// 日志用 action 判别式：截断至 64 字符——超长 action 串（上限内可造出
/// 数十 KB）不得整串入日志（评审整改：防日志膨胀）。
fn loggable_action(action: &str) -> String {
    action.chars().take(64).collect()
}

fn to_json_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(value)
        .map_err(|e| AppError::ValidationError(format!("指令结果序列化失败: {}", e)))
}

// ---------------------------------------------------------------------------
// 生产执行器：命令层 pub fn 直调（State 参数经 AppHandle::state 现场获取，
// 与 commands/chat.rs:294/351 内部同款手法——无需提取/重构任何既有命令函数）
// ---------------------------------------------------------------------------

struct AppHandleCommandExecutor {
    app_handle: tauri::AppHandle,
}

#[async_trait::async_trait]
impl CommandExecutor for AppHandleCommandExecutor {
    async fn execute(
        &self,
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        use tauri::Manager;

        let app = &self.app_handle;
        // 单一事实源（评审整改）：字符串先解析为枚举，match 对枚举穷尽——
        // 编译器强制覆盖；新增 action 漏臂 = 编译失败而非运行期矛盾。
        let action = match ProdAction::parse(action) {
            Some(a) => a,
            None => return Err(AppError::ValidationError("不支持的指令类型".to_string())),
        };
        match action {
            ProdAction::ChatSend => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct ChatSendParams {
                    conversation_id: Option<String>,
                    role_id: Option<String>,
                    content: String,
                }
                let p: ChatSendParams = parse_params(params)?;
                let request = ChatRequest {
                    conversation_id: p.conversation_id,
                    role_id: p.role_id,
                    content: p.content,
                    working_directory: None,
                    onboarding_step: 0,
                    selected_skill_id: None,
                };
                let msg = crate::commands::chat::chat_send_message(
                    request,
                    app.state::<DbPool>(),
                    app.state::<ConversationsPool>(),
                    app.state::<crate::commands::chat::StreamingState>(),
                    app.state::<crate::commands::chat::OnboardingConversations>(),
                    app.state::<crate::services::agent_config::AgentConfigService>(),
                    app.clone(),
                )
                .await?;
                // busy 守卫：返回的是 assistant 提示消息（本轮用户消息未落库）
                // ——如实回执失败，提示气泡已落库、经快照收敛可见。
                if msg.role != "user" {
                    return Err(AppError::ValidationError(
                        "我还在想上一个问题，请稍等片刻".to_string(),
                    ));
                }
                let assistant_message_id = latest_assistant_message_id(
                    &app.state::<ConversationsPool>(),
                    &msg.conversation_id,
                )
                .await?;
                Ok(serde_json::json!({
                    "conversationId": msg.conversation_id,
                    "userMessageId": msg.id,
                    "assistantMessageId": assistant_message_id,
                }))
            }
            ProdAction::ChatStop => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct ChatStopParams {
                    conversation_id: String,
                }
                let p: ChatStopParams = parse_params(params)?;
                crate::commands::chat::chat_stop_streaming(p.conversation_id, app.clone()).await?;
                Ok(serde_json::json!({}))
            }
            ProdAction::ConversationNew => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct ConversationNewParams {
                    role_id: Option<String>,
                }
                let p: ConversationNewParams = parse_params(params)?;
                let conv = crate::commands::chat::chat_new_conversation(
                    None,
                    p.role_id,
                    app.state::<ConversationsPool>(),
                    app.state::<DbPool>(),
                    app.clone(),
                )
                .await?;
                to_json_value(&conv)
            }
            ProdAction::ConversationDelete => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct ConversationDeleteParams {
                    conversation_id: String,
                }
                let p: ConversationDeleteParams = parse_params(params)?;
                crate::commands::chat::chat_delete_conversation(
                    p.conversation_id,
                    app.state::<ConversationsPool>(),
                    app.clone(),
                )
                .await?;
                Ok(serde_json::json!({}))
            }
            ProdAction::TaskToggle => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct TaskToggleParams {
                    task_id: String,
                    is_completed: bool,
                }
                let p: TaskToggleParams = parse_params(params)?;
                let task = crate::commands::task::task_toggle_complete(
                    p.task_id,
                    p.is_completed,
                    app.clone(),
                    app.state::<DbPool>(),
                )
                .await?;
                to_json_value(&task)
            }
            ProdAction::TaskCreate => {
                // params 镜像 CreateTaskInput 字段（camelCase）
                let input: CreateTaskInput = parse_params(params)?;
                let task = crate::commands::task::task_create(
                    input,
                    app.clone(),
                    app.state::<DbPool>(),
                )
                .await?;
                to_json_value(&task)
            }
            ProdAction::SuggestionList => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct SuggestionListParams {
                    conversation_id: String,
                }
                let p: SuggestionListParams = parse_params(params)?;
                let suggestions = crate::commands::suggestion::suggestion_list_pending(
                    p.conversation_id,
                    app.state::<DbPool>(),
                )
                .await?;
                Ok(serde_json::json!({ "suggestions": suggestions }))
            }
            ProdAction::SuggestionConfirm => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct SuggestionConfirmParams {
                    suggestion_id: String,
                }
                let p: SuggestionConfirmParams = parse_params(params)?;
                let pool = app.state::<DbPool>();
                let confirmed =
                    crate::commands::suggestion::confirm_and_create_task(pool.inner(), &p.suggestion_id)
                        .await?;
                to_json_value(&confirmed)
            }
            ProdAction::SuggestionReject => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct SuggestionRejectParams {
                    suggestion_id: String,
                    reason: String,
                }
                let p: SuggestionRejectParams = parse_params(params)?;
                let pool = app.state::<DbPool>();
                let rejected = crate::commands::suggestion::reject_with_reason(
                    pool.inner(),
                    &p.suggestion_id,
                    &p.reason,
                )
                .await?;
                to_json_value(&rejected)
            }
        }
    }
}

/// chat.send 后查询会话内最新 assistant 消息 id（命令层预插的占位气泡）。
async fn latest_assistant_message_id(
    conv_pool: &ConversationsPool,
    conversation_id: &str,
) -> Result<String, AppError> {
    let messages = crate::db::conversations::list_messages(conv_pool, conversation_id).await?;
    messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| m.id.clone())
        .ok_or_else(|| AppError::DbError("未找到 assistant 占位消息".to_string()))
}

// ---------------------------------------------------------------------------
// STREAM_TOKEN 镜像（T3）：llm:stream payload → Frame::StreamToken
// ---------------------------------------------------------------------------

/// llm:stream payload → `Frame::StreamToken(data=原样 JSON)`。镜像全部
/// llm:stream（含桌面端发起的流——手机侧按 conversationId 过滤渲染）。
/// 非 JSON / 超限 payload 返回 None（fail-safe：宁跳过不断流）。
pub fn mirror_stream_payload(payload_json: &str) -> Option<Frame> {
    if payload_json.len() > COMMAND_DATA_MAX_BYTES {
        // 仅记判别式与长度，不落 payload 内容（NFR-M7）
        tracing::warn!(
            bytes = payload_json.len(),
            "llm:stream payload 超单帧上限，跳过 STREAM_TOKEN 镜像"
        );
        return None;
    }
    if serde_json::from_str::<serde_json::Value>(payload_json).is_err() {
        tracing::warn!("llm:stream payload 非法 JSON，跳过 STREAM_TOKEN 镜像");
        return None;
    }
    Some(Frame::StreamToken(StreamTokenPayload {
        data: payload_json.to_string(),
    }))
}

/// 注册 llm:stream 镜像监听（lib.rs setup 调用）：listener 同步闭包内
/// 不得 await——逐帧 spawn 无调度顺序保证（评审裁决 A 整改：token 乱序/
/// done 帧超前），改为 listener 同步 try_send 进中转通道 + 单消费者任务
/// 串行 `try_enqueue_single` 入站（满则丢弃 + warn 不断连；token 丢一帧
/// 由快照在 done 后兜底收敛）。
pub fn register_stream_mirror(app_handle: tauri::AppHandle, state: Arc<CompanionState>) {
    use tauri::Listener;

    let (mirror_tx, mut mirror_rx) = mpsc::channel::<Frame>(MIRROR_CHANNEL_CAPACITY);
    // 单消费者保序：消费顺序 = 事件到达顺序（mpsc FIFO），入站顺序恒定。
    tokio::spawn(async move {
        while let Some(frame) = mirror_rx.recv().await {
            state.try_enqueue_single(frame).await;
        }
    });
    app_handle.listen("llm:stream".to_string(), move |event| {
        if let Some(frame) = mirror_stream_payload(event.payload()) {
            if mirror_tx.try_send(frame).is_err() {
                // 仅记判别式，不含内容（NFR-M7）
                tracing::warn!("STREAM_TOKEN 镜像通道满，丢弃该帧（快照兜底收敛）");
            }
        }
    });
}

/// 将 ack JSON 包装为 COMMAND_RESULT 帧（`enter_session` Command 臂与测试用）。
pub fn command_result_frame(ack_json: String) -> Frame {
    Frame::CommandResult(CommandResultPayload { data: ack_json })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WHY: KNOWN_ACTIONS（route 前置白名单）与生产执行器受理集是两处
    /// 清单——任一侧新增 action 而漏另一侧，会产生「白名单放行但执行器
    /// 拒收」的运行期矛盾。本测试以执行器枚举为事实源强制两清单对齐。
    #[test]
    fn known_actions_align_with_production_executor() {
        let memory_routed = ["memory.list", "memory.sources", "memory.forget"];
        for action in KNOWN_ACTIONS {
            assert!(
                ProdAction::parse(action).is_some() || memory_routed.contains(action),
                "KNOWN_ACTIONS 含执行器与内部路由均不受理的 action: {action}"
            );
        }
        // 执行器全部臂必须被 KNOWN_ACTIONS 放行（否则被 route 前置拦截成死代码）
        let prod_actions = [
            "chat.send",
            "chat.stop",
            "conversation.new",
            "conversation.delete",
            "task.toggle",
            "task.create",
            "suggestion.list",
            "suggestion.confirm",
            "suggestion.reject",
        ];
        for action in prod_actions {
            assert!(
                KNOWN_ACTIONS.contains(&action),
                "执行器受理的 action 未列入 KNOWN_ACTIONS: {action}"
            );
            assert!(ProdAction::parse(action).is_some());
        }
        assert!(!KNOWN_ACTIONS.contains(&"task.delete"), "指令集冻结，勿投机扩容");
    }

    #[test]
    fn loggable_action_truncates_long_strings() {
        // WHY: envelope 长度上限内可造出数十 KB 的 action 串——不截断会
        // 整串写入日志（日志膨胀面）。
        let long = "a".repeat(5000);
        assert_eq!(loggable_action(&long).chars().count(), 64);
        assert_eq!(loggable_action("chat.send"), "chat.send");
    }
}
