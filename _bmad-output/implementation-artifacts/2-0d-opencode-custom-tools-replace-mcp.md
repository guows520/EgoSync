# Story 2.0d: opencode 自定义工具替代 MCP — EgoSync 工具注入方案

Status: done

## 背景

Story 2.0c 将对话引擎切换到 opencode Agent Loop 后，EgoSync 自有工具（`create_role`、`delegate_to_role`、`record_emergence_rejection`）需要注入到 opencode 的工具列表中，使 LLM 能在 Agent Loop 中调用。

初始方案采用 MCP（Model Context Protocol）：Tauri 侧启动 MCP HTTP 服务器（端口 4097），opencode 作为 MCP client 连接。该方案在实际运行中遇到了不可绕过的架构问题，最终改用 opencode 原生的自定义工具（Custom Tools）方案。

## 问题诊断过程与根因

### 问题表现

管家对话时，LLM 的 reasoning 明确说"工具列表中没有 `create_role`"，只看到 11 个内置工具。用户要求创建角色时，管家用文字描述了创建过程但没有实际调用工具。

### 诊断过程

1. **配置验证**：确认 `opencode.json` 中 MCP 配置正确，`mcp.egosync.url` 指向 `http://127.0.0.1:4097/mcp`
2. **MCP 连接验证**：opencode 日志显示 `toolCount=3`，确认 MCP 连接成功、工具注册成功
3. **HTTP 代理拦截**：通过 LLM proxy 拦截 opencode 发给 LLM 的 `chat/completions` 请求，发现 `tools` 字段只有 11 个内置工具，**不包含任何 `egosync_*` MCP 工具**

### 根因分析

opencode 使用 **project instance 架构**——每个 `directory` 对应一个独立的 instance，各 instance 有独立的 InstanceState（包括 MCP 连接状态）。

- EgoSync 使用 `opencode-workspace`（位于 `AppData/Roaming/com.egosync.app/opencode-workspace`）作为 session 目录
- opencode sidecar 启动时，默认 project instance 以 sidecar 的 cwd（`src-tauri`）初始化
- MCP 在 `opencode-workspace` instance 中连接成功，但 LLM 请求实际使用的是默认 instance 的工具列表
- **两个 instance 的 MCP state 互不可见**，导致工具注册成功但 LLM 看不到

### 关键经验

> **opencode 的 MCP 工具绑定在 project instance 的 InstanceState 中**。当通过 API 指定 `directory` 创建 session 时，如果该 directory 与 sidecar 默认 project 不同，MCP 工具可能在错误的 instance 中注册。

## 解决方案：自定义工具（Custom Tools）

### 方案选型

| | MCP 方案 | 自定义工具方案 |
|--|----------|---------------|
| 工具注入 | 依赖 opencode instance state（有 bug） | 文件系统直接加载（100% 可靠） |
| 调试难度 | 需理解 Effect 框架 + InstanceState | 简单 HTTP 文件，日志清晰 |
| 角色支持 | 需每个 agent 配 tools permission | `tools: { create_role: false }` 即可 |
| 依赖 | opencode MCP client 正确连接 | 只需文件存在 |
| 额外进程/端口 | 需要端口 4097 的 MCP HTTP 服务器 | `create_role`/`record_emergence_rejection` 无需端口；`delegate_to_role` 使用 Tauri 内置 loopback bridge 的随机端口 |

### 实现架构

```
LLM 决定调用 create_role / record_emergence_rejection
  → opencode 加载 ~/.config/opencode/tools/*.ts
  → execute() 返回 JSON { action: "create_role", ... } 或 { action: "record_emergence_rejection", ... }
  → opencode bus event 发出 message.part.updated { type: "tool", state.output: "..." }
  → Tauri agent_engine 拦截 tool part
  → 解析 action，emit("role:proposed", payload) 或写入冷却记录

LLM 决定调用 delegate_to_role
  → opencode 加载 ~/.config/opencode/tools/delegate_to_role.ts
  → execute() 读取 EGOSYNC_DELEGATE_BRIDGE_PORT / EGOSYNC_DELEGATE_BRIDGE_TOKEN
  → POST http://127.0.0.1:{port}/delegate-to-role，携带 context.sessionID、target_role_id、task_summary、context
  → Tauri delegate_bridge 校验 Bearer token，并通过 sessionID 找到本轮管家 user message
  → 后端执行真实角色委派，写入角色对话与 routing_metadata
  → tool result 直接返回 role_response 给 opencode 管家会话，由管家转述给用户
```

### 自定义工具文件位置

- **全局路径**：`~/.config/opencode/tools/`
- **原因**：opencode 在所有 project instance 中都加载全局工具，不受 `directory` 参数影响
- **注意**：Windows 上 opencode 使用 `~/.config/opencode/`（非标准的 `%APPDATA%`），Rust 中需用 `dirs::home_dir().join(".config/opencode/tools/")`

### opencode Bus Event Tool Part 结构

通过诊断日志确认的实际数据结构（与预期的 `tool-invocation` / `toolInvocation` 完全不同）：

```json
{
  "type": "tool",
  "tool": "create_role",
  "state": {
    "status": "completed",
    "input": { "name": "产品经理", "icon": "briefcase", "..." : "..." },
    "output": "{\"action\":\"create_role\",\"name\":\"产品经理\",...}",
    "time": { "start": 1779955988633, "end": 1779955988644 }
  },
  "id": "prt_...",
  "messageID": "msg_...",
  "sessionID": "ses_..."
}
```

**关键字段映射**：
- 工具名：`part_raw["tool"]`（不是 `toolInvocation.toolName`）
- 工具状态：`part_raw["state"]["status"]`（`pending` / `running` / `completed`）
- 工具输入：`part_raw["state"]["input"]`
- 工具输出：`part_raw["state"]["output"]`（字符串，需 JSON 解析）
- part type：`"tool"`（不是 `"tool-invocation"`）

### 角色 Agent 工具隔离

角色 agent 不应调用管家专属工具。在 `opencode.json` 角色 agent 配置中禁用：

```json
{
  "role-xxx": {
    "tools": {
      "create_role": false,
      "delegate_to_role": false,
      "record_emergence_rejection": false
    }
  }
}
```

Butler agent 不设 `tools` 字段（全部可用）。

### 自定义工具返回值设计

`create_role` 与 `record_emergence_rejection` 的 `execute()` 返回 JSON 字符串，包含 `action` 字段供 Tauri 侧路由，以及 `_instruction` 字段引导 LLM 回复行为：

```typescript
return JSON.stringify({
  action: "create_role",
  name: args.name,
  _instruction: "角色已创建成功。你的回复只需简短确认..."
})
```

`_instruction` 字段不影响 Tauri 侧处理，只被 LLM 当作上下文指令。

`delegate_to_role` 是例外：它不再返回 `{ action: "delegate_to_role" }` 元数据，而是在 custom tool 内部直接调用本地 delegate bridge，并把后端返回的 `role_response` 原样作为 tool result 返回。这样管家 follow-up 使用真实角色回复转述，避免基于空 JSON 元数据生成“已收到/会处理”的伪确认。

```typescript
const response = await fetch(`http://127.0.0.1:${port}/delegate-to-role`, {
  method: "POST",
  headers: {
    "content-type": "application/json",
    "Authorization": `Bearer ${token}`,
  },
  body: JSON.stringify({
    session_id: context.sessionID,
    target_role_id: args.target_role_id.trim(),
    task_summary: args.task_summary.trim(),
    context: args.context?.trim() || undefined,
  }),
  signal: context.abort,
})
const result = await response.json().catch(() => ({ role_response: undefined }))
return result.role_response || "委派失败：后端未返回角色回复。"
```

### Delegate Bridge 环境变量

`delegate_to_role.ts` 依赖 Tauri sidecar 启动时注入两个环境变量：

| 环境变量 | 用途 |
|---|---|
| `EGOSYNC_DELEGATE_BRIDGE_PORT` | 本地 loopback bridge 随机端口 |
| `EGOSYNC_DELEGATE_BRIDGE_TOKEN` | Bearer token，防止其他本地进程伪造委派请求 |

bridge 只监听 `127.0.0.1`，请求体携带 `session_id = context.sessionID`。Tauri 侧通过 `DelegateBridge::register_session(session_id, butler_user_message_id)` 建立 opencode session 与本轮管家 user message 的映射，用于追加 `routing_metadata`。

## 代码变更总结

### 删除
| 文件 | 说明 |
|------|------|
| `services/mcp_host.rs` | MCP Host 整个模块（~745 行）|

### 新增/修改
| 文件 | 改动 |
|------|------|
| `services/mod.rs` | 删除 `pub mod mcp_host` |
| `lib.rs` | 删除 MCP host 启动代码；新增 `write_custom_tools()` 调用；启动 delegate bridge 并在 sidecar 启动前注入 bridge token/port |
| `services/agent_config.rs` | 新增 `write_custom_tools()` + 3 个工具 TS 定义常量；`full_sync()` 移除 MCP 配置写入 + 角色禁用管家工具；butler 清理旧 tools 字段；`delegate_to_role.ts` 通过本地 bridge 返回真实 `role_response` |
| `services/agent_engine.rs` | `message.part.updated` 新增 `"tool"` 拦截分支 + `handle_tool_part()`；system prompt 工具名 `egosync_*` → 直接名称；butler session 注册到 delegate bridge，委派完成后按真实 tool result 转述 |
| `services/delegate_bridge.rs` | 新增本地 loopback bridge，校验 token，按 opencode sessionID 定位管家 user message，执行真实角色委派并返回 `role_response` |
| `services/event_router.rs` | 订阅 opencode 全局 `/event`，按 sessionID 分发事件，避免跨会话 token 污染 |
| `services/sidecar.rs` | 支持注入 bridge token/port 环境变量，并支持 fixed working_dir |

### 配置变更（opencode.json）
- 删除 `mcp` 段
- 删除全局 `tools` 段
- Butler agent 不再有 `tools: { "egosync*": true }`
- 角色 agent 新增 `tools: { create_role: false, ... }`

## 可复用经验

### 1. opencode 自定义工具是最可靠的工具注入方式

MCP 受 project instance 架构影响，工具可见性不可控。自定义工具通过文件系统加载，100% 可靠。**优先使用自定义工具，MCP 仅在需要连接外部服务时考虑。**

### 2. opencode bus event 的 tool part 结构与文档不同

不要假设 `tool-invocation` / `toolInvocation`。实际 part type 是 `"tool"`，结构是 `{ tool, state: { status, input, output } }`。**必须通过诊断日志确认实际结构。**

### 3. Windows 上 opencode 全局配置路径是 `~/.config/opencode/`

不是 Windows 标准的 `%APPDATA%`（`dirs::config_dir()` 返回的路径）。**Rust 中必须用 `dirs::home_dir().join(".config/opencode/")`。**

### 4. `delegate_to_role` 必须返回真实角色回复，而不是仅返回元数据

`delegate_to_role` 与 `create_role` 不同：用户期望角色真的处理任务，并由管家转述结果。因此 custom tool 需要同步调用本地 delegate bridge，等待后端 `execute_delegate_to_role(...)` 完成后返回 `role_response`。只返回 `{ action: "delegate_to_role" }` 会让管家在没有角色结果的情况下生成伪确认。

### 5. 工具返回值中的 `_instruction` 字段可引导 LLM 回复

在 tool result JSON 中添加 `_instruction` 字段，LLM 会将其视为上下文指令。可用于控制 LLM 不复述工具已展示的信息。

### 6. 问题排查必须有证据链

本次排查中，最终定位根因的关键证据是 HTTP 代理拦截的 LLM 请求（证明 tools 列表不含 MCP 工具）和 bus event 的 raw JSON 日志（证明 tool part 结构与预期不同）。**猜测无用，必须拦截实际数据。**

### 7. 降级路径是安全网

opencode 502 时自动降级到直接 LLM，用户体验不中断。**始终保留降级路径，不要让单一依赖的不可用阻塞整个功能。**

## Change Log

- 2026-05-28: MCP 方案替换为 opencode Custom Tools 方案。删除 mcp_host.rs，新增全局自定义工具写入、bus event tool part 拦截、角色工具隔离。140 单元测试通过，端到端验证成功。
- 2026-05-30: `delegate_to_role` custom tool 改为同步调用本地 delegate bridge，并返回真实 `role_response`；新增 bridge token/port 环境变量、sessionID→管家 user message 映射和 routing_metadata 追加路径。
