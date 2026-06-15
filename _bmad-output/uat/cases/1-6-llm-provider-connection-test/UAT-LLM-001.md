---
用例编号: UAT-LLM-001
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 有效 OpenAI 兼容 Provider 配置
      ref: valid_openai_provider
      state: { provider: "openai_compatible", base_url: "真实可用的 OpenAI 兼容服务地址", model: "真实可用模型名" }
      auto_generatable: false
      requirement: 需提供一个真实有效的 OpenAI 兼容服务地址、模型名与对应的有效 API Key。
  isolation: write-isolated
  notes: 会创建配置并写入 keyring；测试后删除该配置与对应 keyring 条目。
---

# UAT-LLM-001 用户配置有效 OpenAI 兼容 Provider 并测试连接成功

## 业务场景
用户在设置中添加自己的 OpenAI 兼容模型服务（填入服务地址、密钥、模型名），希望在正式使用前先确认配置正确、网络通畅，避免后续对话时才发现连不上。

## 前置条件
- 应用已启动并进入「全局设置」的「LLM」分页
- 准备好一组真实有效的 OpenAI 兼容服务地址、模型名、API Key

## 测试步骤
1. 点击新增 LLM 配置
2. Provider 类型选择「OpenAI 兼容」
3. 填入服务地址、模型名、有效 API Key，并填写一个配置名称
4. 点击「测试连接」
5. 观察连接测试的状态反馈

## 预期结果
- 测试过程中显示进行中（loading）状态
- 最终显示「连接成功」（绿色对勾或等价成功提示）
- 保存后该配置出现在配置列表中

## 实际结果

## 测试结论
