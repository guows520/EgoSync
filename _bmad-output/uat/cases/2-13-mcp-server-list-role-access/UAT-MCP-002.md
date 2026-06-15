---
用例编号: UAT-MCP-002
测试模块: MCP secret 安全持久化
story_key: 2-13-mcp-server-list-role-access
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 含明文密钥的 MCP 配置
      ref: uat_mcp_plain_secret
      state: { name: "UAT-明文密钥MCP", env_refs: '{"TOKEN":"sk-plain-1234567890"}' }
      auto_generatable: true
      requirement: false
    - type: 含 keyring/空 env 引用的 MCP 配置
      ref: uat_mcp_bad_ref
      state: { cases: 'keyring:calendar-token / env: / env:   ' }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证明文密钥被拒；如个别用例意外保存需删除。可隔离数据目录运行。
---

# UAT-MCP-002 拒绝明文密钥，仅允许 env: 引用

## 业务场景
用户试图把 API key/token 明文填进 MCP 配置，或使用未支持的 keyring 引用/空 env 引用。系统必须拒绝保存，提示只能使用 env: 环境变量引用，secret 不落明文。

## 前置条件
- 应用已启动，进入全局设置 → MCP 工具 → 新增 server。

## 测试步骤
1. 新增 server，环境变量引用填明文 {"TOKEN":"sk-plain-1234567890"}，尝试保存。
2. 观察提示。
3. 改填 {"TOKEN":"keyring:calendar-token"}，尝试保存，观察提示。
4. 改填 {"TOKEN":"env:"} 与 {"TOKEN":"env:   "}，分别尝试保存，观察提示。
5. 改填合法 {"TOKEN":"env:UAT_WEATHER_TOKEN"} 保存。

## 预期结果
- 明文密钥被拒绝，提示「secret 只能保存 env: 引用」类友好信息。
- keyring: 引用被拒绝。
- env 冒号后为空/纯空格被拒绝，提示「env: 引用名称不能为空」。
- 合法 env: 引用可保存成功；secret 不写入数据库明文、opencode.json 明文或日志。

## 实际结果

## 测试结论
