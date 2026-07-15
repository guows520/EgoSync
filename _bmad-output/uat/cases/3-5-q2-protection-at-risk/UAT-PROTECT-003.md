---
用例编号: UAT-PROTECT-003
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
      ref: 一条 at_risk 的 Q2 任务，updated_at 距今 4 天
      state: { quadrant: Q2, protection_status: at_risk, updated_at: 4天前 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证标记 at_risk 不刷新 updated_at 的关键语义。
---

# UAT-PROTECT-003 标记 at_risk 不刷新 updated_at

## 业务场景
系统把一条 Q2 任务标记为"被挤压"时，不应假装"用户刚处理过它"——否则会立即被判定为"已处理"导致状态抖动。希望标记行为只改 protection_status，不动 updated_at。

## 前置条件
- 存在一条 Q2、未完成、updated_at 距今 ≥ 3 天、protection_status = normal 的任务
- 记录其当前 updated_at

## 测试步骤
1. 记录该任务当前 updated_at
2. 触发保护检查，使其被标记为 at_risk
3. 再次检查该任务的 updated_at

## 预期结果
- protection_status 变为 at_risk
- updated_at 保持不变（未被刷新）
- 不会出现"标记后立即又被判定为已处理"的状态抖动

## 实际结果
<留空>

## 测试结论
<留空>
