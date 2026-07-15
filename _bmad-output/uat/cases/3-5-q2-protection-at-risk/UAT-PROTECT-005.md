---
用例编号: UAT-PROTECT-005
测试模块: Q2 任务保护
story_key: 3-5-q2-protection-at-risk
version_anchor: b248df2
exec_mode: auto
destructive: read-only
优先级: 低
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条 Q1 未完成任务，updated_at 距今 5 天
      state: { quadrant: Q1, 已完成: false, updated_at: 5天前 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证非 Q2 任务不被误标。
---

# UAT-PROTECT-005 Q1/Q3/Q4 任务即使长期未处理也不被标记 at_risk

## 业务场景
"被挤压"预警是专门为 Q2（重要不紧急）设计的——Q1 本来就紧急会自然被处理，Q3/Q4 不重要不该被保护。希望系统不把 Q1/Q3/Q4 的长期未处理任务误标为"被挤压"，避免预警泛滥。

## 前置条件
- 存在 Q1、Q3、Q4 各一条未完成任务，updated_at 均距今 ≥ 3 天

## 测试步骤
1. 准备上述三条任务
2. 触发保护检查
3. 检查这三条任务的 protection_status

## 预期结果
- Q1、Q3、Q4 任务的 protection_status 保持 normal
- 不出现"被挤压"预警标识
- 仅 Q2 任务会被标记 at_risk

## 实际结果
<留空>

## 测试结论
<留空>
