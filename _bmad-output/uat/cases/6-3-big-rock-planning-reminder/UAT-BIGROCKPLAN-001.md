---
用例编号: UAT-BIGROCKPLAN-001
测试模块: 大石头规划提醒
story_key: 6-3-big-rock-planning-reminder
version_anchor: d7c489b
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有 active 角色的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 本周无大石头任务
      state: { 本周大石头数: 0 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后清理通知。
---

# UAT-BIGROCKPLAN-001 本周无大石头时管家提醒规划

## 业务场景
用户在周复盘中跳过了规划阶段（或没打开周复盘），到周初配置的提醒时间还没设定本周大石头，希望管家温和提醒"本周还没规划大石头，要不要现在定一下？"，并打开规划界面引导自己补上。

## 前置条件
- 本周无 is_big_rock=true 的任务
- 到达配置的大石头规划提醒时间（默认周一 09:00）

## 测试步骤
1. 确认本周无大石头任务
2. 等待到配置的提醒时间（或手动触发）
3. 观察管家对话区与通知

## 预期结果
- 管家在对话中温和提醒规划大石头
- 生成"轻触"级通知（不打断用户）
- 前端自动打开 WeeklyReviewModal 并进入规划阶段（initialPhase='plan'）
- 每周只触发一次（去重）

## 实际结果
<留空>

## 测试结论
<留空>
