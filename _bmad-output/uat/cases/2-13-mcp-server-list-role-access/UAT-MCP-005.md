---
用例编号: UAT-MCP-005
测试模块: opencode.json MCP 同步保留
story_key: 2-13-mcp-server-list-role-access
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试 MCP server 配置
      ref: uat_mcp_persist
      state: { name: "UAT-持久MCP", enabled: true, env_refs: '{"TOKEN":"env:UAT_WEATHER_TOKEN"}' }
      auto_generatable: false
      requirement: 需要测试 MCP server 地址与 env token。
  isolation: write-isolated
  notes: 会改写 opencode.json 外部 mcp 配置并触发 full_sync；执行后删除 server 并清理 opencode.json。建议隔离数据目录运行。
---

# UAT-MCP-005 重启/全量同步不删除用户外部 MCP 且不恢复内部 MCP host

## 业务场景
用户保存了外部 MCP server 后，期望应用重启或全量同步时不会把外部 MCP 配置删掉；同时系统不应恢复已废弃的 EgoSync 内部 MCP host（4097 端口），内部工具继续走 custom tools。

## 前置条件
- 已保存并启用外部 MCP server「UAT-持久MCP」，且已同步到 opencode 配置。

## 测试步骤
1. 确认「UAT-持久MCP」已保存启用。
2. 完全退出并重启应用（触发 full_sync）。
3. 重新进入 MCP 工具列表，确认该 server 仍在。
4. 观察应用是否仍正常启动、内部委派/创建角色等功能是否正常（custom tools 路径）。

## 预期结果
- 重启/全量同步后外部 MCP 配置仍保留，不被删除。
- 旧的 egosync 内部 MCP host 配置不被恢复，4097 端口不重新启用。
- create_role/delegate_to_role 等内部工具继续可用（custom tools）。

## 实际结果

## 测试结论
