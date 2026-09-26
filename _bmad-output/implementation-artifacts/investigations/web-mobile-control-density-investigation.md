# Investigation: web 版手机浏览器控件密度两症状（建议卡按钮高度 / LLM 卡徽章与图标簇）

## Hand-off Brief

1. **What happened.** owner 2026-09-26 反馈两症状：管卡建议卡「拒绝/确认」按钮小屏高度 44px 偏高大（想低一点）；LLM Provider 卡「当前启用」徽章想改为裸勾图标（去背景色）、4 个图标（✓/⚡/✎/🗑）想更紧凑且靠右成一簇。代码级根因已全部 Confirmed。
2. **Where the case stands.** 两症状根因定位完成并实施：族一（徽章去背景/勾并入右簇/右簇 gap-1）零红线冲突落地；族二（触控盒降档）经 owner 2026-09-26 拍定档位 B=36px，显式重协商 ≥44px 冻结红线（spec-16-4 Change Log「交付后增量二」留痕、局部特例勿外推）。已 Closed，实施与验证回填见文末 Follow-up。
3. **What's needed next.** 无未决项。遗留（deferred 性质）：真机 375px 视觉走查闭环（36px 档位 tap 观感）；MCP 开关 24px 高（Side Findings，存量设计未动）；`epic-16-context.md:45` stale（F12，继续 deferred）。后续若推广 36px 档位至其它触控面，须重新走 owner 重协商。

## Case Info

| Field            | Value                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| Ticket           | N/A（owner 口述，承接 web 版手机浏览器五症状修复 b5f1ebd 的细化反馈）          |
| Date opened      | 2026-09-26                                                                  |
| Status           | Concluded（根因 Confirmed；owner 2026-09-26 决策+实施已回填，见 Follow-up）             |
| System           | web 版手机浏览器 <768px（项目基线 375×812）；无真机截图                       |
| Evidence sources | 代码（HEAD=b5f1ebd 工作树）、spec-16-4 冻结块、spec-web-mobile-adaptive-fixes.md（done）、既有测试钉孔 |

## Problem Statement

owner 原话（假设登记为 Hypothesis 1/2/3，均已验证）：
1. 「管家的建议卡片中，'拒绝'和'确认'的高度可以低一点」——小屏按钮高度想低于当前值。
2. 「LLM Provider 配置的卡片上'当前启用'展示为勾图标即可，不需要背景色」——徽章去背景色、只留勾。
3. 「并且 4 个图标可以紧凑一点，靠右显示」——徽章勾与 3 个操作图标聚成一簇、收紧、靠右。

## Evidence Inventory

| Source   | Status    | Notes     |
| -------- | --------- | --------- |
| 代码（HEAD 工作树） | Available | `ActionCard.tsx:200-218`、`GlobalSettingsModal.tsx:712-726`、`mobileIcons.test.tsx`、`ActionCard.mobile.test.tsx` |
| 冻结规格 | Available | `spec-16-4-mobile-web-form-and-pwa.md:14-57`（冻结块，:20「触控目标 ≥44×44px」）；`spec-web-mobile-adaptive-fixes.md:18/38/53`（上轮 owner 确认映射含 ≥44px 触控） |
| 使用方标签 | Available | `ButlerView.tsx:190-196`（敲门卡 confirmLabel=立即处理 / rejectLabel=稍后） |
| 既有钉孔 | Available | `ActionCard.mobile.test.tsx`（钉 `max-md:min-h-[44px]`/`py-1.5`/无 flex-1/容器 justify-end+flex-wrap）；`GlobalSettingsModal.mobileIcons.test.tsx`（钉三钮 `max-md:h-11 max-md:w-11`、sr-only span、md:hidden 图标、徽章 `max-md:flex`、卡头 `max-md:flex-wrap/gap-y-2`）；`GlobalSettingsModal.test.tsx`（按名点击，未涉徽章 Class/高度） |
| e2e | Available | `tests/e2e/web-specs/web-mobile.spec.ts` 无按钮高度/徽章/图标簇断言（该类改动零 e2e 守门，纯类级+视觉） |
| 真机截图/期望数值 | Missing   | 「低一点/紧凑一点」无量化；影响档位选择，见 Missing Evidence |

## Investigation Backlog

| # | Path to Explore                                          | Priority | Status | Notes                                   |
| - | -------------------------------------------------------- | -------- | ------ | --------------------------------------- |
| 1 | ActionCard 按钮高度的来源与桌面对照值                         | High     | Done   | Finding 1                                |
| 2 | LLM 卡徽章 Chip 与右侧图标簇的现行布局与宽度账                 | High     | Done   | Finding 2/3                              |
| 3 | 冻结红线（≥44px）lineage 与重协商必要性                       | High     | Done   | Finding 4                                |
| 4 | 同族面扫描（MCP 卡开关行/其它配置项）是否同症                   | Medium   | Done   | Side Findings                            |
| 5 | 修复档位与钉孔改写清单（若 owner 拍板）                        | High     | Done   | owner 拍定档位 B；实施回填见 Follow-up   |

## Timeline of Events

| Time        | Event               | Source                | Confidence |
| ----------- | ------------------- | --------------------- | ---------- |
| 16.4（204af71） | 移动形态定稿：触控目标 ≥44×44px 入冻结块；ActionCard 按钮 `max-md:min-h-[44px]` | spec-16-4:20 + git | Confirmed  |
| 2026-09-26 | 五症状修复 b5f1ebd：④ LLM/MCP 卡图标化（徽章 Check + 3 图标钮 44px）；② ActionCard 一行排（保留 min-h-[44px]） | spec-web-mobile-adaptive-fixes.md | Confirmed  |
| 2026-09-26 | owner 反馈两症状（本卷）；本卷开立 | 用户消息 | Confirmed  |

## Confirmed Findings

### Finding 1: ActionCard 小屏按钮高度 = 44px，直接来源是 `max-md:min-h-[44px]`（冻结红线的类级落地）

**Evidence:** `egosync-app/src/components/butler/ActionCard.tsx:204-217`（拒绝钮 `px-3 py-1.5 max-md:min-h-[44px] max-md:flex max-md:items-center max-md:justify-center`；确认钮同构 `px-4`）+ `spec-16-4-mobile-web-form-and-pwa.md:20`（冻结块 Approach「触控目标 ≥44×44px」）+ `spec-web-mobile-adaptive-fixes.md:38`（上轮规格明记「保留 `max-md:min-h-[44px]`（触控红线 AC2 不破）」）。

**Detail:** 文字钮内容高度 = `py-1.5`(12px) + `text-[13px]` 行高(≈19.5px) ≈ 31.5px——**`max-md:min-h-[44px]` 是唯一把高度撑到 44px 的约束**，多出的 ~12px 是为触控留的余量。桌面（≥768px）无 min-h 约束，按钮高 ≈31.5px。owner「高度可以低一点」的体感来源：小屏 44px 的厚重块 vs 桌面 31.5px。使用方两组标签均不受影响：「拒绝/确认」默认建议卡（`ActionCard.tsx:189/216` 默认 props）与「立即处理/稍后」敲门卡（`ButlerView.tsx:192-193`）。

### Finding 2: 「当前启用」徽章是带背景 Chip，去背景/聚合有零冲突落点

**Evidence:** `egosync-app/src/components/settings/GlobalSettingsModal.tsx:718`（`px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-100 text-indigo-700 max-md:flex max-md:items-center max-md:gap-1` + `<Check size={12} className="md:hidden" />` + `<span className="max-md:sr-only">当前启用</span>`）+ `GlobalSettingsModal.mobileIcons.test.tsx:115-120`（只钉徽章含 `md:hidden` Check 与 `max-md:sr-only` 文字、徽章有 `max-md:flex`——**未钉 bg / px / py / text-[11px] 任何视觉类**）。

**Detail:** 全库仅此一处「当前启用」（LLM 卡 isDefault 专属；MCP 卡无徽章）。桌面渲染 = 紫底白字小 Chip（`bg-indigo-100 text-indigo-700` + 文字常显）；小屏 = 同底 Chip 内套 12px 勾（文字 sr-only）。去背景色只需在移动作用域加 `max-md:bg-transparent`（或同族去掉 rounded/px-2/py-0.5），桌面 Chip 原样——**与「桌面逐像素零变化」红线兼容，且无既有钉孔挡路**（mobileIcons 测试不断言背景类；`GlobalSettingsModal.test.tsx` 无徽章 Class 断言）。

### Finding 3: 右侧图标簇 44px 盒 + gap-2 + 徽章留左侧 = 「不紧凑、不靠右」的观感来源

**Evidence:** `GlobalSettingsModal.tsx:720-726`（右组 `flex items-center gap-2`，三钮各 `max-md:h-11 max-md:w-11 max-md:p-0` + 14px 图标）+ `:714-715`（头部 `flex items-center justify-between max-md:flex-wrap max-md:gap-y-2`；左组 radio + 名字 + 徽章）。

**Detail:** 小屏每个操作钮是 44×44 盒套 14px 图标——盒内每边 ~15px 空缘，三盒 + gap-2(8px) 共 148px 宽、44px 高；徽章勾（~28px 宽 Chip）留在左组名字旁。于是 4 个图标分居两处（左 1 右 3）、右侧盒大且疏。「紧凑+靠右一簇」的最小改动 = 徽章勾挪进右组与三钮并列 + `gap-2→gap-1`（右簇宽 148→164px 含勾；375px 内容区 295px 仍放得下，超宽由头部既有 `max-md:flex-wrap` 兜底）。**若 owner 的「紧凑」包含缩小触控盒本身（44→40/36），则触及 Finding 4 红线**。

### Finding 4: 44px 触控是双锁冻结约束——缩小需 owner 显式重协商

**Evidence:** `spec-16-4-mobile-web-form-and-pwa.md:14`（`<frozen-after-approval>` 起始）→ `:20`（「触控目标 ≥44×44px」）→ `:57`（冻结块结束）+ `spec-web-mobile-adaptive-fixes.md:18`（上轮 owner 确认④映射明文含「≥44px 触控」）与 `:53`（实现范式「44px 触控」）+ 钉孔 `ActionCard.mobile.test.tsx`（`max-md:min-h-[44px]`）与 `GlobalSettingsModal.mobileIcons.test.tsx`（`max-md:h-11 max-md:w-11`）。

**Detail:** 本仓移动端触控 44px 是贯穿性设计（BottomTabBar、ChatInput、TasksTab、RoleHeader、「⋯」菜单、登出弹窗等全部 ≥44px）。单独把这两处降到 40/36px 会造成全 App 触控目标不一致（同页不同高度的可点目标），且破除两条已批准 intent。按本仓纪律（冻结边界只认 owner 显式重协商、改动须留 Spec Change Log），**档位必须 owner 拍**——owner 即诉求人，重协商本身无障碍，但须显式授权并在规格留痕。

## Deduced Conclusions

### Deduction 1: 两症状的「根因」均为上轮/16.4 修复的类级参数，而非布局缺陷

**Based on:** Finding 1/2/3

**Reasoning:** 三个症状点全部收敛到具体 Class 值（min-h-[44px]、bg-indigo-100 Chip、h-11/w-11 + gap-2），且这些值都是有意的红线/映射落地，不是 bug。

**Conclusion:** 修复是「参数级调档 + 徽章聚合」，无结构性改动；风险面在钉孔改写授权而非实现难度。

### Deduction 2: 三处诉求可拆成「零冲突」与「需重协商」两族，可分别拍板

**Based on:** Finding 2/3/4

**Reasoning:** 徽章去背景、勾挪位、gap 收紧均不碰 44px 盒、不碰桌面类；只有「按钮再矮/图标盒再小」才破 ≥44px。

**Conclusion:** 建议拆批：族一（徽章+聚合+间距）可直接实施；族二（触控盒降档）待 owner 档位授权。若 owner 只想要族一，ActionCard 高度问题仍悬置。

### Deduction 3: e2e 与本轮钉孔对两症状零守门，验证靠类级钉孔 + 视觉走查

**Based on:** Evidence Inventory（e2e 无相关断言；mobileIcons/ActionCard.mobile 测试钉的正是将被改写的值）

**Reasoning:** web-mobile.spec.ts 不断言高度/徽章背景/图标簇；两个类钉孔文件断言的恰是拟改值（改值必须同步改钉孔——owner 授权改写，模式同 b5f1ebd）。

**Conclusion:** 实施时钉孔改写清单 = `ActionCard.mobile.test.tsx`（min-h 档位）、`GlobalSettingsModal.mobileIcons.test.tsx`（h/w 档位 + 徽章断言形态）；验证以 vitest 全绿 + build + 375px 单跑 e2e（回归）为主，视觉效果留真机/模拟器走查。

## Hypothesized Paths

### Hypothesis 1: 「高度低一点」的期望值是降到 40px 或 36px 一档

**Status:** Closed（owner 拍定档位 B=36px；见 Follow-up）

**Theory:** owner 观感来源是 44px 盒的厚重；行业常见次级文字钮 36-40px。

**Supporting indicators:** 桌面同钮仅 ~31.5px；`text-[13px]` 小字配 44px 盒比例失衡。

**Would confirm:** owner 给档位，或实施后截图确认。

**Would refute:** owner 表示「只想去点 padding/间距，盒保持 44」。

### Hypothesis 2: 「4 个图标紧凑、靠右」= 徽章勾并入右簇 + 收紧 gap，不一定缩盒

**Status:** Closed（成立——族一独立交付，见 Follow-up）

**Theory:** 观感问题是「分居两处 + 盒大且疏」；聚合+收紧即可显著改善。

**Supporting indicators:** Finding 3 的宽度账显示聚合后 375px 仍单行放得下。

**Would confirm:** 实施族一后截图确认。

**Would refute:** owner 明确说「盒子也要小」。

### Hypothesis 3: 徽章去背景期望覆盖桌面（全断点）

**Status:** Closed（owner 确认仅 max-md:，桌面 Chip 保留）

**Theory:** owner 在移动语境反馈，大概率只指小屏；但「不需要背景色」若指全断点，桌面 Chip 也要去底。

**Supporting indicators:** 本轮全程移动语境；桌面 Chip 为存量 16.4 前设计样式。

**Would confirm:** owner 明示。

**Would refute:** owner 确认仅小屏（推荐此解，桌面红线零风险）。

## Missing Evidence

| Gap              | Impact                               | How to Obtain   |
| ---------------- | ------------------------------------ | --------------- |
| ~~「低一点/紧凑一点」的量化期望~~ | ~~档位选择（40/36px）无锚点~~ | **✓ 2026-09-26 已闭环：owner 拍定档位 B=36px**（见 Follow-up） |
| ~~徽章去背景的作用断点~~ | ~~桌面是否同步去底涉及桌面红线~~ | **✓ 2026-09-26 已闭环：owner 确认只管手机（仅 max-md:）**（见 Follow-up） |
| 真机视觉（iOS Safari 动态工具栏/安卓字体缩放） | 36px 档位 tap 观感/误触率 + 徽章勾簇视觉效果无真机数据 | 人工走查（已登记 deferred-work 2026-09-26「控件密度降档真机走查」条目） |

## Source Code Trace

| Element       | Detail                                      |
| ------------- | ------------------------------------------- |
| Error origin  | `egosync-app/src/components/butler/ActionCard.tsx:204-217`（min-h-[44px] 撑高）；`egosync-app/src/components/settings/GlobalSettingsModal.tsx:718`（徽章 Chip 背景类）、`:720-726`（右簇 44px 盒 + gap-2）、`:714-715`（徽章留左组） |
| Trigger       | <768px 渲染：`max-md:` 类生效（按钮 44px 高、徽章 Chip 带底、右簇 3×44px 盒） |
| Condition     | 16.4 冻结触控红线 + 上轮④映射共同锁定的类值；owner 现要求视觉降档 |
| Related files | `GlobalSettingsModal.mobileIcons.test.tsx`、`ActionCard.mobile.test.tsx`（断言语改值）、`ButlerView.tsx:190-196`（同构使用方）、`spec-16-4-mobile-web-form-and-pwa.md:20`（红线出处）、`epic-16-context.md:45`（ActionCard 小屏形态冻结件，stale 未授权修订——D12） |

## Conclusion

**Confidence:** High（代码级根因全部 Confirmed；唯一开放项是档位/断点口径，属 owner 决策而非证据缺失）

- 症状一直接原因：`ActionCard.tsx` 两钮的 `max-md:min-h-[44px]`（内容仅需 ~31.5px，44px 全为触控余量）；桌面同钮因无 min-h 只有 ~31.5px——「小屏比桌面厚重」即观感来源。
- 症状二/三直接原因：徽章是 `bg-indigo-100` 底 Chip 停留在左组名字旁；右簇 = 3 个 44px 盒（14px 图标 + ~15px 空缘）+ gap-2——四图标分居两处、右侧大而疏。
- 核心矛盾：44px 为 16.4 冻结 + 上轮规格双锁红线；「降高度/缩盒」需 owner 显式重协商（授权 + 留痕），「徽章去底/聚合/收紧间距」零红线冲突。

## Recommended Next Steps

### Fix direction（分两族，均只加/改 `max-md:` 类对，桌面零变化）

**族一（零红线冲突，建议先做）：LLM 卡徽章 + 右簇聚合**
1. 徽章移动端去背景：`:718` Chip 类加 `max-md:bg-transparent`（连同 `max-md:rounded-none max-md:px-0 max-md:py-0` 去 Chip 外壳，仅留 12px 裸勾）；桌面 Chip 原样。
2. 勾挪进右簇：左组徽章加 `max-md:hidden`，右组最前插 `<Check size={12} className="md:hidden text-indigo-600 shrink-0" />`（桌面无此元素，DOM 变化但桌面渲染不变）。
3. 右簇收紧：`:720` `gap-2 → max-md:gap-1`。
4. 钉孔同步：`mobileIcons.test.tsx` 徽章断言形态微调（无背景类断言需补「无 bg 有 max-md:bg-transparent」钉孔防回归）；右簇新位置下三钮/勾的 DOM 关系断言更新。

**族二（需 owner 档位授权 + 重协商 ≥44px 红线）：触控盒降档**
- 档位 A（折中，推荐）：ActionCard 两钮 `max-md:min-h-[44px] → max-md:min-h-[40px]`（`min-h-10`）；LLM/MCP 三图标钮 `max-md:h-11 max-md:w-11 → max-md:h-10 max-md:w-10`；MCP 卡同步（同族一致性）。
- 档位 B（更紧）：36px（`min-h-9`/`h-9 w-9`）——低于 Apple HIG 44pt 最多的一档，误触风险最高。
- 档位 C（不降盒）：维持 44px，仅族一——ActionCard 高度问题不解决（需向 owner 明示）。
- 授权与留痕：owner 显式授权后，`spec-16-4:20` 行追加重协商注记（同 2026-09-26 五症状模式：Change Log + 案卷），改写 `ActionCard.mobile.test.tsx` 与 `GlobalSettingsModal.mobileIcons.test.tsx` 两处钉孔（本次均在「新测试新建文件」规则之外、属既有钉孔同型改写，须逐项授权）。

**不做的（明示）**：不改桌面任何 Class（红线）；不动 MCP 开关行（24px 高开关为 16.4 存量，见 Side Findings）；不给图标盒加 tooltip/title（上轮 F8 已论证可访问名方案）。

### Diagnostic

1. owner 给档位（或按档位 A 实施后 375px 模拟器截图确认）。
2. 确认徽章去背景作用断点（推荐仅 `max-md:`）。
3. 若族二授权：钉孔改写清单 + `spec-16-4` Change Log 条目先行，再动代码（纪律同 b5f1ebd）。

## Reproduction Plan

1. `dsh web` 或 `npm run dev`（egosync-app）起 web 版，DevTools 切 375×812 视口。
2. 管家视图 → 任一见习/敲门建议卡：测两钮高度（现状 44px；桌面 ≥768px 同钮 ~31.5px）。
3. 设置 → LLM Provider 卡：观察「当前启用」Chip（有底）居名字右旁；测 3 个操作钮盒 44×44、间距 8px；四图标分居两处。
4. 预期（族一后）：徽章裸勾入右簇与三钮连排、间距 4px；桌面双断点截图对照零变化。
5. 预期（族二档位 A 后）：两族高度 40px，截图与 tap 走查确认可接受。

## Side Findings

- **MCP 卡开关 24px 高（<44px）**：`GlobalSettingsModal.tsx` MCP 行启用开关为 `h-6 w-11`（24×44px），高度方向低于红线——16.4 存量设计（ButlerSettingsContent/Role SettingsTab 同款开关全局一致），非本卷引入；若将来统一触控审计可一并处理。
- **`epic-16-context.md:45` 仍 stale**（「ActionCard 全宽堆叠」）：b5f1ebd 评审 F12 已登记 deferred-work 待授权修订；本卷族二若落地，该文件的按钮高度描述同样需同步（同一授权批次）。
- **样式一致性提示**：若族二降档，全 App 44px 触控面（BottomTabBar/ChatInput/TasksTab/RoleHeader/登出弹窗等）保持不变——建议实施后在案卷明记「本降档为 owner 显式重协商的局部例外」，避免后续开发者误当默认值推广。
- **图标尺寸余量**：44px 盒内图标 14px（空缘 ~15px/边）——族二若降盒至 40px，图标可同步 14px 不变；若只族一，盒内空缘不变，视觉增密有限（需向 owner 明示族一的改善边界）。

## Follow-up: 2026-09-26 实施回填（owner 决策 → bmad-build oneshot 落地）

owner 2026-09-26 拍板：**档位 B（36px）+ 徽章去背景只管手机**。评审为 Decision-based case → **Closed（Owner Decision + Implemented）**；根因结论（Finding 1–4/Deduction 1–3）原样保留未改。决策与实施记录：

- **族一（零冲突，全做）**：徽章拆双断点双形态——桌面 Chip 原样（`max-md:hidden` 收口）；小屏裸 Check（`text-indigo-600 dark:text-indigo-400`）并入右组与三钮连排、`max-md:sr-only` 保留「当前启用」可访问名；LLM/MCP 两卡右组 `max-md:gap-1` 收紧（「4 图标紧凑靠右」= DOM 同组实证）。
- **族二（owner 重协商 ≥44px 红线，档位 B）**：ActionCard 两钮 `max-md:min-h-[44px]→[36px]`；LLM/MCP 三图标钮 `max-md:h-11/w-11→h-9/w-9`；MCP 同步以保同族一致。
- **钉孔授权改写**（owner 授权链延续 b5f1ebd）：`ActionCard.mobile.test.tsx`、`GlobalSettingsModal.mobileIcons.test.tsx` 按新档位/双形态改写并加档位痕迹钉孔；`GlobalSettingsModal.test.tsx`/`ActionCard.test.tsx`/`web-resident-loop.spec.ts` 零改动。
- **留痕**：spec-16-4 Change Log 新增「交付后增量二」条目 + 冻结块 :20 授权修订批注；spec `spec-web-mobile-control-density.md`（oneshot，done）；重协商口径「局部特例勿外推」已写入 ActionCard 代码注释。
- **验证**：定向四文件 51/51 绿；`npm run build`（tsc + vite）零错误；全量 vitest 85 文件 / 955 测试绿；375px e2e（`web-mobile.spec.ts`）11/11 绿。实现期一处钉孔修正：徽章文本为 Chip 直接文本，`getAllByText` 首项即 Chip 本体（旧结构为 sr-only 子 span）——断言改按类特征（`max-md:hidden`/`max-md:sr-only`）定位而非 parent/DOM 序。评审回补（blind-hunter，spec Review Triage Log G1–G10）：右组 `gap-2` 桌面基线钉孔、档位痕迹逐类负向钉孔、DOM 序依赖改类特征定位。
- **决策点回填**（原 Open）：H1（期望值）→ 36px 确认，观感问题待真机走查闭环（见 Missing Evidence 真机项）；H2（聚合不一定缩盒）→ 成立（族一独立交付）；H3（徽章断点）→ 确认仅 `max-md:`，桌面 Chip 逐像素保留。

