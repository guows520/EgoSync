---
用例编号: UAT-TONE-004
测试模块: 个性更新即时生效
story_key: 2-4-role-personalized-tone
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 角色
      ref: 目标角色
      state: { name: "产品教练", status: active, personality_prompt: "" }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider，用于对比修改前后回复风格。
  isolation: write-isolated
  notes: 修改个性描述后立即对话，验证下次回复风格随之变化。语气变化需人工判读，故 manual。跑完后清理。
---

# UAT-TONE-004 修改个性描述后下次对话即反映新语调

## 业务场景
用户调整了角色个性描述并保存，希望紧接着的下一轮对话就能体现新的语调，而不是要重启应用才生效。

## 前置条件
- 进入专属角色"产品教练"角色视图，个性描述初始为空或较中性。
- 已配置可用 LLM Provider。

## 测试步骤
1. 先向角色发一条问题并记录其回复风格作为基线。
2. 打开设置页，把个性描述改为一个鲜明风格（如"用非常简短、犀利、只给要点的方式回答"）并保存。
3. 回到对话，发送一条同类问题。

## 预期结果
- 保存后无需重启应用。
- 修改后下一轮回复明显体现新设定的语调（如变得更简短犀利）。

## 实际结果

## 测试结论
