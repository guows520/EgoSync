---
用例编号: UAT-EXPORT-001
测试模块: 数据导出
story_key: 7-1-data-export-json-markdown
version_anchor: 22ef393437a68ba7f57284b99c5907fa15a5fcec
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有完整数据的 EgoSync
      state: { 已安装: true, 有角色: true, 有任务: true, 有对话: true }
      auto_generatable: true
      requirement: false
    - type: 导出目录
      ref: 用户选择的空目录
      state: { 可写: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 导出只读不修改源数据。
---

# UAT-EXPORT-001 一键导出全部数据为多种格式

## 业务场景
用户希望随时能把自己的所有数据（角色、任务、记忆、对话、使命宣言、建议、通知、简报、周复盘等）导出为标准格式，确保自己拥有数据的完全控制权且可迁移。

## 前置条件
- 应用有完整数据
- 用户已选择导出目录

## 测试步骤
1. 打开全局设置"数据与主权"Tab
2. 点击"导出存档"按钮
3. 选择导出格式（SQLite备份/JSON/Markdown 可多选）
4. 选择保存目录
5. 等待导出完成

## 预期结果
- 支持三种格式可多选：SQLite备份（直接复制db文件）、JSON（跨版本迁移）、Markdown（人类可读，按角色分章节）
- 导出执行中按钮显示 spinner + "导出中..."
- 完成后显示 ✓ "导出完成" + 文件路径
- 导出文件包含全部数据表
- 不修改源数据

## 实际结果
<留空>

## 测试结论
<留空>
