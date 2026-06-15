---
用例编号: UAT-CHAT-003
测试模块: 管家流式对话
story_key: 1-7-butler-first-streaming-conversation
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 异常 LLM 配置
      ref: broken_llm
      state: { is_default: true, connection: "异常（断网或余额不足）" }
      auto_generatable: false
      requirement: 需制造真实 LLM 调用失败场景，如断网、Key 失效或余额不足的真实配置。
  isolation: write-isolated
  notes: 制造失败对话；测试后删除测试对话并恢复网络。
---

# UAT-CHAT-003 LLM 报错时以管家自然语言友好提示

## 业务场景
对话时网络突然断开或模型账号余额不足。此时不应弹出刺眼的红色错误框或技术性报错，而应由管家用自然、温和的语气在对话气泡里告诉用户"我现在连不上模型，能帮我检查下配置吗"，保持产品的人格化体验。

## 前置条件
- 默认 LLM 配置存在但会调用失败（断网/Key 失效/余额不足）
- 处于管家界面

## 测试步骤
1. 制造 LLM 调用失败条件（如断开网络）
2. 在管家对话框输入任意消息并发送
3. 观察错误的呈现形式与文案

## 预期结果
- 错误以管家自然语言的对话气泡形式展示（如"我现在连不上模型……"）
- 不出现 toast、红框、snackbar 等技术性错误提示
- 文案友好、引导用户检查配置

## 实际结果

## 测试结论
