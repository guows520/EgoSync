# Epic 15 Context: 引擎无头化与服务器形态（Headless Engine & Server）

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

将 EgoSync 全部业务逻辑物理迁入 `crates/egosync-engine`（无 tauri 依赖），桌面 Tauri 壳经四条接缝注入宿主能力、行为逐字节一致；新建 axum server binary 以同一 command 注册表暴露 HTTP/SSE API，单用户认证一体交付；前端获得双传输抽象与全量对等验证。引擎由此可脱离 Tauri 运行于任意宿主——这是云端托管赛道（自托管单用户、云脑单源、V1 收尾后启动）全部后续故事的唯一前置；桌面版同时独立获得引擎物理分层、并发护栏与事件常量源。复杂度不在新代码量，而在"改动既有引擎代码而不改变桌面行为"的抽取纪律。

## Stories

- Story 15.1: 引擎 crate 骨架、宿主接缝与纯模块平移
- Story 15.2: 事件总线、Handle 接缝与泛域服务迁移
- Story 15.3: chat/agent_engine 域迁移与 ChatSessionRegistry 抽取
- Story 15.4: server binary 与单用户认证
- Story 15.5: 前端双传输与对等测试套件

## Requirements & Constraints

- **桌面零回归（硬边界）**：每个故事收口 = `npm run test:all` + tests/e2e 桌面套件全绿，跳过任何一项即未完成；抽取期禁止顺手重构被平移的代码。
- **传输对等**：同一 command 名、参数形状、返回形状、事件名/载荷、错误形状在 invoke 与 HTTP/SSE 双通道同构；契约变更必须双通道同步，禁止单侧改契约让测试变绿。
- **服务器形态**：单实例单用户自托管，无多租户、无注册体系（任何"以防万一"的多租户抽象都是违约）；除 healthz 与静态资源外全端点强制认证，认证失败统一 401 不泄露用户存在性。
- **云脑单源**：无任何跨实例复制/同步代码路径；远程模式/多客户端只是视图。
- **密钥与安全**：LLM Key 只存在于 SecretStore 实现之后，任何 API 响应不含 key 字段（浏览器只见 api_key_ref）；CSP 严格、CORS 显式拒绝跨源；密钥与事件 payload 明文永不入日志。

## Technical Decisions

- **「同一引擎、双宿主」范式**：桌面壳与 server 都是薄适配层；宿主差异只允许存在于四条接缝——EngineEvents / SecretStore / sidecar 路径注入 / runtime Handle 注入——与 command 注册表；任何"云端专属业务逻辑"都是对本范式的违约。
- **事件契约**：EngineEvents 签名为 `emit(event: &str, payload: serde_json::Value)`（对象安全，禁泛型方法）；`events.rs` 事件名常量是唯一发射源，TS 侧事件名与 payload 类型构建期生成；发射点"强类型 payload → 常量名+Value"机械改写是抽取期唯一豁免"禁止顺手重构"的改写。
- **engine crate**：Cargo.toml 物理不声明 tauri/keyring（CI 兜底断言）；仓库根禁建 Cargo workspace，src-tauri 与 server 以 path 依赖引用；四个 companion_* service（pairing/connection/dispatch/snapshot）留桌面壳不迁——桌面宿主专属功能，云端无消费者。
- **Handle 注入**：三个同步启动入口（spawn_scheduler / 两个 spawn_hourly_watch）必须接收注入的 `tokio::runtime::Handle`——tauri setup 闭包无 runtime 上下文，裸 tokio::spawn 会 panic（历史启动事故前车之鉴）；验收必含桌面启动路径 e2e 断言。
- **commands.json 构建期工件**：每条 command 含名字、参数 schema（`State<'_, T>` 注入参数与客户端参数由 schema 标注区分）、capability 标记；四方消费（桌面 generate_handler / server 路由 / 前端 capabilities / 对等测试），对等用例由工件全量生成（禁抽样禁手抄）；desktop-only = 4 个 rfd 对话框 command + 7 个 companion_* command + data_export/data_import（云端走 /api/export、/api/import HTTP 流，同一能力双落点）。
- **server（axum 0.8）**：`POST /api/cmd/{command}`（参数 camelCase 与 invoke 同构）+ `GET /api/events`（SSE，KeepAlive 30s 心跳，broadcast 扇出，慢客户端滞后断开）+ 两级 healthz（存活/深度）；错误白名单冻结：全部 AppError variant 一律 200 + 原样单键 map JSON，非 200 仅 401/429/404/进程级 5xx 四类；body 上限 50MB；业务端点禁用响应超时（chat 分钟级挂起是正常态）；禁止手工维护路由映射表与旁路 API 端点。
- **认证**：`EGOSYNC_TOKEN` env 或首访 `/api/setup` 设置，服务端只存 Argon2id 哈希；登录换 httpOnly SameSite=Strict Cookie（EventSource 不能带 Authorization 头，SSE 必须走 Cookie）；env/setup 优先级冻结——env 存在 ⇒ setup 拒绝、login 仅比对 env，切换须重启，禁止任何 fail-open 组合读法；auth/setup 端点按 IP 限流 5 次/分钟；无任何 dev 免认证旁路。
- **前端 transport**：`src/transport/` 双实现（TauriTransport 与现状逐字节一致 / HttpTransport fetch+EventSource）+ `window.__TAURI_INTERNALS__` 运行时探测；单一 Vite 构建产物双宿主复用（不做双构建）；`useEngineEvent` 接替 useTauriEvent；capabilities.ts 构建期从 capabilities.rs 生成，禁手写双清单；重连后由 transport 层重放只读 query 白名单（写入类 command 永不重放，防副作用重放）；`skill-scope-updated` 为前端→前端事件，浏览器分支进程内消化，不进传输契约。

## UX & Interaction Patterns

- 能力门控可见性：desktop-only 能力在 web 端入口不出现——经 capabilities 驱动显隐，而非运行时报错。
- TitleBar/App 引用的窗口 API 按同一宿主探测门控：Tauri 下渲染原样，浏览器下窗口控制区退化为普通标题栏（布局细节归后续 UX 定稿）。

（WEB UX 章节尚未产出；UX 定稿后的新约束以勘注补入。）

## Cross-Story Dependencies

- 故事强顺序 15.1 → 15.2 → 15.3 → 15.4 → 15.5（绞杀者逐步抽取，每步桌面全绿收口才进下一步）。
- 抽取对象为桌面既有 Epic 1～11 已实现的引擎代码；与 Epic 12～14（手机伴侣）无交叉，伴侣代码零改动。
- 本 Epic 是 Epic 16（WEB 客户端）与 Epic 17（自托管部署运维）的唯一前置；15.5 传输对等基建也是桌面远程模式（16.3，可选后置）的前置。
- 15.4 产出的 commands.json 工件是 15.5 对等测试与 capabilities 生成的直接输入。
