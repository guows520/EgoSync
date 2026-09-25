# Epic 16 Context: WEB 客户端（Web Client）

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

让任意现代浏览器（桌面/移动，含 iOS Safari）访问云端实例即可使用核心体验：流式对话、任务操作、建议确认/拒绝、仪表盘、简报复盘；刷新或断线重连后恢复状态、不丢已产生消息；与桌面版视觉零分叉（同一 React 组件体系、同一 Vite 构建产物双宿主复用），desktop-only 能力入口不出现。本 Epic 交付 FR-45 全部验收（含 2026-09-25 补入的「移动端专用形态」与「PWA 可安装」两条）与 FR-46 认证前端流程；FR-48 桌面远程模式由可选后置故事 16.3 交付，不阻塞云端版首版验收。16.4「移动端 WEB 形态与 PWA 增强」已于 2026-09-25 完成实现并通过 step-04 三轮评审（规格状态 done、sprint-status review）：移动端底部三 tab 形态 + PWA 三件套（manifest/图标/约 90 行外壳 SW）+ CSP 增列 manifest-src/worker-src；Desktop 桌面 e2e 仍受环境阻断（见 deferred-work D3），留待人工收口。

## Stories

- Story 16.1: WEB 入口、认证与首访流（已完成）
- Story 16.2: 实时事件、流式体验与浏览器适配（已完成）
- Story 16.3: 桌面客户端远程模式（FR-48，可选后置）（已完成）
- Story 16.4: 移动端 WEB 形态与 PWA 增强（2026-09-25 实现完成、评审通过，规格 spec-16-4 status=done；本 Epic 交付重点）

> 16.1–16.3 的实现细节以代码与各故事文件为准，本文件只保留 Epic 级目标、约束与架构决策。

## Requirements & Constraints

- **浏览器核心体验（FR-45）**：桌面/移动浏览器无需安装任何应用即可登录并完成核心操作；流式输出实时逐字渲染、停止与忙等会话语义与桌面一致；刷新/断线后从服务端事实源恢复，进行中流式会话按服务端状态呈现，不伪造进行态。
- **移动专用形态（FR-45 补条）**：<768px 视口（375～430 CSS px，覆盖 iPhone SE ～ 15 Pro Max）呈现底部 tab 导航替代侧栏、全屏对话、触控目标 ≥44×44px、刘海/底部安全区避让；≥768px 桌面布局零变化。
- **PWA 可安装（FR-45 补条）**：manifest / 图标 / 应用外壳离线缓存就位，可加到主屏获得类原生入口；业务数据不落浏览器（刷新仍从服务端重取）。
- **数据红线（NFR-C3）**：浏览器不落任何业务数据——无 localStorage/IndexedDB 业务缓存，仅允许主题偏好等非业务态；SW 外壳缓存不含业务数据；`/api/*` 与 SSE 直通零缓存；LLM Key 仅存服务端，任何 API 出参只有 api_key_ref。
- **桌面零回归（NFR-C4，硬边界）**：每故事收口 = `npm run test:all` + tests/e2e 全量（桌面 + web）零改动全绿；≥768px 布局逐像素不变；截图双份留档（桌面回归 + 移动新留档）。
- **连接诚实明示（NFR-C6/C7）**：SSE 消费与桌面同构；连接状态（连接中 / 在线 / 重连中）可见呈现，不静默失败；断线补齐只重放只读查询，写入类一律不重放。
- **视觉零分叉（UX-C1 为硬验收）**：WEB 与桌面并排对比，唯一允许差异 = 宿主门控项。
- **显式不做（防范围蔓延）**：PWA Web 推送（iOS 16.4+ 仅限加主屏支持，维持「WEB 端应用内通知」）；原生壳（Capacitor/Tauri Mobile）；布局体系重构（仅断点收敛，不改组件体系）。

## Technical Decisions

- **双宿主复用**：单一 Vite 构建产物，tauri frontendDist 与 server 内嵌静态是同一份 dist，不做双构建；运行时探测宿主选 TauriTransport 或 HttpTransport（fetch `POST /api/cmd/{cmd}` + EventSource `/api/events`）；事件经 useEngineEvent hook 统一接入，组件层零改动。
- **能力门控**：desktop-only 入口（选工作目录、配对管理等）经 capabilities 清单驱动显隐，而非运行时报错；清单与引擎注册表同源，禁止手写双份。
- **响应式实现模式（16.2 建立，16.4 沿用）**：Tailwind `md:` 桌面值 + `max-md:` 移动值双声明；768px 断点沿用 UX 既有断点体系，不新增断点；16.4 只做断点收敛，不改既有桌面测试口径（禁删测试改断言）。
- **安全区与视口**：`viewport-fit=cover` 与 dvh（100vh 抖动缓解）已由 16.2 落地，16.4 补全底部 tab / 输入区 / 模态的 `env(safe-area-inset-*)` 避让，既有缓解不回退。
- **PWA 零新依赖**：手写 manifest + Service Worker（约百行、可单测），不引 vite-plugin-pwa / workbox；图标从既有 SVG 生成（构建期脚本或静态 PNG，dev 决策）。
- **SW 缓存纪律**：仅预缓存应用外壳——hash 静态资产 cache-first、index.html stale-while-revalidate，并与 16.1 已建立的 index.html no-cache 启动协商对齐（防发版后白屏）。
- **CSP 契约**：以 'self' 为基线、零 http(s) 外链源（byte 对 byte 契约测试守门）；16.4 增量 = `manifest-src 'self'` + `worker-src 'self'`，契约测试同步收紧。
- **移动端任务拖拽降级**：触屏上拖拽排序须降级为「移动模式」排序入口或长按拖拽，禁止拖拽失效且无替代路径。

## UX & Interaction Patterns

> 以下为「云端版 WEB 移动形态」勘定（即 16.2 延期的 UX 定稿），只做小屏形态收敛与 PWA 交互，不新造视觉。

- **底部 tab 导航**：`管家 / 角色 / 设置`（初案，文案与顺序允许实现期微调；tab 数量/位置等结构变更须回报产品负责人）；主区域全屏。角色详情的对话 / 任务 / 记忆既有顶部 tab 小屏保持（可横向滚动）；仪表盘 = 管家视图内既有 tab，不新设入口。
- **五大核心面小屏形态**：管家对话全屏、输入区贴底 + 安全区避让、虚拟键盘弹出不遮发送按钮；管家摘要宽度 100%、padding 16px；ActionCard 全宽堆叠；任务四象限分组单列；仪表盘网格降列（沿用 16.2 塌缩基线）。
- **触控与密度**：触控目标 ≥44×44px（视觉更小也保点击区域）；密度双模式对齐 Android 伴侣（对话轻 / 仪表盘密），以既有桌面值为锚做小屏收敛，不新造密度值；既有动效（呼吸 / 色温）移动端不变，prefers-reduced-motion 已支持。
- **PWA 安装体验**：不加侵入式安装弹窗，依赖浏览器原生「添加到主屏幕」+ manifest 元数据（单用户自托管产品气质）；maskable 图标 192/512 + theme-color，standalone 启动即品牌色，无额外启动画面造景。
- **离线语义如实呈现**：外壳离线可用 = 界面可打开；业务数据仍需服务端重取，断线时连接状态条诚实明示，不为离线假装有数据；iOS Safari 普通浏览与 standalone 两模式安全区均生效。

## Cross-Story Dependencies

- **16.4 前置**：16.2 的响应式基线（375px 可用基线、dvh / viewport-fit 落地、`md:`/`max-md:` 测试口径）+ 16.1 的静态服务 / SPA 回退（SW 缓存与 index.html no-cache 协商的对齐面）；与 16.3 互不依赖；与 Epic 17 并行无冲突。
- **全 Epic 前置（Epic 15）**：15.4 的 server API 面 / 认证端点与 15.5 的 Transport 抽象、useEngineEvent、capabilities 同源生成、只读重连补齐白名单。
- **下游**：16.4 建议作为 Epic 16 → done 的前置（FR-45「移动浏览器」验收需逐条过）并排在 Epic 16 retrospective 之前；17.3 的 CI web-e2e-smoke 可追加一条移动视口冒烟（可选，归 16.4 交付物）。
