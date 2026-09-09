# IME 布局：先诊断、后机制性修复（CAP-7 契约）

> 证据边界（必须遵守）：症状 Confirmed（用户截图 2：输入条与键盘间约数百像素空区）；代码层级 Confirmed（`MainActivity.kt:23-58` enableEdgeToEdge + 外层 Scaffold 默认 padding + `AndroidManifest.xml:19-23` adjustResize + `AppNavHost.kt:235-278` 内层 Scaffold（contentWindowInsets=0，bottomBar=NavigationBar）+ `ChatScreen.kt:125-129` 整屏 `imePadding()`）。**各层 Insets 的精确像素贡献未由运行数据确认（调查 Hypothesis 1，Open/高概率）**——禁止直接宣称「`adjustResize + imePadding()` 双重计算」为已定案根因；仓库既往仅确认嵌套 Scaffold 的 system-bars 顶部双重 inset 并修复过（`spec-companion-android-mobile-ui-fixes.md`），IME 无仪器记录。

## A. 诊断阶段（先行，唯一允许落码的内容）

### A.1 仅 debug 构建记录（release 零开销）

在一次 composer 焦点切换（收起→弹出）前后各记录一轮，字段：

- root 容器 bounds；
- 外层 Scaffold content Box bounds、内层 Scaffold content Box bounds；
- NavigationBar bounds（top/bottom）；
- composer（输入条 Surface）bounds；
- 外层 `padding.calculateBottomPadding()`、内层 `padding.calculateBottomPadding()`；
- `WindowInsets.ime.getBottom()`、`WindowInsets.navigationBars.getBottom()`。

**禁止记录用户输入内容**（仅几何数值）。日志走 `CompanionLog`，tag 独立（如 `Companion/ImeDiag`）便于 adb 过滤。

### A.2 单变量 A/B 验证（逐项独立，一次只改一项）

| # | 变量 | A（现状） | B（对照） | 观察指标 |
| --- | --- | --- | --- | --- |
| 1 | ChatScreen `imePadding` | 保留 | 移除 | 空区变化量 |
| 2 | bottomBar 显示 | 显示 | 键盘弹出时隐藏 | 空区是否 ≈ NavigationBar 高度 |
| 3 | NavigationBar windowInsets | 默认 | 显式消费/置零 | NavigationBar bounds 与 content padding 变化 |
| 4 | Scaffold PaddingValues 消费 | `Box(padding)` 原样 | 显式 consume/传零 | 内层 content bounds 变化 |

每轮 A/B 在同设备、同输入法、同导航模式（手势 + 三键各一轮）下执行；记录结果表（变量、空区像素差、结论）。定案判据：定位到唯一或主要贡献层（复现调查 Hypothesis 1 的 would-confirm / would refute 口径：若所有 inset 仅消费一次而空区来自输入法候选区/窗口尺寸，则另案记录，不强行套用本修复模板）。

### A.3 定案后

- **临时诊断日志必须完全删除**（AGENTS 规则十三）。
- 诊断结论（含数值表）写入实施 story / 本规格 `.decision-log.md` 追加条目，作为 B 阶段依据。

## B. 修复阶段（依据 A 的定案实施）

1. **唯一 IME inset owner**：确定单层负责 `WindowInsets.ime`（优先 ChatScreen 输入区）；其余层不得再重复施加 IME padding；若诊断结论支持，外层 Scaffold 只处理 system bars。
2. **显式处理 Scaffold PaddingValues 消费**：内外层 Scaffold 的 content padding 消费路径显式化（`Modifier.padding` vs `consumeWindowInsets` 的选择以诊断数据为准），禁止同一空间被多层坐标系重复保留。
3. **bottomBar 处理**：键盘显示时不得同时保留整个 app NavigationBar 高度与完整 IME 高度——依实测选择「键盘弹出时隐藏 bottomBar」或「仅应用未消费的剩余 IME inset」；两者择一，禁止叠加。
4. **输入面收敛**：速记条随遮罩退役后，对话 composer 成为唯一底部输入面（离线排队亦经 composer 录入）——IME 修复的验证与 inset owner 判定均以 composer 为准，不为已删除组件保留避让逻辑。
5. **验证矩阵**（复现设备）：
   - 原输入法 + 手势导航：composer 底部与键盘顶部间距 8–12dp，无 bottomBar 高度残留空区；
   - 原输入法 + 三键导航：同上，且三键导航栏本身不被键盘遮挡错位；
   - 键盘收起：布局完整回位（无残留 inset）；
   - 截图对比存档于实施 story。
6. **禁止**：不采用伪修复（如给 composer 加负 margin / 固定高度 hack 抵消）；不预先假设根因而跳过 A 阶段。
