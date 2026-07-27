# Investigation: DeepSeek 提供商配置保存失败

## Hand-off Brief

1. **发生了什么。** 2026-07-07 的提交 `3b925650` 新增了 DeepSeek 等四个厂商 provider ID，但没有同步扩展 SQLite `llm_configs.provider` CHECK 约束，保存时确定性失败。
2. **当前状态。** 根因已确认，置信度 High；应用层接受七种 provider，而数据库只接受三种，且保存链路原样写入 provider。
3. **下一步。** 新增保留 `network_location` 的表重建迁移，并加入全部新增 provider 的持久化回归测试。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-27 |
| Status | Concluded |
| System | Windows / 项目工作区 |
| Evidence sources | 用户报错、源代码、数据库迁移、版本控制 |

## Problem Statement

用户报告：配置 DeepSeek 提供商的模型并保存时出现 `CHECK constraint failed: provider IN ('openai_compatible', 'anthropic', 'minimax')`。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户提供的数据库错误 | Available | 明确显示 provider 列只接受三个值 |
| 源代码 | Available | 待追踪 |
| 数据库 schema/migrations | Available | 待追踪 |
| 运行日志与数据库实例 | Missing | 尚未提供 |
| 版本控制历史 | Available | 待检查 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 精确错误字符串与数据库约束定义 | High | In Progress | 建立错误源强锚点 |
| 2 | DeepSeek provider 标识的生成与传递 | High | Open | 确认实际写入值 |
| 3 | 保存 LLM 配置的调用链 | High | Open | 确认触发条件 |
| 4 | 近期相关变更 | Medium | Open | 检查 schema 与 UI/后端是否失配 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-27 | 用户保存 DeepSeek 配置时收到 CHECK constraint 错误 | 用户报告 | Confirmed |

## Confirmed Findings

### Finding 1: 数据库拒绝 provider 值

**Evidence:** 用户提供的原始错误

**Detail:** provider 列的 CHECK 约束仅允许 `openai_compatible`、`anthropic`、`minimax`。

## Deduced Conclusions

待调查。

## Hypothesized Paths

### Hypothesis 1: DeepSeek 被保存为独立 provider 值 `deepseek`

**Status:** Open

**Theory:** 应用层新增或暴露了 DeepSeek 选项，但数据库约束没有同步允许该值，或 DeepSeek 本应映射为 `openai_compatible` 却未映射。

**Supporting indicators:** 错误明确指向 provider 枚举约束。

**Would confirm:** 找到保存路径写入 `deepseek`，且 schema 不接受该值。

**Would refute:** 保存路径实际写入允许值，错误来自其他记录或旧数据库 schema。

**Resolution:** 待查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 实际提交的 provider 值 | 区分缺少映射与旧 schema | 追踪保存代码/运行日志 |
| 当前数据库 schema 版本 | 判断迁移是否遗漏或未执行 | 检查迁移与数据库元数据 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | 待查 |
| Trigger | 保存 DeepSeek LLM 配置 |
| Condition | provider 写入值违反数据库 CHECK 约束 |
| Related files | 待查 |

## Conclusion

**Confidence:** Low

当前仅确认数据库约束拒绝了待写入 provider；根因尚待源代码与迁移证据确认。

## Recommended Next Steps

### Fix direction

待调查。

### Diagnostic

追踪错误字符串、数据库约束、provider 映射和保存调用链。

## Reproduction Plan

配置 DeepSeek 模型并保存；预期当前版本稳定复现 CHECK constraint 失败。

## Side Findings

- 无。

## Follow-up: 2026-07-27

### Outcome 2：证据范围扫描

#### New Evidence

- **Confirmed:** 前端类型把 `deepseek` 定义为合法 provider：`egosync-app/src/types/settings.ts:1`。
- **Confirmed:** 设置界面下拉框直接使用 `<option value="deepseek">`：`egosync-app/src/components/settings/GlobalSettingsModal.tsx:657`。
- **Confirmed:** 保存逻辑未经映射地把 `editForm.provider` 放入创建/更新输入：`egosync-app/src/components/settings/GlobalSettingsModal.tsx:285-307`。
- **Confirmed:** 后端 `create_config` 未归一化 provider，直接执行 `provider: input.provider` 后插库：`egosync-app/src-tauri/src/services/llm_config.rs:102-140`。
- **Confirmed:** 当前最新 provider 约束迁移只允许 `openai_compatible`、`anthropic`、`minimax`：`egosync-app/src-tauri/migrations/024_llm_provider_minimax.sql:4-21`。
- **Confirmed:** 后续迁移 025-029 没有扩展 provider；029 仅添加 `network_location`：`egosync-app/src-tauri/migrations/029_llm_config_network_location.sql:1-5`。
- **Confirmed:** 后端输入和持久化模型将 provider 表示为无约束 `String`，没有在数据库前拦截非法值：`egosync-app/src-tauri/src/models/settings.rs:47-80`。

#### Evidence Inventory Update

| Source | Status | Notes |
| --- | --- | --- |
| 用户错误 | Available | 与迁移 024 的约束文本完全一致 |
| 前端 provider 类型与 UI | Available | 支持 7 类 provider，包括 DeepSeek |
| 保存调用链 | Available | UI → Tauri command → service → db insert |
| 数据库 migrations | Available | 数据库只支持 3 类 provider |
| 版本控制 | Partial | 找到相关提交；尚未检查引入额外 provider 的具体 diff |
| 自动化测试 | Partial | 尚未发现覆盖新增 provider 持久化的证据 |
| 运行日志/实际数据库 metadata | Missing | 当前源码已足以解释错误，仍可用于确认安装实例迁移版本 |

#### Backlog Changes

| # | Path | Status |
| --- | --- | --- |
| 1 | 精确错误与数据库约束 | Done |
| 2 | DeepSeek provider 生成与传递 | Done |
| 3 | 保存调用链 | Done |
| 4 | 相关提交与测试覆盖 | In Progress |
| 5 | 反证：是否设计上应统一映射为 openai_compatible | Open |

### Interim Conclusion

证据范围已证明存在跨层契约不一致：前端允许 `deepseek/zhipu/kimi/bailian`，数据库只允许三种 provider，保存链路不做映射。下一阶段需要通过运行时分发代码和提交历史判断正确修复机制：扩展数据库约束，还是把兼容提供商归一化为 `openai_compatible`。

### Outcome 3：因果推理与反证

#### Confirmed Findings

1. 运行时将除 `anthropic` 和特殊 `minimax` 外的 provider 交给通用 `OpenAiProvider`，因此 DeepSeek 的协议实现路径是 OpenAI-compatible：`egosync-app/src-tauri/src/services/llm_config.rs:228-247`。
2. 同步到 OpenCode 时，代码明确区分“EgoSync provider ID”和“OpenCode provider ID”；仅输出阶段把所有 OpenAI-compatible 厂商映射为 `openai`：`egosync-app/src-tauri/src/services/llm_config.rs:266-284`。
3. 模型列表实现也明确声明智谱、DeepSeek、Kimi、百炼属于 OpenAI-compatible，但仍接收原始 provider ID：`egosync-app/src-tauri/src/services/llm_config.rs:387-411`。
4. `zhipu/deepseek/kimi/bailian` 由提交 `3b9256500c5693dff7139979c6139d7a360e3a16`（2026-07-07 17:30:59 +08:00）引入前端类型和 UI；该提交没有新增或修改数据库 migration。
5. 当前 Rust 测试中没有这些四个 provider 的持久化覆盖；搜索命中仅为代码注释，而非测试用例。

#### Hypothesis Resolution

##### Hypothesis 1：新增厂商 ID 后数据库约束迁移遗漏

**Status:** Confirmed

**Resolution:** 提交 3b925650 同时新增厂商级 provider ID、默认地址和运行时兼容处理，但变更文件中没有 migration；数据库仍执行 migration 024 的三值 CHECK。创建路径原样写入 `deepseek`，确定性触发约束失败。

##### Hypothesis 2：DeepSeek 本应在保存前归一化为 `openai_compatible`

**Status:** Refuted（作为主要修复方向）

**Resolution:** 代码注释明确把持久化 ID 称为 EgoSync provider IDs，并只在向 OpenCode 输出时映射；前端还依赖独立 ID 保留厂商选择、标签和默认 Base URL。保存时归一化会丢失厂商身份，并使编辑时回显为“OpenAI兼容”，与当前产品模型冲突。

##### Hypothesis 3：用户数据库迁移未执行或数据库过旧

**Status:** Refuted

**Resolution:** 报错中的三值约束与仓库最新 provider 约束 migration 024 完全一致；后续 025-029 也没有扩展它。即使全部迁移执行成功仍会失败。

#### Causal Chain

`3b925650 新增厂商 provider ID` → `未新增数据库迁移` → `UI 提交 deepseek` → `后端原样持久化` → `migration 024 CHECK 仅接受三值` → `SQLite code 275`。

#### Refutation Pass

主动检查了运行时是否要求统一存储 `openai_compatible`。结果相反：统一映射只发生在 OpenCode 输出边界，内部明确保留 EgoSync provider ID。因此扩展数据库约束与现有架构一致。

### Updated Conclusion

**Confidence:** High

根因已确认：2026-07-07 的 provider UI 增强提交扩大了应用层 provider 合法集合，却遗漏对应 SQLite schema migration 和持久化测试，导致应用层与数据库约束发生确定性契约失配。


### Outcome 4：源码追踪定稿

| Element | Detail |
| --- | --- |
| Error origin | `egosync-app/src-tauri/src/db/settings.rs:28-44`，`insert_llm_config` 原样绑定 `config.provider`，SQLite 返回 CHECK 约束错误 |
| Trigger | `egosync-app/src/components/settings/GlobalSettingsModal.tsx:285-307`，创建或更新表单时提交 `editForm.provider` |
| Condition | provider 为 `zhipu/deepseek/kimi/bailian` 之一，违反 `egosync-app/src-tauri/migrations/024_llm_provider_minimax.sql:7` 的三值约束 |
| Runtime semantics | `egosync-app/src-tauri/src/services/llm_config.rs:228-247` 将新增厂商走通用 OpenAI provider；`266-284` 仅在 OpenCode 输出边界映射为 `openai` |
| Introducing change | Git commit `3b9256500c5693dff7139979c6139d7a360e3a16`，2026-07-07 17:30:59 +08:00；新增 UI/type/service 支持但没有 migration |
| Test gap | `egosync-app/src-tauri/src/db/settings.rs:232-364` 有基础持久化和 network location 测试，但没有新增 provider 的持久化覆盖 |

### Outcome 5：最终结论与交接

## Final Conclusion

**Confidence:** High

根因是应用层 provider 合法集合与 SQLite schema 的确定性契约失配。提交 `3b925650` 将 `zhipu/deepseek/kimi/bailian` 加入前端类型和选择器，保存链路直接把这些值传入数据库；但最新 provider 约束仍来自 migration 024，只接受 `openai_compatible/anthropic/minimax`。这不是用户数据库过旧，也不是 DeepSeek API 行为问题。

## Recommended Next Steps

### Fix direction

1. 新增下一序号 migration（当前为 029，建议 `030_llm_provider_extended.sql`）。
2. 按 SQLite 约束变更惯例重建 `llm_configs`，允许全部七个 provider。
3. 新表必须包含 migration 029 已添加的 `network_location` 列及其 `internal/external` CHECK，并在数据复制语句中保留该列。
4. 不修改已应用的 migration 024，避免 SQLx migration checksum 冲突。
5. 在 `egosync-app/src-tauri/src/db/settings.rs` 或迁移测试区域增加参数化测试，证明四个新增 provider 均可持久化并读取。测试意图是保证 UI 合法值与数据库合法值持续一致，而不只是断言一次 INSERT 成功。

### Diagnostic / Verification

- 执行 `npm run test:all`，覆盖前端 Vitest 与 Rust `cargo test`。
- 在从 migration 029 升级的临时数据库上运行新 migration，确认已有记录和 `network_location` 不丢失。
- 分别保存 `zhipu/deepseek/kimi/bailian` 配置并重新打开设置，确认 provider 标签和 Base URL 回显保持厂商身份。
- 将其中一项设为默认并测试模型列表/连接，确认运行时仍走 OpenAI-compatible 路径。

## Reproduction Plan

### 修复前

1. 打开全局设置并新增 LLM 配置。
2. provider 选择 DeepSeek，填写 API 地址、密钥和模型。
3. 点击保存。
4. 预期稳定出现 `CHECK constraint failed: provider IN ('openai_compatible', 'anthropic', 'minimax')`。

### 修复后

1. 使用已有 migration 029 数据库启动应用，让 migration 030 自动执行。
2. 保存全部四种新增 provider。
3. 预期全部成功，重启后配置、厂商类型和 `network_location` 均保持不变。

## Status

**Concluded.** 根因已确认；没有修改产品代码，也没有执行修复后测试。后续应进入实现流程。
