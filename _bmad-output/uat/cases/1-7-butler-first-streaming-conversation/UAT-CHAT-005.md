---
用例编号: UAT-CHAT-005
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
      requirement: 需真实有效默认 LLM 配置以产生可中断的真实流式回复。
  isolation: write-isolated
  notes: 测试后删除测试对话。
---

# UAT-CHAT-005 流式回复进行中点击停止可中断且已生成内容保留

## 业务场景
管家正在长篇回复，用户发现方向不对或已得到想要的信息，想立刻打断。发送按钮此时应变成停止按钮，点击后管家立即停笔，且已经说出来的部分内容保留在对话里，不会凭空消失。

## 前置条件
- 默认 LLM 正常
- 处于管家界面

## 测试步骤
1. 发送一个会产生较长回复的问题
2. 在管家逐字回复过程中，观察发送按钮是否变为停止按钮
3. 点击停止按钮
4. 观察回复是否立即停止、已生成内容是否保留
5. 关闭并重开应用，确认被中断的内容是否仍保留

## 预期结果
- 流式过程中发送按钮变为停止按钮（同色系）
- 点击停止后回复立即终止
- 已生成的部分内容保留在对话气泡中，并被持久化（重开后仍在）

## 实际结果

## 测试结论
