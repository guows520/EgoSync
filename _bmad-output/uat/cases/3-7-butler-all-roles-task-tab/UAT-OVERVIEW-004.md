---
用例编号: UAT-OVERVIEW-004
测试模块: 管家任务概览
story_key: 3-7-butler-all-roles-task-tab
version_anchor: 76a65ac
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 概览中一条角色A的任务
      state: { 归属: 角色A }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证就地编辑不跳转、自动同步原角色。
---

# UAT-OVERVIEW-004 在概览中就地编辑任意角色任务不跳转

## 业务场景
用户在管家概览看到某条角色任务需要改，希望直接点开就编辑，保存后自动同步到原角色，不用切到那个角色视图再改，避免来回跳转打断思路。

## 前置条件
- 概览中至少有一条角色任务

## 测试步骤
1. 打开任务概览
2. 点击某条角色任务的编辑按钮
3. 修改任务标题
4. 保存
5. 观察概览与原角色视图

## 预期结果
- 弹出 TaskModal 编辑模式，预填该任务内容
- 不切换视图、不跳转到原角色
- 保存后任务按 id 落库，role_id/owner_type 不变（自动归属原 owner）
- 概览刷新显示新标题
- 切到原角色视图，该任务也已更新

## 实际结果
<留空>

## 测试结论
<留空>
