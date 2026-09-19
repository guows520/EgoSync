---
title: '前端双传输与对等测试套件（Story 15.5）'
type: 'feature'
created: '2026-09-19'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '1cfc1636ea92da87aa69f20fdb360a3c8fb4893a'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-15-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 前端 114 处 invoke 全部直连 `@tauri-apps/api`（集中在 19 个 service 文件），UI 与 Tauri 宿主物理耦合——同一 React 应用无法在浏览器运行；"API 面与 Tauri command 一一对应"仅靠纪律维持，单侧契约漂移无门禁变红。

**Approach:** 新建 `src/transport/` 双实现（TauriTransport 与现状逐字节一致 / HttpTransport fetch+EventSource）+ `window.__TAURI_INTERNALS__` 运行时探测；19 个 service 文件 import 换一行；`useEngineEvent` 接替 `useTauriEvent` 并退役旧 hook；capabilities/events 从 commands.json 工件与 events.rs 构建期生成（禁手写双清单）；对等测试用例由工件全量驱动——vitest 与 server `cargo test` 消费同一 commands.json，任何单侧契约漂移在 CI 变红，"对等"从纪律变成机制。

## Boundaries & Constraints

**Always:**

- 桌面零回归硬边界：收口 = `npm run test:all` 全绿 + `tests/e2e`（15.2 裁决 A 口径：执行 + 既有缺陷逐项归因）+ `npm run build` tsc 零错。
- 传输对等契约：同一 command 名、参数形状（camelCase 单层扁平对象、可选 `?? null`、复合输入包 `input` 键——现状惯例原样）、返回形状、事件名在双通道同构；Tauri 分支行为与现状逐字节一致（invoke 透传、listen 异步语义 + 静默容错 + mounted 防泄漏原样平移）。
- 同源生成：TS 侧 capabilities/events 由构建期从 `crates/egosync-engine/commands.json` + `crates/egosync-engine/src/events.rs` 生成；生成物提交入库 + CI 新鲜度门禁（regenerate + `git diff --exit-code`，与 commands.json/dispatch_gen 同范式）；对等用例由工件全量枚举驱动（禁抽样禁手抄——测试循环遍历工件条目，测试代码内零手写命令名/事件名）。
- 重连补齐白名单进 commands.json 工件（engine `capabilities.rs` 单源常量导出）；白名单只收只读 query；写入类 command 永不重放；Tauri 分支恒 Online 永不触发。
- 任何契约变更（含 AppError 判别头、replayWhitelist 字段）双通道同步 + 对等测试同步更新。

**Never:**

- 不做双 Vite 构建/双入口/双 dist 配置（单一构建产物双宿主复用；vite.config.ts 仅按架构 [M] 加 dev proxy `/api`→`localhost:8080`）。
- server 静态资源服务/SPA 回退/登录页/setup 向导归 16.1——本故事不给 server 内嵌 dist、不做 web e2e。
- 连接状态的 UI 呈现（徽章/横幅）与重连后视图刷新接线归 16.2——本故事只交付 transport 层信号（`onConnectionStateChange`）与重放协议（结果经内部事件交付）。
- 不动 engine 业务逻辑与命令实现；engine/server 侧改动仅限传输面四处：`capabilities.rs` 加白名单常量、`gen_commands.rs` 加工件导出字段、`routes.rs` 加 AppError 响应判别头、serde_json 启 `preserve_order` 特性。
- 不引入 React Context 全局重构、不新增任何 npm/Rust 依赖；TitleBar 浏览器退化仅去掉窗口控制区与拖拽区（布局细节归 UX 阶段）。
- AppError 13 variant「一律 200 + 原样单键 map」冻结款不动（epics 写 14 为规划期计数漂移，以 error.rs 现役 13 为准——15-4 已裁定同款先例）。

## I/O & Edge-Case Matrix

| 场景 | 输入 / 状态 | 期望输出 / 行为 | 错误处理 |
|------|-------------|-----------------|----------|
| 宿主探测 | `window.__TAURI_INTERNALS__` 存在 / 不存在 | 懒单例分别选中 TauriTransport / HttpTransport；测试环境默认 Tauri 分支（test-setup 既有全局桩不变） | N/A |
| invoke 成功（全量逐命令） | commands.json 全部 103 条 web-ok 命令 + 按参数 schema 机械生成的最小参数 | 双通道 resolve 值深等同构；底层请求形状同构：Tauri 收 `invoke(cmd, args)` ⇄ HTTP 收 `POST /api/cmd/{cmd}` + body=`JSON.stringify(args)` | N/A |
| invoke 业务错误 | AppError 13 variant 逐一（单键 map 形状） | 双通道 reject 值为同一 map 对象（含 invoke 拒绝值与 HTTP 200 body 解析值逐字节同构） | HTTP 侧以响应头 `X-Egosync-App-Error: 1` 判别（状态与 body 冻结款不变） |
| invoke 传输层错误 | 401 / 429 / 404 / 网络失败 | 仅 HTTP 侧存在：reject `HttpTransportError`（status + 解析后 body，形状测试钉死）；Tauri 分支无此路径 | 401 不触发任何跳转（全局拦截归 16.1） |
| 事件投递 | events.rs 全部 27 个事件名逐条 | Tauri 分支 listen 透传；HTTP 分支经单条 EventSource 多路复用，SSE 帧 `event:`/`data:` 解析为同名回调、payload JSON 同构 | listen/订阅失败静默容错（useTauriEvent 现状语义平移） |
| 前端本地事件 | `skill-scope-updated`（唯一前端→前端、Rust 无监听） | Tauri 分支保持 emit/listen 原样；浏览器分支进程内 EventEmitter 消化，不进 SSE 契约 | N/A |
| SSE 断连恢复 | EventSource onerror → 自动重连 → onopen-after-error | 状态机 connecting→online→reconnecting→online；恢复时 transport 层重放白名单 31 条（`{}` 重放）并将结果集经内部事件 `transport:reconnected`（前端本地事件）交付；写入类零重放；连续恢复只重放一次（并发防护） | 重放中单条 AppError 逐条入结果集不中断整体 |
| Tauri 连接状态 | 任意时序 | 恒 online；onConnectionStateChange 初始回调后永不变化；零重放 | N/A |
| 密钥边界 | llm_config_* 全量出参（真实 engine 响应） | 响应 JSON 任意层级无 `api_key` 字段（只有 api_key_ref） | N/A |
| 契约漂移 | 引擎侧增删命令/事件而未再生 TS 生成物 | CI 新鲜度门禁（regen+diff）与 vitest 运行时工件对等断言双红 | N/A |

</frozen-after-approval>

## Code Map

- `egosync-app/src/services/*.ts`（19 文件）-- invoke 全部 114 处的唯一所在（组件/hooks 零直接调用，companionService.ts:1 注释即此约定）；替换点=每文件顶部 `import { invoke } from '@tauri-apps/api/core'` 一行；参数惯例单层扁平 camelCase、可选 `?? null`（chatService.ts:17-19）、复合输入包 `input` 键（roleService.ts:5）；错误惯例 service 从不 catch、hook/组件 try/catch + setError——全部原样不动。
- `egosync-app/src/services/skillService.ts:10` -- 全仓唯一前端→前端 `emit('skill-scope-updated')`；调用方 SettingsTab.tsx:99、ButlerSettingsContent.tsx:94；消费方 ChatStream.tsx:1125-1131；测试锚点 ChatStream.test.tsx:2286-2331。
- `egosync-app/src/hooks/useTauriEvent.ts:4-31` -- 现任事件 hook（listen 异步、mounted 防泄漏、:23 静默容错、:29-30 deps 展开）；消费者 22 处/8 文件：App.tsx:71-176（notification:new/q2:reminder/briefing:generated/bigrock:reminder/bigrock:protection/review:generated/skill-registry-updated/role:proposed）、ChatStream.tsx:1012/1020/1125（llm:stream 高频流式/conversation:title-updated/skill-scope-updated）、OnboardingView.tsx:49/96、CompanionPairingSection.tsx:96/106/110（companion:paired|connected|disconnected——桌面壳事件非 engine events.rs 常量）、useNotifications.ts:39、useTasks.ts:33/52、useAllTasks.ts:36/56、SettingsTab.tsx:168。
- `egosync-app/src/App.tsx:2,240` + `components/layout/TitleBar.tsx:3,10-26` -- 唯二 `@tauri-apps/api/window` 耦合点：getCurrentWindow().show()（:240，web 下抛错被吞——改为探测跳过）；TitleBar isMaximized/onResized/minimize/toggleMaximize/close + :26 `data-tauri-drag-region`（App.tsx:349 无条件渲染——改为宿主门控，浏览器退化普通标题栏）。
- `egosync-app/src/test-setup.ts:3-32` -- 全局 `window.__TAURI_INTERNALS__` 桩（测试默认 Tauri 分支的机制基础，保留并补充说明注释）。
- `egosync-app/vite.config.ts` / `package.json:11-12` -- vite 默认入口+react 插件+alias '@'→src、envPrefix VITE_/TAURI_ENV_*；`test:frontend`=vitest run、`test:all`=vitest+壳 cargo+engine cargo、`build`=tsc && vite build（新增 `gen:transport` script 供 CI 新鲜度步骤与手动再生成调用，不改 test:all 构成）。
- `egosync-app/vitest.config.ts:5-17` -- jsdom、globals、setupFiles、include `src/**/*.{test,spec}.{ts,tsx}`（测试与源码同目录；新对等测试放 src/transport/ 自动纳入）；14 个 mock `@tauri-apps/api` 的测试文件清单见任务（12 处 core+1 处 event+DashboardTab.a11y）；另 4 个组件测试 mock hook 本身（机械改 mock 模块名）。
- `crates/egosync-engine/commands.json` -- 单源工件：`{version:1, count:103, commands:[{name,module,capability:"web-ok",ctxInjected,params:[{name,camelName,type,kind∈ctx-injected|client}]}]}`；本故事在顶层新增 `replayWhitelist`（31 条）。
- `crates/egosync-engine/src/events.rs:9-93` -- 27 个 `pub const *_EVENT: &str` 事件名常量（唯一发射源，无 skill-scope-updated——自洽）；TS 事件名清单生成源。
- `crates/egosync-engine/src/capabilities.rs` -- `DESKTOP_ONLY_COMMANDS` 15 条（:22-38）、`PERF_TEST_GATED_COMMANDS` 2 条（:42）、`is_web_command`/`capability_of`（Web 名单经 include_str! 工件发源 :76-95）；新增 `RECONNECT_REPLAY_COMMANDS` 常量。
- `server/src/bin/gen_commands.rs` + `server/src/dispatch_gen.rs` -- syn 扫描引擎命令源产出双工件（运行 `cd server && cargo run --features gen --bin gen_commands`）；:93-99 物理排除 desktop-only 15+门控 2；`dispatch(ctx,name,Value)->Result<Value,AppError>`；新增 replayWhitelist 字段导出；任何对命令注册面的改动不得破坏其扫描规则与壳 lib.rs:458-581 `commands::域::命令,` 行格式（parity_test 扫描依赖）。
- `server/src/routes.rs:22-63` -- POST /api/cmd/{command}：WEB_OK_COMMANDS 之外 404、空 body⇒`{}`、非 JSON⇒200+ValidationError、Ok/Err 均 200+`Json(value)`（**当前无错误判别信号**——本故事 Err 分支追加 `X-Egosync-App-Error: 1` 响应头，状态/体不变）。
- `server/src/sse.rs:85-110` -- SSE 帧 `event: <名>`/`data: <payload JSON>`、KeepAlive 30s、容量 256 滞后断开；`server/src/auth.rs` -- login 换 `egosync_session` httpOnly SameSite=Strict Cookie、`/api/auth/*` 与 setup 共享 5 次/分/IP 限流（**transport 侧必须缓存 status 结果**——15-4 Design Notes 遗嘱）。
- `server/tests/common/mod.rs:24-132` -- InProcessServer/Client/build_test_state 测试范式（对等 server 套件复用）；`server/tests/parity_test.rs` -- 既有注册面对等断言（120=103∪15∪2）；`server/tests/api_test.rs` -- 13 variant 白名单形状既有覆盖（判别头断言追加于此）。
- `.github/workflows/ci.yml:63-65` / `server-ci.yml:56-60` -- ci.yml 三平台跑 `test:frontend`（新增 TS 生成物新鲜度步骤）；server-ci 已有 commands.json/dispatch_gen 新鲜度（Rust 侧不动，TS 侧门禁挂 ci.yml）。
- `crates/egosync-engine/src/error.rs:3-36` -- AppError 13 variant 单键 map Serialize（epics「14」为规划期计数漂移，以代码为准）；variant 清单机械枚举自源码扫描（15-4 二轮先例）。

## Tasks & Acceptance

**Execution:**

- [x] `crates/egosync-engine/src/capabilities.rs` + `server/src/bin/gen_commands.rs` -- 新增 `RECONNECT_REPLAY_COMMANDS` 常量（31 条只读 query：app_get_butler_skills、app_is_first_launch、app_is_llm_configured、app_performance_snapshot、app_sidecar_status、briefing_get_latest、chat_get_butler_conversation、chat_list_conversations、dashboard_get_status、llm_config_list、mcp_server_list、mcp_server_list_available_for_butler、mcp_server_list_for_butler、memory_count、memory_list、memory_list_all、mission_get、notification_count_unread、notification_list、review_get_bigrock_suggestions、review_get_latest、role_list、role_list_archived、scheduler_get_times、settings_get_schedule、skill_list_all_role_skills、skill_list_registry、skill_list_selectable_for_scope、task_check_protection_status、task_list_all、task_list_butler）+ gen_commands 将其导出为 commands.json 顶层 `replayWhitelist` 字段（确定性排序）+ engine 单测钉名单 ⊆ web-ok ∧ 全部客户端参数可缺省（Option 或无） -- 重连补齐白名单的单一事实源
- [x] `server/src/routes.rs` + `server/tests/api_test.rs` -- AppError 响应分支追加响应头 `X-Egosync-App-Error: 1`（状态码 200 与单键 map body 冻结款零改动）；api_test 错误路径断言同步追加判别头断言 -- HttpTransport 解包业务错误的机械判别信号（Tauri 通道的对应信号=promise rejection，载荷两侧同构）
- [x] `crates/egosync-engine/Cargo.toml` + `server/Cargo.toml` -- serde_json 启用 `preserve_order` 特性 + 键序等价属性测试（构造字段序非字母序的样本，断言 `to_string(to_value(x)) == to_string(x)`） -- 消除 Value 往返的键序漂移，使 HTTP body 与 Tauri 直接序列化在字节层一致
- [x] `egosync-app/src/transport/{types.ts, index.ts, localEvents.ts}` -- Transport 接口（`invoke<T>(cmd,args?)` / `on(event,cb)->unlisten` / `capabilities` / `onConnectionStateChange`）+ `getTransport()` 懒单例（`window.__TAURI_INTERNALS__` 探测）+ `isTauriHost()` + 模块级 `invoke`/`emitFrontendEvent` 再导出 + 进程内 EventEmitter 与 `FRONTEND_LOCAL_EVENTS` 路由（skill-scope-updated、transport:reconnected） -- 传输抽象落位（架构目录命名：index/types/tauri/http/capabilities）
- [x] `egosync-app/src/transport/tauri.ts` -- TauriTransport：invoke 逐字节透传 `@tauri-apps/api/core`；`on` 把 listen 的 Promise<unlisten> 异步语义包成同步 unlisten（保留静默容错与卸载竞态防护——useTauriEvent 现状语义）；连接状态恒 online 初始回调一次；emitFrontendEvent 走 `@tauri-apps/api/event` emit -- 桌面行为零变化的基线实现
- [x] `egosync-app/src/transport/http.ts` -- HttpTransport：fetch `POST /api/cmd/{cmd}`（`credentials:'same-origin'`、Content-Type JSON、空参⇒`{}`）；200+判别头⇒reject body 解析值、200⇒resolve body、401/429/404/网络⇒reject `HttpTransportError{status,body}`；单条 EventSource `/api/events` 多路复用（按帧 `event:` 字段分发给全部同名 handler）；连接状态机（connecting→online→reconnecting→online）；onopen-after-error 触发白名单重放（`{}` 逐条、结果含错误也入集、并发防护只重放一次）并经 `transport:reconnected` 内部事件交付结果集；`getAuthStatus()` 首次请求后缓存（限流预算保护——15-4 遗嘱）；FRONTEND_LOCAL_EVENTS 走进程内不进 EventSource -- 浏览器宿主实现
- [x] `egosync-app/scripts/gen-transport.mjs` + `egosync-app/package.json` -- 零依赖 Node 生成器：读 commands.json（web-ok 103/desktop-only 15/门控 2/replayWhitelist 31）与 events.rs（正则解析 `pub const (\w+_EVENT): &str = "..."`，解析器同时导出供测试 import——单一解析器同源）；确定性输出 `src/transport/capabilities.ts`（清单常量+isWebCommand/isDesktopOnly 帮助函数）与 `src/transport/events.ts`（27 事件名清单+字面量联合类型）；`gen:transport` npm script；生成文件带生成头注释 -- TS 侧同源生成
- [x] `egosync-app/src/transport/{capabilities.ts, events.ts}`（生成物提交）+ `egosync-app/src/transport/{capabilities.test.ts, events.test.ts}` -- capabilities：运行时 fs 读 commands.json 断言生成清单==工件（对等断言「TS 能力清单==引擎注册表」）+ 白名单不变量（⊆ web-ok ∧ 参数可缺省）；events：三集合同源断言（events.rs 解析集==events.ts 导出集==测试枚举集）+ 逐事件名双通道路由测试 -- 漂移即红的机制层
- [x] `egosync-app/src/transport/parity.test.ts` -- 工件全量驱动的双通道对等套件：遍历 commands.json 全部 103 条 web-ok 命令，每条按参数 schema 机械生成最小参数 fixture（String→"x"、数值→1、bool→true、Option→省略、Vec→[]、结构体→{}）与确定性响应 fixture；Tauri 通道 mock `@tauri-apps/api/core` invoke（resolve fixture / reject AppError map），HTTP 通道 mock global fetch（200+body / 200+判别头+body / 401 等）；断言双通道 resolve/reject 深等同构 + 底层请求形状同构（invoke(cmd,args) ⇄ fetch(url,{body})）；13 variant 逐一错误路径黄金用例（variant 清单自 error.rs 源码机械枚举）；401/429/404 形状冻结断言（仅 HTTP 侧） -- 「同参数经双通道响应 JSON 逐字节同构（含 AppError 形状）」的机制化
- [x] `egosync-app/src/transport/{tauri.test.ts, http.test.ts}` -- 单元钉语义：tauri（同步 unlisten、mounted 竞态、恒 online、emit 透传）；http（SSE 帧解析、状态机转换、重放触发/不触发、写入类零重放、重放结果集含错误不中断、auth status 缓存只请求一次、FRONTEND_LOCAL_EVENTS 不建 EventSource）；EventSource/fetch 经 vi.stubGlobal 注入假实现（jsdom 无 EventSource）
- [x] `egosync-app/src/services/*.ts`（19 文件） -- 每文件一行：`import { invoke } from '@/transport'` 替换 `@tauri-apps/api/core`（函数签名零变化）；skillService.ts:10 `emit` 换 `emitFrontendEvent`（其余逻辑不动） -- services 层解耦
- [x] `egosync-app/src/hooks/useEngineEvent.ts` + 8 个消费文件 22 处迁移 + 删除 `hooks/useTauriEvent.ts` -- 签名与语义与旧 hook 完全一致（`useEngineEvent<T>(eventName, handler, deps=[])`，内部走 `getTransport().on`）；App.tsx/ChatStream.tsx/OnboardingView.tsx/CompanionPairingSection.tsx/useNotifications.ts/useTasks.ts/useAllTasks.ts/SettingsTab.tsx 全部 import 换名（事件名与 handler 逻辑零改动）；4 个 mock useTauriEvent 的组件测试机械改 mock 模块名 -- 事件 hook 交接
- [x] `egosync-app/src/components/layout/TitleBar.tsx` + `egosync-app/src/App.tsx` -- `isTauriHost()` 门控：TitleBar 浏览器分支不渲染窗口控制区与 `data-tauri-drag-region`（保留标题文本，布局细节不做）；App.tsx:240 `getCurrentWindow().show()` 仅 Tauri 分支执行（web 下不再依赖 catch 吞错） -- 桌面壳组件门控
- [x] `egosync-app/src/test-setup.ts` + 14 个 mock `@tauri-apps/api` 的测试文件 -- mock 对象替换：`vi.mock('@/transport', () => ({ invoke: vi.fn() }))` 范式替换 `vi.mock('@tauri-apps/api/core')`（12 core+1 event+DashboardTab.a11y，useNotifications 的 event mock 换 mock transport.on/useEngineEvent）；断言与测试意图零弱化（调用序断言原样保留）；test-setup 保留 `__TAURI_INTERNALS__` 全局桩并补注释说明「测试默认 Tauri 分支」 -- 测试基建随抽象迁移
- [x] `egosync-app/vite.config.ts` -- dev server 加 proxy `'/api' → http://localhost:8080`（架构 [M] 仅此项；构建产物与配置零分裂） -- 浏览器分支本地开发通道
- [x] `server/tests/command_parity_test.rs`（新增，工件驱动） -- include_str! commands.json 遍历 103 条：InProcessServer + build_test_state 独立 tempdir，`POST /api/cmd/{name}` body `{}`，断言非 404、状态 200、body 为合法 JSON、带判别头的响应 body 必为 13 variant 单键 map（判别头=错误分类器）；llm_config_* 响应递归扫描无 `api_key` 键；每命令 HTTP body 字节 == `to_string(parse(body))`（serde_json 规范编码往返一致——对象键序依赖 preserve_order，即其生效的逐命令证据）+ 键序等价属性测试（preserve_order 生效证明）。禁止二次 dispatch 比对（性能快照/UUID 类命令非确定性会误红） -- server 侧消费同一工件的全量对等
- [x] `.github/workflows/ci.yml` -- `test:frontend` 前插入新鲜度步骤：`cd egosync-app && npm run gen:transport && git diff --exit-code -- src/transport/capabilities.ts src/transport/events.ts` -- TS 生成物漂移门禁（Rust 侧门禁已在 server-ci）
- [x] 收口验证 -- `npm run test:all` 全绿 + `npm run build` tsc 零错 + `cd server && cargo test` 全绿（含新增 command_parity_test） + 工件再生成零漂移（commands.json/dispatch_gen/capabilities.ts/events.ts 四产物 regen+diff） + tests/e2e 桌面套件（15.2 裁决 A 口径执行+归因；环境级缺陷按 15-4 基线对照范式记录证据链） -- 硬边界

**Acceptance Criteria:**

- Given `src/transport/` 落位，when 实现完成，then Transport 接口四成员齐备 + TauriTransport（invoke/listen 行为与现状逐字节一致）+ HttpTransport（fetch POST /api/cmd + EventSource /api/events，Cookie 凭证 same-origin 自动携带）
- Given 运行时宿主探测，when `window.__TAURI_INTERNALS__` 存在与否，then 分别选中 Tauri/Http transport；Vite 构建配置单一零分裂（仅新增 dev proxy）
- Given services 改造，when import 替换，then 19 个 service 文件每文件一行级改动、函数签名零变化；全部 22 处 useTauriEvent 消费迁移 useEngineEvent、旧 hook 文件删除
- Given capabilities/events 同源，when 生成，then 构建期从 commands.json 工件与 events.rs 生成、仓库无手写双清单；对等断言 TS 清单==引擎注册表、事件三集合（engine 常量==TS 生成==测试枚举）同源一致
- Given 重连补齐协议，when SSE 断开恢复，then onConnectionStateChange 通知状态并重放白名单 31 条只读 query（白名单经 commands.json 工件单源）；写入类 command 永不重放；Tauri 分支恒 Online 不触发
- Given 桌面壳组件门控，when TitleBar/App 引用窗口 API，then 宿主探测门控：Tauri 下渲染原样、浏览器下窗口控制区退化为普通标题栏且 show() 跳过；skill-scope-updated 在浏览器分支进程内消化不进传输契约
- Given 测试迁移，when 14 个 vitest mock 文件替换，then mock transport 替换 mock 对象、测试意图与断言强度不变
- Given 工件驱动的全量对等，when 测试运行，then 103 条 web-ok command 逐条双通道用例（vitest 与 server cargo test 消费同一 commands.json）；13 个 AppError variant 的 200+单键 map 形状双通道一致；401/429/404 仅 HTTP 侧存在且形状冻结
- Given 密钥边界，when 扫描 llm_config 全量出参，then 响应 JSON 任意层级无 api_key 字段
- Given CI 接入与桌面回归，when 套件运行，then TS 生成物新鲜度门禁进 ci.yml、红=契约漂移必须双侧同步修复；`npm run test:all` 全绿、e2e 按 15.2 裁决 A 口径归因收口

## Implementation Notes

- **实现收口记录（2026-09-19，dev）**：Rust 四个传输面改动点全部落地（capabilities.rs RECONNECT_REPLAY_COMMANDS 31 条 + gen_commands replayWhitelist 导出 + routes.rs 判别头 + serde_json preserve_order 双 crate）；TS 侧 src/transport/ 七文件 + gen-transport.mjs 生成器 + 261 条传输/对等测试；19 service 文件 import 一行替换（git diff 逐文件验证仅 import 行变化）；useEngineEvent 平移 22 处消费/8 文件（useTauriEvent 删除）；14 个 mock 文件迁移；TitleBar/App.tsx 宿主门控。
- **验证链**：`npm run test:all` 全绿（vitest 48 文件/701 用例 + 壳 41+37 + engine 818，EXIT=0）；`npm run build` tsc 零错误 + vite 构建成功（786.54 kB，既有 chunk 警告）；`cd server && cargo test` 全绿（5 lib + 24 api + 2 command_parity + 5 parity + 4 secret = 40 用例，EXIT=0）；四产物 regen+diff 零漂移（commands.json/dispatch_gen.rs/capabilities.ts/events.ts 校验和前后一致）；桌面冒烟 xvfb release 构建 ≥35s 存活零 panic（独立 HOME 全新库，31+32 双池迁移全绿，delegate bridge 监听，keyring/dbus 降级 warn 属环境项）；server 冒烟 EGOSYNC_TOKEN env 态 + curl 四路径核验：未认证 401 `{"error":"unauthorized"}`、login 签发 httpOnly SameSite=Strict Cookie、role_list 200 `[]` 无判别头、AppError 200+`x-egosync-app-error: 1`+单键 map（NotFound 实证）、未知命令 404 `{"error":"unknown command"}`。
- **e2e 归因记录（2026-09-19，15.2 裁决 A 口径，共四 runs）**：
  1. **Run 1（基线原状条件，历史数据目录污染）**：9/9 全部止于会话创建超时——应用对历史库启动即失败 `migration 32 was previously applied but has been modified`。**归因（既有缺陷，证据链）**：032_auth_sessions.sql 在 15.4 评审修复提交 3a5c85c（#4 YAGNI 删 last_seen_at 列）中被修改，而历史开发库 `~/.local/share/com.egosync.desktop/egosync.db` 已按旧版 migration 32 落盘（checksum 7cabf4b1≠现文件 cc552550，sha384 直证）；本故事对 migrations 零改动（`git diff 1cfc163 -- crates/egosync-engine/migrations/` 为空），基线提交 1cfc163 同样含 3a5c85c ⇒ 同条件同败。sqlx 对已应用迁移的 checksum 变更拒绝启动是平台语义，非 15.5 回归。
  2. **Run 2（清库对照实验，15.2 范式：移走历史数据目录）**：**3/9 通过**（task-management 5/5、llm-streaming 3/3、performance 4/4）——较 15.2/15.3 基线（2/9：llm-streaming+task-management）通过集只增未减。正向执行级证据：应用经 15.5 传输层接线后正常启动渲染；任务 CRUD 全程穿行 service→`@/transport`→TauriTransport→engine；llm:stream 流式 UI 契约（输入框保持可用、发送复位）穿行 useEngineEvent→TauriTransport.on。
  3. **Run 3（复跑，未清残留 driver）**：9/9 会话创建超时——tauri-driver.log 8 次 `can not listen to address: 127.0.0.1:4444`/`FATAL ... 4445`：上一 run 泄漏的 driver 占用 4444 端口，后续每个 spec 重试各自再 spawn driver 全部撞端口。归因为 15.2/15.3 已在案的 driver 泄漏/抖动环境级缺陷（「e2e 平台修复（driver 泄漏）仍应作为独立工作项跟踪」原句）；与本故事无关（应用本体零报错）。
  4. **Run 4（终局复核：pkill driver + 清库）**：3/9 与 Run 2 完全一致。6 个失败 spec（accessibility 6 axe 检查、briefing-review 2、butler-conversation 1「分身管家标题」、cold-start-onboarding 1「首次启动应显示 Onboarding 视图」、conflict-arbitration 1、role-crud 4「保存更改/归档/恢复/删除」）逐项归因：全部为 15.2/15.3 基线既败组合（陈旧选择器文案漂移、beforeSession 擦错目录致 spec 间库态串扰、axe 在 wry 环境的既有失败、右键菜单交互漂移），无一件可归因 15.5 改动（本故事前端改动仅 import 行/事件 hook 名/宿主门控，全部经 vitest 701 用例钉死）。实验后已恢复原历史数据目录。
- **评审补丁轮（2026-09-19，第一轮 review 后）**：9 组补丁（G1 emit Promise 恢复基线签名 / G2 EventSource 致命关闭退避重建+桥接重挂 / G3 text() 中断包 HttpTransportError / G4 useEngineEvent 生命周期测试 5 例 / G5 TitleBar 浏览器退化+App show 门控测试 / G6 判别头 source-scan 耦合测试 / G7 engine 白名单 31 条全量值钉 / G8 状态订阅者隔离（订阅即回调+扇出双点）/ G9 gen-transport 守卫先于 map+空白名单合法化）。补丁后全量重跑验证：`npm run test:all` 全绿（vitest 51 文件/713 用例=701+12 新增；壳 41+1+1；engine 818）；`npm run build` tsc 零错误+构建成功；`cd server && cargo test` 全绿（5+24+2+5+4=40）；双侧工件 regen 零漂移；桌面冒烟 release 重建（4m52s）后独立 HOME 全新库启动 40s 存活零 panic（双池迁移全绿、delegate bridge 监听，sidecar/keyring 降级 warn 为既有环境项不变）。补丁期被测试套件拦下的自误一处：G3 首版 `.catch` 误包 AppError 业务 rejection（parity 118 用例变红）——改 `then` 第二参数只捕 text() 自身拒绝后全绿（对等门禁有效性的现场证据）。7 组 defer 入 deferred-work.md；驳回项理由逐条在 Review Triage Log。
- **评审补丁轮 e2e 归因记录（2026-09-19，15.2 裁决 A 口径，基线对照实验收口）**：补丁后 release 二进制（22:08 构建）共五轮 e2e 尝试，全部失败于会话/IPC 层，归因链如下——① 全套件首跑 9/9「4444 无法连接」：本 shell PATH 缺 `~/.cargo/bin`（tauri-driver not found，日志直证），非应用问题；② 补 PATH 后 9/9：1 spec 死于 IPC Origin 缺陷、8 spec 会话创建超时——tauri-driver.log 含 GTK 初始化 panic + 端口占用 15 处（泄漏连锁）；③ 单独 3 spec：同样全会话超时；④ 根因修正：shell 无 DISPLAY/XAUTHORITY（X99 残留 Xvfb 拒绝未授权连接，日志「Authorization required」直证）——`xvfb-run -a` 包装后会话创建全部成功，但 3/3 死于 `IPC app_complete_onboarding failed: Origin header is not a valid URL`（before-all 种子步骤）；⑤ **决定性基线对照实验**：`git worktree` 出基线提交 1cfc1636（早于一切 15.5 改动）构建二进制，同条件（xvfb-run + dev 的 DISPLAY/XAUTHORITY 双条件）跑同一 spec——**基线二进制死于逐字相同的 Origin 错误**。结论：IPC Origin 缺陷为环境级（wry/tao IPC 层对 WebDriver executeAsync 调用的 Origin 校验失败，15.4 已在案「按日漂移」项当日发作），与 15.5 改动无因果——失败路径为 `browser.executeAsync → __TAURI_INTERNALS__.invoke` 原生桥（零前端代码参与），本故事 Rust 改动全部 cfg(test) 不进 release 二进制，基线==补丁后二进制同败双证。dev 子代理当日 Run 2/4 的 3/9 通过证明传输层接线在真实桌面路径可用（task CRUD/llm:stream/perf 三个 spec 穿行 service→transport→engine）；当日 e2e 平台缺陷（driver 泄漏、DISPLAY 授权漂移、IPC Origin）建议随 15.2 先例作独立工作项跟踪（deferred-work G15 条目在案）。
- **实现裁量记录**：① Transport 接口含 4 核心成员 + `emitFrontendEvent`（spec tauri.ts 任务本身要求传输层实现它，模块级再导出委托 `getTransport()`）——与 spec 字面「Transport 接口（invoke/on/capabilities/onConnectionStateChange）」并存无冲突；② mock useTauriEvent 的测试文件实测 5 个（spec 写 4——useAllTasks.test.tsx 是 hook 测试也在 mock 之列，机械计数为准），5 个全部迁移；③ commands.json 因 preserve_order 全面改键序（插入序 vs 旧字母序）致大 diff（851+/818-），确定性与再生成幂等已由校验和零漂移证明；④ e2e 环境级缺陷（历史库 migration 32 checksum、driver 端口泄漏）建议按 15.2 先例作独立工作项跟踪，其中 032 修改违反「已应用迁移不可变」sqlx 约定值得在 15.6 前修（恢复 last_seen_at 或提供修复迁移）。

## Spec Change Log

## Review Triage Log

### 第一轮（2026-09-19，三层并行评审：盲审猎手 16 条 / 边界猎手 10 条 / 验证缺口 3 主 2 附）

**逐条裁决**（发现 → 判定 + 证据）：

1. [盲审] `emitFrontendEvent` 用 `void` 丢弃 emit Promise —— **high**：亲证基线 `notifyScopeUpdated` 返回 `Promise<void>`（`emit(...)` 直返），调用方 SettingsTab.tsx:97-103 / ButlerSettingsContent.tsx:91 的 `await + catch + setError 兜底`沦为死代码、emit rejection 变 unhandled rejection；违反冻结 AC「函数签名零变化」。
2. [盲审] capabilities 生成物零生产消费者、浏览器 desktop-only 入口未门控 —— **low**：真（浏览器分支点配对/数据导出会 404），但 capability 门控 UI 从未存在于任何宿主（先于本故事的缺口、非本故事造成），浏览器宿主 16.1 才对用户可达（Never 条款：静态服务/登录页归 16.1），epic 16.1 AC 明确承接 → defer。
3. [盲审] getAuthStatus 不在 Transport 接口 + 失败路径不缓存 —— **low**：接口位置系规格任务文本裁定（冻结 AC 四成员接口；auth 仅 HTTP 侧概念）；失败不缓存实为正确语义（网络故障应重试、`/api/auth/status` 恒 200——auth.rs 亲证）；真实关切=16.1 登录接线需要缓存失效策略 → defer（16.1）。
4. [盲审] 判别头常量 TS/Rust 双写无耦合门禁 —— **medium**：真（双侧各写字面量，server 改名后双侧测试照绿、集成静默断裂）；本故事自建的契约串应有机械门禁（source-scan 先例：events.rs 正则解析进测试）→ patch。
5. [盲审] 对等套件双通道皆 mock、真实双宿主字节对等无自动化证据 —— **low**：真（fixture 对称证明的是映射对称而非真实载荷）；间接证据链在（同 engine + preserve_order 属性测试 + canonical 往返 + e2e 3 spec 穿真 invoke），强化需 16.1 web e2e → defer。
6. [盲审] 重连补齐只覆盖查询不覆盖事件 —— **low，驳回（意图排除）**：冻结 AC「重放白名单 31 条只读 query」即人批的恢复协议（查询重放=断连窗口状态补偿的设计选择），事件回放协议不在批准意图内；16.2 消费语义可复议。
7. [盲审] 重放风暴无节流 —— **low，驳回（规则 82）**：单用户 31 条只读 POST、EventSource 原生退避管频率；节流=守卫未演示状态。
8. [盲审] engine 白名单测试「值钉」名不副实 —— **low**：真（只钉 len==31 + ⊆ web-ok，换成员本地不红、仅 CI 新鲜度门禁兜底）；events.test.ts 27 事件全量值钉先例在 → patch。
9. [盲审] `vi.mock('@/transport', () => ({ invoke }))` 部分 mock 范式脆弱 —— **low，驳回（规则 82）**：701 用例全绿无实际破坏；共享 mock 工厂重构超出直接修正。
10. [盲审] test:all 不含 server crate —— **low**：真（本地跑不到 server 对等套件），但规格 Code Map 明言「不改 test:all 构成」且 server-ci 工作流 CI 兜底；聚合属 chore → defer。
11. [盲审] 测试以 process.cwd() 锚定仓库根 —— **low，驳回（规则 82）**：日常使用（egosync-app 下 npm run test / CI）不受影响；向上搜索助手=robustness 增强超出直接修正。
12. [盲审] command_parity_test `tempdir().keep()` 每次泄漏临时目录 —— **low，驳回（规则 82）**：每 run 一目录的有意调试成本；清理逻辑=守卫。
13. [盲审] gen-transport 守卫死代码 + 空白名单误报「缺字段」+ d.mts 无漂移检查 —— **low**：守卫顺序 bug 亲证为真（`doc.commands.map` 先于 `Array.isArray` 守卫执行，键缺失时 raw TypeError）→ patch；d.mts 漂移部分驳回（规则 82：小文件人肉同步成本可接受）。
14. [盲审] 413/5xx 未钉形状 + 重放结果集混 Error 实例 —— **low**：413/5xx 与 404 走同一非 200 分支（形状已冻结、同路径不同状态码，该半驳回）；Error 实例混入 `transport:reconnected` 结果集=16.2 消费形状问题 → defer（16.2 信封）。
15. [盲审] commands.json 键序翻转产生机械噪声、语义变更被淹没 —— **false**：插入序=struct 字段序，确定性且 regen 幂等（校验和零漂移亲证）；未来该工件 diff 只含语义变更，翻转是一次性成本已吸收。
16. [盲审] 首连失败永停 connecting、消费方无法区分「正在连」与「离线重试」 —— **low，驳回（规则 86）**：四态状态机与「首 onopen 前 onerror 保持 connecting」为规格冻结语义，修复=改本故事规格；16.2 复议建议随行记录不改路由。
17. [边界] `res.text()` 中途 reject 裸漏违反 HttpTransportError 契约 —— **medium**：亲证裸 then 链（http.ts:58-78），body 读取中断时原始流错误逃出契约；冻结矩阵「网络失败 ⇒ HttpTransportError{status:0,body:null}」覆盖 mid-body reset → patch。
18. [边界] /api/events 非 200 → EventSource 致命关闭永不恢复 —— **medium**：亲证 onerror 无 readyState CLOSED 分支（http.ts:152-158）；WHATWG 对非 200=永久关闭、原生自动重连不适用 ⇒ 冻结 AC「onopen-after-error 触发重放」在致命关闭路径永不触发 → patch。
19. [边界] 状态订阅者抛错中断扇出与恢复重放 —— **low**：亲证 setState 裸循环（http.ts:165-167），onopen 内 setState 先于 replayAfterReconnect，一个坏订阅者断全部状态通知与恢复 → patch。
20. [边界] getAuthStatus 并发首调重复请求 —— **low，驳回（规则 82）**：无现存消费者（16.1 才接线）、burst 可忽略；单飞字段=守卫未演示状态。
21. [边界] emit Promise 丢弃（unhandled rejection）—— 同 #1（同根因）。
22. [边界] gen 脚本 commands 键缺失时 raw TypeError —— 同 #13 守卫部分（同根因）。
23. [边界] 主张核查：services 签名零变化不实（notifyScopeUpdated 返回 void）—— 同 #1（同根因，高置信 claim）。
24. [边界] 主张核查：TauriTransport 零行为变化被 emit 违反 —— 同 #1（同根因，高置信 claim）。
25. [边界] 主张核查：SSE 恢复协议在致命关闭路径失效 —— 同 #18（同根因）。
26. [边界] 主张核查：auth status 失败不缓存重打限流 —— **low**：主张属实（仅成功缓存，与任务文本「首次请求后缓存」有措辞偏差），但失败重试是正确方向、429 自放大需消费者存在（16.1 侧退避更对位）→ 并入 #3 defer。
27. [验证缺口] useEngineEvent 零真实观测（22 消费点的测试全部 mock 该 hook）—— **high**（预验证采纳）：删 deps 或清理函数不会红任何测试，核心「桌面行为零变化」保证无测试钉 → patch。
28. [验证缺口] TitleBar/App 浏览器退化分支零验证（vitest 全程 Tauri 桩）—— **medium**（预验证采纳）：浏览器分支渲染与 show() 门控无任何断言 → patch。
29. [验证缺口] HTTP/SSE 契约串（URL 前缀）双写无耦合测试 —— **low**：真；但真耦合验证=浏览器 transport 对真 server=16.1 web e2e 领地（Never 条款排除本故事做 web e2e）→ defer；判别头部分由 #4 patch 覆盖。
30. [验证缺口·附] e2e 红区盲区：基线即红的 6 spec 内的回归不可见 —— **low**：真（e2e CI 禁用 + 基线红）；登记风险册 → defer。
31. [验证缺口·附] getAuthStatus 缓存永不失效（pre-login false 钉死整个进程期）—— **low**：真，16.1 登录流必须处理 → 并入 #3 defer。

**分组与路由**（同根因归组、组内最高判定定级；无 intent_gap/bad_spec ⇒ 无 loopback，review_loop_iteration 维持 0）：

- **patch × 9 组**：G1（#1/#21/#23/#24 emit Promise，high）／G2（#18/#25 致命关闭不恢复，medium）／G3（#17 text() 裸漏，medium）／G4（#27 hook 无测试，high）／G5（#28 浏览器退化无测试，medium）／G6（#4 判别头耦合门禁，medium）／G7（#8 白名单值钉，low）／G8（#19 状态扇出容错，low）／G9（#13/#22 gen 守卫，low）。
- **defer × 7 组**：G10（#2 capability UI 门控 → 16.1）／G11（#3/#26/#31 auth 缓存失效 → 16.1）／G12（#5 真实双宿主字节证据 → 16.1 web e2e）／G13（#14 重放错误信封 → 16.2）／G14（#29 URL 契约耦合 → 16.1 web e2e）／G15（#30 e2e 红区盲 → 风险册）／G16（#10 test:all 聚合 → chore）。
- **驳回**：#6（意图排除）／#7 #9 #11 #12 #20（规则 82）／#13-d.mts 部分（规则 82）／#15（false：regen 幂等亲证）／#16（规则 86）。

**处置记录**：实施子代理非后台代理、不可续用（规程允许：cannot be continued ⇒ 本人应用补丁）。9 组补丁由会话本人应用后全量重跑 Verification 命令；7 组 defer 追加至 deferred-work.md。

## Design Notes

- **AppError 判别头（关键设计裁量）**：server 现状对 Ok/Err 一律 200+`Json(value)`（routes.rs:53-57），HTTP 层无任何错误判别信号；而 Tauri 通道的错误信号=promise rejection（rejection 值即单键 map）。备选方案「前端按单键对象且键 ∈ 13 variant 名探测」被驳回：成功响应恰为单键 variant 名对象时误判，且该风险无法用机械手段排除（工件无返回值 schema）。裁决：Err 分支追加 `X-Egosync-App-Error: 1` 响应头——状态码与 body 冻结款零改动，头只是 HTTP 通道的错误信号（与 Tauri 的 rejection 信号对偶），解包后两侧错误对象同构；api_test 同步断言头存在。
- **对等用例「全量生成」的实现形态**：不生成 `.gen.test.ts` 代码文件，而是测试运行时遍历 commands.json 工件逐条构造用例（vitest fs 读文件 + cargo include_str!）——覆盖面机械等于工件、测试代码内零手写命令名/事件名，等价于「由工件全量生成」且无再生时间窗（比代码生成更新鲜）；capabilities.ts/events.ts 两个 TS 模块仍走代码生成+提交+新鲜度门禁。测试的 events.rs 正则解析器与生成器共用同一实现（scripts/gen-transport.mjs 导出）——解析逻辑单一来源。
- **键序与逐字节同构**：serde_json 默认 Value::Object 为 BTreeMap（字母序），`to_string(to_value(T))` 与直接 `to_string(T)` 键序不同——桌面 invoke（直接序列化）与 server body（经 dispatch 的 Value 往返，dispatch_gen.rs:672-673）会差在键序。修法=engine 与 server 启用 serde_json `preserve_order` 特性（IndexMap 保插入序=结构体字段序），配键序等价属性测试；桌面构建经 cargo 特性统一随 engine 获得同款行为（双宿主一致）。fixture 层的逐字节断言由 vitest 双通道共享同一 fixture 字符串天然满足。
- **事件 payload 类型生成范围裁量**：架构决策 #2 写「TS 侧事件名与 payload 类型构建期从 events.rs 生成」——events.rs 仅含事件名常量（payload 为 serde_json::Value，无类型信息），payload 类型从 Rust 侧生成需引入 ts-rs/specta 全套机制且非本故事任何 AC 所要求（三集合 AC 只覆盖事件名）。裁量：15.5 只生成事件名清单与字面量联合类型，payload 类型维持组件内手写接口现状；该偏差登记于此供人工复核。
- **companion:paired/connected/disconnected 三事件**：桌面壳 companion service 发射（非 engine events.rs 常量、不进 SSE 契约）；CompanionPairingSection 为桌面专属功能，useEngineEvent 的 Tauri 分支照常收听、浏览器分支自然永不触发——不属违例，登记防评审误报。
- **重连状态机与内部事件**：初始 connecting（首 onopen 前的 onerror 保持 connecting）→ online → onerror 转 reconnecting → onopen 恢复 online 并重放；EventSource 原生自动重连（浏览器管退避，架构「应用层只管状态呈现」）。重放结果集经 `transport:reconnected` 前端本地事件交付（payload={results: {命令名: 结果或错误}}），UI 消费归 16.2——本故事只钉协议与测试。白名单=「全部 `{}` 可重放（无客户端参数或全 Option）的只读 query」人工冻结 31 条，机械不变量（⊆ web-ok ∧ 参数可缺省）由 engine 单测与 vitest 双侧钉死。
- **HttpTransport 生命周期**：EventSource 懒建（首个 on() 订阅时）且进程存活期不主动关闭（应用级单连接）；测试经构造函数/DI 或 vi.stubGlobal 注入假 fetch/EventSource，不依赖全局桩。`getAuthStatus()` 缓存防 `/api/auth/status` 轮询消耗 login 的 5 次/分限流预算（15-4 Design Notes 遗嘱落地）。
- **计数勘误（机械计数为准）**：service 文件 19（epics 写 22）；AppError variant 13（epics 写 14，15-4 同款先例）；mock @tauri-apps/api 的测试文件恰 14（与 epics 一致：12 core + 1 event + 1 a11y）。
- **e2e 环境既有缺陷预期**：15-4 记录的 WebDriver 会话超时与 IPC Origin 缺陷按日漂移；若复现按 15-4 基线对照范式（checkout 基线提交对照实验）归因记录，不宣称通过。

## Verification

**Commands:**

- `cd egosync-app && npm run test:all` -- vitest（含新增 transport/对等/迁移后全部既有测试）+ 壳 cargo + engine cargo 全绿，EXIT=0
- `cd egosync-app && npm run build` -- tsc 零类型错误 + vite 构建成功
- `cd server && cargo test` -- 全绿（api_test 判别头断言 + parity_test + 新增 command_parity_test 103 条全量）
- `cd server && cargo run --features gen --bin gen_commands && git diff --exit-code -- crates/egosync-engine/commands.json server/src/dispatch_gen.rs` -- Rust 工件新鲜度零漂移
- `cd egosync-app && npm run gen:transport && git diff --exit-code -- src/transport/capabilities.ts src/transport/events.ts` -- TS 生成物新鲜度零漂移
- `cd egosync-app/tests/e2e && npm test` -- 桌面 e2e 执行 + 既有缺陷逐项归因（15.2 裁决 A 口径，证据链入 Implementation Notes）
- 桌面冒烟：dev 或 release 构建启动 ≥30s 零 panic（transport 接线后桌面启动路径必测——15.x 系惯例）
- server 冒烟（对等链路人工核验）：`EGOSYNC_TOKEN=devtoken123 EGOSYNC_DATA_DIR=$(mktemp -d) cargo run` + curl 判别头路径——login 取 Cookie 后 POST /api/cmd/role_list 断言响应与未认证 401 形状

**Manual checks (if no CLI):**

- 无
