---
用例编号: UAT-DELEG-002
测试模块: 多角色并行委派
story_key: 2-3-butler-intent-routing
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 产品角色
      state: { name: "产品经理", status: active }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 健康角色
      state: { name: "健康教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 委派模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider 与 opencode sidecar/delegate bridge 就绪。
  isolation: write-isolated
  notes: 创建 2 个专属角色；单条用户消息触发两次委派。委派识别与转述需人工判读，故 manual。跑完后清理专属角色及对话。
---

# UAT-DELEG-002 单条消息涉及两个角色时并行委派并合并转述

## 业务场景
用户一句话里包含两件分属不同角色的事，希望管家能一次识别两个角色、分别处理，再把两边结果一起讲清楚，不用拆成两次提问。

## 前置条件
- 存在 active 专属角色"产品经理"和"健康教练"。
- LLM Provider 可用，sidecar/bridge 就绪。
- 当前处于管家视角。

## 测试步骤
1. 在管家对话中输入"帮我安排今天的工作和健身"并发送。
2. 观察管家是否同时提及两个角色的委派。
3. 等待两边结果都转述完毕。

## 预期结果
- 管家在同一轮内识别出 2 个目标角色（产品经理 + 健康教练）并分别委派。
- 用户感知为管家"稍等"后流式恢复，最终把两个角色的结果一并转述。
- 全程在管家对话流内，无视图切换或弹窗。

## 实际结果

## 测试结论
