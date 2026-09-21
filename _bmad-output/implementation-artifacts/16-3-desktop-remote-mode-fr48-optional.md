---
title: '桌面客户端远程模式（Story 16.3，FR-48）'
type: 'feature'
created: '2026-09-20'
status: 'in-review'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: 'e285060c90c9a2311024863dd09ac188b9f2965b'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-16-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 云端实例已可部署（Epic 15/16 完成），但桌面应用只能跑本地引擎——已部署云端的用户在桌面上要维持两个入口，数据形态割裂。

**Approach:** 桌面应用增加「远程模式」：设置中配置实例 URL+令牌、确认诚实代价后切换（重启式），桌面变为该实例的客户端（HttpTransport 指向远端，语义与浏览器等价）；可随时切回本地。切换 = 连接目标变更，永不合并/同步两侧数据；重启桌面 app 记忆上次模式。

## Boundaries & Constraints

**Always:**
- 状态机按架构裁决 B：LOCAL→REMOTE_CONNECTING→REMOTE_ONLINE；不可达→REMOTE_OFFLINE 指数退避（1s→30s 封顶+抖动）；切回本地=显式动作+本地引擎以既有本地数据重启
- LOCAL→REMOTE 守卫：本地引擎完整停机后才进远程态（重启式以进程退出达成，强于运行中停机）
- 模式在进程生命周期内恒定（切换必经重启），前端零热换传输
- 远程模式零本地业务写入：不打开 egosync.db/conversations.db、不写 opencode-workspace、不启 sidecar/调度器/伴侣/委派桥/双 watch
- 远程态桌面 = 浏览器等价物：desktop-only 业务能力按 web-ok 集隐藏；选工作目录按 AC 禁用并说明原因
- 浏览器与本地桌面零回归：http.ts 浏览器路径字节级不变、既有 vitest/web e2e/桌面组件测试全绿
- 服务器浏览器语义不变：cookie 登录/SSE 既有路径零变化；Bearer 与 CORS 为叠加通道
- 重启前显式执行既有退出清理（watchdog cancel + sidecar.stop）；连接池不显式关闭（与现状退出路径等价）
- 切换方式裁决（人工 2026-09-20）：重启式——确认对话框 → 持久化模式配置 → 既有退出清理 → `app.restart()` 进新模式；一生低频操作，不为省数秒重启引入脊柱级改动

**Never:**
- 不做运行中引擎热停机/热换传输（调度器/双 watch/EventRouter/伴侣引擎补取消令牌 + 池关闭重建——超出本故事范围，如未来要做另立故事分步交付）
- 不写任何数据迁移/合并/同步代码路径（NFR-C5 断言：切换路径只触碰模式文件、keyring、重启）
- 不在远程模式向本地写业务数据；令牌不落明文磁盘（仅 keyring）
- 不削弱既有 cookie 安全语义（HttpOnly/SameSite=Strict 不动）；SSE 不以令牌明文入 URL（一次性短时票据）
- 不做多远程实例档案、不做多用户

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 本地→远程切换 | 设置远程 tab 填 URL+令牌，测试连接返回 authenticated | 允许切换；确认诚实代价文案后持久化配置并重启进远程 | 测试失败→切换按钮禁用+原因 |
| 远程模式启动恢复 | desktop-mode.json 为 remote 时重启 app | 引擎装配整体跳过；前端直接远程引导（AuthGate） | 文件缺失/损坏→回退本地模式（fail-safe） |
| 远程不可达 | 启动或运行中远端失联 | REMOTE_OFFLINE：徽标+横幅+指数退避重连；离线屏有「重试」与「切回本地」逃生口 | 网络错误≠401，不误报令牌失效 |
| 令牌失效 | 远端返回 401 | 令牌重录视图（存 keyring 后重验） | 重录仍 401→如实错误文案 |
| 远端未初始化 | status 返回 setupRequired | 走 setup 向导（浏览器等价） | 可中途切回本地 |
| 切回本地 | 远程模式设置确认 | mode=local 持久化+重启；引擎以既有本地库启动 | 本地库迁移照既有路径 |
| SSE 断线重连 | EventSource 致命关闭 | 重取票据+重建连接，退避 1s→30s+抖动 | 票据过期→重取，不升级为离线 |
| 切换取消 | 确认对话框点取消 | 零变更零重启 | N/A |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/lib.rs` — setup 闭包 L42-454 为引擎装配序列（init_db L58→池 manage L66-75→opencode 同步链→sidecar→EventRouter→watchdog/委派桥→双 watch→companion→scheduler L407→EngineCtx L419-454）；invoke_handler L458-581；`RunEvent::Exit` L584-596 仅 watchdog cancel+sidecar.stop（远程模式 Exit 分支不得取未 manage 的 state——会 panic）
- `crates/egosync-engine/src/services/sidecar.rs` — start:381 / stop:569（幂等，8.6 加固）/ health_check:648；调度器 scheduler.rs:346、双 watch task_deadline_watch.rs:90 / task_protection_watch.rs:79、EventRouter event_router.rs:68 均无取消令牌（远程模式整体不启动即可，无需补令牌）
- `server/src/auth.rs` — login:258 收 `{token}`、Argon2id/env 校验、发 HttpOnly SameSite=Strict cookie；`/api/auth/status`:170 返 `{setupRequired,authenticated}`；会话表 SHA256 哈希
- `server/src/sse.rs` / `routes.rs` — SSE 帧同构/心跳/idle；路由经 dispatch_gen 白名单（desktop-only 物理 404）
- `egosync-app/src/transport/index.ts` — isTauriHost L27、懒单例 getTransport L31-39（零参构造）、getAuthStatus L46-52（HttpTransport 才走缓存通道）、`__resetTransportForTests` L83
- `egosync-app/src/transport/http.ts` — 相对路径 fetch L65 / EventSource L211、cookie `credentials:'same-origin'`、401→`auth:unauthorized` L91、SSE 致命关闭 5s 重建 L39/L241、重放白名单 L303-320；浏览器路径不得变
- `egosync-app/src/main.tsx` — 渲染入口（AuthGate 包 App L19-23）；异步引导需在此挂（index.html `#egosync-splash` 覆盖等待期）
- `src/components/auth/AuthGate.tsx` — 状态机 checking/setup/login/offline/ready L25；桌面直通 L29-31；401 拦截 L59-66（isTauriHost 门控需加模式条件）；离线屏 L99-116（加切回本地逃生口）
- `src/components/settings/GlobalSettingsModal.tsx` — tab 结构 L572-582；companion 分区门控模板 L1204（`!isDesktopOnly(cmd) || isTauriHost()`——各门控点需加 `mode==='local'`）
- `src/components/chat/ChatStream.tsx` — 选工作目录 L1516-1541（desktop-only 门控；远程模式改禁用+说明）
- `src/hooks/useConnectionState.ts` L18-28 + `src/components/layout/ConnectionStatus.tsx` — 三态连接 UI（初始值按宿主硬编码需改按模式；加 REMOTE 标识）
- `src/transport/capabilities.ts` — 生成物禁手改：web-ok 103 条 / desktop-only 15 条 / 重放白名单 31 条
- `src-tauri/src/services/secret_store_keyring.rs` — KeyringSecretStore L18-30（keyring 用法模板，无引擎依赖可直接复用）
- app_data_dir 布局 — egosync.db / conversations.db / opencode-workspace/（lib.rs L55-56、L81-211）

## Tasks & Acceptance

**Execution:**

（评审回环 1 后全部复位待重推导——原实现 aeee913 留存 git 历史作 KEEP 参照。）

- [x] `server/src/auth.rs` + `server/src/middleware`（或现有分层） — 全部 `/api/*` 接受 `Authorization: Bearer <token>` 叠加认证（校验 env/Argon2id 主令牌，非会话表；cookie 路径零变化）；`/api/auth/status` 支持 Bearer — 桌面远程客户端认证通道
- [x] `server/src/` SSE 票据 — `POST /api/events/ticket`（Bearer）发一次性 30s 票据（内存表）；`GET /api/events?ticket=` 校验后建流（cookie 路径不变）— EventSource 无法带头的解法
- [x] `server/src/` CORS 层 — Origin 白名单 `tauri://localhost`、`http://tauri.localhost`，放行 Authorization/content-type 头，覆盖 preflight 与 SSE 响应 — 桌面 webview 跨源访问
- [x] `egosync-app/src-tauri/src/services/desktop_mode.rs`（新） — `app_data_dir/desktop-mode.json` 读写（mode+remoteUrl，缺失/损坏回退 local）+ keyring 令牌存取（键 `remote_instance_token`）— DB 外模式源（先有鸡问题）；**[T2 修订] 读侧统一裁决：mode=remote 而 remoteUrl 缺失/空白 ⇒ 按 local 处理（read_mode 与 desktop_get_boot_config 同源裁决）——杜绝「Rust 进远程 builder / 前端回退 TauriTransport 直通」的砖死会话两层分歧**
- [x] `egosync-app/src-tauri/src/commands/desktop_mode.rs`（新） — `desktop_get_boot_config`（返 mode/URL/令牌）、`remote_mode_save_config`、`remote_mode_restart`（watchdog cancel+sidecar.stop 后 `app.restart()`）— 双模式常驻注册（不依赖引擎 state）；**[T7 修订] 前两命令 async 化（文件+keyring I/O 移出主线程——引导关键路径）；[T12 修订] 校验分支（未知 mode/空令牌/URL 形态/local 保留 URL）抽纯函数补单测**
- [x] `egosync-app/src-tauri/src/lib.rs` — setup 首行读模式：remote 跳过全部引擎装配、invoke_handler 只注册壳命令、Exit 分支按模式；local 路径字节级不变 — 启动恢复与切换的同一机制；**[T18 修订] 远程 builder 补 .setup（仅 Windows DWM 边框修复 + return Ok(())，仍零引擎装配）；[T8 修订] 守卫源码序测试删除 `.or_else` 兜底——精确匹配守卫语句，守卫删除/重排必须红**
- [x] `src/transport/index.ts` + `types.ts` — `setTransportBoot(boot)` 注入；getTransport 三路解析（browser 相对路径 / desktop-local Tauri / desktop-remote HttpTransport 绝对 base+Bearer）；getAuthStatus 直通仅 local — 进程内模式恒定；**[T4 修订] appMode 收敛单一事实源（transport 侧 getDesktopMode 重复导出撤销，消费面统一 appMode）；setDesktopMode 注入拒绝语义对齐 setTransportBoot（注释「重复调用静默忽略」兑现为 console.error + 忽略）**
- [x] `src/transport/http.ts` — 构造选项 {kind, baseUrl, token}；绝对 URL+Authorization 头；SSE 票据流程；remote 模式应用层退避 governor（1s→30s+抖动）；browser 分支行为不变
- [x] `src/appMode.ts`（新）+ `src/main.tsx` — 模块级模式态与 `getDesktopMode()`；main 异步引导（Tauri 宿主先 invoke boot config 再渲染，splash 覆盖；splash 兜底仅 Tauri 宿主——浏览器 checking 态仍由品牌 splash 覆盖）
- [x] `src/components/auth/AuthGate.tsx` + `LoginView.tsx`（或 RemoteLoginView 新） — 远程桌面跑完整状态机（去直通）；令牌重录（验证→keyring→重验）；离线屏加「切回本地」；401→重录；**[T3 修订] handleSwitchToLocal 失败后 rethrow（RemoteLoginView 侧 catch 生效 + finally 复位按钮，错误就地可见）；[T1 新增] 见下方增补任务**
- [x] `src/components/settings/GlobalSettingsModal.tsx` — 「远程模式」tab：local 态=URL+令牌+测试连接+切换（诚实代价确认→save+restart）；remote 态=连接信息+切回本地（确认→save+restart）
- [x] `src/hooks/useConnectionState.ts` + `ConnectionStatus.tsx` + `src/components/chat/ChatStream.tsx` + GlobalSettingsModal 各门控点 — 初始态按模式；REMOTE 徽标/横幅；desktop-only 门控加 `mode==='local'`；选工作目录远程态禁用+说明
- [x] 测试三面 — vitest：transport 解析矩阵 / http.ts remote 行为（URL、头、票据、退避——mock fetch+EventSource）/ AuthGate 远程分支 / 设置 tab 流程 / 门控；server cargo test：Bearer、票据一次性+TTL、CORS 头；src-tauri 单测：模式文件往返+损坏回退；**[T22 修订] ButlerSettingsContent.test/SettingsTab.test 补远程桌面用例（setDesktopMode('remote') 后 desktop-only 入口隐藏）；[T10 修订] AuthGate 远程重录测试桩补非 200 分流用例；[T11 修订] http.remote.test 补退避中间档（1s→2s→4s）断言**

**回环增补任务（review loop 1 —— T1 bad_spec 根因 + T5 待裁决项）：**

- [x] **[T1] 远程 setup 流（冻结矩阵第 5 行）** — `src/components/auth/` 新远程 setup 视图（或 SetupView 参数化）：远端 `setupRequired` 时桌面远程**不得**复用浏览器 SetupView（相对路径 fetch 在 webview 源下不可达远端）；走绝对 URL 直连 `POST {base}/api/setup {token}`（未初始化实例无令牌可验——此调用无 Bearer）→ 成功后令牌即主令牌：写 keyring（remote_mode_save_config mode=remote）→ updateRemoteToken → getAuthStatus 重验 → ready；**setup 态含「切回本地」逃生口**（矩阵行第三列「可中途切回本地」）。配套 vitest：setup 提交链（绝对 URL、成功落 keyring、重验进 ready、失败如实呈现、逃生口可切）
- [x] **[T5·已裁决 2026-09-21] Bearer 通道滥用防护（人工裁决：补）** — `require_auth` 的 Bearer 失败按 IP 计数限流：**仅计验证失败**（成功访问不计数、不限流），阈值/窗口沿用登录面口径（5 次/分钟/IP，滑动窗口，超限 429 统一形状）；计数器独立于 auth 路由限流器（AppState 新增实例）；`/api/cmd/*` 与 `/api/events/ticket` 均覆盖。server 测试：连续错误 Bearer 第 6 次 429、成功访问不计数、窗口滑动恢复

**Acceptance Criteria:**

- Given 本地模式桌面 + 已部署远端，when 设置中测试连接通过并确认切换，then 诚实代价文案呈现、配置持久化、app 重启后直接进入远程态（本地引擎未启动：无 sidecar 进程、egosync.db 未打开）
- Given 远程模式运行中，when 查看界面，then REMOTE 标识+三态连接呈现；拔线后指数退避重连（1s→30s+抖动）恢复
- Given 远程模式，when desktop-only 业务命令面被访问，then 能力隐藏（选工作目录禁用并说明）；invoke 均达远端
- Given 远程模式切回本地，when 确认，then 重启后本地引擎以既有本地数据启动；切换全程无任何读取/写入两侧业务库的代码路径（NFR-C5）
- Given 远程模式中重启 app，when 启动，then 直接进入远程引导（本地引擎不启动）
- Given 全量验证，when build + vitest + server/engine cargo test + test:web，then 全绿且 engine crate 与 http.ts 浏览器路径零源码 diff

## Implementation Notes

### 决策记录

- **Bearer/票据/双通道认证**：`require_auth` 全端点叠加 Bearer 分支（Bearer 提供即以 Bearer 判定，不回落 Cookie——显式凭据结果确定，免 fail-open 组合读法）；`/api/events` 专用 `require_auth_with_sse_ticket` 三通道（Bearer / 票据 / Cookie）；票据校验与消费原子化（并发同票据仅首个成功）、30s TTL、签发时顺手清扫过期项
- **CORS 层置于跨源拒绝之外**：白名单 Origin（`tauri://localhost` / `http://tauri.localhost`）回显具体值 + VARY: Origin；预检 204 + Max-Age 600；非白名单 preflight 403（拒绝语义不被 CORS 旁路）
- **模式预读在 builder 构造前**：`dirs::data_dir()/identifier` 与 Tauri PathResolver 同式派生（DB 外事实源——app_settings 表在引擎启动后才可得，先有鸡问题）；远程态 builder 只注册三个壳命令 + setup 首行守卫 `return Ok(())`（先于 init_db——零本地业务写入的硬前提）+ Exit 分支远程守卫先于 state() 取用（未 manage 会 panic）
- **壳命令直连 IPC**：desktop_mode 三命令经 `@tauri-apps/api/core` 直连（不走 transport——远程模式下业务 invoke 走远端 HTTP，壳命令必须始终走本机）；并钉死壳命令不入远端 dispatch 面（server 测试）
- **传输三路解析**：`setTransportBoot` 进程内一次性注入（重复注入 console.error 拒绝——防热换）；remote 缺 URL 回退 TauriTransport（fail-safe 有数据一侧）但 appMode 如实呈现 remote（UI 层引导处置）；`updateRemoteToken` 就地换令牌（实例不重建——AuthGate 订阅挂在既有实例上）
- **SSE 票据流前端侧**：世代号（teardown 递增）防异步签发-建流竞态孤儿连接；票据重试仅一次防签发-建流死循环；onerror 全接管（票据一次性语义下原生重连复用已消费票据必然失败）；退避 1s→2s→4s…封顶 30s + ±20% 乘性抖动（防雷鸣群重连）
- **keyring 令牌零回显**：`desktop_get_boot_config` 仅 remote 态读 keyring（本地引导零 keyring I/O）；切回本地保留 keyring 令牌与 URL（用户自己的钥匙串——切回远程免重录，无安全恶化）

### 触碰文件

server：auth.rs（Bearer 叠加）、sse.rs（SseTicketStore + ticket_handler）、cors.rs（新）、lib.rs（路由分层 + AppState.sse_tickets + CORS 层）、bootstrap.rs/security.rs（接线）、tests（desktop_remote_test.rs 新 12 例 + common/parity 增）；src-tauri：services/desktop_mode.rs（新）、commands/desktop_mode.rs（新）、lib.rs（模式预读 + 双 builder + Exit 守卫 + 源码序钉测试）；前端：transport（types/index/http）、appMode.ts（新）、main.tsx（异步引导）、auth（AuthGate + RemoteLoginView 新）、settings（GlobalSettingsModal + RemoteModeSection 新）、门控三件（ChatStream/ButlerSettingsContent/SettingsTab）、useConnectionState/ConnectionStatus、desktopModeService.ts（新）+ 六个新测试文件。

### 意外发现（父代理 diff 级复核修复）

- **remoteGating.test.tsx 未处理 TypeError**：ChatStream 挂载即拉可选 Skill，测试漏 mock skillService——全局 invoke 桩 null 载荷经 `setAvailableSkills(null)` 击穿 ChatInput（React 恢复后测试仍过但埋雷）。已补同款 mock（与 ChatStream.test.tsx 一致）。
- **main.tsx splash 兜底误伤浏览器**：引导完成即撤 splash 的兜底原为无条件执行——浏览器宿主 checking 态的品牌 splash（16.1 语义）会在 400ms 内被换成灰底兜底文案。已限定仅 Tauri 宿主执行兜底。
- **实现子代理两度死于环境**：磁盘被 cargo target 瞬时填满（ENOSPC）→ 子代理崩溃遗留部分变更；第二次失败无输出。剩余工作由父代理接手直接完成（step-03 兜底条款），全部验证由父代理独立重跑。

### 评审轮 2 补丁（2026-09-21 step-04——16 项 patch 由父代理亲自落实，实现子代理不可续）

- **U1 隐藏窗口（high）**：main.tsx 引导完成后 `isRemoteDesktop() ⇒ getCurrentWindow().show()`（远程认证界面全在 App 挂载前，App.tsx 的 show() 不可达）；本地态首显时机不变。main.remote.test 三态断言（远程 show / 本地与浏览器零调用）。
- **U2 CORS expose（high）**：cors.rs 白名单实际响应回写 `Access-Control-Expose-Headers: x-egosync-app-error`（常量与 routes.rs 同源引用）——跨源 JS 可读判别头，远程模式业务错误不再被当成功值 resolve；无 Origin 响应零变化（测试钉）。desktop_remote_test 补成功/401 双响应 expose 断言 + 无 Origin 零 expose 断言。
- **U15 错误文案（medium）**：新增 `src/lib/errorMessage.ts`（toErrorMessage：Error→message / AppError 单键对象→首值 / 兜底 String），五处 catch 换用（AuthGate/RemoteLoginView/RemoteSetupView/RemoteModeSection×2）+ 单测 3 例 + AuthGate 单键对象形状用例。
- **U17 首连状态机（medium）**：http.ts onerror 远程分支补 `connecting → reconnecting`（浏览器首连保持 connecting 冻结语义）+ http.remote.test 用例；U16 票据竞态：赋值移到世代检查后 + 用例；U22 票据签发 401：事件发射 + 无退避定时器用例。
- **其余 patch**：U6 useConnectionState 惰性初值钉（订阅隔离断言——防 16.2 首帧闪错回归）；U7 ConnectionStatus 改 isRemoteDesktop() 谓词；U9 onSwitchToLocal 类型如实 `() => Promise<void>`（两视图）；U11 离线屏展示远端地址；U12 main.tsx splash 兜底收窄至 settings-demo 分支（正式路径由 AuthGate/App 管理）；U13 测试改名（引导检查网络失败）+ 补真「重录视图内 verifyToken 断网」用例；U18 read_mode_file 读侧 URL 归一（trim/空白⇒None）+ 2 测；U23 令牌臂裁决抽 `resolve_boot_token` 纯函数（local⇒None 零 keyring I/O / remote 透传 / 出错⇒None）+ 2 测；U24 RemoteModeSection 双切换失败路径用例；U21 main.remote.test splash 语义断言（浏览器/Tauri 均不由 main 撤）。
- **补丁后全电池复验**（父代理独立）：build ✅ / vitest 67 文件 877 例（865+12）✅ / server cargo 64 例 ✅ / src-tauri cargo 107 例（103+4）✅ / engine 零 diff ✅ / http.ts 本轮增量仅两处且均 this.remote 门控（浏览器路径行为零变化——http.test.ts 全绿钉住；U20 维持 T21 裁决）✅ / test:web 3 specs ✅。
- 首次编译错误（HeaderValue::from_static 非 Result——if let 误用）即改即过，无连续失败。

### 重推导轮（2026-09-21 step-03，review loop 1 之后）

- **执行方式**：分发实现子代理（规格为唯一事实源）——先恢复 KEEP 参照 aeee913 的代码面（36 文件原样），再逐条落地 12 项修订（净增量 18 文件 +1115/−115）。子代理一次成功（前轮两度死于磁盘 ENOSPC，本轮目标目录已预热、增量构建）。
- **T5 落地形状**：`AppState.bearer_rate` 独立 `RateLimiter` 实例（与 auth 路由限流器互不侵占预算）；`require_auth` / `require_auth_with_sse_ticket` 的 Bearer 分支**校验先行、仅失败路径计数**（有效令牌始终放行）；三端点（/api/cmd/*、/api/events/ticket、/api/events Bearer 分支）共享计数器；超限 429 统一形状。集成测试：第 6 次失败 429、成功不计数不限流、端点共享、auth 限流器独立性；窗口滑动恢复经单测注入跨窗记录（60s 窗口不可在集成测试内等待）。
- **T1 落地形状**：`RemoteSetupView`（绝对 URL 直连 `POST {base}/api/setup`，无 Bearer——未初始化实例无令牌可验）→ 成功后 keyring（模式文件幂等重写）→ `updateRemoteToken` → 重验进 ready；404（已初始化/env 锁定）统一锁定说明 + 返回登录；网络失败断网文案；setup 态含切回本地逃生口。测试 4 例（提交链全钉：绝对 URL/无 Bearer 头、keyring 落点、重验 Bearer、逃生口 save→restart 序）。
- **T2 落地形状**：`services::desktop_mode::resolve_mode`（remote 缺/空 URL ⇒ local）为读侧统一裁决——`read_mode`（进程引导）与 `desktop_get_boot_config`（前端引导）同源引用；前端 index.ts 的 TauriTransport 回退保留为纵深防御（真实 boot 通道对该形态已不可达，index.remote.test 注释更新）。
- **父代理验证轮**（独立重跑，不采信子代理自述）：build ✅ 零类型错误 / vitest 66 文件 865 例全绿（857 + 新增 8，零未处理错误）/ server cargo test 64 例（lib 9 + api 24 + csp 2 + desktop_remote 14 + parity 6 + secret 4 + web_entry 5）✅ / src-tauri cargo test 103 例 ✅ / engine cargo test 822 例 + crates/ 零 diff ✅ / http.ts 与 aeee913 逐字节一致（浏览器路径守门）✅ / test:web 3 specs 8 tests ✅（注：链式复验首跑 1 spec 偶发失败——紧接全部 cargo 套件后高负载所致，隔离复跑与子代理轮均全绿，判定环境偶发非回归）。矩阵 8 行逐行复核：第 5 行（远端未初始化）由 RemoteSetupView 4 例闭合。
- **子代理自报变异验证**：T8 删守卫 → 测试红（掏空语义成立）；该变异在父代理轮 1 已由验证缺口层独立证实。

### 首轮父代理验证轮（2026-09-21 step-03，评审回环 1 前的历史）

全量独立重跑（不采信子代理自述）：build ✅ / vitest 857+13 例（含修复后零未处理错误——修复后全量复跑）/ server cargo test 61 例 ✅ / src-tauri cargo test 95 例 ✅ / engine cargo test 822 例 ✅ 且 crates/ 零 diff / test:web 3 specs ✅。I/O 矩阵 8 行逐行核对测试覆盖（RemoteModeSection.test 4 行、AuthGate.remote.test 3 行、http.remote.test SSE 行、lib.rs 源码序钉 + main.remote.test 启动恢复行、desktop_remote_test 12 例服务侧行）。

## Spec Change Log

### 2026-09-21 · review loop 1（step-04 评审回环）

- **触发**：三层评审 22 项发现（见 Review Triage Log）。T1（bad_spec）——冻结矩阵第 5 行「远端未初始化→走 setup 向导（浏览器等价）/可中途切回本地」未落成任务，实现渲染了不可用的相对路径 SetupView 且 setup 态无逃生口；T5（intent_gap）——冻结块对 Bearer 通道滥用防护无措辞，限流对等无唯一读法，回退后人工裁决。
- **T5 人工裁决（2026-09-21）**：补上限流——require_auth 的 Bearer 失败按 IP 计数（仅计失败，成功不计数；5 次/分钟/IP 滑动窗口，超限 429），覆盖 /api/cmd/* 与 /api/events/ticket。裁决已转正为任务（见 Tasks 回环增补）。
- **修订**：Tasks 增补「远程 setup 流」任务（T1 根因闭合）；并入 10 项 patch 级修订条款（T2 读侧统一裁决 / T3 切回 rethrow / T4 appMode 单源 / T7 命令 async 化 / T8 守卫测试删 or_else / T10 401 桩 / T11 退避档位 / T12 命令校验单测 / T18 远程 builder 补 DWM setup / T22 门控测试补例）；全部任务复位待重推导。
- **已避开的坏状态**：远程桌面遇未初始化实例的死胡同界面（提交必失败且无逃生口）；remote 缺 URL 的砖死会话（两层裁决分歧）；切回失败按钮永久禁用+错误不可见；守卫测试空洞化（删守卫不红）；Windows 远程态无边框黑边回归；模式双源漂移。
- **KEEP（重推导必须保留的既有验证面）**：原实现 commit `aeee913`（git 历史）为重推导参照——模式预读/双 builder/Exit 守卫、Bearer+一次性票据+CORS 白名单设计、http.ts 远程通道（世代号防竞态/单次重试防环/退避 governor）、main.tsx 异步引导（splash 兜底仅 Tauri 宿主）、RemoteModeSection 诚实代价流、desktop_remote_test.rs 12 例与全部前端测试结构——该实现经 build / vitest 857 / server 61 / src-tauri 95 / engine 822 / test:web 3 specs 全绿验证，重推导应在其设计上落实修订条款而非另起炉灶。

## Review Triage Log

评审轮 1（2026-09-21，step-04 三层并行：盲扫 / 边界 / 验证缺口；基线 e285060，diff 5192 行 243kB）。逐项裁决（来源：盲=盲扫层、边=边界层、缺=验证缺口层[预验证]）：

| # | 来源 | 发现 | 裁决 | 证据与处置 |
|---|------|------|------|-----------|
| T1 | 盲+边 | 远程桌面遇未初始化实例：AuthGate setup 态无远程分支，复用 SetupView（相对路径 fetch 在 `tauri://localhost` 源下不可达远端实例，提交必失败）；且 setup 态无「切回本地」逃生口 | medium → bad_spec | 已核 AuthGate.tsx L162-164、authService.setup 相对 fetch、SetupView 无逃生口 prop。冻结矩阵第 5 行明书「走 setup 向导（浏览器等价）/可中途切回本地」，但非冻结任务清单未把该行落成任务（任务 10 只覆盖令牌重录/离线屏/401）——规格任务面缺口致实现照单漏做 |
| T2 | 盲+边 | remote+缺 remoteUrl 两层分歧：Rust `read_mode` 只看 mode 字段（`remote_url` serde default）→ 走远程 builder（引擎零装配、仅 3 壳命令）；前端 index.ts 回退 TauriTransport（getAuthStatus 直通 authenticated:true）→ gate ready → 全部业务 invoke 命中未注册命令 | medium → patch（并入修订） | 已核 services/desktop_mode.rs 解析、index.ts 三路解析、lib.rs 远程 builder。触发面=手改/半写模式文件（写路径必验 URL）。最小修=Rust 读侧统一裁决：remote 缺 URL ⇒ local（两层同源）；前端回退保留为纵深防御 |
| T3 | 盲+边 | RemoteLoginView「切回本地」失败路径：AuthGate.handleSwitchToLocal 内部 catch 吞错不 rethrow → Promise 永不 reject → RemoteLoginView 侧 catch 死代码、isSwitching 永不复位（按钮停在「正在切回本地...」永久禁用），错误落进 login 态不可见的 offlineMessage | medium → patch（并入修订） | 已核 AuthGate.tsx L105-115 与 RemoteLoginView.tsx L69-78 的吞错链。最小修=handleSwitchToLocal 失败后 rethrow + RemoteLoginView finally 复位；离线屏路径（AuthGate 自有 isSwitchingLocal，catch 内已复位）不受影响 |
| T4 | 盲+边 | 模式态双源：appMode 模块变量与 transport boot 并存，消费面混用（ConnectionStatus/RemoteModeSection 用 transport 版 getDesktopMode，ChatStream/AuthGate/门控用 appMode 版）；setDesktopMode 注释「重复调用静默忽略」与实现（直接覆写）相悖 | medium → patch（并入修订） | 已核 appMode.ts L23-26 与 index.ts 双版本 getDesktopMode。未来调用点漏配一半即门控与传输不一致。最小修=收敛单一事实源 + 注入拒绝语义对齐 setTransportBoot |
| T5 | 盲+边 | Bearer 通道无限速：/api/cmd/* 与 /api/events/ticket 挂 require_auth 无限流，错误 Bearer 可按网络速率无限试探主令牌（登录面有 5/min/IP 防暴力破解；Argon2id 态每试还烧 CPU） | medium → intent_gap | 已核 auth.rs rate_limit 仅覆盖 auth 路由。冻结块对新增通道滥用防护无措辞，「不削弱既有安全语义」是否涵盖限流对等存在两种读法（字面=cookie/SSE 语义；精神=暴力破解姿态不倒退）——无唯一解，人工裁决 |
| T6 | 盲 | Argon2id 库态每请求一次且同步阻塞 tokio worker | low → 拒绝 | 单用户低 QPS；「每请求一次 Argon2id」为规格钉死口径；T5 若落地限流，DoS 面同步收窄。日常无人可感 |
| T7 | 盲 | desktop_get_boot_config / remote_mode_save_config 为同步命令，文件+keyring I/O 跑主线程（引导关键路径） | low → patch（并入修订） | 已核两命令非 async（remote_mode_restart 已是 async）。改 async 即移出主线程——引导期 keyring 往返不再阻塞渲染 |
| T8 | 盲+边+缺 | 守卫测试 `.or_else` 兜底掏空断言：删除 setup 守卫后回退匹配更早的 `if remote_mode {`（tracing 块），95+6 测试全绿——「远程零本地写入」最硬不变量的钉子失效 | medium → patch（并入修订） | 验证缺口层变异验证（删守卫全绿）。最小修=删 or_else，精确匹配守卫语句（守卫删除/重排即测试失败） |
| T9 | 盲 | 规格 Verification 基线数字（796/45）未随实现更新 | 拒绝 | 修法=改本 build 规格（规则明令拒绝）；最新数字（857/61）已录 Implementation Notes 供复跑对照 |
| T10 | 盲 | AuthGate.remote.test「重录验证 401 形态」测试桩恒返 200，非 200 分流零覆盖（测试名不副实） | low → patch（并入修订） | 修=桩改返非 200，断言 loginErrorText 对应分流文案 |
| T11 | 盲 | http.remote.test 退避测试无中间档断言（delays.push 为死代码；1s 与 30s 封顶之外的回归测不出） | low → patch（并入修订） | 修=补 1s→2s→4s 档位序列断言 |
| T12 | 盲 | Rust 命令层校验分支零测试（未知 mode 拒绝/空令牌拒绝/URL 校验接线/local 保留 URL） | low → patch（并入修订） | 重推导时为命令层校验补单测（校验逻辑抽纯函数直测） |
| T13 | 盲 | app_data_dir 预读派生（dirs::data_dir()/identifier）与 Tauri PathResolver 等价性未钉死，Tauri 升级漂移会静默错读模式目录 | maybe-false → defer | 评审员已核 vendored tauri-2.11.2 今日逐字一致；若真发生=medium（builder 期模式错读）。下次 Tauri 升级时双路径交叉校验可定谳——记 deferred-work |
| T14 | 盲 | CORS 层 Vary 用 insert 覆写既有 Vary（若压缩层存在会破坏缓存键） | false | 已核 server/src/lib.rs 全部分层：无压缩层、无其他 Vary 来源——今日无坏结果。潜在脆弱仅在将来引入 Vary 生产层时成立 |
| T15 | 盲 | local 写路径跳过 URL 校验；文件 I/O 失败映射 SidecarError 语义错位 | low → 拒绝 | 用户可达路径全经测试连接/既有已验证值（RemoteLoginView/AuthGate 存的 URL 均为写入时已验值）；错误变体仅影响日志归类，排障面可忽略 |
| T16 | 盲 | 远程模式无用户文档与令牌轮换指引 | low → defer | 真实缺口但属发布期文档工作（README/使用说明），非本故事代码——记 deferred-work |
| T17 | 盲(附注) | settings-demo 分支仅判 isTauriHost（远程桌面下 demo 页会把引擎命令打到远端） | 拒绝 | 验证缺口层核为规格已记录的刻意设计（demo 页桌面专属，见 Design Notes/规格记录）；用户流程不可达 |
| T18 | 边 | 远程 builder 无 .setup——Windows DWM 边框修复（本地态 setup 闭包内）在远程态缺失，无边框窗口黑边回归 | low → patch（并入修订） | 已核 lib.rs 远程分支仅 invoke_handler。修=远程分支补 .setup（DWM 修复 + return Ok(())，仍零引擎装配） |
| T19 | 边 | scheme-only/带 query 的 URL 可经直接 IPC 持久化进模式文件 | false | 用户路径经测试连接拦截：坏 URL 的 fetch 形态必然失败→切换被阻（RemoteModeSection.handleTest 全量分流）。程序不可达状态（前端不会发出此类 URL）——Rust 校验弱但用户面守卫足 |
| T20 | 边 | save 成功+restart 失败→用户以为切换失败，下次启动意外进远程 | low → 拒绝 | restart 失败罕见且 switchError 已如实可见；模式文件状态=用户显式确认的意图（诚实代价文案第四条已预告重启语义） |
| T21 | 边 | AC「http.ts 浏览器路径字节级不变」与源码已改相悖（行为等价：既有 http.test.ts 24 例零 diff 钉住浏览器语义） | 拒绝 | 修法=改本 build 规格；行为等价解释已录 Implementation Notes（构造选项注入，缺省构造字节不变） |
| T22 | 缺 | Butler/SettingsTab 远程门控（isTauriHost→isLocalDesktop）零测试钉住：只有浏览器宿主用例，无 setDesktopMode('remote') 用例——删门控无测试失败 | low → patch（并入修订） | 验证缺口层证实（变异无红）。修=两测试文件各补远程桌面用例（入口隐藏断言，与 remoteGating.test 同款驱动） |

**分组与路由：**

- **bad_spec（回环）**：T1——根因在非冻结任务面（冻结矩阵第 5 行未落成任务）。触发规格修订+代码回退+重推导（step-03）。
- **intent_gap（回环，人工裁决）**：T5——根因在冻结块对新增 Bearer 通道滥用防护的沉默，且无唯一读法。回退代码后请人工定夺限流策略。
- **patch（本轮回环下并入规格修订，随重推导落实）**：T2、T3、T4、T7、T8、T10、T11、T12、T18、T22。
- **defer**：T13、T16 → 已录 deferred-work.md。
- **拒绝**：T6、T9、T14、T15、T17、T19、T20、T21（证据见各行）。

### 评审轮 2（2026-09-21，重推导后 step-04 三层：盲扫 14 / 边界 7 / 验证缺口 4+1；diff 6344 行 312KB）

边界层首跑撞 token 上限，重发（精简笔记指令）后完成。逐项裁决（编号 U*，来源同上轮）：

| # | 来源 | 发现 | 裁决 | 证据与处置 |
|---|------|------|------|-----------|
| U1 | 盲+缺 | 远程认证界面渲染在不可见窗口：tauri.conf.json `visible:false`，全仓唯一 `show()` 在 App 挂载且角色加载完后（App.tsx:264）——离线屏/令牌重录/初始化向导全在 App 挂载之前；远端宕机时用户看到「应用没打开」，矩阵 3-5 行逃生口物理不可达 | **high → patch** | 已核 tauri.conf.json:25 与 App.tsx:264（jsdom 测不了窗口可见性，桌面 e2e 环境受阻故全绿）。修=main.tsx 引导完成时远程桌面即 `getCurrentWindow().show()`（本地态 show 时机不变——防首帧闪白既定设计）；main.remote.test 补远程 show / 本地不 show 断言 |
| U2 | 盲 | CORS 层缺 `Access-Control-Expose-Headers: x-egosync-app-error`：跨源 fetch 对非安全列表响应头不可读（http.ts:133 判别头恒 null）——远程模式**所有 200+头+错误体形态的业务错误被当成功值 resolve**，错误语义整体断裂（集成测试 reqwest / 前端测试裸 fetch 桩都不模拟 CORS 头过滤，故测不出） | **high → patch** | 已核 cors.rs 无 expose、http.ts:133 消费点、routes.rs 错误头。修=cors.rs 白名单实际响应回写 expose 头 + server 测试；浏览器同源可读头语义由此恢复等价 |
| U3 | 盲 | 限流判定在 Argon2 校验之后：超限后第 6+ 次错误 Bearer 仍各烧一次 Argon2 才拿 429（登录面限流在 handler 之前，两通道不对称——T5 声称的 CPU 燃烧面未真正收窄） | low → defer | 修复（验证前 peek 窗口拦截）会连有效令牌一并 429——与 T5 任务冻结措辞「仅计失败、成功不限流」直接冲突，属设计取舍：peek 与「成功不限流」不可兼得（无凭据无法预知有效性）。现形态已把爆破钳到 5 次/min/IP，剩余面=登录面同款分布式暴露。记 deferred-work |
| U4 | 盲 | T1 旅程「正常路径不可达」：设置 tab 测试连接对未初始化实例报错禁切（指引去浏览器），与矩阵第 5 行 in-app setup 向导矛盾 | false | 规格任务面自答：「测试连接通过才可切」为切换守卫；矩阵行描述远程态运行时状态机（status 返回 setupRequired ⇒ 向导）——两流各司其职；in-app setup 可达面=远程态期间实例重置/keyring 失效等边缘（设计如此） |
| U5 | 盲 | 远程通道全程无超时：黑洞化主机挂死 checking | false | 浏览器等价语义（同源 web 同样挂死——冻结设计「桌面=浏览器等价物」）；AC 覆盖的拔线=快速失败形态；jsdom/e2e 均按此语义全绿 |
| U6 | 盲 | useConnectionState 远程初始态（connecting）无测试钉住——把条件改回 `isTauriHost()?online` 全部测试仍绿（16.2 首帧闪错态修复在远程桌面静默回归） | low → patch | remoteGating.test 补断言（boot remote 后初始 connecting） |
| U7 | 盲 | ConnectionStatus 直比 `getDesktopMode()==='remote'` 而非 `isRemoteDesktop()` 谓词（T4 单源纪律漏配点——'remote' 仅在 Tauri 宿主成立纯靠巧合） | low → patch | 一行改谓词（import 自 appMode） |
| U8 | 盲 | SETUP_TOKEN_MIN_LEN=8 前后端手工双份（RemoteSetupView.tsx:28 / auth.rs:68） | low → 拒绝 | 跨语言常量天然双份（注释已锚定「与 Rust 同源约定」）；服务端为权威——漂移时 401 携服务端错误文案可见，无静默坏结果 |
| U9 | 盲 | onSwitchToLocal prop 类型 `() => void` 吞 rejection（实参为 async 失败 rethrow 函数）——两个现有调用点恰好 try/catch，下一个即 unhandled rejection | low → patch | 类型改 `() => Promise<void>`（RemoteLoginView/RemoteSetupView 两处） |
| U10 | 盲 | /api/auth/status 在登录面 5/min/IP 总量限流组（不分成败）：桌面 onboarding（gate 检查+验证+重验+测试连接）同 IP 同预算，常规用量贴边，超限 429「尝试过于频繁」 | low → defer | 预算设计属 15.4 既有面（浏览器同预算同计数方式）；本故事增量边际（onboarding +2-3 调用）；429 60s 自愈且文案如实。预算拆分/豁免值得独立决策——记 deferred-work |
| U11 | 盲 | 离线屏不显示远端地址：配错/失效 URL 时无法就地诊断（login/setup 视图均展示 URL，唯离线屏没有） | low → patch | 离线屏补 URL 展示（getTransportBoot().remoteUrl） |
| U12 | 盲 | main.tsx splash 兜底（400ms 撤）在远程模式实为主路径：checking 网络往返远超 400ms，品牌屏被换成灰底「正在检查登录状态...」 | low → patch | AuthGate（setup/login/offline 态）与 App（ready 态）已各自管理 splash 生命周期——main.tsx 兜底冗余，删除（浏览器分支本就不执行；本地态 show 前窗口隐藏无感） |
| U13 | 盲 | AuthGate.remote.test「重录验证网络失败」用例名不符实：桩对全部 status reject（注释声称首次成功）——实际测 gate 首查失败→离线屏；重录视图内 verifyToken 网络失败分支零覆盖 | low → patch | 改名（离线语义）+ 补真重录网络失败用例（首次 200 authenticated:false → 登录视图 → 提交后 reject → 断网文案） |
| U14 | 盲 | http:// 远程实例全程无明文传输警告（令牌 Bearer 明文过线） | low → 拒绝 | 局域网合法场景（测试钉 http://192.168.1.10:8080）；无坏结果，提示属发布期打磨（U/T16 文档 defer 已覆盖远程模式说明面） |
| U15 | 边 | 壳命令失败呈现 `[object Object]`：Tauri reject 值为序列化 AppError 单键对象（{ValidationError:"…"}——本仓 GlobalSettingsModal.test mock 形状为证），5 处新 catch 均用 `e instanceof Error ? e.message : String(e)`：AuthGate/RemoteLoginView/RemoteSetupView/RemoteModeSection×2 | **medium → patch** | 共享错误文案提取器（Error→message / 单键对象→首值 / 兜底 String），五处换用 + 断言单键对象形状的测试 |
| U16 | 边 | SSE 票据签发竞态：`this.sseTicket = await fetchSseTicket()` 赋值先于世代检查——teardown（世代翻新）后票据仍入库，跨订阅生命周期滞留（重订阅复用旧票=多一次建流失败后自愈） | low → patch | 赋值移到世代检查之后 |
| U17 | 边 | 远程 SSE 首连持续失败永不离开 connecting：onerror 仅在 online 态置 reconnecting——首连失败期间无「重连中」横幅（矩阵拔线行「三态诚实呈现」缺口，gate 已过而事件流失联时用户无感） | **medium → patch** | 远程接管分支补 `connecting → reconnecting`（浏览器首连保持 connecting 为冻结语义，不受影响）+ http.remote.test 用例 |
| U18 | 边 | 读侧不 trim remoteUrl：手改模式文件带首尾空白 ⇒ 远程引导携空白 base URL 全部 malformed、落误导性离线屏（写侧 validate_save_request trim、读侧不归一——两层不对称） | low → patch | read_mode_file 归一 trim（空白归 None）+ 单测 |
| U19 | 边 | claim「全部 /api/* 接受 Bearer」证伪：login/logout/setup 无 Bearer 分支 | false | bootstrap 端点 cookie-only 属设计（远程桌面不经 login——Bearer 直连；setup 时无令牌可验；logout 属会话通道）；且携 Bearer 调用这些端点不 401 而是照常工作（头被忽略）——主张的坏结果不成立 |
| U20 | 边 | claim「http.ts 浏览器路径零源码 diff」证伪 | 携带 T21 | 同位同主张、代码未变——维持 T21 裁决（拒绝；行为等价解释已录实现记录，http.test.ts 24 例钉住语义） |
| U21 | 缺 | main.tsx splash 宿主门控无测试——本 diff 自己修的「误伤浏览器」可静默回归 | low → patch | main.remote.test 补 splash 断言（与 U12 合并为新语义：浏览器/Tauri 远程均不由 main.tsx 撤 splash） |
| U22 | 缺 | SSE 票据签发 401 分支（emit auth:unauthorized、不退避）零测试——令牌轮换经 SSE 通道抵达重录视图的唯一信号无钉 | low → patch | http.remote.test 补 401 用例（事件发射 + rebuildTimer null） |
| U23 | 缺 | desktop_get_boot_config 本地态「零 keyring I/O/令牌零回显」冻结款无测试——有人把两臂统一为始终读 keyring 即静默击穿 | low → patch | 令牌臂决策抽 services 纯函数（local⇒None 且不触 keyring / remote⇒透传 / 出错⇒None fail-safe）+ 直测（T12 范式） |
| U24 | 缺 | RemoteModeSection 双切换流失败路径（save/restart reject）无测试——T3 同款吞错 bug 可在此静默复发 | low → patch | 补两例（错误文案呈现 + isSwitching 复位） |

**轮 2 分组与路由：** 无 intent_gap、无 bad_spec（两条 high 根因均为实现疏漏——冻结块「桌面=浏览器等价物」语义已足够清晰：U1 之下逃生口不可达=矩阵行落空；U2 之下错误分流断裂=网络错误≠401 分流落空）→ **不回环**。patch：U1、U2、U6、U7、U9、U11、U12、U13、U15、U16、U17、U18、U21、U22、U23、U24（16 项；实现子代理已不可续——按 step-04 兜底条款由父代理亲自落实）。defer：U3、U10 → deferred-work.md。拒绝：U4、U5、U8、U14、U19、U20。

## Design Notes

- **认证裁决**：远程桌面 Bearer 头（fetch）+ SSE 一次性票据（EventSource 无法带自定义头；票据 30s TTL、单次使用，防令牌入 URL/日志）+ CORS 白名单三件叠加，cookie 会话不动（SameSite=Strict 无法跨 tauri 源携带，不走 CORS+cookie 弱化路线）；**Bearer 失败面限流（人工裁决 2026-09-21）：仅计失败、5 次/分钟/IP 滑动窗口、超限 429——与登录面防盗器对等**
- **模式文件在 DB 外**：app_settings 表在 egosync.db 内而引擎启动要先读模式——鸡生蛋；desktop-mode.json 为最小事实源，损坏回退 local（数据在本地、fail-safe 到有数据的一侧）
- **重启式切换复用面**：条件装配（启动恢复 AC 要求的机制）+ Exit 清理 + `AppHandle::restart()`；实现需验证 restart 路径是否触发 RunEvent::Exit——无论触发与否，restart 命令内先显式 cancel+sidecar.stop（幂等，8.6 保证）
- **前端模式进程恒定** → 无 context/provider/热换：模块级 `getDesktopMode()` 即可；测试默认 boot=local（test-setup 桩不变）
- **网络错误与 401 严格分流**：离线（退避重连）绝不能误判为令牌失效（重录视图只由 401 触发）——AuthGate 既有 checkFailureText 分类可复用
- 远程桌面保留 TitleBar/窗口管理（桌面壳仍在）；主令牌即单用户令牌，无会话管理协议

## Verification

**Commands:**
- `cd egosync-app && npm run build` — expected: tsc 零类型错误
- `cd egosync-app && npx vitest run` — expected: 既有 796+新增全绿（含 http.ts 浏览器路径回归例）
- `cd server && cargo test` — expected: 既有 45+新增 Bearer/票据/CORS 全绿
- `cd crates/egosync-engine && cargo test` — expected: 全绿且零源码 diff（本故事不动 engine crate）
- `cd egosync-app/src-tauri && cargo test` — expected: 模式文件往返/损坏回退/重启清理序全绿
- `cd egosync-app/tests/e2e && npm run test:web` — expected: 3 specs 8 tests 全过（浏览器零回归）

**Manual checks (if no CLI):**
- 切换流程真机走查（本地→远程→切回、拔线重连、令牌重录）——桌面 e2e 本 VM 阻断（16-2 先例：tauri-driver 会话层环境限制），留档截图+补验建议
