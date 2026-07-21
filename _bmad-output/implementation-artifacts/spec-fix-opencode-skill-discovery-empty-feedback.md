---
title: '修复 opencode Skill 发现成功但无结果时缺少反馈'
type: 'bugfix'
created: '2026-07-21'
status: 'done'
baseline_commit: 'dcd6ff7db5fcdd51ab4bb65ceb77811f33e30574'
context:
  - '_bmad-output/project-context.md'
  - '_bmad-output/implementation-artifacts/2-12-opencode-ecosystem-skill-discovery-import.md'
  - '_bmad-output/implementation-artifacts/investigations/opencode-skill-discovery-no-feedback-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 角色设置页的 opencode 生态 Skill 发现请求成功但返回零候选、零跳过项时，按钮仅从“发现中...”恢复为“发现”，用户无法判断扫描是否执行完成。该缺口使“尚未扫描”和“已扫描但未发现”呈现为相同状态。

**Approach:** 在角色设置页为发现流程增加最小的“成功完成过扫描”状态，并在成功空结果时于 opencode Skill 区块内显示固定文案：“已扫描项目级与全局 Skill 目录，未发现 opencode Skill。”补充组件测试与 UAT 预期，不改变后端扫描结果协议。

## Boundaries & Constraints

**Always:** 提示必须位于触发发现操作的 Skill 区块内；仅在请求成功、候选数量为零且跳过原因数量为零时显示；新一轮扫描开始、角色切换或请求失败时不得残留旧空结果提示；保留现有 loading、候选列表、跳过摘要和错误提示行为。

**Ask First:** 如果实现必须修改 Tauri command、Rust 扫描服务、共享 Skill 类型，或必须改变 Butler 设置页的交互，则暂停并说明原因。

**Never:** 不新增全局 toast；不复用远离操作区的页面级保存成功提示；不把后端错误显示成“未发现”；不顺手调整目录权限错误语义、扫描路径或导入流程；不重构相邻 SettingsTab 状态。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 成功空结果 | find-skills 已启用；返回 `items=[]`、`reasons=[]` | loading 结束并显示指定空结果文案 | 不显示错误或跳过摘要 |
| 成功有候选 | 返回至少一个候选 | 展示现有候选结果，不显示空结果文案 | N/A |
| 全部无效 | 候选为空但 `reasons` 非空 | 展示现有跳过摘要，不显示纯空结果文案 | N/A |
| 请求失败 | service reject | 展示现有友好错误，不显示空结果文案 | 保持现有 `toFriendlyError` 行为 |
| 再次扫描 | 已显示空结果后重新点击 | 立即清除旧提示并进入 loading，完成后按新结果显示 | N/A |

</frozen-after-approval>

## Code Map

- `egosync-app/src/components/role/SettingsTab.tsx` -- 发现 handler、局部状态与 opencode Skill 区块反馈渲染。
- `egosync-app/src/components/role/SettingsTab.test.tsx` -- 发现成功、失败、候选与跳过摘要的组件行为测试。
- `_bmad-output/uat/UAT-Simplified-Manual.md` -- 用例 9 阶段 E 的手工验收步骤与预期。
- `_bmad-output/implementation-artifacts/investigations/opencode-skill-discovery-no-feedback-investigation.md` -- 根因证据与范围边界。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src/components/role/SettingsTab.tsx` -- 增加一次成功扫描完成状态；在 handler 开始、成功和失败路径正确维护；在发现区块内渲染互斥的空结果提示。
- [x] `egosync-app/src/components/role/SettingsTab.test.tsx` -- 增加成功空结果意图测试，并在已有候选、跳过和错误场景中验证空提示不误显示。
- [x] `_bmad-output/uat/UAT-Simplified-Manual.md` -- 在用例 9 阶段 E 增加“无新 Skill 时必须显示指定友好文案”的预期。

**Acceptance Criteria:**
- Given find-skills 已启用且扫描成功返回零候选、零跳过项，when loading 结束，then 同一区块显示“已扫描项目级与全局 Skill 目录，未发现 opencode Skill。”且按钮恢复为“发现”。
- Given 扫描返回候选、跳过原因或错误，when 结果渲染，then 继续显示对应现有反馈且不显示空结果文案。
- Given 用户在已显示空结果后再次扫描，when 新扫描开始，then 旧空结果提示立即消失。
- Given 角色发生切换，when SettingsTab 同步新角色状态，then 不继承上一角色的扫描完成提示。

## Spec Change Log

## Verification

**Commands:**
- `npm run test:frontend -- --run src/components/role/SettingsTab.test.tsx`（工作目录 `egosync-app`）-- 预期 SettingsTab 全部测试通过。
- `npm run build`（工作目录 `egosync-app`）-- 预期 TypeScript 严格检查与 Vite 构建通过。

**Manual checks:**
- 在无项目级/全局 opencode Skill 的环境点击“发现”，确认指定文案出现在按钮所在区块且重新点击时立即清除。

## Suggested Review Order

**扫描结果状态与竞态保护**

- 以角色 ID 快照隔离异步扫描，避免旧角色响应污染当前界面。
  [`SettingsTab.tsx:377`](../../egosync-app/src/components/role/SettingsTab.tsx#L377)

- 成功、失败与完成态均受当前角色守卫，保持 loading 和错误语义一致。
  [`SettingsTab.tsx:392`](../../egosync-app/src/components/role/SettingsTab.tsx#L392)

**空结果反馈**

- 仅对成功且无候选、无跳过项的扫描显示指定友好文案。
  [`SettingsTab.tsx:722`](../../egosync-app/src/components/role/SettingsTab.tsx#L722)

**验收覆盖**

- 成功空结果测试验证文案出现且按钮恢复可发现状态。
  [`SettingsTab.test.tsx:827`](../../egosync-app/src/components/role/SettingsTab.test.tsx#L827)

- 角色切换竞态测试验证旧请求不会显示新角色的扫描提示。
  [`SettingsTab.test.tsx:842`](../../egosync-app/src/components/role/SettingsTab.test.tsx#L842)

- UAT 阶段 E 固化无新 Skill 时的用户可见预期。
  [`UAT-Simplified-Manual.md:717`](../uat/UAT-Simplified-Manual.md#L717)
