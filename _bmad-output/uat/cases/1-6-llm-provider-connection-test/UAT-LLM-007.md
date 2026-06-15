---
用例编号: UAT-LLM-007
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 低
data_contract:
  entities:
    - type: 有效 Anthropic 格式配置
      ref: valid_anthropic
      state: { provider: "anthropic", model: "claude-*" }
      auto_generatable: false
      requirement: 需提供真实有效的 Anthropic 格式服务地址、claude 系列模型名与有效 API Key。
  isolation: write-isolated
  notes: 创建配置写 keyring；测试后删除配置与条目。
---

# UAT-LLM-007 配置 Anthropic 格式 Provider 并测试连接

## 业务场景
部分用户使用 Anthropic（Claude）模型。Anthropic 的接口鉴权方式与 OpenAI 不同，应用需走专门的 Anthropic 路径完成连接测试，让用户同样能验证配置正确。

## 前置条件
- 应用进入「LLM」分页
- 准备真实有效的 Anthropic 格式服务地址、claude 模型名、有效 API Key

## 测试步骤
1. 新增配置，Provider 类型选择「Anthropic」
2. 填入服务地址、claude 系列模型名、有效 API Key
3. 点击「测试连接」
4. 观察连接结果

## 预期结果
- 显示「连接成功」（走 Anthropic 鉴权路径）
- 若 Key 无效则显示明确的鉴权错误，而非崩溃或无反应

## 实际结果

## 测试结论
