---
用例编号: UAT-SKILL-004
测试模块: 关闭 Skill 阻断
story_key: 2-10-role-skill-config-proactivity-ui
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_block_find
      state: { name: "UAT-阻断发现角色", skills_config: '{ "find-skills": false, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
    - type: LLM Provider 配置
      ref: uat_llm_provider
      state: { configured: true }
      auto_generatable: false
      requirement: 需配置一个可用的 LLM Provider（如有效 API key 经 env 引用），以便角色能进入真实对话流程触发阻断逻辑。
  isolation: write-isolated
  notes: 需要真实发起对话，使用专属测试角色与独立会话；执行后删除测试角色与会话记录。
---

# UAT-SKILL-004 关闭 find-skills 时对话中阻断「发现/推荐 Skill」请求

## 业务场景
用户已关闭某角色的 find-skills 能力。当用户在与该角色对话中明确要求「帮我发现/推荐/搜索可用的 Skill」时，角色不应执行该动作，而应温和说明该能力当前未启用。

## 前置条件
- 存在测试角色「UAT-阻断发现角色」，其 find-skills 关闭、skill-creator 开启。
- 已配置可用 LLM，使角色能够正常对话。

## 测试步骤
1. 进入「UAT-阻断发现角色」的对话界面。
2. 发送消息：「帮我搜索并推荐几个可以用的 Skill」。
3. 观察角色的回复内容与界面行为。

## 预期结果
- 角色不执行 Skill 发现/推荐动作，不调用底层 Skill 工具。
- 角色返回温和的中文边界提示，说明 find-skills 当前未启用（措辞是否友好需人工确认）。
- 对话本身正常结束，不报错、不卡死。

## 实际结果

## 测试结论
