# Investigation: 管家已绑定高德 MCP 但天气工具未暴露

## Hand-off Brief

1. **发生了什么。** 管家已绑定名为 `amap-maps` 的 MCP，提示词也宣称它可用，但该记录实际指向 `https://mcp.api-inference.modelscope.net/invalid-path/mcp`；运行时没有注册任何该 MCP 的具体工具。
2. **当前结论。** 直接故障原因是“绑定成功”与“服务可用”被混为一谈：无效 MCP 配置被写入 OpenCode 并展示为已启用，但工具发现失败后系统没有阻止绑定或暴露降级状态。
3. **下一步建议。** 先用供应商提供的真实高德 MCP endpoint/凭证替换当前占位地址并通过 `initialize + tools/list` 测试；随后补充绑定前后健康校验及运行态工具可见性反馈。

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-07-24 |
| Status | Concluded（暂不实施） |
| System | Windows；EgoSync desktop；OpenCode sidecar；管家会话 |
| Evidence sources | 源码、运行数据库、生成的 opencode.json、对话记录、egosync.log |

## Problem Statement

用户报告：已给管家配置高德地图 MCP，但询问“今天北京的天气怎么样”时，管家回复当前会话没有暴露天气查询工具。

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| `egosync-app/src-tauri/src/services/mcp_server.rs` | Available | 管家绑定、scope 同步、连接测试实现 |
| `egosync-app/src-tauri/src/services/agent_config.rs` | Available | MCP 写入 OpenCode、管家提示词生成 |
| `egosync-app/src-tauri/src/services/agent_engine.rs` | Available | 发送前 scope 同步与 session 创建 |
| `%APPDATA%/com.egosync.desktop/egosync.db` | Available | MCP 配置及管家绑定实态 |
| `%APPDATA%/com.egosync.desktop/opencode-workspace/opencode.json` | Available | 实际生成配置 |
| `%APPDATA%/com.egosync.desktop/conversations.db` | Available | 用户消息、模型思考、最终回复、工具事件 |
| `%APPDATA%/com.egosync.app/egosync.log` | Partial | 有 sidecar/session 日志；OpenCode stderr 仅 debug 级别，缺少 MCP 注册错误详情 |
| 对实际远程 endpoint 发起 HTTP/MCP 探测 | Missing by choice | 用户要求暂不直接执行，因此未发起外部探测 |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-07-19T02:51:03Z | 创建 `amap-maps`，URL 已是 `invalid-path/mcp` | `egosync.db:mcp_servers` | Confirmed |
| 2026-07-24T07:42:58Z | `amap-maps` 被绑定到管家 | `egosync.db:butler_mcp_servers` | Confirmed |
| 2026-07-24T07:42:58Z | OpenCode sidecar 因 MCP 变更重启 | `egosync.log:25128-25134` | Confirmed |
| 2026-07-24T07:43:24Z | 用户询问北京天气；管家 session 正常创建 | `egosync.log:25135-25138` | Confirmed |
| 2026-07-24T07:43:24Z | 模型看见 `amap-maps` 的提示说明，但工具列表中没有具体工具 | `conversations.db`, assistant message `f1d6f37c-...` | Confirmed |
| 2026-07-24T07:43:24Z | 无任何 MCP 工具 process event，最终回复“没有暴露天气查询工具” | `conversations.db:message_process_events` | Confirmed |

## Confirmed Findings

### Finding 1: 管家绑定和配置同步本身已生效

**Evidence:** `egosync.db:butler_mcp_servers` 存在 `amap-maps` 绑定；生成的 `opencode.json` 同时包含 `mcp.amap-maps` 和管家提示词的 `[外部 MCP 工具]` 段。

**Detail:** 因而本次不是“用户没有把 MCP 添加给管家”，也不是“提示词没同步”。

### Finding 2: 当前所谓高德 MCP 指向明显的无效占位地址

**Evidence:** `%APPDATA%/com.egosync.desktop/egosync.db` 与 `opencode-workspace/opencode.json` 均记录 URL：`https://mcp.api-inference.modelscope.net/invalid-path/mcp`。

**Detail:** 当前配置不是可确认的真实高德 MCP endpoint；名称和描述为“高德”不能证明底层服务有效。

### Finding 3: 模型只看到了“已配置”说明，没有看到实际 MCP 工具

**Evidence:** `conversations.db` 中 assistant message `f1d6f37c-87cb-44e3-bdb6-793d34c1c1c0` 的 thinking 明确记录：“系统提示启用了 amap-maps，但具体工具没有出现在函数列表里”；该会话没有工具事件。

**Detail:** 模型的最终回答与运行时工具列表一致，不是天气意图识别失败，也不是模型拒绝调用一个已存在的工具。

### Finding 4: 当前绑定逻辑不验证服务健康

**Evidence:** `egosync-app/src-tauri/src/services/mcp_server.rs:414-425` 的 `add_to_butler` 只检查 `enabled`，写绑定后同步提示词；没有调用 `test_server`。连接测试其实已实现完整的 `initialize` 和 `tools/list`：`egosync-app/src-tauri/src/services/mcp_server.rs:145-201`。

**Detail:** “enabled + bound”仅代表数据库状态，不代表 OpenCode 已发现工具。

### Finding 5: 提示词状态与运行态状态来自不同事实源

**Evidence:** `butler_enabled_mcp_lines` 仅从绑定表和 `enabled` 生成说明：`egosync-app/src-tauri/src/db/mcp_servers.rs:221-227`；`build_butler_entry_with_skills_and_mcp` 无条件把这些行写成“外部 MCP 工具”：`egosync-app/src-tauri/src/services/agent_config.rs:569-574`。

**Detail:** MCP 注册失败时，模型仍会被告知“已启用”，形成“提示里有、工具列表没有”的矛盾。

### Finding 6: sidecar 健康不等于 MCP 健康，且关键错误不可见

**Evidence:** sidecar 启动只等待 OpenCode 服务健康；stdout/stderr 仅以 debug 级别记录：`egosync-app/src-tauri/src/services/sidecar.rs:269-295`。本次 info 日志显示 sidecar 成功启动，但没有 MCP 工具注册结果。

**Detail:** 系统无法向 UI 或用户区分“OpenCode 正常、某个 MCP 失效”。

## Deduced Conclusions

### Deduction 1: 直接故障链

**Based on:** Findings 1-3。

**Reasoning:** 管家绑定已同步 → OpenCode 收到一个指向 `invalid-path` 的 MCP 配置 → 该 server 未向会话注册具体工具 → 模型只能看到提示说明，看不到可调用函数 → 回复“没有暴露工具”。

**Conclusion:** 这不是天气路由问题；首要修复对象是 MCP endpoint/凭证与工具发现健康状态。

### Deduction 2: 产品层缺陷放大了配置错误

**Based on:** Findings 4-6。

**Reasoning:** 绑定流程不做协议探测，提示词又只依据数据库生成，runtime refresh 只验证 sidecar 存活，最终让无效配置以“已启用”姿态进入对话。

**Conclusion:** 即使修正本次 endpoint，未来仍会对任何失效 MCP 复现同类误导，需补健康闭环。

## Hypothesized Paths

### Hypothesis 1: endpoint 是 UAT/占位数据误用于当前桌面数据

**Status:** Open

**Theory:** `invalid-path`、同库中的 `UAT-天气MCP` 及多个 UAT 角色表明当前数据环境混入了 UAT 配置。

**Would confirm:** 找到该记录的创建来源、导入记录或 UAT 脚本操作日志。

**Would refute:** 用户确认该 URL 是供应商实际签发且可通过 MCP `tools/list`。

## Conclusion

**Confidence:** High（症状链已确认）；Medium（未按用户要求对远程地址进行实际 HTTP/MCP 探测）。

根因由两层组成：

1. **直接配置根因：** 管家绑定的 `amap-maps` 指向 `invalid-path/mcp`，运行时没有发现天气工具。
2. **系统设计根因：** EgoSync 把“已启用/已绑定”当作“运行时可用”，未在绑定及重启后校验 `tools/list`，且提示词与实际工具清单可能相互矛盾。

## Recommended Next Steps

### Fix direction

#### A. 配置修复（最小、优先）

1. 用高德/托管平台实际签发的 MCP endpoint 替换当前 `invalid-path` 地址，并补齐所需 header/env 凭证。
2. 保存前执行现有“测试连接”，验收必须包括 `initialize` 成功且 `tools/list` 返回至少一个工具；天气场景还应确认工具清单包含天气能力。
3. 绑定到管家并刷新 runtime；使用新 session 验证工具事件产生。
4. 清理或隔离同库中的 UAT MCP，避免误选。

#### B. 产品修复（推荐后续 story）

1. `add_to_butler` / `add_to_role` 绑定前执行 `test_server`；失败时默认不绑定，或明确进入“已保存但不可用”状态，不能显示为正常启用。
2. runtime restart 后查询 OpenCode 的 MCP/tool 注册状态，并按 server namespace 核对工具数量；缺失时返回“部分失败”。
3. 管家提示词只列出 runtime-ready MCP；若已配置但失效，单列“暂不可用服务”，避免模型误判。
4. UI 增加状态：未测试、可用、不可用、刷新失败；展示最后检测时间及可读错误。
5. MCP 注册失败以 info/warn 结构化日志记录：scope、server key、tool count、错误阶段；不得输出密钥。

### Verification plan

1. 无效 endpoint：绑定操作应失败或显示不可用，不得显示“正常启用”。
2. 有效 MCP：`initialize`、`tools/list` 成功，且 OpenCode 会话工具列表出现对应 namespace/tool。
3. 管家问天气：产生 MCP tool process event，并以工具结果作答。
4. MCP 失效：提示词不得声称其可用；用户收到明确“服务连接失败”，而非笼统“没有暴露工具”。
5. 切换管家/角色 scope：session cache key 改变，工具不串 scope。

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| 远程 URL 的实际 HTTP/MCP 响应 | 将“明显无效”提升为网络层直接确认 | 经用户授权后运行现有 MCP 测试命令 |
| OpenCode 对该 MCP 的原始注册错误 | 精确区分 404、鉴权、协议或工具为空 | 将相关 stderr 提升为 warn，或增加 MCP 状态 API 采集 |
| `amap-maps` 的数据来源 | 判断是 UAT 污染还是人工误配 | 查导入记录、UAT运行记录或操作审计 |

## Side Findings

- 当前工作区存在尚未提交的 Story 10-2 管家 MCP 绑定实现；本次未修改任何代码或运行配置。
- 绑定发生后 sidecar 确实重启并创建了新的管家 session，因此“只是旧 session 缓存未刷新”已被当前证据反驳。
