---
title: 'Story 16.2: WEB 实时事件、流式体验与浏览器适配'
type: 'feature'
created: '2026-09-20'
status: 'done'
baseline_commit: 'a7d60dffcec21d01e57a70d2b68aaadc8db16bf7'
route: 'dispatch'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-16-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 16.1 交付了 WEB 入口与认证，但实时层只有 15.5 的传输基座、没有 UI 消费：连接状态不可见、重连补齐白名单零消费者、流式中途刷新无恢复呈现；375px 移动视口布局溢出（通知面板 380px 固定宽、模态 540px、双栏固定比例）、iOS Safari 100vh 抖动与 safe-area 零处理。

**Approach:** 纯前端 + e2e 故事——engine/server 契约零改动，在既有传输基座上补齐消费层：连接状态 hook 与指示器、`transport:reconnected` 的视图消费（含带参 query 定向重拉）、流式刷新恢复呈现、375px 响应式基线与 iOS Safari 缓解、wdio web 模式三条端到端用例。

## Boundaries & Constraints

**Always:**
- 服务端是事实源：刷新/重连后一律以 query 重拉为准；进行中会话按服务端落库状态呈现（is_complete=false 行 + 后续到达的 llm:stream 帧），不伪造 token 与进行态
- NFR-C3：浏览器不落业务数据——localStorage 维持仅 4 个既有 UI 偏好键，不新增任何浏览器存储
- 双宿主视觉零分叉（唯一允许差异 = 宿主门控项）；桌面分支行为零回归（既有测试套件守护）
- 连接状态三态（connecting/online/reconnecting）诚实呈现，禁止静默失败
- index.html 变更后 CSP 契约测试必须仍绿（内联脚本 hash 不得漂移）

**Never:**
- 不加引擎命令/事件名/不扩重放白名单（含 busy 查询命令——见 Design Notes 决策）
- 不重放写入类命令；不引入像素对比工具；不做 PWA/Service Worker/推送
- 不动桌面 tauri-driver e2e 链路（既有 9 个 spec 零改动全绿）
- 不做 UX 定稿级移动形态（断点方案归 UX 勘注定稿，本故事只交付可用基线）
- onboarding 流式的刷新恢复不在范围（一次性流程，重走即恢复）

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 在线流式 | 浏览器登录后发对话 | llm:stream 逐 token 渲染、stop 可用、同会话 busy 提示一致（Ok 通道语义透传） | 流错误走既有 done 帧复位 |
| 断线 | 网络中断/服务器重启 | 徽标+横幅呈「重连中」；EventSource 原生重连 + 白名单重放 + `transport:reconnected` → 视图恢复最新态 | 持续失败则持续呈重连中，不静默 |
| 流式中途刷新 | 流式进行中按 F5 | 历史重拉：完整消息如实呈现；is_complete=false assistant 行呈「生成中」占位（非空气泡）；流仍在进行则后续 token 帧 按 messageId 归位；done 后重拉补齐 | 无帧到达则占位停留，不伪造内容 |
| 375px 视口 | 手机浏览器走核心界面 | 导航/对话/任务/仪表盘/通知面板不溢出、可操作（堆叠/收缩基线） | N/A |
| iOS Safari | 移动 Safari 完整使用 | EventSource 流式正常、httpOnly Cookie SameSite=Strict 正常携带、100vh 抖动缓解（dvh 优先+回退）、viewport-fit=cover + safe-area | 不支持 dvh 的浏览器回退 100vh |
| 视觉零分叉 | 桌面浏览器 vs 桌面版并排截图 | 差异仅宿主门控项；浏览器侧截图留档；桌面侧本环境不可产（16-1 先例：环境限制留档+组件测试守护+真机补拍建议） | N/A |

</frozen-after-approval>

## Code Map

**复用基座（勿动契约）：**
- `egosync-app/src/transport/http.ts:138-150,209-247,303-320` — onConnectionStateChange（订阅即回当前态）、SSE 懒建/致命关闭 5s 重建、replayAfterReconnect→`transport:reconnected`（31 条白名单结果集）；**http.ts 目标零改动**
- `src/transport/types.ts:11,77` — ConnectionState 三态与 Transport 接口；`src/transport/localEvents.ts:17` — transport:reconnected 已注册，进程内总线
- `src/hooks/useEngineEvent.ts:11` — hook 范式（新 hook 仿此形状）；宿主分支在 `transport/index.ts:33` getTransport
- `src/transport/capabilities.ts`、`events.ts` — 生成物禁手改（gen:transport）
- `server/src/sse.rs` — SSE 已完备（帧同构/30s 心跳/Lagged 断慢端/idle 120s），零改动
- `crates/egosync-engine/src/commands/chat.rs:258-272,469,501` — busy 消息、chat_get_history、chat_list_conversations；`agent_engine.rs:934,2641-2654` — done 帧契约、token 只进内存、落库仅在终止点

**修改面：**
- `src/components/chat/ChatStream.tsx:727-765,852-1011,1013` — 初拉/refreshTrigger/handleStreamEvent/llm:stream 订阅：刷新恢复与去重在此做
- `src/components/notifications/NotificationPanel.tsx:41`（w-[380px]）、`src/components/layout/Modal.tsx:11`（w-[540px]）及调用侧宽度、`role/RoleView.tsx:89,106`、`butler/ButlerView.tsx:114,155`（65/35 双栏）、`butler/DashboardTab.tsx:326`（grid-cols-2）、`role/RoleHeader.tsx:44`（px-8）— 响应式修改点
- `index.html:5`（viewport meta）、`src/index.css:58`（body overflow-hidden+h-screen）、`App.tsx:352`、`auth/AuthGate.tsx:93,101`、`auth/LoginView.tsx:52`、`auth/SetupView.tsx:80` — dvh/safe-area 面
- `egosync-app/tests/e2e/` — wdio.conf.ts:65-90 桌面链路（勿动）、helpers/app-helper.ts:7-21 依赖 __TAURI_INTERNALS__（web 需另写探测）；9 个既有 spec 零改动

**e2e 基建事实：** wdio 9、无 playwright、puppeteer 仅 pa11y 传递依赖（需显式声明）；16.1 冒烟脚本未入库需重写；SSE 长连接使 networkidle0 永不触发（用 domcontentloaded）；认证面限流 5 次/min 与脚本共用预算须留窗口。

## Tasks & Acceptance

**Execution:**

- [x] `egosync-app/src/hooks/useConnectionState.ts`（新）— 包装 onConnectionStateChange 暴露三态；Tauri 分支恒 online（types 契约既有行为）
- [x] `egosync-app/src/components/layout/ConnectionStatus.tsx`（新）+ App 布局挂载 — online 常驻低显著度徽标、reconnecting 显著横幅（含重连中文文案）、connecting 短暂态
- [x] `src/components/chat/ChatStream.tsx` — 刷新恢复三件：is_complete=false 行渲染「生成中」占位；streamBubbles 与历史行按 messageId 去重（不双显）；订阅 `transport:reconnected` 定向重拉当前会话 chat_get_history（带参命令不在白名单）
- [x] 视图重连消费（useNotifications/useTasks/useAllTasks/useDashboard/refreshAllRoles 所在文件）— 订阅 `transport:reconnected`，消费白名单重放结果集或以其为重拉触发器，使通知/任务/仪表盘/角色列表重连后反映最新态；写入类零重放
- [x] 响应式基线（Code Map 修改面所列文件）— NotificationPanel/Modal 收缩为 w-full+max-w、RoleView/ButlerView 小屏单栏堆叠、DashboardTab 网格塌缩、px-8 边距收缩；375px 无横向溢出
- [x] `index.html` + `src/index.css` + `App.tsx`/`auth/*` — viewport-fit=cover；h-screen 改 dvh 优先 + @supports 回退 100vh；对话输入区底部 safe-area inset；内联脚本零改动（CSP 契约测试确认仍绿）
- [x] `tests/e2e/wdio.web.conf.ts`（新）+ `helpers/web-helper.ts`（新）+ `specs/web-streaming.spec.ts`、`specs/web-events.spec.ts`、`specs/web-reconnect.spec.ts`（新）— 起本地 server（真实二进制+dist）+ chromium headless；流式/事件/断线重连各 ≥1 条；就绪探测走 healthz+DOM 标记不依赖 Tauri internals；`tests/e2e/package.json` 增 `test:web` script
- [x] vitest 补强 — useConnectionState 三态矩阵、ConnectionStatus 呈现矩阵、ChatStream 刷新恢复（占位/去重/定向重拉）、重连视图消费；既有 757 用例零回归
- [x] 冒烟走查 — `cd server && EGOSYNC_STATIC_DIR=../egosync-app/dist cargo run` + 浏览器脚本三场景：在线流式、拔线重连、流式中途 F5；浏览器侧截图留档 `_bmad-output/implementation-artifacts/screenshots/16-2/`

**Acceptance Criteria:**

- Given 浏览器登录态，when 流式对话进行/通知到达/任务分类/仪表盘变化，then 全部事件驱动视图实时更新，体验与桌面一致
- Given 断线，when SSE 断开，then 连接状态可见呈现（徽标+横幅）；重连成功后白名单重放使视图反映最新态，写入类零重放（无重复确认副作用）
- Given 流式中途刷新，when F5 后重拉，then 已落库消息完整呈现、未完成行呈生成中占位、不伪造 token；流终止后自动补齐完整回复
- Given 375px 视口与 iOS Safari，when 走查核心界面与完整流程，then 布局不溢出可操作、流式正常、Cookie 正常、100vh 抖动有缓解
- Given e2e web 模式，when 执行 `npm run test:web`，then 流式/事件/断线重连用例全过；桌面 e2e 链路零改动全绿
- Given 全量验证，when 执行 build + vitest + server cargo test，then 全绿（server/engine 源码零 diff）

## Implementation Notes

（实现期追加——决策、触碰文件、意外发现）

### 决策记录

- **Tailwind 实为 3.4.19**（非 lockfile 面上的 3.4.17 假设）：`h-dvh`/`max-md:`/`min-[480px]:` 全部可用。dvh 采用「基类 `h-screen` + `supports-[height:100dvh]:h-dvh` 变体」双声明共存（twMerge 不同 variant 不冲突）——比 `@supports` CSS 块更贴近既有工具类风格；`index.css` body 侧保留声明级 `height:100vh; height:100dvh;` 渐进增强。
- **刷新恢复去重双规则**（ChatStream `hiddenPendingResumeIds`）：规则A（活跃流式桶 id 与历史行 id 相同——委派 follow-up 桶 id = assistant 占位行 id）+ 规则B（`isStreaming` 时末位用户消息之后的未完成助手行 = 在途流占位行）。规则B 以「最后一条用户消息」为边界——更早的死流占位行不受影响，按服务端状态如实停留「生成中」（诚实呈现，不伪造）。**无 localSendInitiatedRef**——「最后一条用户消息」即边界，推演覆盖 busy 插入、委派 mid-done、本地发送后死流占位、切换会话返回、多窗口。
- **done 就地补全**（恢复流 final done 分支）：本地完成段属于恢复流时（末位未完成助手行存在），就地补全该行（content/thinking/isComplete）而非追加 `__completed__N_final` 本地消息——后者会在历史重拉 merge 后与权威全文行双显（本地 id 与服务端行 id 无关联、内容仅为全文后缀）。本地发起的流（桌面既有路径）无占位行，追加逻辑零变化。过程事件回填键也随之对齐服务端行 id。
- **响应式类口径**：断言桌面行为的既有测试（`RoleView.test` 的 `w-[35%]` 等）与新响应式并存的正确写法 = 「≥md 维持桌面值（`md:w-[35%]`）+ 小屏塌缩（`max-md:w-full` / 基类 `w-full h-[58%]`）」。RoleView 测试断言随响应式基线更新为 `md:w-[35%]` + `h-[42%]`（守护语义「单一宽度过渡、无 slide-in 叠加」不变，非删测试）。
- **e2e web 浏览器驱动**：环境无 chromedriver；网络可达 chrome-for-testing-public（HTTP 200 验证），钉版下载 chromedriver 148.0.7778.97（与 puppeteer 缓存 Chrome for Testing 148 精确匹配）到 `tests/e2e/.chromedriver/`，wdio 走原生 webdriver 协议（`goog:chromeOptions.binary` + `wdio:chromedriverOptions.binary`）——不引入 devtools protocol / 新依赖。
- **e2e 认证限流预算（5 次/min/IP 滑动窗口）**：`onPrepare` 首轮起服建库 → kill → node:sqlite 直写 `onboarding_completed=true`（绕过无 LLM 环境下 onboarding UI 的「配置大模型服务」死路，与桌面 `seedCompleteOnboarding` 同语义、零认证请求）→ 重启。3 specs 借持久 user-data-dir 保 Cookie；非首个 spec 加载前静默 61s 等上一窗口条目老化（`auth-window-opened` 标记驱动）——每 spec status(+login) ≤3 次恒在预算内，结构化确定性。
- **e2e 断线控制**：onPrepare 在 launcher 进程、specs 在 worker 进程（模块单例不共享）——服务端 PID 文件跨进程控制（kill 读 PID SIGKILL；restart 同 env 重启并覆写 PID）。SIGKILL 后以 healthz 探测端口关闭（进程死 ≠ socket 释放）。
- **无 LLM 环境的流式确定性锚点**：`chat_send_message` 先插 user 行（complete）+ assistant 占位行（is_complete=false，token 仅内存、终止才落库）→ 错误路径下占位行永远未完成——web-streaming spec 以「F5 后 pending-generation-placeholder」断言刷新恢复，与桌面 llm-streaming spec 的静态契约限制说明同源。冒烟「在线流式」场景同理以在线徽标+管家视图留档（web-01），真实 token 流需接 LLM 后人工补验。
- **web specs 必须与桌面 specs 分目录**：桌面 wdio.conf.ts 的 `specs: ['./specs/**/*.ts']` glob 会吞掉 specs/ 下任何新增 spec——web specs 放 `web-specs/` 兄弟目录，桌面链路配置与既有 9 specs 零改动（零回归约束的结构性保证）。
- **冒烟截图四张**（`_bmad-output/implementation-artifacts/screenshots/16-2/`）：web-01-online（在线徽标+管家视图）、web-02-reconnecting（拔线重连横幅）、web-03-refresh-resume（流式中途 F5——「生成中」占位+用户消息）、web-04-375px-butler（375px 无横向溢出——e2e 内真实断言 scrollWidth ≤ clientWidth）。桌面侧本 headless VM 不可产（16-1 先例），桌面分支由组件测试守护。

### 触碰文件

前端（src）：`hooks/useConnectionState.ts`（新）+ `.test.ts`（新）、`components/layout/ConnectionStatus.tsx`（新）+ `.test.tsx`（新）、`components/layout/Sidebar.tsx`（底部 rail 挂载徽标）、`components/chat/ChatStream.tsx`（刷新恢复三件 + 重连重拉 + 输入区 safe-area）、`components/chat/ChatBubble.tsx`（PendingResumePlaceholder + isPendingResume）、`transport/localEvents.ts`（TransportReconnectedPayload 类型）+ `transport/index.ts`（re-export）、`hooks/useNotifications.ts`/`useTasks.ts`/`useAllTasks.ts`/`useDashboard.ts`/`App.tsx`（重连消费）、`components/notifications/NotificationPanel.tsx`、`components/layout/Modal.tsx`、`components/role/RoleView.tsx`/`RoleHeader.tsx`、`components/butler/ButlerView.tsx`/`DashboardTab.tsx`/`ButlerSettingsContent.tsx`、`index.html`（仅 viewport meta）、`src/index.css`（body dvh）。

测试：`hooks/transportReconnected.consumption.test.tsx`（新 13 例——评审轮 +5 错误清理/静默失败例）、`components/chat/ChatStream.test.tsx`（+12 例 16.2 段——评审轮 +4 showActions 占位/流式复位/convId 空重试/角色过滤例）、`components/role/RoleView.test.tsx`（响应式断言口径更新）、`hooks/useDashboard.test.ts` / `components/butler/DashboardTab.test.tsx` / `DashboardTab.a11y.test.tsx`（transport mock 补 `getTransport`——useDashboard 新增订阅所需，与 useNotifications.test 既有模式一致）、`App.test.tsx`（评审轮 +2 transport:reconnected 角色列表消费例）、`components/layout/ConnectionStatus.test.tsx`（评审轮 +1 可达性/层级钉例）。评审更正：ChatBubble.test.tsx 实际零改动——isPendingResume 的渲染行为经 ChatStream.test 的 `pending-generation-placeholder` testid 穿透覆盖。

e2e：`wdio.web.conf.ts`（新）、`helpers/web-helper.ts`（新）、`web-specs/web-streaming.spec.ts`、`web-specs/web-events.spec.ts`、`web-specs/web-reconnect.spec.ts`（新，置于 specs/ 的兄弟目录——桌面链路 `specs/**/*.ts` glob 零改动即不吞 web specs）、`package.json`（`test:web` script + 评审轮 `setup:web-drivers` + engines ≥22.5）、`scripts/setup-web-drivers.mjs`（评审轮新——钉版 chromedriver 供给脚本）、`tsconfig.json`（include 增 web-specs + web conf）。**桌面链路 wdio.conf.ts / 既有 9 specs / app-helper 零改动**。

零改动（事实源冻结）：`http.ts`、`tauri.ts`、`capabilities.ts`、`events.ts`、`server/`、`crates/egosync-engine/`、`index.html` 内联脚本（CSP hash 契约）。

### 意外发现

- **mid-run 编辑 e2e 配置的连锁故障**（已修复）：onPrepare 若以包装进程 spawn（如 strace），kill 直子进程会留孤儿真服务进程持端口——后续重启实例 bind 冲突即死、登录 INSERT 间歇失败（「会话写入失败」）。教训：跑 e2e 期间不动 e2e 配置；onPrepare 的 kill-then-seed 改为「等端口真正关闭」（`waitForPortClosed`）再直写 DB。
- **useDashboard 引入 useEngineEvent 后三个既有测试文件失败**：mock `@/transport` 只有 `invoke` 无 `getTransport`——补 mock（零事件投递）即可，断言不动。
- **wdio 9 自动驱动管理的可用性**：`@wdio/utils` 的 `startWebDriver` 支持 `wdio:chromedriverOptions.binary` + `goog:chromeOptions.binary` 显式指定双二进制路径，不走任何下载路径（网络安装被禁的场景完全可用）。
- **auth_sessions 表在 egosync.db**（不在 conversations.db）——e2e 的 DB 直写（onboarding 预置）与 node:sqlite 检查均对 `egosync.db`。

### 父代理验证轮（2026-09-20 step-03 diff 级复核）

- **web-streaming 刷新恢复用例间歇性抖动**（子代理两轮绿 ≠ 稳定；父代理 8 轮全量跑复现 3 次失败，含修复中途 1 次）：失败时占位符元素已在 DOM 且可见（诊断转储：`convCount:1`、innerText 含「生成中…」），但轮询停摆至超时。取证链（时间戳注入 → 单轮复现）：末次有效轮询 +15s 整=超时时刻；`badge-online` 距刷新仅 66–137ms（弱锚点）。三重根因，三重修复：
  1. **弱锚点（测试）**：「等输入框恢复可用」可在 React 禁用输入前通过 → 刷新赶在落库可见前重拉。修复：服务端事实源轮询（REST `chat_get_history` 直至未完成助手行确认落库再刷新）。
  2. **元素引用陈旧 + 环境停摆（测试）**：刷新后应用启动中 React 重渲染会替换 DOM 节点；且本 VM（3.6GB + swap 899MB）三连套件下渲染线程偶发 ~15s 停摆（无 OOM kill、无应用错误、元素最终必现——环境负载非产品缺陷）。修复：每轮 poll 重新 `$()` 查询 + 预算 15s→30s。修复后三连跑 3/3 全绿。
  3. **徽标初始态不诚实（产品小瑕疵，e2e 锚点因此弱化）**：`useConnectionState` 初始硬编码 `'online'`，浏览器启动首帧闪现错误「在线」（真实态=连接中）。修复：惰性初始 `isTauriHost() ? 'online' : 'connecting'`——三态诚实呈现补全；徽标等待从此只在真正建连后通过。测试随增 Tauri 宿主初始态例（782→783）。
- **`.chromedriver/`（20MB 钉版下载物 + 16572 行第三方声明）误入 git 追踪**：父代理 diff 审查发现（intent-to-add 后 diff 膨胀至 19k 行），补 `tests/e2e/.gitignore` 条目并撤销暂存——运行时下载物同 node_modules 语义，不入库。
- **独立复核全部命令**（不采信子代理自述）：build ✅ / vitest 783/783 ✅ / server cargo test 45 用例 ✅ / test:web 3 specs 8 tests ×3 连跑 ✅；桌面 e2e 阻断项与子代理结论一致（结构性零改动佐证成立）。


## Spec Change Log

## Review Triage Log

三层评审（盲扫 15 / 边界 16 / 验证缺口 3+1）已于 2026-09-20 分诊；每条经父代理到源码逐点核实后定谳（不采信评审自述等级）。同源发现已合并。

**patch — 产品代码：**

- **ChatStream renderMessage（showActions 分支）缺 isPendingResume** — 盲扫+边界同报 — high：建议卡/任务分解卡与未完成助手行同屏（刷新恢复+历史遗留 action）时占位行渲染回「空气泡」，本故事 Intent 要消除的形态在半数渲染分支回归 — 已修：renderMessage 补齐同一谓词。
- **handleTransportReconnected convId 空时早退，吞掉全部恢复** — 边界报 — high：初次会话解析在断线期间失败（页面于宕机期加载）→ 重连后聊天区永久空白、输入锁死，无自愈路径（初始化 effect 仅挂载时跑一次）— 已修：提取 initializeConversation，convId 空时走重试。
- **流式中途断线 → UI 永久卡死流式态** — 盲扫报 — high：服务器重启后 done 永不到达，isStreaming/输入锁/冻结气泡永不复位（I/O 矩阵「服务器重启」输入行）— 已修：reconnected 时 isStreaming 则 resetStreamingState（清场+解锁），服务端真值由历史重拉呈现（死流行诚实停留「生成中」）。
- **chat_list_conversations 重放结果未按角色过滤** — 边界报 — medium：角色视图重连后会话列表串台他角色会话 — 已修：roleId 存在时过滤重放结果集。
- **4 个 hook 重放消费成功后不 setError(null)** — 盲扫+边界同报 — low：初载失败的错误横幅与重连后的新鲜数据同屏常驻 — 已修：重放消费（与回退重拉成功）路径补 setError(null)。
- **useDashboard 重连回退失败只 console.error（与初载路径不一致）** — 边界报 — low：静默失败违背 NFR-C7 口径 — 已修：回退 catch 补 setError(DASHBOARD_LOAD_ERROR)。
- **内联对话框硬宽度漏改（380/480px 于 375px 溢出）** — 盲扫报 — medium：ButlerSettingsContent 两处（380/480）+ SettingsTab 一处（380）未走共享 Modal 的移动宽度处理；Sidebar 确认框 360px 贴边 — 已修：统一补 max-md 收缩类（与 MODAL_MOBILE_WIDTH 同模式）。
- **重连横幅 top-0 无 safe-area-inset-top、z-40 被通知面板（z-50）全遮、徽标 div 无 role** — 边界+盲扫报 — low：iOS 刘海遮挡 + 面板开着时断线提示不可见 + 读屏不播报 — 已修：横幅 top 侧安全区 + z-[60] + 徽标 role="status"；通知面板补 right 侧安全区（iOS 横屏）。
- **App 根容器无左右 safe-area inset（iOS 横屏刘海下）** — 边界报 — low — 已修：根容器补 pl/pr env() 内边距（非刘海环境零影响）。

**patch — 测试与 e2e 基建：**

- **App.tsx transport:reconnected 消费零测试** — 验证缺口报（预核实）— medium：整段删除无任何验证路径失败 — 已补：App.test.tsx 增两例（重放命中直接消费不重查 / 缺失回退 refreshAllRoles）。
- **375px 断言结构性恒真 + 只覆盖管家聊天视图** — 盲扫+验证缺口同报 — medium：scrollWidth 双层 overflow-hidden 裁剪下几乎恒真；通知面板/模态收缩无守护 — 已修：改 getBoundingClientRect 视口内断言 + 375px 用例扩至通知面板与共享 Modal（添加角色）。
- **web e2e 登录探测一次性 isExisting 竞态** — 盲扫报 — low：冷加载慢时误判免登录 → 徽标 30s 超时误报 — 已修：等「登录表单或徽标」二选一有界等待。
- **restartWebServer spawn 无 error 处理 / waitForPortClosed 超时事件未处理 / mocha 120s 贴 61s+30s+30s 上限 / 无 --headless 依赖 autoXvfb / 截图直写受控目录弄脏工作树 / chromedriver 供给无可执行脚本** — 盲扫+边界报 — low~medium（测试基建健壮性与可复现性）— 已修：spawn error 拒绝化、socket 超时重试、mocha timeout 180000、args 补 --headless=new、截图改写 logs/web/screenshots（_bmad-output 留档冻结为本次验证证据）、scripts/setup-web-drivers.mjs + engines 声明。
- **spec「触碰文件」声称 ChatBubble.test.tsx 改动（实际零 diff）** — 验证缺口报 — low（记录失实）— 已修：触碰文件一节更正（isPendingResume 渲染行为经 ChatStream.test 穿透覆盖）。
- **epic-16-context 仍写「UI 只订阅连接状态」** — 盲扫报 — low（架构口径失真）— 已修：更新为实际落地模式。
- **sprint-status 16-2 状态与故事不同步** — 盲扫报 — low — 已修：置 review（收口时随 step-05 置 done）。

**defer（超出本故事冻结范围，入 deferred-work.md）：**

- test:web 的 CI 接线（Linux job + 供给步骤）——CI 改动超出本故事边界，供给脚本已先行落地。
- 记忆/使命宣言/晨间简报/周复盘等面的重连重放消费——I/O 矩阵「断线」行枚举为通知/任务/仪表盘/角色列表四视图+聊天，均已落地；更广覆盖属后续增强。
- 恢复流 done 后历史重拉失败静默（console 有痕、刷新即愈的双失败窗口）。

**reject（核实后不成立）：**

- 「徽标 <640px 无文字」——侧栏 rail 物理宽度 64px，文字本无处安放；图标+横幅承载语义，非缺陷。
- 「PID 回收误杀无关进程」——pidFile 为单次运行内写入，运行期内 PID 回收不可信发生；保险性 /proc cmdline 校验收益不成比例。

### 评审修复执行记录（2026-09-20 step-04 fix loop）

全部 patch 项已落地并全量验证（build ✅ / vitest 796/796 / test:web 连续 11 轮全量绿）。修复执行中的新发现与二次修复：

- **e2e 375px 用例的关闭策略缺陷（自查）**：移动端通知面板 w-full 全屏覆盖，侧栏铃铛被盖住——二次点击铃铛关闭会点击拦截。改经面板关闭按钮；顺带补 `aria-label="关闭通知中心"`（图标按钮无可访问名称的 a11y 缺口，既有测试以 `name: ''` 定位恰好编码了此缺陷，已随更新为实名定位）。
- **e2e 铃铛选择器套件序失配**：events spec 留下未读通知 → aria-label 动态变「有新通知」→ 固定选择器失配。改 `title="通知"`（恒定属性）。
- **e2e 刷新恢复消息内容改每轮唯一**（含时间戳）：累计 DB 下同文案旧行使「未完成行」轮询锚点立即通过、刷新赶在本轮落库前执行（三连跑第 3 轮复现一次）。
- **启动窗口空聊天死态（产品级二次修复，e2e 取证）**：套件序下偶发（~1/6）「刷新后徽标 online、输入框在场、但聊天区空壳（bodyTextLen=86）」45s 不愈——服务端日志定位到 send 触发的 sidecar 重启 + 系统提示词构建与刷新后的启动 REST 竞争，**单次瞬时 REST 失败 → initializeConversation 无重试 → 永久空白（对真实用户同样成立：本地服务器忙时刷新页面 → 空聊天死态）**。修复：initializeConversation 有限重试自愈（最多 3 次尝试、1.5s 退避、代际守卫取消——重连恢复/角色切换发起的新初始化自然作废旧重试）+ 单测钉（瞬时失败 ×2 后第三次成功自愈）。
- **e2e UI 导航点击改 JS click**：套件序下应用在 SSE/数据加载窗口的布局微移使 WebDriver 命中测试瞬态拦截（实测 5 秒 3 连拦截、scrollIntoView 越界）；JS click 绕开命中测试，断言严格性不变（溢出断言仍逐元素矩形级）。
- **诊断转储**：刷新恢复等待失败时随错误呈应用状态（徽标态/输入框/正文字长/消息数/REST 直查结果）——未来失败可判因（区分「应用半启动」vs「元素不可见」vs「服务端空」）。

## Design Notes

- **流式中途刷新不加引擎命令（决策）**：架构裁决「云端不新增事件名、不做历史回放」，busy 查询属契约扩张。is_complete=false 空行 + 重连后继续到达的 llm:stream 帧 + done 后重拉，已满足「按服务端状态呈现、不伪造进行态」。
- **transport:reconnected 分工（决策）**：白名单 31 条无参 query 由 transport 层重放（既有）；视图消费重放结果集或以其为触发器重拉均可——优先直接消费结果集避免重复查询；带参 chat_get_history 由 ChatStream 定向重拉，不扩白名单。
- **连接状态形态（决策）**：常驻低显著度徽标 + reconnecting 显著横幅；退避由浏览器 EventSource 管（架构④：应用层只管状态呈现）。
- **dvh 方案**：先确认安装的 Tailwind ≥3.4 与否（h-dvh 可用性）；否则 CSS `min-height:100dvh` + `@supports not (height:100dvh)` 回退。
- **e2e web 断线用例锚点**：以连接状态 DOM/`transport:reconnected` 为断言锚点；SSE 测试须先订阅后触发（broadcast 无重放——server 测试先例）；headless chromium 需 `--headless=new`。
- **视觉零分叉留档**：桌面侧本 headless VM 不可产（16-1 已证 WebKitGTK 黑屏）；浏览器侧截图 + 组件测试守护桌面分支 + 真机补拍建议，同 16-1 处置。

## Verification

**Commands:**
- `cd egosync-app && npm run build` — expected: tsc 零类型错误 — **✅ 2026-09-20 实测通过**（vite 产物构建完成；验证轮修复 useConnectionState 与评审修复轮后重跑均绿）
- `cd egosync-app && npx vitest run` — expected: 既有 + 新增全绿 — **✅ 2026-09-20 实测 796/796**（757 基线 + 39 新增：useConnectionState 5 / ConnectionStatus 6 / ChatStream 刷新恢复+评审 13 / transport:reconnected 视图消费 13 / App 重连消费 2；含 csp.contract 5/5——内联脚本 hash byte 对 byte 不变；评审修复轮后独立重跑）
- `cd server && cargo test` — expected: 全绿（佐证 server 零源码改动） — **✅ 2026-09-20 实测通过**（单测/集成/Doc-tests 0 failed；父代理独立重跑；评审轮 server 零触碰）
- `cd egosync-app/tests/e2e && npm run test:web` — expected: 三条 web 用例通过 — **✅ 2026-09-20 实测 3 specs / 8 tests 全过**（SSE 事件 1、断线重连 3、流式+刷新恢复+375px 4——375px 用例经评审强化：矩形级溢出断言 + 通知面板 + 共享 Modal 覆盖）；**评审修复全部落地后连续 11 轮全量绿**（含期间发现并修复的启动窗口空聊天竞态，详证 Review Triage Log 执行记录段）
- `cd egosync-app/tests/e2e && npm test` — expected: 桌面链路零回归 — **⚠️ 本 VM 环境阻断（非代码回归），详证如下**：tauri-driver 会话创建全部超时（`POST http://127.0.0.1:4444/session` aborted），日志三重环境症状：①`Authorization required, but no authorization protocol specified`（预置 Xvfb :99 带 -auth，跨会话无凭据）；②上一轮遗留 tauri-driver 进程持 4444/4445 端口（`can not listen to address`）；③清理端口 + 自起 `Xvfb :98 -ac` 后重试仍超时。结构性佐证桌面链路零改动：桌面 wdio.conf.ts `specs: ['./specs/**/*.ts']` glob 不吞 web specs（web specs 在 `web-specs/` 兄弟目录，`grep web- specs/` 零命中）、桌面 9 specs 与 app-helper 零 diff（git status 佐证）、应用二进制本身在 Xvfb :98 正常启动（引擎双库迁移完成、无 GTK panic——阻断在 WebDriver 会话层而非应用层）。连续 3 次失败按项目规则停止尝试；需在有正常 X 显示/授权的机器（或 CI）实跑收口。

**Manual checks (if no CLI):**
- 冒烟三场景（在线流式/拔线重连/流式中途 F5）+ 核心体验一档清单走查：流式/任务/建议确认拒绝/仪表盘/简报复盘 — **浏览器侧三场景已由 web e2e spec 自动化覆盖并截图留档**（screenshots/16-2/ 四张）；真实 LLM token 流与建议确认拒绝/简报复盘需接 LLM 后人工补验（本环境无 LLM API Key，桌面 16-1 限制同源）
- 375px 视口（DevTools 模拟，如有 iOS 真机再补真机走查）导航/对话/任务/仪表盘不溢出可操作 — **✅ e2e setWindowSize(375,812) 断言 scrollWidth ≤ clientWidth + 截图留档**；iOS 真机 Safari 走查建议真机补验（dvh/safe-area 已按 viewport-fit=cover + env() 实装）
