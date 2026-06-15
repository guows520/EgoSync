---
用例编号: UAT-MCP-006
测试模块: MCP 删除与孤儿绑定清理
story_key: 2-13-mcp-server-list-role-access
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 测试 MCP server 配置
      ref: uat_mcp_delete
      state: { name: "UAT-待删MCP", enabled: true, env_refs: '{"TOKEN":"env:UAT_WEATHER_TOKEN"}' }
      auto_generatable: false
      requirement: 需要测试 MCP server 地址与 env token。
    - type: 绑定该 MCP 的角色
      ref: uat_role_mcp_bound
      state: { name: "UAT-绑定待删MCP角色", mcp_enabled: "UAT-待删MCP" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 删除 MCP 会解除所有角色绑定；执行后删除测试角色并清理 opencode.json 条目。
---

# UAT-MCP-006 删除 MCP server 解除所有角色绑定

## 业务场景
用户删除一个被角色绑定的外部 MCP server。删除后该 server 从全局列表移除，并解除所有角色绑定，不留下无法移除的孤儿绑定。

## 前置条件
- 已存在 MCP server「UAT-待删MCP」，且被「UAT-绑定待删MCP角色」绑定。

## 测试步骤
1. 在全局设置 MCP 工具列表删除「UAT-待删MCP」，确认删除提示说明会解除所有角色绑定。
2. 确认删除。
3. 进入「UAT-绑定待删MCP角色」设置页外部 MCP 区块。
4. 查看该角色 MCP 绑定情况。

## 预期结果
- 删除提示说明会从全局列表移除并解除所有角色绑定。
- 删除后该 server 从列表消失。
- 绑定角色不再显示该 MCP，无残留孤儿绑定。

## 实际结果

## 测试结论
