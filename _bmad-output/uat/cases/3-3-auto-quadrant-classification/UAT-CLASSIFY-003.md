---
用例编号: UAT-CLASSIFY-003
测试模块: 任务自动分类
story_key: 3-3-auto-quadrant-classification
version_anchor: 0871096146e95899b732ed7902c8c7706896c066
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
      ref: 一条 Q2 任务，截止时间距今 1 天，未手动覆盖
      state: { quadrant: Q2, 截止: 明天, manual_override: false }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 需等待每小时定时检查触发，或手动触发临期检查。
---

# UAT-CLASSIFY-003 临期 Q2 任务自动升入 Q1

## 业务场景
用户把一件重要的事放在 Q2（重要不紧急），但没意识到截止日快到了。希望系统自动监测到"还有 2 天就到期"，把它升入 Q1（重要且紧急），提醒自己优先处理。

## 前置条件
- 存在一条 Q2、未完成、未手动覆盖、截止时间距今 ≤ 2 天的任务

## 测试步骤
1. 创建或准备一条 Q2 任务，截止时间设为明天
2. 确保该任务未被手动覆盖
3. 等待每小时临期检查触发（或手动触发检查）
4. 观察该任务象限变化

## 预期结果
- 该 Q2 任务自动升入 Q1
- 任务记录变更原因（如"截止日期已进入 2 天内，自动升入 Q1"）
- Q3/Q4 的临期任务不会被升入 Q1（不重要任务不应变为重要紧急）

## 实际结果
<留空>

## 测试结论
<留空>
