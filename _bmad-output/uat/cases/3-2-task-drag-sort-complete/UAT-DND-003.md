---
用例编号: UAT-DND-003
测试模块: 任务完成
story_key: 3-2-task-drag-sort-complete
version_anchor: 5585768dcab685f56ddec0d45c4d2a0d0a3bd0f2
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条已完成任务 task_completed
      state: { 已完成: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 撤销完成会清除 completed_at，验证后可重新标记完成。
---

# UAT-DND-003 撤销完成恢复任务到原位置

## 业务场景
用户不小心把一条任务点成了完成，希望点绿勾就能撤销，任务回到未完成状态并回到原来的位置，不丢失排序。

## 前置条件
- 某角色下有至少一条已完成任务

## 测试步骤
1. 进入角色工作台任务Tab
2. 展开"已完成 (N)"摘要
3. 点击已完成任务的绿色对勾
4. 观察任务状态与位置变化

## 预期结果
- 任务恢复正常显示（去除灰显与删除线）
- 任务按其原 sort_order 回到未完成列表中对应位置
- 完成时间戳被清除

## 实际结果
<留空>

## 测试结论
<留空>
