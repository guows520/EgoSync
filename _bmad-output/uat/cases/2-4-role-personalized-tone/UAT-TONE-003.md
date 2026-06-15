---
用例编号: UAT-TONE-003
测试模块: 角色语调差异
story_key: 2-4-role-personalized-tone
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 产品角色
      state: { name: "产品经理", status: active, personality_prompt: "简洁专业，偏结构化表达" }
      auto_generatable: true
      requirement: ""
    - type: 角色
      ref: 家庭角色
      state: { name: "家庭", status: active, personality_prompt: "温暖关怀，偏情感支持" }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider，用于产生真实角色回复以判读语调差异。
  isolation: write-isolated
  notes: 创建两个个性不同的专属角色，对同一问题分别提问比较语调。语调差异需人工主观判读，故 manual。跑完后清理。
---

# UAT-TONE-003 不同角色对同一问题有可感知的语调差异

## 业务场景
用户分别向"产品经理"和"家庭"角色问同一个问题，期望两者回复风格明显不同——产品经理更结构化专业，家庭角色更温暖关怀——这样不同角色才有区分感。

## 前置条件
- 存在两个个性描述不同的 active 专属角色："产品经理"（简洁专业）与"家庭"（温暖关怀）。
- 已配置可用 LLM Provider。

## 测试步骤
1. 进入"产品经理"角色视图，发送一个生活/情绪相关问题（如"我最近压力很大，怎么办"）并阅读回复。
2. 进入"家庭"角色视图，发送完全相同的问题并阅读回复。
3. 对比两条回复的语气与结构。

## 预期结果
- "产品经理"回复偏结构化、专业判断（如分点、给优先级/行动建议）。
- "家庭"回复偏温暖、关怀、情感支持（先回应感受再给建议）。
- 两者差异明显可感知，且差异来自角色身份/个性（非前端伪装）。

## 实际结果

## 测试结论
