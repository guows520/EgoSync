# Investigation: 模型网络位置与代理绕过

## Hand-off Brief

1. **发生了什么。** 模型配置没有网络位置语义，Rust 直连和 opencode sidecar 两条请求通道都无法按 endpoint 选择是否绕过代理。
2. **当前状态。** 源码根因区域已确认，业务代码未修改；`NO_PROXY` 对 bundled opencode/runtime 的支持仍需一次黑盒验证。
3. **下一步。** 先验证 sidecar 的 bypass 行为，再按“配置字段 + Rust client 路由 + sidecar bypass/restart”实施。

## Case Info
| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-27 |
| Status | Concluded |
| System | Windows / Tauri 2 / React 18 / Rust / opencode sidecar |
| Evidence sources | 用户问题描述、项目源码、CodeGraph 索引、项目上下文、Git 工作区状态 |

## Problem Statement

应用将在公司内网使用。外网大模型服务必须通过 proxy；内网大模型服务经过 proxy 后会被公司网络拦截。期望在大模型配置中增加“模型网络位置”（内网/外网），内网模型调用时不经过 proxy。当前描述属于待源码验证的用户假设。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| 用户问题描述 | Available | 提供目标场景，未附运行日志或网络抓包 |
| `_bmad-output/project-context.md` | Available | 描述 LLM/provider、opencode sidecar 与配置边界 |
| CodeGraph | Available | 索引健康且为最新：2591 files / 37743 nodes |
| 项目源码 | Available | 待沿配置与请求链路取证 |
| 运行日志/抓包 | Missing | 尚不能证明请求当前具体经过哪一种代理注入机制 |
| 自动化测试 | Partial | 待定位相关测试覆盖 |
| Git 工作区 | Partial | 存在用户未提交改动，调查期间不得覆盖 |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 定位模型配置结构、UI、存储与 IPC | High | In Progress | 确认新增字段传播范围 |
| 2 | 定位 proxy 配置来源与作用域 | High | Open | 环境变量、进程环境、HTTP client 或 provider config |
| 3 | 追踪实际 LLM 请求执行者 | High | Open | EgoSync Rust、opencode sidecar 或 provider SDK |
| 4 | 形成最小修改方案与兼容策略 | High | Open | 暂不实现 |
| 5 | 定义验证矩阵与测试边界 | Medium | Open | 内网/外网 × 有/无代理 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-27 | 用户提出内外网模型差异化代理需求 | 用户描述 | Confirmed |
| 2026-07-27 | CodeGraph 索引确认健康且最新 | `codegraph status .` | Confirmed |

## Confirmed Findings

### Finding 1: 架构声明 LLM 请求由 opencode sidecar/provider 执行

**Evidence:** `_bmad-output/project-context.md`

**Detail:** 项目上下文规定前端不得直连 LLM，Rust 后端通过 opencode sidecar 工作；`agent_config.rs` 程序化维护 `opencode.json`。仍需源码验证实际实现是否与文档一致。

## Deduced Conclusions

暂无，等待源码链路证据。

## Hypothesized Paths

### Hypothesis 1: proxy 通过 sidecar 进程环境变量全局注入

**Status:** Open

**Theory:** 若 `HTTP_PROXY`/`HTTPS_PROXY` 在启动 opencode 时全局注入，则所有 provider 请求都会走代理，导致内网模型无法按配置绕过。

**Supporting indicators:** 用户报告表现与进程级全局代理一致。

**Would confirm:** sidecar spawn 代码向 opencode 注入代理环境变量，且没有 provider/request 级 bypass。

**Would refute:** proxy 仅由单个 provider/client 请求级配置控制。

**Resolution:** 待查。

### Hypothesis 2: 应通过 NO_PROXY 动态加入内网模型主机

**Status:** Open

**Theory:** 如果底层 HTTP 栈尊重 `NO_PROXY`，按内网模型 base URL 主机生成 bypass 列表可能是最小变更。

**Supporting indicators:** 常见 HTTP 客户端支持 proxy bypass 环境变量。

**Would confirm:** opencode/provider HTTP 栈明确尊重 `NO_PROXY`，且 sidecar 生命周期允许配置变化后可靠刷新环境。

**Would refute:** 底层忽略 `NO_PROXY`，或多个角色并发使用内外网模型使进程级 bypass 无法安全表达。

**Resolution:** 待查。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 实际请求是否经过代理的运行证据 | 无法将网络拦截机制定为 Confirmed | 调试日志、代理日志或抓包 |
| opencode/provider 对 `NO_PROXY` 的真实支持 | 决定最小方案是否可靠 | 源码/依赖文档或隔离验证 |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | 待定位 |
| Trigger | 选择模型并发起 LLM 调用 |
| Condition | 内网模型请求继承/使用外网代理 |
| Related files | 待 CodeGraph 调查 |

## Conclusion

**Confidence:** Low

当前仅确认需求场景与文档架构，尚未确认 proxy 的代码注入点和实际 LLM 请求执行层。

## Recommended Next Steps

### Fix direction

待源码调查后给出；暂不修改业务代码。

### Diagnostic

优先用静态源码确认配置和进程环境链路；若仍有不确定性，再建议增加诊断日志位置，但未经用户确认不添加。

## Reproduction Plan

待明确 proxy 机制后，建立“外网模型必须经代理成功、内网模型必须直连成功”的双路径验证。

## Side Findings

- Git 工作区已有多处用户改动；本调查不触碰这些文件。

## Follow-up: 2026-07-27

### New Evidence

- `egosync-app/src-tauri/src/models/settings.rs:5-35`：Rust 的配置、创建输入、更新输入均无网络位置字段。
- `egosync-app/src/types/settings.ts:3-29`：前端配置类型同样无该字段。
- `egosync-app/src/components/settings/GlobalSettingsModal.tsx:221-303,629-656`：编辑表单、保存参数和 UI 当前只传播 name/provider/baseUrl/model/apiKey。
- `egosync-app/src-tauri/migrations/024_llm_provider_minimax.sql:4-17`：当前数据库表无网络位置列。
- `egosync-app/src-tauri/src/services/llm_config.rs:126-213`：同步到 opencode 的 provider options 只有 apiKey/baseURL/compatibility 等，无代理路由信息。
- `egosync-app/src-tauri/src/services/sidecar.rs:460-464`：sidecar 启动时明确把 `NO_PROXY` 固定设为 `localhost,127.0.0.1`；代码未清除父进程的 HTTP(S) proxy 环境。
- `egosync-app/src-tauri/src/llm/openai.rs:19-32`、`llm/anthropic.rs:18-30`：模型 HTTP client 使用默认 `reqwest::Client::builder()`，没有按配置调用 `.no_proxy()`。
- `egosync-app/src-tauri/src/services/llm_config.rs:228-295`：获取模型列表另建默认 `reqwest::Client`，且未保存表单只传 provider/baseUrl/apiKey，新增字段若不进入此命令会漏掉内网场景。
- `egosync-app/src-tauri/src/services/agent_bridge.rs:18-24`：EgoSync 到 localhost sidecar 的客户端已经显式 `.no_proxy()`；这只能保护 localhost 桥接，不能控制 sidecar 到模型服务的请求。
- `egosync-app/src-tauri/src/services/agent_engine.rs:2313-2334`、`mission_inferrer.rs:433-444`、`task_classifier.rs:306-317`：多个运行路径从默认配置构造直连 provider，均需统一传播网络位置。
- `egosync-app/src-tauri/src/lib.rs:172-209`：默认 provider 配置先同步，随后 sidecar 启动；启动时具备从 DB 计算 bypass 环境的架构窗口。

### Additional Findings

#### Confirmed root cause area

当前配置模型不能表达“该 endpoint 必须直连”。Rust 直连客户端和 opencode sidecar 都只能继承各自默认代理行为，因此在存在公司代理的运行环境中，没有配置级机制将内网 endpoint 从代理路径排除。

#### Two mechanisms must be fixed

1. Rust `reqwest` 通道：按网络位置创建 client；内网使用 `.no_proxy()`，外网保留当前默认行为。
2. opencode sidecar 通道：进程级生成 proxy bypass。现有固定 `NO_PROXY=localhost,127.0.0.1` 应改为“保留既有 bypass + localhost + 内网模型 endpoint host”。环境变化只有重启 sidecar 后才会生效。

#### Semantic conflict

“内网/外网”实际上是 endpoint 的路由属性，而数据库对象是模型配置。同一 scheme/host/port 若被两个配置分别标成内网和外网，sidecar 的进程级 `NO_PROXY` 无法同时满足两种语义。推荐保存时拒绝这种冲突，而不是静默选择一方。

### Updated Hypotheses

#### Hypothesis 1: proxy 通过 sidecar 进程环境全局影响 provider 请求

**Status:** Confirmed（代码结构）；实际公司环境行为仍缺运行证据。

**Resolution:** sidecar 未清理父进程代理环境，只固定覆盖 `NO_PROXY`；provider 路由不含配置级选择。用户现象与该结构一致。

#### Hypothesis 2: 动态 `NO_PROXY` 可解决 opencode 内网直连

**Status:** Open。

**Resolution:** 代码具备注入环境和重启 sidecar 的能力，但尚无 bundled opencode/runtime 对 `NO_PROXY` 的隔离验证。实现前应先做最小运行实验确认。

### Backlog Changes

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | 模型配置结构、UI、存储与 IPC | High | Done | 字段需端到端传播 |
| 2 | proxy 来源与作用域 | High | Done | Rust client 默认行为 + sidecar 进程环境，两条路径 |
| 3 | 实际 LLM 请求执行者 | High | Done | Rust 直连与 opencode sidecar 并存 |
| 4 | 最小修改方案与兼容策略 | High | Done | 见 Updated Conclusion |
| 5 | 验证矩阵 | Medium | Done | 需隔离代理验证与回归测试 |

### Updated Conclusion

**Confidence: Medium。** 根因区域和缺失机制已由源码确认；唯一关键未确认项是 bundled opencode/runtime 是否按预期遵守动态 `NO_PROXY`。

推荐方案：

1. 新增受限枚举 `ModelNetworkLocation = external | internal`，数据库列 `network_location` 非空，历史数据默认 `external`，保持现有行为。
2. 字段贯穿 migration、Rust model/input、DB CRUD/select、TS types、设置表单、新建/编辑、模型列表参数和连接测试。
3. 建立一个最小的 Rust HTTP client 构造函数：external 保持当前 client；internal 显式 `.no_proxy()`。所有 OpenAI/Anthropic provider 和获取模型列表统一使用，避免遗漏。
4. opencode 启动前，从标记为 internal 的配置解析 endpoint 的 scheme/host/port，生成 bypass；合并而非覆盖现有 `NO_PROXY`，始终包含 localhost。建议同时设置 runtime 实际识别的大小写形式，但须先以隔离实验确定。
5. 当 internal endpoint 集合发生变化时更新 sidecar 环境并重启；仅写 `opencode.json` 不足以改变已运行进程的环境。
6. 对同一 endpoint 的内外网标记冲突执行显式校验失败。

不推荐方案：

- 仅根据 IP/域名自动猜内外网：违背显式配置需求，VPN、split DNS、私有域名会误判。
- 只修改 `NO_PROXY`：会漏掉 Rust 直连 provider 和“获取模型列表”。
- 只给 Rust client `.no_proxy()`：主 Agent 的 sidecar 请求仍会经过代理。
- 内网配置使用 `NO_PROXY=*`：会让同一 sidecar 的外网请求也绕过代理。

### Verification Plan

1. 使用可观测测试代理与两个 mock endpoint，覆盖 external 必须经过 proxy、internal 必须直连。
2. 覆盖 Rust：连接测试、获取模型列表、OpenAI chat、Anthropic chat。
3. 覆盖 sidecar：启动时 internal、启动时 external、运行中从 external 切 internal、修改 internal base URL。
4. 覆盖兼容性：旧数据库迁移后全部为 external；无系统 proxy 时两类配置均正常。
5. 覆盖冲突：同一 scheme/host/port 不允许同时标记 internal/external。

### Remaining Evidence Gap

在实现前执行一次最小黑盒实验：给 sidecar 注入一个不可达代理和目标 host 的 `NO_PROXY`，确认 bundled opencode 发出的 provider 请求确实绕过代理。若不支持，需升级为应用控制的选择性代理/转发层；不要在未验证时直接承诺 `NO_PROXY` 方案有效。



## Follow-up: 2026-07-27 #2

### New Evidence

- 用户确认已在线下验证：动态设置 `NO_PROXY` 后，bundled opencode 的模型请求可以绕过代理。

### Updated Hypotheses

#### Hypothesis 2: 动态 `NO_PROXY` 可解决 opencode 内网直连

**Status:** Confirmed。

**Resolution:** 用户提供既有线下验证结果，确认当前 bundled opencode/runtime 会遵守动态 `NO_PROXY`。因此不再需要应用控制的选择性转发层。

### Updated Conclusion

**Confidence: High。** 源码已确认配置和两条请求通道缺少网络位置路由；线下验证已确认 sidecar 可以通过动态 `NO_PROXY` 绕过代理。推荐实现路径现已具备闭环证据：配置字段、Rust 内网 client `.no_proxy()`、sidecar 合并动态 `NO_PROXY`，以及环境变化后的受控重启。

### Recommended Next Action

先创建一份完整实施 Story，锁定数据迁移、端到端字段传播、sidecar 重启语义、endpoint 冲突规则和验收矩阵；评审确认后再执行实现。该变更跨数据库、React/Tauri IPC、Rust HTTP client 和 sidecar 生命周期，不建议无 Story 直接编码。

## Follow-up: 2026-07-27 #3

### Corrected Design Decision

之前建议以 `scheme + host + port` 判断内外网冲突；Story 设计复核后改为 **host 级冲突**。原因是 sidecar 的 `NO_PROXY` 路由以 host/domain 为可靠公共语义，同一 host 不同 scheme/port 的相反标记无法保证被 runtime 分别处理。旧规则标记为待清理，不与新规则折中合并。
