---
title: 'companion-android 三主 ViewModel 经 AppModelContainer 重接 SnapshotStore 只读'
type: 'refactor'
created: '2026-08-26'
status: 'done'
baseline_commit: 'f638a7c1f6bb9c0259cd1cc7bf1936231546d485'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** ChatViewModel / TasksViewModel / DashboardViewModel 三个主 VM 在 AppNavHost.kt 以无参构造硬编码 mock（各自直引 `SnapshotStore` 静态数据），container 对它们不生效——「换 ConnectionClient 实现零改动」的承诺对三个主界面是空话；当前仅 Settings/Notifications/降级遮罩真正读 container。

**Approach:** AppModelContainer 暴露快照取数口 `snapshotStore`，三个主 VM 构造函数改收 `container`，初始状态与回复轮换数据一律经 `container.snapshotStore` 取只读快照；构造点在 AppNavHost 三处 initializer 改传 container。纯接线重构，UI 渲染数据不变。

## Boundaries & Constraints

**Always:**
- ConnectionClient 接口及其签名零改动；FakeConnectionClient 不动
- 只读语义：VM 不向 snapshotStore 回写；既有交互态（sendMessage 流式打字机、toggleTask 本地翻转、respondActionCard）逻辑保持原样
- 遵循 SettingsViewModel 的既有范式：VM 构造收 `container: AppModelContainer`
- UiState 数据类默认值与 `sample()`/@Preview 行为保持不变（预览路径允许保留 SnapshotStore 直接引用，见 Design Notes）
- 注释用中文，贴合各文件既有 KDoc 风格；仅改动必要行

**Ask First:**
- 若发现必须把 SnapshotStore 从 object 改为 class、或必须新增任何依赖（含测试库）才能完成，HALT 询问
- 若发现三 VM 之外还有隐藏构造点/调用方受签名变更影响，HALT 询问

**Never:**
- 不引入 Room/Hilt/OkHttp/任何网络库/Robolectric 等新依赖；依赖边界冻结为 Compose BOM/Material3/Navigation-Compose/kotlinx-coroutines + material-icons-core
- 不触碰 BriefingScreen / WeeklyReviewScreen / NotificationCenterScreen / TasksScreen·DashboardScreen 预览等仍直读 SnapshotStore 的页面（超本次范围）
- 不做响应式快照流改造（SNAPSHOT/STATE_DELTA 帧驱动属未来真实连接层工作）
- 不改后端、桌面端、图标系统（f638a7c 已冻结）

## I/O & Edge-Case Matrix

本变更为等价重接线：运行时数据源值恒等于原静态 mock（同一 object），无新输入/输出分支。删除本节。

</frozen-after-approval>

## Code Map

以下 Kotlin 路径均省略前缀 `companion-android/app/src/main/java/com/egosync/companion/`：

- `AppModelContainer.kt` -- 手工 DI 容器；新增 `snapshotStore` 取数口（唯一装配缝），紧邻 connection/quickNotes/notifications
- `ui/AppNavHost.kt` -- 三处硬编码构造点（L197/L212/L226 initializer）；改传 container
- `ui/chat/ChatViewModel.kt` -- 构造收 container；initialChat 种子与 butlerReplies 回复轮换经 container 取
- `ui/tasks/TasksViewModel.kt` -- 构造收 container；tasks 种子经 container 取
- `ui/dashboard/DashboardViewModel.kt` -- 构造收 container；roles 种子经 container 取

只读参考（禁止修改）：`connection/ConnectionClient.kt`（接口契约）；`sync/SnapshotStore.kt`（mock 数据源本体）；`ui/settings/SettingsViewModel.kt`（container 注入范式样板）

## Tasks & Acceptance

**Execution:**
- [x] `AppModelContainer.kt` -- 新增 `val snapshotStore = SnapshotStore` 属性并附中文 KDoc 说明装配缝语义 -- 建立 VM→container→SnapshotStore 唯一通道
- [x] `ui/chat/ChatViewModel.kt` -- 构造改 `(private val container: AppModelContainer)`；`_uiState` 种子改传 `container.snapshotStore.initialChat`；`sendMessage` 回复轮换改读 `container.snapshotStore.butlerReplies`；KDoc 相应微调 -- 聊天数据源脱离全局硬编码
- [x] `ui/tasks/TasksViewModel.kt` -- 构造改收 container；`_uiState` 种子改传 `container.snapshotStore.tasks` -- 任务数据源脱离全局硬编码
- [x] `ui/dashboard/DashboardViewModel.kt` -- 构造改收 container；`_uiState` 种子改传 `container.snapshotStore.roles` -- 仪表盘数据源脱离全局硬编码
- [x] `ui/AppNavHost.kt` -- 三处 `initializer { XxxViewModel() }` 改为 `initializer { XxxViewModel(container) }` -- 接通注入链路最后一环

**Acceptance Criteria:**
- Given 应用构建并进入任一主 Tab，when Chat/Tasks/Dashboard 渲染，then 界面数据与原 SnapshotStore mock 完全一致（视觉零回归），且 grep 可证三 VM 文件运行时取数路径均为 `container.snapshotStore.*`
- Given `cd companion-android && ./gradlew :app:assembleDebug`，when 执行，then BUILD SUCCESSFUL
- Given git diff 全量检查，when 核对 `connection/` 目录与 `build.gradle.kts`/`libs.versions.toml`，then ConnectionClient 接口与依赖清单零改动、无任何网络调用引入
- Given `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'`，when 执行，then 既有测试全部通过（无新增单测：AppModelContainer 依赖 android Context，JVM 单测不可达且禁加 Robolectric，验证以构建+grep+人审替代）

## Spec Change Log

## Design Notes

**装配缝选型**：容器暴露 `snapshotStore` 引用而非让 VM 各自 import 全局 object——今日指向 mock object，未来真实层把该属性替换为帧驱动版本化存储时，VM 与页面零改动（与容器 KDoc「替换 [connection] 实现——其余代码零改动」同一承诺延伸到数据面）。SnapshotStore 自身 KDoc 已预告此演进，本次不提前实现。

**预览路径例外**：UiState 默认值（`= SnapshotStore.tasks` 等）保留——`sample()` 被 Chat/Tasks/Dashboard 三 Screen 的 @Preview 无参消费，动它将波及预览块，违反外科手术边界；默认值不在运行时取数路径上（VM 显式传参覆盖）。人审时勿将残留的默认值引用误判为未完成项。

**注入范式**：与 SettingsViewModel 完全对齐——NavHost initializer 传 container，VM 以 `private val container` 持有并经其取数。

## Verification

以下命令均在 `companion-android/` 下执行（git/grep 在仓库根）：

**Commands:**
- `./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'` -- expected: 全部通过
- `git diff --stat` -- expected: 改动面恰为上述 5 个 Kotlin 文件；`git diff -- companion-android/app/src/main/java/com/egosync/companion/connection/ companion-android/app/build.gradle.kts companion-android/gradle/` 为空（接口与依赖零改动）
- `grep -rn "snapshotStore" companion-android/app/src/main/java/com/egosync/companion/{AppModelContainer.kt,ui/chat/ChatViewModel.kt,ui/tasks/TasksViewModel.kt,ui/dashboard/DashboardViewModel.kt}` -- expected: 四文件均命中（容器定义 + 三 VM 运行时取数）

**Manual checks (if no CLI):**
- 人审三 VM：交互逻辑（流式打字机/toggleTask/ActionCard）逐行未变，仅数据来源行变更

## Suggested Review Order

**装配缝：唯一设计决策**

- 设计入口——容器新增快照取数口，KDoc 说明换装语义与只读边界
  [`AppModelContainer.kt:31`](../../companion-android/app/src/main/java/com/egosync/companion/AppModelContainer.kt#L31)

**注入链路接通（NavHost 三处构造点）**

- Chat 工厂改传 container，接通 VM→container 数据链最后一环
  [`AppNavHost.kt:197`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L197)

- Tasks/Dashboard 两工厂同型透传，无额外逻辑
  [`AppNavHost.kt:212`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L212)
  · [`AppNavHost.kt:226`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L226)

**三 VM 数据源替换（种子 + 取数路径）**

- Chat 最复杂：构造持 container，种子取 initialChat；确认交互逻辑逐行未动
  [`ChatViewModel.kt:47`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L47)
  · [`ChatViewModel.kt:50`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L50)

- 回复轮换取数行替换（原直引全局单例），打字机循环本身零改动
  [`ChatViewModel.kt:74`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L74)

- Tasks 种子经 container；toggleTask 本地翻转语义保持
  [`TasksViewModel.kt:29`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L29)
  · [`TasksViewModel.kt:32`](../../companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksViewModel.kt#L32)

- Dashboard 种子经 container；纯只读渲染
  [`DashboardViewModel.kt:26`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardViewModel.kt#L26)
  · [`DashboardViewModel.kt:29`](../../companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardViewModel.kt#L29)
