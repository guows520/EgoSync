//! Story 13.1：桌面快照引擎——只读聚合 + 10MB 截断 + 分帧 + debounce 推送。
//!
//! 硬边界（违反即返工）：
//! - 本模块只读聚合既有 db/service API，经 `CompanionState::enqueue_outbound`
//!   出站通道发帧；**不 import `db::memories`**（记忆内容不上机，memoryCount
//!   仅作为仪表盘指标数字出现——AC1 负向约束）。
//! - 帧编码只调 `companion_proto::frames::{encode_frame, decode_frame}`；
//!   分帧 envelope 是 app 层 JSON，不进 companion-proto crate、不 bump
//!   PROTOCOL_VERSION（裁决 1）。
//! - 日志纪律（NFR-M7）：只记事件类别/域计数/字节数，帧明文与快照 JSON
//!   永不入日志。

use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use companion_proto::frames::{Frame, SnapshotPayload, StateDeltaPayload};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};

use crate::db::pool::{ConversationsPool, DbPool};
use crate::error::AppError;
use crate::models::dashboard::{DashboardMetricsQuery, DashboardMetricsScope};
use crate::models::snapshot::{
    DesktopSnapshot, SnapshotBriefing, SnapshotConversation, SnapshotDashboard, SnapshotMessage,
    SnapshotMetrics, SnapshotNotification, SnapshotRole, SnapshotTask, SnapshotWeeklyReview,
    SNAPSHOT_MAX_BYTES, SNAPSHOT_SCHEMA_VERSION,
};
use crate::services::companion_connection::CompanionState;

/// debounce 窗口（生产值）：窗口内合并后续写信号，窗口静默后重建一次。
/// 测试经 [`CompanionSnapshotEngine::with_debounce`] 注入短窗。
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(2000);

/// 快照明文分片尺寸：envelope JSON + AEAD tag 后仍 < 65519 单帧上限
/// （裁决 5）。注意：story 原写 60_000，但 60_000×4/3(base64)=80_000 明文
/// 超 65519 上限、被 crate 既有 `oversized_frame_encode_is_rejected` 拒绝
/// （规则七：冲突显式择一——以可工作的 48_000 为准，60_000 列待清理）。
/// 48_000→base64=64_000，+envelope 开销 < 65519 上限。base64 规避 UTF-8
/// 多字节字符被字节级切片截断的问题。
const CHUNK_PLAINTEXT_BYTES: usize = 48_000;

/// 每会话携带的消息条数上限（`get_recent_messages` 现成入口）。
const MESSAGES_PER_CONVERSATION: i64 = 200;

// ---------------------------------------------------------------------------
// 分帧 envelope（app 层协议，13.2 Kotlin FrameCodec 镜像此结构）
// ---------------------------------------------------------------------------

/// 单分帧 envelope：`data` 字段承载的 JSON 形如
/// `{"seq":0,"total":3,"chunkBase64":"..."}`（camelCase）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkEnvelope {
    pub seq: u32,
    pub total: u32,
    pub chunk_base64: String,
}

/// 分帧承载的帧类型：建连=SNAPSHOT（全量替换），变化推送=STATE_DELTA
/// （载荷同为全量快照，仅触发时机不同——裁决 2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFrameKind {
    Snapshot,
    StateDelta,
}

/// 快照 → 分帧序列：JSON 序列化 → 按明文字节切片 → base64 → envelope JSON
/// 塞进 `SnapshotPayload/StateDeltaPayload.data`（对协议层 opaque）。
pub fn frame_snapshot(
    snapshot: &DesktopSnapshot,
    kind: SnapshotFrameKind,
) -> Result<Vec<Frame>, AppError> {
    let json = serde_json::to_vec(snapshot)
        .map_err(|e| AppError::ValidationError(format!("快照序列化失败: {}", e)))?;
    let total = json.len().div_ceil(CHUNK_PLAINTEXT_BYTES).max(1) as u32;
    let mut frames = Vec::with_capacity(total as usize);
    for (seq, chunk) in json.chunks(CHUNK_PLAINTEXT_BYTES).enumerate() {
        let envelope = ChunkEnvelope {
            seq: seq as u32,
            total,
            chunk_base64: BASE64.encode(chunk),
        };
        let data = serde_json::to_string(&envelope)
            .map_err(|e| AppError::ValidationError(format!("分帧 envelope 序列化失败: {}", e)))?;
        frames.push(match kind {
            SnapshotFrameKind::Snapshot => Frame::Snapshot(SnapshotPayload { data }),
            SnapshotFrameKind::StateDelta => Frame::StateDelta(StateDeltaPayload { data }),
        });
    }
    Ok(frames)
}

/// 分帧重组（接收方断言用；13.2 Kotlin 侧镜像此逻辑）：按 seq 收齐 total 片、
/// base64 解码、拼接、JSON 反序列化。
pub fn reassemble(frames: &[Frame]) -> Result<DesktopSnapshot, AppError> {
    let mut envelopes: Vec<ChunkEnvelope> = Vec::with_capacity(frames.len());
    for frame in frames {
        let data = match frame {
            Frame::Snapshot(p) => &p.data,
            Frame::StateDelta(p) => &p.data,
            _ => {
                return Err(AppError::ProtocolError(
                    "分帧重组仅接受 Snapshot/StateDelta 帧".to_string(),
                ))
            }
        };
        let envelope: ChunkEnvelope = serde_json::from_str(data)
            .map_err(|e| AppError::ProtocolError(format!("分帧 envelope 无效: {}", e)))?;
        envelopes.push(envelope);
    }
    let total = envelopes
        .first()
        .map(|e| e.total as usize)
        .ok_or_else(|| AppError::ProtocolError("分帧序列为空".to_string()))?;
    if envelopes.len() != total {
        return Err(AppError::ProtocolError(format!(
            "分帧不完整：期望 {total} 片，实际 {} 片",
            envelopes.len()
        )));
    }
    envelopes.sort_by_key(|e| e.seq);
    let mut json = Vec::new();
    for (expected, envelope) in envelopes.iter().enumerate() {
        if envelope.seq != expected as u32 {
            return Err(AppError::ProtocolError(format!(
                "分帧序号不连续：期望 {expected}，实际 {}",
                envelope.seq
            )));
        }
        let bytes = BASE64
            .decode(envelope.chunk_base64.as_bytes())
            .map_err(|e| AppError::ProtocolError(format!("分帧 base64 解码失败: {}", e)))?;
        json.extend_from_slice(&bytes);
    }
    serde_json::from_slice(&json)
        .map_err(|e| AppError::ProtocolError(format!("快照反序列化失败: {}", e)))
}

// ---------------------------------------------------------------------------
// 快照聚合与 10MB 截断
// ---------------------------------------------------------------------------

/// 聚合中间态：会话 `updated_at DESC`、简报/复盘 `ASC`（截断阶段依赖此序）。
struct SnapshotParts {
    generated_at: String,
    roles: Vec<SnapshotRole>,
    tasks: Vec<SnapshotTask>,
    dashboard: SnapshotDashboard,
    conversations: Vec<SnapshotConversation>,
    briefings: Vec<SnapshotBriefing>,
    weekly_reviews: Vec<SnapshotWeeklyReview>,
    notifications: Vec<SnapshotNotification>,
    truncated: bool,
    data_cutoff_at: Option<String>,
    truncated_domains: Vec<String>,
}

impl SnapshotParts {
    fn assemble(&self) -> DesktopSnapshot {
        DesktopSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            generated_at: self.generated_at.clone(),
            data_cutoff_at: self.data_cutoff_at.clone(),
            truncated: self.truncated,
            truncated_domains: self.truncated_domains.clone(),
            roles: self.roles.clone(),
            tasks: self.tasks.clone(),
            dashboard: self.dashboard.clone(),
            conversations: self.conversations.clone(),
            briefings: self.briefings.clone(),
            weekly_reviews: self.weekly_reviews.clone(),
            notifications: self.notifications.clone(),
        }
    }
}

fn serialized_len(snapshot: &DesktopSnapshot) -> usize {
    serde_json::to_vec(snapshot).map(|v| v.len()).unwrap_or(usize::MAX)
}

/// 生成全量快照（内存持有、不持久化）。
///
/// 负向保证：不读记忆库（memoryCount 仅来自仪表盘指标）；角色卡投影不含
/// `personality_prompt`。全部数据源失败显式传播（`?`），不吞错。
pub async fn build_snapshot(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
) -> Result<DesktopSnapshot, AppError> {
    let mut parts = fetch_parts(pool, conv_pool).await?;
    apply_size_cap(&mut parts);
    Ok(parts.assemble())
}

async fn fetch_parts(
    pool: &DbPool,
    conv_pool: &ConversationsPool,
) -> Result<SnapshotParts, AppError> {
    let roles = crate::db::roles::list_active_roles(pool)
        .await?
        .into_iter()
        .map(|r| SnapshotRole {
            id: r.id,
            name: r.name,
            icon: r.icon,
            color: r.color,
            goal: r.goal,
            status: r.status,
            energy: r.energy,
            proactivity_level: r.proactivity_level,
        })
        .collect();

    let tasks = crate::db::tasks::list_all_tasks(pool, None, None)
        .await?
        .into_iter()
        .map(|t| SnapshotTask {
            id: t.id,
            owner_type: t.owner_type,
            role_id: t.role_id,
            title: t.title,
            deadline: t.deadline,
            quadrant: t.quadrant,
            is_big_rock: t.is_big_rock,
            is_completed: t.is_completed,
            protection_status: t.protection_status,
            role_name: t.role_name,
            role_color: t.role_color,
        })
        .collect();

    let statuses =
        crate::services::dashboard_service::get_dashboard_status(pool, conv_pool).await?;
    // 全时间窗（scope All + 两窗口 None）——最简口径；移动端筛选交互的数据
    // 需求走 schemaVersion 演进承载（见 story 开放问题）。
    let metrics_raw = crate::services::dashboard_service::get_dashboard_metrics(
        pool,
        conv_pool,
        DashboardMetricsQuery {
            scope: DashboardMetricsScope::All,
            start_at: None,
            end_at: None,
        },
    )
    .await?;
    let metrics = SnapshotMetrics {
        task_count: metrics_raw.task_count,
        memory_count: metrics_raw.memory_count,
        conversation_count: metrics_raw.conversation_count,
        pending_task_count: metrics_raw.pending_task_count,
        generated_at: metrics_raw.generated_at,
    };

    let conversations = crate::db::conversations::list_all_conversations(conv_pool).await?;
    let mut snapshot_conversations = Vec::with_capacity(conversations.len());
    for conv in conversations {
        let messages =
            crate::db::conversations::get_recent_messages(conv_pool, &conv.id, MESSAGES_PER_CONVERSATION)
                .await?;
        snapshot_conversations.push(SnapshotConversation {
            id: conv.id,
            role_id: conv.role_id,
            title: conv.title,
            updated_at: conv.updated_at,
            messages: messages
                .into_iter()
                .map(|m| SnapshotMessage {
                    id: m.id,
                    role: m.role,
                    content: m.content,
                    thinking_content: m.thinking_content,
                    is_complete: m.is_complete,
                    created_at: m.created_at,
                })
                .collect(),
        });
    }

    // 「本季度」过滤：全量取回后 Rust 侧按季度区间字符串比较
    // （YYYY-MM-DD 字典序即时间序，参照 review_generator 本周过滤模式）。
    let (quarter_start, quarter_end) = current_quarter_range();
    let briefings = crate::db::briefings::list_all_briefings(pool)
        .await?
        .into_iter()
        .filter(|b| b.date >= quarter_start && b.date <= quarter_end)
        .map(|b| SnapshotBriefing {
            id: b.id,
            content: b.content,
            date: b.date,
        })
        .collect();
    let weekly_reviews = crate::db::weekly_reviews::list_all_weekly_reviews(pool)
        .await?
        .into_iter()
        .filter(|r| r.week_start >= quarter_start && r.week_start <= quarter_end)
        .map(|r| SnapshotWeeklyReview {
            id: r.id,
            week_start: r.week_start,
            week_end: r.week_end,
            summary: r.summary,
            energy_trends: r.energy_trends,
            bigrock_status: r.bigrock_status,
            new_memories_count: r.new_memories_count,
        })
        .collect();

    // 未读通知：is_read == false。
    let notifications = crate::db::notifications::list_notifications(pool)
        .await?
        .into_iter()
        .filter(|n| !n.is_read)
        .map(|n| SnapshotNotification {
            id: n.id,
            role_id: n.role_id,
            level: n.level,
            content: n.content,
            created_at: n.created_at,
            role_name: n.role_name,
            role_icon: n.role_icon,
            role_color: n.role_color,
        })
        .collect();

    Ok(SnapshotParts {
        generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        roles,
        tasks,
        dashboard: SnapshotDashboard { statuses, metrics },
        conversations: snapshot_conversations,
        briefings,
        weekly_reviews,
        notifications,
        truncated: false,
        data_cutoff_at: None,
        truncated_domains: Vec::new(),
    })
}

/// 当前季度 `[start, end]`（本地日期，YYYY-MM-DD，闭区间）。
fn current_quarter_range() -> (String, String) {
    quarter_range_at(chrono::Local::now().date_naive())
}

/// 指定日期所在季度 `[start, end]`（YYYY-MM-DD，闭区间）。纯函数注入日期
/// （评审 P9）：月份映射错位会让边界简报/复盘被静默漏算，必须可测。
fn quarter_range_at(date: chrono::NaiveDate) -> (String, String) {
    use chrono::Datelike;

    let year = date.year();
    let month = date.month();
    let start_month = (month - 1) / 3 * 3 + 1;
    let (end_month, end_day) = match start_month {
        1 => (3, 31),
        4 => (6, 30),
        7 => (9, 30),
        _ => (12, 31),
    };
    (
        format!("{:04}-{:02}-01", year, start_month),
        format!("{:04}-{:02}-{:02}", year, end_month, end_day),
    )
}

fn min_assign(current: &mut Option<String>, candidate: &str) {
    match current {
        Some(c) if c.as_str() <= candidate => {}
        _ => *current = Some(candidate.to_string()),
    }
}

fn max_assign(current: &mut Option<String>, candidate: &str) {
    match current {
        Some(c) if c.as_str() >= candidate => {}
        _ => *current = Some(candidate.to_string()),
    }
}

/// 单条 JSON 序列化字节数（截断增量核算用）。
fn item_json_len<T: serde::Serialize>(v: &T) -> usize {
    serde_json::to_vec(v).map(|b| b.len()).unwrap_or(usize::MAX)
}

/// JSON 数组字节数近似：Σ(条目 + 1 逗号) + 2 方括号，较精确值略高
/// （保守方向——估高只会提前停止截断，残差由末尾精确测量兜底）。
fn array_json_len(sizes: &[usize]) -> usize {
    sizes.iter().map(|s| s + 1).sum::<usize>() + 2
}

/// 10MB 上限截断（裁决 6 顺序）：会话域先截（最旧会话整段丢弃，但始终
/// 保留最新 1 个会话——单会话超限交给阶段 B 减半，不得把唯一/最新会话
/// 整段丢掉；仍超限则各会话消息数减半迭代，保留最新一半）→ 本季度简报
/// （最旧起丢）→ 周复盘（最旧起丢）。角色/任务/仪表盘指标/未读通知
/// 永不截断（AC4 核心域不缺失）。
///
/// 尺寸核算：初始一次精确测量 + 逐条增量估算（评审 P5——原实现每轮全量
/// clone+序列化，病态数据下 O(n²) 可致引擎停摆数十秒，阻塞 OnConnect
/// 与写信号处理）。末尾仍做一次精确测量兜底。
fn apply_size_cap(parts: &mut SnapshotParts) {
    if serialized_len(&parts.assemble()) <= SNAPSHOT_MAX_BYTES {
        return;
    }

    let mut conv_sizes: Vec<usize> = parts.conversations.iter().map(item_json_len).collect();
    let mut briefing_sizes: Vec<usize> = parts.briefings.iter().map(item_json_len).collect();
    let mut review_sizes: Vec<usize> = parts.weekly_reviews.iter().map(item_json_len).collect();
    // 固定开销 = 初始精确总量 − 三可截域初始估算（其余域 + JSON 结构开销）
    let fixed_overhead = serialized_len(&parts.assemble()).saturating_sub(
        array_json_len(&conv_sizes)
            + array_json_len(&briefing_sizes)
            + array_json_len(&review_sizes),
    );
    let est = |c: &[usize], b: &[usize], r: &[usize]| {
        fixed_overhead + array_json_len(c) + array_json_len(b) + array_json_len(r)
    };

    let mut truncated_conversations = false;
    let mut truncated_briefings = false;
    let mut truncated_reviews = false;

    // 阶段 A：最旧会话整段丢弃（updated_at DESC → 尾部最旧），保留最新
    // 1 个会话（评审 P4：否则单会话超限时会话域被整段清空、阶段 B 永不
    // 可达——减半最新会话的消息优于丢弃最新会话本身）。
    while est(&conv_sizes, &briefing_sizes, &review_sizes) > SNAPSHOT_MAX_BYTES
        && parts.conversations.len() > 1
    {
        parts.conversations.pop();
        conv_sizes.pop();
        truncated_conversations = true;
    }

    // 阶段 B：各会话消息数减半迭代（保留最新一半——消息时间正序，尾部最新）。
    let mut limit = parts
        .conversations
        .iter()
        .map(|c| c.messages.len())
        .max()
        .unwrap_or(0);
    while est(&conv_sizes, &briefing_sizes, &review_sizes) > SNAPSHOT_MAX_BYTES && limit > 1 {
        limit /= 2;
        for (i, conv) in parts.conversations.iter_mut().enumerate() {
            let len = conv.messages.len();
            if len > limit {
                conv.messages.drain(..len - limit);
                conv_sizes[i] = item_json_len(conv);
                truncated_conversations = true;
            }
        }
    }

    // 阶段 C：本季度简报最旧起丢。整域删空时记录最后删除（=最新）一条
    // 日期——cutoff 语义见下方元数据计算。
    let mut newest_dropped_briefing: Option<String> = None;
    while est(&conv_sizes, &briefing_sizes, &review_sizes) > SNAPSHOT_MAX_BYTES
        && !parts.briefings.is_empty()
    {
        newest_dropped_briefing = parts.briefings.first().map(|b| b.date.clone());
        parts.briefings.remove(0);
        briefing_sizes.remove(0);
        truncated_briefings = true;
    }

    // 阶段 D：周复盘最旧起丢。
    let mut newest_dropped_review: Option<String> = None;
    while est(&conv_sizes, &briefing_sizes, &review_sizes) > SNAPSHOT_MAX_BYTES
        && !parts.weekly_reviews.is_empty()
    {
        newest_dropped_review = parts.weekly_reviews.first().map(|r| r.week_start.clone());
        parts.weekly_reviews.remove(0);
        review_sizes.remove(0);
        truncated_reviews = true;
    }

    // 兜底（评审 P4，规则十二显式失败）：可截域耗尽仍超限——核心域自身
    // >10MB 的病态数据。快照照发（手机拿到完整核心域优于失明），但
    // truncated 如实标记 + error 级日志，不静默破坏 10MB 契约。
    let oversize = serialized_len(&parts.assemble()) > SNAPSHOT_MAX_BYTES;
    if oversize {
        tracing::error!(
            bytes = serialized_len(&parts.assemble()),
            "快照在可截域耗尽后仍超 10MB 上限（核心域过大），标记截断并照发"
        );
    }

    if !(truncated_conversations || truncated_briefings || truncated_reviews) && !oversize {
        return;
    }

    // 截断元数据：dataCutoffAt =「早于该时间戳的数据在至少一个被截域中
    // 缺失」。域内取保留数据的最旧时间戳；跨域取最大值（评审 P4：取最小
    // 会漏报——会话保留自 1 月、简报保留自 3 月时，完整区间自 3 月起，
    // 报 1 月会让手机误信 1-3 月的简报完整）。域被整段删空时无保留数据，
    // 取该域最后删除（=最新）条目时间戳——其后该域无数据，完整性空真。
    let mut cutoff: Option<String> = None;
    if truncated_conversations {
        let mut bound: Option<String> = None;
        for conv in &parts.conversations {
            let oldest = conv
                .messages
                .first()
                .map(|m| m.created_at.as_str())
                .unwrap_or(conv.updated_at.as_str());
            min_assign(&mut bound, oldest);
        }
        if let Some(b) = bound {
            max_assign(&mut cutoff, &b);
        }
    }
    if truncated_briefings {
        let bound = parts
            .briefings
            .first()
            .map(|b| b.date.clone())
            .or(newest_dropped_briefing);
        if let Some(b) = bound {
            max_assign(&mut cutoff, &b);
        }
    }
    if truncated_reviews {
        let bound = parts
            .weekly_reviews
            .first()
            .map(|r| r.week_start.clone())
            .or(newest_dropped_review);
        if let Some(b) = bound {
            max_assign(&mut cutoff, &b);
        }
    }

    parts.truncated = true;
    // cutoff 为 None 仅可能出现在纯超限兜底路径（无任何条目被删）——数据
    // 实际完整，generated_at 即「完整截至此刻」的如实表述
    parts.data_cutoff_at = Some(cutoff.unwrap_or_else(|| parts.generated_at.clone()));
    parts.truncated_domains = [
        truncated_conversations.then(|| "conversations".to_string()),
        truncated_briefings.then(|| "briefings".to_string()),
        truncated_reviews.then(|| "weeklyReviews".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect();

    tracing::info!(
        truncated_domains = ?parts.truncated_domains,
        data_cutoff_at = ?parts.data_cutoff_at,
        oversize,
        "快照超出 10MB 上限，已按裁决顺序截断最旧可截断域"
    );
}

// ---------------------------------------------------------------------------
// debounce 引擎
// ---------------------------------------------------------------------------

/// 写信号：仅携带事件类别（日志纪律：不携带 payload，业务数据不入日志）。
#[derive(Debug, Clone)]
pub struct WriteSignal {
    pub event: &'static str,
}

/// 建连请求：`enter_session` 建连后经直连通道请求全量快照
/// （不走 Tauri 事件——`app_handle: None` 的测试路径可用）。
/// 通道为 watch（评审 E6/B13）：幂等合并——引擎忙于重建时多次建连
/// 只补发一次全量快照，无容量上限即无「第 N 次请求被丢」的白屏窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRequest {
    OnConnect,
}

/// 快照引擎：订阅写信号 → debounce 合并 → 重建 → 出站分帧推送。
///
/// 快照在内存持有、不持久化；每次重建现生成（桌面重启后首次连接现生成）。
pub struct CompanionSnapshotEngine {
    pool: DbPool,
    conv_pool: ConversationsPool,
    state: Arc<CompanionState>,
    debounce: Duration,
    notify_tx: mpsc::Sender<WriteSignal>,
    notify_rx: tokio::sync::Mutex<mpsc::Receiver<WriteSignal>>,
    snapshot_request_tx: watch::Sender<SnapshotRequest>,
    snapshot_request_rx: tokio::sync::Mutex<watch::Receiver<SnapshotRequest>>,
    rebuild_count: std::sync::atomic::AtomicU64,
}

/// 引擎事件（`run()` 内部分派用）。通道关闭（senders 随引擎/监听器消亡）
/// 不设枚举变体——直接 break 退出主循环（评审 B6：原 Shutdown 变体不可达）。
enum EngineEvent {
    OnConnect,
    Write(WriteSignal),
}

impl CompanionSnapshotEngine {
    /// 生产构造（2000ms debounce 窗口）。
    pub fn new(
        pool: DbPool,
        conv_pool: ConversationsPool,
        state: Arc<CompanionState>,
    ) -> Self {
        Self::with_debounce(pool, conv_pool, state, DEBOUNCE_WINDOW)
    }

    /// 测试构造：注入短 debounce 窗口（可测性必需——真实 2s 窗口会拖慢全量套件）。
    pub fn with_debounce(
        pool: DbPool,
        conv_pool: ConversationsPool,
        state: Arc<CompanionState>,
        debounce: Duration,
    ) -> Self {
        let (notify_tx, notify_rx) = mpsc::channel(256);
        // watch 初始值仅为占位：接收端 `changed()` 只对建连后的 send 触发，
        // 引擎启动不会误发一次快照
        let (snapshot_request_tx, snapshot_request_rx) =
            watch::channel(SnapshotRequest::OnConnect);
        Self {
            pool,
            conv_pool,
            state,
            debounce,
            notify_tx,
            notify_rx: tokio::sync::Mutex::new(notify_rx),
            snapshot_request_tx,
            snapshot_request_rx: tokio::sync::Mutex::new(snapshot_request_rx),
            rebuild_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 写信号入口（Tauri 事件监听器克隆持有，try_send 非阻塞）。
    pub fn notify_signal(&self) -> mpsc::Sender<WriteSignal> {
        self.notify_tx.clone()
    }

    /// 建连请求通道（setup 时注入 `CompanionState`）。
    pub fn snapshot_request_tx(&self) -> watch::Sender<SnapshotRequest> {
        self.snapshot_request_tx.clone()
    }

    /// 重建次数（测试观测点）。
    pub fn rebuild_count(&self) -> u64 {
        self.rebuild_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 长跑主循环：消费建连请求（全量 SNAPSHOT）与写信号（debounce 后
    /// STATE_DELTA）。错误只 warn 不退出——引擎崩溃等于手机永久失明。
    pub async fn run(&self) {
        loop {
            // guard 在块结束时释放——handler 体内会再锁 notify_rx（debounce），
            // 跨 handler 持锁会自死锁
            let event = {
                let mut request_rx = self.snapshot_request_rx.lock().await;
                let mut notify_rx = self.notify_rx.lock().await;
                // Option：None = 通道关闭（senders 随引擎消亡），主循环退出
                tokio::select! {
                    changed = request_rx.changed() => {
                        changed.ok().map(|()| EngineEvent::OnConnect)
                    }
                    maybe_signal = notify_rx.recv() => match maybe_signal {
                        Some(signal) => Some(EngineEvent::Write(signal)),
                        None => None,
                    },
                }
            };
            match event {
                Some(EngineEvent::OnConnect) => {
                    if let Err(e) = self.handle_connected().await {
                        tracing::warn!(error = %e, "建连全量快照生成失败");
                    }
                }
                Some(EngineEvent::Write(signal)) => {
                    tracing::debug!(event = signal.event, "companion 写信号触发快照 debounce");
                    self.wait_debounce_window().await;
                    if let Err(e) = self.rebuild_and_push_delta().await {
                        tracing::warn!(error = %e, "debounce 快照重建失败");
                    }
                }
                None => break,
            }
        }
        tracing::info!("companion 快照引擎退出");
    }

    /// debounce 窗口：固定窗口自首信号起算——窗口内到达的后续信号被消费并
    /// 丢弃（已并入本次重建），窗口结束后重建一次。延迟有界（连续写不会
    /// 无限推迟重建），N 次快速写只触发 1 次重建。
    async fn wait_debounce_window(&self) {
        let deadline = tokio::time::Instant::now() + self.debounce;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let channel_closed = {
                let mut notify_rx = self.notify_rx.lock().await;
                tokio::select! {
                    _ = tokio::time::sleep(remaining) => false,
                    more = notify_rx.recv() => more.is_none(),
                }
            };
            if channel_closed {
                break;
            }
        }
    }

    /// 建连即全量快照：重建 + 全量 SNAPSHOT 分帧入队（响应 OnConnect）。
    pub async fn handle_connected(&self) -> Result<(), AppError> {
        let snapshot = build_snapshot(&self.pool, &self.conv_pool).await?;
        let frames = frame_snapshot(&snapshot, SnapshotFrameKind::Snapshot)?;
        self.push_frames(frames).await;
        Ok(())
    }

    /// 变化推送：重建 + STATE_DELTA 分帧入队（载荷同为全量快照——裁决 2）。
    async fn rebuild_and_push_delta(&self) -> Result<(), AppError> {
        let snapshot = build_snapshot(&self.pool, &self.conv_pool).await?;
        let frames = frame_snapshot(&snapshot, SnapshotFrameKind::StateDelta)?;
        self.push_frames(frames).await;
        Ok(())
    }

    /// 分帧整序列入队：单次 `enqueue_outbound` 持锁入队（评审 P3——逐帧
    /// 分次入队在帧间可被新连接换代，残缺序列跨会话泄漏）；通道满时由
    /// `enqueue_outbound` 中止序列并强制断连（评审 P1）。
    async fn push_frames(&self, frames: Vec<Frame>) {
        self.rebuild_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let frame_count = frames.len();
        self.state.enqueue_outbound(frames).await;
        tracing::debug!(frames = frame_count, "companion 快照分帧已入队");
    }
}

// ---------------------------------------------------------------------------
// Tauri 事件订阅接线（lib.rs setup 调用）
// ---------------------------------------------------------------------------

/// 触发快照重建的写事件清单（快照引擎只看事件名不看 payload）。
/// pub：接线契约测试扫描命令层 emit 字面量断言本清单覆盖（评审 B16/P7）。
pub const WRITE_SIGNAL_EVENTS: &[&str] = &[
    // 既有事件
    "notification:new",
    "task:classified",
    "briefing:generated",
    "review:generated",
    "bigrock:protection",
    "bigrock:reminder",
    "q2:reminder",
    "role:proposed",
    "role:delegated",
    // Story 13.1 命令层补发事件（commands/{role,task,chat,notification,
    // task_decomposition}.rs）。task_decomposition emit 的 task:tool-action
    // 建任务写操作须触发（评审 P6——原清单漏订阅）。
    "task:tool-action",
    "role:created",
    "role:updated",
    "role:archived",
    "role:restored",
    "role:deleted",
    "task:created",
    "task:updated",
    "task:deleted",
    "task:reordered",
    "message:saved",
    // 评审补（决策①）：会话级写 + 通知已读须触发 STATE_DELTA，否则手机
    // 会话/通知域永远滞后。conversation:title-updated 为既有前端事件，补订阅；
    // data:imported 整库导入替换全量数据，同样须触发重建。
    "conversation:created",
    "conversation:deleted",
    "conversation:title-updated",
    "notification:read",
    "data:imported",
];

/// `llm:stream` payload 是否为完成帧（done=true）。纯函数抽出（评审 P7）：
/// done 解析的三分支（done=true / 非 JSON / done 缺失或非布尔）必须有
/// 单测锁行为——监听器路径本身无 Tauri 事件注入手段，靠纯函数测试兜底。
/// 非 JSON / done 缺失一律视为未完成（fail-safe：宁少推不误推）。
fn llm_stream_done(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|v| v.get("done").and_then(|d| d.as_bool()))
        .unwrap_or(false)
}

/// 注册写信号监听：订阅既有事件 + 本 story 补发事件，转发 `WriteSignal`。
/// `llm:stream` 特判：仅 `done=true`（assistant 消息完成）才转发。
pub fn register_write_signal_listeners(
    app_handle: tauri::AppHandle,
    notify_tx: mpsc::Sender<WriteSignal>,
) {
    use tauri::Listener;

    for event in WRITE_SIGNAL_EVENTS {
        let tx = notify_tx.clone();
        let name = *event;
        app_handle.listen(name.to_string(), move |_event| {
            // try_send 非阻塞：监听器运行在 Tauri 主线程，不得阻塞
            let _ = tx.try_send(WriteSignal { event: name });
        });
    }

    let tx = notify_tx.clone();
    app_handle.listen("llm:stream".to_string(), move |event| {
        // 仅记判别结果，不落 payload 内容（NFR-M7）
        if llm_stream_done(event.payload()) {
            let _ = tx.try_send(WriteSignal {
                event: "llm:stream",
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_range_is_closed_and_ordered() {
        // WHY: 本季度过滤依赖区间字符串比较——start/end 错位或开闭区间
        // 弄反都会让边界上的简报/复盘被静默漏掉或多算。
        let (start, end) = current_quarter_range();
        assert!(start < end, "季度区间必须有序: {start}..{end}");
        assert!(start.ends_with("-01"), "季度起始必为当季首月 1 号: {start}");
        assert_eq!(start.len(), 10);
        assert_eq!(end.len(), 10);
    }

    #[test]
    fn quarter_range_at_covers_all_months_and_boundaries() {
        // WHY: 月份→季度映射是纯查表，错一位（如 Q3 起于 6 月）就会让
        // 边界月份的简报/复盘被静默丢弃——注入日期逐边界断言（评审 B12）。
        let cases = [
            // (日期, 期望 start, 期望 end)——含四季首日、季末、跨年边界
            ("2026-01-01", "2026-01-01", "2026-03-31"), // Q1 首日
            ("2026-03-31", "2026-01-01", "2026-03-31"), // Q1 末日在区间内
            ("2026-04-01", "2026-04-01", "2026-06-30"), // Q2 首日
            ("2026-06-15", "2026-04-01", "2026-06-30"),
            ("2026-07-01", "2026-07-01", "2026-09-30"), // Q3 首日
            ("2026-08-29", "2026-07-01", "2026-09-30"),
            ("2026-10-01", "2026-10-01", "2026-12-31"), // Q4 首日
            ("2026-12-31", "2026-10-01", "2026-12-31"), // 年末仍 Q4（不跨年）
        ];
        for (date, start, end) in cases {
            let (s, e) = quarter_range_at(date.parse().expect("parse date"));
            assert_eq!((s.as_str(), e.as_str()), (start, end), "日期 {date}");
        }
    }

    #[test]
    fn llm_stream_done_covers_three_branches() {
        // WHY: assistant 消息完成信号靠 done 字段判别，解析错误若被当成
        // done=true 会半途推送残缺快照、当成 panic 会杀死监听器——三分支
        // 行为必须锁死（评审 P7）。
        assert!(llm_stream_done(r#"{"done":true,"token":""}"#));
        assert!(!llm_stream_done(r#"{"done":false,"token":"x"}"#));
        assert!(!llm_stream_done(r#"{"token":"x"}"#), "done 缺失=未完成");
        assert!(!llm_stream_done("not json"), "坏 JSON=未完成（fail-safe）");
        assert!(!llm_stream_done(r#"{"done":"yes"}"#), "非布尔 done=未完成");
    }

    /// 从命令源码提取 `事件名样式`（`xxx:yyy`）的字符串字面量。
    /// 简易扫描：双引号包裹且匹配 `[a-z]+(-[a-z]+)*:[a-z-]+`。
    fn extract_event_names(src: &str) -> Vec<&str> {
        let bytes = src.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                if let Some(close) = src[i + 1..].find('"') {
                    let inner = &src[i + 1..i + 1 + close];
                    let parts: Vec<&str> = inner.split(':').collect();
                    let valid = parts.len() == 2
                        && !parts[0].is_empty()
                        && !parts[1].is_empty()
                        && inner
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c == '-' || c == ':');
                    if valid {
                        out.push(inner);
                    }
                    i += close + 2;
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    #[test]
    fn write_signal_events_cover_command_layer_emitters() {
        // WHY: 命令层 emit 与引擎订阅清单是两处独立维护的字符串集合，
        // 任何一侧单独演进（新增 emit 忘订阅 / 清单 typo）都会静默砍断
        // STATE_DELTA 推送——手机永远旧状态且无报错。以源码扫描锁契约
        // （评审 B16/P7）：命令文件中出现的每个事件名字面量必须被订阅
        // 或显式豁免。触发验证：在命令层新增 emit("xxx:yyy") 而不订阅，
        // 本测试即失败。
        const COMMAND_SOURCES: &[&str] = &[
            include_str!("../commands/role.rs"),
            include_str!("../commands/task.rs"),
            include_str!("../commands/chat.rs"),
            include_str!("../commands/notification.rs"),
            include_str!("../commands/task_decomposition.rs"),
            include_str!("../commands/data.rs"),
        ];
        // 豁免：llm:stream 由 register_write_signal_listeners 特判订阅
        // （仅 done=true 转发），不走 WRITE_SIGNAL_EVENTS 清单。
        const EXEMPT: &[&str] = &["llm:stream"];

        let mut emitted: Vec<&str> = Vec::new();
        for src in COMMAND_SOURCES {
            for name in extract_event_names(src) {
                if !emitted.contains(&name) {
                    emitted.push(name);
                }
            }
        }
        assert!(!emitted.is_empty(), "源码扫描必须提取到事件名（提取器失效）");

        let mut missing = Vec::new();
        for name in &emitted {
            let covered =
                WRITE_SIGNAL_EVENTS.contains(name) || EXEMPT.contains(name);
            if !covered {
                missing.push(*name);
            }
        }
        assert!(
            missing.is_empty(),
            "命令层 emit 的事件未被快照引擎订阅（STATE_DELTA 静默断链）: {missing:?}"
        );

        // 既有常量契约（评审 B16）：task.rs 的 TASK_CLASSIFIED_EVENT 与
        // 清单里的字符串必须指向同一事件，防止两处漂移。
        assert!(WRITE_SIGNAL_EVENTS
            .contains(&crate::commands::task::TASK_CLASSIFIED_EVENT));
    }
}
