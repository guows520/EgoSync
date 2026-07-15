---
用例编号: UAT-OVERVIEW-002
测试模块: 管家任务概览
story_key: 3-7-butler-all-roles-task-tab
version_anchor: 76a65ac
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 3 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 3 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 多个角色各有任务
      state: { 跨owner: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证角色多选筛选的前端过滤行为。
---

# UAT-OVERVIEW-002 按角色多选筛选任务概览

## 业务场景
用户只想看某几个角色的任务，希望用多选 chips 快速勾选/取消角色（含"管家"），概览立即按所选 owner 过滤，不用切回单角色视图。

## 前置条件
- 至少 3 个 active 角色，各有任务

## 测试步骤
1. 打开管家任务概览
2. 在角色多选筛选区取消勾选某个角色
3. 观察概览变化
4. 重新勾选该角色
5. 点击"全不选"再"全选"

## 预期结果
- 取消勾选后该角色任务立即从概览消失（前端过滤）
- 重新勾选后任务恢复显示
- 多选支持含"管家"在内的所有 owner
- 角色多选与象限/大石头筛选可叠加生效

## 实际结果
<留空>

## 测试结论
<留空>
