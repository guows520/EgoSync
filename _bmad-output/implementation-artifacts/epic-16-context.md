# Epic 16 Context: WEB 客户端（Web Client）

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

让任意现代浏览器（桌面/移动，含 iOS Safari）访问云端实例即可使用核心体验——流式对话、任务操作、建议确认/拒绝、仪表盘、简报复盘；刷新或断线重连后恢复状态、不丢已产生消息；与桌面版视觉零分叉（同一 React 组件体系、同一 Vite 构建产物双宿主复用），desktop-only 入口不出现。本 Epic 交付 FR-45 全部验收与 FR-46 前端流程（认证后端已在 Epic 15 落地）；FR-48 桌面远程模式作为可选后置故事 16.3 在此交付，不阻塞云端版首版验收。

## Stories

- Story 16.1: WEB 入口、认证与首访流
- Story 16.2: 实时事件、流式体验与浏览器适配
- Story 16.3: 桌面客户端远程模式（FR-48，可选后置）

## Requirements & Constraints

- **浏览器核心体验（FR-45）**：桌面/移动浏览器（含 iOS Safari）无需安装即可登录并完成核心操作；流式输出实时逐字渲染，体验与桌面一致（停止、同会话 busy 提示语义一致）；核心功能集分档清单归 UX 阶段定稿（尚未产出），未定稿前按流式对话/任务操作/建议确认拒绝/仪表盘/晨间简报与周复盘清单走查。
- **刷新与重连恢复**：服务端是事实源——已产生消息已落库，刷新后重拉；进行中流式会话按服务端状态呈现已完成部分，不伪造进行态。
- **认证前端流程（FR-46 前端侧）**：无注册体系；未初始化实例首访呈现 setup 向导（中文文案）设置令牌，env 令牌锁定态如实呈现说明；无有效 Cookie 访问任意业务端点（含 SSE）一律 401 全局拦截回登录页；令牌错误统一失败文案，不泄露实例/用户存在性；登出清会话回登录页。
- **浏览器不落业务数据（NFR-C3）**：无 localStorage/IndexedDB 业务缓存，业务数据仅内存态持有（仅允许主题偏好等非业务态）；浏览器永不见 LLM Key——API 出参只有 api_key_ref。
- **desktop-only 入口不出现**：选工作目录、配对管理、本地数据导入导出等能力经 capabilities 驱动显隐，而非运行时报错。
- **断线诚实明示（NFR-C7）**：连接状态（连接中/在线/重连中）可见呈现，不静默失败；SSE 消费与 invoke 通道同构（NFR-C6 消费侧验证）。
- **16.3（FR-48）**：切换远程 = 本地引擎待机、不产生本地数据副本；切换是连接目标变更，永不触发数据合并/同步；重启桌面 app 记忆上次模式。

## Technical Decisions

- **单一 Vite 构建产物双宿主复用**：server 内嵌静态与桌面 frontendDist 是同一份 dist，不做双构建；运行时探测 `window.__TAURI_INTERNALS__` 选 TauriTransport 或 HttpTransport（fetch `POST /api/cmd/{command}` + EventSource `/api/events`）。
- **静态服务口径**：目录经 `EGOSYNC_STATIC_DIR` 注入、缺省回退 `../egosync-app/dist`、皆缺则 API-only 警告运行；SPA 回退 index.html；`/api/*` 未知路径 JSON 404 不落 SPA 面；index.html 直出且 no-cache（hash 资产长缓存，防重部署白屏）。
- **事件与流式**：useEngineEvent（useTauriEvent 的传输无关继任者，15.5 交付）全量接入，组件层零改动；SSE 事件名/载荷与 emit 同构；`skill-scope-updated` 为唯一前端→前端事件，浏览器分支进程内消化、不进传输契约；SSE 全客户端广播 = 桌面 emit 多窗口语义；多端并发编辑令牌级 last-write-wins，无会话管理协议。
- **重连补齐（transport 层职责 + 16.2 视图消费口径）**：SSE onopen-after-error 触发重连信号（Tauri 分支恒在线不触发）；重放冻结的只读 query command 白名单，写入类一律不重放（防 notification_ack 副作用重放）；白名单进 commands.json 工件；UI 三面消费（16.2 落地）——①连接状态（connecting/online/reconnecting 三态，徽标+重连横幅）；②白名单命中面（通知/任务/仪表盘/角色列表直接消费重放结果集，带参命令以其为触发器定向重拉，聊天历史由 ChatStream 定向补拉）；③流式恢复（重连时流式态复位 + 历史重拉，is_complete=false 行诚实呈现「生成中」占位）。
- **能力门控同源**：capabilities.ts 构建期从 engine capabilities.rs 生成，禁止手写双清单（漂移后果 = WEB 入口可见但请求 404）。
- **桌面壳组件门控**：TitleBar 与 App 的 window API 引用按同一宿主探测门控——Tauri 下渲染原样，浏览器下 TitleBar 退化为普通标题栏；Tauri 启动逻辑（单实例、窗口准备）按探测跳过。
- **认证落地口径**：登录 Cookie httpOnly + SameSite=Strict + Max-Age 30 天（关浏览器免重登）；登出端点幂等删会话行 + 过期 Cookie；SSE 用 Cookie 凭证（EventSource 不能带 Authorization 头）。
- **错误语义**：业务错误 = 200 + AppError 原样 JSON（transport 统一解包，双通道同构）；非 200 仅 401/429/404/5xx 四类传输层错误。
- **CSP/安全头**：以 'self' 为基线，仅放行 Google Fonts 两域（style/font）与 index.html 内联主题脚本的 hash-source（byte 对 byte 契约测试守门）；另加 nosniff、X-Frame-Options: DENY、Referrer-Policy: no-referrer。
- **16.3 状态机**：LOCAL（默认）→ REMOTE_CONNECTING → REMOTE_ONLINE；不可达 → REMOTE_OFFLINE 指数退避（1s→30s 封顶+抖动）；LOCAL→REMOTE 守卫 = 本地引擎完整停机（sidecar 杀进程、调度器取消、连接池关闭）后才切传输；REMOTE 态桌面 = 浏览器等价物，无本地数据兜底。
- **通知降级**：WEB 端通知为应用内通知，无 Service Worker 推送（页面关闭即无通知；iOS Safari 普通浏览无 Notification API）。
- **测试**：vitest 用 mock transport 覆盖 setup（含 env 锁定态）/登录/401 拦截/登出；e2e web 模式（本地 server + 浏览器驱动）流式/事件/断线重连至少各一条用例；桌面 e2e 保持零改动全绿。

## UX & Interaction Patterns

- **视觉零分叉为硬验收**：WEB 与桌面版并排截图对比，唯一允许差异 = 宿主门控项（TitleBar 退化、desktop-only 入口缺席）；截图留档进 PR。
- **响应式基线**：375px 级移动视口下导航/对话/任务/仪表盘不溢出、可操作；断点与移动交互细节由 UX 阶段定稿，本 Epic 交付可用基线；iOS Safari 100vh 地址栏抖动等已知问题需缓解。
- **WEB UX 专属章节尚未定稿**：定稿后新约束以勘注补入、不回改既有拆分；未定稿前遵循既有仪表盘视觉优先、对话常驻、键盘可操作原则，不虚构额外视觉方案。

## Cross-Story Dependencies

- **前置 Epic 15**：16.1/16.2 直接消费 15.4 的 server API 面/认证端点（/api/auth/status、login、logout、/api/setup、401 语义）与 15.5 的 Transport 抽象、useEngineEvent、capabilities 同源生成、只读补齐白名单。
- **故事顺序**：16.1 → 16.2 严格串行（入口与认证一体先行交付）；两者与 Epic 17（17.1→17.2→17.3）并行，同为 Epic 15 的后继。
- **16.3 可选后置**：依赖 15.4/15.5 + 16.1–16.2 + 产品决策，不阻塞云端版首版验收。
- **下游**：16.2 的浏览器驱动 e2e 是 17.3 tests/e2e web 模式的前置能力（桌面模式零改动）；16.1 的首访 setup 被 17.1 部署文档引用（从零到浏览器可访问路径）。
