---
用例编号: UAT-Q2REMIND-001
测试模块: Q2 保护提醒
story_key: 4-6-q2-protection-butler-reminder
version_anchor: 1b21b4bf26e7a663b59928eb148e103c308b8399
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
      ref: 一条 at_risk 的 Q2 任务，标题"写季度总结"
      state: { quadrant: Q2, protection_status: at_risk, 标题: "写季度总结" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证后清理提醒记录。
---

# UAT-Q2REMIND-001 Q2 任务被挤时管家温和提醒

## 业务场景
用户的某条 Q2（重要不紧急）任务被持续挤压 3 天以上，希望管家在对话中以自然语言温和提醒"你的'写季度总结'任务已经 3 天没动了，要不要今天安排一下？"，不用 toast/红框打断，遵循管家对话反馈模式。

## 前置条件
- 存在一条 at_risk 的 Q2 任务

## 测试步骤
1. 确保某 Q2 任务已被标记为 at_risk
2. 等待调度器触发 Q2 保护检查
3. 观察管家对话区

## 预期结果
- 管家在对话中以自然语言提醒，包含任务标题
- 提醒文案温和（如"你的'XX'任务已经 3 天没动了，要不要今天安排一下？"）
- 不使用 toast / 红框
- 同时生成"轻触"级通知（不打断用户）

## 实际结果
<留空>

## 测试结论
<留空>
