---
用例编号: UAT-REASON-006
测试模块: 推理透明与不确定性
story_key: 2-9-reasoning-transparency-uncertainty
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 含可引用记忆且大模型可用的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效 LLM Provider
    - type: 已遗忘的测试记忆
      ref: 一条已被遗忘的记忆
      state: { 已遗忘: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证已遗忘记忆不再进入新推理依据。使用测试记忆/测试身份。
---

# UAT-REASON-006 已遗忘记忆不再进入新建议的推理依据

## 业务场景
用户遗忘了一条不准确的记忆后，希望 AI 从此真的"忘了"——后续的建议和"为什么"解释里都不应该再拿这条已被删掉的信息当依据，避免错误记忆继续影响自己。

## 前置条件
- EgoSync 已启动，默认大模型可用
- 存在一条已被遗忘的测试记忆（内容明确可辨识）

## 测试步骤
1. 确认该记忆已被遗忘，不在记忆面板出现
2. 与管家/角色就相关话题发起新对话，引导其给出建议
3. 追问"为什么/依据是什么"
4. 检查新回复及其依据链中是否还引用或体现该已遗忘记忆

## 预期结果
- 新建议与"为什么"解释中不再注入、引用或恢复该已遗忘记忆内容
- AI 的依据链只来自当前仍可见的记忆
- 历史对话中既有的旧引用文本不被篡改

## 实际结果
<留空>

## 测试结论
<留空>
