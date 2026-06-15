---
用例编号: UAT-MCP-004
测试模块: 角色级 MCP 访问隔离
story_key: 2-13-mcp-server-list-role-access
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试 MCP server 配置
      ref: uat_mcp_weather
      state: { name: "UAT-天气MCP", enabled: true, env_refs: '{"TOKEN":"env:UAT_WEATHER_TOKEN"}' }
      auto_generatable: false
      requirement: 需要可用测试 MCP server 及其 env token（UAT_WEATHER_TOKEN）。
    - type: 启用 MCP 的角色
      ref: uat_role_mcp_on
      state: { name: "UAT-接入MCP角色", mcp_enabled: "UAT-天气MCP" }
      auto_generatable: true
      requirement: false
    - type: 未启用 MCP 的角色
      ref: uat_role_mcp_off
      state: { name: "UAT-无MCP角色", mcp_enabled: "" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 会为角色绑定/解绑 MCP 并真实对话；执行后删除两个角色与 MCP server，清理会话与 opencode.json 条目。
---

# UAT-MCP-004 角色级 MCP 可用性隔离

## 业务场景
用户为某角色启用了一个外部 MCP server，另一个角色未启用。已启用角色能声明并使用该外部工具；未启用角色不得声明或调用该工具。

## 前置条件
- 已有可用 MCP server「UAT-天气MCP」。
- 「UAT-接入MCP角色」已绑定该 server；「UAT-无MCP角色」未绑定。
- 两个角色均可正常对话。

## 测试步骤
1. 进入「UAT-接入MCP角色」设置页「外部 MCP 工具」区块，确认 UAT-天气MCP 已绑定，且仅展示已绑定 server。
2. 与该角色对话，请求使用天气工具，观察是否能调用外部工具。
3. 进入「UAT-无MCP角色」对话，提出同样请求。
4. 观察未启用角色的反馈。

## 预期结果
- 角色设置页只展示该角色已绑定的 MCP server，可搜索添加/移除绑定。
- 已启用角色可声明并调用该外部工具（工具显示名形如「UAT-天气MCP:xxx」，不泄露原始 namespace）。
- 未启用角色不声明、不调用该工具。

## 实际结果

## 测试结论
