---
用例编号: UAT-FORGET-005
测试模块: 选择性遗忘
story_key: 2-8-selective-memory-forget
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含可删除记忆且大模型可用的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider，用于复跑同源对话验证不回流
    - type: 已遗忘的测试记忆
      ref: 一条已被遗忘、其来源对话仍存在的测试记忆
      state: { 已遗忘: true, 来源对话存在: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证同源屏蔽：再次触发同源对话提炼不应把已遗忘记忆写回。使用测试记忆/测试身份，验证后清理任何新增记忆。
---

# UAT-FORGET-005 已遗忘记忆不因旧对话再提炼而回流（业务异常：同源屏蔽）

## 业务场景
用户特意遗忘了一条记忆，最不希望看到的是：因为产生这条记忆的旧对话还在，系统后续又把这条"已经被自己删掉的事"重新记回来。用户希望遗忘是真的算数的，不会被旧对话悄悄翻案。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 存在一条已被遗忘、但其来源对话仍保留的测试记忆

## 测试步骤
1. 确认该记忆已被遗忘且不在记忆面板出现
2. 回到其来源对话，再补充几句不引入新事实的话以触发该对话的再次记忆提炼
3. 等待提炼收尾
4. 查看记忆面板是否重新出现这条已被遗忘的同源记忆

## 预期结果
- 已遗忘记忆不会因旧对话再次提炼而被写回记忆面板
- 该来源对话/消息本身仍完整存在、未被改动
- 用户若日后通过全新消息再次明确表达同一事实，可作为新来源正常记入（不属于本异常范围）

## 实际结果
<留空>

## 测试结论
<留空>
