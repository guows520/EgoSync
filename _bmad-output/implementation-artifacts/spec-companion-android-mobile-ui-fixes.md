---
title: 'companion-android 对话界面会话管理移植与四项 UI 一致性修复'
type: 'feature'
created: '2026-05-27'
status: 'done'
baseline_commit: 'eb7ef8c09a5ae519cc1e1e603b3578e01c2e38e7'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/companion-android-mobile-ui-issues-investigation.md'
  - '{project-root}/companion-android/README.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** companion-android 移动端原型有四项与桌面事实源不一致的 UI 缺陷：主界面顶部异常大空白（嵌套 Scaffold 双重状态栏 inset）、首个底部 Tab 标签写「管家」应为「对话」、对话界面缺「新会话/历史会话」入口、管家对话框头像用铃铛图标而非房子图标。

**Approach:** 按 `companion-android-mobile-ui-issues-investigation.md` 的根因分别落地——内层 Scaffold 置零 contentWindowInsets 消除双重 inset；Tab label 改「对话」并同步 README；移植桌面 `ChatHeader` 为移动端对话头部（新对话/历史对话下拉 + 会话删除），VM 由单会话扩展为多会话 mock；全局 8 处 `ConciergeBell` 改 `Home` 与桌面全站一致。

## Boundaries & Constraints

**Always:**
- 桌面端 `egosync-app/src/components/chat/*` 与 `layout/Sidebar.tsx` 为对话/管家图标的唯一事实源，移动端 mock 行为须镜像其语义（新对话/历史对话/会话删除/按角色隔离）。
- 仅外科手术改动 4 类症状相关代码；不顺手重构相邻排版或注释。
- 保持既有中文注释风格与 Tailwind/M3 token 用色；图标 tint 不变。
- 多会话状态机须可纯 JUnit 验证（不依赖 Android Context）。
- 保留现有 mock 触发行为：第 2 轮回复浮现的拆分提案/建议卡/角色涌现卡按 `selectRole` 既有隔离规则（管家视图承载，角色视图不承载）继续工作。

**Ask First:** 无（两项开放决策 A/B 已在调查中由用户裁定：完整多会话 mock + 全局图标统一）。

**Never:**
- 不为多会话引入真实持久化或网络层（仍为纯前端 mock，仅内存态）。
- 不删 `LucideIcons.ConciergeBell` 定义（图标库为调色板，保留以备他处）。
- 不引入 Robolectric/Context 依赖到 ChatViewModelTest。
- 不改动桌面端任何代码。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 管家视图新建会话 | activeRoleId=null，点击「新对话」 | 列表追加空会话「新对话」并设为当前；进行中流式被中断 | N/A |
| 角色视图新建会话 | activeRoleId="role-pm"，点击「新对话」 | 新会话归属该角色（不出现在管家列表） | N/A |
| 切换会话 | 点历史列表中某会话 | 消息流换为该会话内容；进行中流式被中断；卡片态按 selectRole 规则保留/清空 | N/A |
| 删除当前会话 | 点当前会话删除钮 | 从列表移除；回退到最近剩余会话；无剩余则自动新建空会话 | N/A |
| 新会话首条消息 | 空会话发送首条用户消息 | 会话标题更新为该消息截断（≤16 字符），updatedAt 刷新 | N/A |
| 历史列表为空 | 某角色列表仅当前空会话且删除 | 列表至少保留 1 条「新对话」（永不真空） | N/A |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/AppNavHost.kt` -- 底部 Tab 定义（:63-68）+ MainShellRoute 嵌套 Scaffold（:208-247）+ ChatRoute 构造 VM（:251-268）
- `companion-android/.../ui/chat/ChatScreen.kt` -- 顶部布局（:115-125）、RoleSwitcherRow（:256-287）、MessageBubble 头像（:467）、ThinkingBubble 头像（:559）
- `companion-android/.../ui/chat/ChatViewModel.kt` -- 单会话 history map（:88,95）、selectRole（:100-122）、persistCurrentMessages（:396-398）；仅依赖 `container.snapshotStore`
- `companion-android/.../sync/SnapshotStore.kt` -- ChatMessage（:146）、initialChat（:536）、roleChatSeeds（:610）
- `companion-android/.../ui/icons/LucideIcons.kt` -- 缺 `History` 图标；`Home`/`Plus`/`Trash2`/`Square` 已存在（:40,:72,:113,:130）
- 6 文件 ConciergeBell 用点：`PairingScreen.kt:80`、`OnboardingScreen.kt:245,290`、`WeeklyReviewScreen.kt:141`、`DashboardScreen.kt:206`、`BriefingScreen.kt:90`
- 桌面母本：`egosync-app/src/components/chat/ChatHeader.tsx`、`ConversationList.tsx`、`ChatBubble.tsx:309`（`assistantIcon ?? Home`）

## Tasks & Acceptance

**Execution:**
- [x] `AppNavHost.kt` -- 将 `TabSpec("chat","管家",...)` label 改为「对话」；为 `MainShellRoute` 的 `Scaffold` 增 `contentWindowInsets = WindowInsets(0,0,0,0)`（import `androidx.compose.foundation.layout.WindowInsets`）-- 消除顶部双重 inset 与 Tab 标签问题
- [x] `LucideIcons.kt` -- 新增 `val History`（lucide path：`M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8` / `M3 3v5h5` / `M12 7v5l4 2`）-- ChatHeader 历史对话钮所需
- [x] `SnapshotStore.kt` -- 新增 `data class ChatConversation(id,title,updatedAt:Long,messages)`；为管家追加 1 条历史种子会话（2-3 条 mock 消息，offsetHours≈26）；为各角色会话种子补标题 -- 多会话数据基础
- [x] `ChatViewModel.kt` -- 构造改为 `ChatViewModel(private val store: SnapshotStore = SnapshotStore)`；`history` 改为 `Map<String?, MutableList<ChatConversation>>`；UiState 增 `conversations` 与 `currentConversationId`；实现 `newConversation()`/`selectConversation(id)`/`deleteConversation(id)`；`persistCurrentMessages` 回写当前会话 `messages`+`updatedAt`；空会话首条用户消息更新标题；`init` 用 store 种子构建初始会话列表 -- 多会话语义核心
- [x] `AppNavHost.kt` ChatRoute -- 构造改为 `ChatViewModel(container.snapshotStore)` -- 适配新签名（VM 仅需快照源）-- 注：与任务1同文件，合并一次编辑
- [x] `ChatScreen.kt` -- 在 `Column` 顶部 `RoleSwitcherRow` 之上新增 `ChatHeaderRow`（Plus「新对话」+ History「历史对话」→ `DropdownMenu` 列出当前角色会话：标题/相对时间/Trash2 删除）；空态显示「暂无历史对话」；新增 `private fun formatRelativeTime(now, updatedAt)`（镜像桌面 `ConversationList.tsx:13-25`：刚刚/N分钟前/N小时前/M月D日 HH:mm）；接 `onNewConversation`/`onSelectConversation`/`onDeleteConversation` 回调 -- 移植桌面 ChatHeader
- [x] `ChatScreen.kt` -- `MessageBubble`(:467) 与 `ThinkingBubble`(:559) 的 `LucideIcons.ConciergeBell` 改为 `LucideIcons.Home` -- 管家头像与桌面一致
- [x] 6 文件（Pairing/Onboarding×2/WeeklyReview/Dashboard/Briefing）-- `ConciergeBell` 改 `Home` -- 全局图标统一
- [x] `README.md` -- Tab 1 标签「管家」改「对话」（:34）；对话 Tab 结构行下补一行「会话头：新对话/历史对话下拉（标题+相对时间+删除）」-- 文档同步
- [x] `ChatViewModelTest.kt` 新建（`app/src/test/.../ui/chat/`）-- 验证：新建归属当前角色并隔离、切换换消息流、删除当前会话回退与全删兜底、流式中断守卫 -- 锁定多会话语义意图

**Acceptance Criteria:**
- Given 主界面任一 Tab，when 渲染，then 顶部留白为单倍状态栏高度（不再双倍）
- Given 底部首个 Tab，when 查看，then 标签为「对话」且图标为 MessageSquare
- Given 对话 Tab，when 顶部，then 可见「新对话」与「历史对话」两按钮
- Given 管家视图有多会话，when 点历史展开，then 列表显示标题+相对时间+删除钮，且非空
- Given 角色视图新建会话，when 切回管家视图，then 管家列表不含该角色会话（按角色隔离）
- Given 删除当前会话，when 无剩余，then 自动新建空会话（列表永不为空）
- Given 管家消息气泡与思考态，when 渲染头像，then 图标为 Home（房子）
- Given 全局任意管家头像位（配对/引导/周复盘/仪表盘/简报），when 渲染，then 图标为 Home

## Spec Change Log

（空，由 step-04 评审循环追加。）

## Suggested Review Order

**多会话状态机（核心）**

- 入口：新建/切换/删除三操作与卡片随会话隔离的语义全在此
  [`ChatViewModel.kt:183`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L183)

- 切换会话先校验存在再中断流式，卡片不跨会话携带
  [`ChatViewModel.kt:219`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L219)

- 角色切换恢复离开时的会话 + 未种子化角色兜底建会话
  [`ChatViewModel.kt:147`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L147)

- 新会话工厂：updatedAt 严格最大，防同毫秒排序不稳
  [`ChatViewModel.kt:207`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L207)

- 会话记忆表：管家/各角色各自记住活跃会话 id
  [`ChatViewModel.kt:94`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L94)

- 落定回写后同步刷新会话列表快照（排序/相对时间不失真）
  [`ChatViewModel.kt:567`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L567)

- 首条消息命名：16 字符截断并防代理对残破
  [`ChatViewModel.kt:293`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L293)

**对话头部 UI（移植桌面 ChatHeader）**

- 新对话/历史对话按钮 + DropdownMenu 列表（标题+相对时间+删除）
  [`ChatScreen.kt:295`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L295)

- 相对时间格式化，镜像桌面 ConversationList
  [`ChatScreen.kt:275`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L275)

**顶部双重 inset + Tab 标签**

- 内层 Scaffold 置零 contentWindowInsets，消除双倍状态栏留白
  [`AppNavHost.kt:214`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L214)

- Tab 1 标签「管家」→「对话」
  [`AppNavHost.kt:65`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L65)

**管家图标统一（ConciergeBell→Home）**

- 消息气泡头像改 Home（思考态同款见 :691）
  [`ChatScreen.kt:599`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L599)

- History 图标新增（历史对话钮所需）
  [`LucideIcons.kt:55`](../../companion-android/app/src/main/java/com/egosync/companion/ui/icons/LucideIcons.kt#L55)

**外围（数据/测试/文档）**

- ChatConversation 数据类 + 管家历史种子会话
  [`SnapshotStore.kt:164`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L164)

- 多会话状态机 13 个回归用例（含评审 patch 的 3 个新增）
  [`ChatViewModelTest.kt:46`](../../companion-android/app/src/test/java/com/egosync/companion/ui/chat/ChatViewModelTest.kt#L46)

- README Tab 结构同步
  [`README.md:34`](../../companion-android/README.md#L34)

## Design Notes

**ChatViewModel 测试可达性**：现有 `ChatViewModel(container)` 私有构造 + `AppModelContainer.get(context)` 依赖 SharedPreferences，纯 JUnit 无法构造。VM 全文仅用 `container.snapshotStore`，而 `SnapshotStore` 是 `object`（无需构造）。故构造改为 `ChatViewModel(store: SnapshotStore = SnapshotStore)`：默认值满足 AppNavHost 生产用法（`ChatViewModel()` 即可，仍显式传 `container.snapshotStore` 以示依赖），测试可直接 `ChatViewModel(SnapshotStore)`。与 `PairingViewModel(FakeConnectionClient())` 直接依赖的既有测试模式一致，不引入 Robolectric。

**新会话标题**：空会话标题为「」（UI 显示「新对话」占位，镜像桌面 `conv.title || '新对话'`）；首条用户消息落定后取 `trimmed.take(16)`（≥17 字符加「…」）为标题。不复杂化：不按消息内容做智能命名。

**历史下拉**：移动端用 M3 `DropdownMenu`（锚定历史钮）替代桌面绝对定位 div，语义等价；删除钮用 `DropdownMenuItem` 的 `trailingIcon` 槽承载 `IconButton(Trash2)`。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL，零编译错误
- `cd companion-android && ./gradlew :app:testDebugUnitTest` -- expected: 全绿（含新增 ChatViewModelTest）

**Manual checks (无可用模拟器时的替代验证):**
- 顶部空白：代码级确认 `MainShellRoute` Scaffold 含 `contentWindowInsets = WindowInsets(0,0,0,0)`，外层 MainActivity Scaffold 仍提供单倍安全区。真机/Preview 截图对比为最终视觉确认，当前环境无法运行，须在接入设备时补做。
- 图标全局替换：grep 确认 `ConciergeBell` 在 main 源码中除 `LucideIcons.kt` 定义外无其他引用点。
