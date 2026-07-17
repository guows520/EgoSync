---
title: '管家对话优先使用明确使命宣言'
type: 'bugfix'
created: '2026-07-17'
status: 'done'
baseline_commit: '515c1e9d0184c0ea4a0c36b39dae24f2a79cdcc7'
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/mission-statement-not-used-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 用户已保存使命后，管家对话仍只看到角色、任务和记忆，回答“我最看重什么”时继续依据行为推断。该遗漏同时存在于 legacy LLM 和 opencode 链路。

**Approach:** 读取并格式化使命，向两条链路注入同一规则：明确使命是身份、价值观和长期优先级的第一依据；行为只作佐证或用于指出偏离；无使命时才进行带不确定性声明的行为推断。

## Boundaries & Constraints

**Always:** 复用 `db::mission::get_mission`；覆盖 `build_butler_system_prompt` 和 `build_butler_dynamic_prompt`；free 保持原意；structured 渲染为使命、原则和角色目标；读取失败返回 `AppError`；测试验证优先级意图。

**Ask First:** 改变 DB schema、前端保存格式、路由机制或新增 LLM 调用。

**Never:** 不复制为普通记忆；不修改 `mission_inferrer`；不重构相邻代码；不增加缓存；不让行为静默覆盖使命。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 无使命 | 无记录、null 或空白 | 允许行为推断但须说明不确定性，不注入空使命块 | 正常继续 |
| 自由文本使命 | format=free，content 非空 | 两条管家链路均包含原使命和“明确使命优先”规则 | 正常继续 |
| 结构化使命 | structured JSON | 渲染为可读分段 | JSON 异常时保留原文并记录 warning |
| 使命与行为冲突 | 两者方向不一致 | 指出行为可能偏离使命，不得改写价值观 | 正常继续 |
| DB 读取失败 | mission 查询失败 | 与其它必要动态上下文一致，构造 prompt 失败并返回 AppError | 显式失败，不静默跳过使命 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/src/services/agent_engine.rs` -- 两条 prompt builder、格式化助手和单元测试。
- `egosync-app/src-tauri/src/db/mission.rs` -- 既有使命单例读取 API，只调用不修改。
- `egosync-app/src-tauri/migrations/020_mission.sql` -- 测试库复用的既有 schema。
- `_bmad-output/uat/UAT-Simplified-Manual.md` -- 用例 8 步骤 10 的行为验收来源，只读。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 新增统一使命上下文与 structured 格式化，接入两条链路。
- [x] `egosync-app/src-tauri/src/services/agent_engine.rs` -- 测试库加载迁移 020，覆盖无使命、free、structured、优先级和双链路。

**Acceptance Criteria:**
- Given 已设置 free 或 structured 使命，when 任一链路构造上下文，then 包含可读使命并要求相关回答首先依据使命。
- Given 行为资料与使命不一致，when 管家回答用户最看重什么，then prompt 要求把行为描述为偏离或冲突证据，而不是据此覆盖使命。
- Given 未设置使命，when 管家依据行为回答价值观问题，then 将结论表述为推断并说明不确定性。
- Given mission DB 查询失败，when 构造管家上下文，then 返回错误且不得静默退化为“无使命”。

## Spec Change Log

## Design Notes

同一助手生成两条 builder 的使命规则，避免主链路和降级链路漂移：

```text
[使命与价值观依据]
- 已设置使命时，以明确使命为第一依据；行为仅作佐证。
- 冲突时指出行为可能偏离使命，不得用行为覆盖使命。
[用户明确设定的使命宣言]
使命：……
原则：……
角色目标：父亲——……
```

## Verification

**Commands:**
- `cargo fmt --check` -- 修改文件符合 Rust 格式。
- `cargo test services::agent_engine::tests:: --lib` -- agent_engine 测试全部通过。
- `cargo check` -- Rust 后端完整编译通过。

**Manual checks:**
- 按 UAT 用例 8 验证无使命推断、保存使命后使命优先。

## Suggested Review Order

**使命依据与格式边界**

- 先看统一入口如何区分无使命、明确使命和损坏数据。
  [`agent_engine.rs:1150`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1150)

- 检查新旧 structured schema 如何被渲染为可读依据。
  [`agent_engine.rs:1075`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1075)

- 核对明确使命、行为佐证和冲突处理的优先级语义。
  [`agent_engine.rs:1070`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1070)

**双运行链路接入**

- legacy system prompt 在其它行为上下文前注入使命。
  [`agent_engine.rs:1437`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1437)

- opencode dynamic prompt 复用相同使命上下文。
  [`agent_engine.rs:1579`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L1579)

**测试与失败语义**

- 测试库复用生产 mission migration，避免伪造 schema。
  [`agent_engine.rs:5946`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L5946)

- 无使命测试锁定“行为推断必须声明不确定性”。
  [`agent_engine.rs:6852`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L6852)

- 历史 structured 形状在两条链路均保持可读。
  [`agent_engine.rs:6939`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L6939)

- DB 失败测试确保使命读取不会被静默跳过。
  [`agent_engine.rs:6984`](../../egosync-app/src-tauri/src/services/agent_engine.rs#L6984)
