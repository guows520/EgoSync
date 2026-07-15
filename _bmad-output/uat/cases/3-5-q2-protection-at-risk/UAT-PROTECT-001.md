---
用例编号: UAT-PROTECT-001
测试模块: Q2 任务保护
story_key: 3-5-q2-protection-at-risk
version_anchor: b248df2
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
      ref: 一条 Q2 未完成任务，updated_at 距今 ≥ 3 天
      state: { quadrant: Q2, 已完成: false, updated_at: 4天前, protection_status: normal }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需等待每小时保护检查触发，或前端打开任务面板时触发一次。验证后可重置。
---

# UAT-PROTECT-001 连续 3 天未处理的 Q2 任务标记"被挤压"

## 业务场景
用户把一件重要的事放在 Q2，但好几天没碰它。希望系统监测到"这条 Q2 已被持续挤压 3 天以上"，主动标记预警，提醒自己重要不紧急的事别被遗忘。

## 前置条件
- 存在一条 Q2、未完成、未软删除、updated_at 距今 ≥ 3 天的任务

## 测试步骤
1. 准备一条满足上述条件的 Q2 任务
2. 打开任务面板（触发保护检查）或等待每小时定时检查
3. 观察该任务卡片视觉变化

## 预期结果
- 任务 protection_status 更新为 at_risk
- 卡片显示琥珀色"被挤压"预警图标（AlertTriangle）+ 文案
- 卡片左边框出现琥珀色竖线
- normal 状态的任务无任何预警标识

## 实际结果
<留空>

## 测试结论
<留空>
