---
用例编号: UAT-OVERVIEW-005
测试模块: 管家任务概览
story_key: 3-7-butler-all-roles-task-tab
version_anchor: 76a65ac
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证新建任务可选归属、默认管家。
---

# UAT-OVERVIEW-005 在概览中新建任务可选归属角色

## 业务场景
管家在概览里想快速记一条任务，希望顶部有"+新建"，新建时可选归属哪个角色（默认管家），不用切到具体角色视图再建。

## 前置条件
- 至少 2 个 active 角色

## 测试步骤
1. 打开任务概览
2. 点击顶部"+新建任务"
3. 在表单中选择归属为某个角色（非管家）
4. 填写任务内容并保存
5. 观察概览与该角色视图

## 预期结果
- 新建表单默认归属为"管家"
- 可切换归属为任意 active 角色
- 保存后任务出现在概览，归属标签为所选角色
- 切到该角色视图，任务也已出现

## 实际结果
<留空>

## 测试结论
<留空>
