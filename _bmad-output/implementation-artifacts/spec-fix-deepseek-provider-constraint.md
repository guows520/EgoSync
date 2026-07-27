---
title: '修复扩展 LLM provider 无法保存'
type: 'bugfix'
created: '2026-07-27'
baseline_commit: '0c5b78f'
status: 'done'
context:
  - '{project-root}/_bmad-output/implementation-artifacts/investigations/deepseek-provider-save-error-investigation.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** 前端和后端允许 `zhipu`、`deepseek`、`kimi`、`bailian`，但 SQLite `llm_configs.provider` 的最新 CHECK 约束仅允许 `openai_compatible`、`anthropic`、`minimax`，导致新增厂商配置保存时稳定失败。

**Approach:** 新增顺序迁移重建 `llm_configs`，使数据库合法值与应用层一致，同时完整保留现有数据及 migration 029 引入的 `network_location`。增加迁移级回归测试，验证新增 provider 和升级数据均可正常持久化。

## Boundaries & Constraints

**Always:** 使用新 migration 030；允许全部七个现有应用层 provider；重建表时保留每个现有字段、默认值、CHECK 约束和数据；测试必须说明 UI/schema 契约一致性为何重要。

**Ask First:** 若迁移序号 030 已占用、发现生产 schema 与仓库 migration 029 不一致，或修复必须修改前端/运行时 provider 语义，则暂停确认。

**Never:** 不修改已发布 migration 001 或 024；不在保存时把厂商 ID 归一化为 `openai_compatible`；不删除 provider CHECK；不重构无关数据库代码；不触碰 `docs/skills` 嵌套仓库。

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 新增厂商配置 | provider 分别为 `zhipu/deepseek/kimi/bailian` | INSERT 成功且读取值保持厂商 ID | 任一值失败则测试失败 |
| 原有 provider | `openai_compatible/anthropic/minimax` | 行为保持不变 | 回归失败即阻止交付 |
| 升级已有数据 | migration 029 schema，已有配置含 `network_location` | migration 030 后所有字段和值不变 | 数据或列丢失即测试失败 |
| 非法 provider | 未知字符串 | CHECK 约束继续拒绝 | 必须返回数据库约束错误 |

</frozen-after-approval>

## Code Map

- `egosync-app/src-tauri/migrations/024_llm_provider_minimax.sql` -- 当前三值约束及 SQLite 重建表范式，只读参考。
- `egosync-app/src-tauri/migrations/029_llm_config_network_location.sql` -- 最新表结构增量，新迁移必须保留该列。
- `egosync-app/src-tauri/migrations/030_llm_provider_extended.sql` -- 新增迁移，扩展 provider CHECK 并无损复制数据。
- `egosync-app/src-tauri/src/db/pool.rs` -- SQLx migration 初始化及迁移集成测试位置。
- `egosync-app/src-tauri/src/db/settings.rs` -- LLM 配置持久化行为及现有数据库测试惯例。

## Tasks & Acceptance

**Execution:**
- [x] `egosync-app/src-tauri/migrations/030_llm_provider_extended.sql` -- 按现有 SQLite 表重建模式扩展 CHECK，复制全部字段并保留 `network_location`。
- [x] `egosync-app/src-tauri/src/db/pool.rs` -- 添加从 migration 029 状态升级的集成测试，证明数据与网络位置无损且新增 provider 可写入、非法 provider 仍被拒绝。
- [x] `egosync-app/src-tauri/src/db/settings.rs` -- 已评估；迁移集成测试已完整覆盖契约，无需修改，避免重复测试。

**Acceptance Criteria:**
- Given 已完整执行 migration 001-029 的数据库，when 执行 migration 030，then 原有 LLM 配置的所有字段和 `network_location` 均保持不变。
- Given migration 030 已执行，when 保存应用层定义的任一合法 provider，then 数据库接受并原值返回。
- Given migration 030 已执行，when 写入未知 provider，then CHECK 约束仍拒绝该值。
- Given 修复完成，when 运行 Rust 数据库测试及项目完整测试，then 无失败、无跳过且没有修改 provider 运行时分发语义。

## Spec Change Log

## Verification

- `cargo test llm_provider_extension_preserves_data_and_enforces_app_provider_contract --manifest-path egosync-app/src-tauri/Cargo.toml` -- **通过**：1 passed，0 failed，0 ignored。
- `cargo test db:: --manifest-path egosync-app/src-tauri/Cargo.toml` -- **通过**：204 passed，0 failed，0 ignored。
- `npm run test:all --prefix egosync-app` -- 前端 **通过**：42 files、438 tests；Rust 阶段曾受 C 盘仅约 0.59 GB 导致的无诊断编译退出影响。
- `cargo test --manifest-path egosync-app/src-tauri/Cargo.toml -- --test-threads=1` -- **本故事测试通过；全库非绿**：796 passed、6 failed、0 ignored。失败均位于未修改的 `agent_config.rs` / `agent_engine.rs` 既有提示词契约测试，已记录到 deferred work。
- `git diff --check` -- **通过**：无空白错误。

## Suggested Review Order

**数据库契约**

- 先核对七种 provider 与既有字段、默认值及 CHECK 是否完整保留。
  [`030_llm_provider_extended.sql:4`](../../egosync-app/src-tauri/migrations/030_llm_provider_extended.sql#L4)

**迁移回归**

- 再验证真实 001–029 升级、数据无损、原值回读及非法值拒绝。
  [`pool.rs:472`](../../egosync-app/src-tauri/src/db/pool.rs#L472)
