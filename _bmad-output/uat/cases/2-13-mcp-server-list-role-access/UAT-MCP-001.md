---
用例编号: UAT-MCP-001
测试模块: MCP server 管理
story_key: 2-13-mcp-server-list-role-access
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试 MCP server 配置
      ref: uat_mcp_weather
      state: { name: "UAT-天气MCP", type: "streamable_http", url: "https://uat-weather-mcp.example.com/mcp", env_refs: '{"TOKEN":"env:UAT_WEATHER_TOKEN"}', enabled: true }
      auto_generatable: false
      requirement: 需要一个可用的测试 MCP server（如 uat-weather-mcp）及其访问地址；其 token 通过环境变量 UAT_WEATHER_TOKEN 注入，不得明文。
  isolation: write-isolated
  notes: 会在全局设置新增/删除 MCP server；执行后删除该 server，并清理 opencode.json 中对应外部 mcp 条目。建议隔离数据目录运行。
---

# UAT-MCP-001 新增外部 MCP server 并保存

## 业务场景
用户在全局设置的「MCP 工具」中新增一个外部 MCP server（如天气服务），填写名称、类型、连接参数、说明、启用状态，期望保存成功，并明确这是外部工具接入而非 EgoSync 内部工具。

## 前置条件
- 应用已启动，能进入全局设置 → MCP 工具标签页。
- 已设置环境变量 UAT_WEATHER_TOKEN，并有可用的测试 MCP server 地址。

## 测试步骤
1. 打开全局设置，点击「MCP 工具」标签。
2. 阅读区块顶部说明，确认提示「这是外部工具接入，不是 EgoSync 内部 create_role/delegate 工具」。
3. 新增 server：名称 UAT-天气MCP，类型选择 HTTP/SSE（streamable_http），URL 填测试地址，环境变量引用填 {"TOKEN":"env:UAT_WEATHER_TOKEN"}，说明任意，启用。
4. 保存。
5. 查看列表是否出现该 server。

## 预期结果
- MCP 工具标签存在，且有明确「外部工具接入」说明文案。
- 表单可填写名称、类型、连接参数、说明、启用状态。
- 保存成功后列表显示「UAT-天气MCP」。

## 实际结果

## 测试结论
