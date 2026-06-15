---
用例编号: UAT-IMPORT-002
测试模块: 自定义 Skill 导入校验
story_key: 2-11-custom-skill-md-import-role-binding
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 无效 SKILL.md 文件
      ref: uat_invalid_skill_md
      state: { frontmatter_missing_name: true, description_missing: true }
      auto_generatable: true
      requirement: false
    - type: 空 SKILL.md 文件
      ref: uat_empty_skill_md
      state: { content: "" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 校验失败不应写库；若意外写入需清理。可在隔离数据目录运行。
---

# UAT-IMPORT-002 导入无效/空 SKILL.md 显示友好中文错误

## 业务场景
用户误选了一个缺少 frontmatter 字段或内容为空的 SKILL.md。系统应给出友好的中文提示，而不是暴露技术堆栈，也不创建无效 Skill。

## 前置条件
- 准备两个文件：一个 frontmatter 缺少 name/description 的无效 SKILL.md，一个内容为空的 SKILL.md。
- 应用已启动并进入某角色 Skill 区域。

## 测试步骤
1. 点击「导入自定义 Skill」，选择无效（缺字段）SKILL.md。
2. 观察提示信息。
3. 再次点击导入，选择空内容的 SKILL.md。
4. 观察提示信息。
5. 查看 Skill 列表是否新增条目。

## 预期结果
- 对缺字段文件显示友好中文错误（不含 Rust/JS 堆栈、不含绝对路径或文件全文）。
- 对空文件提示「文件内容为空」类友好信息。
- Skill 列表不新增任何无效条目。
- 文案是否友好、是否不泄露隐私需人工确认。

## 实际结果

## 测试结论
