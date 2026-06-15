---
用例编号: UAT-ONBOARD-005
测试模块: 五步引导首角色
story_key: 1-8-onboarding-five-step-first-role
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 全新用户数据目录
      ref: fresh_user
      state: { onboarding_completed: "不存在", roles: "空" }
      auto_generatable: true
      requirement: ""
    - type: 已配置默认 LLM
      ref: default_llm
      state: { is_default: true, connection: "正常" }
      auto_generatable: false
      requirement: 需真实有效默认 LLM 配置。
  isolation: write-isolated
  notes: 全新目录；测试后清理。
---

# UAT-ONBOARD-005 角色确认弹窗取消后可继续对话不强制创建

## 业务场景
管家提议角色后，用户可能觉得时机不对或想再聊聊。此时点击取消，不应强行创建角色，而是关闭弹窗让用户继续和管家对话，保证用户始终掌握决定权。

## 前置条件
- 全新用户进入引导并推进到角色提议步骤
- 默认 LLM 正常

## 测试步骤
1. 推进引导至管家提议角色、弹出确认弹窗
2. 在弹窗中点击取消
3. 观察是否关闭弹窗并可继续与管家对话
4. 确认此时未创建任何角色

## 预期结果
- 取消后弹窗关闭，引导对话可继续
- 此时未向角色列表写入任何角色
- 不会出现"未确认就被自动创建"的情况

## 实际结果

## 测试结论
