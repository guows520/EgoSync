# Investigation: opencode Skill 发现无反馈

## Hand-off Brief

1. **What happened.** 已确认前端在发现调用成功但返回 `items=[]`、`skipped.reasons=[]` 时，只结束 loading，不产生任何可见反馈。
2. **Where the case stands.** 根因已定位到 `SettingsTab.handleDiscoverOpencode` 的成功空结果分支缺失；后端将目录不存在/不可读作为“空扫描成功”返回，前端错误分支本身有提示。
3. **What's needed next.** 在 Skill 区域增加一次扫描已完成的局部状态，并为零结果渲染友好信息，同时补充空结果组件测试；无需修改后端扫描协议。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-21 |
| Status | Concluded |
| System | Windows；React/TypeScript 前端；Tauri/Rust 后端 |
| Evidence sources | UAT 文档、前后端源码、组件测试、git 历史 |

## Problem Statement

UAT 用例 9 阶段 E 中，单击 opencode 生态 Skill 的“发现”按钮后短暂显示“发现中”，随后恢复原状；没有新 Skill 时也没有友好提示。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| UAT 用例 | Available | 阶段 E 定义发现与导入步骤，但没有规定成功空结果的预期 |
| 前端源码 | Available | 已追踪按钮、handler、结果区和错误提示 |
| 后端源码 | Available | 已确认扫描目录和空结果返回语义 |
| 组件测试 | Available | 覆盖禁用、错误、有结果、导入；缺少空结果反馈测试 |
| git 历史 | Available | 相关 handler 于 2026-06-10 引入，空结果分支从引入时即缺失 |
| 本次运行日志 | Missing | 未采集；不影响确认代码中的空结果反馈缺口 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | UAT 阶段 E 精确步骤 | High | Done | 已确认第 27-31 步及预期 |
| 2 | 前端按钮与 loading | High | Done | 已定位 `handleDiscoverOpencode` |
| 3 | 服务/IPC/后端扫描结果 | High | Done | 空目录返回成功空集合 |
| 4 | 现有反馈约定 | Medium | Done | 错误和其它成功操作均有可见反馈 |
| 5 | 测试与 git 历史 | Medium | Done | 已确认测试缺口与引入提交 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-06-10 | `3537dc1` 引入 opencode Skill 发现 handler；未实现成功空结果反馈 | git history | Confirmed |
| 2026-07-21 | UAT 报告点击后只出现短暂 loading | 用户报告 | Confirmed |
| 2026-07-21 | 源码调查确认空结果路径与报告现象一致 | source trace | Confirmed |

## Confirmed Findings

### Finding 1: 成功空结果分支没有设置任何反馈状态

**Evidence:** `egosync-app/src/components/role/SettingsTab.tsx:375-397`

**Detail:** handler 开始时清空错误、成功消息、候选列表和跳过列表；调用成功后只写入 `items`、展开状态和跳过原因。`items.length === 0` 且无跳过原因时，没有消息或“已扫描”状态。`finally` 只把 loading 恢复为 false。

### Finding 2: UI 仅在有跳过项或有候选项时渲染发现结果

**Evidence:** `egosync-app/src/components/role/SettingsTab.tsx:699-725`

**Detail:** 跳过摘要受 `opencodeSkipped.length > 0` 控制；结果区域受 `opencodeSkills.length > 0` 控制。两者均为空时，按钮恢复“发现”，页面与点击前视觉相同。

### Finding 3: 后端把不存在/不可读的扫描根目录视为正常空结果

**Evidence:** `egosync-app/src-tauri/src/services/skill_registry.rs:351-395`, `egosync-app/src-tauri/src/services/skill_registry.rs:404-407`

**Detail:** 后端扫描项目级 `.opencode/skills` 和用户全局 `.config/opencode/skills`；`read_dir` 失败时直接 `Ok(())`，最后正常返回可能为空的 `items` 和 `reasons`。因此“没有任何目录或 Skill”是合法成功结果，不会进入前端 catch。

### Finding 4: 失败分支已有可见错误提示，问题不是统一错误处理缺失

**Evidence:** `egosync-app/src/components/role/SettingsTab.tsx:393-396`, `egosync-app/src/components/role/SettingsTab.tsx:989-997`, `egosync-app/src/components/role/SettingsTab.test.tsx:827-837`

**Detail:** discover reject 会转换为友好错误并在界面渲染，且已有测试。因此用户看到“无其它变化”更符合成功空集合，而非异常被静默吞掉。

### Finding 5: 验收与测试均遗漏成功空结果语义

**Evidence:** `_bmad-output/uat/UAT-Simplified-Manual.md:637-643`, `_bmad-output/uat/UAT-Simplified-Manual.md:715-723`, `egosync-app/src/components/role/SettingsTab.test.tsx:135`

**Detail:** UAT 只描述“查看扫描结果列表”；测试默认 mock 空集合，但没有点击后断言空结果反馈。该缺口使实现长期未被测试发现。

## Deduced Conclusions

### Deduction 1: 本次现象是确定性的 UI 状态建模遗漏

**Based on:** Findings 1-4

**Reasoning:** loading 正常出现说明 handler 已执行；若调用失败，现有 catch 会显示错误；若有结果或跳过项，相应区块会出现。只有成功且 `items/reasons` 均为空时，代码会精确产生“loading 消失、页面无变化”的现象。

**Conclusion:** 根因不是按钮失效，也不需要修改 IPC 或扫描算法；是前端未区分“尚未扫描”和“扫描完成但为零结果”。

## Hypothesized Paths

### Hypothesis 1: 空结果成功分支缺少用户反馈

**Status:** Confirmed

**Resolution:** Findings 1-3 直接确认。

### Hypothesis 2: 发现请求异常被静默吞掉

**Status:** Refuted

**Resolution:** catch 设置友好错误，界面有错误渲染且测试覆盖，见 Finding 4。

### Hypothesis 3: 发现请求未真正执行或 IPC 未注册

**Status:** Refuted

**Resolution:** loading 与用户报告一致；service 调用和 Tauri command 均存在，后端合法返回空集合即可完整解释现象，无需该假设。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 本次运行的实际响应 payload | 只能确认当时是否确为 `items=[]/reasons=[]` | 若需要审计具体环境，可复现并记录 invoke 返回值；不阻塞修复 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Trigger | `SettingsTab.tsx:675-684` 的“发现 opencode Skill”按钮 |
| Frontend handler | `SettingsTab.tsx:375-397` `handleDiscoverOpencode` |
| IPC service | `src/services/skillService.ts:10` 调用 `skill_discover_opencode` |
| Tauri command | `src-tauri/src/commands/skill.rs:103-112` |
| Backend scan | `src-tauri/src/services/skill_registry.rs:351-395` |
| Producing condition | `result.items.length === 0 && result.skipped.reasons.length === 0` |
| Missing presentation | `SettingsTab.tsx:699-725` 无零结果分支 |

## Conclusion

**Confidence:** High

根因已确认：发现流程只建模 loading、候选结果、跳过摘要和错误，没有建模“扫描成功但零结果”。后端又明确允许扫描目录不存在或无 Skill 时返回成功空集合，因此前端会在 `finally` 中结束 loading 后恢复到点击前的视觉状态。未采集本次运行 payload 只影响对具体机器返回值的审计，不影响对代码缺陷机制的确认。

## Recommended Next Steps

### Fix direction

采用前端局部状态修复，不修改后端：

1. 在 `SettingsTab` 增加最小的“已完成发现”状态，例如 `hasDiscoveredOpencode`。
2. 每次点击开始时重置为 false；discover 成功后设为 true；错误时保持 false。
3. 当已完成且 `opencodeSkills.length === 0`、`opencodeSkipped.length === 0` 时，在 opencode Skill 区块按钮下方显示中性信息：`未发现 opencode Skill，已扫描项目级与全局 Skill 目录。`
4. 若 `items=[]` 但存在跳过原因，保留现有跳过摘要即可；可在摘要标题中明确“未发现可导入的 Skill”，但不必新增全局 toast。
5. 不建议复用页面底部的 `settingsSavedMessage`：该提示离操作区域较远，且语义混合；局部提示更清晰，修改仍很小。

### Tests

在 `SettingsTab.test.tsx` 增加意图测试：

- find-skills 已启用、discover 返回空集合时，显示“已扫描但未发现”的局部提示，并恢复按钮文本。
- 有候选结果时不显示空结果提示。
- reject 时只显示错误，不显示空结果提示（现有错误测试可追加断言）。

同时更新 UAT 阶段 E 预期：无新 Skill 时必须显示友好中文反馈，不得仅恢复按钮状态。

## Reproduction Plan

1. 保证项目级和全局 opencode Skill 目录不存在或为空。
2. 进入已启用 find-skills 的角色设置页，点击“发现”。
3. 修复前：按钮恢复，无其它变化。
4. 修复后：按钮恢复，并在同一区块显示已扫描但未发现的提示。
5. 再分别验证有候选、全为无效项、后端报错三种分支，确保提示互斥且旧功能不回归。

## Side Findings

- 此缺陷不是近期 2026-07-20 Skill 热加载修复引入的回归；相关空结果分支从 2026-06-10 的 `3537dc1` 初始实现起即存在。
- 后端将所有 `read_dir` 失败统一视为空目录，包括权限错误；这是另一个可观测性语义问题，但与当前“无新 Skill 应友好提示”的最小修复无关，不建议本次顺手扩大范围。
