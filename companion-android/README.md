# EgoSync 伴侣 · Android 高保真前端原型

> **这是什么**：EgoSync 手机伴侣 App（PRD §4.14，FR-40/41/43）的 Compose 高保真原型。
> 纯前端 + mock 数据——无网络、无加密、无 NSD 发现、无系统推送；
> 连接能力只有接口 + Fake 实现。桌面端是唯一事实源，本原型仅验证移动端的
> 信息架构、降级态体验与"老管家"美学。
>
> 权威依据：`_bmad-output/planning-artifacts/architecture.md` 手机伴侣增量章节、
> `_bmad-output/planning-artifacts/prd-egosync.md` §4.14。

## 构建

```bash
# JDK 17 + Android SDK（platform-37 / build-tools 36）
./gradlew :app:assembleDebug          # 产出 app/build/outputs/apk/debug/app-debug.apk
./gradlew :app:testDebugUnitTest      # 单元测试
```

- 单模块 `:app`；Gradle KTS + version catalog（`gradle/libs.versions.toml`）
- minSdk 26 / compileSdk & targetSdk 37；AGP 9.3.2（内置 Kotlin 编译）+ Kotlin 2.4.10 + Compose BOM 2026.08.00
- 依赖白名单：Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines（+ lifecycle-viewmodel-compose、activity-compose 等 Compose 编译期必需件）
- **无** Room / Hilt / 全局状态框架 / OkHttp（架构反模式清单生效）

## 页面地图

```text
冷启动
 └─ 未配对 → 配对流（全屏，无底栏）
     ① 欢迎说明（桌面唯一事实源 / 端到端加密 / 诚实降级）
     ② 扫码取景模拟（静息取景框 + 扫掠线，点「模拟扫码成功」推进）
     ③ 连接中动画（发现设备 → 交换密钥 → 验证身份 三阶段高亮）
     ④ 配对成功 → 进入主界面（配对关系持久化，再次启动直达）
 └─ 已配对 → 主界面
     ├─ 💬 对话（Tab 1）
     │    ├─ 会话头：「新对话」按钮 + 「历史对话」下拉（标题+相对时间+删除，移植桌面 ChatHeader）
     │    ├─ 消息流：用户/管家气泡、思考中态、流式打字机（逐字浮现+光标）
     │    ├─ 输入框（降级态禁用并说明原因）
     │    └─ 建议 ActionCard：确认/拒绝按钮态（确认→✓ 已转交管家执行）
     ├─ 📋 任务（Tab 2）
     │    └─ 四象限分组：Q1 红/Q2 蓝/Q3 琥珀/Q4 灰；大石头🪨标记；勾选完成（划线）
     ├─ 📊 仪表盘（Tab 3）
     │    ├─ 管家概览卡（平均能量/待办 + 三个二级页入口 + 未读角标）
     │    └─ 角色卡横向滑动：emoji 图标、能量条呼吸动效（全 App 唯一装饰动效）、
     │        任务/记忆/会话/待办统计四宫格
     ├─ 👤 我的（Tab 4）
     │    ├─ 外观：深色默认/浅色切换（持久化）
     │    ├─ 通知：whisper/tap/knock 三级开关（仅应用内语义）
     │    ├─ 配对设备卡（当前连接状态圆点）+ 解除配对（确认对话框）
     │    └─ 隐藏入口：连点「版本号」7 次 → 状态模拟（Debug）
     └─ 二级页（push 路由，fade 转场）
          ├─ 📄 晨间简报：管家问候 + 分节文本 + 行动点卡片（确认/稍后）
          ├─ 📈 周复盘：正向叙事成绩单 + 能量趋势 Canvas 柱状图（自绘）+ 大石头推进
          └─ 🔔 通知中心：whisper/tap/knock 三级分组 + 未读圆点 + 敲门级确认/拒绝

全局组件
 └─ 降级态遮罩（offline/degraded 时覆盖主界面）：
     灰色蒙层（拦截交互=引擎功能禁用）+ 数据截止时间标注 + 说明文案
     + 底部速记输入条（唯一开放入口；恢复连接后自动提交管家，Snackbar 提示）
```

## 与桌面端的设计语言对照表

母本：`egosync-app/`（`tailwind.config.js` + `src/index.css` CSS 变量 + 组件实际用色）。
取用规则：✅ 色彩 token / 信息层级 / 动效节奏 / 组件行为语义；❌ 布局骨架与桌面交互范式（Sidebar/hover/宽屏双栏）——一切以移动端 M3 组件重新承载。

### M3 ColorScheme ← 桌面 token

| M3 角色（深色/浅色） | 桌面 token 来源 | 取值 |
|---|---|---|
| `background` | `--bg-base` | `#0F1117` / `#F8F9FA` |
| `surface` | `--bg-surface` | `#1A1B2E` / `#FFFFFF` |
| `surfaceVariant` | `--bg-elevated` | `#252638` / `#FAFAFA` |
| `onBackground` / `onSurface` | `--text-primary` | `#E8E8ED` / `#1A1A2E` |
| `onSurfaceVariant` | `--text-secondary` | `#9CA3AF` / `#6B7280` |
| `outline` | `--border-default` | `#374151` / `#E5E7EB` |
| `primary` | 管家 `--role-accent`（`BUTLER_ACCENT`）+ dark 按钮 `bg-indigo-600` | `#6366F1` / `#4F46E5` |
| `secondary` | 角色色温·暖（`roleIcons.ts` 琥珀） | `#F59E0B` |
| `tertiary` | `--energy-high` | `#10B981` |
| `error` | `--color-error` | `#EF4444` |

### 语义色与组件行为 ← 桌面组件

| 移动端用法 | 桌面母本 | 说明 |
|---|---|---|
| 能量色谱 ≥70/40~69/<40 = 翠绿/琥珀/**红** | `DashboardTab.tsx` emerald/amber/**red**-500、`RoleSidebarIcon.tsx` | 组件实际用红；CSS `--energy-low` 灰 token 桌面从未引用，弃 |
| 呼吸动效：opacity 0.6↔1.0、3s ease-in-out | `index.css` `.breathe` + `--duration-breath: 3s` | 全 App 唯一装饰动效 |
| 圆角 6/10/12/24dp（Shapes） | `--radius-button/card/dialog/input` | 输入框 24dp 胶囊形态 |
| 动效时长 200/250/300ms | `--duration-fast/normal/color` | 页面转场取 220/180ms 近似档 |
| 用户气泡 primary 实色白字 / 管家气泡 surface+outline 边框 | `ChatBubble.tsx` `bg-indigo-600`+白 / 白底+border | 深色模式主交互色为 indigo 实色，非 M3 亮 tonal 惯例 |
| 思考态三弹跳点（160ms 交错、1.4s） | `ChatBubble.tsx` `BounceDots` | 8dp 圆点 |
| ActionCard 确认=indigo 实色钮；确认态 indigo 勾圆徽+边框、拒绝态灰 X 圆徽 | `ActionCard.tsx` | `rounded-[10px]` 卡片 |
| 大石头=琥珀描边文字徽章 | `TaskOverviewTab.tsx:114` | 非 emoji |
| 通知三级：whisper 灰 / tap 蓝 / knock 红 + 行内徽章 | `NotificationPanel.tsx` `levelConfig` | 徽章制；knock 的 `animate-pulse` 不取（动效白名单） |
| 角色图标=实色容器（46dp 圆角方块）+白图标 | `RoleHeader.tsx` / `DashboardTab.tsx` | accent 实色，非 tint |
| 能量条 6dp 高、轨道灰、色随分档 | `RoleHeader.tsx` h-[6px] / `DashboardTab.tsx` h-1.5 | |
| 角色域色温 | `roleIcons.ts` 8 色板 + `App.tsx` `--role-accent` | 移动端按域预映射：工作=靛蓝#4F46E5（冷）/家庭=琥珀#F59E0B（暖）+6% tint；接真实数据后改角色自带 color |
| 配对/引导分步节奏 | `OnboardingView.tsx` step 1→5 | 步进+进度指示 |
| 信息密度双模式（`InfoDensity` token，`ui/theme/Density.kt`）：对话流轻量 / 仪表盘密集，屏幕入口声明模式取值 | `ChatStream.tsx` space-y-3、`ChatBubble.tsx` px-5 py-3；`DashboardTab.tsx` grid gap-2.5 | PRD §4.14「两种模式自然切换」，非运行时用户开关 |
| 动效白名单：呼吸 3s + 思考点 1.4s + 转场 220/180ms + 色温 300ms 单次过渡（`ui/theme/Motion.kt`）；系统「移除动画」开启时全部静态（呼吸定格 alpha 0.6） | `index.css` `--duration-breath: 3s` / `BounceDots` / `prefers-reduced-motion` 全局降 0.01ms | Android 无该媒体查询，以 ANIMATOR_DURATION_SCALE=0 为平台等价物 |
| 色温过渡：域 accent 变化 300ms 渐变（角色卡图标底/徽章 tint/页指示点） | `App.tsx` `--role-accent` 切换 + `--duration-color: 300ms` 过渡 | 指示点取当前页角色域 accent，为「微妙色温变化」的移动落点 |

## Mock / Debug 开关使用说明

### 状态模拟（核心 debug 能力）

1. 进入 **我的** Tab → 滚到底部「关于」卡片
2. **连点「版本号」7 次**（Android 开发者选项惯例）→ 提示"状态模拟已开启"
3. 「关于」上方出现 **状态模拟（Debug）** 分组，六档可选：

| 档位 | 效果 |
|------|------|
| 局域网直连 direct | 全部功能可用 |
| 中继转发 relay | 全部功能可用 |
| 连接中（宽限） connecting | 顶部状态条「正在连接桌面引擎…」+ 写操作禁用 |
| 重连中（宽限） reconnecting | 顶部状态条「连接已断开，正在重连…」+ 写操作禁用 |
| 离线 · 无缓存 offline | 降级横幅「暂无缓存数据」+ 对话入待发箱 |
| 降级 · 只读缓存 degraded | 降级横幅「数据截至 今天 08:15」+ 对话入待发箱 |

切换即时生效，驱动顶部状态条/降级横幅、对话/任务/通知的操作禁用态全局变化（连接状态可在「我的」页配对设备卡查看）。

### 对话离线待发箱（FR-43 演示，T-S10）

切到 offline/degraded（或任意非就绪档）→ 对话输入仍常开，发消息 → 气泡呈「待发送」态并入队
（幂等 commandId，Keystore 加密落盘）→ 切回 direct/relay → 队列逐条自动串行发送，
成功删条目、气泡转常态；连接失败留队待下次恢复。

### 管家对话 mock 行为

- 发送消息 → 思考中（~0.9s）→ 流式打字机逐字回复（回复文案轮换）
- **第 2 次发言后**会浮现一张待确认建议 ActionCard（确认/拒绝按钮态演示）
- 配对成功后再次冷启动直达主界面（配对关系持久化于 SharedPreferences）
- 解除配对（我的页）→ 回到配对流

### 主题

深色默认（保护专注力）；我的页可切浅色，选择持久化。
色彩角色预留「工作=冷色 accent（靛蓝系）/ 家庭=暖色 accent（琥珀系）」映射位：
`ui/theme/Color.kt` 中 `RoleDomain.accent()`。

## 后续接入真实连接层的替换点

所有 Fake 实现集中在 `AppModelContainer`（手工 DI，无框架），替换时**只改容器一处，UI 零改动**。

### 1. `connection/ConnectionClient.kt` — 连接客户端接口（替换 `FakeConnectionClient`）

```kotlin
interface ConnectionClient {
    val state: StateFlow<ConnectionState>   // 三态：Direct / Relay / Offline(降级信息)
    val paired: StateFlow<Boolean>          // 配对关系
    fun setDebugMode(mode: DebugConnectionMode)  // Debug 状态模拟（真实实现可空实现）
    fun completePairing()
    fun unpair()
}

sealed interface TransportStatus {
    data object Connecting : TransportStatus             // 冷启动宽限态（20s 内不降级）
    data object Reconnecting : TransportStatus           // 会话失去后重连（宽限重启）
    data object Direct : TransportStatus                 // NSD 发现 → WS 直连桌面
    data object Relay : TransportStatus                  // WS 连中继，按 relay_id 转发
    data class Degraded(snapshotAvailable: Boolean, dataAsOf: String?) : TransportStatus
}
```

真实实现职责：NSD/mDNS 发现（`_egosync._tcp`）→ Noise XX 握手 → 同一加密帧协议双承载（直连 WS / 中继 WS）→ 断线重连发最新快照补齐（SNAPSHOT 帧）。
UI 已订阅 `state` 流：顶部状态条/降级横幅、「我的」页配对设备卡状态由该流驱动；写操作判据为独立三面 `commandReady`（T-S2）+ 配对健康面 `pairingHealth`（T-S3/T-S4）。

### 2. `sync/SnapshotStore.kt` — 快照数据（mock → SNAPSHOT/STATE_DELTA 帧驱动）

原型中为静态 object。真实层替换为版本化快照存储：`SNAPSHOT` 全量替换 + `STATE_DELTA` 增量合并（对应架构中的 `StateMerger`）。字段口径见架构「快照引擎」节（活跃/近期会话各 200 条、10MB 上限截断明示）。

### 3. `sync/ChatOutbox.kt` — 对话离线待发箱（T-S10：FIFO + 幂等 commandId + 落盘）

```kotlin
class ChatOutbox(persistence: ChatOutboxPersistence?) {
    val entries: StateFlow<List<OutboxEntry>>   // commandId(跨重发稳定)/roleId/conversationId(占位null)/content/localMessageId
    fun enqueue(entry: OutboxEntry)             // !commandReady 发送入队（变更同步落盘）
    fun remove(commandId: String)               // 成功/业务错误删条目（同步落盘）
}
```

落盘 `chat_outbox.bin`：Keystore AES-GCM 加密 + 原子写（镜像 `SnapshotCacheFile`），
进程被杀/冷启动构造时恢复；flush 守门 `commandReady && paired` 由 `ChatViewModel`
判定并逐条串行重发（一次一条、等本轮流式 done；连接类失败留队、业务错误删条目提示）。

### 4. `notify/NotificationDispatch.kt` — 通知分发抽象（V1 应用内实现）

```kotlin
interface NotificationDispatch {
    fun dispatch(notice: NoticeItem)
    fun markRead(id: String)
    fun markAllRead()
    fun respond(id: String, confirmed: Boolean)  // 敲门级快捷决策
}
```

`InAppNotificationAdapter` 为 V1 实现。FR-42 系统推送接入时新增 FCM/UnifiedPush 适配器实现同一接口。

### 5. `ui/chat/ChatViewModel.kt` — 对话指令（mock 循环 → COMMAND/STREAM_TOKEN 帧）

`sendMessage` 改为发 COMMAND 帧；回复改为 STREAM_TOKEN 帧驱动（打字机 UI 已按流式语义实现，`ChatMessage.streaming` 字段即为此预留）。

## 技术注记

- **AGP 9 内置 Kotlin 编译**：不需要（且不能再使用）`org.jetbrains.kotlin.android` 插件；仅保留 `org.jetbrains.kotlin.plugin.compose` 编译器插件
- **手工 DI**：`AppModelContainer` 在 `MainActivity` 组装，ViewModel 经 `viewModelFactory` 注入依赖——架构禁 Hilt 的官方替代模式
- **动效克制**：呼吸能量条（dashboard）与页面 fade 转场之外，仅扫码页有一条功能性扫掠线
- **低能量颜色**：采用 UX 规范的暗淡灰（弃 PRD FR-19 红色条款，避免负罪感；PRD 待清理标记）
- 已裁决的文档冲突：默认主题为 **dark**（本任务指令与 PRD 调性一致，覆盖 UX 规范的浅色默认）

## 目录结构

```text
companion-android/
├── app/src/main/java/com/egosync/companion/
│   ├── MainActivity.kt / AppModelContainer.kt
│   ├── pairing/        # 配对流（PairingScreen / PairingViewModel）
│   ├── connection/     # ConnectionState / ConnectionClient / Fake
│   ├── sync/           # SnapshotStore（mock 快照）/ QuickNoteQueue
│   ├── notify/         # NotificationDispatch / InAppNotificationAdapter
│   └── ui/
│       ├── AppNavHost.kt          # 导航图（pairing 根 + 四 Tab + 二级页）
│       ├── components/            # DegradedOverlay（降级态遮罩）
│       ├── chat/ tasks/ dashboard/ settings/
│       ├── briefing/ review/ notify/   # 二级页
│       └── theme/                 # Color / Theme / Type（dark 默认 + RoleAccent）
└── app/src/test/java/com/egosync/companion/
    ├── sync/QuickNoteQueueTest.kt
    ├── connection/FakeConnectionClientTest.kt
    └── pairing/PairingViewModelTest.kt
```
