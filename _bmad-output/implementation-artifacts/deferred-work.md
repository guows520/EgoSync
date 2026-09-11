# Deferred Work

## Deferred from: SPEC-companion-connection-chat-ux 实施 (2026-09-08)

- **T-S8-B IME 布局修复待真机诊断定案**：T-S8-A 诊断接线已落（MainActivity/AppNavHost/ChatScreen 三层几何快照，仅 debug 构建，`adb logcat -s ImeDiag` 采集；release 零引用）。B 阶段（inset owner 收敛 + bottomBar 处理）依 SPEC ime-diagnosis.md §A.2 单变量 A/B 数据定案——须用户真机执行（原输入法 × 手势/三键导航各一轮），不得以代码推理跳过（AGENTS 规则十三）。定案后诊断日志须完全删除（grep 零残留）。
- **ChatOutbox 发送窗口丢帧语义**：会话反复闪断时逐条串行 flush 若某条持续失败会阻塞后续条目（FIFO 严格序）；当前以「失败后等待 gate 重开再续」的弱语义实现（无死循环但无跳过）。单条污染/毒丸条目的隔离重试属后续健壮性增强，SPEC 范围外。
- **恢复事件导航断言仅为代码走查**：AppNavHost 恢复导航（PAIRING + popUpTo(0)）与 PairingScreen recoveryHint 无 Compose UI 测试（仓库无 compose-ui-test/Robolectric 基建，为单断言引入整套基建违反简单至上）——VM/连接层语义已由 JVM 测试覆盖，UI 层留人工验收。T-S7 解绑文案断言同因。
- **AppModelContainer 恢复收集器无 JVM 测试**：容器 init 的 pairingRecovery → 清快照/通知/事件链需 Android Context，JVM 不可构造；语义（清态 + recoveryMessage 映射）在 RealConnectionClient 测试与文案映射走查覆盖。

## Deferred from: code review of story-13.3 Chunk B (2026-08-31)

- **actionType 子串分类脆弱**：`StreamCoordinator.kt` `toTraceBlock` 用 `toolName.contains("read"/"edit"/...)` 顺序匹配归类工具——`update_and_read` 之类名字会被首个命中子串随机归类。属展示层图标分类，需对照桌面前端（ChatStream 消费方式）的映射表统一后修正，避免双端分类漂移。
- **超时重试不复用 commandId 的重复执行窗口**：手机重试生成新 commandId，桌面幂等缓存无法去重——task.create 类非幂等指令在「慢执行+用户重试」组合下重复执行。接口 doc 已诚实声明，复用 commandId 的重放机制属 14.1 速记队列；已通过 ack 超时 15s→60s 缓解（伪超时概率大幅下降）。

## Deferred from: code review of story-13.3 Chunk C (2026-08-31)

- **conversation.delete 失败不回滚本地**：删除指令失败时本地已移除、桌面仍存——下次 STATE_DELTA 会话「复活」，与用户操作矛盾但最终一致（桌面是事实源）；onError snackbar 已即时反馈。回滚需保存被删会话快照，复杂度收益比低，暂缓。
- **toggle 超时回滚 vs 已收敛快照竞态**：task.toggle 60s ack 超时无条件回滚本地翻转，但桌面可能已执行成功且写信号快照（约 2s）已收敛 done=true——回滚后 UI 与桌面短暂不一致且无后续收敛信号。与 Chunk B「commandId 非复用」deferred 同源（超时+重试组合窗口），14.1 速记队列统一收口。


## Deferred from: code review of story-13.3 Chunk A (2026-08-31)

- **解析失败 ack 恒用空 commandId（超限 envelope 手机 pending 无法关单）**：`models/companion_command.rs` parse 失败（含长度前置检查）即回 `commandId:""` 的 ack——超限 envelope 手机侧确携带 commandId，但回执无法匹配。仅恶意/异常客户端可达（手机编码层 `MAX_PLAINTEXT_LEN=65519` 发送前自拒），桌面已有显式错误回执满足 AC2 字面。建议作为协议边界登记（性质同 §9.3 幂等边界）。
- **route 执行无超时包装**：`companion_dispatch.rs` execute 内命令（DB 锁/opencode 派发）理论上可挂起，手机 120s 看门狗覆盖用户体验，reqwest 有自身超时。桌面侧 `tokio::time::timeout` 为纵深防御，当前价值有限。


## Deferred from: companion-android 任务屏补齐三路评审 (2026-05-27)

- **VM 编排层测试缺口（createTask/4s delay 无测试）**：`TasksViewModel.createTask` 的 id 序列、roleName 回查（未知角色回退「未知角色」）、4s delay→applyClassified 固定归 Q2 均只有静态核对。需以 TestDispatcher 注入 delay（或抽 orchestrateCreateTask 纯化）补 VM 级测试，锁定「智能判断 4s 后必归组、指定象限永不被改」两条路径。仓库有 PairingViewModelTest 先例可循。
- **离线蒙层拦截纯前端筛选（既有机制）**：DegradedOverlay 消费全部指针事件，象限/大石头/归属筛选在离线只读态一律不可操作——筛选本不依赖引擎。需产品决策：为筛选区留交互豁免，或蒙层文案明示「筛选不可用」。
- **ownerKey() "unknown" 回退键不可控（真实层前置问题）**：ownerType=ROLE 且 roleId=null 的任务 ownerKey 归 "unknown"，不在 ownerKeys()（管家+roles）内——solo 语义排除不到它、无 chip 可控。当前 mock 不可构造该状态；接真实层前应将 "unknown" 纳入 allOwnerKeys 或在 TaskItem 层约束 ROLE 必须携带 roleId。
- **新建任务 mock 生命周期不对称（spec 冻结语义内）**：新建任务只落 uiState 不落 SnapshotStore，VM 重建后种子任务重现而用户创建的任务消失（mock 既定语义，代码注释已声明）。接真实层（COMMAND 帧）时自然消除。

## Deferred from: companion-android 图标替换对抗评审 (2026-05-27)

- **adaptive icon 缺 `<monochrome>` 层（pre-existing，非本次引入）**：`mipmap-anydpi-v26/ic_launcher.xml` 无 monochrome drawable，Android 13+ 主题化图标（Themed Icons）不生效、launcher 开启主题图标时本应用回退默认表现。旧版矢量图标同样缺失，非本次回归。建议：增加 `<monochrome android:drawable="@mipmap/ic_launcher_foreground"/>`（或专用单色前景）。
- **桌面端 egosync-app 的 foreground 安全区问题（同源）**：本次移动端已对 foreground PNG 做安全区后处理（缩放居中至 66dp），但桌面端 `egosync-app/src-tauri/icons/android/` 的源 PNG 仍是 Tauri 生成器原样（内容 94×88dp 超出 66dp 安全区）。待桌面端用尊重安全区的工具重新生成后，移动端可重新同步而无需本地后处理。
- **companion-android/.kotlin/ 未进 .gitignore（pre-existing）**：Kotlin 编译器 session 目录以 untracked 出现在 `git status`，易被误提交。建议 `companion-android/.gitignore` 追加 `.kotlin/` 一行。

## Deferred from: code review of Epic 9 (9-5-task-time-picker) (2026-07-22)

- **TaskModal deadline 时区处理为"本地时间当作 UTC"（既有行为，非本次引入）**：`TaskModal.tsx:26-32` 的 `datetimeLocalToIso` 直接给本地 `datetime-local` 值追加 `:00Z`（视为 UTC），`isoToDatetimeLocal` 简单剥离时区后缀。经 `git show HEAD` 确认这两个转换函数在本次改动前已存在且完全未变，Story 9.5 仅替换了选择器 UI，deadline 数据契约保持不变（满足 9-5 AC-4）。已完成取证调查（见 `investigations/task-deadline-timezone-investigation.md`）：唯一确定性消费方是"临期升 Q1"后台任务，按 UTC 日末字符串（日粒度）比较，实际影响为 UTC 日界偏移（非均匀提前 8 小时），其余 now 比较逻辑不碰 deadline。**2026-07-22 用户决策：影响小、暂不修复，延后单独排期**；未来推荐方案 A（deadline 明确为本地墙钟，后端阈值改用本地日期）。

## Deferred from: code review of 8-4-performance-benchmark (2026-06-29)

- **硬编码魔法字符串**：`app.rs` 中 `"perf-test"`、`"fact"`、`"perf-test-conv"` 等硬编码字符串分散多处。属测试代码，可接受；若后续扩展可提取为常量。
- **app_seed_perf_data 部分失败无回滚**：循环插入记忆时某条失败通过 `?` 立即返回，已插入数据保留。依赖 `wdio.conf.ts` 的 `beforeSession` DB 清理兜底，非生产路径。
- **measureStreamRenderLatency 轮询效率**：使用 10ms 间隔轮询 DOM 检查 token 出现，最多 500 次。MutationObserver 更优但当前实现可用，精度满足警告不阻断的阈值需求。
- **auditCssAnimations/auditTsxAnimations 注释/字符串中误报**：静态审计脚本未跳过 CSS/TSX 注释和字符串中的 `requestAnimationFrame`/`setInterval` 匹配，可能误报。属静态审计已知限制，当前 src 中未触发。

## Deferred from: code review of 8-3-wcag-accessibility-audit (2026-06-28)

- **NotificationPanel 未在本次 diff 中修改**：`App.tsx:411` 调用了 NotificationPanel，但该组件未在 diff 中修改，无法确认其是否支持 Escape 关闭与是否有可理解的 aria-label。属 pre-existing 问题，需后续人工检查或纳入回归测试。
- **其他未修改的 Modal 调用点需人工检查**：本次 diff 仅修改了部分 Modal 调用点，但项目中仍有其他调用点。需确认是否都传递了 `ariaLabel`，避免产生无名称 dialog。属 pre-existing 问题。
- **RoleSidebarIcon 能量状态仅靠颜色区分**：色盲友好辅助（形状区分、文字+图标）已按用户决策还原，能量小点统一为圆形。AC #2 色盲友好未实现，作为已知限制保留，待 V2 视觉优化时统一处理。

## Deferred from: code review of 8-2-e2e-test-core-journeys (2026-06-28)

- **Windows msedgedriver 路径假设未验证**：`wdio.conf.ts:829` 假定 `msedgedriver.exe` 位于 `%LOCALAPPDATA%\msedgedriver\`，但 CI 中 `cargo install msedgedriver-tool` + 运行该工具的实际输出位置未经验证。若不一致，`driverArgs` 为空、tauri-driver 找不到原生 driver，Windows E2E 步骤将失败。需 CI 首次运行（或本地 Windows）验证 msedgedriver-tool 的落盘路径后，确认或修正该路径推断逻辑。

## Deferred from: code review of 8-2-e2e-test-core-journeys (2026-06-29)

- **tauri-driver 进程清理不彻底 + 文件描述符未关闭**：`wdio.conf.ts:100-116` 用 `detached: true` 启动 tauri-driver，Windows `shell: true` 下 `kill()` 可能只杀 shell 不杀 tauri-driver.exe；`driverLogFd` 从未 `closeSync`。CI 全新环境不受影响，本地多次运行可能端口 4444 占用。改进项，非阻塞。
- **归档恢复测试假阳性**：`role-crud.spec.ts:114-115` 仅验证图标数量 +1，未验证恢复的就是之前归档的角色。测试健壮性改进。
- **角色定位逻辑竞态条件**：`role-crud.spec.ts:18-27` `enterRoleByName` 用固定 500ms pause 等待 UI 更新，渲染慢时可能失败。测试稳定性改进。
- **硬编码超时缺乏配置化**：`wdio.conf.ts:51-55` 和各 spec 中大量硬编码 timeout，CI vs 本地可能需要不同值。配置化改进。
- **构建模式偏离 debug→release**：spec 原计划 `--debug --no-bundle`，实际用 release（Dev Notes 偏离 #4 已记录）。已记录偏离，release 与现有 `tauri build` 产物路径一致。
- **AC2.1/2.2 降级**：冷启动未验证消息气泡、管家对话未预置历史消息（Dev Notes 偏离 #2 已记录 LLM 限制）。CI 无 LLM API Key 限制。
- **AC2.4 LLM 流式降级**：仅验证静态 UI 契约，未验证流式光标/禁用/停止按钮（Dev Notes 偏离 #5 已记录）。CI 无 LLM 限制。
- **AC2.6 冲突仲裁未实现**：仲裁特性 5-3~5-6 已 deferred-v2，仅验证健壮性。待 V2 仲裁特性落地。
- **AC2.7 简报复盘 Modal 无 UI 入口**：周复盘 Modal 仅由后台调度触发，E2E 无法点击打开（Dev Notes 已记录）。应用架构限制。
- **AC4 性能验证未完成**：Task 5.3 标记 [ ]，7 条旅程总耗时 ≤ 5 分钟未测量。待 CI 首次运行验证。
- **opencode-workspace 目录未清理**：`wdio.conf.ts:91-97` beforeSession 仅清理 DB 文件，opencode-workspace 配置可能残留。sidecar 占位失败降级不影响 E2E。
- **Task 6.1/6.2 本地验证未完成**：需 tauri-driver 安装才能本地运行。待本地环境准备。

## Deferred from: code review of 7-2-data-destroy-initial-state (2026-06-27)

- **跨库+Keyring 非原子，部分失败留下不一致状态且前端无提示**：`destroy_all_data`（data_export.rs:695-744）主库与对话库分别独立事务（SQLite 不支持跨库事务，Dev Notes 已声明）。主库提交后若对话库事务失败，会出现"角色已清空但对话仍在"的不一致状态；前端仅显示通用错误，未告知"部分销毁"。属架构固有限制，本 story 范围外；V2 可考虑销毁前停掉后台调度并提供"部分销毁"明确提示。

## Deferred from: code review of 7-1-data-export-json-markdown (2026-06-27)

- **复制活动数据库一致性**：`export_sqlite` 用 `std::fs::copy`（data_export.rs:531）复制存在打开连接的 `egosync.db`/`conversations.db`，导出前已执行 `wal_checkpoint(TRUNCATE)` 降低风险。单用户桌面场景导出瞬间并发写概率极低，可接受；若 V2 需更强保证可改用 `VACUUM INTO`（SQLite 3.27+）。
- **通知 JOIN 反规范化导出**：`gather_export_data` 用 `list_notifications` 返回 `NotificationWithRole`（含冗余 role_name/icon/color 字段，data_export.rs:234），JSON 导出含重复字段。未来 Story 7-4 数据导入需处理此结构；非本 story 范围。

## Deferred from: code review of 6-6-big-rock-daily-protection (2026-06-27)

- **周五去重为内存变量，应用重启会重复**：`last_bigrock_friday_check_date`（scheduler.rs:1088）在内存中，周五当天重启调度器会重置 → 可能重复发送周五汇总提醒。与既有周复盘去重模式一致，非本故事独创；V2 若做"启动补检测/去重持久化"应统一三处触发逻辑。
- **周五对陈旧大石头双重提醒**：周五时同一陈旧大石头既收到 AC1 逐任务"还没动"提醒，又被计入 AC4 汇总"还有 N 个未完成"。spec 将两者定义为不同职责，属设计取舍；若 UX 反馈过于打扰，可在周五抑制逐任务提醒只发汇总。

## Deferred from: code review of 6-4-weekly-review-scorecard (2026-06-27)

- **单点触发时刻无重试窗口**：周复盘在精确 HH:MM 触发，且 `last_review_trigger_week` 在 spawn 前置位（scheduler.rs:530-540）；该分钟若错过（休眠/时钟跳变）或生成降级失败（`Ok(false)`），本周不再重试。系 briefing/bigrock 共有设计，非本故事引入；V2 若做"启动补检测"应统一三处触发逻辑。
- **`collect_new_skill_names` N+1 查询**：对每个 skill_id 逐条调用 `get_skill`（review_generator.rs:289-300）。Dev Notes 已说明 `skill_role_bindings` 表数据量小，可接受；若未来 Skill 量大可改为单次 JOIN 查询。
- **故事文件元信息未更新**：`6-4-weekly-review-scorecard.md` Status 仍为 `ready-for-dev`、File List/Dev Agent Record 仍为占位（line 7, 337）。属 dev-story 收尾流程职责，非代码缺陷。

## Deferred from: code review of 6-3-big-rock-planning-reminder (2026-06-26)

- **错过精确触发分钟则当周不再提醒**：大石头提醒触发条件为 `current_hhmm == bigrock_time` 精确匹配（scheduler.rs:492-493），若 App 在配置分钟未运行（关闭/休眠/tick 错过该分钟），本周不会补提醒。此为轮询调度器固有限制，且与 Story 6.1 简报触发（scheduler.rs:445）同模式，非本次改动引入。若 V2 需要"启动时补检测错过的提醒"，应统一改造简报与大石头两处触发逻辑。

## Deferred from: Epic 5 产品决策 — 冲突仲裁三 Story 延迟至 V2 (2026-06-25)

**涉及 Story**：5-4-three-step-arbitration / 5-5-arbitration-modal-visualization / 5-6-arbitration-auto-execution

**决策原因**：
当前冲突检测基于任务截止时间（deadline）的 ±1 小时窗口判断。但 deadline 仅代表"最晚需要完成的时间"，不代表"计划在此时执行"。两个 deadline 接近的任务完全可以提前完成其中一个，并不真正冲突。基于此粗粒度检测去做三步仲裁（使命对齐→四象限定位→能量平衡）并自动调整任务，逻辑链建立在不稳固的前提上。

**根本问题**：要实现有意义的冲突检测，需要引入"计划时间段"（planned_start + planned_end 或 planned_date + estimated_minutes），让用户在周计划时为任务分配具体执行时间，再基于时间段重叠判断冲突。这符合《高效能人士的七个习惯》习惯三"个人管理四步骤"中"安排进度"步骤的描述。

**为何不在 V1 做**：
- 引入计划时间段需要新增数据库字段、前端周计划视图、时间拖拽交互等，整体改动较大
- 与 V1"轻量"产品定位冲突，会增加用户输入负担
- 5-3 的 datetime-local 改造已为 V2 打下基础（deadline 已支持精确到小时）

**V2 方向**：
1. 给任务增加"计划日期 + 预计耗时"字段（或 planned_start / planned_end）
2. 引入书中描述的"周计划"视图，让用户为每个角色的要事安排具体时间段
3. 基于时间段重叠（而非 deadline 接近）检测真正的执行冲突
4. 在此基础上再实现 5-4 三步仲裁、5-5 仲裁可视化、5-6 自动执行
5. 5-3 已完成的 conflicts 表、conflict_detector 服务、datetime-local 输入可复用

## Deferred from: code review of 5-3-time-conflict-detection (2026-06-25)

- **deadline 解析对非标准格式失败致漏检 (LOW→deferred)**：`db/conflicts.rs:195-219` `deadlines_within_one_hour` 仅解析 RFC3339 与 `%Y-%m-%d`。无秒/无时区（如 `2026-06-25T10:00`）解析为 None→返回 false→该任务对漏检冲突。依赖存储格式约定（ISO8601 Z），当前数据符合则无影响。建议后续显式校验/规范化 deadline 写入格式。
- **deadline 字典序排序在混合格式下提前 break (LOW→deferred)**：`services/conflict_detector.rs:146-164` 排序按 deadline 字符串字典序，滑动窗口靠 `break` 提前终止。混合格式（date-only 与 full datetime）字典序与时间序不一致时会错误 break，漏检后续冲突。与上一条同源于格式约定。
- **冲突历史无界增长 (LOW→deferred)**：`hooks/useConflicts.ts:22` 每次挂载经 `conflict_list` 加载全量冲突（含 resolved/dismissed）；`021_conflicts.sql` 无保留/清理策略。长期运行 conflicts 表与前端列表持续膨胀。建议加状态过滤的默认查询 + 定期归档/清理。
- **冲突检测规模化性能 (LOW→deferred)**：`conflict_detector.rs:144-175` 双重循环 O(n²)；`conflicts.rs:135-154` `auto_resolve_stale_conflicts` 对每条 detected 冲突各发 2 次任务查询。每 60s 全量扫描，任务/冲突量大时调度器开销上升。建议批量查询或加时间窗口索引优化。

## Deferred from: code review of 5-1-mission-statement-setting (2026-06-24)

- **缺少 ButlerSettingsContent 使命宣言组件级测试 (LOW→deferred, 测试增强)**：`ButlerSettingsContent.test.tsx` 无 mission 加载/结构化 JSON 解析/保存反馈的断言。service 层 `missionService.test.ts` 已覆盖 invoke 参数，但组件层逻辑（`format='structured'` 时的 JSON 解析填充、模式切换、保存成功反馈）未被测试锁定。建议后续补一组组件测试覆盖加载填充与结构化往返。

## Deferred from: code review of 4-5-three-tier-notification (2026-06-22)

- **每日敲门上限存在竞态 (LOW→deferred)**：`services/notification_service.rs:82-93` 的 `create_notification_for_role` 先 `count_knock_today` 再 insert，两步非原子。调度器对每个到期角色独立 `tokio::spawn`，多角色时间点对齐时第 4+ 次 knock 可能在计数与写入之间穿插，导致超过 `DAILY_KNOCK_LIMIT`。当前调度节奏下影响小。建议后续用单事务 + 行锁或 `INSERT ... WHERE (SELECT count...) < 3` 收敛。
- **mark_read 对已读通知返回 NotFound 语义误导 (LOW→deferred)**：`db/notifications.rs:58-62` 对已读通知返回 `AppError::NotFound("通知 X 已读")`，错误变体与语义不符。前端 `NotificationPanel`/`App` 均有 `!isRead` 守卫，当前无功能影响。建议改为幂等返回 Ok 或使用 `ValidationError`。

## Deferred from: code review of 4-3-proactivity-dial-behavior (2026-06-22)

- **`moderate` 档过滤未校验 priority 取值域 (LOW→deferred, pre-existing 4.2)**：`filter_suggestions_by_proactivity`（`suggestion_generator.rs`）在 `moderate` 档仅 `filter(|s| s.priority != "low")`，任何非精确 `"low"` 的值（含 LLM 返回的未知优先级如 `"urgent"`）都会被保留。根因在 4.2 的 `generate_suggestions`（`suggestion_generator.rs:264`）只做 `trim().to_lowercase()`，不校验 priority 是否属 `ALLOWED_PRIORITIES`。非本 story 引入；保留语义对 AC2 无害（只显式丢弃 low）。建议在 4.2 生成侧或 `create_suggestion` 写入侧统一收敛 priority 取值域时一并处理。

## Deferred from: code review of 4-2-proactive-suggestion-generation (2026-06-21)

- **同一时间点多角色并发 LLM 调用无全局限流 (LOW→deferred, 4.1 同源遗留)**：`services/scheduler.rs` 对每个到期角色独立 `tokio::spawn` → `run_work_loop_for_role` → `generate_suggestions` 发起 LLM 调用。多角色时间点对齐时会瞬时并发多个 LLM 请求。单次调用已有 `timeout(60s)`（`suggestion_generator.rs:271`）防挂死，但无全局并发上限/信号量。V1 用户角色数有限可接受；若未来角色增多，建议引入信号量或错峰抖动（与 4.1 deferred 项合并处理）。
- **build_role_task_summary 在热循环中重复 get_role (LOW→deferred, pre-existing)**：`services/agent_engine.rs:1128` `build_role_task_summary` 内部又 `get_role(main_pool, role_id)`，而 4.2 调用方 `generate_suggestions` 已持有完整 `role`。该重复查询在每个调度 tick 触发，属热路径冗余 IO。建议后续让 `build_role_task_summary` 接受已有 `&Role` 入参或拆分纯聚合函数。非 4.2 引入。

## Deferred from: code review of 4-1-background-scheduler-work-loop (2026-06-20)

- **同一 tick 多角色到期并发 spawn (LOW→deferred, 属 4.2 范畴)**：`services/scheduler.rs:111` 对每个到期角色独立 `tokio::spawn`，当前为占位实现无副作用。Story 4.2 接入 LLM 调用后，若多个角色 interval 对齐（如同时到达整点 4h/8h 边界）会瞬时并发多个 LLM 请求，可能触发 provider 限流或资源峰值。建议 4.2 实现时引入并发上限（信号量）或错峰抖动。

## Deferred from: code review of 3-6-quadrant-grouped-display (2026-06-20)

- **`task.quadrant` 非法值致渲染前分桶崩溃 (LOW→deferred, pre-existing)**：`TasksTab.tsx:252-258` 的 `groupedTasks[task.quadrant].push(task)` 未校验 quadrant ∈ {Q1..Q4}；若上游写入非法象限值（理论脏数据）将抛 TypeError。该 reduce 非本 story 引入，渲染侧用 `allQuadrants` 遍历安全，仅分桶侧暴露。建议分桶处加 `if (!groupedTasks[task.quadrant]) return;` 兜底，或在类型/DB 层收敛取值域。
- **前端测试套件全量运行 flaky (MED→deferred, pre-existing/越界)**：`ButlerSettingsContent.test.tsx:95`（`getByRole('switch',{name:'find-skills'})`）在全量 `vitest run` 下失败，但单独运行 11/11 通过 → 测试隔离/异步竞态导致的 flaky。与 story 3-6 仅改 `TasksTab` 的 diff 无因果关系，但使 3-6 的「test:frontend 199 全通过」声明当前不可复现。建议单独立项排查（疑似并行执行下共享 mock/计时竞态）。

## Deferred from: code review of 3-4-big-rock-marking (2026-06-19)

- **big_rock 计数检查与写入非原子（TOCTOU 竞态，LOW→deferred，V1 单用户可接受）**：`db/tasks.rs:13-44`（create_task）与 `:67-113`（update_task）中 `count_big_rocks_by_role` 与 INSERT/UPDATE 未包裹同一事务，并发写入（用户 + opencode agent）理论上可双双读到 count<3 而突破 3 个上限。V1 单用户桌面应用实际风险极低；若未来引入多写入方，应仿照 `reorder_tasks` 用事务包裹 count+write。

## Deferred from: code review of 3-3-auto-quadrant-classification (2026-06-18)

- **临期阈值 UTC 与 deadline 本地日期边界偏差 (LOW→deferred, 需全应用时区决策)**：`services/task_deadline_watch.rs:55-65` `compute_imminent_threshold` 基于 `SystemTime` UTC 秒推导日期，而 `deadline` 来自 `<input type="date">` 的本地墙钟日期。全应用刻意 UTC-only（`db::settings::chrono_now` 裸 SystemTime，无 chrono 依赖），单独为本功能引入本地时区会与约定不一致。影响有限且自愈：UTC+8 用户跨日边界附近阈值最多偏早一天，每小时循环随 UTC 推进会在 ≤1 个时区偏移内补上，非永久漏判。建议待全应用时区策略统一时一并处理。
- **`extract_json_object` 贪婪截取首个 `{` 到末个 `}` (LOW, robustness)**：`services/task_classifier.rs:334-341` 当 LLM 在 JSON 前后输出含散落花括号的散文时，截取区间会变成非法 JSON 而解析失败。当前失败安全降级到 Q2，非正确性破坏；可后续改为按花括号深度扫描提取首个完整 JSON 对象增强健壮性。

## Deferred from: code review of 2-12-opencode-ecosystem-skill-discovery-import (2026-06-09)

- **duplicate 分支 replace_bindings 跨角色解绑 (HIGH→deferred, pre-existing)**：`import_opencode_skill` 重复导入时调 `replace_bindings(skill_id, false, [当前角色])`，其语义为 DELETE 该 skill 全部绑定后只重插当前角色，会静默解绑该 Skill 已绑定的其它角色。2.11 `import_custom_skill` 使用完全相同模式 → 非本次引入，应作为统一 binding "merge vs replace" 语义问题单独立项处理。
- **async 命令内阻塞 std::fs I/O (MED→deferred, pre-existing)**：`scan_opencode_root` / `import_opencode_skill` 在 async fn 内同步 `read_dir`/`read_to_string`，慢盘或大目录会阻塞 tokio 工作线程。既有 skill_registry 同步 I/O 模式一致，建议统一迁移到 `tokio::fs` 或 `spawn_blocking`。
- **read_dir 权限失败静默 (LOW→deferred, pre-existing)**：目录存在但权限不可读时 `let Ok(entries) = read_dir else { return Ok(()) }`，与"目录不存在"同等静默，用户无反馈。低概率边界，可与扫描可观测性增强一并处理。
- **content_hash 全局唯一不分 source_type (MED→deferred, 用户裁决)**：内容相同的 opencode Skill 会被 `find_skill_by_content_hash`（不分 source_type）误判为某 custom Skill 的 duplicate，返回 `entry.source_type='custom'`。用户 Decision #1 选项 1 未选改 `(content_hash, source_type)`：V1 同内容跨源场景极罕见，暂保持全局唯一；若未来生态导入增多再立项加 source_type 维度。

## Deferred from: code review of 2-6-conversation-memory-extraction (2026-05-30)

- **streaming 标志插入失败永久卡死会话 (HIGH, pre-existing)**：`chat.rs` 中 `streaming.insert` 被提前到 busy 检查后、两条 `insert_message` 之前；任一 DB 写失败 `?` 提前返回时清理任务尚未 spawn，会话本进程内永久返回"我还在想上一个问题"。git 取证确认 baseline 34aab28 顺序安全，此错误顺序来自工作树未提交的前序 opencode 重构，**非 Story 2.6 引入**。⚠️ 必须在该批前序工作提交前修复（恢复 baseline 的"先插 assistant 再 insert streaming"顺序）。
- **记忆去重仅精确文本匹配 (LOW)**：唯一索引 `(source_conversation_id, category, content)` 拦不住 LLM 同义重述；V1 已知边界，Dev Notes 已声明。
- **提炼无取消钩子 + 跨库悬挂记忆 TOCTOU (MED→deferred)**：会话删除/新消息无法打断已开始的提炼；存在性校验与写入间存在 TOCTOU 窗口，可能写入悬挂 source_conversation_id。V1 无消费方，待 2.7/2.8 处理。
- **insert_memories DB 层不校验 source id 归属 (LOW)**：归属白名单仅在 pipeline 层；当前调用链一致，防御性提示。

## Deferred from: code review of 2-0c-dialog-engine-switch-to-opencode (2026-05-26)

All items resolved in the same session:

- ~~streaming_state TOCTOU race~~ ✅ Fixed: merged check+insert into single lock scope
- ~~OnboardingConversations map 无清理~~ ✅ Fixed: auto-remove on step >= 5; also cleaned on conversation delete
- ~~chat_delete_conversation 不清理活跃流/session 状态~~ ✅ Fixed: cancels token, removes streaming/opencode/onboarding state
- ~~SSE parser 未处理 `\r\n\r\n`~~ ✅ Fixed: normalize `\r\n` → `\n` before buffering
- ~~ensure_success 不读取 error body~~ ✅ Fixed: new `ensure_success_with_body` reads body on error; used in create_session, send_message, abort_session
## Deferred from: code review of 2-8-selective-memory-forget (2026-06-01)

- **useMemories refetch 失败覆盖删除成功状态 (LOW, pre-existing)**：`useMemories.ts:35-40` 在 list 失败时 `setMemories([]) + setError('记忆暂时加载失败…')`，会把刚刚成功的删除结果覆盖成"加载失败"空态。该 hook 未在本故事 diff 中修改，属既有行为；删除主流程已正确，仅在 refetch 阶段网络/DB 抖动时短暂误导。
- **确认对话框无障碍缺口 (LOW, a11y)**：`MemoryTab.tsx:243-276` 的自定义确认 UI 为普通 `<div>`，缺 `role="alertdialog"`、`aria-live`、打开时焦点转移与 Esc 关闭，读屏/纯键盘用户体验不佳。统一归并到 Epic 8 story 8-3（WCAG 可达性审计）处理，避免本故事局部引入与项目其它面板不一致的 a11y 模式。

## Deferred from: code review of 2-11-custom-skill-md-import-role-binding (2026-06-04)

- **command 层拼装受控目录路径 (LOW, 整洁度)**：`commands/skill.rs:35-39` 在命令层拼 `app_data_dir/opencode-workspace/.opencode/skills`，受控目录约定属业务/配置逻辑，应下沉到 service（建议新增 `skill_registry::resolve_skills_root(app_data_dir)`，command 只传 app_data_dir）。无功能影响，boss 决定 defer。
- **description 无长度上限 (LOW, prompt 膨胀)**：外部 SKILL.md 的 description 原样进 DB 并注入每个启用该 Skill 的角色 prompt 与 opencode.json，超长内容会膨胀配置与每次请求 prompt。boss 决定本次不做限长、不加 UI，遗留后续（建议届时复用 app_settings KV，key=skill_description_max_len，默认 2000，按 chars 截断）。
- **缺"日志不含内容/路径"不变量测试 (LOW, 可观测性)**：现有实现设计上合规（tracing 仅记 id/hash/状态，prompt 仅用 name/description，不含用户原始路径），但 diff 中无断言测试锁定该不变量；FS 写失败时 `AppError` 可能携带受控绝对路径进 warn 日志（非用户原始路径，风险低）。建议补一条断言测试固化"日志/prompt 不含文件全文与路径"。

## Deferred from: code review of 2-7-memory-panel-traceability (2026-06-01)

- sourceStates/sourceRequestIds 在 role/category 切换时未清理（MemoryTab.tsx）：memory id 全局唯一不串号，仅轻微内存累积，无功能危害。
- categoryLabels 对未知/历史 category 无兜底标签（MemoryTab.tsx）：insert_memories 有 validate 护栏，仅影响潜在历史脏数据。
- 缺少部分来源缺失 / 前端展开竞态 / Tauri State 注入护栏的测试（memory_query.rs, MemoryTab.test.tsx）：测试增强项，非阻塞。

## Deferred from: code review of 3-2-task-drag-sort-complete (2026-06-17)

- **`reorder_tasks` N+1 查询 (LOW, performance)**：每个 ID 执行一次 SELECT role_id + 一次 UPDATE sort_order，任务量大时往返次数线性增长。SQLite 本地 IO 影响极小，可后续合并为单条 `UPDATE ... CASE WHEN id=? THEN ?` 批量优化。
- **缺 `aria-live` 屏幕阅读器反馈 (LOW, a11y)**：拖拽排序、完成切换无 `aria-live` 区域向辅助技术通报结果。归并到 Epic 8 story 8-3（WCAG 可访问性审计）统一处理。

## Deferred from: code review of 2-13-mcp-server-list-role-access (2026-06-11)

- command 连接测试超时误报 + Windows 孤儿进程（mcp_server.rs::test_command_server）：stdio/local command 仍以 2s 内不退出作为“可长期运行”的启发式判断，慢速失败可能被误报为成功；`cmd /C` 启动的孙进程（npx→node）可能不被 child.kill 完整回收。平台特定，非阻塞。注意：remote `streamable_http`/SSE 测试连接假成功已在 2026-06-13 修复为 MCP protocol/SSE content-type 校验。
- add_to_role 不校验归档角色（mcp_server.rs::add_to_role）：可对 archived 角色建立 MCP 绑定，full_sync 会给 archived agent 加 disable 兜底，仅状态污染。
- sync_role_updated_with_skills_and_mcp 未处理 archived disable 标记（agent_config.rs）：单角色同步路径未设 disable，full_sync 与 sync_role_archived 兜底，低风险。
- 角色级 MCP 硬隔离需真实 opencode 数据流人工 UAT（2.0d 根因领域）：动态切换顶层 mcp scope 对已建 session 的即时生效性仅有单测覆盖，需人工复验。Dev Agent Record 已承认 E2E 未返回结构化结论。

## Deferred from: code review of 3-5-q2-protection-at-risk (2026-06-20)

- **历法计算三处重复 (LOW, 整洁度/DRY)**：`days_to_ymd` 与「now±N 天 → ISO 时间戳」逻辑在 `services/task_protection_watch.rs:50-78`、`services/task_deadline_watch.rs:55-79`、`db/settings.rs:145-174` 三处各有一份（含 719468/146097 等魔数）。本 story 新增第三份。建议抽公共时间工具（如 `util::clock`）统一，单测集中。属既有扩散，非本 story 引入。
- **`spawn_hourly_watch` 无优雅关闭 (LOW, 生命周期)**：`task_protection_watch.rs:88-100` 的后台 `loop { ... interval.tick() }` 无取消/关闭钩子，应用退出时随进程结束。与既有 `task_deadline_watch::spawn_hourly_watch` 完全同模式，属全局后台任务生命周期约定，非本 story 单独承担。
- **状态字面量硬编码无共享常量 (LOW, 漂移风险)**：`'at_risk'` / `'normal'` 在 `db/tasks.rs:466-509`（mark/clear/update/completion）、`commands`、前端 `TasksTab.tsx` / 类型层多处以裸字符串出现，无 Rust 端共享常量或枚举护栏，拼写漂移不会被编译期捕获。建议后续统一为 `ProtectionStatus` 枚举 + `as_str()`。

## Deferred from: code review of 6-1-daily-morning-briefing (2026-06-25)

- **briefing_time 无格式校验 (LOW)**：`briefing_generator::get_briefing_time` 直接返回 `app_settings` 原始字符串，调度器用 `current_hhmm == briefing_time` 精确匹配。若配置为非 `HH:MM`（如 `8:00`、含空格、`25:00`）则永久不触发。配置入口属 Story 6.2，当前仅走默认值 `08:00`，无实际风险。建议 6.2 写入时校验格式。
- **内存去重键在 spawn 前置 (LOW)**：`scheduler.rs:431-432` 在 `tokio::spawn` 之前即置 `last_briefing_trigger_date`，瞬时 LLM 失败（返回 Ok(false)）当天不再重试，需重启应用方可恢复。符合 AC10"同一天只触发一次"语义且避免 LLM 故障时每分钟重试，可改为仅在确认写入/已存在时置位作为未来优化。
- **tick 精确分钟匹配可能跳过触发 (LOW)**：`scheduler.rs:431` 与现有角色调度器同模式，60s tick 若因系统休眠/负载漂移整分钟漏 tick，当天简报不触发（DB 去重不补触发）。属既有调度设计约定。
- **50 条记忆窗口可能挤掉昨日记忆 (LOW)**：`briefing_generator.rs:186-196` 取最近 50 条记忆后在 Rust 侧过滤昨日，重度用户当日记忆 >50 条时昨日记忆段落可能为空。Dev Notes（story line 301）已知此约束。

## Deferred from: code review of fix-opencode-agent-name-identity (2026-07-15)

- **Butler prompt 既有断言与当前静态 prompt 不一致（MEDIUM，测试门禁）**：`services::agent_config::tests` 中 3 项既有测试仍期待旧文案 `你是EgoSync管家` 或 prompt 不出现 `find-skills`，而基线 `9b16c880` 的静态 prompt 已不满足这些断言。当前结果为 31 通过、3 失败；失败不是移除 Agent `name` 所致，应单独校准 Butler prompt 契约与测试。

## Deferred from: code review of disable-opencode-question-tool (2026-07-16)

- **Rust 全仓格式门禁存在基线债务（测试门禁）**：`cargo fmt --check` 对多个未修改文件及 `agent_config.rs` 的历史代码报告大量差异，无法作为本次外科式修复的通过门禁。本次新增代码未引入 `git diff --check` 错误；全仓格式统一应单独实施，避免把无关格式重写混入功能修复。

## Deferred from: code review of fix-custom-skill-delete-conflict (2026-07-20)

- **OpenCode 运行时刷新并发协调（HIGH）**：上一轮同批未提交改动使用全局待刷新布尔标记；两个消息流并发进入或已有 stream 正在运行时，sidecar 重启可能与请求交错。需单独明确“活动 stream 是否允许被中断”的产品语义，再设计进程级互斥/共享刷新任务与并发集成测试。
- **Skill registry 事件异步响应过期（MEDIUM）**：`SettingsTab` 与 `App` 的事件刷新未使用请求序号或取消保护，快速切换角色或连续事件时可能由旧响应覆盖新状态。应在独立状态一致性修复中统一处理。
- **Skill 全局删除原子性（MEDIUM）**：后端 registry、角色绑定/配置和 Agent 配置同步跨多个步骤，异常时可能形成部分成功。该路径为本次前端修复前已存在，需单独评估数据库事务边界与配置文件补偿策略。

## Deferred from: code review of fix-agent-stream-error-duplicate (2026-07-23)

- **Agent Engine 定向测试存在既有 Butler prompt 断言失败（测试门禁）**：`cargo test agent_engine` 运行 110 项时 109 通过、1 失败；失败项 `services::agent_engine::tests::test_build_butler_system_prompt_omits_disabled_meta_skills` 在 `agent_engine.rs:6922` 仍断言旧提示词包含“你是 EgoSync 的分身管家”。本次补丁未修改该 prompt 构建或断言区域，且新增代码已通过编译及 `test_sse_error_maps_to_done_payload` 精确测试；应另行校准 Butler prompt 契约与历史断言。

## Deferred from: code review of 8-6-opencode-sidecar-upgrade-lifecycle (2026-07-26)

- 非 Windows 平台的 `kill_process_on_port` 仍调用 Windows `cmd/netstat/taskkill`；该问题在 baseline `4b69704` 已存在。位置：`egosync-app/src-tauri/src/services/sidecar.rs:742`。

## Deferred from: code review of 8-7-model-network-location-proxy-bypass (2026-07-27)

- NSIS installer hook 文件当前未跟踪；若只提交统一 diff，干净检出上的 Windows NSIS 构建会缺少 `windows/installer-hooks.nsh`。来源：Story 8.6 / `tauri.conf.json:56`。
- 非 Windows 平台的陈旧 sidecar 端口清理仍执行 `cmd`、`netstat`、`taskkill`，Linux/macOS 重启时无法清理占用端口的旧进程。来源：Story 8.6 / `sidecar.rs:749-782`。
- Windows Job Object/stop 生命周期存在多项缺口：父进程加入 Job 前的子进程逃逸窗口、正常退出时 Job 句柄未及时关闭、停止失败后 Child 句柄丢失，以及瞬时退出进程测试竞态。来源：Story 8.6 / `sidecar.rs:469-600,1376`。
- 聊天执行事件通过正文匹配历史消息；thinking-only、重复正文及多 assistant bubble 时可能丢失或错误归属。来源：其他聊天改动 / `ChatStream.tsx:970-978`。
- Thinking 分段在空元事件、重复 text/tool 快照、非单调 replacement、DB 写入失败及超时提前返回时可能错误切段或丢失持久化内容。来源：其他聊天改动 / `agent_engine.rs:863,2849-2858,2993-3009,3134-3178`。
- Thinking 插入连续读取事件前未刷新 `readRun`，会把“读取 → thinking → 写入”显示成“thinking → 读取 → 写入”。来源：其他聊天改动 / `ChatStream.tsx:479-513`。

## Deferred from: code review of fix-deepseek-provider-constraint (2026-07-27)

- **完整 Rust 套件存在 6 项既有 Agent prompt/permission 契约失败**：`cargo test -- --test-threads=1` 结果为 796 passed、6 failed、0 ignored；失败位于未修改的 `services/agent_config.rs`（5 项）与 `services/agent_engine.rs`（1 项），包括旧 Butler prompt 文案、`find-skills` 禁用预期及 permission JSON 形态断言。本次数据库 migration 030 与新增回归测试均通过，且 baseline 后上述失败文件无差异；应另行统一 Agent 配置生成契约与测试期望。

## Deferred from: mobile-desktop-parity 调查拆分 — 前端 UI 对等工作多目标切分 (2026-08-25)

调查见 `investigations/mobile-desktop-parity-investigation.md`。用户裁决：先做 #1 图标系统重做（已立项为 spec-companion-android-icon-parity），以下三项后续迭代。**全部为纯移动前端工作，数据继续跑在 mock/SnapshotStore 只读，不碰后端**（桌面 companion 模块 Phase 2/3 整体后延），符合纯前端原型硬性边界（不引入网络/Room/Hilt/OkHttp）。

- **#2 设计语言补齐**：信息密度双模式（对话流轻量 / 仪表盘密集）+ 动效克制（仅角色卡呼吸 `--breath-duration:3s`）+ 色温随角色偏移（工作冷 / 家庭暖）+ `prefers-reduced-motion` 降 0ms。色彩 token 已对齐（`Color.kt:10-26`），补的是密度 / 动效 / 色温三件。**✅ 已完成于 2026-08-26，见 `spec-companion-android-design-language-parity.md`。**
- **#2 附带 defer（评审发现，2026-08-26）：reduced-motion 活跟踪**——`rememberReducedMotion()` 按规格「读取一次并缓存」实现，应用运行中切换系统「移除动画」开关不生效（须重建 Activity 才刷新）；web `prefers-reduced-motion` 是实时媒体查询，语义上不完全等价。后续增强：DisposableEffect + ContentObserver 监听 `Settings.Global.getUriFor(ANIMATOR_DURATION_SCALE)` 活跟踪（纯 compose-runtime 零新依赖），或 minSdk≥28 时改用 `ValueAnimator.areAnimatorsEnabled()`。位置：`companion-android/app/src/main/java/com/egosync/companion/ui/theme/Motion.kt:432-441`。
- **#3 三 ViewModel 重接 SnapshotStore 只读**：`ChatViewModel`/`TasksViewModel`/`DashboardViewModel` 从硬编码 mock（`AppNavHost.kt:197/212/226` 无参构造）→ 经 `AppModelContainer` 接 `SnapshotStore` 只读快照。这是「换 ConnectionClient 实现零改动」真正生效的前提（当前仅 Settings/Notifications/降级遮罩读 container）。**✅ 已完成于 2026-08-26，见 `spec-companion-android-snapshotstore-vm-wiring.md`。**
- **#4 FR 屏补全（17 项）**：14 缺失 + 3 部分 FR 的屏/交互。逐 FR 桌面基线 → 移动新增对照见调查 case file 的 Recommended Next Steps B 表。本身需再拆成若干 story（建议按屏分组：chat 相关 FR-1/2/20/29/30；dashboard 相关 FR-12/38；role/memory 相关 FR-5/8/9；tasks 相关 FR-23；review FR-17；onboarding FR-21；notify FR-24）。**✅ 已完成于 2026-08-27，见 `spec-mobile-fr-parity` 六组规格（组 1-6）；其中 onboarding FR-21 后经 2026-09-10 裁决从移动端移除（`spec-companion-android-remove-onboarding.md`），勿再实现。**

边界：D 类三项（FR-14/15/7）采用语义一致（经后端事件 → 通知承载），不在此前端工作内；FR-20/21 强制语义适配。

## Deferred from: quick-dev 评审 of spec-companion-android-snapshotstore-vm-wiring (2026-08-26)

- **真实连接层回复流健壮性（预存在逻辑，本次评审顺带暴露）**：`ChatViewModel.sendMessage` 回复轮换 `replies[replyIndex % replies.size]` 在 `butlerReplies` 为空列表时触发整数除零 ArithmeticException（`viewModelScope` 协程内未捕获将崩溃）；空字符串回复会经打字机循环留下一颗永久空白管家气泡。当前静态 mock（4 条固定回复）下不可达，且表达式为基线 f638a7c 既有代码原样搬移、非本次引入；接入 SNAPSHOT/STATE_DELTA 帧驱动数据时应一并补空列表/空串守卫。位置：`companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt:74-82`。

## Deferred from: quick-dev 评审 of spec-mobile-fr-parity-group1-chat (2026-08-26)

- **聊天自动滚动以逐字文本为 key 致高频重启（预存在模式，本次评审顺带暴露）**：`ChatScreen` 的 `LaunchedEffect` 把 `uiState.messages.lastOrNull()?.text` 作为 key，打字机每 24ms 变一次文本即重启 `animateScrollToItem`，滚动抖动且流式期间用户无法上滑阅读历史。该模式在基线 bf845248 的原 ChatScreen.kt:74 已存在（`LaunchedEffect(totalItems, uiState.messages.lastOrNull()?.text)`），组 1 改动原样延续、未引入新缺陷；修法为仅以结构变化（条目数/卡片增删）触发滚动，或先判断列表已贴近底部才自动滚。位置：`companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt`。

## Deferred from: quick-dev 评审 of spec-mobile-fr-parity-group2-dashboard (2026-08-26)

- **自定义时间窗的绝对锚定与未来日期提示（原型粒度降级的接线期事项）**：`ActivityWindow.Custom` 以相对天数（oldest/newestDaysAgo）入状态，跨午夜后标签与窗口整体前移一天（非用户操作所致）；`toDaysAgo()` 把 DatePicker 可选的未来日期静默钳为今天。当前 mock 分桶模型下均为有意降级（Design Notes 已声明），接真实 SNAPSHOT 帧数据时应改为携带绝对日期并给未来日期行内提示。位置：`companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt`（TimeFilterChip/toDaysAgo）。
- **新 UI 的 densitySpec 接线观察项**：仪表盘活动统计节与设置页主动性节沿用固定 dp，未接 `densitySpec(InfoDensity)`；与全 app 多数既有组件一致（仅 3 处用 densitySpec），「沿用既有约定」条款两种解读下倾向合规。若后续统一密度双模式改造，这两节应一并纳入。位置：`companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt:417`、`ui/settings/SettingsScreen.kt:150`。

## Deferred from: code review of 12-3-relay-server-stateless-relay-and-docker (2026-08-28)

- **每帧转发抢全局 Mutex**（relay-server/src/forward.rs:63）：转发循环对每条 binary 帧调用 `registry.peer_sender()` 抢全局 tokio Mutex，数百并发下锁竞争为吞吐瓶颈。修法为本地缓存 peer sender + 槽位替换时刷新（`register()` 返回值正可用于此，当前被丢弃）。AC5 已由现有 20 对测试满足，待 LOADTEST.md 命令修正后有压测证据证瓶颈再优化。
- **全局连接数上限/速率限制**（relay-server/src/lib.rs:38-43）：无连接信号量/注册表条目上限，公网恶意海量连接可致内存无上限增长。AC5 只要求「数百并发」而非防御海量，属 V1 公网加固项（与 phone 槽抢占防御的 12.4 增强路径相关联）。

## Deferred from: code review of 12-4-android-scan-pairing-and-tri-state-connection (2026-08-28)

- **中继槽位抢占 DoS 与桌面重连放大**（companion-android/.../connection/RelayClient.kt + egosync-app/src-tauri/src/services/companion_connection.rs::relay_connect_and_serve/run_relay_client）：中继鉴权零知识，任何持有 16-hex relayId 的一方可注册为 phone 并把真手机挤出单槽位；桌面被拒后 `return Ok(())` + 无限退避重连，放大为持久握手-拒绝-重连 churn，可致永久中继拒服。根治需改 relay-server（phone 槽换绑/速率限制/连接数上限），与本清单 12.3 评审的「全局连接数上限/速率限制」项合并；V1 本地优先单设备、relayId 仅经面对面扫码流转，暂受风险。

## Deferred from: code review of 13-1-desktop-snapshot-engine-and-state-push (2026-08-29)

- **role:deleted/task:deleted payload 形状不一致**（egosync-app/src-tauri/src/commands/role.rs:194 发全对象；commands/task.rs:115 发裸 id）：快照引擎忽略 payload 故无功能影响；13.2 手机端开始消费这些事件时统一，届时需兼顾桌面前端既有监听方。
- **dataCutoffAt 混合 RFC3339 与 YYYY-MM-DD 字符串比较**（egosync-app/src-tauri/src/services/companion_snapshot.rs min_assign）：同日边界精度 <24h，属信息性字段；13.2 移动端展示语义确定时一并收口。
- **notify 通道满丢信号 / OnConnect 与 Write 处理顺序**（egosync-app/src-tauri/src/services/companion_snapshot.rs）：256 容量溢出在截断 O(n²) 修复后几乎不可达；STATE_DELTA 可能先于建连 SNAPSHOT 到达，由 13.2 手机端 FrameCodec 状态机语义收口（两帧载荷均为全量快照，功能无损）。

## Deferred from: quick-dev 三路评审 of spec-companion-offline-deadlock-recovery (2026-08-30)

- **自愈后无用户可见提示**：heal 清配对后直接落配对扫码流，应用内无一次性「配对信息损坏已重置」说明（仅 logcat warn），用户可能困惑为何要求重扫。修法：经容器 events（Snackbar）或 PairingScreen WELCOME 态注入一次性提示。位置：`companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt:200`。
- **容器层 flush 守门与导航接线无自动化测试**：`AppModelContainer` 恢复收集器（paired 守门）与 `AppNavHost` 遮罩门控/出口接线均为 Context/组合层依赖，项目无对应 JVM/Compose 测试基建（androidTest 仅 PairingSmokeTest），FR-43 无丢失守门当前仅代码+注释保障。修法：提取收集器为可注入协作对象或引入 Compose UI 测试。位置：`companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt:113`、`ui/AppNavHost.kt:136`。
- **自愈后旧快照/通知残留理论场景**：heal 在客户端层无法清容器层快照缓存与通知（遮罩出口路径的 unpair 已有此清理）；目标场景（mock 时代遗留、从未建会话）无缓存文件，实践中不可达。若未来出现「真实配对后元数据损坏」形态，应由容器观察 paired 跳变补清理。位置：`companion-android/app/src/main/java/com/egosync/companion/connection/RealConnectionClient.kt:200`。

## Deferred from: bmad-build 评审 of spec-companion-android-remove-onboarding (2026-09-10)

- **导航接线无执行级测试缝隙（评审发现：验证缺口）**：AppNavHost 冷启动装配行（`remember { coldStartDestination(container.connection.paired.value) }`，AppNavHost.kt:106）与配对完成导航（`onEnterApp` 的 `popUpTo(Routes.PAIRING){inclusive}` 清栈直达 DASHBOARD，AppNavHost.kt:254-261）均无执行级测试——`AppNavHostTest` 仅锁定 `coldStartDestination` 纯函数契约，装配行回归（接错标志/内联旁路重引首启门槛）或导航目标/清栈回归时全量套件仍绿。根因：仓库测试栈无 compose-ui-test/Robolectric/Navigation-Compose 测试依赖（依赖白名单约束），androidTest 仅 PairingSmokeTest 且不在常规验证路径（CI 只跑 `:app:testDebugUnitTest` + `:app:assembleDebug`）。关闭缺口需先立测试基建（测试依赖或仪器化导航用例），超出本移除任务边界。source_spec: `_bmad-output/implementation-artifacts/spec-companion-android-remove-onboarding.md`；evidence: 评审子代理 diff 全量追踪（33 个测试类无 AppNavHost 引用）+ `companion-android/gradle/libs.versions.toml` 无相关依赖 + `.github/workflows/release.yml:326-330` 任务清单。

## Deferred from: bmad-build 评审 of spec-fix-companion-chat-stream-ux (2026-09-11)

- **ChatScreen 渲染行为无执行级测试**：FR-29「流式中展开/历史折叠」、独立溯源条目布局、`initiallyExpanded` 翻转记忆等 Compose 渲染契约仅由 VM 状态断言间接覆盖（traceByMessageId/streamingTrace 值正确，渲染位置与折叠态无断言）。根因与既有缺口同源：仓库无 compose-ui-test/Robolectric 基建（依赖白名单约束），CI 常规路径仅 `:app:testDebugUnitTest` + `:app:assembleDebug`。修法：引入 Compose UI 测试依赖后为流式渲染区补渲染级用例，或以真机预览人审清单过渡。source_spec: `_bmad-output/implementation-artifacts/spec-fix-companion-chat-stream-ux.md`；evidence: 评审验证缺口层 mutation 实验全绿 + 全测试树 grep 无 ChatScreen 引用。
- **非查看会话的流式轮次整轮丢失执行溯源**：`onStreamState` 非查看 done 路径（ChatViewModel.kt `appendStreamSegments` 分支）落库段落但从不挂载 `s.traceBlocks`——用户流式中切走、done 落库后切回，该轮无任何 FR-29 溯源；桌面从 DB 回放 process 事件（ChatStream.tsx processEventsByMessageId）而手机快照无 trace 域，属跨端数据通道缺口非本变更引入。修法：非查看 done 分支同挂溯源（锚点 id 已可用）或快照 schema 增 per-message trace（见案件 backlog #6）。source_spec: `_bmad-output/implementation-artifacts/spec-fix-companion-chat-stream-ux.md`；evidence: 评审盲扫层 diff 追踪（挂载仅存在于 renderStream 查看路径）+ 桌面 ChatStream processEventsByMessageId 对照。
