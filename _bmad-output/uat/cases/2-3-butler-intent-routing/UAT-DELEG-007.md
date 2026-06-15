---
用例编号: UAT-DELEG-007
测试模块: 事实记忆与任务分流
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 家庭角色
      state: { name: "家庭助理", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 sidecar/bridge 就绪。
  isolation: write-isolated
  notes: 验证管家对"陈述事实"不委派、对"任务安排"委派的分流。需人工判读，故 manual。跑完后清理对话。
---

# UAT-DELEG-007 陈述事实不委派、交代任务才委派（业务异常防护）

## 业务场景
用户有时只是陈述一个事实或偏好（不需要派活），有时是交代一件需要后续行动的任务。管家应当区分两者：事实直接回应，任务才委派给角色，避免把闲聊当任务乱派。

## 前置条件
- 存在 active 专属角色"家庭助理"。
- LLM Provider 可用，sidecar/bridge 就绪，处于管家视角。

## 测试步骤
1. 在管家对话输入一句事实陈述（如"我儿子叫小米米，他喜欢薯条"）并发送，观察管家反应。
2. 再输入一句任务安排（如"明天下午要参加儿子的家长会，帮我安排"）并发送，观察管家反应。

## 预期结果
- 对事实陈述：管家直接确认/回应，不发起委派。
- 对任务安排：管家发起委派给匹配角色（如家庭助理），并在管家气泡中转述处理结果。

## 实际结果

## 测试结论
