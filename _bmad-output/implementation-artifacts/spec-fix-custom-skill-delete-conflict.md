---
title: '修复自定义 Skill 假删除导致同名冲突'
type: 'bugfix'
created: '2026-07-20'
status: 'done'
baseline_commit: '25217b9ef09e792e117aef63ed654b393b1c082b'
context:
  - '{project-root}/AGENTS.md'
  - '{project-root}/_bmad-output/uat/UAT-Simplified-Manual.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 角色设置页将“删除自定义 Skill”实现成仅解除当前角色绑定，同时显示“已删除”，导致 Skill registry 和受控文件仍存在；用户随后创建同名 Skill 时被查重逻辑拒绝。

**Approach:** 采用方案一，将角色设置页的自定义 Skill 删除动作改为调用现有全局删除命令；删除成功后刷新角色列表和当前 Skill 列表，使所有受影响角色的前端状态与后端保持一致。

## Boundaries & Constraints

**Always:** 保持现有后端 `skill_delete`/`delete_custom_skill` 的全局删除语义；删除操作必须清理 registry、角色绑定和角色配置；界面成功提示必须与真实结果一致；沿用项目既有服务层和错误处理方式。

**Ask First:** 如果实施过程中发现现有 `skill_delete` 无法完成数据库级删除，或必须修改数据库结构、清理用户现场数据、删除历史非受控目录，先征求用户确认。

**Never:** 不直接修改用户 SQLite 数据；不删除 `opencode-workspace/skills` 历史目录；不重构无关 Skill 导入、发现或开关逻辑；不把“关闭 Skill”改造成删除。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 删除自定义 Skill | 当前角色可见一个 `sourceType=custom` Skill | 调用全局删除接口，刷新角色和列表，显示删除成功 | 删除失败时保留当前列表并显示友好错误 |
| 共享 Skill 删除 | Skill 被多个角色启用或绑定 | 所有角色中的该 Skill ID 被后端清理，前端角色状态同步刷新 | 不允许只更新当前角色造成陈旧状态 |
| 非全局移除动作 | opencode Skill 的“从角色移除”操作 | 继续调用 `removeFromRole`，不改变原有语义 | 沿用原有错误处理 |

</frozen-after-approval>

## Code Map

- `egosync-app/src/components/role/SettingsTab.tsx` -- 自定义 Skill 删除确认、服务调用、列表与角色状态刷新。
- `egosync-app/src/components/role/SettingsTab.test.tsx` -- 删除交互和服务调用的前端测试。
- `egosync-app/src/services/skillService.ts` -- 已存在 `delete` 与 `removeFromRole` 两种明确服务接口。
- `egosync-app/src-tauri/src/commands/skill.rs` -- 已存在 `skill_delete` 全局删除命令和同步逻辑，本次默认不修改。
- `egosync-app/src-tauri/src/services/skill_registry.rs` -- `delete_custom_skill` 清理 registry、绑定、角色配置和受控目录。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src/components/role/SettingsTab.tsx` -- 将自定义 Skill 删除改为 `skillService.delete`，并刷新所有活动角色与当前角色 Skill 列表。
- [x] `egosync-app/src/components/role/SettingsTab.test.tsx` -- 更新删除测试，证明调用全局删除而非仅解绑，并验证刷新行为。

**Acceptance Criteria:**
- Given 当前角色存在自定义 Skill，when 用户在“删除自定义 Skill”确认框点击确认删除，then 前端调用 `skill_delete` 对应服务且不调用 `skill_remove_from_role`。
- Given 删除成功可能影响多个角色，when 后端返回成功，then 前端重新读取角色列表并通过既有回调同步所有角色状态。
- Given 删除失败，when 服务返回错误，then 不显示成功提示并向用户显示可理解的错误。
- Given 用户移除 opencode 来源 Skill，when 执行原有移除操作，then 仍只解除当前角色绑定。

## Spec Change Log

## Verification

**Commands:**
- `npm run test:frontend -- --run src/components/role/SettingsTab.test.tsx` -- 27/27 通过。
- `npm run test:frontend -- --run src/App.test.tsx src/components/role/SettingsTab.test.tsx` -- 35/35 通过。
- `npm run build` -- TypeScript 检查和前端生产构建通过。
- `git diff --check` -- 本次差异不存在空白或补丁格式错误。

## Suggested Review Order

**全局删除与状态一致性**

- 删除成功与刷新失败分层处理，避免误导用户重复删除。
  [`SettingsTab.tsx:245`](../../../egosync-app/src/components/role/SettingsTab.tsx#L245)

- 确认文案明确全局删除范围。
  [`SettingsTab.tsx:997`](../../../egosync-app/src/components/role/SettingsTab.tsx#L997)

**创建后运行时刷新（上一轮同批变更）**

- 下一消息入口消费待刷新标记并重启运行时。
  [`agent_engine.rs:2325`](../../../egosync-app/src-tauri/src/services/agent_engine.rs#L2325)

- Skill 创建完成后记录运行时刷新请求。
  [`delegate_bridge.rs:125`](../../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L125)

- 全局事件刷新角色列表。
  [`App.tsx:158`](../../../egosync-app/src/App.tsx#L158)

**回归测试**

- 覆盖全局删除、失败边界及 opencode 仅解绑。
  [`SettingsTab.test.tsx:496`](../../../egosync-app/src/components/role/SettingsTab.test.tsx#L496)

- 验证 registry 更新事件触发角色刷新。
  [`App.test.tsx:172`](../../../egosync-app/src/App.test.tsx#L172)

- 验证运行时刷新标记只消费一次。
  [`delegate_bridge.rs:142`](../../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L142)
