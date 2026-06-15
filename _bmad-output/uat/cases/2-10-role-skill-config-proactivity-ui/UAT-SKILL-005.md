---
用例编号: UAT-SKILL-005
测试模块: 关闭 Skill 阻断
story_key: 2-10-role-skill-config-proactivity-ui
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_block_create
      state: { name: "UAT-阻断创建角色", skills_config: '{ "find-skills": true, "skill-creator": false }' }
      auto_generatable: true
      requirement: false
    - type: LLM Provider 配置
      ref: uat_llm_provider
      state: { configured: true }
      auto_generatable: false
      requirement: 需配置一个可用的 LLM Provider 以触发真实对话流程。
  isolation: write-isolated
  notes: 需真实对话，使用专属测试角色与独立会话；执行后清理测试角色与会话。
---

# UAT-SKILL-005 关闭 skill-creator 时对话中阻断「创建/扩展 Skill」请求

## 业务场景
用户已关闭某角色的 skill-creator 能力。当用户要求该角色「帮我创建/扩展一个新的 Skill」时，角色不应执行创建动作，而应温和说明该能力未启用。

## 前置条件
- 存在测试角色「UAT-阻断创建角色」，其 skill-creator 关闭、find-skills 开启。
- 已配置可用 LLM。

## 测试步骤
1. 进入「UAT-阻断创建角色」的对话界面。
2. 发送消息：「帮我创建一个新的 Skill 来处理周报」。
3. 观察角色的回复内容与界面行为。

## 预期结果
- 角色不执行创建/扩展 Skill 动作，不调用底层 Skill 工具。
- 返回温和中文边界提示，说明 skill-creator 当前未启用。
- 对话正常结束，无报错。

## 实际结果

## 测试结论
