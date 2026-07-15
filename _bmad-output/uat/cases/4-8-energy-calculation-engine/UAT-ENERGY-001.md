---
用例编号: UAT-ENERGY-001
测试模块: 能量值引擎
story_key: 4-8-energy-calculation-engine
version_anchor: 92a316e7f554017b93e41e54462e9e59c95c391d
exec_mode: auto
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
      ref: 角色有任务与对话记录
      state: { 有任务: true, 有对话: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 能量值会写入 roles.energy，验证后可重置。
---

# UAT-ENERGY-001 工作循环后自动计算并更新角色能量值

## 业务场景
用户希望每个角色有一个动态变化的"能量值"反映角色健康度，系统每次工作循环后自动重算并更新，让自己直觉地感知哪个角色需要更多关注。

## 前置条件
- 角色有任务与对话记录

## 测试步骤
1. 记录角色当前 energy 与 energy_updated_at
2. 触发一次工作循环
3. 检查 roles.energy 与 energy_updated_at

## 预期结果
- 工作循环完成后自动计算能量值
- 新值写入 roles.energy（0-100 整数）
- energy_updated_at 写入当前 ISO 8601 时间戳
- 能量值基于公式：0.4*任务完成率 + 0.3*近期活跃 + 0.2*目标推进 + 0.1*(100-at_risk惩罚)

## 实际结果
<留空>

## 测试结论
<留空>
