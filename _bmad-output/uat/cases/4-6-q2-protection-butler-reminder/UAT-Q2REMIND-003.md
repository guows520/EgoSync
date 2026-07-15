---
用例编号: UAT-Q2REMIND-003
测试模块: Q2 保护提醒
story_key: 4-6-q2-protection-butler-reminder
version_anchor: 1b21b4bf26e7a663b59928eb148e103c308b8399
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
      ref: 一条 at_risk Q2 任务，有提醒记录
      state: { protection_status: at_risk, 有提醒记录: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证用户处理后提醒停止。
---

# UAT-Q2REMIND-003 用户处理 at_risk 任务后提醒立即停止

## 业务场景
用户看到管家提醒后去处理了这条 Q2 任务（完成或编辑），希望提醒立即停止，不再继续骚扰，protection_status 恢复 normal。

## 前置条件
- 一条 at_risk Q2 任务已有提醒记录

## 测试步骤
1. 编辑该 at_risk 任务（修改任意字段）并保存，或标记完成
2. 等待下一轮 Q2 保护检查
3. 检查提醒记录与 protection_status

## 预期结果
- 用户处理后 protection_status 恢复 normal
- q2_reminders 表中该任务的提醒记录被删除
- 后续不再为该任务生成提醒
- 任务卡片预警标识消失

## 实际结果
<留空>

## 测试结论
<留空>
