---
baseline_commit: 5ea024ceea29d7cf23a44a0dbbe0a5010d47cb21
---

# Story 13.2: 手机实时快照视图换装

Status: done

## Story

As a 手机用户,
I want 手机的仪表盘、任务、对话历史、简报、复盘和通知中心显示来自桌面的真实数据，并随推送即时更新,
So that 手机不再是我桌面数据的静态演示，而是活的远程视图。

## Acceptance Criteria

1. **SnapshotStore 帧驱动换装（AC1，UX-M2）**
   - Given `sync/SnapshotStore` 换装（`AppModelContainer` 唯一换装点，AppModelContainer.kt:41）
   - When 实现完成
   - Then SnapshotStore 由帧驱动：`SNAPSHOT` 全量替换、`STATE_DELTA` 经 StateMerger 合并（**全量替换语义**，见 Dev Notes 裁决）、`PING` 保活；对上层 ViewModel 保持既有取数接口，UI 层零改动
   - And 快照缓存落盘为单一版本化文件 + 元数据（含数据截止时间），用 Android Keystore 派生 AES 加密；**不引入 Room**（架构反模式）
2. **STATE_DELTA 即时刷新（AC2）**
   - Given 手机与桌面连接期间
   - When 桌面状态变化推送 `STATE_DELTA`
   - Then 仪表盘（角色卡/能量条/统计四宫格）、任务四象限、对话会话列表与历史消息、晨间简报、周复盘、通知中心未读数据即时刷新，无需手动下拉
   - And 呼吸动效/色温过渡等既有动效行为不变（UX-M1）
3. **重连全量替换（AC3）**
   - Given 手机断线后重连
   - When 收到全量 `SNAPSHOT`
   - Then 各屏以最新快照替换呈现；断线期间的变化补齐
   - And 快照元数据中的数据截止时间对 UI 可用（降级标注消费方在 Epic 14）
4. **10MB 截断明示（AC4，NFR-M3）**
   - Given 快照被 10MB 截断（`truncated=true`）
   - When 手机渲染
   - Then 会话/简报等截断域按 `dataCutoffAt` 明示，不以缺失数据冒充完整
5. **记忆页防呆（AC5，UX-M5）**
   - Given 记忆页数据通道
   - When 换装 SnapshotStore
   - Then 真实 SnapshotStore 中**不存在**记忆条目内容（mock 的 memories/memorySources 数据源被移除出快照通道）；记忆页置于"待接指令通道"状态（13.3 接 COMMAND 现查），不显示假数据
6. **零回归验证（AC6，UX-M3）**
   - Given 换装前后对比
   - When 逐屏截图比对（四 Tab 主界面 + 三个二级页 + 记忆页）
   - Then 视觉与交互零回归（数据内容由 mock 换为真实快照数据，布局/样式/动效零变化）
   - And `./gradlew :app:testDebugUnitTest` 全绿（StateMerger 合并逻辑、快照加解密、版本化文件读写、截断元数据）

## Tasks / Subtasks

- [x] T1 Kotlin 快照领域模型与分帧重组器（AC:1,3）
  - [x] 新建 `sync/SnapshotModels.kt`：`DesktopSnapshot` 七域 data class，字段名严格对齐桌面 `egosync-app/src-tauri/src/models/snapshot.rs`（camelCase，见 Dev Notes §2 完整 schema）+ `schemaVersion`（=1）校验
  - [x] 新建 `sync/ChunkReassembler.kt`：镜像桌面 `companion_snapshot.rs` 的 `ChunkEnvelope{seq,total,chunkBase64}` 重组（解析 `Frame.Snapshot/StateDelta.data` → 收齐 total 片 → seq 连续性断言 → base64 解码 → 拼接 → JSON 反序列化）；`seq=0` 到达即重开缓冲（容忍重连后的新序列）
  - [x] 单测：多片重组往返、残缺序列（缺片/seq 跳号）、base64/JSON 损坏显式报错
- [x] T2 SnapshotStore 换装为帧驱动实例类 + StateMerger（AC:1,2）
  - [x] `SnapshotStore` 由 `object` 改为实例类：内部 `MutableStateFlow<SnapshotState>`，暴露既有取数接口（roles/tasks/briefing/weeklyReview/notices/conversations/memoriesOf/activityMetrics…，字段名与签名不变）
  - [x] `StateMerger`：`applySnapshot(DesktopSnapshot)`——Snapshot 与 StateDelta **同一处理**（全量替换）；元数据（truncated/dataCutoffAt/truncatedDomains/generatedAt）随 state 暴露
  - [x] 移除全部 mock 数据（对照表见 Dev Notes §5）；演示性数据（对话流/建议/引导）迁往各自 ui 包私有文件，SnapshotStore.kt 内不再含任何假数据（供 AC5 防呆断言扫描）
- [x] T3 帧分发出口接入 RealConnectionClient（AC:1,2）
  - [x] `RealConnectionClient.startSessionLoop`（L499-534）的 `else -> Unit`（L518）分支改为将 SNAPSHOT/STATE_DELTA 帧交给注入的消费者；`ConnectionClient` 接口六成员签名**零改动**（UX-M2），帧消费者经构造注入（同 secrets/nsd/wsOpener/ioDispatcher 注入缝手法，保持 JVM 可测）
  - [x] 重组器为 per-session 实例：承载切换/重连时会话循环重建，残缺序列不跨会话（对齐桌面"整序列持锁入队"语义）
  - [x] FrameCodec 零改动（8 帧编解码 12.4 已完整）
- [x] T4 快照缓存落盘（AC:1,3,4）
  - [x] 新建 `sync/SnapshotCacheFile.kt`：单一版本化文件（filesDir，如 `snapshot_cache.bin`：版本头 + IV‖GCM 密文 + 元数据），AES-GCM 密钥由 Android Keystore 派生；**照抄 `connection/KeyStore.kt` PairingSecrets 先例**（alias 另立如 `egosync_companion_snapshot_wrap`，原子写 tmp+rename）
  - [x] 收到新快照后异步落盘；启动时若在线等待 SNAPSHOT、离线则读缓存并置 `Offline(snapshotAvailable=true, dataAsOf=…)`
  - [x] 单测：加解密往返、版本拒绝（未知版本显式失败不静默）、损坏文件自愈（删除重建空态）
- [x] T5 14 处生产直读点收敛到 container.snapshotStore（AC:1,2；清单见 Dev Notes §4）
  - [x] AppModelContainer.kt:41 换装为帧驱动实例并完成帧分发接线
  - [x] ViewModel 默认参数、Screen 组合函数直读、InAppNotificationAdapter 种子、TasksViewModel companion `nextTaskId`（L203）逐一改经实例取数；Preview/`sample()` 改用独立样例文件
- [x] T6 记忆页防呆（AC:5）
  - [x] `MemoryViewModel`（L49-50）与 `MemoryScreen`（L173 memorySources 直读）改"待接指令通道"态：列表区显示占位说明（如"记忆需实时查询桌面引擎，将在指令通道接入后可用"），不渲染任何假条目；`MemoryStoreTest` 随契约重构
- [x] T7 元数据接入与截断明示（AC:3,4）
  - [x] `ConnectionStateMachine`/`RealConnectionClient` 各处硬编码 `Offline(false, null)`（ConnectionStateMachine.kt:112 等）改由快照缓存存在性 + `dataCutoffAt` 填真实值
  - [x] `truncated=true` 时受影响域（conversations/briefings/weeklyReviews 对应屏）显示"数据截止至 {dataCutoffAt}"轻标注；`dataCutoffAt` 混合格式（RFC3339 或 YYYY-MM-DD）按"有日期显示日期、有时分显示日期+时分"统一格式化为中文文案（13.1 deferred 收口）
- [x] T8 已知崩溃守卫（AC:2；13.1 评审 deferred 项）
  - [x] `ChatViewModel.sendMessage` `butlerReplies` 空列表整数除零（ChatViewModel.kt:74-82）补空守卫
  - [x] `ActivityWindow.Custom` 相对天数跨午夜前移改携带绝对日期（DashboardScreen 相关）
- [x] T9 测试补齐（AC:6）
  - [x] StateMerger 全量替换语义（镜像桌面 `write_signal_pushes_state_delta`）、重连全量替换（镜像 `reconnect_receives_latest_snapshot_after_gap`）
  - [x] STATE_DELTA 先于 SNAPSHOT 到达的容忍性测试（两者同载荷全量，任意先到皆正确）
  - [x] 快照防呆负向断言：SnapshotStore/快照模型中不存在记忆条目内容字段（对应桌面 `snapshot_scope_completeness_and_memory_exclusion`）
  - [x] `ActivityMetricsTest` 随真实口径重构（per-role 派生统计，见 Dev Notes §5 决策）
- [x] T10 验证与零回归（AC:6）
  - [x] `./gradlew :app:testDebugUnitTest` + `./gradlew :app:assembleDebug` 全绿
  - [x] 逐屏截图比对（对照 `companion-android/README.md` 页面地图）；无设备环境则如实声明验证边界，禁止声称已验证（惯例同 12.4）
  - [x] 桌面侧零改动确认：`git diff` 不含 egosync-app/src-tauri 与 crates/companion-proto

### Review Findings

> 三路评审（盲审 20 条 / 边界 11 条 / 验收审计 8 条）分诊结果——组 1（sync 核心 + 帧通道）；UI 层（组 2）另行评审。去重后：decision 2、patch 16、dismiss 10。

- [x] [Review][Decision] ActivityWindow 时间窗契约失效：`activityMetrics(window)` 签名承诺时间窗筛选，但快照 metrics 域仅聚合数（`snapshot.rs` SnapshotMetrics 无时间明细），window 参数被 `@Suppress("UNUSED_PARAMETER")` 完全忽略；`Recent`（相对天数）与 `Custom`（绝对日期）口径不一；`ActivityWindowTest` 用 `assertEquals(all, recent)` 把"不筛选"锁死为契约 — **已决策 C（boss，2026-08-29）**：另立 story 扩展桌面 metrics schema 后真正实现筛选；本 story 保留现状，SnapshotStore 加注释标注窗口待桌面 schema 支持（后续 patch）
- [x] [Review][Decision] 通知已读状态随 STATE_DELTA 全量替换重置（`SnapshotStore.kt:406` 注释自陈"已读状态为本地暂存，STATE_DELTA 全量替换会重置"），用户可见回退 — **已决策 A（boss，2026-08-29）**：本 story 引入本地已读持久化（手机记录已读通知 ID，取数时过滤），转 patch 执行
- [x] [Review][Patch] 本地已读持久化（决策 A 落地）：手机记录已读通知 ID 并在通知取数处过滤，STATE_DELTA 全量替换不再重置已读状态 [sync/SnapshotStore.kt:406]
- [x] [Review][Patch] ActivityWindow 窗口待实现标注（决策 C 落地）：SnapshotStore/Mapper 注释标注 window 参数待桌面 metrics schema 扩展；桌面 schema 扩展另立 story（待创建） [sync/SnapshotStore.kt]
- [x] [Review][Patch] loadCached 无守卫竞态：缓存读协程迟到时无条件 applySnapshot，旧缓存覆盖刚到达的在线 SNAPSHOT [sync/SnapshotStore.kt:370]
- [x] [Review][Patch] loadCached 异常覆盖不全：exists()/readBytes() 之间文件被 clear() 删除或磁盘 IO 错误抛 IOException 逃逸协程 → 启动崩溃；decodeToString 字符集异常同未被捕获 [sync/SnapshotStore.kt:370]
- [x] [Review][Patch] onFrame 未捕获 decodeToString 的 CharacterCodingException，base64 成功但字节非 UTF-8 时杀死整条会话（与"载荷级错误不杀会话"注释矛盾）[sync/SnapshotFrameHandler.kt:39]
- [x] [Review][Patch] unpair 清除竞态：clear() 与在途 save 协程/在飞 SNAPSHOT 帧无顺序保证，解配后缓存或内存数据复活 [sync/SnapshotStore.kt:439]
- [x] [Review][Patch] 并发 save 互踩：固定 tmp 文件名交叉写字节损坏 + obtainKey 无同步密钥别名互覆 + writeBytes 失败 tmp 残留 [sync/SnapshotCacheFile.kt:47]
- [x] [Review][Patch] 10MB 快照 base64 解码/拼接/org.json 解析全在主线程（scope=Main.immediate 已证实 AppModelContainer.kt:35），掉帧/ANR [connection/RealConnectionClient.kt:516]
- [x] [Review][Patch] 冷启动缓存加载完成后 Offline ConnectionState 不刷新（offlineInfo 为 pull 式求值，未纳入 combine 流源），降级标注滞后至首个连接事件 [connection/RealConnectionClient.kt:102]
- [x] [Review][Patch] unpair 后通知中心残留旧桌面未读通知：collect 仅 loaded=true 分支刷新，clear 后不清空 [notify/InAppNotificationAdapter.kt:29]
- [x] [Review][Patch] 负数 seq/total（如 seq=-3,total=-2）绕过全部守卫，静默产出空载荷掩盖根因 [sync/ChunkReassembler.kt:36]
- [x] [Review][Patch] clear() 忽略 File.delete() 返回值，解配删缓存失败完全静默 [sync/SnapshotCacheFile.kt:85]
- [x] [Review][Patch] 时间格式化硬编码 ZoneOffset.UTC，"X月X日 HH:mm"显示 UTC 钟面（东八区偏 8 小时）[sync/SnapshotMapper.kt:286]
- [x] [Review][Patch] 空洞测试补强：SnapshotFrameHandlerTest"重组失败重置缓冲"实际走 seq=0 重开分支未触发 ChunkReassemblyException，该 catch 分支零覆盖；SnapshotParserTest"空域与可空字段合法"未构造字段缺失/null 用例 [test/.../sync/SnapshotFrameHandlerTest.kt]
- [x] [Review][Patch] formatDataCutoff 定义后无调用点：dataAsOf 直出原始 ISO 串至降级横幅（AC4 数据截止时间可用性）[sync/SnapshotMapper.kt:294]
- [x] [Review][Patch] activityMetrics/memoryCount 签名 Int→Int? 属 T2"签名不变"的受控例外（§5 裁决要求），需在本 story 显式登记 [sync/SnapshotStore.kt:288]
- [x] [Review][Patch] AC5 防呆反射扫描仅覆盖记忆内容，未覆盖 §2 负向契约的 personality_prompt/skills_config [test/.../sync/SnapshotParserTest.kt]
- [x] [Review][Patch] truncated=true 且 dataCutoffAt=null 时截断明示标签静默缺失（AC4，落点在组 2 文件，随本 story 一并修）[ui/AppNavHost.kt:394]

> **组 2（UI 层）三路评审分诊**（盲审 12 条 / 边界 10 条 / 验收审计 7 条，去重后 patch 14、dismiss 6）：mock 迁移完整性、T5 十四处直读点收敛、Int? 适配（null→"—"）三项核验通过；AC2 核心链路订阅达成（3 处盲区待修）、AC4 标注落地（缺域过滤待修）、AC5 达成、AC6 声明与 diff 相符。核验通过项：mock 数据全部迁 `ChatDemoData`/`OnboardingDemoData`/`PreviewSamples` 且零悬空引用；无 `SnapshotStore.xxx` 静态残留；`memoryCount?.toString() ?: "—"` 三处正确无 `!!`/0 冒充。

- [x] [Review][Patch] TasksViewModel collect 缺快照同一性守卫：任何 state 重发执行 `store.tasks + seedTask`，已分类 seedTask 回滚 Q3、classifyingIds 悬空、nextTaskId 回退（ChatViewModel 有 `state.snapshot !== lastSnapshot` 守卫，Tasks 无）[ui/tasks/TasksViewModel.kt:101]
- [x] [Review][Patch] 五个 VM collect 缺 loaded=false else 分支：unpair/密钥失效自愈（store.clear 不导航）后主界面残留陈旧 roles/history/tasks/planStates [ui/dashboard/DashboardViewModel.kt:60 等五处同模式]
- [x] [Review][Patch] onSnapshotReplaced 与进行中流式回复零协调：不取消 streamJob，打字机空转、responding 滞留、回复静默丢失 [ui/chat/ChatViewModel.kt:141]
- [x] [Review][Patch] onSnapshotReplaced 回落换会话不清 actionCards/decomposition/roleProposal/traceByMessageId，与 selectConversation 清卡片不变量冲突 [ui/chat/ChatViewModel.kt:141]
- [x] [Review][Patch] 幽灵角色：当前查看角色被快照删除后 activeRoleId 不回退、createConversationIn 为已删角色伪造孤儿会话 [ui/chat/ChatViewModel.kt:141]
- [x] [Review][Patch] delegationKeywords/delegationReplies 写死 mock 角色键（role-pm 等），真实快照 UUID 角色委派路由失效（名字退"角色"、气泡挂不存在 senderRoleId）[ui/chat/ChatViewModel.kt:319]
- [x] [Review][Patch] Settings 冷启动空角色：setProactivityLevel 写入 levels[""]，首个 applyRoles 重建后用户选择静默回退 MODERATE [ui/settings/SettingsViewModel.kt:33]
- [x] [Review][Patch] Briefing/WeeklyReview 空数据冒充真实页：快照未加载/无简报时 EMPTY_BRIEFING/EMPTY_REVIEW 渲染空问候卡/空成绩单冒充"已生成"（NFR-M3）；行动点分区头对恒空 actionPoints 无条件渲染 [ui/briefing/BriefingScreen.kt:148]
- [x] [Review][Patch] truncatedCutoffLabel 缺域过滤：BriefingRoute/WeeklyReviewRoute 未传 domain（ChatRoute 传了 "conversations"），仅 conversations 被截断时简报/复盘屏误报"被截断" [ui/AppNavHost.kt:364]
- [x] [Review][Patch] TasksRoute roles 组合期直读：roles-only 的 STATE_DELTA（角色改名/新角色）不触发归属筛选 chips 刷新 [ui/AppNavHost.kt:301]
- [x] [Review][Patch] 仪表盘全局"记忆数量"指标经 uiState getter 拉取：仅 memoryCount 变化的快照不触发重组 [ui/dashboard/DashboardViewModel.kt:40]
- [x] [Review][Patch] WeeklyReviewViewModel planStates 构造时定格：快照未加载时构造或新增角色后，updateItem/addItem/removeItem 对缺失 roleId 全为 no-op，用户键入被静默拒绝 [ui/review/WeeklyReviewViewModel.kt:91]
- [x] [Review][Patch] MemoryScreen 生产死代码残留 unused import MemorySourceMessage（MemoryCard 等组件保留待 13.3 复用，仅清 import）[ui/memory/MemoryScreen.kt]
- [x] [Review][Patch] AC2 核心测试补齐：二次 applySnapshot 刷新/冷启动空快照兜底/同对象跳过/TasksViewModel 守卫均无用例；ChatViewModelTest 断言弱化为仅比对 text，恢复 senderRoleId/fromButler 等关键字段校验 [test/.../ui/chat/ChatViewModelTest.kt]

## Dev Notes

### 1. 冻结契约（不得触碰）

- **本 story 是纯 Android story**：不改桌面 Rust 代码、不改 `crates/companion-proto`（13.1 硬边界）。协议层 8 帧类型/schema.json/`PROTOCOL_VERSION=1` 冻结。
- **帧 wire 形态**：`[u32 BE 长度前缀][Noise 密文]`，内层 JSON `{"type":"<snake_case帧名>", …payload}`，payload camelCase + `deny_unknown_fields`。单帧密文 ≤65535、明文 ≤65519。Kotlin 侧 `FrameCodec.kt` 已完整实现，**零改动**。
- **分帧 envelope（app 层，塞在 `Frame.Snapshot/StateDelta.data` 字符串内）**：
  ```json
  {"seq": 0, "total": 3, "chunkBase64": "..."}
  ```
  桌面按 **48,000 字节明文切片**后 base64（勿改此常量；60,000×4/3 会超 65519 上限——13.1 评审血泪）。接收方只需重组，与切片大小无关。
- **STATE_DELTA = 全量快照**（13.1 裁决 2）：与 SNAPSHOT 载荷完全相同，仅触发时机不同（建连 vs 写信号 debounce 2000ms 后）。**StateMerger 实现为全量替换即可，严禁自作主张做字段级 diff**。域级 diff 是明确 deferred 项。
- **`schemaVersion`（=1）≠ `protocolVersion`（=1）**：快照结构演进走前者，消费方须校验，未知版本显式失败。
- **建连即全量 SNAPSHOT**：进入会话后随时可能开始收分帧序列；`STATE_DELTA` 可能先于 `SNAPSHOT` 到达（13.1 评审 deferred，由本 story 收口——同载荷全量替换，天然容忍）。
- **PING 保活 12.4 已实现**（手机 30s 发、RealConnectionClient.kt:502-505，桌面 120s 空闲超时）——AC1 中"PING 保活"为既有能力，本 story 无需改动，勿重复实现。
- **日志纪律（NFR-M7）**：帧明文/快照 JSON/chunk 内容永不入日志；只记帧类型判别式/域计数/字节数，tag 沿用 `Companion/` 前缀。

### 2. DesktopSnapshot 完整 schema（`models/snapshot.rs`，消费目标）

顶层（camelCase）：`schemaVersion: Int`、`generatedAt: String`（ISO 8601 UTC）、`dataCutoffAt: String?`、`truncated: Boolean`、`truncatedDomains: List<String>`（取值 conversations/briefings/weeklyReviews）+ 七域：

| 域 | 字段 |
|---|---|
| `roles: [SnapshotRole]` | id, name, icon, color, goal, status, energy: Int, proactivityLevel |
| `tasks: [SnapshotTask]` | id, ownerType, roleId?, title, deadline?, quadrant, isBigRock, isCompleted, protectionStatus, roleName?, roleColor? |
| `dashboard.statuses: [DashboardStatus]` | roleId, roleName, roleIcon, roleColor, energy: Int, pendingTasksCount: Long, lastActiveAt?, hasUrgent: Boolean |
| `dashboard.metrics` | taskCount, memoryCount, conversationCount, pendingTaskCount（均 Long）, generatedAt |
| `conversations: [SnapshotConversation]` | id, roleId?, title, updatedAt, messages[]（每会话最近 200 条：id, role, content, thinkingContent, isComplete, createdAt） |
| `briefings: [SnapshotBriefing]` | id, content, date（仅本季度） |
| `weeklyReviews: [SnapshotWeeklyReview]` | id, weekStart, weekEnd, summary, **energyTrends: String（JSON：`{"<roleId>": {"energy": Int, "energyUpdatedAt": String?}, …}`，各角色当前能量快照——非时间序列，解析手法镜像桌面 `egosync-app/src/hooks/useWeeklyReview.ts`，非法 JSON 降级 null）**, bigrockStatus: String（JSON：`[{id, title, isCompleted, completedAt?, roleName?}]`）, newMemoriesCount: Long（仅本季度） |
| `notifications: [SnapshotNotification]` | id, roleId, level, content, createdAt, roleName, roleIcon, roleColor（仅未读） |

**记忆内容与 `personality_prompt`/`skills_config` 永不在快照中**（桌面有负向断言测试锁定，Android 侧做镜像防呆）。

### 3. 换装点与装配缝

- **唯一换装点**：`AppModelContainer.kt:41` `val snapshotStore = SnapshotStore`（注释已预留）。容器手工 DI、进程级单例（L94-100 双检锁）、`scope = CoroutineScope(SupervisorJob()+Dispatchers.Main.immediate)`（L31）。
- **`ConnectionClient` 六成员签名零改动**（state/paired/setDebugMode/completePairing/unpair + 既有成员）；帧消费者经 `RealConnectionClient` 构造注入（主构造 L53-80 已有 secrets/nsd/wsOpener/ioDispatcher 可注入先例）。
- `AppModelContainer.init`（L52-65）的速记 flush mock 逻辑**本 story 不动**（真实"COMMAND 确认后删队"语义属 13.3/14.1）。

### 4. 14 处生产直读点（必须收敛到 container.snapshotStore）

| 位置 | 现状 |
|---|---|
| `ui/dashboard/DashboardViewModel.kt:30` | `activityMetrics` getter 直调 object 静态方法 |
| `ui/tasks/TasksViewModel.kt:21` | UiState 默认参数；L203 `nextTaskId` 挂 object 推导 |
| `ui/chat/ChatViewModel.kt:28,35` | 默认参数（`ChatViewModel(private val store: SnapshotStore = SnapshotStore)` L78——改注入实例） |
| `ui/review/WeeklyReviewViewModel.kt:29` | 默认参数；L111 已走 container |
| `ui/settings/SettingsViewModel.kt:25-30` | 默认参数 |
| `notify/InAppNotificationAdapter.kt:15` | `_notices` 种子直读 object（改由快照 notifications 域驱动） |
| `ui/onboarding/OnboardingViewModel.kt:48` | 直读 onboardingPlaceholders（引导流 mock，迁私有文件） |
| `ui/briefing/BriefingScreen.kt:51` | 组合函数体内直读（无 ViewModel） |
| `ui/review/WeeklyReviewScreen.kt:75,604-625` | 直读 weeklyReview + energyTrend |
| `ui/memory/MemoryScreen.kt:173` | 直读 memorySources（按 T6 处置） |

Preview/`sample()` 直读（ChatScreen.kt:973-1053、TasksScreen.kt:905-940、DashboardScreen.kt:813、NotificationCenterScreen.kt:262、OnboardingScreen.kt:312-336、SettingsScreen.kt:49）：改引用独立样例文件（如 `ui/*/PreviewSamples.kt`），不参与快照通道。

### 5. mock 数据对照表（关键约束 #6）与处置决策

| SnapshotStore mock | 处置 |
|---|---|
| roles / tasks | 快照 roles / tasks 域 |
| briefing | briefings 最新一条（按 date） |
| weeklyReview | weeklyReviews 最新一条（按 weekStart） |
| notices | notifications 域（未读），InAppNotificationAdapter 改快照驱动 |
| initialChat / butlerHistoryChat / roleChatSeeds / roleConversationTitles | conversations 域派生（会话列表+历史消息） |
| activityLedger | dashboard.statuses + metrics 派生（见下） |
| memories / memorySources | **移除**（13.3 指令通道现查；T6 防呆） |
| bigRockSuggestions | **移出快照通道**：复盘页建议区置空态"待接指令通道"（生成性内容不在快照口径，同记忆页手法） |
| energyTrend（List&lt;Int&gt;）/ energyTrendDays（一~日） | **语义不匹配防呆**：原型的 7 柱周趋势图无真实数据源（桌面 energyTrends 为各角色**当前**能量快照 JSON，非时间序列；桌面 6.5 亦如此渲染）。处置：解析真实 energyTrends（镜像 useWeeklyReview 的解析与 null 降级）；趋势图区置"能量趋势需历史数据，待指令通道接入"诚实占位（当前能量已由仪表盘角色卡实时呈现），布局/样式不动仅内容变化——与 AC5 记忆页占位同一先例。禁止用假 7 点数据或全局数字冒充趋势 |
| butlerReplies / delegationKeywords / delegationReplies / decompositionProposal / executionTrace / roleProposal / toolExecutionStages / onboardingGreeting / onboardingPlaceholders / onboardingReplies | 演示对话流 mock：迁至各 ui 包私有 demo 文件（发送行为 13.3 换 COMMAND/STREAM_TOKEN，本 story 保留既有交互不变） |

**派生统计决策（必须遵守）**：Android 角色卡统计四宫格（任务/记忆/会话/待办）中，任务数、待办数、会话数可从快照派生（tasks/conversations 按 roleId 聚合）；**记忆数快照不可得**（桌面仅有全局 metrics.memoryCount，无 per-role 拆分）→ 记忆格显示"—"（诚实降级，NFR-M3；13.3 指令通道可现查）。**禁止**为凑数显示全局 memoryCount 或假数字。`activityMetrics(scopeId, window)` 的时间窗筛选：快照口径为全时间窗，13.2 期间筛选项呈现快照全量口径（FR-38 移动端筛选交互走后续 schemaVersion 演进，勿自行扩 schema）。

**过渡态已知限制（写进完成笔记，勿"修复"）**：13.2 期间本地写操作（新建任务/勾选/发送对话）仍是演示行为，STATE_DELTA 全量替换会覆盖本地暂存——这是 13.3 指令通道换装前的预期状态，不是缺陷。

### 6. 落盘模式参照（勿引入新依赖）

`connection/KeyStore.kt` 的 `PairingSecrets` 是完整先例：Android Keystore AES-GCM 包裹 + filesDir 单文件（IV‖GCM 密文）+ `atomicWrite`（tmp+rename，L68-75）。快照版本化文件复制该模式，别名另立；密钥派生逻辑抽成可 JVM 测试的接口缝（同 SecretsProvider 手法）。依赖白名单见 `gradle/libs.versions.toml`（**无 Room/Hilt/全局状态框架**；注意 README 依赖段"无 OkHttp"与 12.4 实际增项矛盾，以 toml 为准）。

### 7. 测试基础设施

JUnit4 + kotlinx-coroutines-test（runTest/StandardTestDispatcher/advanceTimeBy），**无 Turbine/mockito**——mock 手法为手写 Fake（FakeSharedPreferences/FakeSecrets/FakeNsd/FakeWebSocket，见 RealConnectionClientOrchestrationTest.kt:39-122）。重组逻辑的现成参照伪码：桌面 `tests/test_companion.rs` 的 `phone_receive_snapshot`（:1042-1084）——跳过非目标帧、按 total 收齐、校验无漂移、重组。`isReturnDefaultValues = true` 已开。

### 8. 前序 story 情报（13.1/12.4 教训，直接适用）

1. **分片数学上限**：任何 Kotlin 侧自行分帧（如回传）必须实测 48,000×4/3+envelope < 65519，勿照搬示例值。
2. **状态机/重组抽纯函数 + 注入测试**，不依赖真网络；真网络路径留 androidTest/手动冒烟并如实记录。
3. **双承载切换**：单会话槽替换语义保证旧会话终止；重组器 per-session 实例防残缺序列跨会话（12.4 P5 + 13.1 P3 整改先例）。
4. **空列表/空串守卫**：真实快照可能为空域——所有取默认值/除法/last() 处必须守卫（T8）。
5. **截图比对无设备时如实声明验证边界**，禁止声称已验证（AGENTS.md 规则十二；12.4 惯例）。
6. 13.1 留给本 story 的收口项：`dataCutoffAt` 混合格式展示语义（T7）、STATE_DELTA 先于 SNAPSHOT（§1）、`role:deleted` 事件 payload 形状统一（属 13.3 指令通道范围，本 story 不碰）、snapshotstore-vm-wiring 空列表守卫（T8）。

### 9. UX-DR 契约（epics.md:3038-3042 原文要点）

- **UX-M1 UI 零重做**：只允许换装真实实现，禁止重新设计样式与交互。
- **UX-M2 装配缝换装**：`AppModelContainer` 唯一换装点，UI 层零改动。
- **UX-M3 视觉零回归是验收硬条件**：对照 `companion-android/README.md` 页面地图逐屏比对。
- **UX-M5 记忆页数据通道防呆**：记忆页严禁接入真实快照，必须走 COMMAND 现查现显（13.3）；降级态记忆入口置灰并说明原因。
- NFR-M3（诚实代价：明示不掩盖）/ M4（桌面零回归）/ M5（桌面唯一事实源，手机仅加密缓存）/ M7（基线规范全量继承，帧明文永不入日志）。

### Project Structure Notes

- 新增文件全部落 `companion-android/app/src/main/java/com/egosync/companion/sync/`（快照模型/重组器/StateMerger/缓存文件）与 `connection/`（帧消费者注入）；测试同构落 `app/src/test/java/com/egosync/companion/{sync,connection}/`。
- 桌面侧（egosync-app/src-tauri、crates/）**零改动**——验收时以 git diff 佐证。
- 既有 `sync/QuickNoteQueue.kt`、`notify/InAppNotificationAdapter.kt` 接口保持，仅换数据来源。

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story-13.2（L3356-3394）]
- [Source: _bmad-output/planning-artifacts/epics.md#关键实现约束（L19-31）]
- [Source: _bmad-output/planning-artifacts/epics.md#移动伴侣增量 NFR/UX-DR（L3008-3042）]
- [Source: _bmad-output/implementation-artifacts/13-1-desktop-snapshot-engine-and-state-push.md（裁决表/Dev Agent Record/评审整改）]
- [Source: _bmad-output/implementation-artifacts/12-4-android-scan-pairing-and-tri-state-connection.md（连接实现/教训）]
- [Source: _bmad-output/implementation-artifacts/sprint-plan-2026-08-27-*.md（13.2 交付边界 L94、验收补充 L144-146）]
- [Source: egosync-app/src-tauri/src/models/snapshot.rs（快照 schema 事实源）]
- [Source: egosync-app/src-tauri/src/services/companion_snapshot.rs（ChunkEnvelope/reassemble 蓝本）]
- [Source: egosync-app/src-tauri/tests/test_companion.rs（行为契约基线，phone_receive_snapshot :1042）]
- [Source: companion-android/app/src/main/java/com/egosync/companion/（AppModelContainer.kt:41 装配缝、SnapshotStore.kt:312、RealConnectionClient.kt:499-534、KeyStore.kt 落盘先例）]

## Dev Agent Record

### Open Questions（待用户确认，不影响开工）

1. **能量趋势图处置**：原型为周一~周日 7 柱趋势，桌面仅提供各角色当前能量快照（无历史序列）。本 story 默认方案＝趋势图区置诚实占位（同记忆页手法）。备选＝改渲染各角色当前能量柱（UI 适配，X 轴由星期改角色名——属布局微调，需复核 UX-M3 容差）。如选备选，开工前告知。
2. **角色卡记忆格（per-role memoryCount 不可得）**：默认显示"—"。如需立即有数，须桌面 schemaVersion=2 增 per-role memoryCount——违反本 story 纯 Android 边界，不推荐。

> **两项 Open Questions 均按默认方案落地**：能量趋势图诚实空态占位（「暂无逐日能量数据（当前快照仅含角色当前能量）」）；角色卡/统计卡记忆格显示「—」。

### Agent Model Used

GLM-5.3（Amelia · DS 工作流）

### Debug Log References

- `./gradlew :app:testDebugUnitTest` → BUILD SUCCESSFUL，146 tests / 0 failed
- `./gradlew :app:assembleDebug` → BUILD SUCCESSFUL（app-debug.apk 40.6MB）
- 桌面侧零改动佐证：`git status egosync-app/ crates/` → 0 文件

### Completion Notes List

1. **T1**：`SnapshotModels.kt`（DesktopSnapshot 七域 + schemaVersion 校验）与 `ChunkReassembler.kt`（乱序容忍/收齐连续性断言/seq=0 重开缓冲/显式异常）完成；ChunkReassemblerTest 11 用例绿。
2. **T2**：SnapshotStore `object` → 实例类（StateMerger 全量替换 + 缓存异步落盘 + 取数接口签名不变）；全部 mock 数据移出，演示数据迁 `ui/chat/ChatDemoData.kt`、`ui/onboarding/OnboardingDemoData.kt`、`ui/PreviewSamples.kt`（Preview/sample 专用，不参与快照通道）。MemoryStoreTest 反射扫描锁定模型无记忆内容字段（AC5 防呆）。
3. **T3**：`FrameConsumer` 注入缝 + `SnapshotFrameHandler`（每会话实例；重组/解析失败重置不杀会话）；RealConnectionClient SNAPSHOT/STATE_DELTA 分发 + offlineInfo 注入，ConnectionClient 六成员签名零改动。
4. **T4**：`SnapshotCacheFile`（4B 版本头 + IV‖GCM；未知版本显式 SnapshotCacheException、损坏自愈删除、原子写）+ `KeystoreSnapshotCipher`（alias `egosync_companion_snapshot_wrap`，照抄 PairingSecrets）。JVM 假密文 7 用例绿。
5. **T5**：14 处生产直读点全部收敛经 `container.snapshotStore` 实例；SettingsViewModel 空角色守卫（冷启动 `roles.first().id` 崩溃修复）；TasksViewModel nextTaskId 换实例推导。
6. **T6**：记忆页「记忆通道尚未接通：内容不随快照下发…」占位态 + 来源展开占位；MemoryStoreTest 重写为 AC5 契约（memoriesOf 恒空，即便快照加载）。
7. **T7**：DEGRADED/离线态 offlineInfo 接入（缓存存在性 + dataAsOf）；截断明示落地三处——ChatScreen（conversations 域）、BriefingScreen、WeeklyReviewScreen（含 dataCutoffAt 混合格式 formatDataCutoff 统一中文文案）；标注仅在 truncated 且对应域被截断时出现。
8. **T8**：butlerReplies 空表除零守卫（runNormalReply/runDelegation 双路径）；ActivityWindow.Custom 改携带绝对日期（oldestDate/newestDate），跨午夜不再漂移。
9. **T9**：StateMergerTest（全量替换/重连最新胜出/先到皆正确）、SnapshotParserTest（schema 校验/防呆反射扫描）、SnapshotFrameHandlerTest（SNAPSHOT=STATE_DELTA 同路/重组失败重置/解析失败丢弃）、SnapshotCacheFileTest（往返/未知版本显式失败/自愈/clear/原子写）、ActivityMetricsTest（快照全量口径 + 记忆数 null）、ChatViewModelTest（快照夹具种子重构，六场景语义保留）。
10. **T10**：见 Debug Log。**验证边界声明（规则十二）**：逐屏截图比对未执行——无设备/模拟器环境；以 `:app:assembleDebug` 成功 + Preview 样例数据（内容对齐原 mock，视觉基准不丢）+ 既有布局代码零改动（UX-M1，仅数据源换装）作为代理验证。
11. **评审受控例外登记（P14）**：`activityMetrics` 返回 `Map<MetricType, Int?>`、`RoleCard.memoryCount: Int?` 相对 mock 版为 `Int → Int?` 破坏性变更——系 §5 裁决「记忆数快照不可得 → null（'—'）」的直接要求，T2「签名不变」就此两点让位（上游 ViewModel 已同步适配）；另 `ActivityWindow` 时间窗参数冻结不生效，真正筛选待桌面 schema 扩展（评审决策 C，另立 story）。
11. **过渡态已知限制（§5 明示，勿当缺陷）**：本地写操作（新建任务/勾选/发消息）为演示行为，STATE_DELTA 全量替换会覆盖本地暂存；13.3 指令通道收口。
12. **能量趋势图**（Open Question 1 默认方案）：空数据时诚实占位文案；EnergyTrendChart 图表本体保留（Preview 样例仍渲染）。
13. **评审受控例外登记（组 2 P1/P6）**：`TasksViewModel` 构造签名 `AppModelContainer → SnapshotStore`（与 ChatViewModel 同款注入缝，换取 JVM 单测可构造，AppNavHost 同步适配）；委派第二段反馈对未命中演示表的 UUID 角色改用角色名模板（`delegationReplies` 仅对同名演示 id 生效，指令通道 13.3 收口后由真实回执取代）。
14. **测试夹具注意**：`store.roles` 门面由 `SnapshotMapper.toRoleCards` 从 `snapshot.dashboard.statuses` 派生（非 `snapshot.roles`）——模拟角色增删须同步改 dashboard.statuses；VM 的 `lastSnapshot` 基线在协程体首跑时捕获，测试需先 `runCurrent()` 启动 collect 再触发快照变更。
13. **修复既有测试随机性**：ChunkReassemblerTest「多片乱序」用例曾用 `shuffled()`，seq=0 落尾时与「新序列起点重开缓冲」语义歧义（1/5 概率随机失败）——固定 seq=0 前置、其余乱序，消除随机性并注释原因。
14. **StateMerger.reset() + SnapshotStore.clear()**：unpair 配套清空内存态与快照缓存（信任锚清除外延）。

### File List

**新增（main）**
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotModels.kt`
- `companion-android/app/src/main/java/com/egosync/companion/sync/ChunkReassembler.kt`
- `companion-android/app/src/main/java/com/egosync/companion/sync/StateMerger.kt`
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotMapper.kt`
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotCacheFile.kt`
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotFrameHandler.kt`
- `companion-android/app/src/main/java/com/egosync/companion/connection/FrameConsumer.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatDemoData.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingDemoData.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/PreviewSamples.kt`

**新增（test）**
- `companion-android/app/src/test/java/com/egosync/companion/sync/ChunkReassemblerTest.kt`
- `companion-android/app/src/test/java/com/egosync/companion/sync/SnapshotParserTest.kt`
- `companion-android/app/src/test/java/com/egosync/companion/sync/StateMergerTest.kt`
- `companion-android/app/src/test/java/com/egosync/companion/sync/SnapshotCacheFileTest.kt`
- `companion-android/app/src/test/java/com/egosync/companion/sync/SnapshotFrameHandlerTest.kt`

**修改（main）**
- `companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt`（唯一换装点接线 + unpair 清缓存）
- `companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionClient.kt`（OfflineSnapshotInfo 数据类）
- `companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt`（帧分发 + offlineInfo 注入）
- `companion-android/app/src/main/java/com/egosync/companion/notify/InAppNotificationAdapter.kt`（快照驱动）
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt`（实例类换装）
- `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt`（Route 取数 + 截断标注）
- `companion-android/app/src/main/java/com/egosync/companion/ui/briefing/BriefingScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardViewModel.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/memory/MemoryScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/memory/MemoryViewModel.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/notify/NotificationCenterScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/onboarding/OnboardingViewModel.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewViewModel.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsViewModel.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt`
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt`

**修改（test）**
- `companion-android/app/src/test/java/com/egosync/companion/sync/ActivityMetricsTest.kt`（快照全量口径重写）
- `companion-android/app/src/test/java/com/egosync/companion/sync/MemoryStoreTest.kt`（AC5 契约重写）
- `companion-android/app/src/test/java/com/egosync/companion/ui/chat/ChatViewModelTest.kt`（快照夹具种子重构）

**桌面侧**：零改动（egosync-app/ 与 crates/ 无 diff，git status 佐证）

### Change Log

| 日期 | 变更 | 说明 |
|---|---|---|
| 2026-08-29 | Story 13.2 实现完成 | T1-T10 全部落地；146 单测全绿 + assembleDebug 成功；Status → review |
| 2026-08-29 | 三路评审组 1（sync 核心 + 帧通道）收口 | 2 项决策（活动窗口→另立 story 扩展桌面 schema；通知已读→本地持久化）+ 18 项 patch 全部修复：缓存覆盖守卫（P1）/缓存与帧异常覆盖（P2）/帧字符集容错（P3）/unpair 封存+磁盘单飞互斥（P4/P5）/负数分帧守卫（P9）/tmp 残留清理/clear 失败显式日志/解析移出主线程（P6）/Offline combine 流源（P7）/通知清空+已读持久化（P8/决策 A）/本地时区（P11）/空洞测试补强+防呆扫描扩展（P12/P15）/dataAsOf 格式化（P13）/截断标签兜底（P16）；159 单测全绿 + assembleDebug 成功；组 2（UI 层）评审待跑 |
| 2026-08-29 | 三路评审组 2（UI 层）收口，story → done | 0 决策 / 14 patch（全修复）/ 6 dismiss；核验通过：mock 迁移零悬空、T5 直读收敛、Int? 适配、AC5 达成、AC6 相符。修复：TasksViewModel 快照同一性守卫+seedTask 不回滚（P1）/五 VM 补 loaded=false 清空分支（P2）/onSnapshotReplaced 停流+回落清卡片+幽灵角色回退（P3/P4/P5）/委派关键词随快照角色名派生+UUID 反馈模板（P6）/Settings 空角色档位守卫（P7）/Briefing/WeeklyReview 诚实空态+行动点头空不显（P8）/截断标签域过滤 briefings/weeklyReviews（P9）/TasksRoute roles 订阅派生（P10）/仪表盘 snapshotKey 指标刷新驱动（P11）/WeeklyReview planStates 随快照重建（P12）/MemoryScreen 死代码清理（P13）/换装语义 9 用例补齐（P14）；168 单测全绿 + assembleDebug 成功 |
