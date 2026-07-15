---
用例编号: UAT-Q2REMIND-002
测试模块: Q2 保护提醒
story_key: 4-6-q2-protection-butler-reminder
version_anchor: 1b21b4bf26e7a663b59928eb148e103c308b8399
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
      ref: 一条 at_risk Q2 任务，已被提醒 1 次
      state: { protection_status: at_risk, reminded_count: 1 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证提醒频率限制。
---

# UAT-Q2REMIND-002 同一任务每日最多提醒 1 次连续 3 天后停止

## 业务场景
用户不希望同一条 Q2 任务被反复提醒骚扰，希望系统对同一任务每天最多提醒 1 次，连续提醒 3 天用户仍不响应就停止提醒，避免变成噪音。

## 前置条件
- 一条 at_risk Q2 任务已被提醒 1 次

## 测试步骤
1. 记录该任务当前 reminded_count
2. 同一天再次触发 Q2 保护检查
3. 确认当天不再重复提醒
4. 模拟连续 3 天提醒后，第 4 天触发检查

## 预期结果
- 同一任务每日最多提醒 1 次
- 连续提醒 3 天无响应后停止提醒
- reminded_count 与 last_reminded_at 记录在 q2_reminders 表
- 停止后不再生成新提醒

## 实际结果
<留空>

## 测试结论
<留空>
