# Investigation: web 版手机浏览器五个自适应症状——新增角色模态裁切、建议卡片按钮纵排、通知入口重复、LLM 卡片按钮挤压、切换视图滚动动效

## Hand-off Brief

1. **What happened.** 用户报告 web 版手机浏览器（<768px）五处自适应问题（2026-09-26）：①新增角色模态标题被裁切且不可滚动；②管家建议卡片「拒绝/确认」小屏纵排全宽；③移动设置「通知」行与「调度时间」行落点重复；④LLM Provider 配置卡片操作按钮在小屏被挤压成纵向（MCP 卡同族）；⑤角色切换回管家后对话区有「拉到最底」的平滑滚动动效。
2. **Where the case stands.** 五症状代码级根因全部 **Confirmed**（详见 Findings 1–12）：①共享 Modal（`Modal.tsx:53-61`）缺高度/滚动处理；②③为 16.4/Epic 16 冻结设计的显式变更；④为 flex squeeze 真缺陷（LLM+MCP 同族）；⑤为 `scroll-smooth` 把挂载贴底动画化。**owner 三项裁决已完成（2026-09-26）：授权解冻②③测试/规格钉孔、①走共享 Modal 修复、确认④「4 个图标」映射**。
3. **What's needed next.** 已实施完成（2026-09-26，按 `spec-web-mobile-adaptive-fixes.md`，bmad-build oneshot）：五项修复全部落地，验证全绿（第三处通知钉孔 `App.mobile.test.tsx` 经 owner 追加授权后同型改写）。剩余：桌面 e2e 两 spec 因 tauri-driver 未起环境阻断（deferred-work D3 同族，非本卷回归）；iOS Safari 真机走查（①模态滚动/④图标钮 ≥44px）留人工。

## Case Info

| Field            | Value                                                                  |
| ---------------- | ---------------------------------------------------------------------- |
| Ticket           | N/A（用户口述 2026-09-26，web 版手机浏览器自适应五症状）                 |
| Date opened      | 2026-09-26                                                             |
| Status           | Closed（2026-09-26：调查收口 + owner 三项裁决 + 追加授权；①②③④⑤实施完成并经三层评审 patch 收口；开放项：桌面 e2e 环境阻断（D3）、TaskModal/iOS 真机走查、F10/F12 两项——均见 deferred-work.md） |
| System           | 手机浏览器访问 EgoSync web 版（项目移动基线口径 375×812，<768px 断点；宿主=浏览器）。注：症状②–⑤ 经用户更正为独立流程，与「新增角色」无因果关系 |
| Evidence sources | 前端源码（Modal/AddRoleModal/ActionCard/MobileSettingsView/GlobalSettingsModal/ChatStream/App.tsx）、Epic 16 上下文与 16.4 规格、既有 vitest/e2e 钉孔、git 提交史（204af71/ceea15b/9af04b6） |

## Problem Statement

用户原话（2026-09-26；2026-09-26 中段更正：②–⑤ 为独立流程，非新增角色流程的一部分）：

- **新增角色**
  - 标题栏被遮盖了，显示不全，而且界面不能上下滑动
- **（独立）管家的建议卡片**
  - 「拒绝」和「确认」，一个显示一行太大了，放在一行显示即可
- **（独立）移动设置列表**
  - 通知 配置跟调度时间重复了，而且桌面版中也没有通知的配置项，移除
- **（独立）全局设置-模型服务**
  - LLM Provider 配置的卡片上「当前启用」「测试连接」等按钮显示成纵向了，显示不下就更换成 4 个图标即可，查看一下其它配置项有没有类似问题
- **（独立）视图切换**
  - 从角色切换到管家后，如果对话框显示高度超过屏幕，会有明显的动效拉到最低部，移除这个动效，直接显示最底部即可

## Evidence Inventory

| Source                | Status    | Notes                                                                                              |
| --------------------- | --------- | -------------------------------------------------------------------------------------------------- |
| 前端源码（8 个组件）   | Available | 五症状涉及代码全部直读定位（见 Confirmed Findings）                                                     |
| Epic 16 上下文/16.4 规格 | Available | epic-16-context.md:45（ActionCard 全宽堆叠冻结）、spec-16-4-mobile-web-form-and-pwa.md:31,84          |
| 既有测试钉孔          | Available | ActionCard.mobile.test.tsx:50-63（纵排钉死）、Modal.safearea.test.tsx:24-27（遮罩类钉死）、web-mobile.spec.ts:348-355（通知行落点） |
| git 提交史            | Available | 204af71（16.4 移动形态）、ceea15b（前案四处布局修复）、9af04b6（tab 去重）——确认冻结设计 lineage        |
| 用户机型/视口宽度     | Missing   | 具体机型与视口高度未知；375px 项目基线口径不受影响，但症状①是否裁切取决于内容高度 vs 视口高度         |
| 复现截图/录屏         | Missing   | 无；代码证据已闭合，截图仅能佐证无法改变结论                                                         |

## Investigation Backlog

| # | Path to Explore                                          | Priority | Status | Notes                                   |
| - | -------------------------------------------------------- | -------- | ------ | --------------------------------------- |
| 1 | 共享 Modal 的高度/滚动处理 vs 各使用方自救模式              | High     | Done    | Finding 1/2/3                            |
| 2 | ActionCard 按钮容器小屏类与冻结 lineage                   | High     | Done    | Finding 4 + 冻结 triad（规格/注释/测试）  |
| 3 | MobileSettingsView 通知行落点 vs 桌面 nav                  | High     | Done    | Finding 5/6                              |
| 4 | GlobalSettingsModal LLM/MCP 卡片头 flex  squeeze 分析      | High     | Done    | Finding 7/8 + 其它配置项扫描（Finding 9） |
| 5 | ChatStream 自动滚动 + scroll-smooth 机制与重挂载路径        | High     | Done    | Finding 10/11/12                         |
| 6 | 受修复影响的既有测试/e2e 钉孔清点                          | High     | Done    | 见「Recommended Next Steps」测试影响面    |
| 7 | 实施①：共享 Modal 补 max-md 高度/滚动（含 TaskModal 受益面）  | High     | Done    | owner 裁决：共享修复（方案 A）；Modal.tsx:61 + Modal.scroll.test.tsx（新） |
| 8 | 实施②③：ActionCard 一行排 + 删通知行（含解冻钉孔改写）       | High     | Done    | owner 裁决：授权解冻（ActionCard.mobile.test.tsx:50-63、web-mobile.spec.ts:348-355、spec-16-4:31）；MobileSettingsView.tsx + MobileSettingsView.rows.test.tsx（新）+ spec-16-4 Spec Change Log |
| 9 | 实施④：LLM+MCP 卡片 max-md 图标钮（4 图标映射已确认）       | High     | Done    | owner 裁决：徽章图标 + 测试连接/编辑/删除 3 图标钮，≥44px 触控；GlobalSettingsModal.tsx 两卡 + GlobalSettingsModal.mobileIcons.test.tsx（新） |
| 10 | 实施⑤：ChatStream 消除切换视图的贴底动效（终态路径 b）       | High     | Done    | ChatStream.tsx 保留 scroll-smooth + :881-886 赋值临时置 auto 再恢复；新建 ChatStream.scroll.test.tsx；初版路径 a 被评审证伪（F4） |
| 11 | 验证：受影响组件 vitest + 375px web e2e 走查 + 真机确认       | High     | Done    | vitest 受影响四文件 7 例 + 全量 84 文件（App.mobile.test.tsx 第三处通知钉孔经 owner 追加授权同型改写后全绿）、build tsc 零错误、web-mobile.spec.ts 11/11；桌面 e2e 环境阻断（D3 同族）、iOS 真机走查留人工 |

## Timeline of Events

| Time           | Event                                                                                  | Source                                          | Confidence |
| -------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------- | ---------- |
| Story 16.2     | 响应式基线：模态 375px 统一宽度 `max-md:w-[calc(100%-2rem)]`（不含高度/滚动处理）          | Modal.tsx:7-14                                  | Confirmed  |
| Epic 16 上下文  | 五大核心面小屏形态定稿，含「ActionCard 全宽堆叠」                                        | epic-16-context.md:45                           | Confirmed  |
| 2026-09-25     | 人工裁决：移动设置 tab = 桌面同款全量 7 入口（含「通知」行=调度 tab 别名）                  | spec-16-4-mobile-web-form-and-pwa.md:31         | Confirmed  |
| 2026-09-25     | 204af71：16.4 移动形态落地（ActionCard 小屏纵排、Modal 安全区内边距）                     | git log                                         | Confirmed  |
| 2026-09-26     | ceea15b：前案四处布局修复（tab 全屏化/铃铛右对齐/⋯归位/设置模态去切换行）                  | git log                                         | Confirmed  |
| 2026-09-26     | 用户口述五症状（含一次范围更正：②–⑤ 为独立流程）；本卷开立                                | 用户消息                                        | Confirmed  |
| 2026-09-26     | spec-web-mobile-adaptive-fixes.md 立卷（oneshot）；owner 批准                                 | 用户消息（Approve and continue）                 | Confirmed  |
| 2026-09-26     | 调查完成 + owner 三项裁决（②③授权解冻 / ①共享修复 / ④确认「4 个图标」映射）                | 用户消息                                        | Confirmed  |
| 2026-09-26     | 实施完成；全量 vitest 红 1（第三处通知钉孔在授权清单外）→ 停改报备                        | 父代理实测                                       | Confirmed  |
| 2026-09-26     | owner 追加授权 App.mobile.test.tsx 同型改写；全量 84 文件 954 例转绿                        | 用户消息 + 父代理实测                            | Confirmed  |
| 2026-09-26     | 三层评审（盲扫 N=10/边界/验证缺口）24 条发现 triage；patch 全量落地（含 ④ max-md: 前缀化、② flex-wrap、⑤ 路径 a→b） | spec-web-mobile-adaptive-fixes.md Review Triage Log | Confirmed  |

## Confirmed Findings

### Finding 1: 共享 Modal 无高度上限与滚动容器——超高内容上下裁切且不可滚动

**Evidence:** `egosync-app/src/components/layout/Modal.tsx:53-61`

**Detail:** 遮罩为 `fixed inset-0 z-50 flex items-center justify-center`（含安全区 pt/pb），对话框为 `overflow-hidden relative z-10 animate-in zoom-in-95`——既无 `max-h-*`，也无任何 `overflow-y-auto` 滚动容器。当内容高度超过视口（减去安全区内边距）时，flex 垂直居中把对话框顶出视口上沿（标题区不可见），`overflow-hidden` 又切断一切滚动可能——与症状①「标题栏被遮盖、显示不全、界面不能上下滑动」逐条吻合。这是经典的 flex 居中 + 溢出内容不可达问题。

### Finding 2: 同项目其它模态各自带 max-h+overflow-y-auto 自救，AddRoleModal 是唯一裸奔者

**Evidence:** `egosync-app/src/components/modals/WeeklyReviewModal.tsx:120-121`、`egosync-app/src/components/butler/ButlerSettingsContent.tsx:1312`、`egosync-app/src/components/onboarding/RoleConfirmModal.tsx:106` vs `egosync-app/src/components/modals/AddRoleModal.tsx:47-117`、`egosync-app/src/components/modals/TaskModal.tsx:151-152`

**Detail:** WeeklyReviewModal（`max-h-[80vh] overflow-y-auto`）、ButlerSettingsContent（`max-h-[85vh] overflow-y-auto`）、RoleConfirmModal（`max-h-[90vh] overflow-y-auto`）均自行补齐高度/滚动。AddRoleModal 与 TaskModal 直接用裸共享 Modal。AddRoleModal 内容（p-6 + 标题行 + 名称输入 + 3 行目标域 + 24 图标 6 列网格（4 行 ≈190px）+ 8 色行 + 操作钮）≈600–650px，375×667 级视口（或扣除浏览器 chrome 后的 375×812）即超视口——症状①因此只在移动端发作，桌面窗口高度下不发作。

### Finding 3: 遮罩安全区类被既有测试钉死，修复不得破坏

**Evidence:** `egosync-app/src/components/layout/Modal.safearea.test.tsx:24-27`

**Detail:** 测试断言遮罩（`.fixed`）必须带 `pt-[max(0px,env(safe-area-inset-top))]` / `pb-[max(0px,env(safe-area-inset-bottom))]`。任何对遮罩类的增改须保持这两个类在场（`toHaveClass` 只查在场、允许多类，但删除/替换即红）。

### Finding 4: ActionCard 小屏纵排是冻结设计，非缺陷

**Evidence:** `egosync-app/src/components/butler/ActionCard.tsx:200-219`（容器 `flex max-md:flex-col gap-2 mt-3 max-md:justify-stretch justify-end`；按钮 `max-md:min-h-[44px] max-md:flex-1`）+ `egosync-app/src/components/butler/ActionCard.mobile.test.tsx:50-63`（钉 `max-md:flex-col`/`max-md:justify-stretch`）+ `_bmad-output/implementation-artifacts/epic-16-context.md:45`（「ActionCard 全宽堆叠」列入五大核心面小屏形态）+ `spec-16-4-mobile-web-form-and-pwa.md:84`

**Detail:** 四重证据（代码注释自述引线框、单测钉孔、Epic 16 上下文、16.4 规格）一致表明「拒绝/确认小屏全宽纵排」是 2026-09-25 定稿的有意设计。375px 下两个按钮各占一行，视觉权重过大——用户现在要求改一行排。修复=设计变更 + 解冻单测，须 owner 授权。

### Finding 5: 移动设置「通知」行与「调度时间」行落点完全相同

**Evidence:** `egosync-app/src/components/settings/MobileSettingsView.tsx:53-60`

**Detail:** `{ key: 'scheduler', label: '调度时间', onClick: () => onOpenSettings('scheduler') }`（56 行）与 `{ key: 'notification', label: '通知', onClick: () => onOpenSettings('scheduler') }`（59 行）——两行打开同一个 scheduler tab（敲门通知声音设置所在，`GlobalSettingsModal.tsx:1039-1068`）。同视图出现两个文案不同、落点相同的入口，即用户所述「重复」。

### Finding 6: 桌面 GlobalSettingsModal 无「通知」入口——移动行是移动独有别名

**Evidence:** `egosync-app/src/components/settings/GlobalSettingsModal.tsx:658-661`（nav = 模型服务配置/MCP Server/调度时间/数据与隐私 + capabilities 门控的远程模式/手机伴侣）

**Detail:** 桌面导航四个常驻项无「通知」。移动「通知」行是 16.4 规格（spec-16-4:31 列举 7 入口含通知）为「桌面同款全量」主张添加的别名入口——但桌面实际并无该项，别名的存在反而破坏了「同款全量」口径，且落点重复。用户「桌面版中也没有通知的配置项」的陈述与代码一致，移除后移动设置 = 4 内容入口，敲门声音经「调度时间」仍可达（无功能损失）。

### Finding 7: LLM Provider 卡片头 flex 两侧组均无 shrink 防护——375px 下按钮文字换行纵排

**Evidence:** `egosync-app/src/components/settings/GlobalSettingsModal.tsx:713-727`

**Detail:** 卡片头 `flex items-center justify-between`：左组 = radio(16px)+gap+名称+「当前启用」徽章（无 `min-w-0`、无 `shrink-0`）；右组 = [测试连接][编辑][删除] 三个文字钮（无 `shrink-0`、无 `whitespace-nowrap`）。375px 视口下模态内可用宽 ≈375−16(模态 p-4)−16(卡片 p-4)≈343px…（模态 `p-4`、卡片 `p-4`，实际 ≈311–343px）；左组 ≈16+12+名称(60–100)+12+徽章(≈52)≈150–200px，右组 ≈76+56+56+2×8≈204px，合计超可用宽 → flex 默认 shrink 均分压力 → 按钮文字逐字换行（「测试连接」折成多行）、徽章「当前启用」同样换行——即用户所见「显示成纵向」。

### Finding 8: MCP Server 卡片同族问题（右簇 shrink-0 挤压左列，失败模式不同）

**Evidence:** `egosync-app/src/components/settings/GlobalSettingsModal.tsx:843-885`

**Detail:** 卡片头 `flex items-start justify-between gap-4`：左组 `min-w-0`（文字可换行收缩）；右簇 `shrink-0` = [开关 44px][测试连接 ≈76px][编辑 ≈56px][删除 ≈56px]+gap×3 ≈256px，永不被压缩。375px 下左列仅剩 ≈311−256−16≈39px——名称/URL/描述被压成极窄竖条文本。与症状④同族（固定宽动作簇 vs 窄视口），但症状⑦是「按钮换行」、此处是「文本被挤瘪」。

### Finding 9: 其它配置项同类扫描结果

**Evidence:** `egosync-app/src/components/settings/GlobalSettingsModal.tsx`（scheduler 966-1068、data 1072-1403、remote 1416-1418、companion 1410-1412）；`RemoteModeSection.tsx`、`CompanionPairingSection.tsx`（单独组件，仅浏览签名）

**Detail:** 逐区扫描结论：
- 调度时间区：时间片 chip `flex-wrap`（990 行）无横溢；「敲门通知声音」行（1040-1067）左文本块无 min-w-0 + 右开关 shrink-0——375px 下文本换行多行、开关完整，可用但不优雅（同族低危）；
- LLM 编辑表单「模型名称 + 获取模型列表」行（777-782）：输入 flex-1 + 按钮 shrink-0 且 `whitespace-nowrap`，按钮 ≈96px，输入剩 ≈200px——可接受，非缺陷；
- 数据与隐私区：块级按钮（1105/1222/1328 行「导出存档/导入存档/选择导出包」均为独占行）——无横排挤压；
- 远程模式/手机伴侣为门控分区（浏览器宿主多不渲染）且为独立组件，未发现同族结构。

### Finding 10: ChatStream 滚动容器带 scroll-smooth，程序化跳转被平滑动画化

**Evidence:** `egosync-app/src/components/chat/ChatStream.tsx:1354`（`className="flex-1 overflow-y-auto p-8 scroll-smooth"`）+ `:881-886`（`scrollRef.current.scrollTop = scrollRef.current.scrollHeight`）

**Detail:** 自动贴底 effect 在 `[messages, pendingScrollMessageId, streamBubbles, streamStatus, thinkingContent]` 变化时执行 `scrollTop = scrollHeight`。容器携带 Tailwind `scroll-smooth`（CSS `scroll-behavior: smooth`）——**一切** scrollTop 程序化赋值都被浏览器转为平滑动画。

### Finding 11: 视图切换导致 ButlerView/ChatStream 重挂载，scrollTop 从 0 起步

**Evidence:** `egosync-app/src/App.tsx:529-531`（`currentView === 'butler'` 才渲染 ButlerView）+ `:552-554`（角色 id 匹配才渲染 RoleView）

**Detail:** 两个视图互斥条件渲染，从角色切回管家 = RoleView 卸载、ButlerView 全新建载 → 内部 ChatStream 新 DOM 节点 `scrollTop=0` → 历史加载完成后 effect 执行 `scrollTop=scrollHeight`。对话高度超过视口时，平滑动画从顶部全程滑到底部——即「明显的动效拉到最低部」。对话短于视口时位移小、动画不可察——症状⑤与「对话框高度超过屏幕」强相关，与用户描述一致。

### Finding 12: 无测试钉死 scroll-smooth 类或自动滚动的平滑性

**Evidence:** `egosync-app/src/components/chat/ChatStream.test.tsx:983-1022`（仅钉来源导航 `scrollIntoView({behavior:'smooth'})` 显式参数与手工 `scrollTop=1050` 计算）；jsdom 不应用 CSS `scroll-behavior`

**Detail:** 来源消息居中定位传显式 `behavior:'smooth'`（`ChatStream.tsx:238`），与容器 CSS 类解耦——移除/调整 `scroll-smooth` 类不影响该断言。移除类后唯一的副作用是聊天区鼠标滚轮滚动由平滑变即时（浏览器默认行为）。

## Deduced Conclusions

### Deduction 1: 症状①是共享组件的覆盖缺口，不是 AddRoleModal 一家的错

**Based on:** Finding 1/2/3

**Reasoning:** 三个同项目模态各自补 max-h+overflow-y-auto，证明「滚动自救」是既有惯例；共享 Modal 从 16.2 起只处理了宽度（`max-md:w-[calc(100%-2rem)]`）与安全区，从未处理高度。AddRoleModal/TaskModal 因未自救而暴露。

**Conclusion:** 首选在共享 Modal 补高度/滚动（一次性覆盖全部使用方），次选仅给 AddRoleModal 打补丁（同 WeeklyReviewModal 惯例）。两条路桌面均零视觉变化。

### Deduction 2: 症状②③是冻结设计的显式变更，须 owner 授权并同步规格/测试

**Based on:** Finding 4/5/6

**Reasoning:** ②由 Epic 16 上下文 + 16.4 规格 + 单测三重冻结；③由 16.4 规格列举 + e2e 钉死落点。二者都不是代码缺陷，是设计口径变更。

**Conclusion:** ②③修复必须附带：规格文档变更记录 + 单测/e2e 钉孔改写（owner 显式解冻，参考前案先例：web-mobile-layout-issues-investigation.md Follow-up 2026-09-25 的授权模式）。

### Deduction 3: 症状④按用户方向修图标化后，MCP 卡应同批处理（同族不同症）

**Based on:** Finding 7/8/9

**Reasoning:** LLM 卡（按钮换行）与 MCP 卡（左列挤瘪）同根于「固定宽动作簇 vs 窄视口」；只修 LLM 卡会留下 MCP 卡的次生问题，与用户「查看一下其它配置项有没有类似问题」的指令正相反。

**Conclusion:** ④的修复面 = LLM 卡片 + MCP 卡片（同批、同模式）；调度区敲门声音行列为低危可选。

### Deduction 4: 症状⑤移除动效有两条等价技术路径，均零测试影响

**Based on:** Finding 10/11/12

**Reasoning:** 路径 a=直接删容器 `scroll-smooth` 类（程序化跳转即时化；副作用=滚轮平滑滚动消失，回归浏览器默认）；路径 b=保留类，仅 effect 内赋值前临时置 `scrollBehavior='auto'`、赋值后恢复（程序化即时、滚轮仍平滑，代码稍脏）。

**Conclusion:** 推荐路径 a（与「直接显示最底部即可」的口径一致、实现最简）；若用户在意滚轮平滑可用 b。

## Hypothesized Paths

### Hypothesis 1: 用户在 ≥768px 窄桌面窗口/折叠屏访问，症状与断点口径不符

**Status:** Refuted

**Theory:** 若视口 ≥768px，②（`max-md:flex-col` 不生效，按钮本就一行右对齐）、③（移动设置视图不渲染）、④⑦（桌面模态左侧 nav 纵列+卡片宽 ≈640px+，无挤压）都不成立。

**Supporting indicators:** 用户未提供机型/宽度。

**Would confirm:** 用户提供机型/宽度 ≥768px。

**Would refute:** 症状②「一个显示一行」只在 `max-md:flex-col` 分支成立；③只在移动设置 tab 成立；④只在窄模态成立；⑤的裁切只在窄视口+高内容成立——四个症状全部只在 <768px 分支产生。

**Resolution:** 用户对②③④的描述均为小屏分支独有现象，与项目 375px 基线口径一致；宿主分支无需再议。

### Hypothesis 2: 症状①裁切取决于具体机型视口高度，部分机型可能不复现

**Status:** Open（不影响修复方向）

**Theory:** AddRoleModal 内容 ≈600–650px（Finding 2 估算）。视口可用高度 ≥650px 的机型不裁切；375×667 及扣除浏览器 chrome 后更矮的视口必裁切。

**Supporting indicators:** 用户实机报告裁切=其视口可用高度 < 内容高度；缺失项=具体机型/视口高度。

**Would confirm:** 用户提供机型或视口高度；或修复后在 375×667 模拟器复现/在 375×812 不复现。

**Would refute:** 同内容在任意 375px 宽机型均裁切（说明还叠加了宽度导致的换行增高）。

**Resolution:** 待用户补充或修复后真机走查。修复本身（补高度/滚动）与机型无关，方向不受影响。

## Missing Evidence

| Gap                    | Impact                                              | How to Obtain                          |
| ---------------------- | --------------------------------------------------- | -------------------------------------- |
| 用户机型/视口高度       | 仅影响症状①的复现精度（Finding 2 高度估算佐证），不影响根因与修复方向 | 用户补充或修复后真机走查               |
| 修复后的实机走查         | 五项修复的视觉确认（尤其①滚动区标题可达性、④图标按钮触控 ≥44px） | 修复后 web e2e + 真机验证               |

## Source Code Trace

| Element       | Detail                                                                                              |
| ------------- | --------------------------------------------------------------------------------------------------- |
| Error origin  | ① Modal.tsx:53-61（遮罩 flex 居中+对话框 overflow-hidden，无 max-h/滚动）；② ActionCard.tsx:203-217；③ MobileSettingsView.tsx:56,59；④ GlobalSettingsModal.tsx:713-727（LLM）、843-885（MCP）；⑤ ChatStream.tsx:1354 + 881-886 + App.tsx:529-554（重挂载） |
| Trigger       | ①小屏打开新增角色模态（内容超高）；②小屏管家对话区出现建议/敲门卡片；③小屏设置 tab 浏览列表；④小屏设置→模型服务；⑤小屏角色→管家视图切换 |
| Condition     | 视口 <768px（max-md/md:hidden 分支），宿主=浏览器；⑤另需对话高度 > 视口高度                        |
| Related files | AddRoleModal.tsx、TaskModal.tsx、WeeklyReviewModal.tsx、ButlerSettingsContent.tsx、RoleConfirmModal.tsx、ActionCard.mobile.test.tsx、Modal.safearea.test.tsx、epic-16-context.md:45、spec-16-4-mobile-web-form-and-pwa.md:31,84、web-mobile.spec.ts:348-355、web-resident-loop.spec.ts:271、ChatStream.test.tsx:983-1022 |

## Conclusion

**Confidence:** High

五症状代码级根因全部 Confirmed。分类定性：
- **症状① = 共享组件覆盖缺口（真缺陷）**：Modal 只处理宽度不处理高度，AddRoleModal/TaskModal 裸用中招；同项目其它模态的自救模式证明缺口存在。
- **症状② = 冻结设计的显式变更**：ActionCard 小屏纵排由 Epic 16 上下文 + 16.4 规格 + 单测三重冻结；用户要求改一行排 = 设计口径变更，须 owner 授权并同步规格与测试。
- **症状③ = 冗余入口（真缺陷/规格余量）**：通知行与调度时间行落点完全相同且桌面无对应项；移除后功能零损失（敲门声音经调度时间可达），但规格 16.4:31 明列该行，须规格变更记录 + e2e 钉孔改写。
- **症状④ = 布局 squeeze（真缺陷）**：LLM 卡片头无 shrink 防护导致按钮换行；MCP 卡片同族反模式（右簇挤压左列）；用户提议的图标化方向与 AC2 触控 ≥44px 红线兼容（图标钮仍可保 44px）。
- **症状⑤ = 动效过当（设计微调）**：scroll-smooth 把挂载即贴底的程序化跳转动画化，长对话下全程可见；移除动效后「直接显示最底部」，零测试影响。

无未知代码路径；无影响结论的环境证据缺口（症状①复现精度依赖机型高度，见 H2/缺失证据）。

## Recommended Next Steps

### Fix direction

**① 新增角色模态：补高度上限 + 滚动（二选一，均桌面零变化）**
- 方案 A（共享 Modal，推荐）：`Modal.tsx:61` 对话框类追加 `max-md:max-h-full max-md:overflow-y-auto`——移动端对话框高度不超过遮罩内容盒（已含安全区内边距），超高内容对话框内滚动，标题可达；桌面无 max-md 类不触发、行为零变化。遮罩 pt/pb 安全区类保持不动（Modal.safearea.test.tsx 钉死）。一次性覆盖全部 8 个使用方（含 TaskModal 潜在同症）。
- 方案 B（仅 AddRoleModal）：`AddRoleModal.tsx:49` 内容 div 加 `max-h-[85vh] overflow-y-auto`（同 WeeklyReviewModal:121 惯例），改动面最小；TaskModal 潜在同症遗留（见 Side Findings）。
- 可选增强：模态标题行 `sticky top-0` + 背景不透明，滚动时标题常驻（两方案均适用）。

**② 建议卡片按钮一行排（需解冻 ActionCard.mobile.test.tsx:50-63 + 规格变更记录）**
- `ActionCard.tsx:203`：容器 `flex max-md:flex-col gap-2 mt-3 max-md:justify-stretch justify-end` → `flex flex-wrap justify-end gap-2 mt-3`；按钮去掉 `max-md:flex-1`（不再全宽），保留 `max-md:min-h-[44px]`（触控红线不破）。
- 375px 下两钮右对齐一行（合计 ≈140px，宽裕）；桌面 `justify-end` 与今天逐像素一致。
- 测试改写：ActionCard.mobile.test.tsx 第二个用例（50-63 行）按新类断言（新建断言或经 owner 授权改写既有断言）；web-resident-loop.spec.ts:271 只断言「拒绝」文本在场，不受影响。

**③ 移除移动设置「通知」行（需规格变更记录 + e2e 钉孔改写）**
- `MobileSettingsView.tsx`：删 58-59 行 notification 行；Bell import 如无他用同步删。
- `web-mobile.spec.ts:348-355`：删除「通知入口落点=调度 tab」断言块（owner 授权）；e2e 无其它引用（grep 全库仅此一处 + MobileSettingsView.browser.test.tsx 未钉该行）。
- 敲门口径的可达性不变（调度时间行 →  scheduler tab 内「敲门通知声音」，GlobalSettingsModal.tsx:1039-1068）。

**④ LLM/MCP 卡片按钮图标化（按用户「4 个图标」方向；仅 <768px，桌面零变化）**
- LLM 卡（`GlobalSettingsModal.tsx:714-727`）：卡片头左组加 `min-w-0`；右组三个文字钮在 `max-md:` 下切换为图标钮（测试连接→Plug/Zap、编辑→Pencil、删除→Trash2），图标钮 `max-md:h-11 max-md:w-11`（保 44px 触控）+ `aria-label` + `title` 保留可达性；「当前启用」徽章保留文字（小徽章不挤，亦可缩为 Check 图标徽章——即用户所说「4 个图标」：徽章+3 动作，具体映射待 owner 确认）。
- MCP 卡（`:860-884`）：同批处理——右簇 [开关][测试连接][编辑][删除] 中三个文字钮 `max-md:` 图标化（开关已是紧凑 toggle 不动）；左列 `min-w-0` 已有，图标化后右簇 ≈44+44×3+gap×3≈180px，左列恢复 ≈130px 可用宽。
- 实现范式：文字与图标双渲染 + `max-md:hidden`/`md:hidden` 切换对（项目既有 red line「改动只加 max-md:/md: 类对」）；或条件渲染（单一 class 来源，但桌面走同一 DOM——双渲染更贴红线）。
- 测试影响：GlobalSettingsModal*.test.tsx 均按名称/role 点击（jsdom 不应用 Tailwind 类，文字钮 DOM 在场即原样通过）；e2e web-mobile.spec.ts 未点名这三个按钮；只需新增小屏图标钮 aria-label 的钉孔（新测试文件）。

**⑤ 移除切换视图的贴底动效（零测试影响）**
- 路径 a（推荐）：删 `ChatStream.tsx:1354` 容器的 `scroll-smooth` 类——effect `scrollTop=scrollHeight` 即时落底；副作用=聊天区滚轮滚动由平滑变即时（浏览器默认，可接受）。
- 路径 b（保滚轮平滑）：保留类，`:881-886` effect 内赋值前 `scrollRef.current.style.scrollBehavior='auto'`、赋值后 `requestAnimationFrame` 恢复 `''`。
- 来源消息居中定位（`:238` 显式 `behavior:'smooth'`）不受影响；ChatStream.test.tsx:1016 断言不依赖 CSS 类。

### Diagnostic

- ①修复后须 375px 视口验证：模态开→标题可见/可滚→字段可达→创建成功往返；TaskModal（新建任务长表单）同路径走查。
- ④修复后验证：375px 下 LLM/MCP 卡片头不换行不挤压、图标钮 ≥44px、aria-label 可读（屏幕阅读器）；其它配置区（调度/数据）回归无横溢（e2e `collectHorizontalOverflow` 已有范式）。
- 五项均建议 `tests/e2e/web-specs/web-mobile.spec.ts` 单跑回归（全量套件已知 setWindowSize 负载抖动面，逐 spec 单跑兜底——前案结论沿用）。
- ②③的解冻授权建议 owner 一次性书面授予（范围：ActionCard.mobile.test.tsx 既有断言、web-mobile.spec.ts 通知落点块、规格文档对应行），实施时同步留变更记录。

## Reproduction Plan

1. 手机浏览器（或 DevTools 375×812 / 375×667 设备模拟）登录 web 版。
2. 底部 tab「角色」→ 角色详情「⋯」→ 新建角色 → 预期旧行为：模态标题被裁切、无法滚动（现象①）；同时观察管家对话区建议/敲门卡片 → 预期旧行为：拒绝/确认各占一行全宽（现象②）。
3. 底部 tab「设置」→ 预期旧行为：「通知」与「调度时间」两行并存、落点相同（现象③）。
4. 设置 → 模型服务 → 预期旧行为：LLM 卡片「当前启用」「测试连接/编辑/删除」换行纵排；MCP 卡片左列文本被挤窄（现象④同族）。
5. 底部 tab「角色」进任一角色（对话历史超过一屏）→ 回底部 tab「管家」→ 预期旧行为：对话区从顶部平滑滑动到底部（现象⑤）。
6. 五项修复后按相同路径重走，预期：模态标题可达可滚、按钮一行右对齐、无「通知」行、卡片钮图标化不挤压、切换后直接呈现底部无滑动动画。

## Side Findings

- **TaskModal 同症潜伏**：`TaskModal.tsx:151` 裸用共享 Modal，表单字段多于 AddRoleModal（归属/标题/四象限/优先级/双日期/备注），小屏超高内容大概率同样裁切——方案 A 可一并覆盖，方案 B 则遗留。建议无论选哪案都在修复后走查一次新建/编辑任务。
- **调度区「敲门通知声音」行低危同族**：`GlobalSettingsModal.tsx:1040-1067` 左文本块无 min-w-0，375px 下换行多行但开关完整——可用不优雅；可顺手补 `min-w-0`+`flex-wrap`（零行为变化）。
- **通知行 Removal 的规格连锁**：spec-16-4-mobile-web-form-and-pwa.md:31 明列 7 入口含「通知」——删除行时须在该规格留变更记录（前案先例：Spec Change Log 模式）；否则文档与实现脱节。
- **前案 Side Finding 10 应收口**：web-mobile-layout-issues-investigation.md 遗留的「通知入口=调度 tab 标题不一致」产品裁决项，与本症状③是同一件事——本卷③的裁决即前案 Side Finding 10 的答案。
- **scroll-smooth 的双刃**：OnboardingView.tsx:317、ButlerWorkspacePanel.tsx:125、RoleWorkspacePanel.tsx:136 同样带 `scroll-smooth`——若 owner 介意滚轮平滑的丧失（路径 a），可评估仅撤 ChatStream 主容器（本卷范围）或统一改路径 b 范式；本次不动其它容器（范围外）。
- **MCP/LLM 卡片图标化与 e2e 命名**：图标钮须带稳定 `aria-label`（如「测试连接 {name}」）供 e2e/无障碍查询；web-mobile.spec.ts 现无相关点击，新增断言须防 setWindowSize 抖动（单跑兜底）。

## Follow-up: 2026-09-26（owner 裁决与实施授权）

### New Evidence

无新事实证据（本回合为产品 owner 对修复方向的裁决，非事实补充）。

### Additional Findings

owner 对「Recommended Next Steps」三问的裁决（2026-09-26）：

1. **授权**（问题 1：②③解冻）：授予冻结规则例外——`ActionCard.mobile.test.tsx:50-63` 既有断言、`web-mobile.spec.ts:348-355` 通知落点块、`spec-16-4-mobile-web-form-and-pwa.md:31` 规格行，可按新行为改写并留变更记录（前案 2026-09-25 授权模式的同型授予）。
2. **共享修复**（问题 2：①方案选择）：选方案 A——`Modal.tsx` 对话框类追加 `max-md:max-h-full max-md:overflow-y-auto`，一次覆盖全部 8 个使用方（含潜伏同症的 TaskModal，见 Side Findings）；遮罩安全区 pt/pb 类不动（Modal.safearea.test.tsx 钉死）。
3. **确认**（问题 3：④图标映射）：确认「4 个图标」= 「当前启用」徽章缩为图标徽章 + 测试连接/编辑/删除 3 个图标钮，`max-md:` 切换、44px 触控、aria-label 可达；MCP 卡同批同模式（开关保持紧凑 toggle 不动）。

### Updated Hypotheses

无（五症状根因均 Confirmed，裁决未推翻任何结论；H2（机型高度）不影响实施方向）。

### Backlog Changes

| # | 原状态 | 新状态 | 说明 |
| - | ------ | ------ | ---- |
| 1-6 | Done | Done | 调查项全部关闭 |
| 7-11 | — | Open | 实施五项修复（①②③④⑤，方向已裁决）+ 三层验证 |

### Updated Conclusion

调查 **Concluded**（2026-09-26）。五项修复方向、涉及文件、测试影响面、解冻授权范围均已定案；实施未启动——按用户 2026-09-26 初始指令「先分析原因和方案，暂不直接执行」，本卷止于诊断与方案。实施走 bmad-quick-dev（五项均为有明确钉孔的定向修复）或建 story；实施完成后回填本卷并验证（受影响组件 vitest + 375px web-mobile.spec.ts 单跑 + TaskModal 走查）。

## Follow-up: 2026-09-26 · 实施完成（按 spec-web-mobile-adaptive-fixes.md）

- **作业图落地**（对应 spec Implementation Notes ①–⑤）：① `Modal.tsx:61` 对话框类追加 `max-md:max-h-full max-md:overflow-y-auto`（遮罩安全区类严禁动），新建 `Modal.scroll.test.tsx`；② `ActionCard.tsx` 按钮容器 → `flex flex-wrap justify-end gap-2 mt-3`、两钮去 `max-md:flex-1`，`ActionCard.mobile.test.tsx` 两例按 owner 授权改写（flex-wrap 系评审 patch，见末节）；③ `MobileSettingsView.tsx` 删「通知」行 + Bell import，`web-mobile.spec.ts` 通知钉孔按 owner 授权改写（7→6 项入口、删落点块），新建 `MobileSettingsView.rows.test.tsx`，spec-16-4 Spec Change Log 追加条目；④ `GlobalSettingsModal.tsx` LLM/MCP 两卡图标化（导入 Zap/Pencil；徽章 Check 图标 + 文字 max-md:sr-only；三钮单按钮 + 文字 span max-md:sr-only + 图标 md:hidden + max-md:h-11/w-11/p-0；**loading 分支保持纯 Loader2**），新建 `GlobalSettingsModal.mobileIcons.test.tsx`；⑤ 初版删 `scroll-smooth`，终版改路径 b —— 见下节「三层评审 patch」修订记录。
- **授权范围外的第三处通知钉孔（owner 追加授权，已同型改写）**：全量 vitest 发现 `App.mobile.test.tsx:201/208`（16.4 评审轮 R1 的 initialTab 落点守门用例）钉死 `settings-row-notification` 在场 + 点击落 scheduler tab——案卷清点与 spec Implementation Notes ③ 均漏点该文件（原 grep 只扫 e2e 目录）。按冻结红线与 AGENTS.md Never 先停改、报备；owner 2026-09-26 追加授权后按同型模式改写：201 行在场断言、208 行点击均改 `settings-row-scheduler`，209 行 `data-initial-tab='scheduler'` 落点断言保留——守门语义（点设置行跳对设置页）不变，6/6 全绿。授权链条与变更记录同步补记 spec-16-4 Spec Change Log。
- **验证结果**：见下表（桌面 e2e 两 spec 的运行结果待回填）。

| 验证项 | 命令 | 结果 |
|---|---|---|
| 受影响四文件 | `npx vitest run Modal.scroll/ActionCard.mobile/MobileSettingsView.rows/GlobalSettingsModal.mobileIcons` | ✅ 4 文件 7 例全绿 |
| 既有相关套件 | `npx vitest run`（Modal.test/Modal.safearea/GlobalSettingsModal/GlobalSettingsModal.initialTab/GlobalSettingsModal.browser/ActionCard/MobileSettingsView.browser/ChatStream） | ✅ 8 文件 140 例全绿（含 ChatStream.test.tsx 对⑤零影响确认、GlobalSettingsModal 按名点击对④零影响确认） |
| 全量套件 | `npx vitest run` | ✅ 84 文件 954 例全绿（App.mobile.test.tsx 第三处通知钉孔经 owner 追加授权同型改写后收口） |
| 类型检查+构建 | `npm run build` | ✅ tsc 零错误，dist 重建（859 kB 主捆） |
| Web e2e 375×812 | `npx wdio run wdio.web.conf.ts --spec ./web-specs/web-mobile.spec.ts` | ✅ 11/11 全绿（含改后「6 项入口」用例；先 npm run build 重建 dist） |
| 桌面 e2e 回归 | `npx wdio run wdio.conf.ts --spec ./specs/accessibility.spec.ts` / `--spec ./specs/role-crud.spec.ts` | ⛔ 环境阻断（非产品回归）：tauri-driver 未起（port 4444 not ready within 15s），session 创建即挂——与 16.4 规格验证表桌面 e2e 行同族阻断面（deferred-work D3 已登记、前案已按 AGENTS.md「连续失败 3 次」升级报备）；本卷零 Tauri/IPC 改动，失败点在一切 UI 断言之前。补偿覆盖：≥768px 桌面路径由上述 vitest 桌面断言钉死（本次改动仅 max-md:/md: 类对，类级惰性） |
| TaskModal 走查 | Side Findings 项 | 共享 Modal 修复覆盖（方案 A 受益面）；375px 真机新建/编辑任务走查留人工（已登记 deferred-work，与 iOS Safari 真机清单同批执行） |

## Follow-up: 2026-09-26 · 三层评审 patch 收口（step-04 R1）

- **评审机制**：盲扫 N=10 / 边界 / 验证缺口三层并行（diff 基线 e952e07 + 规格 + 案卷）。24 条去重发现逐条读证裁决，全程记录见 `spec-web-mobile-adaptive-fixes.md` Review Triage Log（F1–F16）：patch ×10 项操作、defer ×2（F10 Modal 滚动增强 / F12 epic-16-context:45 修订，均已入 deferred-work.md）、false/驳回 ×9（含「多配置图标钮同名歧义」系改前既有且冻结块锁定可访问名保留、「loading 无名按钮空窗」被既有测试 GlobalSettingsModal.test.tsx:262 故意钉死、「双滚动容器行为变化」经内层 max-h≤90vh 算术证伪、「e2e 调度落点块删除」有单测层 App.mobile.test.tsx:210 落点守门顶替）。
- **patch 落地（4 组代码 + 全部文档同步）**：F1 `GlobalSettingsModal.tsx` 三处防挤压类（卡头 flex-wrap/gap-y-2、左组 min-w-0、徽章 flex 组）全部 `max-md:` 前缀化——初版误转录为无前缀类，破「只加 max-md:/md: 类对」冻结红线（桌面宽下虽不触发换行，红线须字面合规）；mobileIcons 测试补钉「有前缀类且无对应无前缀类」。F3 `ActionCard.tsx` 容器补 `flex-wrap`（调查卷推荐项，当前标签宽度下 inert）+ 测试钉。F4/F5 `ChatStream.tsx` **路径 a → 路径 b**：保留 `scroll-smooth`，`:881-886` 贴底赋值前临时 `style.scrollBehavior='auto'`、赋值后恢复——初版删类被证实误伤来源消息居中（`:241` 修正赋值由 CSS 平滑化变即时、抢占 `:238` 的平滑滚动，来源导航动画静默丧失；规格原判「:238 不受影响」在效果层被证伪）；新建 `ChatStream.scroll.test.tsx` 钉「类保留 + auto→赋值→恢复序列」（jsdom 不支持 Element.scrollTo、style.scrollBehavior 可读写——探针实证，故用原型补丁观测）。
- **验证（patch 后）**：受影响 5 文件 114 例全绿（ChatStream.test.tsx 来源导航 :1016/:1017、GlobalSettingsModal.test.tsx 按名点击等既有断言零伤）；桌面 e2e 两 spec 仍为环境阻断（D3 同族，deferred-work 已登记）；`npm run build` 与 web-mobile.spec.ts 375px 11/11 为 patch 前已验结果——patch 仅动类名/补类/滚动样式切换与新建测试文件，无逻辑路径变化，e2e 550px 重跑排入人工确认批（与真机走查同批，避免已知 setWindowSize 负载抖动面误判）。
