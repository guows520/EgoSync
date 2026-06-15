---
用例编号: UAT-DISCOVER-003
测试模块: opencode Skill 无效项跳过
story_key: 2-12-opencode-ecosystem-skill-discovery-import
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 测试角色
      ref: uat_role_find_on
      state: { name: "UAT-发现角色", skills_config: '{ "find-skills": true, "skill-creator": true }' }
      auto_generatable: true
      requirement: false
    - type: opencode skills 目录无效条目
      ref: uat_opencode_invalid
      state: { cases: "缺少 SKILL.md / frontmatter 缺 name/description / 不可读文件" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需预置若干无效 Skill 目录；执行后清理预置目录。
---

# UAT-DISCOVER-003 扫描跳过无效 Skill 并给出友好跳过摘要

## 业务场景
opencode skills 目录中混有无效条目（缺 SKILL.md、frontmatter 不全、文件不可读）。扫描时这些无效项不应进入可导入列表，但应以友好中文说明跳过的数量或原因。

## 前置条件
- 测试角色已启用 find-skills。
- skills 目录中同时存在合法与若干无效条目。

## 测试步骤
1. 点击「发现 opencode Skill」。
2. 查看可导入列表与跳过摘要。
3. 若跳过项较多，查看摘要展示形式。

## 预期结果
- 无效项不进入可导入列表。
- 界面以友好中文展示跳过摘要（逐条列出 skill 名/原因，超过若干条折叠计数），不暴露底层堆栈。
- 跳过提示与导入结果提示不互相覆盖。
- 文案是否友好需人工确认。

## 实际结果

## 测试结论
