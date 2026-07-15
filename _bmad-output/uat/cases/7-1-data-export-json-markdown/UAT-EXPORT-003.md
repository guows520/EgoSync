---
用例编号: UAT-EXPORT-003
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
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证导出错误处理。
---

# UAT-EXPORT-003 导出失败时显示错误且不崩溃

## 业务场景
用户选了一个不可写的目录导出，希望系统显示错误提示而不是崩溃，自己能重新选目录再试。

## 前置条件
- 应用可访问

## 测试步骤
1. 点击导出
2. 选择一个不可写的目录（如系统保护目录）
3. 观察错误处理

## 预期结果
- 显示红色错误提示框
- 不崩溃、不卡死
- 错误信息可理解
- 可重新选择目录再试

## 实际结果
<留空>

## 测试结论
<留空>
