---
用例编号: UAT-BIGROCKPROTECT-002
测试模块: 大石头保护
story_key: 6-6-big-rock-daily-protection
version_anchor: f20301b58d138f47419e57488a7f11a6c04338de
exec_mode: auto
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置的 EgoSync
      state: { 已安装: true, 已有active角色: true }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 一条大石头任务，今日已被提醒 1 次
      state: { is_big_rock: true, 今日已提醒: true }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证频率控制。
---

# UAT-BIGROCKPROTECT-002 每个大石头每日最多提醒1次

## 业务场景
用户不希望同一条大石头一天被提醒好几次，希望系统对每个大石头每日最多提醒 1 次，已完成或当日有进展的不提醒，避免变成噪音。

## 前置条件
- 某大石头任务今日已被提醒 1 次

## 测试步骤
1. 确认某大石头今日已提醒
2. 同一天再次触发大石头保护检查
3. 检查是否重复提醒

## 预期结果
- 同一大石头每日最多提醒 1 次
- 当日已提醒的不重复提醒
- 已完成的大石头不提醒
- 当日有进展（updated_at 为今天）的大石头不提醒

## 实际结果
<留空>

## 测试结论
<留空>
