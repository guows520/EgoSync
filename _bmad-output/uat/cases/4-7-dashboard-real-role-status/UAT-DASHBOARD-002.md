---
用例编号: UAT-DASHBOARD-002
测试模块: 管家仪表盘
story_key: 4-7-dashboard-real-role-status
version_anchor: 0139d7a5cea8f5df1a202b80f7ba0f5857658226
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 某角色有 Q1 紧急任务，某角色能量 < 40%
      state: { 含Q1: true, 含低能量: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证视觉优先级排序与色谱。
---

# UAT-DASHBOARD-002 角色卡片按需关注程度排序与配色

## 业务场景
用户希望仪表盘自动把"需要关注"的角色排在前面并视觉突出——有 Q1 紧急任务的标"需关注"，能量值低的用红色边框+红色进度条，正常角色用标准色调，让自己一眼看到优先处理谁。

## 前置条件
- 某角色有 Q1 紧急任务
- 某角色能量值 < 40%

## 测试步骤
1. 打开仪表盘
2. 观察角色卡片排序与视觉

## 预期结果
- 有 Q1 紧急任务的角色：灰色边框 + 琥珀色"需关注"文字标签，排在前面
- 能量值 < 40% 的角色：红色边框 + 红色进度条
- 能量值色谱：≥70% 翠绿 / ≥40% 琥珀 / <40% 红色
- 正常角色用标准色调

## 实际结果
<留空>

## 测试结论
<留空>
