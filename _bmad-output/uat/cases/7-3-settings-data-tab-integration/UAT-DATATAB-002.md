---
用例编号: UAT-DATATAB-002
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
  notes: 验证导出状态流转。
---

# UAT-DATATAB-002 导出按钮loading与完成状态流转

## 业务场景
用户点击导出后希望看到清晰的状态流转——执行中显示 spinner，完成显示 ✓ 和文件路径，出错显示红色提示，让自己知道操作进展。

## 前置条件
- 有可导出数据

## 测试步骤
1. 点击"导出存档"
2. 选择格式与目录
3. 观察按钮状态变化

## 预期结果
- 点击后按钮变为加载状态（spinner + "导出中..."）
- 完成后显示 ✓ "导出完成" + 文件路径
- 出错时显示红色错误提示框
- 状态流转清晰，无 mock 残留

## 实际结果
<留空>

## 测试结论
<留空>
