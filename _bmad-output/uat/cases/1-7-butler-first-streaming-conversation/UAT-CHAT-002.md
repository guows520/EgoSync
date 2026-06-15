---
用例编号: UAT-CHAT-002
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
      requirement: 需真实有效默认 LLM 配置以产生真实回复供持久化验证。
  isolation: write-isolated
  notes: 涉及重启应用；测试后删除测试对话。
---

# UAT-CHAT-002 关闭应用重开后对话历史完整保留

## 业务场景
用户和管家聊到一半，临时关掉了应用。再次打开时，之前的对话内容应当还在，让用户能接着之前的话题继续，不会丢失上下文。

## 前置条件
- 已配置默认 LLM
- 与管家完成过至少一轮完整对话

## 测试步骤
1. 与管家进行 2-3 轮对话，等待最后一条回复完整生成
2. 完全关闭应用
3. 重新启动应用并进入管家界面
4. 查看对话历史

## 预期结果
- 重开后之前的全部对话消息（用户与管家）按原顺序完整保留
- 已完成的管家回复显示为完整内容（无"生成中断"标记）

## 实际结果

## 测试结论
