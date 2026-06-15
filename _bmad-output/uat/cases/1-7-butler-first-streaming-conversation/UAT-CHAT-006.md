---
用例编号: UAT-CHAT-006
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
      requirement: 需真实有效默认 LLM 配置以触发自动标题生成。
  isolation: write-isolated
  notes: 测试后删除测试对话。
---

# UAT-CHAT-006 多对话管理与首条消息后自动生成标题

## 业务场景
用户希望像主流聊天应用一样管理多个话题：能新建对话、在历史列表中看到每个对话的标题和时间、切换和删除对话。并且发出第一条消息后，系统能自动为对话起一个简短贴切的标题，方便日后查找。

## 前置条件
- 默认 LLM 正常
- 处于管家界面

## 测试步骤
1. 新建一个对话，发送第一条有明确主题的消息（如"帮我规划周末出游"）
2. 等待回复完成，观察该对话的标题是否被自动更新
3. 再新建一个对话并发送不同主题消息
4. 打开历史对话列表，查看标题与相对时间（如"刚刚/X分钟前"）
5. 切换回第一个对话，确认内容正确
6. 删除其中一个对话并确认

## 预期结果
- 首条消息发送后，对话标题被自动更新为简短（≤8字）且贴合主题的文字
- 历史列表显示各对话标题 + 相对时间
- 切换对话能正确加载对应历史
- 删除对话后该对话从列表消失

## 实际结果

## 测试结论
