---
用例编号: UAT-TASK-003
测试模块: 任务管理
story_key: 3-1-task-crud-role-view
version_anchor: dccfdc609cfa7410f35b9721304d6e311c0307a9
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条待删除任务 task_to_delete
      state: { 已存在: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证软删除语义：删除后列表移除，但数据库仍保留记录（deleted_at 已写入）。
---

# UAT-TASK-003 删除任务后列表立即移除且为软删除

## 业务场景
用户完成或放弃某条任务后想删掉它，希望确认删除后任务从列表消失，但又担心误删——系统应保留记录可追溯，而不是物理抹除。

## 前置条件
- 某角色下已存在至少一条任务

## 测试步骤
1. 进入角色工作台的"任务清单"Tab
2. 在某条任务卡片上触发删除操作
3. 在确认弹窗中点击"确认删除"
4. 观察任务列表变化
5. 关闭并重新打开任务面板，确认该任务不会重新出现

## 预期结果
- 确认删除后，该任务立即从列表中移除
- 重新打开任务面板，该任务不再出现（默认不返回已软删除任务）
- 数据库层面记录仍保留（deleted_at 已写入），可追溯

## 实际结果
<留空>

## 测试结论
<留空>
