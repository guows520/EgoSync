---
用例编号: UAT-SCHED-002
测试模块: 后台调度器
story_key: 4-1-background-scheduler-work-loop
version_anchor: c83d6dd
exec_mode: semi
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证切换档位后立即生效。
---

# UAT-SCHED-002 切换主动性档位后下次循环立即生效

## 业务场景
用户把某角色从 moderate 改成 proactive，希望不用重启应用，下一轮调度就按新档位的频率触发，体验流畅。

## 前置条件
- 某角色当前为 moderate

## 测试步骤
1. 在角色设置中把主动性从 moderate 切换为 proactive
2. 保存
3. 等待下一轮调度 tick（60 秒内）
4. 观察该角色是否按 proactive 时间点触发

## 预期结果
- 保存成功后，下次调度循环立即按新档位读取时间点
- 无需重启应用
- 调度器每次 tick 动态读取数据库，不缓存旧值

## 实际结果
<留空>

## 测试结论
<留空>
