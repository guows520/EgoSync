---
用例编号: UAT-EXPORT-002
测试模块: 数据导出
story_key: 7-1-data-export-json-markdown
version_anchor: 22ef393437a68ba7f57284b99c5907fa15a5fcec
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 有角色: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证 Markdown 报告可读性。
---

# UAT-EXPORT-002 Markdown导出按角色分章节人类可读

## 业务场景
用户导出 Markdown 报告希望是按角色分章节的人类可读文档，能直接在 Obsidian 或其他 Markdown 编辑器中阅读，而不是一堆 JSON 字段。

## 前置条件
- 有至少 2 个角色及其数据

## 测试步骤
1. 选择仅 Markdown 格式导出
2. 打开导出的 .md 文件
3. 检查内容结构

## 预期结果
- Markdown 按角色分章节
- 每个角色章节含目标、任务、记忆、对话摘要
- 内容人类可读，非原始 JSON
- 可在 Obsidian/Markdown 编辑器中正常渲染

## 实际结果
<留空>

## 测试结论
<留空>
