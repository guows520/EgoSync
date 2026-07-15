---
用例编号: UAT-SCHEDCFG-002
测试模块: 节奏化时间配置
story_key: 6-2-schedule-config-briefing-review
version_anchor: 1d51afb
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证部分更新语义。
---

# UAT-SCHEDCFG-002 部分更新时间配置不影响其他项

## 业务场景
用户只想改晨间简报时间，不动周复盘和大石头规划，希望只传晨间简报的字段保存，其他两项保持不变。

## 前置条件
- 已有三项时间配置

## 测试步骤
1. 记录当前三项时间配置
2. 仅修改晨间简报时间并保存
3. 检查周复盘与大石头规划时间是否变化

## 预期结果
- 仅晨间简报时间更新
- 周复盘与大石头规划时间保持不变
- settings_update_schedule 支持部分更新（未传字段保持原值）

## 实际结果
<留空>

## 测试结论
<留空>
