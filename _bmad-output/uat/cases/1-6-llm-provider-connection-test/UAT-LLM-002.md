---
用例编号: UAT-LLM-002
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 无效 API Key 配置
      ref: invalid_key_provider
      state: { provider: "openai_compatible", base_url: "真实可用的 OpenAI 兼容服务地址", model: "真实可用模型名", api_key: "sk-invalid-000000000000" }
      auto_generatable: true
      requirement: 服务地址与模型名需真实可达；API Key 故意使用无效值以触发 401。
  isolation: write-isolated
  notes: 创建临时配置触发鉴权失败；测试后删除配置及 keyring 条目。
---

# UAT-LLM-002 无效 API Key 测试连接返回明确鉴权错误

## 业务场景
用户不小心填错了 API Key（如复制漏字符或密钥已过期）。点击测试连接时，应用应明确告诉用户"密钥无效"，而不是给出含糊的失败，方便用户快速定位问题。

## 前置条件
- 应用已进入「全局设置」的「LLM」分页
- 服务地址、模型名真实可达
- 准备一个无效 API Key `sk-invalid-000000000000`

## 测试步骤
1. 新增「OpenAI 兼容」配置
2. 填入真实服务地址与模型名，API Key 填入无效值 `sk-invalid-000000000000`
3. 点击「测试连接」
4. 观察错误反馈

## 预期结果
- 测试结果为失败（红色叉或等价失败提示）
- 错误信息明确指向 API Key 无效或未授权（而非泛化的"失败/错误"）

## 实际结果

## 测试结论
