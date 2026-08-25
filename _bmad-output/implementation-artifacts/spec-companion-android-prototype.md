---
title: 'companion-android 手机伴侣 Compose 高保真原型'
type: 'feature'
created: '2026-08-25'
status: 'done'
baseline_commit: '3a23ca42c0eef5f0fec4f4b19ec6da70db5e5293'
context:
  - '{project-root}/_bmad-output/planning-artifacts/architecture.md'
  - '{project-root}/_bmad-output/planning-artifacts/prd-egosync.md'
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** EgoSync 手机伴侣 App（PRD §4.14，FR-40/41/43）已有架构定案但零代码，需要一份可在真机/模拟器安装运行的高保真前端原型，用于验证移动端信息架构、降级态体验与"老管家"美学，并冻结 UI 层与未来真实连接层的接缝。

**Approach:** 在仓库根新建独立单模块 `companion-android/`（Kotlin + Compose/Material3），纯前端 + mock 数据：连接能力只建 `ConnectionClient` 接口 + `FakeConnectionClient`（StateFlow 驱动四态），数据全部来自内存 mock 仓库；完成配对流、四 Tab 主界面、三个二级页、全局连接横幅与降级遮罩、设置页隐藏状态模拟入口；`./gradlew :app:assembleDebug` 可构建通过。

## Boundaries & Constraints

**Always:**
- 包结构遵循 architecture.md：`com.egosync.companion` 下 `pairing/ connection/ sync/ notify/ ui/<feature>/ theme/`；文件名 `XxxScreen.kt / XxxViewModel.kt`
- Material3，dark 为默认 theme，light 可切；色彩角色预留"工作=冷色 accent / 家庭=暖色 accent"映射位（theme 层 `RoleAccent` 映射，不实际联动角色切换动效）
- 依赖仅限：Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines（+ AGP/Kotlin/核心 AndroidX 编译期必需件）；Gradle KTS + version catalog（libs.versions.toml）；minSdk 26 / compile&target = 36
- 全部 Screen 提供 `@Preview`；动效仅限呼吸能量条与页面转场
- README.md 必含：页面地图、mock 开关使用说明、Fake 实现类清单及接口签名（后续接入真实连接层的替换点）

**Ask First:**
- 任何需要引入新运行时依赖的场景
- 需要修改 `companion-android/` 之外任何文件（含仓库根配置）的场景

**Never:**
- 任何网络、Noise 加密、NSD 发现、FCM 代码；不做系统通知（FR-42 DEFERRED），通知中心为应用内页面
- 引入 Room、Hilt、全局状态框架、OkHttp 等运行依赖（架构反模式清单生效）
- 在仓库根建任何共享构建配置或 workspace

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 首跑配对全流程 | 冷启动，无配对记录 | 欢迎说明→扫码取景模拟→连接中动画→配对成功→进入主界面 | 任意一步可返回重试 |
| 连接状态切换 | 设置页"状态模拟"选 direct/relay/offline/degraded | 横幅三态+降级遮罩全局即时切换 | N/A |
| 降级态操作拦截 | offline/degraded 状态下点对话输入/任务勾选 | 引擎依赖入口禁用并说明原因；速记输入条可用 | N/A |
| 速记排队 | 降级态提交速记 | 本地队列计数显示，恢复连接后标记已提交 | N/A |
| 主题切换 | 我的页切换 dark/light | 全局主题即时生效并默认 dark | N/A |
| 未配对冷启动 | 清数据后启动 | 进入配对流而非主界面 | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/README.md` -- 交付说明：页面地图、mock 开关、Fake 替换点清单
- `companion-android/settings.gradle.kts` -- Gradle 设置，单模块 `:app`
- `companion-android/gradle/libs.versions.toml` -- version catalog（AGP 9.3.2 / Kotlin 2.4.10 / Compose BOM 2026.08.00 / Navigation 2.9.8 / coroutines 1.11.0）
- `companion-android/build.gradle.kts` -- 根构建脚本，空壳 + 插件别名声明
- `companion-android/app/build.gradle.kts` -- :app 模块：minSdk 26 / compile&targetSdk 36 / JvmTarget 17 / compose enable + 单测
- `companion-android/app/src/main/AndroidManifest.xml` -- 应用清单：MainActivity、深色默认主题、无任何权限
- `companion-android/app/src/main/java/com/egosync/companion/MainActivity.kt` -- 入口：AppNavHost + 主题包裹
- `companion-android/app/src/main/java/com/egosync/companion/ui/theme/Theme.kt` -- Material3 深色默认 + light scheme + RoleAccent 冷/暖 accent 映射位
- `companion-android/app/src/main/java/com/egosync/companion/ui/theme/Type.kt` -- 系统默认字体栈（不引在线字体）
- `companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionClient.kt` -- 接口 + ConnectionState 三态枚举 + DegradedInfo
- `companion-android/app/src/main/java/com/egosync/companion/connection/FakeConnectionClient.kt` -- StateFlow 驱动的假连接客户端（debug 状态模拟的运行时宿主）
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt` -- 快照数据模型 + Fake 数据（角色/任务/简报/复盘/通知/会话）+ 读接口
- `companion-android/app/src/main/java/com/egosync/companion/sync/QuickNoteQueue.kt` -- 速记队列（内存 mock：提交、计数、恢复后标记已提交）
- `companion-android/app/src/main/java/com/egosync/companion/notify/NotificationDispatch.kt` -- 应用内通知分发接口 + 三级（whisper/tap/knock）语义
- `companion-android/app/src/main/java/com/egosync/companion/notify/InAppNotificationAdapter.kt` -- V1 应用内实现：未读状态管理
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingViewModel.kt` -- 配对流状态机（步骤推进/回退/完成）
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt` -- 欢迎说明→扫码模拟→连接中→成功四步全屏页
- `companion-android/app/src/main/java/com/egosync/companion/connection/ConnectionStatusBanner.kt` -- 顶部三态横幅（局域网直连/中继转发/离线，三色+图标）
| `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt` -- 管家对话：消息流（用户/管家气泡、流式打字机占位、思考中态）、输入框、建议 ActionCard（确认/拒绝）
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt` -- mock 对话状态：发送→思考→流式回显循环
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt` -- 四象限分组列表（Q1~Q4 分色、大石头星标、勾选完成）
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt` -- 象限分组列表状态 + 勾选交互
- `companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt` -- 角色卡横向滑动 + 能量条呼吸动效 + 任务/记忆/会话统计
- `companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardViewModel.kt` -- 角色卡列表状态
- `companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsScreen.kt` -- 设置列表：主题切换、通知级别开关、配对设备卡、解除配对、隐藏"状态模拟"入口
- `companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsViewModel.kt` -- 主题/通知/配对/模拟状态管理
- `companion-android/app/src/main/java/com/egosync/companion/ui/briefing/BriefingScreen.kt` -- 晨间简报（分节文本+行动点卡片）
- `companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt` -- 周复盘（成绩单+能量趋势 Canvas 柱状图自绘）
- `companion-android/app/src/main/java/com/egosync/companion/ui/notify/NotificationCenterScreen.kt` -- 通知中心（whisper/tap/knock 三级分组、未读圆点）
- `companion-android/app/src/main/java/com/egosync/companion/ui/components/DegradedOverlay.kt` -- 降级态遮罩（灰蒙层+数据截止时间+引擎禁用说明+底部速记输入条）
- `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt` -- 导航图：pairing → bottom-bar 四 Tab + 二级页
- `companion-android/app/src/main/res/...` -- 图标（adaptive icon）与字符串资源
- `companion-android/app/src/test/...` -- 单元测试：QuickNoteQueue、PairingViewModel、FakeConnectionClient 状态切换
- `companion-android/gradle/wrapper/...` -- Gradle 9.7.1 wrapper

## Tasks & Acceptance

**Execution:**
- [x] `companion-android/settings.gradle.kts, gradle/libs.versions.toml, build.gradle.kts, app/build.gradle.kts, gradle wrapper, AndroidManifest.xml` -- 搭建 Gradle KTS + version catalog 单模块工程骨架 -- 一切编译的地基
- [x] `ui/theme/{Theme.kt, Type.kt}, res/values/` -- Material3 深色默认主题、light scheme、RoleAccent 映射位、系统字体栈 -- 美学基调的代码化
- [x] `connection/{ConnectionClient.kt, FakeConnectionClient.kt}, sync/{SnapshotStore.kt, QuickNoteQueue.kt}, notify/{NotificationDispatch.kt, InAppNotificationAdapter.kt}` -- 接口 + Fake 实现（状态流、mock 快照、速记队列、应用内通知） -- 原型运行的内脏，未来真实层的替换点
- [x] `pairing/{PairingScreen.kt, PairingViewModel.kt}, ui/AppNavHost.kt, MainActivity.kt` -- 配对四步流 + 导航骨架 -- 首跑体验与全局导航
- [x] `ui/chat/{ChatScreen.kt, ChatViewModel.kt}` -- 管家对话 Tab：消息流/流式打字机/思考中态/输入框/ActionCard 确认拒绝 -- 核心体验一
- [x] `ui/tasks/{TasksScreen.kt, TasksViewModel.kt}` -- 四象限任务 Tab：Q1~Q4 分色、大石头星标、勾选完成 -- 核心体验二
- [x] `ui/dashboard/{DashboardScreen.kt, DashboardViewModel.kt}` -- 仪表盘 Tab：角色卡横滑、能量条呼吸动效、统计数字 -- 栏心体验三（呼吸感动效唯一所在）
- [x] `ui/settings/{SettingsScreen.kt, SettingsViewModel.kt}` -- 我的 Tab：主题切换、通知级别、配对设备卡、解除配对、隐藏状态模拟入口 -- 核心体验四
- [x] `ui/briefing/BriefingScreen.kt, ui/review/WeeklyReviewScreen.kt, ui/notify/NotificationCenterScreen.kt` -- 二级页：晨间简报/周复盘 Canvas 柱状图/通知中心三级分组 -- PRD 核心场景的移动呈现
- [x] `connection/ConnectionStatusBanner.kt, ui/components/DegradedOverlay.kt` -- 全局组件：三态横幅 + 降级遮罩（截止时间+禁用说明+速记条） -- FR-40/43 的状态可见性与诚实代价
- [x] `app/src/test/` -- QuickNoteQueue / PairingViewModel / FakeConnectionClient 状态切换单元测试 -- 意图级验证（速记无丢失、配对可回退、状态可切换）
- [x] `companion-android/README.md` -- 页面地图、mock 开关使用说明、Fake 替换点清单（接口签名） -- 后续接入真实连接层的地图
- [ ] 验证 -- `./gradlew :app:assembleDebug` 构建通过 + 单测通过 -- 交付验收线

**Acceptance Criteria:**
- Given 未配对冷启动，when App 启动，then 进入配对流，四步走完后进入主界面且再次启动直达主界面
- Given 已配对，when 在设置页状态模拟中切换 direct→relay→offline→degraded，then 横幅颜色/图标/文案与降级遮罩即时全局变化，引擎依赖入口禁用
- Given 降级态，when 提交速记，then 队列计数增加；when 状态切回 direct，then 队列清空标记已提交
- Given 管家对话页，when 发送消息，then 依次出现思考中态→流式打字机文字→完成；ActionCard 确认/拒绝按钮有明确状态变化
- Given 周复盘页，when 查看，then 能量趋势柱状图为自绘 Canvas 且带周标签
- Given 全部 Screen 文件，when 编译，then 至少一个 @Preview 注解存在
- Given 仓库根，when 执行 `./gradlew :app:assembleDebug`，then BUILD SUCCESSFUL 且无 lint 致命错误

## Spec Change Log

- **2026-08-25 评审循环 1**（三路对抗评审：盲审/边界猎手/验收审计）：
  - 触发发现：降级遮罩不消费指针事件可点穿（HIGH）；扫掠线动画 no-op（HIGH，自查发现）；建议卡轮次竞态（MEDIUM）；简报行动点死按钮（MEDIUM）；通知中心快照读取、ActionCard 双击、容器随 Activity 重建丢速记（边界猎手）。
  - 修订：遮罩加 pointerInput 全量消费；扫掠线改 offset；建议卡轮次快照；简报行动点接入本地决策状态 + 回调参数；通知中心改 collectAsState；容器改进程级单例；ActionCard PENDING 守卫；配对导航 launchSingleTop；并发发送守卫；横幅空 dataAsOf 回退文案。
  - 避免的坏状态：降级锁定失效（可离线发令）、演示动效假活、演示承诺不兑现。
  - KEEP：StateFlow 驱动全部 UI（流订阅一致性）；手工 DI 单容器组装（真实层替换只改一处）；mock 语气与色彩 token 与 PRD/UX 规范逐项对应。

## Design Notes

设计理由（非显而易见处）：
1. **无 DI 框架下的组装**：架构禁 Hilt。用一个 `AppModelContainer`（普通 object，持有 FakeConnectionClient / SnapshotStore / QuickNoteQueue / InAppNotificationAdapter 单例）在 MainActivity 组装，ViewModel 通过 `viewModel(factory=...)` 接收依赖——接真实层时只改 Container 一处。这是 Android 官方推荐的手工 DI 模式，不是自创范式。
2. **degraded 与 offline 的区分**：offline = 桌面不可达（无快照或极旧），degraded = 桌面可达但引擎降级。Mock 中 degraded 显示"数据截至 08:15，引擎功能受限"。
3. **四个 UI Tab 用 Navigation-Compose 底栏路由**（route: chat/tasks/dashboard/settings），二级页 briefing/review/notifications 为主图 push 路由；pairing 为独立根。
4. **已裁决的文档冲突**：①默认主题——用户指令明确 dark 默认（覆盖 UX 规范的浅色默认）；②低能量颜色——采用 UX 规范暗淡灰（弃 PRD FR-19 红色，避免负罪感），PRD 红色条款记为待清理。
5. **色彩 token（源自桌面 UX 规范，Material3 化）**：深色 background #0F1117 / surface #1A1B2E / elevated #252638 / 文本 #E8E8ED / 次文本 #9CA3AF；功能色 成功#10B981 信息#3B82F6 警告#F59E0B 错误#EF4444；象限色 Q1红#EF4444 Q2蓝#3B82F6 Q3琥珀#F59E0B Q4灰#9CA3AF；角色 accent：管家中性#6366F1 工作冷#4F46E5 家庭暖#D97706 学习#7C3AED 健康#059669；能量色谱 ≥70%翠绿 / 40~69%琥珀 / <40%暗淡灰。

mock 数据黄金样例（直接用于 SnapshotStore）：
- 管家语气：「早上好 boss。今天最重要的一件事是下午 2 点的产品评审，材料产品经理已经备好了。另外，女儿钢琴课在四点半。」
- 角色：产品经理🎯能量82%·任务14/记忆38/会话26；父亲🏠56%·任务6/记忆17/会话12；学习者📚33%·任务9/记忆21/会话8
- 任务：Q1「产品评审会议材料」14:00；Q2「读完《深度工作》第3章」🪨大石头；Q3「回复供应商询价邮件」；Q4「刷 20 分钟行业资讯」
- 通知：whisper=角色沉淀记忆（仅绿点无文案）；tap=「顺便说一句，父亲提醒这周五是女儿钢琴课。」；knock=「产品评审改到今天下午 2 点，和接孩子撞了——需要你决定一下。」
- 简报：「……学习者那边昨晚沉淀了 2 条读书笔记，不着急处理。今天最重要的一件事：评审前把竞品对比页过一遍。」
- 复盘：「产品经理完成 5/7 项，能量 68%→82%；大石头"陪女儿看画展"✓、"读完第3章"→继续推进。下周想先关注哪个方面？」

## Verification

**Commands:**
- `./gradlew :app:assembleDebug --console=plain` -- expected: BUILD SUCCESSFUL，产出 app-debug.apk
- `./gradlew :app:testDebugUnitTest --console=plain` -- expected: 全部单测通过

**实测记录（2026-08-25）：**
- `./gradlew :app:assembleDebug :app:testDebugUnitTest` → BUILD SUCCESSFUL；13/13 单测通过
- 产物 `app/build/outputs/apk/debug/app-debug.apk`（12.4MB）
- 全部 8 个 Screen 文件 + 3 个全局组件均有 @Preview；编译零警告
- **偏差记录**：compileSdk/targetSdk 由 36 → 37（Compose BOM 2026.08.00 全系要求 compileSdk ≥ 37；用户原始指令"target&compile 最新稳定"为准，AGP 9.3.2 支持）
- **AGP 9 适配**：移除 org.jetbrains.kotlin.android 插件（AGP 9 内置 Kotlin 编译，显式拒绝该插件）

**Manual checks (if no CLI):**
- Android Studio 打开 companion-android/ 可同步；任一 Screen 的 @Preview 可渲染
