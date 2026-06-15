---
用例编号: UAT-EMERG-003
测试模块: 角色涌现建议
story_key: 2-5-role-emergence-suggestion
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 管家已发出涌现建议的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: LLM Provider
      ref: 已配置且连接正常的默认大模型
      state: { 可用: true }
      auto_generatable: false
      requirement: 需用户提供有效的 LLM Provider
    - type: 涌现建议上下文
      ref: 管家已在对话中建议创建某新领域角色
      state: { 建议已出现: true, 被拒领域: 健身/运动 }
      auto_generatable: false
      requirement: 需先通过真实对话触发涌现建议
  isolation: write-isolated
  notes: 拒绝会写入该领域 7 天冷却记录（app_settings），验证后需清理对应冷却记录以免影响后续测试。
---

# UAT-EMERG-003 拒绝建议后管家不再短期重复打扰（业务异常：拒绝冷却）

## 业务场景
用户对管家的建角色提议并不感兴趣，希望管家"懂分寸"——被拒绝后礼貌收回，并且在一段时间内不要反复就同一件事来烦自己，避免变成唠叨。

## 前置条件
- 管家已就某领域（如健身）提议创建角色
- 默认大模型可用

## 测试步骤
1. 在管家的创建建议下明确拒绝（如"不用了"）
2. 观察管家的回应措辞
3. 在拒绝后，继续就同一领域（健身）再聊几轮
4. 观察管家是否会再次提议创建同领域角色

## 预期结果
- 管家礼貌接受拒绝（类似"好的，以后有需要再说"），不纠缠
- 后续同一领域对话中，管家短期内不再重复建议创建该领域角色
- 管家仍能正常回答该领域的具体问题，只是不再提议建角色

## 实际结果
<留空>

## 测试结论
<留空>
