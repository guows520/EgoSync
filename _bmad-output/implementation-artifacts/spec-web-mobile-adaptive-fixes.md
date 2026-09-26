---
title: 'web 版手机浏览器五症状自适应修复'
type: 'bugfix'
created: '2026-09-26'
status: 'done'
baseline_commit: 'e952e07'
route: 'oneshot'
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** web 版手机浏览器（<768px）五个自适应症状（用户口述 2026-09-26；②–⑤ 经用户更正为独立流程，与「新增角色」无因果关系）：①新增角色模态标题栏被遮盖、显示不全、界面不能上下滑动；②管家建议卡片「拒绝」「确认」小屏各占一行、过大；③移动设置「通知」行与「调度时间」行落点完全重复，桌面版无「通知」配置项，应移除；④LLM Provider 配置卡片「当前启用」「测试连接」等按钮小屏被挤压成纵向（需同查其它配置项——MCP Server 卡片存在同族反模式）；⑤从角色切换到管家后，对话区高度超屏时有明显的平滑滚动动效拉到最底部，应直接显示底部。

**Approach:** 根因与证据链见调查案卷 `investigations/web-mobile-adaptive-issues-investigation.md`（五症状代码级根因全部 Confirmed）。owner 三项裁决（2026-09-26）：①**授权解冻**——`ActionCard.mobile.test.tsx:50-63`、`web-mobile.spec.ts` 通知相关钉孔、`spec-16-4-mobile-web-form-and-pwa.md:31` 规格行可按新行为改写并留 Spec Change Log 变更记录；②①走**共享修复**——`Modal.tsx` 补 `max-md` 高度/滚动（方案 A，8 个使用方一次覆盖，含潜伏同症的 TaskModal）；③④「4 个图标」映射**确认**——「当前启用」徽章缩图标 + 测试连接/编辑/删除 3 个图标钮，`max-md:` 切换、≥44px 触控、可访问名保留。硬红线：桌面（≥768px）逐像素零变化，改动只加 `max-md:`/`md:` 类对；既有测试断言除 owner 授权范围外零改动。

</frozen-after-approval>

## Implementation Notes

> 代码地图（相对路径均自仓库根）。调查过程不在此重复——实施按本图作业即可。

### ① 共享 Modal 补高度/滚动（症状①，方案 A）

- `egosync-app/src/components/layout/Modal.tsx:61`：对话框类 `... outline-none` 后追加 `max-md:max-h-full max-md:overflow-y-auto`。原理：遮罩为 `fixed inset-0 flex items-center justify-center`（53 行，含安全区 pt/pb），`max-h-full` 使对话框高度以遮罩内容盒为上限，超高内容改为对话框内滚动，标题可达；桌面无 `max-md` 类不触发，逐像素零变化。遮罩类严禁改（`Modal.safearea.test.tsx:24-27` 钉死 pt/pb）。
- 受益面（零额外改动）：AddRoleModal（症状①本体）、TaskModal（潜伏同症，Side Findings，顺手验证）、WeeklyReviewModal/ButlerSettingsContent/RoleConfirmModal 自带 max-h 不变（内层 80/85/90vh 先命中，行为不变）。
- 新建 `egosync-app/src/components/layout/Modal.scroll.test.tsx`：钉对话框带 `max-md:max-h-full max-md:overflow-y-auto` 且**无**桌面态 max-h 类（`md:` 对不存在=桌面不触发）；Modal.test.tsx 一行不动。
- 非目标：标题行 sticky 常驻（增强项，本卷不做，留 deferred-work 如需）。

### ② 建议卡片按钮一行排（症状②，owner 授权改写）

- `egosync-app/src/components/butler/ActionCard.tsx:201-217`：
  - 注释（201-202 行）改写为「2026-09-26 owner 裁决：小屏一行右对齐（原全宽纵排为 16.4 冻结设计，已显式重协商）」；
  - 容器 203 行 `flex max-md:flex-col gap-2 mt-3 max-md:justify-stretch justify-end` → `flex justify-end gap-2 mt-3`；
  - 两个按钮去掉 `max-md:flex-1`，保留 `max-md:min-h-[44px] max-md:flex max-md:items-center max-md:justify-center`（触控红线 AC2 不破；375px 下「拒绝」+「确认」合计 ≈140px、敲门卡「立即处理/稍后」≈160px，一行放得下）。
- `egosync-app/src/components/butler/ActionCard.mobile.test.tsx`（owner 授权改写既有断言）：用例一断言按钮有 `max-md:min-h-[44px]`、`py-1.5`，且**无** `max-md:flex-1`；用例二断言容器为 `justify-end` 且**无** `max-md:flex-col`/`max-md:justify-stretch`。ActionCard.test.tsx 一行不动；web-resident-loop.spec.ts:271 只查「拒绝」文本在场，零影响。

### ③ 移除移动设置「通知」行（症状③，owner 授权改写）

- `egosync-app/src/components/settings/MobileSettingsView.tsx`：删 58-59 行 notification 行；Bell 仅该行使用（line 2 import 唯一引用点），同步删。移除后内容入口 = llm/mcp/scheduler/data 四项；敲门通知声音仍经「调度时间」→ scheduler tab 可达（`GlobalSettingsModal.tsx:1039-1068`），功能零损失。
- `egosync-app/tests/e2e/web-specs/web-mobile.spec.ts`（owner 授权范围含本文件通知相关钉孔）：
  - 307 行用例标题「7 项入口」→「6 项入口」；311-312 行注释改为「4 项内容入口：模型服务/MCP/调度/数据」、key 数组去 `'notification'`；
  - 删 348-356 行整块（「通知」点击 + `h3*=调度时间配置` 落点断言 + 关闭）。
  - 第三处钉孔（owner 追加授权，2026-09-26）：`egosync-app/src/App.mobile.test.tsx:201/208`（initialTab 落点守门用例，清点时漏点）同型改写——在场断言与点击改 `settings-row-scheduler`，209 行 `data-initial-tab='scheduler'` 落点断言保留，守门语义不变。
- 新建 `egosync-app/src/components/settings/MobileSettingsView.rows.test.tsx`（冻结规矩「新测试一律新建文件」）：钉 4 个 `settings-row-{llm,mcp,scheduler,data}` 在场、`settings-row-notification` 不在场、主题/登出行不受影响。MobileSettingsView.browser.test.tsx 一行不动（未钉通知行）。
- `_bmad-output/implementation-artifacts/spec-16-4-mobile-web-form-and-pwa.md:31`：在 Spec Change Log 追加 2026-09-26 条目（见下「规格文档」）。

### ④ LLM/MCP 卡片按钮小屏图标化（症状④，映射已确认）

实现范式（两卡同模式，红线守法）：**单按钮元素 + 文字 span `max-md:sr-only` + 图标 `md:hidden`**——可访问名在任意断点由 span 文本派生（桌面与现状逐像素一致；小屏视觉为图标、读屏与既有按名查询不受影响）；按钮加 `max-md:h-11 max-md:w-11 max-md:p-0 max-md:flex max-md:items-center max-md:justify-center`（44px 触控）。

- `egosync-app/src/components/settings/GlobalSettingsModal.tsx`：
  - line 2 import 追加 `Zap, Pencil`（Trash2/Check/Loader2 已在）。
  - LLM 卡：714 行头部类加 `max-md:flex-wrap max-md:gap-y-2`；715 行左组加 `max-md:min-w-0`；718 行徽章改为 `<span className="px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-100 text-indigo-700 max-md:flex max-md:items-center max-md:gap-1"><Check size={12} className="md:hidden" /><span className="max-md:sr-only">当前启用</span></span>`；721-723 测试连接钮 idle 分支改 `<><span className="max-md:sr-only">测试连接</span><Zap size={14} className="md:hidden" /></>`，**loading 分支保持纯 Loader2 不动**（`GlobalSettingsModal.test.tsx:262` 断言 loading 态按钮 `name: ''`，加 span 即破）；724 编辑钮 → `Pencil`；725 删除钮 → `Trash2`；三钮加上述 max-md 尺寸类。
  - **review F1 修订（2026-09-26）**：头部/左组/徽章的防挤压类全部 `max-md:` 前缀化——冻结块明文「只加 `max-md:`/`md:` 类对，桌面逐像素零变化」，初版误转录为无前缀类（桌面宽下虽不触发换行、但红线须字面合规）；mobileIcons 测试同步钉「有前缀类、且无对应无前缀类」。
  - MCP 卡（860-884）：测试连接（879-881，loading 纯 Loader2 同样保留）、编辑（882）、删除（883）同款改造；开关 toggle（861-878）不动。
  - 防回归要点：既有四测试按名点击 `删除/编辑/测试连接`（`GlobalSettingsModal.test.tsx:135,173,185,258`）——span 文本保证可访问名不变，单配置下唯一匹配，零改动通过。
- 新建 `egosync-app/src/components/settings/GlobalSettingsModal.mobileIcons.test.tsx`：钉 LLM 卡三钮各含 `max-md:sr-only` 文字 span + `md:hidden` 图标（Zap/Pencil/Trash2）、徽章含 `md:hidden` Check、按钮带 `max-md:h-11 max-md:w-11`；按名 `getByRole('button', {name:'测试连接'})` 唯一可命中（名字典不漂移）。既有 GlobalSettingsModal*.test.tsx 三件套一行不动。

### ⑤ 移除切换视图的贴底动效（症状⑤）

- `egosync-app/src/components/chat/ChatStream.tsx:1354`：滚动容器保留 `scroll-smooth`（review F4 修订，见下）。挂载/流式更新的贴底 effect（881-886）赋值前临时置 `style.scrollBehavior='auto'`、赋值后恢复——贴底即时落底，无全程滑动动画（症状⑤ owner 口径「直接显示最底部即可」达成）。来源消息居中（:238 显式 `behavior:'smooth'` + :241 scrollTop 修正）与滚轮平滑全部保留；`ChatStream.test.tsx:1016` 不依赖 CSS 类，零改动。新建 `ChatStream.scroll.test.tsx` 钉「容器带 scroll-smooth + auto→赋值→恢复序列」。
- **review F4 修订记录（2026-09-26，路径 a → 路径 b）**：初版从容器删 `scroll-smooth` 被证实误伤来源消息居中——`centerMessageInScrollContainer`（`:237-242`）由两个滚动站点协作（:238 scrollIntoView smooth 通知浏览器平滑滚动 + :241 计算修正位的 scrollTop 直接赋值），删类后 :241 从「CSS 平滑化」变即时、**抢占** :238 的平滑动画，来源导航的居中动画静默丧失；规格初版「:238 不受影响」的判据在效果层被证伪（只数了 :884 一个站点，漏了 :241）。路径 b：类保留 + 仅贴底赋值临时置 auto 再恢复。代价：流式期间贴底跟随由平滑改即时（token 级高频更新本就不断打断平滑动画，净观感不变）；收益：来源导航动画、滚轮平滑两个既有行为零变化。

### 规格文档

- `_bmad-output/implementation-artifacts/spec-16-4-mobile-web-form-and-pwa.md`：Spec Change Log 追加「2026-09-26 · 交付后增量：移动自适应五症状修复（owner 显式重协商冻结边界）」——记录②③④三项冻结边界重协商（ActionCard 全宽纵排→一行排、设置 7 入口去「通知」、徽章/动作钮小屏图标化）、①⑤非冻结修复，以及 owner 授权范围（ActionCard.mobile.test.tsx、web-mobile.spec.ts 通知钉孔改写）。

### 验证计划（DoD）

1. `npx vitest run` 受影响四文件（ActionCard.mobile/MobileSettingsView.rows/GlobalSettingsModal.mobileIcons/Modal.scroll）+ 全量套件；既有 GlobalSettingsModal.test.tsx/ActionCard.test.tsx/Modal*.test.tsx 必须零改动全绿。
2. `npm run build`（tsc 零错误）。
3. e2e：`web-mobile.spec.ts` 单跑（375×812；须先 `npm run build` 重建 dist——前案结论：首跑旧 dist 会伪失败）；`role-crud.spec.ts`（AddRoleModal 流）+ `accessibility.spec.ts`（axe）回归。全量套件已知 setWindowSize 负载抖动面（deferred-work 既有条目），逐 spec 单跑兜底。
4. 实施完成后回填案卷 `investigations/web-mobile-adaptive-issues-investigation.md` Backlog #7–#11 → Done。

## Review Triage Log

评审轮 1（2026-09-26，step-04 三层并行：盲扫 N=10 / 边界 / 验证缺口；diff 基线 e952e07）。裁决口径：被访代码实际读证；评审层给的严重度一律不采信（缺上下文），由本表重裁。验证缺口层的 gap 发现按规则预采信（该层已自证），本表仍复核了引证。

| # | 发现（层） | 裁决 | 证据与路由 |
| - | ---------- | ---- | ---------- |
| F1 | GlobalSettingsModal 加了三处**无 max-md: 前缀**的类（`flex-wrap gap-y-2`/`min-w-0`/徽章 `flex items-center gap-1`），破「只加 max-md:/md: 类对」红线（盲扫+边界 claim） | **medium / patch** | 读 diff 实证三处均无前缀。桌面宽下头部本就不换行、视觉影响趋零但未经证明，徽章 inline→flex 有亚像素位移风险——红线是冻结块明文，类名前缀化即恢复合规。→ 三处改 `max-md:` 前缀；mobileIcons 测试改钉前缀类。注：根因在本规格非冻结段（Implementation Notes ④ 转录了无前缀类），修正后同段同步，不触发 loopback（冻结块意图「桌面逐像素零变化」始终明确，属机械校正而非重新推导）。 |
| F2 | 症状④的抗挤压半边（LLM 卡头 `flex-wrap`/左组 `min-w-0`）零钉孔， revert 后全绿（验证缺口 gap #2 + 盲扫） | **medium / patch** | 核实 mobileIcons 测试只钉按钮/徽章，无头部断言；既有按名点击与 e2e 横溢检查（wrap 不产生横溢）都抓不到。→ mobileIcons 测试补钉头部两处类（随 F1 前缀化后的形态）。 |
| F3 | ActionCard 容器丢了调查卷 Recommended Next Steps ② 明写的 `flex-wrap`，长标签无换行兜底（盲扫+边界） | **low / patch** | 全库仅两组标签（默认「确认/拒绝」与 ButlerView「立即处理/稍后」，均 ≈160px 放得下 375px）；风险只在未来自定义标签。补 `flex-wrap` 对当前宽度 inert、且本就是调查建议原形。→ 容器加 `flex-wrap`；mobile 测试加钉。 |
| F4 | 来源消息居中（`:238` scrollIntoView smooth + `:241` 即时 scrollTop 修正）在删 `scroll-smooth` 后**丢失平滑动画**——`:241` 赋值由 CSS 平滑化变为即时，抢占 `:238` 的动画（边界 claim + 验证缺口 Other） | **medium / patch** | 读 `:237-242` 实证：赋值紧跟 smooth 调用同帧执行；改前容器 CSS `scroll-behavior:smooth` 使赋值也走动画（两段平滑收敛同一落点），改后赋值即时、挂起的 smooth 被直接赋值中止。规格⑤「:238 不受影响」之说在效果层被证伪——受影响的滚动站点是两个（:884 与 :241），原分析只数了 :884。→ 改走**路径 b**：恢复容器 `scroll-smooth` 类，仅把 :881-886 挂载贴底赋值包一层 `style.scrollBehavior='auto'`→赋值→恢复。:238/:241/滚轮平滑全保留，症状⑤（切换重挂载贴底动效）按 owner 口径即时化。 |
| F5 | 症状⑤修复无任何钉孔，把类加回去（或删掉 auto 包裹）即静默回归（验证缺口 gap #1 + 盲扫前半） | **medium / patch** | 已实证：`ChatStream.test.tsx` 两条断言（显式 scrollIntoView 参数、scrollTop 终值）都与容器 CSS 类无关；jsdom 无 CSS scroll-behavior；e2e 零滚动断言。→ 新建 `ChatStream.scroll.test.tsx`：钉容器带 `scroll-smooth`（F4 路径 b 下该类是 :238/:241/滚轮平滑的载体，删它即回归来源导航动画）；并给 CSSStyleDeclaration 原型打记录补丁观测 auto→赋值→恢复的切换序列（jsdom 支持 style.scrollBehavior 读写，已探针实证）。 |
| F6 | e2e 删通知落点块后，「点设置行进对页」在 web e2e 层无证据（盲扫） | **low / 驳回** | 落点映射守门已存在于单测层（`App.mobile.test.tsx:210` scheduler→scheduler 落点断言 + llm 行 e2e 的 `h3=LLM Provider` 落点仍在）；补回 e2e 属重复覆盖且落在已知 setWindowSize 负载抖动面。用户可感知的产品行为无缺口。 |
| F7 | `App.mobile.test.tsx:209-210` 同型改写后退化自证（点 scheduler 行断 scheduler 落点），案卷「守门语义不变」说过头了（盲扫） | **low / patch** | 改写确实把跨映射守门（notification→scheduler 别名）降为直接映射守门——但别名本身已被 owner 授权移除，「跨映射守门」失去守护对象是产品变化的必然，不是代码缺陷；断言仍能抓住「行未接线/接错 tab」。→ 只修文档措辞（案卷+16.4 Change Log：「直接映射守门保留；跨映射别名守门随别名移除而消失」）。 |
| F8 | 图标钮可访问名无对象区分，多配置 getByRole 歧义、读屏无法区分，偏离案卷 Side Findings 的 `aria-label{name}` 原方案（盲扫两条） | **false** | 改前按钮本就同名（'测试连接'/'编辑'/'删除' 纯文本）——非本卷引入；冻结块明文「可访问名保留」，且按对象加 aria-label 会打破未授权的 `GlobalSettingsModal.test.tsx:135/173/185/258` 精确名点击。现状=冻结意图的正确执行。 |
| F9 | loading 态按钮可访问名为空的窗口期（盲扫） | **false** | 改前 loading 分支同为纯 Loader2 无名按钮，且 `GlobalSettingsModal.test.tsx:262` **故意**钉 `name:''`——既有事实，非本卷引入。 |
| F10 | Modal 缺 `overscroll-contain` 与打开期间背景滚动锁定（盲扫） | **low / defer** | 全库 8 个模态共享同一「不锁背景」既有模式，无任何 overscroll 用法；内层滚动链传导仅在 body 可滚时有感。为避免共享组件全局行为漂移，记 deferred-work 留待模态滚动 UX 专项。 |
| F11 | WeeklyReview/ButlerSettingsContent/RoleConfirmModal 变双层滚动容器、行为变化无佐证（盲扫） | **false** | 三者内层自带 `max-h-[80/85/90vh]`：常规情形（无安全区内边距）90vh < 遮罩内容盒（=100vh）⇒ 外层滚动永不介入，「行为不变」论断成立；极端（刘海屏安全区吃掉 >10vh + RoleConfirm 90vh）外层多一层良性滚动，内容可达性不变。 |
| F12 | `epic-16-context.md:45` 仍写「ActionCard 全宽堆叠」，无 2026-09-26 重协商交叉引用（盲扫） | **medium / defer** | 读证实 stale。但该文件是人类所有的规划冻结件、不在 owner 授权清单内——按「改 agent/rules 类文件的条目一律 defer」同型处理，记 deferred-work 待 owner 授权修订；16.4 Spec Change Log 已载明重协商链，可溯源。 |
| F13 | spec-16-4 正文 :31 仍列 7 入口含「通知」，与同文档 Change Log 自相矛盾（盲扫） | **medium / patch** | 读证实。owner 授权范围明文含「规格行可按新行为改写」——已授权却只追加了 Change Log。→ 改写 :31 为 4 内容入口 + 主题/登出，并加指向 2026-09-26 Change Log 条标记记。 |
| F14 | 案卷 Status Closed 与开放项（桌面 e2e ⛔、TaskModal/iOS 走查留人工）无去向矛盾（盲扫后半） | **low / patch** | 部分属实：桌面 e2e 有 D3 登记，另两项走查无 deferred-work 归属。→ 补两条 deferred-work 条目；Status 措辞改「Closed（实施收尾；开放项见 deferred-work）」。 |
| F15 | 案卷 Timeline 止于「本卷开立」，缺裁决/追加授权/实施完成三条（盲扫） | **low / patch** | 读证实。→ 补三条时间线（owner 三项裁决、App.mobile 追加授权、实施完成）。 |
| F16 | 规格 status 停滞 in-progress + 无实施结果回填段（盲扫末条） | **前半 false / 后半 low / patch** | status 已由父代理置 in-review（评审启动时）——diff 快照滞后一拍；「规格读起来像尚未开工」属实。→ 规格追加「实施结果」段（指向案卷验证表）。 |

**路由汇总**：patch ×8（F1 F2 F3 F4 F5 F7 F13 F14 F15 F16 计 10 项操作）；defer ×2（F10 F12，追加 deferred-work）；false ×3（F8 F9 F11）+ F16 前半。无 intent_gap / bad_spec ⇒ 不触发 loopback（F1 的规格非冻结段校正按 patch 同批处理，理由见该行）。

## 实施结果（2026-09-26 review patch 后回填）

- **改动文件（14+1，相对基线 e952e07）**：Modal.tsx + Modal.scroll.test.tsx（新）；ActionCard.tsx + ActionCard.mobile.test.tsx；MobileSettingsView.tsx + MobileSettingsView.rows.test.tsx（新）；web-mobile.spec.ts；GlobalSettingsModal.tsx + GlobalSettingsModal.mobileIcons.test.tsx（新）；App.mobile.test.tsx（owner 追加授权的同型改写）；ChatStream.tsx + ChatStream.scroll.test.tsx（新，评审 patch）。
- **review patch 落地**：F1 GlobalSettingsModal 三处防挤压类 `max-md:` 前缀化 + F2 测试钉「有前缀/无无前缀」；F3 ActionCard 容器补 `flex-wrap` + 测试钉；F4/F5 ChatStream 路径 b（保留 scroll-smooth，贴底赋值临时 auto 再恢复）+ 新建 scroll 钉孔（类 + auto/恢复序列，原型补丁观测）。
- **验证**：受影响 5 个测试文件 114 例全绿（含既有 ChatStream 来源导航 :1016/:1017、GlobalSettingsModal 按名点击）；`npm run build`（tsc）通过；`web-mobile.spec.ts` 375px 11 项全绿（step-03 期间，实现 subagent 执行）。桌面 e2e（accessibility/role-crud）两 spec 因 tauri-driver 未运行被阻塞——D3 先例，记 deferred-work。
- **开放项去向**：TaskModal 375px 走查 / iOS Safari 真机走查 / Modal 背景滚动锁定（F10）/ epic-16-context:45 修订（F12）——均入 `deferred-work.md`。
