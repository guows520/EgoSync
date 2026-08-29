---
baseline_commit: 0a932fd + 工作区未提交（12.3 relay-server / 12.4 交付产物）
---

# Story 13.1: 桌面快照引擎与状态主动推送

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a 手机用户,
I want 桌面把角色、任务、仪表盘、会话、简报复盘和未读通知的最新状态主动推到我的手机,
So that 我打开手机就能看到与桌面一致的状态，变化即时刷新，不需要手动下拉。

## Acceptance Criteria（AC）

> 完整 AC 以 `_bmad-output/planning-artifacts/epics.md` Story 13.1 段为唯一事实源，以下为逐条搬运，编号供任务引用。

1. **AC1 快照口径与内存持有**：`services/companion_snapshot.rs` 实现快照生成，口径严格为：角色卡状态（含能量）+ 四象限任务 + 仪表盘指标 + 活跃及近期会话各最近 200 条消息 + 本季度晨间简报/周复盘 + 未读通知；**记忆库内容不上机**（角色卡上的记忆条数统计数字属仪表盘指标，允许）；快照在内存持有、不持久化，桌面重启后首次连接现生成。
2. **AC2 debounce 触发与桌面零回归**：桌面端相关写操作（角色/任务/会话/通知/简报等）完成后，快照引擎经既有事件/通知路径触发 debounce 节流重建，不改现有 services 的业务逻辑；重建完成后向已连接手机推送 `STATE_DELTA`；手机断线期间积压的变化在重连后以全量 `SNAPSHOT` 替换补齐（不做历史增量回放）。
3. **AC3 建连即全量快照**：已配对手机建立加密连接（含重连）时，桌面下发全量 `SNAPSHOT` 帧（全量替换式）；手机在线期间状态变化触发 `STATE_DELTA` 主动推送，核心状态无需手机轮询。
4. **AC4 10MB 上限与截断明示**：快照体积达到 10MB 上限时按最旧截断，截断信息进快照元数据（数据截止时间字段）供手机 UI 明示；截断只影响会话/简报等可截断域，角色/任务/指标等核心状态不缺失。
5. **AC5 帧编码与日志纪律**：帧经 companion-proto 加密承载，payload 字段 camelCase；帧明文不入日志（NFR-M7）。
6. **AC6 测试覆盖**：`tests/test_companion.rs` 覆盖：口径完整性（含「记忆内容不在快照中」的负向断言）、debounce 节流、10MB 截断、断线重连补发最新快照。

## Tasks / Subtasks

- [x] Task 1：快照领域结构 `models/snapshot.rs`（AC1、AC4）
  - [x] `DesktopSnapshot` 投影结构（serde camelCase）：`schemaVersion: u32`（快照业务结构版本，独立于 protocolVersion）、`generatedAt`、`dataCutoffAt: Option<String>`、`truncated: bool`、`truncatedDomains: Vec<String>` + 七个数据域（roles/tasks/dashboard/conversations/briefings/weeklyReviews/notifications）
  - [x] 各域用**投影结构**而非直接复用 `Role`/`Task` 等内部模型——角色卡**不含** `personality_prompt`（桌面内部数据不上机）；任务含 `quadrant/isBigRock/isCompleted/protectionStatus`；会话含 `id/roleId/title/updatedAt/messages`；仪表盘含 `DashboardStatus` 数组 + 四项统计指标（memoryCount 仅数字）
  - [x] `models/mod.rs` 注册；`#[serde(rename_all = "camelCase")]`
- [x] Task 2：快照聚合与截断 `services/companion_snapshot.rs`（AC1、AC4）
  - [x] `pub async fn build_snapshot(pool: &DbPool, conv_pool: &ConversationsPool) -> Result<DesktopSnapshot, AppError>`——数据源 API 见 Dev Notes 关键技术情报；全部失败显式传播（`?`），不吞错
  - [x] 「本季度」过滤：`list_all_briefings` / `list_all_weekly_reviews` 全量取回后 Rust 侧按季度区间字符串比较过滤（参照 `review_generator.rs:200-215` 的本周过滤模式）；chrono 推当前季度 `[start, end]` YYYY-MM-DD
  - [x] 会话消息：`list_all_conversations` → 每会话 `get_recent_messages(conv_pool, &id, 200)`
  - [x] 10MB 截断：序列化后 > 10MB 则按裁决 6 顺序截断最旧可截断域并重建，迭代至达标；写 `truncated/dataCutoffAt/truncatedDomains` 元数据
  - [x] 负向保证：聚合函数不 import `db::memories`；memoryCount 来自 `get_dashboard_metrics`
- [x] Task 3：SNAPSHOT/STATE_DELTA 分帧与重组（AC3、AC5）
  - [x] 分帧（发送方）：快照 JSON 序列化 → 按 60_000 字节切片 → base64 每片 → `Frame::Snapshot/StateDelta` 的 `data` 字段承载 envelope JSON `{"seq":i,"total":n,"chunkBase64":"..."}`（camelCase）——**不 bump PROTOCOL_VERSION**（裁决 1）
  - [x] 重组（接收方断言用）：`fn reassemble(frames: &[Frame]) -> Result<DesktopSnapshot, _>`——按 seq 收齐 total 片、拼接、base64 解码、JSON 反序列化；13.2 Kotlin FrameCodec 镜像此逻辑
  - [x] envelope 结构定义在 `companion_snapshot.rs`（app 层协议，不进 companion-proto crate）
- [x] Task 4：出站帧通道——`ActiveSession` 增发送能力（AC2、AC3）
  - [x] `ActiveSession` 增 `outbound: tokio::sync::mpsc::Sender<Frame>`；`enter_session` 建立对应 receiver，`tokio::select!` 增一分支：收到 Frame → `encode_frame(&frame, transport)` → `send_frame(io, ...)`；发送失败按既有断连路径 break
  - [x] `CompanionState` 增 `enqueue_outbound(&self, frame: Frame)`（try_send，会话不在线即丢弃——OnConnect 全量补齐兜底）；单生产者（快照引擎）单消费者（会话循环）FIFO 保证分帧有序；**通道容量 ≥ 256**——10MB 快照 ≈ 180+ 分帧，容量不足会 try_send 中途丢帧截断序列（丢帧自愈靠重连 OnConnect 重发全量，但在线期间静默丢帧违反 AC3）
  - [x] `CompanionState` 增 `snapshot_request_tx: tokio::sync::Mutex<Option<mpsc::Sender<SnapshotRequest>>>`（setup 时注入）；`enter_session` 在 `set_connected` + `EVENT_CONNECTED` 后发送 `SnapshotRequest::OnConnect`——**不走 Tauri 事件**，保证 `app_handle: None` 的测试路径可用
- [x] Task 5：debounce 引擎与触发路径接线（AC2）
  - [x] `CompanionSnapshotEngine`：持有 `notify_tx: mpsc::Sender<WriteSignal>`、debounce 窗口（生产 2000ms 常量，测试构造器可注入短窗）、pool/conv_pool、`Arc<CompanionState>`；`run()` 长跑任务——收到信号后 debounce 窗口内合并后续信号，窗口静默后重建快照并 `enqueue_outbound` 全部分帧（`STATE_DELTA`）
  - [x] `pub async fn handle_connected(&self)`：重建 + 全量 `SNAPSHOT` 分帧入队（响应 `SnapshotRequest::OnConnect`）
  - [x] `lib.rs` setup 装配：构造 engine → `app.manage` → spawn `engine.run()` → 注入 `snapshot_request_tx` 到 CompanionState → `app.listen` 订阅事件清单（见关键技术情报）转发 `WriteSignal`
  - [x] 命令层补发事件（裁决 3，**仅 commands 层、零 services 改动**）：`commands/role.rs`（role:created/updated/archived/restored/deleted——现零 emit）、`commands/task.rs`（task:created/updated/deleted/reordered/completed——现仅 task:classified）、`commands/chat.rs`（消息落库完成处 message:saved）。命令签名增 `app_handle: AppHandle` 参数（Tauri 自动注入，前端 invoke 不变；task.rs:46 / chat.rs:407 已有先例）
- [x] Task 6：测试 `tests/test_companion.rs`（AC6，全部用例见测试策略）
  - [x] 口径完整性 + 记忆内容负向断言；debounce 合并；10MB 截断与元数据；建连收全量 SNAPSHOT（真 WS 路径）；写信号推 STATE_DELTA；断线重连补最新快照；分帧重组 roundtrip
- [x] Task 7：验证收尾（全部 AC）
  - [x] `cd egosync-app && npm run test:all`（vitest + cargo；预存 6 个 lib 失败需区分，见 Previous Story Intelligence）；`npm run build`
  - [x] 逐条勾选 AC Checklist；如实记录未执行的验证边界（无真机环境等），禁止声称未验证的事项

### Review Findings

> 预留：代码评审结论记录区。

## Dev Notes

### 架构硬边界（违反即返工）

1. **桌面边界**：`companion_snapshot` 只读聚合既有 db/service API + 经 `CompanionState` 出站通道发帧；**现有 services 对伴侣一无所知**（NFR-M4 / architecture 硬边界 #2）——触发事件只在 commands 层补发，services 一行不动。
2. **加密边界**：帧编码只调 `companion_proto::frames::{encode_frame, decode_frame}`；不触碰 snow / TransportSession 内部；分帧 envelope 是 app 层 JSON，**不进 companion-proto crate、不 bump PROTOCOL_VERSION**。
3. **协议冻结**：8 帧类型、schema.json、PROTOCOL_VERSION=1 一律不动；`SnapshotPayload{data:String}` 保持 opaque——分帧 envelope 塞进 `data` 字符串内（裁决 1）。
4. **数据边界**：快照**内存持有、不持久化**（不新增迁移、不落盘）；conversations.db 只经 `db::conversations` 读；记忆库内容不上机。
5. **日志纪律（NFR-M7）**：帧明文、快照 JSON、chunk 内容永不入日志——沿用 12.2 P5 整改后的「仅记帧类型判别式」模式（`frame_type_name`），严禁 `?frame`/`?payload` Debug 打印；tracing 只记事件类别/域计数/字节数。

### 设计裁决（规则七，显式择一）

| # | 冲突/歧义 | 裁决 | 理由 |
|---|------|------|------|
| 1 | 10MB 快照 vs 单帧明文上限 65519B（`MAX_PLAINTEXT_LEN`，crate 实测）：SNAPSHOT 必然超限，12-1 评审已点名此为真实场景 | **app 层分帧 envelope（`data` 内 `{"seq","total","chunkBase64"}`），不 bump PROTOCOL_VERSION** | 备选=给 SnapshotPayload 增 `seq/total` 字段须 bump 1→2——12.4 已交付的 Android `FrameCodec` HELLO 校验硬编码 `protocolVersion==1`，bump 即断已配对连接，桌面单 story 无法增量交付（13.2 才改 Kotlin）。协议层 payload 本就「对协议层 opaque」（frames.rs:48-49），分帧属 app 层关注点；13.2 Kotlin 侧镜像 envelope 重组即可。代价：快照 JSON 双层编码 + base64 ~33% 膨胀（仅载荷），可接受 |
| 2 | STATE_DELTA 内容：域级增量 diff vs 全量替换 | **STATE_DELTA 载荷 = 全量快照（与 SNAPSHOT 同载荷同分帧），仅触发时机不同**（建连=SNAPSHOT，变化推送=STATE_DELTA） | architecture.md:2090 明文「下发：全量替换式」——架构已裁决；AC2「不做历史增量回放」同向；13.2 StateMerger 退化为全量替换实现最简（规则二）；域级 diff 需新旧对比逻辑，违反简单至上。带宽代价由 debounce（裁决 4）+ 10MB 截断兜底；域级增量优化列 deferred（见开放问题） |
| 3 | 触发路径：AC2 要求「经既有事件/通知路径」且角色写操作必须触发——但现状 `commands/role.rs` **零 emit**、`commands/task.rs` 仅 `task:classified`、后端无任何 service 间事件通道 | **快照引擎 Rust 侧 `app.listen` 订阅 Tauri 事件 + commands 层补发缺失事件**；**不**给 services 加 mpsc 通知钩子 | mpsc 直连违反硬边界 #2（services 将「知道」伴侣存在）；app.listen 是「既有事件/通知路径」的字面实现；命令层 emit 是传输层副作用、业务逻辑零变更（task.rs:46 / chat.rs:407 先例），新增事件前端可自由消费。补发事件是 AC2「角色写操作触发」的唯一可满足解，非可选 scope creep |
| 4 | debounce 窗口时长 | **2000ms 常量**（测试构造器可注入短窗） | 未有规格约束；2s 平衡「变化即时感」与高频写抖动（连续对话落库不逐条重建）。测试注入是可测性必需（非 YAGNI）——真实 2s 窗口会让 debounce 测试拖慢全量套件 |
| 5 | 分片尺寸 | **48_000 字节明文切片 + base64** | envelope JSON + AEAD tag（16B）后仍 < 65519 单帧明文上限；base64 规避 UTF-8 多字节字符被字节级切片截断的问题。（2026-08-29 评审回写：原值 60_000 漏算 base64 4/3 膨胀——60_000×4/3=80_000 明文超 65519，被 crate 既有 `oversized_frame_encode_is_rejected` 拒绝；按规则七以可工作的 48_000 为准，48_000→base64=64_000，+envelope 开销 < 65519。） |
| 6 | 10MB 截断顺序 | **会话域先截**（最旧会话整段丢弃 → 仍超限则各会话消息数减半迭代）→ 本季度简报（最旧起丢）→ 周复盘（最旧起丢）；角色/任务/仪表盘指标/未读通知**永不截断** | AC4 只落死「会话/简报等可截断域」；会话消息是体积大头且按会话粒度丢弃实现最简；`dataCutoffAt` = 被截断域中保留数据的最旧时间戳 |
| 7 | 快照结构文件归属：塞 `models/companion.rs` vs 新文件 | **新文件 `models/snapshot.rs`** | companion.rs 已承载配对/连接模型；快照域结构含 7 个域的投影类型，独立文件仍符合 models/ 域文件惯例（architecture 目录树未穷举，不冲突） |
| 8 | 快照业务结构演进 | `schemaVersion: u32` 从 1 起 | 区别于 protocolVersion；13.2 消费方校验；快照结构变更走此字段，不碰协议层 |

### 关键技术情报（全部源码核实，实现前勿凭网上示例拼装）

**数据源 API 精确签名（聚合直接调用，全部已存在）：**

- 角色卡：`db::roles::list_active_roles(pool) -> Result<Vec<Role>, AppError>`——`Role` 含 `id/name/icon/color/goal/status/energy: i32/proactivity_level`（`models/role.rs:3`；能量已在字段上，**不需要**调 `energy_calculator::calculate_and_update_energy`——那是有写副作用的计算，快照只读现值）
- 四象限任务：`db::tasks::list_all_tasks(pool, quadrant: Option<&str>, is_big_rock: Option<bool>) -> Result<Vec<CrossRoleTask>, AppError>`——两参传 `None` 取全量；`CrossRoleTask` 含 `role_name/role_color`（`db/tasks.rs:60`）
- 仪表盘：`services::dashboard_service::get_dashboard_status(pool, conv_pool) -> Result<Vec<DashboardStatus>, AppError>`（角色卡态，`dashboard_service.rs:11`）+ `get_dashboard_metrics(pool, conv_pool, DashboardMetricsQuery) -> Result<DashboardMetrics, AppError>`（`{taskCount, memoryCount, conversationCount, pendingTaskCount, generatedAt}`，`dashboard_service.rs:74`）；`DashboardMetricsQuery { scope, start_at: Option<String>, end_at: Option<String> }`（`models/dashboard.rs:25-29`）——快照取 `scope: All` + 两窗口字段 `None`（全时间窗，最简）；移动端筛选交互（FR-38 时间窗）的数据需求属 13.2/13.3 消费方裁决，可走 `schemaVersion` 演进承载
- 会话与消息（独立库）：`db::conversations::list_all_conversations(conv_pool) -> Result<Vec<Conversation>, AppError>`（updated_at DESC，`db/conversations.rs:253`）→ 每会话 `db::conversations::get_recent_messages(conv_pool, &conversation_id, 200) -> Result<Vec<Message>, AppError>`（**现成的 200 条入口**，返回时间正序，`db/conversations.rs:388`）；`Message` 含 `id/conversation_id/role/content/thinking_content/is_complete/created_at`（`models/chat.rs:20`）
- 晨间简报：`db::briefings::list_all_briefings(pool) -> Result<Vec<Briefing>, AppError>`（`Briefing{content, date: "YYYY-MM-DD"}`，`db/briefings.rs:46`）→ Rust 侧本季度过滤
- 周复盘：`db::weekly_reviews::list_all_weekly_reviews(pool) -> Result<Vec<WeeklyReview>, AppError>`（`{week_start, week_end, summary, energy_trends, bigrock_status, new_memories_count}`，`db/weekly_reviews.rs:60`）→ 本季度过滤（按 `week_start` 落区间判定）
- 未读通知：`db::notifications::list_notifications(pool) -> Result<Vec<NotificationWithRole>, AppError>`（`db/notifications.rs:44`）后过滤 `is_read == false`；`count_unread(pool)` 可作断言辅助（`:74`）

**协议层（crate，路径依赖 `egosync-app/src-tauri/Cargo.toml:39`）：**

- `companion_proto::frames::{encode_frame(&Frame, &mut TransportSession) -> Result<Vec<u8>, ProtoError>, decode_frame(&[u8], &mut TransportSession) -> Result<Frame, ProtoError>}`（`frames.rs:99/112`）
- `Frame::Snapshot(SnapshotPayload{data: String})` / `Frame::StateDelta(StateDeltaPayload{data: String})`——payload opaque、`deny_unknown_fields`（`frames.rs:51-63`）
- `PROTOCOL_VERSION: u16 = 1`（`crates/companion-proto/src/lib.rs:24`，schema.json 同步）；`MAX_PLAINTEXT_LEN = 65519`、`MAX_CIPHERTEXT_LEN = 65535`（`crypto.rs:18/22`）——超限编码返回 `ProtoError::Encode`（既有 crate 测试 `oversized_frame_encode_is_rejected` 已锁行为，分帧必须保证单帧不触限）

**连接层（`services/companion_connection.rs`，工作区版本）：**

- `CompanionState` 字段与构造：生产 `CompanionState::new(app_handle)`（走 keyring）/ 测试 `with_static_keypair_for_testing(app_handle, priv, pub)`（`:96-108`）——快照引擎持有 `Arc<CompanionState>`，测试用同款注入
- `ActiveSession { device_id, terminate: watch::Sender<bool> }`（`:62`）——**当前无任何出站帧能力**，`io`/`transport` 是 `enter_session` 任务局部变量（`:702-799`）；出站通道是本 story 核心新增
- `enter_session` 帧循环 `tokio::select!` 模式：term_rx 分支 + 超时 recv_frame 分支（`:729-747`）；出站分支照此增补；会话退出时的「仅当仍为当前活跃会话才复位」守护逻辑（`:783-797`）不得破坏
- 建连点：`state.set_connected(...)` + `state.emit(EVENT_CONNECTED, ...)`（`:723-727`）——`SnapshotRequest::OnConnect` 在此后发送
- 事件常量：`EVENT_PAIRED/CONNECTED/DISCONNECTED = "companion:paired/connected/disconnected"`（`companion_pairing.rs:32-34`）；`emit` 在 `app_handle: None` 时跳过（`:174-180`）——**测试路径收不到 Tauri 事件**，故 OnConnect 必须走直连通道（裁决见 Task 4）
- 装配点：`lib.rs:304-322` setup 内 `CompanionState::new` → `app.manage` → `spawn(start_companion_listener(pool_c, state))`

**触发事件订阅清单（engine `app.listen` 注册）：**

既有事件：`notification:new`（commands/notification.rs:56、scheduler.rs:265）、`task:classified`（commands/task.rs:46）、`briefing:generated`（services/briefing_generator.rs:33）、`review:generated`（services/review_generator.rs:31）、`bigrock:protection`、`bigrock:reminder`、`q2:reminder`（三保护提醒）、`llm:stream`（chat.rs:407，`done=true` 才转发信号）、`role:proposed`、`role:delegated`、`task:*`（task_decomposition/delegate_bridge 既有 emit）。

新增 emit（Task 5，命令签名增 `app_handle: AppHandle`）：`commands/role.rs` role_create/role_update/role_update_skills/role_update_proactivity/role_archive/role_restore/role_delete → `role:created/updated/updated/archived/restored/deleted`（**该文件现零 emit、零 app_handle 参数**，需逐命令补参数）；`commands/task.rs` task_create（已带 `app_handle`——`:46` 即经它 emit task:classified）/task_update/task_delete/task_reorder/task_toggle_complete → `task:created/updated/deleted/reordered/completed`；`commands/chat.rs` 用户消息落库点（`insert_message` 调用处：`:329` 管家发送路径、`:857` 角色对话路径；`:326-333` onboarding 路径）→ `message:saved`——assistant 消息完成由既有 `llm:stream`（`done=true`，`chat.rs:407`）覆盖，无需重复 emit。payload 沿用域对象（前端可消费，快照引擎只看事件名不看 payload）。

**测试基建（`tests/test_companion.rs`）：**

- `test_pool() -> (DbPool, ConversationsPool, TempDir)`（`:35`，真实 SQLite 全迁移）；`setup_listener() -> (DbPool, Arc<CompanionState>, u16, priv, pub, TempDir)`（`:91`）；`phone_handshake/send_frame/send_pairing_nonce`（`:51-89`）；`wait_for(check, timeout_ms, what)` 200ms 轮询断言（`:105`）；中继辅助 `spawn_relay/next_binary/relay_phone_auth`（`:702-748`）
- `tests/common/mod.rs` 仅空壳 `pub fn setup() {}`——无 mock 框架，全部真 socket/真库
- 手机侧收帧断言：WS 读 binary → `decode_frame(&bytes, &mut transport)` → 匹配 `Frame::Snapshot/StateDelta` → 取 `data` → 解 envelope 累积 → 重组断言

**其他事实：**

- 无需新迁移（快照不持久化）；`migrations/` 最新为 `031_paired_devices.sql`
- `db::memories` **禁止**出现在 `companion_snapshot.rs` 的 import 中（负向约束，评审检查点）
- `CompanionState` 的 emit/`app.listen` 均依赖 `AppHandle`；engine 的 `run()` 与 `handle_connected()` 设计为可直接调用的独立方法，监听接线只是生产装配

### 测试策略（规则九：验证意图）

| 用例 | 意图锚点（WHY） |
|------|----------------|
| `snapshot_scope_completeness_and_memory_exclusion` | 桌面唯一事实源下手机只该拿到口径内数据——七域齐全 + 种子记忆内容后序列化 JSON **不含**记忆文本子串（负向）；memoryCount 仅作为数字出现 |
| `debounce_coalesces_rapid_writes` | 高频写（连续对话）不得逐条触发 10MB 级重建拖慢桌面（sprint 风险 #1）——注入短窗，N 次快速 notify → 恰 1 次重建/推送 |
| `snapshot_respects_10mb_cap_and_truncates_oldest` | 截断是诚实性机制而非静默丢数据——构造 >10MB 会话，断言 truncated=true、dataCutoffAt 非空、核心域（角色/任务/指标）完整、最旧会话被丢弃 |
| `connected_phone_receives_full_snapshot_on_connect` | 「打开手机即见桌面一致状态」——真 WS 握手进会话后手机侧收齐全部分帧并重组出合法快照 |
| `write_signal_pushes_state_delta` | 「变化即时刷新、无需下拉」——会话在线时触发写信号 → 手机收到 STATE_DELTA 分帧，重组载荷反映变更 |
| `reconnect_receives_latest_snapshot_after_gap` | 「断线补最新快照、不做历史回放」——断开期间变更数据、重连后收到的 SNAPSHOT 含变更且为单一全量（非增量序列） |
| `chunking_roundtrip_and_size_bound` | 分帧对协议层透明——大快照分帧→重组 roundtrip 无损；单帧编码不触发 `ProtoError::Encode`（受 65519 约束） |

时间相关测试全部注入时钟/短窗（debounce 构造器），不依赖真实 sleep。

### Project Structure Notes

```text
egosync-app/src-tauri/src/
├── models/
│   ├── snapshot.rs                     # [N] DesktopSnapshot + 七域投影结构
│   └── mod.rs                          # [M] 注册
├── services/
│   ├── companion_snapshot.rs           # [N] 聚合 + 截断 + 分帧 envelope + debounce 引擎 + 重组
│   ├── companion_connection.rs         # [M] ActiveSession 出站通道 + enter_session select! 出站分支 + OnConnect 请求
│   └── mod.rs                          # [M] 注册
├── commands/
│   ├── role.rs                         # [M] 六写命令增 app_handle + role:* emit
│   ├── task.rs                         # [M] 五写命令增 task:* emit（app_handle 已有先例）
│   └── chat.rs                         # [M] 消息落库点增 message:saved emit
├── lib.rs                              # [M] setup 装配 engine + app.listen 订阅
└── tests/
    └── test_companion.rs               # [M] 七个新用例 + 手机侧收帧/重组辅助
```

（companion-proto crate、relay-server、companion-android、前端全部零改动。）

### Previous Story Intelligence（12.1/12.2/12.3/12.4 → 13.1）

- **帧日志泄露血泪（12.2 P5）**：`?frame` Debug 打印曾把 Snapshot 明文泄进日志——新代码只准用帧类型判别式（`frame_type_name` 模式）。
- **超限帧是既知真实场景（12.1 评审）**：单帧 64KB 上限 vs 全量快照已被点名——本 story 分帧裁决直接回应。
- **AC 字面承诺须有可测闭环（12.2 D1/D2/D3 模式）**：「全量替换式」「重连补最新」都要有上表用例，不能只实现不验证。
- **超时预算防挂起（12.2 P4 / 12.3）**：重建聚合若慢须有界（SQLx 查询超时或整体预算），debounce 任务不得永久卡死；`enqueue_outbound` 用 try_send 不阻塞。
- **中继 keepalive Ping 透传（12.4 实证修复①）**：会话循环读帧须跳过 Ping/Pong——出站分支与读循环并存时沿用 `WsIo.recv` 既有过滤。
- **双承载切换重复投递（12.4 P5 整改先例）**：单会话槽替换语义已保证任一时刻只有一个会话任务在消费出站通道——新连接取代旧会话时旧任务退出，未消费的分帧自然丢弃，重连 OnConnect 全量补齐兜底。
- **预存测试漂移**：`cargo test --lib` 有 6 个基线即失败（agent_config 5 + agent_engine 1）——判绿时排除该已知集合并如实说明，不得顺手修。
- **工作区未提交基线**：12.3/12.4 产物仍未 commit——本 story 基线为 0a932fd + 工作区；开工前确认工作区状态，勿与他人未提交变更混提。

### Git Intelligence

- HEAD = 0a932fd（12.2 桌面配对交付）；工作区含 12.3（relay-server）+ 12.4（Android 换装 + 桌面中继接线）未提交产物，`tests/test_companion.rs` 与 `companion_connection.rs` 均为工作区版本（分帧/出站通道改动叠加其上）。
- 12.4 对 `enter_session` 的重构（`run_authorized_session` 抽取、`WsIo<S>` 泛型化）是本 story 出站通道改动的直接基座——读工作区版本而非 HEAD 版本。

### Latest Tech Information

- 无新三方依赖（tokio mpsc / base64 / chrono / serde 均已在依赖树；base64 若未直接依赖则用既有依赖确认后引入，见开放问题 Q2）。Rust 侧 `AppHandle::listen` 来自 tauri 2 `Listener` trait——实现时以本仓 tauri 2.x 实测 API 为准（`use tauri::Listener;`）。
- 不引入任何轮询/定时全量推送库；debounce 用 tokio::time 自实现（select + sleep_reset 模式）。

### 范围外（明确不做，防 scope creep）

- Android 侧 SNAPSHOT/STATE_DELTA 解码与 SnapshotStore 换装（13.2）；COMMAND/STREAM_TOKEN/dispatch（13.3）；速记队列（14.1）；NOTICE 业务通知与 NotificationDispatch（14.2）。
- companion-proto crate 任何改动（含 schema.json、PROTOCOL_VERSION、payload 结构）；relay-server 任何改动；前端任何改动。
- 快照持久化/桌面磁盘缓存/新数据库迁移（AC 明确内存持有）。
- 域级增量 STATE_DELTA diff 算法（裁决 2 已定全量替换；优化列开放问题）。
- 记忆页数据通道（UX-M5：记忆走 13.3 指令通道现查）。

### 开放问题（实现后向 boss 汇报，不阻塞开发）

1. **Q1 STATE_DELTA 全量推送的带宽**：大会话（近 10MB）+ 高频写 + 中继路径 = 每次变化近 10MB 推送。debounce + 截断兜底后 V1 可接受；若 UAT 证实中继路径体验差，后续 story 引入域级 delta（旧快照域对比）——届时走 `schemaVersion` bump，不碰协议层。
2. **Q2 base64 依赖确认**：若 src-tauri 尚无直接 base64 依赖，引入 `base64` crate（Rust 生态标准件）需在依赖白名单理由中注明；或改用十六进制编码（无需新依赖，体积翻倍）——实现时择一并记录。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-13.1]（AC 唯一事实源）
- [Source: _bmad-output/planning-artifacts/epics.md#Requirements-Inventory（增量）]（Additional 2/6/8、NFR-M3/M4/M5/M7——注意在 epics.md 而非 prd-egosync.md）
- [Source: _bmad-output/planning-artifacts/architecture.md#快照引擎（FR-41/43）]（口径落死、10MB、debounce、全量替换式下发）
- [Source: _bmad-output/planning-artifacts/architecture.md#Incremental-Core-Architectural-Decisions—手机伴侣基建]（决策 5 快照口径、四条硬边界）
- [Source: _bmad-output/planning-artifacts/prd-egosync.md#FR-41]（实时状态同步 + ASSUMPTION 重连仅补最新快照）
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-…md#§5/§7]（13.1 禁止项：记忆入快照、快照持久化；测试范围五项）
- [Source: egosync-app/src-tauri/src/services/companion_connection.rs]（CompanionState/ActiveSession/enter_session 基座）
- [Source: crates/companion-proto/src/{frames.rs,crypto.rs,lib.rs}]（帧编解码、65519/65535 上限、PROTOCOL_VERSION=1）
- [Source: egosync-app/src-tauri/src/services/{dashboard_service,briefing_generator,review_generator}.rs、src/db/{roles,tasks,conversations,briefings,weekly_reviews,notifications}.rs]（数据源 API）
- [Source: egosync-app/src-tauri/tests/test_companion.rs]（测试基建模式）
- [Source: _bmad-output/implementation-artifacts/12-{1,2,3,4}-*.md]（协议冻结、日志纪律、P5/P6 语义、中继 128KB、R1 换绑限制）
- [Source: _bmad-output/implementation-artifacts/deferred-work.md]（snapshotstore-vm-wiring 评审：空列表守卫/绝对日期属 13.2）
- [Source: _bmad-output/project-context.md]（基线 100 条规范）

## Dev Agent Record

### Agent Model Used

glm-5.3（DeepSeek Harness / bmad-agent-dev Amelia）

### Debug Log References

- 首轮测试 3 处失败已修复：① task 创建 `owner_type: None` 触发「角色任务必须指定角色」→ 改 Butler；② chunking 用例暴露裁决 5 数值缺陷（60_000×4/3=80_000 明文 > 65519 上限，crate 既有 `oversized_frame_encode_is_rejected` 拒绝）→ 分片改 48_000；③ 测试代码自身类型错误（String/&str、partial move）。修复后 7/7 全绿。

### Completion Notes List

- **Task 1**：`models/snapshot.rs` 新建——`DesktopSnapshot`（schemaVersion=1 起步）+ 七域投影结构，全部 camelCase；角色卡投影不含 `personality_prompt`/`skills_config`；已注册 `models/mod.rs`。
- **Task 2**：`services/companion_snapshot.rs::build_snapshot` 聚合七域；本季度过滤用 chrono Local 推 `[start, end]` 闭区间字符串比较；10MB 截断按裁决 6 四阶段（整段丢最旧会话→消息数减半→简报→复盘）迭代重序列化，`dataCutoffAt` 取被截断域保留数据最旧时间戳（空域兜底 generatedAt）；负向保证成立——**未 import `db::memories`**，memoryCount 仅来自 `get_dashboard_metrics`。
- **Task 3**：分帧 `frame_snapshot`（JSON→48_000 字节切片→base64→`ChunkEnvelope{"seq","total","chunkBase64"}` 塞进 `data`，不 bump PROTOCOL_VERSION）+ `reassemble`（seq 完整性校验+base64 解码+反序列化）。envelope 结构 pub 供 13.2 Kotlin 镜像。
- **Task 4**：`ActiveSession` 增 `outbound: mpsc::Sender<Frame>`（容量 512 ≥ 256 下限，10MB≈220 分帧）；`enter_session` select! 增出站分支（编码/发送失败按既有断连路径 break）；`CompanionState` 增 `enqueue_outbound`（try_send 非阻塞、丢弃仅 debug 日志）与 `snapshot_request_tx` 直连通道；`set_connected`+`EVENT_CONNECTED` 后发 `SnapshotRequest::OnConnect`。
- **Task 5**：`CompanionSnapshotEngine`（生产 2000ms/测试注入短窗 debounce，固定窗口+窗口内信号合并丢弃语义，延迟有界）；`run()` 双通道 select（OnConnect→全量 SNAPSHOT；写信号→debounce→STATE_DELTA 全量载荷）；lib.rs setup 装配（构造→注入 request_tx→manage→spawn run→`register_write_signal_listeners` 订阅，`llm:stream` 仅 done=true 转发）；命令层补发事件——role.rs 7 命令、task.rs 5 命令、chat.rs 用户消息落库点 `message:saved`，命令签名增 `app_handle`（Tauri 自动注入，前端 invoke 不变）；**services 层零改动**（硬边界 #2 成立）。（2026-08-29 评审回写：原记「22 事件订阅」计数有误且评审后清单扩充——现为 26 = WRITE_SIGNAL_EVENTS 25 项 + llm:stream 特判 1 项，含新增 task:tool-action / conversation:created / conversation:deleted / conversation:title-updated / notification:read / data:imported，移除 task:completed 改发 task:updated；OnConnect 通道 mpsc→watch 幂等合并。）
- **Task 6**：`tests/test_companion.rs` 增 7 用例 + 4 辅助（setup_listener_with_engine/seed/phone_receive_snapshot/session_pair），全部真 socket/真 SQLite。
- **Task 7**：`cargo test` 全量 829(lib)+1(test_app)+22(test_companion) 全绿；vitest 438/438；`npm run build` tsc 零类型错误。
- **冲突裁决（规则七）**：裁决 5 原文「60_000 字节明文切片 + base64」与 65519 单帧上限数学冲突（60_000×4/3=80_000>65_519）——择 48_000 字节切片（base64=64_000，+envelope 开销 < 65519），已在常量注释声明；story 原值 60_000 列为笔误待清理。
- **Q2 裁决（开放问题）**：`base64` crate 已在依赖树（0.22.1，transitive），置为 src-tauri 直接依赖（无新版本引入），优于十六进制（体积翻倍）。
- **预存失败未复现**：Previous Story Intelligence 提到的「cargo test --lib 6 个基线失败（agent_config 5 + agent_engine 1）」在本环境（Linux/当前工作区）未复现，829/829 全绿——如实记录，未做任何「顺手修复」。
- **验证边界**：未执行真机（Android）E2E 与 `npx tauri build` 安装包 UAT（13.1 范围内 Kotlin 侧零改动，属 13.2 交付物）；中继路径下快照推送未单测（WS 会话路径直连/中继同构，12.4 已验证承载层）。

### File List

- egosync-app/src-tauri/src/models/snapshot.rs（新增）
- egosync-app/src-tauri/src/models/mod.rs（修改：注册 snapshot）
- egosync-app/src-tauri/src/services/companion_snapshot.rs（新增）
- egosync-app/src-tauri/src/services/mod.rs（修改：注册 companion_snapshot）
- egosync-app/src-tauri/src/services/companion_connection.rs（修改：ActiveSession 出站通道 + OnConnect 请求 + enqueue_outbound）
- egosync-app/src-tauri/src/lib.rs（修改：setup 装配快照引擎 + 事件订阅）
- egosync-app/src-tauri/src/commands/role.rs（修改：7 写命令 app_handle + role:* emit）
- egosync-app/src-tauri/src/commands/task.rs（修改：5 写命令 task:* emit）
- egosync-app/src-tauri/src/commands/chat.rs（修改：message:saved emit）
- egosync-app/src-tauri/Cargo.toml（修改：base64 直接依赖）
- egosync-app/src-tauri/tests/test_companion.rs（修改：7 新用例 + 辅助）
- egosync-app/src-tauri/Cargo.lock（生成物：base64 依赖条目）

### Change Log

| Task | AC | 变更摘要 |
|---|---|---|
| 1 | AC1/AC4 | models/snapshot.rs 投影结构 |
| 2 | AC1/AC4 | companion_snapshot 聚合 + 本季度过滤 + 10MB 截断 |
| 3 | AC3/AC5 | 分帧 envelope + 重组（分片 48_000，裁决 5 冲突修正） |
| 4 | AC2/AC3 | ActiveSession 出站通道 + OnConnect 请求 |
| 5 | AC2 | debounce 引擎 + app.listen 订阅 + 命令层补发事件 |
| 6 | AC6 | 七个测试用例 |
| 7 | 全部 | 全量验证与 AC Checklist（cargo 852 + vitest 438 + build 全绿） |

### Review Findings（2026-08-29 三路对抗评审：Blind/EdgeCase/Auditor）

33 条原始发现 → 归并后 17 条（2 decision / 10 patch / 3 defer），2 条 dismiss 不列入。

**patch 应用记录（2026-08-29，用户裁决全部修复）：** 12 条 patch 全部落地并重跑验证——cargo test 857 全绿（lib 832 含 4 新单测 / test_companion 24 含 2 新用例 / test_app 1）+ vitest 438/438 + `npm run build` tsc 零错误。两处与发现原文的偏差（证据驱动）：① P3「会话代次守卫」以更简实现达成——`enqueue_outbound` 改整序列持锁入队，与新连接的会话槽替换互斥，残缺序列天然不跨会话；② 小项簇④「断言收紧为 65519」复核后不采纳——线材 = 4B 长度前缀 + 密文，密文上限 `MAX_CIPHERTEXT_LEN`=65535（明文 65519 + AEAD tag 16B），既有断言 `65535+4` 数学正确，65519 是明文上限非线材上限（crypto.rs:18/22、frames.rs 编码格式为证）。

**decision-needed：**

- [x] [Review][Patch] 会话级写操作未订阅写信号（决策①已裁决：现在补全）— `chat_delete_conversation`/`chat_new_conversation`（两条创建路径）补发 `conversation:created/deleted`；标题更新走既有 `conversation:title-updated` 事件补订阅；`data_import` 补发 `data:imported`（整库替换须触发重建）[commands/chat.rs、commands/data.rs + services/companion_snapshot.rs WRITE_SIGNAL_EVENTS]
- [x] [Review][Patch] 通知标记已读未订阅写信号（决策①已裁决：现在补全）— `notification_mark_read` 签名加 `app_handle`（Tauri 自动注入，前端 invoke 不变）并补发 `notification:read`，入 WRITE_SIGNAL_EVENTS [commands/notification.rs + services/companion_snapshot.rs]

**patch：**

- [x] [Review][Patch] 出站通道溢出静默丢帧无恢复（违反代码自述 AC3 不变量）：容量 512≈2.3 份全量快照（219 帧/份，注释按旧 60KB 算 175 已过期）；try_send 失败仅 debug 日志、连接不断不触发补齐，手机重组永久挂起 → 容量满即丢弃整个帧序列并 terminate 强制断连（走 OnConnect 补齐），同步修正容量推导注释 [services/companion_connection.rs:47-50, enqueue_outbound]
- [x] [Review][Patch] enter_session 出站分支 await send_frame 期间 terminate/ping/idle 超时均不处理，TCP 背压可致会话卡死数分钟且管理操作无法中断 → 发送期间 select! 并入 term_rx [services/companion_connection.rs:enter_session]
- [x] [Review][Patch] push_frames 逐帧加锁，推送中途会话槽被新连接替换 → 残缺分帧序列泄漏进新会话（envelope 无代次）→ 入队前捕获会话代次，代次变更即中止本序列 [services/companion_snapshot.rs:630-638]（实现方式：`enqueue_outbound` 改为整序列持锁入队，与会话槽替换互斥——同效更简）
- [x] [Review][Patch] apply_size_cap 诚实性缺口：核心域自身 >10MB 时四阶段走完无兜底（truncated 保持 false，或 cutoff 回退 generated_at 语义误导）→ 末尾强制 truncated=true + error 级日志，并修正 cutoff 回退 [services/companion_snapshot.rs:370-462]（cutoff 语义修正为「跨域取最大」+ 域清空取最后删除条目时间戳；阶段 A 增「保留最新 1 个会话」守卫使单会话超限可进阶段 B）
- [x] [Review][Patch] 截断循环每轮全量序列化快照 O(n²)，病态数据下引擎停摆数十秒（并放大 OnConnect 丢弃）→ 按域增量字节核算或设迭代上限 [services/companion_snapshot.rs:380-417]（实现：初始精确测量一次 + 逐条增量估算 + 末尾精确兜底）
- [x] [Review][Patch] 写信号清单漏 `task:tool-action`：任务分解 accept/keep_single 真实建任务并 emit 该事件（task_decomposition.rs:25/36），未入 WRITE_SIGNAL_EVENTS → 补入 [services/companion_snapshot.rs:646-669]
- [x] [Review][Patch] 「写事件→监听器→引擎」接线零回归保护（删除任一 emit/监听器，现有测试全绿）：llm:stream done 解析抽纯函数 + 三分支单测；WRITE_SIGNAL_EVENTS ↔ 发射端契约测试；补阶段 B（消息减半）方向测试（现状删掉 rows.reverse() 无断言失败）[services/companion_snapshot.rs:673-701 + tests/test_companion.rs]（契约测试以 include_str! 源码扫描命令层事件名字面量，豁免 llm:stream 特判；另含 TASK_CLASSIFIED_EVENT 常量一致性断言）
- [x] [Review][Patch] OnConnect 请求通道容量 8 + try_send 丢弃无重试：引擎忙于重建时第 9 次建连请求被丢，该连接期手机白屏 → 幂等合并（watch 通道或 pending 标志）[services/companion_snapshot.rs:521 + services/companion_connection.rs:request_snapshot_on_connect]（实现：mpsc→watch 通道，send 幂等合并）
- [x] [Review][Patch] 小项簇：① EngineEvent::Shutdown 不可达（engine 自持双端 sender）删除死代码 ② chat.rs:336 `let _ =` 吞 emit 错误，对齐 role/task 的 warn 模式 ③ task_toggle_complete 取消完成也发 task:completed（改发 task:updated）④ 分帧测试断言 65535+4 收紧为 65519 ⑤ 季度测试注入日期参数以捕获映射错误 [companion_snapshot.rs:497-501、commands/chat.rs、commands/task.rs:131-138、tests/test_companion.rs:1485]（① 改为通道关闭直接 break、变体删除；② 新增 emit_chat_event helper；③ 已改 task:updated 并从清单移除 task:completed；④ 复核后保持 65535+4——见上方偏差说明；⑤ quarter_range_at 注入日期 + 8 边界断言）
- [x] [Review][Patch] 规范与记录回写：裁决 5 分片值 60_000→48_000（原值漏算 base64 4/3 膨胀，数学不可行，实现已按规则七择 48_000）；Completion Notes「22 事件订阅」更正为 21（WRITE_SIGNAL_EVENTS 20 项 + llm:stream 1 项）[本文件 设计裁决表 + Completion Notes List]（本次评审又补 5 项订阅，最终 26 = 清单 25 项 + llm:stream 特判 1 项）

**defer：**

- [x] [Review][Defer] role:deleted 发全对象 vs task:deleted 发裸 id，同族事件 payload 形状不一致 — deferred，13.2 手机端消费这些事件时统一 [commands/role.rs:194, commands/task.rs:115]
- [x] [Review][Defer] dataCutoffAt 混合 RFC3339 与纯日期字符串比较（同日边界精度 <24h）— deferred，信息性字段，13.2 移动端展示语义确定时一并处理 [services/companion_snapshot.rs:360-365,423-444]
- [x] [Review][Defer] notify 通道 256 满丢写信号无补偿；OnConnect 排队在 Write 之后可致 STATE_DELTA 先于 SNAPSHOT 到达 — deferred，前者在截断 O(n²) 修复后几乎不可达，后者由 13.2 手机端帧处理语义收口 [services/companion_snapshot.rs:520,553-570]

**dismiss（2 条，不列入行动）：** ① 每会话 200 条消息上限未标 truncated——AC1 明确 200 条即快照口径（非尺寸截断），不成立；② 「测试全绿未独立复跑」——非代码发现，评审方正在独立复跑闭环（首轮因磁盘写满失败，清理后重跑）。

## Story Status: done

### AC Checklist

- [x] AC1 快照口径与内存持有（七域 + 记忆负向 + 不持久化）
- [x] AC2 debounce 触发与桌面零回归（services 零改动 + STATE_DELTA 推送 + 重连全量补齐）
- [x] AC3 建连即全量快照（含重连；在线变化主动推送）
- [x] AC4 10MB 上限与截断明示（元数据 + 核心域不缺失）
- [x] AC5 帧编码与日志纪律（camelCase + 明文不入日志）
- [x] AC6 测试覆盖（四类必备用例 + 分帧 roundtrip）

## 建议下一步：创建 13-2 故事（`bmad-create-story`）——手机端快照消费与 StateMerger

**开工提醒**：先读 Dev Notes 全部裁决（尤其裁决 1/2/3——分帧不 bump 版本、STATE_DELTA 全量、命令层补 emit）；工作区含 12.3/12.4 未提交产物，确认后开工。
