---
用例编号: UAT-DESTROY-003
测试模块: 数据销毁
story_key: 7-2-data-destroy-initial-state
version_anchor: fbdbc830f0744ea453e037157f8f398438225a95
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
  notes: 验证确认按钮启用条件。
---

# UAT-DESTROY-003 确认按钮在输入正确文字前禁用

## 业务场景
用户希望销毁确认按钮在输入"确认销毁"四个字之前是禁用的，避免误点造成不可恢复的数据丢失。

## 前置条件
- 应用有数据

## 测试步骤
1. 点击"销毁所有数据"
2. 不输入任何文字，检查确认按钮状态
3. 输入"确认"（不完整），检查按钮状态
4. 输入"确认销毁"（完整），检查按钮状态
5. 输入"销毁"（错误文字），检查按钮状态

## 预期结果
- 不输入或输入不完整时确认按钮禁用
- 输入"确认销毁"四个字后按钮启用
- 输入其他文字按钮仍禁用
- placeholder 提示"输入"确认销毁"以继续"

## 实际结果
<留空>

## 测试结论
<留空>
