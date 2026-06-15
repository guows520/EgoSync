---
用例编号: UAT-TONE-006
测试模块: 角色身份隔离
story_key: 2-4-role-personalized-tone
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 对话角色
      state: { name: "产品教练", status: active }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider，用于产生真实角色回复以判读身份。
  isolation: write-isolated
  notes: 验证角色 prompt 三层分离后角色是"用户分身"身份、不复用管家身份。需人工判读身份措辞，故 manual。新建对话避免旧消息干扰。跑完后清理。
---

# UAT-TONE-006 角色以"用户分身"身份回应，不退回通用助理/管家（业务异常防护）

## 业务场景
角色的 system prompt 与管家身份是互斥的。角色应当以"用户的分身/该角色身份"自述，而不是复用管家身份或退回通用助理口吻，保证角色之间和与管家之间有清晰区分。

## 前置条件
- 进入专属角色"产品教练"角色视图，新建一个全新对话。
- 已配置可用 LLM Provider。

## 测试步骤
1. 向角色发送自我认知类问题（如"你是谁？你和管家是什么关系？"）。
2. 阅读回复的身份措辞。

## 预期结果
- 回复以该角色身份（用户分身）自述，体现角色名/目标。
- 不自称"数字管家"/管家，不退回"我是一个通用 AI 助理"式的中性口吻。

## 实际结果

## 测试结论
