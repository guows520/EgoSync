---
用例编号: UAT-TASK-005
测试模块: 任务管理
story_key: 3-1-task-crud-role-view
version_anchor: dccfdc609cfa7410f35b9721304d6e311c0307a9
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少两个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 角色A的专属任务 task_role_a
      state: { 归属角色: 角色A }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证角色间任务隔离，不修改数据。
---

# UAT-TASK-005 不同角色的任务相互隔离

## 业务场景
用户有多个角色（如"读书人"和"健身教练"），希望每个角色的任务各自独立，在"读书人"下创建的任务不会出现在"健身教练"的清单里，避免混乱。

## 前置条件
- 至少有两个 active 角色
- 角色A下已有一条任务

## 测试步骤
1. 进入角色A的工作台任务Tab，确认存在该角色任务
2. 切换到角色B的工作台任务Tab
3. 观察角色B的任务列表
4. 切回角色A确认任务仍在

## 预期结果
- 角色A的任务只出现在角色A的任务面板
- 角色B的任务面板不会显示角色A的任务
- 切换角色时不会短暂显示上一个角色的任务（无串台）

## 实际结果
<留空>

## 测试结论
<留空>
