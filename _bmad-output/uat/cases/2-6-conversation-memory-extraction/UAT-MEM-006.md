---
用例编号: UAT-MEM-006
测试模块: 对话记忆自动提炼
story_key: 2-6-conversation-memory-extraction
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 管家对话
      ref: 含 3 条以上有价值用户消息、正在进行的会话
      state: { 用户消息数: ">=3" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证提炼不阻塞聊天，会写入记忆，建议测试身份/测试 DB 下执行。
---

# UAT-MEM-006 后台提炼不阻塞用户继续聊天

## 业务场景
用户聊完一段话后想立刻接着发下一条消息或切到别的角色继续聊。用户希望系统在背后整理记忆的同时，前台聊天始终顺滑——不能因为"正在记笔记"就让输入框卡住、转圈或无法发送。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 准备一段会触发提炼的有内容对话

## 测试步骤
1. 与管家完成一段有价值对话，使其满足提炼条件
2. 在对话刚结束、提炼可能正在后台进行的窗口内，立即尝试发送下一条消息
3. 同时尝试切换到另一个角色或新建对话
4. 观察输入与切换是否流畅

## 预期结果
- 对话结束后输入框立即可用，能正常发送下一条消息
- 切换角色/新建对话不被记忆提炼阻塞或延迟卡顿
- 不出现因后台提炼导致的界面冻结或长时间无响应

## 实际结果
<留空>

## 测试结论
<留空>
