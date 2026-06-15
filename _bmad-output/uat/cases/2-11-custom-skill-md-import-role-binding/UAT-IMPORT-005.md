---
用例编号: UAT-IMPORT-005
测试模块: 元 Skill 向后兼容
story_key: 2-11-custom-skill-md-import-role-binding
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_compat
      state: { name: "UAT-兼容角色", skills_config: '{ "find-skills": true, "skill-creator": true }', enabled_custom_skill: "uat-compat-skill" }
      auto_generatable: true
      requirement: false
    - type: 已导入自定义 Skill
      ref: uat_compat_skill
      state: { name: "uat-compat-skill", sourceType: "custom" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证切换元 Skill 不丢失自定义 Skill 绑定；执行后删除角色与 Skill。
---

# UAT-IMPORT-005 切换元 Skill 不丢失自定义 Skill 绑定

## 业务场景
用户的角色同时启用了两个默认元 Skill 与一个自定义 Skill。用户切换 find-skills/skill-creator 开关时，不应把已绑定的自定义 Skill 弄丢（旧配置结构自动兼容升级）。

## 前置条件
- 测试角色「UAT-兼容角色」已启用自定义 Skill「uat-compat-skill」，且两个元 Skill 开关均开启。

## 测试步骤
1. 进入「UAT-兼容角色」设置页 Skill 区域。
2. 确认自定义 Skill「uat-compat-skill」已启用。
3. 关闭 find-skills 开关。
4. 再打开 find-skills、关闭 skill-creator。
5. 查看自定义 Skill 列表中 uat-compat-skill 的启用状态。
6. 重启应用后再次查看。

## 预期结果
- 在反复切换元 Skill 开关后，自定义 Skill uat-compat-skill 始终保持启用，未被丢弃。
- 元 Skill 开关状态按操作正确变化。
- 重启后绑定与开关状态均保留。

## 实际结果

## 测试结论
