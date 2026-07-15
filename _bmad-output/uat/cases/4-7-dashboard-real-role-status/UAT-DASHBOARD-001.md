---
用例编号: UAT-DASHBOARD-001
测试模块: 管家仪表盘
story_key: 4-7-dashboard-real-role-status
version_anchor: 0139d7a5cea8f5df1a202b80f7ba0f5857658226
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 3 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 3 }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 各角色有不同数量待办与活跃状态
      state: { 跨角色: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 仅观察仪表盘展示。
---

# UAT-DASHBOARD-001 仪表盘展示所有角色实时状态

## 业务场景
用户在管家仪表盘希望一目了然看到所有活跃角色的实时状态——图标、名称、能量值百分比+进度条、待办任务数、最近活跃时间，快速判断哪些角色需要关注。

## 前置条件
- 至少 3 个 active 角色，各有任务

## 测试步骤
1. 切换到管家视角
2. 点击"仪表盘"Tab
3. 观察角色卡片展示

## 预期结果
- 显示与活跃角色数相等的角色卡片
- 每张卡片含：角色图标+名称、能量值百分比+进度条、待办任务数、最近活跃时间
- 数据为真实聚合（非硬编码 mock）
- 待办任务数与该角色实际未完成任务数一致

## 实际结果
<留空>

## 测试结论
<留空>
