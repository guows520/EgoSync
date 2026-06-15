---
用例编号: UAT-IMPORT-001
测试模块: 自定义 Skill 导入
story_key: 2-11-custom-skill-md-import-role-binding
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 合法 SKILL.md 文件
      ref: uat_valid_skill_md
      state: { name: "uat-weekly-report", description: "生成周报摘要的自定义技能", frontmatter_valid: true }
      auto_generatable: true
      requirement: false
    - type: 测试角色
      ref: uat_role_import
      state: { name: "UAT-导入角色" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 导入会写入全局 Skill registry 并复制文件到受控目录；需在隔离应用数据目录运行，执行后删除导入的 Skill 与测试角色，并清理受控 skills 目录副本。
---

# UAT-IMPORT-001 导入合法自定义 SKILL.md 并预览、持久化

## 业务场景
用户把自己沉淀的本地 SKILL.md 加入 EgoSync 的 Skill 库，期望系统解析出名称与说明并预览，确认后保存到 Skill 库，重启后仍在。

## 前置条件
- 准备一个合法 SKILL.md，其 frontmatter 含 name=uat-weekly-report、description=生成周报摘要的自定义技能。
- 应用已启动，存在测试角色「UAT-导入角色」。

## 测试步骤
1. 进入「UAT-导入角色」设置页的 Skill 区域，点击「导入自定义 Skill」。
2. 选择准备好的合法 SKILL.md 文件。
3. 查看系统展示的解析预览（名称、说明）。
4. 确认导入。
5. 完全退出并重启应用。
6. 再次进入设置页查看自定义 Skill 列表。

## 预期结果
- 预览正确显示 name=uat-weekly-report、description=生成周报摘要的自定义技能。
- 确认后提示导入成功（内联反馈，无全局 toast）。
- 自定义 Skill 出现在 Skill 列表中，sourceType 为 custom。
- 重启后该 Skill 仍在列表中（已复制到受控目录，不依赖原始文件路径）。

## 实际结果

## 测试结论
