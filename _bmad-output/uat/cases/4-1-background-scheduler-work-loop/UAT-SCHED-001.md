---
用例编号: UAT-SCHED-001
测试模块: 后台调度器
story_key: 4-1-background-scheduler-work-loop
version_anchor: c83d6dd
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置、有至少 2 个 active 角色的 EgoSync
      state: { 已安装: true, active角色数: 2 }
      auto_generatable: true
      requirement: false
    - type: 角色配置
      ref: 角色A主动性=moderate，角色B主动性=proactive
      state: { A: moderate, B: proactive }
      auto_generatable: true
      requirement: false
  isolation: read-only
  notes: 验证调度器按档位触发，不修改数据。
---

# UAT-SCHED-001 调度器按主动性档位触发工作循环

## 业务场景
用户希望角色在应用运行期间按自己设定的频率自动"工作"——moderate 档每天几次，proactive 档更频繁，passive 档完全静默，不用自己主动发起对话角色也能持续运转。

## 前置条件
- 至少 2 个 active 角色，分别设为 moderate 和 proactive
- 应用已启动并运行

## 测试步骤
1. 启动应用，保持运行
2. 观察日志（或等待到配置的触发时间点）
3. 确认 moderate 角色按 09:00/14:00/21:00 等时间点触发
4. 确认 proactive 角色按 09:00/11:00/14:00/16:00/21:00 触发
5. 将某角色改为 passive，观察下一轮 tick 是否跳过

## 预期结果
- moderate 角色每日触发 1-2 次工作循环（间隔约 8-12 小时）
- proactive 角色每日触发 3-4 次（间隔约 4-6 小时）
- passive 角色被跳过，不触发工作循环
- 调度器不阻塞应用启动，应用关闭时自然停止

## 实际结果
<留空>

## 测试结论
<留空>
