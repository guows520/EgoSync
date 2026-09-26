---
title: 'web 版手机浏览器控件密度降档（建议卡按钮高度 / LLM 卡徽章与图标簇）'
type: 'bugfix'
created: '2026-09-26'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** owner 2026-09-26 反馈两症状（均为 b5f1ebd 五症状修复的细化）：小屏建议卡「拒绝/确认」按钮 44px 过高（`ActionCard.tsx:204-217` 的 `max-md:min-h-[44px]`——内容仅需 ~31.5px，44px 全为触控余量，观感肥于桌面同钮）；LLM Provider 卡「当前启用」徽章带 `bg-indigo-100` 背景 Chip 停留名字旁、与右侧 3 个 44px 图标盒分居两处且盒大稀疏（`GlobalSettingsModal.tsx:714-726`）。根因与证据链见案卷 `investigations/web-mobile-control-density-investigation.md`（Confidence: High）。

**Approach:** 纯 `max-md:` 类级视觉降档，桌面（≥768px）逐像素零变化。族一（零红线冲突）：徽章移动端去 Chip 外壳只留裸勾、勾挪入右侧与三钮连排成一簇、右簇 `gap-2` 收紧。族二（owner 显式重协商 ≥44px 触控红线，2026-09-26 拍定档位 B=36px）：ActionCard 两钮 `max-md:min-h-[36px]`；LLM/MCP 两卡三图标钮 `max-md:h-9 max-md:w-9`（MCP 同步以保两卡同族一致）。既有钉孔 `ActionCard.mobile.test.tsx` / `GlobalSettingsModal.mobileIcons.test.tsx` 按 owner 授权同型改写（授权模式同 b5f1ebd）；红线重协商留痕 spec-16-4 Spec Change Log + 案卷 Follow-up。

</frozen-after-approval>

## Implementation Notes

1. `egosync-app/src/components/butler/ActionCard.tsx`：两按钮（拒绝 :204-210 / 确认 :211-217）类中 `max-md:min-h-[44px]` → `max-md:min-h-[36px]`；`py-1.5`/`text-[13px]`/`max-md:flex max-md:items-center max-md:justify-center` 与容器 `flex flex-wrap justify-end gap-2 mt-3` 不动。注释段补记：44px 触控余量为 16.4 冻结红线（spec-16-4:20）经 owner 2026-09-26 档位 B 显式重协商降档，本文件为新下限特例，勿外推。
2. `egosync-app/src/components/settings/GlobalSettingsModal.tsx:718` 徽章改桌面专属 Chip：`px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-100 text-indigo-700` 保留，删 `max-md:flex max-md:items-center max-md:gap-1`，span 文本去 `max-md:sr-only`（桌面常显），整 span 加 `max-md:hidden`，删徽章内 `<Check size={12} className="md:hidden" />`。
3. 同文件右组（`:724`；其上 `:720-723` 为实现注释）：`flex items-center gap-2` → `flex items-center gap-2 max-md:gap-1`（桌面基线 gap-2 保留）；最前插条件勾簇——`{conf.isDefault && <span className="md:hidden max-md:flex max-md:items-center max-md:shrink-0"><Check size={12} className="text-indigo-600 dark:text-indigo-400" /><span className="max-md:sr-only">当前启用</span></span>}`——桌面 `md:hidden` 整块不渲染，桌面像素零变化；移动端保留「当前启用」可访问名（读屏不丢）。
4. 同文件 LLM 卡三钮（`:731` 测试连接 / `:734` 编辑 / `:735` 删除）与 MCP 卡三钮（`:889` / `:892` / `:893`）：`max-md:h-11 max-md:w-11` → `max-md:h-9 max-md:w-9`；其余图标钮机制（单按钮 + `max-md:sr-only` 文字 span + `md:hidden` 图标 + `max-md:p-0 max-md:flex max-md:items-center max-md:justify-center`、loading 分支纯 Loader2）一律不动。MCP 开关 toggle（`:870` 右组内）不动；MCP 右组 `flex shrink-0 items-center gap-2`（`:870`）保留基线并追加 `max-md:gap-1`（实施期决定：两卡同族一致，规格未列明，agent-owned 决策）。
5. `egosync-app/src/components/settings/GlobalSettingsModal.mobileIcons.test.tsx`（owner 授权同型改写）：`expectIconButton` 助手中 `max-md:h-11`/`max-md:w-11` → `max-md:h-9`/`max-md:w-9`；徽章断言改形态——`screen.getAllByText('当前启用')` 两处：桌面 Chip span（有 `max-md:hidden`）与右簇 sr-only span（父级 `md:hidden`、内含 Check svg）；卡头 `max-md:flex-wrap`/`max-md:gap-y-2`/`max-md:min-w-0` 断言与两卡按名唯一命中断言保留。
6. `egosync-app/src/components/butler/ActionCard.mobile.test.tsx`（owner 授权同型改写）：两按钮断言 `max-md:min-h-[44px]` → `max-md:min-h-[36px]`；`flex-wrap`/`justify-end`/无 `flex-1`/无 `max-md:flex-col` 等其余断言不动。
7. 文档留痕：`spec-16-4-mobile-web-form-and-pwa.md` Spec Change Log 追加 2026-09-26 第二条目（≥44×44 红线的局部重协商——仅 ActionCard 文字钮与 LLM/MCP 图标钮降 36px，owner 档位 B；BottomTabBar/ChatInput/TasksTab/RoleHeader 等其余触控面 44px 不变，防误当默认值外推）；案卷 `web-mobile-control-density-investigation.md` Follow-up 回填实施与验证；`epic-16-context.md:45` 继续 deferred（F12，不在本次授权内）。
8. 验证命令（egosync-app/ 下）：`npx vitest run src/components/settings/GlobalSettingsModal.mobileIcons.test.tsx src/components/settings/GlobalSettingsModal.test.tsx src/components/butler/ActionCard.mobile.test.tsx src/components/butler/ActionCard.test.tsx` → 全绿且按名点击断言零改动通过；`npm run build` → tsc 零错误；全量 `npx vitest run`；`tests/e2e/` 下 `npx wdio run wdio.web.conf.ts --spec ./web-specs/web-mobile.spec.ts`（375px 回归，含改后「6 项入口」用例——该 spec 无按钮高度/徽章断言，回归目的是防 E2E 选择器受 DOM 调整波及）。
9. 桌面红线自检：改动项必须全部 `max-md:` 作用域（徽章 `max-md:hidden`、右簇 `md:hidden`、gap/min-h/h/w 降档）；两卡按名点击（测试连接/编辑/删除）、sr-only 可访问名在双断点不回退。
10. 实施回填（2026-09-26，oneshot）：① ActionCard 两钮改 `max-md:min-h-[36px]`（拒绝 `:216` / 确认 `:223`），注释补重协商留痕。② LLM 卡徽章（`:718`）→ 桌面专属 Chip（`max-md:hidden`、删移动 Check 与 sr-only）；右组（`:724`）保留 `gap-2` 并追加 `max-md:gap-1`、插条件勾簇（`md:hidden` + Check `text-indigo-600 dark:text-indigo-400` + `max-md:sr-only` 文字）。③ LLM 三钮（`:731`/`:734`/`:735`）与 MCP 三钮（`:889`/`:892`/`:893`）共 6 处 `max-md:h-11 max-md:w-11 max-md:p-0` → `max-md:h-9 max-md:w-9 max-md:p-0`。④ MCP 右组（`:870` `flex shrink-0 items-center gap-2 max-md:gap-1`）同步收紧——规格未列明，实施期决定（两卡同族一致，agent-owned 决策）。⑤ 钉孔两文件按 owner 授权改写（h-9/w-9 正向 + 非 44/40px 逐类负向痕迹；徽章按类特征定位双形态；右组 `gap-2` 桌面基线钉孔；MCP 右组 gap 钉孔）。⑥ 调试修正：徽章文本现为 Chip 自身直接文本，`getAllByText('当前启用')` 首项即 Chip 本体（旧结构为 sr-only 子 span），断言改按类特征（`max-md:hidden`/`max-md:sr-only`）定位而非取 parent。**验证结果**：定向四文件 51/51 绿；`npm run build`（tsc + vite）零错误；全量 vitest 85 文件 / 955 测试绿；375px e2e（`web-mobile.spec.ts`）11/11 绿。三层评审改为 oneshot 单层 blind-hunter（N=8，10 条发现），裁决 patch ×9 / defer ×1（e2e 密度断言），See Review Triage Log。

## Review Triage Log

（oneshot 单层评审 blind-hunter：N = min(floor(√54.8)+1), 10) = 8，实收 10 条，逐条读证裁决）

| # | 发现（盲扫层，severity 忽略） | 裁决 | 证据与处置 |
|---|---|---|---|
| G1 | 规格状态三处矛盾：spec frontmatter `in-progress` vs spec-16-4/案卷写 `done` | medium → patch | 属实。属评审时点差（Finalize 尚未执行）；本条即 Finalize：frontmatter 置 `done`，三处口径归一。 |
| G2 | 验证记录缺失且数字不符：案卷「95+ 文件」vs 实际 85 文件/955 测试；spec 挂「见 Review 后提交」 | low → patch | 属实（案卷 Follow-up 验证行 + spec Implementation Notes ⑩ 均已回填真实数字：定向 51/51、build 零错误、全量 85/955、e2e 11/11）。 |
| G3 | ActionCard.tsx 注释裁决一仍写「触控 ≥44px」与裁决二 36px 并存 | low → patch | 属实。裁决一段落已加〔同日经 owner 重协商降至 36px——见裁决二〕，新旧口径不再并存。 |
| G4 | 桌面 gap 基线缺钉孔：只钉 `max-md:gap-1` 未钉基线 `gap-2`；spec-16-4 表述「gap-2 → max-md:gap-1」读作替换 | medium → patch | 属实。两钉孔文件补 `toHaveClass('gap-2','max-md:gap-1')`（LLM 右组、MCP 右组、ActionCard 按钮行 `gap-2`/`mt-3`）；spec-16-4 条目改为「保留并追加」表述，杜绝按日志删基线。 |
| G5 | 徽章断言依赖 DOM 序（getAllByText[0]/[1]） | low → patch | 属实（当前 DOM 序确定，但脆弱且报错指向差）。改按类特征定位：`max-md:hidden`（桌面 Chip）/`max-md:sr-only`（小屏勾簇文字），`toHaveLength(2)` 保留。 |
| G6 | 375px e2e 开到目标组件却零密度断言；纯观感反馈无改前/改后视觉对照 | medium → defer | 部分属实且超出本增量授权链（owner 授权止于「e2e 回归防选择器波及」；该 spec 有已知 setWindowSize flake）。拆两条登记 deferred-work：①真机 375px 视觉走查（36px tap 观感 + 勾簇视觉，与 F14 iOS 清单同批）；②e2e 密度断言补充（需先评估 flake 与授权）。类级钉孔已闭合回归守门。 |
| G7 | deferred-work 未登记本轮遗留；F12 条目未随增量扩展（epic-16-context.md:45 高度维度也 stale） | medium → patch + defer | 属实。登记三条：真机走查、F12 范围扩展（新条目，旧条目按规矩不动）、e2e 断言补充（与 G6 共项）。 |
| G8 | spec 行号引用错位（右组 :720→实 :724；LLM 三钮 :721-725→实 :731/734/735；MCP :876-887→实 :889/892/893；ActionCard :204-210→实 :216/:223） | low → patch | 属实（徽章注释块插入致行号平移）。Implementation Notes ②③④⑩ 已按当前文件实测行号修正。 |
| G9 | 案卷 Missing Evidence 表未对账（档位/断点两行已闭环仍列开放） | low → patch | 属实。两行划删并标「✓ 2026-09-26 已闭环」，仅存真机视觉一行（指向 deferred-work 新条目）。 |
| G10 | 「非 44px」负向钉孔是 OR 语义（`not.toHaveClass(a,b)` 缺一类即过） | low → patch | 属实。两文件均拆为逐类断言（h-11/w-11/h-10/w-10 四条独立），单独回退一类即红。 |

patch 合计 9 组（G1–G5、G7、G8–G10），defer 2 条（G6/G7 的 deferred-work 登记）；复审复跑定向 4 文件 51/51 绿。
