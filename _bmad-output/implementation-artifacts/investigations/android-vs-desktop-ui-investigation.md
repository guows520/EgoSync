# Investigation: Android 版与桌面版前端 UI 差异对比

## Hand-off Brief

1. **What happened.** 探索型调查已完成：两端 UI 为"同源视觉语言、分叉的实现机制与平台形态"——色彩 token 同源（安卓规格明文取自桌面 UX 规范），但图标哲学相反、暗色默认机制不同、能量低色存在一处实质语义分歧（桌面红 vs 安卓灰）。
2. **Where the case stands.** 双端 UI 清单勘察完毕（两个 subagent 结构化 JSON + 主上下文三处复核），差异对照表已交付，结论置信度 High。
3. **What's needed next.** 若需统一两端体验，优先裁决两个语义级分歧：①能量低色（红 vs 灰，涉及"避免负罪感"设计裁决）；②暗色默认（跟随系统 vs 固定 dark）。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A（探索型，无单据）                                                       |
| Date opened      | 2026-08-25                                                                 |
| Status           | Active                                                                     |
| System           | EgoSync monorepo；桌面=egosync-app (Tauri 2 + React 18)；安卓=companion-android (Kotlin + Jetpack Compose) |
| Evidence sources | 源码（theme 配置、组件代码）、构建配置（package.json / tailwind.config.js / build.gradle.kts / libs.versions.toml）、res 资源目录 |

## Problem Statement

用户原始诉求（视为探索目标，非缺陷假设）：

> 请对比安卓版和桌面版，在风格、图标、样式等等前端 UI 方面，有哪些差异

## Evidence Inventory

| Source   | Status                          | Notes     |
| -------- | ------------------------------- | --------- |
| 桌面端技术栈配置 | Available | `egosync-app/package.json`（lucide-react ^0.292.0, tailwindcss ^3.3.5）；`egosync-app/tailwind.config.js:10-33`（字体/颜色/圆角/动效 token 全部走 CSS 变量） |
| 安卓端技术栈配置 | Available | `companion-android/app/build.gradle.kts:42-53`（Compose BOM + material3 + material-icons-core） |
| 桌面端组件源码 | Partial（未读） | `egosync-app/src/components/{butler,chat,common,layout,modals,notifications,onboarding,role,settings}` |
| 安卓端组件源码 | Partial（未读） | `companion-android/.../ui/{briefing,chat,components,dashboard,notify,review,settings,tasks,theme}` |
| 安卓端主题定义 | Partial（未读） | `ui/theme/{Color,Type,Theme}.kt` 存在 |
| 安卓端资源 | Partial（未读） | `res/drawable`、`res/mipmap-anydpi-v26`、`res/values` |
| 设计规范文档 | Available | `_bmad-output/planning-artifacts/ux-design-specification.md`（桌面专用，`:28` 明示 V1 无移动端）；`_bmad-output/implementation-artifacts/spec-companion-android-prototype.md`（安卓原型规格，含色彩 token 与设计裁决） |
| 版本控制 | Available | `git log -- companion-android`：仅 2 个提交，均为 2026-08-25（d88ef85 首建、f63752a 评审修复） |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | 双端主题层精读：桌面 index.css CSS 变量 vs Android Color.kt/Theme.kt/Type.kt | High | Done | 色板同源（hex 一致）；机制不同（CSS 变量 vs colorScheme） |
| 2 | 图标体系：桌面 lucide-react 用法 vs Android material-icons-core 用法 | High | Done | 桌面=70+ lucide 线框、明文禁 emoji；安卓=emoji 为纲、仅 ArrowBack 矢量 |
| 3 | 组件形态对照：同名域（chat/settings/dashboard 等）在两端的视觉实现差异 | High | Done | 见对照表（气泡/卡片/按钮/动效/Markdown 渲染） |
| 4 | 布局与导航模型差异 | Medium | Done | 桌面=TitleBar+64px 图标栏+模态/抽屉；安卓=状态横幅+底部四 Tab+push 二级页 |
| 5 | docs/ 中设计规范文档核对 | Low | Done | docs/ 无 UI 规范；规范在 `_bmad-output/planning-artifacts/ux-design-specification.md`（桌面专用）+ 安卓原型规格 |

## Timeline of Events

| Time | Event | Source | Confidence |
| ---- | ----- | ------ | ---------- |
| ~2026-05 | 桌面 UX 规范定稿，明示"桌面端优先用户（V1无移动端）" | `_bmad-output/planning-artifacts/ux-design-specification.md:28` | Confirmed |
| 2026-08-25 21:31 | companion-android 首次提交：手机伴侣 Compose 高保真前端原型（纯前端+mock） | git d88ef85 | Confirmed |
| 2026-08-25（同日晚） | 三路对抗评审修复提交 | git f63752a | Confirmed |

## Confirmed Findings

### Finding 4: 安卓端色板与桌面端同源——源自桌面 UX 规范后"Material3 化"

**Evidence:** `spec-companion-android-prototype.md:130`（"色彩 token（源自桌面 UX 规范，Material3 化）"）

**Detail:** 安卓规格明文记录：深色 background #0F1117 / surface #1A1B2E / elevated #252638 / 文本 #E8E8ED / 次文本 #9CA3AF；功能色 成功#10B981 信息#3B82F6 警告#F59E0B 错误#EF4444；象限色 Q1红/Q2蓝/Q3琥珀/Q4灰；角色 accent 管家中性#6366F1、工作冷#4F46E5、家庭暖#D97706、学习#7C3AED、健康#059669。两端差异在机制（CSS 变量 vs Compose colorScheme），不在色彩来源。

### Finding 5: 默认主题被安卓端有意反转：dark 为默认

**Evidence:** `spec-companion-android-prototype.md:129`（"已裁决的文档冲突：①默认主题——用户指令明确 dark 默认（覆盖 UX 规范的浅色默认）"）；`:61`（Theme.kt 深色默认 + light 可切）

**Detail:** UX 规范原定浅色默认；安卓规格裁决为 dark 默认。桌面端实际默认待 index.css 勘察确认。

### Finding 6: 安卓端字体策略 = 系统默认字体栈，不引在线字体

**Evidence:** `spec-companion-android-prototype.md:62`（Type.kt "系统默认字体栈（不引在线字体）"）；对照 `egosync-app/tailwind.config.js:11-12`（Inter + Noto Sans SC + JetBrains Mono）

### Finding 7: 安卓端动效预算极小：仅呼吸能量条与页面转场

**Evidence:** `spec-companion-android-prototype.md:27,98,199-200`

**Detail:** 规格 Always 条款限定"动效仅限呼吸能量条与页面转场"；呼吸能量条为全 App 唯一装饰动效（DashboardScreen.kt:277，2.2s alpha 起伏）。桌面端有 tailwindcss-animate 插件与 duration token 体系（tailwind.config.js:29-33）。

### Finding 8: 安卓端图标体系 = emoji 为主 + 极少矢量图标（刻意决策）

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/ui/AppNavHost.kt:47`（TabSpec 注释"emoji 图标：与角色卡图标风格一致"）；`AppNavHost.kt:50-55`（四 Tab 💬📋📊👤 直接渲染 Text）；全代码库唯一矢量图标 = `Icons.AutoMirrored.Filled.ArrowBack`（3 处二级页返回键，如 `ui/review/WeeklyReviewScreen.kt:63`）

**Detail:** material-icons-core 仅 core 包未引 extended；管家头像=30dp 圆形 Box 内 Text("🤵")（ChatScreen.kt:140-148）；大石头🪨、🌙/☀️ 主题、📴 降级等全为 emoji。自绘图形仅两处：周复盘 Canvas 能量柱状图（WeeklyReviewScreen.kt:251-273）、配对扫码模拟取景框+扫掠线（PairingScreen.kt:150-261）。

### Finding 9: 安卓端暗色机制 = 应用内手动切换，dark 默认，不跟随系统、无动态取色

**Evidence:** `companion-android/app/src/main/java/com/egosync/companion/ui/theme/Theme.kt:52-68`（ThemeMode{DARK,LIGHT} 枚举，默认 DARK）；`res/values/themes.xml:3-5`（windowBackground 固定 #0F1117 不随主题）

### Finding 10: 安卓端组件形态 = M3 标准件 + 平面色分层（无阴影）

**Evidence:** `ui/dashboard/DashboardScreen.kt:125-130,198-205`；`ui/chat/ChatScreen.kt:151-167`；`ui/tasks/TasksScreen.kt:128-166`

**Detail:** 卡片 RoundedCornerShape 14-20dp、elevation 默认 0，靠 surface/elevated 色差分层；聊天气泡不对称圆角（尾角 4dp），管家=surfaceVariant / 用户=primaryContainer，流式=文本尾部拼接光标字符 ▍；动效仅呼吸能量条(1100ms)+扫码线(2200ms)+导航淡入淡出(220/180ms)（DashboardScreen.kt:277-284、AppNavHost.kt:82-85）。

### Finding 11: 两端 App 图标同语义（自我+环绕、靛蓝系）但视觉执行不同

**Evidence:** `companion-android/app/src/main/res/drawable/ic_launcher_foreground.xml:9-24`；`egosync-app/src-tauri/icons/icon.png`（512x512，已目视核对）

**Detail:** 桌面 = 六边形徽章 + 双循环箭头 + 中心圆点（靛蓝/淡紫渐变系）；安卓 = 自适应图标，深底 #0F1117（colors.xml:4）上同心圆（中心实心圆 #A5B4FC r16 + 内环 #A5B4FC r27 + 外细环 #6366F1 r34），注释自述"中心圆点(自我)+外环(环绕系统)，呼应 EgoSync 语义"。安卓无 monochrome 层与 round 变体。

### Finding 12: 桌面端 UI 清单要点（subagent 勘察 + 主上下文复核）

**Evidence:** `egosync-app/src/index.css:8-15,47-54`；`App.tsx:49-53,336-345`；`ChatBubble.tsx:320-371`；`roleIcons.ts:8`；`RoleSidebarIcon.tsx:22-26`

**Detail:** ①色板与安卓同源：暗色 #0F1117/#1A1B2E/#252638/#E8E8ED、功能色 #10B981/#3B82F6/#F59E0B/#EF4444、管家 accent #6366F1 与安卓 Color.kt 逐值一致；②字体 Inter+Noto Sans SC 走 Google Fonts CDN + JetBrains Mono；③图标 lucide-react 70+ 种、strokeWidth=2 黑白线框、`roleIcons.ts:8` 明文"不使用彩色 emoji"；④布局=自绘 TitleBar+64px 固定图标栏+视图级 Header+居中 Modal/右抽屉；⑤聊天气泡：用户右侧深色圆角气泡、管家左侧全宽白卡+方头像，正文 ReactMarkdown+prose 渲染、流式等待=BounceDots 三点弹跳；⑥动效丰富（animate-in 语法族+.breathe 3s+.animate-bounce-forever+prefers-reduced-motion 全局守卫）；⑦角色色温机制：--role-accent 按当前视图动态注入（App.tsx:336-345），安卓无此机制（accent 固定按域映射）。

### Finding 13: 实质语义分歧——能量低色：桌面红色 vs 安卓暗淡灰，阈值亦不同

**Evidence:** `egosync-app/src/components/layout/RoleSidebarIcon.tsx:22-26`（≥80 绿/#10B981、≥40 琥珀/#F59E0B、否则红/#EF4444）；`companion-android/.../ui/theme/Color.kt:39-43`（≥70 绿、≥40 琥珀、否则灰/#9CA3AF）

**Detail:** 安卓规格明文裁决采用"暗淡灰（弃 PRD FR-19 红色，避免负罪感），PRD 红色条款记为待清理"（`spec-companion-android-prototype.md:129`）。桌面代码实际仍用红色，且与自身 token 矛盾：`index.css:32` 定义 --energy-low=#9CA3AF（灰）但侧栏硬编码红。即：桌面同时存在 token 与实现两层不一致，安卓按灰裁决执行。高能量阈值桌面 80 / 安卓 70。

### Finding 14: 暗色模式机制与默认值分歧

**Evidence:** `egosync-app/src/App.tsx:49-53`（localStorage('egosync-theme') → 否则 matchMedia prefers-color-scheme 回退）；`companion-android/.../ui/theme/Theme.kt:52-68`（ThemeMode 枚举默认 DARK，不读系统）；`companion-android/app/src/main/res/values/themes.xml:3-5`（windowBackground 固定 #0F1117）

**Detail:** 桌面首启跟随系统主题（多数用户为浅色）；安卓固定 dark 默认、不跟随系统、无动态取色（dynamicColor 未启用），且启动窗口底色恒为暗色。安卓规格 `:129` 明示此为对 UX 规范浅色默认的有意覆盖。

### Finding 15: 图标哲学相反，且安卓侧"一致性"注释与桌面实现不符

**Evidence:** `egosync-app/src/lib/roleIcons.ts:8`（"全部采用 Lucide 黑白线框图标，不使用彩色 emoji"）；`companion-android/.../ui/AppNavHost.kt:47`（"emoji 图标：与角色卡图标风格一致"）；桌面角色卡实际渲染 lucide（RoleSidebarIcon.tsx:28-32）

**Detail:** 桌面禁 emoji、安卓以 emoji 为纲（💬📋📊👤🤵🪨🌙☀️📴，唯一矢量图标=返回箭头）。安卓注释声称 emoji"与角色卡图标风格一致"，但桌面角色卡图标是 lucide 线框而非 emoji——该"一致"仅对 UX 规范的 mock 文案语义成立（spec:134 角色样例带 🎯🏠📚），对桌面实际 UI 不成立。另：安卓聊天正文为纯 Text 渲染（chat 目录 grep 无 Markdown/AnnotatedString），桌面为 ReactMarkdown+prose 全量排版。

### Finding 1: 两端是完全独立的两套 UI 技术栈

**Evidence:** `egosync-app/package.json:21,40`；`companion-android/app/build.gradle.kts:49-53`

**Detail:** 桌面端 = React 18 + TailwindCSS 3.3.5（darkMode:'class'）+ lucide-react ^0.292.0 图标；安卓端 = Jetpack Compose + Material 3（compose BOM 管理）+ material-icons-core。两端无共享 UI 代码。

### Finding 2: 桌面端设计 token 全部经 CSS 变量注入 Tailwind

**Evidence:** `egosync-app/tailwind.config.js:14-33`

**Detail:** 颜色（accent/sidebar/surface/elevated/energy-high/mid/low）、圆角（button/card/dialog/input）、动效时长（fast/normal/color）均为 `var(--…)` 形式，实际值定义在 `index.css`（待读）。角色强调色为运行时变量 `--role-accent`。

### Finding 3: 安卓端启用 Material 3 且图标依赖仅 core 子集

**Evidence:** `companion-android/app/build.gradle.kts:52-53`；`libs.versions.toml`（待读具体版本）

**Detail:** 使用 androidx.compose.material3 与 material-icons-core（非 extended），说明图标集范围受限，可能存在手绘/自绘图标替代。

## Deduced Conclusions

### Deduction 1: 两端 UI 差异属于"平台栈差异 + 各自设计体系"，而非同构移植

**Based on:** Finding 1、2

**Reasoning:** 若为同一设计的跨平台移植，两端应共享 token 值或至少同源色板；桌面端走 CSS 变量体系而安卓端走 Compose MaterialTheme，二者机制完全不同。

**Conclusion:** 对比应按维度展开（色彩/字体/图标/组件形态/布局导航/动效/暗色模式），而非逐文件 diff。（待 Finding 级证据补全后复核）

### Deduction 2: 差异的定性 = 同源视觉语言 + 分叉的实现机制与平台形态 + 两处语义级分歧

**Based on:** Finding 4（色板同源）、Finding 12-15、Hypothesis 2 Resolution

**Reasoning:** 安卓规格明文取用桌面 UX 规范色板并 Material3 化 → 色彩语言同源；但图标哲学（线框 vs emoji）、暗色默认（跟系统 vs 固定 dark）、字体（在线栈 vs 系统栈）、动效预算、组件保真度（Markdown 排版 vs 纯文本）全部分叉；其中能量低色（红 vs 灰）与暗色默认是仅有的两处"语义级"分歧，其余为平台适配性差异。

**Conclusion:** 安卓端并非桌面端的视觉移植，而是"同一设计语言在移动平台的再表达 + 原型级简化"。若追求两端一致，需裁决的只有语义级分歧；若接受平台差异，现状即合理。

## Hypothesized Paths

### Hypothesis 1: 安卓端刻意采用 Material You/Material 3 平台原生风格，与桌面端自定义"能量/管家"风格不同源

**Status:** Refuted

**Theory:** Android 端遵循 M3 动态配色与平台惯例（底部导航等），桌面端是自定义设计语言（sidebar + energy 色彩语义）。

**Supporting indicators:** tailwind.config 出现 energy-* 语义色与 --role-accent；安卓端用标准 material3。

**Would confirm:** Android Theme.kt/Color.kt 中出现 Material 默认色板或 dynamicColor；桌面 index.css 出现自定义 hex 色板。

**Would refute:** Android Color.kt 复刻了桌面端同一组 hex 色。

**Resolution:** `spec-companion-android-prototype.md:130` 明文"色彩 token 源自桌面 UX 规范，Material3 化"，且列出与桌面同源的完整 hex 清单——"不同源"被驳回。保留的合理部分：安卓在组件层确实走 M3 组件体系（平台惯例），差异是"同源色板 + 不同实现机制/组件形态"，非两套独立设计。

### Hypothesis 2: 两端的真实差异集中在"实现机制与平台形态"，而非视觉语言

**Status:** Confirmed

**Theory:** 同一套 UX 规范色板下，两端差异表现为：①token 机制（CSS 变量 vs Compose colorScheme）；②默认主题（待确认桌面端实际值）；③字体（在线字体栈 vs 系统栈）；④图标（lucide 线性图标 vs material-icons-core）；⑤导航（侧栏 vs 底部 Tab+二级页）；⑥动效预算（丰富 transition vs 仅呼吸条+转场）；⑦组件形态（自绘 Tailwind 卡片 vs M3 标准件）。

**Supporting indicators:** Finding 4-7。

**Would confirm:** 双端 UI 清单勘察结果与上述七项一一对应。

**Would refute:** 发现两端存在色值/语义层面的实质性分歧（即视觉语言也不同）。

**Resolution:** 双端清单勘察逐项证实七类机制/形态差异（Finding 12、8-10）。"无实质语义分歧"部分被驳回一处：能量低色桌面红 vs 安卓灰（Finding 13，且安卓为有意裁决）；暗色默认机制分歧亦属语义级（Finding 14）。结论：视觉语言同源，机制/形态全面分叉，语义层有两处已裁决或待裁决的分歧。

## Missing Evidence

原四项缺口已全部闭合（index.css 实值、安卓 theme 三件套、双端组件细节、规范文档均已取得）。剩余非阻塞观察：

| Gap | Impact | How to Obtain |
| --- | ------ | ------------- |
| 安卓浅色主题下的全量视觉走查 | 已知 DegradedOverlay 硬编码暗色 hex（浅色主题下仍强制暗色，DegradedOverlay.kt:65,90,101,127），其余浅色问题未知 | 模拟器切 light 逐页走查 |
| 桌面端 14 套未启用图标候选（src-tauri/icon-variants/）与安卓同心圆图标是否计划统一 | 影响 App 图标一致性决策 | 产品裁决 |

## Source Code Trace

探索型案件：不适用错误溯源表。区域模型见 Confirmed Findings 与 Backlog。

## Conclusion

**Confidence:** High

探索目标已达成，区域心智模型完整。Confirmed：①两端为独立技术栈（Tailwind+CSS 变量+Lucide vs Compose M3+emoji）；②色彩语言同源（安卓规格明文取自桌面 UX 规范并 Material3 化，暗色四级面/功能色/管家 accent 逐 hex 一致）；③图标哲学相反且安卓注释与桌面实现不符；④暗色默认机制分歧（桌面跟随系统、安卓固定 dark）；⑤能量低色存在唯一实质色彩语义分歧（桌面红/阈值 80 vs 安卓灰/阈值 70，安卓为有意裁决且桌面自身 token 与实现矛盾）；⑥字体、动效预算、组件保真度（Markdown 排版、流式指示器、阴影/悬浮）全面分叉，方向均为"桌面富、安卓简"。定性：安卓端是同一设计语言在移动平台的再表达 + 原型级简化，不是移植也不是漂移。

## Recommended Next Steps

### Fix direction

无缺陷需修。若目标是两端体验统一，按机制分两类：

1. **语义级裁决（需要人拍板）**：能量低色红 vs 灰（涉及"避免负罪感"设计原则，桌面自身 token --energy-low=#9CA3AF 与 RoleSidebarIcon 红色实现已互相矛盾，需一并理顺）；暗色默认跟随系统 vs 固定 dark。
2. **注释纠偏（低成本）**：安卓 AppNavHost.kt:47 "与角色卡图标风格一致"注释与桌面 lucide 实现不符，应改为指向 UX 规范 mock 语义或移除，避免误导后续开发者。

### Diagnostic

不适用（无不确定性残留）。可选：安卓浅色主题全量走查（见 Missing Evidence）。

## Reproduction Plan

不适用（探索型）。本调查交付物为差异对照表（见对话记录），全部条目带 path:line 可复核。

## Side Findings

- `_recovery_pre_cr_dirac/` 目录存在，疑似历史恢复快照，与本调查无关但提示仓库经历过一次大改动。
- 桌面端存在 token 双轨制债务：index.css 语义 token 定义完整但组件实际大量硬编码 slate/indigo 类，--radius-input:24px 定义后从未使用（`egosync-app/src/index.css:18-21` vs 组件实际类名）；`src/index.css.test.ts` 将 CSS 变量视为受控契约。
- 桌面字体依赖 Google Fonts CDN（`egosync-app/index.html:7`），离线回退系统字体；安卓明确不引在线字体——两端策略相反但各有道理。
- 桌面 `src-tauri/icon-variants/` 存在 14 套未启用候选图标。
- 安卓 DegradedOverlay 硬编码暗色 hex，浅色主题下降级遮罩仍为暗色（DegradedOverlay.kt:65,90,101,127）。
- 安卓界面文案全部硬编码中文于 Compose 代码内，strings.xml 仅 app_name（无 i18n 预留）。
