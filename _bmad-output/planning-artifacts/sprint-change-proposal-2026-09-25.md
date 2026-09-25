# 冲刺变更提案：WEB 客户端移动端适配与 PWA 增强

- **日期**：2026-09-25
- **提出人**：Ubuntu（boss / 产品负责人）
- **变更类型**：范围补全——结清 Story 16.2 显式延期的「UX 定稿级移动形态」欠账与 UX-C4 开放项，追加 PWA 可安装增强
- **变更范围定级**：**Moderate（中等）**——Epic 16 内新增一个故事 + PRD / 架构 / UX / e2e 四处工件更新，需 PO/DEV 协同；不触及桌面版（NFR-C4 零回归硬边界不变）
- **状态**：**已批准**（boss 于 2026-09-25 审批通过：7 组提案全数 Approve；工件更新已落地，Story 16.4 路由 DEV 实现）

---

## 1. 问题陈述（Issue Summary）

**触发**：产品负责人 2026-09-25 于 Correct Course 会话提出「web 版本适配移动浏览器」，并确认范围为**完整适配 + PWA 增强**（底部导航级移动形态 + 可加主屏 / 离线外壳）。

**问题定性**（checklist 1.2）：**利益相关方新需求 + 既定延期的欠账结清（混合型）**。

- 云端版立项（决策 #19）后，FR-45 已将「移动浏览器（含 iOS Safari）」列为正式客户端，但移动形态从未定稿；
- Story 16.2（已交付）显式声明：「不做 UX 定稿级移动形态（断点方案归 UX 勘注定稿，本故事只交付可用基线）」；
- UX-C4 至今挂着「断点与移动交互细节由 UX 阶段确定」；
- UX 规范 Responsive Strategy 表中「手机 <768px」标注「V2 考虑」——那是云端版之前的规划，现已过时。

**证据**（checklist 1.3）：

| 证据 | 位置 | 内容 |
|---|---|---|
| 交付边界声明 | `16-2-realtime-streaming-and-browser-compat.md` | 375px 只做「可用基线」：弹窗/面板收缩、双栏塌缩、仪表盘网格降列；UX 定稿级形态明示不做 |
| 未定稿登记 | `epic-16-context.md` | 「断点与移动交互细节由 UX 阶段定稿」「WEB UX 专属章节尚未定稿」 |
| 开放设计需求 | `epics.md` UX-C4 | 「桌面/移动浏览器（含 iOS Safari）可用；断点与移动交互细节由 UX 阶段确定」 |
| 代码现状 | `egosync-app/src` | 全仓仅 1 条媒体查询（prefers-reduced-motion）；Sidebar 固定 `w-16` 左栏；App 根 `flex flex-col h-screen` 桌面壳；375px e2e 仅 1 条溢出走查例（web-streaming.spec.ts） |
| 无 PWA 基建 | `egosync-app/` | 无 manifest、无 Service Worker、无图标集（`public/` 仅 favicon.ico/svg）；零 PWA 依赖（无 vite-plugin-pwa/workbox） |
| 预留槽位 | `architecture.md` Implementation Sequence | 「WEB 认证/首访/响应式（16.1-**16.4**）」与「**16.4+** 仅限加主屏 PWA」两处引用——16.4 编号早已预留，本次正式落位 |

**结论**：手机浏览器当前「能用但不好用」是既定范围的正常状态，非缺陷回退；本次是把云端版承诺给移动浏览器的完整体验正式落版。

## 2. 影响分析（Impact Analysis）

### 2.1 史诗影响

| 史诗 | 影响 | 说明 |
|---|---|---|
| Epic 1-11（桌面已交付） | 无 | 同一 React 组件体系复用；桌面布局 ≥768px 零变化（NFR-C4 硬边界） |
| Epic 12-14（手机伴侣） | 无 | Android 原生客户端自有移动形态，不受 WEB 断点影响；其设计语言（密度双模式 / 44px 触控 / reduced-motion）作为 WEB 移动形态的对齐参考 |
| **Epic 16（in-progress）** | **范围扩充** | 新增 Story 16.4「移动端 WEB 形态与 PWA」；16.1-16.3 已 done 不受影响；架构 Implementation Sequence 原已预留 16.4 槽位 |
| Epic 15 | 无 | 引擎/服务器契约零改动——纯前端 + 静态资源 + SW 范畴 |
| Epic 17（in-progress，17.3 review 中） | 微 | 部署文档可补 PWA / 移动访问说明；CI web-e2e-smoke 可加 1 条移动视口冒烟（可选，归 16.4 交付物） |
| Epic 14（backlog） | 无 | 概念相邻（离线）但域不同：14 = 伴侣端只读降级；16.4 的 SW 外壳缓存不含任何业务数据 |

### 2.2 工件影响

| 工件 | 需要的更新 | 量级 |
|---|---|---|
| `epics.md` | Epic 16 故事列表 + 完整 Story 16.4 定义 + FR / UX 覆盖表 + 依赖图 | 中 |
| `prd-egosync.md` | 更新注记、FR-45 验收标准补移动形态 / 可安装条目、§6.2 Non-Goals 登记（PWA 推送不做） | 小-中 |
| `architecture.md` | 云端增量章节追加「移动适配与 PWA」决策块（断点、底部导航、SW 缓存纪律、CSP 增量、iOS 约束） | 中 |
| `ux-design-specification.md` | 新增「云端版 WEB 移动形态」章节（即 16.2 延期的 UX 勘定）；Responsive Strategy 表加修订注记 | 中 |
| `sprint-status.yaml` | Epic 16 下新增 16-4 backlog 条目 | 小 |
| `.decision-log.md` | 新增 #20 裁决 | 小 |
| `tests/e2e/web-specs/` | 新增移动视口旅程 spec + manifest / SW 断言 | 中 |
| `epic-16-context.md` | 规划变更后重编译（compile-epic-context） | 小 |

### 2.3 技术影响（概要，细节归 Story 16.4）

1. **布局**：`<768px` 侧栏（`w-16` 固定）退场 → 底部 tab 导航；主区全屏；≥768px 维持桌面布局逐像素不变（Tailwind 响应式类：`md:` 桌面值 + `max-md:` 移动值；16.2 已建立 `md:w-[35%]` 守护先例）。
2. **触屏与安全区**：触控目标 ≥44×44px；viewport safe-area 已由 16.2 部分落地（`viewport-fit=cover` + dvh），本次补全底部 tab / 输入区 / 模态避让。
3. **PWA**：manifest（standalone、192/512 maskable 图标、theme-color）+ apple-touch-icon + apple-mobile-web-app-capable；Service Worker 仅预缓存应用外壳（hash 静态资产 cache-first、index.html stale-while-revalidate、API/SSE 直通不缓存）——**业务数据零缓存，NFR-C3「浏览器不落业务数据」红线不变**（刷新仍从服务端重取）。
4. **CSP**：新增 `manifest-src 'self'` 与 `worker-src 'self'`；契约测试同步（既有「全 policy 不得含任何 http(s) 外链源」断言保持）。
5. **零新依赖**：手写 manifest + Service Worker（约百行、可测试），不引 vite-plugin-pwa / workbox（boring stack 纪律 + CSP byte 级契约可控）；图标由实现期从既有 SVG 生成（构建期脚本或提交静态 PNG，归 dev 决策）。
6. **推送明确不做**：iOS Safari 普通浏览无 Notification API、16.4+ 仅加主屏 PWA 支持 Web Push——维持架构既有裁决「WEB 端应用内通知」，PWA 推送登记为显式 Non-Goal / 后置。
7. **风险**：桌面视觉回归（收口 = 对等截图比对 + 桌面 e2e 零改动全绿）；iOS Safari SW / standalone 怪癖（e2e 以 Chrome 覆盖 + 真机走查清单）；SW 缓存导致发版后白屏（index.html no-cache 纪律 16.1 已建立，SW 策略与之对齐）。

### 2.4 成本 / 工期 / 风险评估

- **工作量**：中——单故事（约 3-5 天 agent 工作量）：布局断点改造为主，PWA 基建为确定性小工程，UX 章节约 0.5 天。
- **风险**：中低——改造面集中在前端布局层，有对等测试 / 截图走查 / 既有响应式测试口径三重兜底。
- **时间线**：不阻塞 Epic 17 收尾（17.3 在 review）；建议排在 Epic 16 retrospective 与云端版首版对外验收之前，作为 Epic 16 的最后一块。

## 3. 推荐路径（Recommended Approach）

| 选项 | 评估 | 结论 |
|---|---|---|
| 1. 直接调整（Epic 16 内新增 16.4） | 架构已预留槽位；改造集中前端；桌面零回归有兜底 | **可行（推荐）** |
| 2. 回退已完成故事 | 无可回退物——16.2 基线是地基非错误 | 不可行 |
| 3. MVP 范围复议 | 无需减范围——云端版 V1 范围不变，16.4 是既有承诺（FR-45 移动浏览器）的落位 | 不适用 |

**选定：选项 1（直接调整）**——在 Epic 16 新增 Story 16.4，一次性结清 UX-C4 与 16.2 延期项，并以 PWA 增强兑现「完整适配」。

**理由**：16.4 编号槽位早已预留；改造集中在既有组件体系的断点收敛（非重构）；与 Epic 17 并行无冲突；移动浏览器是云端版（无桌面依赖）的核心使用场景——用户拿着手机打开自家 VPS 的地址，就是云端版最典型的日常。

## 4. 详细变更提案（Detailed Change Proposals）

> 批量模式：以下全部提案一次性呈报，请逐条 **Approve / Edit / Skip**。

### 4.1 `epics.md` — Epic 16 扩充

**提案 1a：Epic 16 故事列表新增 16.4**（Story 列表末尾追加，既有三条不动）

```text
OLD:
- **16.3** `[可选后置]` 桌面客户端远程模式（FR-48）——LOCAL/REMOTE 互斥状态机 + 切换 UI …（本行不变）

NEW（追加一行）:
- **16.4** 移动端 WEB 形态与 PWA——<768px 侧栏退场改底部 tab 导航 + 全屏对话 + 44px 触控目标 + 安全区补全 + manifest/maskable 图标/Service Worker 应用外壳缓存（业务数据零缓存，NFR-C3 不变）+ CSP worker-src/manifest-src 增量 + 移动视口 e2e；结清 16.2 延期的「UX 定稿级移动形态」与 UX-C4 断点细节欠账
```

**提案 1b：新增 Story 16.4 完整定义**（置于 Epic 16 章节内 16.3 之后，全文见下）

**提案 1c：FR Coverage Map / UX 覆盖 / 依赖图三处联动**

| 位置 | OLD | NEW |
|---|---|---|
| FR-45 覆盖行 | `E16（16.1–16.2）` | `E16（16.1–16.4）` |
| UX 覆盖行 | `16.1（…）/ 16.2（流式/重连/响应式）` | `… + 16.4（断点与移动交互细节定稿，UX-C4 欠账结清）` |
| 依赖图 | `→ {16.1 → 16.2 ∥ 17.1 → 17.2 → 17.3}` | 同左 + `→ 16.4（依赖 16.2；与 16.3 互不依赖）` |

**提案 1b 全文：**

```text
### Story 16.4: 移动端 WEB 形态与 PWA 增强

As a 云端版用户（以手机浏览器访问）,
I want 与小屏触屏相称的 WEB 形态（底部 tab 导航、全屏对话、44px 触控目标、安全区避让）与 PWA 可安装能力（manifest、maskable 图标、应用外壳离线缓存）,
So that 手机浏览器是正式可用的完整客户端而非「桌面版压缩显示」，并可加到主屏获得类原生入口；同时结清 Story 16.2 延期的 UX 定稿级移动形态与 UX-C4 断点细节欠账。

**FRs covered:** FR-45（移动形态 + 可安装条目）；UX-C4（断点与移动交互细节定稿——欠账结清）
**NFRs covered:** NFR-C3（外壳缓存不含业务数据——边界重述）；NFR-C4（桌面 ≥768px 零回归守护）
**Additional reqs covered:** 8（前端消费侧——SW 注册）
**前置依赖:** 16.2（响应式基线）+ 16.1（静态服务/SPA 回退）；与 16.3 互不依赖；与 Epic 17 并行

**Acceptance Criteria:**

**Given** <768px 视口（375 ～ 430 CSS px，覆盖 iPhone SE ～ 15 Pro Max）
**When** 打开 WEB 端
**Then** 左侧 56px 侧栏退场，改渲染底部 tab 导航（管家/角色/设置——UX 勘定初案，UX 阶段可微调）；主区域全屏；≥768px 桌面布局逐像素不变（`md:` 桌面值 + `max-md:` 移动值模式，守护 16.2 建立的既有桌面测试口径）

**Given** 手机视口五大核心面（管家对话 / 角色视图 / 任务 / 仪表盘 / 简报复盘）
**When** 逐面走查
**Then** 无横向溢出；触控目标 ≥44×44px；输入框聚焦时虚拟键盘不遮挡发送按钮（视口压缩妥善处理）

**Given** iOS Safari（含刘海/底部横条机型）
**When** 普通浏览与加到主屏 standalone 两种模式
**Then** 底部 tab / 输入区 / 模态以 env(safe-area-inset-*) 避让；100vh 抖动缓解保持（dvh 优先+回退——16.2 既有落地不回退）

**Given** 视觉零分叉（UX-C1）
**When** 桌面版并排对比 ≥768px
**Then** 零差异（移动态差异为有意变更，截图双份留档：桌面回归 + 移动新留档）

**Given** PWA 基建
**When** 部署实例经浏览器访问
**Then** manifest.webmanifest 就位（display=standalone、theme-color、192/512 maskable 图标、start_url=/）；apple-touch-icon + apple-mobile-web-app-capable meta 就位；浏览器「添加到主屏幕」可用

**Given** Service Worker 注册后
**When** 二次访问
**Then** 应用外壳离线可用（hash 静态资产 cache-first、index.html stale-while-revalidate）；/api/* 与 SSE 直通零缓存；业务数据零落盘（无 localStorage/IndexedDB 业务缓存——NFR-C3 红线，刷新=服务端重取语义不变）

**Given** CSP（契约测试守门）
**When** 策略更新
**Then** manifest-src 'self' 与 worker-src 'self' 入列；既有「零 http(s) 外链源」断言保持全绿

**Given** e2e（web 模式）
**When** 新增 web-mobile.spec.ts（375×812 DevTools 模拟）
**Then** 登录→流式对话→任务操作→仪表盘→通知全旅程通过；manifest 字段断言 + SW 注册断言通过；桌面模式 e2e 零改动全绿

**Given** 桌面回归收口（硬边界）
**When** 执行 npm run test:all + tests/e2e 全量（桌面 + web）
**Then** 全绿——跳过任何一项即本故事未完成（显式失败原则）

**不做（显式登记）**：PWA Web 推送（iOS 16.4+ 仅限加主屏——PRD Non-Goal 后置）；原生壳（Capacitor/Tauri Mobile）；布局体系重构（仅断点收敛，不改组件体系）。
```

### 4.2 `prd-egosync.md`

**提案 2a：文件头更新注记追加**（只追加，不回改既有行）

```text
OLD:
**2026-09-17更新**：新增§4.15 云端托管版（自托管）。……V1 MVP 范围不变。（本行不变）

NEW（追加一行）:
**2026-09-25更新**：§4.15 FR-45 验收标准补「移动端形态」与「PWA 可安装」两条（结清 Story 16.2 延期项与 UX-C4 欠账）；§6.2 Non-Goals 登记 PWA Web 推送为显式后置；云端版 V1 范围不变。
```

**提案 2b：FR-45 验收标准补两条**（现有四条之后追加）

```text
NEW:
- 移动浏览器（<768px 视口，含 iOS Safari）呈现移动专用形态：底部 tab 导航替代侧栏、全屏对话、触控目标 ≥44px、刘海/底部安全区避让；≥768px 桌面布局零变化
- 云端实例可安装为主屏应用（PWA）：manifest / 图标 / 应用外壳离线缓存就位；业务数据不落浏览器（刷新仍从服务端重取）
```

**提案 2c：§6.2 Non-Goals 追加一条**

```text
NEW:
- **PWA Web 推送（Service Worker 推送）** — iOS Safari 16.4+ 仅限加主屏 PWA 支持，与架构「WEB 端应用内通知」裁决一致；显式后置 `[NON-GOAL for MVP]`
```

### 4.3 `architecture.md`

**提案 3：云端增量章节追加「移动适配与 PWA（Story 16.4 落地）」决策块**（追加式，只加不回改）

```text
NEW（决策块要点）:
- 断点体系沿用 UX 规范：--bp-tablet: 768px / --bp-mobile: 768px 以下；<768px = 侧栏退场 + 底部 tab；≥768px 逐像素保持桌面
- 底部 tab 初始方案：管家 / 角色 / 设置（与 UX 规范手机适配要点一致；仪表盘 = 管家视图内既有 tab；角色详情内对话/任务/记忆保持既有 tab 结构）——UX 阶段可微调，以视觉零分叉为界
- SW 缓存纪律：仅外壳（hash 资产 cache-first、index.html SWR + 16.1 启动 no-cache 协商）；/api/* 与 SSE 直通零缓存；业务数据零落盘（NFR-C3 重述）
- CSP 增量：manifest-src 'self'、worker-src 'self'；契约测试同步
- manifest 口径：display=standalone、theme-color 取设计 token、192/512 maskable、start_url=/
- iOS 约束登记：普通浏览无推送（维持「WEB 端应用内通知」裁决）；standalone 100dvh 已由 16.2 落地
- 测试：e2e 移动视口旅程（Chrome DevTools 375×812 模拟）+ manifest / SW 断言；桌面 e2e 零改动
```

### 4.4 `ux-design-specification.md`

**提案 4a：Responsive Strategy 表追加修订注记**（追加式，不回改原表）

```text
NEW（表下注记）:
> 2026-09-25 修订注记：云端版（PRD §4.15）立项后，移动浏览器为 WEB 客户端第一类客户端（FR-45），原「手机 <768px = V2 考虑」升级为正式范围——落位于文末新增章节「云端版 WEB 移动形态」。
```

**提案 4b：新增「云端版 WEB 移动形态」章节**（即 16.2 延期的 UX 勘定，章节要点）

```text
- 断点：768px；<768px 底部 tab（管家/角色/设置）+ 全屏对话 + 管家摘要全宽 padding 16px + ActionCard 全宽堆叠（沿用规范既有手机适配要点）
- 触控目标 ≥44×44px（与 Android 伴侣设计语言一致）
- 密度双模式参考 companion-android（对话轻 / 仪表盘密）——WEB 端以既有桌面值为锚做小屏收敛，不新造视觉
- 动效：呼吸 / 色温 300ms 等既有动效移动端不变；reduced-motion 已支持（web 平台等价物 prefers-reduced-motion）
- 安全区：viewport-fit=cover 已开（16.2）；底部 tab 与输入区 env(safe-area-inset-*) 避让
- PWA 安装：不加侵入式安装弹窗；依赖浏览器原生「添加到主屏幕」+ manifest 元数据（单用户自托管产品气质）
- 图标与启动画面：maskable 图标 + theme-color（standalone 启动即品牌色）
```

### 4.5 `sprint-status.yaml`

**提案 5：Epic 16 块新增条目**（2026-09-25 Correct Course 批准后落位）

```yaml
OLD:
   # Epic 16: WEB 客户端 (Web Client)
   epic-16: in-progress
   16-1-web-entry-auth-and-first-visit-flow: done
   16-2-realtime-streaming-and-browser-compat: done
   16-3-desktop-remote-mode-fr48-optional: done

NEW:
   同上 +
   16-4-mobile-web-form-and-pwa: backlog
```

### 4.6 `.decision-log.md`

**提案 6：新增 #20**

```text
20. **WEB 移动端形态与 PWA 定案**（2026-09-25，Correct Course 裁决）：移动浏览器（含 iOS Safari）为云端版第一类客户端（FR-45 既有语义），结清 16.2 延期项与 UX-C4 开放项——<768px 底部 tab + 全屏对话 + 44px 触控 + 安全区避让；PWA = manifest + maskable 图标 + SW 外壳缓存（业务数据零缓存，NFR-C3 不变）；手写 SW 零新依赖；PWA Web 推送显式后置（iOS 16.4+ 仅限加主屏）；桌面 ≥768px 布局零变化（NFR-C4）。
```

### 4.7 测试与 CI

**提案 7：e2e / 单测 / CI 三处**

```text
- 新增 tests/e2e/web-specs/web-mobile.spec.ts：375×812 视口核心旅程（登录 → 流式对话 → 任务操作 → 仪表盘 → 通知）+ manifest 字段断言 + SW 注册断言
- server-ci.yml web-e2e-smoke job 增加 1 条移动视口冒烟（可选，与 17.3 既有链路同模式）
- vitest：新增/更新布局断点断言（Sidebar 移动态、底部 tab 渲染、≥768px 桌面值守护——沿用 16.2 md:/max-md: 守护口径）
```

## 5. 实施交接（Implementation Handoff）

### 5.1 范围定级与路由

- **定级：Moderate（中等）**——单 Epic 内新增故事 + 多工件更新，需 backlog 重组与 PO/DEV 协同；无需 PM/架构师重新规划（架构槽位与决策方向本次已定，UX 细节允许实现期在勘定章节内微调）。
- **路由**：
  - **PO（产品负责人 / boss）**：批准本提案 → sprint-status.yaml 落位 16-4 → epic-16-context.md 重编译；
  - **DEV（bmad-agent-dev / bmad-build）**：实现 Story 16.4（布局断点 + PWA 基建 + e2e + 桌面零回归收口）；
  - **UX 微调裁量**：底部 tab 文案/顺序等细节允许 dev 在勘定章节框架内定稿，涉及结构变更（如 tab 数量/位置）须回报 boss。

### 5.1b 工件更新执行记录（2026-09-25 批准后当日完成）

| 提案 | 工件 | 状态 |
|---|---|---|
| 1a-1c | `epics.md`（Epic 16 故事列表 / Story 16.4 全文 / FR-45 与 UX 覆盖行 / 依赖图） | 已落地 |
| 2a-2c | `prd-egosync.md`（更新注记 / FR-45 补 2 条 / Non-Goal 登记 PWA 推送） | 已落地 |
| 3 | `architecture.md`（⑩移动端适配与 PWA 决策块） | 已落地 |
| 4a-4b | `ux-design-specification.md`（Responsive Strategy 修订注记 /「云端版 WEB 移动形态」新章节） | 已落地 |
| 5 | `sprint-status.yaml`（`16-4-mobile-web-form-and-pwa: backlog`） | 已落地 |
| 6 | `.decision-log.md`（#20 裁决） | 已落地 |
| 7 | `tests/e2e` + CI | **移交 DEV**（Story 16.4 实现期交付，见 §5.2） |

遗留：`epic-16-context.md` 重编译——随 bmad-build 创建 16.4 故事文件时按 compile-epic-context 流程执行。

### 5.2 执行顺序

```text
16.4 与 Epic 17 收尾（17.3 review）并行无冲突
   → 排在 Epic 16 retrospective 之前
   → 建议作为 Epic 16 → done 的前置（FR-45「移动浏览器」验收需逐条过）
```

### 5.3 成功标准

1. 375×812 视口核心旅程 e2e 全绿（`web-mobile.spec.ts`）；
2. 桌面 `npm run test:all` + tests/e2e 全量（桌面 + web）零改动全绿；
3. PWA 三件套（manifest / maskable 图标 / Service Worker）就位，且「业务数据零缓存」断言通过；
4. 截图对等留档：桌面无变化 + 移动态新留档；
5. CSP 契约测试全绿（零 http(s) 外链源红线保持）。

### 5.4 显式不做（防范围蔓延）

- PWA Web 推送（已登记 Non-Goal）；
- 原生壳（Capacitor / Tauri Mobile）；
- 布局体系重构（仅断点收敛，不改组件体系与视觉零分叉承诺）；
- Epic 12-14 伴侣代码任何改动。

---

## 附录：变更分析清单结果（checklist.md）

| # | 项 | 状态 | 结论 |
|---|---|---|---|
| 1.1 | 触发故事识别 | [x] | Story 16.2——显式延期「UX 定稿级移动形态」；非失败，是范围裁剪 |
| 1.2 | 问题精确分类 | [x] | 利益相关方新需求 + 既定延期欠账结清（混合型） |
| 1.3 | 证据收集 | [x] | 交付边界声明 / 未定稿登记 / UX-C4 开放项 / 代码现状 / PWA 基建缺失 / 架构 16.4 预留槽位，见 §1 |
| 2.1 | 当前史诗可完成性 | [x] | Epic 16 可按原计划完成 + 16.4 收口 |
| 2.2 | 史诗级变更需求 | [!] | **需动作**：Epic 16 新增 Story 16.4（提案 1a-1c） |
| 2.3 | 未来史诗影响排查 | [x] | Epic 14/17/12-14 排查完毕，仅 Epic 17 文档/CI 微关联（提案 7） |
| 2.4 | 失效/新增史诗 | [N/A] | 无史诗失效；不新开史诗（16.4 归一即可） |
| 2.5 | 顺序/优先级调整 | [x] | 16.4 置于 16.2 之后、Epic 16 retrospective 之前；不阻塞 17.x |
| 3.1 | PRD 冲突 | [!] | **需动作**：FR-45 验收补 2 条 + Non-Goal 登记 + 更新注记（提案 2a-2c） |
| 3.2 | 架构冲突 | [!] | **需动作**：云端增量章节追加「移动适配与 PWA」决策块（提案 3） |
| 3.3 | UX 冲突 | [!] | **需动作**：新增「云端版 WEB 移动形态」章节 + Responsive Strategy 注记（提案 4a-4b） |
| 3.4 | 其他工件 | [!] | **需动作**：sprint-status / 决策日志 / e2e / epic-context（提案 5-7） |
| 4.1 | 选项 1 直接调整 | [x] | **可行（选定）**——Epic 16 内新增 16.4 |
| 4.2 | 选项 2 回退 | [x] | 不可行——无可回退物 |
| 4.3 | 选项 3 MVP 复议 | [N/A] | 无需减范围 |
| 4.4 | 推荐路径选定 | [x] | 选项 1，理由见 §3 |
| 5.1-5.4 | 提案四要素 | [x] | 见 §1-§4 |
| 5.5 | 交接计划 | [x] | 见 §5 |
| 6.1-6.2 | 完整性与准确性 | [x] | 本稿 |
| 6.3 | 显式批准 | [ ] | **待 boss 批准**（见下） |
| 6.4 | sprint-status 更新 | [ ] | 批准后执行（提案 5） |
| 6.5 | 下一步确认 | [x] | 见 §5.2-5.3 |

---

## 批准区块

- [x] **批准**：按上述 7 组提案执行（工件更新 + sprint-status 落位 + 路由 DEV 实现 16.4）
- [ ] **修改后批准**：请注明调整点
- [ ] **不批准**：请注明原因

> 批准记录：2026-09-25 / boss（7 组提案全数 Approve，无调整点）

