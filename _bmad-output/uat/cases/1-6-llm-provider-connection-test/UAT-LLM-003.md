---
用例编号: UAT-LLM-003
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 本地 Ollama 服务
      ref: ollama_local
      state: { base_url: "http://localhost:11434/v1", model: "本地已拉取的模型名" }
      auto_generatable: false
      requirement: 需在本机运行 Ollama 服务且已拉取至少一个可用模型（用于本地无云端依赖的连接测试）。
  isolation: write-isolated
  notes: 创建本地配置；测试后删除配置。
---

# UAT-LLM-003 配置本地 Ollama 模型并测试连接成功

## 业务场景
注重隐私或希望免费使用的用户会在本机运行 Ollama 本地模型。用户在设置中填入本地地址与本地模型名，期望连接测试能成功，证明可离线使用本地模型。

## 前置条件
- 本机已运行 Ollama 服务，地址 http://localhost:11434/v1
- 本机已拉取至少一个可用模型
- 应用进入「全局设置」的「LLM」分页

## 测试步骤
1. 新增「OpenAI 兼容」配置
2. 服务地址填 http://localhost:11434/v1，模型名填本地已拉取的模型
3. API Key 可留空或填任意值（本地通常不校验）
4. 点击「测试连接」
5. 观察连接结果

## 预期结果
- 显示「连接成功」
- 证明本地模型可被应用调用，无需云端

## 实际结果

## 测试结论
