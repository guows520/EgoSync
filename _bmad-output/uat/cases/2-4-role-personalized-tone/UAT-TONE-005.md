---
用例编号: UAT-TONE-005
测试模块: 空个性兼容
story_key: 2-4-role-personalized-tone
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 角色
      ref: 空个性角色
      state: { name: "学习者", status: active, personality_prompt: "" }
      auto_generatable: true
      requirement: ""
    - type: LLM Provider
      ref: 对话模型
      state: { available: true }
      auto_generatable: false
      requirement: 需配置可用 LLM Provider，用于验证空个性角色仍能正常对话。
  isolation: write-isolated
  notes: 验证个性描述为空的角色不报错、仍能对话且不自称管家。需人工判读身份与口吻，故 manual。跑完后清理。
---

# UAT-TONE-005 个性描述为空的角色仍正常对话且不自称管家（业务异常防护）

## 业务场景
对于尚未填写个性描述的角色（含既有老角色），系统应当兼容：列表、保存、对话都不报错，回复仍以角色身份进行，不退回管家口吻。

## 前置条件
- 存在 active 专属角色"学习者"，其个性描述为空。
- 已配置可用 LLM Provider。

## 测试步骤
1. 在侧边栏确认"学习者"正常显示。
2. 进入"学习者"角色视图，发送一条问题（如"帮我制定一个学习计划"）。
3. 阅读回复内容与身份。

## 预期结果
- 角色列表、设置保存、对话构建均不报错，无需数据回填。
- 角色能正常回复，且根据角色名/目标体现轻量方向（如学习者更偏探索/启发）。
- 回复不自称"数字管家"/管家身份。

## 实际结果

## 测试结论
