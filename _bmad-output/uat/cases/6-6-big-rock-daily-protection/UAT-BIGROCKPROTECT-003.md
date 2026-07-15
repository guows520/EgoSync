---
用例编号: UAT-BIGROCKPROTECT-003
测试模块: 大石头保护
story_key: 6-6-big-rock-daily-protection
version_anchor: f20301b58d138f47419e57488a7f11a6c04338de
exec_mode: semi
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
      ref: 周五仍有 2 个未完成大石头
      state: { 未完成大石头数: 2, 是周五: true }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证周五未完成检查。
---

# UAT-BIGROCKPROTECT-003 周五仍有未完成大石头时提醒周末安排

## 业务场景
周五了，用户的大石头还有几个没完成，希望管家提醒"本周大石头还有 N 个未完成，周末要安排时间吗？"，让自己有机会在周末补救，不至于一周就这么过去了。

## 前置条件
- 周五，仍有未完成大石头

## 测试步骤
1. 确认今天是周五
2. 确认仍有未完成大石头
3. 等待周五调度触发
4. 观察管家对话区

## 预期结果
- 管家提醒"本周大石头还有 N 个未完成，周末要安排时间吗？"
- N 为实际未完成大石头数
- 提醒文案温和，不制造压力
- 生成"轻触"级通知

## 实际结果
<留空>

## 测试结论
<留空>
