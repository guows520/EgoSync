---
用例编号: UAT-LLM-004
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 断网或不可达服务配置
      ref: unreachable_provider
      state: { provider: "openai_compatible", base_url: "http://10.255.255.1/v1", model: "any-model" }
      auto_generatable: true
      requirement: 使用不可路由地址或断开网络以触发连接超时（验证 ≤10 秒超时）。
  isolation: write-isolated
  notes: 创建临时配置触发超时；测试后删除配置。
---

# UAT-LLM-004 服务不可达时连接测试在 10 秒内超时并明确报错

## 业务场景
用户的网络断开，或填入的服务地址根本连不通。点击测试连接后，应用不能一直转圈让用户干等，应在合理时间（10 秒内）放弃并明确告知"连接超时，请检查网络"。

## 前置条件
- 应用进入「全局设置」的「LLM」分页
- 使用不可达地址 http://10.255.255.1/v1，或断开网络

## 测试步骤
1. 新增「OpenAI 兼容」配置
2. 服务地址填入不可达地址，模型名任填，API Key 任填
3. 点击「测试连接」并开始计时
4. 观察多久后返回结果，以及错误文案

## 预期结果
- 测试在约 10 秒内结束（不会无限等待）
- 显示明确的超时/网络错误提示，引导用户检查网络

## 实际结果

## 测试结论
