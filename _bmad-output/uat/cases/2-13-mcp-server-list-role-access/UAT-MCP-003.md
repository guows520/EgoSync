---
用例编号: UAT-MCP-003
测试模块: MCP 连接测试与失败降级
story_key: 2-13-mcp-server-list-role-access
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 可达测试 MCP server
      ref: uat_mcp_reachable
      state: { name: "UAT-可达MCP", type: "streamable_http", reachable: true }
      auto_generatable: false
      requirement: 需要一个真实可达、实现 MCP initialize/tools-list 握手的测试 MCP server。
    - type: 不可达/错配 MCP server
      ref: uat_mcp_unreachable
      state: { name: "UAT-故障MCP", url: "https://127.0.0.1:9/mcp", reachable: false }
      auto_generatable: false
      requirement: 需要一个不可达地址或故意错配的 MCP server 用于失败降级验证。
  isolation: write-isolated
  notes: 会创建临时 MCP server 并发起连接测试；执行后删除两个测试 server。
---

# UAT-MCP-003 连接测试成功/失败均不阻塞应用

## 业务场景
用户对 MCP server 点击「测试连接」。可达且实现 MCP 握手的返回成功；不可达或配置错误的返回友好中文错误并允许修改。无论成败都不影响应用启动、角色 CRUD 或普通对话。

## 前置条件
- 已配置一个真实可达的测试 MCP server（实现 initialize→tools/list 握手）。
- 已配置一个不可达/错配的 MCP server。

## 测试步骤
1. 对「UAT-可达MCP」点击测试连接，观察结果与 loading 反馈。
2. 对「UAT-故障MCP」点击测试连接，观察错误提示。
3. 测试失败后，尝试编辑该 server 配置并保存草稿。
4. 测试期间/之后，切换到某角色发起一次普通对话，确认聊天可用。

## 预期结果
- 可达 server 测试成功（仅 HTTP 200 但无 MCP 握手不应误判为成功）。
- 故障 server 测试显示友好中文错误，允许修改；连接测试失败不阻塞保存草稿。
- 测试期间显示 loading 反馈。
- 应用启动、角色 CRUD、普通对话均不受 MCP 故障影响。

## 实际结果

## 测试结论
