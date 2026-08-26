---
title: 'companion-android chat 屏 FR 对齐（组 1：FR-1/2/20/29/30/33）'
type: 'feature'
created: '2026-08-26'
status: 'done'
baseline_commit: 'bf845248c9d28bfb8ff613b36f7a65226ce93ceb'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/spec-companion-android-icon-parity.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 移动端 chat 屏缺失桌面已交付的 6 项对话核心交互（FR-1 意图路由、FR-2 任务拆分卡、FR-20 角色切换、FR-29 推理溯源、FR-30 不确定性表达、FR-33 工具执行可视），手机上无法获得与桌面等价的对话体验，「同一个产品」感受断裂。

**Approach:** 扩展 ChatViewModel mock 状态机与 ChatScreen 渲染，按 fr-groups.md 组 1 桌面基线逐项落地：两段委派气泡、对话内拆分提案卡、顶部水平角色切换器（桌面侧栏的语义适配）、可折叠执行溯源区、低置信内联标注、工具执行状态行+停止。数据全 mock 只读，图标全走 LucideIcons/RoleIcons。

## Boundaries & Constraints

**Always:**
- 零新依赖：仅 Compose BOM / Material3 / Navigation-Compose / kotlinx-coroutines + 既有 material-icons-core。
- 数据只走 mock / SnapshotStore 只读增补，不碰网络/连接层/后端。
- 图标一律 LucideIcons.kt + RoleIcons.getRoleIcon(id)；新增图标必须逐字移植 lucide 官方 SVG path data（viewport 24、strokeWidth 2、stroke-only）；禁 emoji。
- FR-20 强制语义适配：移动无侧栏，用顶部水平滚动角色切换器；角色图标一律 getRoleIcon。
- 严格逐项一致：功能/布局/图标逐项映射桌面基线（ChatStream.tsx / ChatBubble.tsx / TaskDecompositionCard.tsx / Sidebar.tsx / ButlerSettingsContent.tsx）。
- 沿用既有约定：中文注释、密度 token（densitySpec）、reduced-motion 处理、viewModelFactory + AppModelContainer 接线。

**Ask First:**
- 需要新增 ChevronRight/Play/Square 之外的 Lucide 图标时 → HALT 列候选。
- 需改动 SnapshotStore 既有字段或既有 mock 文案（只允许增补）时。

**Never:**
- 禁引入 Room / Hilt / OkHttp / 网络库 / material-icons-extended。
- 不碰组 2-7 屏（dashboard/role/memory/tasks/review/onboarding/notify）。
- 不改既有 ActionCard、initialChat、ThinkingBubble 行为。
- UI 渲染不出现 emoji（注释除外）。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 委派路由 | 发送含委派关键词消息（如「让产品经理看看」） | 两段气泡：管家「稍等，我让产品经理看一下」→ 产品经理头像+名「来自产品经理的反馈…」 | N/A |
| 常规路由 | 普通消息 | 单段管家流式回复（现状保持） | N/A |
| 拆分卡 | 提案出现（mock 触发） | 对话内卡片：ListTodo 图标+「建议拆分为 N 个任务」+条目列表+接受/不要拆分；点击→处理中→结果消息，双击守卫 | N/A |
| 溯源折叠 | 助手消息带 trace blocks | 历史默认折叠/流式中展开，ChevronRight 切换，含 Think/Narration/Action 三类块 | N/A |
| 低置信 | mock confidence<0.7 的回复 | 气泡尾部内联小字「（置信度较低，仅供参考）」 | N/A |
| 角色切换 | 点击顶部角色 chip | 切换角色视图：气泡头像/名/色温变化，消息流换成该角色 mock 对话 | N/A |
| 工具执行 | 流式期间 | 工具名+运行中指示行；停止按钮（Square）可中断打字机 | N/A |
| 引擎离线 | engineAvailable=false | 输入与发送禁用（现状语义不变） | 既有离线横幅 |

</frozen-after-approval>

## Code Map

- `companion-android/.../ui/chat/ChatViewModel.kt` -- mock 状态机：路由/拆分/溯源/低置信/角色切换状态（主改）
- `companion-android/.../ui/chat/ChatScreen.kt` -- 渲染：角色切换器、两段气泡、拆分卡、溯源区、工具状态行（主改）
- `companion-android/.../ui/icons/LucideIcons.kt` -- 新增 ChevronRight/Play/Square
- `companion-android/.../sync/SnapshotStore.kt` -- 只读增补：委派路由表、拆分提案、trace mock、角色会话种子
- `companion-android/.../ui/AppNavHost.kt` -- ChatRoute 接线新回调（签名变化时）
- 桌面基线（只读参考）：ChatStream.tsx:551/851-1010、ChatBubble.tsx:236-265、TaskDecompositionCard.tsx:12-85、Sidebar.tsx:6-144、ButlerSettingsContent.tsx:1457

## Tasks & Acceptance

**Execution:**
- [x] `ui/icons/LucideIcons.kt` -- 新增 ChevronRight/Play/Square（逐字移植 lucide path data） -- FR-29/33 图标
- [x] `sync/SnapshotStore.kt` -- 只读增补：委派关键词→角色映射、拆分提案、trace blocks、角色会话种子（butler+3 角色）、低置信回复标记 -- FR-1/2/29/30 数据
- [x] `ui/chat/ChatViewModel.kt` -- ChatUiState 扩展（activeRole/拆分卡/trace/工具状态）+ mock 状态机：两段委派时序、工具执行阶段、停止中断、双击守卫 -- 6 项逻辑
- [x] `ui/chat/ChatScreen.kt` -- 顶部水平滚动角色切换器（FR-20）+ 两段委派气泡（FR-1）+ TaskDecompositionCard（FR-2）+ 可折叠 ExecutionTrace（FR-29）+ 低置信标注（FR-30）+ 工具状态行与停止（FR-33） -- 6 项渲染
- [x] `ui/AppNavHost.kt` -- ChatRoute 接线 onRoleSelected/onStopStreaming/onDecompositionRespond
- [x] `RoleIconsTest` -- 不改，回归通过（8 用例）

**Acceptance Criteria:**
- Given 组 1 六项，when 对照 fr-groups.md 桌面基线逐项核对，then 功能/布局/图标映射逐项通过（映射表见 Design Notes）
- Given `./gradlew :app:assembleDebug`，when 构建，then BUILD SUCCESSFUL
- Given `./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'`，when 运行，then 通过
- Given app main 源码，when 图标契约同款 emoji 正则扫描，then 渲染命中为 0

## Design Notes

**FR-20 语义适配决策**：桌面 64px 竖排图标栏 → 移动顶部水平滚动行：首位 chip=管家（Home 图标，镜像 Sidebar 首位），其后角色 chip = RoleIcons.getRoleIcon(icon) + 角色色背景；选中态镜像桌面 indigo 高亮圆角。不选底部抽屉——聊天屏底部已有输入条+NavigationBar，层级冲突；顶部行与桌面「角色视图切换」语义最接近。

**FR-1 两段委派**（镜像 ChatStream.tsx streamBubbles 分桶）：管家视图命中委派关键词 → 气泡一「稍等，我让 X 看一下」（管家头像）→ 延时 → 气泡二「来自 X 的反馈…」（角色头像+名+色温）。移动在消息模型上以发言人字段区分（butler/roleId），不复制流式分桶复杂度。

**FR-30 对齐**：桌面基线为 ButlerSettingsContent.tsx:1457 的文案标注（徽章已移除）；移动对齐为气泡尾部内联小字，muted 色。

**FR-33 边界**：只做「工具名+状态指示」状态行 + 停止（Square）；桌面 @Skill 工作目录选择器不进移动（SPEC 明示排除）。

**图标决策**：FR-2 卡片用既有 ListTodo（fr-groups.md 明示的等效选项），不新增 ListTree。

## Verification

**Commands:**
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'` -- expected: 通过
- `cd companion-android && grep -rnP '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main | grep -v '//'` -- expected: 空输出

**Result (step-03 self-check):** ✅ `assembleDebug` BUILD SUCCESSFUL · `RoleIconsTest` 通过（8 用例，未改动） · emoji 扫描空输出。六项 FR 对照：FR-1 两段委派气泡（delegationKeywords→runDelegation，第二段角色头像 getRoleIcon+名标）；FR-2 拆分卡（ListTodo 等效+条目截止+双击守卫+结果行）；FR-20 顶部水平切换器（管家 Home chip+角色 chip 域色温+选中 primary）；FR-29 可折叠 ExecutionTrace（ChevronRight 旋转+三类块+read 不显标签镜像桌面）；FR-30 低置信内联「（置信度较低，仅供参考）」；FR-33 ToolStatusRow（Play+工具名+运行指示）+Square 停止钮。视觉保真待人审。

**Manual checks (if no CLI):**
- 无渲染环境：视觉保真度（布局/色温/密度）待人审截图比对桌面

## Spec Change Log

## Suggested Review Order

**状态机：mock 路由与流式生命周期（设计意图入口）**

- 发送入口：委派关键词判定与并发守卫，理解整体流程从这里开始
  [`ChatViewModel.kt:121`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L121)

- FR-1 两段委派：交接声明记录 id 供停止回滚，第二段角色反馈流式
  [`ChatViewModel.kt:146`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L146)

- 常规回复：工具阶段轮换（FR-33）+ 低置信路由（FR-30）+ 按视图计轮次触发 FR-2
  [`ChatViewModel.kt:168`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L168)

- 打字机内核：每步 ensureActive 取消防御 + 终态回写 history（评审补丁核心）
  [`ChatViewModel.kt:223`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L223)

- 停止：落定半截气泡并回滚悬空交接声明
  [`ChatViewModel.kt:252`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L252)

**FR-20 角色切换（语义适配）**

- 会话按视图隔离恢复，提案卡仅管家对话流承载
  [`ChatViewModel.kt:96`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L96)

- 顶部水平切换器：管家 Home chip 在首、角色 chip 走 getRoleIcon+域色温
  [`ChatScreen.kt:228`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L228)

**FR-29 执行溯源**

- 折叠头 ChevronRight 旋转 + 三类块渲染（read 不显标签镜像桌面）
  [`ChatScreen.kt:306`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L306)

- 溯源块种子数据（Think/Narration/Action）
  [`SnapshotStore.kt:443`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L443)

**FR-2 拆分提案卡**

- 卡片形态：ListTodo 等效图标+条目截止+PENDING 双击守卫+结果行
  [`ChatScreen.kt:684`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L684)

- 受理逻辑：busy 守卫+回执按 items.size 动态拼装（评审修正）
  [`ChatViewModel.kt:277`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatViewModel.kt#L277)

**FR-1/FR-30 气泡渲染**

- 发言人感知气泡：角色头像名标 + 低置信内联标注
  [`ChatScreen.kt:421`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L421)

**FR-33 工具执行可视与停止**

- 工具名+运行指示状态行；输入条 busy 时换 Square 停止钮
  [`ChatScreen.kt:494`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L494)
  [`ChatScreen.kt:183`](../../companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt#L183)

**外设：图标与接线**

- 新增 3 枚 lucide path 移植图标（ChevronRight/Play/Square）
  [`LucideIcons.kt:70`](../../companion-android/app/src/main/java/com/egosync/companion/ui/icons/LucideIcons.kt#L70)

- ChatRoute 三回调接线（切角色/停止/拆分受理）
  [`AppNavHost.kt:211`](../../companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt#L211)

- mock 数据只读增补区（委派路由表/拆分提案/角色会话种子等）
  [`SnapshotStore.kt:404`](../../companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt#L404)

</content>
</tool_call>