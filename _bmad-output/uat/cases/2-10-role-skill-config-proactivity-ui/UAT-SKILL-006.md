---
用例编号: UAT-SKILL-006
测试模块: Skill 生效边界
story_key: 2-10-role-skill-config-proactivity-ui
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_all_disabled
      state: { name: "UAT-全关角色", skills_config: '{ "find-skills": false, "skill-creator": false }' }
      auto_generatable: true
      requirement: false
    - type: LLM Provider 配置
      ref: uat_llm_provider
      state: { configured: true }
      auto_generatable: false
      requirement: 需配置可用 LLM 使角色进入对话以观察能力边界表现。
  isolation: write-isolated
  notes: 需观察运行态 opencode agent 权限同步效果，使用专属测试角色；执行后删除角色。技术核对（permission.skill=deny）可由测试人员配合查看运行态配置，但业务层以「角色不再声明/执行 Skill 能力」为准。
---

# UAT-SKILL-006 两个元 Skill 全部关闭时角色不再具备任何 Skill 能力

## 业务场景
当用户把某角色的两个默认能力插件都关闭后，该角色在对话中既不能发现/推荐 Skill，也不能创建/扩展 Skill，且不应在自我介绍中声称自己拥有这些能力。

## 前置条件
- 存在测试角色「UAT-全关角色」，两个 Skill 开关均关闭。
- 已配置可用 LLM。

## 测试步骤
1. 进入「UAT-全关角色」对话界面。
2. 发送消息：「你现在能帮我发现新 Skill 或创建 Skill 吗？」。
3. 再发送：「那你直接帮我推荐几个 Skill」。
4. 观察两次回复。

## 预期结果
- 角色明确表示当前不具备发现/创建 Skill 的能力，不进行实际操作。
- 角色不在 prompt/回复中虚假声明自己可用这些 Skill。
- 对话正常进行，无异常。

## 实际结果

## 测试结论
