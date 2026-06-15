---
用例编号: UAT-IMPORT-004
测试模块: 自定义 Skill 角色绑定
story_key: 2-11-custom-skill-md-import-role-binding
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 已导入自定义 Skill
      ref: uat_bind_skill
      state: { name: "uat-bind-skill", sourceType: "custom" }
      auto_generatable: true
      requirement: false
    - type: 测试角色
      ref: uat_role_bind
      state: { name: "UAT-绑定角色" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 会修改角色 enabledSkillIds 并触发 agent 同步；执行后删除测试角色与 Skill。
---

# UAT-IMPORT-004 角色启用/禁用自定义 Skill 并持久化

## 业务场景
用户把已导入的自定义 Skill 绑定到某角色，期望角色拥有该能力；禁用后角色不再声明拥有该 Skill。重启后绑定状态保留。

## 前置条件
- Skill 库已存在自定义 Skill「uat-bind-skill」。
- 存在测试角色「UAT-绑定角色」，初始未启用该 Skill。

## 测试步骤
1. 进入「UAT-绑定角色」设置页 Skill 区域的自定义 Skill 列表。
2. 启用 uat-bind-skill。
3. 完全退出并重启应用，确认仍为启用状态。
4. 回到列表禁用 uat-bind-skill。

## 预期结果
- 启用后该角色记录启用的 Skill id，界面显示已启用。
- 重启后启用状态保留。
- 禁用后该角色不再声明拥有该 Skill。
- 切换过程使用内联反馈，无全局 toast。

## 实际结果

## 测试结论
