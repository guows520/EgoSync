---
用例编号: UAT-SKILL-001
测试模块: 角色 Skill 配置
story_key: 2-10-role-skill-config-proactivity-ui
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_skill_basic
      state: { name: "UAT-技能配置角色", skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 用例会切换角色 Skill 开关并校验持久化，需使用专属测试角色；执行结束后删除该角色，避免污染共享角色库。
---

# UAT-SKILL-001 角色 Skill 开关切换并在重启后保持

## 业务场景
用户希望为某个角色单独控制两个默认能力插件（find-skills 用于发现/推荐 Skill，skill-creator 用于创建/扩展 Skill），并且关掉之后重新打开应用仍然保持关闭状态，不会被悄悄重置。

## 前置条件
- 应用已正常启动，至少存在一个可编辑的测试角色「UAT-技能配置角色」。
- 该角色当前两个 Skill 开关均为开启状态。

## 测试步骤
1. 进入「UAT-技能配置角色」的角色设置页，找到「Skill 配置」区块。
2. 确认区块内展示两个开关：find-skills（来源：Vercel 官方）与 skill-creator（来源：Anthropic 官方），且各自带有名称、来源与简短说明。
3. 关闭 find-skills 开关，观察界面反馈。
4. 关闭 skill-creator 开关，观察界面反馈。
5. 完全退出并重新启动应用。
6. 重新进入「UAT-技能配置角色」的设置页 → Skill 配置区块。

## 预期结果
- 区块标题显示为「Skill 配置」，两个开关均带名称、来源与说明文案。
- 关闭每个开关后界面显示保存成功反馈（内联提示，无独立「保存中...」整行，也无全局弹窗 toast）。
- 重启后两个开关仍保持关闭状态，配置被持久化。

## 实际结果

## 测试结论
