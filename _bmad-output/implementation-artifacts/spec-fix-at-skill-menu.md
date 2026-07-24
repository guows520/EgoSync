---
title: '修复聊天框 @ 技能选择链路'
type: 'bugfix'
created: '2026-07-23'
baseline_commit: 'd3622496b26f10658ee16014a82f913bcf201586'
status: 'done'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/at-skill-menu-investigation.md'
  - '{project-root}/_bmad-output/planning-artifacts/architecture.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 聊天框通过 `@` 选择 Skill 后，发送按钮会因外层容器增高而偏移；同一角色或管家作用域中，已导入且启用的普通 Skill 不会随配置更新刷新；已启用的 `find-skills`、`skill-creator` 因不属于 registry entry 而无法进入候选及发送校验链路。

**Approach:** 将按钮定位约束到仅包含输入框的相对定位容器；为当前聊天作用域建立语义化 Skill 配置更新通知并刷新候选；新增统一的可选 Skill DTO 与后端解析器，组合 registry Skill 和 meta Skill，同时在发送前按当前作用域重新授权并解析为 OpenCode command name。

## Boundaries & Constraints

**Always:** 保持角色与管家作用域隔离；普通 Skill 继续校验 registry 存在、角色绑定与 enabled IDs；meta Skill 继续以现有 `skills_config` 布尔字段为真源；命令失败必须显式失败，不得降级为普通消息；刷新后已失效的当前选择必须清除。

**Ask First:** 若实现需要数据库迁移、改变 `skills_config` 持久化格式、改变 OpenCode command 协议，或扩大到未导入 Skill 的发现/展示行为，必须暂停确认。

**Never:** 不把 `find-skills`/`skill-creator` 写入 `skills` 表；不把某角色启用解释为全局启用；不使用固定像素补偿按钮位置；不从消息正文反向解析 Skill；不由前端直接提交未经后端授权的 command name；不顺手重构相邻设置或消息流代码。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 普通 Skill | 当前 scope 已绑定并启用 registry Skill | `@` 候选显示并可发送对应 command | 未绑定、未启用或不存在时拒绝 |
| Meta Skill | 当前 scope 启用 `find-skills` 或 `skill-creator` | 候选显示并发送对应 command | 配置关闭或伪造 key 时拒绝 |
| 配置热更新 | ChatStream 保持挂载时导入、绑定、启用或关闭 Skill | 仅匹配 scope 的聊天候选刷新 | 刷新失败保留显式错误，不伪造成功 |
| 选择失效 | 已选择 Skill 随配置刷新被移除 | 自动清除选择，普通输入仍可用 | 不发送失效 Skill |
| 布局 | 已选择 Skill 标签显示在输入框上方 | 发送/停止按钮仍与输入框垂直居中 | 不依赖窗口尺寸或固定偏移 |

</frozen-after-approval>

## Code Map

- `egosync-app/src/components/chat/ChatInput.tsx` -- `@` 候选、选择标签和发送/停止按钮布局。
- `egosync-app/src/components/chat/ChatStream.tsx` -- 当前 scope 候选快照、选择状态及消息/command 提交。
- `egosync-app/src/types/skill.ts`、`egosync-app/src/services/skillService.ts` -- 可选 Skill DTO 与 Tauri IPC 封装。
- `egosync-app/src/components/role/SettingsTab.tsx`、`egosync-app/src/components/butler/ButlerSettingsContent.tsx` -- scope Skill 配置成功保存后的刷新通知入口。
- `egosync-app/src-tauri/src/models/skill.rs`、`src-tauri/src/services/skill_registry.rs`、`src-tauri/src/commands/skill.rs` -- registry/meta 候选组合、scope 授权和 command name 解析。
- `egosync-app/src-tauri/src/commands/chat.rs`、`src-tauri/src/services/agent_engine.rs` -- 结构化选择到 OpenCode command 的现有发送链路。
- `egosync-app/src-tauri/src/main.rs` -- 新 Tauri command 注册入口（若新增 command）。

## Tasks & Acceptance

**Execution:**
- [x] `ChatInput.tsx` 与测试 -- 引入输入框专属定位容器并将选择模型从 registry ID 改为统一 key，确保布局和键盘选择行为稳定。
- [x] `ChatStream.tsx`、设置保存入口与测试 -- 抽取候选重载函数，监听匹配当前 role/Butler scope 的更新通知，并在候选失效时清除选择。
- [x] Skill TS/Rust 类型、Service、Command 与 registry service 测试 -- 返回当前 scope 的 registry + meta 候选，并按统一 key 重新授权后解析 command name。
- [x] Chat command/Agent Engine 相关测试 -- 证明普通与 meta Skill 均走 command 分流，伪造或跨 scope key 在 Runtime 调用前失败，普通消息不受影响。

**Acceptance Criteria:**
- Given Skill 在当前作用域启用，when 用户输入 `@`，then 普通 registry Skill 与已启用 meta Skill 均出现，其他作用域或禁用项不出现。
- Given ChatStream 未卸载，when 当前作用域 Skill 配置成功变化，then 候选自动刷新；其他作用域事件不触发当前候选变化。
- Given 用户已选择 Skill，when 该 Skill 被关闭或移除，then 选择被清空且不能继续按旧 key 执行。
- Given 已选择任意 Skill，when 标签显示或流式状态切换，then 发送/停止按钮仍以输入框自身为垂直定位基准。

## Spec Change Log

## Design Notes

统一 key 使用 `registry:<uuid>`、`meta:find-skills`、`meta:skill-creator`，避免 key 空间碰撞。后端 resolver 是授权边界：前端候选只改善体验，不能替代发送时的 scope 校验。刷新通知应携带 scope kind 与 owner ID；管家使用独立 scope 表达，不伪装为 role ID。

## Verification

**Commands:**
- `npm test -- --run src/components/chat/ChatInput.test.tsx src/components/chat/ChatInput.a11y.test.tsx src/components/chat/ChatStream.test.tsx` -- 目标前端行为通过。
- `cargo test skill_registry` -- registry/meta 候选与授权测试通过。
- `cargo test chat` -- message/command 分流和拒绝路径通过。
- `cargo check` -- Rust/Tauri 编译通过。
- `npm run build` -- TypeScript 与前端生产构建通过。

## Suggested Review Order

**后端授权与命令分流**

- 先确认统一 key 在副作用前完成当前 scope 授权与 command 解析。
  [**chat.rs:226**](../../egosync-app/src-tauri/src/commands/chat.rs#L226)

- 检查 registry 与 meta Skill 合并、伪造 key 拒绝及测试覆盖。
  [**skill_registry.rs:164**](../../egosync-app/src-tauri/src/services/skill_registry.rs#L164)

**前端候选刷新与作用域**

- 查看候选快照、并发代次保护、失效选择清除和刷新失败保留策略。
  [**ChatStream.tsx:989**](../../egosync-app/src/components/chat/ChatStream.tsx#L989)

- 查看设置变更后的作用域通知入口，确认跨 scope 变更广播到所有聊天。
  [**SettingsTab.tsx:94**](../../egosync-app/src/components/role/SettingsTab.tsx#L94)

- 查看管家作用域的导入、移除与刷新通知路径。
  [**ButlerSettingsContent.tsx:1**](../../egosync-app/src/components/butler/ButlerSettingsContent.tsx#L1)

**输入布局与回归证据**

- 确认选择标签位于输入框容器外，按钮仅相对输入框容器定位。
  [**ChatInput.tsx:134**](../../egosync-app/src/components/chat/ChatInput.tsx#L134)

- 查看布局、刷新通知、失败保留和作用域隔离的前端回归测试。
  [**ChatStream.test.tsx:2154**](../../egosync-app/src/components/chat/ChatStream.test.tsx#L2154)

- 查看发送按钮垂直定位及选择行为的组件测试。
  [**ChatInput.test.tsx:96**](../../egosync-app/src/components/chat/ChatInput.test.tsx#L96)
