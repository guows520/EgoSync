---
用例编号: UAT-DESTROY-002
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
  notes: 仅验证取消，不实际销毁。
---

# UAT-DESTROY-002 取消销毁操作无任何副作用

## 业务场景
用户点了"销毁所有数据"但后悔了，希望随时能取消，取消后数据完好无损，没有任何副作用。

## 前置条件
- 应用有数据

## 测试步骤
1. 点击"销毁所有数据"按钮
2. 在确认弹窗中点击"取消"
3. 检查数据是否完好

## 预期结果
- 取消后确认弹窗关闭
- 数据完好无损
- 无任何副作用（无备份创建、无数据删除）
- 可重新打开设置正常使用

## 实际结果
<留空>

## 测试结论
<留空>
