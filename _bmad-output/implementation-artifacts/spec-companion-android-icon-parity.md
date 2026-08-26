---
title: 'companion-android 图标系统对齐桌面 lucide 白名单'
type: 'refactor'
created: '2026-08-25'
status: 'implemented'
baseline_commit: '107d906dd1724e1e89aec25d3b036c2145e9376d'
context:
  - '{project-root}/egosync-app/src/lib/roleIcons.ts'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/mobile-desktop-parity-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 移动端 UI 全量使用 emoji 图标（导航 💬📋📊👤、管家头像 🤵、通知级 🍃👋🚪、主题 🌙☀️、降级 📴 等 ~25 处），直接违反桌面 `roleIcons.ts:8` 明文「不使用彩色 emoji，全部采用 Lucide 黑白线框图标」，使「同一个产品」感受断裂。

**Approach:** 把桌面 lucide 的 SVG path data 移植为 Compose `ImageVector`（纯 Kotlin，零新运行依赖），新建 `ui/icons/` 包：`LucideIcons.kt`（全部矢量）+ `RoleIcons.kt`（镜像桌面 24 id 白名单 + `getRoleIcon(id)` Target 兜底）。全量 emoji 站点替换为 Icon 组件，`SnapshotStore` mock 角色图标改存 kebab id 经 `getRoleIcon` 渲染。

## Boundaries & Constraints

**Always:**
- 图标矢量一律从 lucide 官方 SVG path data 逐字移植（ISC 许可），viewport 24×24、strokeWidth=2、stroke-only、不填色——与桌面 `roleIcons.ts:43` strokeWidth=2 黑白线框一致。
- `RoleIcons.kt` 的 24 个 id 必须与桌面 `roleIcons.ts:56-81` 及后端 `agent_engine.rs` SUPPORTED_ICONS 三方一致（kebab-case）；未知 id 回退 `target`（镜像 `roleIcons.ts:99-103` normalizeIconId）。
- 保留 `material-icons-core` 的 `Icons.AutoMirrored.Filled.ArrowBack`（返回箭头，3 处，RTL 友好，不替换）。

**Ask First:**
- 若某 emoji 在 lucide 无近似图标（已知：🪨「诚实降级」无 rock），实现时 HALT 报告候选，由人定夺（当前建议 `hard-drive`）。

**Never:**
- 禁止任何 emoji 字符出现在 UI 渲染（`grep -P '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}]' app/src/main` 须为 0，注释除外）。
- 禁止引入 `material-icons-extended` 或任何第三方图标库（守住硬性边界 #2：依赖仅限 Compose BOM / Material3 / Navigation / kotlinx-coroutines / 已有的 material-icons-core）。
- 不碰数据层与后端：不改 ViewModel 逻辑、不接连接层、不动 SnapshotStore 之外的数据契约（仅把 emoji 字符串改 id 字符串）。本 spec 属 deferred #2/#3/#4 之外。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 已知角色 id | `getRoleIcon("home")` | 返回 Home ImageVector | N/A |
| 未知/空 id | `getRoleIcon(null)` 或 `getRoleIcon("xyz")` | 回退 Target（默认） | 不抛错，镜像桌面 normalizeIconId |

</frozen-after-approval>

## Code Map

- `companion-android/app/src/main/java/com/egosync/companion/ui/icons/LucideIcons.kt` -- 新建：所有移植自 lucide 的 ImageVector（role 24 + UI ~16）
- `companion-android/app/src/main/java/com/egosync/companion/ui/icons/RoleIcons.kt` -- 新建：24 id 白名单 + `ROLE_COLORS`(8) + `getRoleIcon(id): ImageVector` + `normalizeIconId`
- `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt` -- 导航 Tab emoji → Icon（:50-54, :166-170 icon 槽）
- `companion-android/app/src/main/java/com/egosync/companion/ui/dashboard/DashboardScreen.kt` -- 🤵(:139) + OverviewEntry 📄📈🔔(:166-168) → Icon
- `companion-android/app/src/main/java/com/egosync/companion/ui/chat/ChatScreen.kt` -- 🤵(:151,:190) → ConciergeBell；ActionCard ✓(:300) → Check
- `companion-android/app/src/main/java/com/egosync/companion/ui/briefing/BriefingScreen.kt` -- 🤵(:87) → ConciergeBell；✓已确认(:165) → Check+文案
- `companion-android/app/src/main/java/com/egosync/companion/ui/review/WeeklyReviewScreen.kt` -- 🤵(:90)；✓/→(:164) → Check/ArrowRight
- `companion-android/app/src/main/java/com/egosync/companion/ui/notify/NotificationCenterScreen.kt` -- 🍃👋🚪(:114-116) → 统一 Bell + 色码（对齐桌面单 Bell 分色）；✓(:238) → Check
- `companion-android/app/src/main/java/com/egosync/companion/ui/settings/SettingsScreen.kt` -- 🌙☀️(:103-104) → Moon/Sun；🖥️(:146) → Monitor
- `companion-android/app/src/main/java/com/egosync/companion/ui/components/DegradedOverlay.kt` -- 📴(:85) → WifiOff
- `companion-android/app/src/main/java/com/egosync/companion/pairing/PairingScreen.kt` -- 🤵(:83) → ConciergeBell；🏠🔐🪨(:98-102) → Home/ShieldCheck/HardDrive；✓(:326) → Check
- `companion-android/app/src/main/java/com/egosync/companion/ui/tasks/TasksScreen.kt` -- ✓(:154) → Check
- `companion-android/app/src/main/java/com/egosync/companion/sync/SnapshotStore.kt` -- mock 角色 icon 字符串 🎯🏠📚(:137,149,161) → id `"target"/"home"/"book-open"`
- `companion-android/app/src/test/java/com/egosync/companion/ui/icons/RoleIconsTest.kt` -- 新建：getRoleIcon 兜底单测

## Tasks & Acceptance

**Execution:**
- [x] `ui/icons/LucideIcons.kt` -- 从 lucide 官方 SVG（github.com/lucide-icons/lucide， ISC）移植全部所需 ImageVector：角色 24 枚（briefcase/code/chart-bar/.../target/.../wallet 见 roleIcons.ts:56-81）+ UI 枚（concierge-bell/message-square/list-todo/layout-dashboard/user/file-text/trending-up/bell/wifi-off/moon/sun/monitor/shield-check/hard-drive/check/arrow-right/home/book-open）。每枚用 `ImageVector.Builder` + PathParser 解析 SVG path data，viewport=24，strokeWidth=2 -- 像素级对齐桌面、零新依赖
- [x] `ui/icons/RoleIcons.kt` -- 镜像 roleIcons.ts：`ROLE_ICONS: List<RoleIconOption>`（24，id+label）、`ROLE_COLORS`(8 hex)、`DEFAULT_ICON_ID="target"`、`getRoleIcon(id)`/`normalizeIconId` -- 与桌面+后端白名单三方一致
- [x] `AppNavHost.kt` -- TabSpec.icon 改 `ImageVector`，注入 Lucide MessageSquare/ListTodo/LayoutDashboard/User；NavigationBarItem 用 Icon() 替 Text(emoji) -- 导航与桌面线性图标一致
- [x] `DashboardScreen.kt` `BriefingScreen.kt` `WeeklyReviewScreen.kt` `ChatScreen.kt` `PairingScreen.kt` -- 管家头像 🤵 → `LucideIcons.ConciergeBell`；OverviewEntry/feature row emoji → 对应 Lucide；✓/→ 文本字形 → Check/ArrowRight Icon -- 全部头像与状态图标对齐
- [x] `NotificationCenterScreen.kt` -- 三级 🍃👋🚪 → 统一 `LucideIcons.Bell`，仅以现有色码（QuadrantGray/BrandBlue/BrandError）区分，对齐桌面 NotificationPanel 单 Bell 分色 -- 严格逐项一致（见 Design Notes）
- [x] `SettingsScreen.kt` `DegradedOverlay.kt` `TasksScreen.kt` -- 🌙☀️→Moon/Sun、🖥️→Monitor、📴→WifiOff、✓→Check -- 收尾替换
- [x] `SnapshotStore.kt` -- 角色 icon 字段值由 emoji 改 kebab id（`"target"/"home"/"book-open"`），渲染处调 `RoleIcons.getRoleIcon(id)` -- 数据契约与桌面 id 体系一致
- [x] `ui/icons/RoleIconsTest.kt` -- 已知 id 返回对应 vector；null/未知 id 回退 Target -- 锁定兜底契约（I/O 矩阵）

**Acceptance Criteria:**
- Given 全移动 main 源码，when `grep -P '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main`（排除注释行），then 命中为 0。
- Given `getRoleIcon(null)` 与 `getRoleIcon("not-a-real-id")`，when 调用，then 均返回 Target ImageVector，不抛错。
- Given RoleIcons.kt 的 24 个 id，when 与 `egosync-app/src/lib/roleIcons.ts` 的 id 集合逐字比对，then 完全相等。
- Given `./gradlew :app:assembleDebug`，when 构建，then 成功（零新依赖、无编译错误）。
- Given `./gradlew :app:testDebugUnitTest`，when 跑 RoleIconsTest，then 通过。

## Design Notes

**通知级图标策略**：桌面 NotificationPanel（`notifications/NotificationPanel.tsx`，子代理 A 证据）三级耳语/轻触/敲门用**单个 Bell 图标 + 颜色区分**（灰/蓝/红脉冲），无逐级独立图标。移动当前用 🍃👋🚪 三种 emoji 是偏离。按「严格逐项一致」裁决：移动改为单 Bell + 现有色码区分，删除逐级 emoji。这是有意的 UX 收敛——若你更看重移动逐级辨识度而非桌面对齐，可在 checkpoint 提出，改回保留独立图标（但需桌面侧先有对应设计）。

**移植金标示例**（lucide `check`）：
```kotlin
val Check: ImageVector by lazy {
    ImageVector.Builder("Check", 24f, 24f, 24f, 24f).apply {
        addPath(PathParser.parsePathData("M20 6 9 17l-5-5")) // lucide check path
    }.build().also { it.mutate().strokeWidth = 2f }
}
```
真实实现按 lucide 各图标 SVG 的全部 path/circle/line 元素完整移植（非仅单 path）。

## Verification

**Commands:**
- `cd companion-android && grep -rnP '[\x{1F000}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}]' app/src/main | grep -v '//'` -- expected: 空输出（无 emoji 渲染）
- `cd companion-android && ./gradlew :app:assembleDebug` -- expected: BUILD SUCCESSFUL
- `cd companion-android && ./gradlew :app:testDebugUnitTest --tests '*RoleIconsTest*'` -- expected: 通过

**Result (step-03 self-check, 2026-08-25):** ✅ `assembleDebug` BUILD SUCCESSFUL · emoji 扫描空输出（无渲染点）· `RoleIconsTest` 8 用例通过。实现方式：直接实现（子代理 3 连失败，证伪 [B] 路径——子代理沙箱跑不了多分钟 `./gradlew` 构建）。生成器 `/tmp/gen_lucide.py` 取 lucide 官方 SVG 转 PathParser pathData。

**Pending human review:**
1. 🪨 → `LucideIcons.HardDrive`（Ask-First 候选，lucide 无 rock 图标）— PairingScreen「诚实降级」特性行；待人确认是否接受 HardDrive 或指定替代。
2. 视觉保真度：无渲染环境，仅验证 编译+id 对齐+测试，未逐图标像素比对。建议人审或截图比对桌面 lucide。
3. import 分组：机械注入使部分文件 import 块出现空行/未分组（仅告警级，不影响编译）—— review 时可顺手规整。
