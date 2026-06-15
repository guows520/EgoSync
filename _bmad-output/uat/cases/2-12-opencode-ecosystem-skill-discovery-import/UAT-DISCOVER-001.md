---
用例编号: UAT-DISCOVER-001
测试模块: opencode Skill 发现边界
story_key: 2-12-opencode-ecosystem-skill-discovery-import
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_no_find
      state: { name: "UAT-未启用发现角色", skills_config: '{ "find-skills": false, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 仅切换开关并尝试发现，不导入任何 Skill；执行后删除测试角色。
---

# UAT-DISCOVER-001 未启用 find-skills 时阻断发现且不扫描

## 业务场景
用户在未启用 find-skills 的角色上尝试「发现 opencode Skill」。系统应阻断并引导用户先启用 find-skills，且不实际扫描或导入任何 Skill。

## 前置条件
- 存在测试角色「UAT-未启用发现角色」，find-skills 关闭。

## 测试步骤
1. 进入该角色设置页 Skill 区域，点击「发现 opencode Skill」。
2. 观察界面反馈。

## 预期结果
- 显示提示「需要先启用 find-skills 才能发现可用 Skill」并提供启用入口。
- 不展示任何扫描结果，不导入任何 Skill。

## 实际结果

## 测试结论
