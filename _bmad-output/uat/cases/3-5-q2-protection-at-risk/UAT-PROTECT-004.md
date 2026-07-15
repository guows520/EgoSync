---
用例编号: UAT-PROTECT-004
测试模块: Q2 任务保护
story_key: 3-5-q2-protection-at-risk
version_anchor: b248df2
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
      ref: 一条 at_risk 的 Q2 任务，后被临期升入 Q1
      state: { quadrant: Q1, protection_status: at_risk }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证重算的恢复语义。
---

# UAT-PROTECT-004 at_risk 任务升入 Q1 后重算恢复 normal

## 业务场景
一条 Q2 任务被标记"被挤压"后，临期检查把它升入了 Q1（已不再是 Q2）。希望保护检查重算时识别"它已不是 Q2"，把预警清掉，避免对 Q1 任务显示"被挤压"这种语义不符的标识。

## 前置条件
- 存在一条原 at_risk 的 Q2 任务，已被临期升入 Q1

## 测试步骤
1. 确认该任务当前 quadrant = Q1、protection_status = at_risk
2. 触发保护检查重算
3. 检查 protection_status

## 预期结果
- 重算识别该任务已非 Q2，将 protection_status 恢复为 normal
- 预警标识消失
- 已完成或 updated_at 在 3 天内的 at_risk 任务也恢复 normal

## 实际结果
<留空>

## 测试结论
<留空>
