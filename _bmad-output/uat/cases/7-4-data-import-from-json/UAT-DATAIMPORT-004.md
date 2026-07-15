---
用例编号: UAT-DATAIMPORT-004
测试模块: 数据导入
story_key: 7-4-data-import-from-json
version_anchor: 7bde381
exec_mode: semi
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
    - type: 损坏文件
      ref: 一个非存档格式的文件
      state: { 格式错误: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证错误处理。
---

# UAT-DATAIMPORT-004 导入损坏文件时显示错误不崩溃

## 业务场景
用户选了一个不是存档的文件（或损坏的存档），希望系统显示错误提示而不是崩溃，自己能重新选文件再试。

## 前置条件
- 有一个非存档格式的文件

## 测试步骤
1. 点击"导入存档"
2. 选择非存档文件
3. 观察错误处理

## 预期结果
- 显示红色错误提示
- 不崩溃、不卡死
- 当前数据不被破坏
- 可重新选择文件再试

## 实际结果
<留空>

## 测试结论
<留空>
