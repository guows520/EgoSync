---
用例编号: UAT-CHAT-001
测试模块: 管家流式对话
story_key: 1-7-butler-first-streaming-conversation
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 已配置默认 LLM
      ref: default_llm
      state: { is_default: true, connection: "正常" }
      auto_generatable: false
      requirement: 需一组真实有效且连接正常的默认 LLM 配置（含有效 API Key），保证能产生真实流式回复。
  isolation: write-isolated
  notes: 会持久化对话与消息到 conversations.db；测试后删除该测试对话。
---

# UAT-CHAT-001 用户与管家完成首次流式逐字对话

## 业务场景
用户已配置好可用的模型。在管家主界面输入一句话，期望看到管家像真人打字一样逐字流式回复，感受到 AI 在"实时思考"，而不是干等很久后一次性蹦出整段文字。

## 前置条件
- 已配置默认 LLM 且连接正常
- 处于管家主界面

## 测试步骤
1. 在管家对话输入框中输入「你好」
2. 按回车发送
3. 观察管家回复的呈现方式与首字出现的快慢

## 预期结果
- 用户消息立即出现在对话区
- 管家回复以逐字流式方式出现（边生成边显示），首字很快出现（感受上几乎无明显卡顿）
- 流式过程中可见"正在输入"光标或等价提示，完成后光标消失

## 实际结果

## 测试结论
