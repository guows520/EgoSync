---
title: '修复 Skill 真实执行与创建后即时加载'
type: 'bugfix'
created: '2026-07-19'
status: 'done'
baseline_commit: 'f90581757321605e745ae4b72d7a7a2405035719'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/skill-discovery-and-hot-load-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 角色开启 `find-skills`/`skill-creator` 后，opencode 原生 `skill` 工具仍被强制禁止；模型只能模拟发现或把新 Skill 写到非受控目录，导致无注册、无角色绑定、当前会话不可用。

**Approach:** 将开关与角色绑定转换为按 Skill 名称授权；发现和创建必须真实加载对应元 Skill。创建产物通过 EgoSync 本地鉴权桥进入现有导入事务，并自动绑定、启用到当前角色。

## Boundaries & Constraints

**Always:** `skill` 权限默认拒绝，仅允许当前角色开启的元 Skill 和 `enabledSkillIds` 对应名称；后端事务结果是创建成功的唯一依据；失败必须显式返回且不留下半注册状态；真实工具事件须保留 `tool=skill` 与输入名称；创建成功后下一条消息无需重启应用即可调用，其他角色不可见。

**Ask First:** 若固定 opencode 1.15.10 既不支持热加载，也不能通过 session fork 保留历史；若必须引入或再分发第三方 Skill 资产；若现有导入事务无法在不迁移数据库的情况下保证一致性。

**Never:** 不用提示词冒充执行；不设置全局 `skill=allow`；不监听任意目录自动注册；不通过重启应用生效；不把完整 SKILL.md 常驻塞入系统提示词；不修改无关 UAT 资料。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Skill 发现 | 全开角色请求搜索推荐 Skill | 原生 `skill` 调用的 name 为 `find-skills`，返回真实搜索结果 | 调用失败时说明失败，不伪造候选 |
| Skill 创建 | 全开角色创建合法 `uat-greeting` | 先加载 `skill-creator`，再由 bridge 导入、绑定并启用当前角色 | 校验、写盘、DB 或同步失败均返回失败且不宣称完成 |
| 即时执行 | 创建成功后同一 UI 对话下一条消息调用新 Skill | 产生 `tool=skill/name=uat-greeting` 并正常回复 | 热加载不支持时透明 fork session 并保留历史 |
| 权限隔离 | 全关/部分开启/未绑定角色发起对应请求 | 不暴露或调用未授权 Skill | 返回友好中文边界提示 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_config.rs` -- 生成 agent 权限、提示及 opencode 自定义工具文件。
- `egosync-app/src-tauri/src/services/role_config.rs` -- 元 Skill 开关解析与行为约束。
- `egosync-app/src-tauri/src/services/skill_registry.rs` -- SKILL.md 校验、受控复制、registry 与 scope 事务。
- `egosync-app/src-tauri/src/services/delegate_bridge.rs` -- session→role 鉴权业务桥。
- `egosync-app/src-tauri/src/services/agent_engine.rs` -- opencode session、SSE 工具事件与即时状态。
- `egosync-app/src/components/chat/ChatStream.tsx` -- 持久化工具轨迹的 Skill 名称展示。

## Tasks & Acceptance

**Execution:**
- [x] `agent_config.rs` / `role_config.rs` -- 生成名称级 Skill 白名单并要求相关意图真实加载元 Skill；保留角色隔离。
- [x] `skill_registry.rs` / `delegate_bridge.rs` -- 新增受鉴权 `create_skill` 流程，复用受控导入、绑定和配置同步逻辑。
- [x] `agent_engine.rs` / `ChatStream.tsx` -- 显示具体 Skill 名称；验证热加载，必要时以 session fork 透明续接。
- [x] 对应 Rust/React 测试 -- 覆盖开关组合、角色白名单、创建成功/失败、真实事件名称与即时调用。
- [x] `_bmad-output/uat/UAT-Simplified-Manual.md` -- 仅补充用例 9 的对话创建自动绑定和下一条消息即时调用验收。

**Acceptance Criteria:**
- Given 角色仅开启某一元 Skill，when 构建 agent 配置，then 只授权该名称且其他全局 Skill 被隐藏。
- Given 创建事务成功，when 检查文件、registry、角色配置和 agent 配置，then 四处状态一致且当前角色已启用。
- Given 新 Skill 已创建，when 同一对话下一条消息使用它，then 无需应用重启并产生可审计的真实 Skill 工具事件。
- Given 任一步失败，when 模型收到 bridge 结果，then 用户看到明确失败且不存在“已完成”误报。

## Spec Change Log

## Design Notes

模型只承担 Skill 内容起草和使用判断；请求识别、权限、注册、绑定、失败判定均由确定性代码负责。优先验证当前 session 热加载；仅在证据证明不支持时实现 OpenCode `/session/:id/fork`，不得预先增加复杂度。

## Verification

**Commands:**
- `cd egosync-app/src-tauri && cargo test services::role_config services::agent_config services::skill_registry services::delegate_bridge services::agent_engine` -- 相关 Rust 测试全部通过。
- `cd egosync-app && npm test -- --run src/components/chat/ChatStream.test.tsx` -- Skill 工具轨迹展示测试通过。
- `cd egosync-app/src-tauri && cargo check` -- Rust 编译通过且无跳过声明。

**Manual checks (if no CLI):**
- 使用用例 9 阶段 B 验证真实工具反馈、创建后列表状态及下一条消息即时调用；再切换未绑定角色确认隔离。

## Suggested Review Order

**受控创建与一致性**

- 从受鉴权入口理解校验、导入、绑定、同步与补偿回滚。
  [`delegate_bridge.rs:176`](../../egosync-app/src-tauri/src/services/delegate_bridge.rs#L176)

- 名称目录、legacy 迁移与追加式角色绑定保证 OpenCode 可发现且不夺权。
  [`skill_registry.rs:242`](../../egosync-app/src-tauri/src/services/skill_registry.rs#L242)

- 名称级默认拒绝白名单阻止未授权及 legacy 通配符逃逸。
  [`agent_config.rs:450`](../../egosync-app/src-tauri/src/services/agent_config.rs#L450)

**即时加载与可观察性**

- 下一条消息 fork 旧 session，保留历史并刷新 Skill 权限。
  [`agent_engine.rs:2322`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L2322)

- 原生 Skill 事件从输入提取具体名称供实时反馈。
  [`agent_engine.rs:533`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L533)

- 历史轨迹统一展示“加载 {name} Skill”。
  [`ChatStream.tsx:348`](../../egosync-app/src/components/chat/ChatStream.tsx#L348)

**行为约束与验收**

- 元 Skill 提示要求真实加载及后端成功后才可宣称完成。
  [`role_config.rs:142`](../../egosync-app/src-tauri/src/services/role_config.rs#L142)

- UAT 覆盖自动启用、同会话调用及未绑定角色隔离。
  [`UAT-Simplified-Manual.md:611`](../uat/UAT-Simplified-Manual.md#L611)
