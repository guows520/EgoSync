---
用例编号: UAT-DISCOVER-002
测试模块: opencode Skill 扫描
story_key: 2-12-opencode-ecosystem-skill-discovery-import
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_find_on
      state: { name: "UAT-发现角色", skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
    - type: opencode skills 目录预置 Skill
      ref: uat_opencode_skill
      state: { location: ".opencode/skills/uat-discover-skill/SKILL.md", name: "uat-discover-skill", description: "用于发现测试的 opencode skill" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需在 opencode 受控/项目级 skills 目录预置合法 Skill；执行后删除预置 Skill 目录与测试角色。
---

# UAT-DISCOVER-002 启用 find-skills 后扫描 opencode Skill 目录

## 业务场景
用户启用 find-skills 后点击「发现 opencode Skill」，期望系统扫描本机 opencode 的项目级与全局 skills 目录，列出可导入 Skill 的名称、说明、来源位置与是否已导入。

## 前置条件
- 测试角色「UAT-发现角色」已启用 find-skills。
- opencode skills 目录中预置一个合法 Skill「uat-discover-skill」。

## 测试步骤
1. 进入该角色设置页 Skill 区域，点击「发现 opencode Skill」。
2. 查看扫描结果列表。

## 预期结果
- 列表展示 uat-discover-skill，包含 name、description、来源位置、sourceType=opencode、是否已导入标记。
- 仅扫描约定 skills 目录，不递归用户全量目录（性能与隐私边界）。

## 实际结果

## 测试结论
