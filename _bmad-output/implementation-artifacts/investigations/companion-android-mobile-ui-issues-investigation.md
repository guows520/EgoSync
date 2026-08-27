# Investigation: companion-android 移动端四项 UI 不一致问题

## Hand-off Brief

1. **What happened.** 用户报告 `companion-android/` 移动端原型有 4 处 UI 问题：页面顶部异常大空白、底部首个 Tab 标签写「管家」应为「对话」、对话界面缺「新会话/历史会话」按钮、管家对话框头像用了铃铛图标而非房子图标。四项均经源码与桌面端母本对照，根因已确认。
2. **Where the case stands.** 四项根因全部 Confirmed（代码级证据 + 桌面端事实源对照）；用户要求暂不执行，仅给原因与方案。
3. **What's needed next.** 等待用户确认方案后，用 `bmad-quick-dev` 按四项分别落地（其中 #3 工作量最大，需新增 ChatHeader 与 VM 会话 API）。

## Case Info

| Field            | Value                                                                                     |
| ---------------- | ----------------------------------------------------------------------------------------- |
| Ticket           | N/A                                                                                       |
| Date opened      | 2026-05-27                                                                                |
| Status           | Active                                                                                    |
| System           | companion-android（Kotlin + Jetpack Compose Material3，纯前端 + mock 原型；桌面端为事实源） |
| Evidence sources | 源码（AppNavHost.kt / ChatScreen.kt / MainActivity.kt / RoleIcons.kt / LucideIcons）+ 桌面端母本（egosync-app/src/components/chat/*、layout/Sidebar.tsx、onboarding/OnboardingView.tsx、butler/ButlerView.tsx）+ README.md |

## Problem Statement

用户原话（自由文本，视为假设）：

1. 页面顶部怎么要留那么大的空白
2. 把左下角的「管家」更改为「对话」，因为还可以切换其它角色
3. 对话界面上没有看到「新会话」和「历史会话」的按钮
4. 管家对话框中的图标，没有采用房子图标，不一致

## Evidence Inventory

| Source                | Status   | Notes                                                                        |
| --------------------- | -------- | --------------------------------------------------------------------------- |
| AppNavHost.kt         | Available | 底部四 Tab 定义 `tabs`（:63-68）；MainShellRoute 嵌套 Scaffold（:208-247）  |
| MainActivity.kt       | Available | `enableEdgeToEdge()` + 外层 Scaffold（:27,49-61）                            |
| ChatScreen.kt         | Available | 无 ChatHeader；RoleSwitcherRow/MessageBubble/ThinkingBubble（:121-287, 451-571） |
| RoleIcons.kt          | Available | `home → LucideIcons.Home` 映射存在（:66）                                   |
| 桌面 ChatHeader.tsx   | Available | 「新对话」(Plus) + 「历史对话」(History→ConversationList)（:36-61）         |
| 桌面 ChatBubble.tsx  | Available | `assistantIcon ?? Home`（:27,309）——管家气泡头像默认 Home                  |
| 桌面 Sidebar/Onboarding/ButlerView | Available | 全部用 Home 表示管家                                       |

## Investigation Backlog

| # | Path to Explore                              | Priority | Status | Notes                                            |
| - | -------------------------------------------- | -------- | ------ | ------------------------------------------------ |
| 1 | 顶部大空白根因（嵌套 Scaffold 双重 inset）     | High     | Done   | 已确认机制；像素量级需渲染验证                    |
| 2 | Tab 标签「管家」→「对话」                      | High     | Done   | 一行改动，但需同步 README                         |
| 3 | 缺「新会话/历史会话」入口                       | High     | Done   | 需移植桌面 ChatHeader；VM 需补会话 API            |
| 4 | 管家气泡头像 ConciergeBell ≠ 桌面 Home        | High     | Done   | 改 2 处图标；Onboarding/仪表盘/简报等仍用铃铛待评估 |

## Timeline of Events

| Time        | Event                                   | Source                       | Confidence |
| ----------- | --------------------------------------- | ---------------------------- | ---------- |
| 调查期       | 定位四个症状的代码据点                  | companion-android 源码       | Confirmed  |
| 调查期       | 对照桌面端母本确认「正确行为」          | egosync-app/src/components/* | Confirmed  |

## Confirmed Findings

### Finding 1: 顶部大空白——嵌套 Scaffold 双重应用系统栏 inset

**Evidence:**
- `MainActivity.kt:27` `enableEdgeToEdge()`；`:49` 外层 `Scaffold(...)`，无 topBar，未设 `contentWindowInsets` → 内容顶部 padding 含 statusBars 顶 inset。
- `MainActivity.kt:53-56` 内容 Box 用 `Modifier.padding(padding)` 应用 padding（`padding` modifier **不消费** window inset）。
- `AppNavHost.kt:208` `MainShellRoute` 内层 `Scaffold(bottomBar=...)`，无 topBar，未设 `contentWindowInsets` → 默认 `ScaffoldDefaults.contentWindowInsets`（含 systemBars 顶 inset）→ 再叠加一次 statusBars 顶 padding。

**Detail:** 四个 Tab 页内容被两次 statusBars 顶 inset 推下，合计约 2× 状态栏高度（典型 56–72dp，含刘海时更大），呈现为「那么大的空白」。外层 Scaffold 负责安全区即可，内层 Scaffold 不应再叠加。

### Finding 2: 底部首个 Tab 标签为「管家」，应为「对话」

**Evidence:** `AppNavHost.kt:64` `TabSpec("chat", "管家", LucideIcons.MessageSquare)`。

**Detail:** Tab 已用 `MessageSquare` 图标（对话语义），但 label 写「管家」。用户理由成立：该 Tab 承载管家 + 角色切换（`RoleSwitcherRow` 可切其它角色，`ChatScreen.kt:258-287`），称「管家」会误导。`README.md:34`「💬 管家（Tab 1）」需同步。

### Finding 3: 对话界面缺「新会话」「历史会话」入口

**Evidence:**
- 移动端 `ChatScreen.kt`（全文）顶部仅有 `RoleSwitcherRow`（:121），无任何「新对话/历史对话」按钮，也无会话列表弹层。
- 桌面端 `ChatHeader.tsx:36-61` 提供「新对话」(Plus) 与「历史对话」(History) 两按钮，点历史展开 `ConversationList`（会话列表 + 切换 + 删除）。
- 桌面 `ChatStream.tsx:1227` `onNewConversation={handleNewConversation}`，`:1051` 创建新会话逻辑。
- 移动端 `ChatViewModel.kt:88` `history = mutableMapOf<String?, List<ChatMessage>>()` 仅按角色保存**单条**当前会话，无多会话列表/新建/切换 API。

**Detail:** 移动端在信息架构上缺失了桌面对话头部组件，且 VM mock 仅支持每角色单会话，无法承载「新建/历史切换/删除」语义。要达高保真需补 UI 与 VM。

### Finding 4: 管家气泡头像用 ConciergeBell，与桌面 Home 不一致

**Evidence:**
- 移动端 `ChatScreen.kt:467` `MessageBubble`：`... ?: LucideIcons.ConciergeBell`（管家气泡头像）。
- 移动端 `ChatScreen.kt:559` `ThinkingBubble`：`LucideIcons.ConciergeBell`。
- 移动端 `ChatScreen.kt:271` `RoleSwitcherRow` 管家 chip：`LucideIcons.Home`（此处反而正确，注释 :256「Home 图标，镜像桌面侧栏首位」）。
- 桌面端 `ChatBubble.tsx:309` `const AssistantIcon = assistantIcon ?? Home;`，`:27` 注释「助手气泡左上角图标。未传时回退到 Home」。
- 桌面 `Sidebar.tsx:71`、`OnboardingView.tsx:253,304`、`ButlerView.tsx:92` 全部用 `Home` 表示管家。

**Detail:** 同一 ChatScreen 内，顶部角色 chip 用 Home、消息气泡与思考态用 ConciergeBell，自相矛盾；与桌面母本（Home 即管家）也不一致。`LucideIcons.Home` 在 `RoleIcons.kt:66` 已映射可用。

## Deduced Conclusions

### Deduction 1: 修复顶部空白只需取消内层 Scaffold 的 inset 叠加

**Based on:** Finding 1

**Reasoning:** 双重 inset 是由两层默认 `contentWindowInsets` 叠加产生。外层 MainActivity Scaffold 已提供安全区，内层 MainShellRoute Scaffold 只需 `contentWindowInsets = WindowInsets(0)`（或外层设 0、内层保留）。任选其一置零即消除重复，保留安全区不丢失。

**Conclusion:** 一行参数级改动可解决；不必删 Scaffold 结构（底部 NavBar 仍需内层 Scaffold 的 bottomBar 槽）。

### Deduction 2: #4 的修复范围可能不止 ChatScreen

**Based on:** Finding 4 + grep「管家」全量

**Reasoning:** 移动端 ConciergeBell 还出现在 `PairingScreen.kt:80`、`OnboardingScreen.kt:108,246,290`、`WeeklyReviewScreen.kt:141`、`DashboardScreen.kt:206`、`BriefingScreen.kt:90`。而桌面端这些场景一律用 Home。若追求与桌面事实源一致，范围会扩大。

**Conclusion:** 用户本次只点名「管家对话框」；建议先改 ChatScreen 两处，其余按用户判断是否扩展（见决策请求）。

## Hypothesized Paths

### Hypothesis 1: 顶部空白仅由双重 inset 造成（无其它来源）

**Status:** Open

**Theory:** 双重 statusBars inset 即全部成因；不存在某个页面额外加 statusBarsPadding。

**Supporting indicators:** MainActivity 与 MainShellRoute 均未显式设 `contentWindowInsets`。

**Would confirm:** 渲染验证——设 `contentWindowInsets = WindowInsets(0)` 后顶部空白缩为单倍状态栏高度（正常）。

**Would refute:** 改后仍异常大 → 需查 ChatScreen/DashboardScreen 等是否各自加了 statusBarsPadding（已扫，未发现，但需最终确认）。

**Resolution:** 待渲染验证后闭合。

## Missing Evidence

| Gap                          | Impact                              | How to Obtain                       |
| ---------------------------- | ----------------------------------- | ----------------------------------- |
| 渲染后的像素量级              | 确认双重 inset = 「那么大」的全部成因 | 设备/Preview 截图对比改动前后       |
| 用户对 #4 扩展范围的取舍      | 决定改动覆盖 Pairing/Onboarding/… 与否 | 提问（见下方决策请求）              |

## Source Code Trace

| Element       | Detail                                                                                  |
| ------------- | --------------------------------------------------------------------------------------- |
| Error origin  | #1 `MainActivity.kt:49` + `AppNavHost.kt:208`（嵌套 Scaffold）；#2 `AppNavHost.kt:64`；#3 `ChatScreen.kt:121`（无 ChatHeader）；#4 `ChatScreen.kt:467,559` |
| Trigger       | 进入主界面任一 Tab（#1）/ 切到对话 Tab（#2/#3/#4）/ 管家发消息或思考（#4）              |
| Condition     | 默认配置，无特殊状态触发                                                                |
| Related files | `RoleIcons.kt`（#4 可用 Home 映射）、`ChatViewModel.kt`（#3 需扩 VM）、`README.md`（#2 同步）、桌面 `ChatHeader.tsx`/`ChatBubble.tsx`（#3/#4 母本） |

## Conclusion

**Confidence:** High

四项根因均 Confirmed：

- **#1 顶部空白**：`MainActivity` 外层 Scaffold 与 `MainShellRoute` 内层 Scaffold 双重默认 `contentWindowInsets` 叠加 statusBars 顶 inset（High；像素量级待渲染确认，但机制明确）。
- **#2 Tab 标签**：`AppNavHost.kt:64` label 硬编码「管家」（High，单点）。
- **#3 缺新/历史会话**：移动端 ChatScreen 从未移植桌面 `ChatHeader`，VM 仅有单会话 mock（High）。
- **#4 图标不一致**：`ChatScreen` 管家气泡/思考态用 `ConciergeBell`，而桌面母本与角色 chip 均用 `Home`（High）。

## Recommended Next Steps

### Fix direction

按机制分组，均为外科手术式改动：

- **#1 inset 叠加**：`MainShellRoute` 的 Scaffold 设 `contentWindowInsets = WindowInsets(0)`（外层 MainActivity 已管安全区）。需新增 import `androidx.compose.foundation.layout.WindowInsets`。改后须真机/Preview 验证四 Tab 顶部不再双倍。
- **#2 标签改名**：`AppNavHost.kt:64` `"管家"` → `"对话"`；同步 `README.md:34` 文案。
- **#4 图标**：`ChatScreen.kt:467` 与 `:559` 把 `LucideIcons.ConciergeBell` 改为 `LucideIcons.Home`，与 `:271` chip 一致。
- **#3 新/历史会话入口**（工作量最大）：参考桌面 `ChatHeader.tsx`，在 `ChatScreen` 顶部 `RoleSwitcherRow` 之上加 `ChatHeaderRow`（Plus「新对话」+ History「历史对话」→ 展开 `ConversationList`）。VM 需补：
  - `newConversation()`：清空当前角色会话消息、开新会话；
  - 暴露当前角色的会话列表（mock 可为单条「当前会话」，或扩为多条种子）；
  - `selectConversation(roleId, convId)` 与 `deleteConversation(convId)`。
  - 决策点：mock 保真度——仅做「按钮可见 + 单会话占位」还是真正模拟多会话？见决策请求。

建议按 #1→#2→#4→#3 顺序，前三个低风险速做，#3 单独一次提交。

### Diagnostic

- #1 改后用 `@Preview`（带 `statusBars`）或真机截图对比改动前后顶部留白；确认底部 NavBar 不受影响。
- #4 改后确认 `LucideIcons.Home` 在 MessageBubble/ThinkingBubble tint 仍正确（onPrimaryContainer/onPrimary）。

## Reproduction Plan

1. `cd companion-android && ./gradlew :app:assembleDebug`（或开 Android Studio Preview）。
2. 启动到主界面（已配对已引导）。
3. 观察：顶部留白异常大（#1）；底部首个 Tab 显示「管家」（#2）；对话 Tab 顶部无新/历史会话按钮（#3）；管家消息气泡头像为铃铛非房子（#4）。

## Side Findings

- 移动端 ConciergeBell 还广泛出现在 Pairing/Onboarding/WeeklyReview/Dashboard/Briefing（见 Deduction 2）。桌面端这些场景一律 Home。若要做全局一致，范围远超本次 4 项；本次仅 ChatScreen 两处是用户点名项，其余留待决策。
- `ChatViewModel` 的 `history` map（`ChatViewModel.kt:88`）已具备「按角色隔离会话」的骨架，#3 的 VM 扩展可在此之上加多会话结构，而非另起。

## Follow-up: 2026-05-27

### New Evidence

无新代码证据；用户对两项开放决策已裁定（见下）。

### Decisions

| 决策 | 用户裁定 | 影响 |
| ---- | -------- | ---- |
| A：#3 mock 保真度 | **完整多会话 mock** | VM 扩为每角色多会话列表（新建/切换/删除/列表展示），完全镜像桌面 ChatHeader 行为；在 `ChatViewModel.kt:88` `history` map 骨架之上扩展 |
| B：#4 图标范围 | **全局改约 8 处** | ChatScreen 2 处 + Pairing/Onboarding/WeeklyReview/Dashboard/Briefing 约 6 处 `ConciergeBell` 全部改 `LucideIcons.Home`，彻底对齐桌面事实源 |

### Final Hand-off Brief

1. **What happened.** companion-android 四项 UI 问题根因全部 Confirmed：嵌套 Scaffold 双重 inset、Tab label 误写「管家」、未移植桌面 ChatHeader、管家头像铃铛≠桌面 Home。
2. **Where the case stands.** 方案已定且两项决策已裁定（完整多会话 mock + 全局图标统一）；用户要求暂不执行。
3. **What's needed next.** 经用户确认后按 #1→#2→#4→#3 顺序以 `bmad-quick-dev` 落地；#3 工作量最大，单独一次提交。
