---
title: '桌面客户端远程模式（Story 16.3，FR-48）'
type: 'feature'
created: '2026-09-20'
status: 'in-progress'
route: 'dispatch'
review_loop_iteration: 0
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

- [x] `server/src/auth.rs` + `server/src/middleware`（或现有分层） — 全部 `/api/*` 接受 `Authorization: Bearer <token>` 叠加认证（校验 env/Argon2id 主令牌，非会话表；cookie 路径零变化）；`/api/auth/status` 支持 Bearer — 桌面远程客户端认证通道
- [x] `server/src/` SSE 票据 — `POST /api/events/ticket`（Bearer）发一次性 30s 票据（内存表）；`GET /api/events?ticket=` 校验后建流（cookie 路径不变）— EventSource 无法带头的解法
- [x] `server/src/` CORS 层 — Origin 白名单 `tauri://localhost`、`http://tauri.localhost`，放行 Authorization/content-type 头，覆盖 preflight 与 SSE 响应 — 桌面 webview 跨源访问
- [x] `egosync-app/src-tauri/src/services/desktop_mode.rs`（新） — `app_data_dir/desktop-mode.json` 读写（mode+remoteUrl，缺失/损坏回退 local）+ keyring 令牌存取（键 `remote_instance_token`）— DB 外模式源（先有鸡问题）
- [x] `egosync-app/src-tauri/src/commands/desktop_mode.rs`（新） — `desktop_get_boot_config`（返 mode/URL/令牌）、`remote_mode_save_config`、`remote_mode_restart`（watchdog cancel+sidecar.stop 后 `app.restart()`）— 双模式常驻注册（不依赖引擎 state）
- [x] `egosync-app/src-tauri/src/lib.rs` — setup 首行读模式：remote 跳过全部引擎装配、invoke_handler 只注册壳命令、Exit 分支按模式；local 路径字节级不变 — 启动恢复与切换的同一机制
- [x] `src/transport/index.ts` + `types.ts` — `setTransportBoot(boot)` 注入；getTransport 三路解析（browser 相对路径 / desktop-local Tauri / desktop-remote HttpTransport 绝对 base+Bearer）；getAuthStatus 直通仅 local — 进程内模式恒定
- [x] `src/transport/http.ts` — 构造选项 {kind, baseUrl, token}；绝对 URL+Authorization 头；SSE 票据流程；remote 模式应用层退避 governor（1s→30s+抖动）；browser 分支行为不变
- [x] `src/appMode.ts`（新）+ `src/main.tsx` — 模块级模式态与 `getDesktopMode()`；main 异步引导（Tauri 宿主先 invoke boot config 再渲染，splash 覆盖）
- [x] `src/components/auth/AuthGate.tsx` + `LoginView.tsx`（或 RemoteLoginView 新） — 远程桌面跑完整状态机（去直通）；令牌重录（验证→keyring→重验）；离线屏加「切回本地」；401→重录
- [x] `src/components/settings/GlobalSettingsModal.tsx` — 「远程模式」tab：local 态=URL+令牌+测试连接+切换（诚实代价确认→save+restart）；remote 态=连接信息+切回本地（确认→save+restart）
- [x] `src/hooks/useConnectionState.ts` + `ConnectionStatus.tsx` + `src/components/chat/ChatStream.tsx` + GlobalSettingsModal 各门控点 — 初始态按模式；REMOTE 徽标/横幅；desktop-only 门控加 `mode==='local'`；选工作目录远程态禁用+说明
- [x] 测试三面 — vitest：transport 解析矩阵 / http.ts remote 行为（URL、头、票据、退避——mock fetch+EventSource）/ AuthGate 远程分支 / 设置 tab 流程 / 门控；server cargo test：Bearer、票据一次性+TTL、CORS 头；src-tauri 单测：模式文件往返+损坏回退

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

### 父代理验证轮（2026-09-21 step-03）

全量独立重跑（不采信子代理自述）：build ✅ / vitest 857+13 例（含修复后零未处理错误——修复后全量复跑）/ server cargo test 61 例 ✅ / src-tauri cargo test 95 例 ✅ / engine cargo test 822 例 ✅ 且 crates/ 零 diff / test:web 3 specs ✅。I/O 矩阵 8 行逐行核对测试覆盖（RemoteModeSection.test 4 行、AuthGate.remote.test 3 行、http.remote.test SSE 行、lib.rs 源码序钉 + main.remote.test 启动恢复行、desktop_remote_test 12 例服务侧行）。

## Spec Change Log

## Review Triage Log

## Design Notes

- **认证裁决**：远程桌面 Bearer 头（fetch）+ SSE 一次性票据（EventSource 无法带自定义头；票据 30s TTL、单次使用，防令牌入 URL/日志）+ CORS 白名单三件叠加，cookie 会话不动（SameSite=Strict 无法跨 tauri 源携带，不走 CORS+cookie 弱化路线）
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
