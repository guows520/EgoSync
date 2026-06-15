---
用例编号: UAT-DISCOVER-005
测试模块: 管家 opencode Skill 发现导入
story_key: 2-12-opencode-ecosystem-skill-discovery-import
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 管家设置
      ref: uat_butler_find_on
      state: { butler_skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
    - type: opencode skills 目录预置 Skill
      ref: uat_opencode_skill
      state: { name: "uat-butler-skill", description: "管家发现导入测试" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 管家为全局单例，使用 __butler__ scope；执行后取消导入并恢复管家配置，删除受控副本。
---

# UAT-DISCOVER-005 管家发现/导入/取消导入 opencode Skill

## 业务场景
用户在管家设置页同样可以发现、导入 opencode Skill 并取消导入。从管家取消导入只移除管家配置，不影响 Skill 库与角色绑定。

## 前置条件
- 管家已启用 find-skills。
- opencode skills 目录预置 uat-butler-skill。

## 测试步骤
1. 打开管家设置页的「opencode 生态 Skill」区块，点击发现。
2. 导入 uat-butler-skill，确认管家显示已导入。
3. 从管家取消导入该 Skill。
4. 检查 Skill 库与其它角色是否仍保留该 Skill。

## 预期结果
- 管家可发现并导入 opencode Skill，导入后标记为已导入（__butler__ scope）。
- 从管家取消导入只移除管家配置，registry 与角色绑定不受影响。

## 实际结果

## 测试结论
