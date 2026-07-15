---
用例编号: UAT-DATATAB-001
测试模块: 数据Tab集成
story_key: 7-3-settings-data-tab-integration
version_anchor: 0c9a1831d0d9d3c28c05ccf48d3ece37228e4ffd
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察布局。
---

# UAT-DATATAB-001 数据Tab布局含导出与危险区域

## 业务场景
用户希望在全局设置中方便地找到数据管理功能——上方是导出区域（标题+说明+导出按钮），下方是危险区域（红色标题+警告+销毁按钮），布局清晰直观。

## 前置条件
- 全局设置可打开

## 测试步骤
1. 打开全局设置
2. 点击"数据与主权"Tab
3. 观察布局

## 预期结果
- 上方显示导出区域：标题 + 说明 + "导出存档"按钮
- 下方显示危险区域：红色标题 + 警告说明 + "销毁所有数据"按钮
- Tab 标题为"数据与隐私"
- 无 mock 残留

## 实际结果
<留空>

## 测试结论
<留空>
