---
title: 'Story 16.1: WEB 入口、认证与首访流'
type: 'feature'
created: '2026-09-20'
status: 'done'
baseline_commit: 'cf102bf051845b1bd3cd46ba0569b471de7ea2f4'
route: 'dispatch'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/epic-16-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 云端实例（15.4 server）今天只有裸 API——浏览器打开实例地址得不到任何页面；前端在浏览器宿主下没有登录/首访流程，desktop-only 入口点击即 404，登出端点不存在。

**Approach:** server 同源静态服务 `egosync-app/dist`（与桌面 frontendDist 同一构建产物）+ SPA 回退；前端新增 AuthGate（setup 向导 → 登录 → 401 全局拦截 → 登出）并配 capabilities 驱动的 desktop-only 入口显隐；server 侧补 logout 端点与安全响应头。

## Boundaries & Constraints

**Always:**

- 单一 Vite 构建产物双宿主复用：server 服务 `egosync-app/dist`，绝不新建第二套前端构建/配置；web 验证统一用 `npm run build` 产物
- 认证面改动仅两项（均为 boss 2026-09-20 裁决）：① 新增 `POST /api/auth/logout`；② login 的 Set-Cookie 增 `Max-Age=2592000`（30 天持久会话）。其余既有 status/login/setup 的路径、body、状态码、限流 5 次/分/IP、错误形状（401 `{"error":"unauthorized"}` / 404 `{"error":"not found"}`）零改动；15.4 既有测试若断言 Cookie 无 Max-Age，随裁决同步更新断言
- 统一失败文案不泄露实例/用户存在性；env 锁定与已初始化在协议上不可区分——用同一句诚实文案覆盖两态
- auth gate 必须先于任何 useEngineEvent/SSE 订阅生效（防未认证 401 重建循环）；登出后不得残留 SSE 重连
- desktop-only 入口显隐一律 `isDesktopOnly(cmd)`（capabilities 驱动），禁止按宿主硬编码
- 浏览器不落任何业务数据：不新增 localStorage/IndexedDB 业务键（现有 `egosync-theme` 与两处折叠区 UI 态为非业务态，维持）
- 桌面宿主零行为变化：gate 直通、TitleBar 退化与 show() 门控均 15.5 已交付，只守护不重做

**Never:**

- 不做 CORS 放行（跨源显式拒绝模型冻结；同源服务 + 既有 vite dev proxy 解决）
- 不做多端吊销/TTL 清扫/「记住我」可选项（17.x 会话安全故事；本故事按裁决统一 30 天持久 Cookie + logout 删行）
- 不做 setup nonce/stdout 加固方案（已评估，defer 17.x，见 Design Notes）
- 不手改 `src/transport/capabilities.ts`/`events.ts` 生成物、不动 replayWhitelist 31 条、不动 15.5 SSE 状态机（登出 teardown 除外）
- 不改 OnboardingView 五步流程本身；不动 tests/e2e wdio 体系与孤儿 playwright.config.ts
- 不隐藏「数据与隐私」tab 整体——`data_destroy` 是 web-ok，仅隐导出/导入区
- 不为 dev 方便在任何宿主加认证豁免

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 首访未初始化 | status → `{setupRequired:true, authenticated:false}` | setup 向导（中文文案）设置令牌（≥8 字符，前端预校验） | 后端 <8 字符 401 如实呈现 |
| setup 成功 | `POST /api/setup {token}` → 200 | 前端随即自动携令牌 login → 进入应用 | — |
| env 锁定态 setup 尝试 | `POST /api/setup` → 404 | 统一锁定说明「实例已初始化或已由环境变量锁定」 | 不自动重试 |
| 已初始化未登录 | status → `{setupRequired:false, authenticated:false}` | 登录页（含次级「首次部署？」setup 入口） | — |
| 登录成功 | `POST /api/auth/login {token}` → 200 + Set-Cookie（含 Max-Age=30 天） | 进入应用；first_launch 时正常进 onboarding；30 天内关闭浏览器重开仍保持登录 | — |
| 登录失败 | 401 `{"error":"unauthorized"}` | 统一失败文案（不泄露存在性），可重试 | — |
| 认证面限流 | 429 `{"error":"rate limit exceeded"}` | 「尝试过于频繁，请稍后再试」 | 退避提示 |
| 会话中 401 | invoke 返回 401（会话失效） | transport 发 `auth:unauthorized` 本地事件 → 全局回登录页，内存态清空 | 不白屏 |
| 登出 | 点击登出 → `POST /api/auth/logout` | 服务端删 auth_sessions 当前行 + Set-Cookie 过期（Max-Age=0）→ 回登录页 | 幂等：无有效会话也 200 |
| 登出后刷新 | 已登出浏览器 F5 | status authenticated:false → 登录页（不伪造登录态） | — |
| SPA 路由 | `GET /unknown-path`（非 /api） | 200 index.html | — |
| 静态资产缺失 | `GET /assets/missing.js` | 404 | — |
| API 未知路径 | `GET /api/unknown` | JSON 404（现状保持） | — |
| dist 未配置 | EGOSYNC_STATIC_DIR 未设且默认路径不存在 | API-only + 启动 tracing 警告 | 不崩溃 |
| 服务器不可达 | status fetch 网络错误 | 「无法连接服务器」+ 重试按钮 | 不白屏 |
| 桌面宿主 | `isTauriHost()` 为真 | gate 直通渲染 App，零行为变化 | — |

</frozen-after-approval>

## Code Map

**server/（15.4 交付，认证契约冻结）**

- `server/src/lib.rs:57-95` — build_router 三段 merge（probes/public/authed）+ 中间件叠放（CatchPanic→CSP→跨源拒绝→50MB limit）；静态服务挂载点在 :84-95 尾部 fallback；:79 注释预留 16.1
- `server/src/auth.rs:170-182` — GET /api/auth/status → `{setupRequired, authenticated}`（setupRequired=无 env 且库内无哈希）
- `server/src/auth.rs:194-249` — POST /api/setup：env 态/已初始化 → 404 `{"error":"not found"}`；trim 后 <8 字符 → 401
- `server/src/auth.rs:256-312` — POST /api/auth/login：失败统一 401 `{"error":"unauthorized"}`；:302-305 Set-Cookie `egosync_session=<uuid>; Path=/; HttpOnly; SameSite=Strict`（本故事按裁决增 Max-Age=30 天；auth_sessions 行入库跨重启有效，无 TTL 清扫——17.x）
- `server/src/auth.rs:315-326` — require_auth 中间件（/api/cmd、/api/events）；:50-52,:147-164 认证面限流 5 次/分/IP
- `server/src/security.rs:18-19` — CSP_POLICY 常量 + 全响应 CSP 头；:40-66 跨源显式 403 拒绝（无 CORS 放行头）；:5 注释预留 16.1 安全头
- `server/src/main.rs:38-65` — env 解析（EGOSYNC_HOST/PORT/DATA_DIR/TOKEN），静态目录 env 在此扩展
- `server/src/bootstrap.rs:297-354` — build_test_state 轻量测试态
- `server/tests/common/mod.rs:24-52,55-132` — InProcessServer + Client + login() 测试范式（静态测试用 fixture dist 目录）
- `server/tests/api_test.rs` — 24 条集成测试先例（含 :308-330 源码扫描计数「靠机制不靠纪律」范式）
- `server/Cargo.toml:23` — tower-http 仅 catch-panic feature，ServeDir 需加 "fs"

**egosync-app/src/（前端）**

- `src/transport/index.ts:20-22,27-37` — isTauriHost() 探测 + getTransport() 懒单例 + 模块级 invoke；:17 导出 isDesktopOnly/isWebCommand（capabilities 驱动门控原语，无现成 hook）
- `src/transport/http.ts:57-93` — invoke：fetch POST /api/cmd/{cmd}，credentials same-origin（Cookie 自动携带 :61）；401 只 reject 不拦截（:7 注释预留 16.1 全局拦截）；:149-160 getAuthStatus() 已备但零消费者、缓存永不失效（15-5 G11 遗嘱：本故事补类型化 + 登录/登出后失效）；:164-202 SSE 懒建 + 致命关闭 5s 重建（gate 必须先于订阅）
- `src/transport/localEvents.ts:14-17` — FRONTEND_LOCAL_EVENTS 名单（新增 auth:unauthorized 走此机制）
- `src/transport/capabilities.ts` — gen:transport 生成物禁手改（web-ok 103 / desktop-only 15 / 白名单 31）
- `src/App.tsx:218-231` — isFirstLaunch → onboard/refreshAllRoles（登录 gate 插它之前）；:234-245 splash 移除 + show() 已 isTauriHost 门控；:381 isLoadingRoles gating
- `src/components/layout/TitleBar.tsx:24-33` — 浏览器退化已由 15.5 交付（守护即可）
- `src/components/onboarding/OnboardingView.tsx:248-279` — 全屏居中卡片布局（登录页/setup 页视觉基点，slate/indigo + backdrop-blur 设计语言）
- `src/components/chat/ChatStream.tsx:1377-1388` — 选工作目录按钮（desktop-only `chat_pick_working_directory`，待门控）
- `src/components/settings/GlobalSettingsModal.tsx:575,:1181-1183` — 侧栏 companion 按钮 + tab（CompanionPairingSection 挂载即调 desktop-only 命令，必须 tab 级门控，光藏按钮不够）；:969-1179 data tab 导出/导入区（`data_destroy` 是 web-ok——仅隐导出/导入区，保留销毁）
- `src/components/butler/ButlerSettingsContent.tsx:927-931`、`src/components/role/SettingsTab.tsx:835-839` — skill 文件夹按钮（desktop-only，待门控）
- `src/main.tsx:7-13` — URL 分支先例（AuthGate 包裹点）
- `src/services/dashboardService.ts` — service 对象字面量范式；auth 前端调用走 REST 直连（仿 http.ts getAuthStatus），不进 invoke 命令通道
- `src/test-setup.ts:8-37` — 全局 `__TAURI_INTERNALS__` 桩（默认 Tauri 分支）；浏览器分支测试范式 = 删桩-恢复-`__resetTransportForTests()` 三件套（App.test.tsx:179-192、TitleBar.browser.test.tsx 全文件）
- localStorage 现状：`egosync-theme`（App.tsx:50-54 读 / :328-335 写 / index.html:12-15 内联防 FOUC）、role/butler-settings-sections（UI 态）——均非业务态

**构建与 CI**

- `egosync-app/vite.config.ts:23-25` — dev proxy /api→localhost:8080 已配好（Origin/Host 对齐注意见 Design Notes）
- `egosync-app/src-tauri/tauri.conf.json:6-11` — frontendDist `../dist` = `egosync-app/dist` = `npm run build` 产物（单一构建锚点，server 引用同一路径，勿复制）
- 截图对比零现状：tests/e2e 是 wdio+tauri-driver 桌面链路（不动）；16.1 截图留档走手动拍摄存档

## Tasks & Acceptance

**Execution:**

- [x] `server/Cargo.toml` — tower-http 增 "fs" feature — ServeDir 依赖
- [x] `server/src/static_files.rs`（新，`static` 为 Rust 关键字故避名）+ `server/src/lib.rs` + `server/src/main.rs` — 静态服务：env `EGOSYNC_STATIC_DIR`（默认 `../egosync-app/dist` 存在即用，缺失则 tracing 警告并 API-only）；ServeDir + SPA 回退（非 /api 的 GET 未命中文件 → 200 index.html；资产缺失 404；/api/* 保持 JSON 404，`call_fallback_on_method_not_allowed(true)` 保 POST /api/unknown 不被 405 短路）；挂 build_router 尾部，CSP/跨源/limit 中间件天然覆盖静态响应
- [x] `server/src/auth.rs` + `server/src/lib.rs` — login 的 Set-Cookie 增 `Max-Age=2592000`（30 天持久，boss 2026-09-20 裁决，同步更新 15.4 既有断言）；新增 `POST /api/auth/logout`（挂 public 组并入限流）：幂等 200；携带有效会话则删 auth_sessions 当前行；Set-Cookie `egosync_session=; Max-Age=0` 同属性过期 — 其余既有端点契约零改动
- [x] `server/src/security.rs` — 全响应叠加 `X-Content-Type-Options: nosniff`、`X-Frame-Options: DENY`、`Referrer-Policy: no-referrer` — 15-4 遗留安全头补齐；CSP script-src 增 hash-source `'sha256-t7EoxfYkO3wNL2nCWqo4+0sb9alMtMK3TJiFg8kQUkQ='`（index.html 内联防 FOUC 脚本 219 字节 sha256-base64；禁 unsafe-inline）
- [x] `server/tests/web_entry_test.rs`（新）— 集成测试：静态服务（fixture dist：index 命中/深路径 SPA 回退/资产 404/api 404 保持/未配置目录 API-only）、logout（有效会话删行+Cookie 过期+幂等 200+后续请求 401）、安全头与 CSP 覆盖 HTML 断言
- [x] `egosync-app/src/transport/http.ts` — invoke 401 时经 localEvents 发 `auth:unauthorized`；`getAuthStatus()` 返回类型化为 `{setupRequired, authenticated}` 并新增缓存失效 API（登录/setup 成功与登出后调用）；登出/401 后关闭 EventSource（无订阅即关，防 401 重建循环）
- [x] `egosync-app/src/transport/localEvents.ts` — FRONTEND_LOCAL_EVENTS 增 `auth:unauthorized`
- [x] `egosync-app/src/components/auth/`（新，AuthGate + LoginView + SetupView + authErrors + authService）— 状态机 checking/setup/login/offline/ready；桌面宿主直通；setup（≥8 字符预校验，成功自动 login）；登录页统一失败文案/429 文案/断网重试态/次级「首次部署？」入口（404 → 统一锁定说明）；离开 checking 态时移除 `#egosync-splash`（同 App.tsx 机制）；视觉复用 OnboardingView 居中卡片设计语言；主题跟随 `egosync-theme`
- [x] `egosync-app/src/main.tsx` — AuthGate 包裹 App（settings-demo 分支留在 gate 外）；保证 gate 先于任何事件订阅
- [x] `src/components/chat/ChatStream.tsx`、`src/components/settings/GlobalSettingsModal.tsx`、`src/components/butler/ButlerSettingsContent.tsx`、`src/components/role/SettingsTab.tsx` — desktop-only 入口门控五处：工作目录按钮（chat_pick_working_directory）、companion 侧栏按钮+tab（tab 级防挂载即调）、data tab 导出/导入区（destroy 保留）、两处 skill 文件夹按钮——一律 `isDesktopOnly(cmd)` 驱动（谓词 `!isDesktopOnly(cmd) || isTauriHost()`：命令毕业 web-ok 时入口自动在浏览器出现，不按宿主硬编码）
- [x] `src/components/layout/Sidebar.tsx` — web-only 登出按钮（登出 → 缓存失效 → `auth:unauthorized` → gate 回登录页；网络失败仍本地登出）
- [x] `egosync-app/src/**/`（同目录 *.test.tsx）— vitest：AuthGate 全路径矩阵 15 用例（setup/含 env 锁定/登录成功失败/429/断网/401 拦截回登录/登出/缓存失效/桌面直通）；浏览器分支 desktop-only 入口不渲染断言（ChatStream/GlobalSettingsModal/ButlerSettingsContent/SettingsTab 四文件 + companion tab 不挂载）；data destroy 在 web 可见；桌面分支零回归（默认桩下 747 测试全绿，GlobalSettingsModal 数据 tab 补桌面四区可见正断言）；CSP hash 契约源码扫描（csp.contract.test.ts：index.html 内联脚本 hash ⇄ security.rs CSP_POLICY byte 对 byte 钉死 + dist 产物一致性）；http.ts 401 发射/缓存失效/无订阅即关传输级钉
- [x] 验证收口 — `npm run build` + `npx vitest run`（747/55 全绿）+ `cd src-tauri && cargo test`（84 绿）+ engine `cargo test`（822 绿）+ `cd server && cargo test`（44 绿）全绿；`cargo run --bin egosync-server` 冒烟（EGOSYNC_STATIC_DIR 指向 dist → 浏览器首访 setup → onboarding → 登出 → 刷新仍登录页全流程，puppeteer 真浏览器 7 张截图留档）；桌面 vs 浏览器并排截图见下方留档说明

**Acceptance Criteria:**

- Given server 二进制与 `egosync-app/dist` 构建产物，when server 启动（静态目录已配）并访问根路径，then 返回同一构建产物的 index.html；未知非 API 路径 200 回 index.html；缺失资产 404；/api/* 保持 JSON 404
- Given 浏览器宿主，when 应用初始化，then 选中 HttpTransport；desktop-only 入口（工作目录/配对管理/导入导出/skill 文件夹）不渲染；data_destroy 仍可用；TitleBar 退化零回归
- Given 未初始化实例，when 浏览器首访，then `/api/auth/status` 引导 setup 向导（中文），设置成功后进入登录态并进入应用
- Given env 令牌实例，when 经 setup 入口提交，then 后端 404 拒绝且前端如实呈现统一锁定说明（不泄露实例状态）
- Given 已初始化实例，when 无有效 Cookie 访问任意页面或登录令牌错误，then 401 全局拦截回登录页/统一失败文案；正确令牌进入应用
- Given 已登录用户，when 点击登出，then logout 端点清会话行与 Cookie → 回登录页；此后浏览器内刷新仍需重新登录；无 SSE 残留重连
- Given 已登录用户，when 30 天内关闭并重新打开浏览器访问实例，then 仍保持登录（持久 Cookie 裁决落地）
- Given 桌面浏览器与桌面版并排对照，when 截图对比，then 差异仅限宿主门控项，留档可查
- Given 全量验证命令，when 执行，then `npm run build`、`npm run test:all`、`cd server && cargo test` 全绿

## Implementation Notes

（实现期追加——决策、触碰文件、意外发现）

- **desktop-only 门控谓词**：五处门控统一用 `!isDesktopOnly(cmd) || isTauriHost()` 而非裸 `isDesktopOnly(cmd)`。`isDesktopOnly` 是静态清单查询（两宿主下同值），裸用会把桌面入口也藏掉；该谓词保持「capabilities 驱动」——命令将来毕业为 web-ok（清单除名）时入口自动在浏览器出现，无需改组件。
- **登出按钮的宿主分支例外**：登出无对应 invoke 命令（纯浏览器 REST 语义），故 Sidebar 登出按钮用 `!isTauriHost()` 门控——与 TitleBar 浏览器退化同款宿主分支。登出链路：`authService.logout()`（网络失败仍继续）→ `emitFrontendEvent('auth:unauthorized')` → AuthGate 失效缓存 + 回登录页（App 卸载 ⇒ 最后一个事件订阅撤空 ⇒ http.ts 无订阅即关 SSE）。
- **`static_files.rs` 命名**：`static` 是 Rust 关键字，文件名避之。SpaFallbackService 实现 `tower::Service`（非 axum Service）以自定义 fallback；`req.uri().path().to_string()` 在 async 块前同步提取，否则 `Request<B>`（B 可能 !Send）跨 await 触发 E0277。
- **SPA 回退判定**：`/api` 或 `/api/*` → JSON 404；末段含 `.` 的路径（资产形状）→ 404；其余 → 200 index.html。`ServeDir::call_fallback_on_method_not_allowed(true)` 保证 POST /api/unknown 落到 JSON 404 而非 405 短路。
- **中间件覆盖静态面**：静态 fallback 经 `base.fallback_service(...)` 挂载，位于四层中间件（DefaultBodyLimit → reject_cross_origin → security_headers → CatchPanic）之内——CSP/安全头/跨源/上限天然覆盖静态响应（集成测试断言 HTML 响应含全部安全头）。
- **CSP hash 双侧钉死**：`src/security/csp.contract.test.ts` 源码扫描 index.html 内联脚本（219 字节）计算 sha256-base64，与 security.rs CSP_POLICY 的 hash-source 比对（Rust 续行 `\` 拼接还原）；dist 存在时同测产物侧 byte 对 byte + 唯一 inline script 断言。vite 原样保留 index.html 已实测（构建后 dist hash 不变）。
- **AuthGate 浏览器分支 fetch 注入**：测试经 `vi.stubGlobal('fetch', ...)` 注入假 fetch（按 URL 分流 status/body），真实 authService 全链路跑通（非 mock service）——登录/setup/自动登录/缓存失效均实路径验证。
- **登出后 SSE 拆除链**：AuthGate 离开 ready ⇒ App 卸载 ⇒ useEngineEvent 清理 ⇒ `handlers.size === 0` ⇒ `teardownEventSource()`（close + 取消重建定时器 + 状态回 connecting）；再订阅时懒重建。http.test.ts 用 fake timers 钉死「重建定时器已取消」（退避期满零新建）。
- **429 触发实测**：冒烟时连续 curl 超过 5 次/分即触发限流（服务器日志 `认证面限流拒绝`）——限流面实证有效，冒烟节奏须留窗口。
- **桌面截图环境限制（诚实留档）**：`_bmad-output/implementation-artifacts/screenshots/16-1/` 留档 7 张真实浏览器截图（puppeteer + 真实 server + 真实 dist：首访登录页/setup 向导/填令牌/登录后 onboarding/登出后/刷新仍登录页/持久会话重开实证；评审清理：原 05 与 04 字节重复已删，原 08 黑屏占位更名为 `08-desktop-unrenderable-evidence.png`——是环境限制证据非桌面视图）。01-07 摄于评审修复轮前的构建，仅错误路径文案受本轮补丁影响、所摄视图不变；09 为修复轮后构建。桌面侧截图在本 headless VM 不可产：Xvfb + openbox 下 tauri 二进制正常启动（窗口创建、DB 迁移完成），但 WebKitGTK 无 GPU/DMABUF 下不向 X surface 绘制（试过 `WEBKIT_DISABLE_DMABUF_RENDERER`/`WEBKIT_DISABLE_COMPOSITING_MODE`/`GSK_RENDERER=cairo`/`LIBGL_ALWAYS_SOFTWARE` 均黑屏）——环境限制非代码问题。桌面视觉零回归由测试套件守护（默认 Tauri 桌面桩下全部组件测试 + 桌面分支正断言：companion/导出/导入/销毁/工作目录/skill 文件夹全可见）。桌面真机截图建议用户在本机 `npm run tauri dev` 后补拍对照。
- **父代理独立复核（2026-09-20，对照 diff 非子代理自述）**：① 全量重跑——`npm run build` 绿（1737 模块）；vitest 747/55 全绿；server cargo test 44 全绿（含 web_entry_test 4 条）；engine cargo test 822 全绿；src-tauri cargo test 84 全绿。② 真二进制冒烟——`EGOSYNC_STATIC_DIR=../egosync-app/dist cargo run` + curl 实测：根路径 200 text/html、SPA 回退 200、缺失资产 404、`/api/unknown` JSON 404、healthz 200、四安全头 + CSP hash 全量下发、auth/status 未初始化 `{"setupRequired":true,"authenticated":false}`——与 I/O 矩阵逐行吻合。③ 矩阵审计——16 行 I/O 矩阵全部有测试覆盖且在上述运行中实跑通过。④ 偶发排除——首次并发跑 engine 时 9 例失败（mcp_server/delegate_bridge/llm_config），根因是磁盘满（99%，No space left on device）下并发编译的偶发；清理 target 后隔离重跑 822 全绿，且本故事 diff 零触碰 engine crate，非回归。⑤ 生成物零触碰——capabilities.ts/events.ts/replayWhitelist 无 diff，Never 清单全部守住。
- **评审修复轮（2026-09-20，step-04 三层评审后由父代理亲施——实现子代理上下文耗尽不可再接触）**：17 项 patch 全落地——CSP 字体源放行（style-src +fonts.googleapis.com、新增 font-src +fonts.gstatic.com，契约测试加「仅此两第三方域」断言）；index.html 直出/SPA 回退统一 `Cache-Control: no-cache`（`/` 显式路由 + `index_response()` 共用函数——ServeDir append_index 不支持自定义头）；AuthGate 5xx 分流+文案常量化；SetupView 自动登录失败内层改 loginErrorText；Sidebar logout 429 分流（不本地登出、提示稍后重试——修「刷新静默复登」AC 矛盾）；main.tsx demo 分支 isTauriHost 门控；authService.getStatus 死码删除；GlobalSettingsModal.browser.test 补桩恢复；web_entry_test 去 keep()（TestDirs 守卫持有 TempDir、逆序 drop 保证 server 先关）+ logout 拆两测试（认证面 4+2 次留限流余量）；新增 authService.test.ts（logout/login/setup REST 形状 + 错误形状）、gateWiring.contract.test.ts（App-in-AuthGate 接线源码扫描门禁）；AuthGate.test 补 setup成功+登录失败/splash 移除两用例；ci.yml 增「build 后强制执行 csp.contract 产物侧门禁」步骤；截图档案清理（删重复 05、08 更名 unrenderable-evidence）；architecture.md 五处增量 + README 新增 Web 自托管节。
- **评审修复轮验证（全量重跑，对照上方复核同口径）**：`npm run build` 绿（dist 重产出，hash 资产更新）；vitest 757/57 全绿（净增 10 用例；1 处既有断言随文案统一改带句号常量——评审 #2 的修复即消除标点分叉）；test:all 含 src-tauri 84 + engine 822 全绿；server cargo test 45 全绿（web_entry 拆分后 4→5）。真二进制冒烟复测：`/` 200 + `cache-control: no-cache` + CSP 含字体源 + 三安全头；SPA 回退 no-cache；hash 资产 200；`/api/unknown` JSON 404；auth/status 契约不变。
- **30 天持久会话端到端实证（评审 #8 补）**：`09-browser-restart-still-authed.png`——puppeteer 同 userDataDir 双会话：会话 A 登录后**完整关闭浏览器**，会话 B 同 profile 零输入重开直达应用（onboarding 视图，逐字比对）——Cookie 跨浏览器重启存活 = Max-Age 持久语义最直接证据。首版脚本两处缺陷被本轮自我更正后重跑：① 断言只排除登录页（429 离线页假阳性）→ 改为排除全部 gate 失败态标记；② 已认证直通后 SSE 长连接使 networkidle0 永不触发 → 改 domcontentloaded。此实证本身即「冒烟节奏须留限流窗口」的另一教训：验证用 curl 与浏览器脚本共用 5/min/IP 预算。

## Spec Change Log

- **2026-09-20 评审修复轮（step-04）**：三层评审分诊后 17 项 patch + 4 项 defer，无 intent_gap / bad_spec（不触发回环）。冻结块（Intent/Boundaries/I-O 矩阵/Tasks）零改动；变化全部落在实现与测试层：CSP 字体源放行（16.1 第二项放行——style-src/fonts 两源，契约测试钉死仅此两域）、index.html no-cache、登出 429 分流、demo 宿主门控、既有测试文案断言随常量统一（评审 #2 修复的直接结果）。新增 3 个测试文件 + 既有 4 个测试文件补强；web_entry_test fixture 守卫化 + logout 拆分。计划文档（architecture.md 五处增量、README Web 自托管节）随契约同步。评审中三项 reject 的理由与四项 defer 的去向见 Review Triage Log 与 deferred-work.md。

## Review Triage Log

2026-09-20 三层评审（盲扫 17 条 + 边界 9 条 + 验证缺口 6+3 条）分诊：

| # | 来源 | 发现 | 裁决 | 证据与去向 |
|---|---|---|---|---|
| 1 | 盲扫 | SSE 通道 401 不产生认证失效信号（被动浏览会话失效→重建循环永停留「重连中」） | low→defer | auth:unauthorized 仅由 invoke 401 发射（http.ts）；当前无 TTL/吊销、触发面窄，17.x TTL 后成常态；登记 deferred-work |
| 2 | 盲扫 | checkFailureText 只识别 429；5xx 显示「无法连接」；文案与 authErrors 常量标点分叉 | low→patch | AuthGate.tsx 硬编码字面量 vs authErrors.ts 常量并存，两界面同因不同文；复用常量+5xx 分流 |
| 3 | 盲扫 | SetupView 自动登录失败内层用 setupErrorText（401 时显示「令牌长度不足」误导） | low→patch | SetupView.tsx:63 亲证；外层语境缓解但 401 细节仍错；改 loginErrorText |
| 4 | 盲扫 | 静态服务无 Cache-Control（重部署后启发式缓存旧 index→引用已 404 hash 资产→白屏） | medium→patch | static_files.rs 无任何缓存头，ServeDir 带 Last-Modified 触发启发式缓存；index.html 统一 no-cache |
| 5 | 盲扫 | 登出不终止已建立的 SSE 流（服务端侧；跨 tab 残留至 idle timeout） | low→defer | require_auth 只挡新连接亲证；会话生命周期族缺陷归 17.x；登记 deferred-work |
| 6 | 盲扫+验证缺口 | 截图档案矛盾：04/05 字节相同（标签矛盾）、08 黑屏占位命名 desktop-app-view、Notes「7 张」实 8 文件 | low→patch | ls 亲证 04=05=31918 字节；清档+更正记录 |
| 7 | 盲扫 | AC「并排截图对比」桌面侧未达成但任务全勾 | reject | 修复=改本 spec 勾选状态（规则禁止）；环境限制已诚实留档，提交 CHECKPOINT 由 boss 裁决真机补拍或调整 |
| 8 | 盲扫 | 30 天持久会话 AC 无端到端实证 | low→patch | 机制已证（api_test Max-Age 断言+Cookie 标准语义）；补同 userDataDir 重启仍登录的实证截图 |
| 9+10 | 盲扫 | architecture.md/README 未同步契约级变更（logout/Max-Age/静态 env/CSP/安全头） | medium→patch | 规划文档漂移误导后续 agent；15.x 有故事内更新先例；两文档补增量 |
| 11 | 盲扫 | GlobalSettingsModal.browser.test.tsx afterEach 不恢复桩 | low→patch | 文件自称三件套实做两件；补恢复（对齐既有范式） |
| 12 | 盲扫 | splash 移除 setTimeout 未收 cleanup | low→patch | 悬空定时器操作已分离节点；收进 effect cleanup |
| 13 | 盲扫 | authService.getStatus 零消费者死代码；认证调用两路径不一致 | low→patch（删死码）／reject（分层重构） | getStatus 全仓零调用亲证→删除；transport 级 status+缓存 vs service 级动作的分层有 15.5 设计依据，无具名伤害不接受重构 |
| 14 | 盲扫+验证缺口 | web_entry_test tempdir keep() 泄漏 /tmp 目录；logout 用例认证面请求恰 5 次贴 RATE_LIMIT_MAX | low→patch | 本故事验证期亲历磁盘 99% 满事故；去 keep()+压请求数 |
| 15+V3 | 盲扫+验证缺口 | csp.contract.test.ts dist 侧断言在 CI 与常规路径永不执行（vitest 先于 build，dist 缺失静默 return） | medium→patch | 验证缺口层亲证 ci.yml 顺序与 gitignore；CI 增 build 后强制执行该测试 |
| 16+E2+E9 | 盲扫+边界 | logout 429 被当网络失败→本地登出但服务端会话+Cookie 存活→刷新静默复登（AC 矛盾；冒烟实测撞过限流证可达） | medium→patch | Sidebar catch 一切错误仍发事件亲证；429 单独分流提示稍后重试、不本地登出 |
| 17+E4 | 盲扫+边界 | settings-demo 留 gate 外：web 未认证可达→满屏 401 错误态 | low→patch | demo 分支加 isTauriHost() 门控（仍在 gate 外，web 不渲染） |
| E1+E8 | 边界 | CSP 拦截 index.html:7 Google Fonts 外链→web 字体回退，与桌面（csp:null）宿主门控外视觉分叉 | medium→patch | index.html:7 外链+security.rs style-src 'self' 亲证在线环境必拦截；CSP 增 fonts.googleapis.com（style-src）与 fonts.gstatic.com（font-src） |
| E3 | 边界 | getAuthStatus 畸形 body 被永久缓存 | reject | server 是该形状唯一生产者且集成测试钉死契约；投机加固 |
| E5 | 边界 | setup 态被迟到 401 踢回登录 | reject | 触发需会话死亡+特定导航时序，日常不可达；加守卫属投机加固 |
| E6 | 边界 | setup 成功+自动登录失败后重复点击→404 | reject | 404 文案如实（实例确实已初始化）且「返回登录」入口已显性；加 setupDone 守卫属投机加固 |
| E7 | 边界 | EGOSYNC_STATIC_DIR 非 UTF-8 静默落默认 | reject | 投机运维场景 |
| V1 | 验证缺口 | authService.logout() 真实 REST 调用零测试执行（Sidebar mock 掉服务层） | medium→patch（预验证） | 补 stub fetch 断言 POST 形状（URL/method/credentials/body） |
| V2 | 验证缺口 | AuthGate↔App 接线（gate 先于订阅的硬约束）无测试钉住 | medium→patch（预验证） | 仿 csp.contract 源码扫描范式钉死 main.tsx 中 App 在 AuthGate 内 |
| V4 | 验证缺口 | setup 成功+自动登录失败分支无测试 | low→patch（预验证） | 补 stubFetch 组合用例（setup 200+login 失败） |
| V5 | 验证缺口 | splash 移除行为从未被断言（jsdom 恒 no-op） | low→patch（预验证） | 注入 #egosync-splash 断言 hidden/移除 |
| V6 | 验证缺口 | resolve_static_dir env 解析三分支零自动化覆盖 | low→defer（预验证，按 filed 处置） | bin 级 env 胶水+并行竞态需先参数化重构；登记 deferred-work |
| O3 | 验证缺口 | logout 集成测试认证面请求恰 5 次贴限流上限 | low→patch | 与 #14 合并处理（压请求数） |

分诊结论：无 intent_gap、无 bad_spec → 不触发回环。patch 17 项（去重合并后）、defer 4 项。

## Design Notes

- **SSE 401 循环防护**：EventSource 401 是致命关闭 → 5s 重建循环。gate 在 main.tsx 包裹 App，未认证时 App 不挂载 → 零订阅；登出时主动关闭 EventSource + 失效 auth status 缓存。这是 gate 位置的硬约束，不是风格选择。
- **logout 幂等设计**：无有效会话也返回 200 + 过期 Cookie。若挂 require_auth，过期会话登出会 401 联动「被踢回登录页」的误判路径。
- **30 天持久 Cookie 裁决（boss 2026-09-20）**：login Set-Cookie 增 Max-Age=2592000。auth_sessions 行本就落库跨重启有效，服务端无需其他改动；TTL 清扫与多端吊销仍归 17.x。关闭浏览器不再登出是裁决接受的体验取舍。
- **env 锁定不可区分是特性**：15.4 刻意不泄露实例状态（setupRequired=false 同时覆盖 env 态与已初始化态）。文案统一为「实例已初始化或已由环境变量锁定」，诚实且不泄露。
- **静态服务选运行时目录而非编译期嵌入**：`cargo test` 与 server CI 无需前端产物（测试用 fixture 目录）；生产以 env 显式控制；17.x Docker 化时可再评估嵌入单二进制。
- **CSP × index.html 内联主题脚本**：`security.rs` CSP 若 script-src 'self' 无 unsafe-inline，将拦截 index.html:12-15 的内联防 FOUC 脚本。实现时核对 CSP_POLICY 实文，以最小改动放行（首选 CSP hash-source 或脚本外置），禁止整体放开 unsafe-inline；同时实测 dist 产物无其他 inline script。
- **vite dev proxy 的 Origin/Host 对齐**：changeOrigin 只改 Host 不改 Origin，POST 携 Origin 头仍可能 403。dev 工作流以「build dist + server 静态面」为主路径；proxy 仅作轻量调试，若撞 403 调整 proxy 配置而非 server 跨源模型。
- **setup nonce/stdout 评估结论（defer）**：该方案会改 setup API 契约与部署流程，超出本故事「入口与首访流」边界，登记至 17.x 会话/首访安全故事；默认 127.0.0.1 绑定 + EGOSYNC_TOKEN 预设仍是现行缓解。
- **getAuthStatus 缓存失效（15-5 G11 遗嘱）**：登录/setup 成功与登出后必须失效缓存，否则 pre-login 的 authenticated:false 钉死整个进程期。

## Verification

**Commands:**

- `cd egosync-app && npm run build` — expected: tsc 零类型错误，dist 产出
- `cd egosync-app && npm run test:all` — expected: vitest + 全部 cargo test 全绿
- `cd server && cargo test` — expected: 静态服务/logout/安全头集成测试全绿
- `cd server && EGOSYNC_DATA_DIR=/tmp/egosync-smoke EGOSYNC_STATIC_DIR=../egosync-app/dist cargo run` — expected: 浏览器 http://127.0.0.1:8080 完成 setup → onboarding → 登出 → 刷新仍登录页全流程

**Manual checks:**

- 桌面版 `npm run tauri dev` 行为零变化（gate 直通、TitleBar/配对/导入导出照常）
- 桌面浏览器 vs 桌面版并排截图 ≥2 组视图（仪表盘/设置），存 `_bmad-output/implementation-artifacts/screenshots/16-1/`
