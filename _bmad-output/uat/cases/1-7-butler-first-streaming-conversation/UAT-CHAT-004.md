---
用例编号: UAT-CHAT-004
测试模块: 管家流式对话
story_key: 1-7-butler-first-streaming-conversation
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 已配置默认 LLM
      ref: default_llm
      state: { is_default: true, connection: "正常" }
      auto_generatable: false
      requirement: 需真实有效默认 LLM 配置；最好选回复较长的提问以拉长流式窗口便于操作。
  isolation: write-isolated
  notes: 测试后删除测试对话。
---

# UAT-CHAT-004 流式回复进行中再次发送时的并发控制

## 业务场景
管家还在逐字回复上一个问题，用户又急着发了第二条消息。应用不能让两个回复同时乱插，而应等前一个回复完成，或由管家友好提示"我还在想上一个问题……"，避免对话错乱。

## 前置条件
- 默认 LLM 正常
- 处于管家界面

## 测试步骤
1. 发送一个会产生较长回复的问题
2. 在管家回复尚未结束（仍在逐字输出）时，立即输入并发送第二条消息
3. 观察应用的处理方式

## 预期结果
- 不会出现两条回复内容互相穿插错乱
- 表现为：新消息排队等待前一回复完成后处理，或管家提示"我还在想上一个问题……"
- 对话区最终内容有序、可读

## 实际结果

## 测试结论
