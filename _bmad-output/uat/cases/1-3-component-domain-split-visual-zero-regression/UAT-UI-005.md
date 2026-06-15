---
用例编号: UAT-UI-005
测试模块: 组件域目录结构
story_key: 1-3-component-domain-split-visual-zero-regression
version_anchor: 5268797
exec_mode: auto
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅检查文件结构与行数，不改动代码。
---

# UAT-UI-005 组件按域分布且入口文件精简

## 业务场景
为便于后续维护，团队要求组件按业务域分目录，且应用入口文件只承担路由/状态职责。维护者期望看到清晰的域目录结构与精简的 App.tsx。

## 前置条件
- 仓库可访问。

## 测试步骤
1. 检查 `src/components/` 下是否按域分为 layout/butler/role/modals/onboarding/notifications/settings 等目录。
2. 检查工具函数在 `lib/`、Mock 数据集中在 `constants/`。
3. 查看 `src/App.tsx` 行数。
4. 检查 mock 数据文件是否标注"后续将替换为真实数据"的 TODO。

## 预期结果
- components 下存在 7 个域目录，组件分布合理。
- `lib/utils.ts` 提供 cn() 工具；mock 数据集中管理且含 TODO 注释。
- `App.tsx` 行数 ≤ 100。

## 实际结果
- `src/components/` 下域目录：butler / chat / layout / modals / notifications / onboarding / role / settings 共 8 个（覆盖预期的 7 类，分布合理）。✅
- `lib/utils.ts` 提供 `cn()` 工具函数。✅
- Mock 数据集中于 `constants/mockData.ts`，含 1 处"后续替换为真实数据"类 TODO 注释。✅
- **`src/App.tsx` 行数 = 281，超过预期上限 100（超出 181 行）。** ❌

四项中前三项满足，仅"App.tsx ≤ 100 行"未达标。因预期结果明确将行数列为验收项，故判 Fail。

### 复测（2026-06-15，修复后）
修复方案：将 App.tsx 的逻辑关注点抽取为 5 个 hook（useThemeToggle / useSourceNavigation / useRoleLifecycle / useButlerProposal / useModalStack）+ 2 个展示组件（AppMainContent / AppModalStack），入口仅保留路由/状态编排。纯机械重构，行为不变。
- `src/components/` 域目录 8 个（覆盖预期 7 类）。✅
- `lib/utils.ts` 提供 `cn()`；`constants/mockData.ts` 含 TODO 注释。✅
- **`src/App.tsx` 行数 = 99，≤ 100 上限。** ✅
- 回归保护：`App.test.tsx` 4 用例 + 全量 164 测试通过，行为零回归。✅
四项全部达标。

## 测试结论
Pass（复测通过，原 Fail 已由 bmad-investigate 立案 + 重构修复后解决）
