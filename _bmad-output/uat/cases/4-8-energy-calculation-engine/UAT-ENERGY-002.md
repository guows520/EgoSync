---
用例编号: UAT-ENERGY-002
测试模块: 能量值引擎
story_key: 4-8-energy-calculation-engine
version_anchor: 92a316e7f554017b93e41e54462e9e59c95c391d
exec_mode: auto
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
      ref: 角色有 3 个活跃大石头、有 at_risk Q2 任务
      state: { 活跃大石头: 3, at_risk数: 2 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证公式各子项。
---

# UAT-ENERGY-002 能量值公式各子项按规则计算

## 业务场景
用户希望能量值不是随便给的数字，而是有明确公式可追溯——任务完成率占 40%、近期活跃占 30%、目标推进占 20%、at_risk 惩罚占 10%，让自己理解为什么某角色能量低。

## 前置条件
- 角色有已知数量的任务、大石头、at_risk 任务

## 测试步骤
1. 记录角色任务总数、近 7 天完成数、活跃大石头数、at_risk Q2 任务数、最近对话时间
2. 触发能量计算
3. 检查 tracing 日志中的各子项值与最终能量值

## 预期结果
- task_completion_rate = 近7天完成数/总任务数*100（总任务数为0时为0）
- recent_activity_score = 今天100/昨天80/3天前50/7天+10/无对话0
- goal_progress = (1 - 活跃大石头/3)*100，clamped [0,100]
- at_risk_penalty = at_risk Q2 任务数*20（最高60）
- 最终 energy = 0.4*完成率 + 0.3*活跃 + 0.2*目标推进 + 0.1*(100-惩罚)
- tracing 日志记录各子项值与最终值

## 实际结果
<留空>

## 测试结论
<留空>
