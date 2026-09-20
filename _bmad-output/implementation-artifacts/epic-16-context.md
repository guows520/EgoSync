# Epic 16 Context: WEB 客户端（Web Client）

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

让任意现代浏览器（桌面/移动，含 iOS Safari）成为云端实例的可用客户端：登录后使用核心体验——流式对话、任务操作、建议确认/拒绝、仪表盘、简报复盘；刷新/断线重连后不丢已产生消息；视觉与桌面版零分叉（同一 React 组件体系、同一 Vite 构建产物双宿主复用）；desktop-only 入口不出现。本 Epic 交付 FR-45 全部验收与 FR-46 前端流程（认证后端已在 Epic 15 落地）；FR-48 桌面远程模式作为可选后置故事在此交付，不阻塞云端首版验收。

## Stories

- Story 16.1: WEB 入口、认证与首访流
- Story 16.2: 实时事件、流式体验与浏览器适配
- Story 16.3: 桌面客户端远程模式（FR-48，可选后置）

## Requirements & Constraints

- **浏览器核心体验（FR-45）**：桌面与移动浏览器（含 iOS Safari）无需安装任何应用即可登录并完成核心操作；流式输出实时渲染、体验与桌面一致；刷新或断线重连恢复会话状态、不丢已产生消息。核心功能集分档清单归 UX 阶段定稿（尚未产出），未定稿前按流式/任务/建议确认拒绝/仪表盘/简报复盘清单走查。
- **认证前端流程（FR-46 前端侧）**：无注册体系；未初始化实例首访呈现 setup 向导（中文文案）设置令牌，env 令牌锁定态如实呈现说明；已初始化实例无有效 Cookie 一律 401 全局拦截回登录页；令牌错误呈现统一失败文案，不泄露实例/用户存在性；登录换 httpOnly SameSite=Strict Cookie；登出清 Cookie 回登录页。
- **浏览器不落业务数据（NFR-C3）**：无 localStorage/IndexedDB 业务缓存，业务数据仅内存态持有（仅允许主题偏好等非业务态）；浏览器永不见 LLM Key（API 出参只有 api_key_ref）。
- **断线诚实明示（NFR-C7）**：连接状态（连接中/在线/重连中）可见呈现，不静默失败；SSE 消费与 invoke 通道同构（NFR-C6 消费侧验证）。
- **16.3（FR-48）**：切换远程=本地引擎待机、不产生本地数据副本；切换是连接目标变更，永不触发数据合并/同步；重启桌面 app 记忆上次模式。

## Technical Decisions

- **单一 Vite 构建产物双宿主复用**：server 内嵌静态服务 dist/（与桌面 frontendDist 同一构建产物）+ SPA 回退路由（未知路径回 index.html）；运行时探测 `window.__TAURI_INTERNALS__` 选 HttpTransport；不做双构建配置。
- **能力门控**：desktop-only 能力（选工作目录、配对管理、本地数据导入导出等）经 transport.capabilities 驱动入口显隐（非运行时报错）；capabilities 构建期从共享注册表生成，禁手写。TitleBar/App 的窗口 API 与 Tauri 启动逻辑（单实例、窗口准备）按同一探测门控，浏览器下 TitleBar 退化为普通标题栏。
- **事件与流式**：useEngineEvent（15.5 交付的 hook）接入全部事件驱动视图，组件层零改动（hook 内部分支）；SSE 全客户端广播=桌面 emit 多窗口语义；同会话流式互斥跨标签生效（第二发送收 busy 消息，Ok 通道语义透传）；多端并发编辑令牌级 last-write-wins，无会话管理协议。
- **断线重连**：EventSource 原生自动重连，连接状态经 healthz 探测呈现；重连后由 transport 层重放只读 query command 白名单（15.5 冻结的协议），写入类一律不重放（防副作用重复）；UI 只订阅连接状态。
- **刷新恢复**：服务端为事实源——已产生消息已落库，刷新后重拉；进行中流式会话按服务端状态呈现已完成部分，不伪造进行态。
- **移动端**：iOS Safari 的 EventSource 已验证支持；SameSite=Strict Cookie 同源正常携带；100vh 地址栏抖动等已知问题需缓解；375px 级视口交付可用基线，断点方案归 UX 定稿。
- **16.3 状态机**：LOCAL/REMOTE 互斥双态；REMOTE_CONNECTING/REMOTE_ONLINE 远端不可达 → REMOTE_OFFLINE 指数退避重连（1s→30s 封顶+抖动）；LOCAL→REMOTE 守卫=本地引擎完整停机（sidecar 杀进程、调度器取消、连接池关闭）后才切传输；REMOTE 态桌面=浏览器等价物，无本地数据兜底；两侧数据零合并零同步（无任何数据迁移代码路径）。
- **验证方式**：视觉零回归=桌面浏览器与桌面版并排截图对比留档（唯一允许差异=宿主门控项）；vitest 覆盖 setup（含 env 锁定态）/登录/401 拦截/登出（mock transport）；流式/事件/断线重连各至少一条浏览器驱动 e2e（本地 server）。

## UX & Interaction Patterns

- 视觉零分叉：同一组件体系同一 Tailwind 产物渲染，禁止重做 UI。
- 能力门控可见性：desktop-only 入口在 web 端不出现（capabilities 驱动显隐，而非运行时报错）。
- TitleBar 窗口控制在浏览器退化为普通标题栏（Tauri 下渲染原样）。
- 响应式与浏览器兼容（含 iOS Safari）：本 Epic 交付可用基线，断点与移动交互细节由 UX 阶段确定。
- WEB UX 章节尚未产出；UX 定稿后的新约束以勘注补入（核心功能分档清单届时为准）。

## Cross-Story Dependencies

- 强顺序 16.1 → 16.2（入口与认证一体先行交付）；与 Epic 17 并行，同为 Epic 15 的后继。
- 直接消费 Epic 15 交付物：15.4 的 server/认证端点（/api/auth/status、/api/auth/login、/api/setup、401 语义）与 15.5 的 transport 抽象、useEngineEvent、只读补齐白名单、capabilities 生成。
- 16.2 的浏览器驱动 e2e 是 17.3 tests/e2e web 模式的前置能力（桌面模式零改动）。
- 16.3 可选后置：依赖 15.4/15.5 + 16.1–16.2 + 产品决策，不阻塞云端版首版验收。
